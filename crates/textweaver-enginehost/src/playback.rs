//! [`Playback`]: the audio-clock client every host-backed backend shares.
//!
//! A backend sends each utterance to a host and hands the host's replies to
//! its `Playback`: audio ([`Playback::on_audio`]), word positions already
//! mapped to byte ranges of the utterance text ([`Playback::on_word`]), the
//! end ([`Playback::on_end`]) and errors. `Playback` queues the audio in
//! the [`Feed`], and [`Playback::emit`] reports events against the playback
//! clock (samples the output actually consumed), per ADR-0003:
//!
//! - `Started` when the utterance's first sample is played;
//! - `Word { byte_range, audio_ms }` when the audio reaches the word,
//!   `audio_ms` being its offset from the utterance's first sample (pauses
//!   excluded), so backends declare `PLAYBACK_EVENTS`;
//! - `Error` (if synthesis failed) then `Finished` when the last sample is
//!   played;
//! - `Cancelled` for every utterance [`Playback::stop`] discarded, and
//!   nothing else for it afterwards.
//!
//! Utterances queue: several may be spoken ahead (the service's
//! lookahead); their audio plays back to back and their events fire in
//! order. Audio that arrives for a later utterance before an earlier one
//! has ended (possible when utterances go to different hosts) waits in that
//! utterance's buffer until its turn.
//!
//! Pause and resume are native: the clock stops with the audio.
//!
//! An output that stops taking samples while audio waits and nothing is
//! paused (a Bluetooth headset that went away, a device that was switched)
//! is reopened after [`DEVICE_STALL`], whether or not a reading is in
//! progress, so an announcement is not stuck behind a dead device. An
//! output whose device the audio system reports gone, or whose device was
//! chosen again ([`crate::audio::set_output_device`]), is reopened at
//! once, on the chosen device or the current default; the audio waiting
//! in the feed plays there. The stall wait and its backoff are measured
//! on the client's [`Clock`] ([`Playback::set_clock`]).
//!
//! [`Playback::capture`] collects a text's audio and word offsets instead
//! of playing them (`synthesize_to_file`).
//!
//! Each utterance and capture carries the engine's own word-mapping state
//! `W` (ECI: the byte range of each index mark; SAPI: a UTF-16 map and a
//! repeat filter), which [`Playback::on_word`] hands back to the engine to
//! turn a host position into a byte range.

use std::collections::{HashMap, VecDeque};
use std::ops::Range;
use std::sync::Arc;
use std::time::{Duration, Instant};

use textweaver_core::UtteranceId;
use textweaver_speech::{BackendId, EventSink, RawEvent, SpeechError, WordTiming};

use crate::audio::{AudioOutput, Feed, Player};
use crate::clock::Clock;
use crate::protocol::EndStatus;
use crate::wav;

/// Audio waiting, nothing paused, and the playback clock still for this
/// long: the output is reopened. Later reopens of the same stall wait
/// twice as long each time, up to [`DEVICE_STALL_MAX`], so a device that
/// is slow to wake is not reopened over and over.
pub const DEVICE_STALL: Duration = Duration::from_secs(1);

/// The longest wait between two reopens of one stalled output.
pub const DEVICE_STALL_MAX: Duration = Duration::from_secs(8);

/// Watches the playback clock for a stalled output.
#[derive(Debug, Default)]
struct StallWatch {
    /// The clock position last seen, and when it was first seen.
    seen: Option<(u64, Instant)>,
    /// How long the clock may stand still before the next reopen.
    wait: Option<Duration>,
    /// Reopens so far, for any reason.
    reopens: u32,
    /// When the output was last reopened.
    last_reopen: Option<Instant>,
}

/// One utterance in flight.
#[derive(Debug)]
struct Active<W> {
    id: UtteranceId,
    token: u64,
    host: usize,
    words: W,
    /// Words received and not yet emitted: (byte range, sample).
    marks: VecDeque<(Range<u32>, u64)>,
    /// Feed position of the utterance's first sample, once it has one.
    start: Option<u64>,
    /// Samples pushed into the feed.
    total: u64,
    /// Samples received before this utterance's turn to play.
    waiting: Vec<i16>,
    /// The host sent `End` (or died).
    done: bool,
    started: bool,
    failed: Option<String>,
}

/// A capture in progress.
#[derive(Debug)]
struct Capture<W> {
    host: usize,
    words: W,
    samples: Vec<i16>,
    marks: Vec<(Range<u32>, u64)>,
    done: bool,
    failed: Option<String>,
}

