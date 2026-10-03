//! Text to phoneme ids, keeping track of which word each id belongs to.
//!
//! Piper voices take IPA phonemes from eSpeak NG. Two phonemizers give
//! them ([`Phonemizer`]):
//!
//! - [`LibPhonemizer`] (feature `espeak-lib`): an installed libespeak-ng,
//!   through the same run-time loader as the espeak-ng speech backend
//!   (`espeak_TextToPhonemes`, IPA mode). Piper itself does this.
//! - [`RustPhonemizer`] (feature `espeak-rs`): the pure-Rust eSpeak NG port
//!   (`espeak-ng` crate) with its English data built in, when the library
//!   is missing.
//!
//! [`prepare`] phonemizes one chunk clause by clause, puts each clause's
//! punctuation back (as Piper's phonemizer does), and turns the phonemes
//! into model input ids (`^ _ p _ p _ ... $`). It also records, for every
//! id, the text word it came from: eSpeak separates words with spaces, so
//! the phoneme words of a clause line up with its text words. When the
//! counts differ (eSpeak joined or split a word), they are matched in
//! proportion to their lengths ([`align`]).

use std::ops::Range;

use crate::PiperError;
use crate::config::VoiceConfig;
use crate::text::{Clause, words};

/// Turns one clause of text into IPA phonemes, words separated by spaces.
pub trait Phonemizer {
    /// A short name for logs and reports ("libespeak-ng", "espeak-ng-rs").
    fn name(&self) -> &'static str;
    /// Phonemizes `clause` with eSpeak voice `voice` (`en-us`).
    fn phonemize(&mut self, voice: &str, clause: &str) -> Result<String, PiperError>;
}

/// An installed libespeak-ng, loaded at run time (feature `espeak-lib`).
#[cfg(feature = "espeak-lib")]
#[derive(Debug, Default)]
pub struct LibPhonemizer;

#[cfg(feature = "espeak-lib")]
impl LibPhonemizer {
    /// The library phonemizer, when libespeak-ng loads and has
    /// `espeak_TextToPhonemes`.
    pub fn new() -> Option<Self> {
        textweaver_speech::backends::espeak::phonemes_available().then_some(LibPhonemizer)
    }
}

#[cfg(feature = "espeak-lib")]
impl Phonemizer for LibPhonemizer {
    fn name(&self) -> &'static str {
        "libespeak-ng"
    }

    fn phonemize(&mut self, voice: &str, clause: &str) -> Result<String, PiperError> {
        textweaver_speech::backends::espeak::text_to_phonemes(voice, clause)
            .map(|parts| parts.join(" "))
            .map_err(PiperError::Phonemes)
    }
}

/// The pure-Rust eSpeak NG port with its English data built in (feature
/// `espeak-rs`). The data is written once into a folder the port reads.
#[cfg(feature = "espeak-rs")]
pub struct RustPhonemizer {
    data_dir: std::path::PathBuf,
    translators: std::collections::HashMap<String, espeak_ng::Translator>,
}

#[cfg(feature = "espeak-rs")]
impl std::fmt::Debug for RustPhonemizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RustPhonemizer")
            .field("data_dir", &self.data_dir)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "espeak-rs")]
impl RustPhonemizer {
    /// The port, with its built-in data written into `data_dir` (an
    /// `espeak-ng-data` folder) unless it is there already.
    pub fn new(data_dir: &std::path::Path) -> Result<Self, PiperError> {
        if !data_dir.join("phontab").is_file() || !data_dir.join("en_dict").is_file() {
            std::fs::create_dir_all(data_dir).map_err(|e| PiperError::io(data_dir, e))?;
            espeak_ng::install_bundled_language(data_dir, "en")
                .map_err(|e| PiperError::io(data_dir, e))?;
        }
        Ok(RustPhonemizer {
            data_dir: data_dir.to_owned(),
            translators: std::collections::HashMap::new(),
        })
    }

    /// The languages whose data is built in.
    pub fn languages() -> &'static [&'static str] {
        espeak_ng::bundled_languages()
    }
}

