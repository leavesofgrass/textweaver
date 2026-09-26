//! Export a document back to Markdown (`tw text --format markdown`).
//!
//! The canonical text is written line by line with its markers turned into
//! Markdown: `#` headings, `-` or numbered list items indented by depth,
//! pipe tables with a separator after the header row, fenced code blocks,
//! `>` quotes, `**bold**`, `*italic*`, `` `code` ``, links, and images.
//! Markdown special characters in the text are not escaped; the full
//! exporters (Markdown, HTML, text with options) arrive in wave 2.

use textweaver_core::{CharPos, MarkerKind};
use textweaver_text::{Document, Marker};

/// Inline markup inserted at a position: closings sort before openings.
struct Insert {
    at: usize,
    closing: bool,
    order: usize,
    text: String,
}

/// The document as Markdown.
pub fn to_markdown(doc: &Document) -> String {
    let index = doc.marker_index();
    let markers = doc.markers();
    let mut inserts: Vec<Insert> = Vec::new();
    for (order, m) in markers.iter().enumerate() {
        let (open, close) = match m.kind {
            MarkerKind::Bold => ("**".to_owned(), "**".to_owned()),
            MarkerKind::Italic => ("*".to_owned(), "*".to_owned()),
            MarkerKind::Underline => ("<u>".to_owned(), "</u>".to_owned()),
            MarkerKind::Code if m.level == 0 => ("`".to_owned(), "`".to_owned()),
            MarkerKind::Link => (
                "[".to_owned(),
                format!("]({})", m.reference.as_deref().unwrap_or("")),
            ),
            MarkerKind::Image => (
                "![".to_owned(),
                format!("]({})", m.reference.as_deref().unwrap_or("")),
            ),
            _ => continue,
        };
        if m.range.is_empty() {
            continue;
        }
        inserts.push(Insert {
            at: m.range.start.0,
            closing: false,
            order,
            text: open,
        });
        inserts.push(Insert {
            at: m.range.end.0,
            closing: true,
            order: usize::MAX - order,
            text: close,
        });
    }
    inserts.sort_by_key(|i| (i.at, !i.closing, i.order));

    let mut out = String::new();
    let mut next_insert = 0;
    let lines = doc.line_count();
    for line in 0..lines {
        let range = doc.line_range(line);
        let at = range.start;
        let in_code = index
            .enclosing(MarkerKind::Code, at)
            .or_else(|| {
                index
                    .starting_at(at)
                    .iter()
                    .find(|m| m.kind == MarkerKind::Code)
            })
            .filter(|m| m.level == 1);
        // Block prefixes for markers starting on this line.
        let starting = index.starting_at(at);
        if let Some(code) = starting
            .iter()
            .find(|m| m.kind == MarkerKind::Code && m.level == 1)
        {
            out.push_str(&quote_prefix(doc, at));
            out.push_str("```");
            out.push_str(code.label.as_deref().unwrap_or(""));
            out.push('\n');
        }
        out.push_str(&quote_prefix(doc, at));
        if in_code.is_none() {
            out.push_str(&line_prefix(starting));
        }
        let is_row = starting.iter().any(|m| m.kind == MarkerKind::TableRow);
        if is_row {
            out.push_str("| ");
        }
        // The line's text with inline markup.
        let text = doc.slice(range);
        let mut pos = range.start.0;
        let mut chars = text.chars();
        loop {
            while next_insert < inserts.len() && inserts[next_insert].at <= pos {
                if inserts[next_insert].at == pos && in_code.is_none() {
                    out.push_str(&inserts[next_insert].text);
                }
                next_insert += 1;
            }
            match chars.next() {
                Some(c) => {
                    out.push(c);
                    pos += 1;
                }
                None => break,
            }
        }
        if is_row {
            out.push_str(" |");
            if starting.iter().any(Marker::is_header_row) {
                let cols = index.enclosing(MarkerKind::TableRow, at).map_or(1, |row| {
                    index
                        .starting_in(row.range)
                        .iter()
                        .filter(|m| m.kind == MarkerKind::TableCell)
                        .count()
                        .max(1)
                });
                out.push('\n');
                out.push_str(&"|---".repeat(cols));
                out.push('|');
            }
        }
        if let Some(code) = in_code
            && code.range.end <= range.end
        {
            out.push('\n');
            out.push_str(&quote_prefix(doc, at));
            out.push_str("```");
        }
        if line + 1 < lines {
            out.push('\n');
        }
    }
    out.push('\n');
    out
}

fn line_prefix(starting: &[Marker]) -> String {
    for m in starting {
        match m.kind {
            MarkerKind::Heading => {
                return format!("{} ", "#".repeat(usize::from(m.level.clamp(1, 6))));
            }
            MarkerKind::ListItem => {
                let indent = "  ".repeat(usize::from(m.level.saturating_sub(1)));
                let bullet = m.label.as_deref().unwrap_or("-");
                return format!("{indent}{bullet} ");
            }
            _ => {}
        }
    }
    String::new()
}

fn quote_prefix(doc: &Document, at: CharPos) -> String {
    let depth = doc
        .marker_index()
        .containing(at)
        .filter(|m| m.kind == MarkerKind::Quote)
        .count()
        + doc
            .marker_index()
            .starting_at(at)
            .iter()
            .filter(|m| m.kind == MarkerKind::Quote && m.range.is_empty())
            .count();
    "> ".repeat(depth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LoadOptions, Loader, MarkdownLoader, Source};

    #[test]
    fn round_trips_common_structure() {
        let src = "# Title\n\nSome **bold** and [a link](https://x.org).\n\n- one\n  - two\n\n1. first\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n> Quoted.\n\n```rust\nlet x = 1;\n```\n";
        let doc = MarkdownLoader
            .load(
                &Source::Bytes {
                    data: src.as_bytes().to_vec(),
                    hint: "md".into(),
                },
                &LoadOptions::default(),
            )
            .unwrap();
        assert_eq!(
            to_markdown(&doc),
            "# Title\n\nSome **bold** and [a link](https://x.org).\n\n- one\n  - two\n\n1. first\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n> Quoted.\n\n```rust\nlet x = 1;\n```\n"
        );
    }
}
