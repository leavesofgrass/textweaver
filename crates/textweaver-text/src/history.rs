//! Pure navigation history (Paperback's `reader_core/history.rs` shape).
//!
//! Recording a jump while not at the newest entry truncates the forward
//! entries (Star's rule). Capacity defaults to Star's `nav_history_size` (50).

use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, EditOutcome};

/// Back/forward navigation history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct History {
    entries: Vec<CharPos>,
    /// Index of the current entry; `entries.len()` means "past the newest".
    index: usize,
    capacity: usize,
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
        }
    }

    /// Records that the reader jumped away from `from`. Forward entries are
    /// discarded; the oldest entry is dropped beyond capacity.
    pub fn record(&mut self, from: CharPos) {
        self.entries.truncate(self.index);
        if self.entries.last() != Some(&from) {
            self.entries.push(from);
        }
        if self.entries.len() > self.capacity {
            let excess = self.entries.len() - self.capacity;
            self.entries.drain(..excess);
        }
        self.index = self.entries.len();
    }

    /// Goes back from `current`; returns the position to jump to.
    pub fn back(&mut self, current: CharPos) -> Option<CharPos> {
        if self.index == 0 {
            return None;
        }
        if self.index == self.entries.len() {
            // Remember where we were so forward can return to it.
            self.entries.push(current);
        }
        self.index -= 1;
        Some(self.entries[self.index])
    }

    /// Goes forward; returns the position to jump to.
    pub fn forward(&mut self) -> Option<CharPos> {
        if self.index + 1 >= self.entries.len() {
            return None;
        }
        self.index += 1;
        let pos = self.entries[self.index];
        if self.index + 1 == self.entries.len() {
            // Back at the live position: drop the placeholder.
            self.entries.pop();
            self.index = self.entries.len();
        }
        Some(pos)
    }

    /// The recorded positions, oldest first.
    pub fn entries(&self) -> &[CharPos] {
        &self.entries
    }

    /// Moves every entry across an edit.
    pub fn shift(&mut self, outcome: &EditOutcome) {
        for e in &mut self.entries {
            *e = outcome.map_pos(*e, textweaver_core::Bias::Before);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_forward_and_truncate() {
        let mut h = History::default();
        h.record(CharPos(10));
        h.record(CharPos(20));
        assert_eq!(h.back(CharPos(30)), Some(CharPos(20)));
        assert_eq!(h.back(CharPos(20)), Some(CharPos(10)));
        assert_eq!(h.forward(), Some(CharPos(20)));
        h.record(CharPos(25));
        assert_eq!(h.forward(), None);
        assert_eq!(h.entries(), &[CharPos(10), CharPos(25)]);
    }

    #[test]
    fn capacity_drops_oldest() {
        let mut h = History::with_capacity(2);
        for p in [1, 2, 3] {
            h.record(CharPos(p));
        }
        assert_eq!(h.entries(), &[CharPos(2), CharPos(3)]);
    }
}
