//! Text units: the grapheme, word, sentence, line, or paragraph at a position.
//!
//! Units are produced by segment iterators that walk the rope one *block* at
//! a time and never materialize the whole document:
//!
//! | Unit | Block segmented | Rule |
//! |---|---|---|
//! | Grapheme | a line with its line break (in windows when long) | UAX #29 extended grapheme clusters; graphemes tile the text |
//! | Word | a line (in windows when long) | UAX #29 word segments that contain an alphanumeric char, with hyphenated compounds (`well-known`, `12-14`) joined into one word as Star's `\b\w[\w'-]*` does |
//! | Sentence | a paragraph | UAX #29 sentence boundaries, refined (see below) |
//! | Line | a line | the line without its line break; blank lines are empty ranges |
//! | Paragraph | a run of non-blank lines | blank (whitespace-only) lines separate paragraphs |
//!
//! Lines longer than a few thousand chars are segmented into words and
//! graphemes in windows split where UAX #29 always has a boundary (before a
//! whitespace char that follows a non-whitespace char), so a word step in a
//! one-line, megabyte-long file stays fast; the result is the same as
//! segmenting the whole line (property-tested).
//!
//! Paragraphs longer than [`SENTENCE_WINDOW`] × 2 chars (a PDF or text file
//! without blank lines) are segmented into sentences in windows too. Window
//! `k` starts at the first true sentence start at or after
//! `paragraph start + k × SENTENCE_WINDOW`, found by segmenting a short stretch
//! around that point and trusting only the sentences whose context lies
//! wholly inside it. The result equals segmenting the whole paragraph as long
//! as no sentence (with the space after it) is longer than half of
//! [`SENTENCE_LOOKAROUND`] (property-tested); a single "sentence" longer than
//! [`MAX_SENTENCE`] chars, which only machine-made text has, is split at a
//! space.
//!
//! Sentence refinements, in order:
//!
//! 1. A single line break inside a paragraph is a soft wrap (plain-text files
//!    keep their lines) unless a block marker starts on the next line or the
//!    break is inside a code block: list items, table rows, and code lines
//!    are sentences of their own (Star ran them together, bug 4).
//! 2. A sentence ending in an abbreviation from [`ABBREVIATIONS`] (`Dr.`,
//!    `Mr.`, `e.g.`, `pp.`, ...) continues into the next one (Star split
//!    after every `Dr.`, bug 11). One ending in an entry of
//!    [`AMBIGUOUS_ABBREVIATIONS`] (`a.m.`, `etc.`, ...) continues only when
//!    the next one does not start with a capital letter.
//! 3. An ellipsis (`…`) followed by whitespace and a capital letter ends a
//!    sentence, as in Star.
//! 4. A footnote reference (`footnote.[1] It`) stays with the sentence it
//!    ends; inside a code block every line is one sentence.
//!
//! Sentences and words exclude surrounding whitespace. Differences from Star
//! are measured by `cargo xtask parity` (docs/parity-report.md).

use std::collections::VecDeque;

use textweaver_core::{CharPos, CharRange, Direction, MarkerKind, Unit};
use unicode_segmentation::UnicodeSegmentation;

use crate::Document;
use crate::document::is_line_break;

/// Abbreviations whose final period never ends a sentence.
pub const ABBREVIATIONS: &[&str] = &[
    "Mr.", "Mrs.", "Ms.", "Mx.", "Dr.", "Prof.", "St.", "Mt.", "Ft.", "Rev.", "Fr.", "Gen.",
    "Gov.", "Sen.", "Rep.", "Hon.", "Capt.", "Lt.", "Col.", "Sgt.", "Cmdr.", "Adm.", "Pres.",
    "e.g.", "i.e.", "E.g.", "I.e.", "cf.", "Cf.", "vs.", "viz.", "approx.", "ca.", "pp.", "p.",
    "Fig.", "Figs.", "fig.", "Eq.", "Eqs.", "eq.", "No.", "Nos.", "Vol.", "vol.", "Sec.", "sec.",
    "Ch.", "ch.", "Chap.", "Ref.", "Refs.", "Dept.", "Univ.", "Assoc.", "ed.", "eds.", "Ave.",
    "Blvd.", "Rd.",
];

/// Abbreviations that often end a sentence too: the sentence continues only
/// when the next word does not start with a capital letter ("at 9:30 a.m. on
/// Friday", but "until 5 p.m. Then we left").
pub const AMBIGUOUS_ABBREVIATIONS: &[&str] = &[
    "a.m.", "p.m.", "A.M.", "P.M.", "etc.", "al.", "Jr.", "Sr.", "Inc.", "Ltd.", "Corp.", "Co.",
    "min.", "hr.", "hrs.", "est.", "approx.",
];

/// Paragraphs longer than twice this many chars are segmented into
/// sentences one window of about this size at a time.
#[cfg(not(test))]
pub const SENTENCE_WINDOW: usize = 8192;
/// Paragraphs longer than twice this many chars are segmented into
/// sentences one window of about this size at a time.
#[cfg(test)]
pub const SENTENCE_WINDOW: usize = 48;

/// How far (in chars) sentence windowing looks back and ahead of a window
/// edge to find a true sentence start.
#[cfg(not(test))]
pub const SENTENCE_LOOKAROUND: usize = 1024;
/// How far (in chars) sentence windowing looks back and ahead of a window
/// edge to find a true sentence start.
#[cfg(test)]
pub const SENTENCE_LOOKAROUND: usize = 64;

/// In a windowed paragraph, a run this long without a sentence boundary is
/// split at a space.
#[cfg(not(test))]
pub const MAX_SENTENCE: usize = 65536;
/// In a windowed paragraph, a run this long without a sentence boundary is
/// split at a space.
#[cfg(test)]
pub const MAX_SENTENCE: usize = 256;

