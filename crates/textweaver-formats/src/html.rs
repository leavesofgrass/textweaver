//! HTML loader: parsed with scraper (html5ever) into canonical text and
//! markers.
//!
//! Star's rules kept: its skip list (`script`, `style`, `nav`, `footer`,
//! `aside`, `noscript`, `svg`, `canvas`, `meta`, `link`, `base`, `iframe`,
//! `template`, `button`, `form`) and its image text (`alt`, then `title`,
//! then `aria-label`; an empty result is decorative and produces nothing; a
//! `longdesc` URL is appended as "(long description: URL)").
//!
//! Star's bugs fixed (the Star parity reference, Part 1 §1.4 and §7):
//!
//! - void elements such as an unclosed `<meta charset>` no longer swallow the
//!   rest of the document (html5ever knows they have no content);
//! - `<title>` becomes the document title, not a second copy of the `<h1>`;
//! - a nested list keeps its parent item on its own line;
//! - `<caption>` is kept: it labels the `Table` marker and is a paragraph of
//!   its own before the table;
//! - an image followed by a block element no longer merges paragraphs;
//! - `<br>` breaks the line in the canonical text.
//!
//! Also skipped: elements with the `hidden` attribute or `aria-hidden="true"`,
//! and images with `role="presentation"`.
//!
//! An inline `<svg>` drawing is not dropped with the rest of the skip list:
//! it is one graphic named by its `aria-label` or its `<title>` (ADR-0044).
//!
//! MathML (`<math>`) is read as math, in web pages as in EPUB 3 (W5c3,
//! ADR-0035): LaTeX with its delimiters under a `Math` marker, as
//! `mathml.rs` describes.
//!
//! The charset comes from a byte order mark, a `<meta charset>` (or
//! `http-equiv` content type, or XML declaration) in the first 1024 bytes,
//! else UTF-8 when valid, else Windows-1252 (see [`crate::encoding`]).

use ropey::Rope;
use scraper::{ElementRef, Html, Node};
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, DocumentMeta, HEADER_ROW_LABEL, Marker};

use crate::builder::Builder;
use crate::{
    LoadError, LoadOptions, Loader, Source, decode_bytes, encoding, meta_for, note_encoding,
    title_from_path,
};

/// Loads HTML and XHTML.
#[derive(Clone, Copy, Debug, Default)]
pub struct HtmlLoader;

impl Loader for HtmlLoader {
    fn id(&self) -> &'static str {
        "html"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["html", "htm", "xhtml", "xht"]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let bytes = source.read()?;
        let declared = encoding::sniff_html_charset(&bytes);
        let decoded = decode_bytes(&bytes, declared.as_deref());
        let mut meta = meta_for(source, self.id());
        note_encoding(&mut meta, &decoded);
        let (canonical, markers) = convert(&decoded.text, options, &mut meta);
        if meta.title.is_none() {
            meta.title = markers
                .iter()
                .filter(|m| m.kind == MarkerKind::Heading && m.level == 1)
                .min_by_key(|m| m.range.start)
                .map(|m| {
                    canonical
                        .chars()
                        .skip(m.range.start.0)
                        .take(m.range.len())
                        .collect()
                })
                .or_else(|| title_from_path(source));
        }
        Ok(Document::new(meta, Rope::from_str(&canonical), markers))
    }
}

/// Star's skip list (`star/documents/html.py:8-26`).
const SKIP: &[&str] = &[
    "script", "style", "nav", "footer", "aside", "noscript", "svg", "canvas", "meta", "link",
    "base", "iframe", "template", "button", "form",
];

/// Elements that start and end a paragraph-like block.
const BLOCKS: &[&str] = &[
    "div", "section", "article", "main", "header", "address", "figure", "details", "summary",
    "center", "hgroup", "search", "fieldset", "body",
];

