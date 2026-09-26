use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use textweaver_a11y::{Announcer, LogAnnouncer, Priority, StatusLineAnnouncer, Verbosity};
use textweaver_core::{CharPos, CharRange};
use textweaver_editor::autosave::RecoverySnapshot;
use textweaver_formats::{LoadError, Registry, Source};
use textweaver_keymap::{ActionId, Frontend, Keymap, Layer, Platform};
use textweaver_speech::{SayMode, SpeechService};
use textweaver_store::{
    Bookmark, DocKey, DocState, Note, Paths, Settings, SettingsStore, StateStore, StoreError,
};
use textweaver_text::{Document, History, SearchQuery};

use crate::command::{Command, Effect, NoteCommand, PromptPurpose};
use crate::edit::{AfterLeave, EditState, SaveThen};
use crate::notes::{UserHighlight, migrate_legacy_notes};
use crate::playback::{Playback, ReadKind, SpeechTrack};
use crate::text_util;
use crate::view::Viewport;

/// Interaction mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// Reading; single keys navigate.
    #[default]
    Browse,
    /// Speech Cursor: line-by-line reading.
    SpeechCursor,
    /// Editing text (wave 2).
    Edit,
    /// Typing a search.
    Find,
    /// Typing a command name.
    Command,
    /// Typing a go-to target.
    GoTo,
    /// Typing a path to open.
    Open,
    /// Answering any other prompt (save as, table size, note text, ...).
    Prompt,
}

impl Mode {
    /// The keymap layer for this mode.
    pub fn layer(self) -> Layer {
        match self {
            Mode::Browse => Layer::Browse,
            Mode::SpeechCursor => Layer::SpeechCursor,
            Mode::Edit => Layer::Edit,
            Mode::Find | Mode::Command | Mode::GoTo | Mode::Open | Mode::Prompt => Layer::Global,
        }
    }

    /// True while a prompt owns the keyboard.
    pub fn is_prompt(self) -> bool {
        matches!(
            self,
            Mode::Find | Mode::Command | Mode::GoTo | Mode::Open | Mode::Prompt
        )
    }

    /// A short spoken and displayed name.
    pub fn name(self) -> &'static str {
        match self {
            Mode::Browse => "Browse",
            Mode::SpeechCursor => "Speech Cursor",
            Mode::Edit => "Edit",
            Mode::Find => "Find",
            Mode::Command => "Command",
            Mode::GoTo => "Go to",
            Mode::Open => "Open",
            Mode::Prompt => "Prompt",
        }
    }

    pub(crate) fn for_prompt(p: PromptPurpose) -> Mode {
        match p {
            PromptPurpose::Find => Mode::Find,
            PromptPurpose::GoTo => Mode::GoTo,
            PromptPurpose::Open => Mode::Open,
            PromptPurpose::CommandPalette => Mode::Command,
            _ => Mode::Prompt,
        }
    }
}

/// The last search and its matches.
///
/// A search counts every match but keeps at most 10,000 of them, those
/// around the cursor; stepping past the ones kept searches again (Phase 2:
/// a common word in a 10 MB file no longer costs tens of megabytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindState {
    /// The query as run.
    pub query: SearchQuery,
    /// The matches kept, in document order: match number
    /// [`first_index`](Self::first_index) and the ones after it.
    pub hits: Vec<CharRange>,
    /// Index into [`hits`](Self::hits) of the match the cursor is on.
    pub current: Option<usize>,
    /// How many matches the document has in all.
    pub total: usize,
    /// The number (from 0) of `hits[0]` among all matches.
    pub first_index: usize,
}

/// One open document and the reader's place in it.
#[derive(Debug)]
pub struct Session {
    /// The document.
    pub doc: Document,
    /// Its persistence key.
    pub key: DocKey,
    /// The cursor (reading position).
    pub cursor: CharPos,
    /// The selection, if any.
    pub selection: Option<CharRange>,
    /// Navigation history.
    pub history: History,
    /// Range currently highlighted as spoken.
    pub spoken: Option<CharRange>,
    /// The sentence containing the spoken range, for sentence highlighting.
    pub spoken_sentence: Option<CharRange>,
    /// Bookmarks, sorted by position.
    pub bookmarks: Vec<Bookmark>,
    /// The last search.
    pub find: Option<FindState>,
    /// The Speech Cursor line (0-based) while in Speech Cursor mode.
    pub speech_cursor_line: Option<usize>,
    /// Where the selection started (its fixed end).
    pub selection_anchor: Option<CharPos>,
    /// Sticky column for line moves.
    pub goal_column: Option<usize>,
    /// Title used in announcements and the title line.
    pub title: String,
    /// State loaded at open, kept so unknown keys survive the next save.
    pub saved: DocState,
    /// Notes, sorted by position.
    pub notes: Vec<Note>,
    /// User highlights, sorted by position.
    pub highlights: Vec<UserHighlight>,
    /// The open file's modification time and size when it was opened or
    /// last saved (`None` for documents not read from a file).
    pub disk: Option<crate::disk::FileStamp>,
    /// The document text's stamp, saved with its positions so a change
    /// made outside textweaver is noticed on the next open.
    pub(crate) text_stamp: Option<textweaver_store::TextStamp>,
}

impl Session {
    /// A session at the start of `doc`.
    pub fn new(doc: Document, key: DocKey, title: impl Into<String>, history_size: usize) -> Self {
        Session {
            doc,
            key,
            cursor: CharPos::ZERO,
            selection: None,
            history: History::with_capacity(history_size),
            spoken: None,
            spoken_sentence: None,
            bookmarks: Vec::new(),
            find: None,
            speech_cursor_line: None,
            selection_anchor: None,
            goal_column: None,
            title: title.into(),
            saved: DocState::default(),
            notes: Vec::new(),
            highlights: Vec::new(),
            disk: None,
            text_stamp: None,
        }
    }

    /// Percentage of the cursor through the document.
    pub fn percent(&self) -> u8 {
        text_util::percent(&self.doc, self.cursor)
    }

