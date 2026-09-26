//! DAISY 3 books and DTBook files (Bookshare and the national libraries
//! for print-disabled readers).
//!
//! - **A DAISY 3 book** is a package file (`.opf`), a navigation file
//!   (NCX), SMIL files that synchronize text and audio, and the text itself
//!   in DTBook XML. Open the package file, or the zip it came in (see
//!   [`ArchiveLoader`](crate::ArchiveLoader)). The DTBook files are read in
//!   spine order (the order the SMIL files point into them), and the NCX's
//!   entries become `SectionBreak` markers (label = the entry's title,
//!   level = its depth), so "next chapter" follows the book's own table of
//!   contents. Title, author, and language come from the package's Dublin
//!   Core metadata.
//! - **A DTBook file** (`.xml` whose root is `dtbook`) is read on its own;
//!   its `level1`–`level6` elements become the sections. Other `.xml` files
//!   read as plain text.
//!
//! DTBook's structure maps onto the HTML loader's rules: headings (`h1`–
//! `h6`, and `hd` at its level's depth), paragraphs, lists (`list
//! type="ol"` numbered), tables, images with their alt text, image groups
//! with captions, producer's notes, sidebars, notes, poems (a line per
//! `line`), emphasis, and code. Print page numbers (`pagenum`) are not read
//! aloud: each starts a `PageBreak` marker labeled with the printed number,
//! whose range is that page's text.

use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;

use ropey::Rope;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, DocumentMeta, Marker};

use crate::builder::{Builder, OpenId};
use crate::epub::{Sections, TocEntry, label_sections_by_heading};
use crate::package::{dir_of, parse_xml, resolve};
use crate::{LoadError, LoadOptions, Loader, Source, meta_for, title_from_path};

/// Where a page number starts, in the HTML the DTBook becomes.
const PAGE_ID: &str = "textweaver-pagenum-";

/// Most files one book may read (SMIL and DTBook together).
const MAX_FILES: usize = 20_000;

/// Loads DAISY 3 package files and DTBook XML.
#[derive(Clone, Copy, Debug, Default)]
pub struct DaisyLoader;

impl Loader for DaisyLoader {
    fn id(&self) -> &'static str {
        "daisy"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["opf", "xml", "dtbook"]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let bytes = source.read()?;
        if is_daisy_package(&bytes) {
            let name = match source {
                Source::Path(p) => p
                    .to_string_lossy()
                    .rsplit(['/', '\\'])
                    .next()
                    .unwrap_or("")
                    .to_owned(),
                _ => String::from("package.opf"),
            };
            let folder = match source {
                Source::Path(p) => p.parent().map(Path::to_owned),
                _ => None,
            };
            let mut read = |rel: &str| -> Result<Option<Vec<u8>>, LoadError> {
                let Some(folder) = &folder else {
                    return Ok(None);
                };
                // `rel` was resolved against the package's folder, so `..`
                // cannot leave it.
                let path = folder.join(rel);
                match crate::archive::read_path(&path) {
                    Ok(b) => Ok(Some(b)),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                    Err(e) => Err(LoadError::Io(path, e)),
                }
            };
            let mut doc = load_package(&name, &bytes, &mut read, options)?;
            if let Source::Path(p) = source {
                doc.meta.path = Some(p.clone());
            }
            if doc.meta.title.is_none() {
                doc.meta.title = title_from_path(source);
            }
            return Ok(doc);
        }
        if is_dtbook(&bytes) {
            let mut meta = meta_for(source, self.id());
            let text = decode_xml(&bytes, &mut meta);
            let mut b = Builder::new();
            let mut sections = Sections { open: Vec::new() };
            let book = read_dtbook(
                &mut b,
                &mut sections,
                &text,
                "",
                &[],
                true,
                options,
                &mut meta,
            )?;
            fill_meta(&mut meta, &book);
            if meta.title.is_none() {
                meta.title = title_from_path(source);
            }
            let (text, mut markers) = b.finish();
            label_sections_by_heading(&text, &mut markers);
            return Ok(Document::new(meta, Rope::from_str(&text), markers));
        }
        // Some other XML (or a package that is not a DAISY book): text.
        crate::TextLoader.load(source, options)
    }
}