/// How a unit's segments are grouped into blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlockKind {
    /// A window of a line (the whole line unless it is long), including
    /// the line break after the last window when `with_break` is set.
    Window {
        /// Include the line break in the last window.
        with_break: bool,
    },
    /// One line, without its line break.
    Line,
    /// A run of non-blank lines.
    Paragraph,
    /// A run of non-blank lines, or a window of one when it is long.
    Sentences,
    /// The whole document.
    Whole,
}

/// A block of text: its char range and the lines it spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Block {
    range: CharRange,
    first_line: usize,
    last_line: usize,
    /// For a sentence window, the paragraph it is part of.
    para: Option<Para>,
}

/// The paragraph a sentence window belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Para {
    range: CharRange,
    first_line: usize,
    last_line: usize,
}

impl Para {
    fn block(self) -> Block {
        Block {
            range: self.range,
            first_line: self.first_line,
            last_line: self.last_line,
            para: None,
        }
    }
}

fn block_kind(unit: Unit) -> BlockKind {
    match unit {
        Unit::Grapheme => BlockKind::Window { with_break: true },
        Unit::Word => BlockKind::Window { with_break: false },
        Unit::Line => BlockKind::Line,
        Unit::Sentence => BlockKind::Sentences,
        Unit::Paragraph => BlockKind::Paragraph,
        Unit::Document | Unit::Marker { .. } => BlockKind::Whole,
    }
}

fn line_block(doc: &Document, line: usize) -> Block {
    Block {
        range: doc.line_range(line),
        first_line: line,
        last_line: line,
        para: None,
    }
}

/// Target window size, in chars, for segmenting words and graphemes of a
/// long line: a caret move in a one-line, megabyte-long text file segments a
/// few thousand chars, not the whole line.
#[cfg(not(test))]
const WINDOW: usize = 2048;
#[cfg(test)]
const WINDOW: usize = 8;

/// Start of window `k` of the line `ls..le`: the first safe split at or after
/// `ls + k * WINDOW`. A safe split is a whitespace char that follows a
/// non-whitespace char other than a zero-width joiner; UAX #29 always has
/// both a word and a grapheme boundary there.
fn window_start(doc: &Document, ls: usize, le: usize, k: usize) -> usize {
    if k == 0 {
        return ls;
    }
    let p = ls.saturating_add(k.saturating_mul(WINDOW));
    if p >= le {
        return le;
    }
    let text = doc.text();
    let mut prev = text.char(p - 1);
    for (i, c) in text.chars_at(p).take(le - p).enumerate() {
        if c.is_whitespace() && !prev.is_whitespace() && prev != '\u{200d}' {
            return p + i;
        }
        prev = c;
    }
    le
}

/// The window of line `line` containing `pos` (clamped to the line).
fn window_block(doc: &Document, line: usize, pos: usize, with_break: bool) -> Block {
    let r = doc.line_range(line);
    let (ls, le) = (r.start.0, r.end.0);
    let (mut a, mut b) = (ls, le);
    if le - ls > 2 * WINDOW {
        let pos = pos.clamp(ls, le);
        let mut k = (pos - ls) / WINDOW;
        while k > 0 && window_start(doc, ls, le, k) > pos {
            k -= 1;
        }
        loop {
            let next = window_start(doc, ls, le, k + 1);
            if next <= pos && next < le {
                k += 1;
            } else {
                break;
            }
        }
        a = window_start(doc, ls, le, k);
        b = window_start(doc, ls, le, k + 1);
    }
    if with_break && b == le {
        b = if line + 1 < doc.line_count() {
            doc.text().line_to_char(line + 1)
        } else {
            doc.len_chars()
        };
    }
    Block {
        range: CharRange::new(a, b),
        first_line: line,
        last_line: line,
        para: None,
    }
}

fn paragraph_block(doc: &Document, first: usize, last: usize) -> Block {
    Block {
        range: CharRange::new(doc.line_range(first).start, doc.line_range(last).end),
        first_line: first,
        last_line: last,
        para: None,
    }
}

/// The paragraph containing non-blank line `line`.
fn paragraph_around(doc: &Document, line: usize) -> Block {
    let (first, last) = doc.paragraph_lines(line).unwrap_or((line, line));
    paragraph_block(doc, first, last)
}

/// The first block at or after `line` (forward) or at or before it (backward).
fn block_from_line(doc: &Document, kind: BlockKind, line: usize, dir: Direction) -> Option<Block> {
    let n = doc.line_count();
    match kind {
        BlockKind::Whole => Some(Block {
            range: doc.full_range(),
            first_line: 0,
            last_line: n - 1,
            para: None,
        }),
        BlockKind::Sentences => {
            let p = block_from_line(doc, BlockKind::Paragraph, line, dir)?;
            let pos = match dir {
                Direction::Forward => p.range.start.0,
                Direction::Backward => p.range.end.0.saturating_sub(1),
            };
            Some(sentence_window(doc, &p, pos))
        }
        BlockKind::Line => (line < n).then(|| line_block(doc, line)),
        BlockKind::Window { with_break } => (line < n).then(|| {
            let r = doc.line_range(line);
            let pos = match dir {
                Direction::Forward => r.start.0,
                Direction::Backward => r.end.0.saturating_sub(1).max(r.start.0),
            };
            window_block(doc, line, pos, with_break)
        }),
        BlockKind::Paragraph => {
            if line >= n {
                return None;
            }
            let mut l = line;
            match dir {
                Direction::Forward => {
                    while l < n && doc.line_is_blank(l) {
                        l += 1;
                    }
                    (l < n).then(|| paragraph_around(doc, l))
                }
                Direction::Backward => loop {
                    if !doc.line_is_blank(l) {
                        return Some(paragraph_around(doc, l));
                    }
                    if l == 0 {
                        return None;
                    }
                    l -= 1;
                },
            }
        }
    }
}

