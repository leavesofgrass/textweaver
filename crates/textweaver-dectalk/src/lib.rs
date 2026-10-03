//! DECtalk for textweaver (ADR-0021).
//!
//! DECtalk is the classic formant synthesizer ("Perfect Paul"); Star
//! supported it, and screen-reader users know its nine speakers well. It is
//! proprietary: textweaver ships none of it, never downloads it, and never
//! tests against the community source build. It uses a DECtalk the user
//! installed and licensed, found by [`discovery`]:
//! `TEXTWEAVER_DECTALK_LIBRARY`, then the backend option (a
//! `[speech.dectalk] library` setting), then the usual install folders.
//!
//! The library is loaded at run time with `libloading`, never linked, and
//! always in a separate host process, `textweaver-dectalk-host`, built for
//! the library's architecture (DECtalk for Windows is usually a 32-bit DLL:
//! `textweaver-dectalk-host-x86.exe`). The host has DECtalk synthesize into
//! memory with an `[:index mark]` before every word and streams the PCM and
//! each mark's sample offset over the shared engine-host protocol
//! (ADR-0012, [`protocol`]). [`DectalkBackend`] plays the audio in the main
//! process and turns the offsets into audio-clock word events, so the
//! highlight follows the word being heard, as with Eloquence.
//!
//! Wiring (for the application and the backend registry):
//! - [`backend_info`] describes the backend (id `"dectalk"`, priority
//!   [`PRIORITY`], below SAPI's; available when a DECtalk library and a
//!   host that can run it are found);
//! - [`factory`] builds it on the speech thread;
//! - [`discovery::diagnose`] explains which library was chosen and why.

pub mod backend;
pub mod discovery;
pub mod host;
pub mod protocol;
pub mod voices;
pub mod words;

use std::path::PathBuf;
use std::time::Duration;

pub use backend::{DectalkBackend, STALL_TIMEOUT, Synthesis};
pub use discovery::{HOST_NAME, HOST_NAME_X86};
pub use textweaver_enginehost::audio::AudioOutput;
pub use textweaver_enginehost::wav;
use textweaver_speech::{BackendFactory, BackendId, BackendInfo, Caps};
pub use voices::Speaker;

/// The backend id.
pub const BACKEND_ID: BackendId = "dectalk";

/// Automatic-selection priority: below ETI-Eloquence (1000) and SAPI
/// (500), above the built-in engines. DECtalk is chosen automatically
/// only where neither is available; users who want it pick it.
pub const PRIORITY: i32 = 300;

/// Environment variable naming the DECtalk library.
pub const LIBRARY_ENV: &str = "TEXTWEAVER_DECTALK_LIBRARY";
/// Environment variable naming the host executable.
pub const HOST_ENV: &str = "TEXTWEAVER_DECTALK_HOST";

/// What the backend can do (plus `TONES` when it plays to the device).
/// DECtalk's own text rules are not declared as native normalization:
/// textweaver's normalization runs first, as for the built-in engines.
pub const CAPS: Caps = Caps::WORD_EVENTS
    .union(Caps::AUDIO_CLOCK)
    .union(Caps::PAUSE)
    .union(Caps::PITCH)
    .union(Caps::VOLUME)
    .union(Caps::SYNTH_TO_FILE)
    .union(Caps::PLAYBACK_EVENTS)
    .union(Caps::SILENCE);

/// Backend configuration.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DectalkConfig {
    /// The DECtalk library, tried after `TEXTWEAVER_DECTALK_LIBRARY` and
    /// before the usual install folders (see [`discovery`]).
    pub library: Option<PathBuf>,
    /// The host executable; `None` searches (see
    /// [`discovery::host_candidates`]).
    pub host: Option<PathBuf>,
    /// Where audio goes.
    pub output: AudioOutput,
    /// Run the host with its fake engine (tests only).
    pub fake_engine: bool,
    /// How long the engine may go silent while it owes audio before it is
    /// treated as hung; `None` uses [`STALL_TIMEOUT`] (10 s).
    pub stall_timeout: Option<Duration>,
    /// More arguments for the host (tests: the fake engine's
    /// `--start-delay-ms`).
    pub host_args: Vec<std::ffi::OsString>,
}

/// The DECtalk library to load: the first DECtalk among the candidates,
/// starting with `TEXTWEAVER_DECTALK_LIBRARY` (see [`discovery`]).
pub fn library_path() -> Option<PathBuf> {
    let candidates = discovery::library_candidates(None, &discovery::Places::current());
    discovery::choose_library(&candidates)
        .ok()
        .map(|c| c.candidate.path)
}

/// Host executables that could run the configured library (see
/// [`discovery::host_candidates`]).
pub fn host_candidates(config: &DectalkConfig) -> Vec<PathBuf> {
    if config.fake_engine {
        return discovery::host_candidates(config, None);
    }
    discovery::diagnose(config).hosts
}

/// Describes the backend for the registry. `available` means a DECtalk
/// library and a host for its architecture were found (the library's
/// header and exports are read, but it is not loaded); the engine itself
/// starts in [`factory`], which reports
/// [`SpeechError::Unavailable`](textweaver_speech::SpeechError) if it
/// cannot.
pub fn backend_info() -> BackendInfo {
    backend_info_for(&DectalkConfig::default())
}

/// [`backend_info`] for a configuration (a library named in the settings).
pub fn backend_info_for(config: &DectalkConfig) -> BackendInfo {
    BackendInfo {
        available: probe(config),
        ..backend_description()
    }
}

/// True when a DECtalk library and a host for its architecture are found
/// for `config`: the probe behind [`backend_info_for`].
pub fn probe(config: &DectalkConfig) -> bool {
    let d = discovery::diagnose(config);
    d.library.is_ok() && !d.hosts.is_empty()
}

/// [`backend_info`] without the availability probe (`available` is false):
/// what a registry needs at registration, before it probes once.
pub fn backend_description() -> BackendInfo {
    BackendInfo {
        id: BACKEND_ID,
        name: "DECtalk",
        priority: PRIORITY,
        opt_in: false,
        available: false,
        caps: CAPS | Caps::TONES,
    }
}

/// A factory that starts the host and creates a [`DectalkBackend`] on the
/// speech thread.
pub fn factory(config: DectalkConfig) -> BackendFactory {
    Box::new(move || Ok(Box::new(DectalkBackend::new(config)?) as _))
}
