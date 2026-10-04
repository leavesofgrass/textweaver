//! Difficult words: mark rare words so a reader can pre-scan dense text.
//!
//! star (`star/vocab.py`) flagged words of four or more letters whose
//! `wordfreq` Zipf frequency was below 4.5. The engine here does the same
//! against any [`WordList`], and marks every occurrence with its canonical
//! range instead of returning a set of lowercase strings.
//!
//! Two kinds of list:
//!
//! - **SCOWL levels, built in** ([`ScowlList::builtin`], cargo feature
//!   `scowl`, on by default). SCOWL (Spell Checker Oriented Word Lists, by
//!   Kevin Atkinson; an MIT-like licence, notices in `third_party/scowl/`)
//!   gives each word a *size*: the smallest dictionary it belongs in, from
//!   35 (small, common) through 50 (medium), 60, and 70 (large) to 80 (valid
//!   but unusual). A word is difficult when its size is above
//!   [`DifficultOptions::max_level`] (50 by default). A word SCOWL does not
//!   list at all counts as rarer than size 80. The list is derived by
//!   `tools/scowl_levels.py` (see `third_party/scowl/README.md`). SCOWL
//!   measures which dictionaries include a word, not how often it is used,
//!   so it is a rough guide: `ubiquitous` is in size 35.
//! - **Frequency lists** ([`FrequencyList`]) on the Zipf scale, loaded from
//!   `word<TAB>zipf` lines (what `wordfreq` exports) or a ranked list, with
//!   [`FrequencyList::parse`]. The Zipf scale is log10 of a word's frequency
//!   per billion words: "the" is about 7.7, everyday words 5 to 6, uncommon
//!   words below 4, rare below 3. `wordfreq`'s own data is not vendored: it
//!   is CC BY-SA 4.0.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange};
use textweaver_text::Document;

use crate::util::{ByteToPos, SkipSet, code_marker_ranges, text_skip_ranges, word_segments};

/// How common a word is, as a list reports it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Commonness {
    /// A Zipf frequency: higher is more common.
    Zipf(f32),
    /// A SCOWL size: lower is more common.
    Level(u8),
}

/// A source of word commonness for [`difficult_text`] and
/// [`difficult_range`].
pub trait WordList {
    /// How common `word` is (any case), or `None` when the list does not
    /// have it.
    fn commonness(&self, word: &str) -> Option<Commonness>;

    /// True when the list has no words.
    fn is_empty(&self) -> bool;

    /// How common a word the list lacks should be taken to be, for lists
    /// that cover the whole language (SCOWL: rarer than its largest size).
    /// `None`, the default, leaves unlisted words to
    /// [`DifficultOptions::mark_unknown`].
    fn unlisted(&self) -> Option<Commonness> {
        None
    }
}

/// Word frequencies on the Zipf scale, keyed by lowercase word.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrequencyList {
    zipf: HashMap<String, f32>,
}

/// A line of a word list file that could not be read.
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

impl WordList for FrequencyList {
    fn commonness(&self, word: &str) -> Option<Commonness> {
        self.zipf(word).map(Commonness::Zipf)
    }

    fn is_empty(&self) -> bool {
        self.zipf.is_empty()
    }
}

/// The SCOWL size a word SCOWL does not list is taken to have: rarer than
/// every size in the built-in list (which stops at 80).
pub const UNLISTED_LEVEL: u8 = 95;

/// Default for [`DifficultOptions::max_level`]: words first appearing in a
/// SCOWL size above 50 (medium) are difficult.
pub const DEFAULT_MAX_LEVEL: u8 = 50;

/// One word of a [`ScowlList`]: where it is in the text, and its size.
#[derive(Clone, Copy, Debug)]
struct Entry {
    start: u32,
    len: u16,
    level: u8,
}

/// Words by SCOWL size, in the format `third_party/scowl/README.md`
/// describes: sections headed `@SIZE`, one lowercase word per line, each
/// section sorted. Looking a word up is one binary search over an index
/// merged from the sections.
#[derive(Clone, Debug, Default)]
pub struct ScowlList {
    text: String,
    /// Sorted by word bytes.
    index: Vec<Entry>,
    levels: Vec<u8>,
}

