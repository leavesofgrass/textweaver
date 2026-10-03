//! How far ahead of the listener the Piper worker synthesizes.
//!
//! A model run cannot be interrupted, so a Stop that comes while the
//! worker is in one waits for it to finish before the next reading can
//! start. Before Wave 8b the worker synthesized every chunk it was given
//! as fast as it could: the rest of the sentence and the service's two
//! sentences of lookahead, a second or more of work after every restart,
//! so a Stop almost always found it busy (W8b-pf measured 830 to 980 ms
//! from Stop to the first audio of the next reading).
//!
//! Now the worker keeps only as much audio queued as it needs to stay
//! ahead: before each piece it waits while the audio already queued would
//! last longer than [`Pace::keep`], which is several times the expected
//! synthesis time of that piece (from the real-time factor of the last
//! few pieces), and never less than [`MIN_LEAD`]. While it waits, it is
//! free, so the next reading starts at once. Under load the real-time
//! factor rises, the margin grows with it, and the worker goes back to
//! synthesizing straight through: it never lets the audio run dry to save
//! time on a Stop.
//!
//! Until the audio queued covers a whole sentence's margin
//! ([`Pace::whole_sentence`]: the start of a reading), the pieces are
//! short phrases, so the first audio comes after a short model run and a
//! Stop never waits long for a run; then whole sentences, for the best
//! intonation.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// The least audio kept queued ahead of the listener.
pub(crate) const MIN_LEAD: Duration = Duration::from_millis(1200);

/// The most audio kept queued ahead (a cap on the margin when a chunk is
/// long and the machine is slow, beyond which the worker never waits).
pub(crate) const MAX_LEAD: Duration = Duration::from_secs(30);

/// How many times the expected synthesis time of the next chunk is kept
/// queued: one for the run itself, the rest for a machine that slows
/// down mid-reading (other programs, a laptop throttling).
const SAFETY: f64 = 3.0;

/// How many recent chunks the estimates use.
const HISTORY: usize = 6;

/// One synthesized chunk, for the estimates.
#[derive(Clone, Copy, Debug)]
struct Sample {
    /// Synthesis time over audio time.
    rtf: f64,
    /// Seconds of audio per byte of text.
    per_byte: f64,
}

/// The worker's pacing state for the current reading.
#[derive(Debug, Default)]
pub(crate) struct Pace {
    /// When the audio sent so far will have played, if it plays without a
    /// break; `None` when nothing is queued (a new reading).
    until: Option<Instant>,
    /// The last few chunks, newest last. Kept across readings: the
    /// machine and the voice do not change with a Stop.
    recent: VecDeque<Sample>,
}

impl Pace {
    /// A new reading: nothing is queued any more.
    pub(crate) fn reset(&mut self) {
        self.until = None;
    }

    /// A chunk of `audio` was synthesized from `bytes` bytes of text in
    /// `elapsed`, and sent to be played at `now`.
    pub(crate) fn sent(&mut self, now: Instant, audio: Duration, elapsed: Duration, bytes: usize) {
        let from = self.until.filter(|u| *u > now).unwrap_or(now);
        self.until = Some(from + audio);
        let secs = audio.as_secs_f64();
        if secs > 0.0 && bytes > 0 {
            if self.recent.len() == HISTORY {
                self.recent.pop_front();
            }
            self.recent.push_back(Sample {
                rtf: elapsed.as_secs_f64() / secs,
                per_byte: secs / bytes as f64,
            });
        }
    }

    /// The audio queued ahead of the listener at `now`, by the worker's
    /// own reckoning.
    pub(crate) fn lead(&self, now: Instant) -> Duration {
        self.until
            .map_or(Duration::ZERO, |u| u.saturating_duration_since(now))
    }

    /// True when the audio queued covers what [`keep`](Self::keep) asks
    /// for a sentence of `bytes` bytes: the worker may then synthesize it
    /// whole, for the best intonation. Before (the start of a reading, or
    /// a machine that cannot keep up), it synthesizes a phrase at a time,
    /// so the next audio comes sooner and a Stop never waits long for a
    /// model run; each phrase adds more audio than its synthesis takes, so
    /// the lead builds.
    pub(crate) fn whole_sentence(&self, now: Instant, queued: Duration, bytes: usize) -> bool {
        self.keep(bytes)
            .is_some_and(|keep| self.lead(now).max(queued) >= keep)
    }

    /// How much audio should still be queued when a chunk of `bytes`
    /// bytes of text starts: [`SAFETY`] times its expected synthesis time,
    /// at least [`MIN_LEAD`]. `None` before any chunk has been timed (the
    /// first chunk never waits).
    pub(crate) fn keep(&self, bytes: usize) -> Option<Duration> {
        // The middle of the recent real-time factors (one slow piece, such
        // as the first after the voice loads, does not hold the worker
        // busy for the next six; [`SAFETY`] covers a machine that slows
        // down), and the wordiest piece's audio per byte.
        let mut rtfs: Vec<f64> = self.recent.iter().map(|s| s.rtf).collect();
        rtfs.sort_by(f64::total_cmp);
        let rtf = *rtfs.get(rtfs.len() / 2)?;
        let per_byte = self.recent.iter().map(|s| s.per_byte).reduce(f64::max)?;
        let synth = rtf * per_byte * bytes as f64;
        let keep = Duration::try_from_secs_f64(synth * SAFETY).unwrap_or(MAX_LEAD);
        Some(keep.clamp(MIN_LEAD, MAX_LEAD))
    }

