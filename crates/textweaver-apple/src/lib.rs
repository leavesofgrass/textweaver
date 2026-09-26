//! Apple speech for textweaver (ADR-0008).
//!
//! Two macOS backends over Apple's system voices, which include
//! ETI-Eloquence (Reed, Shelley, Rocko, Sandy, Flo, Eddy, Grandma, Grandpa):
//!
//! - `nsspeech` (`NsSpeechBackend`): Apple's classic engine, the one behind
//!   `NSSpeechSynthesizer`, driven through its C API (the Speech Synthesis
//!   Manager); the system plays the audio; word events arrive as each word
//!   is spoken; the most responsive (first word about 50 ms after `speak`
//!   with Reed).
//! - `avspeech` (`AvSpeechBackend`): `AVSpeechSynthesizer` writing into
//!   buffers; each word's sample offset comes from the running sample count;
//!   textweaver plays the audio, so word events carry `audio_ms` and pause
//!   and resume are exact.
//!
//! Threads (probes 5 and 6 in `tools/avspeech-spike/`): both backends are
//! created and driven on the speech service's thread (ADR-0003).
//! `nsspeech` needs nothing else: its callbacks run on the engine's own
//! threads. `NSSpeechSynthesizer` itself was not usable there, because it
//! delivers its callbacks only through the main run loop. `avspeech`'s
//! callbacks arrive through the main dispatch queue, so it works only while
//! the application's main thread runs its run loop (`run_main_loop_until`,
//! or `pump_main_loop` on each tick); otherwise each utterance ends after
//! five seconds with an error saying so.
//!
//! The default voice is Eloquence Reed ([`DEFAULT_VOICE`]) when installed.
//!
//! Wiring: [`backends`] lists what this build offers and [`factory`] creates
//! a backend by id, for the speech registry. [`normalizes_natively`] says
//! which voices need no textweaver normalization (Eloquence).
//!
//! On other platforms the crate holds only the platform-independent pieces
//! (range mapping, rate tables, voice naming, WAV and AIFF handling, each
//! tested everywhere); [`available`] is false, [`backends`] is empty, and
//! [`factory`] returns `None`.
//!
//! Owner: Agent F.

pub mod audio;
pub mod range;
pub mod rate;
pub mod voices;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::avspeech::{AvSpeechBackend, Output, Synthesis, SynthesisEnd};
#[cfg(target_os = "macos")]
pub use macos::nsspeech::NsSpeechBackend;
#[cfg(target_os = "macos")]
pub use macos::runloop::{is_main_thread, pump_main_loop, run_main_loop_until};

/// Main run-loop helpers for platforms other than macOS, where there is no
/// run loop to service: they do nothing and return false, so applications
/// can call them without `cfg`.
#[cfg(not(target_os = "macos"))]
mod no_runloop {
    use std::time::Duration;

    /// Always false: there is no Cocoa main thread off macOS.
    pub fn is_main_thread() -> bool {
        false
    }

    /// Does nothing off macOS; returns false.
    pub fn pump_main_loop(max: Duration) -> bool {
        let _ = max;
        false
    }

    /// Does nothing off macOS; returns false at once without calling
    /// `done`.
    pub fn run_main_loop_until(done: impl FnMut() -> bool) -> bool {
        let _ = done;
        false
    }
}

#[cfg(not(target_os = "macos"))]
pub use no_runloop::{is_main_thread, pump_main_loop, run_main_loop_until};

use textweaver_speech::{BackendFactory, BackendInfo, Caps};

pub use voices::normalizes_natively;

/// True when this build can use Apple speech (macOS only).
pub fn available() -> bool {
    cfg!(target_os = "macos")
}

/// Identifier of the preferred default voice: Eloquence Reed.
pub const DEFAULT_VOICE: &str = "com.apple.eloquence.en-US.Reed";

/// Backend id of the `NSSpeechSynthesizer` backend.
pub const NSSPEECH_ID: &str = "nsspeech";

/// Backend id of the `AVSpeechSynthesizer` backend.
pub const AVSPEECH_ID: &str = "avspeech";

/// Registry entry for the `nsspeech` backend.
pub fn nsspeech_info() -> BackendInfo {
    BackendInfo {
        id: NSSPEECH_ID,
        name: "Apple speech, classic engine (system voices, fastest response)",
        priority: 80,
        opt_in: false,
        available: available(),
        caps: Caps::WORD_EVENTS | Caps::PAUSE | Caps::PITCH | Caps::VOLUME | Caps::SYNTH_TO_FILE,
    }
}

/// Registry entry for the `avspeech` backend.
pub fn avspeech_info() -> BackendInfo {
    BackendInfo {
        id: AVSPEECH_ID,
        name: "Apple speech, AVSpeechSynthesizer (system voices, exact highlighting)",
        priority: 70,
        opt_in: false,
        available: available(),
        caps: Caps::WORD_EVENTS
            | Caps::AUDIO_CLOCK
            | Caps::PAUSE
            | Caps::PITCH
            | Caps::VOLUME
            | Caps::SYNTH_TO_FILE,
    }
}

/// The Apple backends compiled into this build: both on macOS, none
/// elsewhere.
pub fn backends() -> Vec<BackendInfo> {
    if available() {
        vec![nsspeech_info(), avspeech_info()]
    } else {
        Vec::new()
    }
}

/// A factory for backend `id` (`"nsspeech"` or `"avspeech"`), which creates
/// the backend on the speech thread. `None` for other ids and on platforms
/// other than macOS.
pub fn factory(id: &str) -> Option<BackendFactory> {
    #[cfg(target_os = "macos")]
    {
        match id {
            NSSPEECH_ID => Some(Box::new(|| {
                Ok(Box::new(NsSpeechBackend::new()?) as Box<dyn textweaver_speech::SpeechBackend>)
            })),
            AVSPEECH_ID => Some(Box::new(|| {
                Ok(Box::new(AvSpeechBackend::new(Output::Speakers)?)
                    as Box<dyn textweaver_speech::SpeechBackend>)
            })),
            _ => None,
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = id;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_matches_platform() {
        let all = backends();
        if cfg!(target_os = "macos") {
            assert_eq!(all.len(), 2);
            assert!(all.iter().all(|b| b.available && !b.opt_in));
        } else {
            assert!(all.is_empty());
            assert!(factory(NSSPEECH_ID).is_none());
            assert!(factory(AVSPEECH_ID).is_none());
        }
        assert!(factory("espeak").is_none());
        if !cfg!(target_os = "macos") {
            assert!(!pump_main_loop(std::time::Duration::ZERO));
            assert!(!run_main_loop_until(|| true));
            assert!(!is_main_thread());
        }
        assert_eq!(nsspeech_info().id, "nsspeech");
        assert_eq!(avspeech_info().id, "avspeech");
    }
}
