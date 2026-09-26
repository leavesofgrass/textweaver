//! Highlight pacing when an engine has no (or unreliable) word events, and
//! the clocks the speech service schedules against.
//!
//! Everything here is pure and driven by an injectable [`Clock`], so it is
//! tested with a [`FakeClock`] instead of wall time. The constants and clamp
//! rules are Star's (`star/tts/manager/_playback.py`, inventoried in
//! `docs/history/star-parity.md` Part 2 section 1):
//!
//! | Star name | Here | Value |
//! |---|---|---|
//! | timer interval | [`word_interval`] | `60 / max(1, wpm × max(0.1, highlight_speed))` s |
//! | `_CB_TIMEOUT` | [`PacingConfig::callback_timeout`] | 1.5 s |
//! | `_CB_DEAD` | [`PacingConfig::callback_dead`] | 6 s |
//! | `_MAX_AHEAD` paced / unpaced | [`PacingConfig::max_ahead_paced`] / [`PacingConfig::max_ahead_unpaced`] | 1 / 4 |
//! | `_ANCHOR_TIMEOUT` | [`ANCHOR_TIMEOUT`] | 0.75 s |
//! | `espeak_highlight_offset_ms` | [`PacingConfig::latency_offset`] | 120 ms |
//!
//! Star bugs fixed here (Part 2 section 7.1): the start word is painted at
//! once instead of one interval late (B13); the pacer reports the same word
//! only once instead of on every tick (B5, deduplicated by the service); the
//! pacer stops asking for ticks at the end of the word list (B4); and the
//! interval can change mid-utterance when the rate changes (B9).

use std::sync::{Arc, Mutex};
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

/// A clock that only moves when told to. Clones share the same time, so a
/// test can keep one handle and give another to the code under test.
#[derive(Clone, Debug, Default)]
pub struct FakeClock {
    now: Arc<Mutex<Duration>>,
}

impl FakeClock {
    /// A fake clock at time zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Moves the clock forward by `d`.
    pub fn advance(&self, d: Duration) {
        let mut now = self.now.lock().unwrap_or_else(|p| p.into_inner());
        *now += d;
    }

    /// Sets the clock to `t` (may move backwards; tests only).
    pub fn set(&self, t: Duration) {
        *self.now.lock().unwrap_or_else(|p| p.into_inner()) = t;
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Duration {
        *self.now.lock().unwrap_or_else(|p| p.into_inner())
    }
}

/// The playback clock: a [`Clock`] that stands still while speech is
/// natively paused, so highlights scheduled on the engine's audio clock stay
/// aligned with the audio after a resume.
pub struct PlaybackClock {
    inner: Box<dyn Clock>,
    paused_at: Option<Duration>,
    paused_total: Duration,
}

impl PlaybackClock {
    /// Wraps `inner`.
    pub fn new(inner: Box<dyn Clock>) -> Self {
        PlaybackClock {
            inner,
            paused_at: None,
            paused_total: Duration::ZERO,
        }
    }

    /// Playback time: wall time minus the time spent paused.
    pub fn now(&self) -> Duration {
        let wall = self.paused_at.unwrap_or_else(|| self.inner.now());
        wall.saturating_sub(self.paused_total)
    }

    /// Wall time of the underlying clock.
    pub fn wall(&self) -> Duration {
        self.inner.now()
    }

    /// Stops playback time. Idempotent.
    pub fn pause(&mut self) {
        if self.paused_at.is_none() {
            self.paused_at = Some(self.inner.now());
        }
    }

    /// Restarts playback time. Idempotent.
    pub fn resume(&mut self) {
        if let Some(at) = self.paused_at.take() {
            self.paused_total += self.inner.now().saturating_sub(at);
        }
    }

    /// True while paused.
    pub fn is_paused(&self) -> bool {
        self.paused_at.is_some()
    }

