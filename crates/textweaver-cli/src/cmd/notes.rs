//! `tw notes links FILE [--type T] [--json]`: a document's relations
//! between notes, the knowledge graph as a list (B1-g1). Each note with
//! links, then its links out ("supports: Chapter 3 note") and what links
//! to it ("cites this, from: Week 4 note"), from the library's saved
//! notes. Reads only; never writes state.
//!
//! `tw notes graph [--to FORMAT] [--out FILE]`: the whole library's graph
//! as JSON, DOT, GraphML, Mermaid, PlantUML, a CSV edge list or a
//! Markdown list (B1-g2, [`Graph`]).
//! Owner: B1-g1, B1-g2.

use std::path::{Path, PathBuf};

use serde::Serialize;
use textweaver_app::store::notes::{collapse, doc_key};
use textweaver_app::store::{
    Backlinks, DocKey, Graph, GraphFormat, Library, Note, NotedDoc, Paths, Relation, RelationType,
    StateStore,
};

/// Arguments for `tw notes`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// What to do.
    #[command(subcommand)]
    pub command: Command,
}

/// `tw notes` subcommands.
#[derive(clap::Subcommand, Debug)]
pub enum Command {
    /// List a document's links between notes: out, and what links here.
    Links(LinksArgs),
    /// Write the library's knowledge graph: every note with links.
    Graph(GraphArgs),
}

/// Arguments for `tw notes graph`.
#[derive(clap::Args, Debug)]
pub struct GraphArgs {
    /// The format: md (a Markdown list, the default), json, dot, graphml,
    /// mermaid, plantuml or csv. Without it, the extension of the output
    /// file decides.
    #[arg(long, value_name = "FORMAT")]
    pub to: Option<String>,
    /// The same as --to json.
    #[arg(long, conflicts_with = "to")]
    pub json: bool,
    /// Write to this file instead of the standard output.
    #[arg(short = 'o', long, value_name = "FILE")]
    pub out: Option<PathBuf>,
    /// Read the state under this directory (like `TEXTWEAVER_HOME`).
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
}

/// Arguments for `tw notes links`.
#[derive(clap::Args, Debug)]
pub struct LinksArgs {
    /// Document whose links to list.
    pub file: PathBuf,
    /// Only links of this type, such as supports or "see also".
    #[arg(long = "type", value_name = "TYPE")]
    pub rel_type: Option<String>,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
    /// Read the state under this directory (like `TEXTWEAVER_HOME`).
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
}

/// One link, out of or into a note.
#[derive(Debug, Serialize, PartialEq, Eq)]
struct Link {
    /// The stored type, `SUPPORTS`.
    #[serde(rename = "type")]
    rel_type: String,
    /// The type as it reads aloud, `supports`.
    spoken: String,
    /// The other note's document (empty: this document).
    doc: String,
    /// The other note's id.
    id: String,
    /// The other note's text, shortened, with its document's title when
    /// it is in another document; absent when the note was not found.
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    /// Why the link exists.
    #[serde(skip_serializing_if = "String::is_empty")]
    note: String,
}

/// A note with its links.
#[derive(Debug, Serialize, PartialEq, Eq)]
struct NoteLinks {
    id: String,
    /// The note's text, shortened.
    label: String,
    out: Vec<Link>,
    #[serde(rename = "in")]
    incoming: Vec<Link>,
}

/// The whole report.
#[derive(Debug, Serialize, PartialEq, Eq)]
struct Report {
    document: String,
    notes: Vec<NoteLinks>,
}

/// A note as a short label.
fn label(n: &Note) -> String {
    let text = if n.note.trim().is_empty() {
        &n.anchor
    } else {
        &n.note
    };
    if text.trim().is_empty() {
        "Empty note".to_owned()
    } else {
        collapse(text, 60)
    }
}

/// The label of note `id` in document `doc` (empty: `here`).
fn find(here: &str, notes: &[Note], lib: &[NotedDoc], doc: &str, id: &str) -> Option<String> {
    if doc.is_empty() || doc_key(doc) == doc_key(here) {
        return notes.iter().find(|n| n.id == id).map(label);
    }
    let d = lib
        .iter()
        .find(|d| doc_key(&d.path.to_string_lossy()) == doc_key(doc))?;
    let n = d.notes.iter().find(|n| n.id == id)?;
    Some(format!("{}, in {}", label(n), d.title))
}

