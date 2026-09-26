//! Mapping SAPI word positions (UTF-16 code units) to UTF-8 byte ranges of
//! the utterance text, and filtering the word stream.
//!
//! SAPI reports each `SPEI_WORD_BOUNDARY` as a character position and length
//! in UTF-16 code units of the text it was given. The host sends the
//! utterance text converted losslessly from UTF-8, so every UTF-16 position
//! corresponds to exactly one place in `Utterance::text`.
//!
//! What the engines measured on 2026-09-25 do (ADR-0009):
//! - Microsoft David and Zira expand "9:30 a.m." into several events on the
//!   same source range; [`WordFilter`] passes the first and drops the
//!   repeats, so the highlight does not flicker.
//! - eSpeak and the VW voices report sub-token ranges ("Dr" of "Dr.", "9"
//!   and "30" of "9:30"); those pass through unchanged.
//!
//! Ranges are snapped outward to character boundaries (a position inside a
//! surrogate pair covers the whole character), clamped to the text, and
//! trimmed of surrounding whitespace. A zero-length event covers the word
//! that starts at its position.

use std::ops::Range;

/// UTF-16 to UTF-8 position table for one text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Utf16Map {
    /// For each UTF-16 code unit: the byte offset of the character that
    /// contains it. One more entry at the end: the text's byte length.
    byte_of_unit: Vec<u32>,
    /// For each UTF-16 code unit: true when a character starts there.
    starts_char: Vec<bool>,
    text: String,
}

impl Utf16Map {
    /// Builds the table for `text`.
    pub fn new(text: &str) -> Self {
        let mut byte_of_unit = Vec::with_capacity(text.len() + 1);
        let mut starts_char = Vec::with_capacity(text.len() + 1);
        for (b, c) in text.char_indices() {
            let b = u32::try_from(b).unwrap_or(u32::MAX);
            for i in 0..c.len_utf16() {
                byte_of_unit.push(b);
                starts_char.push(i == 0);
            }
        }
        byte_of_unit.push(u32::try_from(text.len()).unwrap_or(u32::MAX));
        starts_char.push(true);
        Utf16Map {
            byte_of_unit,
            starts_char,
            text: text.to_owned(),
        }
    }

    /// Length of the text in UTF-16 code units.
    pub fn utf16_len(&self) -> u32 {
        u32::try_from(self.byte_of_unit.len() - 1).unwrap_or(u32::MAX)
    }

    /// The UTF-8 byte range for `len` UTF-16 units starting at `start`,
    /// snapped outward to character boundaries, trimmed of whitespace.
    /// `None` when the range lies outside the text or covers only
    /// whitespace.
    pub fn byte_range(&self, start: u32, len: u32) -> Option<Range<u32>> {
        let n = self.byte_of_unit.len() - 1;
        let s = start as usize;
        if s >= n {
            return None;
        }
        let b0 = self.byte_of_unit[s] as usize;
        let b1 = if len == 0 {
            // The word that starts here: up to the next whitespace.
            let rest = &self.text[b0..];
            let skip = rest.len() - rest.trim_start().len();
            let word = &rest[skip..];
            b0 + skip + word.find(char::is_whitespace).unwrap_or(word.len())
        } else {
            let mut e = s.saturating_add(len as usize).min(n);
            while !self.starts_char[e] {
                e += 1;
            }
            self.byte_of_unit[e] as usize
        };
        let slice = &self.text[b0..b1];
        let lead = slice.len() - slice.trim_start().len();
        let trimmed = slice.trim();
        if trimmed.is_empty() {
            return None;
        }
        let start = b0 + lead;
        let end = start + trimmed.len();
        debug_assert!(self.text.is_char_boundary(start) && self.text.is_char_boundary(end));
        Some(u32::try_from(start).ok()?..u32::try_from(end).ok()?)
    }
}

/// Drops repeated and backward word events so the highlight only moves
/// forward.
#[derive(Clone, Debug, Default)]
pub struct WordFilter {
    last: Option<Range<u32>>,
}

impl WordFilter {
    /// A filter for one utterance.
    pub fn new() -> Self {
        Self::default()
    }

