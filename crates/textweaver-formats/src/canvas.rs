//! Canvas files (`.canvas`, Obsidian's canvases), read as a document.
//!
//! The format is JSON Canvas 1.0, <https://jsoncanvas.org/spec/1.0/>: a
//! `nodes` array of cards (`text`, `file`, `link`) and groups, each with a
//! position and size (`x`, `y`, `width`, `height`) and an optional `color`,
//! and an `edges` array of connections between nodes (`fromNode`,
//! `toNode`, optional `fromEnd` and `toEnd` arrows, `label`).
//!
//! **What is read.** The document starts with a one-line summary
//! ("Canvas: 4 cards, 1 group, 3 connections."). Then each node is a
//! heading, so `h` moves card to card:
//!
//! - a **text card**'s heading is its first line, in plain words, and its
//!   other lines follow, read as Markdown (its own headings sit one level
//!   below the card's);
//! - a **file card** is a link to the file ("Mitochondria, note"; "Energy"
//!   for `file#Energy`), which follows through the vault resolver like a
//!   wiki link: by path from the canvas's folder, then from the vault root,
//!   then by name;
//! - a **link card** is a web link ("example.com/page, web page");
//! - a **group** is a heading with its label ("Week 1, group"), and the
//!   nodes inside it follow as its section, one heading level down.
//!
//! A preset color is said after the name ("Plan, red"); colors written
//! as `#rrggbb` are not named. The connections come last, under a
//! "Connections" heading, one line each: "From A to B: label", "Both ways
//! between A and B", or "Between A and B" when neither end has an arrow.
//!
//! **Reading order.** A node belongs to the smallest group whose rectangle
//! holds the node's center. Within the canvas, and within each group, nodes
//! are read top to bottom in rows, left to right within a row: a node
//! joins the row of the first node above it when its top edge is above
//! that node's middle. Connections are in the reading order of the node
//! they start from, then of the node they reach.
//!
//! A canvas costs bounded work: the file size and JSON nesting limits are
//! the JSON loader's ([`json::MAX_BYTES`](crate::json::MAX_BYTES)), and
//! at most [`MAX_NODES`] nodes and [`MAX_EDGES`] connections are read.

use std::collections::HashMap;

use ropey::Rope;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, DocumentMeta, Marker};

use crate::builder::Builder;
use crate::json::{Json, parse, text_of};
use crate::{LoadError, LoadOptions, Loader, Source, add_warning, meta_for, note_encoding};

/// Most nodes read from one canvas; the rest are left out, and the
/// document says so.
pub const MAX_NODES: usize = 5_000;

/// Most connections read from one canvas.
pub const MAX_EDGES: usize = 20_000;

/// Longest card name in a heading, in characters; a longer first line is
/// cut at a word and read in full below the heading.
const MAX_NAME: usize = 80;

/// The warning a canvas carries when [`MAX_NODES`] or [`MAX_EDGES`] cut it.
pub const CUT_WARNING: &str =
    "This canvas holds too many cards or connections to read in full, so the rest were left out.";

/// What a node is.
#[derive(Clone, Debug, PartialEq)]
enum Kind {
    /// Markdown text.
    Text(String),
    /// A file in the vault, with `#Heading` or `#^block` when given.
    File { file: String, subpath: String },
    /// A web address.
    Link(String),
    /// A group, with its label.
    Group(Option<String>),
    /// A type this reader does not know.
    Other(String),
}

/// One node of the canvas.
#[derive(Clone, Debug)]
struct Node {
    id: String,
    kind: Kind,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    color: Option<&'static str>,
}

impl Node {
    fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    fn area(&self) -> f64 {
        self.w.max(0.0) * self.h.max(0.0)
    }

    fn holds(&self, (cx, cy): (f64, f64)) -> bool {
        cx >= self.x && cy >= self.y && cx <= self.x + self.w && cy <= self.y + self.h
    }

    fn is_group(&self) -> bool {
        matches!(self.kind, Kind::Group(_))
    }
}

