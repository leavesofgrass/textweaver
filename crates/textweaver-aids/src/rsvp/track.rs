//! The words an RSVP session steps through, prepared once from a document.

use textweaver_core::{CharPos, CharRange, Unit};
use textweaver_text::{Document, segments_in};
use unicode_segmentation::UnicodeSegmentation;

/// Most punctuation chars shown with a word on either side ("(see)," not
/// a run of dashes).
const MAX_ATTACHED: usize = 8;

/// The optimal recognition point: the index of the grapheme the eye should
/// fixate in a word of `len` graphemes. The common RSVP table (Spritz,
/// OpenSpritz): first letter for 1, second for 2 to 5, third for 6 to 9,
/// fourth for 10 to 13, fifth for longer words.
pub fn optimal_recognition_point(len: usize) -> usize {
    match len {
        0 | 1 => 0,
        2..=5 => 1,
        6..=9 => 2,
        10..=13 => 3,
        _ => 4,
    }
}

const SENTENCE_END: u8 = 1;
const PARAGRAPH_END: u8 = 2;
const CLAUSE_END: u8 = 4;

/// One word of a [`WordTrack`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TrackWord {
    /// The word unit (canonical chars), as navigation and speech see it.
    pub(crate) word: CharRange,
    /// The word with the punctuation attached to it (`"(see),"`).
    pub(crate) display: CharRange,
    /// Byte range of the display text in the track's text arena.
    text: (u32, u32),
    /// Byte range of the pivot grapheme, relative to the display text.
    pivot: (u32, u32),
    /// Graphemes in the word unit (not counting attached punctuation).
    pub(crate) graphemes: u32,
    /// Index of the word's sentence in [`WordTrack::sentence_starts`].
    pub(crate) sentence: u32,
    /// Index of the word's paragraph in [`WordTrack::paragraph_starts`].
    pub(crate) paragraph: u32,
    flags: u8,
}

impl TrackWord {
    pub(crate) fn sentence_end(&self) -> bool {
        self.flags & SENTENCE_END != 0
    }
    pub(crate) fn paragraph_end(&self) -> bool {
        self.flags & PARAGRAPH_END != 0
    }
    pub(crate) fn clause_end(&self) -> bool {
        self.flags & CLAUSE_END != 0
    }
}

/// The words of a document or range, with their display text, recognition
/// points, and sentence and paragraph boundaries.
///
/// Built once (about 20 ms per megabyte in a release build) and then
/// independent of the document, so an [`Rsvp`](super::Rsvp) session can
/// live in app state. Rebuild it after the document is edited.
#[derive(Clone, Debug, Default)]
pub struct WordTrack {
    arena: String,
    words: Vec<TrackWord>,
    sentence_starts: Vec<u32>,
    paragraph_starts: Vec<u32>,
}

fn is_clause_punct(c: char) -> bool {
    matches!(
        c,
        ',' | ';' | ':' | '\u{2014}' | '\u{2013}' | ')' | '(' | '"' | '\u{201d}'
    )
}

impl WordTrack {
    /// Every word of `doc`.
    pub fn from_document(doc: &Document) -> Self {
        Self::from_range(doc, doc.full_range())
    }

