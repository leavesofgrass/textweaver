//! EPUB loader (EPUB 2 and 3): the spine's XHTML documents in reading order,
//! through the HTML loader's rules, with the table of contents as chapters.
//!
//! - `META-INF/container.xml` names the package document (OPF); its
//!   `<metadata>` gives the title (`dc:title`), author (`dc:creator`, all of
//!   them joined), and language (`dc:language`).
//! - The spine's HTML and XHTML items are read in order, each starting a new
//!   paragraph, with the same skip list, image alt text, and structure as
//!   [`HtmlLoader`](crate::HtmlLoader) (the EPUB 3 navigation document is not
//!   read as content). Unlike Star, chapters are not joined with `---` rules
//!   and no title heading is invented: the title is metadata.
//! - The table of contents (the EPUB 3 `nav` with `epub:type="toc"`, else
//!   the EPUB 2 NCX) becomes `SectionBreak` markers: one per entry, labeled
//!   with the entry's title, `level` = nesting depth (1 for top-level
//!   entries), starting where the entry points (a spine item, or an element
//!   id inside one) and ending where the next entry of the same or a
//!   shallower depth starts. So "next chapter" follows the book's own table
//!   of contents, and nested entries nest. Without a table of contents,
//!   every spine item is a level-1 section labeled by its first heading.
//! - A book whose content documents are encrypted (DRM) is refused with a
//!   clear error rather than read as noise; obfuscated fonts are fine.

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
    let mut book = Book::default();

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
    }
    if book.toc.is_empty()
        && let Some(ncx) = ncx_path
        && let Some(text) = pkg.read_text(&ncx)?
    {
        book.toc = ncx_toc(&text, dir_of(&ncx))?;
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
        crate::html::walk_into(&mut b, &text, options, &mut scratch, Some(&mut hook));
        for w in crate::warnings(&scratch) {
            crate::add_warning(meta, &w);
        }
    }
    let (text, mut markers) = b.finish();
    label_sections_by_heading(&text, &mut markers);
    Ok((text, markers))
}

/// Gives unlabeled sections the text of their first heading.
pub(crate) fn label_sections_by_heading(text: &str, markers: &mut [Marker]) {
    let headings: Vec<(CharRange, String)> = markers
        .iter()
        .filter(|m| m.kind == MarkerKind::Heading)
        .map(|m| {
            let t: String = text
                .chars()
                .skip(m.range.start.0)
                .take(m.range.len())
                .collect();
            (m.range, t.split_whitespace().collect::<Vec<_>>().join(" "))
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
