//! Piper neural voices, run in-process on RTen (ADR-0023).
//!
//! [Piper](https://github.com/OHF-Voice/piper1-gpl) voices are VITS models
//! exported to ONNX. textweaver runs them with RTen, Robert Knight's
//! pure-Rust ONNX runtime, following `rten-examples/src/piper.rs`: no
//! ONNX Runtime, no C++ and no subprocess. (tract and candle cannot run
//! the VITS graphs yet; the Wave 3 pure-Rust research.)
//!
//! - [`config`]: a voice's `.onnx.json`.
//! - [`text`]: clauses, words, and the chunks synthesized one at a time.
//! - [`phonemes`]: eSpeak NG phonemes (the installed library or the
//!   pure-Rust port) to model input ids, tracking each word.
//! - [`model`]: the ONNX graph on RTen, with each phoneme's duration from
//!   the `w_ceil` tensor.
//! - [`synth`]: text to audio with word timing; rate and pitch; timing
//!   measurements.
//! - [`catalog`]: the voice list (`voices.json`), licences from each
//!   voice's `MODEL_CARD`.
//! - [`store`]: voices installed in the data folder.
//! - `download` (feature `download`): fetching a voice after the user
//!   confirms, each file checked against Hugging Face's hash.
//! - [`backend`]: the `piper` speech backend, and its registry entry.
//!
//! Features: `playback` (audio device), `espeak-lib` (phonemes from an
//! installed libespeak-ng), `espeak-rs` (the pure-Rust phonemizer with
//! English built in), `download`.

pub mod backend;
pub mod catalog;
pub mod config;
#[cfg(feature = "download")]
pub mod download;
pub mod model;
mod pace;
pub mod phonemes;
pub mod store;
pub mod synth;
pub mod text;

use std::path::{Path, PathBuf};

pub use backend::{BACKEND_ID, PiperBackend, PiperConfig, backend_description, factory, probe};
pub use catalog::{Catalog, CatalogVoice, Licence, LicenceKind};
pub use config::VoiceConfig;
pub use phonemes::{Phonemizer, PhonemizerChoice, phonemizer};
pub use store::{InstalledVoice, VoiceStore};
pub use synth::{Chunk, Measurement, Next, SynthParams, Synthesizer, Timing, Upcoming, measure};

/// Piper failures, each written to be read aloud.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PiperError {
    /// A file could not be read or written.
    #[error("{path}: {message}")]
    Io {
        /// The file.
        path: PathBuf,
        /// What went wrong.
        message: String,
    },
    /// A voice's settings file is not usable.
    #[error("{0}")]
    Config(String),
    /// The model could not be loaded or run.
    #[error("{0}")]
    Model(String),
    /// Text could not be turned into phonemes.
    #[error("phonemes: {0}")]
    Phonemes(String),
    /// The voice needs something this build does not have.
    #[error("{0}")]
    Unsupported(String),
    /// No voice by that name is installed.
    #[error("the Piper voice {0} is not installed")]
    NotInstalled(String),
    /// A download failed or did not match its published hash.
    #[error("download: {0}")]
    Download(String),
    /// Speech was stopped while the work waited (never shown: the
    /// utterance just ends).
    #[error("stopped")]
    Stopped,
}

impl PiperError {
    /// An I/O error about `path`.
    pub fn io(path: &Path, e: std::io::Error) -> Self {
        PiperError::Io {
            path: path.to_owned(),
            message: e.to_string(),
        }
    }
}
