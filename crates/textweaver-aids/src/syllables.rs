//! Syllable splitting for display: `read·a·bil·i·ty`.
//!
//! Star split words with Pyphen (Hunspell's en-US hyphenation dictionary,
//! `star/syllables.py`) and showed the result only on screen; speech and the
//! word map used the untouched text. textweaver does the same, and adds what
//! Star lacked: an [`OffsetMap`] from the display text back to canonical
//! positions (ADR-0005), so a highlight on the display text lands on the
//! right chars and a click or caret in it maps back exactly.
//!
//! Splitting uses the `hypher` crate: Knuth-Liang hyphenation with the TeX
//! en-US patterns embedded as a compact automaton (27 KiB, no load time, no
//! dependencies, no unsafe code). Hyphenation points are where a word may be
//! broken at a line end, which for English is close to, but not exactly, the
//! spoken syllables; that is the same trade-off Star made with Pyphen.
//!
//! In the map, every original char is a `Literal` span and every inserted
//! separator an `Inserted` span anchored at the syllable that follows it.

use std::ops::Range;

use hypher::{Lang, hyphenate_bounded};
use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange, OffsetMap, SpanKind, SpokenBuilder};
use textweaver_text::Document;

use crate::util::{SkipSet, code_marker_ranges, text_skip_ranges, word_segments};

/// The middle dot Star used between syllables.
pub const MIDDOT: &str = "\u{b7}";

/// Options for [`split_text`], [`split_range`], and [`split_word`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyllableOptions {
    /// Shown between syllables. Star's default is a middle dot; a hyphen or
    /// a thin space also work.
    pub separator: String,
    /// Fewest chars before the first break. Pyphen's default, 2.
    pub left_min: usize,
    /// Fewest chars after the last break. Pyphen's default, 2 (TeX uses 3
    /// for English line breaking, which hides syllables like `i·ty`).
    pub right_min: usize,
    /// Words shorter than this (in chars) are never split.
    pub min_word_len: usize,
    /// Leave URLs and email addresses alone.
    pub skip_urls: bool,
    /// Leave code alone: `Code` markers, inline `` `code` ``, code-like tokens.
    pub skip_code: bool,
}

impl Default for SyllableOptions {
    fn default() -> Self {
        SyllableOptions {
            separator: MIDDOT.to_owned(),
            left_min: 2,
            right_min: 2,
            min_word_len: 4,
            skip_urls: true,
            skip_code: true,
        }
    }
}

/// The syllables of one word, as slices of it. A word that cannot or need
/// not be split comes back whole. Words containing digits are never split.
pub fn split_word<'a>(word: &'a str, opts: &SyllableOptions) -> Vec<&'a str> {
    if word.chars().count() < opts.min_word_len.max(1)
        || word.chars().any(|c| c.is_numeric())
        || !word.chars().all(char::is_alphabetic)
    {
        return split_compound(word, opts);
    }
    hyphenate_bounded(
        word,
        Lang::English,
        opts.left_min.max(1),
        opts.right_min.max(1),
    )
    .collect()
}

/// A word with apostrophes or other joiners (`don't`, `o'clock`): split each
/// alphabetic run on its own and keep the joiners with the run before them.
fn split_compound<'a>(word: &'a str, opts: &SyllableOptions) -> Vec<&'a str> {
    if word.chars().any(|c| c.is_numeric())
        || word.chars().count() < opts.min_word_len.max(1)
        || word.chars().all(char::is_alphabetic)
    {
        return vec![word];
    }
    let mut breaks: Vec<usize> = Vec::new();
    let mut run_start: Option<usize> = None;
    let push_run = |a: usize, b: usize, breaks: &mut Vec<usize>| {
        let run = &word[a..b];
        if run.chars().count() >= opts.min_word_len.max(1) {
            let mut at = a;
            let parts: Vec<&str> = hyphenate_bounded(
                run,
                Lang::English,
                opts.left_min.max(1),
                opts.right_min.max(1),
            )
            .collect();
            for p in &parts[..parts.len().saturating_sub(1)] {
                at += p.len();
                breaks.push(at);
            }
        }
    };
    for (i, c) in word.char_indices() {
        if c.is_alphabetic() {
            run_start.get_or_insert(i);
        } else if let Some(a) = run_start.take() {
            push_run(a, i, &mut breaks);
        }
    }
    if let Some(a) = run_start {
        push_run(a, word.len(), &mut breaks);
    }
    let mut out = Vec::with_capacity(breaks.len() + 1);
    let mut prev = 0;
    for b in breaks {
        out.push(&word[prev..b]);
        prev = b;
    }
    out.push(&word[prev..]);
    out
}