impl ScowlList {
    /// Parses a level list. Words are expected lowercase; lines outside a
    /// section, and blank lines, are errors or skipped respectively.
    pub fn parse(text: String) -> Result<ScowlList, ParseError> {
        // Sections: (level, entries in file order).
        let mut sections: Vec<(u8, Vec<Entry>)> = Vec::new();
        let mut offset = 0usize;
        for (i, raw) in text.split('\n').enumerate() {
            let start = offset;
            offset += raw.len() + 1;
            let line = raw.strip_suffix('\r').unwrap_or(raw);
            if line.is_empty() {
                continue;
            }
            let err = |reason: String| ParseError {
                line: i + 1,
                reason,
            };
            if let Some(level) = line.strip_prefix('@') {
                let level: u8 = level
                    .trim()
                    .parse()
                    .map_err(|_| err(format!("{level:?} is not a size")))?;
                sections.push((level, Vec::new()));
                continue;
            }
            let Some((level, words)) = sections.last_mut() else {
                return Err(err("a word before the first @SIZE line".into()));
            };
            let len = u16::try_from(line.len()).map_err(|_| err("word too long".into()))?;
            let start = u32::try_from(start).map_err(|_| err("list too large".into()))?;
            words.push(Entry {
                start,
                len,
                level: *level,
            });
        }
        let word =
            |e: &Entry| &text.as_bytes()[e.start as usize..e.start as usize + e.len as usize];
        // Each section is sorted already (checked); merge them.
        for (level, entries) in &sections {
            if let Some(bad) = entries.windows(2).position(|w| word(&w[0]) > word(&w[1])) {
                return Err(ParseError {
                    line: 0,
                    reason: format!(
                        "size {level} is not sorted at {:?}",
                        String::from_utf8_lossy(word(&entries[bad + 1]))
                    ),
                });
            }
        }
        let mut levels: Vec<u8> = sections.iter().map(|s| s.0).collect();
        levels.sort_unstable();
        levels.dedup();
        let mut heads: Vec<std::iter::Peekable<std::vec::IntoIter<Entry>>> = sections
            .into_iter()
            .map(|(_, e)| e.into_iter().peekable())
            .collect();
        let index = merge(&text, &mut heads);
        Ok(ScowlList {
            text,
            index,
            levels,
        })
    }

    /// Number of words.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// True when the list is empty.
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// The sizes the list has, in rising order.
    pub fn levels(&self) -> &[u8] {
        &self.levels
    }

    /// Every word with its size, in byte order.
    pub fn words(&self) -> impl Iterator<Item = (&str, u8)> + '_ {
        self.index.iter().map(|e| {
            let s = e.start as usize;
            (&self.text[s..s + e.len as usize], e.level)
        })
    }

    fn word(&self, e: &Entry) -> &[u8] {
        &self.text.as_bytes()[e.start as usize..e.start as usize + e.len as usize]
    }

    fn find(&self, key: &str) -> Option<u8> {
        self.index
            .binary_search_by(|e| self.word(e).cmp(key.as_bytes()))
            .ok()
            .map(|i| self.index[i].level)
    }

    /// The SCOWL size of `word` (any case; curly apostrophes and a
    /// possessive `'s` are handled), or `None` when not listed.
    pub fn level(&self, word: &str) -> Option<u8> {
        let lower: String = word
            .chars()
            .map(|c| if c == '\u{2019}' { '\'' } else { c })
            .flat_map(char::to_lowercase)
            .collect();
        self.find(&lower).or_else(|| {
            lower
                .strip_suffix("'s")
                .filter(|s| !s.is_empty())
                .and_then(|s| self.find(s))
        })
    }

    /// The list built into textweaver (SCOWL `rel-2026.02.25`, sizes 35 to
    /// 80, 225,038 words), unpacked on first use and kept for the life of
    /// the program. `None` when built without the `scowl` feature or if
    /// the embedded data cannot be read.
    pub fn builtin() -> Option<&'static ScowlList> {
        #[cfg(feature = "scowl")]
        {
            static LIST: std::sync::OnceLock<Option<ScowlList>> = std::sync::OnceLock::new();
            LIST.get_or_init(builtin::load).as_ref()
        }
        #[cfg(not(feature = "scowl"))]
        {
            None
        }
    }
}

