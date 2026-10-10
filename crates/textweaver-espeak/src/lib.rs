//! eSpeak NG in a helper program (beta 1).
//!
//! textweaver can run eSpeak NG two ways. The in-process backend
//! (`textweaver_speech::backends::espeak`, the `espeak` feature) loads
//! libespeak-ng into textweaver itself and lets it play the audio. This
//! crate runs it instead in its own helper program,
//! `textweaver-espeak-host`, beside the Eloquence, DECtalk, and SAPI 5
//! hosts and speaking the same engine-host protocol (ADR-0012,
//! [`protocol`]): the host has libespeak-ng synthesize into memory and
//! streams the audio and each word's position back; [`EspeakHostBackend`]
//! plays the audio in textweaver and raises each word event when that
//! word is heard.
//!
//! Two things follow. Any program can use any installed eSpeak NG,
//! whatever the architecture of either: the host is built for the
//! library's (an x86 host for a 32-bit library, the x64 host for the x64
//! installer's library under an ARM64 program; see [`discovery`]). And an
//! engine crash ends the helper, not the reader: the utterance reports an
//! error and the next one starts a new helper.
//!
//! The library is found as the in-process backend finds it, the
//! components folder first (`textweaver_store::components_dir`), so a copy
//! placed there works either way.
//!
//! Wiring (`textweaver-engines`): [`backend_description`], [`probe`], and
//! [`factory`] make the registry entry; the `[speech.espeak] helper`
//! setting decides between this and the in-process backend.

pub mod backend;
pub mod discovery;
pub mod host;
pub mod protocol;

use std::path::PathBuf;
use std::time::Duration;

pub use backend::{EspeakHostBackend, STALL_TIMEOUT};
pub use discovery::{HOST_NAME, HOST_NAME_X86};
pub use textweaver_enginehost::audio::AudioOutput;
pub use textweaver_speech::backends::espeak::RATE_RANGE;
use textweaver_speech::{BackendFactory, BackendId, BackendInfo, Caps};

/// The backend id: the same as the in-process backend's, since both are
/// eSpeak NG and only one is registered at a time.
pub const BACKEND_ID: BackendId = "espeak";

/// Automatic-selection priority, the in-process backend's.
pub const PRIORITY: i32 = 50;

/// Environment variable naming the host executable.
pub const HOST_ENV: &str = "TEXTWEAVER_ESPEAK_HOST";

/// What the backend can do (plus `TONES` when it plays to the device).
/// textweaver plays the audio, so it can also pause natively, play
/// silence between utterances, and raise word events as they are heard.
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
pub struct EspeakHostConfig {
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
    /// More arguments for the host (tests).
    pub host_args: Vec<std::ffi::OsString>,
}

impl EspeakHostConfig {
    /// The default configuration, with the output the in-process backend
    /// would use: none (a silent output timed like real playback) when
    /// `TEXTWEAVER_ESPEAK_OUTPUT=virtual`, else the audio device.
    pub fn from_env() -> Self {
        let virtual_output = std::env::var("TEXTWEAVER_ESPEAK_OUTPUT")
            .is_ok_and(|v| v.eq_ignore_ascii_case("virtual"));
        EspeakHostConfig {
            output: if virtual_output {
                AudioOutput::Null { speed: 1.0 }
            } else {
                AudioOutput::default()
            },
            ..EspeakHostConfig::default()
        }
    }
}

/// True when a libespeak-ng file and a host for its architecture are
/// found for `config` (nothing is loaded or started).
pub fn probe(config: &EspeakHostConfig) -> bool {
    if config.fake_engine {
        return !discovery::host_candidates(config, None).is_empty();
    }
    discovery::choose_library(&discovery::library_candidates())
        .is_some_and(|c| !discovery::host_candidates(config, c.arch).is_empty())
}

/// The registry entry without the availability probe (`available` is
/// false).
pub fn backend_description() -> BackendInfo {
    BackendInfo {
        id: BACKEND_ID,
        name: "eSpeak NG",
        priority: PRIORITY,
        opt_in: false,
        available: false,
        caps: CAPS | Caps::TONES,
    }
}

/// A factory that starts the host and creates an [`EspeakHostBackend`]
/// on the speech thread.
pub fn factory(config: EspeakHostConfig) -> BackendFactory {
    Box::new(move || Ok(Box::new(EspeakHostBackend::new(config)?) as _))
}