    /// True when `range` should be emitted: it is not the range just
    /// emitted, and it does not start before it.
    pub fn accept(&mut self, range: &Range<u32>) -> bool {
        if let Some(last) = &self.last
            && (*last == *range || range.start < last.start)
        {
            return false;
        }
        self.last = Some(range.clone());
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// UTF-16 position and length of `word`'s `nth` occurrence in `text`.
    fn utf16_of(text: &str, word: &str) -> (u32, u32) {
        let b = text.find(word).unwrap();
        let start = text[..b].encode_utf16().count() as u32;
        (start, word.encode_utf16().count() as u32)
    }

    fn slice(text: &str, r: Range<u32>) -> &str {
        &text[r.start as usize..r.end as usize]
    }

    #[test]
    fn ascii_positions_map_one_to_one() {
        let t = "Dr. Smith opened the library.";
        let m = Utf16Map::new(t);
        assert_eq!(m.utf16_len(), t.len() as u32);
        let (s, l) = utf16_of(t, "Smith");
        assert_eq!(slice(t, m.byte_range(s, l).unwrap()), "Smith");
    }

    #[test]
    fn multibyte_text_maps_to_byte_ranges() {
        let t = "Café crème, naïve résumé.";
        let m = Utf16Map::new(t);
        for w in ["Café", "crème", "naïve", "résumé"] {
            let (s, l) = utf16_of(t, w);
            assert_eq!(slice(t, m.byte_range(s, l).unwrap()), w, "{w}");
        }
    }

    #[test]
    fn astral_characters_take_two_units() {
        let t = "I 😀 you 𝔸lpha end";
        let m = Utf16Map::new(t);
        let (s, l) = utf16_of(t, "you");
        assert_eq!(slice(t, m.byte_range(s, l).unwrap()), "you");
        let (s, l) = utf16_of(t, "𝔸lpha");
        assert_eq!(slice(t, m.byte_range(s, l).unwrap()), "𝔸lpha");
        // A range that starts or ends inside a surrogate pair covers the
        // whole character.
        let (s, _) = utf16_of(t, "😀");
        assert_eq!(slice(t, m.byte_range(s + 1, 1).unwrap()), "😀");
        assert_eq!(slice(t, m.byte_range(s, 1).unwrap()), "😀");
    }

    #[test]
    fn sub_token_ranges_pass_through() {
        let t = "Dr. Smith at 9:30 a.m.";
        let m = Utf16Map::new(t);
        assert_eq!(slice(t, m.byte_range(0, 2).unwrap()), "Dr");
        let (s, _) = utf16_of(t, "9:30");
        assert_eq!(slice(t, m.byte_range(s, 1).unwrap()), "9");
        assert_eq!(slice(t, m.byte_range(s + 2, 2).unwrap()), "30");
    }

    #[test]
    fn repeated_events_on_one_range_are_emitted_once() {
        // David: "9:30 a.m." gives four events on the same source range.
        let t = "at 9:30 a.m. today";
        let m = Utf16Map::new(t);
        let (s, l) = utf16_of(t, "9:30 a.m.");
        let mut f = WordFilter::new();
        let mut out = Vec::new();
        for (s, l) in [(0, 2), (s, l), (s, l), (s, l), (s, l), utf16_of(t, "today")] {
            let r = m.byte_range(s, l).unwrap();
            if f.accept(&r) {
                out.push(slice(t, r));
            }
        }
        assert_eq!(out, ["at", "9:30 a.m.", "today"]);
    }

    #[test]
    fn backward_events_are_dropped() {
        let mut f = WordFilter::new();
        assert!(f.accept(&(4..9)));
        assert!(!f.accept(&(0..3)));
        assert!(f.accept(&(4..6)), "a sub-range at the same start moves on");
        assert!(f.accept(&(10..12)));
    }

    #[test]
    fn whitespace_is_trimmed_and_out_of_range_is_none() {
        let t = "  hello   world ";
        let m = Utf16Map::new(t);
        assert_eq!(slice(t, m.byte_range(0, 8).unwrap()), "hello");
        assert_eq!(m.byte_range(7, 2), None, "whitespace only");
        assert_eq!(m.byte_range(99, 1), None);
        // A length past the end is clamped.
        let (s, _) = utf16_of(t, "world");
        assert_eq!(slice(t, m.byte_range(s, 99).unwrap()), "world");
        assert_eq!(Utf16Map::new("").byte_range(0, 1), None);
    }

    #[test]
    fn zero_length_events_cover_the_word_at_their_position() {
        let t = "naïve résumé";
        let m = Utf16Map::new(t);
        let (s, _) = utf16_of(t, "résumé");
        assert_eq!(slice(t, m.byte_range(s, 0).unwrap()), "résumé");
        assert_eq!(slice(t, m.byte_range(0, 0).unwrap()), "naïve");
    }

    proptest! {
        #[test]
        fn every_range_is_on_char_boundaries(text in "\\PC{0,40}", start in 0u32..60, len in 0u32..20) {
            let m = Utf16Map::new(&text);
            if let Some(r) = m.byte_range(start, len) {
                prop_assert!(r.start < r.end);
                prop_assert!(r.end as usize <= text.len());
                prop_assert!(text.is_char_boundary(r.start as usize));
                prop_assert!(text.is_char_boundary(r.end as usize));
                let s = &text[r.start as usize..r.end as usize];
                prop_assert_eq!(s.trim(), s);
            }
        }

        #[test]
        fn each_word_maps_back_to_itself(words in proptest::collection::vec("[a-zé😀ß]{1,6}", 1..8)) {
            let text = words.join(" ");
            let m = Utf16Map::new(&text);
            let mut unit = 0u32;
            for w in &words {
                let l = w.encode_utf16().count() as u32;
                let r = m.byte_range(unit, l).unwrap();
                prop_assert_eq!(&text[r.start as usize..r.end as usize], w.as_str());
                unit += l + 1;
            }
        }
    }
}