fn next_block(doc: &Document, kind: BlockKind, b: &Block, dir: Direction) -> Option<Block> {
    if let Some(p) = b.para {
        match dir {
            Direction::Forward if b.range.end < p.range.end => {
                return Some(sentence_window(doc, &p.block(), b.range.end.0));
            }
            Direction::Backward if b.range.start > p.range.start => {
                return Some(sentence_window(doc, &p.block(), b.range.start.0 - 1));
            }
            _ => return next_block(doc, kind, &p.block(), dir),
        }
    }
    if let BlockKind::Window { with_break } = kind {
        let line = doc.line_range(b.first_line);
        match dir {
            Direction::Forward if b.range.end < line.end => {
                return Some(window_block(doc, b.first_line, b.range.end.0, with_break));
            }
            Direction::Backward if b.range.start > line.start => {
                let pos = b.range.start.0 - 1;
                return Some(window_block(doc, b.first_line, pos, with_break));
            }
            _ => {}
        }
    }
    match (kind, dir) {
        (BlockKind::Whole, _) => None,
        (_, Direction::Forward) => block_from_line(doc, kind, b.last_line + 1, dir),
        (_, Direction::Backward) => {
            if b.first_line == 0 {
                None
            } else {
                block_from_line(doc, kind, b.first_line - 1, dir)
            }
        }
    }
}

/// True when a window may start at `i`: a whitespace char right after a
/// non-whitespace char other than a zero-width joiner (UAX #29 has a word
/// and a grapheme boundary there, and no token continues across it).
fn is_safe_split(doc: &Document, i: usize) -> bool {
    if i == 0 || i >= doc.len_chars() {
        return false;
    }
    let text = doc.text();
    let (prev, c) = (text.char(i - 1), text.char(i));
    c.is_whitespace() && !prev.is_whitespace() && prev != '\u{200d}'
}

/// The last safe split at or before `x` and not before `lo`, looking back at
/// most [`SENTENCE_LOOKAROUND`] chars; `lo` when the scan reaches it, else `x`.
fn split_at_or_before(doc: &Document, x: usize, lo: usize) -> usize {
    let stop = x.saturating_sub(SENTENCE_LOOKAROUND).max(lo);
    let mut i = x;
    while i > stop {
        if is_safe_split(doc, i) {
            return i;
        }
        i -= 1;
    }
    if stop == lo { lo } else { x }
}

/// The first safe split at or after `x`, or `hi`.
fn split_at_or_after(doc: &Document, x: usize, hi: usize) -> usize {
    if x >= hi {
        return hi;
    }
    let text = doc.text();
    let mut prev = if x == 0 { ' ' } else { text.char(x - 1) };
    for (i, c) in text.chars_at(x).take(hi - x).enumerate() {
        if c.is_whitespace() && !prev.is_whitespace() && prev != '\u{200d}' {
            return x + i;
        }
        prev = c;
    }
    hi
}

/// A block over `range` (inside one paragraph) with the lines it spans.
fn sub_block(doc: &Document, range: CharRange, para: Option<Para>) -> Block {
    let last = range.end.0.saturating_sub(1).max(range.start.0);
    Block {
        range,
        first_line: doc.line_of(range.start),
        last_line: doc.line_of(CharPos(last)),
        para,
    }
}

/// The first true sentence start at or after `q` in paragraph `p` (or the
/// paragraph end): the start of a sentence found by segmenting a stretch
/// around `q` whose context lies inside the stretch (neither the first
/// sentence, which may have lost its beginning, nor the last, which may have
/// lost its end, unless they touch the paragraph's edges).
fn sentence_start_at_or_after(doc: &Document, p: &Block, q: usize) -> usize {
    let (ps, pe) = (p.range.start.0, p.range.end.0);
    let a = split_at_or_before(doc, q.saturating_sub(SENTENCE_LOOKAROUND).max(ps), ps);
    let mut ahead = SENTENCE_LOOKAROUND;
    loop {
        let z = split_at_or_after(doc, q.saturating_add(ahead), pe);
        let local = sub_block(doc, CharRange::new(a, z), None);
        let segs = segment_block(doc, Unit::Sentence, &local);
        let lo = usize::from(a > ps);
        let hi = if z == pe {
            segs.len()
        } else {
            segs.len().saturating_sub(1)
        };
        if let Some(r) = segs
            .get(lo..hi.max(lo))
            .and_then(|s| s.iter().find(|r| r.start.0 >= q))
        {
            return r.start.0;
        }
        if z == pe {
            return pe;
        }
        if ahead >= MAX_SENTENCE {
            // No boundary in a very long run: split it at a space.
            return split_at_or_after(doc, q, pe);
        }
        ahead = ahead.saturating_mul(2);
    }
}

/// Start of sentence window `k` of paragraph `p`.
fn sentence_boundary(doc: &Document, p: &Block, k: usize) -> usize {
    let (ps, pe) = (p.range.start.0, p.range.end.0);
    if k == 0 {
        return ps;
    }
    let q = ps.saturating_add(k.saturating_mul(SENTENCE_WINDOW));
    if q >= pe {
        return pe;
    }
    sentence_start_at_or_after(doc, p, q)
}

