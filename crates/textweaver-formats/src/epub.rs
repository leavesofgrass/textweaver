//! EPUB loader (EPUB 2 and 3): the spine's XHTML documents in reading order,
//! through the HTML loader's rules, with the table of contents as chapters.
//!
//! - `META-INF/container.xml` names the package document (OPF); its
//!   `<metadata>` gives the title (`dc:title`), author (`dc:creator`, all of
//!   them joined), and language (`dc:language`).
//! - The spine's HTML and XHTML items are read in order, each starting a new
//!   paragraph, with the same skip list, image alt text, and structure as
//!   [`HtmlLoader`](crate::HtmlLoader) (the EPUB 3 navigation document is not
//!   read as content). Unlike star, chapters are not joined with `---` rules
//!   and no title heading is invented: the title is metadata.
//! - The table of contents (the EPUB 3 `nav` with `epub:type="toc"`, else
//!   the EPUB 2 NCX) becomes `SectionBreak` markers: one per entry, labeled
//!   with the entry's title, `level` = nesting depth (1 for top-level
//!   entries), starting where the entry points (a spine item, or an element
//!   id inside one) and ending where the next entry of the same or a
//!   shallower depth starts. So "next chapter" follows the book's own table
//!   of contents, and nested entries nest. Without a table of contents,
//!   every spine item is a level-1 section labeled by its first heading.
//! - Print page numbers become `PageBreak` markers labeled with the printed
//!   number, as in PDF and DAISY: the content's `epub:type="pagebreak"` and
//!   `role="doc-pagebreak"` elements, and the ids the EPUB 3 page list (or
//!   the EPUB 2 NCX `pageList`) names. Each page runs to the next, across
//!   chapters, and the numbers themselves are not read aloud.
//! - MathML is read as math (EPUB 3; ADR-0029): each `<math>` becomes LaTeX
//!   with its delimiters under a `Math` marker, as in the Markdown and DOCX
//!   loaders, so speech reads it as math (through MathCAT when the math
//!   engine setting asks) and writers typeset it. Without any math, its
//!   `alttext` is read. An `epub:switch` reads its MathML case, else its
//!   default.
//! - A book whose content documents are encrypted (DRM) is refused with a
//!   clear error rather than read as noise; obfuscated fonts are fine.
//! - The package document's accessibility metadata (EPUB Accessibility 1.0
//!   and 1.1) is kept as an [`Accessibility`] under
//!   [`ACCESSIBILITY_PROPERTY`]: the summary, access modes, sufficient
//!   access modes, features, hazards, conformance claim
//!   (`dcterms:conformsTo` as a `meta` or the older `link`) and certifier
//!   (`a11y:certifiedBy`). EPUB 2's `<meta name content>` form is read too.
//!   A book without any has the property with nothing in it, so readers
//!   can say "No accessibility information given."; [`accessibility`]
//!   reads it back. Values are cut at [`MAX_ACCESS_CHARS`], lists at
//!   [`MAX_ACCESS_ITEMS`], and control characters are dropped.

use std::collections::{HashMap, HashSet};

use ropey::Rope;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, DocumentMeta, Marker};

use crate::builder::{Builder, OpenId};
use crate::package::{Package, child, dir_of, parse_xml, resolve};
use crate::{LoadError, LoadOptions, Loader, Source, meta_for, title_from_path};

/// Loads EPUB books.
#[derive(Clone, Copy, Debug, Default)]
pub struct EpubLoader;

impl Loader for EpubLoader {
    fn id(&self) -> &'static str {
        "epub"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["epub"]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let mut pkg = Package::open(source.read()?, "EPUB")?;
        let mut meta = meta_for(source, self.id());
        let book = read_book(&mut pkg)?;
        meta.title = book.title.clone();
        meta.author = book.author.clone();
        meta.language = book.language.clone();
        if let Ok(json) = serde_json::to_string(&book.access) {
            meta.properties
                .insert(ACCESSIBILITY_PROPERTY.to_owned(), json);
        }
        if meta.title.is_none() {
            meta.title = title_from_path(source);
        }
        let (text, markers) = convert(&mut pkg, &book, options, &mut meta)?;
        if pkg.flattened() {
            crate::add_warning(&mut meta, crate::NESTING_WARNING);
        }
        Ok(Document::new(meta, Rope::from_str(&text), markers))
    }
}

