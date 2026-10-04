//! Editing for textweaver.
//!
//! - [`Editor`]: a rope, a selection, and an undo stack of grouped
//!   [`Edit`](textweaver_core::Edit)s. Every Markdown command is one undo
//!   step; typing and deleting coalesce into word-sized steps.
//! - [`markdown`]: formatting commands as pure `(text, selection) -> edits`
//!   functions, ported from `star/gui/mixin_authoring.py` with its bugs
//!   fixed (the commands toggle; see the table in that module).
//! - [`find`]: find and replace in edit mode, without star's offset bugs.
//! - [`echo`]: what to speak while typing and moving.
//! - [`autosave`]: snapshot policy and files, recovery scan, the save rule,
//!   and format-preserving atomic saves.
//! - [`session`]: read and edit mode for one document with star's Save /
//!   Discard / Cancel flow, Save As adoption, and autosave.
//!
//! The editor never touches a `Document`; the app applies the same edits to
//! the document with `Document::apply` so markers and bookmarks shift.
//!
//! Owner: Agent C.

pub mod autosave;
pub mod echo;
pub mod find;
pub mod markdown;
pub mod session;
mod undo;

pub use find::FindOptions;
pub use markdown::{FormatError, Formatted, MarkdownOp};
pub use session::{
    Choice, DocInfo, EditSession, LeaveOutcome, PendingSnapshot, SaveOutcome, SaveRequest,
    SaveStart, SessionError, SnapshotOp,
};
pub use undo::{Editor, SavePoint, Selection, UndoLimits};