/// A finished capture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Captured {
    /// The audio, volume applied.
    pub samples: Vec<i16>,
    /// Each word's byte range and first sample, in order.
    pub words: Vec<(Range<u32>, u64)>,
    /// Why synthesis failed, if it did.
    pub failed: Option<String>,
}

impl Captured {
    /// The word timings of this capture on its own clock (sample 0 is the
    /// first sample of [`samples`](Self::samples)), for
    /// `SpeechBackend::synthesize_utterance`. See [`word_timings`].
    pub fn word_timings(&self, sample_rate: u32) -> Vec<WordTiming> {
        word_timings(&self.words, sample_rate)
    }
}

/// Turns word positions, each a byte range and the sample at which the
/// engine reached the word, into [`WordTiming`]s in milliseconds at
/// `sample_rate` Hz. The times are measured on the synthesized audio's own
/// sample clock (the first sample of the file is 0 ms), rounded down as the
/// playback clock rounds them (ADR-0003), so they never point past the
/// word's first sample. Positions keep their order.
pub fn word_timings(words: &[(Range<u32>, u64)], sample_rate: u32) -> Vec<WordTiming> {
    let rate = u64::from(sample_rate.max(1));
    words
        .iter()
        .map(|(range, sample)| WordTiming {
            byte_range: range.clone(),
            audio_ms: u32::try_from(sample.saturating_mul(1000) / rate).unwrap_or(u32::MAX),
        })
        .collect()
}

/// The shared playback client. See the module docs.
pub struct Playback<W> {
    backend: BackendId,
    output: AudioOutput,
    feed: Arc<Feed>,
    player: Option<(Player, u32)>,
    sample_rate: u32,
    next_token: u64,
    active: VecDeque<Active<W>>,
    captures: HashMap<u64, Capture<W>>,
    cancelled: Vec<UtteranceId>,
    stall: StallWatch,
    /// The clock the stall watch reads.
    clock: Clock,
}

impl<W> std::fmt::Debug for Playback<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Playback")
            .field("backend", &self.backend)
            .field("output", &self.output)
            .field("sample_rate", &self.sample_rate)
            .field("active", &self.active.len())
            .field("captures", &self.captures.len())
            .finish_non_exhaustive()
    }
}

impl<W> Playback<W> {
    /// A client for `backend` playing to `output`, at `sample_rate` Hz
    /// until [`set_sample_rate`](Self::set_sample_rate) says otherwise.
    /// The output opens on the first [`ensure_player`](Self::ensure_player).
    pub fn new(backend: BackendId, output: AudioOutput, sample_rate: u32) -> Self {
        Playback {
            backend,
            output,
            feed: Arc::new(Feed::default()),
            player: None,
            sample_rate,
            next_token: 1,
            active: VecDeque::new(),
            captures: HashMap::new(),
            cancelled: Vec::new(),
            stall: StallWatch::default(),
            clock: Clock::system(),
        }
    }

    /// Sets the clock the stalled-output watch reads ([`DEVICE_STALL`]
    /// and its backoff); [`Clock::system`] unless set. Tests use a
    /// [`Clock::manual`] one and move it themselves.
    pub fn set_clock(&mut self, clock: Clock) {
        self.clock = clock;
        self.stall.seen = None;
    }

    /// Where audio goes.
    pub fn output(&self) -> AudioOutput {
        self.output
    }

    /// The sample queue and playback clock.
    pub fn feed(&self) -> &Arc<Feed> {
        &self.feed
    }

    /// The hosts' sample rate in Hz.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Sets the sample rate a newly started host reported. The output
    /// reopens at the new rate on the next [`ensure_player`](Self::ensure_player).
    pub fn set_sample_rate(&mut self, hz: u32) {
        self.sample_rate = hz;
    }

    /// Opens the output at the current sample rate, unless it is open. The
    /// device opens on its own thread: this returns at once, audio waits in
    /// the feed meanwhile, and a device that cannot be opened fails what is
    /// queued (on the next [`emit`](Self::emit)).
    pub fn ensure_player(&mut self) -> Result<(), SpeechError> {
        let rate = self.sample_rate;
        if self.player.as_ref().is_some_and(|(_, r)| *r == rate) {
            return Ok(());
        }
        self.player = None;
        let p = Player::start_for(self.backend, self.output, Arc::clone(&self.feed), rate)?;
        self.player = Some((p, rate));
        Ok(())
    }

