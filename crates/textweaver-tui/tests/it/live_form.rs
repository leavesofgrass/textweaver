//! The live source form in the terminal reader (beta 5, phase 4): the
//! keys a reader presses reach the app's form. Each field is the prompt
//! line, its label first and its value after it, inside one 40-cell
//! Braille line; Tab and Shift+Tab move and keep what was typed; Up and
//! Down change a choice in place; a bad field opens the form again on it;
//! Escape cancels.
//!
//! Nothing reaches a network and no audio plays: the form only writes
//! settings, here held in memory.

#![cfg(feature = "mqtt")]

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::{AccessMode as ModeSetting, LiveTls, Settings};
use textweaver_app::testing::recording_service;
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig, Command};
use textweaver_tui::Tui;
use textweaver_tui::ui::{BRAILLE_CELLS, braille_cells};

const WIDTH: u16 = 80;
const HEIGHT: u16 = 24;

struct Harness {
    tui: Tui,
    term: Terminal<TestBackend>,
}

fn launch() -> Harness {
    let mut settings = Settings::default();
    settings.accessibility.mode = ModeSetting::ScreenReader;
    let (speech, _log) = recording_service().unwrap();
    let app = App::new(AppConfig {
        settings,
        speech,
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    let mut h = Harness {
        tui: Tui::with_color_support(app, ColorSupport::NoColor),
        term: Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap(),
    };
    h.draw();
    h
}

impl Harness {
    fn draw(&mut self) {
        let tui = &mut self.tui;
        self.term.draw(|f| tui.draw(f)).unwrap();
    }

    fn press(&mut self, code: KeyCode, mods: KeyModifiers) {
        self.tui.handle_key(KeyEvent::new(code, mods));
        self.draw();
    }

    fn key(&mut self, code: KeyCode) {
        self.press(code, KeyModifiers::NONE);
    }

    fn typed(&mut self, s: &str) {
        for c in s.chars() {
            self.key(KeyCode::Char(c));
        }
    }

    /// The prompt line, at the bottom.
    fn prompt_line(&self) -> String {
        let buf = self.term.backend().buffer();
        (0..WIDTH)
            .map(|x| buf[(x, HEIGHT - 1)].symbol())
            .collect::<String>()
            .trim_end()
            .to_owned()
    }

    fn field_text(&self) -> String {
        self.tui
            .app()
            .prompt_model()
            .expect("the form is open")
            .text()
    }

    /// Asserts the prompt line starts with `label` and shows `value`, all
    /// within one Braille line.
    fn shows(&self, label: &str, value: &str) {
        let line = self.prompt_line();
        assert!(
            line.starts_with(label),
            "{line:?} does not start with {label:?}"
        );
        assert!(line.contains(value), "{value:?} is not in {line:?}");
        let upto = &line[..line.find(value).unwrap_or(0) + value.len()];
        assert!(
            braille_cells(upto) <= BRAILLE_CELLS,
            "{upto:?} is wider than one Braille line"
        );
    }
}

#[test]
fn the_form_is_the_prompt_line_and_its_keys_move_and_change_fields() {
    let mut h = launch();
    h.tui.dispatch(Command::Action(ActionId::SetUpLiveSource));
    h.draw();
    h.shows("Name, 1 of 7", "");
    h.typed("Practice log");
    h.shows("Name, 1 of 7", "Practice log");
    h.key(KeyCode::Tab);
    h.shows("Broker host, 2 of 7", "");
    h.typed("logger.example.ts.net");
    h.key(KeyCode::Tab);
    // A long label is cut to its first clause on the line; the rest is
    // said.
    h.shows("Encryption", "Off");
    h.key(KeyCode::Down);
    h.shows("Encryption", "Bundled roots");
    h.key(KeyCode::Tab);
    h.shows("Port, 4 of 7", "8883");
    // Shift+Tab goes back, and the values stay.
    h.press(KeyCode::BackTab, KeyModifiers::SHIFT);
    assert_eq!(h.field_text(), "Bundled roots");
    h.press(KeyCode::BackTab, KeyModifiers::SHIFT);
    assert_eq!(h.field_text(), "logger.example.ts.net");
    // Enter with no topic: said, and the form opens on the topic.
    h.key(KeyCode::Enter);
    h.shows("Topic, 5 of 7", "");
    h.typed("log/checkins");
    h.key(KeyCode::Enter);
    let s = h.tui.app().settings().live.sources["practice-log"].clone();
    assert_eq!(s.tls, LiveTls::System);
    assert_eq!(s.topics, ["log/checkins"]);
    assert!(h.tui.app().list_model().is_some(), "offered for opening");
}

#[test]
fn escape_cancels_the_form() {
    let mut h = launch();
    h.tui.dispatch(Command::Action(ActionId::SetUpLiveSource));
    h.draw();
    h.typed("Practice log");
    h.key(KeyCode::Tab);
    h.key(KeyCode::Esc);
    assert!(h.tui.app().prompt_model().is_none());
    assert!(h.tui.app().settings().live.sources.is_empty());
    assert!(
        h.tui
            .app()
            .status_text()
            .contains("Canceled. No live source saved."),
        "{}",
        h.tui.app().status_text()
    );
}