/// A connection between two nodes.
#[derive(Clone, Debug)]
struct Edge {
    from: String,
    to: String,
    from_arrow: bool,
    to_arrow: bool,
    label: Option<String>,
}

fn number(v: Option<&Json>) -> f64 {
    match v {
        Some(Json::Number(n)) => n.parse::<f64>().ok().filter(|f| f.is_finite()),
        _ => None,
    }
    .unwrap_or(0.0)
}

fn string(v: Option<&Json>) -> Option<String> {
    v.and_then(Json::as_str).map(str::to_owned)
}

/// A preset color's name ("1" to "6" in the spec); `None` for `#rrggbb`.
fn color_word(c: &str) -> Option<&'static str> {
    Some(match c.trim() {
        "1" => "red",
        "2" => "orange",
        "3" => "yellow",
        "4" => "green",
        "5" => "cyan",
        "6" => "purple",
        _ => return None,
    })
}

fn node(v: &Json) -> Option<Node> {
    let kind = match v.get("type").and_then(Json::as_str)? {
        "text" => Kind::Text(string(v.get("text")).unwrap_or_default()),
        "file" => Kind::File {
            file: string(v.get("file")).unwrap_or_default(),
            subpath: string(v.get("subpath")).unwrap_or_default(),
        },
        "link" => Kind::Link(string(v.get("url")).unwrap_or_default()),
        "group" => Kind::Group(string(v.get("label")).filter(|l| !l.trim().is_empty())),
        other => Kind::Other(other.to_owned()),
    };
    Some(Node {
        id: string(v.get("id")).unwrap_or_default(),
        kind,
        x: number(v.get("x")),
        y: number(v.get("y")),
        w: number(v.get("width")),
        h: number(v.get("height")),
        color: v.get("color").and_then(Json::as_str).and_then(color_word),
    })
}

fn edge(v: &Json) -> Option<Edge> {
    let end = |key: &str, default: bool| match v.get(key).and_then(Json::as_str) {
        Some("arrow") => true,
        Some("none") => false,
        _ => default,
    };
    Some(Edge {
        from: string(v.get("fromNode"))?,
        to: string(v.get("toNode"))?,
        from_arrow: end("fromEnd", false),
        to_arrow: end("toEnd", true),
        label: string(v.get("label")).filter(|l| !l.trim().is_empty()),
    })
}

