//! Speech for textweaver (ADR-0003, ADR-0004, ADR-0005).
//!
//! - [`SpeechBackend`]: one engine. Not `Send`: engines such as AVSpeech,
//!   WinRT, and espeak-ng have thread affinity, so the service builds the
//!   backend on its own thread from a `Send` [`BackendFactory`].
//! - [`SpeechService`]: owns the speech thread, the queue, the generation
//!   counter, and the playback clock; turns engine events into
//!   [`SpeechStatus`] updates with document positions. Its logic is the
//!   thread-free [`ServiceCore`], which tests drive with a fake clock.
//! - [`normalize`]: the transform chain that turns source text into spoken
//!   text while composing [`OffsetMap`](textweaver_core::OffsetMap)s.
//! - [`pacing`]: highlight timing when an engine has no word events, and
//!   the clocks.
//! - [`queue`]: generations, lookahead, and cancellation by id.
//! - [`backends`]: `null`, `recording` (a test double, always compiled),
//!   `espeak` (feature `espeak`), `omnivox` (feature `omnivox`), and the
//!   extensible [`BackendRegistry`] with Star's selection rules.
//!
//! This crate depends only on `textweaver-core`: it takes
//! [`Utterance`](textweaver_core::Utterance)s, never documents.

pub mod backend;
pub mod backends;
pub mod normalize;
pub mod pacing;
pub mod queue;
pub mod service;
pub mod wav;

pub use backend::{
    BackendFactory, BackendId, BackendInfo, Caps, EventSink, FileSynthesis, RawEvent,
    SpeechBackend, SpeechError, Voice, VoiceParams, WordTiming,
};
pub use backends::{
    BackendRegistry, NullBackend, RecordingBackend, RecordingHandle, RecordingMode, Selection,
    resolve_preferred_voice, resolve_voice,
};
pub use normalize::{NormalizeConfig, Pipeline, TableMode};
pub use pacing::{Clock, FakeClock, PacingConfig, SystemClock};
pub use service::{
    Earcon, ReadingGeneration, SayMode, ServiceConfig, ServiceCore, SpeechService, SpeechStatus,
};

pub use textweaver_core as core;
