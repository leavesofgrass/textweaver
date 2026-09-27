//! The document window's text for the document view: its paragraphs, with
//! heading levels, and the styles over it, from the document's markers.
//!
//! The window itself is the app's [`DocWindow`] (Agent W3a, ADR-0024):
//! about [`WINDOW_UNITS`] UTF-16 units around the focus, aligned to
//! paragraphs, sliding while reading and recentring on jumps. Positions
//! stay document-absolute everywhere; only the window's text is laid out,
//! painted, and exposed to screen readers.
//!
//! [`DocWindow`]: textweaver_app::DocWindow

use textweaver_app::core::{CharPos, CharRange, MarkerKind};
use textweaver_app::text::Document;

use crate::runs::{self, Paragraph};

/// The GUI's window budget, in UTF-16 units. Chosen by measurement
/// (ADR-0027, "Measurements"): large enough that reading rarely moves it,
/// small enough that building its text runs stays well inside a frame.
/// The app's default is 500,000.
pub const WINDOW_UNITS: usize = 120_000;

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
    let from = CharPos(window.start.0.saturating_sub(WINDOW_UNITS));
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