/// Converts an HTML document to canonical text and markers, filling `meta`
/// from `<title>`, `<html lang>`, and `<meta name>` elements.
pub fn convert(
    source: &str,
    options: &LoadOptions,
    meta: &mut DocumentMeta,
) -> (String, Vec<Marker>) {
    // The text is rarely longer than its source: room enough, nearly always.
    let mut b = Builder::with_capacity(source.len());
    walk_into(&mut b, source, options, meta, None);
    b.finish()
}

/// Where a link or picture in an archived page points (MHTML and email,
/// ADR-0035): the address to use, and a description for a picture that
/// has no alternative text of its own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Resolved {
    /// The link target or picture address to record.
    pub(crate) target: String,
    /// The picture's description (its part's `Content-Description`).
    pub(crate) description: Option<String>,
}

/// Resolves an `href` or `src` value.
pub(crate) type Resolver<'a> = &'a dyn Fn(&str) -> Option<Resolved>;

/// [`walk_into`] with links and pictures resolved by `resolve` (the parts
/// of a web archive or an email).
pub(crate) fn walk_resolved_into<'a>(
    b: &'a mut Builder,
    source: &str,
    options: &'a LoadOptions,
    meta: &mut DocumentMeta,
    resolve: Resolver<'a>,
) {
    walk(b, source, options, meta, None, false, Some(resolve));
}

/// Called with each element `id` (and `<a name>`) as the walker reaches it,
/// before the element's content, so a caller can open markers there (EPUB
/// table-of-contents targets).
pub(crate) type AnchorHook<'a> = &'a mut dyn FnMut(&str, &mut Builder);

/// Converts an HTML document into an existing builder (EPUB chapters share
/// one), filling `meta` from its head.
pub(crate) fn walk_into<'a>(
    b: &'a mut Builder,
    source: &str,
    options: &'a LoadOptions,
    meta: &mut DocumentMeta,
    on_anchor: Option<AnchorHook<'a>>,
) {
    walk(b, source, options, meta, on_anchor, false, None);
}

/// [`walk_into`] for an EPUB 3 chapter: an `epub:switch` reads its MathML
/// case, or else its default. (MathML is read as math in every page.)
pub(crate) fn walk_epub_into<'a>(
    b: &'a mut Builder,
    source: &str,
    options: &'a LoadOptions,
    meta: &mut DocumentMeta,
    on_anchor: Option<AnchorHook<'a>>,
) {
    walk(b, source, options, meta, on_anchor, true, None);
}

fn walk<'a>(
    b: &'a mut Builder,
    source: &str,
    options: &'a LoadOptions,
    meta: &mut DocumentMeta,
    on_anchor: Option<AnchorHook<'a>>,
    epub: bool,
    resolve: Option<Resolver<'a>>,
) {
    let html = Html::parse_document(source);
    let root = html.root_element();
    if let Some(lang) = root.attr("lang").or_else(|| root.attr("xml:lang"))
        && !lang.trim().is_empty()
    {
        meta.language = Some(lang.trim().to_owned());
    }
    let mut w = Walker {
        b,
        options,
        lists: Vec::new(),
        in_cell: 0,
        on_anchor,
        depth: 0,
        flattened: false,
        epub,
        resolve,
    };
    for child in root.child_elements() {
        if child.value().name() == "head" {
            read_head(child, meta);
        } else {
            w.element(child);
        }
    }
    if w.flattened {
        crate::add_warning(meta, crate::NESTING_WARNING);
    }
}