    /// A fresh utterance token (never 0).
    pub fn next_token(&mut self) -> u64 {
        let t = self.next_token;
        self.next_token += 1;
        t
    }

    /// Sets the linear output gain (volume); it applies at once and to
    /// captures.
    pub fn set_gain(&self, gain: f32) {
        self.feed.set_gain(gain);
    }

    /// The current gain.
    pub fn gain(&self) -> f32 {
        self.feed.gain()
    }

    /// Native pause: the output plays silence and the clock stops.
    pub fn pause(&self) {
        self.feed.set_paused(true);
    }

    /// Resumes after [`pause`](Self::pause).
    pub fn resume(&self) {
        self.feed.set_paused(false);
    }

    /// Plays a tone over speech (device output only).
    pub fn tone(&mut self, hz: f32, ms: u32) {
        if self.ensure_player().is_ok()
            && let Some((p, _)) = &self.player
        {
            p.tone(hz, ms, self.feed.gain());
        }
    }

    /// Queues utterance `id`, sent to host `host` as `token`.
    pub fn enqueue(&mut self, id: UtteranceId, token: u64, host: usize, words: W) {
        self.active.push_back(Active {
            id,
            token,
            host,
            words,
            marks: VecDeque::new(),
            start: None,
            total: 0,
            waiting: Vec::new(),
            done: false,
            started: false,
            failed: None,
        });
    }

    /// Starts collecting `token`'s audio from host `host` instead of
    /// playing it.
    pub fn capture(&mut self, token: u64, host: usize, words: W) {
        self.captures.insert(
            token,
            Capture {
                host,
                words,
                samples: Vec::new(),
                marks: Vec::new(),
                done: false,
                failed: None,
            },
        );
    }

    /// True while utterances are queued or playing.
    pub fn is_busy(&self) -> bool {
        !self.active.is_empty()
    }

    /// True when host `host` still owes audio (an utterance or capture it
    /// has not ended).
    pub fn owes(&self, host: usize) -> bool {
        self.active.iter().any(|a| a.host == host && !a.done)
            || self.captures.values().any(|c| c.host == host && !c.done)
    }

    /// Whether capture `token` has ended; `None` when there is no such
    /// capture.
    pub fn capture_done(&self, token: u64) -> Option<bool> {
        self.captures.get(&token).map(|c| c.done)
    }

    /// Removes capture `token` and returns what it collected, the audio
    /// scaled by the current gain.
    pub fn take_capture(&mut self, token: u64) -> Option<Captured> {
        let c = self.captures.remove(&token)?;
        Some(Captured {
            samples: wav::apply_gain(c.samples, self.feed.gain()),
            words: c.marks,
            failed: c.failed,
        })
    }

    /// Index of the utterance whose audio goes into the feed now: the
    /// first one not yet ended.
    fn playing_index(&self) -> Option<usize> {
        self.active.iter().position(|a| !a.done)
    }

    /// After an utterance ends, lets the next ones into the feed: pushes
    /// their buffered audio and fixes their start positions.
    fn advance_queue(&mut self) {
        let open = self.playing_index().unwrap_or(self.active.len());
        for a in self.active.iter_mut().take(open + 1) {
            if a.start.is_none() {
                a.start = Some(self.feed.pushed());
            }
            if !a.waiting.is_empty() {
                let w = std::mem::take(&mut a.waiting);
                self.feed.push(&w);
                a.total += w.len() as u64;
            }
        }
    }

    /// Audio from a host for `token`.
    pub fn on_audio(&mut self, token: u64, samples: &[i16]) {
        if let Some(c) = self.captures.get_mut(&token) {
            c.samples.extend_from_slice(samples);
            return;
        }
        let playing = self.playing_index();
        let Some(i) = self.active.iter().position(|a| a.token == token) else {
            return;
        };
        if Some(i) == playing {
            let at = self.feed.push(samples);
            let a = &mut self.active[i];
            a.start.get_or_insert(at);
            a.total += samples.len() as u64;
        } else {
            self.active[i].waiting.extend_from_slice(samples);
        }
    }

    /// A word position from a host for `token`, at `sample` samples into
    /// the utterance; `map` turns it into a byte range of the text using
    /// the utterance's word state (`None` drops it).
    pub fn on_word(
        &mut self,
        token: u64,
        sample: u64,
        map: impl FnOnce(&mut W) -> Option<Range<u32>>,
    ) {
        if let Some(c) = self.captures.get_mut(&token) {
            if let Some(range) = map(&mut c.words) {
                c.marks.push((range, sample));
            }
        } else if let Some(a) = self.active.iter_mut().find(|a| a.token == token)
            && let Some(range) = map(&mut a.words)
        {
            a.marks.push_back((range, sample));
        }
    }

