//! Mapping positions between two versions of a text by aligning their
//! words.
//!
//! Edit mode shows a document's source (Markdown with its markup), while
//! reading uses the canonical text (ADR-0002). Bookmarks, notes, the cursor,
//! and history are canonical positions; entering edit mode maps them into
//! the source, and leaving maps them back into the rebuilt canonical text.
//! Both directions go through [`Aligner`]: the two texts' words are aligned
//! (markup such as `#`, `**`, and link targets only adds or drops words), and
//! a position is carried over by the word it is in or before.

use ropey::Rope;
use textweaver_core::CharPos;

/// One word: a hash of its lowercase text, and its char range.
///
/// Words are compared by a 64-bit hash of their lowercase letters rather
/// than by an owned lowercase `String` each: a 5 MB document has about a
/// million words, and allocating a string for every word of both texts
/// made entering and leaving edit mode take up to two seconds
/// (docs/audit-2026-09.md, finding P3). A collision can only mis-pair two
/// different words, which moves a carried position by a word at worst.
#[derive(Clone, Copy, Debug)]
struct Word {
    text: u64,
    start: usize,
    end: usize,
}

/// FNV-1a over the chars of a word, lowercased.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn hash_char(h: u64, c: char) -> u64 {
    let mut h = h;
    for l in c.to_lowercase() {
        h ^= u64::from(u32::from(l));
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

fn words(text: &Rope) -> Vec<Word> {
    // About one word in six chars in prose.
    let mut out = Vec::with_capacity(text.len_chars() / 6);
    let mut current: Option<Word> = None;
    let mut i = 0usize;
    for chunk in text.chunks() {
        for c in chunk.chars() {
            if c.is_alphanumeric() {
                match current.as_mut() {
                    Some(w) => {
                        w.text = hash_char(w.text, c);
                        w.end = i + 1;
                    }
                    None => {
                        current = Some(Word {
                            text: hash_char(FNV_OFFSET, c),
                            start: i,
                            end: i + 1,
                        });
                    }
                }
            } else if let Some(w) = current.take() {
                out.push(w);
            }
            i += 1;
        }
    }
    out.extend(current);
    out
}

/// How far ahead a word is looked for in the other text.
const LOOKAHEAD: usize = 64;

/// A word alignment between two texts, for carrying positions across.
#[derive(Clone, Debug)]
pub struct Aligner {
    identical: bool,
    from_len: usize,
    to_len: usize,
    from: Vec<Word>,
    to: Vec<Word>,
    /// For each word of `from`, the matching word of `to`.
    pairs: Vec<Option<usize>>,
}

impl Aligner {
    /// Aligns `from` with `to`.
    pub fn new(from: &Rope, to: &Rope) -> Self {
        let from_len = from.len_chars();
        let to_len = to.len_chars();
        if from == to {
            return Aligner {
                identical: true,
                from_len,
                to_len,
                from: Vec::new(),
                to: Vec::new(),
                pairs: Vec::new(),
            };
        }
        let a = words(from);
        let b = words(to);
        let mut pairs = vec![None; a.len()];
        let mut j = 0;
        for i in 0..a.len() {
            let limit = (j + LOOKAHEAD).min(b.len());
            // A match further ahead must be confirmed by the next word, so a
            // common word ("the") does not pull the alignment off course.
            let found = (j..limit).find(|&k| {
                b[k].text == a[i].text
                    && (k == j
                        || match (a.get(i + 1), b.get(k + 1)) {
                            (Some(x), Some(y)) => x.text == y.text,
                            (None, None) => true,
                            _ => false,
                        })
            });
            if let Some(k) = found {
                pairs[i] = Some(k);
                j = k + 1;
            }
        }
        Aligner {
            identical: false,
            from_len,
            to_len,
            from: a,
            to: b,
            pairs,
        }
    }

    /// The position in `to` corresponding to `pos` in `from`: the same place
    /// in the same word; else the start of the word after the last aligned
    /// word before it; else the same fraction of the text.
    pub fn map(&self, pos: CharPos) -> CharPos {
        if self.identical {
            return pos.clamp_to(self.to_len);
        }
        let p = pos.0.min(self.from_len);
        // The word containing `p`, else the first word after it.
        let i = self.from.partition_point(|w| w.end <= p);
        if let Some(w) = self.from.get(i)
            && let Some(k) = self.pairs[i]
        {
            let t = &self.to[k];
            let offset = p.saturating_sub(w.start).min(t.end - t.start);
            // Before the word (in the space ahead of it): the word's start.
            return CharPos(t.start + offset);
        }
        // The last aligned word before `p`.
        let prev = (0..i.min(self.pairs.len()))
            .rev()
            .find_map(|x| self.pairs[x]);
        match prev {
            Some(k) => match self.to.get(k + 1) {
                Some(next) => CharPos(next.start),
                None => CharPos(self.to_len),
            },
            None => {
                let frac = p.saturating_mul(self.to_len) / self.from_len.max(1);
                CharPos(frac.min(self.to_len))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(from: &str, to: &str, pos: usize) -> usize {
        Aligner::new(&Rope::from_str(from), &Rope::from_str(to))
            .map(CharPos(pos))
            .0
    }

    #[test]
    fn identical_texts_map_to_themselves() {
        assert_eq!(map("abc def", "abc def", 4), 4);
        assert_eq!(map("abc", "abc", 10), 3);
    }

    #[test]
    fn markup_is_skipped() {
        let src = "# Title\n\nSome **bold** words.";
        let canon = "Title\n\nSome bold words.";
        // "bold" in the source (at 16) is "bold" in the canonical text (12);
        // the markup in front of it maps to the word too.
        assert_eq!(map(src, canon, 16), 12);
        assert_eq!(map(src, canon, 14), 12);
        assert_eq!(map(canon, src, 12), 16);
        // "Title"
        assert_eq!(map(src, canon, 2), 0);
        assert_eq!(map(canon, src, 0), 2);
        // Inside a word keeps the offset.
        assert_eq!(map(canon, src, 14), 18);
    }

    #[test]
    fn link_targets_and_unmatched_words_fall_forward() {
        let src = "See [the site](https://example.com) now.";
        let canon = "See the site now.";
        // "https" has no counterpart: the next canonical word after "site".
        let https = src.find("https").unwrap();
        assert_eq!(map(src, canon, https), canon.find("now").unwrap());
        assert_eq!(
            map(canon, src, canon.find("now").unwrap()),
            src.find("now").unwrap()
        );
    }

    #[test]
    fn inserted_words_do_not_derail_later_matches() {
        let before = "one two three four five";
        let after = "one two NEW WORDS HERE three four five";
        assert_eq!(map(before, after, 8), after.find("three").unwrap());
        assert_eq!(map(before, after, 19), after.find("five").unwrap());
    }

    #[test]
    fn words_match_whatever_their_case() {
        // Hashing lowercase letters keeps the case-insensitive match.
        let src = "ÉCOLE Straße and MORE words";
        let canon = "école straße and more words";
        assert_eq!(map(src, canon, src.chars().count() - 5), 22);
        assert_eq!(map(canon, src, 6), 6);
        let w = words(&Rope::from_str("Ab ab AB aB x"));
        assert!(w[..4].iter().all(|x| x.text == w[0].text));
        assert_ne!(w[0].text, w[4].text);
        assert_eq!((w[4].start, w[4].end), (12, 13));
    }

    #[test]
    fn end_and_empty() {
        assert_eq!(map("a b", "x y z", 3), 5);
        assert_eq!(map("", "abc", 0), 0);
    }
}