/// Merges sorted runs of entries into one sorted index.
fn merge(text: &str, heads: &mut [std::iter::Peekable<std::vec::IntoIter<Entry>>]) -> Vec<Entry> {
    let word = |e: &Entry| &text.as_bytes()[e.start as usize..e.start as usize + e.len as usize];
    let mut out = Vec::with_capacity(heads.iter().map(|h| h.len()).sum());
    loop {
        let mut best: Option<(usize, Entry)> = None;
        for (k, h) in heads.iter_mut().enumerate() {
            if let Some(&e) = h.peek()
                && best.is_none_or(|(_, b)| word(&e) < word(&b))
            {
                best = Some((k, e));
            }
        }
        let Some((k, e)) = best else { break };
        heads[k].next();
        // The same word in two sizes keeps the smaller (the lists come
        // from one source, so this only guards hand-made files).
        match out.last_mut() {
            Some(last) if word(last) == word(&e) => last.level = last.level.min(e.level),
            _ => out.push(e),
        }
    }
    out
}

#[cfg(feature = "scowl")]
mod builtin {
    use std::io::{Cursor, Read};

    use super::ScowlList;

    static DATA: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../third_party/scowl/scowl-levels.zip"
    ));

    pub(super) fn load() -> Option<ScowlList> {
        let mut zip = zip::ZipArchive::new(Cursor::new(DATA)).ok()?;
        let mut entry = zip.by_name("scowl-levels.txt").ok()?;
        let mut text = String::with_capacity(usize::try_from(entry.size()).unwrap_or(0));
        entry.read_to_string(&mut text).ok()?;
        ScowlList::parse(text).ok()
    }
}

impl WordList for ScowlList {
    fn commonness(&self, word: &str) -> Option<Commonness> {
        self.level(word).map(Commonness::Level)
    }

    fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    fn unlisted(&self) -> Option<Commonness> {
        Some(Commonness::Level(UNLISTED_LEVEL))
    }
}

/// Options for [`difficult_text`] and [`difficult_range`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DifficultOptions {
    /// Frequency lists: words below this Zipf value are difficult. star
    /// used 4.5.
    pub threshold: f32,
    /// SCOWL: words whose size is above this are difficult (35 marks the
    /// most, 80 only the rarest; default 50).
    pub max_level: u8,
    /// Shorter words (in chars) are never marked. star used 4.
    pub min_len: usize,
    /// Mark words missing from a frequency list (likely rare, but also
    /// names and typos). Off by default. SCOWL lists decide this
    /// themselves: a word SCOWL lacks is rarer than size 80.
    pub mark_unknown: bool,
    /// Skip words that start with a capital letter mid-sentence (names).
    pub skip_capitalized: bool,
}

impl Default for DifficultOptions {
    fn default() -> Self {
        DifficultOptions {
            threshold: 4.5,
            max_level: DEFAULT_MAX_LEVEL,
            min_len: 4,
            mark_unknown: false,
            skip_capitalized: true,
        }
    }
}

fn is_difficult<L: WordList + ?Sized>(
    word: &str,
    prev: Option<char>,
    list: &L,
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
    match list.commonness(word).or_else(|| list.unlisted()) {
        Some(Commonness::Zipf(z)) => z < o.threshold,
        Some(Commonness::Level(l)) => l > o.max_level,
        None => o.mark_unknown,
    }
}

/// The difficult words of `text` (first char at `base`), as canonical
/// ranges in order. URLs and code are skipped.
pub fn difficult_text<L: WordList + ?Sized>(
    text: &str,
    base: CharPos,
    list: &L,
    opts: &DifficultOptions,
) -> Vec<CharRange> {
    let skip = SkipSet::new(text_skip_ranges(text, base, true, true));
    difficult_with_skip(text, base, list, opts, &skip)
}

/// The difficult words of `range` of `doc`; `Code` markers are skipped.
pub fn difficult_range<L: WordList + ?Sized>(
    doc: &Document,
    range: CharRange,
    list: &L,
    opts: &DifficultOptions,
) -> Vec<CharRange> {
    let range = range.clamp_to(doc.len_chars());
    let text = doc.slice(range);
    let mut skips = text_skip_ranges(&text, range.start, true, true);
    skips.extend(code_marker_ranges(doc, range));
    difficult_with_skip(&text, range.start, list, opts, &SkipSet::new(skips))
}

