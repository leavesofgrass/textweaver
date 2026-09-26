//! The engine contract.

use std::ops::Range;
use std::path::Path;

use serde::{Deserialize, Serialize};
use textweaver_core::{Pitch, Rate, Utterance, UtteranceId, Volume};

/// Stable backend identifier ("null", "espeak", "omnivox").
pub type BackendId = &'static str;

bitflags::bitflags! {
    /// What an engine can do. The service adapts to missing capabilities
    /// (timer pacing without `WORD_EVENTS`, emulated pause without `PAUSE`).
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Caps: u32 {
        /// Emits `RawEvent::Word` for spoken words.
        const WORD_EVENTS = 1 << 0;
        /// Word events carry `audio_ms` on the engine's audio clock.
        const AUDIO_CLOCK = 1 << 1;
        /// Native pause and resume.
        const PAUSE = 1 << 2;
        /// Pitch control.
        const PITCH = 1 << 3;
        /// Volume control.
        const VOLUME = 1 << 4;
        /// Rate changes apply to speech already in progress.
        const LIVE_RATE = 1 << 5;
        /// SSML mark events (index marks).
        const SSML_MARKS = 1 << 6;
        /// Can synthesize to a file.
        const SYNTH_TO_FILE = 1 << 7;
        /// Can play tones.
        const TONES = 1 << 8;
        /// Must be created and driven on the process's main thread (the GUI
        /// hosts such engines; the TUI cannot use them).
        const REQUIRES_MAIN_THREAD = 1 << 9;
        /// The engine normalizes text itself (numbers, dates, times,
        /// currency, abbreviations), as ETI-Eloquence does. The service then
        /// skips those transforms and still applies Markdown residue, the
        /// pronunciation lexicon, split caps, and punctuation verbosity
        /// (see [`crate::normalize`]).
        const NATIVE_NORMALIZATION = 1 << 10;
    }
}

/// An installed voice.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Voice {
    /// Engine-specific id, passed back in [`VoiceParams::voice`].
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// BCP 47 language tags the voice speaks.
    pub languages: Vec<String>,
    /// Gender, when the engine reports one.
    pub gender: Option<String>,
}

/// Parameters applied with [`SpeechBackend::set_params`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceParams {
    /// Voice id; `None` means the engine default.
    pub voice: Option<String>,
    /// Canonical rate; the backend maps it to its own scale.
    pub rate: Rate,
    /// Pitch offset.
    pub pitch: Pitch,
    /// Volume.
    pub volume: Volume,
}

/// An event from an engine about one utterance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawEvent {
    /// Audio for the utterance started.
    Started,
    /// A word is being (or is about to be) spoken.
    Word {
        /// Byte range of the word in `Utterance::text`.
        byte_range: Range<u32>,
        /// When the word sounds, in ms from the utterance's audio start, for
        /// engines with an audio clock (espeak-ng `audio_position`). The
        /// service schedules the highlight at this time plus the latency
        /// offset instead of firing on arrival.
        audio_ms: Option<u32>,
    },
    /// The utterance finished playing.
    Finished,
    /// The utterance was cancelled by `stop`.
    Cancelled,
    /// The engine failed on this utterance.
    Error(String),
}

/// Receives engine events. The service's sink drops events whose generation
/// is stale before they reach the service logic.
pub trait EventSink {
    /// Delivers one event for utterance `id`.
    fn emit(&mut self, id: UtteranceId, event: RawEvent);
    /// False when `id` belongs to a stale generation; backends may use it to
    /// abandon work early.
    fn is_current(&self, id: UtteranceId) -> bool;
}

/// Speech failures.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SpeechError {
    /// The engine is not installed or failed to initialize.
    #[error("backend {0} unavailable: {1}")]
    Unavailable(BackendId, String),
    /// The backend does not support this operation.
    #[error("not supported by this backend: {0}")]
    Unsupported(&'static str),
    /// The engine reported an error.
    #[error("engine error: {0}")]
    Engine(String),
    /// Unknown voice id.
    #[error("unknown voice: {0}")]
    UnknownVoice(String),
    /// I/O failure (subprocess pipes, output files).
    #[error("i/o error: {0}")]
    Io(String),
    /// The speech thread has stopped.
    #[error("speech service stopped")]
    ServiceStopped,
}

/// One speech engine.
///
/// Threading: no `Send` bound. The service calls every method from its own
/// speech thread, on which the backend was created by a [`BackendFactory`].
///
/// Timing contract: [`speak`](Self::speak) starts speaking and returns
/// without waiting for audio to finish. Events may be emitted during `speak`
/// or later from [`poll`](Self::poll), which the service calls every few
/// milliseconds while speech is active. `Finished` or `Cancelled` ends every
/// utterance exactly once.
pub trait SpeechBackend {
    /// Stable id.
    fn id(&self) -> BackendId;
    /// What the engine can do.
    fn capabilities(&self) -> Caps;
    /// Installed voices.
    fn voices(&self) -> Result<Vec<Voice>, SpeechError>;
    /// Applies voice, rate, pitch, and volume. Takes effect for the next
    /// utterance, or immediately with `LIVE_RATE`.
    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError>;
    /// The rate the engine actually achieves, in wpm, for timer pacing.
    fn effective_wpm(&self) -> u16;
    /// Starts speaking `utterance`.
    fn speak(&mut self, utterance: &Utterance, sink: &mut dyn EventSink)
    -> Result<(), SpeechError>;
    /// Delivers pending events. Default: nothing to deliver.
    fn poll(&mut self, sink: &mut dyn EventSink) {
        let _ = sink;
    }
    /// Stops all speech immediately; pending utterances end with `Cancelled`.
    fn stop(&mut self);
    /// Pauses (requires `PAUSE`).
    fn pause(&mut self) -> Result<(), SpeechError> {
        Err(SpeechError::Unsupported("pause"))
    }
    /// Resumes after `pause` (requires `PAUSE`).
    fn resume(&mut self) -> Result<(), SpeechError> {
        Err(SpeechError::Unsupported("resume"))
    }
    /// Writes `text` as audio to `path` (requires `SYNTH_TO_FILE`).
    fn synthesize_to_file(&mut self, text: &str, path: &Path) -> Result<(), SpeechError> {
        let _ = (text, path);
        Err(SpeechError::Unsupported("synthesize_to_file"))
    }
    /// Plays a tone (requires `TONES`). Default: silently ignored.
    fn tone(&mut self, hz: f32, ms: u32) {
        let _ = (hz, ms);
    }
}

/// Creates a backend on the speech thread.
pub type BackendFactory =
    Box<dyn FnOnce() -> Result<Box<dyn SpeechBackend>, SpeechError> + Send + 'static>;

/// A backend the registry knows about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BackendInfo {
    /// Stable id.
    pub id: BackendId,
    /// Human-readable name.
    pub name: &'static str,
    /// Auto-selection priority; higher is tried first.
    pub priority: i32,
    /// Only used when chosen explicitly (never auto-selected).
    pub opt_in: bool,
    /// Compiled in and able to start on this machine.
    pub available: bool,
}
