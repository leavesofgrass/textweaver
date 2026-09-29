//! Extractive summaries without a model (ADR-0037).
//!
//! [`summarize`] picks the sentences that best stand for a document and
//! returns them in document order, each with its canonical range, so a
//! reader can jump to it. The method is LexRank (Erkan and Radev, 2004):
//!
//! 1. The document's sentences come from `textweaver-text`'s sentence
//!    units, so a summary sentence starts and ends where navigation and
//!    speech say it does. Headings, tables, code blocks, image text, and
//!    footnote bodies are left out, and so are sentences of fewer than
//!    [`Options::min_words`] words.
//! 2. Each sentence becomes a TF-IDF vector over its words: lowercase,
//!    English stop words left out ([`is_stop_word`]), no stemming.
//! 3. Every two sentences are linked, weighted by the cosine of their
//!    vectors: continuous LexRank, with no threshold. The cosine matrix is
//!    never built; with unit vectors as the rows of a sparse matrix N it is
//!    N N^T, so each step multiplies by N^T and then by N, in time linear
//!    in the number of words.
//! 4. Power iteration with damping 0.85 finds each sentence's centrality.
//! 5. The top `k` sentences, in document order, are the summary. Ties go
//!    to the earlier sentence, so the result is the same on every run.
//!
//! Big documents are sampled so a summary stays fast (under 200 ms on
//! 10 MB in a release build): a text longer than
//! [`Options::text_budget`] characters is read in [`SAMPLE_WINDOWS`]
//! windows spread evenly through it, whole paragraphs at a time, and at
//! most [`Options::max_candidates`] of the sentences found, again spread
//! evenly, are ranked. [`Summary`] says whether that happened.
//!
//! No model, no download, no dependency beyond core and text.

use std::collections::HashMap;

use textweaver_core::{CharPos, CharRange, MarkerKind, Unit};
use textweaver_text::{Document, segments_in};

mod stopwords;

pub use stopwords::is_stop_word;

/// The damping factor of the power iteration.
pub const DAMPING: f64 = 0.85;

/// The default number of sentences in a summary (`[summary] sentences`).
pub const DEFAULT_SENTENCES: usize = 5;

/// The most sentences ranked by default.
pub const DEFAULT_MAX_CANDIDATES: usize = 50_000;

/// The most characters read by default; a longer text is sampled.
pub const DEFAULT_TEXT_BUDGET: usize = 600_000;

/// How many evenly spread windows a sampled text is read in.
pub const SAMPLE_WINDOWS: usize = 64;

/// Power iteration stops when the scores move less than this in total.
const TOLERANCE: f64 = 1e-10;

/// Power iteration stops after this many rounds, converged or not.
const MAX_ITERATIONS: usize = 200;

/// How a summary is made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// How many sentences the summary has, at most.
    pub sentences: usize,
    /// The most sentences ranked; more are sampled evenly.
    pub max_candidates: usize,
    /// The most characters read; a longer text is sampled in
    /// [`SAMPLE_WINDOWS`] windows spread evenly through it.
    pub text_budget: usize,
    /// Sentences with fewer words than this are not candidates.
    pub min_words: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            sentences: DEFAULT_SENTENCES,
            max_candidates: DEFAULT_MAX_CANDIDATES,
            text_budget: DEFAULT_TEXT_BUDGET,
            min_words: 4,
        }
    }
}

/// One sentence of a summary.
#[derive(Clone, Debug, PartialEq)]
pub struct Sentence {
    /// Where it is in the document (canonical chars).
    pub range: CharRange,
    /// Its text on one line: runs of white space become one space.
    pub text: String,
    /// Its LexRank centrality (the scores of all ranked sentences sum
    /// to 1).
    pub score: f64,
}

/// A summary and how it was made.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Summary {
    /// The chosen sentences, in document order.
    pub sentences: Vec<Sentence>,
    /// How many sentences were ranked.
    pub ranked: usize,
    /// How many candidate sentences were found in the text read.
    pub candidates: usize,
    /// How many characters of paragraphs were read.
    pub chars_read: usize,
    /// How many characters the text summarized has.
    pub chars: usize,
    /// True when the text was longer than the budget and read in windows.
    pub windowed: bool,
}