/// The sentence window of paragraph `p` containing `pos`: the paragraph
/// itself unless it is long.
fn sentence_window(doc: &Document, p: &Block, pos: usize) -> Block {
    let (ps, pe) = (p.range.start.0, p.range.end.0);
    if pe - ps <= 2 * SENTENCE_WINDOW {
        return *p;
    }
    let pos = pos.clamp(ps, pe - 1);
    let mut k = (pos - ps) / SENTENCE_WINDOW;
    let mut start = sentence_boundary(doc, p, k);
    while k > 0 && start > pos {
        k -= 1;
        start = sentence_boundary(doc, p, k);
    }
    let mut end = sentence_boundary(doc, p, k + 1).max(start);
    while end <= pos && end < pe {
        k += 1;
        start = end;
        end = sentence_boundary(doc, p, k + 1).max(start);
    }
    let para = Para {
        range: p.range,
        first_line: p.first_line,
        last_line: p.last_line,
    };
    sub_block(doc, CharRange::new(start, end), Some(para))
}

/// Char offset of every byte boundary in `s`, as a lookup from byte to char.
struct ByteToChar {
    starts: Vec<usize>,
}

impl ByteToChar {
    fn new(s: &str) -> Self {
        let mut starts: Vec<usize> = s.char_indices().map(|(b, _)| b).collect();
        starts.push(s.len());
        ByteToChar { starts }
    }

    /// Char index of byte offset `b` (a char boundary).
    fn char_of(&self, b: usize) -> usize {
        self.starts.partition_point(|&x| x < b)
    }
}

fn trimmed(chars: &[char], start: usize, end: usize) -> Option<(usize, usize)> {
    let mut s = start;
    let mut e = end;
    while s < e && chars[s].is_whitespace() {
        s += 1;
    }
    while e > s && chars[e - 1].is_whitespace() {
        e -= 1;
    }
    (s < e).then_some((s, e))
}

fn is_hyphen(s: &str) -> bool {
    matches!(s, "-" | "\u{2010}" | "\u{2011}")
}

/// Word segments of one line of text, as char ranges relative to its start.
fn words_in(text: &str) -> Vec<(usize, usize)> {
    let map = ByteToChar::new(text);
    // (start, end, is_word) for every UAX #29 word-boundary segment.
    let segs: Vec<(usize, usize, bool, bool)> = text
        .split_word_bound_indices()
        .map(|(b, s)| {
            (
                map.char_of(b),
                map.char_of(b + s.len()),
                s.chars().any(char::is_alphanumeric),
                is_hyphen(s),
            )
        })
        .collect();
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < segs.len() {
        let (s, mut e, word, _) = segs[i];
        if !word {
            i += 1;
            continue;
        }
        // Join `word - word` chains into one hyphenated compound.
        while i + 2 < segs.len() && segs[i + 1].3 && segs[i + 2].2 && segs[i + 1].0 == e {
            e = segs[i + 2].1;
            i += 2;
        }
        out.push((s, e));
        i += 1;
    }
    out
}

fn is_abbreviation(token: &str, list: &[&str]) -> bool {
    list.contains(&token)
}

/// The last whitespace-separated token of `chars`, without leading openers.
fn last_token(chars: &[char]) -> String {
    let start = chars
        .iter()
        .rposition(|c| c.is_whitespace())
        .map_or(0, |i| i + 1);
    chars[start..]
        .iter()
        .skip_while(|c| matches!(c, '(' | '[' | '"' | '\'' | '\u{201c}' | '\u{2018}'))
        .collect()
}

/// Sentence segments of one paragraph, as char ranges relative to its start.
/// `hard_break(i)` says whether the line break at char `i` separates
/// sentences; no sentence boundary falls strictly inside an `atomic` range
/// (a footnote reference such as `[1]` after a period).
fn sentences_in(
    text: &str,
    hard_break: impl Fn(usize) -> bool,
    atomic: &[(usize, usize)],
) -> Vec<(usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    // Soft line breaks become spaces (char for char, so offsets hold).
    let prepared: String = chars
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            if is_line_break(c) && !hard_break(i) {
                ' '
            } else {
                c
            }
        })
        .collect();
    let map = ByteToChar::new(&prepared);
    let mut raw: Vec<(usize, usize)> = Vec::new();
    for (b, s) in prepared.split_sentence_bound_indices() {
        let (a, z) = (map.char_of(b), map.char_of(b + s.len()));
        split_at_ellipses(&chars, a, z, &mut raw);
    }
    // Trim, then merge across abbreviations.
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut joinable = false;
    for (a, z) in raw {
        let Some((s, e)) = trimmed(&chars, a, z) else {
            continue;
        };
        if joinable && let Some(prev) = out.last_mut() {
            let gap_has_break = (prev.1..s).any(|i| is_line_break(chars[i]) && hard_break(i));
            let next_upper = chars[s..e]
                .iter()
                .find(|c| c.is_alphanumeric())
                .is_some_and(|c| c.is_uppercase());
            let token = last_token(&chars[prev.0..prev.1]);
            let always = is_abbreviation(&token, ABBREVIATIONS);
            if !gap_has_break && (always || !next_upper) {
                prev.1 = e;
                joinable = ends_with_abbreviation(&chars[prev.0..prev.1]);
                continue;
            }
        }
        out.push((s, e));
        joinable = ends_with_abbreviation(&chars[s..e]);
    }
    keep_atomic_ranges_whole(&chars, &mut out, atomic);
    out
}

/// Moves any sentence boundary that falls inside an atomic range to its end.
fn keep_atomic_ranges_whole(
    chars: &[char],
    out: &mut Vec<(usize, usize)>,
    atomic: &[(usize, usize)],
) {
    for &(a, z) in atomic {
        let mut i = 0;
        while i + 1 < out.len() {
            let (end, next) = (out[i].1, out[i + 1].0);
            let splits = (a < end && end < z) || (a < next && next < z);
            if splits {
                out[i].1 = out[i].1.max(z.min(out[i + 1].1));
                let mut s = out[i].1.max(out[i + 1].0);
                while s < out[i + 1].1 && chars[s].is_whitespace() {
                    s += 1;
                }
                if s >= out[i + 1].1 {
                    out[i].1 = out[i].1.max(out[i + 1].1);
                    out.remove(i + 1);
                    continue;
                }
                out[i + 1].0 = s;
            }
            i += 1;
        }
    }
}

