//! SVG drawings, as files and inline in web pages (ADR-0044), read the way
//! SVG-AAM exposes them to a screen reader: `<title>` (or `aria-label`)
//! names an element, `<desc>` describes it, and `role="img"` on the
//! drawing makes it one graphic whose parts are not read.
//!
//! A drawing reads as its name first (a level-1 heading for a file, a
//! graphic for an inline drawing), then its description, then each titled
//! part (a group, or a chart's bar with a `<title>`) as a list item in
//! document order, then the text it shows (`<text>`, with its `<tspan>`s)
//! a line each. A drawing with none of these reads "Drawing with no
//! description". An inline drawing with none of them is decorative and
//! reads nothing, as an image with empty alternative text does.
//!
//! Files are parsed with roxmltree (with a DOCTYPE allowed, no external
//! entities, nesting and node counts bounded); an inline drawing comes from
//! the HTML parser. Both become the same small tree first, so both read
//! alike.

use ropey::Rope;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, Marker};

use crate::builder::Builder;
use crate::{
    LoadError, LoadOptions, Loader, Source, add_warning, decode_bytes, encoding, meta_for,
    note_encoding, title_from_path,
};

/// Largest SVG file read.
pub const MAX_BYTES: usize = 32 << 20;

/// Most elements of one drawing looked at.
const MAX_ELEMENTS: usize = 500_000;

/// What a drawing with nothing to read says.
pub const NO_DESCRIPTION: &str = "Drawing with no description";

/// Elements whose content is never shown, or never text.
const HIDDEN: &[&str] = &[
    "defs",
    "symbol",
    "style",
    "script",
    "metadata",
    "clipPath",
    "clippath",
    "mask",
    "pattern",
    "marker",
    "linearGradient",
    "lineargradient",
    "radialGradient",
    "radialgradient",
    "filter",
    "foreignObject",
    "foreignobject",
];

/// An element of a drawing, reduced to what reading needs.
#[derive(Debug, Default)]
pub(crate) struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Child>,
}

#[derive(Debug)]
enum Child {
    Element(Node),
    Text(String),
}

impl Node {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    fn elements(&self) -> impl Iterator<Item = &Node> {
        self.children.iter().filter_map(|c| match c {
            Child::Element(e) => Some(e),
            Child::Text(_) => None,
        })
    }

    /// All the text inside, words joined by single spaces.
    fn text(&self) -> String {
        let mut out = String::new();
        // Depth first, children in order.
        let mut order: Vec<&Child> = self.children.iter().rev().collect();
        while let Some(c) = order.pop() {
            match c {
                Child::Text(t) => {
                    for w in t.split_whitespace() {
                        if !out.is_empty() {
                            out.push(' ');
                        }
                        out.push_str(w);
                    }
                }
                Child::Element(e) => {
                    if !is_hidden(e) {
                        order.extend(e.children.iter().rev());
                    }
                }
            }
        }
        out
    }

    /// The text of the first child element named `name`.
    fn child_text(&self, name: &str) -> Option<String> {
        self.elements()
            .find(|e| e.name == name)
            .map(Node::text)
            .filter(|t| !t.is_empty())
    }

    /// Its accessible name: `aria-label`, else its own `<title>`.
    fn accessible_name(&self) -> Option<String> {
        self.attr("aria-label")
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|l| !l.is_empty())
            .or_else(|| self.child_text("title"))
    }
}

fn is_hidden(e: &Node) -> bool {
    HIDDEN.contains(&e.name.as_str())
        || e.attr("aria-hidden")
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
        || e.attr("display").is_some_and(|v| v.trim() == "none")
        || e.attr("visibility").is_some_and(|v| v.trim() == "hidden")
}

/// What a drawing gives a reader.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Reading {
    /// Its name.
    pub(crate) name: Option<String>,
    /// Its description.
    pub(crate) description: Option<String>,
    /// Its titled parts, in order ("Quarter 1: 40 units").
    pub(crate) parts: Vec<String>,
    /// The text it shows, in order.
    pub(crate) texts: Vec<String>,
}

impl Reading {
    /// Nothing at all to read.
    pub(crate) fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.description.is_none()
            && self.parts.is_empty()
            && self.texts.is_empty()
    }
}