/// One entry of the table of contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TocEntry {
    pub(crate) title: String,
    /// Normalized member path.
    pub(crate) file: String,
    pub(crate) fragment: Option<String>,
    pub(crate) depth: u8,
}

/// What the package document says.
#[derive(Debug, Default)]
struct Book {
    title: Option<String>,
    author: Option<String>,
    language: Option<String>,
    /// Content documents in reading order (member paths).
    spine: Vec<String>,
    toc: Vec<TocEntry>,
    /// The page list: each entry's title is a printed page number.
    pages: Vec<TocEntry>,
    /// The accessibility metadata.
    access: Accessibility,
}

/// The `DocumentMeta::properties` key holding an EPUB's accessibility
/// metadata, as JSON (see [`accessibility`]).
pub const ACCESSIBILITY_PROPERTY: &str = "textweaver.accessibility";

/// Most characters kept from one accessibility value (the summary, a
/// conformance claim, a certifier); the rest is cut, ending "...".
pub const MAX_ACCESS_CHARS: usize = 1_000;

/// Most entries kept in one accessibility list (features, hazards).
pub const MAX_ACCESS_ITEMS: usize = 32;

/// Most characters in one listed term ("structuralNavigation").
const MAX_TERM_CHARS: usize = 64;

/// What an EPUB's package document says about its accessibility (EPUB
/// Accessibility 1.0 and 1.1, schema.org terms). The terms are kept as
/// written; [`term_words`] says one in words.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Accessibility {
    /// The publisher's summary (`schema:accessibilitySummary`).
    pub summary: Option<String>,
    /// How the content can be perceived (`schema:accessMode`: `textual`,
    /// `visual`, `auditory`).
    pub access_modes: Vec<String>,
    /// Sets of access modes that are enough to read it all
    /// (`schema:accessModeSufficient`), each a list such as
    /// `["textual", "visual"]`.
    pub sufficient: Vec<Vec<String>>,
    /// Features (`schema:accessibilityFeature`: `tableOfContents`).
    pub features: Vec<String>,
    /// Hazards (`schema:accessibilityHazard`: `none`, `flashing`).
    pub hazards: Vec<String>,
    /// Conformance claims (`dcterms:conformsTo`), in words: "EPUB
    /// Accessibility 1.1, WCAG 2.1 Level AA".
    pub conformance: Vec<String>,
    /// Who certified the claim (`a11y:certifiedBy`).
    pub certifier: Option<String>,
}

impl Accessibility {
    /// True when the book gives no accessibility metadata at all.
    pub fn is_empty(&self) -> bool {
        *self == Accessibility::default()
    }

    /// Whether the publisher says the text alone is enough to read it all
    /// (a sufficient access mode of `textual` only), so it suits a screen
    /// reader: `None` when no sufficient access mode is given.
    pub fn reads_as_text(&self) -> Option<bool> {
        if self.sufficient.is_empty() {
            return None;
        }
        Some(
            self.sufficient
                .iter()
                .any(|set| set.len() == 1 && set[0].eq_ignore_ascii_case("textual")),
        )
    }
}

/// The accessibility metadata an EPUB loader left on `meta`: `None` for a
/// document that is not an EPUB (or was loaded before this was read), and
/// an empty [`Accessibility`] for an EPUB that gives none.
pub fn accessibility(meta: &DocumentMeta) -> Option<Accessibility> {
    meta.properties
        .get(ACCESSIBILITY_PROPERTY)
        .and_then(|s| serde_json::from_str(s).ok())
}

/// A schema.org accessibility term in words: `tableOfContents` is "table
/// of contents", `textual` is "text", `visual` is "images", `auditory` is
/// "audio", and `MathML` stays as it is.
pub fn term_words(term: &str) -> String {
    match term {
        "textual" => return "text".into(),
        "visual" => return "images".into(),
        "auditory" => return "audio".into(),
        "tactile" => return "touch".into(),
        "MathML" | "ARIA" | "MathML-chemistry" => return term.into(),
        "ttsMarkup" => return "text-to-speech markup".into(),
        _ => {}
    }
    let mut out = String::with_capacity(term.len() + 4);
    let mut prev_lower = false;
    for c in term.chars() {
        if c.is_uppercase() && prev_lower {
            out.push(' ');
        }
        prev_lower = c.is_lowercase() || c.is_ascii_digit();
        out.extend(c.to_lowercase());
    }
    out
}

