//! The utterance queue: pure bookkeeping for what is playing, what is queued,
//! and which generation is current. Agent B implements the two-chunk
//! lookahead and cancellation by id; Phase 0 keeps only the generation rule.

use textweaver_core::UtteranceId;

/// Generation bookkeeping shared by the queue and the event sink.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Generation(u64);

impl Generation {
    /// The current generation number.
    pub fn get(self) -> u64 {
        self.0
    }

    /// Bumps the generation. Star's rule: bump before every stop or restart,
    /// so late events from the old reading are recognizably stale.
    pub fn bump(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }

    /// True when `id` belongs to the current generation.
    pub fn is_current(self, id: UtteranceId) -> bool {
        id.generation == self.0
    }
}