impl Summary {
    /// True when the text was sampled: a part of it was read, or some
    /// candidate sentences were not ranked.
    pub fn sampled(&self) -> bool {
        self.windowed || self.ranked < self.candidates
    }
}

/// The `k` sentences that best stand for `doc`, in document order, with
/// their ranges and text. Deterministic.
pub fn summarize(doc: &Document, k: usize) -> Vec<(CharRange, String)> {
    let options = Options {
        sentences: k,
        ..Options::default()
    };
    summarize_with(doc, &options)
        .sentences
        .into_iter()
        .map(|s| (s.range, s.text))
        .collect()
}

/// A summary of `doc` made with `options`.
pub fn summarize_with(doc: &Document, options: &Options) -> Summary {
    summarize_range(doc, doc.full_range(), options)
}

/// A summary of the part of `doc` inside `range` (a chapter, say).
pub fn summarize_range(doc: &Document, range: CharRange, options: &Options) -> Summary {
    let range = range.clamp_to(doc.len_chars());
    let read = paragraphs_to_read(doc, range, options.text_budget);
    let skip = skipped_ranges(doc, range);
    let mut vocabulary = Vocabulary::default();
    let mut ranges: Vec<CharRange> = Vec::new();
    let mut vectors: Vec<Vec<u32>> = Vec::new();
    let mut terms = Vec::new();
    for p in &read {
        for s in segments_in(doc, Unit::Sentence, *p) {
            // A sentence cut by a sampling window is not a candidate.
            if s.is_empty() || !p.contains_range(s) || skip.covers(s.start) {
                continue;
            }
            terms.clear();
            let chars = doc.text().slice(s.start.0..s.end.0).chars();
            let words = for_each_word(chars, |w| terms.push(vocabulary.id(w)));
            if words >= options.min_words && !terms.is_empty() {
                ranges.push(s);
                vectors.push(terms.clone());
            }
        }
    }
    let found = ranges.len();
    let chosen = spread(found, options.max_candidates);
    let ranked: Vec<Vec<u32>> = if chosen.len() == found {
        vectors
    } else {
        chosen
            .iter()
            .map(|&i| std::mem::take(&mut vectors[i]))
            .collect()
    };
    let scores = lexrank_ids(ranked, vocabulary.len());
    let mut order: Vec<usize> = (0..chosen.len()).collect();
    order.sort_by(|&a, &b| scores[b].total_cmp(&scores[a]).then(a.cmp(&b)));
    order.truncate(options.sentences);
    order.sort_unstable();
    let sentences = order
        .into_iter()
        .map(|i| {
            let range = ranges[chosen[i]];
            Sentence {
                range,
                text: one_line(&doc.slice(range)),
                score: scores[i],
            }
        })
        .collect();
    Summary {
        sentences,
        ranked: chosen.len(),
        candidates: found,
        chars_read: read.iter().map(CharRange::len).sum(),
        chars: range.len(),
        windowed: range.len() > options.text_budget,
    }
}

/// LexRank centrality of each sentence, given its terms (stop words
/// already out, as [`tokenize`] leaves them). The scores sum to 1; an
/// empty input gives an empty output.
pub fn lexrank<S: AsRef<str>>(sentences: &[&[S]]) -> Vec<f64> {
    let mut vocabulary = Vocabulary::default();
    let vectors = sentences
        .iter()
        .map(|terms| terms.iter().map(|t| vocabulary.id(t.as_ref())).collect())
        .collect();
    lexrank_ids(vectors, vocabulary.len())
}

/// Term ids, given in the order terms are first seen.
#[derive(Default)]
struct Vocabulary(HashMap<String, u32>);

