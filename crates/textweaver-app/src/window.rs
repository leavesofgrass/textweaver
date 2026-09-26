//! The document window: the part of a document a GUI text view holds at a
//! time (Wave 3, Agent W3a).
//!
//! A GUI cannot lay out ten million characters at once (the wxDragon spike
//! took 9.3 s to load them into a native control). It shows a
//! [`DocWindow`] instead: about [`WINDOW_UNITS`] UTF-16 code units of text
//! around the focus, starting and ending on paragraph boundaries, so no
//! paragraph is cut in two and a screen reader never meets half a sentence
//! at the edge.
//!
//! - **Around the focus.** [`DocWindow::around`] centres a window on a
//!   position (the cursor, the spoken word).
//! - **Extends while reading.** [`DocWindow::follow`] is called with the
//!   focus after every change. When the focus comes within an eighth of the
//!   window of its end, the window slides forward: paragraphs leave the
//!   front and join the end ([`WindowChange::Forward`]), so a frontend can
//!   update its text in place instead of replacing it. Scrolling back near
//!   the start slides it back the same way.
//! - **Recentres on jumps.** A focus outside the window (a heading jump, a
//!   search, go to) gives a new window around it ([`WindowChange::Recentred`]).
//! - **Positions.** A text control counts in its own units: UTF-16 code
//!   units (Windows and macOS controls), UTF-8 bytes (Parley and
//!   AccessKit), or chars (GTK). [`DocWindow::to_ctrl`] and
//!   [`DocWindow::to_doc`] map between those offsets, counted from the
//!   window's start, and document [`CharPos`]itions, through the document's
//!   [`DisplayIndex`](textweaver_text::DisplayIndex), which answers in
//!   `O(log n)` from the rope itself.
//!
//! The window holds only a char range; offsets are computed when asked, so
//! it is `Copy` and costs nothing to keep. After an edit, call
//! [`DocWindow::follow`] again: a window past the end of the text, or one
//! for an older [`Session::revision`](crate::Session::revision), is
//! recentred.
//!
//! The terminal reader keeps its own slicing (`textweaver-tui`'s `layout`,
//! a screenful of wrapped rows from the viewport's top line); both slice
//! the same rope, and neither copies the whole document.

use std::ops::Range;

use textweaver_core::{CharPos, CharRange};
use textweaver_text::Document;

/// About how many UTF-16 code units a window holds: half a million, a few
/// hundred pages.
pub const WINDOW_UNITS: usize = 500_000;

/// How a text control counts positions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Units {
    /// UTF-16 code units (Windows RichEdit and UI Automation, macOS
    /// `NSTextView`).
    Utf16,
    /// UTF-8 bytes (Rust strings, Parley, AccessKit).
    Utf8,
    /// Unicode scalar values, the document's own count (GTK).
    Chars,
}

impl Units {
    /// The offset of `pos` from the start of `doc`, in these units.
    pub fn offset(self, doc: &Document, pos: CharPos) -> usize {
        let pos = pos.clamp_to(doc.len_chars());
        match self {
            Units::Utf16 => doc.display().to_utf16(pos) as usize,
            Units::Utf8 => doc.display().to_byte(pos),
            Units::Chars => pos.0,
        }
    }

    /// The document position at `offset` in these units: rounded down to
    /// a char start, clamped to the end.
    pub fn position(self, doc: &Document, offset: usize) -> CharPos {
        match self {
            Units::Utf16 => doc
                .display()
                .to_char(u32::try_from(offset).unwrap_or(u32::MAX)),
            Units::Utf8 => doc.display().byte_to_char(offset),
            Units::Chars => CharPos(offset.min(doc.len_chars())),
        }
    }
}

/// What [`DocWindow::follow`] did to the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowChange {
    /// Nothing: the focus is well inside the window.
    Unchanged,
    /// The window slid forward: `dropped` left its front and `added` joined
    /// its end. Either may be empty.
    Forward {
        /// Text no longer in the window, at its old start.
        dropped: CharRange,
        /// Text new in the window, after its old end.
        added: CharRange,
    },
    /// The window slid back: `added` joined its front and `dropped` left
    /// its end.
    Backward {
        /// Text new in the window, before its old start.
        added: CharRange,
        /// Text no longer in the window, at its old end.
        dropped: CharRange,
    },
    /// A new window around the focus: replace everything shown.
    Recentred,
}

