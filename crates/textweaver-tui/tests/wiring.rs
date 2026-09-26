//! Scripted TUI tests for the wave 2 wiring (Agent D3), on ratatui's
//! `TestBackend`: the pending question on the status line, the library
//! list (Alt+L), the single-key switch (F9), and selection with
//! Shift+arrows through the keymap's select actions.

use std::path::{Path, PathBuf};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use textweaver_app::a11y::Priority;
use textweaver_app::core::CharPos;
use textweaver_app::store::{Paths, Settings};
use textweaver_app::testing::recording_service;
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig};
use textweaver_tui::Tui;

const WIDTH: u16 = 80;
const HEIGHT: u16 = 24;

struct Harness {
    tui: Tui,
    term: Terminal<TestBackend>,
}

fn launch(settings: Settings, home: &Path) -> Harness {
    let (speech, _log) = recording_service().unwrap();
    let app = App::new(AppConfig {
        settings,
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

fn ch(c: char) -> KeyEvent {
    let mods = if c.is_ascii_uppercase() {
        KeyModifiers::SHIFT
    } else {
        KeyModifiers::NONE
    };
    KeyEvent::new(KeyCode::Char(c), mods)
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

    fn row_text(&self, y: u16) -> String {
        let buf = self.term.backend().buffer();
        (0..WIDTH).map(|x| buf[(x, y)].symbol()).collect()
    }

    fn status(&self) -> String {
        let size = self.term.backend().buffer().area;
        let a = self.tui.areas(size).status;
        (a.y..a.y + a.height)
            .map(|y| self.row_text(y))
            .collect::<Vec<_>>()
            .join("")
    }

    fn screen(&self) -> String {
        (0..HEIGHT)
            .map(|y| self.row_text(y))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn cursor(&self) -> CharPos {
        self.tui.app().session().unwrap().cursor
    }
}

fn doc(dir: &Path, name: &str, text: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, text).unwrap();
    p
}

const TEXT: &str = "One two three. Four five six.\n\nSeven eight.";

#[test]
fn the_pending_question_stays_on_the_status_line() {
    let dir = tempfile::tempdir().unwrap();
    let file = doc(dir.path(), "q.txt", TEXT);
    let mut h = launch(Settings::default(), &dir.path().join("home"));
    h.tui.app_mut().open(&file).unwrap();
    h.draw();
    h.press(ch('q'));
    assert!(
        h.status().contains("Quit textweaver? y or n"),
        "{}",
        h.status()
    );
    assert!(
        h.row_text(HEIGHT - 1).contains("y yes"),
        "{}",
        h.row_text(HEIGHT - 1)
    );
    // Something else is announced meanwhile: the question stays in view.
    h.tui.app_mut().announce("Done reading.", Priority::Polite);
    h.draw();
    let status = h.status();
    assert!(status.contains("Quit textweaver? y or n"), "{status}");
    assert!(status.contains("Done reading."), "{status}");
    // Answered: the status line is back to plain announcements.
    h.press(ch('n'));
    assert!(!h.status().contains("y or n"), "{}", h.status());
    assert!(h.status().contains("Cancelled."));
    assert!(h.row_text(HEIGHT - 1).contains("quit"));
    assert!(!h.tui.should_quit());
}

#[test]
fn alt_l_lists_the_library_and_enter_opens() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("Readings");
    std::fs::create_dir_all(&folder).unwrap();
    doc(&folder, "chapter.txt", TEXT);
    let loose = doc(dir.path(), "loose.txt", "Loose text.");
    let mut settings = Settings::default();
    settings.library.add_folder(&folder);
    let mut h = launch(settings, &dir.path().join("home"));
    h.tui.app_mut().open(&loose).unwrap();
    h.draw();
    h.press(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::ALT));
    // The folders are scanned on a background thread; the list opens on a
    // tick of the event loop.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while h.tui.app().library_scanning() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(2));
        h.tui.tick();
    }
    h.draw();
    let list = h.tui.list().expect("library list");
    assert_eq!(list.title, "Library");
    assert_eq!(list.items.len(), 2);
    assert!(h.screen().contains("Library (1 of 2)"), "{}", h.screen());
    assert!(
        h.status().starts_with("Library, 2 documents."),
        "{}",
        h.status()
    );
    h.press(key(KeyCode::Down));
    assert!(
        h.status().starts_with("loose.txt, recent"),
        "{}",
        h.status()
    );
    h.press(key(KeyCode::Up));
    h.press(key(KeyCode::Enter));
    assert!(h.tui.list().is_none());
    assert!(
        h.row_text(0).contains("textweaver: chapter.txt"),
        "{}",
        h.row_text(0)
    );
}

#[test]
fn f9_switches_single_keys_off_and_on() {
    let dir = tempfile::tempdir().unwrap();
    let file = doc(dir.path(), "k.txt", TEXT);
    let mut h = launch(Settings::default(), &dir.path().join("home"));
    h.tui.app_mut().open(&file).unwrap();
    h.draw();
    h.press(key(KeyCode::F(9)));
    assert!(
        h.status().starts_with("Single-key shortcuts off."),
        "{}",
        h.status()
    );
    // "." and the extra note key "'" do nothing now; Alt+. still moves.
    h.press(ch('.'));
    assert_eq!(h.cursor(), CharPos(0));
    h.press(ch('\''));
    assert!(
        h.status().starts_with("Single-key shortcuts off."),
        "{}",
        h.status()
    );
    h.press(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::ALT));
    assert_eq!(h.cursor(), CharPos(15));
    // q cannot quit by accident either; Ctrl+Q still asks.
    h.press(ch('q'));
    assert_eq!(h.tui.app().pending_confirmation(), None);
    h.press(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));
    assert!(h.status().contains("Quit textweaver? y or n"));
    h.press(key(KeyCode::Esc));
    h.press(key(KeyCode::F(9)));
    assert!(
        h.status().starts_with("Single-key shortcuts on."),
        "{}",
        h.status()
    );
    h.press(ch('.'));
    assert_eq!(h.cursor(), CharPos(TEXT.find("Seven").unwrap()));
}

#[test]
fn shift_arrows_select_through_the_keymap() {
    let dir = tempfile::tempdir().unwrap();
    let file = doc(dir.path(), "s.txt", TEXT);
    let mut h = launch(Settings::default(), &dir.path().join("home"));
    h.tui.app_mut().open(&file).unwrap();
    h.draw();
    h.press(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT));
    h.press(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT));
    assert!(h.status().starts_with("two selected"), "{}", h.status());
    let s = h.tui.app().session().unwrap();
    assert_eq!(s.doc.slice(s.selection.unwrap()), "One two");
    // The selection is drawn in the selection style.
    let sel_bg = h.tui.theme().selection.bg;
    let buf = h.term.backend().buffer();
    assert_eq!(Some(buf[(1, 1)].bg), sel_bg);
    h.press(KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT));
    assert!(h.status().starts_with("two unselected"), "{}", h.status());
}
