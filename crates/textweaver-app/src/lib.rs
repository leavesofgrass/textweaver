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
//!   saved as soon as they change (see below).
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
//! - `highlight.lead_words` shifts only the drawn highlight, counted so
//!   Star's default of 1 is the word being heard ([`App::highlight_lead`]);
//!   the TUI now honors it too (Star's TUI ignored it).
//! - The position is also saved every 30 seconds while it moves
//!   ([`App::tick`]), and settings are saved as soon as they change.
//!
//! # Editing, notes, and JSON-RPC (wave 2)
//!
//! - **Edit mode** (Ctrl+E) edits the source text with typing echo,
//!   Markdown formatting, undo and redo, Save, Save As, New, Star's
//!   Save / Discard / Cancel decisions, autosave snapshots, and recovery at
//!   startup ([`App::offer_recovery`]); see the `edit` module notes for how
//!   positions move between the source and the canonical text.
//! - **Notes and highlights** ([`NoteCommand`], [`Note`],
//!   [`UserHighlight`]): add, list, jump, edit, delete, all announced; with
//!   bookmark rename and delete from the bookmark list.
//! - **[`rpc`]**: JSON-RPC 2.0 over any reader and writer, which
//!   `tw serve --stdio` runs (ADR-0015). Actions that ask first (quit,
//!   delete note) take `confirm`.
//!
//! # Wave 2 wiring (Agent D3)
//!
//! - **Speech status** is matched by the `ReadingGeneration` each `read`
//!   returns; anything from another reading is dropped. Capability changes
//!   (a voice without word events, pitch, or volume) are announced.
//! - **Notes and highlights** are the store's typed `DocState::notes` and
//!   `DocState::highlights`, moved across edits by `DocState::shift`.
//! - **Library** (`open_library`): folder documents and recent files;
//!   opening records the bookshelf, and positions sync through library
//!   folders' sidecars under `reading.sync_conflict_policy`.
//! - **Settings** reach the engines and the speech service
//!   ([`speech_registry_for`], [`service_config`]); `[keyboard]
//!   character_keys` is applied at startup and toggled with F9.
//!
//! # Screen readers (Agent P2c)
//!
//! `[accessibility] mode` (self-voicing, screen-reader, hybrid; cycled with
//! Alt+Shift+A) decides whether each message, echo, caret move, and piece
//! of read text is spoken, shown on the status line, or both, so a screen
//! reader that reads the status line does not repeat textweaver's voice.
//! See the `access` module and `docs/screen-readers.md`.
//!
//! # Authoring and navigation (Agent P2b)
//!
//! - **Structure while editing** (`structure`): the Markdown source is
//!   parsed with pulldown-cmark's source offsets, so markers sit on the
//!   text being written and are parsed again when typing pauses; the same
//!   markers carry positions between the source and the canonical text.
//!   [`App::wait_for_structure`] waits for a background parse.
//! - **Lists** (`lists`): the outline (Alt+O) and the citation picker
//!   filter as you type ([`Command::FilterList`], [`App::list_filter`]);
//!   spelling, find-and-replace, and template lists.
//! - **Citations** (`citations`), **export and preview** (`publish`;
//!   both need the `publish` cargo feature, on by default; without it the
//!   commands say they are not in this build),
//!   **spelling** (`spell`), **tables** (`tables`), **links and
//!   footnotes** (`links`), **find and replace one at a time**
//!   (`replace`), **templates** (`templates`, [`local_date`]).
//! - Exports and reference lookups run on other threads and finish in
//!   [`App::tick`]; [`App::wait_for_background`] waits for them in tests.
//!   Files and addresses open through [`App::set_launcher`] (the system's
//!   default program otherwise, only in a session that keeps files).
//!
//! # The app core for the GUI (Wave 3, Agent W3a; ADR-0024)
//!
//! - **[`window`]**: [`DocWindow`], about [`WINDOW_UNITS`] UTF-16 units of
//!   text around the focus, paragraph-aligned, which slides while reading
//!   and recentres on jumps, with control offsets in [`Units`];
//!   [`Session::revision`] says when the text changed.
//! - **[`list_model`]**: the list shown ([`App::list_model`]) and the prompt
//!   open ([`App::prompt_model`]), driven by [`Command::ListKey`],
//!   [`Command::ListFocus`], and [`Command::PromptKey`], for every frontend.
//! - **[`wake`]**: [`App::set_waker`], rung by the speech thread, the writer,
//!   and background jobs; [`App::tick_interval`].
//! - **[`Command::ReplaceRange`]**: edits made in a native text control.
//! - **[`settings_schema`]**: [`SettingsSchema`] from the store's own keys,
//!   [`App::set_setting`], and the settings screen (the Settings action).
//! - **Off the input thread**: [`opening`] (large files open in the
//!   background), settings saves on the writer, the misspelling count after
//!   a save, and [`App::start_speech_in_background`].
//!
//! Owner: Agent D.

