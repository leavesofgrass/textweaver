//! The document's text as AccessKit text runs: pure data, no widgets.
//!
//! AccessKit exposes a document as a node with `TextRun` children. Each run
//! carries its text (`value`), the UTF-8 length of each of its characters
//! (`character_lengths`), and where its words start (`word_starts`, a `u8`
//! per word, so a run holds at most [`MAX_RUN_CHARS`] characters). Runs on
//! the same line are chained (`next_on_line`); a paragraph's last run ends
//! with its `\n`, which is how AccessKit finds paragraph ends.
//!
//! A "character" here is a grapheme cluster, as screen readers expect when
//! they move by character, so an accented letter written with a combining
//! mark, or an emoji sequence, is one character. Document positions are
//! [`CharPos`] (Unicode scalar values); [`RunSet::position`] and
//! [`RunSet::char_pos`] convert between the two.
//!
//! Runs are keyed by their document start, so a run keeps its AccessKit id
//! while its text stays put: moving the spoken word changes only the runs of
//! the visual lines it leaves and reaches (ADR-0027, "Stable run ids"; a
//! paragraph's runs are kept by line in [`ParaRuns`]).

use std::ops::Range;

use textweaver_app::core::{CharPos, CharRange};
use unicode_segmentation::UnicodeSegmentation;

/// The most characters (grapheme clusters) in one run: `word_starts` holds
/// `u8` indices, and 255 leaves room for the index one past the end.
pub const MAX_RUN_CHARS: usize = 255;

/// The most UTF-8 bytes in one run character (`character_lengths` is `u8`).
/// A longer cluster (stacked combining marks) is split at scalar values.
const MAX_CHAR_BYTES: usize = 255;

/// Why a run is highlighted, for its text attributes. Only the highlights a
/// screen reader can report as attributes are listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RunMark {
    /// The word being spoken: a background colour.
    SpokenWord,
    /// A search match.
    FindHit,
}

/// One paragraph of the window: canonical text between line breaks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Paragraph {
    /// Where it starts in the document.
    pub start: CharPos,
    /// Its text, without the line break.
    pub text: String,
    /// True when a `\n` follows it in the document.
    pub has_break: bool,
    /// Heading level (1 to 6), when the paragraph is a heading.
    pub heading: Option<u8>,
    /// Its list item, when the paragraph starts one: drawn only (a bullet
    /// or number in the hanging indent); its text and runs do not change.
    pub list: Option<ListMark>,
    /// Where its visual lines start, as char offsets into `text` (sorted,
    /// without 0), when the paragraph has been laid out. `None`: not laid
    /// out; the paragraph is then one line for accessibility.
    pub line_starts: Option<Vec<usize>>,
}

/// A list item's mark, from the document's `ListItem` marker: its depth
/// and, for an ordered item, its number.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListMark {
    /// The nesting depth, from 1.
    pub level: u8,
    /// The ordered item's label ("2."); `None` draws a bullet.
    pub label: Option<String>,
}

impl ListMark {
    /// What is drawn before the item: its number, or a bullet by depth (a
    /// disc, a circle, then a square, as browsers draw them), so the
    /// nesting shows by shape as well as by indent.
    pub fn glyph(&self) -> &str {
        match &self.label {
            Some(l) => l.as_str(),
            None => match self.level.max(1) % 3 {
                1 => "\u{2022}",
                2 => "\u{25E6}",
                _ => "\u{25AA}",
            },
        }
    }
}

impl Paragraph {
    /// Chars in the paragraph, not counting the break.
    pub fn len_chars(&self) -> usize {
        self.text.chars().count()
    }

    /// Chars including the break.
    pub fn span_chars(&self) -> usize {
        self.len_chars() + usize::from(self.has_break)
    }
}

/// Stable identity of a run: its document start. Two runs never start at
/// the same place.
pub type RunKey = CharPos;

/// One text run.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    /// Document position of its first char.
    pub start: CharPos,
    /// Index of its paragraph in the window.
    pub paragraph: usize,
    /// The text, with the paragraph's `\n` on its last run.
    pub text: String,
    /// UTF-8 length of each character.
    pub char_lens: Vec<u8>,
    /// Unicode scalar values in each character (1 for most).
    pub char_scalars: Vec<u8>,
    /// Indices of the characters where words start.
    pub word_starts: Vec<u8>,
    /// True when the next run is on the same line.
    pub continues_line: bool,
    /// Its highlight, if any.
    pub mark: Option<RunMark>,
    /// Heading level of its paragraph.
    pub heading: Option<u8>,
}