    /// Converts a playback-time deadline into a wall-time wait from now.
    /// `None` while paused (playback time does not advance).
    pub fn wait_until(&self, deadline: Duration) -> Option<Duration> {
        if self.is_paused() {
            None
        } else {
            Some(deadline.saturating_sub(self.now()))
        }
    }
}

impl std::fmt::Debug for PlaybackClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlaybackClock")
            .field("paused_at", &self.paused_at)
            .field("paused_total", &self.paused_total)
            .finish_non_exhaustive()
    }
}

/// Star `_ANCHOR_TIMEOUT`: with an engine that reports words, the timer waits
/// this long for the first word event before it starts estimating.
pub const ANCHOR_TIMEOUT: Duration = Duration::from_millis(750);

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

/// Estimated time per word: `60 / max(1, wpm × max(0.1, highlight_speed))`
/// seconds (Star's formula, including its guards).
pub fn word_interval(wpm: u16, highlight_speed: f32) -> Duration {
    let speed = if highlight_speed.is_finite() {
        highlight_speed.max(0.1)
    } else {
        1.0
    };
    let rate = f64::from((f32::from(wpm) * speed).max(1.0));
    // Whole microseconds, so 1200 wpm is exactly 50 ms.
    Duration::from_micros((60_000_000.0 / rate).round() as u64)
}

/// Star's highlight timer as a pure state machine.
///
/// The timer moves one word per interval. Engine word events ("callbacks")
/// are fed in with [`on_callback`](Self::on_callback); they record the
/// engine-confirmed word and pull the estimate forward. Each tick applies
/// Star's three regimes, measured by the age of the last callback:
///
/// - younger than `callback_timeout` (fresh): clamp back to the confirmed
///   word, never lead the engine;
/// - younger than `callback_dead` (pause): run at most `max_ahead` words past
///   the confirmed word, then hold;
/// - older (dead), or no callback ever: run free.
///
/// Engines that report words make the timer wait up to [`ANCHOR_TIMEOUT`]
/// for the first callback before it estimates anything.
#[derive(Clone, Debug)]
pub struct TimerPacer {
    config: PacingConfig,
    interval: Duration,
    max_ahead: usize,
    word_count: usize,
    /// Next word the timer would paint.
    idx: usize,
    /// Star `_current_word_idx`: the shared estimate.
    estimate: Option<usize>,
    /// Star `_last_cb_word_idx` and `_last_cb_time`.
    last_cb: Option<(usize, Duration)>,
    next_tick: Duration,
    /// While `Some`, waiting for the first callback until this time.
    anchor_deadline: Option<Duration>,
}

impl TimerPacer {
    /// A pacer for `word_count` words at `interval` per word. `paced` selects
    /// `max_ahead_paced` (engines whose word events follow the audio clock)
    /// instead of `max_ahead_unpaced`.
    pub fn new(config: PacingConfig, word_count: usize, interval: Duration, paced: bool) -> Self {
        let max_ahead = if paced {
            config.max_ahead_paced
        } else {
            config.max_ahead_unpaced
        };
        TimerPacer {
            config,
            interval: interval.max(Duration::from_millis(1)),
            max_ahead: usize::try_from(max_ahead).unwrap_or(usize::MAX),
            word_count,
            idx: 0,
            estimate: None,
            last_cb: None,
            next_tick: Duration::ZERO,
            anchor_deadline: None,
        }
    }

    /// Starts at word `start` at time `now` and returns the word to paint at
    /// once (Star painted it one interval late, bug B13). With
    /// `expect_callbacks`, the timer then waits up to [`ANCHOR_TIMEOUT`] for
    /// the first callback before estimating.
    pub fn start(&mut self, now: Duration, start: usize, expect_callbacks: bool) -> Option<usize> {
        self.next_tick = now + self.interval;
        self.anchor_deadline = expect_callbacks.then_some(now + ANCHOR_TIMEOUT);
        if start < self.word_count {
            self.estimate = Some(start);
            self.idx = start + 1;
            Some(start)
        } else {
            self.idx = start;
            None
        }
    }

