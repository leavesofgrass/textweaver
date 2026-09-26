//! Playback: the sample feed, the playback clock, and the outputs.
//!
//! These live in the shared engine host
//! ([`textweaver_enginehost::audio`]) and are re-exported here unchanged:
//! the backend pushes the host's PCM into a [`Feed`], an output pulls from
//! it and counts what it consumed (the playback clock), and pausing stops
//! the clock with the audio. [`AudioOutput::Null`] consumes samples on a
//! timer without making a sound (tests).

pub use textweaver_enginehost::audio::{AudioOutput, Feed, Player};
