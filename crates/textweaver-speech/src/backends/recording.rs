//! The recording backend: a test double that records every call and emits
//! scripted events.
//!
//! It is always compiled (no feature needed) so other crates' tests can use
//! it, for example the app and TUI tests that check highlight ranges against
//! the spoken source ranges:
//!
//! ```
//! use std::time::Duration;
//! use textweaver_speech::core::{CharPos, CharRange, Utterance};
//! use textweaver_speech::{RecordingBackend, ServiceConfig, SpeechService, SpeechStatus};
//!
//! let (backend, handle) = RecordingBackend::new();
//! let service = SpeechService::spawn(backend.into_factory(), ServiceConfig::default()).unwrap();
//! service.read(vec![Utterance::literal("Hello brave world.", CharPos(0))]);
//! let mut words = Vec::new();
//! while let Ok(s) = service.statuses().recv_timeout(Duration::from_secs(5)) {
//!     match s {
//!         SpeechStatus::Position { source_range: Some(r), .. } => words.push(r),
//!         SpeechStatus::Finished { .. } => break,
//!         _ => {}
//!     }
//! }
//! assert_eq!(words, [CharRange::new(0, 5), CharRange::new(6, 11), CharRange::new(12, 17)]);
//! assert_eq!(handle.spoken_texts(), ["Hello brave world."]);
//! ```
//!
//! Modes ([`RecordingMode`]):
//!
//! - `Instant` (default): `speak` emits `Started`, one `Word` per spoken word
//!   (as counted by [`spoken_words`]), and `Finished` before it returns.
//!   Deterministic and fast.
//! - `Manual`: `speak` emits nothing; the test drives every event through the
//!   [`RecordingHandle`] (delivered on the next `poll`).
//! - `Timed`: behaves like an audio-clock engine playing a queue: when an
//!   utterance's audio starts, `Started` and every `Word` (with `audio_ms`)
//!   arrive in a burst, and `Finished` arrives once its duration has passed
//!   on the backend's clock.
//!
//! `stop` ends every unfinished utterance with `Cancelled` (delivered on the
//! next `poll`).
//!
//! Files: `synthesize_to_file` and `synthesize_utterance` write a silent WAV
//! of one word-length per spoken word ([`RecordingBackend::FILE_MS_PER_WORD`],
//! or `ms_per_word` in `Timed` mode) and report word `i` at
//! `i × ms_per_word`, so audio export can be tested with exact timings.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use textweaver_core::{Rate, Utterance, UtteranceId};

use crate::backend::{
    BackendFactory, BackendId, Caps, EventSink, FileSynthesis, RawEvent, SpeechBackend,
    SpeechError, Voice, VoiceParams, WordTiming,
};
use crate::pacing::{Clock, SystemClock, spoken_words};
use crate::wav::silent_wav;

/// How the recording backend responds to `speak`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RecordingMode {
    /// `Started`, every `Word`, and `Finished` during `speak`.
    #[default]
    Instant,
    /// Nothing on its own; events come from the [`RecordingHandle`].
    Manual,
    /// An audio-clock engine: words with `audio_ms = i × ms_per_word` in a
    /// burst when the utterance's audio starts; `Finished` after
    /// `words × ms_per_word` on the backend clock. Delivered by `poll`.
    Timed {
        /// Audio duration of one word, in ms.
        ms_per_word: u32,
    },
}

/// One recorded call.
#[derive(Clone, Debug, PartialEq)]
pub enum Call {
    /// `set_params`.
    SetParams(VoiceParams),
    /// `speak`, with the utterance exactly as the service passed it.
    Speak(Utterance),
    /// `stop`.
    Stop,
    /// `pause`.
    Pause,
    /// `resume`.
    Resume,
    /// `tone`.
    Tone {
        /// Frequency in Hz.
        hz: f32,
        /// Duration in ms.
        ms: u32,
    },
    /// `synthesize_to_file`.
    SynthesizeToFile {
        /// Text passed in.
        text: String,
        /// Output path.
        path: PathBuf,
    },
}

/// An utterance spoken and not yet finished or cancelled.
#[derive(Clone, Debug)]
struct Pending {
    id: UtteranceId,
    /// Audio start and end on the backend clock (`Timed` mode only).
    timing: Option<(Duration, Duration)>,
    /// Events not yet released (`Timed` mode holds the burst until the
    /// audio starts).
    held: Vec<RawEvent>,
}

