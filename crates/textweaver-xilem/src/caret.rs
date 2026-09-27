//! Caret movement over the window's paragraphs that needs no layout: by
//! character (grapheme cluster), word, and paragraph. Moves by visual line
//! and page need the text layout and live in the document view.
//!
//! A paragraph's line break counts as one position after its last char, so
//! Right at the end of a paragraph lands on the next paragraph's start in
//! two steps, as in a native text control.

use textweaver_app::core::CharPos;
use unicode_segmentation::UnicodeSegmentation;

use crate::runs::Paragraph;

/// Index of the paragraph holding `pos` (the last one starting at or
/// before it), or 0.
pub fn paragraph_at(paras: &[Paragraph], pos: CharPos) -> usize {
    paras
        .partition_point(|p| p.start.0 <= pos.0)
        .saturating_sub(1)
}

/// Byte index in `text` of char offset `off` (clamped to the end).
pub fn byte_of(text: &str, off: usize) -> usize {
    text.char_indices().nth(off).map_or(text.len(), |(b, _)| b)
}

/// Char offset of byte index `b` in `text`.
pub fn char_of(text: &str, b: usize) -> usize {
    text[..b.min(text.len())].chars().count()
}

/// Char offset within its paragraph, clamped to the paragraph (not its
/// break).
fn local(p: &Paragraph, pos: CharPos) -> usize {
    pos.0.saturating_sub(p.start.0).min(p.len_chars())
}

/// The window's end: the end of the last paragraph, including its break.
pub fn window_end(paras: &[Paragraph]) -> CharPos {
    paras
        .last()
        .map_or(CharPos::ZERO, |p| CharPos(p.start.0 + p.span_chars()))
}

/// One character (grapheme cluster) forward.
pub fn next_char(paras: &[Paragraph], pos: CharPos) -> CharPos {
    let Some(p) = paras.get(paragraph_at(paras, pos)) else {
        return pos;
    };
    let off = pos.0.saturating_sub(p.start.0);
    let len = p.len_chars();
    if off >= len {
        // On the break (or past it): the next paragraph's start.
        return CharPos(
            (p.start.0 + p.span_chars())
                .max(pos.0)
                .min(window_end(paras).0),
        );
    }
    let b = byte_of(&p.text, off);
    let g = p.text[b..].graphemes(true).next().unwrap_or("");
    CharPos(p.start.0 + off + g.chars().count())
}

/// One character (grapheme cluster) back.
pub fn prev_char(paras: &[Paragraph], pos: CharPos) -> CharPos {
    let i = paragraph_at(paras, pos);
    let Some(p) = paras.get(i) else {
        return pos;
    };
    let off = local(p, pos);
    if pos.0 <= p.start.0 {
        // At a paragraph's start: onto the previous paragraph's break.
        return match i.checked_sub(1).and_then(|j| paras.get(j)) {
            Some(q) => CharPos(q.start.0 + q.len_chars()),
            None => pos,
        };
    }
    if pos.0 > p.start.0 + p.len_chars() {
        return CharPos(p.start.0 + p.len_chars());
    }
    let b = byte_of(&p.text, off);
    let g = p.text[..b].graphemes(true).next_back().unwrap_or("");
    CharPos(p.start.0 + off - g.chars().count())
}

/// Byte indices of the word starts in `text` (segments with a
/// non-whitespace char).
fn word_starts(text: &str) -> impl Iterator<Item = usize> + '_ {
    text.split_word_bound_indices()
        .filter(|(_, w)| w.chars().any(|c| !c.is_whitespace()))
        .map(|(i, _)| i)
}

/// The next word start after `pos`; the start of the next paragraph that
/// has text; else the window's end.
pub fn next_word(paras: &[Paragraph], pos: CharPos) -> CharPos {
    let i = paragraph_at(paras, pos);
    for (j, p) in paras.iter().enumerate().skip(i) {
        let b = if j == i {
            byte_of(&p.text, local(p, pos))
        } else {
            0
        };
        let found = word_starts(&p.text).find(|&w| if j == i { w > b } else { w >= b });
        if let Some(w) = found {
            return CharPos(p.start.0 + char_of(&p.text, w));
        }
        if j > i && p.text.is_empty() {
            // A blank line is a stop, as in word processors.
            return p.start;
        }
    }
    window_end(paras)
}

/// The word start before `pos`; else the window's start.
pub fn prev_word(paras: &[Paragraph], pos: CharPos) -> CharPos {
    let i = paragraph_at(paras, pos);
    for j in (0..=i).rev() {
        let p = &paras[j];
        let limit = if j == i {
            byte_of(&p.text, local(p, pos))
        } else {
            p.text.len() + 1
        };
        if j == i && pos.0 <= p.start.0 {
            continue;
        }
        if let Some(w) = word_starts(&p.text).filter(|&w| w < limit).last() {
            return CharPos(p.start.0 + char_of(&p.text, w));
        }
        if j < i && p.text.is_empty() {
            return p.start;
        }
    }
    paras.first().map_or(CharPos::ZERO, |p| p.start)
}

