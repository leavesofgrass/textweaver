//! Editing for textweaver.
//!
//! - [`Editor`]: a rope, a selection, and an undo stack of grouped
//!   [`Edit`](textweaver_core::Edit)s. Every Markdown command is one undo step.
//! - [`markdown`]: formatting commands as pure `(text, selection) -> Edit`
//!   functions, ported from `star/gui/mixin_authoring.py`.
//! - [`echo`]: what to speak while typing and moving.
//! - [`autosave`]: snapshot policy and the save-in-place rule.
//!
//! The editor never touches a `Document`; the app applies the same edits to
//! the document with `Document::apply` so markers and bookmarks shift.
//!
//! Owner: Agent C.

pub mod autosave;
pub mod echo;
pub mod markdown;
mod undo;

pub use undo::{Editor, Selection};
