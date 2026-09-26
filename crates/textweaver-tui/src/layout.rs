//! Laying out a window slice of the document into display rows.
//!
//! Only the lines from the viewport's first line down to what fits (plus
//! enough to reach the focus position) are laid out, so opening a large
//! document costs nothing extra. Positions stay document-absolute.
//!
//! [`Decor`] draws extra text between chars without changing positions:
//! the syllable separators of the syllable display (`read·a·bil·i·ty`).
//! A separator takes columns before its char, so wrapping and the
//! cursor's column count it, while every highlight and the cursor stay on
//! the document's own positions.

use ratatui::text::Span;
use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::text::Document;
use textweaver_app::text_util::{line_count, line_range};

/// How chars are measured and drawn: the tab width, and extra spaces after
/// each space for word spacing (reading aids' terminal text spacing).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cells {
    /// Columns a tab takes.
    pub tab: usize,
    /// Extra columns after each space.
    pub word_extra: usize,
}

impl Cells {
    /// Tabs of `tab` columns, no extra word spacing.
    pub fn tab(tab: usize) -> Self {
        Cells { tab, word_extra: 0 }
    }

    /// Display width of `c`.
    pub fn width(self, c: char) -> usize {
        if c == ' ' {
            1 + self.word_extra
        } else {
            char_width(c, self.tab)
        }
    }

    /// How `c` is drawn.
    pub fn text(self, c: char) -> String {
        if c == ' ' {
            " ".repeat(1 + self.word_extra)
        } else {
            display_text(c, self.tab)
        }
    }
}

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

/// Text drawn before some chars without changing positions: the syllable
/// separators.
pub struct Decor<'a> {
    /// The positions in a line (given as its range) that get the text
    /// before them, in order.
    pub breaks: &'a dyn Fn(CharRange) -> Vec<CharPos>,
    /// The text drawn at each break.
    pub text: String,
}

impl Decor<'_> {
    /// Columns the text takes.
    pub fn width(&self) -> usize {
        Span::raw(self.text.as_str()).width()
    }
}

/// Word-wraps `chars` to `width` columns. Returns `(start, end)` char
/// offsets of each row; every char belongs to exactly one row, and an empty
/// line is one empty row. Whitespace at a break stays at the end of the
/// row it follows.
pub fn wrap(chars: &[char], width: usize, tab: usize) -> Vec<(usize, usize)> {
    wrap_cells(chars, width, Cells::tab(tab))
}

/// [`wrap`] measuring with `cells`.
pub fn wrap_cells(chars: &[char], width: usize, cells: Cells) -> Vec<(usize, usize)> {
    wrap_extra(chars, width, cells, |_| 0)
}

/// [`wrap_cells`] with `extra(i)` more columns before char `i` (decor).
pub fn wrap_extra(
    chars: &[char],
    width: usize,
    cells: Cells,
    extra: impl Fn(usize) -> usize,
) -> Vec<(usize, usize)> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut start = 0;
    let mut used = 0;
    let mut last_break: Option<usize> = None;
    for (i, &c) in chars.iter().enumerate() {
        let w = cells.width(c) + extra(i);
        if used + w > width && i > start && !c.is_whitespace() {
            let brk = match last_break {
                Some(b) if b > start && b <= i => b,
                _ => i,
            };
            rows.push((start, brk));
            start = brk;
            used = (start..i).map(|j| cells.width(chars[j]) + extra(j)).sum();
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
    window_cells(doc, top_line, width, height, Cells::tab(tab), focus, margin)
}

/// [`window`] measuring with `cells`.
pub fn window_cells(
    doc: &Document,
    top_line: usize,
    width: usize,
    height: usize,
    cells: Cells,
    focus: Option<CharPos>,
    margin: usize,
) -> Vec<Row> {
    window_decor(doc, top_line, width, height, cells, None, focus, margin)
}

/// [`window_cells`] with `decor` drawn between chars.
#[allow(clippy::too_many_arguments)]
pub fn window_decor(
    doc: &Document,
    top_line: usize,
    width: usize,
    height: usize,
    cells: Cells,
    decor: Option<&Decor<'_>>,
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
        let wrapped = match decor {
            Some(d) => {
                let breaks = (d.breaks)(line_range(doc, line));
                let w = d.width();
                wrap_extra(&chars, width, cells, |i| {
                    if breaks.binary_search(&CharPos(base + i)).is_ok() {
                        w
                    } else {
                        0
                    }
                })
            }
            None => wrap_cells(&chars, width, cells),
        };
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
    column_cells(doc, row, pos, Cells::tab(tab))
}

/// [`column()`] measuring with `cells`.
pub fn column_cells(doc: &Document, row: &Row, pos: CharPos, cells: Cells) -> usize {
    column_decor(doc, row, pos, cells, &[], 0)
}

/// [`column_cells`] counting `sep_width` columns at each of `breaks` (the
/// row's decor positions, in order) before `pos`. A break at `pos` itself
/// is counted, so the cursor sits on the char, after its separator.
pub fn column_decor(
    doc: &Document,
    row: &Row,
    pos: CharPos,
    cells: Cells,
    breaks: &[CharPos],
    sep_width: usize,
) -> usize {
    let end = pos.clamp_to(row.range.end.0).max(row.range.start);
    let text: usize = doc
        .text()
        .slice(row.range.start.0..end.0)
        .chars()
        .map(|c| cells.width(c))
        .sum();
    let seps = breaks
        .iter()
        .filter(|b| **b > row.range.start && **b <= end && **b < row.range.end)
        .count();
    text + seps * sep_width
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
    fn word_spacing_widens_spaces() {
        let cells = Cells {
            tab: 4,
            word_extra: 2,
        };
        let chars: Vec<char> = "ab cd ef".chars().collect();
        // "ab" + 3 + "cd" = 7 fits; "ef" wraps.
        assert_eq!(wrap_cells(&chars, 8, cells), vec![(0, 6), (6, 8)]);
        let doc = Document::from_plain_text("ab cd");
        let rows = window_cells(&doc, 0, 20, 1, cells, None, 0);
        assert_eq!(column_cells(&doc, &rows[0], CharPos(3), cells), 5);
        assert_eq!(cells.text(' '), "   ");
    }

    #[test]
    fn decor_takes_columns_but_not_positions() {
        let doc = Document::from_plain_text("readability test");
        let breaks = |_: CharRange| vec![CharPos(4), CharPos(5), CharPos(8)];
        let decor = Decor {
            breaks: &breaks,
            text: "\u{b7}".into(),
        };
        let rows = window_decor(&doc, 0, 40, 1, Cells::tab(4), Some(&decor), None, 0);
        assert_eq!(rows[0].range, CharRange::new(0, 16));
        // "read·a·bil|ity": the cursor on "i" (8) is after three separators.
        let b = breaks(rows[0].range);
        assert_eq!(
            column_decor(&doc, &rows[0], CharPos(8), Cells::tab(4), &b, 1),
            10
        );
        assert_eq!(
            column_decor(&doc, &rows[0], CharPos(3), Cells::tab(4), &b, 1),
            3
        );
        // Narrow: the separators count toward the width.
        let rows = window_decor(&doc, 0, 14, 2, Cells::tab(4), Some(&decor), None, 0);
        assert_eq!(rows[0].range, CharRange::new(0, 12));
    }

    #[test]
    fn columns_count_display_width() {
        let doc = Document::from_plain_text("a\tb");
        let rows = window(&doc, 0, 20, 1, 4, None, 0);
        assert_eq!(column(&doc, &rows[0], CharPos(2), 4), 5);
    }
}