/// A paragraph-aligned slice of a document, about [`WINDOW_UNITS`] UTF-16
/// code units long. See the [module docs](self).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DocWindow {
    range: CharRange,
    budget: usize,
    revision: u64,
}

impl DocWindow {
    /// A window of about [`WINDOW_UNITS`] around `focus`.
    pub fn around(doc: &Document, focus: CharPos) -> Self {
        Self::with_budget(doc, focus, WINDOW_UNITS)
    }

    /// A window of about `budget` UTF-16 code units (at least 1,024) around
    /// `focus`: the focus in its middle, except near the document's ends.
    pub fn with_budget(doc: &Document, focus: CharPos, budget: usize) -> Self {
        let budget = budget.max(1024);
        let f = Units::Utf16.offset(doc, focus);
        Self::span(doc, f.saturating_sub(budget / 2), budget)
    }

    /// A window of `budget` units from about unit `start`, aligned.
    fn span(doc: &Document, start: usize, budget: usize) -> Self {
        let total = doc.display().len_utf16();
        let range = if total <= budget {
            doc.full_range()
        } else {
            let start = start.min(total - budget);
            let s = Units::Utf16.position(doc, start);
            let e = Units::Utf16.position(doc, start + budget);
            let slack = budget / 4;
            CharRange::new(
                boundary_before(doc, s, slack),
                boundary_after(doc, e, slack),
            )
        };
        DocWindow {
            range,
            budget,
            revision: 0,
        }
    }

    /// The same window, marked as made for text revision `revision`
    /// ([`Session::revision`](crate::Session::revision)): [`follow_session`]
    /// recentres it when the text changes.
    ///
    /// [`follow_session`]: Self::follow_session
    pub fn for_revision(mut self, revision: u64) -> Self {
        self.revision = revision;
        self
    }

    /// The text revision this window was made for (0 when not set).
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The chars in the window.
    pub fn range(&self) -> CharRange {
        self.range
    }

    /// How many UTF-16 code units the window aims for.
    pub fn budget(&self) -> usize {
        self.budget
    }

    /// True when `pos` is in the window (its end counts only at the end of
    /// the document, where a caret can be).
    pub fn contains(&self, doc: &Document, pos: CharPos) -> bool {
        self.range.contains(pos) || (pos == self.range.end && pos.0 == doc.len_chars())
    }

    /// True when the window covers the whole document.
    pub fn is_whole(&self, doc: &Document) -> bool {
        self.range == doc.full_range()
    }

    /// The window's text.
    pub fn text(&self, doc: &Document) -> String {
        doc.slice(self.range)
    }

    /// The window's length in `units`.
    pub fn len(&self, doc: &Document, units: Units) -> usize {
        units.offset(doc, self.range.end) - units.offset(doc, self.range.start)
    }

