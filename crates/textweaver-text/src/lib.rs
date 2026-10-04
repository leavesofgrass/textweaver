//! The textweaver document model (ADR-0002) and the pure operations on it.
//!
//! A [`Document`] is canonical text in a rope plus [`Marker`]s that annotate
//! ranges of it. Everything else in this crate is a pure function of a
//! document and a position: [`units`] (where is the word or sentence at a
//! position), [`navigate`](mod@navigate) (where is the next one), [`history`], [`search`],
//! and [`narrate`] (which utterances read a range aloud).
//!
//! Positions are [`CharPos`](textweaver_core::CharPos) throughout; units are
//! computed by segment iterators over the rope ([`units::Units`]), marker
//! lookups by per-kind index tables ([`Document::marker_index`]).

pub mod document;
pub mod history;
pub mod marker;
pub mod narrate;
pub mod navigate;
pub mod search;
pub mod slug;
pub mod units;

pub use document::{DisplayIndex, Document, DocumentData, DocumentMeta};
pub use history::History;
pub use marker::{HEADER_ROW_LABEL, Marker, MarkerIndex, MarkerTables};
pub use narrate::{
    BlockEnd, InlineSpeech, NarrationPolicy, ReadingPass, TableNarration, block_ends, plan,
    plan_with,
};
pub use navigate::{GoTo, NavOptions, NavTarget, ParseGoToError, go_to, go_to_checked, navigate};
pub use search::{SearchError, SearchQuery, find, find_all};
pub use units::{
    Units, first_unit, last_unit, next_unit, prev_unit, segments, segments_in, unit_at,
};

pub use textweaver_core as core;