#[cfg(feature = "espeak-rs")]
impl Phonemizer for RustPhonemizer {
    fn name(&self) -> &'static str {
        "espeak-ng-rs"
    }

    fn phonemize(&mut self, voice: &str, clause: &str) -> Result<String, PiperError> {
        let lang = voice.split('+').next().unwrap_or(voice);
        let primary = lang.split(['-', '_']).next().unwrap_or(lang);
        if !espeak_ng::has_bundled_language(primary) {
            return Err(PiperError::Phonemes(format!(
                "the built-in phonemizer has no {primary} data; install eSpeak NG"
            )));
        }
        if !self.translators.contains_key(lang) {
            let t = espeak_ng::Translator::new(lang, Some(&self.data_dir))
                .map_err(|e| PiperError::Phonemes(e.to_string()))?;
            self.translators.insert(lang.to_owned(), t);
        }
        let t = self
            .translators
            .get(lang)
            .ok_or_else(|| PiperError::Phonemes("no translator".into()))?;
        t.text_to_ipa(clause)
            .map(|ipa| ipa.split_whitespace().collect::<Vec<_>>().join(" "))
            .map_err(|e| PiperError::Phonemes(e.to_string()))
    }
}

/// Which phonemizer to use.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PhonemizerChoice {
    /// libespeak-ng when installed, else the pure-Rust port.
    #[default]
    Auto,
    /// Only libespeak-ng.
    Library,
    /// Only the pure-Rust port.
    Rust,
}

/// The phonemizer `choice` asks for, or why none is available.
/// `espeak_data` is where the pure-Rust port keeps its data.
pub fn phonemizer(
    choice: PhonemizerChoice,
    espeak_data: &std::path::Path,
) -> Result<Box<dyn Phonemizer>, PiperError> {
    let _ = espeak_data;
    let mut why = Vec::new();
    if choice != PhonemizerChoice::Rust {
        #[cfg(feature = "espeak-lib")]
        match LibPhonemizer::new() {
            Some(p) => return Ok(Box::new(p)),
            None => why.push("libespeak-ng is not installed"),
        }
        #[cfg(not(feature = "espeak-lib"))]
        why.push("this build cannot load libespeak-ng");
    }
    if choice != PhonemizerChoice::Library {
        #[cfg(feature = "espeak-rs")]
        return RustPhonemizer::new(espeak_data).map(|p| Box::new(p) as Box<dyn Phonemizer>);
        #[cfg(not(feature = "espeak-rs"))]
        why.push("this build has no built-in phonemizer");
    }
    Err(PiperError::Phonemes(why.join("; ")))
}

/// A chunk turned into model input.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Prepared {
    /// Model input ids.
    pub ids: Vec<i64>,
    /// For each id, the index (into [`words`](Self::words)) of the word
    /// it was phonemized from; `None` for the start and end ids, spaces,
    /// and punctuation.
    pub owners: Vec<Option<u32>>,
    /// The byte ranges of the chunk's words, in the utterance text.
    pub words: Vec<Range<u32>>,
    /// The phoneme string, for logs and tests.
    pub phonemes: String,
    /// True when every clause's phoneme words matched its text words one
    /// to one; false when some were matched by length ([`align`]).
    pub exact: bool,
}

/// Phonemizes the clauses `clauses` of `text` (one synthesis chunk) and
/// builds the model input. Byte ranges are offsets into `text`.
pub fn prepare(
    config: &VoiceConfig,
    phonemizer: &mut dyn Phonemizer,
    text: &str,
    clauses: &[Clause],
) -> Result<Prepared, PiperError> {
    let special = config.special_ids()?;
    let mut out = Prepared {
        exact: true,
        ..Prepared::default()
    };
    let mut builder = IdBuilder {
        config,
        pad: &special.pad,
        out: &mut out,
    };
    builder.push_ids(&special.bos, None);
    builder.push_ids(&special.pad, None);
    for clause in clauses {
        let clause_text = &text[clause.range.clone()];
        let phonemes = if config.uses_espeak() {
            phonemizer.phonemize(&config.espeak.voice, clause_text)?
        } else {
            clause_text.to_lowercase()
        };
        let text_words: Vec<Range<usize>> = words(clause_text)
            .into_iter()
            .map(|w| w.start + clause.range.start..w.end + clause.range.start)
            .collect();
        let groups: Vec<&str> = phonemes.split_whitespace().collect();
        let lens: Vec<usize> = text_words
            .iter()
            .map(|w| text[w.clone()].chars().count())
            .collect();
        let owners = owners_by_char(&lens, &groups);
        if groups.len() != text_words.len() {
            builder.out.exact = false;
        }
        let first_word = u32::try_from(builder.out.words.len()).unwrap_or(u32::MAX);
        builder
            .out
            .words
            .extend(text_words.iter().map(|w| to_u32(w.start)..to_u32(w.end)));
        for (g, (group, starts)) in groups.iter().zip(owners).enumerate() {
            if g > 0 {
                builder.push_phonemes(" ", None);
            }
            builder.push_group(group, &starts, first_word);
        }
        builder.push_phonemes(clause.tail(), None);
    }
    // A trailing space after the last clause is not spoken.
    if builder.out.phonemes.ends_with(' ') {
        builder.out.phonemes.pop();
        let pad = special.pad.len();
        let space = config.ids(' ').map_or(0, <[i64]>::len);
        let drop = space + pad;
        let keep = builder.out.ids.len().saturating_sub(drop);
        builder.out.ids.truncate(keep);
        builder.out.owners.truncate(keep);
    }
    builder.push_ids(&special.eos, None);
    Ok(out)
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

struct IdBuilder<'a> {
    config: &'a VoiceConfig,
    pad: &'a [i64],
    out: &'a mut Prepared,
}