impl Run {
    /// Its key (see [`RunKey`]).
    pub fn key(&self) -> RunKey {
        self.start
    }

    /// Chars (scalar values) in the run.
    pub fn len_chars(&self) -> usize {
        self.char_scalars.iter().map(|&n| usize::from(n)).sum()
    }

    /// Document range of the run.
    pub fn range(&self) -> CharRange {
        CharRange::new(self.start.0, self.start.0 + self.len_chars())
    }

    /// Characters (clusters) in the run.
    pub fn len(&self) -> usize {
        self.char_lens.len()
    }

    /// True for a run with no characters (only in an empty document).
    pub fn is_empty(&self) -> bool {
        self.char_lens.is_empty()
    }
}

/// A position in the runs: a run index and a character index in it (which
/// may be the run's length, meaning its end).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunPos {
    /// Index into [`RunSet::runs`].
    pub run: usize,
    /// Character (cluster) index within the run.
    pub index: usize,
}

/// The runs of a window, in document order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunSet {
    /// The runs.
    pub runs: Vec<Run>,
}

/// A character of a paragraph: its byte range in the paragraph text and
/// how many scalar values it has.
#[derive(Clone, Debug)]
struct Cluster {
    bytes: Range<usize>,
    scalars: usize,
    word_start: bool,
}

/// Splits `text` into characters (grapheme clusters, split further when a
/// cluster is longer than [`MAX_CHAR_BYTES`]) and marks word starts.
fn clusters(text: &str) -> Vec<Cluster> {
    // Word starts and cluster starts both come in text order, so one walk
    // over each finds them (no set of every word start).
    let mut words = text
        .split_word_bound_indices()
        .filter(|(_, w)| w.chars().any(|c| !c.is_whitespace()))
        .map(|(i, _)| i)
        .peekable();
    let mut word_start_at = |i: usize| {
        while words.next_if(|&w| w < i).is_some() {}
        words.peek() == Some(&i)
    };
    let mut out = Vec::with_capacity(text.len());
    for (i, g) in text.grapheme_indices(true) {
        let word_start = word_start_at(i);
        if g.len() <= MAX_CHAR_BYTES {
            out.push(Cluster {
                bytes: i..i + g.len(),
                scalars: g.chars().count(),
                word_start,
            });
            continue;
        }
        // A pathological cluster: split at scalar values.
        let mut piece_start = i;
        let mut scalars = 0;
        let mut first = true;
        for (j, c) in g.char_indices() {
            let at = i + j;
            if at + c.len_utf8() - piece_start > MAX_CHAR_BYTES {
                out.push(Cluster {
                    bytes: piece_start..at,
                    scalars,
                    word_start: first && word_start,
                });
                first = false;
                piece_start = at;
                scalars = 0;
            }
            scalars += 1;
        }
        out.push(Cluster {
            bytes: piece_start..i + g.len(),
            scalars,
            word_start: first && word_start,
        });
    }
    out
}

/// The runs of one paragraph, grouped by visual line
/// ([`Paragraph::line_starts`]; a paragraph not laid out is one group).
///
/// A visual line always ends a run, so a line's runs depend only on its
/// own characters and the marks over them. When a mark moves, only the
/// lines it touches are built again ([`ParaRuns::rebuild`]): a highlight
/// move on a paragraph of a thousand lines costs a line or two, and the
/// other lines keep their runs, and so their AccessKit ids (the
/// performance report's G5). [`RunSet::build`] is these groups joined.
#[derive(Clone, Debug, Default)]
pub struct ParaRuns {
    paragraph: usize,
    start: CharPos,
    has_break: bool,
    heading: Option<u8>,
    clusters: Vec<Cluster>,
    /// Char offset (within the paragraph) where each cluster starts, then
    /// the paragraph's length.
    starts: Vec<usize>,
    /// The cluster index where each line starts, then the cluster count.
    bounds: Vec<usize>,
    lines: Vec<Vec<Run>>,
}

