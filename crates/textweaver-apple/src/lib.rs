//! Apple speech for textweaver (ADR-0008).
//!
//! Two macOS backends over Apple's system voices, which include
//! ETI-Eloquence (Reed, Shelley, Rocko, ...):
//!
//! - `nsspeech`: `NSSpeechSynthesizer`; the system plays audio; word events
//!   on arrival; the most responsive.
//! - `avspeech`: `AVSpeechSynthesizer` writing into buffers; each word's
//!   sample offset comes from the running sample count; textweaver plays the
//!   audio, so word events carry `audio_ms`.
//!
//! On other platforms the crate is empty and [`available`] is false.
//!
//! Owner: Agent F.

/// True when this build can use Apple speech (macOS only).
pub fn available() -> bool {
    cfg!(target_os = "macos")
}

/// Identifier of the preferred default voice: Eloquence Reed.
pub const DEFAULT_VOICE: &str = "com.apple.eloquence.en-US.Reed";