/// The report for `file`: its notes with links, filtered by `only`.
fn build(file: &Path, notes: &[Note], lib: &[NotedDoc], only: Option<RelationType>) -> Report {
    let abs = textweaver_app::store::library::resolve_path(file);
    let here = abs.to_string_lossy().into_owned();
    let mut docs: Vec<(String, &[Note])> = vec![(here.clone(), notes)];
    for d in lib {
        docs.push((d.path.to_string_lossy().into_owned(), d.notes.as_slice()));
    }
    let index = Backlinks::build(docs.iter().map(|(p, n)| (p.as_str(), *n)));
    let keep = |rel_type: &str| only.is_none_or(|t| RelationType::parse(rel_type) == Some(t));
    let spoken = |rel_type: &str| {
        Relation {
            rel_type: rel_type.to_owned(),
            ..Relation::default()
        }
        .spoken_type()
    };
    let mut out = Vec::new();
    for n in notes {
        let links_out: Vec<Link> = n
            .relations
            .iter()
            .filter(|r| keep(&r.rel_type))
            .map(|r| Link {
                rel_type: r.rel_type.clone(),
                spoken: r.spoken_type(),
                doc: r.target_doc.clone(),
                id: r.target_id.clone(),
                label: find(&here, notes, lib, &r.target_doc, &r.target_id),
                note: r.note.clone(),
            })
            .collect();
        let links_in: Vec<Link> = index
            .backlinks(&here, &n.id)
            .iter()
            .filter(|b| keep(&b.rel_type))
            .map(|b| Link {
                rel_type: b.rel_type.clone(),
                spoken: spoken(&b.rel_type),
                doc: if doc_key(&b.from_doc) == doc_key(&here) {
                    String::new()
                } else {
                    b.from_doc.clone()
                },
                id: b.from_id.clone(),
                label: find(&here, notes, lib, &b.from_doc, &b.from_id),
                note: b.note.clone(),
            })
            .collect();
        if links_out.is_empty() && links_in.is_empty() {
            continue;
        }
        out.push(NoteLinks {
            id: n.id.clone(),
            label: label(n),
            out: links_out,
            incoming: links_in,
        });
    }
    Report {
        document: here,
        notes: out,
    }
}

/// The report as lines, each with its meaning first: the note, then one
/// line per link, type first.
fn render(r: &Report) -> String {
    if r.notes.is_empty() {
        return "No links in this document.\n".to_owned();
    }
    let missing = "a note not found".to_owned();
    let mut lines: Vec<String> = Vec::new();
    for (i, n) in r.notes.iter().enumerate() {
        if i > 0 {
            lines.push(String::new());
        }
        lines.push(format!(
            "Note: {}. Links: {} out, {} in.",
            n.label,
            n.out.len(),
            n.incoming.len()
        ));
        for l in &n.out {
            let other = l.label.as_ref().unwrap_or(&missing);
            lines.push(format!("{}: {other}", l.spoken));
        }
        for l in &n.incoming {
            let other = l.label.as_ref().unwrap_or(&missing);
            lines.push(format!("{} this, from: {other}", l.spoken));
        }
    }
    lines.push(String::new());
    lines.join("\n")
}

/// Runs `tw notes`.
pub fn run(args: Args) -> anyhow::Result<()> {
    match args.command {
        Command::Links(a) => links(a),
        Command::Graph(a) => graph(a),
    }
}

/// The format `tw notes graph` writes: `--to` (or `--json`), else the
/// extension of `--out`, else the Markdown list.
fn graph_format(args: &GraphArgs) -> anyhow::Result<GraphFormat> {
    if args.json {
        return Ok(GraphFormat::Json);
    }
    if let Some(name) = &args.to {
        return GraphFormat::parse(name).ok_or_else(|| {
            let names: Vec<&str> = GraphFormat::ALL.iter().map(|f| f.as_str()).collect();
            anyhow::anyhow!(
                "Unknown graph format {name}. Use one of: {}.",
                names.join(", ")
            )
        });
    }
    Ok(args
        .out
        .as_deref()
        .and_then(GraphFormat::from_path)
        .unwrap_or(GraphFormat::Markdown))
}

fn graph(args: GraphArgs) -> anyhow::Result<()> {
    let format = graph_format(&args)?;
    let paths = match &args.home {
        Some(home) => Paths::under(home),
        None => Paths::platform()?,
    };
    let states = StateStore::new(paths.state_dir());
    let docs = Library::load(&paths.library_file())?.noted_documents(&states);
    let g = Graph::build(&docs);
    let text = g.render(format);
    match &args.out {
        Some(out) => {
            textweaver_app::store::atomic_write(out, text.as_bytes())
                .map_err(|e| anyhow::anyhow!("Could not write the knowledge graph: {e}"))?;
            crate::cmd::outln!(
                "Knowledge graph written: {} File: {}",
                g.counts(),
                out.display()
            );
        }
        None => crate::cmd::out!("{text}"),
    }
    Ok(())
}