#[derive(Debug, Default)]
struct State {
    calls: Vec<Call>,
    /// Events waiting for the next `poll`.
    outbox: VecDeque<(UtteranceId, RawEvent)>,
    pending: Vec<Pending>,
    voices: Vec<Voice>,
    fail_next_speak: Option<String>,
    /// Backend clock time of a native pause (`PAUSE` capability only).
    paused_at: Option<Duration>,
    mode: RecordingMode,
    caps: Caps,
    params: VoiceParams,
}

fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(|p| p.into_inner())
}

/// Inspects and drives a [`RecordingBackend`] from the test thread. Cheap to
/// clone and `Send`.
#[derive(Clone, Debug)]
pub struct RecordingHandle {
    state: Arc<Mutex<State>>,
}

impl RecordingHandle {
    /// Every call so far, in order.
    pub fn calls(&self) -> Vec<Call> {
        lock(&self.state).calls.clone()
    }

    /// Forgets the recorded calls.
    pub fn clear_calls(&self) {
        lock(&self.state).calls.clear();
    }

    /// The utterances passed to `speak`, in order.
    pub fn spoken(&self) -> Vec<Utterance> {
        lock(&self.state)
            .calls
            .iter()
            .filter_map(|c| match c {
                Call::Speak(u) => Some(u.clone()),
                _ => None,
            })
            .collect()
    }

    /// The texts passed to `speak`, in order.
    pub fn spoken_texts(&self) -> Vec<String> {
        self.spoken().into_iter().map(|u| u.text).collect()
    }

    /// The last parameters passed to `set_params`.
    pub fn params(&self) -> VoiceParams {
        lock(&self.state).params.clone()
    }

    /// Queues an event for utterance `id`, delivered on the next `poll`.
    pub fn emit(&self, id: UtteranceId, event: RawEvent) {
        let mut s = lock(&self.state);
        if matches!(
            event,
            RawEvent::Finished | RawEvent::Cancelled | RawEvent::Error(_)
        ) {
            s.pending.retain(|p| p.id != id);
        }
        s.outbox.push_back((id, event));
    }

    /// Queues `Started` for `id`.
    pub fn start(&self, id: UtteranceId) {
        self.emit(id, RawEvent::Started);
    }

    /// Queues a `Word` event for the `n`-th spoken word of utterance `id` (as
    /// counted by [`spoken_words`]), with an optional audio time. Returns
    /// false if `id` was never spoken or has fewer words.
    pub fn word(&self, id: UtteranceId, n: usize, audio_ms: Option<u32>) -> bool {
        let text = {
            let s = lock(&self.state);
            s.calls.iter().rev().find_map(|c| match c {
                Call::Speak(u) if u.id == id => Some(u.text.clone()),
                _ => None,
            })
        };
        let Some(range) = text.and_then(|t| spoken_words(&t).into_iter().nth(n)) else {
            return false;
        };
        self.emit(
            id,
            RawEvent::Word {
                byte_range: range,
                audio_ms,
            },
        );
        true
    }

    /// Queues `Finished` for `id`.
    pub fn finish(&self, id: UtteranceId) {
        self.emit(id, RawEvent::Finished);
    }

    /// Utterances spoken and not yet finished or cancelled, oldest first.
    pub fn pending(&self) -> Vec<UtteranceId> {
        lock(&self.state).pending.iter().map(|p| p.id).collect()
    }

    /// Replaces the voice list.
    pub fn set_voices(&self, voices: Vec<Voice>) {
        lock(&self.state).voices = voices;
    }

    /// Makes the next `speak` fail with `SpeechError::Engine(message)`.
    pub fn fail_next_speak(&self, message: impl Into<String>) {
        lock(&self.state).fail_next_speak = Some(message.into());
    }

    /// Changes the mode for subsequent `speak` calls.
    pub fn set_mode(&self, mode: RecordingMode) {
        lock(&self.state).mode = mode;
    }
}

/// A backend that records calls and emits scripted events.
pub struct RecordingBackend {
    state: Arc<Mutex<State>>,
    clock: Box<dyn Clock>,
}

impl std::fmt::Debug for RecordingBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordingBackend").finish_non_exhaustive()
    }
}

impl RecordingBackend {
    /// Default capabilities: word events, pitch, volume, tones, and
    /// synthesis to file. No native pause, so the service emulates it.
    pub const DEFAULT_CAPS: Caps = Caps::WORD_EVENTS
        .union(Caps::PITCH)
        .union(Caps::VOLUME)
        .union(Caps::TONES)
        .union(Caps::SYNTH_TO_FILE);

    /// Sample rate of the silent WAV files `synthesize_to_file` writes.
    pub const FILE_SAMPLE_RATE: u32 = 16_000;