/// Whitespace collapsed, control characters dropped, cut at `max`
/// characters (ending "...").
fn clean_value(s: &str, max: usize) -> String {
    let joined = s
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(char::is_control, "");
    if joined.chars().count() <= max {
        return joined;
    }
    let mut cut: String = joined.chars().take(max).collect();
    cut.push_str("...");
    cut
}

/// A conformance claim in words: EPUB Accessibility 1.0's link form
/// (`http://www.idpf.org/epub/a11y/accessibility-20170105.html#wcag-aa`)
/// becomes "EPUB Accessibility 1.0, WCAG 2.0 Level AA"; 1.1's text
/// ("EPUB Accessibility 1.1 - WCAG 2.1 Level AA") reads its dash as a comma.
fn conformance_words(claim: &str) -> String {
    let claim = clean_value(claim, MAX_ACCESS_CHARS);
    let lower = claim.to_ascii_lowercase();
    if lower.contains("idpf.org/epub/a11y/accessibility") {
        let level = match lower.rsplit_once('#').map(|(_, f)| f) {
            Some("wcag-a") => ", WCAG 2.0 Level A",
            Some("wcag-aa") => ", WCAG 2.0 Level AA",
            Some("wcag-aaa") => ", WCAG 2.0 Level AAA",
            _ => "",
        };
        return format!("EPUB Accessibility 1.0{level}");
    }
    claim.replace(" - ", ", ")
}

/// Reads the accessibility metadata from the package document's
/// `<metadata>`.
fn read_accessibility(opf: &roxmltree::Document<'_>) -> Accessibility {
    let mut a = Accessibility::default();
    let Some(metadata) = opf
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "metadata")
    else {
        return a;
    };
    fn push(list: &mut Vec<String>, value: String) {
        if !value.is_empty() && list.len() < MAX_ACCESS_ITEMS && !list.contains(&value) {
            list.push(value);
        }
    }
    for n in metadata.descendants().filter(|n| n.is_element()) {
        let (property, value) = match n.tag_name().name() {
            "meta" => match (n.attribute("property"), n.attribute("name")) {
                (Some(p), _) => (p, text_of(n)),
                (None, Some(name)) => (name, n.attribute("content").unwrap_or("").to_owned()),
                (None, None) => continue,
            },
            // EPUB Accessibility 1.0 claims conformance with a link.
            "link" => {
                let rel = n.attribute("rel").unwrap_or("");
                if rel.split_whitespace().any(|r| r.ends_with("conformsTo"))
                    && let Some(href) = n.attribute("href")
                {
                    push(&mut a.conformance, conformance_words(href));
                }
                continue;
            }
            "conformsTo" => ("dcterms:conformsTo", text_of(n)),
            _ => continue,
        };
        let local = property.rsplit(':').next().unwrap_or(property);
        let terms = || {
            value
                .split(',')
                .map(|t| clean_value(t, MAX_TERM_CHARS))
                .filter(|t| !t.is_empty())
        };
        match local {
            "accessibilitySummary" if a.summary.is_none() => {
                a.summary = Some(clean_value(&value, MAX_ACCESS_CHARS)).filter(|s| !s.is_empty());
            }
            "accessMode" => terms().for_each(|t| push(&mut a.access_modes, t)),
            "accessModeSufficient" => {
                let set: Vec<String> = terms().take(MAX_ACCESS_ITEMS).collect();
                if !set.is_empty()
                    && a.sufficient.len() < MAX_ACCESS_ITEMS
                    && !a.sufficient.contains(&set)
                {
                    a.sufficient.push(set);
                }
            }
            "accessibilityFeature" => terms().for_each(|t| push(&mut a.features, t)),
            "accessibilityHazard" => terms().for_each(|t| push(&mut a.hazards, t)),
            "conformsTo" => push(&mut a.conformance, conformance_words(&value)),
            "certifiedBy" if a.certifier.is_none() => {
                a.certifier = Some(clean_value(&value, MAX_ACCESS_CHARS)).filter(|s| !s.is_empty());
            }
            _ => {}
        }
    }
    a
}

