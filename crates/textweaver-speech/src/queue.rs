//! The utterance queue: pure bookkeeping for what is playing, what the engine
//! already holds (lookahead), what waits, and which generation is current.
//!
//! Rules (ADR-0003):
//!
//! - The generation is bumped before every stop or restart, and every
//!   utterance handed to the engine is stamped with the current generation.
//!   A restart re-stamps the utterances it keeps, so late events from the
//!   engine's copies are recognizably stale.
//! - The engine holds the playing utterance plus up to `lookahead` more
//!   (two by default), so engines with per-utterance latency do not leave a
//!   gap between sentences.
//! - Queued chunks are cancelled by id.

use std::collections::VecDeque;

use textweaver_core::{Utterance, UtteranceId};

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

/// Default number of chunks handed to the engine ahead of the playing one.
pub const DEFAULT_LOOKAHEAD: usize = 2;

/// Playing, submitted, and waiting utterances.
#[derive(Clone, Debug)]
pub struct ReadingQueue {
    generation: Generation,
    lookahead: usize,
    next_chunk: u32,
    /// Handed to the engine, in order; the front is playing (or about to).
    submitted: VecDeque<Utterance>,
    /// Not yet handed to the engine.
    waiting: VecDeque<Utterance>,
}

impl Default for ReadingQueue {
    fn default() -> Self {
        Self::new(DEFAULT_LOOKAHEAD)
    }
}

impl ReadingQueue {
    /// An empty queue that keeps `lookahead` chunks in the engine ahead of
    /// the playing one.
    pub fn new(lookahead: usize) -> Self {
        ReadingQueue {
            generation: Generation::default(),
            lookahead,
            next_chunk: 0,
            submitted: VecDeque::new(),
            waiting: VecDeque::new(),
        }
    }

    /// The current generation.
    pub fn generation(&self) -> Generation {
        self.generation
    }

    /// True when `id` belongs to the current generation.
    pub fn is_current(&self, id: UtteranceId) -> bool {
        self.generation.is_current(id)
    }

    /// Nothing submitted and nothing waiting.
    pub fn is_empty(&self) -> bool {
        self.submitted.is_empty() && self.waiting.is_empty()
    }

    /// Number of utterances submitted or waiting.
    pub fn len(&self) -> usize {
        self.submitted.len() + self.waiting.len()
    }

    /// The utterance at the front (playing or about to play).
    pub fn front(&self) -> Option<&Utterance> {
        self.submitted.front().or_else(|| self.waiting.front())
    }

    /// Looks up a submitted utterance by id.
    pub fn submitted(&self, id: UtteranceId) -> Option<&Utterance> {
        self.submitted.iter().find(|u| u.id == id)
    }

    /// Position of `id` among the submitted utterances (0 = front).
    pub fn submitted_index(&self, id: UtteranceId) -> Option<usize> {
        self.submitted.iter().position(|u| u.id == id)
    }

    /// Every utterance, submitted first, in order.
    pub fn iter(&self) -> impl Iterator<Item = &Utterance> {
        self.submitted.iter().chain(self.waiting.iter())
    }

    fn stamp(&mut self, mut u: Utterance) -> Utterance {
        u.id = UtteranceId {
            generation: self.generation.get(),
            chunk: self.next_chunk,
        };
        self.next_chunk = self.next_chunk.saturating_add(1);
        u
    }

    /// Bumps the generation, drops everything, and returns what was dropped
    /// (submitted first). The caller stops the engine.
    pub fn clear(&mut self) -> Vec<Utterance> {
        self.generation.bump();
        self.next_chunk = 0;
        let mut out: Vec<Utterance> = self.submitted.drain(..).collect();
        out.extend(self.waiting.drain(..));
        out
    }

    /// Replaces everything with `utterances` under a new generation,
    /// numbering chunks from 0. Returns the new generation.
    pub fn start(&mut self, utterances: Vec<Utterance>) -> u64 {
        self.clear();
        for u in utterances {
            let u = self.stamp(u);
            self.waiting.push_back(u);
        }
        self.generation.get()
    }

    /// Restarts with `utterances` (typically the remainder of the current
    /// reading, trimmed) under a new generation. Chunk numbers continue.
    pub fn restart(&mut self, utterances: Vec<Utterance>) -> u64 {
        self.generation.bump();
        self.submitted.clear();
        self.waiting.clear();
        for u in utterances {
            let u = self.stamp(u);
            self.waiting.push_back(u);
        }
        self.generation.get()
    }

    /// Appends an utterance after everything queued, in the current
    /// generation. Returns its id.
    pub fn push_back(&mut self, u: Utterance) -> UtteranceId {
        let u = self.stamp(u);
        let id = u.id;
        self.waiting.push_back(u);
        id
    }