mod access;
mod actions;
pub mod align;
mod app;
mod authoring;
mod authoring_state;
mod batch;
pub mod browse;
// In-reader export, preview, and citations: the full modules with the
// `publish` feature, stand-ins that say "not in this build" without it.
mod announce;
#[cfg(feature = "publish")]
mod citations;
#[cfg(not(feature = "publish"))]
#[path = "lean/citations.rs"]
mod citations;
mod colors;
mod command;
mod dictation;
pub mod disk;
mod edit;
mod export;
mod extra;
mod find_scan;
mod frame_cache;
mod goto;
#[cfg(feature = "grammar")]
mod grammar;
mod help;
mod language;
mod library;
mod links;
#[cfg(feature = "lint")]
pub mod lint;
pub mod list_model;
mod lists;
pub mod logfile;
mod marks;
mod math_display;
mod math_explore;
mod mdline;
pub mod menu;
mod nav;
mod notes;
pub mod opening;
mod pages;
pub mod path_complete;
mod playback;
#[cfg(feature = "publish")]
pub mod preview_server;
#[cfg(not(feature = "publish"))]
#[path = "lean/preview_server.rs"]
mod preview_server;
#[cfg(feature = "publish")]
mod publish;
#[cfg(not(feature = "publish"))]
#[path = "lean/publish.rs"]
mod publish;
mod reading_aids;
mod relocate;
mod replace;
mod restart;
pub mod rpc;
pub mod settings_io;
pub mod settings_schema;
mod speech_cursor;
mod spell;
mod status;
mod structure;
mod study;
mod summary;
mod tables;
mod tasks;
mod templates;
pub mod testing;
pub mod text_util;
mod themes;
mod view;
mod voice;
pub mod voice_manager;
pub mod wake;
pub mod window;
mod words;
mod writer;
mod writes;

pub use access::{
    SENTENCE_GAP, STATUS_TEXT_LIMIT, access_mode_from_setting, access_mode_setting, digit_row,
    keymap_preset, sentence_duration,
};
pub use app::{App, AppConfig, AppError, FindState, Mode, Session};
pub use authoring::osc52;
pub use authoring_state::{ClientFactory, Launcher, open_with_system};
pub use colors::{COLOR_CHOICES, COLOR_SETTINGS, MarkColors, is_color_setting};
pub use command::{CaretMove, Command, Confirm, Effect, NoteCommand, PromptPurpose};
pub use export::{SubtitlePlan, subtitle_plan};
pub use extra::{extra_bindings, extra_chords, extra_lookup};
pub use goto::parse_go_to;
pub use help::{
    action_help, category_title, chords_text, chords_text_in, help_entries, key_text, named_key,
    named_key_in, palette_matches, palette_matches_in, resolve_command, resolve_command_in,
    short_chords_text, spoken_chord, spoken_key, spoken_text, written_text,
};
pub use list_model::{ListKey, ListModel, PromptKey, PromptModel};
pub use math_explore::MathMove;
#[cfg(feature = "publish")]
pub use notes::{NotesRecords, export_notes, notes_references};
pub use notes::{UserHighlight, parse_tags};
pub use opening::{
    open_failure_message, open_failure_message_in, open_failure_reason, open_failure_reason_in,
};
pub use playback::{Playback, load_options, narration_policy};
pub use restart::SpeechStarter;
pub use settings_schema::{Setting, SettingKind, SettingsSchema};
pub use templates::local_date;
pub use textweaver_engines::{
    CODE_FACTORY_LIBRARY, apple_preference, dectalk_config, eci_config, piper_config, sapi_config,
    service_config, speech_registry, speech_registry_for,
};
pub use textweaver_store::Note;
pub use view::{Highlight, HighlightKind, Viewport};
pub use wake::{Waker, channel_waker};
pub use window::{DocWindow, Units, WINDOW_UNITS, WindowChange};
pub use words::system_language;

pub use reading_aids::{RSVP_STEP, RSVP_WINDOW};
pub use textweaver_a11y as a11y;
pub use textweaver_aids as aids;
#[cfg(feature = "publish")]
pub use textweaver_cite as cite;
pub use textweaver_core as core;
#[cfg(feature = "dictation")]
pub use textweaver_dictation as voice_typing;
pub use textweaver_editor as editor;
pub use textweaver_engines as engines;
pub use textweaver_engines::apple;
pub use textweaver_engines::eci;
pub use textweaver_engines::piper;
pub use textweaver_formats as formats;
pub use textweaver_keymap as keymap;
pub use textweaver_lexicon as lexicon;
pub use textweaver_speech as speech;
pub use textweaver_store as store;
pub use textweaver_summary as summaries;
pub use textweaver_text as text;
pub use textweaver_theme as theme;
