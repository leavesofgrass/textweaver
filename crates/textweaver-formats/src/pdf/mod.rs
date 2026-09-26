//! PDF loader (feature `pdf`, on by default): pure Rust, on lopdf, with the
//! page layout rebuilt into Markdown-quality structure (ADR-0010).
//!
//! The pipeline, per page and then across the document:
//!
//! 1. **Glyphs** (`interp`): the content stream is interpreted (text
//!    state, fonts, form XObjects, marked content), giving each glyph's
//!    position, size, and bold/italic/monospace style; images are located.
//! 2. **Lines, tables, blocks** (`layout`): glyphs become lines split at
//!    wide gaps; aligned rows of short cells become tables; lines stack into
//!    blocks by spacing, overlap, size, and weight.
//! 3. **Running heads and reading order** (`layout`): Star's algorithm
//!    removes repeated margin text and page numbers and orders blocks column
//!    by column within bands divided by full-width blocks.
//! 4. **Structure** (`structure`): paragraphs with wrapped lines joined
//!    and line-end hyphens removed, continued across column and page ends;
//!    headings from the tag tree, size, weight, numbering, and the outline;
//!    bulleted and numbered lists with nesting; monospaced blocks as code;
//!    images with alternate text (marked content `/Alt` or the structure
//!    tree's `Figure` elements).
//! 5. **Markers**: `PageBreak` per page (label = the printed page label
//!    from `/PageLabels`, such as `iv` or `A-3`, else the page number;
//!    range = that page's text, ready for page navigation, which the reader
//!    does not offer yet), `SectionBreak` per
//!    outline entry (label = its title, level = its depth), and the usual
//!    heading, paragraph, list, table, code, and image markers.
//!
//! A page whose content cannot be parsed is skipped, never fatal; a
//! password-protected PDF is refused with a clear message.
//!
//! Pages with no text but a picture (scans) are recognized by OCR (feature
//! `ocr`, ADR-0023; see `ocr`): the recognized words are placed on the page
//! as glyphs and go through the same layout. Without OCR, or when no
//! engine can run, a PDF with no text layer loads as one sentence saying
//! so and what is missing.

mod fonts;
#[cfg(feature = "ocr")]
pub mod image;
mod interp;
mod layout;
mod metrics;
#[cfg(feature = "ocr")]
mod ocr;
mod structure;

use std::collections::HashMap;

use lopdf::{Dictionary, Object, ObjectId};
use ropey::Rope;
use textweaver_core::MarkerKind;
use textweaver_text::{Document, DocumentMeta};

use crate::builder::Builder;
use crate::{LoadError, LoadOptions, Loader, Source, meta_for, title_from_path};

/// What a PDF without a text layer reads as.
pub const NO_TEXT_LAYER: &str = "This PDF has no text layer. It is probably a scanned image, so its text must be recognized (OCR) before it can be read aloud.";

/// Loads PDF documents.
#[derive(Clone, Copy, Debug, Default)]
pub struct PdfLoader;

impl Loader for PdfLoader {
    fn id(&self) -> &'static str {
        "pdf"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["pdf"]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let bytes = source.read()?;
        let mut pdf = lopdf::Document::load_mem(&bytes)
            .map_err(|e| LoadError::Parse(format!("not a readable PDF: {e}")))?;
        if pdf.is_encrypted() && pdf.decrypt("").is_err() {
            return Err(LoadError::Unsupported(
                "this PDF is protected by a password, so its text cannot be read".into(),
            ));
        }
        let mut meta = meta_for(source, self.id());
        read_info(&pdf, &mut meta);
        let (text, markers, pages) = convert(&pdf, &bytes, options, &mut meta);
        meta.properties.insert("pages".into(), pages.to_string());
        if meta.title.is_none() {
            meta.title = markers
                .iter()
                .find(|m| m.kind == MarkerKind::Heading && m.level == 1)
                .map(|m| {
                    text.chars()
                        .skip(m.range.start.0)
                        .take(m.range.len())
                        .collect()
                })
                .or_else(|| title_from_path(source));
        }
        Ok(Document::new(meta, Rope::from_str(&text), markers))
    }
}