/// The ids (and `<a name>` anchors) present in an HTML document.
pub(crate) fn anchor_ids(source: &str) -> std::collections::HashSet<String> {
    let html = Html::parse_document(source);
    let mut out = std::collections::HashSet::new();
    for el in html.root_element().descendent_elements() {
        if let Some(id) = el.attr("id") {
            out.insert(id.to_owned());
        }
        if el.value().name() == "a"
            && let Some(n) = el.attr("name")
        {
            out.insert(n.to_owned());
        }
    }
    out
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn read_head(head: ElementRef<'_>, meta: &mut DocumentMeta) {
    for el in head.descendent_elements() {
        match el.value().name() {
            "title" if meta.title.is_none() => {
                let t = collapse(&el.text().collect::<String>());
                if !t.is_empty() {
                    meta.title = Some(t);
                }
            }
            "meta" => {
                let (Some(name), Some(content)) = (el.attr("name"), el.attr("content")) else {
                    continue;
                };
                let content = collapse(content);
                if content.is_empty() {
                    continue;
                }
                match name.to_ascii_lowercase().as_str() {
                    "author" => meta.author = Some(content),
                    "viewport" | "generator" | "robots" => {}
                    other => {
                        meta.properties.insert(other.to_owned(), content);
                    }
                }
            }
            _ => {}
        }
    }
}

struct Walker<'a> {
    b: &'a mut Builder,
    options: &'a LoadOptions,
    /// Next number of each open list (`None` for unordered lists).
    lists: Vec<Option<i64>>,
    /// Inside a table cell: blocks become spaces.
    in_cell: usize,
    on_anchor: Option<AnchorHook<'a>>,
    /// Element nesting, bounded by [`MAX_NESTING`](crate::MAX_NESTING).
    depth: usize,
    /// Set once content past the nesting limit was flattened.
    flattened: bool,
    /// An EPUB 3 chapter: `epub:switch` is read.
    epub: bool,
    /// Resolves links and pictures (web archives and email).
    resolve: Option<Resolver<'a>>,
}

fn marker(kind: MarkerKind) -> Marker {
    Marker::new(kind, CharRange::empty(0))
}

fn is_hidden(el: &ElementRef<'_>) -> bool {
    let v = el.value();
    v.attr("hidden").is_some()
        || v.attr("aria-hidden")
            .is_some_and(|a| a.eq_ignore_ascii_case("true"))
}

