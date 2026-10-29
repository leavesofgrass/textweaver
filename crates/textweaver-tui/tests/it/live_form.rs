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

/// The join code field is drawn as stars, and the code typed is never
/// on the line until F3 shows it; F3 again hides it. Each change is said
/// in words, and the field's hint names the key.
#[test]
fn the_join_code_is_stars_until_f3_shows_it() {
    let mut h = launch();
    h.tui.dispatch(Command::Action(ActionId::SetUpLiveSource));
    h.draw();
    h.typed("Club");
    h.key(KeyCode::Tab);
    h.typed("logger.example");
    h.key(KeyCode::Tab);
    for _ in 0..4 {
        h.key(KeyCode::Down);
    }
    h.key(KeyCode::Tab);
    h.shows("Join code, 4 of 7", "");
    let status = h.tui.app().status_text();
    assert!(status.contains("F3 shows it."), "{status}");
    h.typed("ABCD");
    h.shows("Join code, 4 of 7", "****");
    assert!(!h.prompt_line().contains("ABCD"), "{}", h.prompt_line());
    h.key(KeyCode::F(3));
    h.shows("Join code, 4 of 7", "ABCD");
    let status = h.tui.app().status_text();
    assert!(status.contains("Showing the join code"), "{status}");
    h.key(KeyCode::F(3));
    h.shows("Join code, 4 of 7", "****");
    assert!(!h.prompt_line().contains("ABCD"), "{}", h.prompt_line());
    let status = h.tui.app().status_text();
    assert!(status.contains("Hiding the join code"), "{status}");
    assert_eq!(h.field_text(), "ABCD");
}

/// A join link pasted into the form (the terminal's bracketed paste)
/// fills it, and the link is never on the prompt line or said; the join
/// code field keeps only the code from a paste of the "Show the join
/// code" list.
#[test]
fn a_pasted_join_link_fills_the_form_and_is_never_shown() {
    let mut h = launch();
    h.tui.dispatch(Command::Action(ActionId::SetUpLiveSource));
    h.draw();
    h.tui
        .paste("Join link: twjoin://10.0.0.5:8884/ABCD-EFGH-JKMN-PQRS?topic=net/agenda\r\n");
    h.draw();
    h.shows("Name, 1 of 7", "10.0.0.5");
    assert!(!h.prompt_line().contains("twjoin"), "{}", h.prompt_line());
    let status = h.tui.app().status_text();
    assert!(!status.contains("twjoin"), "{status}");
    assert!(!status.contains("PQRS"), "{status}");
    for _ in 0..3 {
        h.key(KeyCode::Tab);
    }
    h.shows("Join code, 4 of 7", "****");
    assert!(
        h.prompt_line().ends_with(&"*".repeat(19)),
        "{}",
        h.prompt_line()
    );
    assert!(!h.prompt_line().contains("ABCD"), "{}", h.prompt_line());
    h.key(KeyCode::Tab);
    h.shows("Port, 5 of 7", "8884");
    h.key(KeyCode::Tab);
    h.shows("Topic, 6 of 7", "net/agenda");
    // Back to the code: a pasted list keeps only the code.
    h.press(KeyCode::BackTab, KeyModifiers::SHIFT);
    h.press(KeyCode::BackTab, KeyModifiers::SHIFT);
    for _ in 0..19 {
        h.key(KeyCode::Backspace);
    }
    assert_eq!(h.field_text(), "");
    h.tui
        .paste("WXYZ-WXYZ-WXYZ-WXYZ\r\nGroup 1 of 4: WXYZ\r\nGroup 2 of 4: WXYZ\r\n");
    h.draw();
    assert_eq!(h.field_text(), "WXYZ-WXYZ-WXYZ-WXYZ");
    let status = h.tui.app().status_text();
    assert!(status.contains("Join code pasted."), "{status}");
}

/// Copy the join link goes to the terminal's clipboard (OSC 52), as Copy
/// does, and only "Join link copied." is said.
#[test]
fn copy_the_join_link_reaches_the_terminal_clipboard() {
    let tmp = tempfile::tempdir().unwrap();
    let (speech, _log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        paths: Some(textweaver_app::store::Paths::under(tmp.path())),
        ..AppConfig::for_tests()
    });
    let _ = app.update_settings(|s| s.live.broker.listen = vec!["127.0.0.1:0".into()]);
    let mut tui = Tui::with_color_support(app, ColorSupport::NoColor);
    tui.dispatch(Command::Action(ActionId::StartBroker));
    assert!(tui.app().live_broker_running());
    let _ = tui.take_clipboard_sequence();
    tui.dispatch(Command::Action(ActionId::CopyJoinLink));
    let osc = tui
        .take_clipboard_sequence()
        .expect("sent to the clipboard");
    assert!(osc.starts_with("\u{1b}]52;c;"), "{osc:?}");
    let status = tui.app().status_text();
    assert!(status.contains("Join link copied."), "{status}");
    assert!(!status.contains("twjoin"), "{status}");
    tui.dispatch(Command::Action(ActionId::StopBroker));
}

/// Shift+F10 (or the Applications key) on the join code field opens its
/// context menu: Copy, Paste, and Show the join code. Show opens the field
/// shown as typed, Escape goes back to it, and its text is kept.
#[test]
fn the_join_code_field_has_a_context_menu() {
    let mut h = launch();
    h.tui.dispatch(Command::Action(ActionId::SetUpLiveSource));
    h.draw();
    h.typed("Club");
    h.key(KeyCode::Tab);
    h.typed("logger.example");
    h.key(KeyCode::Tab);
    for _ in 0..4 {
        h.key(KeyCode::Down);
    }
    h.key(KeyCode::Tab);
    h.typed("ABCD");
    h.press(KeyCode::F(10), KeyModifiers::SHIFT);
    let list = h.tui.app().list_model().expect("the field's menu");
    assert_eq!(list.title, "Join code field");
    assert_eq!(list.items, ["Copy", "Paste", "Show the join code"]);
    h.key(KeyCode::Down);
    h.key(KeyCode::Down);
    h.key(KeyCode::Enter);
    h.shows("Join code, 4 of 7", "ABCD");
    assert!(h.tui.app().prompt_model().unwrap().is_shown());
    h.key(KeyCode::Menu);
    let list = h.tui.app().list_model().expect("the field's menu");
    assert_eq!(list.items[2], "Hide the join code");
    h.key(KeyCode::Esc);
    h.shows("Join code, 4 of 7", "****");
    assert_eq!(h.field_text(), "ABCD");
    // On another field the key does nothing.
    h.key(KeyCode::Tab);
    h.press(KeyCode::F(10), KeyModifiers::SHIFT);
    assert!(h.tui.app().list_model().is_none());
}
