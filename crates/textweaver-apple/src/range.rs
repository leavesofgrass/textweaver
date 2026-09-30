//! Word ranges: from Apple's UTF-16 `NSRange`s to UTF-8 byte ranges.
//!
//! Apple's speech callbacks (`willSpeakWord`, `willSpeakRangeOfSpeechString`)
//! report ranges in UTF-16 code units of the `NSString` that was spoken.
//! textweaver's `RawEvent::Word` carries UTF-8 byte ranges of
//! `Utterance::text`, so every range goes through [`Utf16Index`].
//!
//! Eloquence reports sub-token pieces: "Dr" for "Dr.", "9" and "30" for
//! "9:30", "a" and "m" for "a.m.". [`word_extent`] widens a piece to the
//! written word that contains it, with surrounding punctuation trimmed, and
//! [`WordTracker`] drops a repeat of the word just reported, so the highlight
//! moves once per written word ("Dr", "9:30", "a.m") whatever the engine's
//! tokenization. Every piece lies inside one written word (measured on macOS
//! 14 and 15), so widening never merges two words.

use std::ops::Range;

/// UTF-16 code-unit offsets to UTF-8 byte offsets for one string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Utf16Index {
    /// For each UTF-16 unit (and one past the end), the byte offset of the
    /// char containing it.
    starts: Vec<u32>,
    /// For each UTF-16 unit, true when it is the second (low) surrogate of a
    /// pair, that is, inside a char rather than at its start.
    inside: Vec<bool>,
    len_bytes: u32,
}

impl Utf16Index {
    /// Builds the index for `text`.
    pub fn new(text: &str) -> Self {
        let mut starts = Vec::with_capacity(text.len() + 1);
        let mut inside = Vec::with_capacity(text.len() + 1);
        for (byte, c) in text.char_indices() {
            let b = to_u32(byte);
            for unit in 0..c.len_utf16() {
                starts.push(b);
                inside.push(unit > 0);
            }
        }
        let len_bytes = to_u32(text.len());
        starts.push(len_bytes);
        inside.push(false);
        Utf16Index {
            starts,
            inside,
            len_bytes,
        }
    }

    /// Length of the string in UTF-16 code units.
    pub fn utf16_len(&self) -> usize {
        self.starts.len() - 1
    }

    /// Byte offset for UTF-16 offset `unit`, rounded down to the start of
    /// its char and clamped to the end of the string.
    pub fn floor_byte(&self, unit: usize) -> u32 {
        self.starts.get(unit).copied().unwrap_or(self.len_bytes)
    }

    /// Byte offset for UTF-16 offset `unit`, rounded up to the end of its
    /// char when it falls inside a surrogate pair, and clamped.
    pub fn ceil_byte(&self, unit: usize) -> u32 {
        match self.inside.get(unit) {
            Some(true) => self.floor_byte(unit + 1),
            Some(false) => self.floor_byte(unit),
            None => self.len_bytes,
        }
    }

    /// The UTF-8 byte range for the UTF-16 range `location..location +
    /// length`, widened to whole chars and clamped to the string. `None`
    /// when the range is empty or starts past the end (including
    /// `NSNotFound`).
    pub fn byte_range(&self, location: usize, length: usize) -> Option<Range<u32>> {
        if location >= self.utf16_len() || length == 0 {
            return None;
        }
        let start = self.floor_byte(location);
        let end = self.ceil_byte(location.saturating_add(length));
        (end > start).then_some(start..end)
    }
}

/// Characters that separate written words besides whitespace.
fn is_separator(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\u{2014}' | '\u{2013}' | '\u{2026}' | '/')
}

/// Punctuation trimmed from the ends of a written word.
fn is_edge_punctuation(c: char) -> bool {
    matches!(
        c,
        '.' | ','
            | ';'
            | ':'
            | '!'
            | '?'
            | '"'
            | '\''
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '\u{201C}'
            | '\u{201D}'
            | '\u{2018}'
            | '\u{2019}'
            | '\u{00AB}'
            | '\u{00BB}'
            | '\u{00BF}'
            | '\u{00A1}'
    )
}

