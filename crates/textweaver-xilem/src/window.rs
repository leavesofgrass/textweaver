//! The text window: the part of a large document the document view holds.
//!
//! Positions stay document-absolute everywhere; only the window's text is
//! laid out, painted, and exposed to screen readers. The window follows the
//! focus (the caret, or the spoken word): it is rebuilt around the focus
//! when the focus comes within [`MARGIN_CHARS`] of an edge that is not the
//! document's own edge.
//!
//! This is a thin adapter over [`Document`] until Agent W3a's shared window
//! model lands in `textweaver-app` (ADR-0023, "Windowing"); the document
//! view only needs [`TextWindow::range`] and [`window_paragraphs`].

use textweaver_app::core::{CharPos, CharRange, MarkerKind};
use textweaver_app::text::Document;

use crate::runs::{self, Paragraph};

/// Target size of a window, in chars. Chosen by measurement (ADR-0023,
/// "Measurements"): large enough that reading rarely moves it, small enough
/// that building its text runs stays well under a frame.
pub const WINDOW_CHARS: usize = 120_000;

/// How close the focus may come to an inner edge before the window moves.
pub const MARGIN_CHARS: usize = 8_000;

/// How far back from a cut inside a very long paragraph to look for a
/// space to cut at instead.
const CUT_SEARCH: usize = 400;

/// A style the document view draws over a range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpanStyle {
    /// A heading of this level (1 to 6).
    Heading(u8),
    /// A link.
    Link,
    /// Code, inline or a block.
    Code,
    /// A block quote.
    Quote,
    /// Bold text.
    Bold,
    /// Italic text.
    Italic,
    /// Underlined text.
    Underline,
    /// Struck-through text.
    Strikethrough,
}

/// A styled range (document positions).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StyledSpan {
    /// The chars.
    pub range: CharRange,
    /// The style.
    pub style: SpanStyle,
}

/// The window's place in the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextWindow {
    /// The chars in the window.
    pub range: CharRange,
    /// The document's length when the window was made.
    pub doc_len: usize,
}

/// Snaps `pos` back to just after a space within [`CUT_SEARCH`] chars, so a
/// cut inside a long paragraph does not split a word.
fn snap_to_space(doc: &Document, pos: usize, floor: usize) -> usize {
    let text = doc.text();
    let lo = pos.saturating_sub(CUT_SEARCH).max(floor);
    (lo..pos)
        .rev()
        .find(|&i| text.char(i).is_whitespace())
        .map_or(pos, |i| i + 1)
}

impl TextWindow {
    /// A window of about `size` chars around `focus`, a third of it before
    /// the focus, aligned to line starts where the lines allow.
    pub fn around(doc: &Document, focus: CharPos, size: usize) -> TextWindow {
        let len = doc.len_chars();
        let size = size.max(1);
        if len <= size {
            return TextWindow {
                range: CharRange::new(0, len),
                doc_len: len,
            };
        }
        let focus = focus.0.min(len);
        let want_start = focus.saturating_sub(size / 3);
        let line_start = doc.line_range(doc.line_of(CharPos(want_start))).start.0;
        let start = if want_start - line_start > size / 3 {
            // A very long line: cut inside it.
            snap_to_space(doc, want_start, line_start)
        } else {
            line_start
        };
        let want_end = (start + size).min(len);
        let end = if want_end >= len {
            len
        } else {
            let line = doc.line_range(doc.line_of(CharPos(want_end)));
            // Include the line's break, so the window ends a paragraph.
            let line_end = (line.end.0 + 1).min(len);
            if line_end - want_end > size / 3 {
                snap_to_space(doc, want_end, start + 1)
            } else {
                line_end
            }
        };
        TextWindow {
            range: CharRange::new(start, end.max(start)),
            doc_len: len,
        }
    }

    /// True when the window should be rebuilt to show `focus`: it is outside
    /// the window, or within [`MARGIN_CHARS`] of an inner edge.
    pub fn needs_move(&self, focus: CharPos) -> bool {
        let f = focus.0;
        let r = self.range;
        if f < r.start.0 || f > r.end.0 {
            return true;
        }
        let near_start = r.start.0 > 0 && f < r.start.0 + MARGIN_CHARS.min(r.len() / 4);
        let near_end = r.end.0 < self.doc_len && f + MARGIN_CHARS.min(r.len() / 4) > r.end.0;
        near_start || near_end
    }
}

/// The window's paragraphs, with heading levels from the document's
/// markers.
pub fn window_paragraphs(doc: &Document, window: CharRange) -> Vec<Paragraph> {
    let text = doc.slice(window);
    let mut paras = runs::paragraphs(window.start, &text);
    // A window cut inside a line ends a paragraph without its break.
    let index = doc.marker_index();
    let mut headings = index
        .starting_in(window)
        .iter()
        .filter(|m| m.kind == MarkerKind::Heading)
        .peekable();
    for p in &mut paras {
        let end = p.start.0 + p.span_chars();
        while let Some(h) = headings.peek() {
            if h.range.start.0 < p.start.0 {
                headings.next();
            } else if h.range.start.0 < end.max(p.start.0 + 1) {
                p.heading = Some(h.level.clamp(1, 6));
                break;
            } else {
                break;
            }
        }
    }
    paras
}