fn links(args: LinksArgs) -> anyhow::Result<()> {
    let only = match &args.rel_type {
        Some(t) => Some(RelationType::parse(t).ok_or_else(|| {
            let names: Vec<&str> = RelationType::ALL.iter().map(|t| t.spoken()).collect();
            anyhow::anyhow!("Unknown link type {t}. Use one of: {}.", names.join(", "))
        })?),
        None => None,
    };
    let paths = match &args.home {
        Some(home) => Paths::under(home),
        None => Paths::platform()?,
    };
    let states = StateStore::new(paths.state_dir());
    let notes = states
        .load(&DocKey::for_path(&args.file))
        .map(|s| s.notes)
        .unwrap_or_default();
    let here = DocKey::for_path(&args.file).0;
    let lib: Vec<NotedDoc> = Library::load(&paths.library_file())
        .unwrap_or_default()
        .noted_documents(&states)
        .into_iter()
        .filter(|d| doc_key(&d.path.to_string_lossy()) != here)
        .collect();
    let report = build(&args.file, &notes, &lib, only);
    if args.json {
        crate::cmd::outln!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        crate::cmd::out!("{}", render(&report));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(id: &str, text: &str, rels: Vec<Relation>) -> Note {
        Note {
            id: id.into(),
            note: text.into(),
            relations: rels,
            ..Note::default()
        }
    }

    fn rel(t: &str, doc: &str, id: &str) -> Relation {
        Relation {
            rel_type: t.into(),
            target_doc: doc.into(),
            target_id: id.into(),
            note: String::new(),
        }
    }

    #[test]
    fn links_list_out_and_in_with_meaning_first() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("week3.md");
        let other = dir.path().join("week4.md");
        let notes = vec![
            note(
                "a",
                "Mitochondria make energy",
                vec![rel("SUPPORTS", "", "b")],
            ),
            note("b", "Chapter 3 note", vec![]),
            note("c", "No links", vec![]),
        ];
        let lib = vec![NotedDoc {
            path: other,
            title: "Pharmacology 2".into(),
            notes: vec![note(
                "x",
                "Week 4 note",
                vec![rel("CITES", &file.to_string_lossy(), "b")],
            )],
        }];
        let r = build(&file, &notes, &lib, None);
        assert_eq!(r.notes.len(), 2, "notes without links are left out");
        let text = render(&r);
        assert!(text.contains("supports: Chapter 3 note"), "{text}");
        assert!(
            text.contains("supports this, from: Mitochondria make energy"),
            "{text}"
        );
        assert!(
            text.contains("cites this, from: Week 4 note, in Pharmacology 2"),
            "{text}"
        );
        for line in text.lines().filter(|l| !l.is_empty()) {
            let head: String = line.chars().take(40).collect();
            assert!(head.contains(':'), "meaning in the first 40 cells: {line}");
        }
        let only = build(&file, &notes, &lib, Some(RelationType::Cites));
        assert_eq!(only.notes.len(), 1);
        assert_eq!(only.notes[0].incoming[0].spoken, "cites");
        let json = serde_json::to_value(&only).unwrap();
        assert_eq!(json["notes"][0]["in"][0]["type"], "CITES");
        assert_eq!(
            render(&build(&file, &[], &[], None)),
            "No links in this document.\n"
        );
    }

    #[test]
    fn graph_format_from_to_json_or_the_file_name() {
        let args = |to: Option<&str>, json: bool, out: Option<&str>| GraphArgs {
            to: to.map(Into::into),
            json,
            out: out.map(PathBuf::from),
            home: None,
        };
        let f = |a: GraphArgs| graph_format(&a).unwrap();
        assert_eq!(f(args(None, false, None)), GraphFormat::Markdown);
        assert_eq!(f(args(Some("dot"), false, Some("g.csv"))), GraphFormat::Dot);
        assert_eq!(f(args(None, true, None)), GraphFormat::Json);
        assert_eq!(
            f(args(None, false, Some("g.graphml"))),
            GraphFormat::GraphMl
        );
        assert_eq!(f(args(None, false, Some("g.txt"))), GraphFormat::Markdown);
        assert!(graph_format(&args(Some("svg"), false, None)).is_err());
    }

    #[test]
    fn graph_writes_the_library_to_a_file() {
        let home = tempfile::tempdir().unwrap();
        let paths = Paths::under(home.path());
        let a = home.path().join("a.md");
        let mut lib = Library::default();
        lib.record_open(&a, "Alpha", "markdown");
        lib.save(&paths.library_file()).unwrap();
        let st = textweaver_app::store::DocState {
            notes: vec![
                note("x", "First", vec![rel("SUPPORTS", "", "y")]),
                note("y", "Second", vec![]),
            ],
            ..Default::default()
        };
        StateStore::new(paths.state_dir())
            .save(&DocKey::for_path(&a), &st)
            .unwrap();
        let out = home.path().join("graph.csv");
        graph(GraphArgs {
            to: None,
            json: false,
            out: Some(out.clone()),
            home: Some(home.path().to_owned()),
        })
        .unwrap();
        let text = std::fs::read_to_string(&out).unwrap();
        assert_eq!(
            text,
            "source,type,target\r\n\"First, in Alpha\",supports,\"Second, in Alpha\"\r\n"
        );
    }
}