    /// Records an engine-confirmed word (Star `on_word_cb`): the confirmed
    /// word is always recorded, and the estimate only moves forward.
    pub fn on_callback(&mut self, word: usize, now: Duration) {
        self.last_cb = Some((word, now));
        if self.estimate.is_none_or(|e| word >= e) {
            self.estimate = Some(word);
        }
        self.anchor_deadline = None;
    }

    /// Sets the estimate directly (tests and restarts; Star's harness wrote
    /// `_current_word_idx`).
    pub fn set_estimate(&mut self, word: Option<usize>) {
        self.estimate = word;
    }

    /// Sets the last confirmed word and when it was confirmed without moving
    /// the estimate (tests; Star's harness wrote `_last_cb_word_idx`).
    pub fn set_last_callback(&mut self, cb: Option<(usize, Duration)>) {
        self.last_cb = cb;
        if cb.is_some() {
            self.anchor_deadline = None;
        }
    }

    /// Changes the word interval from the next tick on (rate changed).
    pub fn set_interval(&mut self, interval: Duration) {
        self.interval = interval.max(Duration::from_millis(1));
    }

    /// The word interval.
    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// The engine-confirmed word, if any.
    pub fn last_confirmed(&self) -> Option<usize> {
        self.last_cb.map(|(w, _)| w)
    }

    /// The current estimate (the word most recently painted or confirmed).
    pub fn estimate(&self) -> Option<usize> {
        self.estimate
    }

    /// When the next tick is due, or `None` when the timer has nothing left
    /// to do (past the last word and not holding).
    pub fn next_deadline(&self) -> Option<Duration> {
        if self.idx >= self.word_count && self.estimate.is_some_and(|e| e + 1 >= self.word_count) {
            // At the end: only a callback could change anything.
            if self.last_cb.is_none_or(|(cb, _)| cb + 1 >= self.word_count) {
                return None;
            }
        }
        Some(self.next_tick)
    }

    /// Runs every tick due at or before `now` and returns the words to paint,
    /// in order. Consecutive repeats are possible (the fresh regime repaints
    /// the confirmed word); the caller drops them.
    pub fn tick(&mut self, now: Duration) -> Vec<usize> {
        let mut paints = Vec::new();
        while self.next_tick <= now {
            let t = self.next_tick;
            self.next_tick += self.interval;
            if let Some(deadline) = self.anchor_deadline {
                if t < deadline {
                    continue;
                }
                self.anchor_deadline = None;
            }
            if let Some(e) = self.estimate
                && e > self.idx
            {
                self.idx = e;
            }
            if let Some((cb, at)) = self.last_cb {
                let age = t.saturating_sub(at);
                if self.idx > cb && age < self.config.callback_timeout {
                    self.idx = cb;
                } else if self.idx > cb.saturating_add(self.max_ahead)
                    && age < self.config.callback_dead
                {
                    continue;
                }
            }
            if self.idx < self.word_count {
                paints.push(self.idx);
                self.estimate = Some(self.idx);
                self.idx += 1;
            }
        }
        paints
    }
}