/// Reads a drawing's root `<svg>` element.
pub(crate) fn read(root: &Node) -> Reading {
    let mut r = Reading {
        name: root.accessible_name(),
        description: root
            .attr("aria-description")
            .map(str::to_owned)
            .or_else(|| root.child_text("desc")),
        ..Reading::default()
    };
    let one_graphic = root
        .attr("role")
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("img"));
    if one_graphic {
        return r;
    }
    // Depth first, in document order.
    let mut stack: Vec<&Node> = root.elements().collect::<Vec<_>>();
    stack.reverse();
    while let Some(e) = stack.pop() {
        if is_hidden(e) || matches!(e.name.as_str(), "title" | "desc") {
            continue;
        }
        if e.name == "text" {
            let t = e.text();
            if !t.is_empty() {
                r.texts.push(t);
            }
            continue;
        }
        if let Some(name) = e.accessible_name() {
            let part = match e.child_text("desc") {
                Some(d) => format!("{name}: {d}"),
                None => name,
            };
            r.parts.push(part);
        }
        let from = stack.len();
        stack.extend(e.elements());
        stack[from..].reverse();
    }
    r
}

fn marker(kind: MarkerKind) -> Marker {
    Marker::new(kind, CharRange::empty(0))
}

/// Writes a drawing's parts and text after its name and description.
fn write_rest(b: &mut Builder, r: &Reading) {
    if let Some(d) = &r.description {
        b.paragraph_break();
        let id = b.open(marker(MarkerKind::Paragraph));
        b.text(d);
        b.close(id);
    }
    if !r.parts.is_empty() {
        b.paragraph_break();
        let list = b.open(marker(MarkerKind::List).with_level(1));
        for p in &r.parts {
            b.line_break();
            let id = b.open(marker(MarkerKind::ListItem).with_level(1));
            b.text(p);
            b.close(id);
        }
        b.close(list);
    }
    if !r.texts.is_empty() {
        b.paragraph_break();
        for (i, t) in r.texts.iter().enumerate() {
            if i > 0 {
                b.line_break();
            }
            b.text(t);
        }
    }
    b.paragraph_break();
}

/// An SVG file: its name as a level-1 heading, then the rest.
pub(crate) fn write_file(b: &mut Builder, r: &Reading) {
    if r.is_empty() {
        b.text(NO_DESCRIPTION);
        return;
    }
    if let Some(name) = &r.name {
        let id = b.open(marker(MarkerKind::Heading).with_level(1));
        b.text(name);
        b.close(id);
    }
    write_rest(b, r);
}

/// An inline drawing in a web page: a graphic named by its name (or
/// "Drawing" when only its description or text says what it is), then
/// the rest. A drawing with nothing to read is left out.
pub(crate) fn write_inline(b: &mut Builder, r: &Reading) {
    if r.is_empty() {
        return;
    }
    let name = r.name.as_deref().unwrap_or("Drawing");
    b.space();
    let id = b.open(marker(MarkerKind::Image));
    b.text(name);
    b.close(id);
    if r.description.is_some() || !r.parts.is_empty() || !r.texts.is_empty() {
        write_rest(b, r);
    } else {
        b.soft_space();
    }
}

/// The small tree of an inline `<svg>` from the HTML parser.
pub(crate) fn from_html(el: scraper::ElementRef<'_>) -> Node {
    let mut budget = MAX_ELEMENTS;
    from_html_at(el, 0, &mut budget)
}

fn from_html_at(el: scraper::ElementRef<'_>, depth: usize, budget: &mut usize) -> Node {
    let v = el.value();
    let mut node = Node {
        name: v.name().to_owned(),
        attrs: v
            .attrs()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect(),
        children: Vec::new(),
    };
    for c in el.children() {
        match c.value() {
            scraper::Node::Text(t) => node.children.push(Child::Text(t.to_string())),
            scraper::Node::Element(_) if depth < crate::MAX_NESTING && *budget > 0 => {
                *budget -= 1;
                if let Some(e) = scraper::ElementRef::wrap(c) {
                    node.children
                        .push(Child::Element(from_html_at(e, depth + 1, budget)));
                }
            }
            _ => {}
        }
    }
    node
}

/// The small tree of a parsed SVG file (iterative: no depth can overflow
/// the stack).
fn from_xml(root: roxmltree::Node<'_, '_>) -> Node {
    fn shallow(n: roxmltree::Node<'_, '_>) -> Node {
        Node {
            name: n.tag_name().name().to_owned(),
            attrs: n
                .attributes()
                .map(|a| (a.name().to_owned(), a.value().to_owned()))
                .collect(),
            children: Vec::new(),
        }
    }
    // Built bottom-up: a stack of (built node, children still to read, in
    // reverse order).
    fn kids<'a, 'i>(n: roxmltree::Node<'a, 'i>) -> Vec<roxmltree::Node<'a, 'i>> {
        let mut k: Vec<_> = n.children().collect();
        k.reverse();
        k
    }
    let mut stack: Vec<(Node, Vec<roxmltree::Node<'_, '_>>)> = vec![(shallow(root), kids(root))];
    let mut seen = 0usize;
    loop {
        let Some(top) = stack.last_mut() else {
            return Node::default();
        };
        let Some(next) = top.1.pop() else {
            let Some((done, _)) = stack.pop() else {
                return Node::default();
            };
            match stack.last_mut() {
                Some(parent) => parent.0.children.push(Child::Element(done)),
                None => return done,
            }
            continue;
        };
        if next.is_text() {
            if let Some(t) = next.text() {
                top.0.children.push(Child::Text(t.to_owned()));
            }
        } else if next.is_element() {
            seen += 1;
            if seen <= MAX_ELEMENTS {
                stack.push((shallow(next), kids(next)));
            }
        }
    }
}