    /// The 0-based line of the cursor.
    pub fn line(&self) -> usize {
        text_util::line_of(&self.doc, self.cursor)
    }
}

/// Construction parameters.
pub struct AppConfig {
    /// Settings.
    pub settings: Settings,
    /// Key bindings.
    pub keymap: Keymap,
    /// The speech service.
    pub speech: SpeechService,
    /// Where to persist state; `None` keeps everything in memory (tests).
    pub paths: Option<Paths>,
    /// How announcements are delivered, in addition to the status line the
    /// app always keeps ([`App::status`]).
    pub announcer: Box<dyn Announcer>,
    /// Speak announcements through the speech service (self-voicing).
    pub self_voicing: bool,
    /// Name of the speech backend, for the title line.
    pub backend_name: String,
}

impl AppConfig {
    /// Defaults with silent speech, no persistence, and a logging announcer:
    /// the configuration tests start from.
    pub fn for_tests() -> Self {
        AppConfig {
            settings: Settings::default(),
            keymap: Keymap::defaults(Platform::current(), Frontend::Terminal),
            speech: SpeechService::null(),
            paths: None,
            announcer: Box::new(LogAnnouncer::default()),
            self_voicing: false,
            backend_name: "null".into(),
        }
    }
}

/// Application failures surfaced to the user.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The document could not be loaded.
    #[error(transparent)]
    Load(#[from] LoadError),
    /// State could not be saved.
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// What the last `ShowList` effect listed, so `Command::Choose` can act.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ListKind {
    Bookmarks,
    Actions(Vec<ActionId>),
    Info,
    Notes,
    Highlights,
    SaveChoice(AfterLeave),
    Recovery,
    /// The library: the documents listed, in order.
    Library(Vec<PathBuf>),
    /// The speech engine's voices: id and name, in order.
    Voices(Vec<(String, String)>),
}

/// The application: the only owner of mutable state.
pub struct App {
    pub(crate) session: Option<Session>,
    pub(crate) speech: SpeechService,
    pub(crate) announcer: Box<dyn Announcer>,
    pub(crate) status: StatusLineAnnouncer,
    pub(crate) keymap: Keymap,
    pub(crate) settings: Settings,
    pub(crate) settings_dirty: bool,
    pub(crate) paths: Option<Paths>,
    pub(crate) registry: Registry,
    pub(crate) mode: Mode,
    pub(crate) return_mode: Mode,
    pub(crate) list: Option<ListKind>,
    pub(crate) playback: Playback,
    pub(crate) pause_origin: Option<CharPos>,
    pub(crate) reading: ReadKind,
    pub(crate) track: SpeechTrack,
    /// Where continuous reading goes on when the planned window finishes.
    pub(crate) continue_from: Option<CharPos>,
    /// Where the text handed to the speech service ends.
    pub(crate) planned_end: Option<CharPos>,
    /// The backend's capabilities as last reported.
    pub(crate) speech_caps: textweaver_speech::Caps,
    pub(crate) view: Viewport,
    pub(crate) self_voicing: bool,
    pub(crate) backend_name: String,
    pub(crate) spoken_log: Vec<CharRange>,
    pub(crate) edit: Option<EditState>,
    pub(crate) save_then: Option<SaveThen>,
    pub(crate) suggested_path: Option<PathBuf>,
    pub(crate) replace_query: Option<String>,
    pub(crate) pending_item: Option<usize>,
    /// An action waiting for a yes or no ([`ActionId::needs_confirmation`]).
    pub(crate) pending_confirm: Option<ActionId>,
    /// A checked settings import and its file name, waiting for a yes or no.
    pub(crate) pending_import: Option<(textweaver_store::ImportPlan, String)>,
    pub(crate) recovery: Vec<(PathBuf, RecoverySnapshot)>,
    pub(crate) untitled: u32,
    pub(crate) last_position_save: Option<(Instant, CharPos)>,
    pub(crate) prompt_purpose: PromptPurpose,
    /// Reading positions synced through the library folders' sidecars.
    pub(crate) library_sync: textweaver_store::LibrarySync,
    /// Built-in and user themes (ADR-0020).
    pub(crate) themes: textweaver_theme::Registry,
    /// RSVP while it is showing.
    pub(crate) rsvp: Option<crate::reading_aids::RsvpState>,
    /// A question about the open file changing on disk, waiting for y or n.
    pub(crate) pending_disk: Option<crate::disk::DiskQuestion>,
    /// The user said yes to saving over a file changed on disk.
    pub(crate) overwrite_confirmed: bool,
    /// When the open file was last checked for changes on disk.
    pub(crate) last_disk_check: Option<Instant>,
    /// Writing the recovery snapshot failed and has not worked since.
    pub(crate) snapshot_trouble: bool,
    /// A note or highlight chosen for deletion in its list, waiting for y
    /// or n (deleting one at the cursor asks too).
    pub(crate) pending_list_delete: Option<(ListKind, usize)>,
    /// The voices shown by the last voice list, in list order.
    pub(crate) voice_list: Vec<textweaver_speech::Voice>,
    /// Text copied or cut, waiting for the frontend
    /// ([`App::take_clipboard`]).
    pub(crate) clipboard: Option<String>,
    /// The background writer: saves, snapshots, positions, and the disk
    /// check (crate::writer).
    pub(crate) writer: crate::writer::Writer,
    /// Saves on the writer, by id, with what follows each.
    pub(crate) pending_saves: Vec<(u64, SaveThen)>,
    /// A change-on-disk check is on the writer.
    pub(crate) disk_check_pending: bool,
}

impl App {
    /// Most highlight ranges kept by [`App::spoken_log`].
    pub const SPOKEN_LOG_LIMIT: usize = 4096;

