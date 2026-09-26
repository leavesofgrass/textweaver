//! Audio and subtitle export (ADR-0011): a document read to WAV through any
//! backend that can synthesize to a file, converted with ffmpeg when present
//! (MP3, M4B with chapters from headings), with SRT and WebVTT cues from the
//! narration's sentence timings.
//!
//! Owner: Agent B. Wave 2 skeleton.

/// This crate's name, so the skeleton has one public item.
pub const CRATE: &str = env!("CARGO_PKG_NAME");