impl IdBuilder<'_> {
    fn push_ids(&mut self, ids: &[i64], owner: Option<u32>) {
        self.out.ids.extend_from_slice(ids);
        self.out
            .owners
            .extend(std::iter::repeat_n(owner, ids.len()));
    }

    /// A phoneme word whose characters belong to the words `starts` says
    /// (`(first char, word)`, in order, offset by `first_word`).
    fn push_group(&mut self, group: &str, starts: &[(usize, u32)], first_word: u32) {
        for (i, c) in group.chars().enumerate() {
            let owner = starts
                .iter()
                .rev()
                .find(|(at, _)| *at <= i)
                .map(|(_, w)| first_word + w);
            let mut buf = [0u8; 4];
            self.push_phonemes(c.encode_utf8(&mut buf), owner);
        }
    }

    /// Each phoneme's ids followed by the pad ids; phonemes the voice does
    /// not know are skipped (Piper does the same).
    fn push_phonemes(&mut self, phonemes: &str, owner: Option<u32>) {
        for c in phonemes.chars() {
            let Some(ids) = self.config.ids(c) else {
                log::debug!("piper: the voice has no phoneme {c:?}; skipped");
                continue;
            };
            self.out.phonemes.push(c);
            let ids = ids.to_vec();
            self.push_ids(&ids, owner);
            let pad = self.pad.to_vec();
            self.push_ids(&pad, owner);
        }
    }
}

/// How many clauses [`PhonemeCache`] remembers.
pub const CACHE_CLAUSES: usize = 512;

/// Phonemes already worked out, by eSpeak voice and clause text, newest
/// last.
///
/// The pure-Rust phonemizer reads and parses its dictionary on every
/// call, about 90 ms however short the clause (Wave 8b, on the owner's
/// machine): as long as the model run itself for a short phrase. The
/// cache takes that off the path to the first audio whenever the text was
/// seen before or worked out ahead: a restart at the same place, a rate
/// or pitch change, the next sentence (handed over as lookahead), and
/// repeated announcements ("Next heading").
#[derive(Debug, Default)]
pub struct PhonemeCache {
    entries: std::collections::VecDeque<(String, String, String)>,
    /// Clauses a thread is phonemizing now (voice, clause).
    working: Vec<(String, String)>,
}

/// True when `part` is `whole`, or its start or its end at a word break.
fn part_of(part: &str, whole: &str) -> bool {
    part == whole
        || whole
            .strip_prefix(part)
            .is_some_and(|rest| rest.starts_with(char::is_whitespace))
        || whole
            .strip_suffix(part)
            .is_some_and(|before| before.ends_with(char::is_whitespace))
}

impl PhonemeCache {
    /// Claims `clause` for phonemizing: false when it is known already or
    /// another thread is phonemizing it (then [`get`](Self::get) will
    /// have it soon).
    pub fn start(&mut self, voice: &str, clause: &str) -> bool {
        if self.get(voice, clause).is_some()
            || self.working.iter().any(|(v, c)| v == voice && c == clause)
        {
            return false;
        }
        self.working.push((voice.to_owned(), clause.to_owned()));
        true
    }