    /// Creates the application.
    pub fn new(config: AppConfig) -> Self {
        let speech_caps = config.speech.capabilities();
        let library_sync = Self::make_library_sync(&config.settings);
        let mut keymap = config.keymap;
        keymap.set_character_keys(config.settings.keyboard.character_keys);
        let mut app = App {
            session: None,
            speech: config.speech,
            announcer: config.announcer,
            status: StatusLineAnnouncer::default(),
            keymap,
            settings: config.settings,
            settings_dirty: false,
            paths: config.paths,
            registry: Registry::with_builtins(),
            mode: Mode::Browse,
            return_mode: Mode::Browse,
            list: None,
            playback: Playback::Idle,
            pause_origin: None,
            reading: ReadKind::Continuous,
            track: SpeechTrack::default(),
            continue_from: None,
            planned_end: None,
            speech_caps,
            view: Viewport::default(),
            self_voicing: config.self_voicing,
            backend_name: config.backend_name,
            spoken_log: Vec::new(),
            edit: None,
            save_then: None,
            suggested_path: None,
            replace_query: None,
            pending_item: None,
            pending_confirm: None,
            pending_import: None,
            recovery: Vec::new(),
            untitled: 0,
            last_position_save: None,
            prompt_purpose: PromptPurpose::Find,
            library_sync,
            themes: textweaver_theme::Registry::builtin(),
            rsvp: None,
            pending_disk: None,
            overwrite_confirmed: false,
            last_disk_check: None,
            snapshot_trouble: false,
            pending_list_delete: None,
            voice_list: Vec::new(),
            clipboard: None,
            writer: crate::writer::Writer::spawn(),
            pending_saves: Vec::new(),
            disk_check_pending: false,
        };
        app.apply_voice_settings();
        app.load_themes();
        app
    }

    /// The open document, if any.
    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    /// The current mode.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// The action waiting for a yes or no, if any. While one is pending the
    /// frontend sends every key press as a [`Command::Confirm`].
    pub fn pending_confirmation(&self) -> Option<ActionId> {
        self.pending_confirm
    }

    /// True while a yes-or-no question is open: an action's
    /// ([`pending_confirmation`](Self::pending_confirmation)) or a settings
    /// import's. The frontend then sends every key press as a
    /// [`Command::Confirm`].
    pub fn confirmation_pending(&self) -> bool {
        self.pending_confirm.is_some()
            || self.pending_import.is_some()
            || self.pending_disk.is_some()
            || self.pending_list_delete.is_some()
    }

    /// Answers a pending confirmation.
    fn confirm(&mut self, answer: crate::command::Confirm) -> Vec<Effect> {
        use crate::command::Confirm;
        if self.pending_disk.is_some() {
            return self.confirm_disk(answer);
        }
        if self.pending_import.is_some() {
            return self.confirm_import(answer);
        }
        if let Some((kind, n)) = self.pending_list_delete.clone() {
            return match answer {
                Confirm::Yes => {
                    self.pending_list_delete = None;
                    self.list = Some(kind.clone());
                    match kind {
                        ListKind::Highlights => self.delete_highlight(n),
                        _ => self.delete_note(n),
                    }
                }
                Confirm::No => {
                    self.pending_list_delete = None;
                    self.tell("Kept.");
                    match kind {
                        ListKind::Highlights => self.list_highlights(),
                        _ => self.notes_command(NoteCommand::List),
                    }
                }
                Confirm::Repeat => {
                    self.tell(list_delete_question(&kind));
                    vec![Effect::Redraw]
                }
            };
        }
        let Some(a) = self.pending_confirm else {
            return vec![Effect::Redraw];
        };
        match answer {
            Confirm::Yes => {
                self.pending_confirm = None;
                self.run_action(a)
            }
            Confirm::No => {
                self.pending_confirm = None;
                self.tell("Cancelled.");
                vec![Effect::Redraw]
            }
            Confirm::Repeat => {
                self.tell(a.confirmation_prompt().unwrap_or("Press y or n."));
                vec![Effect::Redraw]
            }
        }
    }

    /// The key bindings.
    pub fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    /// The settings.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Where state is persisted, if anywhere.
    pub fn paths(&self) -> Option<&Paths> {
        self.paths.as_ref()
    }

    /// The status line: the latest announcement. Frontends draw it as their
    /// status line, which terminal screen readers read as it changes.
    pub fn status(&self) -> &StatusLineAnnouncer {
        &self.status
    }

    /// The status line text, or an empty string.
    pub fn status_text(&self) -> &str {
        self.status.current.as_deref().unwrap_or("")
    }

    /// Name of the speech backend in use.
    pub fn backend_name(&self) -> &str {
        &self.backend_name
    }

    /// The viewport: size and first visible line.
    pub fn viewport(&self) -> Viewport {
        self.view
    }

    /// Every range the reading highlight has covered, oldest first (capped at
    /// [`App::SPOKEN_LOG_LIMIT`]; cleared when a document opens). Frontends
    /// and tests use it to check highlighting against what was spoken.
    pub fn spoken_log(&self) -> &[CharRange] {
        &self.spoken_log
    }

    /// Announces `text` on the status line and through the announcer.
    ///
    /// Frontends use this for changes only they know about (a list item
    /// gaining focus, a help screen opening), so every change is announced
    /// the same way.
    pub fn announce(&mut self, text: &str, priority: Priority) {
        self.say_at(text, Verbosity::Low, priority);
    }

    /// Announces `text` after whatever is being announced, without
    /// interrupting it: the first item of a list after the list's
    /// introduction. The status line shows both.
    pub fn announce_queued(&mut self, text: &str, priority: Priority) {
        if text.is_empty() {
            return;
        }
        let shown = match self.status.current.as_deref() {
            Some(before) if !before.is_empty() => format!("{before} {text}"),
            _ => text.to_owned(),
        };
        self.status.announce(&shown, priority);
        self.announcer.announce(text, priority);
        let reading = matches!(self.playback, Playback::Reading);
        if self.self_voicing && (!reading || priority == Priority::Assertive) {
            self.speech.say(text, SayMode::Queue);
        }
    }

    /// Announces at a minimum verbosity. Spoken announcements never
    /// interrupt reading unless assertive; the status line always updates.
    pub(crate) fn say_at(&mut self, text: &str, min: Verbosity, priority: Priority) {
        let current = self.settings.speech.verbosity;
        if current < min || text.is_empty() {
            return;
        }
        self.status.announce(text, priority);
        self.announcer.announce(text, priority);
        let reading = matches!(self.playback, Playback::Reading);
        if self.self_voicing && (!reading || priority == Priority::Assertive) {
            self.speech.say(text, SayMode::Announce);
        }
    }