/// The next paragraph's start (or the window's end).
pub fn next_paragraph(paras: &[Paragraph], pos: CharPos) -> CharPos {
    let i = paragraph_at(paras, pos);
    paras.get(i + 1).map_or(window_end(paras), |p| p.start)
}

/// The paragraph's start, or the previous paragraph's when already there.
pub fn prev_paragraph(paras: &[Paragraph], pos: CharPos) -> CharPos {
    let i = paragraph_at(paras, pos);
    match paras.get(i) {
        Some(p) if pos.0 > p.start.0 => p.start,
        _ => i
            .checked_sub(1)
            .and_then(|j| paras.get(j))
            .map_or(pos, |p| p.start),
    }
}

/// The text between two positions in the window (for the clipboard).
pub fn text_between(paras: &[Paragraph], a: CharPos, b: CharPos) -> String {
    let (a, b) = if a.0 <= b.0 { (a, b) } else { (b, a) };
    let mut out = String::new();
    for p in paras {
        let p_end = p.start.0 + p.span_chars();
        if p_end <= a.0 || p.start.0 >= b.0 {
            continue;
        }
        let s = a.0.saturating_sub(p.start.0);
        let e = (b.0 - p.start.0).min(p.span_chars());
        let len = p.len_chars();
        let text_end = e.min(len);
        if s < text_end {
            out.push_str(&p.text[byte_of(&p.text, s)..byte_of(&p.text, text_end)]);
        }
        if p.has_break && e > len {
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runs::paragraphs;

    fn paras(t: &str) -> Vec<Paragraph> {
        paragraphs(CharPos::ZERO, t)
    }

    #[test]
    fn characters_step_over_clusters_and_breaks() {
        let p = paras("ae\u{301}\nb");
        assert_eq!(next_char(&p, CharPos(0)), CharPos(1));
        assert_eq!(next_char(&p, CharPos(1)), CharPos(3));
        assert_eq!(next_char(&p, CharPos(3)), CharPos(4)); // over the break
        assert_eq!(next_char(&p, CharPos(4)), CharPos(5));
        assert_eq!(next_char(&p, CharPos(5)), CharPos(5)); // window end
        assert_eq!(prev_char(&p, CharPos(5)), CharPos(4));
        assert_eq!(prev_char(&p, CharPos(4)), CharPos(3));
        assert_eq!(prev_char(&p, CharPos(3)), CharPos(1));
        assert_eq!(prev_char(&p, CharPos(0)), CharPos(0));
    }

    #[test]
    fn words_move_to_word_starts_across_paragraphs() {
        let p = paras("One two.\n\nThree");
        assert_eq!(next_word(&p, CharPos(0)), CharPos(4));
        assert_eq!(next_word(&p, CharPos(4)), CharPos(7)); // the full stop
        assert_eq!(next_word(&p, CharPos(7)), CharPos(9)); // the blank line
        assert_eq!(next_word(&p, CharPos(9)), CharPos(10));
        assert_eq!(next_word(&p, CharPos(10)), CharPos(15));
        assert_eq!(prev_word(&p, CharPos(15)), CharPos(10));
        assert_eq!(prev_word(&p, CharPos(10)), CharPos(9));
        assert_eq!(prev_word(&p, CharPos(9)), CharPos(7));
        assert_eq!(prev_word(&p, CharPos(5)), CharPos(4));
        assert_eq!(prev_word(&p, CharPos(0)), CharPos(0));
    }

    #[test]
    fn paragraphs_move_to_starts() {
        let p = paras("ab\ncd\nef");
        assert_eq!(next_paragraph(&p, CharPos(1)), CharPos(3));
        assert_eq!(next_paragraph(&p, CharPos(7)), CharPos(8));
        assert_eq!(prev_paragraph(&p, CharPos(4)), CharPos(3));
        assert_eq!(prev_paragraph(&p, CharPos(3)), CharPos(0));
        assert_eq!(prev_paragraph(&p, CharPos(0)), CharPos(0));
    }

    #[test]
    fn clipboard_text_keeps_breaks() {
        let p = paras("ab\ncd\nef");
        assert_eq!(text_between(&p, CharPos(1), CharPos(4)), "b\nc");
        assert_eq!(text_between(&p, CharPos(7), CharPos(1)), "b\ncd\ne");
    }

    #[test]
    fn windows_that_start_later_work() {
        let p = paragraphs(CharPos(100), "xy\nz");
        assert_eq!(paragraph_at(&p, CharPos(50)), 0);
        assert_eq!(next_char(&p, CharPos(100)), CharPos(101));
        assert_eq!(prev_char(&p, CharPos(100)), CharPos(100));
        assert_eq!(window_end(&p), CharPos(104));
    }
}