impl ParaRuns {
    /// The runs of paragraph `p` (index `pi` in the window), split at
    /// `line_starts` (char offsets into its text, sorted, without 0) and
    /// at `marks` (document ranges; later entries win where they overlap).
    pub fn new(
        pi: usize,
        p: &Paragraph,
        line_starts: Option<&[usize]>,
        marks: &[(CharRange, RunMark)],
    ) -> Self {
        let clusters = clusters(&p.text);
        let mut starts = Vec::with_capacity(clusters.len() + 1);
        let mut acc = 0usize;
        for c in &clusters {
            starts.push(acc);
            acc += c.scalars;
        }
        starts.push(acc);
        let n = clusters.len();
        let mut bounds = vec![0];
        for &l in line_starts.unwrap_or_default() {
            // The first cluster starting at or after the line's start.
            let at = starts.partition_point(|&s| s < l).min(n);
            if at > 0 && at < n {
                bounds.push(at);
            }
        }
        bounds.sort_unstable();
        bounds.dedup();
        bounds.push(n);
        let mut pr = ParaRuns {
            paragraph: pi,
            start: p.start,
            has_break: p.has_break,
            heading: p.heading,
            clusters,
            starts,
            bounds,
            lines: Vec::new(),
        };
        pr.lines = (0..pr.bounds.len() - 1)
            .map(|g| pr.build_line(g, &p.text, marks))
            .collect();
        pr
    }

    /// The runs, line by line. No line is empty.
    pub fn lines(&self) -> &[Vec<Run>] {
        &self.lines
    }

    /// Every run, in order.
    pub fn runs(&self) -> impl Iterator<Item = &Run> {
        self.lines.iter().flatten()
    }

    /// The runs, in order.
    pub fn into_runs(self) -> Vec<Run> {
        self.lines.into_iter().flatten().collect()
    }

    /// The lines holding any of the characters of `r` (the line of its
    /// start when it is empty), clamped to the paragraph.
    pub fn lines_touching(&self, r: CharRange) -> Range<usize> {
        let a = self.line_of(r.start);
        let b = self.line_of(CharPos(r.end.0.saturating_sub(1).max(r.start.0)));
        a..(b + 1).min(self.lines.len()).max(a)
    }

    /// The line holding the character at `pos` (clamped).
    fn line_of(&self, pos: CharPos) -> usize {
        let off = pos.0.saturating_sub(self.start.0);
        // The cluster holding `off`, then the lines starting at or before it.
        let cl = self.starts.partition_point(|&s| s <= off).saturating_sub(1);
        let inner = &self.bounds[1..self.bounds.len() - 1];
        inner.partition_point(|&b| b <= cl)
    }

    /// Builds `lines` again from `text` (the paragraph's, unchanged) with
    /// `marks`.
    pub fn rebuild(&mut self, lines: Range<usize>, text: &str, marks: &[(CharRange, RunMark)]) {
        for g in lines {
            if g < self.lines.len() {
                self.lines[g] = self.build_line(g, text, marks);
            }
        }
    }

    /// The run holding `pos` and the character index in it, as
    /// [`RunSet::position`] finds it in the paragraph's runs.
    pub fn position(&self, pos: CharPos) -> Option<(&Run, usize)> {
        let g = self
            .lines
            .partition_point(|l| l.first().is_some_and(|r| r.start.0 <= pos.0))
            .saturating_sub(1);
        let line = self.lines.get(g)?;
        let at = position_in(line, pos);
        let run = line.get(at.run)?;
        if at.index == run.len()
            && at.run + 1 == line.len()
            && let Some(next) = self.lines.get(g + 1).and_then(|l| l.first())
        {
            return Some((next, 0));
        }
        Some((run, at.index))
    }

    /// The document position of character `index` of the run keyed `key`
    /// (the index clamped to the run).
    pub fn char_pos(&self, key: RunKey, index: usize) -> Option<CharPos> {
        let g = self
            .lines
            .partition_point(|l| l.first().is_some_and(|r| r.start.0 <= key.0))
            .saturating_sub(1);
        let line = self.lines.get(g)?;
        let r = line.binary_search_by_key(&key.0, |r| r.start.0).ok()?;
        Some(char_pos_in(line, RunPos { run: r, index }))
    }

