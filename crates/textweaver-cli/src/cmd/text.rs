//! `tw text`. Owner: Agent A.
//!
//! Prints a document's canonical text: headings, list items, and table rows
//! on lines of their own, paragraphs separated by a blank line.
//!
//! - `--format text` (default): the canonical text; with `--structure`,
//!   followed by an outline of every marker (`line:column`, kind, text).
//! - `--format markdown`: the document as Markdown (any format: a PDF's
//!   recovered headings, lists, and tables become Markdown).
//! - `--format html`: the document as a standalone, accessible HTML page.
//! - `--format json`: `{ "meta", "text" }`, plus `"markers"` with
//!   `--structure` (each with `kind`, `start`, `end`, `line`, `column`,
//!   `level`, `label`, `reference`, and `text`).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde_json::{Value, json};
use textweaver_app::formats;
use textweaver_app::text::{Document, Marker};

/// Arguments for `tw text`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Document to extract.
    pub file: PathBuf,
    /// Output format.
    #[arg(long, value_parser = ["text", "markdown", "html", "json"], default_value = "text")]
    pub format: String,
    /// Include markers (structure) in the output.
    #[arg(long)]
    pub structure: bool,
}

/// Runs `tw text`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let doc = load_document(&args.file)?;
    print!("{}", render(&doc, &args.format, args.structure)?);
    Ok(())
}

/// Loads a document with the built-in loaders, for the read-only commands.
pub(crate) fn load_document(path: &Path) -> anyhow::Result<Document> {
    formats::load_path(path).with_context(|| format!("cannot open {}", path.display()))
}

/// The output of `tw text` for `doc`, ending with a newline.
pub(crate) fn render(doc: &Document, format: &str, structure: bool) -> anyhow::Result<String> {
    let mut out = match format {
        "markdown" => formats::to_markdown(doc),
        "html" => formats::to_html(
            doc,
            &formats::HtmlOptions {
                standalone: true,
                ..formats::HtmlOptions::default()
            },
        ),
        "json" => {
            // Paths that are not UTF-8 would fail to serialize; show them lossily.
            let mut plain = doc.meta.clone();
            plain.path = None;
            let mut meta = serde_json::to_value(&plain)?;
            meta["path"] = json!(doc.meta.path.as_ref().map(|p| p.display().to_string()));
            let mut v = json!({ "meta": meta, "text": doc.text().to_string() });
            if structure {
                v["markers"] =
                    Value::Array(doc.markers().iter().map(|m| marker_json(doc, m)).collect());
            }
            serde_json::to_string_pretty(&v)?
        }
        _ => {
            let mut s = doc.text().to_string();
            if structure {
                s.push_str("\n\nStructure:\n");
                s.push_str(&outline(doc));
            }
            s
        }
    };
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

/// One marker as JSON, with its 1-based line and column.
pub(crate) fn marker_json(doc: &Document, m: &Marker) -> Value {
    let (line, col) = doc.line_col(m.range.start);
    json!({
        "kind": m.kind,
        "start": m.range.start.0,
        "end": m.range.end.0,
        "line": line + 1,
        "column": col + 1,
        "level": m.level,
        "label": m.label,
        "reference": m.reference,
        "text": doc.slice(m.range),
    })
}

/// Shortens `s` to at most `max` chars on one line.
pub(crate) fn excerpt(s: &str, max: usize) -> String {
    let one_line = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= max {
        one_line
    } else {
        let mut t: String = one_line.chars().take(max.saturating_sub(1)).collect();
        t.push('\u{2026}');
        t
    }
}

/// One line per marker: `line:column  kind [level] [label] "text"`.
fn outline(doc: &Document) -> String {
    let mut s = String::new();
    for m in doc.markers() {
        let (line, col) = doc.line_col(m.range.start);
        let _ = write!(s, "{}:{}\t{}", line + 1, col + 1, m.kind.spoken_name());
        if m.level > 0 {
            let _ = write!(s, " {}", m.level);
        }
        if let Some(l) = &m.label {
            let _ = write!(s, " [{l}]");
        }
        let _ = write!(s, "\t\"{}\"", excerpt(&doc.slice(m.range), 60));
        if let Some(r) = &m.reference {
            let _ = write!(s, " -> {r}");
        }
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(name: &str) -> Document {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name);
        load_document(&path).unwrap()
    }

    #[test]
    fn text_puts_structure_on_its_own_lines() {
        let out = render(&sample("sample.md"), "text", false).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        for l in [
            "Sample Markdown Document",
            "Lists",
            "First bullet item",
            "Step three.",
            "Ada | Engineer | 98",
        ] {
            assert!(lines.contains(&l), "{l:?} not on its own line");
        }
        assert!(!out.contains("title: Sample"), "front matter is not text");
    }

    #[test]
    fn structure_outline_and_json() {
        let doc = sample("sample.md");
        let out = render(&doc, "text", true).unwrap();
        assert!(out.contains("1:1\theading 1\t\"Sample Markdown Document\""));
        let json: Value = serde_json::from_str(&render(&doc, "json", true).unwrap()).unwrap();
        assert_eq!(json["meta"]["title"], "Sample Markdown Document");
        assert_eq!(json["markers"][0]["kind"], "heading");
        assert_eq!(json["markers"][0]["line"], 1);
        let plain: Value = serde_json::from_str(&render(&doc, "json", false).unwrap()).unwrap();
        assert!(plain.get("markers").is_none());
    }

    #[test]
    fn markdown_round_trip_keeps_headings_and_tables() {
        let out = render(&sample("sample.html"), "markdown", false).unwrap();
        assert!(out.starts_with("# Sample HTML Document\n"));
        assert!(out.contains("| Name | Score |\n|---|---|\n| Ada | 98 |"));
        assert!(out.contains("1. Preheat the oven."));
    }

    #[test]
    fn every_new_format_prints() {
        for (name, needle) in [
            ("a/sample.epub", "Chapter One: Beginnings"),
            ("a/sample.docx", "Ada | Engineer and poet | 98"),
            ("a/single.pdf", "1.1 Background"),
            (
                "a/columns.pdf",
                "It ends with this sentence in the left column.",
            ),
        ] {
            let doc = sample(name);
            let out = render(&doc, "text", false).unwrap();
            assert!(out.lines().any(|l| l == needle), "{name}: {out}");
            let md = render(&doc, "markdown", false).unwrap();
            assert!(
                md.contains(needle.trim_start_matches(char::is_numeric)),
                "{name}"
            );
            let html = render(&doc, "html", false).unwrap();
            assert!(html.starts_with("<!DOCTYPE html>"));
        }
    }

    #[test]
    fn excerpts_are_one_line_and_bounded() {
        assert_eq!(excerpt("a\nb  c", 10), "a b c");
        assert_eq!(excerpt("abcdefghij", 5), "abcd\u{2026}");
    }
}
