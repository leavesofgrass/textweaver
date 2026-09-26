//! `tw info`. Owner: Agent A.
//!
//! Facts about a document: format, title, author, language, size in chars,
//! words, sentences, lines, and paragraphs, structure counts (pages and
//! sections of paginated and chaptered sources included), and an estimated
//! reading time at Star's default rate of 265 words per minute.

use std::path::PathBuf;

use serde_json::{Value, json};
use textweaver_app::text::core::{MarkerKind, Unit};
use textweaver_app::text::{Document, units};

use super::text::load_document;

/// Arguments for `tw info`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Document to describe.
    pub file: PathBuf,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
}

/// Star's default reading rate, in words per minute.
const DEFAULT_WPM: usize = 265;

/// Runs `tw info`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let doc = load_document(&args.file)?;
    let facts = facts(&doc, &args.file);
    if args.json {
        println!("{}", serde_json::to_string_pretty(&facts)?);
    } else {
        print!("{}", describe(&facts));
    }
    Ok(())
}

/// Structure counts reported, as (JSON key, marker kind, block level filter).
const COUNTS: &[(&str, MarkerKind, Option<u8>)] = &[
    ("headings", MarkerKind::Heading, None),
    ("lists", MarkerKind::List, None),
    ("list_items", MarkerKind::ListItem, None),
    ("tables", MarkerKind::Table, None),
    ("table_rows", MarkerKind::TableRow, None),
    ("links", MarkerKind::Link, None),
    ("images", MarkerKind::Image, None),
    ("code_blocks", MarkerKind::Code, Some(1)),
    ("block_quotes", MarkerKind::Quote, None),
    ("footnotes", MarkerKind::Footnote, Some(1)),
    ("pages", MarkerKind::PageBreak, None),
    ("sections", MarkerKind::SectionBreak, None),
];

/// Everything `tw info` reports, as JSON.
pub(crate) fn facts(doc: &Document, file: &std::path::Path) -> Value {
    let words = units::segments(doc, Unit::Word).len();
    let index = doc.marker_index();
    let mut structure = serde_json::Map::new();
    for &(key, kind, level) in COUNTS {
        structure.insert(key.to_owned(), json!(index.count(kind, level)));
    }
    let mut headings_by_level = serde_json::Map::new();
    for level in 1..=6u8 {
        let n = index.count(MarkerKind::Heading, Some(level));
        if n > 0 {
            headings_by_level.insert(format!("h{level}"), json!(n));
        }
    }
    json!({
        "file": file.display().to_string(),
        "format": doc.meta.format,
        "title": doc.meta.title,
        "author": doc.meta.author,
        "language": doc.meta.language,
        "properties": doc.meta.properties,
        "chars": doc.len_chars(),
        "words": words,
        "sentences": units::segments(doc, Unit::Sentence).len(),
        "lines": doc.line_count(),
        "paragraphs": units::segments(doc, Unit::Paragraph).len(),
        "structure": structure,
        "headings_by_level": headings_by_level,
        "reading_minutes": words.div_ceil(DEFAULT_WPM),
        "reading_wpm": DEFAULT_WPM,
    })
}

fn plural(n: u64, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The facts as readable lines ("Words: 120").
pub(crate) fn describe(f: &Value) -> String {
    let mut out = String::new();
    let mut line = |label: &str, value: String| {
        out.push_str(label);
        out.push_str(": ");
        out.push_str(&value);
        out.push('\n');
    };
    let s = |v: &Value| v.as_str().map(str::to_owned);
    let n = |v: &Value| v.as_u64().unwrap_or(0);
    line("File", s(&f["file"]).unwrap_or_default());
    line("Format", s(&f["format"]).unwrap_or_default());
    for (label, key) in [
        ("Title", "title"),
        ("Author", "author"),
        ("Language", "language"),
    ] {
        if let Some(v) = s(&f[key]) {
            line(label, v);
        }
    }
    for (label, key) in [
        ("Characters", "chars"),
        ("Words", "words"),
        ("Sentences", "sentences"),
        ("Lines", "lines"),
        ("Paragraphs", "paragraphs"),
    ] {
        line(label, n(&f[key]).to_string());
    }
    let names: &[(&str, &str, &str)] = &[
        ("headings", "heading", "headings"),
        ("lists", "list", "lists"),
        ("list_items", "list item", "list items"),
        ("tables", "table", "tables"),
        ("table_rows", "table row", "table rows"),
        ("links", "link", "links"),
        ("images", "image", "images"),
        ("code_blocks", "code block", "code blocks"),
        ("block_quotes", "block quote", "block quotes"),
        ("footnotes", "footnote", "footnotes"),
        ("pages", "page", "pages"),
        ("sections", "section", "sections"),
    ];
    let parts: Vec<String> = names
        .iter()
        .filter(|(k, ..)| n(&f["structure"][k]) > 0)
        .map(|(k, one, many)| plural(n(&f["structure"][k]), one, many))
        .collect();
    line(
        "Structure",
        if parts.is_empty() {
            "none".to_owned()
        } else {
            parts.join(", ")
        },
    );
    let minutes = n(&f["reading_minutes"]);
    line(
        "Reading time",
        format!(
            "about {} at {} words per minute",
            plural(minutes, "minute", "minutes"),
            n(&f["reading_wpm"])
        ),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_the_markdown_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sample.md");
        let doc = load_document(&path).unwrap();
        let f = facts(&doc, &path);
        assert_eq!(f["format"], "markdown");
        assert_eq!(f["title"], "Sample Markdown Document");
        assert_eq!(f["author"], "Test Author");
        assert_eq!(f["structure"]["tables"], 1);
        assert_eq!(f["structure"]["code_blocks"], 1);
        assert_eq!(f["headings_by_level"]["h2"], 4);
        let text = describe(&f);
        assert!(text.contains("Format: markdown\n"));
        assert!(text.contains("1 table,"));
        assert!(text.contains("Reading time: about 1 minute at 265 words per minute\n"));
    }

    #[test]
    fn describes_every_new_format() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/a");
        for (name, format, expect) in [
            ("sample.epub", "epub", "3 sections"),
            ("sample.docx", "docx", "1 table"),
            ("running.pdf", "pdf", "3 pages"),
        ] {
            let path = dir.join(name);
            let doc = load_document(&path).unwrap();
            let f = facts(&doc, &path);
            assert_eq!(f["format"], format);
            let text = describe(&f);
            assert!(text.contains(expect), "{name}: {text}");
        }
    }
}
