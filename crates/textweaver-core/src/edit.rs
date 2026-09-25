//! Text edits and how positions move across them.
//!
//! An [`Edit`] replaces a char range with new text. Applying one yields an
//! [`EditOutcome`], which maps any pre-edit position or range to its post-edit
//! counterpart, and an inverse edit for undo. The document side
//! (`Document::apply`) and the editor's undo stack both go through here, so
//! markers, bookmarks, notes, and the cursor all shift the same way.

use ropey::Rope;
use serde::{Deserialize, Serialize};

use crate::{Bias, CharPos, CharRange, CoreError};

/// Replace `range` with `text`. Insertions have an empty range; deletions
/// have empty text.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Edit {
    /// The chars being replaced, in pre-edit coordinates.
    pub range: CharRange,
    /// The replacement text.
    pub text: String,
}

impl Edit {
    /// Insert `text` at `at`.
    pub fn insert(at: impl Into<CharPos>, text: impl Into<String>) -> Self {
        Edit {
            range: CharRange::empty(at),
            text: text.into(),
        }
    }

    /// Delete `range`.
    pub fn delete(range: impl Into<CharRange>) -> Self {
        Edit {
            range: range.into(),
            text: String::new(),
        }
    }

    /// Replace `range` with `text`.
    pub fn replace(range: impl Into<CharRange>, text: impl Into<String>) -> Self {
        Edit {
            range: range.into(),
            text: text.into(),
        }
    }

    /// True when the edit changes nothing.
    pub fn is_noop(&self) -> bool {
        self.range.is_empty() && self.text.is_empty()
    }

    /// Number of chars inserted.
    pub fn inserted_chars(&self) -> usize {
        self.text.chars().count()
    }

    /// The outcome of applying this edit, without applying it.
    pub fn outcome(&self) -> EditOutcome {
        let start = self.range.start;
        EditOutcome {
            removed: self.range,
            inserted: CharRange::new(start, start.saturating_add(self.inserted_chars())),
        }
    }

    fn check(&self, len: usize) -> Result<(), CoreError> {
        if self.range.end.0 > len {
            return Err(CoreError::InvalidRange {
                start: self.range.start,
                end: self.range.end,
                len,
            });
        }
        Ok(())
    }

    /// Applies the edit to a rope. Returns the outcome and the inverse edit.
    pub fn apply_to_rope(&self, rope: &mut Rope) -> Result<(EditOutcome, Edit), CoreError> {
        self.check(rope.len_chars())?;
        let removed: String = rope.slice(self.range.to_range()).to_string();
        rope.remove(self.range.to_range());
        rope.insert(self.range.start.0, &self.text);
        let outcome = self.outcome();
        Ok((
            outcome,
            Edit {
                range: outcome.inserted,
                text: removed,
            },
        ))
    }

    /// Applies the edit to a string. Returns the outcome and the inverse edit.
    pub fn apply_to_string(&self, s: &mut String) -> Result<(EditOutcome, Edit), CoreError> {
        let len = s.chars().count();
        self.check(len)?;
        let byte = |c: usize| s.char_indices().nth(c).map_or(s.len(), |(i, _)| i);
        let (a, b) = (byte(self.range.start.0), byte(self.range.end.0));
        let removed = s[a..b].to_owned();
        s.replace_range(a..b, &self.text);
        let outcome = self.outcome();
        Ok((
            outcome,
            Edit {
                range: outcome.inserted,
                text: removed,
            },
        ))
    }
}

/// What an applied edit did, in a form that can move other positions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditOutcome {
    /// The removed range, in pre-edit coordinates.
    pub removed: CharRange,
    /// The inserted text's range, in post-edit coordinates.
    pub inserted: CharRange,
}

impl EditOutcome {
    /// Change in document length, in chars.
    pub fn delta(&self) -> isize {
        // Lengths of in-memory text always fit in isize.
        self.inserted.len() as isize - self.removed.len() as isize
    }

    /// Maps a pre-edit position to its post-edit position.
    ///
    /// Positions before the edit stay; positions after it shift by
    /// [`delta`](Self::delta). Positions inside the removed range, or exactly
    /// at a pure insertion point, go to the start of the inserted text with
    /// [`Bias::Before`] and to its end with [`Bias::After`].
    pub fn map_pos(&self, pos: CharPos, bias: Bias) -> CharPos {
        let r = self.removed;
        let touches = if r.is_empty() {
            pos == r.start
        } else {
            r.start <= pos && pos < r.end
        };
        if pos < r.start {
            pos
        } else if touches {
            match bias {
                Bias::Before => self.inserted.start,
                Bias::After => self.inserted.end,
            }
        } else {
            CharPos(pos.0 - r.len() + self.inserted.len())
        }
    }