    /// Ends a claim from [`start`](Self::start), remembering the phonemes
    /// when there are some.
    pub fn finish(&mut self, voice: &str, clause: &str, phonemes: Option<&str>) {
        self.working.retain(|(v, c)| !(v == voice && c == clause));
        if let Some(p) = phonemes {
            self.insert(voice, clause, p);
        }
    }

    /// True while another thread is phonemizing `clause`, or a clause it
    /// is the start or the end of.
    pub fn coming(&self, voice: &str, clause: &str) -> bool {
        self.working
            .iter()
            .any(|(v, c)| v == voice && part_of(clause, c))
    }

    /// The phonemes of `clause` in eSpeak voice `voice`, if known. A
    /// clause that is the start or the end of a known one (the first
    /// phrase of a long clause, spoken on its own when a reading starts,
    /// and the rest after it) takes that clause's first or last phoneme
    /// words, when they line up with its words one to one.
    pub fn get(&mut self, voice: &str, clause: &str) -> Option<String> {
        if let Some(i) = self
            .entries
            .iter()
            .rposition(|(v, c, _)| v == voice && c == clause)
        {
            // Most recently used goes last.
            let e = self.entries.remove(i)?;
            let p = e.2.clone();
            self.entries.push_back(e);
            return Some(p);
        }
        let n = words(clause).len();
        if n == 0 {
            return None;
        }
        self.entries.iter().rev().find_map(|(v, c, p)| {
            // The start of a known clause, or (the rest after its first
            // phrase) its end, at a word break.
            if v != voice || !part_of(clause, c) {
                return None;
            }
            let head = c.starts_with(clause);
            let groups: Vec<&str> = p.split_whitespace().collect();
            if groups.len() != words(c).len() || n > groups.len() {
                return None;
            }
            let part = if head {
                &groups[..n]
            } else {
                &groups[groups.len() - n..]
            };
            Some(part.join(" "))
        })
    }

    /// Remembers the phonemes of `clause` in eSpeak voice `voice`.
    pub fn insert(&mut self, voice: &str, clause: &str, phonemes: &str) {
        if let Some(i) = self
            .entries
            .iter()
            .position(|(v, c, _)| v == voice && c == clause)
        {
            self.entries.remove(i);
        }
        if self.entries.len() >= CACHE_CLAUSES {
            self.entries.pop_front();
        }
        self.entries
            .push_back((voice.to_owned(), clause.to_owned(), phonemes.to_owned()));
    }

    /// How many clauses are remembered.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when nothing is remembered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A shared [`PhonemeCache`].
pub type SharedCache = std::sync::Arc<std::sync::Mutex<PhonemeCache>>;

/// Locks `cache`; a panic while it was held cannot leave it invalid.
pub fn lock_cache(cache: &SharedCache) -> std::sync::MutexGuard<'_, PhonemeCache> {
    cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A phonemizer that looks in a shared [`PhonemeCache`] first, and
/// remembers what it works out.
pub struct CachedPhonemizer {
    inner: Box<dyn Phonemizer>,
    cache: SharedCache,
    /// True when the clause is no longer wanted (a Stop): ends a wait for
    /// another thread's phonemes.
    stopped: Option<Box<dyn Fn() -> bool + Send>>,
}

impl std::fmt::Debug for CachedPhonemizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedPhonemizer")
            .field("inner", &self.inner.name())
            .finish_non_exhaustive()
    }
}

impl CachedPhonemizer {
    /// `inner`, looking in `cache` first.
    pub fn new(inner: Box<dyn Phonemizer>, cache: SharedCache) -> Self {
        CachedPhonemizer {
            inner,
            cache,
            stopped: None,
        }
    }

    /// Waits for a clause another thread is phonemizing only while
    /// `stopped` is false; once it is true, [`Phonemizer::phonemize`]
    /// returns [`PiperError::Stopped`] at once.
    pub fn with_stop(mut self, stopped: impl Fn() -> bool + Send + 'static) -> Self {
        self.stopped = Some(Box::new(stopped));
        self
    }
}

