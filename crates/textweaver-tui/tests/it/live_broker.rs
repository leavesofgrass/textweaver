//! The sending side and the document view in the terminal reader (beta
//! 5, phase 5b): Send a check-in is the prompt line, its label first,
//! inside one 40-cell Braille line; joining by code is a choice of the
//! live source form; the broker's state is on the title line in words; a
//! shared document keeps its lines, and a new revision leaves the
//! reader's row where it was, saying one short line.
//!
//! The only broker here listens on 127.0.0.1 with a port the system
//! picks; a scripted source stands in for any other. No audio plays.

#![cfg(feature = "mqtt")]

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use textweaver_app::keymap::ActionId;
use textweaver_app::live::{LiveOptions, ScriptHandle, ScriptedSource};
use textweaver_app::store::{AccessMode as ModeSetting, Settings};
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
    _home: tempfile::TempDir,
}

fn launch() -> Harness {
    let mut settings = Settings::default();
    settings.accessibility.mode = ModeSetting::ScreenReader;
    settings.live.broker.listen = vec!["127.0.0.1:0".into()];
    let (speech, _log) = recording_service().unwrap();
    // A data folder for the broker's key and accounts.
    let home = tempfile::tempdir().unwrap();
    let app = App::new(AppConfig {
        settings,
        paths: Some(textweaver_app::store::Paths::under(home.path())),
        speech,
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    let mut h = Harness {
        tui: Tui::with_color_support(app, ColorSupport::NoColor),
        term: Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap(),
        _home: home,
    };
    h.draw();
    h
}

impl Harness {
    fn draw(&mut self) {
        let tui = &mut self.tui;
        self.term.draw(|f| tui.draw(f)).unwrap();
    }

    fn key(&mut self, code: KeyCode) {
        self.tui.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
        self.draw();
    }

    fn typed(&mut self, s: &str) {
        for c in s.chars() {
            self.key(KeyCode::Char(c));
        }
    }

    fn act(&mut self, a: ActionId) {
        self.tui.dispatch(Command::Action(a));
        self.draw();
    }

    fn row(&self, y: u16) -> String {
        let buf = self.term.backend().buffer();
        (0..WIDTH)
            .map(|x| buf[(x, y)].symbol())
            .collect::<String>()
            .trim_end()
            .to_owned()
    }

    fn rows(&self) -> Vec<String> {
        (0..HEIGHT).map(|y| self.row(y)).collect()
    }

    /// Asserts the prompt line starts with `label`, inside one Braille
    /// line.
    fn shows(&self, label: &str) {
        let line = self.row(HEIGHT - 1);
        assert!(
            line.starts_with(label),
            "{line:?} does not start with {label:?}"
        );
        assert!(braille_cells(label) <= BRAILLE_CELLS, "{label:?}");
    }
}

#[test]
fn the_check_in_form_is_the_prompt_line_and_the_broker_is_on_the_title_line() {
    let mut h = launch();
    h.act(ActionId::StartBroker);
    assert!(h.tui.app().live_broker_running());
    assert!(
        h.rows().iter().any(|r| r.contains("Broker on, 0 clients")),
        "{:#?}",
        h.rows()
    );
    h.act(ActionId::SendCheckIn);
    h.shows("Callsign, 1 of 3");
    h.typed("N0CALL");
    h.key(KeyCode::Tab);
    h.shows("Name, 2 of 3");
    h.key(KeyCode::Tab);
    h.shows("Notes, 3 of 3");
    h.key(KeyCode::Tab);
    h.shows("Callsign, 1 of 3");
    assert_eq!(h.tui.app().prompt_model().unwrap().text(), "N0CALL");
    h.key(KeyCode::Esc);
    assert!(h.tui.app().prompt_model().is_none());
    assert!(
        h.tui
            .app()
            .status_text()
            .contains("Canceled. No check-in sent."),
        "{}",
        h.tui.app().status_text()
    );
    h.act(ActionId::StopBroker);
    assert!(!h.rows().iter().any(|r| r.contains("Broker on")));
    h.tui.app_mut().shutdown();
}

#[test]
fn joining_by_code_is_a_choice_of_the_form_with_its_own_field() {
    let mut h = launch();
    h.act(ActionId::SetUpLiveSource);
    h.typed("Club");
    h.key(KeyCode::Tab);
    h.typed("127.0.0.1");
    h.key(KeyCode::Tab);
    for _ in 0..4 {
        h.key(KeyCode::Down);
    }
    assert_eq!(h.tui.app().prompt_model().unwrap().text(), "Join code");
    h.key(KeyCode::Tab);
    h.shows("Join code, 4 of 7");
    h.typed("abcd efgh jkmn pqr");
    h.key(KeyCode::Enter);
    assert!(
        h.tui.app().status_text().contains("Not a join code"),
        "{}",
        h.tui.app().status_text()
    );
    h.shows("Join code, 4 of 7");
    h.key(KeyCode::Esc);
    assert!(h.tui.app().settings().live.sources.is_empty());
}

fn open_document_source(h: &mut Harness) -> ScriptHandle {
    let (source, feed) = ScriptedSource::new();
    let _ = h.tui.app_mut().open_live_source(
        Box::new(source),
        LiveOptions {
            id: "club".into(),
            title: "Club".into(),
            ..LiveOptions::default()
        },
    );
    h.draw();
    feed
}

/// A live document message, plain text with no quotes in it.
fn shared(rev: u64, text: &str) -> String {
    format!(
        r#"{{"type":"textweaver.document","rev":{rev},"title":"Agenda","format":"text","text":"{}"}}"#,
        text.replace('\n', "\\n")
    )
}

#[test]
fn a_shared_document_keeps_its_lines_and_the_readers_row() {
    let mut h = launch();
    let feed = open_document_source(&mut h);
    feed.push_json(
        "docs/agenda",
        &shared(1, "Agenda\n\nCheck-ins\nTraffic\nClose\n"),
    );
    h.tui.tick();
    h.draw();
    let rows = h.rows();
    let at = |rows: &[String], s: &str| rows.iter().position(|r| r.trim() == s);
    let (c, t) = (
        at(&rows, "Check-ins").unwrap(),
        at(&rows, "Traffic").unwrap(),
    );
    assert_eq!(t, c + 1, "one line each: {rows:#?}");
    let status = h.tui.app().status_text().to_owned();
    assert_eq!(status, "Agenda, revision 1, shared live.");
    assert!(braille_cells(&status) <= BRAILLE_CELLS);
    // The reader on Traffic; revision 2 changes the line below it.
    let text = h.tui.app().session().unwrap().doc.text().to_string();
    let p = textweaver_app::core::CharPos(text[..text.find("Traffic").unwrap()].chars().count());
    h.tui.dispatch(Command::SetCursor(p));
    h.draw();
    let cursor = h.term.backend().cursor_position();
    feed.push_json(
        "docs/agenda",
        &shared(2, "Agenda\n\nCheck-ins\nTraffic\nClose at nine\n"),
    );
    h.tui.tick();
    h.draw();
    assert_eq!(h.term.backend().cursor_position(), cursor, "the row stays");
    assert!(h.rows().iter().any(|r| r.trim() == "Close at nine"));
    assert_eq!(h.tui.app().status_text(), "Agenda updated, revision 2.");
}