/// Widens the byte range `piece` of `text` to the written word containing
/// it: the run of characters between separators (whitespace, dashes,
/// ellipses, slashes), with punctuation trimmed from both ends. Returns
/// `piece` unchanged when it is not on char boundaries, or when trimming
/// would leave nothing (a piece that is only punctuation).
pub fn word_extent(text: &str, piece: Range<u32>) -> Range<u32> {
    let (Ok(s), Ok(e)) = (usize::try_from(piece.start), usize::try_from(piece.end)) else {
        return piece;
    };
    if s >= e || e > text.len() || !text.is_char_boundary(s) || !text.is_char_boundary(e) {
        return piece;
    }
    let mut start = s;
    for (i, c) in text[..s].char_indices().rev() {
        if is_separator(c) {
            break;
        }
        start = i;
    }
    let mut end = e;
    for (i, c) in text[e..].char_indices() {
        if is_separator(c) {
            break;
        }
        end = e + i + c.len_utf8();
    }
    let word = &text[start..end];
    let trimmed_front = word.trim_start_matches(is_edge_punctuation);
    let trimmed = trimmed_front.trim_end_matches(is_edge_punctuation);
    if trimmed.is_empty() {
        return piece;
    }
    let new_start = start + (word.len() - trimmed_front.len());
    let new_end = new_start + trimmed.len();
    // A piece that lies wholly in trimmed punctuation keeps its own range.
    if new_end <= s || new_start >= e {
        return piece;
    }
    to_u32(new_start)..to_u32(new_end)
}

/// Turns an engine's word callbacks for one utterance into word ranges:
/// maps UTF-16 to UTF-8, widens sub-token pieces to written words, and
/// drops a repeat of the word just reported.
#[derive(Clone, Debug)]
pub struct WordTracker {
    index: Utf16Index,
    last: Option<Range<u32>>,
}

impl WordTracker {
    /// A tracker for the spoken `text`.
    pub fn new(text: &str) -> Self {
        WordTracker {
            index: Utf16Index::new(text),
            last: None,
        }
    }

    /// True once the last written word of `text` has been reported (or when
    /// `text` has no words): the engine has reached the end.
    pub fn reached_end(&self, text: &str) -> bool {
        let body = text
            .trim_end()
            .trim_end_matches(|c: char| is_edge_punctuation(c) || is_separator(c));
        if body.trim().is_empty() {
            return true;
        }
        self.last
            .as_ref()
            .is_some_and(|r| r.end as usize >= body.len())
    }

    /// The word range to report for an engine range in UTF-16 units, or
    /// `None` when the range is empty, out of bounds, or the same word as the
    /// previous report.
    pub fn word(&mut self, text: &str, location: usize, length: usize) -> Option<Range<u32>> {
        let piece = self.index.byte_range(location, length)?;
        let word = word_extent(text, piece);
        if self.last.as_ref() == Some(&word) {
            return None;
        }
        self.last = Some(word.clone());
        Some(word)
    }
}

/// Spreads runs of words that share one audio offset across the time up to
/// the next word's offset, in proportion to their lengths.
///
/// `AVSpeechSynthesizer` on macOS 14 sometimes delivers several word
/// callbacks after the same buffer, so they share a sample offset; without
/// this the highlight would jump across them at once. `words` holds
/// `(offset, byte range)` in order; a run at the end, with no later offset,
/// is left as it is. Offsets never decrease.
pub fn spread_ties(words: &mut [(u64, Range<u32>)]) {
    let mut i = 0;
    while i < words.len() {
        let at = words[i].0;
        let mut j = i + 1;
        while j < words.len() && words[j].0 == at {
            j += 1;
        }
        if j - i > 1 && j < words.len() && words[j].0 > at {
            let span = words[j].0 - at;
            let lens: Vec<u64> = words[i..j]
                .iter()
                .map(|(_, r)| u64::from(r.end.saturating_sub(r.start)).max(1))
                .collect();
            let total: u64 = lens.iter().sum();
            let mut before = 0;
            for (k, len) in lens.iter().enumerate() {
                words[i + k].0 = at + span * before / total;
                before += len;
            }
        }
        i = j;
    }
}

/// Word marker offsets as sample frames.
///
/// `AVSpeechSynthesisMarker.byteSampleOffset` is documented only as an
/// offset "into the associated audio buffer"; its unit (bytes or sample
/// frames) is not stated. A word starts inside the audio, so when the
/// largest offset is beyond the frames received (`total_frames`), the
/// offsets are bytes and are divided by `frame_bytes`; otherwise they are
/// frames already. Offsets are clamped to the audio. `None` when the
/// offsets go backwards, which would mean they count from the start of each
/// buffer rather than of the audio: the caller then uses the delegate's
/// word callbacks instead.
pub fn marker_samples(raw: &[u64], total_frames: u64, frame_bytes: u32) -> Option<Vec<u64>> {
    if raw.windows(2).any(|p| p[1] < p[0]) {
        return None;
    }
    let max = raw.iter().copied().max().unwrap_or(0);
    let divisor = if max > total_frames && frame_bytes > 1 {
        u64::from(frame_bytes)
    } else {
        1
    };
    Some(
        raw.iter()
            .map(|&r| (r / divisor).min(total_frames))
            .collect(),
    )
}