    /// Utterances to hand to the engine now, moved from waiting to
    /// submitted: enough to have the playing one plus `lookahead` in the
    /// engine.
    pub fn to_submit(&mut self) -> Vec<Utterance> {
        let mut out = Vec::new();
        while self.submitted.len() < 1 + self.lookahead {
            let Some(u) = self.waiting.pop_front() else {
                break;
            };
            out.push(u.clone());
            self.submitted.push_back(u);
        }
        out
    }

    /// Marks `id` as done (finished, cancelled, or failed) and removes it.
    /// Returns the removed utterance, or `None` if `id` is unknown.
    pub fn complete(&mut self, id: UtteranceId) -> Option<Utterance> {
        let i = self.submitted.iter().position(|u| u.id == id)?;
        self.submitted.remove(i)
    }

    /// Cancels one queued chunk by id, wherever it is. Returns true if it was
    /// submitted to the engine (the caller must then restart the engine to
    /// really drop it), false if it was only waiting or unknown.
    pub fn cancel(&mut self, id: UtteranceId) -> bool {
        if let Some(i) = self.waiting.iter().position(|u| u.id == id) {
            self.waiting.remove(i);
            return false;
        }
        if let Some(i) = self.submitted.iter().position(|u| u.id == id) {
            self.submitted.remove(i);
            return true;
        }
        false
    }

    /// Removes and returns every utterance (submitted first) without bumping
    /// the generation; used when pausing to keep the remainder.
    pub fn take_all(&mut self) -> Vec<Utterance> {
        let mut out: Vec<Utterance> = self.submitted.drain(..).collect();
        out.extend(self.waiting.drain(..));
        out
    }

    /// Keeps only the not-yet-submitted utterances for which `keep` is true.
    pub fn retain_waiting(&mut self, mut keep: impl FnMut(&Utterance) -> bool) {
        self.waiting.retain(|u| keep(u));
    }
}

#[cfg(test)]
mod tests {
    use textweaver_core::CharPos;

    use super::*;

    fn utts(n: usize) -> Vec<Utterance> {
        (0..n)
            .map(|i| Utterance::literal(format!("Sentence {i}."), CharPos(i * 12)))
            .collect()
    }

    #[test]
    fn start_stamps_generation_and_chunks() {
        let mut q = ReadingQueue::default();
        let g = q.start(utts(3));
        assert_eq!(g, 1);
        let ids: Vec<_> = q.iter().map(|u| u.id).collect();
        assert_eq!(
            ids,
            (0..3)
                .map(|c| UtteranceId {
                    generation: 1,
                    chunk: c
                })
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn lookahead_keeps_two_chunks_ahead() {
        let mut q = ReadingQueue::default();
        q.start(utts(5));
        let first = q.to_submit();
        assert_eq!(first.len(), 3, "playing + two lookahead");
        assert!(q.to_submit().is_empty());
        let done = q.complete(first[0].id).unwrap();
        assert_eq!(done.id.chunk, 0);
        let next = q.to_submit();
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].id.chunk, 3);
        assert_eq!(q.front().unwrap().id.chunk, 1);
    }

    #[test]
    fn restart_bumps_generation_and_restamps() {
        let mut q = ReadingQueue::default();
        q.start(utts(4));
        let old = q.to_submit();
        let rest = q.take_all();
        assert_eq!(rest.len(), 4);
        let g = q.restart(rest);
        assert_eq!(g, 2);
        assert!(!q.is_current(old[0].id));
        assert!(q.iter().all(|u| u.id.generation == 2));
        // Chunk numbers keep increasing so ids never repeat.
        assert_eq!(q.front().unwrap().id.chunk, 4);
    }

    #[test]
    fn cancel_by_id() {
        let mut q = ReadingQueue::default();
        q.start(utts(5));
        let sub = q.to_submit();
        let waiting_id = q.iter().nth(4).unwrap().id;
        assert!(!q.cancel(waiting_id));
        assert!(q.cancel(sub[1].id));
        assert_eq!(q.len(), 3);
        assert!(!q.cancel(waiting_id), "already gone");
        assert!(q.iter().all(|u| u.id != sub[1].id && u.id != waiting_id));
    }

    #[test]
    fn clear_returns_everything_and_invalidates() {
        let mut q = ReadingQueue::default();
        q.start(utts(3));
        let sub = q.to_submit();
        let dropped = q.clear();
        assert_eq!(dropped.len(), 3);
        assert!(q.is_empty());
        assert!(!q.is_current(sub[0].id));
    }
}