impl Walker<'_> {
    fn paragraph_break(&mut self) {
        if self.in_cell > 0 {
            self.b.space();
        } else if !self.lists.is_empty() {
            self.b.line_break();
        } else {
            self.b.paragraph_break();
        }
    }

    fn line_break(&mut self) {
        if self.in_cell > 0 {
            self.b.space();
        } else {
            self.b.line_break();
        }
    }

    fn children(&mut self, el: ElementRef<'_>) {
        for child in el.children() {
            match child.value() {
                Node::Text(t) => self.b.text(t),
                Node::Element(_) => {
                    if let Some(c) = ElementRef::wrap(child) {
                        self.element(c);
                    }
                }
                _ => {}
            }
        }
    }

    fn wrapped(&mut self, el: ElementRef<'_>, m: Marker) {
        let id = self.b.open(m);
        self.children(el);
        self.b.close(id);
    }

    fn element(&mut self, el: ElementRef<'_>) {
        if self.depth >= crate::MAX_NESTING {
            self.flatten(el);
            return;
        }
        self.depth += 1;
        self.element_inner(el);
        self.depth -= 1;
    }

    /// Content nested past the limit: its text without structure, skipped
    /// and hidden elements still left out. Iterative, so no depth can
    /// overflow the stack.
    fn flatten(&mut self, el: ElementRef<'_>) {
        /// Inline elements that do not separate words.
        const INLINE: &[&str] = &[
            "a", "abbr", "b", "bdi", "bdo", "cite", "code", "data", "dfn", "em", "i", "kbd",
            "mark", "q", "s", "samp", "small", "span", "strong", "sub", "sup", "time", "u", "var",
        ];
        self.flattened = true;
        self.b.space();
        // Depth-first with an explicit stack, children in document order.
        let mut stack: Vec<_> = el.children().collect();
        stack.reverse();
        while let Some(n) = stack.pop() {
            match n.value() {
                Node::Text(t) => self.b.text(t),
                Node::Element(e) => {
                    let hidden = ElementRef::wrap(n).is_some_and(|r| is_hidden(&r));
                    if SKIP.contains(&e.name()) || hidden {
                        continue;
                    }
                    if !INLINE.contains(&e.name()) {
                        self.b.space();
                    }
                    let from = stack.len();
                    stack.extend(n.children());
                    stack[from..].reverse();
                }
                _ => {}
            }
        }
        self.b.space();
    }

    fn element_inner(&mut self, el: ElementRef<'_>) {
        let name = el.value().name();
        if let Some(hook) = &mut self.on_anchor {
            if let Some(id) = el.attr("id") {
                hook(id, self.b);
            }
            if name == "a"
                && let Some(n) = el.attr("name")
            {
                hook(n, self.b);
            }
        }
        if name == "svg" && !is_hidden(&el) {
            return self.svg(el);
        }
        if SKIP.contains(&name) || is_hidden(&el) {
            return;
        }
        match crate::mathml::local(name) {
            "math" => return self.math(el),
            "switch" if self.epub && name.starts_with("epub:") => return self.switch(el),
            _ => {}
        }
        match name {
            "head" | "title" | "caption" => {}
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = name[1..].parse::<u8>().unwrap_or(1);
                self.paragraph_break();
                if self.in_cell > 0 {
                    self.children(el);
                } else {
                    self.wrapped(el, marker(MarkerKind::Heading).with_level(level));
                }
                self.paragraph_break();
            }
            "p" => {
                self.paragraph_break();
                if self.in_cell > 0 || !self.lists.is_empty() {
                    self.children(el);
                } else {
                    self.wrapped(el, marker(MarkerKind::Paragraph));
                }
                self.paragraph_break();
            }
            "br" => self.line_break(),
            "hr" => self.paragraph_break(),
            "ul" | "ol" | "menu" => self.list(el, name == "ol"),
            "li" => self.list_item(el),
            "dl" => {
                self.paragraph_break();
                self.children(el);
                self.paragraph_break();
            }
            "dt" | "dd" => {
                self.line_break();
                self.children(el);
                self.line_break();
            }
            "blockquote" => {
                self.paragraph_break();
                if self.in_cell > 0 {
                    self.children(el);
                } else {
                    self.wrapped(el, marker(MarkerKind::Quote));
                }
                self.paragraph_break();
            }
            "pre" => self.pre(el),
            "code" | "kbd" | "samp" | "tt" => self.wrapped(el, marker(MarkerKind::Code)),
            "a" => match el.attr("href") {
                Some(href) => {
                    let target = self
                        .resolved(href)
                        .map_or_else(|| href.to_owned(), |r| r.target);
                    self.wrapped(el, marker(MarkerKind::Link).with_reference(target));
                }
                None => self.children(el),
            },
            "strong" | "b" => self.wrapped(el, marker(MarkerKind::Bold)),
            "em" | "i" | "cite" | "dfn" => self.wrapped(el, marker(MarkerKind::Italic)),
            "u" | "ins" => self.wrapped(el, marker(MarkerKind::Underline)),
            "img" => self.image(el),
            "figcaption" => {
                self.paragraph_break();
                self.children(el);
                self.paragraph_break();
            }
            "table" => self.table(el),
            _ if BLOCKS.contains(&name) => {
                self.paragraph_break();
                self.children(el);
                self.paragraph_break();
            }
            _ => self.children(el),
        }
    }

    /// An `href` or `src` through the resolver, when there is one.
    fn resolved(&self, reference: &str) -> Option<Resolved> {
        self.resolve.and_then(|r| r(reference))
    }

    /// MathML: LaTeX with its delimiters under a `Math` marker,
    /// as the Markdown and DOCX loaders write math: `$…$` at level 0, or
    /// display math `$$…$$` at level 1 on a line of its own (in a table
    /// cell, between spaces). Without any math, the `alttext` or an
    /// image's alt text, as plain text.
    fn math(&mut self, el: ElementRef<'_>) {
        let m = crate::mathml::read(el);
        let Some(latex) = m.latex else {
            if let Some(text) = m.fallback {
                self.b.text(&text);
            }
            return;
        };
        if m.display {
            self.line_break();
        }
        let text = if m.display {
            format!("$${latex}$$")
        } else {
            format!("${latex}$")
        };
        let id = self
            .b
            .open(marker(MarkerKind::Math).with_level(u8::from(m.display)));
        self.b.text(&text);
        self.b.close(id);
        if m.display {
            self.line_break();
        }
    }

    /// `epub:switch`: the first `epub:case` that holds MathML, else
    /// `epub:default`, never both.
    fn switch(&mut self, el: ElementRef<'_>) {
        let local = |e: &ElementRef<'_>| crate::mathml::local(e.value().name()).to_owned();
        let parts: Vec<ElementRef<'_>> = el.child_elements().collect();
        let case = parts
            .iter()
            .find(|c| local(c) == "case" && c.descendent_elements().any(|d| local(&d) == "math"));
        let chosen = case.or_else(|| parts.iter().find(|c| local(c) == "default"));
        if let Some(c) = chosen {
            self.children(*c);
        }
    }

    fn list(&mut self, el: ElementRef<'_>, ordered: bool) {
        if self.lists.is_empty() {
            self.paragraph_break();
        } else {
            self.line_break();
        }
        let start = el
            .attr("start")
            .and_then(|s| s.trim().parse::<i64>().ok())
            .unwrap_or(1);
        self.lists.push(ordered.then_some(start));
        let depth = u8::try_from(self.lists.len()).unwrap_or(u8::MAX);
        if self.in_cell > 0 {
            self.children(el);
        } else {
            self.wrapped(el, marker(MarkerKind::List).with_level(depth));
        }
        self.lists.pop();
        self.paragraph_break();
    }

    fn list_item(&mut self, el: ElementRef<'_>) {
        self.line_break();
        let depth = u8::try_from(self.lists.len().max(1)).unwrap_or(u8::MAX);
        let mut m = marker(MarkerKind::ListItem).with_level(depth);
        if let Some(Some(n)) = self.lists.last_mut() {
            if let Some(v) = el.attr("value").and_then(|v| v.trim().parse::<i64>().ok()) {
                *n = v;
            }
            m = m.with_label(format!("{n}."));
            *n += 1;
        }
        if self.in_cell > 0 {
            self.children(el);
        } else {
            self.wrapped(el, m);
        }
        self.line_break();
    }

    fn pre(&mut self, el: ElementRef<'_>) {
        self.paragraph_break();
        if self.options.skip_code {
            return;
        }
        let text: String = el.text().collect();
        let lang = std::iter::once(el)
            .chain(el.child_elements().filter(|c| c.value().name() == "code"))
            .flat_map(|e| e.value().classes().collect::<Vec<_>>())
            .find_map(|c| {
                c.strip_prefix("language-")
                    .or_else(|| c.strip_prefix("lang-"))
                    .map(str::to_owned)
            });
        if self.in_cell > 0 {
            self.b.text(&text);
        } else {
            let mut m = marker(MarkerKind::Code).with_level(1);
            if let Some(lang) = lang {
                m = m.with_label(lang);
            }
            let id = self.b.open(m);
            self.b.verbatim(&text);
            self.b.close(id);
        }
        self.paragraph_break();
    }

    fn image(&mut self, el: ElementRef<'_>) {
        if el.attr("role").is_some_and(|r| {
            r.eq_ignore_ascii_case("presentation") || r.eq_ignore_ascii_case("none")
        }) {
            return;
        }
        let resolved = el.attr("src").and_then(|src| self.resolved(src));
        let alt = ["alt", "title", "aria-label"]
            .iter()
            .filter_map(|a| el.attr(a))
            .map(collapse)
            .find(|s| !s.is_empty());
        // An archived picture with no text of its own: its part's
        // description (never for `alt=""`, which marks it decorative).
        let alt = alt.or_else(|| {
            if el.attr("alt").is_some() {
                return None;
            }
            resolved
                .as_ref()
                .and_then(|r| r.description.as_deref())
                .map(collapse)
                .filter(|d| !d.is_empty())
        });
        let Some(alt) = alt else {
            return;
        };
        let mut m = marker(MarkerKind::Image);
        if let Some(r) = &resolved {
            m = m.with_reference(r.target.clone());
        } else if let Some(src) = el.attr("src") {
            m = m.with_reference(src);
        }
        // A picture's words never run into the words beside it
        // ("a cell" and "A nucleus", not "a cellA nucleus").
        self.b.space();
        let id = self.b.open(m);
        self.b.text(&alt);
        if let Some(desc) = el.attr("longdesc").filter(|d| !d.trim().is_empty()) {
            self.b
                .text(&format!(" (long description: {})", desc.trim()));
        }
        self.b.close(id);
        self.b.soft_space();
    }

    /// An inline `<svg>` drawing, read as an SVG file is (see
    /// [`crate::svg`]): a graphic named by its `aria-label` or `<title>`,
    /// then its description, titled parts, and text. One with nothing to
    /// read is decorative and produces nothing, as an image without
    /// alternative text does.
    fn svg(&mut self, el: ElementRef<'_>) {
        if el.attr("role").is_some_and(|r| {
            r.eq_ignore_ascii_case("presentation") || r.eq_ignore_ascii_case("none")
        }) {
            return;
        }
        let reading = crate::svg::read(&crate::svg::from_html(el));
        if self.in_cell > 0 {
            // In a table cell, only the name: a cell stays one line.
            if let Some(name) = reading.name {
                self.b.space();
                let id = self.b.open(marker(MarkerKind::Image));
                self.b.text(&name);
                self.b.close(id);
                self.b.soft_space();
            }
            return;
        }
        crate::svg::write_inline(&mut *self.b, &reading);
    }

    fn table(&mut self, el: ElementRef<'_>) {
        if self.in_cell > 0 {
            // A nested table reads as running text inside its cell.
            self.in_cell += 1;
            self.children(el);
            self.in_cell -= 1;
            return;
        }
        let caption = el
            .child_elements()
            .find(|c| c.value().name() == "caption")
            .map(|c| collapse(&c.text().collect::<String>()))
            .filter(|c| !c.is_empty());
        self.b.paragraph_break();
        if let Some(c) = &caption {
            self.b.text(c);
            self.b.paragraph_break();
        }
        let mut m = marker(MarkerKind::Table);
        if let Some(c) = caption {
            m = m.with_label(c);
        }
        let table = self.b.open(m);
        for (row, in_head) in rows(el) {
            let cells: Vec<ElementRef<'_>> = row
                .child_elements()
                .filter(|c| matches!(c.value().name(), "td" | "th"))
                .collect();
            if cells.is_empty() {
                continue;
            }
            let header = in_head || cells.iter().all(|c| c.value().name() == "th");
            self.b.line_break();
            let mut rm = marker(MarkerKind::TableRow);
            if header {
                rm = rm.with_label(HEADER_ROW_LABEL);
            }
            let row_id = self.b.open(rm);
            for (i, cell) in cells.into_iter().enumerate() {
                if i > 0 {
                    self.b.separator(crate::CELL_SEPARATOR);
                }
                let id = self.b.open_here(marker(MarkerKind::TableCell));
                self.in_cell += 1;
                self.children(cell);
                self.in_cell -= 1;
                self.b.close(id);
            }
            self.b.close(row_id);
        }
        self.b.close(table);
        self.b.paragraph_break();
    }
}

