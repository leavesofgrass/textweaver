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
//! the paragraph it is in (ADR-0023, "Stable run ids").

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
    /// Where its visual lines start, as char offsets into `text` (sorted,
    /// without 0), when the paragraph has been laid out. `None`: not laid
    /// out; the paragraph is then one line for accessibility.
    pub line_starts: Option<Vec<usize>>,
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
    let mut word_starts = std::collections::HashSet::new();
    for (i, w) in text.split_word_bound_indices() {
        if w.chars().any(|c| !c.is_whitespace()) {
            word_starts.insert(i);
        }
    }
    let mut out = Vec::with_capacity(text.len());
    for (i, g) in text.grapheme_indices(true) {
        if g.len() <= MAX_CHAR_BYTES {
            out.push(Cluster {
                bytes: i..i + g.len(),
                scalars: g.chars().count(),
                word_start: word_starts.contains(&i),
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
                    word_start: first && word_starts.contains(&i),
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
            word_start: first && word_starts.contains(&i),
        });
    }
    out
}

/// A cut in a paragraph's clusters: where a run must end.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Cut {
    /// Cluster index where the next run starts.
    at: usize,
    /// True when a new line starts there.
    line: bool,
}

impl RunSet {
    /// Builds the runs for `paragraphs` with `marks` (document ranges; later
    /// entries win where they overlap).
    pub fn build(paragraphs: &[Paragraph], marks: &[(CharRange, RunMark)]) -> RunSet {
        let mut runs = Vec::new();
        for (pi, p) in paragraphs.iter().enumerate() {
            Self::build_paragraph(pi, p, marks, &mut runs);
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

    fn build_paragraph(
        pi: usize,
        p: &Paragraph,
        marks: &[(CharRange, RunMark)],
        runs: &mut Vec<Run>,
    ) {
        let cl = clusters(&p.text);
        // Char offset (within the paragraph) where each cluster starts.
        let mut starts = Vec::with_capacity(cl.len() + 1);
        let mut acc = 0usize;
        for c in &cl {
            starts.push(acc);
            acc += c.scalars;
        }
        starts.push(acc);
        let cluster_at = |char_off: usize| -> usize {
            // The first cluster starting at or after `char_off`.
            starts.partition_point(|&s| s < char_off).min(cl.len())
        };

        // Cuts: visual lines, then marks.
        let mut cuts: Vec<Cut> = Vec::new();
        if let Some(lines) = &p.line_starts {
            for &l in lines {
                let at = cluster_at(l);
                if at > 0 && at < cl.len() {
                    cuts.push(Cut { at, line: true });
                }
            }
        }
        let p_end = p.start.0 + acc;
        let mut mark_of = vec![None; cl.len()];
        for (range, mark) in marks {
            if range.end.0 <= p.start.0 || range.start.0 >= p_end || range.is_empty() {
                continue;
            }
            let a = cluster_at(range.start.0.saturating_sub(p.start.0));
            let b = cluster_at(range.end.0.min(p_end) - p.start.0);
            if a >= b {
                continue;
            }
            for m in &mut mark_of[a..b] {
                *m = Some(*mark);
            }
        }
        for i in 1..cl.len() {
            if mark_of[i] != mark_of[i - 1] {
                cuts.push(Cut { at: i, line: false });
            }
        }
        cuts.sort();
        cuts.dedup_by(|b, a| {
            if a.at == b.at {
                a.line |= b.line;
                true
            } else {
                false
            }
        });

        if cl.is_empty() {
            // A blank line: one run, which holds just the break (if any).
            runs.push(Self::make_run(pi, p, &cl, &starts, 0..0, false, None));
        }
        // Walk the segments between cuts, splitting long ones at word
        // starts (else hard) so no run exceeds MAX_RUN_CHARS.
        let mut seg_start = 0usize;
        let ends = cuts
            .iter()
            .map(|c| (c.at, c.line))
            .chain([(cl.len(), true)]);
        for (seg_end, next_is_line) in ends {
            let mut a = seg_start;
            while a < seg_end {
                let mut b = seg_end.min(a + MAX_RUN_CHARS);
                if b < seg_end {
                    // Prefer to end at a word start in the back half.
                    if let Some(w) = (a + MAX_RUN_CHARS / 2..b).rev().find(|&i| cl[i].word_start) {
                        b = w;
                    }
                }
                let continues_line = !(b >= seg_end && next_is_line);
                let mark = mark_of[a];
                runs.push(Self::make_run(
                    pi,
                    p,
                    &cl,
                    &starts,
                    a..b,
                    continues_line,
                    mark,
                ));
                a = b;
            }
            seg_start = seg_end;
        }
        // The paragraph's break goes on its last run, which ends the line.
        if p.has_break
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
                let start = CharPos(p.start.0 + acc);
                runs.push(Run {
                    start,
                    paragraph: pi,
                    text: "\n".into(),
                    char_lens: vec![1],
                    char_scalars: vec![1],
                    word_starts: Vec::new(),
                    continues_line: false,
                    mark: None,
                    heading: p.heading,
                });
                let n = runs.len();
                runs[n - 2].continues_line = true;
            }
        }
    }

    fn make_run(
        pi: usize,
        p: &Paragraph,
        cl: &[Cluster],
        starts: &[usize],
        range: Range<usize>,
        continues_line: bool,
        mark: Option<RunMark>,
    ) -> Run {
        let slice = &cl[range.clone()];
        let bytes = match (slice.first(), slice.last()) {
            (Some(f), Some(l)) => f.bytes.start..l.bytes.end,
            _ => 0..0,
        };
        Run {
            start: CharPos(p.start.0 + starts[range.start]),
            paragraph: pi,
            text: p.text[bytes].to_owned(),
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
            heading: p.heading,
        }
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
        let n = self.runs.len();
        if n == 0 {
            return RunPos { run: 0, index: 0 };
        }
        let i = self
            .runs
            .partition_point(|r| r.start.0 <= pos.0)
            .saturating_sub(1);
        let run = &self.runs[i];
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

    /// The document position of `at` (clamped).
    pub fn char_pos(&self, at: RunPos) -> CharPos {
        let Some(run) = self.runs.get(at.run) else {
            return self.range().end;
        };
        let index = at.index.min(run.len());
        let off: usize = run.char_scalars[..index]
            .iter()
            .map(|&s| usize::from(s))
            .sum();
        CharPos(run.start.0 + off)
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
}
