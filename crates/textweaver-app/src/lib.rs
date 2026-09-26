//! The textweaver application core, independent of any UI.
//!
//! [`App`] is the only owner of mutable state. Frontends translate input into
//! [`Command`]s, call [`App::dispatch`], and act on the returned [`Effect`]s
//! (redraw, quit, prompt, list). Speech status is pulled with
//! [`App::poll_speech`] from the frontend's event loop. Everything here is
//! testable without a terminal.
//!
//! What the app does, briefly:
//!
//! - **Reading**: continuous from the cursor, or one unit in place
//!   (character, word, sentence, line, selection); pause resumes from the
//!   last confirmed word (or wherever the cursor moved meanwhile); replay
//!   always reads.
//! - **Navigation**: sentence (with Star's "more than three words in" rule
//!   for previous), paragraph, heading (read and move-only variants), table,
//!   list, list item, link, chapter (section breaks, else level-1 headings;
//!   "more than five words in" for previous), caret by word, line, and page,
//!   document start and end, go-to.
//! - **History**: one rule, documented on the `nav` module: every jump of a
//!   sentence or larger, structural jump, find, go-to, and bookmark jump
//!   records its departure point once.
//! - **Speech Cursor**: line mode that reads one line per move, "blank" for
//!   empty lines, clamps without wrapping, and reads on with Enter.
//! - **Find**, **bookmarks**, **selection**, **rate / pitch / volume /
//!   speed presets**, **themes**, **keyboard help** and **command palette**
//!   generated from the keymap.
//! - **Persistence**: the reading position, history, and bookmarks are saved
//!   on quit and on switching documents, and restored on open (first word at
//!   or after the saved position, when `auto_resume` is on). Settings are
//!   saved on quit only if they changed.
//! - **Announcements**: every state change goes to the status line
//!   ([`App::status`], which doubles as the TUI's `StatusLineAnnouncer`) and
//!   the configured announcer, filtered by verbosity; with self-voicing they
//!   are also spoken, but never over document reading unless they are
//!   errors.
//!
//! Owner: Agent D.

mod app;
mod command;
mod goto;
mod help;
mod marks;
mod nav;
mod playback;
mod speech_cursor;
pub mod testing;
pub mod text_util;
mod view;
mod voice;

pub use app::{App, AppConfig, AppError, FindState, Mode, Session};
pub use command::{Command, Effect, PromptPurpose};
pub use goto::parse_go_to;
pub use help::{chords_text, help_entries, palette_matches, resolve_command};
pub use playback::Playback;
pub use view::{Highlight, HighlightKind, Viewport};

pub use textweaver_a11y as a11y;
pub use textweaver_core as core;
pub use textweaver_editor as editor;
pub use textweaver_formats as formats;
pub use textweaver_keymap as keymap;
pub use textweaver_speech as speech;
pub use textweaver_store as store;
pub use textweaver_text as text;