/// Word times for an exported file, `(byte range, ms)` in order, or none
/// when they carry no information: two or more words that all share one
/// time (word callbacks that arrived after the audio, as on macOS 14)
/// would put every word cue at one moment, so export is better off sharing
/// the sentence's measured time among the words itself.
pub fn file_words(words: &[(Range<u32>, u32)]) -> Vec<(Range<u32>, u32)> {
    let all_tied = words.len() > 1 && words.iter().all(|(_, ms)| *ms == words[0].1);
    if all_tied {
        return Vec::new();
    }
    words.to_vec()
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UTF-16 range of `needle`'s `nth` occurrence in `text`.
    fn utf16_range(text: &str, needle: &str, nth: usize) -> (usize, usize) {
        let byte = text
            .match_indices(needle)
            .nth(nth)
            .map(|(i, _)| i)
            .expect("needle present");
        let loc = text[..byte].encode_utf16().count();
        (loc, needle.encode_utf16().count())
    }

    fn slice(text: &str, r: Range<u32>) -> &str {
        &text[r.start as usize..r.end as usize]
    }

    #[test]
    fn ascii_ranges_map_one_to_one() {
        let t = "Dr. Smith opened";
        let ix = Utf16Index::new(t);
        assert_eq!(ix.utf16_len(), t.len());
        assert_eq!(ix.byte_range(4, 5), Some(4..9));
        assert_eq!(slice(t, ix.byte_range(4, 5).unwrap()), "Smith");
    }

    #[test]
    fn multibyte_ranges_map_to_utf8_bytes() {
        let t = "Café crème, naïve résumé.";
        let ix = Utf16Index::new(t);
        for w in ["Café", "crème", "naïve", "résumé"] {
            let (loc, len) = utf16_range(t, w, 0);
            let r = ix.byte_range(loc, len).unwrap();
            assert_eq!(slice(t, r), w);
        }
    }

    #[test]
    fn surrogate_pairs_widen_to_whole_chars() {
        // U+1D11E MUSICAL SYMBOL G CLEF: two UTF-16 units, four UTF-8 bytes.
        let t = "a \u{1D11E} b";
        let ix = Utf16Index::new(t);
        assert_eq!(ix.utf16_len(), 6);
        // The whole pair.
        assert_eq!(slice(t, ix.byte_range(2, 2).unwrap()), "\u{1D11E}");
        // Only the high surrogate: widened to the whole char.
        assert_eq!(slice(t, ix.byte_range(2, 1).unwrap()), "\u{1D11E}");
        // Starting on the low surrogate: rounded down to the char start.
        assert_eq!(slice(t, ix.byte_range(3, 1).unwrap()), "\u{1D11E}");
        // Emoji after it.
        let t2 = "go \u{1F600} now";
        let ix2 = Utf16Index::new(t2);
        let (loc, len) = utf16_range(t2, "now", 0);
        assert_eq!(slice(t2, ix2.byte_range(loc, len).unwrap()), "now");
    }

    #[test]
    fn out_of_range_and_empty_ranges() {
        let ix = Utf16Index::new("abc");
        assert_eq!(ix.byte_range(3, 1), None);
        assert_eq!(ix.byte_range(1, 0), None);
        // NSNotFound is NSIntegerMax.
        assert_eq!(ix.byte_range(isize::MAX as usize, 1), None);
        // Overlong length is clamped.
        assert_eq!(ix.byte_range(1, 100), Some(1..3));
        assert_eq!(Utf16Index::new("").byte_range(0, 1), None);
    }

    #[test]
    fn eloquence_sub_tokens_widen_to_written_words() {
        let t = "Dr. Smith opened the library at 9:30 a.m. Café crème.";
        let ix = Utf16Index::new(t);
        let widen = |needle: &str, nth: usize| {
            let (loc, len) = utf16_range(t, needle, nth);
            slice(t, word_extent(t, ix.byte_range(loc, len).unwrap())).to_string()
        };
        assert_eq!(widen("Dr", 0), "Dr");
        assert_eq!(widen("9", 0), "9:30");
        assert_eq!(widen("30", 0), "9:30");
        assert_eq!(widen("a", 2), "a.m"); // the "a" of "a.m."
        assert_eq!(widen("m", 1), "a.m"); // "m" of "a.m." (after "Smith")
        assert_eq!(widen("crème", 0), "crème");
        assert_eq!(widen("library", 0), "library");
    }

    #[test]
    fn widening_trims_quotes_and_brackets_and_splits_at_dashes() {
        let t = "He said \u{201C}(yes),\u{201D} then\u{2014}later and/or";
        let ix = Utf16Index::new(t);
        let widen = |needle: &str| {
            let (loc, len) = utf16_range(t, needle, 0);
            slice(t, word_extent(t, ix.byte_range(loc, len).unwrap())).to_string()
        };
        assert_eq!(widen("yes"), "yes");
        assert_eq!(widen("then"), "then");
        assert_eq!(widen("later"), "later");
        assert_eq!(widen("and"), "and");
        assert_eq!(widen("or"), "or");
    }

    #[test]
    fn punctuation_only_pieces_keep_their_range() {
        let t = "wait ... go";
        assert_eq!(word_extent(t, 5..8), 5..8);
        // Not on a char boundary: unchanged.
        let t2 = "é";
        assert_eq!(word_extent(t2, 1..2), 1..2);
        // Out of bounds: unchanged.
        assert_eq!(word_extent("ab", 1..9), 1..9);
    }

    #[test]
    fn ties_spread_up_to_the_next_offset() {
        let mut w = vec![
            (0u64, 0u32..3u32),
            (100, 4..7),
            (100, 8..11),
            (100, 12..18),
            (400, 19..22),
            (500, 23..25),
            (500, 26..28),
        ];
        spread_ties(&mut w);
        let at: Vec<u64> = w.iter().map(|x| x.0).collect();
        // The run 4..7, 8..11, 12..18 (3 + 3 + 6 bytes) shares 100..400.
        assert_eq!(at, [0, 100, 175, 250, 400, 500, 500]);
        assert!(at.windows(2).all(|p| p[0] <= p[1]));
        let mut none: Vec<(u64, Range<u32>)> = Vec::new();
        spread_ties(&mut none);
        let mut one = vec![(7u64, 0u32..1u32)];
        spread_ties(&mut one);
        assert_eq!(one[0].0, 7);
    }

    #[test]
    fn tracker_knows_when_the_last_word_was_reported() {
        let t = "Second sentence.";
        let mut tracker = WordTracker::new(t);
        assert!(!tracker.reached_end(t));
        tracker.word(t, 0, 6);
        assert!(!tracker.reached_end(t));
        tracker.word(t, 7, 8);
        assert!(tracker.reached_end(t));
        // Trailing quotes and whitespace do not count as words.
        let q = "He said \u{201C}yes.\u{201D}  ";
        let mut tracker = WordTracker::new(q);
        let (loc, len) = utf16_range(q, "yes", 0);
        tracker.word(q, loc, len);
        assert!(tracker.reached_end(q));
        assert!(WordTracker::new("...").reached_end("..."));
    }

    #[test]
    fn tracker_reports_each_written_word_once_in_order() {
        let t = "Dr. Smith opened the library at 9:30 a.m. Café crème, naïve résumé.";
        // The pieces Eloquence Reed reported on macOS 15 (probe2).
        let pieces = [
            ("Dr", 0),
            ("Smith", 0),
            ("opened", 0),
            ("the", 0),
            ("library", 0),
            ("at", 0),
            ("9", 0),
            ("30", 0),
            ("a", 2),
            ("m", 1),
            ("Café", 0),
            ("crème", 0),
            ("naïve", 0),
            ("résumé", 0),
        ];
        let mut tracker = WordTracker::new(t);
        let words: Vec<String> = pieces
            .iter()
            .filter_map(|(p, nth)| {
                let (loc, len) = utf16_range(t, p, *nth);
                tracker.word(t, loc, len)
            })
            .map(|r| slice(t, r).to_string())
            .collect();
        assert_eq!(
            words,
            [
                "Dr", "Smith", "opened", "the", "library", "at", "9:30", "a.m", "Café", "crème",
                "naïve", "résumé"
            ]
        );
    }

    #[test]
    fn marker_offsets_in_frames_or_bytes() {
        // Frames already: kept as they are.
        assert_eq!(
            marker_samples(&[0, 100, 100, 790], 800, 4),
            Some(vec![0, 100, 100, 790])
        );
        // Bytes (float32 mono): beyond the 1000 frames, so divided by 4.
        assert_eq!(
            marker_samples(&[0, 400, 3600], 1000, 4),
            Some(vec![0, 100, 900])
        );
        // Offsets that go backwards count from each buffer: not used.
        assert_eq!(marker_samples(&[0, 300, 20], 1000, 4), None);
        assert_eq!(marker_samples(&[], 10, 4), Some(vec![]));
    }

    #[test]
    fn file_words_drop_times_that_all_tie() {
        let spread = vec![(0..3, 0), (4..9, 250)];
        assert_eq!(file_words(&spread), spread);
        assert!(file_words(&[(0..3, 900), (4..9, 900)]).is_empty());
        let one = vec![(0..5, 0)];
        assert_eq!(file_words(&one), one);
    }
}
