//! The library's knowledge graph as a file (B1-g2): every note with a
//! link is a node, every relation between notes an edge, written as JSON
//! (`{nodes, edges}`, star's shape, for Gephi and Cytoscape), DOT,
//! GraphML, Mermaid, PlantUML, a CSV edge list, or a Markdown list.
//!
//! The Markdown list is the text equivalent of every drawn format: one
//! heading per note, then one line per link, its type first ("supports:
//! Chapter 3 note, in Biology"), the links out before the links in. A
//! relation whose target note is not in the library still appears, to a
//! node labeled "Note not found", so no link is lost from an export.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::Serialize;

use crate::library::NotedDoc;
use crate::notes::{Note, Relation, collapse, doc_key};

/// Longest node label, in characters.
const LABEL_CHARS: usize = 60;

/// The label of a node whose note is not in the library.
const MISSING_LABEL: &str = "Note not found";

/// A file format the graph can be written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GraphFormat {
    /// A Markdown list: one heading per note, one line per link.
    Markdown,
    /// JSON, `{nodes, edges}`.
    Json,
    /// Graphviz DOT.
    Dot,
    /// GraphML, the XML graph format Gephi, Cytoscape and yEd read.
    GraphMl,
    /// A Mermaid flowchart.
    Mermaid,
    /// A PlantUML diagram.
    PlantUml,
    /// A CSV edge list: source, type, target, with a header row.
    Csv,
}

impl GraphFormat {
    /// Every format, the Markdown list (the text form) first.
    pub const ALL: [GraphFormat; 7] = [
        GraphFormat::Markdown,
        GraphFormat::Json,
        GraphFormat::Dot,
        GraphFormat::GraphMl,
        GraphFormat::Mermaid,
        GraphFormat::PlantUml,
        GraphFormat::Csv,
    ];

    /// The name `tw notes graph --to` takes: `md`, `json`, `dot`,
    /// `graphml`, `mermaid`, `plantuml` or `csv`.
    pub fn as_str(self) -> &'static str {
        match self {
            GraphFormat::Markdown => "md",
            GraphFormat::Json => "json",
            GraphFormat::Dot => "dot",
            GraphFormat::GraphMl => "graphml",
            GraphFormat::Mermaid => "mermaid",
            GraphFormat::PlantUml => "plantuml",
            GraphFormat::Csv => "csv",
        }
    }

    /// The file extension, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            GraphFormat::Mermaid => "mmd",
            GraphFormat::PlantUml => "puml",
            f => f.as_str(),
        }
    }

    /// Parses a format name or extension, ignoring case: `md` or
    /// `markdown`, `json`, `dot` or `gv`, `graphml`, `mermaid` or `mmd`,
    /// `plantuml` or `puml`, `csv`.
    pub fn parse(name: &str) -> Option<Self> {
        let name = name.trim().trim_start_matches('.').to_ascii_lowercase();
        match name.as_str() {
            "markdown" => Some(GraphFormat::Markdown),
            "gv" => Some(GraphFormat::Dot),
            _ => GraphFormat::ALL
                .into_iter()
                .find(|f| f.as_str() == name || f.extension() == name),
        }
    }

    /// The format a file's extension names, if any.
    pub fn from_path(path: &Path) -> Option<Self> {
        GraphFormat::parse(&path.extension()?.to_string_lossy())
    }
}

/// A note in the graph.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GraphNode {
    /// The node's id: the note's id, with `-2`, `-3` added when two
    /// documents hold notes with the same id.
    pub id: String,
    /// The note's document, as its path.
    pub doc: String,
    /// The document's title (its file name for a note not found).
    pub title: String,
    /// The note's text, shortened (its passage when it has no text).
    pub label: String,
    /// True for the target of a relation whose note is not in the library.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub missing: bool,
}

/// A relation in the graph, from node `src` to node `dst`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GraphEdge {
    /// The linking note's node id.
    pub src: String,
    /// The linked note's node id.
    pub dst: String,
    /// The type as stored (`SUPPORTS`), kept when textweaver does not
    /// know it.
    pub rel_type: String,
    /// The type as it reads aloud (`supports`).
    pub spoken: String,
    /// Why the link exists.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// The knowledge graph of a set of documents ([`Graph::build`]).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Graph {
    /// The notes with links, in document and note order, then the notes
    /// not found.
    pub nodes: Vec<GraphNode>,
    /// The relations, in document and note order.
    pub edges: Vec<GraphEdge>,
}

