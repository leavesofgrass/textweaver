use std::path::Path;

use textweaver_a11y::{Announcer, LogAnnouncer, Priority, StatusLineAnnouncer, Verbosity};
use textweaver_core::{CharPos, CharRange};
use textweaver_formats::{LoadError, LoadOptions, Registry, Source};
use textweaver_keymap::{ActionId, Frontend, Keymap, Layer, Platform};
use textweaver_speech::{SayMode, SpeechService};
use textweaver_store::{
    Bookmark, DocKey, DocState, Paths, Recent, Settings, SettingsStore, StateStore, StoreError,
};
use textweaver_text::{Document, History, SearchQuery};

use crate::command::{Command, Effect, PromptPurpose};
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
}

impl Mode {
    /// The keymap layer for this mode.
    pub fn layer(self) -> Layer {
        match self {
            Mode::Browse => Layer::Browse,
            Mode::SpeechCursor => Layer::SpeechCursor,
            Mode::Edit => Layer::Edit,
            Mode::Find | Mode::Command | Mode::GoTo | Mode::Open => Layer::Global,
        }
    }

    /// True while a prompt owns the keyboard.
    pub fn is_prompt(self) -> bool {
        matches!(self, Mode::Find | Mode::Command | Mode::GoTo | Mode::Open)
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
        }
    }

    pub(crate) fn for_prompt(p: PromptPurpose) -> Mode {
        match p {
            PromptPurpose::Find => Mode::Find,
            PromptPurpose::GoTo => Mode::GoTo,
            PromptPurpose::Open => Mode::Open,
            PromptPurpose::CommandPalette => Mode::Command,
        }
    }
}

/// The last search and its matches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindState {
    /// The query as run.
    pub query: SearchQuery,
    /// Every match, in document order.
    pub hits: Vec<CharRange>,
    /// Index of the match the cursor is on.
    pub current: Option<usize>,
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
    pub(crate) view: Viewport,
    pub(crate) self_voicing: bool,
    pub(crate) backend_name: String,
    pub(crate) spoken_log: Vec<CharRange>,
}

impl App {
    /// Themes the `next_theme` action cycles through.
    pub const THEMES: [&'static str; 3] = ["galaxy", "light", "high-contrast"];

    /// Most highlight ranges kept by [`App::spoken_log`].
    pub const SPOKEN_LOG_LIMIT: usize = 4096;

    /// Creates the application.
    pub fn new(config: AppConfig) -> Self {
        let mut app = App {
            session: None,
            speech: config.speech,
            announcer: config.announcer,
            status: StatusLineAnnouncer::default(),
            keymap: config.keymap,
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
            view: Viewport::default(),
            self_voicing: config.self_voicing,
            backend_name: config.backend_name,
            spoken_log: Vec::new(),
        };
        app.apply_voice_settings();
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
        let doc = self
            .registry
            .load(&Source::Path(path.to_owned()), &LoadOptions::default())?;
        let title = doc.meta.title.clone().unwrap_or_else(|| {
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string())
        });
        let key = DocKey::for_path(path);
        if let Some(paths) = &self.paths {
            let file = paths.recent_file();
            let mut recent = Recent::load(&file);
            recent.touch(
                path,
                Some(title.clone()),
                self.settings.library.recent_limit,
            );
            if let Err(e) = recent.save(&file) {
                log::warn!("cannot save recent files: {e}");
            }
        }
        Ok(self.open_document(doc, key, title))
    }