/// True for a DAISY 3 package file: an OPF whose manifest holds DTBook.
pub fn is_daisy_package(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(256 * 1024)];
    let s = String::from_utf8_lossy(head);
    s.contains("<package") && s.contains("application/x-dtbook+xml")
}

/// True for DTBook XML (its root element is `dtbook`).
fn is_dtbook(bytes: &[u8]) -> bool {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(8 * 1024)]);
    head.contains("<dtbook")
}

/// XML bytes as text, flattened past the nesting limit (with the warning).
fn decode_xml(bytes: &[u8], meta: &mut DocumentMeta) -> String {
    let declared = crate::encoding::sniff_html_charset(bytes);
    let text = crate::decode_bytes(bytes, declared.as_deref()).text;
    match crate::xmldepth::limit_depth(&text, crate::MAX_NESTING) {
        Some(flat) => {
            crate::add_warning(meta, crate::NESTING_WARNING);
            flat
        }
        None => text,
    }
}

/// Title, author, and language from a DTBook's head or a package.
#[derive(Debug, Default)]
struct BookMeta {
    title: Option<String>,
    author: Option<String>,
    language: Option<String>,
}

fn fill_meta(meta: &mut DocumentMeta, book: &BookMeta) {
    if meta.title.is_none() {
        meta.title.clone_from(&book.title);
    }
    if meta.author.is_none() {
        meta.author.clone_from(&book.author);
    }
    if meta.language.is_none() {
        meta.language.clone_from(&book.language);
    }
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn text_of(node: roxmltree::Node<'_, '_>) -> String {
    collapse(
        &node
            .descendants()
            .filter(|n| n.is_text())
            .filter_map(|n| n.text())
            .collect::<String>(),
    )
}

/// Loads a DAISY 3 book from its package file. `opf_path` is the package's
/// path among the book's files, and `read` reads another of them by path
/// (relative to the same root), returning `None` when it is missing.
pub(crate) fn load_package(
    opf_path: &str,
    opf_bytes: &[u8],
    read: &mut dyn FnMut(&str) -> Result<Option<Vec<u8>>, LoadError>,
    options: &LoadOptions,
) -> Result<Document, LoadError> {
    let mut meta = DocumentMeta {
        format: "daisy".into(),
        ..DocumentMeta::default()
    };
    let opf_text = decode_xml(opf_bytes, &mut meta);
    let opf = parse_xml(&opf_text)?;
    let base = dir_of(opf_path).to_owned();

    let mut book = BookMeta::default();
    let mut authors = Vec::new();
    for n in opf.descendants().filter(|n| n.is_element()) {
        let t = text_of(n);
        if t.is_empty() {
            continue;
        }
        match n.tag_name().name().to_ascii_lowercase().as_str() {
            "title" if book.title.is_none() => book.title = Some(t),
            "creator" => authors.push(t),
            "language" if book.language.is_none() => book.language = Some(t),
            _ => {}
        }
    }
    if !authors.is_empty() {
        book.author = Some(authors.join(", "));
    }

    let mut manifest: HashMap<String, (String, String)> = HashMap::new();
    let mut dtbooks = Vec::new();
    let mut ncx = None;
    for item in opf.descendants().filter(|n| n.tag_name().name() == "item") {
        let (Some(id), Some(href)) = (item.attribute("id"), item.attribute("href")) else {
            continue;
        };
        let path = resolve(&base, href).0;
        let media = item.attribute("media-type").unwrap_or("").to_owned();
        match media.as_str() {
            "application/x-dtbook+xml" => dtbooks.push(path.clone()),
            "application/x-dtbncx+xml" => ncx = Some(path.clone()),
            _ => {}
        }
        manifest.insert(id.to_owned(), (path, media));
    }
    let smils: Vec<String> = opf
        .descendants()
        .filter(|n| n.tag_name().name() == "itemref")
        .filter_map(|n| n.attribute("idref").and_then(|i| manifest.get(i)))
        .filter(|(p, m)| m == "application/smil" || p.to_ascii_lowercase().ends_with(".smil"))
        .map(|(p, _)| p.clone())
        .take(MAX_FILES)
        .collect();

    // Spine order: the DTBook files in the order the SMIL files point
    // into them; each SMIL's ids map to the DTBook element they show.
    let mut order: Vec<String> = Vec::new();
    let mut smil_targets: HashMap<String, HashMap<String, (String, Option<String>)>> =
        HashMap::new();
    for smil in &smils {
        let Some(bytes) = read(smil)? else { continue };
        let text = decode_xml(&bytes, &mut meta);
        let Ok(xml) = parse_xml(&text) else { continue };
        let dir = dir_of(smil).to_owned();
        let mut ids = HashMap::new();
        for el in xml.descendants().filter(|n| n.is_element()) {
            let first_text = std::iter::once(el)
                .chain(el.descendants())
                .find(|n| n.tag_name().name() == "text")
                .and_then(|t| t.attribute("src"));
            let Some(src) = first_text else { continue };
            let target = resolve(&dir, src);
            if el.tag_name().name() == "text" && !order.contains(&target.0) {
                order.push(target.0.clone());
            }
            if let Some(id) = el.attribute("id") {
                ids.entry(id.to_owned()).or_insert(target);
            }
        }
        smil_targets.insert(smil.clone(), ids);
    }
    for d in &dtbooks {
        if !order.contains(d) {
            order.push(d.clone());
        }
    }
    order.retain(|d| dtbooks.contains(d) || d.to_ascii_lowercase().ends_with(".xml"));

    // The NCX's entries, pointed through the SMIL files at DTBook ids.
    let mut toc: Vec<TocEntry> = Vec::new();
    if let Some(ncx) = &ncx
        && let Some(bytes) = read(ncx)?
    {
        let text = decode_xml(&bytes, &mut meta);
        if let Ok(xml) = parse_xml(&text) {
            let dir = dir_of(ncx).to_owned();
            if let Some(map) = xml.descendants().find(|n| n.tag_name().name() == "navMap") {
                ncx_entries(map, 1, &dir, &smil_targets, &mut toc);
            }
        }
    }

    let mut b = Builder::new();
    let mut sections = Sections { open: Vec::new() };
    let mut first = true;
    for file in &order {
        let Some(bytes) = read(file)? else { continue };
        let text = decode_xml(&bytes, &mut meta);
        let entries: Vec<TocEntry> = toc.iter().filter(|e| &e.file == file).cloned().collect();
        b.paragraph_break();
        let own = read_dtbook(
            &mut b,
            &mut sections,
            &text,
            file,
            &entries,
            toc.is_empty(),
            options,
            &mut meta,
        )?;
        if first {
            // Package metadata wins; the DTBook's head fills gaps.
            fill_meta(&mut meta, &book);
            fill_meta(&mut meta, &own);
            first = false;
        }
    }
    fill_meta(&mut meta, &book);
    let (text, mut markers) = b.finish();
    label_sections_by_heading(&text, &mut markers);
    Ok(Document::new(meta, Rope::from_str(&text), markers))
}

/// Table-of-contents entries from an NCX `navMap`, each pointed at the
/// DTBook element its SMIL target shows.
fn ncx_entries(
    node: roxmltree::Node<'_, '_>,
    depth: usize,
    dir: &str,
    smil: &HashMap<String, HashMap<String, (String, Option<String>)>>,
    out: &mut Vec<TocEntry>,
) {
    if depth > crate::MAX_NESTING {
        return;
    }
    for point in node
        .children()
        .filter(|n| n.tag_name().name() == "navPoint")
    {
        let title = point
            .children()
            .find(|n| n.tag_name().name() == "navLabel")
            .map(text_of)
            .unwrap_or_default();
        if let Some(src) = point
            .children()
            .find(|n| n.tag_name().name() == "content")
            .and_then(|c| c.attribute("src"))
        {
            let (file, frag) = resolve(dir, src);
            let target = frag
                .as_ref()
                .and_then(|f| smil.get(&file).and_then(|ids| ids.get(f)))
                .cloned()
                .or_else(|| {
                    // A navPoint straight at the DTBook, or at a SMIL's start.
                    if file.to_ascii_lowercase().ends_with(".xml") {
                        Some((file.clone(), frag.clone()))
                    } else {
                        smil.get(&file)
                            .and_then(|ids| ids.values().next())
                            .map(|(f, _)| (f.clone(), None))
                    }
                });
            if let Some((file, fragment)) = target {
                out.push(TocEntry {
                    title,
                    file,
                    fragment,
                    depth: u8::try_from(depth).unwrap_or(u8::MAX),
                });
            }
        }
        ncx_entries(point, depth + 1, dir, smil, out);
    }
}

/// Reads one DTBook file into `b`: as HTML through the HTML loader's rules,
/// with `toc` entries (or, when `own_levels`, the DTBook's own levels)
/// opening sections (which stay open into the next file) and page numbers
/// opening pages.
#[allow(clippy::too_many_arguments)]
fn read_dtbook(
    b: &mut Builder,
    sections: &mut Sections,
    text: &str,
    file: &str,
    toc: &[TocEntry],
    own_levels: bool,
    options: &LoadOptions,
    meta: &mut DocumentMeta,
) -> Result<BookMeta, LoadError> {
    let xml = parse_xml(text)?;
    let mut book = BookMeta::default();
    let root = xml.root_element();
    book.language = root
        .attribute(("http://www.w3.org/XML/1998/namespace", "lang"))
        .or_else(|| root.attribute("lang"))
        .map(str::to_owned);
    for m in root.descendants().filter(|n| n.tag_name().name() == "meta") {
        let (Some(name), Some(content)) = (m.attribute("name"), m.attribute("content")) else {
            continue;
        };
        let content = collapse(content);
        match name.to_ascii_lowercase().as_str() {
            "dc:title" if book.title.is_none() => book.title = Some(content),
            "dc:creator" if book.author.is_none() => book.author = Some(content),
            "dc:language" if book.language.is_none() => book.language = Some(content),
            _ => {}
        }
    }
    if book.title.is_none() {
        book.title = root
            .descendants()
            .find(|n| n.tag_name().name() == "doctitle")
            .map(text_of)
            .filter(|t| !t.is_empty());
    }
    let mut html = Html {
        out: String::with_capacity(text.len()),
        pages: Vec::new(),
        levels: Vec::new(),
        depth: 0,
        next_id: 0,
    };
    html.out.push_str("<html><body>");
    for child in root.children() {
        html.node(child, false);
    }
    html.out.push_str("</body></html>");

    let mut entries: Vec<TocEntry> = toc.to_vec();
    if own_levels {
        entries = html
            .levels
            .iter()
            .map(|(id, title, depth)| TocEntry {
                title: title.clone(),
                file: file.to_owned(),
                fragment: Some(id.clone()),
                depth: *depth,
            })
            .collect();
    }
    let ids = crate::html::anchor_ids(&html.out);
    let mut at_start = Vec::new();
    let mut anchored: HashMap<String, Vec<TocEntry>> = HashMap::new();
    for e in entries {
        match e.fragment.clone() {
            Some(f) if ids.contains(&f) => anchored.entry(f).or_default().push(e),
            _ => at_start.push(e),
        }
    }
    for e in &at_start {
        sections.open(b, e);
    }
    let pages = std::mem::take(&mut html.pages);
    let mut page: Option<OpenId> = None;
    let mut scratch = DocumentMeta::default();
    {
        let mut hook = |id: &str, b: &mut Builder| {
            if let Some(n) = id
                .strip_prefix(PAGE_ID)
                .and_then(|n| n.parse::<usize>().ok())
            {
                if let Some(open) = page.take() {
                    b.close(open);
                }
                let mut m = Marker::new(MarkerKind::PageBreak, CharRange::empty(0));
                if let Some(label) = pages.get(n).filter(|l| !l.is_empty()) {
                    m = m.with_label(label.clone());
                }
                page = Some(b.open(m));
                return;
            }
            if let Some(es) = anchored.remove(id) {
                for e in &es {
                    sections.open(b, e);
                }
            }
        };
        crate::html::walk_into(b, &html.out, options, &mut scratch, Some(&mut hook));
    }
    if let Some(open) = page.take() {
        b.close(open);
    }
    for w in crate::warnings(&scratch) {
        crate::add_warning(meta, &w);
    }
    Ok(book)
}

/// DTBook written out as HTML.
struct Html {
    out: String,
    /// Printed page numbers, by index.
    pages: Vec<String>,
    /// Levels found: id, title (its first heading), depth.
    levels: Vec<(String, String, u8)>,
    /// Depth of the enclosing `level` elements.
    depth: u8,
    next_id: usize,
}

fn escape(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
}

impl Html {
    fn open(&mut self, tag: &str, node: roxmltree::Node<'_, '_>, extra: &[(&str, &str)]) {
        self.out.push('<');
        self.out.push_str(tag);
        let mut attrs: Vec<(&str, &str)> = extra.to_vec();
        if let Some(id) = node.attribute("id") {
            attrs.push(("id", id));
        }
        for (k, v) in attrs {
            let _ = write!(self.out, " {k}=\"");
            escape(v, &mut self.out);
            self.out.push('"');
        }
        self.out.push('>');
    }

    fn children(&mut self, node: roxmltree::Node<'_, '_>, in_table: bool) {
        for c in node.children() {
            self.node(c, in_table);
        }
    }

    fn wrap(&mut self, tag: &str, node: roxmltree::Node<'_, '_>, in_table: bool) {
        self.open(tag, node, &[]);
        self.children(node, in_table);
        let _ = write!(self.out, "</{tag}>");
    }

    fn node(&mut self, node: roxmltree::Node<'_, '_>, in_table: bool) {
        if node.is_text() {
            escape(node.text().unwrap_or(""), &mut self.out);
            return;
        }
        if !node.is_element() {
            return;
        }
        let name = node.tag_name().name();
        match name {
            "head" | "col" | "colgroup" => {}
            "pagenum" => {
                let n = self.pages.len();
                self.pages.push(text_of(node));
                let _ = write!(self.out, "<span id=\"{PAGE_ID}{n}\"></span>");
            }
            "level" | "level1" | "level2" | "level3" | "level4" | "level5" | "level6" => {
                let depth = match name
                    .strip_prefix("level")
                    .and_then(|d| d.parse::<u8>().ok())
                {
                    Some(d) => d,
                    None => self.depth.saturating_add(1),
                };
                let id = node.attribute("id").map_or_else(
                    || {
                        self.next_id += 1;
                        format!("textweaver-level-{}", self.next_id)
                    },
                    str::to_owned,
                );
                let title = node
                    .children()
                    .find(|c| {
                        matches!(
                            c.tag_name().name(),
                            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "hd"
                        )
                    })
                    .map(text_of)
                    .unwrap_or_default();
                self.levels.push((id.clone(), title, depth.max(1)));
                let _ = write!(self.out, "<section id=\"");
                escape(&id, &mut self.out);
                self.out.push_str("\">");
                let saved = self.depth;
                self.depth = depth;
                self.children(node, in_table);
                self.depth = saved;
                self.out.push_str("</section>");
            }
            "hd" => {
                let level = self.depth.clamp(1, 6);
                self.wrap(&format!("h{level}"), node, in_table);
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "p" | "em" | "strong" | "sub" | "sup"
            | "code" | "kbd" | "samp" | "cite" | "q" | "blockquote" | "div" | "address" | "dl"
            | "dt" | "dd" | "li" | "tr" | "th" | "td" | "thead" | "tbody" | "tfoot" | "abbr"
            | "dfn" | "span" => self.wrap(name, node, in_table),
            "table" => self.wrap("table", node, true),
            "caption" if in_table => self.wrap("caption", node, in_table),
            "caption" => self.wrap("figcaption", node, in_table),
            "acronym" => self.wrap("abbr", node, in_table),
            "br" => self.out.push_str("<br>"),
            "list" => {
                let tag = if node.attribute("type") == Some("ol") {
                    "ol"
                } else {
                    "ul"
                };
                let start = node.attribute("start").unwrap_or("");
                if start.is_empty() {
                    self.open(tag, node, &[]);
                } else {
                    self.open(tag, node, &[("start", start)]);
                }
                self.children(node, in_table);
                let _ = write!(self.out, "</{tag}>");
            }
            "a" => {
                let href = node.attribute("href").unwrap_or("");
                self.open("a", node, &[("href", href)]);
                self.children(node, in_table);
                self.out.push_str("</a>");
            }
            "img" => {
                let alt = node.attribute("alt").unwrap_or("");
                let src = node.attribute("src").unwrap_or("");
                self.open("img", node, &[("alt", alt), ("src", src)]);
            }
            "imggroup" => self.wrap("figure", node, in_table),
            "noteref" | "annoref" => self.wrap("sup", node, in_table),
            "line" => {
                self.wrap("span", node, in_table);
                self.out.push_str("<br>");
            }
            "doctitle" | "docauthor" | "covertitle" | "byline" | "dateline" | "bridgehead" => {
                self.wrap("p", node, in_table)
            }
            "frontmatter" | "bodymatter" | "rearmatter" | "book" | "prodnote" | "sidebar"
            | "note" | "annotation" | "epigraph" | "poem" | "linegroup" => {
                self.wrap("div", node, in_table)
            }
            _ => self.wrap("span", node, in_table),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DTBOOK: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<dtbook xmlns="http://www.daisy.org/z3986/2005/dtbook/" version="2005-3" xml:lang="en-US">
<head><meta name="dc:Title" content="A Small Book"/><meta name="dc:Creator" content="A. Writer"/></head>
<book>
 <frontmatter><doctitle>A Small Book</doctitle><docauthor>A. Writer</docauthor></frontmatter>
 <bodymatter>
  <level1 id="ch1"><pagenum id="p1" page="normal">1</pagenum><h1>Chapter One</h1>
   <p>The first paragraph.</p>
   <list type="ol"><li>alpha</li><li>beta</li></list>
   <level2 id="s1"><h2>Part A</h2><p>Nested text.</p>
    <pagenum id="p2">2</pagenum>
    <imggroup><img src="a.jpg" alt="A map of the town"/><caption>The town</caption></imggroup>
   </level2>
  </level1>
  <level1 id="ch2"><h1>Chapter Two</h1><poem><linegroup><line>Roses</line><line>Violets</line></linegroup></poem>
   <table><caption>Scores</caption><tr><th>Name</th><th>Score</th></tr><tr><td>Ann</td><td>9</td></tr></table>
  </level1>
 </bodymatter>
</book></dtbook>"#;

    fn load(data: &str, hint: &str) -> Document {
        DaisyLoader
            .load(
                &Source::Bytes {
                    data: data.as_bytes().to_vec(),
                    hint: hint.into(),
                },
                &LoadOptions::default(),
            )
            .unwrap()
    }

    fn texts(d: &Document, kind: MarkerKind) -> Vec<(String, u8, Option<String>)> {
        d.marker_index()
            .iter(kind, None)
            .map(|m| (d.slice(m.range).to_string(), m.level, m.label.clone()))
            .collect()
    }

    #[test]
    fn dtbook_structure_pages_and_sections() {
        let d = load(DTBOOK, "xml");
        assert_eq!(d.meta.format, "daisy");
        assert_eq!(d.meta.title.as_deref(), Some("A Small Book"));
        assert_eq!(d.meta.author.as_deref(), Some("A. Writer"));
        assert_eq!(d.meta.language.as_deref(), Some("en-US"));
        let text = d.text().to_string();
        assert!(!text.contains("\n1\n"), "page numbers are not read: {text}");
        let headings = texts(&d, MarkerKind::Heading);
        assert_eq!(headings[0], ("Chapter One".into(), 1, None));
        assert_eq!(headings[1], ("Part A".into(), 2, None));
        let pages = texts(&d, MarkerKind::PageBreak);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].2.as_deref(), Some("1"));
        assert!(pages[0].0.starts_with("Chapter One"));
        assert!(pages[1].0.starts_with("A map of the town"));
        let sections = texts(&d, MarkerKind::SectionBreak);
        let labels: Vec<_> = sections
            .iter()
            .map(|s| (s.2.clone().unwrap_or_default(), s.1))
            .collect();
        assert_eq!(
            labels,
            [
                ("Chapter One".to_owned(), 1),
                ("Part A".to_owned(), 2),
                ("Chapter Two".to_owned(), 1)
            ]
        );
        assert!(text.contains("Roses\nViolets"), "{text}");
        let items = texts(&d, MarkerKind::ListItem);
        assert_eq!(items[0].2.as_deref(), Some("1."));
        assert!(text.contains("Name | Score"));
        let images = texts(&d, MarkerKind::Image);
        assert_eq!(images[0].0, "A map of the town");
    }

    #[test]
    fn a_daisy_zip_opens_as_the_book() {
        use std::io::Write;
        let opf = r#"<package><manifest><item id="d" href="book.xml" media-type="application/x-dtbook+xml"/></manifest><spine/></package>"#;
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, body) in [("Book/package.opf", opf), ("Book/book.xml", DTBOOK)] {
            z.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            z.write_all(body.as_bytes()).unwrap();
        }
        let data = z.finish().unwrap().into_inner();
        let d = crate::ArchiveLoader
            .load(
                &Source::Bytes {
                    data,
                    hint: "zip".into(),
                },
                &LoadOptions::default(),
            )
            .unwrap();
        assert_eq!(d.meta.format, "daisy");
        assert_eq!(d.meta.title.as_deref(), Some("A Small Book"));
        assert!(d.text().to_string().contains("The first paragraph."));
    }

    #[test]
    fn other_xml_reads_as_text() {
        let d = load("<notes><n>hello</n></notes>", "xml");
        assert_eq!(d.meta.format, "text");
        assert_eq!(d.text().to_string(), "<notes><n>hello</n></notes>");
    }

    #[test]
    fn packages_follow_the_spine_and_the_ncx() {
        let opf = r#"<?xml version="1.0"?><package xmlns="http://openebook.org/namespaces/oeb-package/1.0/">
<metadata><dc-metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:Title>Package Title</dc:Title><dc:Creator>P. Author</dc:Creator><dc:Language>en</dc:Language></dc-metadata></metadata>
<manifest>
 <item id="ncx" href="nav.ncx" media-type="application/x-dtbncx+xml"/>
 <item id="s1" href="one.smil" media-type="application/smil"/>
 <item id="s2" href="two.smil" media-type="application/smil"/>
 <item id="a" href="b.xml" media-type="application/x-dtbook+xml"/>
 <item id="b" href="a.xml" media-type="application/x-dtbook+xml"/>
</manifest>
<spine><itemref idref="s1"/><itemref idref="s2"/></spine></package>"#;
        // The spine reads a.xml first, although the manifest lists b.xml first.
        let smil1 = r#"<smil><body><seq><par id="par1"><text src="a.xml#first"/></par></seq></body></smil>"#;
        let smil2 = r#"<smil><body><seq><par id="par2"><text src="b.xml#second"/></par></seq></body></smil>"#;
        let ncx = r#"<ncx><navMap>
<navPoint id="n1"><navLabel><text>Start</text></navLabel><content src="one.smil#par1"/>
 <navPoint id="n2"><navLabel><text>Later</text></navLabel><content src="two.smil#par2"/></navPoint>
</navPoint></navMap></ncx>"#;
        let a = r#"<dtbook><book><bodymatter><level1><p id="first">From A.</p></level1></bodymatter></book></dtbook>"#;
        let b = r#"<dtbook><book><bodymatter><level1><p>Before.</p><p id="second">From B.</p></level1></bodymatter></book></dtbook>"#;
        let files: HashMap<&str, &str> = [
            ("book/nav.ncx", ncx),
            ("book/one.smil", smil1),
            ("book/two.smil", smil2),
            ("book/a.xml", a),
            ("book/b.xml", b),
        ]
        .into_iter()
        .collect();
        let mut read = |p: &str| Ok(files.get(p).map(|s| s.as_bytes().to_vec()));
        let d = load_package(
            "book/package.opf",
            opf.as_bytes(),
            &mut read,
            &LoadOptions::default(),
        )
        .unwrap();
        assert_eq!(d.text().to_string(), "From A.\n\nBefore.\n\nFrom B.");
        assert_eq!(d.meta.title.as_deref(), Some("Package Title"));
        assert_eq!(d.meta.author.as_deref(), Some("P. Author"));
        let sections = texts(&d, MarkerKind::SectionBreak);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].2.as_deref(), Some("Start"));
        assert_eq!(sections[1], ("From B.".into(), 2, Some("Later".into())));
    }
}