/// A note as a short label: its text, else its passage.
fn label(n: &Note) -> String {
    let text = if n.note.trim().is_empty() {
        &n.anchor
    } else {
        &n.note
    };
    if text.trim().is_empty() {
        "Empty note".to_owned()
    } else {
        collapse(text, LABEL_CHARS)
    }
}

/// The document a relation of a note in `from` points into.
fn target_doc<'a>(from: &'a str, r: &'a Relation) -> &'a str {
    if r.target_doc.is_empty() {
        from
    } else {
        &r.target_doc
    }
}

/// Nodes being added, each id kept unique.
#[derive(Default)]
struct Builder {
    graph: Graph,
    index: HashMap<(String, String), usize>,
    used: HashSet<String>,
}

impl Builder {
    /// Adds `node` under `key` (document key, note id); returns its index.
    fn add(&mut self, key: (String, String), mut node: GraphNode) -> usize {
        let base = if node.id.is_empty() {
            "note".to_owned()
        } else {
            node.id.clone()
        };
        let mut id = base.clone();
        let mut n = 2;
        while !self.used.insert(id.clone()) {
            id = format!("{base}-{n}");
            n += 1;
        }
        node.id = id;
        let i = self.graph.nodes.len();
        self.index.insert(key, i);
        self.graph.nodes.push(node);
        i
    }
}