    /// Maps a pre-edit range. Ranges do not grow at their edges: text
    /// inserted exactly at the start or end stays outside. A range entirely
    /// removed collapses to an empty range at the insertion point.
    pub fn map_range(&self, range: CharRange) -> CharRange {
        if range.is_empty() {
            let p = self.map_pos(range.start, Bias::Before);
            return CharRange::empty(p);
        }
        let start = self.map_pos(range.start, Bias::After);
        let end_before = self.map_pos(range.end, Bias::Before);
        // An edit that starts inside the range and runs past its end pulls the
        // end back to the inserted text's start.
        let end = if self.removed.start < range.end && range.end <= self.removed.end {
            self.inserted.start
        } else {
            end_before
        };
        CharRange::new(start.min(end), end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_and_invert_on_rope() {
        let mut rope = Rope::from_str("hello world");
        let edit = Edit::replace(6..11, "wörld!");
        let (out, inverse) = edit.apply_to_rope(&mut rope).unwrap();
        assert_eq!(rope.to_string(), "hello wörld!");
        assert_eq!(out.delta(), 1);
        inverse.apply_to_rope(&mut rope).unwrap();
        assert_eq!(rope.to_string(), "hello world");
    }

    #[test]
    fn apply_to_string_matches_rope() {
        let mut s = String::from("naïve café");
        let (_, inv) = Edit::delete(2..5).apply_to_string(&mut s).unwrap();
        assert_eq!(s, "na café");
        inv.apply_to_string(&mut s).unwrap();
        assert_eq!(s, "naïve café");
    }

    #[test]
    fn out_of_range_is_an_error() {
        let mut rope = Rope::from_str("abc");
        assert!(Edit::delete(1..9).apply_to_rope(&mut rope).is_err());
    }

    #[test]
    fn map_pos_rules() {
        let ins = Edit::insert(5, "abc").outcome();
        assert_eq!(ins.map_pos(CharPos(4), Bias::Before), CharPos(4));
        assert_eq!(ins.map_pos(CharPos(5), Bias::Before), CharPos(5));
        assert_eq!(ins.map_pos(CharPos(5), Bias::After), CharPos(8));
        assert_eq!(ins.map_pos(CharPos(6), Bias::Before), CharPos(9));

        let del = Edit::delete(2..6).outcome();
        assert_eq!(del.map_pos(CharPos(3), Bias::After), CharPos(2));
        assert_eq!(del.map_pos(CharPos(6), Bias::Before), CharPos(2));
        assert_eq!(del.map_pos(CharPos(10), Bias::Before), CharPos(6));
    }

    #[test]
    fn map_range_does_not_grow_at_edges() {
        let ins_at_start = Edit::insert(10, "xx").outcome();
        assert_eq!(
            ins_at_start.map_range(CharRange::new(10, 15)),
            CharRange::new(12, 17)
        );
        let ins_at_end = Edit::insert(15, "xx").outcome();
        assert_eq!(
            ins_at_end.map_range(CharRange::new(10, 15)),
            CharRange::new(10, 15)
        );
        let covering = Edit::delete(5..20).outcome();
        assert_eq!(
            covering.map_range(CharRange::new(10, 15)),
            CharRange::empty(5)
        );
        let tail = Edit::replace(12..20, "y").outcome();
        assert_eq!(
            tail.map_range(CharRange::new(10, 15)),
            CharRange::new(10, 12)
        );
    }
}

#[cfg(test)]
mod props {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn inverse_restores_text(
            text in "[a-zé ]{0,40}",
            a in 0usize..50, b in 0usize..50,
            ins in "[a-z]{0,8}",
        ) {
            let len = text.chars().count();
            let range = CharRange::new(a.min(len), b.min(len));
            let mut rope = Rope::from_str(&text);
            let (_, inv) = Edit::replace(range, ins).apply_to_rope(&mut rope).unwrap();
            inv.apply_to_rope(&mut rope).unwrap();
            prop_assert_eq!(rope.to_string(), text);
        }

        #[test]
        fn mapped_positions_stay_in_bounds_and_ordered(
            len in 0usize..40,
            a in 0usize..50, b in 0usize..50,
            ins_len in 0usize..10,
            p in 0usize..50, q in 0usize..50,
        ) {
            let range = CharRange::new(a.min(len), b.min(len));
            let out = Edit::replace(range, "x".repeat(ins_len)).outcome();
            let new_len = (len as isize + out.delta()) as usize;
            let (p, q) = (CharPos(p.min(len)), CharPos(q.min(len)));
            for bias in [Bias::Before, Bias::After] {
                let mp = out.map_pos(p, bias);
                prop_assert!(mp.0 <= new_len);
                if p <= q {
                    prop_assert!(mp <= out.map_pos(q, bias));
                }
            }
            let mr = out.map_range(CharRange::new(p, q));
            prop_assert!(mr.start <= mr.end && mr.end.0 <= new_len);
        }
    }
}