    /// The host ended `token`.
    pub fn on_end(&mut self, token: u64, status: EndStatus) {
        if let Some(c) = self.captures.get_mut(&token) {
            c.done = true;
            if status == EndStatus::Failed && c.failed.is_none() {
                c.failed = Some("synthesis failed".into());
            }
        } else if let Some(a) = self.active.iter_mut().find(|a| a.token == token) {
            a.done = true;
            if status == EndStatus::Failed && a.failed.is_none() {
                a.failed = Some("synthesis failed".into());
            }
            self.advance_queue();
        }
    }

    /// An error from a host about `token`. Returns false when no queued
    /// utterance or capture has that token (the caller logs it).
    pub fn on_error(&mut self, token: u64, message: String) -> bool {
        if let Some(c) = self.captures.get_mut(&token) {
            c.failed = Some(message);
            true
        } else if let Some(a) = self.active.iter_mut().find(|a| a.token == token) {
            a.failed = Some(message);
            true
        } else {
            false
        }
    }

    /// Host `host` is gone: everything it owed ends with `message` as its
    /// error (utterances report `Error` then `Finished` when their turn
    /// comes).
    pub fn host_died(&mut self, host: usize, message: &str) {
        for a in self.active.iter_mut().filter(|a| a.host == host && !a.done) {
            a.done = true;
            a.failed = Some(message.to_owned());
        }
        for c in self
            .captures
            .values_mut()
            .filter(|c| c.host == host && !c.done)
        {
            c.done = true;
            c.failed = Some(message.to_owned());
        }
        self.advance_queue();
    }

    /// Discards every queued utterance and the audio in the feed; each
    /// utterance is reported `Cancelled` by the next [`emit`](Self::emit).
    /// Returns the hosts that had utterances, which should be sent `Stop`
    /// (ascending, each once). Captures continue.
    pub fn stop(&mut self) -> Vec<usize> {
        let mut hosts: Vec<usize> = Vec::new();
        for a in self.active.drain(..) {
            if !hosts.contains(&a.host) {
                hosts.push(a.host);
            }
            self.cancelled.push(a.id);
        }
        hosts.sort_unstable();
        self.feed.clear();
        self.feed.set_paused(false);
        hosts
    }

    /// The output could not be opened: everything queued ends with `why`
    /// as its error (nothing can play it), and the next
    /// [`ensure_player`](Self::ensure_player) tries again.
    fn fail_queued(&mut self, why: &str) {
        for a in self.active.iter_mut() {
            a.done = true;
            a.failed.get_or_insert_with(|| why.to_owned());
            a.start.get_or_insert(0);
        }
        self.feed.clear();
        // Every utterance counts as played to its end.
        let played = self.feed.consumed();
        for a in self.active.iter_mut() {
            a.start = Some(played);
            a.total = 0;
            a.waiting.clear();
        }
    }

    /// How many times the output was reopened: stalled ([`DEVICE_STALL`]),
    /// its device gone, or the device chosen again.
    pub fn device_reopens(&self) -> u32 {
        self.stall.reopens
    }

    /// Opens the output again now (its device went away, was chosen
    /// again, or stopped taking samples); audio waiting in the feed stays.
    fn reopen(&mut self, now: Instant) {
        self.player = None;
        if let Err(e) = self.ensure_player() {
            log::warn!("{}: cannot reopen the audio output: {e}", self.backend);
        }
        self.stall.reopens = self.stall.reopens.saturating_add(1);
        self.stall.last_reopen = Some(now);
    }