    /// The runs of line `g` with `marks`.
    fn build_line(&self, g: usize, text: &str, marks: &[(CharRange, RunMark)]) -> Vec<Run> {
        let n = self.clusters.len();
        let (ga, gb) = (self.bounds[g], self.bounds[g + 1]);
        let last_line = g + 2 == self.bounds.len();
        let p_start = self.start.0;
        let p_end = p_start + self.starts[n];
        let cluster_at = |char_off: usize| -> usize {
            // The first cluster starting at or after `char_off`.
            self.starts.partition_point(|&s| s < char_off).min(n)
        };
        let mut mark_of = vec![None; gb - ga];
        for (range, mark) in marks {
            if range.end.0 <= p_start || range.start.0 >= p_end || range.is_empty() {
                continue;
            }
            let a = cluster_at(range.start.0.saturating_sub(p_start)).max(ga);
            let b = cluster_at(range.end.0.min(p_end) - p_start).min(gb);
            if a >= b {
                continue;
            }
            for m in &mut mark_of[a - ga..b - ga] {
                *m = Some(*mark);
            }
        }
        let mut runs = Vec::new();
        if ga == gb {
            // A blank line: one run, which holds just the break (if any).
            runs.push(self.make_run(text, ga..ga, false, None));
        }
        // Walk the segments between mark changes, splitting long ones at
        // word starts (else hard) so no run exceeds MAX_RUN_CHARS. The
        // line's end ends the last one's line.
        let cuts = (ga + 1..gb)
            .filter(|&i| mark_of[i - ga] != mark_of[i - ga - 1])
            .map(|i| (i, false))
            .chain([(gb, true)]);
        let mut seg_start = ga;
        for (seg_end, next_is_line) in cuts {
            let mut a = seg_start;
            while a < seg_end {
                let mut b = seg_end.min(a + MAX_RUN_CHARS);
                if b < seg_end {
                    // Prefer to end at a word start in the back half.
                    if let Some(w) = (a + MAX_RUN_CHARS / 2..b)
                        .rev()
                        .find(|&i| self.clusters[i].word_start)
                    {
                        b = w;
                    }
                }
                let continues_line = !(b >= seg_end && next_is_line);
                runs.push(self.make_run(text, a..b, continues_line, mark_of[a - ga]));
                a = b;
            }
            seg_start = seg_end;
        }
        // The paragraph's break goes on its last run, which ends the line.
        if last_line
            && self.has_break
            && let Some(last) = runs.last_mut()
        {
            last.text.push('\n');
            last.char_lens.push(1);
            last.char_scalars.push(1);
            last.continues_line = false;
            if last.len() > MAX_RUN_CHARS {
                // Move the break to a run of its own.
                last.text.pop();
                last.char_lens.pop();
                last.char_scalars.pop();
                last.continues_line = true;
                runs.push(Run {
                    start: CharPos(p_end),
                    paragraph: self.paragraph,
                    text: "\n".into(),
                    char_lens: vec![1],
                    char_scalars: vec![1],
                    word_starts: Vec::new(),
                    continues_line: false,
                    mark: None,
                    heading: self.heading,
                });
            }
        }
        runs
    }

    fn make_run(
        &self,
        text: &str,
        range: Range<usize>,
        continues_line: bool,
        mark: Option<RunMark>,
    ) -> Run {
        let slice = &self.clusters[range.clone()];
        let bytes = match (slice.first(), slice.last()) {
            (Some(f), Some(l)) => f.bytes.start..l.bytes.end,
            _ => 0..0,
        };
        Run {
            start: CharPos(self.start.0 + self.starts[range.start]),
            paragraph: self.paragraph,
            text: text[bytes].to_owned(),
            char_lens: slice.iter().map(|c| c.bytes.len() as u8).collect(),
            char_scalars: slice.iter().map(|c| c.scalars.min(255) as u8).collect(),
            word_starts: slice
                .iter()
                .enumerate()
                .filter(|(_, c)| c.word_start)
                .map(|(i, _)| i as u8)
                .collect(),
            continues_line,
            mark,
            heading: self.heading,
        }
    }
}

/// The run position of `pos` in `runs` (as [`RunSet::position`]).
fn position_in(runs: &[Run], pos: CharPos) -> RunPos {
    let n = runs.len();
    if n == 0 {
        return RunPos { run: 0, index: 0 };
    }
    let i = runs
        .partition_point(|r| r.start.0 <= pos.0)
        .saturating_sub(1);
    let run = &runs[i];
    let mut off = pos.0.saturating_sub(run.start.0);
    let mut index = 0;
    for &s in &run.char_scalars {
        if off == 0 {
            break;
        }
        let s = usize::from(s);
        if off < s {
            // Inside a cluster: the cluster's start.
            break;
        }
        off -= s;
        index += 1;
    }
    if index == run.len() && i + 1 < n {
        return RunPos {
            run: i + 1,
            index: 0,
        };
    }
    RunPos { run: i, index }
}