/// The styled spans that touch the window.
pub fn window_spans(doc: &Document, window: CharRange) -> Vec<StyledSpan> {
    let mut out = Vec::new();
    // Markers are sorted by start; those starting before the window can
    // still reach into it, so look from the first line's markers on. Block
    // markers (headings, quotes, code blocks) are short enough in practice;
    // scan the ones starting within one window's size before it.
    let from = CharPos(window.start.0.saturating_sub(WINDOW_CHARS));
    let scan = CharRange::new(from.0, window.end.0);
    for m in doc.marker_index().starting_in(scan) {
        if m.range.end.0 <= window.start.0 || m.range.is_empty() {
            continue;
        }
        let style = match m.kind {
            MarkerKind::Heading => SpanStyle::Heading(m.level.clamp(1, 6)),
            MarkerKind::Link => SpanStyle::Link,
            MarkerKind::Code => SpanStyle::Code,
            MarkerKind::Quote => SpanStyle::Quote,
            MarkerKind::Bold => SpanStyle::Bold,
            MarkerKind::Italic => SpanStyle::Italic,
            MarkerKind::Underline => SpanStyle::Underline,
            MarkerKind::Strikethrough => SpanStyle::Strikethrough,
            _ => continue,
        };
        let range = CharRange::new(
            m.range.start.0.max(window.start.0),
            m.range.end.0.min(window.end.0),
        );
        if !range.is_empty() {
            out.push(StyledSpan { range, style });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc_of_lines(n: usize, line: &str) -> Document {
        let mut s = String::new();
        for i in 0..n {
            s.push_str(&format!("{i:05} {line}\n"));
        }
        Document::from_plain_text(&s)
    }

    #[test]
    fn small_documents_are_one_window() {
        let doc = Document::from_plain_text("short\ntext");
        let w = TextWindow::around(&doc, CharPos(3), 1000);
        assert_eq!(w.range, CharRange::new(0, 10));
        assert!(!w.needs_move(CharPos(0)));
        assert!(!w.needs_move(CharPos(10)));
    }

    #[test]
    fn windows_align_to_lines_around_the_focus() {
        let doc = doc_of_lines(10_000, "some words on a line");
        let focus = CharPos(100_000);
        let w = TextWindow::around(&doc, focus, 20_000);
        assert!(w.range.start.0 <= focus.0 && focus.0 < w.range.end.0);
        // Starts at a line start and ends after a break.
        assert_eq!(
            doc.line_range(doc.line_of(w.range.start)).start,
            w.range.start
        );
        assert_eq!(doc.char_at(CharPos(w.range.end.0 - 1)), Some('\n'));
        let len = w.range.len();
        assert!((19_000..=21_000).contains(&len), "{len}");
        assert!(!w.needs_move(focus));
        assert!(w.needs_move(CharPos(w.range.end.0 + 5)));
        assert!(w.needs_move(CharPos(w.range.start.0 + 10)));
    }

    #[test]
    fn a_huge_line_is_cut_at_spaces() {
        let doc = Document::from_plain_text(&"word ".repeat(100_000));
        let w = TextWindow::around(&doc, CharPos(250_000), 30_000);
        assert!(w.range.len() <= 31_000);
        assert!(w.range.start.0 > 0);
        // Cuts land just after a space.
        assert_eq!(doc.char_at(CharPos(w.range.start.0 - 1)), Some(' '));
        assert_eq!(doc.char_at(CharPos(w.range.end.0 - 1)), Some(' '));
    }

    #[test]
    fn paragraphs_know_their_heading_levels() {
        use textweaver_app::text::{DocumentMeta, Marker};
        let text = "Title\nBody text.\nSub\nMore.";
        let markers = vec![
            Marker::new(MarkerKind::Heading, CharRange::new(0, 5)).with_level(1),
            Marker::new(MarkerKind::Heading, CharRange::new(17, 20)).with_level(2),
        ];
        let doc = Document::new(DocumentMeta::default(), text.into(), markers);
        let paras = window_paragraphs(&doc, doc.full_range());
        let levels: Vec<Option<u8>> = paras.iter().map(|p| p.heading).collect();
        assert_eq!(levels, vec![Some(1), None, Some(2), None]);
        let spans = window_spans(&doc, CharRange::new(3, 19));
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].range, CharRange::new(3, 5));
        assert_eq!(spans[1].range, CharRange::new(17, 19));
    }
}