    /// Reopens an output whose device went away or was chosen again, and
    /// an open output whose clock has not moved for the stall wait while
    /// audio waits and nothing is paused.
    fn watch_output(&mut self) {
        if let Some(why) = self.player.as_ref().and_then(|(p, _)| p.failure()) {
            log::warn!("{}: {why}", self.backend);
            self.player = None;
            self.fail_queued(&why);
            return;
        }
        let gone = self.player.as_ref().and_then(|(p, _)| {
            p.lost().or_else(|| {
                p.choice_changed()
                    .then(|| "the audio output device was chosen again".to_owned())
            })
        });
        if let Some(why) = gone {
            let now = self.clock.now();
            // A device that keeps failing is not reopened more often than
            // once per stall wait.
            if self
                .stall
                .last_reopen
                .is_some_and(|t| now.saturating_duration_since(t) < DEVICE_STALL)
            {
                return;
            }
            log::warn!("{}: {why}; opening the audio output again", self.backend);
            self.reopen(now);
            self.stall.seen = None;
            self.stall.wait = None;
            return;
        }
        let consumed = self.feed.consumed();
        // A device still opening is not stalled.
        let waiting = self.player.as_ref().is_some_and(|(p, _)| p.is_open())
            && self.feed.pushed() > consumed
            && !self.feed.is_paused();
        if !waiting {
            self.stall.seen = None;
            self.stall.wait = None;
            return;
        }
        let now = self.clock.now();
        let since = match self.stall.seen {
            Some((at, since)) if at == consumed => since,
            _ => {
                // The clock moved (or just started waiting): all is well.
                if self.stall.seen.is_some_and(|(at, _)| at != consumed) {
                    self.stall.wait = None;
                }
                self.stall.seen = Some((consumed, now));
                return;
            }
        };
        let wait = self.stall.wait.unwrap_or(DEVICE_STALL);
        if now.duration_since(since) < wait {
            return;
        }
        log::warn!(
            "{}: the audio output took no samples for {wait:?}; reopening it",
            self.backend
        );
        self.reopen(now);
        self.stall.seen = Some((consumed, now));
        self.stall.wait = Some((wait * 2).min(DEVICE_STALL_MAX));
    }

    /// Emits cancellations and every event the playback clock has reached,
    /// after reopening an output that stopped taking samples.
    pub fn emit(&mut self, sink: &mut dyn EventSink) {
        self.watch_output();
        for id in self.cancelled.drain(..) {
            sink.emit(id, RawEvent::Cancelled);
        }
        let rate = u64::from(self.sample_rate.max(1));
        let played = self.feed.consumed();
        while let Some(a) = self.active.front_mut() {
            let Some(start) = a.start else { break };
            let finished = a.done && played >= start + a.total;
            if sink.is_current(a.id) {
                if !a.started && (played > start || finished) {
                    a.started = true;
                    sink.emit(a.id, RawEvent::Started);
                }
                if a.started {
                    while let Some((range, sample)) = a.marks.front() {
                        if !finished && played < start + sample {
                            break;
                        }
                        let audio_ms = u32::try_from(sample * 1000 / rate).unwrap_or(u32::MAX);
                        sink.emit(
                            a.id,
                            RawEvent::Word {
                                byte_range: range.clone(),
                                audio_ms: Some(audio_ms),
                            },
                        );
                        a.marks.pop_front();
                    }
                }
            }
            // A stale utterance (the service moved on without `stop`) says
            // nothing more until it ends; its end still keeps the contract.
            if !finished {
                break;
            }
            let Some(a) = self.active.pop_front() else {
                break;
            };
            if !a.started {
                sink.emit(a.id, RawEvent::Started);
            }
            if let Some(e) = a.failed {
                sink.emit(a.id, RawEvent::Error(e));
            }
            sink.emit(a.id, RawEvent::Finished);
        }
    }

    /// Stops the audio at once and closes the output (before a backend
    /// shuts its hosts down).
    pub fn close(&mut self) {
        self.feed.clear();
        self.player = None;
    }
}

impl<W> Drop for Playback<W> {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Rec {
        events: Vec<(UtteranceId, RawEvent)>,
        stale_below: u64,
    }

    impl EventSink for Rec {
        fn emit(&mut self, id: UtteranceId, event: RawEvent) {
            self.events.push((id, event));
        }
        fn is_current(&self, id: UtteranceId) -> bool {
            id.generation >= self.stale_below
        }
    }

    impl Rec {
        fn take(&mut self) -> Vec<(UtteranceId, RawEvent)> {
            std::mem::take(&mut self.events)
        }
    }

    fn id(generation: u64, chunk: u32) -> UtteranceId {
        UtteranceId { generation, chunk }
    }

    /// A client whose clock only moves when the test consumes samples.
    fn client() -> Playback<Vec<Range<u32>>> {
        Playback::new("test", AudioOutput::Null { speed: 1.0 }, 1000)
    }

    fn index(i: u32) -> impl FnOnce(&mut Vec<Range<u32>>) -> Option<Range<u32>> {
        move |w| w.get(i as usize).cloned()
    }

    fn word(range: Range<u32>, ms: u32) -> RawEvent {
        RawEvent::Word {
            byte_range: range,
            audio_ms: Some(ms),
        }
    }

