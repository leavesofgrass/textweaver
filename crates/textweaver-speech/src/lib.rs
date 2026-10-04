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
//! - [`queue`]: generations, lookahead, cancellation by id, and the timed
//!   gap that stands in for a structural pause.
//! - [`pauses`]: pauses at the ends of headings, paragraphs and list items,
//!   as silence or a timed gap.
//! - [`backends`]: `null`, `recording` (a test double, always compiled),
//!   `espeak` (feature `espeak`), `omnivox` (feature `omnivox`),
//!   `speechd` (feature `speechd`: speech-dispatcher with an index mark
//!   before every word), and the extensible [`BackendRegistry`] with star's
//!   selection rules. The out-of-process engines (Eloquence, SAPI5,
//!   DECtalk) and Apple's voices live in their own crates and are
//!   registered by the app.
//!
//! This crate depends only on `textweaver-core`: it takes
//! [`Utterance`](textweaver_core::Utterance)s, never documents.

pub mod backend;
pub mod backends;
pub mod normalize;
pub mod pacing;
pub mod pauses;
pub mod queue;
pub mod service;
pub mod voices;
pub mod wav;

pub use backend::{
    BackendFactory, BackendId, BackendInfo, Caps, EventSink, FileSynthesis, RawEvent,
    SpeechBackend, SpeechError, Voice, VoiceParams, WordTiming,
};
pub use backends::{
    BackendRegistry, NullBackend, RecordingBackend, RecordingHandle, RecordingMode, Selection,
    forget_probes, resolve_preferred_voice, resolve_voice,
};
pub use normalize::{NormalizeConfig, Pipeline, TableMode};
pub use pacing::{Clock, FakeClock, PacingConfig, SystemClock};
pub use pauses::{PauseAt, PauseConfig, PauseKind, PausePlan};
pub use service::{
    Earcon, FirstAudio, FirstAudioStamp, ReadingGeneration, SayMode, ServiceConfig, ServiceCore,
    SpeechService, SpeechStatus, Waker,
};
pub use voices::{VoiceCache, VoiceList};

pub use textweaver_core as core;