    /// Length of one word in written files, in ms (outside `Timed` mode,
    /// whose `ms_per_word` is used instead).
    pub const FILE_MS_PER_WORD: u32 = 250;

    /// A backend in [`RecordingMode::Instant`] with [`Self::DEFAULT_CAPS`],
    /// and the handle that inspects it.
    pub fn new() -> (Self, RecordingHandle) {
        Self::with(RecordingMode::Instant, Self::DEFAULT_CAPS)
    }

    /// A backend with the given mode and capabilities.
    pub fn with(mode: RecordingMode, caps: Caps) -> (Self, RecordingHandle) {
        let voice = |id: &str, name: &str, lang: &str| Voice {
            id: id.into(),
            name: name.into(),
            languages: vec![lang.into()],
            ..Voice::default()
        };
        let state = Arc::new(Mutex::new(State {
            mode,
            caps,
            voices: vec![
                voice("rec-en-gb", "Recording UK", "en-GB"),
                voice("rec-en-us", "Recording US", "en-US"),
            ],
            ..State::default()
        }));
        let handle = RecordingHandle {
            state: Arc::clone(&state),
        };
        (
            RecordingBackend {
                state,
                clock: Box::new(SystemClock::default()),
            },
            handle,
        )
    }

    /// Uses `clock` for `Timed` mode (tests pass a
    /// [`FakeClock`](crate::pacing::FakeClock)).
    pub fn with_clock(mut self, clock: Box<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Wraps the backend in a [`BackendFactory`] for
    /// [`SpeechService::spawn`](crate::SpeechService::spawn).
    pub fn into_factory(self) -> BackendFactory {
        Box::new(move || Ok(Box::new(self) as Box<dyn SpeechBackend>))
    }

    /// Writes the stand-in audio for `text`: a silent 16-bit mono WAV at
    /// [`FILE_SAMPLE_RATE`](Self::FILE_SAMPLE_RATE), one word-length of
    /// silence per spoken word (at least one), and reports word `i` at
    /// `i × ms_per_word`. Deterministic, so export tests can compare cue
    /// files byte for byte.
    fn write_silence(&mut self, text: &str, path: &Path) -> Result<FileSynthesis, SpeechError> {
        let per_word = {
            let mut s = lock(&self.state);
            s.calls.push(Call::SynthesizeToFile {
                text: text.to_owned(),
                path: path.to_owned(),
            });
            match s.mode {
                RecordingMode::Timed { ms_per_word } => ms_per_word,
                _ => Self::FILE_MS_PER_WORD,
            }
        };
        let words = spoken_words(text);
        let n = u64::try_from(words.len().max(1)).unwrap_or(u64::MAX);
        let samples = n
            .saturating_mul(u64::from(per_word))
            .saturating_mul(u64::from(Self::FILE_SAMPLE_RATE))
            / 1000;
        let samples = usize::try_from(samples).map_err(|e| SpeechError::Io(e.to_string()))?;
        std::fs::write(path, silent_wav(samples, Self::FILE_SAMPLE_RATE))
            .map_err(|e| SpeechError::Io(e.to_string()))?;
        let words = words
            .into_iter()
            .enumerate()
            .map(|(i, byte_range)| WordTiming {
                byte_range,
                audio_ms: u32::try_from(i)
                    .unwrap_or(u32::MAX)
                    .saturating_mul(per_word),
            })
            .collect();
        Ok(FileSynthesis { words })
    }

    /// Moves due `Timed` events into the outbox, then delivers the outbox.
    fn deliver(&mut self, sink: &mut dyn EventSink) {
        let now = self.clock.now();
        let events: Vec<(UtteranceId, RawEvent)> = {
            let mut s = lock(&self.state);
            let mut released = Vec::new();
            // While natively paused, audio (and so its events) stands still.
            let mut i = if s.paused_at.is_some() {
                s.pending.len()
            } else {
                0
            };
            while i < s.pending.len() {
                let Some((start, end)) = s.pending[i].timing else {
                    i += 1;
                    continue;
                };
                let id = s.pending[i].id;
                if start <= now {
                    let held: Vec<RawEvent> = s.pending[i].held.drain(..).collect();
                    released.extend(held.into_iter().map(|e| (id, e)));
                }
                if end <= now {
                    released.push((id, RawEvent::Finished));
                    s.pending.remove(i);
                } else {
                    i += 1;
                }
            }
            s.outbox.extend(released);
            s.outbox.drain(..).collect()
        };
        for (id, e) in events {
            sink.emit(id, e);
        }
    }
}

impl SpeechBackend for RecordingBackend {
    fn id(&self) -> BackendId {
        "recording"
    }

