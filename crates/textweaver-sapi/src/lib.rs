//! Windows SAPI5 voices for textweaver (ADR-0009).
//!
//! Each voice runs in a host process, `textweaver-sapi-host`, built for the
//! voice's architecture: x64 for 64-bit voices (Microsoft David, Zira) and
//! x86 for 32-bit-only voices (VW Paul, Kate, James; eSpeak SAPI). The host
//! synthesizes into a memory stream and reports each word boundary with its
//! audio stream offset; this crate's backend plays the audio and turns the
//! offsets into audio-clock word events.
//!
//! On other platforms the crate is empty and [`available`] is false.
//!
//! Owner: Agent G.

/// True when this build can use SAPI5 (Windows only).
pub fn available() -> bool {
    cfg!(windows)
}