fn text_of(node: roxmltree::Node<'_, '_>) -> String {
    node.descendants()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn read_book(pkg: &mut Package) -> Result<Book, LoadError> {
    let container = pkg
        .read_text("META-INF/container.xml")?
        .ok_or_else(|| LoadError::Parse("not an EPUB: no META-INF/container.xml".into()))?;
    let container = parse_xml(&container)?;
    let opf_path = container
        .descendants()
        .find(|n| n.tag_name().name() == "rootfile")
        .and_then(|n| n.attribute("full-path"))
        .map(|p| resolve("", p).0)
        .ok_or_else(|| LoadError::Parse("EPUB container names no package document".into()))?;
    let opf_text = pkg
        .read_text(&opf_path)?
        .ok_or_else(|| LoadError::Parse(format!("EPUB package {opf_path} is missing")))?;
    let opf = parse_xml(&opf_text)?;
    let base = dir_of(&opf_path).to_owned();
    let mut book = Book {
        access: read_accessibility(&opf),
        ..Book::default()
    };

    let mut authors: Vec<String> = Vec::new();
    for n in opf.descendants().filter(|n| n.is_element()) {
        let t = text_of(n);
        if t.is_empty() {
            continue;
        }
        match n.tag_name().name() {
            "title" if book.title.is_none() => book.title = Some(t),
            "creator" => authors.push(t),
            "language" if book.language.is_none() => book.language = Some(t),
            _ => {}
        }
    }
    if !authors.is_empty() {
        book.author = Some(authors.join(", "));
    }

    // id -> (path, media type, properties)
    let mut manifest: HashMap<String, (String, String, String)> = HashMap::new();
    let mut nav_path = None;
    let mut ncx_path = None;
    for item in opf.descendants().filter(|n| n.tag_name().name() == "item") {
        let (Some(id), Some(href)) = (item.attribute("id"), item.attribute("href")) else {
            continue;
        };
        let path = resolve(&base, href).0;
        let media = item.attribute("media-type").unwrap_or("").to_owned();
        let props = item.attribute("properties").unwrap_or("").to_owned();
        if props.split_whitespace().any(|p| p == "nav") {
            nav_path = Some(path.clone());
        }
        if media == "application/x-dtbncx+xml" {
            ncx_path = Some(path.clone());
        }
        manifest.insert(id.to_owned(), (path, media, props));
    }
    let spine_node = opf.descendants().find(|n| n.tag_name().name() == "spine");
    if let Some(toc_id) = spine_node.and_then(|s| s.attribute("toc"))
        && let Some((path, _, _)) = manifest.get(toc_id)
    {
        ncx_path = Some(path.clone());
    }
    let mut seen = HashSet::new();
    for itemref in opf
        .descendants()
        .filter(|n| n.tag_name().name() == "itemref")
    {
        let Some((path, media, props)) = itemref.attribute("idref").and_then(|i| manifest.get(i))
        else {
            continue;
        };
        let html = matches!(media.as_str(), "application/xhtml+xml" | "text/html")
            || path.ends_with(".xhtml")
            || path.ends_with(".html")
            || path.ends_with(".htm");
        if !html || props.split_whitespace().any(|p| p == "nav") {
            continue;
        }
        if seen.insert(path.clone()) {
            book.spine.push(path.clone());
        }
    }
    refuse_drm(pkg, &book.spine)?;

    if let Some(nav) = nav_path
        && let Some(text) = pkg.read_text(&nav)?
    {
        book.toc = nav_toc(&text, dir_of(&nav));
        book.pages = nav_page_list(&text, dir_of(&nav));
    }
    if book.toc.is_empty()
        && let Some(ncx) = &ncx_path
        && let Some(text) = pkg.read_text(ncx)?
    {
        book.toc = ncx_toc(&text, dir_of(ncx))?;
    }
    if book.pages.is_empty()
        && let Some(ncx) = ncx_path
        && let Some(text) = pkg.read_text(&ncx)?
    {
        book.pages = ncx_page_list(&text, dir_of(&ncx))?;
    }
    Ok(book)
}

/// Refuses books whose content documents are encrypted.
fn refuse_drm(pkg: &mut Package, spine: &[String]) -> Result<(), LoadError> {
    let Some(text) = pkg.read_text("META-INF/encryption.xml")? else {
        return Ok(());
    };
    let Ok(xml) = parse_xml(&text) else {
        return Ok(());
    };
    let encrypted: HashSet<String> = xml
        .descendants()
        .filter(|n| n.tag_name().name() == "CipherReference")
        .filter_map(|n| n.attribute("URI"))
        .map(|u| resolve("", u).0)
        .collect();
    if spine.iter().any(|s| encrypted.contains(s)) {
        return Err(LoadError::Unsupported(
            "this EPUB is protected by DRM, so its text cannot be read".into(),
        ));
    }
    Ok(())
}

/// A table-of-contents depth as stored in [`TocEntry`] (levels past 255
/// share the last one).
fn toc_depth(depth: usize) -> u8 {
    u8::try_from(depth).unwrap_or(u8::MAX)
}

/// Entries of the EPUB 3 navigation document's table of contents.
fn nav_toc(text: &str, base: &str) -> Vec<TocEntry> {
    use scraper::{ElementRef, Html};
    let html = Html::parse_document(text);
    let navs: Vec<ElementRef<'_>> = html
        .root_element()
        .descendent_elements()
        .filter(|e| e.value().name() == "nav")
        .collect();
    let is_toc = |e: &ElementRef<'_>| {
        e.attr("epub:type")
            .is_some_and(|t| t.split_whitespace().any(|t| t == "toc"))
            || e.attr("role") == Some("doc-toc")
    };
    let Some(nav) = navs.iter().find(|e| is_toc(e)).or(navs.first()).copied() else {
        return Vec::new();
    };
    fn walk(list: ElementRef<'_>, depth: usize, base: &str, out: &mut Vec<TocEntry>) {
        // Entries nested past the limit are left out of the table of
        // contents; their chapters are still read.
        if depth > crate::MAX_NESTING {
            return;
        }
        for li in list.child_elements().filter(|e| e.value().name() == "li") {
            let label = li
                .child_elements()
                .find(|e| matches!(e.value().name(), "a" | "span"));
            if let Some(a) = label
                && let Some(href) = a.attr("href")
            {
                let title = a.text().collect::<String>();
                let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
                let (file, fragment) = resolve(base, href);
                out.push(TocEntry {
                    title,
                    file,
                    fragment,
                    depth: toc_depth(depth),
                });
            }
            for sub in li.child_elements().filter(|e| e.value().name() == "ol") {
                walk(sub, depth + 1, base, out);
            }
        }
    }
    let mut out = Vec::new();
    if let Some(ol) = nav.descendent_elements().find(|e| e.value().name() == "ol") {
        walk(ol, 1, base, &mut out);
    }
    out
}

