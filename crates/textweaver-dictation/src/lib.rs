//! Voice typing (ADR-0013): a [`Dictation`] trait, a Whisper subprocess
//! backend ([`WhisperDictation`]: whisper.cpp, faster-whisper, or
//! openai-whisper), and the commands spoken while dictating
//! ([`apply_spoken_commands`]).
//!
//! A session goes: [`Dictation::start`] with a file or an audio capture;
//! for a capture, [`Dictation::stop`] ends the recording (Star's "Enter:
//! stop and transcribe"); [`Dictation::poll`] delivers
//! [`DictationEvent`]s: `Recording`, `Transcribing`, a `Partial` per
//! segment, then one of `Final`, `Failed`, or `Cancelled`.
//! [`Dictation::cancel`] (Star's Escape) ends a session at any point.
//!
//! Ported from `star/transcribe.py` and the transcription mixins. Star ran
//! Whisper in-process through Python; textweaver runs a Whisper program,
//! so no Python is needed with whisper.cpp.
//!
//! **In-process Whisper** (feature `rten`, ADR-0023): [`RtenDictation`]
//! runs onnx-community's int8 Whisper models on RTen, the pure-Rust ONNX
//! runtime, with silence skipped by the earshot voice detector. The
//! Whisper programs stay as the fallback. Feature `mic` adds
//! [`MicCapture`], the default microphone through rodio.
//!
//! Owners: Agent J (the trait and the subprocess backend), Agent W3f
//! (in-process Whisper and the microphone).

#[cfg(any(feature = "rten", feature = "mic"))]
pub mod audio;
mod capture;
mod commands;
pub mod engine;
#[cfg(feature = "mic")]
mod mic;
#[cfg(feature = "rten")]
mod onnx_patch;
#[cfg(feature = "rten")]
mod rten_dictation;
#[cfg(feature = "rten")]
pub mod rten_whisper;
mod transcript;
#[cfg(any(feature = "rten", feature = "mic"))]
pub mod vad;
mod whisper;

use std::path::PathBuf;

pub use capture::{AudioCapture, BufferCapture, Pcm, WHISPER_SAMPLE_RATE};
pub use commands::{apply_spoken_commands, command_phrases};
pub use engine::{DetectedEngine, WhisperEngine, detect};
#[cfg(feature = "mic")]
pub use mic::MicCapture;
#[cfg(feature = "rten")]
pub use rten_dictation::{RtenConfig, RtenDictation, SessionTimings};
pub use transcript::{
    DictationEvent, Segment, Transcript, WHISPER_MODELS, WhisperModel, format_timestamp,
};
pub use whisper::{WhisperConfig, WhisperDictation};

/// Where a session's speech comes from.
pub enum DictationInput {
    /// An audio file to transcribe.
    File(PathBuf),
    /// Live recording, from `start` until `stop`.
    Capture(Box<dyn AudioCapture>),
}

impl std::fmt::Debug for DictationInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DictationInput::File(p) => f.debug_tuple("File").field(p).finish(),
            DictationInput::Capture(_) => f.write_str("Capture(..)"),
        }
    }
}

/// What a dictation backend is doing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DictationState {
    /// No session.
    #[default]
    Idle,
    /// Recording from a capture.
    Recording,
    /// Transcribing.
    Transcribing,
}

/// Voice typing.
pub trait Dictation: Send {
    /// The backend's name for people ("whisper.cpp").
    fn name(&self) -> &str;

    /// Starts a session. Fails when one is running, or when it cannot start
    /// at all (a missing file or model); failures later in the session
    /// arrive as [`DictationEvent::Failed`].
    fn start(&mut self, input: DictationInput) -> Result<(), DictationError>;

    /// Ends the input: stops recording and starts transcribing. Does
    /// nothing when not recording (a file already has all its input).
    fn stop(&mut self) -> Result<(), DictationError>;

    /// Abandons the session: the recording is discarded or the
    /// transcription stopped. A [`DictationEvent::Cancelled`] follows, and
    /// nothing else from that session.
    fn cancel(&mut self);

    /// The events since the last poll. Never blocks.
    fn poll(&mut self) -> Vec<DictationEvent>;

    /// What the backend is doing.
    fn state(&self) -> DictationState;
}

/// Dictation failures.
#[derive(Debug, thiserror::Error)]
pub enum DictationError {
    /// No Whisper program was found.
    #[error(
        "Speech recognition needs Whisper, which was not found. Install whisper.cpp (whisper-cli), faster-whisper (whisper-ctranslate2), or OpenAI Whisper (whisper), or set TEXTWEAVER_WHISPER to the program."
    )]
    NoWhisper,
    /// whisper.cpp's model file is missing.
    #[error(
        "The Whisper model \"{model}\" was not found. Download {file} into one of: {}", display_dirs(.searched)
    )]
    ModelNotFound {
        /// The model size.
        model: String,
        /// The file looked for.
        file: String,
        /// The folders searched.
        searched: Vec<PathBuf>,
    },
    /// A session is already running.
    #[error("Dictation is already running")]
    Busy,
    /// Stop was asked of a capture that was not recording.
    #[error("Not recording")]
    NotRecording,
    /// The microphone or other capture failed.
    #[error("Recording failed: {0}")]
    Capture(String),
    /// A program could not be started.
    #[error("Could not start {program}: {message}")]
    Spawn {
        /// The program.
        program: PathBuf,
        /// Why.
        message: String,
    },
    /// A file could not be read or written.
    #[error("{path}: {source}")]
    Io {
        /// The file.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
}

fn display_dirs(dirs: &[PathBuf]) -> String {
    if dirs.is_empty() {
        return "the folder given".to_owned();
    }
    dirs.iter()
        .map(|d| d.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
