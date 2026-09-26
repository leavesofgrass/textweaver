//! Keyboard vocabulary for textweaver (ADR-0006).
//!
//! - [`ActionId`]: every user command, with a category, a help string, and
//!   default chords per [`Frontend`] and [`Platform`].
//! - [`KeyChord`]: parse, format, speak, and match keys, including what a
//!   terminal can and cannot deliver.
//! - [`Keymap`]: defaults plus user overrides keyed by action id, looked up
//!   by [`Layer`] (global chords, browse-mode single keys, Speech Cursor,
//!   edit mode), with conflict detection.
//! - [`Preset`]: named sets of changes to the defaults (`[keyboard]
//!   preset`), such as the screen-reader preset.
//! - [`help`]: the in-app keyboard help and `docs/keyboard.md`, generated
//!   from the same table by `cargo xtask keyboard`.
//!
//! Owner: Agent C.

mod action;
mod chord;
pub mod help;
mod keymap;
mod preset;

pub use action::{ActionId, Category};
pub use chord::{ChordError, Key, KeyChord, Modifiers};
pub use help::{HelpEntry, HelpSection, keyboard_markdown, preset_markdown};
pub use keymap::{Binding, Conflict, Frontend, Keymap, Layer, Platform};
pub use preset::Preset;
