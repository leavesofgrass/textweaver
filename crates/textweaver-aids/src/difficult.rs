//! Difficult words: mark rare words so a reader can pre-scan dense text.
//!
//! Star (`star/vocab.py`) flagged words of four or more letters whose
//! `wordfreq` Zipf frequency was below 4.5. The engine here does the same
//! against any [`FrequencyList`], and marks every occurrence with its
//! canonical range instead of returning a set of lowercase strings.
//!
//! **No word list ships with textweaver yet.** `wordfreq`'s data is
//! CC BY-SA 4.0 (share-alike, compatible with textweaver's GPL-3.0 but not
//! permissive), SUBTLEX's terms are research-only, and Norvig's counts
//! derive from the LDC Web 1T corpus. The candidates with clean licences
//! (SCOWL's size levels, 12dicts' frequency tiers) are waiting for Jon's
//! approval to download and vendor; see `docs/adr/0022-reading-aids.md`. A
//! list can be loaded from a file today with [`FrequencyList::parse`].
//!
//! The Zipf scale: log10 of a word's frequency per billion words. "the" is
//! about 7.7, everyday words 5 to 6, uncommon words below 4, rare below 3.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange};
use textweaver_text::Document;

use crate::util::{ByteToPos, SkipSet, code_marker_ranges, text_skip_ranges, word_segments};

/// Word frequencies on the Zipf scale, keyed by lowercase word.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrequencyList {
    zipf: HashMap<String, f32>,
}

/// A line of a frequency file that could not be read.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("line {line}: {reason}")]
pub struct ParseError {
    /// 1-based line number.
    pub line: usize,
    /// What was wrong.
    pub reason: String,
}

impl FrequencyList {
    /// An empty list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses a frequency file. Each non-empty line that does not start
    /// with `#` is either `word<TAB>zipf` (a number, as `wordfreq` exports)
    /// or a bare word; bare words are taken as a list ranked from most to
    /// least common, and get [`FrequencyList::zipf_for_rank`]. The first
    /// occurrence of a word wins.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let mut list = FrequencyList::new();
        let mut rank = 0usize;
        for (i, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split('\t');
            let word = parts.next().unwrap_or_default().trim();
            if word.is_empty() {
                return Err(ParseError {
                    line: i + 1,
                    reason: "missing word".into(),
                });
            }
            rank += 1;
            let zipf = match parts.next().map(str::trim) {
                Some(z) => z
                    .parse::<f32>()
                    .ok()
                    .filter(|z| z.is_finite())
                    .ok_or_else(|| ParseError {
                        line: i + 1,
                        reason: format!("{z:?} is not a number"),
                    })?,
                None => Self::zipf_for_rank(rank),
            };
            list.zipf.entry(word.to_lowercase()).or_insert(zipf);
        }
        Ok(list)
    }

    /// Builds a list from `(word, zipf)` pairs.
    pub fn from_pairs<'a>(pairs: impl IntoIterator<Item = (&'a str, f32)>) -> Self {
        let mut list = FrequencyList::new();
        for (w, z) in pairs {
            list.zipf.entry(w.to_lowercase()).or_insert(z);
        }
        list
    }

    /// An estimate of the Zipf value of the word at `rank` (1 = most
    /// common) by Zipf's law: about 8 − log10(rank). Rank 1 gets 8, rank
    /// 1,000 gets 5, rank 30,000 about 3.5.
    pub fn zipf_for_rank(rank: usize) -> f32 {
        (8.0 - (rank.max(1) as f64).log10()) as f32
    }

    /// Number of words.
    pub fn len(&self) -> usize {
        self.zipf.len()
    }

    /// True when the list is empty.
    pub fn is_empty(&self) -> bool {
        self.zipf.is_empty()
    }

    /// The Zipf value of `word` (any case), or `None` when not listed.
    pub fn zipf(&self, word: &str) -> Option<f32> {
        if let Some(z) = self.zipf.get(word) {
            return Some(*z);
        }
        self.zipf.get(&word.to_lowercase()).copied()
    }
}

/// Options for [`difficult_text`] and [`difficult_range`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DifficultOptions {
    /// Words below this Zipf value are difficult. Star used 4.5.
    pub threshold: f32,
    /// Shorter words (in chars) are never marked. Star used 4.
    pub min_len: usize,
    /// Mark words missing from the list (likely rare, but also names and
    /// typos). Off by default.
    pub mark_unknown: bool,
    /// Skip words that start with a capital letter mid-sentence (names).
    pub skip_capitalized: bool,
}

impl Default for DifficultOptions {
    fn default() -> Self {
        DifficultOptions {
            threshold: 4.5,
            min_len: 4,
            mark_unknown: false,
            skip_capitalized: true,
        }
    }
}

fn is_difficult(
    word: &str,
    prev: Option<char>,
    list: &FrequencyList,
    o: &DifficultOptions,
) -> bool {
    if word.chars().count() < o.min_len
        || !word
            .chars()
            .all(|c| c.is_alphabetic() || c == '\'' || c == '\u{2019}')
    {
        return false;
    }
    if o.skip_capitalized && word.chars().next().is_some_and(char::is_uppercase) {
        // A capital after sentence punctuation is just a sentence start.
        let sentence_start =
            prev.is_none_or(|c| matches!(c, '.' | '!' | '?' | '\n' | '"' | '\u{201c}'));
        if !sentence_start {
            return false;
        }
    }
    match list.zipf(word) {
        Some(z) => z < o.threshold,
        None => o.mark_unknown,
    }
}

