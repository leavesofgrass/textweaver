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
//!   errors. While reading, navigation feedback goes to the status line
//!   only: the reading itself is what the user hears.
//!
//! # Verbosity
//!
//! | Level | What is announced |
//! |---|---|
//! | Low | Errors; boundaries ("No next heading.", "End of document."); the content reached by a navigation (a preview of the sentence, the heading text); explicit requests (position, rate, pitch, volume, theme, bookmarks, find results, prompts) |
//! | Normal | Low, plus structure labels in navigation ("Heading level 2: ..."), state changes ("Paused.", "Stopped.", "Cancelled.", "Speech Cursor off."), and search clearing |
//! | High | Normal, plus line and percentage in every navigation message, the document title and mode in "where am I", scrolling, and "Done reading." |
//!
//! # Deliberate differences from Star
//!
//! - One history rule for every frontend (Star's TUI and GUI recorded
//!   different events, and the TUI recorded bookmark and chapter jumps
//!   twice); the live position is remembered on the first Back so Forward
//!   returns to it.
//! - Paragraph and heading navigation start from the reading position, not
//!   the top of the viewport; next paragraph at the end says so instead of
//!   clamping silently.
//! - Bookmarks go to the first word at or after their position (Star's GUI
//!   rule, not the TUI's closest word); bookmark and find navigation wrap
//!   and announce it.
//! - Caret moves by word and line stop reading and speak the new word or
//!   line (Star kept reading and paused caret-following for three seconds).
//! - Resume re-reads from the last confirmed word on every backend.
//! - Speech Cursor lines are canonical-text lines (ADR-0002), not wrapped
//!   display lines.
//! - Chapters are section breaks, else level-1 headings (Star's chapter
//!   indices were always 0).
//! - Not yet: the highlight lead/lag setting (`highlight.lead_words`) is
//!   not applied; highlights follow the speech service's positions exactly.
//!
//! Owner: Agent D.

mod app;
mod backends;
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
pub use backends::speech_registry;
pub use command::{Command, Effect, PromptPurpose};
pub use goto::parse_go_to;
pub use help::{chords_text, help_entries, palette_matches, resolve_command};
pub use playback::Playback;
pub use view::{Highlight, HighlightKind, Viewport};

pub use textweaver_a11y as a11y;
pub use textweaver_core as core;
pub use textweaver_eci as eci;
pub use textweaver_editor as editor;
pub use textweaver_formats as formats;
pub use textweaver_keymap as keymap;
pub use textweaver_speech as speech;
pub use textweaver_store as store;
pub use textweaver_text as text;
