//! ETI-Eloquence for textweaver (ADR-0007).
//!
//! Eloquence's engine is the ECI library the user installed: Code Factory's
//! `eci.dll` on Windows (32-bit), OpenEVV's `eci.dll` (x86_64), or Voxin's
//! `libibmeci.so` on Linux (64-bit). It is loaded at run time, never
//! linked, and always runs in a separate host process,
//! `textweaver-eci-host`, which synthesizes into a buffer and reports each
//! index mark with its sample offset over a framed pipe protocol
//! ([`protocol`]). [`EciBackend`] plays the audio in the main process and
//! turns those offsets into audio-clock word events, so the highlight
//! follows the exact word being heard.
//!
//! Wiring (for the application and the backend registry):
//! - [`backend_info`] describes the backend (id `"eci"`, the highest
//!   automatic priority, available when a library and a matching host are
//!   found);
//! - [`factory`] builds it on the speech thread;
//! - [`NORMALIZES_NATIVELY`]: Eloquence expands numbers, dates, times,
//!   currency, and abbreviations itself, so the speech service should skip
//!   its own overlapping normalization transforms for this backend;
//! - [`discovery::diagnose`] explains which library was chosen and why, for
//!   `tw backends` and error messages.
//!
//! Finding things: the library and the host are described in
//! [`discovery`]; the pronunciation dictionaries in [`dictionaries`].

pub mod audio;
mod backend;
pub mod calibration;
pub mod dictionaries;
pub mod discovery;
pub mod host;
pub mod language;
pub mod protocol;
pub mod voices;
pub mod wav;
pub mod words;

use std::path::PathBuf;

pub use audio::AudioOutput;
pub use backend::{EciBackend, STALL_TIMEOUT, Synthesis};
pub use dictionaries::Dictionaries;
pub use discovery::{HOST_NAME, HOST_NAME_X86, Product};
use textweaver_speech::{BackendFactory, BackendId, BackendInfo};
pub use voices::VoiceParam;

/// The backend id.
pub const BACKEND_ID: BackendId = "eci";

/// Automatic-selection priority: above every other engine, so Eloquence is
/// chosen whenever it is installed (ADR-0007).
pub const PRIORITY: i32 = 1000;

/// Eloquence normalizes numbers, abbreviations, dates, times, and currency
/// itself ("9:30 a.m.", "Dr. Smith"); the speech service should skip its
/// overlapping transforms for this backend and keep punctuation and
/// pronunciation handling.
pub const NORMALIZES_NATIVELY: bool = true;

/// Set to `1` to let discovery search Code Factory's default location
/// (ADR-0007: off by default, because an installed copy is not
/// necessarily a licensed one).
pub const CODE_FACTORY_ENV: &str = "TEXTWEAVER_ECI_CODE_FACTORY";

/// Environment variable naming the ECI library.
pub const LIBRARY_ENV: &str = "TEXTWEAVER_ECI_LIBRARY";
/// Environment variable naming the host executable.
pub const HOST_ENV: &str = "TEXTWEAVER_ECI_HOST";

/// Where the ECI library is installed on this platform, if it is: the first
/// existing standard location (see [`discovery`]; `TEXTWEAVER_ECI_LIBRARY`
/// is not consulted here).
pub fn default_library_path() -> Option<PathBuf> {
    let places = discovery::Places {
        env_library: None,
        ..discovery::Places::current()
    };
    discovery::library_candidates(None, &places)
        .into_iter()
        .find(|c| c.exists)
        .map(|c| c.path)
}

/// The ECI library to load: the first existing candidate, starting with
/// `TEXTWEAVER_ECI_LIBRARY` (see [`discovery`]).
pub fn library_path() -> Option<PathBuf> {
    discovery::library_candidates(None, &discovery::Places::current())
        .into_iter()
        .find(|c| c.exists)
        .map(|c| c.path)
}

/// Backend configuration.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EciConfig {
    /// The ECI library, tried after `TEXTWEAVER_ECI_LIBRARY` and before the
    /// standard install locations (see [`discovery`]).
    pub library: Option<PathBuf>,
    /// The host executable; `None` searches (see [`discovery::host_candidates`]).
    pub host: Option<PathBuf>,
    /// Engine sample rate in Hz (8000, 11025, or 22050); `None` keeps the
    /// engine's default (11025 Hz).
    pub sample_rate: Option<u32>,
    /// Where audio goes.
    pub output: AudioOutput,
    /// Run the host with its fake engine (tests only).
    pub fake_engine: bool,
    /// Community pronunciation dictionaries: on by default
    /// ([`Dictionaries::Auto`]), off, or from a directory.
    pub dictionaries: Dictionaries,
    /// How long the engine may go silent while it owes audio before it is
    /// treated as hung; `None` uses [`STALL_TIMEOUT`] (10 s).
    pub stall_timeout: Option<std::time::Duration>,
}

/// Host executables that could run the configured library (see
/// [`discovery::host_candidates`]).
pub fn host_candidates(config: &EciConfig) -> Vec<PathBuf> {
    if config.fake_engine {
        return discovery::host_candidates(config, None);
    }
    let d = discovery::diagnose(config);
    d.hosts
}

/// Describes the backend for the registry. `available` means a usable
/// library and a matching host executable were found (it reads the
/// library's header but does not load it); the engine itself starts in
/// [`factory`], which reports
/// [`SpeechError::Unavailable`](textweaver_speech::SpeechError) if it
/// cannot.
pub fn backend_info() -> BackendInfo {
    let d = discovery::diagnose(&EciConfig::default());
    BackendInfo {
        id: BACKEND_ID,
        name: "ETI-Eloquence",
        priority: PRIORITY,
        opt_in: false,
        available: d.library.is_ok() && !d.hosts.is_empty(),
    }
}

/// A factory that starts the host and creates an [`EciBackend`] on the
/// speech thread.
pub fn factory(config: EciConfig) -> BackendFactory {
    Box::new(move || Ok(Box::new(EciBackend::new(config)?) as _))
}
