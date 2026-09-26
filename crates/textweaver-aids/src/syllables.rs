//! Syllable splitting for display: `read·a·bil·i·ty`.
//!
//! Star split words with Pyphen (Hunspell's en-US hyphenation dictionary,
//! `star/syllables.py`) and showed the result only on screen; speech and the
//! word map used the untouched text. textweaver does the same, and adds what
//! Star lacked: an [`OffsetMap`] from the display text back to canonical
//! positions (ADR-0005), so a highlight on the display text lands on the
//! right chars and a click or caret in it maps back exactly.
//!
//! Splitting is rule-based, with no dictionary and no dependency: vowel
//! groups are syllable centres (a silent final `e` is not one; a final
//! consonant plus `le` is), consonants between them split by the usual
//! English spelling rules (one consonant goes with the next syllable, two
//! are split unless they are a digraph such as `th` or `ph`, longer runs keep
//! a blend such as `str` or `dr` together), and a few suffixes (`-ing`,
//! `-able`, `-ability`, `-ness`, `-ment`, `-ful`, `-less`, `-ly`) split off
//! whole. It is an approximation: good enough to help decode long words,
//! not a dictionary. TeX-pattern hyphenation (for example the `hypher`
//! crate) is a follow-up if Jon wants dictionary-quality splits.
//!
//! In the map, every original char is a `Literal` span and every inserted
//! separator an `Inserted` span anchored at the syllable that follows it.

use std::ops::Range;

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
    /// Fewest letters before the first break (Pyphen's default, 2).
    pub left_min: usize,
    /// Fewest letters after the last break (Pyphen's default, 2).
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

/// Two consonants that spell one sound and start a syllable together.
const DIGRAPHS: &[&str] = &["th", "ch", "sh", "ph", "wh", "gh"];
/// Consonant pairs that can start a syllable (kept together in longer runs).
const ONSETS: &[&str] = &[
    "bl", "br", "cl", "cr", "dr", "fl", "fr", "gl", "gr", "pl", "pr", "sc", "sk", "sl", "sm", "sn",
    "sp", "st", "sw", "tr", "tw", "wr", "th", "ch", "sh", "ph", "wh",
];
/// Three-consonant onsets.
const ONSETS3: &[&str] = &["str", "spr", "scr", "spl", "thr", "chr", "shr", "squ"];
/// Suffixes split off whole, with their own internal breaks (char offsets).
const SUFFIXES: &[(&str, &[usize])] = &[
    ("ability", &[1, 4, 5]),
    ("ibility", &[1, 4, 5]),
    ("able", &[1]),
    ("ible", &[1]),
    ("ing", &[]),
    ("ness", &[]),
    ("ment", &[]),
    ("ful", &[]),
    ("less", &[]),
];

fn is_vowel_letter(c: char) -> bool {
    matches!(
        c,
        'a' | 'e'
            | 'i'
            | 'o'
            | 'u'
            | 'à'
            | 'á'
            | 'â'
            | 'ä'
            | 'è'
            | 'é'
            | 'ê'
            | 'ë'
            | 'ì'
            | 'í'
            | 'î'
            | 'ï'
            | 'ò'
            | 'ó'
            | 'ô'
            | 'ö'
            | 'ù'
            | 'ú'
            | 'û'
            | 'ü'
    )
}