/// Display text with syllable separators, and the map back to the source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitText {
    /// The text to show.
    pub text: String,
    /// Maps byte ranges of [`SplitText::text`] to canonical char ranges.
    pub map: OffsetMap,
}

impl SplitText {
    /// The canonical chars behind display bytes `bytes` (for example, a
    /// mouse selection). Separators map to nothing; `None` when the range
    /// covers only separators.
    pub fn to_source(&self, bytes: Range<usize>) -> Option<CharRange> {
        let a = u32::try_from(bytes.start).ok()?;
        let b = u32::try_from(bytes.end).ok()?;
        self.map.to_source(&self.text, a..b)
    }

    /// The display byte where canonical position `pos` is shown: inside a
    /// word, the exact char; at a syllable start, the char after the
    /// separator. `None` past the end.
    pub fn to_display(&self, pos: CharPos) -> Option<usize> {
        self.map.to_spoken(&self.text, pos).map(|b| b as usize)
    }

    /// The display bytes to highlight for canonical range `range` (a spoken
    /// word, a find hit). Separators inside the range are included; one
    /// right after its end is not. `None` when nothing of `range` is shown.
    pub fn display_range(&self, range: CharRange) -> Option<Range<usize>> {
        if range.is_empty() {
            let b = self.to_display(range.start)?;
            return Some(b..b);
        }
        let mut start: Option<usize> = None;
        let mut end: Option<usize> = None;
        for s in self.map.spans() {
            if s.kind != SpanKind::Literal || !s.source.intersects(range) {
                continue;
            }
            let span_text = &self.text[s.spoken.start as usize..s.spoken.end as usize];
            let skip = range.start.0.saturating_sub(s.source.start.0);
            let take_end = range.end.0.min(s.source.end.0) - s.source.start.0;
            let byte_at = |n: usize| {
                span_text
                    .char_indices()
                    .nth(n)
                    .map_or(span_text.len(), |(i, _)| i)
            };
            let a = s.spoken.start as usize + byte_at(skip);
            let b = s.spoken.start as usize + byte_at(take_end);
            start.get_or_insert(a);
            end = Some(b);
        }
        Some(start?..end?)
    }
}

/// Splits every word of `text` (whose first char is at `base`) into
/// syllables for display.
pub fn split_text(text: &str, base: CharPos, opts: &SyllableOptions) -> SplitText {
    let skip = SkipSet::new(text_skip_ranges(text, base, opts.skip_urls, opts.skip_code));
    split_with_skip(text, base, opts, &skip)
}

/// Splits the words of `range` of `doc` for display, leaving `Code`
/// markers whole when [`SyllableOptions::skip_code`] is set.
pub fn split_range(doc: &Document, range: CharRange, opts: &SyllableOptions) -> SplitText {
    let range = range.clamp_to(doc.len_chars());
    let text = doc.slice(range);
    let mut skips = text_skip_ranges(&text, range.start, opts.skip_urls, opts.skip_code);
    if opts.skip_code {
        skips.extend(code_marker_ranges(doc, range));
    }
    split_with_skip(&text, range.start, opts, &SkipSet::new(skips))
}