/// Converts a loaded PDF to canonical text and markers, recognizing the
/// text of pages that have none (feature `ocr`); also returns the page
/// count.
fn convert(
    pdf: &lopdf::Document,
    #[cfg_attr(not(feature = "ocr"), allow(unused_variables))] bytes: &[u8],
    #[cfg_attr(not(feature = "ocr"), allow(unused_variables))] options: &LoadOptions,
    #[cfg_attr(not(feature = "ocr"), allow(unused_variables))] meta: &mut DocumentMeta,
) -> (String, Vec<textweaver_text::Marker>, usize) {
    let page_ids: Vec<ObjectId> = pdf.get_pages().into_values().collect();
    let tags = Tags::read(pdf);
    let mut fonts = interp::FontCache::default();
    let mut contents = Vec::with_capacity(page_ids.len());
    let mut roles: HashMap<(usize, u32), String> = HashMap::new();
    for (i, &id) in page_ids.iter().enumerate() {
        let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            interp::run_page(pdf, id, &mut fonts)
        }));
        contents.push(run.unwrap_or_else(|_| interp::PageContent {
            failed: true,
            ..interp::PageContent::default()
        }));
        for ((p, m), r) in &tags.roles {
            if *p == id {
                roles.insert((i, *m), r.clone());
            }
        }
    }
    // Pages with no text but a picture (a scan), or that could not be
    // interpreted; an empty page has nothing to recognize.
    let blank: Vec<usize> = (0..contents.len())
        .filter(|&i| {
            let c = &contents[i];
            c.glyphs.is_empty() && (!c.images.is_empty() || c.failed)
        })
        .collect();
    #[cfg_attr(not(feature = "ocr"), allow(unused_mut))]
    let mut no_text = NO_TEXT_LAYER.to_owned();
    #[cfg(feature = "ocr")]
    if !blank.is_empty() && options.ocr.enabled {
        let lang = meta.language.clone();
        let outcome = ocr::recognize_pages(bytes, &mut contents, &blank, lang.as_deref(), options);
        if let Some(sentence) = outcome.report(meta, contents.len(), &blank) {
            no_text = sentence;
        }
    }
    if contents.iter().all(|c| c.glyphs.is_empty()) {
        let text = if page_ids.is_empty() {
            String::new()
        } else {
            no_text
        };
        return (text, Vec::new(), page_ids.len());
    }
    let pages: Vec<layout::Page> = contents
        .iter()
        .zip(&page_ids)
        .map(|(content, id)| {
            let alts: HashMap<u32, String> = tags
                .alts
                .iter()
                .filter(|((p, _), _)| p == id)
                .map(|((_, m), a)| (*m, a.clone()))
                .collect();
            layout::layout(content, &alts)
        })
        .collect();
    let labels = page_labels(pdf, page_ids.len());
    let outline = read_outline(pdf);
    let (text, markers) = build(pages, &roles, &outline, &labels);
    (text, markers, page_ids.len())
}

/// Laid-out pages as canonical text and markers: running heads removed,
/// reading order, structure, and the page and section markers.
fn build(
    mut pages: Vec<layout::Page>,
    roles: &HashMap<(usize, u32), String>,
    outline: &[(String, usize, u8)],
    labels: &[String],
) -> (String, Vec<textweaver_text::Marker>) {
    layout::remove_running(&mut pages);
    for p in &mut pages {
        layout::order(p);
    }
    let (body_size, body_bold) = layout::body_style(&pages);
    let cx = structure::Context {
        body_size,
        body_bold,
        roles,
        outline,
    };
    let (units, sections) = structure::units(&pages, &cx);
    let mut b = Builder::new();
    structure::emit(&mut b, &units, &sections, outline, labels);
    b.finish()
}

fn text_of(pdf: &lopdf::Document, o: &Object) -> Option<String> {
    let o = pdf.dereference(o).ok()?.1;
    let t = lopdf::decode_text_string(o).ok()?;
    let t: String = t.split_whitespace().collect::<Vec<_>>().join(" ");
    (!t.is_empty()).then_some(t)
}