    /// True when the window holds no text (an empty document).
    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }

    /// The control offset of `pos`, counted in `units` from the window's
    /// start; `None` outside the window.
    pub fn to_ctrl(&self, doc: &Document, pos: CharPos, units: Units) -> Option<usize> {
        if !self.contains(doc, pos) {
            return None;
        }
        Some(units.offset(doc, pos) - units.offset(doc, self.range.start))
    }

    /// The document position of control offset `ctrl` (in `units`, from the
    /// window's start), clamped to the window. An offset inside a char (a
    /// surrogate pair, a multi-byte char) maps to that char.
    pub fn to_doc(&self, doc: &Document, ctrl: usize, units: Units) -> CharPos {
        let base = units.offset(doc, self.range.start);
        units
            .position(doc, base.saturating_add(ctrl))
            .max(self.range.start)
            .min(self.range.end)
    }

    /// The part of `range` inside the window, as control offsets; `None`
    /// when it is outside the window.
    pub fn ctrl_range(
        &self,
        doc: &Document,
        range: CharRange,
        units: Units,
    ) -> Option<Range<usize>> {
        let start = range.start.max(self.range.start);
        let end = range.end.min(self.range.end);
        if start > end || (start == end && !range.is_empty()) {
            return None;
        }
        let base = units.offset(doc, self.range.start);
        Some(units.offset(doc, start) - base..units.offset(doc, end) - base)
    }

    /// Keeps `focus` in the window (see the [module docs](self)): slides
    /// the window when the focus nears an edge, recentres it when the focus
    /// left it or the text no longer fits it.
    pub fn follow(&mut self, doc: &Document, focus: CharPos) -> WindowChange {
        let focus = focus.clamp_to(doc.len_chars());
        if self.range.end.0 > doc.len_chars() || !self.contains(doc, focus) {
            *self = Self::with_budget(doc, focus, self.budget).for_revision(self.revision);
            return WindowChange::Recentred;
        }
        if self.is_whole(doc) {
            return WindowChange::Unchanged;
        }
        let u = Units::Utf16;
        let f = u.offset(doc, focus);
        let start = u.offset(doc, self.range.start);
        let end = u.offset(doc, self.range.end);
        let margin = self.budget / 8;
        let old = self.range;
        if self.range.end.0 < doc.len_chars() && end.saturating_sub(f) < margin {
            // Reading on: the focus goes to the first quarter.
            let new = Self::span(doc, f.saturating_sub(self.budget / 4), self.budget);
            if new.range.start >= old.start && new.range.start <= old.end {
                self.range = new.range;
                return WindowChange::Forward {
                    dropped: CharRange::new(old.start, new.range.start),
                    added: CharRange::new(old.end, new.range.end.max(old.end)),
                };
            }
        } else if self.range.start.0 > 0 && f.saturating_sub(start) < margin {
            // Moving back: the focus goes to the last quarter.
            let new = Self::span(doc, f.saturating_sub(self.budget * 3 / 4), self.budget);
            if new.range.end <= old.end && new.range.end >= old.start {
                self.range = new.range;
                return WindowChange::Backward {
                    added: CharRange::new(new.range.start.min(old.start), old.start),
                    dropped: CharRange::new(new.range.end, old.end),
                };
            }
        } else {
            return WindowChange::Unchanged;
        }
        *self = Self::with_budget(doc, focus, self.budget).for_revision(self.revision);
        WindowChange::Recentred
    }

    /// [`follow`](Self::follow) for a session: a window made for an older
    /// text revision is recentred (the text changed under it).
    pub fn follow_session(&mut self, session: &crate::Session, focus: CharPos) -> WindowChange {
        if self.revision != session.revision {
            *self =
                Self::with_budget(&session.doc, focus, self.budget).for_revision(session.revision);
            return WindowChange::Recentred;
        }
        self.follow(&session.doc, focus)
    }
}

/// True for a line holding only spaces and tabs.
fn blank(line: &[char]) -> bool {
    line.iter().all(|c| *c == ' ' || *c == '\t' || *c == '\r')
}

/// The paragraph boundary at or before `pos`, looking back at most `limit`
/// chars: the start of the document, or a line start after a blank line.
/// Without one in reach, the nearest line start; without that, `pos`.
fn boundary_before(doc: &Document, pos: CharPos, limit: usize) -> CharPos {
    let text = doc.text();
    let pos = pos.0.min(text.len_chars());
    let mut chars = text.chars_at(pos);
    // The line being walked (backwards) since the last line break seen.
    let mut line: Vec<char> = Vec::new();
    let mut line_start = None;
    let mut last_break: Option<usize> = None;
    let mut i = pos;
    while pos - i < limit {
        let Some(c) = chars.prev() else {
            // The start of the document is a paragraph start.
            return CharPos(0);
        };
        i -= 1;
        if c == '\n' {
            if let Some(b) = last_break
                && blank(&line)
            {
                // The line between this break and the last one is blank:
                // the paragraph after it starts after the last break.
                return CharPos(b + 1);
            }
            line_start.get_or_insert(i + 1);
            last_break = Some(i);
            line.clear();
        } else {
            line.push(c);
        }
    }
    CharPos(line_start.unwrap_or(pos))
}