/// The first line of Markdown in plain words: no heading, quote, list or
/// task marks, no emphasis marks, links by their text.
fn plain_line(line: &str) -> String {
    let mut t = line.trim();
    t = t.trim_start_matches('#').trim_start();
    t = t.trim_start_matches('>').trim_start();
    for bullet in ["- ", "* ", "+ "] {
        if let Some(rest) = t.strip_prefix(bullet) {
            t = rest.trim_start();
            break;
        }
    }
    if t.len() > 3 && t.starts_with('[') && t.as_bytes().get(2) == Some(&b']') {
        t = t[3..].trim_start();
    }
    let mut out = String::with_capacity(t.len());
    let mut rest = t;
    while let Some(c) = rest.chars().next() {
        if let Some((r, end)) = rest
            .strip_prefix("[[")
            .and_then(|r| r.find("]]").map(|end| (r, end)))
        {
            let inner = &r[..end];
            let shown = inner.rsplit_once('|').map_or(inner, |(_, a)| a);
            out.push_str(shown);
            rest = &r[end + 2..];
            continue;
        }
        let link = (c == '[')
            .then(|| rest.find("]("))
            .flatten()
            .and_then(|close| rest[close..].find(')').map(|paren| (close, paren)));
        if let Some((close, paren)) = link {
            out.push_str(&rest[1..close]);
            rest = &rest[close + paren + 1..];
            continue;
        }
        if !matches!(c, '*' | '_' | '`' | '=' | '~') {
            out.push(c);
        }
        rest = &rest[c.len_utf8()..];
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `s` cut to [`MAX_NAME`] characters at a word; true when it was cut.
fn cut_name(s: &str) -> (String, bool) {
    if s.chars().count() <= MAX_NAME {
        return (s.to_owned(), false);
    }
    let head: String = s.chars().take(MAX_NAME).collect();
    let cut = head
        .rfind(' ')
        .map_or(head.as_str(), |i| &head[..i])
        .trim_end();
    (cut.to_owned(), true)
}

/// A file card's file in words: a note by its name, another file by its
/// file name; and what kind of file it is.
fn file_words(file: &str) -> (String, &'static str) {
    let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
    let lower = name.to_ascii_lowercase();
    let ext = lower.rsplit_once('.').map_or("", |(_, e)| e);
    match ext {
        "md" => (name[..name.len() - 3].to_owned(), "note"),
        "canvas" => (name[..name.len() - 7].to_owned(), "canvas"),
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "avif" => {
            (name.to_owned(), "picture")
        }
        "pdf" => (name.to_owned(), "PDF"),
        _ => (name.to_owned(), "file"),
    }
}

/// A web address without its scheme or a trailing slash.
fn url_words(url: &str) -> String {
    let t = url.trim();
    let t = t.split_once("://").map_or(
        t,
        |(scheme, rest)| if scheme.contains('/') { t } else { rest },
    );
    t.trim_end_matches('/').to_owned()
}

/// A node's name: what a connection to it calls it.
fn name(n: &Node) -> String {
    match &n.kind {
        Kind::Text(t) => {
            let first = t.lines().map(plain_line).find(|l| !l.is_empty());
            first.map_or_else(|| "Empty card".to_owned(), |l| cut_name(&l).0)
        }
        Kind::File { file, subpath } => {
            let (words, _) = file_words(file);
            let sub = subpath.trim().trim_start_matches('#');
            match sub.strip_prefix('^') {
                _ if sub.is_empty() => words,
                Some(block) => format!("{words}, block {block}"),
                None => format!("{words}, {sub}"),
            }
        }
        Kind::Link(url) => url_words(url),
        Kind::Group(Some(label)) => label.trim().to_owned(),
        Kind::Group(None) => "Unnamed group".to_owned(),
        Kind::Other(kind) => format!("Card of type {kind}"),
    }
}

fn marker(kind: MarkerKind) -> Marker {
    Marker::new(kind, CharRange::empty(0))
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The reading order of `items`: rows top to bottom, left to right within
/// a row (see the module documentation).
fn reading_order(nodes: &[Node], items: &mut [usize]) {
    let key = |i: &usize| (nodes[*i].y, nodes[*i].x, *i);
    items.sort_by(|a, b| {
        key(a)
            .partial_cmp(&key(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut start = 0;
    while start < items.len() {
        let first = &nodes[items[start]];
        let middle = first.y + first.h.max(0.0) / 2.0;
        let mut end = start + 1;
        while end < items.len() && nodes[items[end]].y < middle {
            end += 1;
        }
        items[start..end].sort_by(|a, b| {
            (nodes[*a].x, nodes[*a].y, *a)
                .partial_cmp(&(nodes[*b].x, nodes[*b].y, *b))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        start = end;
    }
}

/// For each node, the group it belongs to: the smallest group holding its
/// center, larger than it (or as large and earlier in the file).
fn parents(nodes: &[Node]) -> Vec<Option<usize>> {
    let groups: Vec<usize> = (0..nodes.len()).filter(|&i| nodes[i].is_group()).collect();
    (0..nodes.len())
        .map(|i| {
            let n = &nodes[i];
            let c = n.center();
            groups
                .iter()
                .copied()
                .filter(|&g| g != i)
                .filter(|&g| {
                    let (ga, na) = (nodes[g].area(), n.area());
                    (ga > na || (ga == na && g < i)) && nodes[g].holds(c)
                })
                .min_by(|&a, &b| {
                    (nodes[a].area(), std::cmp::Reverse(a))
                        .partial_cmp(&(nodes[b].area(), std::cmp::Reverse(b)))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        })
        .collect()
}

/// Writes a canvas into a builder.
struct Writer<'a> {
    b: Builder,
    options: &'a LoadOptions,
    scratch: DocumentMeta,
}

impl Writer<'_> {
    /// A heading at `level` holding the node's name (a link for file and
    /// link cards), its kind, and its color.
    fn heading(&mut self, n: &Node, level: u8) {
        self.b.paragraph_break();
        let h = self.b.open(marker(MarkerKind::Heading).with_level(level));
        let link = match &n.kind {
            Kind::File { file, subpath } if !file.trim().is_empty() => {
                Some(format!("{}{}", file.trim(), subpath.trim()))
            }
            Kind::Link(url) if !url.trim().is_empty() => Some(url.trim().to_owned()),
            _ => None,
        };
        let words = name(n);
        match link {
            Some(target) => {
                let l = self.b.open(marker(MarkerKind::Link).with_reference(target));
                self.b.text(&words);
                self.b.close(l);
            }
            None => self.b.text(&words),
        }
        let kind = match &n.kind {
            Kind::File { file, .. } => Some(file_words(file).1),
            Kind::Link(_) => Some("web page"),
            Kind::Group(Some(_)) => Some("group"),
            _ => None,
        };
        if let Some(k) = kind {
            self.b.close_punct(",");
            self.b.text(k);
        }
        if let Some(c) = n.color {
            self.b.close_punct(",");
            self.b.text(c);
        }
        self.b.close(h);
        self.b.paragraph_break();
    }

    /// A text card's lines after its first, as Markdown under the card.
    fn body(&mut self, text: &str, level: u8) {
        // Skip blank lines and the first line, which the heading said; a
        // first line the heading cut is read whole below it.
        let mut offset = 0;
        let mut start = text.len();
        for line in text.split_inclusive('\n') {
            offset += line.len();
            let plain = plain_line(line);
            if !plain.is_empty() {
                start = if cut_name(&plain).1 { 0 } else { offset };
                break;
            }
        }
        let rest = &text[start..];
        if rest.trim().is_empty() {
            return;
        }
        let mark = self.b.closed_markers();
        let b = std::mem::take(&mut self.b);
        self.b = crate::markdown::convert_into(b, rest, self.options, &mut self.scratch);
        self.b.deepen_headings_from(mark, level);
        self.b.paragraph_break();
    }

    fn connections(
        &mut self,
        nodes: &[Node],
        ids: &HashMap<&str, usize>,
        order: &[usize],
        edges: &[Edge],
    ) {
        let place = |id: &str| -> Option<(usize, usize)> {
            let i = *ids.get(id)?;
            Some((order[i], i))
        };
        let mut lines: Vec<((usize, usize), String)> = Vec::new();
        for e in edges {
            let (Some((pf, f)), Some((pt, t))) = (place(&e.from), place(&e.to)) else {
                continue;
            };
            let (a, b) = (name(&nodes[f]), name(&nodes[t]));
            let mut line = match (e.from_arrow, e.to_arrow) {
                (false, true) => format!("From {a} to {b}"),
                (true, false) => format!("From {b} to {a}"),
                (true, true) => format!("Both ways between {a} and {b}"),
                (false, false) => format!("Between {a} and {b}"),
            };
            if let Some(label) = &e.label {
                line.push_str(": ");
                line.push_str(label.trim());
            }
            lines.push(((pf, pt), line));
        }
        if lines.is_empty() {
            return;
        }
        lines.sort_by_key(|(k, _)| *k);
        self.b.paragraph_break();
        let h = self.b.open(marker(MarkerKind::Heading).with_level(1));
        self.b.text("Connections");
        self.b.close(h);
        self.b.paragraph_break();
        let list = self.b.open(marker(MarkerKind::List).with_level(1));
        for (_, line) in lines {
            self.b.line_break();
            let item = self.b.open(marker(MarkerKind::ListItem).with_level(1));
            self.b.text(&line);
            self.b.close(item);
        }
        self.b.close(list);
        self.b.paragraph_break();
    }
}

/// Canonical text and markers for a parsed canvas, and whether a limit
/// cut it short.
fn convert(canvas: &Json, options: &LoadOptions) -> (String, Vec<Marker>, bool) {
    let mut cut = false;
    let mut nodes: Vec<Node> = match canvas.get("nodes") {
        Some(Json::Array(a)) => {
            cut |= a.len() > MAX_NODES;
            a.iter().take(MAX_NODES).filter_map(node).collect()
        }
        _ => Vec::new(),
    };
    // A node without an id is reached by no connection.
    nodes.iter_mut().enumerate().for_each(|(i, n)| {
        if n.id.is_empty() {
            n.id = format!("\u{0}{i}");
        }
    });
    let edges: Vec<Edge> = match canvas.get("edges") {
        Some(Json::Array(a)) => {
            cut |= a.len() > MAX_EDGES;
            a.iter().take(MAX_EDGES).filter_map(edge).collect()
        }
        _ => Vec::new(),
    };
    // Two nodes with one id: connections reach the first.
    let mut ids: HashMap<&str, usize> = HashMap::new();
    for (i, n) in nodes.iter().enumerate() {
        ids.entry(n.id.as_str()).or_insert(i);
    }
    let parent = parents(&nodes);
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    let mut top = Vec::new();
    for (i, p) in parent.iter().enumerate() {
        match p {
            Some(g) => children[*g].push(i),
            None => top.push(i),
        }
    }
    reading_order(&nodes, &mut top);
    for c in &mut children {
        reading_order(&nodes, c);
    }

    let mut w = Writer {
        b: Builder::new(),
        options,
        scratch: DocumentMeta::default(),
    };
    let groups = nodes.iter().filter(|n| n.is_group()).count();
    let cards = nodes.len() - groups;
    let linked = edges
        .iter()
        .filter(|e| ids.contains_key(e.from.as_str()) && ids.contains_key(e.to.as_str()))
        .count();
    if nodes.is_empty() {
        w.b.text("An empty canvas.");
    } else {
        w.b.text(&format!(
            "Canvas: {}, {}, {}.",
            plural(cards, "card", "cards"),
            plural(groups, "group", "groups"),
            plural(linked, "connection", "connections")
        ));
    }
    w.b.paragraph_break();

    // Depth first, without recursion: a hostile canvas may nest groups
    // thousands deep.
    let mut order = vec![usize::MAX; nodes.len()];
    let mut next = 0usize;
    let mut stack: Vec<(Vec<usize>, usize, u8)> = vec![(top, 0, 1)];
    while let Some((items, pos, level)) = stack.last_mut() {
        let Some(&i) = items.get(*pos) else {
            stack.pop();
            continue;
        };
        *pos += 1;
        let level = *level;
        order[i] = next;
        next += 1;
        let n = &nodes[i];
        w.heading(n, level);
        if let Kind::Text(t) = &n.kind {
            w.body(t, level);
        }
        if n.is_group() && !children[i].is_empty() {
            let kids = std::mem::take(&mut children[i]);
            stack.push((kids, 0, level.saturating_add(1).min(6)));
        }
    }
    w.connections(&nodes, &ids, &order, &edges);
    let (text, markers) = w.b.finish();
    (text, markers, cut)
}

/// Loads JSON Canvas files (`.canvas`).
#[derive(Clone, Copy, Debug, Default)]
pub struct CanvasLoader;

impl Loader for CanvasLoader {
    fn id(&self) -> &'static str {
        "canvas"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["canvas"]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let decoded = text_of(source)?;
        let canvas =
            parse(&decoded.text).map_err(|e| LoadError::Parse(format!("not a canvas: {e}")))?;
        if !matches!(canvas, Json::Object(_)) {
            return Err(LoadError::Parse(
                "not a canvas: it is not a JSON object".into(),
            ));
        }
        let mut meta = meta_for(source, self.id());
        note_encoding(&mut meta, &decoded);
        meta.title = match source {
            Source::Path(p) => p
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .filter(|s| !s.is_empty()),
            _ => None,
        };
        let (text, markers, cut) = convert(&canvas, options);
        if cut {
            add_warning(&mut meta, CUT_WARNING);
        }
        Ok(Document::new(meta, Rope::from_str(&text), markers))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(src: &str) -> Document {
        CanvasLoader
            .load(
                &Source::Bytes {
                    data: src.as_bytes().to_vec(),
                    hint: "canvas".into(),
                },
                &LoadOptions::default(),
            )
            .unwrap()
    }

    fn kinds(d: &Document, kind: MarkerKind) -> Vec<(u8, String)> {
        let mut ms: Vec<&Marker> = d.markers().iter().filter(|m| m.kind == kind).collect();
        ms.sort_by_key(|m| m.range.start);
        ms.iter().map(|m| (m.level, d.slice(m.range))).collect()
    }

    fn text_node(id: &str, x: i32, y: i32, w: i32, h: i32, text: &str) -> String {
        format!(
            r#"{{"id":"{id}","type":"text","x":{x},"y":{y},"width":{w},"height":{h},"text":{}}}"#,
            json_string(text)
        )
    }

    fn json_string(s: &str) -> String {
        let mut out = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }

    fn canvas(nodes: &[String], edges: &[&str]) -> String {
        format!(
            r#"{{"nodes":[{}],"edges":[{}]}}"#,
            nodes.join(","),
            edges.join(",")
        )
    }

    #[test]
    fn reading_order_is_rows_then_left_to_right() {
        // Two rows: B is a little lower than A but above A's middle, so
        // it shares A's row; C is in the next row, left of both.
        let src = canvas(
            &[
                text_node("c", -500, 300, 200, 100, "Third"),
                text_node("b", 400, 20, 200, 100, "Second"),
                text_node("a", 0, 0, 200, 100, "First"),
            ],
            &[],
        );
        let d = load(&src);
        let h: Vec<String> = kinds(&d, MarkerKind::Heading)
            .into_iter()
            .map(|(_, t)| t)
            .collect();
        assert_eq!(h, ["First", "Second", "Third"]);
        assert!(
            d.text()
                .to_string()
                .starts_with("Canvas: 3 cards, 0 groups, 0 connections.")
        );
    }

    #[test]
    fn groups_hold_their_nodes_by_geometry_and_nest() {
        let src = canvas(
            &[
                r#"{"id":"g1","type":"group","x":0,"y":0,"width":1000,"height":1000,"label":"Week 1","color":"1"}"#.into(),
                r#"{"id":"g2","type":"group","x":500,"y":0,"width":400,"height":400,"label":"Lab"}"#.into(),
                text_node("in", 50, 50, 100, 100, "Inside"),
                text_node("lab", 600, 50, 100, 100, "Titration"),
                text_node("out", 2000, 0, 100, 100, "Outside"),
                r#"{"id":"g3","type":"group","x":3000,"y":0,"width":100,"height":100}"#.into(),
            ],
            &[],
        );
        let d = load(&src);
        let h = kinds(&d, MarkerKind::Heading);
        assert_eq!(
            h,
            [
                (1, "Week 1, group, red".to_owned()),
                (2, "Inside".to_owned()),
                (2, "Lab, group".to_owned()),
                (3, "Titration".to_owned()),
                (1, "Outside".to_owned()),
                (1, "Unnamed group".to_owned()),
            ]
        );
    }

    #[test]
    fn text_cards_read_as_markdown_under_their_heading() {
        let src = canvas(
            &[text_node(
                "a",
                0,
                0,
                300,
                300,
                "# **Big** idea\n\nSome *words* and [[Note|a link]].\n\n## Detail\n\n- one\n- two",
            )],
            &[],
        );
        let d = load(&src);
        let h = kinds(&d, MarkerKind::Heading);
        assert_eq!(h, [(1, "Big idea".to_owned()), (3, "Detail".to_owned())]);
        let t = d.text().to_string();
        assert!(t.contains("Some words and a link."), "{t}");
        assert_eq!(t.matches("Big idea").count(), 1, "{t}");
        assert_eq!(kinds(&d, MarkerKind::ListItem).len(), 2);
        let links: Vec<&Marker> = d
            .markers()
            .iter()
            .filter(|m| m.kind == MarkerKind::Link)
            .collect();
        assert_eq!(links[0].reference.as_deref(), Some("Note"));
    }

    #[test]
    fn a_long_first_line_is_cut_in_the_heading_and_read_in_full() {
        let long = "word ".repeat(40);
        let d = load(&canvas(&[text_node("a", 0, 0, 10, 10, long.trim())], &[]));
        let h = &kinds(&d, MarkerKind::Heading)[0].1;
        assert!(h.chars().count() <= MAX_NAME, "{h}");
        assert_eq!(
            d.text().to_string().matches("word").count(),
            40 + h.matches("word").count()
        );
    }

    #[test]
    fn file_and_link_cards_are_links() {
        let src = canvas(
            &[
                r##"{"id":"f","type":"file","x":0,"y":0,"width":10,"height":10,"file":"Projects/Mitochondria.md","subpath":"#Energy"}"##.into(),
                r##"{"id":"p","type":"file","x":20,"y":0,"width":10,"height":10,"file":"pics/cell diagram.png","color":"#ff0000"}"##.into(),
                r#"{"id":"l","type":"link","x":40,"y":0,"width":10,"height":10,"url":"https://example.com/page/"}"#.into(),
            ],
            &[],
        );
        let d = load(&src);
        let h: Vec<String> = kinds(&d, MarkerKind::Heading)
            .into_iter()
            .map(|(_, t)| t)
            .collect();
        assert_eq!(
            h,
            [
                "Mitochondria, Energy, note",
                "cell diagram.png, picture",
                "example.com/page, web page"
            ]
        );
        let refs: Vec<String> = d
            .markers()
            .iter()
            .filter(|m| m.kind == MarkerKind::Link)
            .filter_map(|m| m.reference.clone())
            .collect();
        assert_eq!(
            refs,
            [
                "Projects/Mitochondria.md#Energy",
                "pics/cell diagram.png",
                "https://example.com/page/"
            ]
        );
    }

    #[test]
    fn file_cards_follow_through_the_vault_resolver() {
        // A vault: the canvas in one folder, the notes in others. A file
        // card names its file from the vault root, or by name.
        let tmp = tempfile::tempdir().unwrap();
        let vault = tmp.path().join("vault");
        for dir in [".obsidian", "Boards", "Notes", "Projects"] {
            std::fs::create_dir_all(vault.join(dir)).unwrap();
        }
        std::fs::write(vault.join("Notes/Cell.md"), "# Cell\n").unwrap();
        std::fs::write(vault.join("Projects/Plan.md"), "# Plan\n").unwrap();
        let board = vault.join("Boards/board.canvas");
        std::fs::write(
            &board,
            canvas(
                &[
                    r##"{"id":"a","type":"file","x":0,"y":0,"width":10,"height":10,"file":"Notes/Cell.md","subpath":"#Cell"}"##.into(),
                    r#"{"id":"b","type":"file","x":20,"y":0,"width":10,"height":10,"file":"Plan.md"}"#.into(),
                ],
                &[],
            ),
        )
        .unwrap();
        let d = CanvasLoader
            .load(&Source::Path(board.clone()), &LoadOptions::default())
            .unwrap();
        assert_eq!(d.meta.title.as_deref(), Some("board"));
        let folder = board.parent().unwrap();
        let scope = textweaver_vault::LinkScope::for_folder(folder);
        let files = scope.files();
        let found: Vec<String> = d
            .markers()
            .iter()
            .filter(|m| m.kind == MarkerKind::Link)
            .filter_map(|m| m.reference.clone())
            .map(|r| {
                let file = r.split_once('#').map_or(r.as_str(), |(f, _)| f);
                let hit = textweaver_vault::resolve(&scope, &files, folder, file)
                    .unwrap_or_else(|| panic!("{file} did not resolve"));
                hit.path.file_name().unwrap().to_string_lossy().into_owned()
            })
            .collect();
        assert_eq!(found, ["Cell.md", "Plan.md"]);
    }

    #[test]
    fn connections_are_worded_by_their_arrows() {
        let src = canvas(
            &[
                text_node("a", 0, 0, 10, 10, "Alpha"),
                text_node("b", 100, 0, 10, 10, "Beta"),
                text_node("c", 0, 100, 10, 10, "Gamma"),
            ],
            &[
                r#"{"id":"1","fromNode":"c","toNode":"a"}"#,
                r#"{"id":"2","fromNode":"a","toNode":"b","label":"leads to"}"#,
                r#"{"id":"3","fromNode":"b","toNode":"c","fromEnd":"arrow"}"#,
                r#"{"id":"4","fromNode":"a","toNode":"c","toEnd":"none"}"#,
                r#"{"id":"5","fromNode":"b","toNode":"a","fromEnd":"arrow","toEnd":"none"}"#,
                r#"{"id":"6","fromNode":"a","toNode":"ghost"}"#,
            ],
        );
        let d = load(&src);
        let items: Vec<String> = kinds(&d, MarkerKind::ListItem)
            .into_iter()
            .map(|(_, t)| t)
            .collect();
        assert_eq!(
            items,
            [
                "From Alpha to Beta: leads to",
                "Between Alpha and Gamma",
                "From Alpha to Beta",
                "Both ways between Beta and Gamma",
                "From Gamma to Alpha",
            ]
        );
        assert!(
            d.text()
                .to_string()
                .contains("3 cards, 0 groups, 5 connections.")
        );
        assert_eq!(
            kinds(&d, MarkerKind::Heading).last().unwrap().1,
            "Connections"
        );
    }

    #[test]
    fn empty_odd_and_hostile_canvases() {
        assert_eq!(load("{}").text().to_string(), "An empty canvas.");
        let d = load(&canvas(
            &[
                r#"{"id":"x","type":"sticker","x":"a","y":null}"#.into(),
                text_node("e", 0, 100, 10, 10, "  \n"),
                "7".into(),
            ],
            &[r#"{"fromNode":5}"#],
        ));
        let h: Vec<String> = kinds(&d, MarkerKind::Heading)
            .into_iter()
            .map(|(_, t)| t)
            .collect();
        assert_eq!(h, ["Card of type sticker", "Empty card"]);
        let bad = CanvasLoader.load(
            &Source::Bytes {
                data: b"[1, 2]".to_vec(),
                hint: "canvas".into(),
            },
            &LoadOptions::default(),
        );
        assert!(matches!(bad, Err(LoadError::Parse(_))));
        let broken = CanvasLoader.load(
            &Source::Bytes {
                data: b"{\"nodes\": [".to_vec(),
                hint: "canvas".into(),
            },
            &LoadOptions::default(),
        );
        assert!(matches!(broken, Err(LoadError::Parse(_))));
        // Groups nested far deeper than headings go: no recursion, level 6.
        let deep: Vec<String> = (0..3000)
            .map(|i| {
                format!(
                    r#"{{"id":"g{i}","type":"group","x":{i},"y":{i},"width":{w},"height":{w}}}"#,
                    w = 10_000 - 2 * i
                )
            })
            .collect();
        let d = load(&canvas(&deep, &[]));
        assert_eq!(kinds(&d, MarkerKind::Heading).last().unwrap().0, 6);
        // Past the node limit, the rest is left out and said.
        let many: Vec<String> = (0..MAX_NODES + 5)
            .map(|i| text_node(&format!("n{i}"), 0, i as i32, 1, 1, "x"))
            .collect();
        let d = load(&canvas(&many, &[]));
        assert_eq!(crate::warnings(&d.meta), [CUT_WARNING]);
    }
}