impl Graph {
    /// The graph of `docs`' notes: each note with a link out or in is a
    /// node; each relation is an edge.
    pub fn build(docs: &[NotedDoc]) -> Graph {
        let paths: Vec<String> = docs
            .iter()
            .map(|d| d.path.to_string_lossy().into_owned())
            .collect();
        let keys: Vec<String> = paths.iter().map(|p| doc_key(p)).collect();
        let mut targeted: HashSet<(String, String)> = HashSet::new();
        for (d, from) in docs.iter().zip(&paths) {
            for r in d.notes.iter().flat_map(|n| &n.relations) {
                targeted.insert((doc_key(target_doc(from, r)), r.target_id.clone()));
            }
        }
        let mut b = Builder::default();
        for ((d, path), key) in docs.iter().zip(&paths).zip(&keys) {
            for n in &d.notes {
                let k = (key.clone(), n.id.clone());
                if b.index.contains_key(&k) || (n.relations.is_empty() && !targeted.contains(&k)) {
                    continue;
                }
                let node = GraphNode {
                    id: n.id.clone(),
                    doc: path.clone(),
                    title: d.title.clone(),
                    label: label(n),
                    missing: false,
                };
                b.add(k, node);
            }
        }
        for ((d, path), key) in docs.iter().zip(&paths).zip(&keys) {
            for n in &d.notes {
                let Some(&src) = b.index.get(&(key.clone(), n.id.clone())) else {
                    continue;
                };
                for r in &n.relations {
                    let doc = target_doc(path, r);
                    let k = (doc_key(doc), r.target_id.clone());
                    let dst = match b.index.get(&k) {
                        Some(&i) => i,
                        None => {
                            let title = Path::new(doc)
                                .file_name()
                                .map(|f| f.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            let node = GraphNode {
                                id: r.target_id.clone(),
                                doc: doc.to_owned(),
                                title,
                                label: MISSING_LABEL.to_owned(),
                                missing: true,
                            };
                            b.add(k, node)
                        }
                    };
                    let edge = GraphEdge {
                        src: b.graph.nodes[src].id.clone(),
                        dst: b.graph.nodes[dst].id.clone(),
                        rel_type: r.rel_type.clone(),
                        spoken: r.spoken_type(),
                        note: r.note.clone(),
                    };
                    b.graph.edges.push(edge);
                }
            }
        }
        b.graph
    }

    /// The number of notes in the graph (the nodes found).
    pub fn note_count(&self) -> usize {
        self.nodes.iter().filter(|n| !n.missing).count()
    }

    /// The graph written in `format`.
    pub fn render(&self, format: GraphFormat) -> String {
        match format {
            GraphFormat::Markdown => self.markdown(),
            GraphFormat::Json => {
                let mut s = serde_json::to_string_pretty(self).unwrap_or_default();
                s.push('\n');
                s
            }
            GraphFormat::Dot => self.dot(),
            GraphFormat::GraphMl => self.graphml(),
            GraphFormat::Mermaid => self.mermaid(),
            GraphFormat::PlantUml => self.plantuml(),
            GraphFormat::Csv => self.csv(),
        }
    }

    /// "3 notes, 2 links."
    pub fn counts(&self) -> String {
        let plural = |n: usize, w: &str| {
            if n == 1 {
                format!("1 {w}")
            } else {
                format!("{n} {w}s")
            }
        };
        format!(
            "{}, {}.",
            plural(self.note_count(), "note"),
            plural(self.edges.len(), "link")
        )
    }

    /// The node with id `id`.
    fn node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// A node as it reads in a list: "label, in Title".
    fn named(&self, id: &str) -> String {
        match self.node(id) {
            Some(n) if n.title.is_empty() => n.label.clone(),
            Some(n) => format!("{}, in {}", n.label, n.title),
            None => MISSING_LABEL.to_owned(),
        }
    }

    /// The position-based id of a node in the drawn formats (`n1`), safe
    /// in every syntax whatever the note's id holds.
    fn short_id(&self, id: &str) -> String {
        let i = self.nodes.iter().position(|n| n.id == id).unwrap_or(0);
        format!("n{}", i + 1)
    }

    // shortcut: the Markdown list's words are English, as `tw notes links`
    // prints them; take a catalog here when the exports are localized.
    fn markdown(&self) -> String {
        let mut out = format!("# Knowledge graph\n\n{}\n", self.counts());
        for n in self.nodes.iter().filter(|n| !n.missing) {
            out.push_str(&format!("\n## {}\n\n", md_escape(&self.named(&n.id))));
            for e in self.edges.iter().filter(|e| e.src == n.id) {
                out.push_str(&format!(
                    "- {}: {}",
                    md_escape(&e.spoken),
                    md_escape(&self.named(&e.dst))
                ));
                push_note(&mut out, &e.note);
            }
            for e in self.edges.iter().filter(|e| e.dst == n.id) {
                out.push_str(&format!(
                    "- {} this, from: {}",
                    md_escape(&e.spoken),
                    md_escape(&self.named(&e.src))
                ));
                push_note(&mut out, &e.note);
            }
        }
        out
    }

    fn dot(&self) -> String {
        let mut out = String::from("digraph knowledge_graph {\n  node [shape=box];\n");
        for n in &self.nodes {
            out.push_str(&format!(
                "  {} [label=\"{}\", document=\"{}\"];\n",
                self.short_id(&n.id),
                dot_escape(&n.label),
                dot_escape(&n.title)
            ));
        }
        for e in &self.edges {
            out.push_str(&format!(
                "  {} -> {} [label=\"{}\"];\n",
                self.short_id(&e.src),
                self.short_id(&e.dst),
                dot_escape(&e.spoken)
            ));
        }
        out.push_str("}\n");
        out
    }

    fn graphml(&self) -> String {
        let mut out = String::from(concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<graphml xmlns=\"http://graphml.graphdrawing.org/xmlns\">\n",
            "  <key id=\"label\" for=\"node\" attr.name=\"label\" attr.type=\"string\"/>\n",
            "  <key id=\"doc\" for=\"node\" attr.name=\"doc\" attr.type=\"string\"/>\n",
            "  <key id=\"title\" for=\"node\" attr.name=\"title\" attr.type=\"string\"/>\n",
            "  <key id=\"type\" for=\"edge\" attr.name=\"type\" attr.type=\"string\"/>\n",
            "  <key id=\"spoken\" for=\"edge\" attr.name=\"spoken\" attr.type=\"string\"/>\n",
            "  <key id=\"note\" for=\"edge\" attr.name=\"note\" attr.type=\"string\"/>\n",
            "  <graph id=\"knowledge_graph\" edgedefault=\"directed\">\n",
        ));
        for n in &self.nodes {
            out.push_str(&format!("    <node id=\"{}\">\n", xml_escape(&n.id)));
            data(&mut out, "label", &n.label);
            data(&mut out, "doc", &n.doc);
            data(&mut out, "title", &n.title);
            out.push_str("    </node>\n");
        }
        for e in &self.edges {
            out.push_str(&format!(
                "    <edge source=\"{}\" target=\"{}\">\n",
                xml_escape(&e.src),
                xml_escape(&e.dst)
            ));
            data(&mut out, "type", &e.rel_type);
            data(&mut out, "spoken", &e.spoken);
            if !e.note.is_empty() {
                data(&mut out, "note", &e.note);
            }
            out.push_str("    </edge>\n");
        }
        out.push_str("  </graph>\n</graphml>\n");
        out
    }

