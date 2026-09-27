//! `tw search`. Owner: Agent A.
//!
//! Prints every match as `line:column: context`, grep style, followed by a
//! count on standard error. Exits with status 1 when nothing matches (text
//! output only; `--json` always succeeds and reports the count). Plain
//! patterns match across line breaks and are case-insensitive unless
//! `--case-sensitive` is given (see `textweaver_text::search`).

use std::path::PathBuf;

use serde_json::{Value, json};
use textweaver_app::text::core::CharRange;
use textweaver_app::text::{Document, SearchQuery, find_all};

use super::text::load_document;

/// Arguments for `tw search`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Document to search.
    pub file: PathBuf,
    /// Text or pattern to find.
    pub pattern: String,
    /// Treat the pattern as a regular expression.
    #[arg(long)]
    pub regex: bool,
    /// Match case exactly.
    #[arg(long)]
    pub case_sensitive: bool,
    /// Match whole words only.
    #[arg(long)]
    pub whole_word: bool,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
}

/// Chars of context shown on each side of a match.
const CONTEXT: usize = 30;

/// Runs `tw search`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let doc = load_document(&args.file)?;
    let query = SearchQuery {
        pattern: args.pattern.clone(),
        regex: args.regex,
        case_sensitive: args.case_sensitive,
        whole_word: args.whole_word,
        ..SearchQuery::default()
    };
    let hits = find_all(&doc, &query)?;
    if args.json {
        let v = json!({
            "pattern": args.pattern,
            "count": hits.len(),
            "matches": hits.iter().map(|h| hit_json(&doc, *h)).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&v)?);
        return Ok(());
    }
    let lines: String = hits
        .iter()
        .map(|h| format!("{}\n", hit_line(&doc, *h)))
        .collect();
    super::print_all(&lines)?;
    if hits.is_empty() {
        eprintln!("No matches for \"{}\".", args.pattern);
        std::process::exit(1);
    }
    eprintln!(
        "{} {}.",
        hits.len(),
        if hits.len() == 1 { "match" } else { "matches" }
    );
    Ok(())
}

/// The match with up to [`CONTEXT`] chars around it on its line(s), on one
/// output line.
fn context(doc: &Document, hit: CharRange) -> String {
    let first = doc.line_range(doc.line_of(hit.start));
    let last = doc.line_range(doc.line_of(hit.end));
    let start = hit.start.saturating_sub(CONTEXT).max(first.start);
    let end = hit.end.saturating_add(CONTEXT).min(last.end.max(hit.end));
    let mut s = String::new();
    if start > first.start {
        s.push('\u{2026}');
    }
    s.push_str(&doc.slice(CharRange::new(start, end)).replace('\n', " "));
    if end < last.end {
        s.push('\u{2026}');
    }
    s
}

fn hit_line(doc: &Document, hit: CharRange) -> String {
    let (line, col) = doc.line_col(hit.start);
    format!("{}:{}: {}", line + 1, col + 1, context(doc, hit))
}

fn hit_json(doc: &Document, hit: CharRange) -> Value {
    let (line, col) = doc.line_col(hit.start);
    json!({
        "start": hit.start.0,
        "end": hit.end.0,
        "line": line + 1,
        "column": col + 1,
        "text": doc.slice(hit),
        "context": context(doc, hit),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_and_context() {
        let doc = Document::from_plain_text(
            "first line\nthe quick brown fox jumps over the lazy dog and keeps running far away",
        );
        let q = SearchQuery {
            pattern: "lazy".into(),
            ..SearchQuery::default()
        };
        let hits = find_all(&doc, &q).unwrap();
        assert_eq!(hits.len(), 1);
        let line = hit_line(&doc, hits[0]);
        assert!(line.starts_with("2:36: \u{2026}"), "{line}");
        assert!(line.ends_with('\u{2026}'), "{line}");
        let v = hit_json(&doc, hits[0]);
        assert_eq!(v["text"], "lazy");
        assert_eq!(v["line"], 2);
    }
}