fn ends_with_abbreviation(chars: &[char]) -> bool {
    let token = last_token(chars);
    is_abbreviation(&token, ABBREVIATIONS) || is_abbreviation(&token, AMBIGUOUS_ABBREVIATIONS)
}

/// Splits `a..z` after every `…` that is followed by whitespace and a capital.
fn split_at_ellipses(chars: &[char], a: usize, z: usize, out: &mut Vec<(usize, usize)>) {
    let mut start = a;
    let mut i = a;
    while i < z {
        if chars[i] == '\u{2026}' {
            let mut j = i + 1;
            while j < z && chars[j].is_whitespace() {
                j += 1;
            }
            if j > i + 1 && j < z && chars[j].is_uppercase() {
                out.push((start, j));
                start = j;
                i = j;
                continue;
            }
        }
        i += 1;
    }
    if start < z {
        out.push((start, z));
    }
}

/// Positions (absolute) of line breaks inside `block` that separate
/// sentences: those followed by the start of a block marker, and those
/// inside a code block.
fn hard_breaks(doc: &Document, block: &Block) -> Vec<CharPos> {
    if doc.markers().is_empty() {
        return Vec::new();
    }
    let index = doc.marker_index();
    let mut out = Vec::new();
    for line in block.first_line..block.last_line {
        let brk = doc.line_range(line).end;
        if brk < block.range.start || brk >= block.range.end {
            continue;
        }
        let next_start = doc.line_range(line + 1).start;
        let starts_block = index.starting_at(next_start).iter().any(|m| m.is_block());
        let in_code = index
            .enclosing(MarkerKind::Code, brk)
            .is_some_and(|m| m.level == 1);
        if starts_block || in_code {
            out.push(brk);
        }
    }
    out
}

/// True when the whole block lies in a code block: its sentences are its
/// lines (prose rules would split `println!("hi")` after the `!`).
fn in_code_block(doc: &Document, block: &Block) -> bool {
    !doc.markers().is_empty()
        && doc
            .marker_index()
            .enclosing(MarkerKind::Code, block.range.start)
            .is_some_and(|m| m.level == 1 && block.range.end <= m.range.end)
}

/// The segments of `unit` inside one block, as absolute ranges, in order.
fn segment_block(doc: &Document, unit: Unit, block: &Block) -> Vec<CharRange> {
    let base = block.range.start.0;
    let abs = |(s, e): (usize, usize)| CharRange::new(base + s, base + e);
    match unit {
        Unit::Line | Unit::Paragraph | Unit::Document => vec![block.range],
        Unit::Grapheme => {
            let text = doc.slice(block.range);
            let map = ByteToChar::new(&text);
            text.grapheme_indices(true)
                .map(|(b, g)| abs((map.char_of(b), map.char_of(b + g.len()))))
                .collect()
        }
        Unit::Word => words_in(&doc.slice(block.range))
            .into_iter()
            .map(abs)
            .collect(),
        Unit::Sentence if in_code_block(doc, block) => (block.first_line..=block.last_line)
            .filter_map(|l| {
                let r = doc.line_range(l);
                let text: Vec<char> = doc.slice(r).chars().collect();
                trimmed(&text, 0, text.len())
                    .map(|(s, e)| CharRange::new(r.start.0 + s, r.start.0 + e))
            })
            .collect(),
        Unit::Sentence => {
            let breaks = hard_breaks(doc, block);
            let atomic: Vec<(usize, usize)> = if doc.markers().is_empty() {
                Vec::new()
            } else {
                doc.marker_index()
                    .starting_in(block.range)
                    .iter()
                    .filter(|m| m.kind == MarkerKind::Footnote && m.level == 0)
                    .map(|m| {
                        (
                            m.range.start.0 - base,
                            m.range.end.0.min(block.range.end.0) - base,
                        )
                    })
                    .collect()
            };
            let text = doc.slice(block.range);
            sentences_in(&text, |i| breaks.contains(&CharPos(base + i)), &atomic)
                .into_iter()
                .map(abs)
                .collect()
        }
        Unit::Marker { kind, level } => doc
            .marker_index()
            .iter(kind, level)
            .map(|m| m.range)
            .collect(),
    }
}

/// Iterates the ranges of one unit through the document, block by block.
///
/// Forward, it yields every unit not entirely before `from` (the unit
/// containing `from` first, if any), in document order. Backward, it yields
/// every unit starting before `from`, nearest first.
pub struct Units<'a> {
    doc: &'a Document,
    unit: Unit,
    kind: BlockKind,
    dir: Direction,
    from: CharPos,
    block: Option<Block>,
    buf: VecDeque<CharRange>,
}

impl<'a> Units<'a> {
    /// An iterator over `unit` starting at `from` in direction `dir`.
    pub fn new(doc: &'a Document, unit: Unit, from: CharPos, dir: Direction) -> Self {
        let kind = block_kind(unit);
        let at = from.clamp_to(doc.len_chars());
        let line = doc.line_of(at);
        let block = match kind {
            BlockKind::Window { with_break } => Some(window_block(doc, line, at.0, with_break)),
            BlockKind::Sentences => {
                block_from_line(doc, BlockKind::Paragraph, line, dir).map(|p| {
                    let last = p.range.end.0.saturating_sub(1).max(p.range.start.0);
                    sentence_window(doc, &p, at.0.clamp(p.range.start.0, last))
                })
            }
            _ => block_from_line(doc, kind, line, dir),
        };
        let mut it = Units {
            doc,
            unit,
            kind,
            dir,
            from,
            block,
            buf: VecDeque::new(),
        };
        it.fill_current();
        it
    }