/// The difficult words of `text` (first char at `base`), as canonical
/// ranges in order. URLs and code are skipped.
pub fn difficult_text(
    text: &str,
    base: CharPos,
    list: &FrequencyList,
    opts: &DifficultOptions,
) -> Vec<CharRange> {
    let skip = SkipSet::new(text_skip_ranges(text, base, true, true));
    difficult_with_skip(text, base, list, opts, &skip)
}

/// The difficult words of `range` of `doc`; `Code` markers are skipped.
pub fn difficult_range(
    doc: &Document,
    range: CharRange,
    list: &FrequencyList,
    opts: &DifficultOptions,
) -> Vec<CharRange> {
    let range = range.clamp_to(doc.len_chars());
    let text = doc.slice(range);
    let mut skips = text_skip_ranges(&text, range.start, true, true);
    skips.extend(code_marker_ranges(doc, range));
    difficult_with_skip(&text, range.start, list, opts, &SkipSet::new(skips))
}

fn difficult_with_skip(
    text: &str,
    base: CharPos,
    list: &FrequencyList,
    opts: &DifficultOptions,
    skip: &SkipSet,
) -> Vec<CharRange> {
    let mut out = Vec::new();
    if list.is_empty() && !opts.mark_unknown {
        return out;
    }
    let mut conv = ByteToPos::new(text, base);
    for (b, word) in word_segments(text) {
        // The last non-space char before the word, for the sentence-start test.
        let prev = text[..b]
            .chars()
            .rev()
            .find(|c| !c.is_whitespace() || *c == '\n');
        if !is_difficult(word, prev, list, opts) {
            continue;
        }
        let start = conv.pos(b);
        let r = CharRange::new(start, start.saturating_add(word.chars().count()));
        if !skip.overlaps(r) {
            out.push(r);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> FrequencyList {
        FrequencyList::parse(
            "# test list\nthe\t7.7\nread\t5.6\nbook\t5.5\nabout\t6.2\npharmacology\t2.4\n\
             anatomy\t3.4\nstatistics\t4.3\nthese\t6.0\nstudents\t5.1\n",
        )
        .unwrap()
    }

    fn words(text: &str, o: &DifficultOptions) -> Vec<String> {
        difficult_text(text, CharPos(0), &list(), o)
            .into_iter()
            .map(|r| text.chars().skip(r.start.0).take(r.len()).collect())
            .collect()
    }

    #[test]
    fn marks_rare_words() {
        let t = "These students read a book about anatomy, statistics and pharmacology.";
        assert_eq!(
            words(t, &DifficultOptions::default()),
            ["anatomy", "statistics", "pharmacology"]
        );
        let strict = DifficultOptions {
            threshold: 3.0,
            ..DifficultOptions::default()
        };
        assert_eq!(words(t, &strict), ["pharmacology"]);
        // "and" is unknown but short; unknown words marked on request.
        let unknown = DifficultOptions {
            mark_unknown: true,
            min_len: 3,
            ..DifficultOptions::default()
        };
        assert!(words(t, &unknown).contains(&"and".to_owned()));
    }

    #[test]
    fn names_urls_and_code_are_skipped() {
        let t = "Anatomy class with Pharmacology Smith at https://anatomy.example `anatomy`";
        let got = words(t, &DifficultOptions::default());
        assert_eq!(
            got,
            ["Anatomy"],
            "sentence-initial capital is still checked"
        );
    }

    #[test]
    fn ranked_lists_and_errors() {
        let l = FrequencyList::parse("the\nof\nand\n").unwrap();
        assert_eq!(l.len(), 3);
        assert_eq!(l.zipf("THE"), Some(8.0));
        assert!((l.zipf("and").unwrap() - (8.0 - 3f32.log10())).abs() < 1e-5);
        assert!((FrequencyList::zipf_for_rank(1000) - 5.0).abs() < 1e-5);
        let e = FrequencyList::parse("ok\t5\nbad\tx\n").unwrap_err();
        assert_eq!(e.line, 2);
        assert_eq!(e.to_string(), "line 2: \"x\" is not a number");
        let pairs = FrequencyList::from_pairs([("Word", 3.0)]);
        assert_eq!(pairs.zipf("word"), Some(3.0));
    }

    #[test]
    fn empty_list_marks_nothing() {
        let t = "pharmacology";
        assert!(
            difficult_text(
                t,
                CharPos(0),
                &FrequencyList::new(),
                &DifficultOptions::default()
            )
            .is_empty()
        );
    }

    #[test]
    fn document_ranges() {
        let doc = Document::from_plain_text("Some anatomy.\n\nMore pharmacology.");
        let r = difficult_range(
            &doc,
            doc.full_range(),
            &list(),
            &DifficultOptions::default(),
        );
        assert_eq!(r, vec![CharRange::new(5, 12), CharRange::new(20, 32)]);
    }
}
