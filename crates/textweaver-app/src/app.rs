use std::path::Path;

use textweaver_a11y::{Announcer, LogAnnouncer, Priority};
use textweaver_core::{CharPos, CharRange, Direction, Unit};
use textweaver_formats::{LoadError, LoadOptions, Registry, Source};
use textweaver_keymap::{ActionId, Frontend, Keymap, Layer, Platform};
use textweaver_speech::{SpeechService, SpeechStatus};
use textweaver_store::{DocKey, Paths, Settings};
use textweaver_text::narrate::NarrationPolicy;
use textweaver_text::{Document, History, NavOptions};

use crate::command::{Command, Effect};

/// Interaction mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// Reading; single keys navigate.
    #[default]
    Browse,
    /// Speech Cursor: line-by-line reading.
    SpeechCursor,
    /// Editing text.
    Edit,
    /// Typing a search.
    Find,
    /// Typing a command name.
    Command,
}

impl Mode {
    /// The keymap layer for this mode.
    pub fn layer(self) -> Layer {
        match self {
            Mode::Browse => Layer::Browse,
            Mode::SpeechCursor => Layer::SpeechCursor,
            Mode::Edit => Layer::Edit,
            Mode::Find | Mode::Command => Layer::Global,
        }
    }
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
    /// How announcements are delivered.
    pub announcer: Box<dyn Announcer>,
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
        }
    }
}

/// Application failures surfaced to the user.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The document could not be loaded.
    #[error(transparent)]
    Load(#[from] LoadError),
}

/// The application: the only owner of mutable state.
pub struct App {
    session: Option<Session>,
    speech: SpeechService,
    announcer: Box<dyn Announcer>,
    keymap: Keymap,
    settings: Settings,
    paths: Option<Paths>,
    registry: Registry,
    mode: Mode,
}

impl App {
    /// Creates the application.
    pub fn new(config: AppConfig) -> Self {
        App {
            session: None,
            speech: config.speech,
            announcer: config.announcer,
            keymap: config.keymap,
            settings: config.settings,
            paths: config.paths,
            registry: Registry::with_builtins(),
            mode: Mode::Browse,
        }
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

    fn announce(&mut self, text: &str) {
        self.announcer.announce(text, Priority::Polite);
    }

    /// Opens a document and makes it current.
    pub fn open(&mut self, path: &Path) -> Result<Vec<Effect>, AppError> {
        let doc = self
            .registry
            .load(&Source::Path(path.to_owned()), &LoadOptions::default())?;
        let title = doc.meta.title.clone().unwrap_or_else(|| {
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
        self.session = Some(Session {
            doc,
            key: DocKey::for_path(path),
            cursor: CharPos::ZERO,
            selection: None,
            history: History::with_capacity(self.settings.reading.nav_history_size),
            spoken: None,
        });
        self.announce(&format!("Opened {title}"));
        Ok(vec![Effect::Redraw])
    }

    /// Handles one command.
    pub fn dispatch(&mut self, cmd: Command) -> Vec<Effect> {
        match cmd {
            Command::Open(path) => match self.open(&path) {
                Ok(e) => e,
                Err(e) => {
                    self.announce(&e.to_string());
                    vec![Effect::Redraw]
                }
            },
            Command::Action(a) => self.action(a),
            _ => vec![Effect::Redraw],
        }
    }

    fn action(&mut self, a: ActionId) -> Vec<Effect> {
        match a {
            ActionId::Quit => {
                self.speech.stop();
                return vec![Effect::Quit];
            }
            ActionId::Stop => self.speech.stop(),
            ActionId::PlayPause | ActionId::ReadFromCursor => self.read_from_cursor(),
            ActionId::NextSentence => self.step(Unit::Sentence, Direction::Forward),
            ActionId::PreviousSentence => self.step(Unit::Sentence, Direction::Backward),
            ActionId::NextParagraph => self.step(Unit::Paragraph, Direction::Forward),
            ActionId::PreviousParagraph => self.step(Unit::Paragraph, Direction::Backward),
            ActionId::CaretNextWord => self.step(Unit::Word, Direction::Forward),
            ActionId::CaretPreviousWord => self.step(Unit::Word, Direction::Backward),
            other => self.announce(&format!("{} is not implemented yet", other.id())),
        }
        vec![Effect::Redraw]
    }

    fn step(&mut self, unit: Unit, dir: Direction) {
        let wrap = self.settings.reading.wrap_navigation;
        let Some(s) = self.session.as_mut() else {
            return;
        };
        match textweaver_text::navigate(&s.doc, s.cursor, unit, dir, NavOptions { wrap }) {
            Some(t) => {
                s.cursor = t.range.start;
                let text = s.doc.slice(t.range);
                self.announce(&text);
            }
            None => self.announce(&format!(
                "No {} {}",
                match dir {
                    Direction::Forward => "next",
                    Direction::Backward => "previous",
                },
                unit.spoken_name()
            )),
        }
    }

    fn read_from_cursor(&mut self) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let range = CharRange::new(s.cursor, s.doc.end());
        let utterances = textweaver_text::plan(&s.doc, range, &NarrationPolicy::default());
        self.speech.read(utterances);
    }

    /// Drains speech status updates and applies them (highlight, cursor).
    pub fn poll_speech(&mut self) -> Vec<Effect> {
        let mut changed = false;
        while let Some(status) = self.speech.try_status() {
            if let (
                SpeechStatus::Position {
                    source_range: Some(r),
                    ..
                },
                Some(s),
            ) = (&status, self.session.as_mut())
            {
                s.spoken = Some(*r);
                if self.settings.reading.cursor_follows_speech {
                    s.cursor = r.start;
                }
                changed = true;
            }
        }
        if changed {
            vec![Effect::Redraw]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_navigate_quit() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.txt");
        std::fs::write(&p, "One two. Three four. Five.").unwrap();
        let mut app = App::new(AppConfig::for_tests());
        app.open(&p).unwrap();
        app.dispatch(Command::Action(ActionId::NextSentence));
        assert_eq!(app.session().unwrap().cursor, CharPos(9));
        assert_eq!(
            app.dispatch(Command::Action(ActionId::Quit)),
            vec![Effect::Quit]
        );
    }
}
