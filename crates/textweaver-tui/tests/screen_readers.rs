//! The terminal side of screen reader coexistence (Agent P2c) on
//! ratatui's `TestBackend`: the first-run question on the status line, the
//! cursor parked on the status line, the title line frozen while reading
//! with a quiet screen, the mode in the title, and Alt+Shift+A.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Position;
use textweaver_app::a11y::AccessMode;
use textweaver_app::a11y::detect::Detected;
use textweaver_app::core::CharPos;
use textweaver_app::store::{AccessMode as ModeSetting, CursorPlacement};
use textweaver_app::store::{DocKey, Settings};
use textweaver_app::testing::recording_service;
use textweaver_app::text::Document;
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig, Playback};
use textweaver_tui::Tui;

const WIDTH: u16 = 80;
const HEIGHT: u16 = 12;

struct Harness {
    tui: Tui,
    term: Terminal<TestBackend>,
}

fn launch(settings: Settings, lines: usize) -> Harness {
    let (speech, _log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        settings,
        speech,
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    let text: String = (1..=lines)
        .map(|i| format!("Line number {i} has a sentence.\n"))
        .collect();
    app.open_document(
        Document::from_plain_text(&text),
        DocKey::untitled(1),
        "Lines".into(),
    );
    let mut h = Harness {
        tui: Tui::with_color_support(app, ColorSupport::TrueColor),
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

    fn press(&mut self, k: KeyEvent) {
        self.tui.handle_key(k);
        self.draw();
    }

    fn row_text(&self, y: u16) -> String {
        let buf = self.term.backend().buffer();
        (0..WIDTH).map(|x| buf[(x, y)].symbol()).collect()
    }

    fn title(&self) -> String {
        self.row_text(0)
    }

    fn status_area(&self) -> ratatui::layout::Rect {
        let size = self.term.backend().buffer().area;
        self.tui.areas(size).status
    }

    fn cursor(&mut self) -> Position {
        self.term.get_cursor_position().unwrap()
    }
}

#[test]
fn the_first_run_question_stays_on_the_status_line() {
    let mut h = launch(Settings::default(), 5);
    assert!(h.tui.app_mut().offer_hybrid(&Detected {
        name: Some("JAWS".into())
    }));
    h.tui.app_mut().announce(
        "Something else happened.",
        textweaver_app::a11y::Priority::Polite,
    );
    h.draw();
    let line = h.tui.status_line();
    assert!(
        line.starts_with("JAWS is running. Use hybrid mode"),
        "{line}"
    );
    assert!(h.tui.hints(WIDTH).contains("y yes"));
    h.press(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    assert_eq!(h.tui.app().access_mode(), AccessMode::Hybrid);
    assert!(h.title().contains("hybrid"), "{}", h.title());
}

#[test]
fn the_cursor_can_wait_on_the_status_line() {
    let mut follow = launch(Settings::default(), 5);
    let body_cursor = follow.cursor();
    assert!(body_cursor.y < follow.status_area().y);

    let mut s = Settings::default();
    s.accessibility.cursor = CursorPlacement::Status;
    let mut h = launch(s, 5);
    let status = h.status_area();
    assert_eq!(h.cursor(), Position::new(status.x, status.y));
    // A prompt keeps the cursor at its caret, where typing happens.
    h.press(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL));
    assert_eq!(h.cursor().y, HEIGHT - 1);
}

#[test]
fn a_quiet_screen_freezes_the_title_while_reading() {
    let mut s = Settings::default();
    s.accessibility.quiet_screen = true;
    s.accessibility.mode = ModeSetting::ScreenReader;
    let mut h = launch(s, 40);
    let first = h.title();
    assert!(first.contains("line 1 of"), "{first}");
    assert!(first.contains("screen reader mode"), "{first}");
    // Screen say-all (screen-reader mode) is continuous reading.
    h.press(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    assert_eq!(h.tui.app().playback(), Playback::Reading);
    assert!(h.tui.app().quiet_screen_active());
    for n in 1..6u64 {
        h.tui
            .app_mut()
            .tick(std::time::Instant::now() + std::time::Duration::from_secs(10 * n));
        h.draw();
    }
    assert!(h.tui.app().session().unwrap().cursor > CharPos(100));
    assert!(h.title().contains("line 1 of"), "{}", h.title());
    // Stopping lets it move again.
    h.press(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(!h.title().contains("line 1 of"), "{}", h.title());
}

#[test]
fn alt_shift_a_cycles_the_mode() {
    let mut h = launch(Settings::default(), 3);
    let alt_shift = KeyModifiers::ALT | KeyModifiers::SHIFT;
    h.press(KeyEvent::new(KeyCode::Char('A'), alt_shift));
    assert_eq!(h.tui.app().access_mode(), AccessMode::Hybrid);
    h.press(KeyEvent::new(KeyCode::Char('A'), alt_shift));
    assert_eq!(h.tui.app().access_mode(), AccessMode::ScreenReader);
    assert!(h.tui.status_line().starts_with("Screen reader mode."));
}