    fn fill_current(&mut self) {
        let Some(b) = self.block else {
            return;
        };
        let segs = segment_block(self.doc, self.unit, &b);
        let from = self.from;
        match self.dir {
            Direction::Forward => self
                .buf
                .extend(segs.into_iter().filter(|r| r.end > from || r.start >= from)),
            Direction::Backward => self
                .buf
                .extend(segs.into_iter().rev().filter(|r| r.start < from)),
        }
    }
}

impl Iterator for Units<'_> {
    type Item = CharRange;

    fn next(&mut self) -> Option<CharRange> {
        loop {
            if let Some(r) = self.buf.pop_front() {
                return Some(r);
            }
            let b = self.block?;
            if matches!(self.unit, Unit::Marker { .. }) {
                self.block = None;
                return None;
            }
            self.block = next_block(self.doc, self.kind, &b, self.dir);
            self.block?;
            self.fill_current();
        }
    }
}

/// All ranges of `unit` in the document, in order.
pub fn segments(doc: &Document, unit: Unit) -> Vec<CharRange> {
    Units::new(doc, unit, CharPos::ZERO, Direction::Forward).collect()
}

/// The ranges of `unit` that intersect `range`, in order (units that end
/// exactly at `range.start` are excluded, empty units inside it included).
pub fn segments_in(doc: &Document, unit: Unit, range: CharRange) -> Vec<CharRange> {
    Units::new(doc, unit, range.start, Direction::Forward)
        .take_while(|r| r.start < range.end)
        .collect()
}

/// The range of `unit` containing `pos`, or the next one after it.
pub fn unit_at(doc: &Document, pos: CharPos, unit: Unit) -> Option<CharRange> {
    if let Unit::Marker { kind, level } = unit {
        let index = doc.marker_index();
        let inner = index
            .containing(pos)
            .filter(|m| m.kind == kind && level.is_none_or(|l| m.level == l))
            .min_by_key(|m| m.range.len());
        if let Some(m) = inner {
            return Some(m.range);
        }
        return index
            .step(kind, level, pos, Direction::Forward, false)
            .map(|(m, _)| m.range);
    }
    Units::new(doc, unit, pos, Direction::Forward).next()
}

/// The first `unit` starting after `pos`.
pub fn next_unit(doc: &Document, pos: CharPos, unit: Unit) -> Option<CharRange> {
    Units::new(doc, unit, pos, Direction::Forward).find(|r| r.start > pos)
}

/// The last `unit` starting before `pos`.
pub fn prev_unit(doc: &Document, pos: CharPos, unit: Unit) -> Option<CharRange> {
    Units::new(doc, unit, pos, Direction::Backward).next()
}

/// The first `unit` in the document.
pub fn first_unit(doc: &Document, unit: Unit) -> Option<CharRange> {
    Units::new(doc, unit, CharPos::ZERO, Direction::Forward).next()
}

/// The last `unit` in the document.
pub fn last_unit(doc: &Document, unit: Unit) -> Option<CharRange> {
    let end = doc.end().saturating_add(1);
    Units::new(doc, unit, end, Direction::Backward).next()
}

#[cfg(test)]
mod tests {
    use ropey::Rope;
    use textweaver_core::MarkerKind;

    use super::*;
    use crate::{DocumentMeta, Marker};

    fn texts(doc: &Document, unit: Unit) -> Vec<String> {
        segments(doc, unit)
            .into_iter()
            .map(|r| doc.slice(r))
            .collect()
    }

    #[test]
    fn units_on_plain_text() {
        let d = Document::from_plain_text("One two. Three!\n\nFour five?");
        assert_eq!(segments(&d, Unit::Word).len(), 5);
        assert_eq!(segments(&d, Unit::Sentence).len(), 3);
        assert_eq!(segments(&d, Unit::Paragraph).len(), 2);
        assert_eq!(
            unit_at(&d, CharPos(5), Unit::Word),
            Some(CharRange::new(4, 7))
        );
    }

    #[test]
    fn words_follow_uax29_with_hyphen_compounds() {
        let d = Document::from_plain_text(
            "O'Brien's well-known essay (pp. 12-14) cost $12.50; e.g. 2.0.1 at 9:30 — café 3rd.",
        );
        assert_eq!(
            texts(&d, Unit::Word),
            [
                "O'Brien's",
                "well-known",
                "essay",
                "pp",
                "12-14",
                "cost",
                "12.50",
                "e.g",
                "2.0.1",
                "at",
                "9",
                "30",
                "café",
                "3rd"
            ]
        );
    }

    #[test]
    fn sentences_respect_abbreviations() {
        let d = Document::from_plain_text(
            "Dr. Smith opened the library at 9:30 a.m. on Friday. She had\nread 1,250 pages. Mr. O'Brien (pp. 12-14) argues otherwise... but few agree. The café on 3rd St. sells crème brûlée.",
        );
        assert_eq!(
            texts(&d, Unit::Sentence),
            [
                "Dr. Smith opened the library at 9:30 a.m. on Friday.",
                "She had\nread 1,250 pages.",
                "Mr. O'Brien (pp. 12-14) argues otherwise... but few agree.",
                "The café on 3rd St. sells crème brûlée.",
            ]
        );
    }

    #[test]
    fn ambiguous_abbreviation_ends_before_capital() {
        let d = Document::from_plain_text("We left at 5 p.m. Then it rained. See e.g. The Book.");
        assert_eq!(
            texts(&d, Unit::Sentence),
            ["We left at 5 p.m.", "Then it rained.", "See e.g. The Book."]
        );
    }

    #[test]
    fn ellipsis_before_capital_ends_sentence() {
        let d = Document::from_plain_text("Wait\u{2026} Then go\u{2026} now.");
        assert_eq!(
            texts(&d, Unit::Sentence),
            ["Wait\u{2026}", "Then go\u{2026} now."]
        );
    }