/// The document position of `at` in `runs` (clamped; past the last run,
/// the end of the runs).
fn char_pos_in(runs: &[Run], at: RunPos) -> CharPos {
    let Some(run) = runs.get(at.run) else {
        return runs.last().map_or(CharPos::ZERO, |r| r.range().end);
    };
    let index = at.index.min(run.len());
    let off: usize = run.char_scalars[..index]
        .iter()
        .map(|&s| usize::from(s))
        .sum();
    CharPos(run.start.0 + off)
}

impl RunSet {
    /// Builds the runs for `paragraphs` with `marks` (document ranges; later
    /// entries win where they overlap).
    pub fn build(paragraphs: &[Paragraph], marks: &[(CharRange, RunMark)]) -> RunSet {
        let mut runs = Vec::new();
        for (pi, p) in paragraphs.iter().enumerate() {
            runs.extend(ParaRuns::new(pi, p, p.line_starts.as_deref(), marks).into_runs());
        }
        if runs.is_empty() {
            runs.push(Run {
                start: CharPos::ZERO,
                paragraph: 0,
                text: String::new(),
                char_lens: Vec::new(),
                char_scalars: Vec::new(),
                word_starts: Vec::new(),
                continues_line: false,
                mark: None,
                heading: None,
            });
        }
        RunSet { runs }
    }

    /// The document range the runs cover.
    pub fn range(&self) -> CharRange {
        match (self.runs.first(), self.runs.last()) {
            (Some(f), Some(l)) => CharRange::new(f.start.0, l.range().end.0),
            _ => CharRange::empty(0),
        }
    }

    /// The run position of `pos`, clamped to the runs. A position between
    /// two runs is the start of the later one, except at the very end.
    pub fn position(&self, pos: CharPos) -> RunPos {
        position_in(&self.runs, pos)
    }

    /// The document position of `at` (clamped).
    pub fn char_pos(&self, at: RunPos) -> CharPos {
        char_pos_in(&self.runs, at)
    }

    /// The run holding `key`, if any.
    pub fn index_of(&self, key: RunKey) -> Option<usize> {
        self.runs.binary_search_by_key(&key.0, |r| r.start.0).ok()
    }

    /// The whole text of the runs (what a screen reader reads as the
    /// document's value).
    pub fn text(&self) -> String {
        self.runs.iter().map(|r| r.text.as_str()).collect()
    }
}