/// The paragraph boundary at or after `pos`, looking ahead at most `limit`
/// chars: the end of the document, or the start of a blank line. Without
/// one in reach, the next line start; without that, `pos`.
fn boundary_after(doc: &Document, pos: CharPos, limit: usize) -> CharPos {
    let text = doc.text();
    let len = text.len_chars();
    let pos = pos.0.min(len);
    if pos == len {
        return CharPos(len);
    }
    // Already at a line start before a blank line (or the end)?
    let at_line_start = pos == 0 || text.char(pos - 1) == '\n';
    let mut line: Vec<char> = Vec::new();
    let mut line_start = at_line_start.then_some(pos);
    let mut first_line_start = None;
    for (k, c) in text.chars_at(pos).take(limit).enumerate() {
        let i = pos + k;
        if c == '\n' {
            if let Some(s) = line_start
                && blank(&line)
            {
                return CharPos(s);
            }
            line_start = Some(i + 1);
            first_line_start.get_or_insert(i + 1);
            line.clear();
        } else {
            line.push(c);
        }
        if i + 1 == len {
            return CharPos(len);
        }
    }
    CharPos(first_line_start.unwrap_or(pos).min(len))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `n` paragraphs of about 100 chars, each numbered.
    fn paragraphs(n: usize) -> Document {
        let text: Vec<String> = (0..n)
            .map(|i| format!("Paragraph {i:05} {}", "word ".repeat(17).trim_end()))
            .collect();
        Document::from_plain_text(&text.join("\n\n"))
    }

    /// True when `pos` starts a paragraph or the blank line after one (or
    /// ends the document).
    fn is_boundary(doc: &Document, pos: CharPos) -> bool {
        let t = doc.text();
        let p = pos.0;
        p == 0
            || p == doc.len_chars()
            || (p >= 2 && t.char(p - 1) == '\n' && t.char(p - 2) == '\n')
            || (p >= 1 && t.char(p - 1) == '\n' && t.char(p) == '\n')
    }

    #[test]
    fn a_small_document_is_one_window() {
        let doc = Document::from_plain_text("One.\n\nTwo.");
        let w = DocWindow::around(&doc, CharPos(3));
        assert!(w.is_whole(&doc));
        assert_eq!(w.text(&doc), "One.\n\nTwo.");
        assert!(w.contains(&doc, doc.end()));
    }

    #[test]
    fn windows_are_paragraph_aligned_and_about_the_budget() {
        let doc = paragraphs(1000);
        for focus in [0, 50_000, 60_123, doc.len_chars() - 1] {
            let w = DocWindow::with_budget(&doc, CharPos(focus), 10_000);
            let r = w.range();
            assert!(w.contains(&doc, CharPos(focus)), "{focus} in {r:?}");
            assert!(is_boundary(&doc, r.start), "start {r:?}");
            assert!(is_boundary(&doc, r.end), "end {r:?}");
            let n = w.len(&doc, Units::Utf16);
            assert!((10_000..=15_000).contains(&n), "{n} units");
            assert!(w.text(&doc).starts_with("Paragraph") || r.start.0 == 0);
        }
    }

    #[test]
    fn reading_on_slides_forward_and_jumps_recentre() {
        let doc = paragraphs(1000);
        let mut w = DocWindow::with_budget(&doc, CharPos(0), 10_000);
        let first = w.range();
        // Reading through the window: unchanged until near its end.
        assert_eq!(w.follow(&doc, CharPos(1000)), WindowChange::Unchanged);
        let near_end = CharPos(first.end.0 - 100);
        match w.follow(&doc, near_end) {
            WindowChange::Forward { dropped, added } => {
                assert_eq!(dropped.start, first.start);
                assert_eq!(dropped.end, w.range().start);
                assert_eq!(added.start, first.end);
                assert_eq!(added.end, w.range().end);
                assert!(is_boundary(&doc, w.range().start));
            }
            other => panic!("expected a forward slide, got {other:?}"),
        }
        assert!(w.contains(&doc, near_end));
        // A jump far away recentres.
        assert_eq!(w.follow(&doc, CharPos(90_000)), WindowChange::Recentred);
        assert!(w.contains(&doc, CharPos(90_000)));
        // Back near the start of the window: it slides back.
        let back = CharPos(w.range().start.0 + 10);
        let before = w.range();
        match w.follow(&doc, back) {
            WindowChange::Backward { added, dropped } => {
                assert_eq!(added.end, before.start);
                assert_eq!(dropped.end, before.end);
                assert!(w.contains(&doc, back));
            }
            other => panic!("expected a backward slide, got {other:?}"),
        }
    }

    #[test]
    fn a_shorter_text_recentres() {
        let doc = paragraphs(1000);
        let mut w = DocWindow::with_budget(&doc, CharPos(80_000), 10_000);
        let short = paragraphs(10);
        assert_eq!(w.follow(&short, CharPos(0)), WindowChange::Recentred);
        assert!(w.is_whole(&short));
    }

    #[test]
    fn control_offsets_in_every_unit() {
        // "é" is one UTF-16 unit and two bytes; the emoji two units, four
        // bytes.
        let doc = Document::from_plain_text("aé😀b\n\nc");
        let w = DocWindow::around(&doc, CharPos(0));
        let at = |p: usize, u: Units| w.to_ctrl(&doc, CharPos(p), u).unwrap();
        assert_eq!(at(3, Units::Utf16), 4);
        assert_eq!(at(3, Units::Utf8), 7);
        assert_eq!(at(3, Units::Chars), 3);
        assert_eq!(
            w.to_doc(&doc, 3, Units::Utf16),
            CharPos(2),
            "inside the pair"
        );
        assert_eq!(
            w.to_doc(&doc, 5, Units::Utf8),
            CharPos(2),
            "inside the emoji"
        );
        assert_eq!(w.to_doc(&doc, 99, Units::Utf16), doc.end());
        assert_eq!(
            w.ctrl_range(&doc, CharRange::new(2, 3), Units::Utf16),
            Some(2..4)
        );
        for u in [Units::Utf16, Units::Utf8, Units::Chars] {
            for p in 0..=doc.len_chars() {
                let c = w.to_ctrl(&doc, CharPos(p), u).unwrap();
                assert_eq!(w.to_doc(&doc, c, u), CharPos(p), "{u:?} {p}");
            }
        }
    }

    #[test]
    fn offsets_count_from_the_window_start() {
        let doc = paragraphs(1000);
        let w = DocWindow::with_budget(&doc, CharPos(50_000), 10_000);
        let start = w.range().start;
        assert_eq!(w.to_ctrl(&doc, start, Units::Utf16), Some(0));
        assert_eq!(w.to_ctrl(&doc, CharPos(0), Units::Utf16), None);
        assert_eq!(w.to_doc(&doc, 0, Units::Chars), start);
        assert_eq!(
            w.ctrl_range(&doc, CharRange::new(0, start.0 + 5), Units::Chars),
            Some(0..5),
            "clipped to the window"
        );
        assert_eq!(
            w.ctrl_range(&doc, CharRange::new(0, 10), Units::Chars),
            None
        );
    }

    #[test]
    fn one_huge_paragraph_falls_back_to_lines_then_chars() {
        // No blank lines at all: windows end on line starts.
        let lines: Vec<String> = (0..2000).map(|i| format!("line {i:05} text")).collect();
        let doc = Document::from_plain_text(&lines.join("\n"));
        let w = DocWindow::with_budget(&doc, CharPos(15_000), 4_000);
        let r = w.range();
        assert_eq!(doc.text().char(r.start.0 - 1), '\n');
        assert!(r.end.0 == doc.len_chars() || doc.text().char(r.end.0 - 1) == '\n');
        // No line breaks either: the window is cut by units.
        let doc = Document::from_plain_text(&"x".repeat(100_000));
        let w = DocWindow::with_budget(&doc, CharPos(50_000), 4_000);
        assert_eq!(w.len(&doc, Units::Chars), 4_000);
    }
}