impl Vocabulary {
    fn id(&mut self, term: &str) -> u32 {
        if let Some(&id) = self.0.get(term) {
            return id;
        }
        let id = u32::try_from(self.0.len()).unwrap_or(u32::MAX);
        self.0.insert(term.to_owned(), id);
        id
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

/// [`lexrank`] over term ids below `vocabulary`.
fn lexrank_ids(sentences: Vec<Vec<u32>>, vocabulary: usize) -> Vec<f64> {
    let n = sentences.len();
    if n == 0 {
        return Vec::new();
    }
    // Term frequencies per sentence, sorted by id.
    let mut vectors: Vec<Vec<(u32, f64)>> = sentences
        .into_iter()
        .map(|mut v| {
            v.sort_unstable();
            let mut tf: Vec<(u32, f64)> = Vec::with_capacity(v.len());
            for id in v {
                match tf.last_mut() {
                    Some((last, count)) if *last == id => *count += 1.0,
                    _ => tf.push((id, 1.0)),
                }
            }
            tf
        })
        .collect();
    // Document frequency and inverse document frequency.
    let mut df = vec![0u32; vocabulary];
    for v in &vectors {
        for &(id, _) in v {
            df[id as usize] += 1;
        }
    }
    let total = n as f64;
    let idf: Vec<f64> = df
        .iter()
        .map(|&d| {
            if d == 0 {
                0.0
            } else {
                (total / f64::from(d)).ln()
            }
        })
        .collect();
    // TF-IDF weights, each sentence's vector scaled to length 1 (terms in
    // every sentence weigh nothing and are dropped).
    for v in &mut vectors {
        v.retain_mut(|(id, w)| {
            *w *= idf[*id as usize];
            *w > 0.0
        });
        let norm = v.iter().map(|(_, w)| w * w).sum::<f64>().sqrt();
        for (_, w) in v.iter_mut() {
            *w /= norm;
        }
    }
    // With the unit vectors as the rows of N, the cosine matrix is
    // S = N N^T, so (S - I) x, the links without a sentence's link to
    // itself, takes two passes over the vectors: no n-by-n matrix is built.
    let mut by_term = vec![0.0f64; vocabulary];
    let mut links_times = |x: &[f64], out: &mut [f64]| {
        by_term.fill(0.0);
        for (v, &xi) in vectors.iter().zip(x) {
            for &(id, w) in v {
                by_term[id as usize] += w * xi;
            }
        }
        for ((v, &xi), o) in vectors.iter().zip(x).zip(out.iter_mut()) {
            let s: f64 = v.iter().map(|&(id, w)| w * by_term[id as usize]).sum();
            // A sentence with a vector is similar to itself by 1.
            *o = if v.is_empty() { 0.0 } else { (s - xi).max(0.0) };
        }
    };
    // Each sentence's total similarity to the others (rounding leftovers
    // count as none).
    let mut row_sums = vec![0.0f64; n];
    links_times(&vec![1.0; n], &mut row_sums);
    for r in &mut row_sums {
        if *r < 1e-12 {
            *r = 0.0;
        }
    }
    // Power iteration: p' = (1 - d) / n + d M^T p, where M is S - I with
    // each row divided by its sum; a sentence like no other spreads its
    // score evenly. S is symmetric, so M^T p = (S - I) (p / row sums).
    let base = (1.0 - DAMPING) / total;
    let mut p = vec![1.0 / total; n];
    let mut share = vec![0.0f64; n];
    let mut next = vec![0.0f64; n];
    for _ in 0..MAX_ITERATIONS {
        let mut dangling = 0.0;
        for ((s, &pi), &r) in share.iter_mut().zip(&p).zip(&row_sums) {
            if r == 0.0 {
                dangling += pi;
                *s = 0.0;
            } else {
                *s = pi / r;
            }
        }
        links_times(&share, &mut next);
        let even = base + DAMPING * dangling / total;
        for x in &mut next {
            *x = even + DAMPING * *x;
        }
        let moved: f64 = p.iter().zip(&next).map(|(a, b)| (a - b).abs()).sum();
        std::mem::swap(&mut p, &mut next);
        if moved < TOLERANCE {
            break;
        }
    }
    p
}

/// Splits `text` into words and returns how many words it has and its
/// terms: lowercase words of two or more characters, not all digits, not
/// stop words. An apostrophe inside a word joins it (`don't` is `dont`),
/// and a final `'s` is dropped.
pub fn tokenize(text: &str) -> (usize, Vec<String>) {
    let mut terms = Vec::new();
    let words = for_each_word(text.chars(), |t| terms.push(t.to_owned()));
    (words, terms)
}

/// Calls `term` with each term of `chars` (see [`tokenize`]) and returns
/// how many words there are, stop words included.
fn for_each_word(chars: impl Iterator<Item = char>, mut term: impl FnMut(&str)) -> usize {
    let mut words = 0;
    let mut word = String::new();
    let mut finish = |word: &mut String| {
        if word.is_empty() {
            return;
        }
        words += 1;
        if word.ends_with("'s") {
            word.truncate(word.len() - 2);
        }
        word.retain(|c| c != '\'');
        let long = word.chars().nth(1).is_some();
        if long && !word.chars().all(|c| c.is_ascii_digit()) && !is_stop_word(word) {
            term(word);
        }
        word.clear();
    };
    let mut chars = chars.peekable();
    while let Some(c) = chars.next() {
        if c.is_alphanumeric() {
            word.extend(c.to_lowercase());
        } else if matches!(c, '\'' | '\u{2019}')
            && !word.is_empty()
            && chars.peek().is_some_and(|n| n.is_alphanumeric())
        {
            word.push('\'');
        } else {
            finish(&mut word);
        }
    }
    finish(&mut word);
    words
}

/// `s` on one line: runs of white space become one space, trimmed.
fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// At most `max` of `0..n`, spread evenly and in order (all of them when
/// `n <= max`).
fn spread(n: usize, max: usize) -> Vec<usize> {
    if n <= max {
        return (0..n).collect();
    }
    // The i-th of `max` picks is floor(i * n / max): strictly increasing,
    // since n > max.
    (0..max).map(|i| i * n / max).collect()
}

/// The paragraphs to split into sentences, inside `range`: all of them
/// when `range` fits in `budget` characters; else, in each of
/// [`SAMPLE_WINDOWS`] windows spread evenly through `range`, the
/// paragraphs in it, until the window's share of the budget is used. A
/// paragraph that begins before the window, or is longer than the share,
/// is cut; only whole sentences inside a piece are candidates.
fn paragraphs_to_read(doc: &Document, range: CharRange, budget: usize) -> Vec<CharRange> {
    let inside = |p: CharRange| p.intersection(range).filter(|p| !p.is_empty());
    if range.len() <= budget {
        return segments_in(doc, Unit::Paragraph, range)
            .into_iter()
            .filter_map(inside)
            .collect();
    }
    let share = (budget / SAMPLE_WINDOWS).max(1);
    let mut out = Vec::new();
    let mut read_to = range.start;
    for w in 0..SAMPLE_WINDOWS {
        let start = CharPos(range.start.0 + w * range.len() / SAMPLE_WINDOWS).max(read_to);
        let window = CharRange::new(start, CharPos((start.0 + share).min(range.end.0)));
        if window.is_empty() {
            continue;
        }
        let mut used = 0;
        for p in segments_in(doc, Unit::Paragraph, window) {
            let Some(p) = inside(p) else {
                continue;
            };
            let start = p.start.max(window.start).max(read_to);
            if start >= p.end {
                continue;
            }
            let left = share - used;
            let piece = if p.end.0 - start.0 <= left {
                CharRange::new(start, p.end)
            } else if used == 0 {
                // A paragraph longer than the share: its first part.
                CharRange::new(start, CharPos(start.0 + left))
            } else {
                break;
            };
            used += piece.len();
            read_to = piece.end;
            out.push(piece);
            if used >= share {
                break;
            }
        }
    }
    out
}

/// Ranges whose sentences are never summary sentences, sorted and merged.
struct SkipSet(Vec<CharRange>);

impl SkipSet {
    /// True when `pos` is inside a skipped range.
    fn covers(&self, pos: CharPos) -> bool {
        let i = self.0.partition_point(|r| r.end <= pos);
        self.0.get(i).is_some_and(|r| r.contains(pos))
    }
}

/// Headings, tables, code blocks, image text, and footnote bodies that
/// meet `range`.
fn skipped_ranges(doc: &Document, range: CharRange) -> SkipSet {
    let mut ranges: Vec<CharRange> = doc
        .markers()
        .iter()
        .filter(|m| match m.kind {
            MarkerKind::Heading | MarkerKind::Table | MarkerKind::Image => true,
            MarkerKind::Code | MarkerKind::Footnote => m.level == 1,
            _ => false,
        })
        .map(|m| m.range)
        .filter(|r| !r.is_empty() && r.intersects(range))
        .collect();
    ranges.sort_by_key(|r| (r.start, r.end));
    let mut merged: Vec<CharRange> = Vec::with_capacity(ranges.len());
    for r in ranges {
        match merged.last_mut() {
            Some(last) if r.start <= last.end => *last = last.cover(r),
            _ => merged.push(r),
        }
    }
    SkipSet(merged)
}

#[cfg(test)]
mod tests;