/// Byte ranges of the words in spoken text, as the timer counts them: runs
/// of non-whitespace that contain a letter or digit, trimmed of leading and
/// trailing punctuation ("Hello," is the word "Hello").
///
/// The timer counts the words the engine actually speaks, not source words,
/// so an expansion such as "2024" spoken as "twenty twenty-four" is two
/// timer words that both highlight "2024" (fixes Star's word-count mismatch,
/// Part 2 section 7.2 N2).
pub fn spoken_words(text: &str) -> Vec<std::ops::Range<u32>> {
    let to_u32 = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    let mut out = Vec::new();
    let flush = |a: usize, b: usize, out: &mut Vec<std::ops::Range<u32>>| {
        let tok = &text[a..b];
        let Some(first) = tok.find(char::is_alphanumeric) else {
            return;
        };
        let last = tok
            .char_indices()
            .rev()
            .find(|(_, c)| c.is_alphanumeric())
            .map_or(tok.len(), |(i, c)| i + c.len_utf8());
        out.push(to_u32(a + first)..to_u32(a + last));
    };
    let mut start: Option<usize> = None;
    for (i, c) in text.char_indices() {
        if c.is_whitespace() {
            if let Some(a) = start.take() {
                flush(a, i, &mut out);
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(a) = start {
        flush(a, text.len(), &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spoken_words_trim_punctuation() {
        let t = "\"Hello,\" she said \u{2014} twenty-four caf\u{e9}s... (ok)";
        let words: Vec<&str> = spoken_words(t)
            .into_iter()
            .map(|r| &t[r.start as usize..r.end as usize])
            .collect();
        assert_eq!(
            words,
            ["Hello", "she", "said", "twenty-four", "caf\u{e9}s", "ok"]
        );
        assert!(spoken_words("  ... ").is_empty());
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn interval_at_default_rate() {
        let d = word_interval(265, 1.0);
        assert!((d.as_secs_f32() - 60.0 / 265.0).abs() < 1e-6);
        // Star's guards: speed floored at 0.1, rate×speed at 1.
        assert!((word_interval(265, 0.0).as_secs_f32() - 60.0 / 26.5).abs() < 1e-4);
        assert!((word_interval(0, 1.0).as_secs_f32() - 60.0).abs() < 1e-4);
        assert!((word_interval(1200, 1.0).as_secs_f32() - 0.05).abs() < 1e-6);
    }

    /// Star's test fixture: rate 1200 (interval 50 ms), 400 words, unpaced
    /// (`_MAX_AHEAD = 4`), no anchor wait. Runs the timer for `run` and
    /// returns the painted words. `engine` is called before every tick with
    /// the tick time, to refresh callbacks like Star's background thread.
    fn run_timer(
        pacer: &mut TimerPacer,
        start: usize,
        run: Duration,
        mut engine: impl FnMut(&mut TimerPacer, Duration),
    ) -> Vec<usize> {
        let mut paints = Vec::new();
        paints.extend(pacer.start(Duration::ZERO, start, false));
        let mut t = Duration::ZERO;
        while t < run {
            t += ms(10);
            engine(pacer, t);
            paints.extend(pacer.tick(t));
        }
        paints
    }

    fn star_pacer() -> TimerPacer {
        TimerPacer::new(
            PacingConfig::default(),
            400,
            word_interval(1200, 1.0),
            false,
        )
    }

    #[test]
    fn overshot_estimate_clamps_back_to_engine_truth() {
        // test_highlight_pacing.py:46-71
        let mut p = star_pacer();
        let paints = run_timer(&mut p, 0, ms(800), |p, t| {
            if t == ms(10) {
                p.set_estimate(Some(60));
            }
            // The engine re-confirms word 20 every 30 ms.
            if t.as_millis() % 30 == 0 || t == ms(10) {
                p.set_last_callback(Some((20, t)));
            }
        });
        assert!(!paints.is_empty());
        assert!(*paints.iter().max().unwrap() <= 21, "{paints:?}");
    }

    #[test]
    fn pause_holds_instead_of_running_away() {
        // test_highlight_pacing.py:74-88
        let mut p = star_pacer();
        p.set_estimate(Some(10));
        p.set_last_callback(Some((10, Duration::ZERO)));
        let paints = run_timer(&mut p, 10, ms(2500), |_, _| {});
        assert!(!paints.is_empty());
        assert_eq!(*paints.iter().max().unwrap(), 14, "{paints:?}");
    }

    #[test]
    fn dead_callback_stream_still_free_runs() {
        // test_highlight_pacing.py:91-107: the last callback is 10 s old.
        let mut p = star_pacer();
        let paints = run_timer(&mut p, 10, ms(1500), |p, t| {
            if t == ms(10) {
                p.set_estimate(Some(10));
                p.set_last_callback(Some((10, Duration::ZERO)));
            }
        });
        // Callback time 0 with the clock starting at 10 s would be the same
        // as Star's `now - 10.0`; emulate by shifting the pacer's view.
        let mut q = star_pacer();
        q.set_estimate(Some(10));
        q.start(ms(10_000), 10, false);
        q.set_last_callback(Some((10, Duration::ZERO)));
        let mut late = Vec::new();
        let mut t = ms(10_000);
        while t < ms(11_500) {
            t += ms(10);
            late.extend(q.tick(t));
        }
        assert!(*late.iter().max().unwrap() > 14, "{late:?}");
        // Within the first 1.5 s of a fresh callback the timer does not run.
        assert!(*paints.iter().max().unwrap() <= 14);
    }

    #[test]
    fn no_callbacks_ever_free_runs() {
        // test_highlight_pacing.py:110-120
        let mut p = star_pacer();
        let paints = run_timer(&mut p, 0, ms(1500), |_, _| {});
        assert!(*paints.iter().max().unwrap() > 5);
        // One word per 50 ms, starting at once.
        assert_eq!(paints.len(), 31);
        assert_eq!(paints[..4], [0, 1, 2, 3]);
    }

    #[test]
    fn word_callback_always_records_engine_position() {
        // test_highlight_pacing.py:123-150: a callback behind the estimate is
        // recorded but does not move the estimate backwards.
        let mut p = TimerPacer::new(PacingConfig::default(), 8, ms(50), false);
        p.start(Duration::ZERO, 0, true);
        p.set_estimate(Some(6));
        p.on_callback(1, ms(5));
        assert_eq!(p.last_confirmed(), Some(1));
        assert_eq!(p.estimate(), Some(6));
    }

    #[test]
    fn start_word_is_painted_immediately_and_timer_stops_at_the_end() {
        let mut p = TimerPacer::new(PacingConfig::default(), 3, ms(100), false);
        assert_eq!(p.start(Duration::ZERO, 0, false), Some(0));
        assert_eq!(p.tick(ms(100)), vec![1]);
        assert_eq!(p.tick(ms(250)), vec![2]);
        assert_eq!(p.next_deadline(), None);
        assert!(p.tick(ms(1000)).is_empty());
    }

    #[test]
    fn anchor_waits_for_first_callback() {
        let mut p = TimerPacer::new(PacingConfig::default(), 20, ms(100), true);
        assert_eq!(p.start(Duration::ZERO, 0, true), Some(0));
        assert!(p.tick(ms(700)).is_empty(), "no estimate before the anchor");
        p.on_callback(2, ms(710));
        // Fresh callback: the timer paints the confirmed word, not beyond.
        assert_eq!(p.tick(ms(800)), vec![2]);
        let mut q = TimerPacer::new(PacingConfig::default(), 20, ms(100), true);
        q.start(Duration::ZERO, 0, true);
        // No callback within 750 ms: free-run from word 1.
        assert_eq!(q.tick(ms(800)), vec![1]);
    }

    #[test]
    fn interval_change_applies_to_following_ticks() {
        let mut p = TimerPacer::new(PacingConfig::default(), 50, ms(100), false);
        p.start(Duration::ZERO, 0, false);
        assert_eq!(p.tick(ms(100)), vec![1]);
        p.set_interval(ms(10));
        // The already scheduled tick at 200 ms runs, then every 10 ms.
        assert_eq!(p.tick(ms(220)), vec![2, 3, 4]);
    }

    #[test]
    fn playback_clock_stands_still_while_paused() {
        let fake = FakeClock::new();
        let mut c = PlaybackClock::new(Box::new(fake.clone()));
        fake.advance(ms(100));
        assert_eq!(c.now(), ms(100));
        c.pause();
        fake.advance(ms(500));
        assert_eq!(c.now(), ms(100));
        assert_eq!(c.wait_until(ms(150)), None);
        c.resume();
        fake.advance(ms(20));
        assert_eq!(c.now(), ms(120));
        assert_eq!(c.wait_until(ms(150)), Some(ms(30)));
        assert_eq!(c.wall(), ms(620));
    }
}