    fn capabilities(&self) -> Caps {
        lock(&self.state).caps
    }

    fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        Ok(lock(&self.state).voices.clone())
    }

    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
        let mut s = lock(&self.state);
        s.calls.push(Call::SetParams(params.clone()));
        s.params = params.clone();
        Ok(())
    }

    fn effective_wpm(&self) -> u16 {
        match lock(&self.state).params.rate {
            Rate::Wpm(w) => w,
        }
    }

    fn speak(
        &mut self,
        utterance: &Utterance,
        sink: &mut dyn EventSink,
    ) -> Result<(), SpeechError> {
        let id = utterance.id;
        let now = self.clock.now();
        {
            let mut s = lock(&self.state);
            s.calls.push(Call::Speak(utterance.clone()));
            if let Some(msg) = s.fail_next_speak.take() {
                return Err(SpeechError::Engine(msg));
            }
            let words = spoken_words(&utterance.text);
            match s.mode {
                RecordingMode::Manual => s.pending.push(Pending {
                    id,
                    timing: None,
                    held: Vec::new(),
                }),
                RecordingMode::Instant => {
                    s.outbox.push_back((id, RawEvent::Started));
                    for w in words {
                        s.outbox.push_back((
                            id,
                            RawEvent::Word {
                                byte_range: w,
                                audio_ms: None,
                            },
                        ));
                    }
                    s.outbox.push_back((id, RawEvent::Finished));
                }
                RecordingMode::Timed { ms_per_word } => {
                    // Audio starts when the previous utterance's audio ends.
                    let start = s
                        .pending
                        .iter()
                        .filter_map(|p| p.timing.map(|(_, end)| end))
                        .max()
                        .unwrap_or(now)
                        .max(now);
                    let n = u64::try_from(words.len().max(1)).unwrap_or(u64::MAX);
                    let end =
                        start + Duration::from_millis(n.saturating_mul(u64::from(ms_per_word)));
                    let mut held = vec![RawEvent::Started];
                    for (i, w) in words.into_iter().enumerate() {
                        let i = u32::try_from(i).unwrap_or(u32::MAX);
                        held.push(RawEvent::Word {
                            byte_range: w,
                            audio_ms: Some(i.saturating_mul(ms_per_word)),
                        });
                    }
                    s.pending.push(Pending {
                        id,
                        timing: Some((start, end)),
                        held,
                    });
                }
            }
        }
        self.deliver(sink);
        Ok(())
    }

    fn poll(&mut self, sink: &mut dyn EventSink) {
        self.deliver(sink);
    }

    fn stop(&mut self) {
        let mut s = lock(&self.state);
        s.calls.push(Call::Stop);
        s.outbox.clear();
        s.paused_at = None;
        let pending: Vec<UtteranceId> = s.pending.drain(..).map(|p| p.id).collect();
        for id in pending {
            s.outbox.push_back((id, RawEvent::Cancelled));
        }
    }

    fn pause(&mut self) -> Result<(), SpeechError> {
        let now = self.clock.now();
        let mut s = lock(&self.state);
        s.calls.push(Call::Pause);
        if s.caps.contains(Caps::PAUSE) {
            s.paused_at.get_or_insert(now);
            Ok(())
        } else {
            Err(SpeechError::Unsupported("pause"))
        }
    }

    fn resume(&mut self) -> Result<(), SpeechError> {
        let now = self.clock.now();
        let mut s = lock(&self.state);
        s.calls.push(Call::Resume);
        if !s.caps.contains(Caps::PAUSE) {
            return Err(SpeechError::Unsupported("resume"));
        }
        if let Some(at) = s.paused_at.take() {
            // Audio stood still while paused: shift every pending utterance.
            let delta = now.saturating_sub(at);
            for p in &mut s.pending {
                if let Some((start, end)) = &mut p.timing {
                    if *start > at {
                        *start += delta;
                    }
                    *end += delta;
                }
            }
        }
        Ok(())
    }

    fn synthesize_to_file(&mut self, text: &str, path: &Path) -> Result<(), SpeechError> {
        self.write_silence(text, path).map(|_| ())
    }

    fn synthesize_utterance(
        &mut self,
        utterance: &Utterance,
        path: &Path,
    ) -> Result<FileSynthesis, SpeechError> {
        self.write_silence(&utterance.text, path)
    }

    fn tone(&mut self, hz: f32, ms: u32) {
        lock(&self.state).calls.push(Call::Tone { hz, ms });
    }
}