    /// The words of `doc` that intersect `range`.
    pub fn from_range(doc: &Document, range: CharRange) -> Self {
        let range = range.clamp_to(doc.len_chars());
        let words: Vec<CharRange> = segments_in(doc, Unit::Word, range)
            .into_iter()
            .filter(|w| !w.is_empty())
            .collect();
        let sentences = segments_in(doc, Unit::Sentence, range);
        let paragraphs = segments_in(doc, Unit::Paragraph, range);
        let rope = doc.text();
        let len = doc.len_chars();

        let mut track = WordTrack {
            arena: String::new(),
            words: Vec::with_capacity(words.len()),
            sentence_starts: Vec::new(),
            paragraph_starts: Vec::new(),
        };
        let mut prev_display_end = 0usize;
        let (mut si, mut pi) = (0usize, 0usize);
        let mut last_sentence: Option<usize> = None;
        let mut last_paragraph: Option<usize> = None;
        for (i, w) in words.iter().enumerate() {
            // Attach leading punctuation back to whitespace or the previous
            // word's display, and trailing punctuation up to whitespace or
            // the next word.
            let mut a = w.start.0;
            while a > prev_display_end
                && w.start.0 - a < MAX_ATTACHED
                && !rope.char(a - 1).is_whitespace()
            {
                a -= 1;
            }
            let next_start = words.get(i + 1).map_or(len, |n| n.start.0);
            let mut b = w.end.0;
            while b < next_start && b - w.end.0 < MAX_ATTACHED && !rope.char(b).is_whitespace() {
                b += 1;
            }
            prev_display_end = b;

            let text_start = track.arena.len();
            for chunk in rope.slice(a..b).chunks() {
                track.arena.push_str(chunk);
            }
            let text_end = track.arena.len();
            let display = &track.arena[text_start..text_end];

            // The pivot is found in the word unit, after any leading
            // punctuation.
            let lead_bytes: usize = display
                .chars()
                .take(w.start.0 - a)
                .map(char::len_utf8)
                .sum();
            let word_bytes: usize = display[lead_bytes..]
                .chars()
                .take(w.len())
                .map(char::len_utf8)
                .sum();
            let core = &display[lead_bytes..lead_bytes + word_bytes];
            let graphemes = core.graphemes(true).count();
            let orp = optimal_recognition_point(graphemes);
            let (pivot_start, pivot_len) = core
                .grapheme_indices(true)
                .nth(orp)
                .map_or((0, 0), |(i, g)| (i, g.len()));
            let pivot = (
                to_u32(lead_bytes + pivot_start),
                to_u32(lead_bytes + pivot_start + pivot_len),
            );

            let mut flags = 0u8;
            if display[lead_bytes + word_bytes..]
                .chars()
                .any(is_clause_punct)
            {
                flags |= CLAUSE_END;
            }

            while si < sentences.len() && sentences[si].end <= w.start {
                si += 1;
            }
            while pi < paragraphs.len() && paragraphs[pi].end <= w.start {
                pi += 1;
            }
            // A word outside every unit (should not happen) joins the
            // previous sentence or paragraph.
            let s_key = si.min(sentences.len().saturating_sub(1));
            let p_key = pi.min(paragraphs.len().saturating_sub(1));
            if last_sentence != Some(s_key) || track.sentence_starts.is_empty() {
                track.sentence_starts.push(to_u32(i));
                last_sentence = Some(s_key);
            }
            if last_paragraph != Some(p_key) || track.paragraph_starts.is_empty() {
                track.paragraph_starts.push(to_u32(i));
                last_paragraph = Some(p_key);
            }

            track.words.push(TrackWord {
                word: *w,
                display: CharRange::new(a, b),
                text: (to_u32(text_start), to_u32(text_end)),
                pivot,
                graphemes: to_u32(graphemes),
                sentence: to_u32(track.sentence_starts.len() - 1),
                paragraph: to_u32(track.paragraph_starts.len() - 1),
                flags,
            });
        }
        // Mark the last word of each sentence and paragraph.
        let n = track.words.len();
        for i in 0..n {
            let (s, p) = (track.words[i].sentence, track.words[i].paragraph);
            let next = track.words.get(i + 1).map(|w| (w.sentence, w.paragraph));
            if next.is_none_or(|(ns, _)| ns != s) {
                track.words[i].flags |= SENTENCE_END;
            }
            if next.is_none_or(|(_, np)| np != p) {
                track.words[i].flags |= PARAGRAPH_END | SENTENCE_END;
            }
        }
        track
    }

    /// A track over plain text.
    pub fn from_text(text: &str) -> Self {
        Self::from_document(&Document::from_plain_text(text))
    }

    /// Number of words.
    pub fn len(&self) -> usize {
        self.words.len()
    }

    /// True when there are no words.
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// Number of sentences.
    pub fn sentence_count(&self) -> usize {
        self.sentence_starts.len()
    }

    /// Number of paragraphs.
    pub fn paragraph_count(&self) -> usize {
        self.paragraph_starts.len()
    }

    pub(crate) fn get(&self, i: usize) -> Option<&TrackWord> {
        self.words.get(i)
    }

    /// The display text of word `i` (with attached punctuation).
    pub fn text(&self, i: usize) -> Option<&str> {
        let w = self.words.get(i)?;
        self.arena.get(w.text.0 as usize..w.text.1 as usize)
    }

