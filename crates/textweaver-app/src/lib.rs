//! The textweaver application core, independent of any UI.
//!
//! [`App`] is the only owner of mutable state. Frontends translate input into
//! [`Command`]s, call [`App::dispatch`], and act on the returned [`Effect`]s
//! (redraw, quit, prompt). Speech status is pulled with [`App::poll_speech`]
//! from the frontend's event loop. Everything here is testable without a
//! terminal.
//!
//! Owner: Agent D. Phase 0 opens documents, moves by the text crate's
//! placeholder units, reads with the speech service, and quits.

mod app;
mod command;

pub use app::{App, AppConfig, AppError, Mode, Session};
pub use command::{Command, Effect, PromptPurpose};

pub use textweaver_a11y as a11y;
pub use textweaver_core as core;
pub use textweaver_editor as editor;
pub use textweaver_formats as formats;
pub use textweaver_keymap as keymap;
pub use textweaver_speech as speech;
pub use textweaver_store as store;
pub use textweaver_text as text;
