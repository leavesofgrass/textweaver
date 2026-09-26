//! Splitting an utterance into index-marked words.
//!
//! The host puts DECtalk's `[:index mark]` for word `i` immediately before
//! the word and sends the word with its trailing whitespace as text, so
//! DECtalk still sees the whole sentence (its own text rules for numbers
//! and abbreviations work across the marks). A word is a maximal run of
//! non-whitespace. Its highlight range is that run with leading and
//! trailing punctuation trimmed ("(hello)," highlights `hello`); a run with
//! no letters or digits ("—") keeps its full range. These are the ECI
//! backend's rules (ADR-0007), so both engines highlight the same words.

use std::ops::Range;

use crate::protocol::Piece;

/// One word of an utterance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    /// Byte range of the whole whitespace-delimited run in the utterance text.
    pub run: Range<usize>,
    /// Byte range to highlight (the run without surrounding punctuation).
    pub highlight: Range<u32>,
}

/// The words of `text`, in order.
pub fn words(text: &str) -> Vec<Word> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        match (c.is_whitespace(), start) {
            (false, None) => start = Some(i),
            (true, Some(s)) => {
                out.push(Word {
                    run: s..i,
                    highlight: trim_punctuation(text, s..i),
                });
                start = None;
            }
            _ => {}
        }
    }
    out
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn trim_punctuation(text: &str, run: Range<usize>) -> Range<u32> {
    let s = &text[run.clone()];
    let first = s.char_indices().find(|(_, c)| c.is_alphanumeric());
    let last = s.char_indices().rev().find(|(_, c)| c.is_alphanumeric());
    match (first, last) {
        (Some((a, _)), Some((b, c))) => to_u32(run.start + a)..to_u32(run.start + b + c.len_utf8()),
        _ => to_u32(run.start)..to_u32(run.end),
    }
}

/// The pieces to send for `text`: `Index(i)` before word `i`, then the text
/// from the word's start to the next word's start (so whitespace is
/// preserved). Leading whitespace is dropped.
pub fn pieces(text: &str, words: &[Word]) -> Vec<Piece> {
    let mut out = Vec::with_capacity(words.len() * 2);
    for (i, w) in words.iter().enumerate() {
        let end = words.get(i + 1).map_or(text.len(), |n| n.run.start);
        out.push(Piece::Index(to_u32(i)));
        out.push(Piece::Text(text[w.run.start..end].to_string()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hl<'a>(text: &'a str, ws: &[Word]) -> Vec<&'a str> {
        ws.iter()
            .map(|w| &text[w.highlight.start as usize..w.highlight.end as usize])
            .collect()
    }

    #[test]
    fn splits_on_whitespace_and_trims_punctuation() {
        let t = "Dr. Smith opened the library at 9:30 a.m. (really),  she said — \"yes\".";
        let ws = words(t);
        assert_eq!(
            hl(t, &ws),
            [
                "Dr", "Smith", "opened", "the", "library", "at", "9:30", "a.m", "really", "she",
                "said", "—", "yes"
            ]
        );
        assert_eq!(&t[ws[8].run.clone()], "(really),");
    }

    #[test]
    fn multibyte_ranges_are_byte_offsets_on_char_boundaries() {
        let t = "Café crème, naïve 日本語 ok";
        let ws = words(t);
        assert_eq!(hl(t, &ws), ["Café", "crème", "naïve", "日本語", "ok"]);
        assert_eq!(ws[0].highlight, 0..5);
        for w in &ws {
            assert!(t.is_char_boundary(w.highlight.start as usize));
            assert!(t.is_char_boundary(w.highlight.end as usize));
        }
    }

    #[test]
    fn pieces_interleave_marks_and_keep_every_byte_after_the_first_word() {
        let t = "  One two\nthree ";
        let p = pieces(t, &words(t));
        assert_eq!(
            p,
            [
                Piece::Index(0),
                Piece::Text("One ".into()),
                Piece::Index(1),
                Piece::Text("two\n".into()),
                Piece::Index(2),
                Piece::Text("three ".into()),
            ]
        );
    }

    #[test]
    fn empty_and_blank_text_have_no_words() {
        assert!(words("").is_empty());
        assert!(words(" \n\t ").is_empty());
        assert!(pieces("  ", &[]).is_empty());
    }

    proptest::proptest! {
        #[test]
        fn words_are_ordered_nonempty_and_within_runs(t in "\\PC{0,40}") {
            let ws = words(&t);
            let mut prev = 0usize;
            for w in &ws {
                proptest::prop_assert!(w.run.start >= prev);
                proptest::prop_assert!(w.run.start < w.run.end);
                proptest::prop_assert!(w.highlight.start as usize >= w.run.start);
                proptest::prop_assert!(w.highlight.end as usize <= w.run.end);
                proptest::prop_assert!(w.highlight.start < w.highlight.end);
                prev = w.run.end;
            }
            let joined: String = pieces(&t, &ws)
                .into_iter()
                .filter_map(|p| match p { Piece::Text(s) => Some(s), Piece::Index(_) => None })
                .collect();
            proptest::prop_assert_eq!(joined.as_str(), t.trim_start());
        }
    }
}