/// Loads SVG files.
#[derive(Clone, Copy, Debug, Default)]
pub struct SvgLoader;

impl Loader for SvgLoader {
    fn id(&self) -> &'static str {
        "svg"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["svg"]
    }

    /// Pictures, not documents: a folder scan leaves them out; they open by
    /// name.
    fn scan_extensions(&self) -> &'static [&'static str] {
        &[]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, _options: &LoadOptions) -> Result<Document, LoadError> {
        let head = source.read_head(MAX_BYTES + 1)?;
        if head.len() > MAX_BYTES {
            return Err(LoadError::Parse(format!(
                "the drawing is larger than {} megabytes",
                MAX_BYTES >> 20
            )));
        }
        if let Some(kind) = encoding::binary_kind(&head[..head.len().min(encoding::SNIFF_BYTES)]) {
            // A compressed `.svgz` renamed, or not a drawing at all.
            return Err(LoadError::Binary(
                title_from_path(source).unwrap_or_else(|| "This drawing".into()),
                kind,
            ));
        }
        let declared = encoding::sniff_xml_encoding(&head);
        let decoded = decode_bytes(&head, declared.as_deref());
        let mut meta = meta_for(source, self.id());
        note_encoding(&mut meta, &decoded);
        let limited = crate::xmldepth::limit_depth(&decoded.text, crate::MAX_NESTING);
        if limited.is_some() {
            add_warning(&mut meta, crate::NESTING_WARNING);
        }
        let text = limited.as_deref().unwrap_or(&decoded.text);
        let xml = crate::package::parse_xml(text)?;
        let root = xml.root_element();
        if root.tag_name().name() != "svg" {
            return Err(LoadError::Parse("it is not an SVG drawing".into()));
        }
        let reading = read(&from_xml(root));
        meta.title = reading.name.clone().or_else(|| title_from_path(source));
        let mut b = Builder::new();
        write_file(&mut b, &reading);
        let (text, markers) = b.finish();
        Ok(Document::new(meta, Rope::from_str(&text), markers))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(src: &str) -> Result<Document, LoadError> {
        SvgLoader.load(
            &Source::Bytes {
                data: src.as_bytes().to_vec(),
                hint: "svg".into(),
            },
            &LoadOptions::default(),
        )
    }

    fn kinds(d: &Document, kind: MarkerKind) -> Vec<String> {
        d.marker_index()
            .iter(kind, None)
            .map(|m| d.slice(m.range))
            .collect()
    }

    #[test]
    fn title_description_parts_then_text() {
        let d = load(
            r#"<?xml version="1.0"?>
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
  <title>Sales by quarter</title>
  <desc>Sales rose all year.</desc>
  <defs><text>hidden def</text></defs>
  <g><title>Quarter 1</title><desc>40 units</desc><rect/></g>
  <rect aria-label="Quarter 2"/>
  <g aria-hidden="true"><title>Secret</title><text>no</text></g>
  <text x="1">Units <tspan>sold</tspan></text>
  <text>2026</text>
</svg>"#,
        )
        .unwrap();
        assert_eq!(
            d.text().to_string(),
            "Sales by quarter\n\nSales rose all year.\n\nQuarter 1: 40 units\nQuarter 2\n\nUnits sold\n2026"
        );
        assert_eq!(kinds(&d, MarkerKind::Heading), ["Sales by quarter"]);
        assert_eq!(d.meta.title.as_deref(), Some("Sales by quarter"));
        assert_eq!(kinds(&d, MarkerKind::ListItem).len(), 2);
    }

    #[test]
    fn nothing_to_read_and_one_graphic() {
        let d = load(r#"<svg xmlns="http://www.w3.org/2000/svg"><circle r="4"/></svg>"#).unwrap();
        assert_eq!(d.text().to_string(), NO_DESCRIPTION);
        let img = load(
            r#"<svg xmlns="http://www.w3.org/2000/svg" role="img" aria-label="Logo"><text>LOGO</text></svg>"#,
        )
        .unwrap();
        assert_eq!(img.text().to_string(), "Logo");
        assert!(matches!(load("<html/>"), Err(LoadError::Parse(_))));
        assert!(matches!(load("<svg"), Err(LoadError::Parse(_))));
    }
}