#[cfg(test)]
mod tests {
    use textweaver_core::CharPos;

    use super::*;
    use crate::pacing::FakeClock;

    #[derive(Default)]
    struct Collect(Vec<(UtteranceId, RawEvent)>);

    impl EventSink for Collect {
        fn emit(&mut self, id: UtteranceId, event: RawEvent) {
            self.0.push((id, event));
        }
        fn is_current(&self, _: UtteranceId) -> bool {
            true
        }
    }

    fn utt(text: &str, chunk: u32) -> Utterance {
        let mut u = Utterance::literal(text, CharPos(0));
        u.id = UtteranceId {
            generation: 1,
            chunk,
        };
        u
    }

    #[test]
    fn instant_mode_emits_words_and_finishes() {
        let (mut b, h) = RecordingBackend::new();
        let mut sink = Collect::default();
        b.speak(&utt("Hi there.", 0), &mut sink).unwrap();
        let kinds: Vec<_> = sink.0.iter().map(|(_, e)| e.clone()).collect();
        assert_eq!(
            kinds,
            [
                RawEvent::Started,
                RawEvent::Word {
                    byte_range: 0..2,
                    audio_ms: None
                },
                RawEvent::Word {
                    byte_range: 3..8,
                    audio_ms: None
                },
                RawEvent::Finished
            ]
        );
        assert_eq!(h.spoken_texts(), ["Hi there."]);
    }

    #[test]
    fn manual_mode_is_driven_by_the_handle_and_stop_cancels() {
        let (mut b, h) = RecordingBackend::with(RecordingMode::Manual, Caps::empty());
        let mut sink = Collect::default();
        let u = utt("One two.", 0);
        b.speak(&u, &mut sink).unwrap();
        assert!(sink.0.is_empty());
        h.start(u.id);
        assert!(h.word(u.id, 1, None));
        assert!(!h.word(u.id, 2, None));
        b.poll(&mut sink);
        assert_eq!(sink.0.len(), 2);
        b.stop();
        b.poll(&mut sink);
        assert_eq!(sink.0.last().unwrap().1, RawEvent::Cancelled);
        assert!(h.pending().is_empty());
        assert!(b.pause().is_err());
    }

    #[test]
    fn files_are_silence_with_exact_word_timings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("u.wav");
        let (mut b, h) = RecordingBackend::new();
        let s = b
            .synthesize_utterance(&utt("One two three.", 0), &path)
            .unwrap();
        let ms: Vec<u32> = s.words.iter().map(|w| w.audio_ms).collect();
        assert_eq!(ms, [0, 250, 500]);
        assert_eq!(s.words[2].byte_range, 8..13);
        let bytes = std::fs::read(&path).unwrap();
        // 3 words × 250 ms at 16 kHz, 2 bytes per sample.
        assert_eq!(bytes.len(), 44 + 3 * 4000 * 2);
        assert!(matches!(
            h.calls().last(),
            Some(Call::SynthesizeToFile { text, .. }) if text == "One two three."
        ));
    }

    #[test]
    fn timed_mode_plays_a_queue_on_the_clock() {
        let clock = FakeClock::new();
        let (b, _h) = RecordingBackend::with(
            RecordingMode::Timed { ms_per_word: 100 },
            RecordingBackend::DEFAULT_CAPS | Caps::AUDIO_CLOCK,
        );
        let mut b = b.with_clock(Box::new(clock.clone()));
        let mut sink = Collect::default();
        b.speak(&utt("a b c", 0), &mut sink).unwrap();
        b.speak(&utt("d e", 1), &mut sink).unwrap();
        // Only the first burst: the second utterance's audio has not started.
        assert_eq!(sink.0.len(), 4);
        assert_eq!(
            sink.0[3].1,
            RawEvent::Word {
                byte_range: 4..5,
                audio_ms: Some(200)
            }
        );
        clock.advance(Duration::from_millis(300));
        b.poll(&mut sink);
        let tail: Vec<_> = sink.0[4..]
            .iter()
            .map(|(id, e)| (id.chunk, e.clone()))
            .collect();
        assert_eq!(tail[0], (0, RawEvent::Finished));
        assert_eq!(tail[1], (1, RawEvent::Started));
        assert_eq!(tail.len(), 4);
        clock.advance(Duration::from_millis(200));
        b.poll(&mut sink);
        assert_eq!(sink.0.last().unwrap().1, RawEvent::Finished);
    }
}
