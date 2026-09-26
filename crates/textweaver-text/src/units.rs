//! Text units: the grapheme, word, sentence, line, or paragraph at a position.
//!
//! Units are produced by segment iterators that walk the rope one *block* at
//! a time and never materialize the whole document:
//!
//! | Unit | Block segmented | Rule |
//! |---|---|---|
//! | Grapheme | a line with its line break | UAX #29 extended grapheme clusters; graphemes tile the text |
//! | Word | a line | UAX #29 word segments that contain an alphanumeric char, with hyphenated compounds (`well-known`, `12-14`) joined into one word as Star's `\b\w[\w'-]*` does |
//! | Sentence | a paragraph | UAX #29 sentence boundaries, refined (see below) |
//! | Line | a line | the line without its line break; blank lines are empty ranges |
//! | Paragraph | a run of non-blank lines | blank (whitespace-only) lines separate paragraphs |
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

/// How a unit's segments are grouped into blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlockKind {
    /// One line, including its line break.
    LineWithBreak,
    /// One line, without its line break.
    Line,
    /// A run of non-blank lines.
    Paragraph,
    /// The whole document.
    Whole,
}

/// A block of text: its char range and the lines it spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Block {
    range: CharRange,
    first_line: usize,
    last_line: usize,
}

fn block_kind(unit: Unit) -> BlockKind {
    match unit {
        Unit::Grapheme => BlockKind::LineWithBreak,
        Unit::Word | Unit::Line => BlockKind::Line,
        Unit::Sentence | Unit::Paragraph => BlockKind::Paragraph,
        Unit::Document | Unit::Marker { .. } => BlockKind::Whole,
    }
}

fn line_block(doc: &Document, line: usize, with_break: bool) -> Block {
    let mut range = doc.line_range(line);
    if with_break {
        let next = if line + 1 < doc.line_count() {
            doc.text().line_to_char(line + 1)
        } else {
            doc.len_chars()
        };
        range = CharRange::new(range.start, next);
    }
    Block {
        range,
        first_line: line,
        last_line: line,
    }
}

fn paragraph_block(doc: &Document, first: usize, last: usize) -> Block {
    Block {
        range: CharRange::new(doc.line_range(first).start, doc.line_range(last).end),
        first_line: first,
        last_line: last,
    }
}

/// The paragraph containing non-blank line `line`.
fn paragraph_around(doc: &Document, line: usize) -> Block {
    let mut first = line;
    while first > 0 && !doc.line_is_blank(first - 1) {
        first -= 1;
    }
    let mut last = line;
    let n = doc.line_count();
    while last + 1 < n && !doc.line_is_blank(last + 1) {
        last += 1;
    }
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
        }),
        BlockKind::Line | BlockKind::LineWithBreak => {
            (line < n).then(|| line_block(doc, line, kind == BlockKind::LineWithBreak))
        }
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
        if joinable {
            if let Some(prev) = out.last_mut() {
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
        let line = doc.line_of(from.clamp_to(doc.len_chars()));
        let block = block_from_line(doc, kind, line, dir);
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
    }
}