/// The EPUB 3 navigation document's page list (`epub:type="page-list"`
/// or `role="doc-pagelist"`): links to where each print page starts.
fn nav_page_list(text: &str, base: &str) -> Vec<TocEntry> {
    use scraper::Html;
    let html = Html::parse_document(text);
    let Some(nav) = html.root_element().descendent_elements().find(|e| {
        e.value().name() == "nav"
            && (e
                .attr("epub:type")
                .is_some_and(|t| t.split_whitespace().any(|t| t == "page-list"))
                || e.attr("role") == Some("doc-pagelist"))
    }) else {
        return Vec::new();
    };
    nav.descendent_elements()
        .filter(|e| e.value().name() == "a")
        .filter_map(|a| {
            let (file, fragment) = resolve(base, a.attr("href")?);
            Some(TocEntry {
                title: crate::html::page_label(&a.text().collect::<String>()),
                file,
                fragment,
                depth: 1,
            })
        })
        .collect()
}

/// An EPUB 2 NCX `pageList`: its `pageTarget`s, in order.
fn ncx_page_list(text: &str, base: &str) -> Result<Vec<TocEntry>, LoadError> {
    let xml = parse_xml(text)?;
    Ok(xml
        .descendants()
        .filter(|n| n.tag_name().name() == "pageTarget")
        .filter_map(|t| {
            let src = child(t, "content")?.attribute("src")?;
            let (file, fragment) = resolve(base, src);
            Some(TocEntry {
                title: crate::html::page_label(&child(t, "navLabel").map(text_of)?),
                file,
                fragment,
                depth: 1,
            })
        })
        .collect())
}

