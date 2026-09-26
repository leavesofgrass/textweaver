//! Scripted TUI tests for the Phase 2 authoring keys (Agent P2b), on
//! ratatui's `TestBackend`: the outline (Alt+O) filtering as you type,
//! heading chords and the editing basics in edit mode.

use std::path::{Path, PathBuf};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use textweaver_app::core::CharRange;
use textweaver_app::store::{Paths, Settings};
use textweaver_app::testing::recording_service;
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig, Mode};
use textweaver_tui::Tui;

const WIDTH: u16 = 80;
const HEIGHT: u16 = 24;

struct Harness {
    tui: Tui,
    term: Terminal<TestBackend>,
}

fn launch(home: &Path) -> Harness {
    let (speech, _log) = recording_service().unwrap();
    let app = App::new(AppConfig {
        settings: Settings::default(),
        speech,
        paths: Some(Paths::under(home)),
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    let mut h = Harness {
        tui: Tui::with_color_support(app, ColorSupport::TrueColor),
        term: Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap(),
    };
    h.draw();
    h
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn alt(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT)
}

impl Harness {
    fn draw(&mut self) {
        let tui = &mut self.tui;
        self.term.draw(|f| tui.draw(f)).unwrap();
    }

    fn press(&mut self, k: KeyEvent) {
        self.tui.handle_key(k);
        self.draw();
    }

    fn typed(&mut self, s: &str) {
        for c in s.chars() {
            self.press(key(KeyCode::Char(c)));
        }
    }

    /// The document text from the cursor, `n` chars.
    fn at_cursor(&self, n: usize) -> String {
        let s = self.tui.app().session().unwrap();
        s.doc
            .slice(CharRange::new(s.cursor, s.cursor.saturating_add(n)))
    }
}

fn doc(dir: &Path, name: &str, text: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, text).unwrap();
    p
}

const ESSAY: &str =
    "# Essay\n\nIntro words.\n\n## Methods\n\nWe measured.\n\n## Results\n\nIt worked.\n";

#[test]
fn alt_o_lists_headings_and_typing_filters_them() {
    let dir = tempfile::tempdir().unwrap();
    let file = doc(dir.path(), "essay.md", ESSAY);
    let mut h = launch(&dir.path().join("home"));
    h.tui.app_mut().open(&file).unwrap();
    h.draw();
    h.press(alt('o'));
    let list = h.tui.list().expect("outline");
    assert_eq!(list.title, "Outline, 3 headings");
    assert_eq!(list.items.len(), 3);
    // Letters filter instead of jumping; Space is part of the filter.
    h.typed("re");
    let list = h.tui.list().expect("filtered outline");
    assert_eq!(list.items, ["Results, level 2"]);
    assert_eq!(h.tui.app().list_filter(), Some("re"));
    // Backspace removes a letter instead of closing.
    h.press(key(KeyCode::Backspace));
    assert_eq!(h.tui.app().list_filter(), Some("r"));
    assert!(h.tui.list().is_some());
    h.typed("es");
    h.press(key(KeyCode::Enter));
    assert!(h.tui.list().is_none());
    assert_eq!(h.at_cursor(7), "Results");
}

#[test]
fn edit_mode_has_heading_chords_and_the_editing_basics() {
    let dir = tempfile::tempdir().unwrap();
    let file = doc(dir.path(), "essay.md", ESSAY);
    let mut h = launch(&dir.path().join("home"));
    h.tui.app_mut().open(&file).unwrap();
    h.draw();
    h.press(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
    assert_eq!(h.tui.app().mode(), Mode::Edit);
    // Alt+H and Alt+Shift+H move by heading while editing (the caret
    // starts on the title's text).
    assert_eq!(h.at_cursor(5), "Essay");
    h.press(alt('h'));
    assert_eq!(h.at_cursor(7), "Methods");
    h.press(KeyEvent::new(
        KeyCode::Char('H'),
        KeyModifiers::ALT | KeyModifiers::SHIFT,
    ));
    assert_eq!(h.at_cursor(5), "Essay");
    // Alt+Backspace deletes the word before the caret.
    h.press(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    h.press(KeyEvent::new(KeyCode::Backspace, KeyModifiers::ALT));
    let text = h.tui.app().session().unwrap().doc.text().to_string();
    assert!(text.starts_with("# \n"), "{text:?}");
    // Ctrl+A selects everything.
    h.press(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
    let sel = h
        .tui
        .app()
        .edit_session()
        .unwrap()
        .editor()
        .unwrap()
        .selection();
    assert_eq!(sel.range().start.0, 0);
    assert_eq!(sel.range().end.0, text.chars().count());
}
