//! Pure navigation history (Paperback's `reader_core/history.rs` shape).
//!
//! The app records the **departure** point of every jump (one consistent
//! rule, unlike Star's TUI and GUI). The rules, each fixing a Star bug
//! (docs/star-parity.md, Part 1 §4.4):
//!
//! - Recording while browsing back discards the forward entries.
//! - Going back from the live position remembers that position, so going
//!   forward again returns to it (Star lost it).
//! - No position is ever stored twice: recording a position already in the
//!   history moves it to the newest place (Star pushed duplicates for
//!   bookmark and chapter jumps).
//! - Capacity defaults to Star's `nav_history_size` (50); the oldest entry
//!   is dropped first.

use serde::{Deserialize, Serialize};
use textweaver_core::{Bias, CharPos, EditOutcome};

/// Back/forward navigation history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct History {
    entries: Vec<CharPos>,
    /// Index of the entry the reader is at while browsing; `entries.len()`
    /// means "at the live position".
    index: usize,
    capacity: usize,
    /// True while the newest entry is the live position remembered by the
    /// first `back`, which forward drops again when it returns there.
    #[serde(default)]
    live_placeholder: bool,
}

impl Default for History {
    fn default() -> Self {
        History::with_capacity(Self::DEFAULT_CAPACITY)
    }
}

impl History {
    /// Star's `nav_history_size` default.
    pub const DEFAULT_CAPACITY: usize = 50;

    /// An empty history holding at most `capacity` entries.
    pub fn with_capacity(capacity: usize) -> Self {
        History {
            entries: Vec::new(),
            index: 0,
            capacity: capacity.max(1),
            live_placeholder: false,
        }
    }

    /// The maximum number of entries.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// True while the reader has gone back and not returned to the live
    /// position.
    pub fn is_browsing(&self) -> bool {
        self.index < self.entries.len()
    }

    /// True when [`back`](Self::back) from `current` would move.
    pub fn can_back(&self, current: CharPos) -> bool {
        if self.is_browsing() {
            self.index > 0
        } else {
            match self.entries.last() {
                Some(&last) if last == current => self.entries.len() >= 2,
                Some(_) => true,
                None => false,
            }
        }
    }

    /// True when [`forward`](Self::forward) would move.
    pub fn can_forward(&self) -> bool {
        self.index + 1 < self.entries.len()
    }

    /// Records that the reader jumped away from `from`. Forward entries are
    /// discarded, an existing equal entry is removed (no duplicates), and the
    /// oldest entry is dropped beyond capacity.
    pub fn record(&mut self, from: CharPos) {
        if self.is_browsing() {
            self.entries.truncate(self.index);
        }
        self.live_placeholder = false;
        self.entries.retain(|&e| e != from);
        self.entries.push(from);
        if self.entries.len() > self.capacity {
            let excess = self.entries.len() - self.capacity;
            self.entries.drain(..excess);
        }
        self.index = self.entries.len();
    }

    /// Goes back from `current`; returns the position to jump to.
    ///
    /// From the live position, `current` is remembered (unless it is already
    /// the newest entry) so that [`forward`](Self::forward) can return to it.
    pub fn back(&mut self, current: CharPos) -> Option<CharPos> {
        if self.is_browsing() {
            if self.index == 0 {
                return None;
            }
            self.index -= 1;
            return Some(self.entries[self.index]);
        }
        if !self.can_back(current) {
            return None;
        }
        if self.entries.last() != Some(&current) {
            self.entries.retain(|&e| e != current);
            self.entries.push(current);
            self.live_placeholder = true;
        }
        // The newest entry is now the live position; step behind it.
        self.index = self.entries.len() - 2;
        Some(self.entries[self.index])
    }

    /// Goes forward; returns the position to jump to. Reaching the newest
    /// entry returns to the live position.
    pub fn forward(&mut self) -> Option<CharPos> {
        if !self.can_forward() {
            return None;
        }
        self.index += 1;
        let pos = self.entries[self.index];
        if self.index + 1 == self.entries.len() {
            // Back at the live position.
            if self.live_placeholder {
                self.entries.pop();
                self.live_placeholder = false;
            }
            self.index = self.entries.len();
        }
        Some(pos)
    }

