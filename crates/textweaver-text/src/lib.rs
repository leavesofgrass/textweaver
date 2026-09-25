//! The textweaver document model (ADR-0002) and the pure operations on it.
//!
//! A [`Document`] is canonical text in a rope plus [`Marker`]s that annotate
//! ranges of it. Everything else in this crate is a pure function of a
//! document and a position: [`units`] (where is the word or sentence at a
//! position), [`navigate`](mod@navigate) (where is the next one), [`history`], [`search`],
//! and [`narrate`] (which utterances read a range aloud).
//!
//! Owner: Agent A. Phase 0 bodies are deliberately naive placeholders that
//! compile and behave sensibly on plain text; Agent A replaces them.

pub mod document;
pub mod history;
pub mod marker;
pub mod narrate;
pub mod navigate;
pub mod search;
pub mod units;

pub use document::{DisplayIndex, Document, DocumentMeta};
pub use history::History;
pub use marker::{Marker, MarkerIndex};
pub use narrate::{NarrationPolicy, plan};
pub use navigate::{GoTo, NavOptions, NavTarget, go_to, navigate};
pub use search::{SearchError, SearchQuery, find, find_all};

pub use textweaver_core as core;
