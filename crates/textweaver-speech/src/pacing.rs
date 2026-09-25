//! Highlight pacing when an engine has no word events (and scheduling when it
//! has audio-clock events).
//!
//! Pure and clock-injected so it is testable with a fake clock. Constants are
//! Star's (`star/tts/manager/_playback.py`); Agent B confirms each against the
//! parity inventory and ports `tests/test_highlight_pacing.py`.

use std::time::{Duration, Instant};

/// A monotonic clock, injectable for tests.
pub trait Clock: Send {
    /// Time since an arbitrary fixed origin.
    fn now(&self) -> Duration;
}

/// The real clock.
#[derive(Clone, Copy, Debug)]
pub struct SystemClock {
    origin: Instant,
}

impl Default for SystemClock {
    fn default() -> Self {
        SystemClock {
            origin: Instant::now(),
        }
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
}

/// Pacing parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PacingConfig {
    /// Star `_CB_TIMEOUT`: how long to wait for a word callback before the
    /// timer takes over.
    pub callback_timeout: Duration,
    /// Star `_CB_DEAD`: after this long without callbacks the engine is
    /// treated as having no word events for the rest of the utterance.
    pub callback_dead: Duration,
    /// Star `_MAX_AHEAD` while paced: how many words the timer may run ahead
    /// of the last confirmed word.
    pub max_ahead_paced: u32,
    /// Star `_MAX_AHEAD` while unpaced.
    pub max_ahead_unpaced: u32,
    /// Delay added to audio-clock word events before highlighting (espeak-ng
    /// default 120 ms).
    pub latency_offset: Duration,
    /// Star `highlight_speed` multiplier.
    pub highlight_speed: f32,
}

impl Default for PacingConfig {
    fn default() -> Self {
        PacingConfig {
            callback_timeout: Duration::from_millis(1500),
            callback_dead: Duration::from_secs(6),
            max_ahead_paced: 1,
            max_ahead_unpaced: 4,
            latency_offset: Duration::from_millis(120),
            highlight_speed: 1.0,
        }
    }
}

/// Estimated time per word: `60 / (wpm × highlight_speed)` seconds.
pub fn word_interval(wpm: u16, highlight_speed: f32) -> Duration {
    let rate = f32::from(wpm.max(1)) * highlight_speed.max(0.01);
    Duration::from_secs_f32(60.0 / rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_at_default_rate() {
        let d = word_interval(265, 1.0);
        assert!((d.as_secs_f32() - 60.0 / 265.0).abs() < 1e-6);
    }
}
