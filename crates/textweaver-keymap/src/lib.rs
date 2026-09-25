//! Keyboard vocabulary for textweaver (ADR-0006).
//!
//! - [`ActionId`]: every user command, with a category, a help string, and
//!   default chords per [`Frontend`] and [`Platform`].
//! - [`KeyChord`]: parse, format, and match keys, including terminal keys.
//! - [`Keymap`]: defaults plus user overrides keyed by action id, looked up
//!   by [`Layer`] (global chords, browse-mode single keys, Speech Cursor,
//!   edit mode), with conflict detection.
//!
//! `docs/keyboard.md` and the in-app help are generated from this crate by
//! `cargo xtask keyboard`.
//!
//! Owner: Agent C.

mod action;
mod chord;
mod keymap;

pub use action::{ActionId, Category};
pub use chord::{ChordError, Key, KeyChord, Modifiers};
pub use keymap::{Binding, Conflict, Frontend, Keymap, Layer, Platform};
