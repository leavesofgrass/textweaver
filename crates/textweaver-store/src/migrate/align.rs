//! Mapping star positions onto textweaver text by word alignment
//! (ADR-0002).
//!
//! star saved positions as char offsets into its own `plain_text`, whose
//! shape differs from textweaver's canonical text (single newlines joined,
//! list items and table cells run together, tables narrated). A saved
//! offset is mapped in three steps:
//!
//! 1. the offset becomes star's word index: the first word starting at or
//!    after it, else the last word (star's own restore rule);
//! 2. the word sequences of both texts are aligned (common prefix and
//!    suffix, unique-word anchors, then a longest common subsequence in the
//!    gaps), and the star word maps to the textweaver word aligned with it,
//!    which has the same text; an unaligned word maps to the next aligned
//!    one;
//! 3. with nothing aligned, the saved percentage is used.
//!
//! Both texts are tokenized with star's word rule, `\b\w[\w'-]*`, so the
//! sequences differ only where the texts do.

use std::collections::HashMap;

use textweaver_core::{CharPos, CharRange};

/// star's word characters: Unicode letters and digits, and `_` (Python's
/// `\w`).
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// A word: its first char's index and its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    /// Char offset of the first char.
    pub start: usize,
    /// Char offset one past the last char.
    pub end: usize,
    /// The word.
    pub text: String,
}

/// star's words (`\b\w[\w'-]*`): a word starts at a word character that
/// does not follow one, and runs over word characters, `'`, and `-`.
pub fn star_words(text: &str) -> Vec<Word> {
    let mut words = Vec::new();
    let mut prev_word = false;
    let mut current: Option<Word> = None;
    for (i, c) in text.chars().enumerate() {
        match current.as_mut() {
            Some(w) if is_word(c) || c == '\'' || c == '-' => {
                w.text.push(c);
                w.end = i + 1;
            }
            _ => {
                if let Some(w) = current.take() {
                    words.push(w);
                }
                if is_word(c) && !prev_word {
                    current = Some(Word {
                        start: i,
                        end: i + 1,
                        text: c.to_string(),
                    });
                }
            }
        }
        prev_word = is_word(c);
    }
    words.extend(current);
    words
}

/// Cells above which a gap is not aligned by dynamic programming.
const LCS_MAX_CELLS: usize = 400_000;

/// Recursion limit for anchor splitting.
const MAX_DEPTH: usize = 48;

/// Aligns `a` with `b`: for each index of `a`, the index of the equal
/// element of `b` it is matched with. Matches keep their order (a common
/// subsequence), found by trimming the common prefix and suffix, anchoring
/// on words unique to both sides, and running a longest common
/// subsequence on small gaps.
pub fn align(a: &[&str], b: &[&str]) -> Vec<Option<usize>> {
    let mut out = vec![None; a.len()];
    align_range(a, b, 0, a.len(), 0, b.len(), &mut out, 0);
    out
}