impl Phonemizer for CachedPhonemizer {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    /// The cached phonemes, or those another thread is working out (a
    /// wait a Stop ends), or this phonemizer's own.
    fn phonemize(&mut self, voice: &str, clause: &str) -> Result<String, PiperError> {
        /// How often a wait looks again.
        const STEP: std::time::Duration = std::time::Duration::from_millis(2);
        /// The longest wait for another thread, after which this one
        /// works it out too.
        const MOST: std::time::Duration = std::time::Duration::from_secs(2);
        let deadline = std::time::Instant::now() + MOST;
        loop {
            {
                let mut c = lock_cache(&self.cache);
                if let Some(p) = c.get(voice, clause) {
                    return Ok(p);
                }
                if (!c.coming(voice, clause) && c.start(voice, clause))
                    || std::time::Instant::now() > deadline
                {
                    break;
                }
            }
            if self.stopped.as_ref().is_some_and(|s| s()) {
                return Err(PiperError::Stopped);
            }
            std::thread::sleep(STEP);
        }
        let p = self.inner.phonemize(voice, clause);
        lock_cache(&self.cache).finish(voice, clause, p.as_ref().ok().map(String::as_str));
        p
    }
}

/// Matches phoneme words to text words: for each phoneme word, the index
/// of its text word. One to one when the counts agree; otherwise each
/// phoneme word goes to the text word covering the same fraction of the
/// clause (by character count), which keeps the order. `None` only when
/// the clause has no text words.
pub fn align(word_lens: &[usize], groups: &[&str]) -> Vec<Option<u32>> {
    if word_lens.is_empty() {
        return vec![None; groups.len()];
    }
    if word_lens.len() == groups.len() {
        return (0..groups.len()).map(|i| Some(to_u32(i))).collect();
    }
    let total_text: usize = word_lens.iter().map(|&l| l.max(1)).sum();
    let group_lens: Vec<usize> = groups.iter().map(|g| g.chars().count().max(1)).collect();
    let total_ph: usize = group_lens.iter().sum();
    // Fraction of the clause where each text word ends.
    let mut ends = Vec::with_capacity(word_lens.len());
    let mut acc = 0usize;
    for &l in word_lens {
        acc += l.max(1);
        ends.push(acc as f64 / total_text as f64);
    }
    let mut out = Vec::with_capacity(groups.len());
    let mut acc = 0usize;
    for &l in &group_lens {
        let mid = (acc as f64 + l as f64 / 2.0) / total_ph as f64;
        acc += l;
        let w = ends
            .iter()
            .position(|&e| mid <= e)
            .unwrap_or(ends.len() - 1);
        out.push(Some(to_u32(w)));
    }
    out
}

/// Which text words each phoneme word holds: for each phoneme word,
/// `(char index, word)` pairs, one per word starting there. Built on
/// [`align`]; a text word that no phoneme word was matched to was joined
/// into its neighbour by eSpeak (libespeak-ng writes "in the" as `ɪnðə`),
/// so that phoneme word's characters are shared between the joined words
/// in proportion to their lengths, and every word gets its own start.
pub fn owners_by_char(word_lens: &[usize], groups: &[&str]) -> Vec<Vec<(usize, u32)>> {
    let base = align(word_lens, groups);
    let n = to_u32(word_lens.len());
    let mut out: Vec<Vec<(usize, u32)>> = base
        .iter()
        .map(|o| o.map(|w| vec![(0, w)]).unwrap_or_default())
        .collect();
    for (i, own) in base.iter().enumerate() {
        let Some(w) = *own else { continue };
        // The words before the first match belong to the first group.
        let first = if i == 0 { 0 } else { w };
        let next = base[i + 1..].iter().flatten().next().copied().unwrap_or(n);
        let last = next.saturating_sub(1).max(w);
        if first == w && last == w {
            continue;
        }
        let chars = groups[i].chars().count().max(1);
        let total: usize = (first..=last).map(|k| word_lens[k as usize].max(1)).sum();
        let mut acc = 0usize;
        let mut starts: Vec<(usize, u32)> = Vec::new();
        for k in first..=last {
            let at = ((acc * chars + total / 2) / total).min(chars - 1);
            match starts.last_mut() {
                // Too few characters to go round: the later word wins.
                Some(prev) if prev.0 == at => prev.1 = k,
                _ => starts.push((at, k)),
            }
            acc += word_lens[k as usize].max(1);
        }
        out[i] = starts;
    }
    out
}