/// Title and author from the document information dictionary, language
/// from the catalog.
fn read_info(pdf: &lopdf::Document, meta: &mut textweaver_text::DocumentMeta) {
    if let Some(info) = pdf
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|o| pdf.dereference(o).ok())
        .and_then(|(_, o)| o.as_dict().ok())
    {
        let get = |k: &[u8]| info.get(k).ok().and_then(|o| text_of(pdf, o));
        meta.title = get(b"Title");
        meta.author = get(b"Author");
        if let Some(s) = get(b"Subject") {
            meta.properties.insert("subject".into(), s);
        }
    }
    if let Ok(cat) = pdf.catalog()
        && let Some(lang) = cat.get(b"Lang").ok().and_then(|o| text_of(pdf, o))
    {
        meta.language = Some(lang);
    }
}

/// Printed page labels from the catalog's `/PageLabels` number tree
/// (decimal, roman, or letter numbering with prefixes and start values), or
/// an empty list when the PDF has none.
///
/// A hostile file can claim any start value and prefix: counters saturate,
/// letters and roman numerals fall back to decimal when they would grow
/// long, and labels are cut to a few dozen characters (`crate::counter`).
fn page_labels(pdf: &lopdf::Document, pages: usize) -> Vec<String> {
    let Some(tree) = pdf
        .catalog()
        .ok()
        .and_then(|c| c.get(b"PageLabels").ok())
        .and_then(|o| pdf.dereference(o).ok())
        .and_then(|(_, o)| o.as_dict().ok())
    else {
        return Vec::new();
    };
    let mut ranges: Vec<(usize, &Dictionary)> = Vec::new();
    collect_nums(pdf, tree, &mut ranges, 0);
    ranges.sort_by_key(|(start, _)| *start);
    if ranges.is_empty() {
        return Vec::new();
    }
    // Each range's style, read once (a file can list many ranges).
    let styles: Vec<(usize, u64, String, Vec<u8>)> = ranges
        .iter()
        .map(|&(start, style)| {
            let first = style
                .get(b"St")
                .ok()
                .and_then(|o| o.as_i64().ok())
                .map_or(1, |n| crate::counter::clamp(i128::from(n)));
            let prefix = style
                .get(b"P")
                .ok()
                .and_then(|o| text_of(pdf, o))
                .map(crate::counter::cap_label)
                .unwrap_or_default();
            let kind = style
                .get(b"S")
                .and_then(Object::as_name)
                .unwrap_or(b"")
                .to_vec();
            (start, first, prefix, kind)
        })
        .collect();
    (0..pages)
        .map(|p| {
            let i = styles.partition_point(|(s, ..)| *s <= p);
            let Some((start, first, prefix, kind)) = i.checked_sub(1).map(|i| &styles[i]) else {
                return (p + 1).to_string();
            };
            let offset = u64::try_from(p - start).unwrap_or(u64::MAX);
            let n = first
                .saturating_add(offset)
                .min(crate::counter::MAX_COUNTER);
            let number = match kind.as_slice() {
                b"D" => crate::counter::decimal(n),
                b"R" => crate::counter::roman(n),
                b"r" => crate::counter::roman(n).to_lowercase(),
                b"A" => crate::counter::letters(n),
                b"a" => crate::counter::letters(n).to_lowercase(),
                _ => String::new(),
            };
            let label = crate::counter::cap_label(format!("{prefix}{number}"));
            if label.is_empty() {
                (p + 1).to_string()
            } else {
                label
            }
        })
        .collect()
}