#[allow(clippy::too_many_arguments)]
fn align_range(
    a: &[&str],
    b: &[&str],
    mut a0: usize,
    mut a1: usize,
    mut b0: usize,
    mut b1: usize,
    out: &mut [Option<usize>],
    depth: usize,
) {
    while a0 < a1 && b0 < b1 && a[a0] == b[b0] {
        out[a0] = Some(b0);
        a0 += 1;
        b0 += 1;
    }
    while a0 < a1 && b0 < b1 && a[a1 - 1] == b[b1 - 1] {
        out[a1 - 1] = Some(b1 - 1);
        a1 -= 1;
        b1 -= 1;
    }
    if a0 == a1 || b0 == b1 {
        return;
    }
    let (n, m) = (a1 - a0, b1 - b0);
    if (n + 1).saturating_mul(m + 1) <= LCS_MAX_CELLS {
        lcs(a, b, a0, a1, b0, b1, out);
        return;
    }
    if depth >= MAX_DEPTH {
        return;
    }
    // Words occurring exactly once on each side anchor the alignment
    // (patience diff); the longest increasing run of their positions keeps
    // the anchors in order.
    let mut counts: HashMap<&str, (usize, usize, usize, usize)> = HashMap::new();
    for (i, w) in a.iter().enumerate().take(a1).skip(a0) {
        let e = counts.entry(w).or_insert((0, 0, 0, 0));
        e.0 += 1;
        e.1 = i;
    }
    for (j, w) in b.iter().enumerate().take(b1).skip(b0) {
        if let Some(e) = counts.get_mut(w) {
            e.2 += 1;
            e.3 = j;
        }
    }
    let mut pairs: Vec<(usize, usize)> = counts
        .values()
        .filter(|(ca, _, cb, _)| *ca == 1 && *cb == 1)
        .map(|(_, i, _, j)| (*i, *j))
        .collect();
    pairs.sort_unstable();
    let anchors = longest_increasing(&pairs);
    if anchors.is_empty() {
        return;
    }
    let (mut pa, mut pb) = (a0, b0);
    for (i, j) in anchors {
        align_range(a, b, pa, i, pb, j, out, depth + 1);
        out[i] = Some(j);
        pa = i + 1;
        pb = j + 1;
    }
    align_range(a, b, pa, a1, pb, b1, out, depth + 1);
}

/// The longest subsequence of `pairs` (sorted by `.0`) whose `.1` values
/// increase.
fn longest_increasing(pairs: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut tails: Vec<usize> = Vec::new(); // index into pairs
    let mut prev: Vec<Option<usize>> = vec![None; pairs.len()];
    for (k, &(_, j)) in pairs.iter().enumerate() {
        let pos = tails.partition_point(|&t| pairs[t].1 < j);
        if pos > 0 {
            prev[k] = Some(tails[pos - 1]);
        }
        if pos == tails.len() {
            tails.push(k);
        } else {
            tails[pos] = k;
        }
    }
    let mut out = Vec::new();
    let mut cur = tails.last().copied();
    while let Some(k) = cur {
        out.push(pairs[k]);
        cur = prev[k];
    }
    out.reverse();
    out
}