/// The rows of a table in document order, each with whether it is in the
/// table head.
fn rows(table: ElementRef<'_>) -> Vec<(ElementRef<'_>, bool)> {
    let mut out = Vec::new();
    for child in table.child_elements() {
        match child.value().name() {
            "tr" => out.push((child, false)),
            "thead" | "tbody" | "tfoot" => {
                let head = child.value().name() == "thead";
                out.extend(
                    child
                        .child_elements()
                        .filter(|r| r.value().name() == "tr")
                        .map(|r| (r, head)),
                );
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use textweaver_core::MarkerKind;

    use super::*;

    fn load(src: &str) -> Document {
        HtmlLoader
            .load(
                &Source::Bytes {
                    data: src.as_bytes().to_vec(),
                    hint: "html".into(),
                },
                &LoadOptions::default(),
            )
            .unwrap()
    }

    fn kinds(d: &Document, kind: MarkerKind) -> Vec<String> {
        d.marker_index()
            .iter(kind, None)
            .map(|m| d.slice(m.range))
            .collect()
    }

    #[test]
    fn unclosed_meta_does_not_empty_the_document() {
        let d = load(
            "<html lang=\"en\"><head><meta charset=\"utf-8\"><title>T</title><link rel=x></head><body><p>Hello <b>world</b>.</p></body></html>",
        );
        assert_eq!(d.text().to_string(), "Hello world.");
        assert_eq!(d.meta.title.as_deref(), Some("T"));
        assert_eq!(d.meta.language.as_deref(), Some("en"));
        assert_eq!(kinds(&d, MarkerKind::Bold), ["world"]);
    }

    #[test]
    fn inline_svg_reads_its_label_or_title() {
        let d = load(
            "<p>Before <svg aria-label=\"Sales chart\"><title>ignored</title><text>9</text></svg> after.</p>\
             <svg><title>Logo of the club</title><circle r=\"3\"/></svg>\
             <svg><circle r=\"3\"/></svg><svg role=\"presentation\"><title>Deco</title></svg>\
             <svg aria-hidden=\"true\"><title>Hidden</title></svg><p>End.</p>",
        );
        assert_eq!(
            kinds(&d, MarkerKind::Image),
            ["Sales chart", "Logo of the club"]
        );
        let text = d.text().to_string();
        // The chart's own text follows its name.
        assert!(text.contains("Before Sales chart\n\n9\n\nafter."), "{text}");
        for gone in ["ignored", "Deco", "Hidden"] {
            assert!(!text.contains(gone), "{gone} in {text}");
        }
    }

    #[test]
    fn declared_charset_decodes_legacy_pages() {
        let d = HtmlLoader
            .load(
                &Source::Bytes {
                    data: b"<html><head><meta charset=iso-8859-1></head><body><p>Caf\xe9 cr\xe8me.</p></body></html>".to_vec(),
                    hint: "html".into(),
                },
                &LoadOptions::default(),
            )
            .unwrap();
        assert_eq!(d.text().to_string(), "Café crème.");
        assert_eq!(
            d.meta.properties.get("encoding").map(String::as_str),
            Some("windows-1252")
        );
    }

    #[test]
    fn nested_lists_keep_their_parent_items() {
        let d = load(
            "<ul><li>Apples</li><li>Oranges<ul><li>Blood oranges</li></ul></li></ul><ol start=3><li>c</li><li>d</li></ol>",
        );
        assert_eq!(
            d.text().to_string(),
            "Apples\nOranges\nBlood oranges\n\nc\nd"
        );
        let labels: Vec<Option<String>> = d
            .marker_index()
            .iter(MarkerKind::ListItem, None)
            .map(|m| m.label.clone())
            .collect();
        assert_eq!(
            labels,
            [None, None, None, Some("3.".into()), Some("4.".into())]
        );
        let levels: Vec<u8> = d
            .marker_index()
            .iter(MarkerKind::ListItem, None)
            .map(|m| m.level)
            .collect();
        assert_eq!(levels, [1, 1, 2, 1, 1]);
    }

    #[test]
    fn tables_keep_captions_and_empty_cells() {
        let d = load(
            "<table><caption>Scores</caption><tr><th>Name</th><th>Score</th></tr><tr><td>Ada</td><td></td></tr></table><p>After.</p>",
        );
        assert_eq!(
            d.text().to_string(),
            "Scores\n\nName | Score\nAda | \n\nAfter."
        );
        let table = d
            .marker_index()
            .nth(MarkerKind::Table, None, 0)
            .unwrap()
            .clone();
        assert_eq!(table.label.as_deref(), Some("Scores"));
        assert_eq!(
            kinds(&d, MarkerKind::TableCell),
            ["Name", "Score", "Ada", ""]
        );
    }

    #[test]
    fn images_skip_lists_and_hidden_content() {
        let d = load(
            "<nav>Menu</nav><img src=a.png alt=\"A chart\"><blockquote><p>Quoted.</p></blockquote><img src=s.gif alt=\"\"><img src=t.png title=\"Titled\"><p hidden>secret</p><span aria-hidden=true>x</span><footer>f</footer><form>q</form>",
        );
        assert_eq!(d.text().to_string(), "A chart\n\nQuoted.\n\nTitled");
        assert_eq!(kinds(&d, MarkerKind::Image), ["A chart", "Titled"]);
        assert_eq!(kinds(&d, MarkerKind::Quote), ["Quoted."]);
    }

    #[test]
    fn mathml_in_a_web_page_is_math() {
        let d = load(
            "<p>Area <math><mi>\u{3c0}</mi><msup><mi>r</mi><mn>2</mn></msup></math>.</p><math display=\"block\"><mi>x</mi></math><p>After.</p>",
        );
        let levels: Vec<u8> = d
            .marker_index()
            .iter(MarkerKind::Math, None)
            .map(|m| m.level)
            .collect();
        assert_eq!(levels, [0, 1], "{}", d.text());
        assert!(d.text().to_string().starts_with("Area $"), "{}", d.text());
        // `epub:switch` is EPUB's; a web page reads its content as usual.
        let d = load(
            "<epub:switch><epub:case>a</epub:case><epub:default>b</epub:default></epub:switch>",
        );
        assert_eq!(d.text().to_string(), "ab");
    }

    #[test]
    fn a_resolver_rewrites_links_and_describes_pictures() {
        let resolve = |r: &str| {
            r.starts_with("cid:").then(|| Resolved {
                target: format!("https://example.org/{}", &r[4..]),
                description: Some("The logo".into()),
            })
        };
        let mut b = Builder::new();
        let options = LoadOptions::default();
        let mut meta = DocumentMeta::default();
        walk_resolved_into(
            &mut b,
            "<p><img src=\"cid:a\"><img src=\"cid:b\" alt=\"\"><a href=\"cid:c\">link</a></p>",
            &options,
            &mut meta,
            &resolve,
        );
        let (text, markers) = b.finish();
        assert_eq!(text, "The logo link");
        let refs: Vec<String> = markers.iter().filter_map(|m| m.reference.clone()).collect();
        assert_eq!(
            refs,
            ["https://example.org/a", "https://example.org/c"],
            "{markers:?}"
        );
    }

    #[test]
    fn pre_keeps_lines_and_br_breaks() {
        let d =
            load("<p>a<br>b</p><pre><code class=\"language-rust\">let x = 1;\n  y();</code></pre>");
        assert_eq!(d.text().to_string(), "a\nb\n\nlet x = 1;\n  y();");
        let code = d
            .marker_index()
            .nth(MarkerKind::Code, Some(1), 0)
            .unwrap()
            .clone();
        assert_eq!(code.label.as_deref(), Some("rust"));
    }
}