/// Splits `text` (starting at document position `start`) into paragraphs at
/// `\n`. The last paragraph has no break unless `ends_with_break`... it is
/// simply whatever follows the last `\n`, and it is dropped when empty and
/// the text ended with a break.
pub fn paragraphs(start: CharPos, text: &str) -> Vec<Paragraph> {
    let mut out = Vec::new();
    let mut pos = start.0;
    let mut rest = text;
    loop {
        match rest.find('\n') {
            Some(i) => {
                let t = &rest[..i];
                let n = t.chars().count();
                out.push(Paragraph {
                    start: CharPos(pos),
                    text: t.to_owned(),
                    has_break: true,
                    heading: None,
                    list: None,
                    line_starts: None,
                });
                pos += n + 1;
                rest = &rest[i + 1..];
            }
            None => {
                if !rest.is_empty() || out.is_empty() {
                    out.push(Paragraph {
                        start: CharPos(pos),
                        text: rest.to_owned(),
                        has_break: false,
                        heading: None,
                        list: None,
                        line_starts: None,
                    });
                }
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(text: &str) -> RunSet {
        RunSet::build(&paragraphs(CharPos::ZERO, text), &[])
    }

    #[test]
    fn paragraphs_split_at_breaks() {
        let p = paragraphs(CharPos(10), "ab\n\ncd");
        assert_eq!(p.len(), 3);
        assert_eq!(
            (p[0].start, p[0].text.as_str(), p[0].has_break),
            (CharPos(10), "ab", true)
        );
        assert_eq!(
            (p[1].start, p[1].text.as_str(), p[1].has_break),
            (CharPos(13), "", true)
        );
        assert_eq!(
            (p[2].start, p[2].text.as_str(), p[2].has_break),
            (CharPos(14), "cd", false)
        );
        // A final break does not start an empty paragraph.
        assert_eq!(paragraphs(CharPos::ZERO, "ab\n").len(), 1);
        assert_eq!(paragraphs(CharPos::ZERO, "").len(), 1);
    }

    #[test]
    fn runs_cover_the_text_and_end_paragraphs_with_breaks() {
        let s = set("Hello world.\nSecond line.");
        assert_eq!(s.text(), "Hello world.\nSecond line.");
        assert_eq!(s.runs.len(), 2);
        assert!(s.runs[0].text.ends_with('\n'));
        assert!(!s.runs[0].continues_line);
        assert_eq!(s.runs[0].word_starts, vec![0, 6, 11]);
        assert_eq!(s.runs[1].start, CharPos(13));
        assert_eq!(s.range(), CharRange::new(0, 25));
    }

    #[test]
    fn long_paragraphs_split_at_words_under_the_limit() {
        let text = "word ".repeat(200);
        let s = set(&text);
        assert!(s.runs.len() >= 4);
        for r in &s.runs {
            assert!(r.len() <= MAX_RUN_CHARS, "{}", r.len());
        }
        // All but the last continue the line (one paragraph, not laid out).
        assert!(s.runs[..s.runs.len() - 1].iter().all(|r| r.continues_line));
        // Splits land on word starts.
        for r in &s.runs[1..] {
            assert!(r.text.starts_with("word"), "{:?}", &r.text[..8]);
        }
        assert_eq!(s.text(), text);
    }

    #[test]
    fn a_word_longer_than_a_run_is_cut_hard() {
        let text = "x".repeat(600);
        let s = set(&text);
        assert_eq!(s.runs.len(), 3);
        assert_eq!(s.text(), text);
    }

    #[test]
    fn graphemes_are_one_character() {
        // e + combining acute, a family emoji (ZWJ sequence), and CRLF-free text.
        let text = "e\u{301}x \u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}!";
        let s = set(text);
        let r = &s.runs[0];
        assert_eq!(r.len(), 5);
        assert_eq!(r.char_scalars, vec![2, 1, 1, 5, 1]);
        assert_eq!(r.len_chars(), text.chars().count());
        // Positions inside a cluster snap to its start.
        assert_eq!(s.position(CharPos(1)), RunPos { run: 0, index: 0 });
        assert_eq!(s.position(CharPos(2)), RunPos { run: 0, index: 1 });
        assert_eq!(s.char_pos(RunPos { run: 0, index: 4 }), CharPos(9));
    }

    #[test]
    fn positions_round_trip_on_cluster_starts() {
        let text = "Ünïcödé 🎉 text\nwith 𝄞 music\n\nand more.";
        let s = set(text);
        for c in 0..=text.chars().count() {
            let p = s.position(CharPos(c));
            assert_eq!(s.char_pos(p), CharPos(c), "at {c}");
        }
    }

    #[test]
    fn the_spoken_word_is_its_own_run_with_a_mark() {
        let text = "One two three.\nFour.";
        let marks = [(CharRange::new(4, 7), RunMark::SpokenWord)];
        let s = RunSet::build(&paragraphs(CharPos::ZERO, text), &marks);
        let texts: Vec<&str> = s.runs.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(texts, vec!["One ", "two", " three.\n", "Four."]);
        assert_eq!(s.runs[1].mark, Some(RunMark::SpokenWord));
        assert!(s.runs[0].continues_line && s.runs[1].continues_line);
        assert!(!s.runs[2].continues_line);
        // Runs outside the marked paragraph keep their keys.
        let plain = set(text);
        assert_eq!(plain.runs[1].key(), s.runs[3].key());
    }

    #[test]
    fn visual_lines_end_runs_and_lines() {
        let mut p = paragraphs(CharPos::ZERO, "aaa bbb ccc");
        p[0].line_starts = Some(vec![4, 8]);
        let s = RunSet::build(&p, &[]);
        let texts: Vec<&str> = s.runs.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(texts, vec!["aaa ", "bbb ", "ccc"]);
        assert!(s.runs.iter().all(|r| !r.continues_line));
    }

    #[test]
    fn empty_documents_and_blank_lines_have_runs() {
        let s = set("");
        assert_eq!(s.runs.len(), 1);
        assert!(s.runs[0].is_empty());
        let s = set("a\n\nb");
        let texts: Vec<&str> = s.runs.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(texts, vec!["a\n", "\n", "b"]);
        assert_eq!(s.position(CharPos(2)), RunPos { run: 1, index: 0 });
    }

    #[test]
    fn positions_between_runs_prefer_the_later_run() {
        let s = set("ab\ncd");
        assert_eq!(s.position(CharPos(3)), RunPos { run: 1, index: 0 });
        assert_eq!(s.position(CharPos(5)), RunPos { run: 1, index: 2 });
        assert_eq!(s.position(CharPos(50)), RunPos { run: 1, index: 2 });
    }

    #[test]
    fn a_full_run_puts_the_break_in_its_own_run() {
        let text = format!("{}\nb", "x".repeat(MAX_RUN_CHARS));
        let s = set(&text);
        assert!(s.runs.iter().all(|r| r.len() <= MAX_RUN_CHARS));
        assert_eq!(s.text(), text);
        let br = s.runs.iter().position(|r| r.text == "\n").unwrap();
        assert!(s.runs[br - 1].continues_line);
        assert!(!s.runs[br].continues_line);
    }

    #[test]
    fn index_of_finds_runs_by_key() {
        let s = set("ab\ncd\nef");
        assert_eq!(s.index_of(CharPos(3)), Some(1));
        assert_eq!(s.index_of(CharPos(4)), None);
    }

    /// A long one-line paragraph laid out in lines of about 60 chars, with
    /// accents and a combining mark, and a line start inside a cluster.
    fn long_paragraph() -> Paragraph {
        let text = "Caf\u{e9} cr\u{e8}me, e\u{301}t\u{e9} and words to read along. ".repeat(60);
        let mut p = paragraphs(CharPos(7), &text).remove(0);
        p.has_break = true;
        let n = p.len_chars();
        // Line starts every 61 chars, and one inside "e" and U+0301.
        let mut lines: Vec<usize> = (61..n).step_by(61).collect();
        lines.push(text[..text.find('\u{301}').unwrap()].chars().count());
        lines.sort_unstable();
        p.line_starts = Some(lines);
        p
    }

    #[test]
    fn rebuilding_the_touched_lines_matches_building_it_all() {
        let p = long_paragraph();
        let lines = p.line_starts.clone();
        let mut pr = ParaRuns::new(0, &p, lines.as_deref(), &[]);
        let mut old: Option<CharRange> = None;
        // Walk a mark across the paragraph a word at a time, and across
        // line ends, as the spoken word does.
        let n = p.len_chars();
        for (k, at) in (0..n).step_by(13).enumerate() {
            let len = 3 + k % 9;
            let r = CharRange::new(p.start.0 + at, (p.start.0 + at + len).min(p.start.0 + n));
            let marks = [(r, RunMark::SpokenWord)];
            for m in [old, Some(r)].into_iter().flatten() {
                let touched = pr.lines_touching(m);
                pr.rebuild(touched, &p.text, &marks);
            }
            old = Some(r);
            let fresh = RunSet::build(std::slice::from_ref(&p), &marks);
            let got: Vec<Run> = pr.runs().cloned().collect();
            assert_eq!(got, fresh.runs, "mark at {at}");
        }
        assert!(pr.lines().len() > 30);
        assert!(pr.lines().iter().all(|l| !l.is_empty()));
    }

    #[test]
    fn paragraph_positions_match_the_run_set() {
        let p = long_paragraph();
        let marks = [(CharRange::new(100, 104), RunMark::SpokenWord)];
        let pr = ParaRuns::new(0, &p, p.line_starts.as_deref(), &marks);
        let set = RunSet::build(std::slice::from_ref(&p), &marks);
        for c in p.start.0..=p.start.0 + p.span_chars() + 2 {
            let want = set.position(CharPos(c));
            let (run, index) = pr.position(CharPos(c)).unwrap();
            assert_eq!(run.start, set.runs[want.run].start, "at {c}");
            assert_eq!(index, want.index, "at {c}");
            assert_eq!(pr.char_pos(run.start, index), Some(set.char_pos(want)));
        }
        assert_eq!(pr.char_pos(CharPos(1), 0), None);
    }

    #[test]
    fn a_blank_paragraph_has_one_line() {
        let p = paragraphs(CharPos(3), "a\n\nb").remove(1);
        let pr = ParaRuns::new(1, &p, None, &[]);
        assert_eq!(pr.lines().len(), 1);
        assert_eq!(pr.lines()[0][0].text, "\n");
        assert_eq!(pr.lines_touching(CharRange::new(5, 5)), 0..1);
    }
}