    /// Announces an essential message (errors, "no next heading").
    pub(crate) fn tell(&mut self, text: &str) {
        self.say_at(text, Verbosity::Low, Priority::Polite);
    }

    /// Announces a structural or state detail (Normal verbosity).
    pub(crate) fn note(&mut self, text: &str) {
        self.say_at(text, Verbosity::Normal, Priority::Polite);
    }

    /// Announces an error.
    pub(crate) fn error(&mut self, text: &str) {
        self.say_at(text, Verbosity::Low, Priority::Assertive);
    }

    /// Shows `text` on the status line only (used while reading, when the
    /// reading itself is the audible feedback).
    pub(crate) fn show(&mut self, text: &str) {
        self.status.announce(text, Priority::Polite);
        self.announcer.announce(text, Priority::Polite);
    }

    /// Opens a document and makes it current. The previous document's
    /// position is saved first.
    pub fn open(&mut self, path: &Path) -> Result<Vec<Effect>, AppError> {
        // Taken before reading, so a change made while loading is noticed.
        let stamp = crate::disk::FileStamp::of(path);
        let mut doc = self
            .registry
            .load(&Source::Path(path.to_owned()), &self.load_options())?;
        if doc.meta.path.is_none() {
            doc.meta.path = Some(path.to_owned());
        }
        let title = doc.meta.title.clone().unwrap_or_else(|| {
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string())
        });
        let key = DocKey::for_path(path);
        // The recent list and the bookshelf, on the writer.
        self.record_library_open(path, &title, &doc.meta.format);
        let effects = self.open_document(doc, key, title);
        if let Some(s) = self.session.as_mut() {
            s.disk = stamp;
        }
        Ok(effects)
    }

    /// Makes an already loaded document current (tests, in-memory sources).
    /// Saved state for `key` is restored when persistence is configured.
    ///
    /// Callers resolve unsaved edits first ([`Command::Open`] asks); as a
    /// safety net, edit mode is dropped here without saving, keeping its
    /// recovery snapshot.
    pub fn open_document(&mut self, doc: Document, key: DocKey, title: String) -> Vec<Effect> {
        if self.edit.take().is_some() {
            log::warn!("a document was opened over unsaved edit mode");
        }
        if self.session.is_some() {
            if let Err(e) = self.save_position() {
                log::warn!("cannot save position: {e}");
            }
            self.flush_library_sync();
        }
        self.stop_speech();
        self.rsvp = None;
        self.mode = Mode::Browse;
        self.return_mode = Mode::Browse;
        self.last_position_save = None;
        self.list = None;
        self.spoken_log.clear();
        let mut s = Session::new(doc, key, title, self.settings.reading.nav_history_size);
        s.text_stamp = Some(crate::relocate::text_stamp(&s.doc));
        // The state saves queued above (the previous document, or this one
        // on a reload) must be on disk before this reads them back.
        if !self.writer.flush(std::time::Duration::from_secs(2)) {
            log::warn!("the writer is slow; reading the saved state anyway");
        }
        let loaded = self.state_store().and_then(|store| store.load(&s.key));
        let resume = match s.doc.meta.path.clone() {
            Some(path) => self.resume_point(&path, loaded.as_ref()),
            None => loaded
                .as_ref()
                .map(|st| st.position)
                .filter(|p| *p > CharPos::ZERO)
                .map(|pos| crate::library::ResumePoint {
                    pos,
                    synced: false,
                    unresolved: false,
                }),
        };
        if let Some(store) = self.state_store()
            && let Some(mut state) = loaded
        {
            if migrate_legacy_notes(&mut state, &s.doc)
                && let Err(e) = store.save(&s.key, &state)
            {
                log::warn!("cannot save migrated notes: {e}");
            }
            for &h in &state.history {
                s.history.record(h.clamp_to(s.doc.len_chars()));
            }
            s.bookmarks = state.bookmarks.clone();
            s.bookmarks.sort_by_key(|b| b.pos);
            let len = s.doc.len_chars();
            for b in &mut s.bookmarks {
                b.pos = b.pos.clamp_to(len);
            }
            s.notes = state.notes.clone();
            for n in &mut s.notes {
                n.range = n.range.clamp_to(len);
            }
            s.notes.sort_by_key(|n| (n.range.start, n.range.end));
            s.highlights = state.highlights.clone();
            for h in &mut s.highlights {
                h.range = h.range.clamp_to(len);
            }
            s.highlights.retain(|h| !h.range.is_empty());
            s.highlights.sort_by_key(|h| (h.range.start, h.range.end));
            s.saved = state;
        }
        let mut resumed = None;
        if let Some(r) =
            resume.filter(|r| self.settings.reading.auto_resume && r.pos > CharPos::ZERO)
        {
            s.cursor = text_util::first_word_at_or_after(&s.doc, r.pos.clamp_to(s.doc.len_chars()));
            resumed = Some((text_util::percent(&s.doc, s.cursor), r));
        }
        let title = s.title.clone();
        self.session = Some(s);
        self.view.top_line = 0;
        self.scroll_to_cursor();
        let msg = match resumed {
            Some((p, r)) if r.synced => {
                format!("Opened {title}. Resumed at {p} percent, from another device.")
            }
            Some((p, r)) if r.unresolved => format!(
                "Opened {title}. Resumed at {p} percent. Another device is at a different place; kept this device's."
            ),
            Some((p, _)) => format!("Opened {title}. Resumed at {p} percent."),
            None => format!("Opened {title}."),
        };
        self.tell(&msg);
        if self.settings.speech.auto_play {
            self.read_from_cursor();
        }
        vec![Effect::Redraw]
    }

    pub(crate) fn state_store(&self) -> Option<StateStore> {
        self.paths.as_ref().map(|p| StateStore::new(p.state_dir()))
    }

    /// The position the reader is on: the spoken word while reading, else
    /// the cursor.
    pub fn reading_position(&self) -> Option<CharPos> {
        let s = self.session.as_ref()?;
        Some(match (&self.playback, s.spoken) {
            (Playback::Reading, Some(r)) => r.start,
            _ => s.cursor,
        })
    }

    /// Saves the reading position, history, bookmarks, notes, and
    /// highlights of the open document (a no-op without persistence, and
    /// while editing, when positions are in the source text; leaving edit
    /// mode saves them).
    ///
    /// The file is written by the background writer: this returns at once,
    /// and a failure is logged when the writer reports it (Phase 2). The
    /// `Result` is kept for callers written before; it is always `Ok`.
    pub fn save_position(&mut self) -> Result<(), AppError> {
        self.save_state(crate::writer::StateNote::Quiet);
        Ok(())
    }

    /// Changes settings from outside the command loop (a frontend's own
    /// dialog, such as the GUI's font chooser) and saves them at once, so
    /// the app's copy and the file agree and a later save cannot undo the
    /// change.
    pub fn update_settings(&mut self, change: impl FnOnce(&mut Settings)) -> Result<(), AppError> {
        change(&mut self.settings);
        self.settings_dirty = true;
        self.save_settings()
    }

    /// Saves settings if they changed since loading (explicit changes only).
    pub fn save_settings(&mut self) -> Result<(), AppError> {
        if !self.settings_dirty {
            return Ok(());
        }
        if let Some(paths) = &self.paths {
            SettingsStore::new(paths.clone()).save(&self.settings)?;
        }
        self.settings_dirty = false;
        Ok(())
    }

    /// Saves everything and stops speech: what quitting does. Waits for the
    /// background writer (at most ten seconds, saying so when it takes more
    /// than a moment). Safe to call more than once.
    pub fn shutdown(&mut self) {
        if let Err(e) = self.save_position() {
            log::warn!("cannot save position: {e}");
        }
        if let Err(e) = self.save_settings() {
            log::warn!("cannot save settings: {e}");
        }
        self.flush_library_sync();
        self.stop_speech();
        self.finish_writes();
    }

    /// Handles one command. Settings changed by it are saved at once.
    pub fn dispatch(&mut self, cmd: Command) -> Vec<Effect> {
        let edits_text = matches!(
            cmd,
            Command::Insert(_)
                | Command::DeleteBack
                | Command::DeleteForward
                | Command::MoveCaret { .. }
                | Command::Tick
                | Command::Resize { .. }
        );
        let effects = self.dispatch_inner(cmd);
        if self.edit.is_some() && !edits_text {
            // A navigation or search moved the cursor: the caret follows.
            self.sync_editor_caret();
        }
        if self.settings_dirty
            && let Err(e) = self.save_settings()
        {
            self.error(&format!("Could not save settings: {e}"));
        }
        self.send_snapshot_ops();
        effects
    }

    /// Opens `path` (after edit mode was resolved), announcing failures.
    pub(crate) fn dispatch_open(&mut self, path: &Path) -> Vec<Effect> {
        match self.open(path) {
            Ok(e) => e,
            Err(e) => {
                let name = path.display();
                self.error(&format!("Could not open {name}: {e}"));
                vec![Effect::Redraw]
            }
        }
    }

    fn dispatch_inner(&mut self, cmd: Command) -> Vec<Effect> {
        match cmd {
            Command::Open(path) => {
                self.leave_prompt();
                self.open_command(path)
            }
            Command::Action(a) => self.action(a),
            Command::Confirm(answer) => self.confirm(answer),
            Command::Insert(text) => self.insert(&text),
            Command::DeleteBack => self.delete(false),
            Command::DeleteForward => self.delete(true),
            Command::MoveCaret {
                by,
                direction,
                extend,
            } => self.move_caret(by, direction, extend),
            Command::Notes(c) => {
                self.leave_prompt();
                self.notes_command(c)
            }
            Command::DeleteItem(n) => self.delete_item(n),
            Command::MarkItem(n) => self.mark_item(n),
            Command::RenameItem(n) => self.rename_item(n),
            Command::Tick => self.tick(Instant::now()),
            Command::Find(pattern) => {
                self.leave_prompt();
                self.run_find(&pattern);
                vec![Effect::Redraw]
            }
            Command::GoTo(target) => {
                self.leave_prompt();
                self.go_to(target);
                vec![Effect::Redraw]
            }
            Command::Select(range) => {
                self.select(range);
                vec![Effect::Redraw]
            }
            Command::SetCursor(pos) => {
                self.set_cursor(pos);
                vec![Effect::Redraw]
            }
            Command::ExtendSelection(unit, dir) => {
                self.extend_selection(unit, dir);
                vec![Effect::Redraw]
            }
            Command::Answer(text) => self.answer(text),
            Command::Choose(n) => self.choose(n),
            Command::Resize { width, height } => {
                self.view.width = width;
                self.view.height = height;
                self.scroll_to_focus();
                vec![Effect::Redraw]
            }
            Command::Cancel => {
                if self.confirmation_pending() {
                    // Cancelling a question answers no.
                    return self.confirm(crate::command::Confirm::No);
                }
                if self.mode.is_prompt() || self.list.is_some() {
                    let list = self.list.take();
                    self.leave_prompt();
                    self.save_then = None;
                    self.suggested_path = None;
                    self.replace_query = None;
                    self.pending_item = None;
                    match list {
                        Some(ListKind::Recovery) => self.postpone_recovery(),
                        Some(ListKind::SaveChoice(_)) => self.tell("Still editing."),
                        _ => self.note("Cancelled."),
                    }
                }
                vec![Effect::Redraw]
            }
        }
    }

    /// How often the reading position is saved while the app runs.
    pub const POSITION_SAVE_INTERVAL: Duration = Duration::from_secs(30);

    /// Periodic housekeeping; call it from the event loop (a few times a
    /// second is plenty). Applies what the background writer finished
    /// ([`poll_writes`](Self::poll_writes)), queues the autosave snapshot
    /// while editing with unsaved changes, and queues the reading position
    /// every [`POSITION_SAVE_INTERVAL`](Self::POSITION_SAVE_INTERVAL) when
    /// it moved, so a crash loses little. Nothing here waits on the disk.
    pub fn tick(&mut self, now: Instant) -> Vec<Effect> {
        let mut effects = self.poll_writes();
        let rsvp_moved = self.rsvp_tick(now);
        if rsvp_moved {
            effects.push(Effect::Redraw);
        }
        let asked = self.disk_tick(now);
        if !asked.is_empty() {
            effects.extend(asked);
            return effects;
        }
        if self.edit.is_some() {
            self.autosave_tick(now);
            self.send_snapshot_ops();
            return effects;
        }
        let Some(pos) = self.reading_position() else {
            return effects;
        };
        let due = match self.last_position_save {
            None => {
                // Start the clock at the first tick after opening.
                self.last_position_save = Some((now, pos));
                false
            }
            Some((t, at)) => {
                at != pos && now.saturating_duration_since(t) >= Self::POSITION_SAVE_INTERVAL
            }
        };
        if due {
            if let Err(e) = self.save_position() {
                log::warn!("cannot save position: {e}");
            }
            self.last_position_save = Some((now, pos));
        }
        effects
    }

    /// Opens a prompt: switches mode and returns the effect that shows it.
    pub(crate) fn prompt(&mut self, purpose: PromptPurpose) -> Vec<Effect> {
        if !self.mode.is_prompt() {
            self.return_mode = self.mode;
        }
        self.mode = Mode::for_prompt(purpose);
        self.prompt_purpose = purpose;
        let label = purpose.label().to_owned();
        self.tell(&label);
        vec![Effect::Prompt { label, purpose }]
    }

    pub(crate) fn leave_prompt(&mut self) {
        if self.mode.is_prompt() {
            self.mode = self.return_mode;
        }
    }

    fn answer(&mut self, text: String) -> Vec<Effect> {
        let mode = self.mode;
        let purpose = self.prompt_purpose;
        self.leave_prompt();
        if mode == Mode::Prompt {
            return self.answer_prompt(purpose, &text);
        }
        match mode {
            Mode::Find => self.run_find(&text),
            Mode::GoTo => match crate::goto::parse_go_to(&text) {
                Some(t) => self.go_to(t),
                None => self.error(&format!(
                    "Not a go-to target: {text}. Type a line number, a percentage such as 50%, start, or end."
                )),
            },
            Mode::Open => {
                let trimmed = text.trim().trim_matches('"');
                if trimmed.is_empty() {
                    self.note("Cancelled.");
                } else {
                    return self.dispatch(Command::Open(trimmed.into()));
                }
            }
            Mode::Command => return self.run_named_command(&text),
            _ => {}
        }
        vec![Effect::Redraw]
    }

    /// Answers of the prompts in [`Mode::Prompt`].
    fn answer_prompt(&mut self, purpose: PromptPurpose, text: &str) -> Vec<Effect> {
        match purpose {
            PromptPurpose::SaveAs => return self.answer_save_as(text),
            PromptPurpose::TableSize => return self.answer_table(text),
            PromptPurpose::ImagePath => return self.answer_image(text),
            PromptPurpose::ExportSettings => return self.answer_export_settings(text),
            PromptPurpose::ImportSettings => return self.answer_import_settings(text),
            PromptPurpose::ReplaceFind => return self.answer_replace(text, false),
            PromptPurpose::ReplaceWith => return self.answer_replace(text, true),
            PromptPurpose::NoteText => self.add_note(text),
            PromptPurpose::EditNote => {
                if let Some(i) = self.pending_item.take() {
                    self.edit_note(i, text);
                }
            }
            PromptPurpose::RenameBookmark => {
                if let Some(i) = self.pending_item.take() {
                    self.rename_bookmark(i, text);
                }
            }
            PromptPurpose::Find
            | PromptPurpose::GoTo
            | PromptPurpose::Open
            | PromptPurpose::CommandPalette => {}
        }
        vec![Effect::Redraw]
    }

    fn choose(&mut self, n: usize) -> Vec<Effect> {
        match self.list.take() {
            Some(ListKind::Bookmarks) => self.go_to_bookmark(n),
            Some(ListKind::Actions(actions)) => {
                if let Some(&a) = actions.get(n) {
                    return self.action(a);
                }
            }
            Some(ListKind::Notes) => self.go_to_note(n, false),
            Some(ListKind::Highlights) => self.go_to_highlight(n),
            Some(ListKind::SaveChoice(after)) => return self.answer_save_choice(n, after),
            Some(ListKind::Recovery) => return self.answer_recovery(n),
            Some(ListKind::Library(paths)) => {
                if let Some(path) = paths.get(n).cloned() {
                    return self.open_command(path);
                }
            }
            Some(ListKind::Voices(voices)) => {
                if let Some((id, name)) = voices.get(n).cloned() {
                    self.select_voice(&id, &name);
                }
            }
            Some(ListKind::Info) | None => {}
        }
        vec![Effect::Redraw]
    }

    /// Delete on a list item (bookmarks, notes, highlights).
    fn delete_item(&mut self, n: usize) -> Vec<Effect> {
        match self.list.clone() {
            Some(ListKind::Bookmarks) => self.delete_bookmark(n),
            // Deleting a note or highlight asks first, as the delete_note
            // action does: a stray Delete in the list cannot lose one.
            Some(kind @ (ListKind::Notes | ListKind::Highlights)) => {
                let question = list_delete_question(&kind);
                self.list = None;
                self.pending_list_delete = Some((kind, n));
                self.tell(question);
                vec![Effect::Redraw]
            }
            _ => {
                self.tell("Nothing to delete in this list.");
                vec![Effect::Redraw]
            }
        }
    }

    /// The item a letter chooses at once in the list shown, if the list has
    /// such keys: in the Save, Discard, Cancel list, `s`, `d`, and `c`.
    /// Other letters (and other lists) jump to the next item starting with
    /// the letter instead, which the frontend does.
    pub fn list_accelerator(&self, c: char) -> Option<usize> {
        match (&self.list, c.to_ascii_lowercase()) {
            (Some(ListKind::SaveChoice(_)), 's') => Some(0),
            (Some(ListKind::SaveChoice(_)), 'd') => Some(1),
            (Some(ListKind::SaveChoice(_)), 'c') => Some(2),
            _ => None,
        }
    }

    /// Space on a list item: in the voice list, adds the voice to the
    /// favourites or removes it.
    fn mark_item(&mut self, n: usize) -> Vec<Effect> {
        match self.list.clone() {
            Some(ListKind::Voices(_)) => self.toggle_favourite_voice(n),
            _ => {
                self.tell("Nothing to mark in this list.");
                vec![Effect::Redraw]
            }
        }
    }

    /// F2 on a list item: rename a bookmark or edit a note.
    fn rename_item(&mut self, n: usize) -> Vec<Effect> {
        match self.list.clone() {
            Some(ListKind::Bookmarks) => self.rename_bookmark_prompt(n),
            Some(ListKind::Notes) => {
                let Some(text) = self
                    .session
                    .as_ref()
                    .and_then(|s| s.notes.get(n))
                    .map(|x| x.note.clone())
                else {
                    return vec![Effect::Redraw];
                };
                self.list = None;
                self.pending_item = Some(n);
                let mut e = self.prompt(PromptPurpose::EditNote);
                self.tell(&format!("Editing note: {text}"));
                e.push(Effect::Redraw);
                e
            }
            _ => {
                self.tell("Nothing to rename in this list.");
                vec![Effect::Redraw]
            }
        }
    }

    pub(crate) fn action(&mut self, a: ActionId) -> Vec<Effect> {
        if let Some(question) = a.confirmation_prompt() {
            if self.mode.is_prompt() {
                self.leave_prompt();
            }
            self.pending_confirm = Some(a);
            self.tell(question);
            return vec![Effect::Redraw];
        }
        self.run_action(a)
    }

    /// Runs an action (after its confirmation, if it needs one).
    fn run_action(&mut self, a: ActionId) -> Vec<Effect> {
        if self.session.is_none() && needs_document(a) {
            self.tell("No document is open. Press Control O to open one.");
            return vec![Effect::Redraw];
        }
        if self.mode.is_prompt() {
            self.leave_prompt();
        }
        if self.rsvp_action(a, Instant::now()) {
            return vec![Effect::Redraw];
        }
        use ActionId as A;
        match a {
            A::Quit => return self.quit(),
            // Reading
            A::PlayPause => self.play_pause(),
            A::Stop => self.stop_action(),
            A::ReadFromCursor => {
                if self.mode == Mode::SpeechCursor {
                    self.speech_cursor_exit_and_read();
                } else {
                    self.read_from_cursor();
                }
            }
            A::ReadCurrentCharacter => self.read_current_character(),
            A::ReadCurrentWord => self.read_current_unit(textweaver_core::Unit::Word),
            A::ReadCurrentSentence => self.read_current_unit(textweaver_core::Unit::Sentence),
            A::ReadCurrentLine => self.read_current_line(),
            A::ReadParagraph => self.read_current_unit(textweaver_core::Unit::Paragraph),
            A::ReadSelection => self.read_selection(),
            A::SayPosition => self.say_position(),
            A::WordCount => self.word_count(),
            A::LinkAddress => self.link_address(),
            A::ReplaySentence => self.replay_sentence(),
            A::ReplayParagraph => self.replay_paragraph(),
            A::RsvpToggle => self.rsvp_toggle(Instant::now()),
            A::RsvpPlayPause => self.rsvp_play_pause(Instant::now()),
            A::RsvpFaster => self.rsvp_rate(true),
            A::RsvpSlower => self.rsvp_rate(false),
            A::RsvpPositionNext => self.rsvp_position_next(),
            A::ReadingLevel => self.say_reading_level(),
            // Navigation
            A::NextSentence => self.next_sentence(),
            A::PreviousSentence => self.previous_sentence(),
            A::NextParagraph => self.next_paragraph(),
            A::PreviousParagraph => self.previous_paragraph(),
            A::NextHeading => self.heading(textweaver_core::Direction::Forward, true),
            A::PreviousHeading => self.heading(textweaver_core::Direction::Backward, true),
            A::SkipNextHeading => self.heading(textweaver_core::Direction::Forward, false),
            A::SkipPreviousHeading => self.heading(textweaver_core::Direction::Backward, false),
            A::NextHeadingLevel1
            | A::NextHeadingLevel2
            | A::NextHeadingLevel3
            | A::NextHeadingLevel4
            | A::NextHeadingLevel5
            | A::NextHeadingLevel6
            | A::PreviousHeadingLevel1
            | A::PreviousHeadingLevel2
            | A::PreviousHeadingLevel3
            | A::PreviousHeadingLevel4
            | A::PreviousHeadingLevel5
            | A::PreviousHeadingLevel6 => {
                if let Some((level, next)) = a.heading_level_jump() {
                    let dir = if next {
                        textweaver_core::Direction::Forward
                    } else {
                        textweaver_core::Direction::Backward
                    };
                    self.heading_level_jump(level, dir);
                }
            }
            A::NextTable => self.marker_jump(
                textweaver_core::MarkerKind::Table,
                textweaver_core::Direction::Forward,
            ),
            A::PreviousTable => self.marker_jump(
                textweaver_core::MarkerKind::Table,
                textweaver_core::Direction::Backward,
            ),
            A::NextList => self.marker_jump(
                textweaver_core::MarkerKind::List,
                textweaver_core::Direction::Forward,
            ),
            A::PreviousList => self.marker_jump(
                textweaver_core::MarkerKind::List,
                textweaver_core::Direction::Backward,
            ),
            A::NextListItem => self.marker_jump(
                textweaver_core::MarkerKind::ListItem,
                textweaver_core::Direction::Forward,
            ),
            A::PreviousListItem => self.marker_jump(
                textweaver_core::MarkerKind::ListItem,
                textweaver_core::Direction::Backward,
            ),
            A::NextLink => self.marker_jump(
                textweaver_core::MarkerKind::Link,
                textweaver_core::Direction::Forward,
            ),
            A::PreviousLink => self.marker_jump(
                textweaver_core::MarkerKind::Link,
                textweaver_core::Direction::Backward,
            ),
            A::NextChapter => self.chapter(textweaver_core::Direction::Forward),
            A::PreviousChapter => self.chapter(textweaver_core::Direction::Backward),
            A::HistoryBack => self.history_back(),
            A::HistoryForward => self.history_forward(),
            A::GoTo => return self.prompt(PromptPurpose::GoTo),
            A::DocumentStart => self.document_edge(textweaver_core::Direction::Backward),
            A::DocumentEnd => self.document_edge(textweaver_core::Direction::Forward),
            A::CaretNextWord => self.caret_word(textweaver_core::Direction::Forward),
            A::CaretPreviousWord => self.caret_word(textweaver_core::Direction::Backward),
            A::CaretNextLine => self.caret_line(textweaver_core::Direction::Forward),
            A::CaretPreviousLine => self.caret_line(textweaver_core::Direction::Backward),
            A::SelectNextWord => self.extend_selection(
                textweaver_core::Unit::Word,
                textweaver_core::Direction::Forward,
            ),
            A::SelectPreviousWord => self.extend_selection(
                textweaver_core::Unit::Word,
                textweaver_core::Direction::Backward,
            ),
            A::SelectNextLine => self.extend_selection(
                textweaver_core::Unit::Line,
                textweaver_core::Direction::Forward,
            ),
            A::SelectPreviousLine => self.extend_selection(
                textweaver_core::Unit::Line,
                textweaver_core::Direction::Backward,
            ),
            A::PageDown => self.page(textweaver_core::Direction::Forward),
            A::PageUp => self.page(textweaver_core::Direction::Backward),
            A::ScrollDown => self.scroll_lines(1),
            A::ScrollUp => self.scroll_lines(-1),
            // Speech Cursor
            A::SpeechCursorToggle => self.speech_cursor_toggle(),
            A::SpeechCursorNextLine => self.speech_cursor_move(1),
            A::SpeechCursorPreviousLine => self.speech_cursor_move(-1),
            A::SpeechCursorRereadLine => self.speech_cursor_reread(),
            A::SpeechCursorExitAndRead => self.speech_cursor_exit_and_read(),
            // Voice
            A::RateUp => self.change_rate(20),
            A::RateDown => self.change_rate(-20),
            A::PitchUp => self.change_pitch(1),
            A::PitchDown => self.change_pitch(-1),
            A::VolumeUp => self.change_volume(10),
            A::VolumeDown => self.change_volume(-10),
            A::CycleSpeedPreset => self.cycle_speed_preset(),
            // Search
            A::Find => return self.prompt(PromptPurpose::Find),
            A::FindNext => return self.find_step(textweaver_core::Direction::Forward),
            A::FindPrevious => return self.find_step(textweaver_core::Direction::Backward),
            // Bookmarks
            A::AddNote => return self.notes_command(NoteCommand::Add),
            A::ListNotes => return self.notes_command(NoteCommand::List),
            A::NextNote => return self.notes_command(NoteCommand::Next),
            A::PreviousNote => return self.notes_command(NoteCommand::Previous),
            A::HighlightSelection => return self.notes_command(NoteCommand::ToggleHighlight),
            A::DeleteNote => return self.delete_note_here(),
            A::AddBookmark => self.add_bookmark(),
            A::ListBookmarks => return self.list_bookmarks(),
            A::NextBookmark => self.bookmark_step(textweaver_core::Direction::Forward),
            A::PreviousBookmark => self.bookmark_step(textweaver_core::Direction::Backward),
            // File
            A::Open => return self.prompt(PromptPurpose::Open),
            A::OpenLibrary => return self.open_library(),
            A::ExportSettings => return self.settings_file_prompt(false),
            A::ImportSettings => return self.settings_file_prompt(true),
            // View and help
            A::NextTheme => self.next_theme(),
            A::ToggleLineNumbers => self.toggle_line_numbers(),
            A::ToggleCharacterKeys => self.toggle_character_keys(),
            A::BionicToggle => self.bionic_toggle(),
            A::RulerCycle => self.ruler_cycle(),
            A::CommandPalette => return self.prompt(PromptPurpose::CommandPalette),
            A::KeyboardHelp => return self.keyboard_help(),
            A::Help => return self.help(),
            A::ReadDocument => {
                self.stop_speech();
                self.read_from(CharPos::ZERO);
            }
            // File and editing
            A::NewDocument => return self.new_document(),
            A::Save => return self.save(None),
            A::SaveAs => return self.save_as(),
            A::ToggleEditMode => return self.toggle_edit(),
            A::Undo
            | A::Redo
            | A::Bold
            | A::Italic
            | A::Underline
            | A::Strikethrough
            | A::InlineCode
            | A::CodeBlock
            | A::InsertLink
            | A::Heading
            | A::BulletList
            | A::NumberedList
            | A::BlockQuote
            | A::HorizontalRule
            | A::InsertTable
            | A::AddTableRow
            | A::InsertImage
            | A::Replace => return self.edit_action(a),
            A::ChooseVoice => return self.choose_voice(),
            A::Copy => return self.copy(),
            A::Cut => return self.cut(),
            A::NextTableCell => return self.table_cell(textweaver_core::Direction::Forward),
            A::PreviousTableCell => return self.table_cell(textweaver_core::Direction::Backward),
            A::CycleTypingEcho => self.cycle_typing_echo(),
        }
        vec![Effect::Redraw]
    }
}

/// The question asked before deleting from the notes or highlights list.
fn list_delete_question(kind: &ListKind) -> &'static str {
    match kind {
        ListKind::Highlights => "Remove this highlight? y or n",
        _ => "Delete this note? y or n",
    }
}

/// Actions that do nothing useful without a document.
fn needs_document(a: ActionId) -> bool {
    use textweaver_keymap::Category as C;
    a != ActionId::Stop
        && matches!(
            a.category(),
            C::Reading | C::Navigation | C::SpeechCursor | C::Search | C::Bookmarks
        )
}