    /// Makes an already loaded document current (tests, in-memory sources).
    /// Saved state for `key` is restored when persistence is configured.
    pub fn open_document(&mut self, doc: Document, key: DocKey, title: String) -> Vec<Effect> {
        if self.session.is_some()
            && let Err(e) = self.save_position()
        {
            log::warn!("cannot save position: {e}");
        }
        self.stop_speech();
        self.mode = Mode::Browse;
        self.list = None;
        self.spoken_log.clear();
        let mut s = Session::new(doc, key, title, self.settings.reading.nav_history_size);
        let mut resumed = None;
        if let Some(store) = self.state_store()
            && let Some(state) = store.load(&s.key)
        {
            for &h in &state.history {
                s.history.record(h.clamp_to(s.doc.len_chars()));
            }
            s.bookmarks = state.bookmarks.clone();
            s.bookmarks.sort_by_key(|b| b.pos);
            if self.settings.reading.auto_resume && state.position > CharPos::ZERO {
                s.cursor = text_util::first_word_at_or_after(&s.doc, state.position);
                resumed = Some(text_util::percent(&s.doc, s.cursor));
            }
            s.saved = state;
        }
        let title = s.title.clone();
        self.session = Some(s);
        self.view.top_line = 0;
        self.scroll_to_cursor();
        let msg = match resumed {
            Some(p) => format!("Opened {title}. Resumed at {p} percent."),
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

    /// Saves the reading position, history, and bookmarks of the open
    /// document (a no-op without persistence).
    pub fn save_position(&mut self) -> Result<(), AppError> {
        let Some(pos) = self.reading_position() else {
            return Ok(());
        };
        let Some(store) = self.state_store() else {
            return Ok(());
        };
        let Some(s) = self.session.as_mut() else {
            return Ok(());
        };
        let pos = text_util::word_start(&s.doc, pos);
        let mut state = s.saved.clone();
        state.position = pos;
        state.pct = text_util::percent(&s.doc, pos);
        state.ts = textweaver_store::now_ts();
        state.history = s.history.entries().to_vec();
        state.bookmarks = s.bookmarks.clone();
        store.save(&s.key, &state)?;
        s.saved = state;
        Ok(())
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

    /// Saves everything and stops speech: what quitting does. Safe to call
    /// more than once.
    pub fn shutdown(&mut self) {
        if let Err(e) = self.save_position() {
            log::warn!("cannot save position: {e}");
        }
        if let Err(e) = self.save_settings() {
            log::warn!("cannot save settings: {e}");
        }
        self.stop_speech();
    }

    /// Handles one command.
    pub fn dispatch(&mut self, cmd: Command) -> Vec<Effect> {
        match cmd {
            Command::Open(path) => {
                self.leave_prompt();
                match self.open(&path) {
                    Ok(e) => e,
                    Err(e) => {
                        let name = path.display();
                        self.error(&format!("Could not open {name}: {e}"));
                        vec![Effect::Redraw]
                    }
                }
            }
            Command::Action(a) => self.action(a),
            Command::Insert(_) => {
                self.tell("Editing is not available yet.");
                vec![Effect::Redraw]
            }
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
                if self.mode.is_prompt() || self.list.is_some() {
                    self.leave_prompt();
                    self.list = None;
                    self.note("Cancelled.");
                }
                vec![Effect::Redraw]
            }
        }
    }

    /// Opens a prompt: switches mode and returns the effect that shows it.
    pub(crate) fn prompt(&mut self, purpose: PromptPurpose) -> Vec<Effect> {
        if !self.mode.is_prompt() {
            self.return_mode = self.mode;
        }
        self.mode = Mode::for_prompt(purpose);
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
        self.leave_prompt();
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

    fn choose(&mut self, n: usize) -> Vec<Effect> {
        match self.list.take() {
            Some(ListKind::Bookmarks) => self.go_to_bookmark(n),
            Some(ListKind::Actions(actions)) => {
                if let Some(&a) = actions.get(n) {
                    return self.action(a);
                }
            }
            Some(ListKind::Info) | None => {}
        }
        vec![Effect::Redraw]
    }

    pub(crate) fn action(&mut self, a: ActionId) -> Vec<Effect> {
        if self.session.is_none() && needs_document(a) {
            self.tell("No document is open. Press Control O to open one.");
            return vec![Effect::Redraw];
        }
        if self.mode.is_prompt() {
            self.leave_prompt();
        }
        use ActionId as A;
        match a {
            A::Quit => {
                self.shutdown();
                return vec![Effect::Quit];
            }
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
            A::ReadSelection => self.read_selection(),
            A::SayPosition => self.say_position(),
            A::ReplaySentence => self.replay_sentence(),
            A::ReplayParagraph => self.replay_paragraph(),
            // Navigation
            A::NextSentence => self.next_sentence(),
            A::PreviousSentence => self.previous_sentence(),
            A::NextParagraph => self.next_paragraph(),
            A::PreviousParagraph => self.previous_paragraph(),
            A::NextHeading => self.heading(textweaver_core::Direction::Forward, true),
            A::PreviousHeading => self.heading(textweaver_core::Direction::Backward, true),
            A::SkipNextHeading => self.heading(textweaver_core::Direction::Forward, false),
            A::SkipPreviousHeading => self.heading(textweaver_core::Direction::Backward, false),
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
            A::AddBookmark => self.add_bookmark(),
            A::ListBookmarks => return self.list_bookmarks(),
            A::NextBookmark => self.bookmark_step(textweaver_core::Direction::Forward),
            A::PreviousBookmark => self.bookmark_step(textweaver_core::Direction::Backward),
            // File
            A::Open => return self.prompt(PromptPurpose::Open),
            // View and help
            A::NextTheme => self.next_theme(),
            A::ToggleLineNumbers => self.toggle_line_numbers(),
            A::CommandPalette => return self.prompt(PromptPurpose::CommandPalette),
            A::KeyboardHelp => return self.keyboard_help(),
            A::Help => return self.help(),
            // Editing and saving arrive with edit mode in wave 2.
            other => {
                let msg = format!("{} is not available yet.", other.help());
                self.tell(&msg);
            }
        }
        vec![Effect::Redraw]
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
