//! The shared engine-host protocol and playback client (ADR-0012): one
//! framed PCM-plus-events protocol and one audio-clock playback client for
//! every speech engine that runs in a helper process (ECI, SAPI).
//!
//! Owner: Agent H. Wave 2 skeleton.

/// This crate's name, so the skeleton has one public item.
pub const CRATE: &str = env!("CARGO_PKG_NAME");