    #[test]
    fn list_items_are_separate_sentences() {
        let text = "Intro\n\nFirst item\nSecond item\nthird item";
        let markers = vec![
            Marker::new(MarkerKind::List, CharRange::new(7, 40)).with_level(1),
            Marker::new(MarkerKind::ListItem, CharRange::new(7, 17)).with_level(1),
            Marker::new(MarkerKind::ListItem, CharRange::new(18, 29)).with_level(1),
            Marker::new(MarkerKind::ListItem, CharRange::new(30, 40)).with_level(1),
        ];
        let d = Document::new(DocumentMeta::default(), Rope::from_str(text), markers);
        assert_eq!(
            texts(&d, Unit::Sentence),
            ["Intro", "First item", "Second item", "third item"]
        );
        // Without markers the lines are soft-wrapped prose.
        let plain = Document::from_plain_text(text);
        assert_eq!(
            texts(&plain, Unit::Sentence),
            ["Intro", "First item\nSecond item\nthird item"]
        );
    }

    #[test]
    fn documented_differences_from_star() {
        // Trailing hyphen and apostrophe are not part of the word.
        let d = Document::from_plain_text("rock- and roll, the students' books");
        assert_eq!(
            texts(&d, Unit::Word),
            ["rock", "and", "roll", "the", "students", "books"]
        );
        // Non-ASCII capitals start sentences; three periods before a capital
        // end one.
        let d = Document::from_plain_text("Il est parti. Élan revint... Puis rien.");
        assert_eq!(
            texts(&d, Unit::Sentence),
            ["Il est parti.", "Élan revint...", "Puis rien."]
        );
    }

    #[test]
    fn code_block_sentences_are_lines() {
        let text = "Run it.\n\nfn main() {\n    println!(\"hi\"); x();\n}";
        let markers = vec![
            Marker::new(MarkerKind::Code, CharRange::new(9, text.chars().count())).with_level(1),
        ];
        let d = Document::new(DocumentMeta::default(), Rope::from_str(text), markers);
        assert_eq!(
            texts(&d, Unit::Sentence),
            ["Run it.", "fn main() {", "println!(\"hi\"); x();", "}"]
        );
    }

    #[test]
    fn lines_and_paragraphs() {
        let d = Document::from_plain_text("a b\n\n  \nc\nd\n");
        assert_eq!(texts(&d, Unit::Line), ["a b", "", "  ", "c", "d"]);
        assert_eq!(texts(&d, Unit::Paragraph), ["a b", "c\nd"]);
        let g = segments(&d, Unit::Grapheme);
        assert_eq!(g.len(), d.len_chars());
    }

    #[test]
    fn next_prev_and_ends() {
        let d = Document::from_plain_text("One. Two.\n\nThree. Four.");
        let s = |r: Option<CharRange>| r.map(|r| d.slice(r));
        assert_eq!(
            s(next_unit(&d, CharPos(0), Unit::Sentence)).as_deref(),
            Some("Two.")
        );
        assert_eq!(
            s(next_unit(&d, CharPos(6), Unit::Sentence)).as_deref(),
            Some("Three.")
        );
        assert_eq!(
            s(prev_unit(&d, CharPos(11), Unit::Sentence)).as_deref(),
            Some("Two.")
        );
        assert_eq!(
            s(prev_unit(&d, CharPos(12), Unit::Sentence)).as_deref(),
            Some("Three.")
        );
        assert_eq!(s(prev_unit(&d, CharPos(0), Unit::Sentence)), None);
        assert_eq!(
            s(first_unit(&d, Unit::Paragraph)).as_deref(),
            Some("One. Two.")
        );
        assert_eq!(s(last_unit(&d, Unit::Sentence)).as_deref(), Some("Four."));
        assert_eq!(s(last_unit(&d, Unit::Word)).as_deref(), Some("Four"));
        assert_eq!(
            s(unit_at(&d, CharPos(9), Unit::Sentence)).as_deref(),
            Some("Three.")
        );
        assert_eq!(
            segments_in(&d, Unit::Sentence, CharRange::new(2, 12)).len(),
            3
        );
    }

    #[test]
    fn empty_document() {
        let d = Document::from_plain_text("");
        assert!(segments(&d, Unit::Word).is_empty());
        assert!(segments(&d, Unit::Sentence).is_empty());
        assert_eq!(segments(&d, Unit::Line), [CharRange::empty(0)]);
        assert!(last_unit(&d, Unit::Word).is_none());
    }
}

#[cfg(test)]
mod props {
    use proptest::prelude::*;

    use super::*;

    fn doc_text() -> impl Strategy<Value = String> {
        proptest::collection::vec(
            prop_oneof![
                Just("Dr. "),
                Just("word "),
                Just("Two"),
                Just(". "),
                Just("! "),
                Just("\n"),
                Just("\n\n"),
                Just("well-known "),
                Just("é"),
                Just("e.g. "),
                Just("  "),
                Just("A"),
                Just("1,250"),
                Just("…"),
                Just("\u{2028}"),
                Just("🇫🇷"),
                Just("e\u{301}"),
            ],
            0..40,
        )
        .prop_map(|v| v.concat())
    }

    fn ordered_in_bounds(v: &[CharRange], len: usize) -> bool {
        v.windows(2).all(|w| w[0].end <= w[1].start) && v.iter().all(|r| r.end.0 <= len)
    }

