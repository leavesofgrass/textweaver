//! Speech for textweaver (ADR-0003, ADR-0004).
//!
//! - [`SpeechBackend`]: one engine. Not `Send`: engines such as AVSpeech,
//!   WinRT, and espeak-ng have thread affinity, so the service builds the
//!   backend on its own thread from a `Send` [`BackendFactory`].
//! - [`SpeechService`]: owns the speech thread, the queue, the generation
//!   counter, and the playback clock; turns engine events into
//!   [`SpeechStatus`] updates with document positions.
//! - [`normalize`]: the transform chain that turns source text into spoken
//!   text while composing [`OffsetMap`](textweaver_core::OffsetMap)s.
//! - [`pacing`]: highlight timing when an engine has no word events.
//!
//! This crate depends only on `textweaver-core`: it takes
//! [`Utterance`](textweaver_core::Utterance)s, never documents.
//!
//! Owner: Agent B. Phase 0 provides the contract, a null backend, and a
//! minimal service that reports sentence-level positions.

pub mod backend;
pub mod backends;
pub mod normalize;
pub mod pacing;
pub mod queue;
pub mod service;

pub use backend::{
    BackendFactory, BackendId, BackendInfo, Caps, EventSink, RawEvent, SpeechBackend, SpeechError,
    Voice, VoiceParams,
};
pub use service::{Earcon, SayMode, ServiceConfig, SpeechService, SpeechStatus};

pub use textweaver_core as core;