fn difficult_with_skip<L: WordList + ?Sized>(
    text: &str,
    base: CharPos,
    list: &L,
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
        words_with(text, &list(), o)
    }

    fn words_with<L: WordList + ?Sized>(text: &str, l: &L, o: &DifficultOptions) -> Vec<String> {
        difficult_text(text, CharPos(0), l, o)
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
        let empty = ScowlList::parse(String::new()).unwrap();
        assert!(words_with(t, &empty, &DifficultOptions::default()).is_empty());
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

    fn small_scowl() -> ScowlList {
        ScowlList::parse(
            "@35\nbook\ndon't\neven\nread\nreading\nstudent\nstudents\nthese\n\
             @50\nserendipity\n@70\nperspicacity\n"
                .to_owned(),
        )
        .unwrap()
    }

    #[test]
    fn scowl_levels_mark_words_above_the_threshold() {
        let l = small_scowl();
        assert_eq!(l.len(), 10);
        assert_eq!(l.levels(), [35, 50, 70]);
        assert_eq!(l.level("Reading"), Some(35));
        assert_eq!(l.level("don\u{2019}t"), Some(35));
        assert_eq!(l.level("student's"), Some(35));
        assert_eq!(l.level("perspicacity"), Some(70));
        assert_eq!(l.level("xyzzy"), None);
        let t = "These students read a book on serendipity and perspicacity, even xyzzyology.";
        let d = DifficultOptions::default();
        // Above 50: size 70, and a word SCOWL lacks.
        assert_eq!(words_with(t, &l, &d), ["perspicacity", "xyzzyology"]);
        let wide = DifficultOptions { max_level: 35, ..d };
        assert_eq!(
            words_with(t, &l, &wide),
            ["serendipity", "perspicacity", "xyzzyology"]
        );
        let narrow = DifficultOptions { max_level: 80, ..d };
        assert_eq!(words_with(t, &l, &narrow), ["xyzzyology"]);
        let none = DifficultOptions {
            max_level: UNLISTED_LEVEL,
            ..d
        };
        assert!(words_with(t, &l, &none).is_empty());
    }

    #[test]
    fn scowl_parse_errors() {
        assert!(ScowlList::parse("word\n".into()).is_err());
        assert!(ScowlList::parse("@x\n".into()).is_err());
        let e = ScowlList::parse("@35\nzebra\napple\n".into()).unwrap_err();
        assert!(e.reason.contains("not sorted"), "{e}");
        // The same word in two sizes keeps the smaller.
        let l = ScowlList::parse("@50\nword\n@35\nword\n".into()).unwrap();
        assert_eq!((l.len(), l.level("word")), (1, Some(35)));
        // CR LF files read too.
        let l = ScowlList::parse("@35\r\nbook\r\n".into()).unwrap();
        assert_eq!(l.level("book"), Some(35));
    }

    #[cfg(feature = "scowl")]
    #[test]
    fn the_builtin_list_loads_and_classifies() {
        let l = ScowlList::builtin().expect("built-in SCOWL list");
        // The counts third_party/scowl/README.md reports.
        assert_eq!(l.len(), 225_038);
        assert_eq!(l.levels(), [35, 40, 50, 60, 65, 70, 80]);
        for (w, level) in [
            ("the", 35),
            ("students", 35),
            ("don't", 35),
            ("naïve", 35),
            ("naive", 35),
            ("colour", 35),
            ("pharmacology", 40),
            ("serendipity", 50),
            ("mitochondria", 60),
            ("antidisestablishmentarianism", 70),
        ] {
            assert_eq!(l.level(w), Some(level), "{w}");
        }
        let t = "The students studied mitochondria with obvious serendipity.";
        assert_eq!(
            words_with(t, l, &DifficultOptions::default()),
            ["mitochondria"]
        );
        // The same list is returned every time.
        assert!(std::ptr::eq(l, ScowlList::builtin().unwrap()));
    }
}