    /// How long to wait at `now` before synthesizing a chunk of `bytes`
    /// bytes, given `queued`, the audio the output itself still holds (it
    /// stays queued while paused, when the worker's own reckoning runs
    /// on). Zero to go at once.
    pub(crate) fn wait(&self, now: Instant, bytes: usize, queued: Duration) -> Duration {
        let Some(keep) = self.keep(bytes) else {
            return Duration::ZERO;
        };
        self.lead(now).max(queued).saturating_sub(keep)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: fn(u64) -> Duration = Duration::from_millis;

    #[test]
    fn the_first_chunk_never_waits() {
        let p = Pace::default();
        assert_eq!(p.keep(500), None);
        assert_eq!(p.wait(Instant::now(), 500, Duration::from_secs(9)), MS(0));
    }

    #[test]
    fn it_waits_only_while_well_ahead() {
        let t0 = Instant::now();
        let mut p = Pace::default();
        // 40 bytes gave 2 s of audio in 0.2 s: rtf 0.1, 0.05 s a byte.
        p.sent(t0, MS(2000), MS(200), 40);
        assert_eq!(p.lead(t0), MS(2000));
        // A 100-byte chunk: 5 s of audio, 0.5 s to synthesize, keep 1.5 s.
        assert_eq!(p.keep(100), Some(MS(1500)));
        assert_eq!(p.wait(t0, 100, MS(0)), MS(500));
        assert_eq!(p.wait(t0 + MS(600), 100, MS(0)), MS(0));
        // A short one keeps the least: 0.6 s to synthesize, keep 1.2 s.
        assert_eq!(p.keep(40), Some(MIN_LEAD));
        assert_eq!(p.wait(t0, 40, MS(0)), MS(800));
        // A long chunk keeps more queued: 400 bytes, 2 s to synthesize.
        assert_eq!(p.keep(400), Some(MS(6000)));
        assert_eq!(p.wait(t0, 400, MS(0)), MS(0));
    }

    #[test]
    fn whole_sentences_once_the_lead_covers_them() {
        let t0 = Instant::now();
        let mut p = Pace::default();
        // Nothing timed yet: phrases.
        assert!(!p.whole_sentence(t0, MS(0), 40));
        // rtf 0.1, 0.05 s a byte; 2 s queued.
        p.sent(t0, MS(2000), MS(200), 40);
        // A 100-byte sentence needs 1.5 s queued: whole.
        assert!(p.whole_sentence(t0, MS(0), 100));
        // A 200-byte one needs 3 s: a phrase first.
        assert!(!p.whole_sentence(t0, MS(0), 200));
        // What the output holds counts too (paused).
        assert!(p.whole_sentence(t0, MS(3000), 200));
    }

    #[test]
    fn one_slow_piece_does_not_stop_the_waiting() {
        let t0 = Instant::now();
        let mut p = Pace::default();
        // The first piece after loading took 1.5 s for 2 s of audio; the
        // next two were quick.
        p.sent(t0, MS(2000), MS(1500), 40);
        p.sent(t0, MS(2000), MS(200), 40);
        p.sent(t0, MS(2000), MS(200), 40);
        assert_eq!(p.keep(40), Some(MIN_LEAD));
        assert_eq!(p.wait(t0, 40, MS(0)), MS(4800));
    }

    #[test]
    fn a_slow_machine_never_waits() {
        let t0 = Instant::now();
        let mut p = Pace::default();
        // Real time: 1 s of audio took 1 s.
        p.sent(t0, MS(1000), MS(1000), 20);
        p.sent(t0, MS(1000), MS(1000), 20);
        assert_eq!(p.lead(t0), MS(2000));
        assert_eq!(p.wait(t0, 20, MS(0)), MS(0));
    }

    #[test]
    fn the_output_queue_counts_while_paused() {
        let t0 = Instant::now();
        let mut p = Pace::default();
        p.sent(t0, MS(2000), MS(200), 40);
        // Ten seconds later, the reckoning says all has played; a paused
        // output still holds 4 s.
        let later = t0 + Duration::from_secs(10);
        assert_eq!(p.lead(later), MS(0));
        assert_eq!(p.wait(later, 40, MS(4000)), MS(2800));
    }

    #[test]
    fn a_reset_forgets_the_queue_but_not_the_machine() {
        let t0 = Instant::now();
        let mut p = Pace::default();
        p.sent(t0, MS(2000), MS(200), 40);
        p.reset();
        assert_eq!(p.lead(t0), MS(0));
        assert_eq!(p.keep(10), Some(MIN_LEAD));
        // Audio after a gap starts from now, not from the old end.
        p.sent(t0 + MS(5000), MS(1000), MS(100), 20);
        assert_eq!(p.lead(t0 + MS(5000)), MS(1000));
    }
}
