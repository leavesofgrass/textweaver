//! Shared leaf types for textweaver.
//!
//! Every other crate in the workspace depends on this one and on nothing else
//! from the workspace unless the dependency graph in ADR-0001 allows it. The
//! types here are the contract between the text model, the speech service, the
//! editor, persistence, and the frontends:
//!
//! - [`CharPos`] and [`CharRange`]: canonical, persisted document positions in
//!   Unicode scalar values (chars), compatible with Star's offsets.
//! - [`Unit`] and [`MarkerKind`]: what a reading or navigation command moves by.
//! - [`OffsetMap`]: how spoken text maps back to source text, so highlighting is
//!   exact after normalization (ADR-0005).
//! - [`Edit`] and [`EditOutcome`]: text changes and how positions move across them.
//! - [`Rate`], [`Pitch`], [`Volume`]: engine-independent voice parameters (ADR-0004).
//! - [`Utterance`]: one chunk of text handed to the speech service.
//! - Small preference enums shared by settings, speech, and accessibility.
//! - [`fs`]: the one atomic file write every crate saves with.
//! - [`process`]: finding, starting and reading other programs, with one
//!   rule for `PATH`, no console windows, and one text-decoding rule.

mod edit;
mod error;
pub mod fs;
mod offset_map;
mod pos;
mod prefs;
pub mod process;
mod unit;
mod utterance;
mod voice;

pub use edit::{Edit, EditOutcome};
pub use error::CoreError;
pub use offset_map::{OffsetMap, Span, SpanKind, SpokenBuilder};
pub use pos::{Bias, CharPos, CharRange, Direction};
pub use prefs::{CapsIndication, HighlightGranularity, PunctuationLevel, Verbosity};
pub use unit::{MarkerKind, Unit};
pub use utterance::{Utterance, UtteranceId, UtteranceKind};
pub use voice::{Pitch, Rate, Volume};

/// Convenience result type for fallible core operations.
pub type Result<T, E = CoreError> = std::result::Result<T, E>;