/// Entries of an EPUB 2 NCX `navMap`.
fn ncx_toc(text: &str, base: &str) -> Result<Vec<TocEntry>, LoadError> {
    let xml = parse_xml(text)?;
    fn walk(node: roxmltree::Node<'_, '_>, depth: usize, base: &str, out: &mut Vec<TocEntry>) {
        if depth > crate::MAX_NESTING {
            return;
        }
        for point in node
            .children()
            .filter(|n| n.tag_name().name() == "navPoint")
        {
            let title = child(point, "navLabel").map(text_of).unwrap_or_default();
            if let Some(src) = child(point, "content").and_then(|c| c.attribute("src")) {
                let (file, fragment) = resolve(base, src);
                out.push(TocEntry {
                    title,
                    file,
                    fragment,
                    depth: toc_depth(depth),
                });
            }
            walk(point, depth + 1, base, out);
        }
    }
    let mut out = Vec::new();
    if let Some(map) = xml.descendants().find(|n| n.tag_name().name() == "navMap") {
        walk(map, 1, base, &mut out);
    }
    Ok(out)
}

/// The open `SectionBreak` markers, innermost last.
pub(crate) struct Sections {
    pub(crate) open: Vec<(u8, OpenId)>,
}

impl Sections {
    pub(crate) fn open(&mut self, b: &mut Builder, e: &TocEntry) {
        while let Some(&(depth, id)) = self.open.last() {
            if depth < e.depth {
                break;
            }
            b.close(id);
            self.open.pop();
        }
        let mut m = Marker::new(MarkerKind::SectionBreak, CharRange::empty(0)).with_level(e.depth);
        if !e.title.is_empty() {
            m = m.with_label(e.title.clone());
        }
        self.open.push((e.depth, b.open(m)));
    }
}

/// The spine's chapters as canonical text and markers; chapter warnings
/// (content nested too deeply) are added to `meta`.
fn convert(
    pkg: &mut Package,
    book: &Book,
    options: &LoadOptions,
    meta: &mut DocumentMeta,
) -> Result<(String, Vec<Marker>), LoadError> {
    let spine: HashSet<&str> = book.spine.iter().map(String::as_str).collect();
    let toc: Vec<TocEntry> = if book.toc.iter().any(|e| spine.contains(e.file.as_str())) {
        book.toc.clone()
    } else {
        // No usable table of contents: one section per spine item.
        book.spine
            .iter()
            .map(|f| TocEntry {
                title: String::new(),
                file: f.clone(),
                fragment: None,
                depth: 1,
            })
            .collect()
    };
    let mut b = Builder::new();
    let mut sections = Sections { open: Vec::new() };
    let mut pages = crate::html::Pages::default();
    for file in &book.spine {
        let Some(text) = pkg.read_text(file)? else {
            continue;
        };
        let ids = crate::html::anchor_ids(&text);
        let mut at_start: Vec<&TocEntry> = Vec::new();
        let mut anchored: HashMap<&str, Vec<&TocEntry>> = HashMap::new();
        for e in toc.iter().filter(|e| &e.file == file) {
            match e.fragment.as_deref() {
                Some(f) if ids.contains(f) => anchored.entry(f).or_default().push(e),
                _ => at_start.push(e),
            }
        }
        pages.by_id = book
            .pages
            .iter()
            .filter(|p| &p.file == file && !p.title.is_empty())
            .filter_map(|p| Some((p.fragment.clone()?, p.title.clone())))
            .collect();
        b.paragraph_break();
        for e in at_start {
            sections.open(&mut b, e);
        }
        let mut scratch = DocumentMeta::default();
        let mut hook = |id: &str, b: &mut Builder| {
            if let Some(es) = anchored.remove(id) {
                for e in es {
                    sections.open(b, e);
                }
            }
        };
        pages = crate::html::walk_epub_into(
            &mut b,
            &text,
            options,
            &mut scratch,
            Some(&mut hook),
            std::mem::take(&mut pages),
        );
        for w in crate::warnings(&scratch) {
            crate::add_warning(meta, &w);
        }
    }
    pages.close(&mut b);
    let (text, mut markers) = b.finish();
    label_sections_by_heading(&text, &mut markers);
    Ok((text, markers))
}