    proptest! {
        #[test]
        fn graphemes_tile_the_text(text in doc_text()) {
            let d = Document::from_plain_text(&text);
            let g = segments(&d, Unit::Grapheme);
            let mut at = 0;
            for r in &g {
                prop_assert_eq!(r.start.0, at);
                prop_assert!(!r.is_empty());
                at = r.end.0;
            }
            prop_assert_eq!(at, d.len_chars());
        }

        #[test]
        fn units_are_ordered_and_nest(text in doc_text()) {
            let d = Document::from_plain_text(&text);
            let len = d.len_chars();
            let lines = segments(&d, Unit::Line);
            let paras = segments(&d, Unit::Paragraph);
            let words = segments(&d, Unit::Word);
            let sents = segments(&d, Unit::Sentence);
            for v in [&lines, &paras, &words, &sents] {
                prop_assert!(ordered_in_bounds(v, len));
            }
            for w in &words {
                prop_assert!(!w.is_empty());
                prop_assert!(lines.iter().any(|l| l.contains_range(*w)));
                prop_assert!(sents.iter().any(|s| s.contains_range(*w)));
            }
            for s in &sents {
                prop_assert!(!s.is_empty());
                prop_assert!(paras.iter().any(|p| p.contains_range(*s)));
                let t = d.slice(*s);
                prop_assert_eq!(t.trim(), t.as_str());
            }
        }

        #[test]
        fn windows_segment_like_whole_lines(text in doc_text()) {
            // WINDOW is 8 chars under test, so long lines are windowed.
            let d = Document::from_plain_text(&text);
            let mut words = Vec::new();
            for l in 0..d.line_count() {
                let r = d.line_range(l);
                for (s, e) in words_in(&d.slice(r)) {
                    words.push(CharRange::new(r.start.0 + s, r.start.0 + e));
                }
            }
            prop_assert_eq!(segments(&d, Unit::Word), words);
            let whole = d.text().to_string();
            let map = ByteToChar::new(&whole);
            let graphemes: Vec<CharRange> = whole
                .grapheme_indices(true)
                .map(|(b, g)| CharRange::new(map.char_of(b), map.char_of(b + g.len())))
                .collect();
            prop_assert_eq!(segments(&d, Unit::Grapheme), graphemes);
        }

        #[test]
        fn iteration_agrees_with_segments(text in doc_text(), p in 0usize..200) {
            let d = Document::from_plain_text(&text);
            let pos = CharPos(p.min(d.len_chars()));
            for unit in [Unit::Word, Unit::Sentence, Unit::Line, Unit::Paragraph, Unit::Grapheme] {
                let all = segments(&d, unit);
                let next = all.iter().find(|r| r.start > pos).copied();
                prop_assert_eq!(next_unit(&d, pos, unit), next);
                let prev = all.iter().rev().find(|r| r.start < pos).copied();
                prop_assert_eq!(prev_unit(&d, pos, unit), prev);
                let at = all.iter().find(|r| r.end > pos || r.start >= pos).copied();
                prop_assert_eq!(unit_at(&d, pos, unit), at);
                prop_assert_eq!(last_unit(&d, unit), all.last().copied());
            }
        }

        #[test]
        fn windowed_sentences_match_whole_paragraphs(text in short_sentences(), p in 0usize..600) {
            // SENTENCE_WINDOW is 48 chars under test, so most paragraphs here
            // are windowed.
            let d = Document::from_plain_text(&text);
            let whole = unwindowed_sentences(&d);
            let short = SENTENCE_LOOKAROUND / 2;
            prop_assume!(whole.iter().all(|r| r.len() < short));
            prop_assume!(whole.windows(2).all(|w| w[1].start.0 - w[0].start.0 < short));
            let all = segments(&d, Unit::Sentence);
            prop_assert_eq!(&all, &whole);
            let pos = CharPos(p.min(d.len_chars()));
            let next = all.iter().find(|r| r.start > pos).copied();
            prop_assert_eq!(next_unit(&d, pos, Unit::Sentence), next);
            let prev = all.iter().rev().find(|r| r.start < pos).copied();
            prop_assert_eq!(prev_unit(&d, pos, Unit::Sentence), prev);
            let at = all.iter().find(|r| r.end > pos || r.start >= pos).copied();
            prop_assert_eq!(unit_at(&d, pos, Unit::Sentence), at);
        }

        #[test]
        fn windowed_sentences_are_ordered_even_when_long(text in doc_text(), reps in 1usize..8) {
            // Long runs without boundaries are split, never overlapped.
            let d = Document::from_plain_text(&text.repeat(reps));
            let sents = segments(&d, Unit::Sentence);
            prop_assert!(ordered_in_bounds(&sents, d.len_chars()));
            let mut back: Vec<CharRange> =
                Units::new(&d, Unit::Sentence, d.end(), Direction::Backward).collect();
            back.reverse();
            let before_end: Vec<CharRange> =
                sents.iter().copied().filter(|r| r.start < d.end()).collect();
            prop_assert_eq!(back, before_end);
        }
    }

    fn short_sentences() -> impl Strategy<Value = String> {
        proptest::collection::vec(
            prop_oneof![
                Just("Hi. "),
                Just("Dr. X. "),
                Just("go! "),
                Just("e.g. "),
                Just("A b? "),
                Just("\n"),
                Just("\n\n"),
                Just("é. "),
                Just("Wait\u{2026} "),
                Just("1,250. "),
                Just("well-known. "),
                Just("a.m. "),
                Just("Yes.[1] "),
            ],
            0..150,
        )
        .prop_map(|v| v.concat())
    }

    /// Sentences found by segmenting every paragraph whole.
    fn unwindowed_sentences(d: &Document) -> Vec<CharRange> {
        let mut out = Vec::new();
        let mut line = 0;
        while let Some(p) = block_from_line(d, BlockKind::Paragraph, line, Direction::Forward) {
            out.extend(segment_block(d, Unit::Sentence, &p));
            line = p.last_line + 1;
        }
        out
    }
}
