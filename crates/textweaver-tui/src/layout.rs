//! Laying out a window slice of the document into display rows.
//!
//! Only the lines from the viewport's first line down to what fits (plus
//! enough to reach the focus position) are laid out, so opening a large
//! document costs nothing extra. Positions stay document-absolute.

use ratatui::text::Span;
use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::text::Document;
use textweaver_app::text_util::{line_count, line_range};

/// Display width of one char (tabs expand to `tab` columns).
pub fn char_width(c: char, tab: usize) -> usize {
    match c {
        '\t' => tab.max(1),
        c if c.is_control() => 1,
        c => {
            let mut buf = [0u8; 4];
            Span::raw(&*c.encode_utf8(&mut buf)).width()
        }
    }
}

/// How a char is drawn (tabs and control chars become spaces).
pub fn display_text(c: char, tab: usize) -> String {
    match c {
        '\t' => " ".repeat(tab.max(1)),
        c if c.is_control() => " ".into(),
        c => c.to_string(),
    }
}

/// Word-wraps `chars` to `width` columns. Returns `(start, end)` char
/// offsets of each row; every char belongs to exactly one row, and an empty
/// line is one empty row. Whitespace at a break stays at the end of the
/// row it follows.
pub fn wrap(chars: &[char], width: usize, tab: usize) -> Vec<(usize, usize)> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut start = 0;
    let mut used = 0;
    let mut last_break: Option<usize> = None;
    for (i, &c) in chars.iter().enumerate() {
        let w = char_width(c, tab);
        if used + w > width && i > start && !c.is_whitespace() {
            let brk = match last_break {
                Some(b) if b > start && b <= i => b,
                _ => i,
            };
            rows.push((start, brk));
            start = brk;
            used = chars[start..i].iter().map(|&c| char_width(c, tab)).sum();
            last_break = chars[start..i]
                .iter()
                .rposition(|c| c.is_whitespace())
                .map(|p| start + p + 1);
        }
        used += w;
        if c.is_whitespace() {
            last_break = Some(i + 1);
        }
    }
    rows.push((start, chars.len()));
    rows
}

/// One display row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    /// Canonical line (0-based).
    pub line: usize,
    /// Document chars on the row.
    pub range: CharRange,
    /// First row of its line.
    pub first: bool,
    /// Last row of its line (the line's end belongs to it).
    pub last: bool,
}

impl Row {
    /// True when the cursor at `pos` is drawn on this row.
    pub fn holds(&self, pos: CharPos) -> bool {
        self.range.contains(pos) || (self.last && pos == self.range.end)
    }
}

/// The chars of line `line` without its newline.
pub fn line_chars(doc: &Document, line: usize) -> Vec<char> {
    let r = line_range(doc, line);
    doc.text().slice(r.to_range()).chars().collect()
}

/// Lays out rows from `top_line` so that `height` rows are visible and the
/// row holding `focus` (if it is at or below `top_line`) is among them with
/// `margin` rows below it where possible. Returns the visible rows.
pub fn window(
    doc: &Document,
    top_line: usize,
    width: usize,
    height: usize,
    tab: usize,
    focus: Option<CharPos>,
    margin: usize,
) -> Vec<Row> {
    let lines = line_count(doc);
    let limit = height.saturating_mul(4).max(height + 1).max(512);
    let mut rows: Vec<Row> = Vec::new();
    let mut focus_row = None;
    let mut line = top_line.min(lines.saturating_sub(1));
    while line < lines {
        let base = line_range(doc, line).start.0;
        let chars = line_chars(doc, line);
        let wrapped = wrap(&chars, width, tab);
        let n = wrapped.len();
        for (i, (a, b)) in wrapped.into_iter().enumerate() {
            let row = Row {
                line,
                range: CharRange::new(base + a, base + b),
                first: i == 0,
                last: i + 1 == n,
            };
            if focus_row.is_none() && focus.is_some_and(|f| row.holds(f)) {
                focus_row = Some(rows.len());
            }
            rows.push(row);
        }
        line += 1;
        let laid_out_to = rows.last().map_or(CharPos::ZERO, |r| r.range.end);
        let done = match focus_row {
            Some(f) => rows.len() >= height.max(f + 1 + margin),
            // Keep going only while the focus is still further down.
            None => rows.len() >= height && focus.is_none_or(|f| f <= laid_out_to),
        };
        if done || rows.len() >= limit {
            break;
        }
    }
    let skip = focus_row.map_or(0, |f| (f + 1 + margin).saturating_sub(height).min(f));
    rows.into_iter().skip(skip).take(height).collect()
}

/// Display column of `pos` on `row`.
pub fn column(doc: &Document, row: &Row, pos: CharPos, tab: usize) -> usize {
    let end = pos.clamp_to(row.range.end.0).max(row.range.start);
    doc.text()
        .slice(row.range.start.0..end.0)
        .chars()
        .map(|c| char_width(c, tab))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(s: &str, w: usize) -> Vec<String> {
        let chars: Vec<char> = s.chars().collect();
        wrap(&chars, w, 4)
            .into_iter()
            .map(|(a, b)| chars[a..b].iter().collect())
            .collect()
    }

    #[test]
    fn wraps_at_spaces_and_hard_breaks_long_words() {
        assert_eq!(rows("one two three", 8), vec!["one two ", "three"]);
        assert_eq!(rows("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
        assert_eq!(rows("", 4), vec![""]);
        assert_eq!(rows("ab cd", 5), vec!["ab cd"]);
        assert_eq!(rows("aaaa bbbb", 4), vec!["aaaa ", "bbbb"]);
    }

    #[test]
    fn window_keeps_focus_visible() {
        let text = (1..=30)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let doc = Document::from_plain_text(&text);
        let focus = line_range(&doc, 20).start;
        let rows = window(&doc, 0, 20, 5, 4, Some(focus), 1);
        assert_eq!(rows.len(), 5);
        assert!(rows.iter().any(|r| r.holds(focus)));
        assert_eq!(rows[3].line, 20);
        let rows = window(&doc, 0, 20, 5, 4, None, 1);
        assert_eq!(rows[0].line, 0);
    }

    #[test]
    fn columns_count_display_width() {
        let doc = Document::from_plain_text("a\tb");
        let rows = window(&doc, 0, 20, 1, 4, None, 0);
        assert_eq!(column(&doc, &rows[0], CharPos(2), 4), 5);
    }
}