    #[test]
    fn events_follow_the_playback_clock() {
        let mut p = client();
        let mut rec = Rec::default();
        let a = id(1, 0);
        let t = p.next_token();
        p.enqueue(a, t, 0, vec![0..5, 6..11]);
        p.on_word(t, 0, index(0));
        p.on_audio(t, &[1; 100]);
        p.on_word(t, 100, index(1));
        p.on_audio(t, &[1; 100]);
        p.on_end(t, EndStatus::Done);
        p.emit(&mut rec);
        assert!(rec.take().is_empty(), "nothing played yet");
        p.feed().skip(1);
        p.emit(&mut rec);
        assert_eq!(rec.take(), [(a, RawEvent::Started), (a, word(0..5, 0))]);
        p.feed().skip(98);
        p.emit(&mut rec);
        assert!(rec.take().is_empty());
        p.feed().skip(1);
        p.emit(&mut rec);
        assert_eq!(rec.take(), [(a, word(6..11, 100))]);
        p.feed().skip(100);
        p.emit(&mut rec);
        assert_eq!(rec.take(), [(a, RawEvent::Finished)]);
        assert!(!p.is_busy());
    }

    #[test]
    fn later_audio_waits_for_its_turn() {
        let mut p = client();
        let mut rec = Rec::default();
        let (a, b) = (id(1, 0), id(1, 1));
        p.enqueue(a, 1, 0, vec![]);
        p.enqueue(b, 2, 1, vec![]);
        // Host 1 answers first: its audio must not play before a's.
        p.on_audio(2, &[2; 10]);
        p.on_end(2, EndStatus::Done);
        assert_eq!(p.feed().pushed(), 0);
        p.on_audio(1, &[1; 20]);
        p.on_end(1, EndStatus::Done);
        assert_eq!(p.feed().pushed(), 30);
        p.feed().skip(30);
        p.emit(&mut rec);
        assert_eq!(
            rec.take(),
            [
                (a, RawEvent::Started),
                (a, RawEvent::Finished),
                (b, RawEvent::Started),
                (b, RawEvent::Finished),
            ]
        );
    }

    #[test]
    fn stop_cancels_everything_and_names_the_hosts() {
        let mut p = client();
        let mut rec = Rec::default();
        p.enqueue(id(1, 0), 1, 1, vec![]);
        p.enqueue(id(1, 1), 2, 0, vec![]);
        p.enqueue(id(1, 2), 3, 1, vec![]);
        p.on_audio(1, &[1; 50]);
        assert_eq!(p.stop(), [0, 1]);
        assert_eq!(p.feed().consumed(), 50, "the queue is discarded");
        // Late replies for stopped tokens are ignored.
        p.on_audio(1, &[1; 50]);
        p.on_end(1, EndStatus::Aborted);
        p.emit(&mut rec);
        assert_eq!(
            rec.take(),
            [
                (id(1, 0), RawEvent::Cancelled),
                (id(1, 1), RawEvent::Cancelled),
                (id(1, 2), RawEvent::Cancelled),
            ]
        );
        assert_eq!(p.feed().pushed(), 50);
        assert!(p.stop().is_empty());
    }

    /// A client whose output takes no samples (a dead device), on a
    /// manual clock the test moves.
    fn stalled_client() -> (Playback<Vec<Range<u32>>>, Clock) {
        let mut p = Playback::new("test", AudioOutput::Null { speed: 0.0 }, 1000);
        let clock = Clock::manual();
        p.set_clock(clock.clone());
        p.ensure_player().unwrap();
        (p, clock)
    }

    /// Moves `clock` by `ms` and lets `p` look at its output.
    fn tick(p: &mut Playback<Vec<Range<u32>>>, clock: &Clock, ms: u64) {
        clock.advance(Duration::from_millis(ms));
        p.emit(&mut Rec::default());
    }

