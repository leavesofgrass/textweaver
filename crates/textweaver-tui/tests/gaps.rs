//! Agent P2e's terminal features: the digit row on any keyboard layout,
//! syllables drawn between chars with exact highlights, difficult words
//! underlined, and math exploration keys.

use std::path::Path;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Modifier;
use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::Settings;
use textweaver_app::testing::recording_service;
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig, Command, HighlightKind};
use textweaver_tui::Tui;
use textweaver_tui::physical::PeekedKey;

const WIDTH: u16 = 60;
const HEIGHT: u16 = 12;

struct Harness {
    tui: Tui,
    term: Terminal<TestBackend>,
    _dir: tempfile::TempDir,
}

fn launch(settings: Settings, name: &str, text: &str) -> Harness {
    let (speech, _log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        settings,
        speech,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(name);
    std::fs::write(&file, text).unwrap();
    app.open(Path::new(&file)).unwrap();
    let mut h = Harness {
        tui: Tui::with_color_support(app, ColorSupport::TrueColor),
        term: Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap(),
        _dir: dir,
    };
    h.draw();
    h
}

fn ch(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
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
    fn body_row(&self) -> u16 {
        let size = self.term.backend().buffer().area;
        self.tui.areas(size).body.y
    }
    fn status(&self) -> String {
        let size = self.term.backend().buffer().area;
        let a = self.tui.areas(size).status;
        (a.y..a.y + a.height).map(|y| self.row(y)).collect()
    }
    fn cursor(&self) -> CharPos {
        self.tui.app().session().unwrap().cursor
    }
    fn line_of(&self, pos: CharPos) -> String {
        let doc = &self.tui.app().session().unwrap().doc;
        let line = textweaver_app::text_util::line_of(doc, pos);
        doc.slice(textweaver_app::text_util::line_range(doc, line))
    }
}

/// Headings at each level, then text, so every heading-level key has a
/// target in both directions.
const HEADINGS: &str = "# One\n\nText.\n\n## Two\n\nText.\n\n### Three\n\nText.\n\n#### Four\n\nText.\n\n##### Five\n\nText.\n\n###### Six\n\nThe end.\n";

#[test]
fn shifted_digits_of_other_layouts_reach_the_previous_heading() {
    let mut h = launch(Settings::default(), "h.md", HEADINGS);
    h.press(key(KeyCode::End));
    // German Shift+3 types §, UK Shift+3 £, Spanish Shift+3 ·.
    for (c, heading) in [('§', "Three"), ('"', "Two"), ('&', "Six"), ('¤', "Four")] {
        h.press(key(KeyCode::End));
        h.press(ch(c));
        assert!(
            h.line_of(h.cursor()).contains(heading),
            "{c}: {}",
            h.status()
        );
    }
    // US characters work as before.
    h.press(key(KeyCode::End));
    h.press(ch('@'));
    assert!(h.line_of(h.cursor()).contains("Two"));
}

#[test]
fn the_console_names_the_digit_key_on_any_layout() {
    let mut h = launch(Settings::default(), "h.md", HEADINGS);
    // French AZERTY: the 1 key types & without Shift; Windows says VK_1.
    h.tui.digit_keys_mut().set_pending([PeekedKey {
        ch: '&',
        digit: Some((1, false)),
    }]);
    h.press(key(KeyCode::End));
    h.press(key(KeyCode::Home));
    h.press(ch('&'));
    assert_eq!(h.cursor(), CharPos(0), "{}", h.status());
    h.tui.digit_keys_mut().set_pending([PeekedKey {
        ch: 'é',
        digit: Some((2, false)),
    }]);
    h.press(ch('é'));
    assert!(h.line_of(h.cursor()).contains("Two"), "{}", h.status());
    // Shift with the 3 key types 3 on AZERTY: the previous level 3.
    h.press(key(KeyCode::End));
    h.tui.digit_keys_mut().set_pending([PeekedKey {
        ch: '3',
        digit: Some((3, true)),
    }]);
    h.press(ch('3'));
    assert!(h.line_of(h.cursor()).contains("Three"), "{}", h.status());
    // A key the console says is not on the digit row keeps its meaning:
    // US Shift+7 types & and is not a heading key.
    h.press(key(KeyCode::End));
    let end = h.cursor();
    h.tui.digit_keys_mut().set_pending([PeekedKey {
        ch: '&',
        digit: None,
    }]);
    h.press(ch('&'));
    assert_eq!(h.cursor(), end);
}

#[test]
fn azerty_digit_row_without_the_console() {
    let mut s = Settings::default();
    s.keyboard.digit_row = textweaver_app::store::DigitRow::Azerty;
    let mut h = launch(s, "h.md", HEADINGS);
    h.press(ch('é'));
    assert!(h.line_of(h.cursor()).contains("Two"), "{}", h.status());
    h.press(key(KeyCode::End));
    h.press(ch('1'));
    assert!(h.line_of(h.cursor()).contains("One"), "{}", h.status());
}

#[test]
fn syllables_draw_between_chars_and_highlights_stay_exact() {
    let mut s = Settings::default();
    s.reading_aids.syllables = true;
    let text = "Readability matters here.\n";
    let mut h = launch(s, "s.txt", text);
    let word = CharRange::new(0, 11);
    let split = h.tui.app().syllable_display(word).unwrap();
    assert!(split.text.contains('\u{b7}'), "{}", split.text);
    let y = h.body_row();
    let row = h.row(y);
    assert!(row.starts_with(&split.text), "{row} vs {}", split.text);
    // Select the word: the selection covers the whole drawn word, with its
    // separators, and nothing after it.
    h.tui.dispatch(Command::Select(word));
    h.draw();
    let shown = split.display_range(word).unwrap();
    let width = split.text[shown].chars().count() as u16;
    let bg = |x: u16| h.term.backend().buffer()[(x, y)].bg;
    let sel = h.tui.theme().highlight(HighlightKind::Selection).bg;
    for x in 0..width {
        assert_eq!(Some(bg(x)), sel, "column {x}");
    }
    assert_ne!(Some(bg(width + 1)), sel);
    // The cursor on "matters" is drawn after the separators before it.
    h.tui.app_mut().set_cursor(CharPos(12));
    h.draw();
    let p = h.term.get_cursor_position().unwrap();
    let col = row[..row.find("mat").unwrap()].chars().count();
    assert_eq!(usize::from(p.x), col);
    // Speech and positions are the text as it is.
    assert_eq!(
        h.tui.app().session().unwrap().doc.slice(word),
        "Readability"
    );
    // Hidden again.
    h.press(h.tui.key_for(ActionId::SyllablesToggle));
    assert!(
        h.status().starts_with("Syllables hidden."),
        "{}",
        h.status()
    );
    assert!(h.row(y).starts_with("Readability matters"));
}

#[test]
fn difficult_words_are_underlined_and_named_at_high_verbosity() {
    let mut s = Settings::default();
    s.speech.verbosity = textweaver_app::a11y::Verbosity::High;
    let text = "The cat saw obvious mitochondria today.\n";
    let mut h = launch(s, "d.txt", text);
    h.press(h.tui.key_for(ActionId::DifficultWordsToggle));
    assert!(
        h.status().starts_with("Difficult words underlined."),
        "{}",
        h.status()
    );
    let y = h.body_row();
    let under = |h: &Harness, x: usize| {
        h.term.backend().buffer()[(x as u16, y)]
            .modifier
            .contains(Modifier::UNDERLINED)
    };
    let hard = text.find("mitochondria").unwrap();
    assert!(under(&h, hard), "{}", h.row(y));
    assert!(!under(&h, text.find("cat").unwrap()));
    // A word move onto it says so.
    h.tui
        .app_mut()
        .set_cursor(CharPos(text.find("obvious").unwrap()));
    h.press(key(KeyCode::Right));
    assert_eq!(h.cursor(), CharPos(hard));
    assert!(
        h.status().contains("mitochondria, difficult word"),
        "{}",
        h.status()
    );
}

#[test]
fn math_exploration_keys() {
    let text = "Area: $\\frac{a}{b}$ and more.\n";
    let mut h = launch(Settings::default(), "m.md", text);
    h.tui
        .app_mut()
        .set_cursor(CharPos(text.find("frac").unwrap() - 1));
    h.press(h.tui.key_for(ActionId::ExploreMath));
    assert!(h.tui.app().math_exploring(), "{}", h.status());
    h.press(key(KeyCode::Down));
    assert!(h.status().starts_with("numerator"), "{}", h.status());
    h.press(key(KeyCode::Right));
    assert!(h.status().starts_with("denominator"), "{}", h.status());
    h.press(key(KeyCode::Down));
    assert!(h.status().starts_with("No parts inside."), "{}", h.status());
    h.press(key(KeyCode::Up));
    h.press(key(KeyCode::Esc));
    assert!(!h.tui.app().math_exploring());
    assert!(h.status().starts_with("Left math."), "{}", h.status());
    // Another key leaves at once and does its own work.
    h.press(h.tui.key_for(ActionId::ExploreMath));
    assert!(h.tui.app().math_exploring());
    h.press(ch('j'));
    assert!(!h.tui.app().math_exploring());
}
