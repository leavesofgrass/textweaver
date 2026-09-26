//! Scripted TUI tests for the reading aids (Agent D3, ADR-0022) on
//! ratatui's `TestBackend`: RSVP (Alt+Shift+R, Alt+Shift+P, navigation,
//! Escape), bionic reading (Alt+Shift+B), the reading ruler
//! (Alt+Shift+U), and terminal text spacing.

use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Modifier;
use textweaver_app::core::CharPos;
use textweaver_app::store::{DocKey, Settings};
use textweaver_app::testing::recording_service;
use textweaver_app::text::Document;
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig};
use textweaver_tui::Tui;

const WIDTH: u16 = 60;
const HEIGHT: u16 = 16;

const TEXT: &str = "One two three.\nFour five six.\nSeven eight nine.\n\nTen eleven twelve.";

struct Harness {
    tui: Tui,
    term: Terminal<TestBackend>,
}

fn launch(settings: Settings) -> Harness {
    let (speech, _log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        settings,
        speech,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    app.open_document(
        Document::from_plain_text(TEXT),
        DocKey::untitled(1),
        "aids".into(),
    );
    let mut h = Harness {
        tui: Tui::with_color_support(app, ColorSupport::TrueColor),
        term: Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap(),
    };
    h.draw();
    h
}

fn alt_shift(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT | KeyModifiers::SHIFT)
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
    fn row(&self, y: u16) -> String {
        let buf = self.term.backend().buffer();
        (0..WIDTH).map(|x| buf[(x, y)].symbol()).collect()
    }
    fn screen(&self) -> String {
        (0..HEIGHT)
            .map(|y| self.row(y))
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn modifiers(&self, x: u16, y: u16) -> Modifier {
        self.term.backend().buffer()[(x, y)].modifier
    }
    fn status(&self) -> String {
        let size = self.term.backend().buffer().area;
        let a = self.tui.areas(size).status;
        (a.y..a.y + a.height).map(|y| self.row(y)).collect()
    }
    fn rsvp_word(&self) -> String {
        self.tui
            .app()
            .rsvp()
            .and_then(|r| r.frame())
            .map(|f| f.text.to_owned())
            .unwrap_or_default()
    }
}

#[test]
fn rsvp_shows_one_word_steps_and_closes() {
    let mut h = launch(Settings::default());
    h.press(alt_shift('R'));
    assert!(
        h.status().starts_with("RSVP on. Word 1 of 12"),
        "{}",
        h.status()
    );
    assert_eq!(h.rsvp_word(), "One");
    // The box is drawn, with the word bold and its pivot underlined, and
    // not on the cursor's row.
    let cursor_row = h.term.backend().cursor_position().y;
    let box_row = (1..HEIGHT - 2)
        .find(|&y| y != cursor_row && h.row(y).contains("One") && h.row(y).trim() != "")
        .filter(|&y| !h.row(y).starts_with("One two"))
        .expect("an RSVP box");
    let col = u16::try_from(h.row(box_row).find("One").unwrap()).unwrap();
    assert!(h.modifiers(col, box_row).contains(Modifier::BOLD));
    assert!(
        (col..col + 3).any(|x| h.modifiers(x, box_row).contains(Modifier::UNDERLINED)),
        "pivot underlined"
    );
    assert!(h.row(box_row + 1).contains("two"), "next word shown below");

    // Play, then time passes: the next word, and the cursor follows.
    h.press(alt_shift('P'));
    assert!(h.status().starts_with("RSVP playing."));
    h.tui
        .app_mut()
        .tick(Instant::now() + Duration::from_millis(400));
    h.draw();
    assert_eq!(h.rsvp_word(), "two");
    assert_eq!(h.tui.app().session().unwrap().cursor, CharPos(4));
    assert!(h.screen().contains("two"));
    h.press(alt_shift('P'));
    assert!(h.status().starts_with("RSVP paused."));

    // The sentence key moves the RSVP word; faster says the rate.
    h.press(KeyEvent::new(KeyCode::Down, KeyModifiers::ALT));
    assert_eq!(h.rsvp_word(), "Four");
    h.press(KeyEvent::new(
        KeyCode::Up,
        KeyModifiers::ALT | KeyModifiers::SHIFT,
    ));
    assert!(
        h.status().starts_with("RSVP 325 words per minute."),
        "{}",
        h.status()
    );
    assert!(
        h.row(HEIGHT - 1).contains("close RSVP"),
        "{}",
        h.row(HEIGHT - 1)
    );

    // Escape closes it and the box is gone.
    h.press(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(h.tui.app().rsvp().is_none());
    assert!(h.status().starts_with("RSVP off."));
    assert!(!h.row(HEIGHT - 1).contains("close RSVP"));
}

#[test]
fn bionic_reading_bolds_the_start_of_each_word() {
    let mut h = launch(Settings::default());
    assert!(!h.modifiers(0, 1).contains(Modifier::BOLD));
    h.press(alt_shift('B'));
    assert!(h.status().starts_with("Bionic reading on."));
    // "One": 1 of 3 letters bold; "three.": 2 of 5.
    assert!(h.row(1).starts_with("One two three."));
    assert!(h.modifiers(0, 1).contains(Modifier::BOLD));
    assert!(!h.modifiers(1, 1).contains(Modifier::BOLD));
    assert!(h.modifiers(8, 1).contains(Modifier::BOLD));
    assert!(h.modifiers(9, 1).contains(Modifier::BOLD));
    assert!(!h.modifiers(10, 1).contains(Modifier::BOLD));
    h.press(alt_shift('B'));
    assert!(!h.modifiers(0, 1).contains(Modifier::BOLD));
}

#[test]
fn the_ruler_marks_the_line_without_colour_alone() {
    let mut h = launch(Settings::default());
    h.press(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    h.press(alt_shift('U'));
    assert!(h.status().starts_with("Current line marked."));
    // A gutter column holds the mark; the text moves right by one.
    assert!(
        h.row(2).starts_with("\u{258c}Four five six."),
        "{:?}",
        h.row(2)
    );
    assert!(h.row(1).starts_with(" One two"), "{:?}", h.row(1));
    assert!(h.modifiers(1, 2).contains(Modifier::UNDERLINED));
    assert!(!h.modifiers(1, 1).contains(Modifier::UNDERLINED));
    // The hardware cursor still sits on the word.
    assert_eq!(h.term.backend().cursor_position().x, 1);
    h.press(alt_shift('U'));
    assert!(h.status().starts_with("Reading ruler on."));
    assert!(h.row(1).starts_with('\u{2502}'), "{:?}", h.row(1));
    assert!(h.row(3).starts_with('\u{2502}'), "{:?}", h.row(3));
    h.press(alt_shift('U'));
    assert!(h.row(2).starts_with("Four"), "off again");
}

#[test]
fn text_spacing_adds_rows_and_word_space() {
    let mut s = Settings::default();
    s.reading_aids.spacing.line_height = 2.0;
    s.reading_aids.spacing.word_spacing = 0.25;
    let h = launch(s);
    assert!(h.row(1).starts_with("One  two  three."), "{:?}", h.row(1));
    assert_eq!(h.row(2).trim(), "", "a blank row between lines");
    assert!(h.row(3).starts_with("Four  five"), "{:?}", h.row(3));
    // The cursor is on the first word.
    let p = h.term.backend().cursor_position();
    assert_eq!((p.x, p.y), (0, 1));
}