    #[test]
    fn an_output_that_stops_taking_samples_is_reopened() {
        // Before: an announcement queued behind a dead device (outside a
        // reading, where the service's watchdog does not look) waited
        // forever. The clock is the test's: no real waiting, and no
        // failure on a busy machine.
        let (mut p, clock) = stalled_client();
        p.enqueue(id(1, 0), 1, 0, vec![]);
        p.on_audio(1, &[1; 100]);
        p.on_end(1, EndStatus::Done);
        tick(&mut p, &clock, 0);
        let stall = DEVICE_STALL.as_millis() as u64;
        tick(&mut p, &clock, stall - 1);
        assert_eq!(p.device_reopens(), 0, "not before the stall wait");
        tick(&mut p, &clock, 1);
        assert_eq!(p.device_reopens(), 1);
        // Still stalled: the next reopen waits twice as long.
        tick(&mut p, &clock, 2 * stall - 1);
        assert_eq!(p.device_reopens(), 1);
        tick(&mut p, &clock, 1);
        assert_eq!(p.device_reopens(), 2);
        // Then four times, then never more than DEVICE_STALL_MAX.
        tick(&mut p, &clock, 4 * stall);
        assert_eq!(p.device_reopens(), 3);
        let max = DEVICE_STALL_MAX.as_millis() as u64;
        tick(&mut p, &clock, max);
        assert_eq!(p.device_reopens(), 4);
        tick(&mut p, &clock, max - 1);
        assert_eq!(p.device_reopens(), 4);
        tick(&mut p, &clock, 1);
        assert_eq!(p.device_reopens(), 5, "the wait stops growing");
        assert_eq!(p.feed().pushed(), 100, "the audio waits in the feed");
    }

    #[test]
    fn a_paused_or_idle_output_is_not_reopened() {
        let (mut p, clock) = stalled_client();
        // Idle: nothing waits.
        for _ in 0..20 {
            tick(&mut p, &clock, 1000);
        }
        assert_eq!(p.device_reopens(), 0);
        // Paused with audio waiting.
        p.enqueue(id(1, 0), 1, 0, vec![]);
        p.on_audio(1, &[1; 100]);
        p.pause();
        for _ in 0..20 {
            tick(&mut p, &clock, 1000);
        }
        assert_eq!(p.device_reopens(), 0);
        // Resumed, it is watched again.
        p.resume();
        tick(&mut p, &clock, 0);
        tick(&mut p, &clock, DEVICE_STALL.as_millis() as u64);
        assert_eq!(p.device_reopens(), 1);
    }

    #[test]
    fn a_lost_device_is_reopened_at_once_and_its_audio_kept() {
        // The audio system said the device went away (a headset
        // unplugged): the output opens again on the next look, without
        // waiting for the stall, and nothing queued is lost.
        let (mut p, clock) = stalled_client();
        p.enqueue(id(1, 0), 1, 0, vec![]);
        p.on_audio(1, &[1; 100]);
        fn player(p: &Playback<Vec<Range<u32>>>) -> &Player {
            &p.player.as_ref().expect("open").0
        }
        player(&p).mark_lost("unplugged");
        assert_eq!(player(&p).lost().as_deref(), Some("unplugged"));
        tick(&mut p, &clock, 0);
        assert_eq!(p.device_reopens(), 1);
        assert_eq!(player(&p).lost(), None, "a fresh output");
        assert_eq!(p.feed().pushed(), 100);
        assert_eq!(p.feed().consumed(), 0);
        // A device that keeps failing is reopened at most once per stall
        // wait.
        player(&p).mark_lost("unplugged again");
        tick(&mut p, &clock, DEVICE_STALL.as_millis() as u64 - 1);
        assert_eq!(p.device_reopens(), 1);
        tick(&mut p, &clock, 1);
        assert_eq!(p.device_reopens(), 2);
    }

    #[test]
    fn pause_holds_the_clock() {
        let mut p = client();
        let mut rec = Rec::default();
        p.enqueue(id(1, 0), 1, 0, vec![0..1, 1..2]);
        p.on_audio(1, &[1; 10]);
        p.on_word(1, 5, index(0));
        p.on_end(1, EndStatus::Done);
        p.feed().skip(1);
        p.pause();
        assert_eq!(p.feed().skip(9), 0);
        p.emit(&mut rec);
        assert_eq!(rec.take(), [(id(1, 0), RawEvent::Started)]);
        p.resume();
        p.feed().skip(9);
        p.emit(&mut rec);
        assert_eq!(
            rec.take(),
            [(id(1, 0), word(0..1, 5)), (id(1, 0), RawEvent::Finished)]
        );
    }