fn lcs(
    a: &[&str],
    b: &[&str],
    a0: usize,
    a1: usize,
    b0: usize,
    b1: usize,
    out: &mut [Option<usize>],
) {
    let (n, m) = (a1 - a0, b1 - b0);
    let w = m + 1;
    // table[i][j]: LCS length of a[a0+i..a1] and b[b0+j..b1].
    let mut table = vec![0u32; (n + 1) * w];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            table[i * w + j] = if a[a0 + i] == b[b0 + j] {
                table[(i + 1) * w + j + 1] + 1
            } else {
                table[(i + 1) * w + j].max(table[i * w + j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if a[a0 + i] == b[b0 + j] {
            out[a0 + i] = Some(b0 + j);
            i += 1;
            j += 1;
        } else if table[(i + 1) * w + j] >= table[i * w + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
}

/// How a position was mapped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapMethod {
    /// The saved word was aligned with the same word in textweaver's text.
    Aligned,
    /// The saved word has no counterpart (star-only narration, for
    /// example); the next aligned word was used.
    Nearby,
    /// A note's quoted text was found in the document.
    Anchor,
    /// Nothing aligned; the saved percentage was used.
    Percentage,
}

impl MapMethod {
    /// A few words for the migration report.
    pub fn describe(self) -> &'static str {
        match self {
            MapMethod::Aligned => "matched word for word",
            MapMethod::Nearby => "placed at the nearest matching word",
            MapMethod::Anchor => "placed by its quoted text",
            MapMethod::Percentage => "placed by percentage",
        }
    }
}

/// A mapped position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mapped {
    /// The position in textweaver's text.
    pub pos: CharPos,
    /// How it was found.
    pub method: MapMethod,
}

/// Maps positions in one star `plain_text` onto one textweaver text.
#[derive(Clone, Debug)]
pub struct PositionMapper {
    tw_len: usize,
    tw_words: Vec<Word>,
    star_words: Vec<Word>,
    star_len: usize,
    to_tw: Vec<Option<usize>>,
    exact: bool,
    /// The textweaver text with whitespace collapsed, and each collapsed
    /// char's index in the original text (for anchor search).
    collapsed: Vec<char>,
    collapsed_origin: Vec<usize>,
}

impl PositionMapper {
    /// A mapper from `star_text` (star's `plain_text`, from its parse
    /// cache) to `tw_text`. Without star's text, `tw_text` with single
    /// newlines as spaces stands in for it (the documented difference
    /// between the two); positions are then checked against the saved
    /// percentage.
    pub fn new(tw_text: &str, star_text: Option<&str>) -> Self {
        let tw_words = star_words(tw_text);
        let (star_words_v, star_len, exact) = match star_text {
            Some(s) => (star_words(s), s.chars().count(), true),
            None => (tw_words.clone(), tw_text.chars().count(), false),
        };
        let a: Vec<&str> = star_words_v.iter().map(|w| w.text.as_str()).collect();
        let b: Vec<&str> = tw_words.iter().map(|w| w.text.as_str()).collect();
        let to_tw = if exact {
            align(&a, &b)
        } else {
            (0..a.len()).map(Some).collect()
        };
        let mut collapsed = Vec::new();
        let mut collapsed_origin = Vec::new();
        let mut in_space = true;
        for (i, c) in tw_text.chars().enumerate() {
            if c.is_whitespace() {
                if !in_space {
                    collapsed.push(' ');
                    collapsed_origin.push(i);
                }
                in_space = true;
            } else {
                collapsed.extend(c.to_lowercase());
                for _ in c.to_lowercase() {
                    collapsed_origin.push(i);
                }
                in_space = false;
            }
        }
        PositionMapper {
            tw_len: tw_text.chars().count(),
            tw_words,
            star_words: star_words_v,
            star_len,
            to_tw,
            exact,
            collapsed,
            collapsed_origin,
        }
    }

    /// True when star's own text was available, so alignment is exact.
    pub fn exact(&self) -> bool {
        self.exact
    }

    /// Share of star's words aligned with textweaver words, 0 to 1.
    pub fn aligned_share(&self) -> f64 {
        if self.to_tw.is_empty() {
            return 0.0;
        }
        self.to_tw.iter().filter(|m| m.is_some()).count() as f64 / self.to_tw.len() as f64
    }

    /// Length of the textweaver text in chars.
    pub fn tw_len(&self) -> usize {
        self.tw_len
    }

    /// The first word at or after `pct` percent of the textweaver text.
    pub fn map_percentage(&self, pct: u8) -> Mapped {
        let target = self.tw_len * usize::from(pct.min(100)) / 100;
        let pos = self
            .tw_words
            .iter()
            .find(|w| w.start >= target)
            .or(self.tw_words.last())
            .map_or(target, |w| w.start);
        Mapped {
            pos: CharPos(pos.min(self.tw_len)),
            method: MapMethod::Percentage,
        }
    }

    /// Maps star word `index`.
    pub fn map_word(&self, index: usize, pct: Option<u8>) -> Mapped {
        if self.star_words.is_empty() || self.tw_words.is_empty() {
            return self.map_percentage(pct.unwrap_or(0));
        }
        let index = index.min(self.star_words.len() - 1);
        if let Some(j) = self.to_tw[index] {
            return Mapped {
                pos: CharPos(self.tw_words[j].start),
                method: MapMethod::Aligned,
            };
        }
        let next = self.to_tw[index..].iter().flatten().next();
        let prev = self.to_tw[..index].iter().rev().flatten().next();
        match next.or(prev) {
            Some(&j) => Mapped {
                pos: CharPos(self.tw_words[j].start),
                method: MapMethod::Nearby,
            },
            None => self.map_percentage(pct.unwrap_or_else(|| {
                crate::percent(CharPos(self.star_words[index].start), self.star_len)
            })),
        }
    }

    /// star's word index for a saved char offset: the first word starting
    /// at or after it, else the last word.
    pub fn star_word_at(&self, offset: usize) -> usize {
        let i = self.star_words.partition_point(|w| w.start < offset);
        i.min(self.star_words.len().saturating_sub(1))
    }

    /// Maps a saved star char offset (with its saved percentage, when
    /// known). Without star's own text, a result more than ten points away
    /// from the saved percentage is replaced by the percentage.
    pub fn map_offset(&self, offset: usize, pct: Option<u8>) -> Mapped {
        let m = self.map_word(self.star_word_at(offset), pct);
        if !self.exact
            && let Some(p) = pct
            && crate::percent(m.pos, self.tw_len).abs_diff(p) > 10
        {
            return self.map_percentage(p);
        }
        m
    }

    /// Maps the end of a saved star range: the end of the textweaver word
    /// aligned with (or nearest to) the last star word starting before
    /// `offset`.
    pub fn map_end(&self, offset: usize, pct: Option<u8>) -> CharPos {
        let i = self.star_words.partition_point(|w| w.start < offset);
        let m = self.map_word(i.saturating_sub(1), pct);
        match self.tw_words.iter().find(|w| w.start == m.pos.0) {
            Some(w) => CharPos(w.end),
            None => m.pos,
        }
    }

    /// Finds a note's quoted text (whitespace and case ignored) in the
    /// textweaver text, taking the occurrence nearest `near`.
    pub fn find_anchor(&self, anchor: &str, near: CharPos) -> Option<CharRange> {
        let needle: Vec<char> = anchor
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
            .chars()
            .collect();
        if needle.len() < 3 || needle.len() > self.collapsed.len() {
            return None;
        }
        let mut best: Option<(usize, CharRange)> = None;
        for start in 0..=self.collapsed.len() - needle.len() {
            if self.collapsed[start..start + needle.len()] != needle[..] {
                continue;
            }
            let s = self.collapsed_origin[start];
            let e = self.collapsed_origin[start + needle.len() - 1] + 1;
            let dist = s.abs_diff(near.0);
            if best.as_ref().is_none_or(|(d, _)| dist < *d) {
                best = Some((dist, CharRange::new(s, e)));
            }
        }
        best.map(|(_, r)| r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_tokenizer_matches_the_parity_table() {
        let words: Vec<String> = star_words(
            "O'Brien's well-known 12-14 3rd e.g. 2.0.1 reader@example.org $12.50 €4 rock- café",
        )
        .into_iter()
        .map(|w| w.text)
        .collect();
        assert_eq!(
            words,
            vec![
                "O'Brien's",
                "well-known",
                "12-14",
                "3rd",
                "e",
                "g",
                "2",
                "0",
                "1",
                "reader",
                "example",
                "org",
                "12",
                "50",
                "4",
                "rock-",
                "café"
            ]
        );
        let w = star_words("  ab cd");
        assert_eq!((w[1].start, w[1].end), (5, 7));
    }

    /// The tokenizer reproduces star's own word tokens for every parity
    /// fixture (`fixtures/star-parity/*.json`, exported from star 0.1.31).
    #[test]
    fn star_tokenizer_reproduces_star_word_tokens() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/star-parity");
        let mut checked = 0;
        for name in ["sample.md.json", "sample.txt.json", "sample.html.json"] {
            let Ok(text) = std::fs::read_to_string(dir.join(name)) else {
                continue;
            };
            let v: serde_json::Value = serde_json::from_str(&text).unwrap();
            let plain = v["plain_text"].as_str().unwrap();
            let ours: Vec<(usize, usize, String)> = star_words(plain)
                .into_iter()
                .map(|w| (w.start, w.end, w.text))
                .collect();
            let theirs: Vec<(usize, usize, String)> = v["word_tokens"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| {
                    (
                        t[0].as_u64().unwrap() as usize,
                        t[1].as_u64().unwrap() as usize,
                        t[2].as_str().unwrap().to_owned(),
                    )
                })
                .collect();
            assert_eq!(ours, theirs, "{name}");
            checked += 1;
        }
        assert_eq!(checked, 3, "parity fixtures present");
    }

    #[test]
    fn alignment_keeps_order_and_skips_insertions() {
        let a = [
            "the", "cat", "Table", "with", "two", "columns", "sat", "down",
        ];
        let b = ["the", "cat", "sat", "on", "down"];
        let m = align(&a, &b);
        assert_eq!(
            m,
            vec![Some(0), Some(1), None, None, None, None, Some(2), Some(4)]
        );
    }

    #[test]
    fn large_inputs_align_through_anchors() {
        let a: Vec<String> = (0..3000).map(|i| format!("w{i}")).collect();
        let mut b: Vec<String> = a.clone();
        b.insert(1500, "extra".into());
        b.remove(10);
        let ar: Vec<&str> = a.iter().map(String::as_str).collect();
        let br: Vec<&str> = b.iter().map(String::as_str).collect();
        let m = align(&ar, &br);
        assert_eq!(m[10], None);
        assert_eq!(m[11], Some(10));
        assert_eq!(m[2000], Some(2000));
        assert_eq!(m.iter().filter(|x| x.is_some()).count(), 2999);
    }

    #[test]
    fn offsets_map_through_star_text() {
        // star joined lines and narrated the table; textweaver keeps lines.
        let star = "Intro text. Table with 2 columns. Name Age. Ann 30. End here.";
        let tw = "Intro text.\n\nName\tAge\nAnn\t30\n\nEnd here.";
        let m = PositionMapper::new(tw, Some(star));
        assert!(m.exact());
        // "End" in star's text.
        let off = star.find("End").unwrap();
        let mapped = m.map_offset(off, Some(90));
        assert_eq!(mapped.method, MapMethod::Aligned);
        assert_eq!(&tw[mapped.pos.0..mapped.pos.0 + 3], "End");
        // "Table" (narration) has no counterpart: next aligned word, "Name".
        let off = star.find("Table").unwrap();
        let mapped = m.map_offset(off, None);
        assert_eq!(mapped.method, MapMethod::Nearby);
        assert_eq!(&tw[mapped.pos.0..mapped.pos.0 + 4], "Name");
        // An offset between words goes to the next word; past the end, the
        // last word.
        assert_eq!(m.map_offset(off - 1, None).pos, mapped.pos);
        let last = m.map_offset(10_000, None);
        assert_eq!(&tw[last.pos.0..last.pos.0 + 4], "here");
    }

    #[test]
    fn approximate_text_falls_back_to_percentage() {
        let tw = "one two three four five six seven eight nine ten";
        let m = PositionMapper::new(tw, None);
        assert!(!m.exact());
        let near = m.map_offset(8, Some(16));
        assert_eq!(near.method, MapMethod::Aligned);
        assert_eq!(near.pos, CharPos(8));
        let far = m.map_offset(8, Some(90));
        assert_eq!(far.method, MapMethod::Percentage);
        assert_eq!(&tw[far.pos.0..far.pos.0 + 3], "ten");
        let empty = PositionMapper::new("", Some(""));
        assert_eq!(empty.map_offset(5, Some(50)).pos, CharPos(0));
    }

    #[test]
    fn anchors_are_found_nearest_the_estimate() {
        let tw = "A claim here.\nMore text.\nA   claim\nhere again.";
        let m = PositionMapper::new(tw, None);
        let r = m.find_anchor("a claim here", CharPos(30)).unwrap();
        assert_eq!(r.start, CharPos(25));
        assert_eq!(&tw[r.start.0..r.end.0], "A   claim\nhere");
        let r = m.find_anchor("A CLAIM here", CharPos(0)).unwrap();
        assert_eq!(r, CharRange::new(0, 12));
        assert!(m.find_anchor("absent words", CharPos(0)).is_none());
        assert!(
            m.find_anchor("a", CharPos(0)).is_none(),
            "too short to trust"
        );
    }
}