/// Break points (char offsets) inside an all-letter word, by the rules
/// above, before applying the minimum lengths.
fn rule_breaks(w: &[char]) -> Vec<usize> {
    let n = w.len();
    if n < 2 {
        return Vec::new();
    }
    // Which letters are vowels here.
    let mut v: Vec<bool> = (0..n)
        .map(|i| {
            let c = w[i];
            if c == 'y' {
                // "y" is a vowel except at the start or before a vowel.
                i > 0 && !w.get(i + 1).copied().is_some_and(is_vowel_letter)
            } else if c == 'u' && i > 0 && w[i - 1] == 'q' {
                false // "qu" is a consonant sound
            } else {
                is_vowel_letter(c)
            }
        })
        .collect();
    let cons = |i: usize| i < n && !v[i];
    // A final consonant + "le" (+ "s" or "d") is a syllable: ta-ble, bub-bles.
    let le_at = if n >= 4 && w[n - 2..] == ['l', 'e'] && cons(n - 3) {
        Some(n - 3)
    } else if n >= 5
        && w[n - 3..n - 1] == ['l', 'e']
        && matches!(w[n - 1], 's' | 'd')
        && cons(n - 4)
    {
        Some(n - 4)
    } else {
        None
    };
    if le_at.is_none() && n >= 3 {
        // Silent final e ("make"), and silent e in "-es" and "-ed" endings
        // ("makes", "jumped") unless the ending is sounded ("horses", "wanted").
        if w[n - 1] == 'e' && cons(n - 2) && v[..n - 2].iter().any(|&x| x) {
            v[n - 1] = false;
        } else if n >= 4 && w[n - 2] == 'e' && cons(n - 3) && v[..n - 3].iter().any(|&x| x) {
            let silent = match w[n - 1] {
                's' => !matches!(w[n - 3], 's' | 'x' | 'z' | 'c' | 'g' | 'h'),
                'd' => !matches!(w[n - 3], 't' | 'd'),
                _ => false,
            };
            if silent {
                v[n - 2] = false;
            }
        }
    }
    // Vowel groups: (start, end) of each run of vowels.
    let mut nuclei: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < n {
        if v[i] {
            let s = i;
            while i < n && v[i] {
                i += 1;
            }
            nuclei.push((s, i));
        } else {
            i += 1;
        }
    }
    let mut breaks = Vec::new();
    for pair in nuclei.windows(2) {
        let (e, s) = (pair[0].1, pair[1].0);
        let cluster: String = w[e..s].iter().collect();
        let k = s - e;
        let b = if le_at.is_some_and(|l| l >= e && l < s) {
            le_at.unwrap_or(e) // break before the consonant of "-le"
        } else {
            match k {
                0 => e,
                1 if w[e] == 'x' => e + 1, // ex-am
                1 => e,                    // V-CV: pa-per
                2 if DIGRAPHS.contains(&cluster.as_str()) => e,
                2 if cluster == "ck" || cluster == "ng" => e + 2,
                2 => e + 1, // VC-CV: hap-py
                _ => {
                    let tail3: String = w[s - 3..s].iter().collect();
                    let tail2: String = w[s - 2..s].iter().collect();
                    if ONSETS3.contains(&tail3.as_str()) && k > 3 {
                        s - 3
                    } else if ONSETS.contains(&tail2.as_str()) {
                        s - 2
                    } else {
                        s - 1
                    }
                }
            }
        };
        if b > 0 && b < n {
            breaks.push(b);
        }
    }
    breaks
}

/// Break points (char offsets) in an all-letter word, suffixes first.
fn letter_breaks(word: &str, left_min: usize, right_min: usize) -> Vec<usize> {
    let lower: Vec<char> = word.chars().flat_map(char::to_lowercase).collect();
    if lower.len() != word.chars().count() {
        return Vec::new(); // a letter that lowercases to several; leave it
    }
    let n = lower.len();
    let mut breaks: Vec<usize> = Vec::new();
    let mut stem_len = n;
    for (suffix, inner) in SUFFIXES {
        let sl = suffix.chars().count();
        if n < sl + 3 {
            continue;
        }
        let tail: String = lower[n - sl..].iter().collect();
        if tail != *suffix {
            continue;
        }
        let stem = &lower[..n - sl];
        if !stem.iter().any(|&c| is_vowel_letter(c) || c == 'y') {
            continue;
        }
        // A doubled consonant before a vowel suffix splits: run-ning.
        let doubled = stem.len() >= 2
            && stem[stem.len() - 1] == stem[stem.len() - 2]
            && !is_vowel_letter(stem[stem.len() - 1])
            && is_vowel_letter(lower[n - sl]);
        stem_len = if doubled { n - sl - 1 } else { n - sl };
        breaks.extend(rule_breaks(&lower[..stem_len]));
        breaks.push(stem_len);
        breaks.extend(inner.iter().map(|&i| n - sl + i));
        break;
    }
    if stem_len == n {
        breaks = rule_breaks(&lower);
        // A sounded "-ed" after a consonant cluster keeps the cluster:
        // want-ed, start-ed (the rules alone give wan-ted).
        if n >= 5
            && lower[n - 2..] == ['e', 'd']
            && matches!(lower[n - 3], 't' | 'd')
            && !is_vowel_letter(lower[n - 4])
        {
            for b in &mut breaks {
                if *b == n - 3 {
                    *b = n - 2;
                }
            }
        }
    }
    breaks.sort_unstable();
    breaks.dedup();
    breaks.retain(|&b| b >= left_min.max(1) && b + right_min.max(1) <= n);
    breaks
}

