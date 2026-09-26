//! The viewport and what is highlighted in it.
//!
//! The app keeps the first visible line in canonical lines; frontends lay out
//! a window slice of lines from there (wrapping as they must), so positions
//! stay document-absolute and nothing materializes the whole document.

use textweaver_a11y::Priority;
use textweaver_a11y::Verbosity;
use textweaver_core::{CharPos, CharRange, HighlightGranularity};

use crate::app::{App, Mode};
use crate::playback::Playback;
use crate::text_util;

/// Size of the document area and the first line shown in it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Viewport {
    /// First visible canonical line (0-based).
    pub top_line: usize,
    /// Columns available for text.
    pub width: u16,
    /// Rows available for text.
    pub height: u16,
}

/// Why a range is highlighted. Frontends map each kind to a style; the
/// kinds are listed from lowest to highest drawing priority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HighlightKind {
    /// A bookmarked word.
    Bookmark,
    /// A search match.
    FindHit,
    /// The selection.
    Selection,
    /// The sentence being spoken.
    SpokenSentence,
    /// The search match the cursor is on.
    CurrentFindHit,
    /// The word (or range) being spoken.
    SpokenWord,
}

/// One highlighted range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Highlight {
    /// The chars.
    pub range: CharRange,
    /// Why.
    pub kind: HighlightKind,
}

impl App {
    /// The position the viewport keeps in view and where the hardware
    /// cursor belongs: the spoken word while reading, else the Speech
    /// Cursor line, else the cursor.
    pub fn focus(&self) -> Option<CharPos> {
        let s = self.session.as_ref()?;
        if let (Playback::Reading, Some(r)) = (self.playback, s.spoken) {
            return Some(r.start);
        }
        if self.mode == Mode::SpeechCursor {
            if let Some(line) = s.speech_cursor_line {
                return Some(text_util::line_range(&s.doc, line).start);
            }
        }
        Some(s.cursor)
    }

    fn margin(&self) -> usize {
        let m = usize::from(self.settings.display.scroll_margin);
        m.min(usize::from(self.view.height.saturating_sub(1)) / 2)
    }

    /// Scrolls so `line` is visible with the scroll margin.
    pub(crate) fn scroll_to_line(&mut self, line: usize) {
        let height = usize::from(self.view.height);
        if height == 0 {
            return;
        }
        let margin = self.margin();
        if line < self.view.top_line + margin {
            self.view.top_line = line.saturating_sub(margin);
        } else if line + margin >= self.view.top_line + height {
            self.view.top_line = (line + margin + 1).saturating_sub(height);
        }
    }

    pub(crate) fn scroll_to_cursor(&mut self) {
        if let Some(line) = self.session.as_ref().map(|s| s.line()) {
            self.scroll_to_line(line);
        }
    }

    pub(crate) fn scroll_to_focus(&mut self) {
        let line = self
            .focus()
            .zip(self.session.as_ref())
            .map(|(p, s)| text_util::line_of(&s.doc, p));
        if let Some(line) = line {
            self.scroll_to_line(line);
        }
    }

    /// Scrolls the viewport without moving the cursor.
    pub(crate) fn scroll_lines(&mut self, delta: isize) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let last = text_util::line_count(&s.doc).saturating_sub(1);
        let top = self.view.top_line.saturating_add_signed(delta).min(last);
        if top == self.view.top_line {
            let edge = if delta < 0 { "Top" } else { "Bottom" };
            self.say_at(
                &format!("{edge} of document."),
                Verbosity::Normal,
                Priority::Polite,
            );
            return;
        }
        self.view.top_line = top;
        self.say_at(
            &format!("Line {} at top.", top + 1),
            Verbosity::High,
            Priority::Polite,
        );
    }

    /// Highlights intersecting `range` (typically the visible window), in
    /// drawing order: later entries win where they overlap.
    pub fn highlights(&self, range: CharRange) -> Vec<Highlight> {
        let Some(s) = self.session.as_ref() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut push = |r: CharRange, kind| {
            if r.intersects(range) || (r.is_empty() && range.contains(r.start)) {
                out.push(Highlight { range: r, kind });
            }
        };
        for b in &s.bookmarks {
            if range.contains(b.pos) {
                let r = text_util::word_containing(&s.doc, b.pos)
                    .unwrap_or_else(|| CharRange::new(b.pos, b.pos.saturating_add(1)));
                push(r, HighlightKind::Bookmark);
            }
        }
        if let Some(f) = &s.find {
            let first = f.hits.partition_point(|h| h.end <= range.start);
            for (i, h) in f.hits.iter().enumerate().skip(first) {
                if h.start >= range.end {
                    break;
                }
                let kind = if f.current == Some(i) {
                    HighlightKind::CurrentFindHit
                } else {
                    HighlightKind::FindHit
                };
                push(*h, kind);
            }
        }
        if let Some(sel) = s.selection.filter(|r| !r.is_empty()) {
            push(sel, HighlightKind::Selection);
        }
        if self.settings.highlight.enabled {
            let g = self.settings.highlight.granularity;
            if matches!(
                g,
                HighlightGranularity::Sentence | HighlightGranularity::Both
            ) {
                if let Some(r) = s.spoken_sentence.or(s.spoken) {
                    push(r, HighlightKind::SpokenSentence);
                }
            }
            if matches!(g, HighlightGranularity::Word | HighlightGranularity::Both) {
                if let Some(r) = s.spoken {
                    push(r, HighlightKind::SpokenWord);
                }
            }
        }
        out.sort_by_key(|h| h.kind);
        out
    }
}