fn collect_nums<'a>(
    pdf: &'a lopdf::Document,
    node: &'a Dictionary,
    out: &mut Vec<(usize, &'a Dictionary)>,
    depth: usize,
) {
    if depth > 32 {
        return;
    }
    if let Ok(nums) = node.get(b"Nums").and_then(Object::as_array) {
        for pair in nums.chunks(2) {
            if let [k, v] = pair
                && let Ok(k) = k.as_i64()
                && let Ok((_, Object::Dictionary(d))) = pdf.dereference(v)
                && let Ok(k) = usize::try_from(k)
            {
                out.push((k, d));
            }
        }
    }
    if let Ok(kids) = node.get(b"Kids").and_then(Object::as_array) {
        for kid in kids {
            if let Ok((_, Object::Dictionary(d))) = pdf.dereference(kid) {
                collect_nums(pdf, d, out, depth + 1);
            }
        }
    }
}

/// The outline (bookmarks): title, 0-based page, and depth (1 = top).
fn read_outline(pdf: &lopdf::Document) -> Vec<(String, usize, u8)> {
    let Ok(toc) = pdf.get_toc() else {
        return Vec::new();
    };
    toc.toc
        .into_iter()
        .filter(|e| e.page > 0)
        .map(|e| {
            let title = e.title.split_whitespace().collect::<Vec<_>>().join(" ");
            (
                title,
                e.page - 1,
                u8::try_from(e.level.clamp(1, 6)).unwrap_or(1),
            )
        })
        .collect()
}

/// What a tagged PDF's structure tree says about marked content.
#[derive(Default)]
struct Tags {
    /// (page, MCID) → the standard role of its nearest block element.
    roles: HashMap<(ObjectId, u32), String>,
    /// (page, MCID) → alternate text of the `Figure` around it.
    alts: HashMap<(ObjectId, u32), String>,
}

/// Inline structure types: their content belongs to the block around them.
const INLINE: &[&str] = &[
    "Span",
    "Link",
    "Annot",
    "Ruby",
    "Warichu",
    "Reference",
    "BibEntry",
    "Quote",
    "Code",
    "Note",
    "Lbl",
    "Em",
    "Strong",
    "Sub",
    "NonStruct",
    "Private",
];

impl Tags {
    fn read(pdf: &lopdf::Document) -> Tags {
        let mut tags = Tags::default();
        let Some(root) = pdf
            .catalog()
            .ok()
            .and_then(|c| c.get(b"StructTreeRoot").ok())
            .and_then(|o| pdf.dereference(o).ok())
            .and_then(|(_, o)| o.as_dict().ok())
        else {
            return tags;
        };
        let role_map = root
            .get(b"RoleMap")
            .ok()
            .and_then(|o| pdf.dereference(o).ok())
            .and_then(|(_, o)| o.as_dict().ok());
        let mut w = TagWalker {
            pdf,
            role_map,
            tags: &mut tags,
            visited: 0,
        };
        if let Ok(k) = root.get(b"K") {
            w.walk(k, None, None, None, 0);
        }
        tags
    }
}

struct TagWalker<'a> {
    pdf: &'a lopdf::Document,
    role_map: Option<&'a Dictionary>,
    tags: &'a mut Tags,
    visited: usize,
}