/// Where each word starts, in samples, given the frames each id lasts
/// (`w_ceil`) and the samples per frame. Returns `(word index, first
/// sample)` in order, one per word that owns at least one id.
pub fn word_starts(
    owners: &[Option<u32>],
    frames: &[f32],
    samples_per_frame: f64,
) -> Vec<(u32, u64)> {
    let mut out: Vec<(u32, u64)> = Vec::new();
    let mut at = 0f64;
    for (owner, &f) in owners.iter().zip(frames) {
        if let Some(w) = *owner
            && out.last().is_none_or(|&(last, _)| last != w)
        {
            // Truncation is intended: the word starts at this sample.
            out.push((w, (at * samples_per_frame) as u64));
        }
        at += f64::from(f.max(0.0));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::tests::JSON;
    use crate::text::clauses;

    /// Phonemizes by a fixed table, like eSpeak would.
    struct Table(Vec<(&'static str, &'static str)>);

    impl Phonemizer for Table {
        fn name(&self) -> &'static str {
            "table"
        }
        fn phonemize(&mut self, _voice: &str, clause: &str) -> Result<String, PiperError> {
            self.0
                .iter()
                .find(|(t, _)| *t == clause)
                .map(|(_, p)| (*p).to_owned())
                .ok_or_else(|| PiperError::Phonemes(format!("no entry for {clause:?}")))
        }
    }

    #[test]
    fn the_cache_remembers_and_derives_a_first_phrase() {
        let cache = SharedCache::default();
        let mut p = CachedPhonemizer::new(
            Box::new(Table(vec![
                ("The bell rang at noon", "ðə bˈɛl ɹˈæŋ æt nˈuːn"),
                ("in the old tower", "ɪn ðɪ ˈoʊld tˈaʊɚ"),
            ])),
            cache.clone(),
        );
        assert_eq!(p.name(), "table");
        let full = p.phonemize("en-us", "The bell rang at noon").unwrap();
        assert_eq!(full, "ðə bˈɛl ɹˈæŋ æt nˈuːn");
        // The table has no entry for the first phrase: it comes from the
        // whole clause, word for word.
        assert_eq!(
            p.phonemize("en-us", "The bell rang").unwrap(),
            "ðə bˈɛl ɹˈæŋ"
        );
        // The rest of the clause after it, too.
        assert_eq!(p.phonemize("en-us", "at noon").unwrap(), "æt nˈuːn");
        // Only at a word break, and only in the same voice.
        assert!(p.phonemize("en-us", "The bell ran").is_err());
        assert!(p.phonemize("en-us", "t noon").is_err());
        assert!(p.phonemize("en-gb", "The bell rang").is_err());
        // Joined phoneme words (a count that differs) are never sliced.
        lock_cache(&cache).insert("en-us", "in the old tower", "ɪnðɪ ˈoʊld tˈaʊɚ");
        assert_eq!(lock_cache(&cache).get("en-us", "in the"), None);
        assert_eq!(lock_cache(&cache).len(), 2);
    }

    #[test]
    fn the_cache_forgets_the_least_recently_used() {
        let mut c = PhonemeCache::default();
        assert!(c.is_empty());
        for i in 0..CACHE_CLAUSES {
            c.insert("en-us", &format!("clause {i}"), "x");
        }
        // Using the oldest keeps it; the next oldest goes instead.
        assert!(c.get("en-us", "clause 0").is_some());
        c.insert("en-us", "one more", "y");
        assert_eq!(c.len(), CACHE_CLAUSES);
        assert!(c.get("en-us", "clause 0").is_some());
        assert!(c.get("en-us", "clause 1").is_none());
    }

    #[test]
    fn prepares_ids_with_word_owners() {
        let config = VoiceConfig::from_json(JSON).unwrap();
        let text = "This is, the end.";
        let cs = clauses(text);
        let mut p = Table(vec![("This is", "ðɪs ɪz"), ("the end", "ðə ˈend")]);
        let prep = prepare(&config, &mut p, text, &cs).unwrap();
        assert_eq!(prep.phonemes, "ðɪs ɪz, ðə ˈend.");
        assert!(prep.exact);
        let w: Vec<&str> = prep
            .words
            .iter()
            .map(|r| &text[r.start as usize..r.end as usize])
            .collect();
        assert_eq!(w, vec!["This", "is", "the", "end"]);
        // ^ _ then ð _ ɪ _ s _ (word 0) ...
        assert_eq!(&prep.ids[..4], &[1, 0, 41, 0]);
        assert_eq!(prep.ids.last(), Some(&2));
        assert_eq!(prep.ids.len(), prep.owners.len());
        assert_eq!(&prep.owners[..3], &[None, None, Some(0)]);
        // Every word owns some ids, in order.
        let mut seen: Vec<u32> = prep.owners.iter().flatten().copied().collect();
        seen.dedup();
        assert_eq!(seen, vec![0, 1, 2, 3]);
    }

    #[test]
    fn unknown_phonemes_are_skipped() {
        let config = VoiceConfig::from_json(JSON).unwrap();
        let text = "hi";
        let mut p = Table(vec![("hi", "hQi")]);
        let prep = prepare(&config, &mut p, text, &clauses(text)).unwrap();
        assert_eq!(prep.phonemes, "hi.");
    }

    #[test]
    fn a_phonemizer_error_is_reported() {
        let config = VoiceConfig::from_json(JSON).unwrap();
        let mut p = Table(vec![]);
        assert!(matches!(
            prepare(&config, &mut p, "hi", &clauses("hi")),
            Err(PiperError::Phonemes(_))
        ));
    }

    #[test]
    fn alignment_is_one_to_one_or_proportional() {
        assert_eq!(align(&[3, 2], &["ab", "c"]), vec![Some(0), Some(1)]);
        // eSpeak joined "of the": the joined word is timed as "of" (where
        // it starts), and "the" gets no event of its own.
        assert_eq!(align(&[2, 3, 3], &["ʌvðə", "ˈend"]), vec![Some(0), Some(2)]);
        // eSpeak split one word (a number) into three.
        assert_eq!(
            align(&[3, 4], &["wˈʌn", "hˈʌndɹɪd", "θɹˈiː"]),
            vec![Some(0), Some(1), Some(1)]
        );
        assert_eq!(align(&[], &["a"]), vec![None]);
        assert!(align(&[1], &[]).is_empty());
    }

    #[test]
    fn joined_words_share_their_phonemes() {
        // "in the morning": libespeak-ng joins "in the".
        assert_eq!(
            owners_by_char(&[2, 3, 7], &["ɪnðə", "mˈɔːɹnɪŋ"]),
            vec![vec![(0, 0), (2, 1)], vec![(0, 2)]]
        );
        // One to one needs no sharing.
        assert_eq!(
            owners_by_char(&[2, 3], &["ab", "cde"]),
            vec![vec![(0, 0)], vec![(0, 1)]]
        );
        // A word split in two keeps one start.
        assert_eq!(
            owners_by_char(&[3, 4], &["wˈʌn", "hˈʌndɹɪd", "θɹˈiː"]),
            vec![vec![(0, 0)], vec![(0, 1)], vec![(0, 1)]]
        );
        assert!(owners_by_char(&[1], &[]).is_empty());
        assert_eq!(
            owners_by_char(&[], &["a"]),
            vec![Vec::<(usize, u32)>::new()]
        );
    }

    #[test]
    fn a_joined_clause_times_every_word() {
        let config = VoiceConfig::from_json(JSON).unwrap();
        let text = "is this the end";
        let mut p = Table(vec![(text, "ɪz ðɪsðə ˈend")]);
        let prep = prepare(&config, &mut p, text, &clauses(text)).unwrap();
        assert!(!prep.exact);
        let mut seen: Vec<u32> = prep.owners.iter().flatten().copied().collect();
        seen.dedup();
        assert_eq!(seen, vec![0, 1, 2, 3]);
    }

    #[test]
    fn word_starts_follow_durations() {
        let owners = [None, None, Some(0), Some(0), None, Some(1), Some(1), None];
        let frames = [2.0, 1.0, 3.0, 1.0, 2.0, 4.0, 1.0, 1.0];
        assert_eq!(
            word_starts(&owners, &frames, 256.0),
            vec![(0, 3 * 256), (1, 9 * 256)]
        );
    }

    proptest::proptest! {
        #[test]
        fn alignment_is_monotonic_and_complete(
            lens in proptest::collection::vec(1usize..12, 1..10),
            groups in proptest::collection::vec("[a-zəɪ]{1,8}", 0..14),
        ) {
            let g: Vec<&str> = groups.iter().map(String::as_str).collect();
            let out = align(&lens, &g);
            proptest::prop_assert_eq!(out.len(), g.len());
            let idx: Vec<u32> = out.iter().map(|o| o.unwrap()).collect();
            proptest::prop_assert!(idx.windows(2).all(|w| w[0] <= w[1]));
            proptest::prop_assert!(idx.iter().all(|&i| (i as usize) < lens.len()));
        }
    }
}