fn split_with_skip(text: &str, base: CharPos, opts: &SyllableOptions, skip: &SkipSet) -> SplitText {
    let mut b = SpokenBuilder::new();
    // `done` is the byte of `text` copied so far; `pos` its canonical position.
    let mut done = 0usize;
    let mut pos = base;
    let sep = opts.separator.as_str();
    for (wb, word) in word_segments(text) {
        let pieces = split_word(word, opts);
        if pieces.len() < 2 {
            continue;
        }
        // Copy the text before the word.
        let before = &text[done..wb];
        b.push_literal(before, pos);
        pos = pos.saturating_add(before.chars().count());
        let word_chars = word.chars().count();
        let word_range = CharRange::new(pos, pos.saturating_add(word_chars));
        if skip.overlaps(word_range) {
            b.push_literal(word, pos);
        } else {
            let mut at = pos;
            for (i, piece) in pieces.iter().enumerate() {
                if i > 0 && !sep.is_empty() {
                    b.push_inserted(sep, at);
                }
                b.push_literal(piece, at);
                at = at.saturating_add(piece.chars().count());
            }
        }
        pos = word_range.end;
        done = wb + word.len();
    }
    b.push_literal(&text[done..], pos);
    let (text, map) = b.finish();
    SplitText { text, map }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> SyllableOptions {
        SyllableOptions::default()
    }

    #[test]
    fn splits_long_words() {
        let parts = split_word("readability", &opts());
        assert!(parts.len() >= 4, "{parts:?}");
        assert_eq!(parts.concat(), "readability");
        assert_eq!(split_word("hyphenation", &opts()).concat(), "hyphenation");
        assert_eq!(split_word("cat", &opts()), ["cat"]);
        assert_eq!(split_word("v2x9", &opts()), ["v2x9"]);
    }

    #[test]
    fn text_keeps_punctuation_digits_and_spaces() {
        let s = split_text("Hello, syllables! v2 3.14 --", CharPos(0), &opts());
        assert!(s.text.starts_with("Hel\u{b7}lo, "), "{}", s.text);
        assert!(s.text.contains("syl\u{b7}la\u{b7}bles!"), "{}", s.text);
        assert!(s.text.ends_with(" v2 3.14 --"));
        assert_eq!(s.text.replace(MIDDOT, ""), "Hello, syllables! v2 3.14 --");
        s.map.check_invariants(&s.text).unwrap();
    }

    #[test]
    fn map_points_back_to_source() {
        let src = "An extraordinary day";
        let s = split_text(src, CharPos(10), &opts());
        s.map.check_invariants(&s.text).unwrap();
        // The whole word "extraordinary" (chars 3..16 of src) maps back.
        let r = s.display_range(CharRange::new(13, 26)).unwrap();
        assert_eq!(s.text[r.clone()].replace(MIDDOT, ""), "extraordinary");
        assert_eq!(s.to_source(r), Some(CharRange::new(13, 26)));
        // A separator alone maps to nothing.
        let sep = s.text.find(MIDDOT).unwrap();
        assert_eq!(s.to_source(sep..sep + MIDDOT.len()), None);
        // Every source position round-trips.
        for i in 10..30 {
            let d = s.to_display(CharPos(i)).unwrap();
            let back = s.to_source(d..d + 1).unwrap();
            assert_eq!(back.start, CharPos(i), "pos {i}");
        }
        assert_eq!(s.to_display(CharPos(30)), None);
    }

    #[test]
    fn urls_and_code_stay_whole() {
        let s = split_text(
            "visit https://understanding.example/readability or `understanding`",
            CharPos(0),
            &opts(),
        );
        assert!(s.text.contains("https://understanding.example/readability"));
        assert!(s.text.contains("`understanding`"));
    }

    #[test]
    fn apostrophes_and_hyphens() {
        let s = split_text("wouldn't well-understood", CharPos(0), &opts());
        assert_eq!(s.text.replace(MIDDOT, ""), "wouldn't well-understood");
        assert!(s.text.contains("un\u{b7}der"), "{}", s.text);
        s.map.check_invariants(&s.text).unwrap();
    }

    #[test]
    fn custom_separator_and_empty_text() {
        let o = SyllableOptions {
            separator: "-".into(),
            ..opts()
        };
        assert_eq!(split_text("hello", CharPos(0), &o).text, "hel-lo");
        let e = split_text("", CharPos(0), &o);
        assert!(e.text.is_empty() && e.map.is_empty());
    }
}