impl TagWalker<'_> {
    fn standard_role(&self, name: &[u8]) -> String {
        let mut name = name.to_vec();
        for _ in 0..8 {
            let Some(next) = self
                .role_map
                .and_then(|m| m.get(&name).ok())
                .and_then(|o| o.as_name().ok())
            else {
                break;
            };
            name = next.to_vec();
        }
        String::from_utf8_lossy(&name).into_owned()
    }

    fn walk(
        &mut self,
        node: &Object,
        page: Option<ObjectId>,
        role: Option<&str>,
        alt: Option<&str>,
        depth: usize,
    ) {
        self.visited += 1;
        if depth > 128 || self.visited > 2_000_000 {
            return;
        }
        let Ok((_, node)) = self.pdf.dereference(node) else {
            return;
        };
        match node {
            Object::Integer(mcid) => self.leaf(page, *mcid, role, alt),
            Object::Array(items) => {
                for i in items {
                    self.walk(i, page, role, alt, depth + 1);
                }
            }
            Object::Dictionary(d) => {
                let ty = d.get(b"Type").and_then(Object::as_name).unwrap_or(b"");
                let pg = d.get(b"Pg").and_then(Object::as_reference).ok().or(page);
                match ty {
                    b"MCR" => {
                        if let Ok(mcid) = d.get(b"MCID").and_then(Object::as_i64) {
                            self.leaf(pg, mcid, role, alt);
                        }
                    }
                    b"OBJR" => {}
                    _ => {
                        let own = d
                            .get(b"S")
                            .and_then(Object::as_name)
                            .map(|s| self.standard_role(s))
                            .unwrap_or_default();
                        let own_alt = d.get(b"Alt").ok().and_then(|o| text_of(self.pdf, o));
                        let role = if own.is_empty() || INLINE.contains(&own.as_str()) {
                            role.map(str::to_owned)
                        } else {
                            Some(own.clone())
                        };
                        let alt = if own == "Figure" {
                            own_alt.or_else(|| alt.map(str::to_owned))
                        } else {
                            alt.map(str::to_owned)
                        };
                        if let Ok(k) = d.get(b"K") {
                            self.walk(k, pg, role.as_deref(), alt.as_deref(), depth + 1);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn leaf(&mut self, page: Option<ObjectId>, mcid: i64, role: Option<&str>, alt: Option<&str>) {
        let (Some(page), Ok(mcid)) = (page, u32::try_from(mcid)) else {
            return;
        };
        if let Some(r) = role {
            self.tags.roles.insert((page, mcid), r.to_owned());
        }
        if let Some(a) = alt {
            self.tags.alts.insert((page, mcid), a.to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use lopdf::{StringFormat, dictionary};

    use super::*;

    /// A document whose catalog's `/PageLabels` tree holds `nums`.
    fn with_labels(nums: Vec<Object>) -> lopdf::Document {
        let mut pdf = lopdf::Document::with_version("1.7");
        let tree = pdf.add_object(dictionary! { "Nums" => nums });
        let catalog = pdf.add_object(dictionary! {
            "Type" => "Catalog",
            "PageLabels" => tree,
        });
        pdf.trailer.set("Root", catalog);
        pdf
    }

    fn style(s: &str, start: i64, prefix: Option<&str>) -> Object {
        let mut d = dictionary! { "S" => Object::Name(s.as_bytes().to_vec()), "St" => start };
        if let Some(p) = prefix {
            d.set(
                "P",
                Object::String(p.as_bytes().to_vec(), StringFormat::Literal),
            );
        }
        Object::Dictionary(d)
    }

    #[test]
    fn hostile_page_labels_are_clamped() {
        let huge_prefix = "x".repeat(10_000);
        let pdf = with_labels(vec![
            Object::Integer(0),
            style("R", i64::MAX, None),
            Object::Integer(2),
            style("A", i64::MAX - 1, Some(&huge_prefix)),
            Object::Integer(4),
            style("D", i64::MIN, None),
            Object::Integer(6),
            style("a", 1_000_000_000, None),
            Object::Integer(8),
            style("r", 3_998, None),
        ]);
        let labels = page_labels(&pdf, 10);
        assert_eq!(labels.len(), 10);
        for l in &labels {
            assert!(l.chars().count() <= crate::counter::MAX_LABEL_CHARS, "{l}");
        }
        // Roman past 3,999 and letters past ten repeats read as decimal,
        // and counters stop at the cap instead of overflowing.
        assert_eq!(labels[0], "999999");
        assert_eq!(labels[1], "999999");
        assert!(labels[2].starts_with("xxxx"));
        // A negative start is clamped to 1.
        assert_eq!(labels[4], "1");
        assert_eq!(labels[5], "2");
        assert_eq!(labels[6], "999999");
        assert_eq!(labels[8], "mmmcmxcviii");
        assert_eq!(labels[9], "mmmcmxcix");
    }

    #[test]
    fn page_labels_start_where_their_range_starts() {
        let pdf = with_labels(vec![
            Object::Integer(0),
            style("r", 1, None),
            Object::Integer(3),
            style("D", 1, Some("A-")),
        ]);
        assert_eq!(page_labels(&pdf, 5), vec!["i", "ii", "iii", "A-1", "A-2"]);
    }
}