    fn mermaid(&self) -> String {
        let mut out = format!(
            "flowchart LR\n  accTitle: Knowledge graph\n  accDescr: {} The Markdown list export is its text equivalent.\n",
            self.counts()
        );
        for n in &self.nodes {
            out.push_str(&format!(
                "  {}[\"{}\"]\n",
                self.short_id(&n.id),
                mermaid_escape(&n.label)
            ));
        }
        for e in &self.edges {
            out.push_str(&format!(
                "  {} -->|\"{}\"| {}\n",
                self.short_id(&e.src),
                mermaid_escape(&e.spoken),
                self.short_id(&e.dst)
            ));
        }
        out
    }

    fn plantuml(&self) -> String {
        let mut out = String::from("@startuml\ntitle Knowledge graph\n");
        for n in &self.nodes {
            out.push_str(&format!(
                "rectangle \"{}\" as {}\n",
                plantuml_escape(&n.label),
                self.short_id(&n.id)
            ));
        }
        for e in &self.edges {
            out.push_str(&format!(
                "{} --> {} : {}\n",
                self.short_id(&e.src),
                self.short_id(&e.dst),
                plantuml_escape(&e.spoken)
            ));
        }
        out.push_str("@enduml\n");
        out
    }

    fn csv(&self) -> String {
        let mut out = String::from("source,type,target\r\n");
        for e in &self.edges {
            out.push_str(&format!(
                "{},{},{}\r\n",
                csv_field(&self.named(&e.src)),
                csv_field(&e.spoken),
                csv_field(&self.named(&e.dst))
            ));
        }
        out
    }
}

/// One GraphML `<data>` line.
fn data(out: &mut String, key: &str, value: &str) {
    out.push_str(&format!(
        "      <data key=\"{key}\">{}</data>\n",
        xml_escape(value)
    ));
}

/// Ends a Markdown list line, with the link's reason after a dash.
fn push_note(out: &mut String, note: &str) {
    let note = collapse(note, 200);
    if !note.is_empty() {
        out.push_str(" - ");
        out.push_str(&md_escape(&note));
    }
    out.push('\n');
}

/// Text in a Markdown line: the characters that would turn it into markup
/// (emphasis, a link, code, an HTML tag) escaped with a backslash, so the
/// list reads as written when rendered.
fn md_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '\\' | '*' | '_' | '`' | '[' | ']' | '<') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Text inside a DOT quoted string: backslashes and quotes escaped, line
/// breaks as spaces.
fn dot_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' | '\r' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// Text inside an XML element or attribute: the five special characters
/// as entities, and the characters XML 1.0 forbids dropped.
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\u{27}' => out.push_str("&apos;"),
            '\t' | '\n' | '\r' => out.push(c),
            c if c < ' ' || c == '\u{fffe}' || c == '\u{ffff}' => {}
            c => out.push(c),
        }
    }
    out
}

/// Text inside a Mermaid quoted label: the characters Mermaid reads as
/// syntax written as its entity codes (`#quot;`).
fn mermaid_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '#' => out.push_str("#35;"),
            '"' => out.push_str("#quot;"),
            '<' => out.push_str("#lt;"),
            '>' => out.push_str("#gt;"),
            '&' => out.push_str("#amp;"),
            '`' => out.push_str("#96;"),
            '\n' | '\r' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// Text inside a PlantUML quoted name or after an arrow's colon: quotes
/// and backslashes (which start `\n`) as PlantUML's Unicode escapes.
fn plantuml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("<U+0022>"),
            '\\' => out.push_str("<U+005C>"),
            '\n' | '\r' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// A CSV field (RFC 4180): quoted when it holds a comma, quote or line