    /// The recorded positions, oldest first (including the remembered live
    /// position while browsing).
    pub fn entries(&self) -> &[CharPos] {
        &self.entries
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when nothing is recorded.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Forgets everything (a new document).
    pub fn clear(&mut self) {
        self.entries.clear();
        self.index = 0;
        self.live_placeholder = false;
    }

    /// Moves every entry across an edit. Entries that collapse onto the same
    /// position are merged, keeping the newer one.
    pub fn shift(&mut self, outcome: &EditOutcome) {
        let browsing = self.is_browsing();
        let at = self.entries.get(self.index).copied();
        let at = at.map(|p| outcome.map_pos(p, Bias::Before));
        for e in &mut self.entries {
            *e = outcome.map_pos(*e, Bias::Before);
        }
        let mut seen: Vec<CharPos> = Vec::with_capacity(self.entries.len());
        for &e in self.entries.iter().rev() {
            if !seen.contains(&e) {
                seen.push(e);
            }
        }
        seen.reverse();
        self.entries = seen;
        self.index = match (browsing, at) {
            (true, Some(p)) => self
                .entries
                .iter()
                .position(|&e| e == p)
                .unwrap_or(self.entries.len()),
            _ => self.entries.len(),
        };
        if self.index >= self.entries.len() {
            self.live_placeholder = false;
            self.index = self.entries.len();
        }
    }
}

#[cfg(test)]
mod tests {
    use textweaver_core::Edit;

    use super::*;

    #[test]
    fn back_forward_and_truncate() {
        let mut h = History::default();
        h.record(CharPos(10));
        h.record(CharPos(20));
        assert_eq!(h.back(CharPos(30)), Some(CharPos(20)));
        assert_eq!(h.back(CharPos(20)), Some(CharPos(10)));
        assert_eq!(h.back(CharPos(10)), None);
        assert_eq!(h.forward(), Some(CharPos(20)));
        h.record(CharPos(25));
        assert_eq!(h.forward(), None);
        assert_eq!(h.entries(), &[CharPos(10), CharPos(25)]);
    }

    #[test]
    fn forward_returns_to_the_live_position() {
        let mut h = History::default();
        h.record(CharPos(10));
        assert_eq!(h.back(CharPos(50)), Some(CharPos(10)));
        assert!(h.is_browsing());
        assert_eq!(h.forward(), Some(CharPos(50)));
        assert!(!h.is_browsing());
        assert_eq!(h.entries(), &[CharPos(10)]);
        assert_eq!(h.forward(), None);
    }

    #[test]
    fn back_from_a_recorded_position_skips_it() {
        let mut h = History::default();
        h.record(CharPos(10));
        h.record(CharPos(20));
        // The reader is back at 20 by other means.
        assert_eq!(h.back(CharPos(20)), Some(CharPos(10)));
        assert_eq!(h.forward(), Some(CharPos(20)));
        assert_eq!(h.entries(), &[CharPos(10), CharPos(20)]);
        let mut single = History::default();
        single.record(CharPos(5));
        assert_eq!(single.back(CharPos(5)), None);
    }

    #[test]
    fn no_duplicates() {
        let mut h = History::default();
        for p in [1, 2, 1, 3, 2] {
            h.record(CharPos(p));
        }
        assert_eq!(h.entries(), &[CharPos(1), CharPos(3), CharPos(2)]);
    }

    #[test]
    fn capacity_drops_oldest() {
        let mut h = History::with_capacity(2);
        for p in [1, 2, 3] {
            h.record(CharPos(p));
        }
        assert_eq!(h.entries(), &[CharPos(2), CharPos(3)]);
    }

    #[test]
    fn shift_moves_and_merges() {
        let mut h = History::default();
        h.record(CharPos(2));
        h.record(CharPos(8));
        h.record(CharPos(20));
        h.shift(&Edit::delete(1..10).outcome());
        assert_eq!(h.entries(), &[CharPos(1), CharPos(11)]);
    }
}

#[cfg(test)]
mod props {
    use proptest::prelude::*;

    use super::*;

    #[derive(Clone, Debug)]
    enum Op {
        Record(usize),
        Back(usize),
        Forward,
    }

    fn op() -> impl Strategy<Value = Op> {
        prop_oneof![
            (0usize..20).prop_map(Op::Record),
            (0usize..20).prop_map(Op::Back),
            Just(Op::Forward),
        ]
    }

    proptest! {
        #[test]
        fn invariants_hold(ops in proptest::collection::vec(op(), 0..60), cap in 1usize..8) {
            let mut h = History::with_capacity(cap);
            for op in ops {
                match op {
                    Op::Record(p) => h.record(CharPos(p)),
                    Op::Back(p) => {
                        let was_live = !h.is_browsing();
                        if let Some(target) = h.back(CharPos(p)) {
                            if was_live {
                                prop_assert_ne!(target, CharPos(p));
                                // One forward returns to where we were.
                                let mut probe = h.clone();
                                prop_assert_eq!(probe.forward(), Some(CharPos(p)));
                                prop_assert!(!probe.is_browsing());
                            }
                        }
                    }
                    Op::Forward => {
                        let _ = h.forward();
                    }
                }
                let e = h.entries();
                // No duplicates.
                for (i, a) in e.iter().enumerate() {
                    prop_assert!(!e[i + 1..].contains(a));
                }
                // Capacity (the live placeholder may add one while browsing).
                prop_assert!(e.len() <= cap + 1);
                prop_assert!(h.index <= e.len());
            }
        }
    }
}