    /// Word `i` split at its recognition point: `(before, pivot, after)`.
    pub fn split(&self, i: usize) -> Option<(&str, &str, &str)> {
        let w = self.words.get(i)?;
        let t = self.text(i)?;
        let (a, b) = (w.pivot.0 as usize, w.pivot.1 as usize);
        Some((t.get(..a)?, t.get(a..b)?, t.get(b..)?))
    }

    /// The canonical range of word `i` (the word unit).
    pub fn word_range(&self, i: usize) -> Option<CharRange> {
        self.words.get(i).map(|w| w.word)
    }

    /// The canonical range of word `i` with its attached punctuation.
    pub fn display_range(&self, i: usize) -> Option<CharRange> {
        self.words.get(i).map(|w| w.display)
    }

    /// Index of the first word of sentence `s`.
    pub(crate) fn sentence_start(&self, s: u32) -> Option<usize> {
        self.sentence_starts.get(s as usize).map(|&i| i as usize)
    }

    /// Index of the first word of paragraph `p`.
    pub(crate) fn paragraph_start(&self, p: u32) -> Option<usize> {
        self.paragraph_starts.get(p as usize).map(|&i| i as usize)
    }

    /// The word containing `pos`, else the first word after it, else the
    /// last word (Star's restore rule: first word at or after).
    pub fn word_at_or_after(&self, pos: CharPos) -> Option<usize> {
        if self.words.is_empty() {
            return None;
        }
        let i = self.words.partition_point(|w| w.word.end <= pos);
        Some(i.min(self.words.len() - 1))
    }
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orp_table() {
        let got: Vec<usize> = [0, 1, 2, 5, 6, 9, 10, 13, 14, 40]
            .into_iter()
            .map(optimal_recognition_point)
            .collect();
        assert_eq!(got, [0, 0, 1, 1, 2, 2, 3, 3, 4, 4]);
    }

    #[test]
    fn words_carry_punctuation_and_pivots() {
        let t = WordTrack::from_text("Hello, world! (See) well\u{2014}known readability.");
        let texts: Vec<&str> = (0..t.len()).map(|i| t.text(i).unwrap()).collect();
        assert_eq!(
            texts,
            [
                "Hello,",
                "world!",
                "(See)",
                "well\u{2014}",
                "known",
                "readability."
            ]
        );
        assert_eq!(t.split(0), Some(("H", "e", "llo,")));
        assert_eq!(t.split(2), Some(("(S", "e", "e)")));
        assert_eq!(t.split(5), Some(("rea", "d", "ability.")));
        assert_eq!(t.word_range(0), Some(CharRange::new(0, 5)));
        assert_eq!(t.display_range(0), Some(CharRange::new(0, 6)));
    }

    #[test]
    fn sentence_and_paragraph_ends() {
        let t = WordTrack::from_text("Dr. Smith came. He sat, then left.\n\nNew part here");
        assert_eq!(t.sentence_count(), 3);
        assert_eq!(t.paragraph_count(), 2);
        let ends: Vec<(bool, bool, bool)> = (0..t.len())
            .map(|i| {
                let w = t.get(i).unwrap();
                (w.clause_end(), w.sentence_end(), w.paragraph_end())
            })
            .collect();
        // Dr. Smith came. | He sat, then left. | New part here
        assert_eq!(
            ends,
            [
                (false, false, false),
                (false, false, false),
                (false, true, false),
                (false, false, false),
                (true, false, false),
                (false, false, false),
                (false, true, true),
                (false, false, false),
                (false, false, false),
                (false, true, true),
            ]
        );
    }

    #[test]
    fn lookup_by_position() {
        let t = WordTrack::from_text("one two  three");
        assert_eq!(t.word_at_or_after(CharPos(0)), Some(0));
        assert_eq!(t.word_at_or_after(CharPos(3)), Some(1));
        assert_eq!(t.word_at_or_after(CharPos(5)), Some(1));
        assert_eq!(t.word_at_or_after(CharPos(7)), Some(2));
        assert_eq!(t.word_at_or_after(CharPos(99)), Some(2));
        assert_eq!(WordTrack::default().word_at_or_after(CharPos(0)), None);
    }

    #[test]
    fn range_track_and_empty() {
        let doc = Document::from_plain_text("alpha beta gamma delta");
        let t = WordTrack::from_range(&doc, CharRange::new(6, 16));
        assert_eq!(t.len(), 2);
        assert_eq!(t.text(0), Some("beta"));
        assert!(WordTrack::from_text("  ...  ").is_empty());
    }
}