/// break, quotes doubled. A field a spreadsheet would run as a formula
/// (starting with `=`, `+`, `-`, `@` or a tab) gets a leading apostrophe,
/// so a note imported from another vault cannot run one.
fn csv_field(s: &str) -> String {
    let mut s = s.to_owned();
    if s.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        s.insert(0, '\u{27}');
    }
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn note(id: &str, text: &str, rels: Vec<Relation>) -> Note {
        Note {
            id: id.into(),
            note: text.into(),
            relations: rels,
            ..Note::default()
        }
    }

    fn rel(t: &str, doc: &str, id: &str, why: &str) -> Relation {
        Relation {
            rel_type: t.into(),
            target_doc: doc.into(),
            target_id: id.into(),
            note: why.into(),
        }
    }

    /// Two documents: labels with quotes, markup and commas, an unknown
    /// type, a note id used in both, and a link to a note not found.
    fn fixture() -> Vec<NotedDoc> {
        vec![
            NotedDoc {
                path: PathBuf::from("/library/biology.md"),
                title: "Biology".into(),
                notes: vec![
                    note(
                        "a1",
                        "Mitochondria \"make\" energy",
                        vec![
                            rel("SUPPORTS", "", "a2", "same chapter"),
                            rel("CITES", "/library/chem.md", "a1", ""),
                        ],
                    ),
                    note("a2", "Cells & <tissues>, #3 \\ more", vec![]),
                    note("a3", "No links", vec![]),
                ],
            },
            NotedDoc {
                path: PathBuf::from("/library/chem.md"),
                title: "Chemistry, 2nd ed.".into(),
                notes: vec![note(
                    "a1",
                    "=SUM(proteins)",
                    vec![
                        rel("LIKES", "/library/biology.md", "a2", ""),
                        rel("SEE_ALSO", "/library/gone.md", "zz", ""),
                    ],
                )],
            },
        ]
    }

    #[test]
    fn nodes_edges_and_ids() {
        let g = Graph::build(&fixture());
        let ids: Vec<&str> = g.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(
            ids,
            ["a1", "a2", "a1-2", "zz"],
            "a note without links is left out"
        );
        assert_eq!(g.note_count(), 3);
        assert!(g.nodes[3].missing);
        assert_eq!(g.nodes[3].title, "gone.md");
        assert_eq!(g.edges.len(), 4);
        assert_eq!(g.edges[1].dst, "a1-2");
        assert_eq!(g.edges[2].rel_type, "LIKES");
        assert_eq!(g.edges[2].spoken, "likes");
        assert_eq!(
            Graph::build(&[]).render(GraphFormat::Csv),
            "source,type,target\r\n"
        );
    }

    #[test]
    fn format_names() {
        for f in GraphFormat::ALL {
            assert_eq!(GraphFormat::parse(f.as_str()), Some(f));
            assert_eq!(GraphFormat::parse(f.extension()), Some(f));
        }
        assert_eq!(GraphFormat::parse("Markdown"), Some(GraphFormat::Markdown));
        assert_eq!(
            GraphFormat::from_path(Path::new("g.puml")),
            Some(GraphFormat::PlantUml)
        );
        assert_eq!(GraphFormat::parse("svg"), None);
    }

    #[test]
    fn markdown_lines_put_the_meaning_first() {
        let text = Graph::build(&fixture()).render(GraphFormat::Markdown);
        for line in text.lines().filter(|l| l.starts_with("- ")) {
            let head: String = line.chars().take(40).collect();
            assert!(head.contains(':'), "type first: {line}");
        }
    }

    /// One golden file per format in `fixtures/graph` (set `TW_BLESS=1` to
    /// write them again, then read the difference before committing).
    #[test]
    fn every_format_matches_its_golden_file() {
        let g = Graph::build(&fixture());
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/graph");
        for f in GraphFormat::ALL {
            let path = dir.join(format!("graph.{}", f.extension()));
            let text = g.render(f);
            if std::env::var_os("TW_BLESS").is_some() {
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(&path, &text).unwrap();
                continue;
            }
            let want = std::fs::read_to_string(&path).unwrap();
            assert_eq!(
                text,
                want,
                "{} differs from its golden file",
                path.display()
            );
        }
    }
}