    #[test]
    fn a_dead_host_fails_what_it_owed_and_nothing_else() {
        let mut p = client();
        let mut rec = Rec::default();
        p.enqueue(id(1, 0), 1, 0, vec![]);
        p.enqueue(id(1, 1), 2, 1, vec![]);
        p.capture(3, 0, vec![]);
        p.capture(4, 1, vec![]);
        assert!(p.owes(0) && p.owes(1));
        p.host_died(0, "gone");
        assert!(!p.owes(0) && p.owes(1));
        assert_eq!(p.capture_done(3), Some(true));
        assert_eq!(p.capture_done(4), Some(false));
        assert_eq!(p.take_capture(3).unwrap().failed.as_deref(), Some("gone"));
        p.emit(&mut rec);
        assert_eq!(
            rec.take(),
            [
                (id(1, 0), RawEvent::Started),
                (id(1, 0), RawEvent::Error("gone".into())),
                (id(1, 0), RawEvent::Finished),
            ]
        );
        p.on_audio(2, &[1; 4]);
        p.on_end(2, EndStatus::Failed);
        p.feed().skip(4);
        p.emit(&mut rec);
        assert_eq!(
            rec.take(),
            [
                (id(1, 1), RawEvent::Started),
                (id(1, 1), RawEvent::Error("synthesis failed".into())),
                (id(1, 1), RawEvent::Finished),
            ]
        );
    }

    #[test]
    fn errors_attach_to_their_utterance() {
        let mut p = client();
        let mut rec = Rec::default();
        p.enqueue(id(1, 0), 1, 0, vec![]);
        assert!(p.on_error(1, "bad input".into()));
        assert!(!p.on_error(99, "nobody's".into()));
        p.on_end(1, EndStatus::Failed);
        p.emit(&mut rec);
        assert_eq!(
            rec.take(),
            [
                (id(1, 0), RawEvent::Started),
                (id(1, 0), RawEvent::Error("bad input".into())),
                (id(1, 0), RawEvent::Finished),
            ]
        );
    }

    #[test]
    fn stale_utterances_only_end() {
        let mut p = client();
        let mut rec = Rec {
            stale_below: 2,
            ..Rec::default()
        };
        p.enqueue(id(1, 0), 1, 0, vec![0..1, 1..2]);
        p.on_word(1, 0, index(0));
        p.on_audio(1, &[1; 4]);
        p.on_end(1, EndStatus::Done);
        p.feed().skip(4);
        p.emit(&mut rec);
        assert_eq!(
            rec.take(),
            [
                (id(1, 0), RawEvent::Started),
                (id(1, 0), RawEvent::Finished)
            ]
        );
    }

    #[test]
    fn empty_utterances_start_and_finish() {
        let mut p = client();
        let mut rec = Rec::default();
        p.enqueue(id(1, 0), 1, 0, vec![]);
        p.on_end(1, EndStatus::Done);
        p.emit(&mut rec);
        assert_eq!(
            rec.take(),
            [
                (id(1, 0), RawEvent::Started),
                (id(1, 0), RawEvent::Finished)
            ]
        );
    }

    #[test]
    fn captures_collect_audio_and_words_with_gain() {
        let mut p = client();
        p.set_gain(0.5);
        let t = p.next_token();
        p.capture(t, 0, vec![0..3, 4..6]);
        p.on_audio(t, &[100, -100]);
        p.on_word(t, 1, index(0));
        p.on_word(t, 2, index(7));
        assert_eq!(p.capture_done(t), Some(false));
        p.on_end(t, EndStatus::Done);
        assert_eq!(p.capture_done(t), Some(true));
        assert_eq!(
            p.take_capture(t),
            Some(Captured {
                samples: vec![50, -50],
                words: vec![(0..3, 1)],
                failed: None,
            })
        );
        assert_eq!(p.capture_done(t), None);
        assert_eq!(p.feed().pushed(), 0, "captures never play");
    }

    #[test]
    fn word_timings_use_the_capture_clock() {
        let words = vec![(0..3, 0), (4..7, 11025), (8..9, 16537)];
        let t = word_timings(&words, 11025);
        assert_eq!(
            t,
            [
                WordTiming {
                    byte_range: 0..3,
                    audio_ms: 0
                },
                WordTiming {
                    byte_range: 4..7,
                    audio_ms: 1000
                },
                // 1,499.95 ms rounds down, as the playback clock does.
                WordTiming {
                    byte_range: 8..9,
                    audio_ms: 1499
                },
            ]
        );
        let c = Captured {
            samples: vec![0; 20000],
            words,
            failed: None,
        };
        assert_eq!(c.word_timings(11025), t);
        assert!(word_timings(&[], 8000).is_empty());
        // A zero rate cannot divide by zero.
        assert_eq!(word_timings(&[(0..1, 5)], 0)[0].audio_ms, 5000);
    }

    #[test]
    fn tokens_are_unique_and_never_zero() {
        let mut p = client();
        let a = p.next_token();
        let b = p.next_token();
        assert!(a != 0 && b != 0 && a != b);
    }
}