/// Gives unlabeled sections the text of their first heading.
///
/// The headings are read in one pass over `text`, in order of their
/// start, with a char cursor that only moves forward (a book with
/// thousands of headings once walked the text from its start for each).
pub(crate) fn label_sections_by_heading(text: &str, markers: &mut [Marker]) {
    let mut ranges: Vec<CharRange> = markers
        .iter()
        .filter(|m| m.kind == MarkerKind::Heading)
        .map(|m| m.range)
        .collect();
    ranges.sort_by_key(|r| r.start.0);
    // (char position, byte offset) of the cursor.
    let mut cursor = (0usize, 0usize);
    let headings: Vec<(CharRange, String)> = ranges
        .into_iter()
        .map(|r| {
            while cursor.0 < r.start.0 {
                let Some(c) = text[cursor.1..].chars().next() else {
                    break;
                };
                cursor = (cursor.0 + 1, cursor.1 + c.len_utf8());
            }
            let start = cursor.1;
            let end = text[start..]
                .char_indices()
                .nth(r.len())
                .map_or(text.len(), |(b, _)| start + b);
            let t = &text[start..end];
            (r, t.split_whitespace().collect::<Vec<_>>().join(" "))
        })
        .collect();
    for m in markers
        .iter_mut()
        .filter(|m| m.kind == MarkerKind::SectionBreak && m.label.is_none())
    {
        if let Some((_, h)) = headings
            .iter()
            .find(|(r, h)| m.range.contains_range(*r) && !h.is_empty())
        {
            m.label = Some(h.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_take_their_first_heading_in_one_pass() {
        // Non-ASCII before and inside the headings, headings out of order,
        // and one past the end of the text.
        let text = "Ça va. Über  alles\nmore\nZweite\nThird part\nend";
        let heading = |a, b| Marker::new(MarkerKind::Heading, CharRange::new(a, b));
        let section = |a, b| Marker::new(MarkerKind::SectionBreak, CharRange::new(a, b));
        let mut markers = vec![
            section(24, 46),
            heading(24, 30),
            section(0, 24),
            heading(7, 18),
            section(31, 46),
            heading(31, 41),
            section(46, 60),
            heading(50, 55),
        ];
        label_sections_by_heading(text, &mut markers);
        let labels: Vec<Option<&str>> = markers
            .iter()
            .filter(|m| m.kind == MarkerKind::SectionBreak)
            .map(|m| m.label.as_deref())
            .collect();
        assert_eq!(
            labels,
            [Some("Zweite"), Some("Über alles"), Some("Third part"), None]
        );
    }

    #[test]
    fn nav_toc_nests_and_resolves() {
        let nav = r#"<html xmlns:epub="http://www.idpf.org/2007/ops"><body>
            <nav epub:type="landmarks"><ol><li><a href="x.xhtml">Skip</a></li></ol></nav>
            <nav epub:type="toc"><ol>
              <li><a href="text/one.xhtml">One</a>
                <ol><li><a href="text/one.xhtml#s1">One point one</a></li></ol></li>
              <li><span>Two</span></li>
            </ol></nav></body></html>"#;
        let toc = nav_toc(nav, "OEBPS/");
        assert_eq!(toc.len(), 2);
        assert_eq!(toc[0].file, "OEBPS/text/one.xhtml");
        assert_eq!(toc[1].fragment.as_deref(), Some("s1"));
        assert_eq!((toc[0].depth, toc[1].depth), (1, 2));
        assert_eq!(toc[1].title, "One point one");
    }

    #[test]
    fn ncx_toc_reads_nav_points() {
        let ncx = r#"<?xml version="1.0"?>
<!DOCTYPE ncx PUBLIC "-//NISO//DTD ncx 2005-1//EN" "http://www.daisy.org/z3986/2005/ncx-2005-1.dtd">
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/"><navMap>
  <navPoint id="a"><navLabel><text>Chapter  1</text></navLabel><content src="c1.html"/>
    <navPoint id="b"><navLabel><text>Part</text></navLabel><content src="c1.html#p"/></navPoint>
  </navPoint>
</navMap></ncx>"#;
        let toc = ncx_toc(ncx, "").unwrap();
        assert_eq!(toc.len(), 2);
        assert_eq!(toc[0].title, "Chapter 1");
        assert_eq!(toc[1].depth, 2);
    }
}
