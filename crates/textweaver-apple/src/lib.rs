//! Apple speech for textweaver (ADR-0008).
//!
//! Two macOS backends over Apple's system voices, which include
//! ETI-Eloquence (Reed, Shelley, Rocko, Sandy, Flo, Eddy, Grandma, Grandpa):
//!
//! - `nsspeech` ([`NsSpeechBackend`]): `NSSpeechSynthesizer`; the system
//!   plays the audio; word events arrive as each word is spoken; the most
//!   responsive.
//! - `avspeech` ([`AvSpeechBackend`]): `AVSpeechSynthesizer` writing into
//!   buffers; each word's sample offset comes from the running sample count;
//!   textweaver plays the audio, so word events carry `audio_ms` and pause
//!   and resume are exact.
//!
//! Both are created and driven on the speech service's thread, which pumps
//! that thread's run loop from `poll` (ADR-0003); neither needs the main
//! thread. The default voice is Eloquence Reed ([`DEFAULT_VOICE`]) when
//! installed.
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
pub use macos::avspeech::{AvSpeechBackend, Output, Synthesis};
#[cfg(target_os = "macos")]
pub use macos::nsspeech::NsSpeechBackend;
#[cfg(target_os = "macos")]
pub use macos::runloop::{is_main_thread, pump_main_loop, run_main_loop_until};

use textweaver_speech::{BackendFactory, BackendInfo};

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
        name: "Apple speech, NSSpeechSynthesizer (system voices, fastest response)",
        priority: 80,
        opt_in: false,
        available: available(),
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
        assert_eq!(nsspeech_info().id, "nsspeech");
        assert_eq!(avspeech_info().id, "avspeech");
    }
}