/// The syllables of one word, as slices of it. A word that cannot or need
/// not be split comes back whole. Words containing digits are never split;
/// in a word with an apostrophe or other joiner (`wouldn't`), each run of
/// letters is split on its own.
pub fn split_word<'a>(word: &'a str, opts: &SyllableOptions) -> Vec<&'a str> {
    if word.chars().count() < opts.min_word_len.max(1) || word.chars().any(|c| c.is_numeric()) {
        return vec![word];
    }
    let mut cuts: Vec<usize> = Vec::new(); // byte offsets in `word`
    let mut run_start: Option<usize> = None;
    let mut push_run = |a: usize, b: usize| {
        let run = &word[a..b];
        if run.chars().count() < opts.min_word_len.max(1) {
            return;
        }
        let offsets: Vec<usize> = run.char_indices().map(|(i, _)| i).collect();
        for c in letter_breaks(run, opts.left_min, opts.right_min) {
            if let Some(&o) = offsets.get(c) {
                cuts.push(a + o);
            }
        }
    };
    for (i, c) in word.char_indices() {
        if c.is_alphabetic() {
            run_start.get_or_insert(i);
        } else if let Some(a) = run_start.take() {
            push_run(a, i);
        }
    }
    if let Some(a) = run_start {
        push_run(a, word.len());
    }
    let mut out = Vec::with_capacity(cuts.len() + 1);
    let mut prev = 0;
    for b in cuts {
        if b > prev {
            out.push(&word[prev..b]);
            prev = b;
        }
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

    fn show(w: &str) -> String {
        split_word(w, &opts()).join("\u{b7}")
    }

    #[test]
    fn rule_based_splits() {
        let cases = [
            ("readability", "read\u{b7}a\u{b7}bil\u{b7}i\u{b7}ty"),
            ("reading", "read\u{b7}ing"),
            ("running", "run\u{b7}ning"),
            ("hello", "hel\u{b7}lo"),
            ("syllables", "syl\u{b7}la\u{b7}bles"),
            ("table", "ta\u{b7}ble"),
            ("understanding", "un\u{b7}der\u{b7}stand\u{b7}ing"),
            ("hyphenation", "hy\u{b7}phe\u{b7}na\u{b7}tion"),
            ("children", "chil\u{b7}dren"),
            ("instruct", "in\u{b7}struct"),
            ("paper", "pa\u{b7}per"),
            ("happy", "hap\u{b7}py"),
            ("make", "make"),
            ("jumped", "jumped"),
            ("wanted", "want\u{b7}ed"),
            ("hopeful", "hope\u{b7}ful"),
            ("question", "ques\u{b7}tion"),
            ("beyond", "be\u{b7}yond"),
            // left_min 2 keeps a lone first letter attached.
            ("Education", "Edu\u{b7}ca\u{b7}tion"),
        ];
        let mut wrong = Vec::new();
        for (w, want) in cases {
            if show(w) != want {
                wrong.push((w, show(w), want));
            }
        }
        assert!(wrong.is_empty(), "(word, got, want): {wrong:#?}");
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
