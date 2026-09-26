//! Windows SAPI5 voices for textweaver (ADR-0009).
//!
//! Owner: Agent G.

pub mod host;
pub mod protocol;
pub mod voices;
pub mod wav;
pub mod words;

/// True when this build can use SAPI5 (Windows only).
pub fn available() -> bool {
    cfg!(windows)
}
