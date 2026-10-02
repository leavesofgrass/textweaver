//! Braille-first fixes (Wave 8c, W8c-t; the terminal research, QW1 to
//! QW4 and question 9): status text never cut, title parts in a Braille
//! order, keyboard list rows led by the command, cell 1 holding text in
//! screen-reader and hybrid modes, and `[display] hints`.
//!
//! "Cells" here are Braille cells, counted with
//! [`textweaver_tui::ui::braille_cells`] (uncontracted UEB for ASCII),
//! not print characters.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use textweaver_app::a11y::Priority;
use textweaver_app::keymap::ActionId;
use textweaver_app::store::{
    AccessMode as ModeSetting, CursorPlacement, DocKey, HintsLine, Settings,
};
use textweaver_app::testing::recording_service;
use textweaver_app::text::{Document, GoTo};
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig, Command};
use textweaver_tui::Tui;
use textweaver_tui::ui::{BRAILLE_CELLS, braille_cells};

struct Harness {
    tui: Tui,
    term: Terminal<TestBackend>,
    width: u16,
}

/// The owner's setup: screen-reader mode, the cursor on the status line.
fn screen_reader() -> Settings {
    let mut s = Settings::default();
    s.accessibility.mode = ModeSetting::ScreenReader;
    s.accessibility.cursor = CursorPlacement::Status;
    s
}

fn launch(settings: Settings, width: u16, height: u16, text: Option<&str>) -> Harness {
    let (speech, _log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        settings,
        speech,
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    if let Some(text) = text {
        app.open_document(
            Document::from_plain_text(text),
            DocKey::untitled(1),
            "Essay".into(),
        );
    }
    let mut h = Harness {
        tui: Tui::with_color_support(app, ColorSupport::NoColor),
        term: Terminal::new(TestBackend::new(width, height)).unwrap(),
        width,
    };
    h.draw();
    h
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

impl Harness {
    /// Draws a moment later, so a repeated message is not blank.
    fn draw(&mut self) {
        let later = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let tui = &mut self.tui;
        self.term.draw(|f| tui.draw_at(f, later)).unwrap();
    }

    fn press(&mut self, k: KeyEvent) {
        self.tui.handle_key(k);
        self.draw();
    }

    fn act(&mut self, a: ActionId) {
        self.tui.dispatch(Command::Action(a));
        self.draw();
    }

    fn row(&self, y: u16) -> String {
        let buf = self.term.backend().buffer();
        (0..self.width).map(|x| buf[(x, y)].symbol()).collect()
    }

    fn status_area(&self) -> ratatui::layout::Rect {
        self.tui.areas(self.term.backend().buffer().area).status
    }

    /// Every row of the status area, joined with spaces.
    fn status(&self) -> String {
        let a = self.status_area();
        (a.y..a.y + a.height)
            .map(|y| self.row(y).trim_end().to_owned())
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn bottom_row(&self) -> String {
        let h = self.term.backend().buffer().area.height;
        self.row(h - 1)
    }
}

/// Asserts that `key` is in `line` and ends within 40 Braille cells.
fn within_forty_cells(what: &str, line: &str, key: &str) {
    let Some(at) = line.find(key) else {
        panic!("{what}: {key:?} is not in {line:?}");
    };
    let end = braille_cells(&line[..at + key.len()]);
    assert!(
        end <= BRAILLE_CELLS,
        "{what}: {key:?} ends at Braille cell {end}, past {BRAILLE_CELLS}: {line:?}"
    );
}

fn long_paragraph() -> String {
    let words: Vec<String> = (0..80).map(|i| format!("word{i}")).collect();
    format!("{} lastword.", words.join(" "))
}

/// A whole line of about 500 characters, put on the status line by a
/// caret move and by Say Paragraph, is drawn in full, and the status
/// area does not move (QW1).
#[test]
fn long_status_text_is_shown_in_full_and_the_area_stays_still() {
    let para = long_paragraph();
    assert!(para.len() > 480, "{}", para.len());
    let text = format!("Short first line.\n{para}\nThird line.\n");
    let mut h = launch(screen_reader(), 80, 24, Some(&text));
    let before = h.status_area();
    // ceil(600 / 80) rows, fixed.
    assert_eq!(before.height, 8);
    h.press(key(KeyCode::Down));
    let s = h.status();
    assert!(s.contains("word0 word1"), "{s}");
    assert!(s.contains("lastword."), "the line's end was cut: {s}");
    assert_eq!(h.status_area(), before, "the status area moved");
    h.press(key(KeyCode::Up));
    h.press(key(KeyCode::Down));
    h.press(h.tui.key_for(ActionId::ReadParagraph));
    let s = h.status();
    assert!(s.contains("lastword."), "the paragraph's end was cut: {s}");
    assert_eq!(h.status_area(), before, "the status area moved");
    // The cursor waits at the start of the status area.
    let p = h.term.backend().cursor_position();
    assert_eq!((p.x, p.y), (before.x, before.y));
}

/// Self-voicing, 40 columns: a message of 80 characters needs three
/// wrapped rows, not the two its length suggests, and its last word
/// stays on screen (QW1, the row count).
#[test]
fn a_message_wrapped_onto_one_more_row_keeps_its_last_word() {
    let mut h = launch(Settings::default(), 40, 24, Some("One line.\n"));
    let msg = format!("{} {} ending", "x".repeat(36), "y".repeat(36));
    assert_eq!(msg.chars().count(), 80);
    h.tui.app_mut().announce(&msg, Priority::Polite);
    h.draw();
    assert_eq!(h.status_area().height, 3);
    let s = h.status();
    assert!(s.ends_with("ending"), "{s}");
}

/// Edit and Speech Cursor modes on a 400-line document: the mode and
/// "modified" come before the reading state, inside 40 Braille cells,
/// and the brand is gone (QW2).
#[test]
fn the_title_line_puts_the_mode_and_modified_inside_forty_cells() {
    let text: String = (1..=400).map(|i| format!("Line number {i}.\n")).collect();
    let mut h = launch(screen_reader(), 80, 24, Some(&text));
    h.tui.dispatch(Command::GoTo(GoTo::Line(12)));
    h.draw();
    let t = h.row(0);
    within_forty_cells("browse title", &t, ", Ready");
    assert!(!t.contains("textweaver:"), "{t}");
    assert!(t.trim_end().ends_with("  Essay"), "{t}");

    h.act(ActionId::SpeechCursorToggle);
    let t = h.row(0);
    within_forty_cells("Speech Cursor title", &t, ", Speech Cursor");
    // Say Status speaks the same order.
    h.act(ActionId::SayStatus);
    let all = h.status();
    let s = all.rsplit_once("Essay").map_or(all.as_str(), |(_, b)| b);
    let mode = s
        .find("Speech Cursor")
        .unwrap_or_else(|| panic!("the mode is said: {all}"));
    let state = s.find("Ready").or_else(|| s.find("Stopped"));
    assert!(state.is_some_and(|st| mode < st), "{s}");
    h.act(ActionId::SpeechCursorToggle);

    h.act(ActionId::ToggleEditMode);
    h.press(key(KeyCode::Char('x')));
    let t = h.row(0);
    assert!(t.starts_with("Line 12 of 400, "), "{t}");
    within_forty_cells("edit title", &t, ", Edit, modified");
}

/// The keyboard shortcuts list: each row leads with the command's name,
/// then its keys, inside 40 Braille cells with the place before them
/// (QW3).
#[test]
fn keyboard_list_rows_lead_with_the_command_and_its_keys() {
    let mut h = launch(screen_reader(), 80, 24, Some("One line.\n"));
    h.act(ActionId::KeyboardHelp);
    let list = h.tui.list().expect("the keyboard list").clone();
    assert!(
        list.items.iter().any(|i| i.starts_with("Find: ")),
        "{:?}",
        &list.items[..5]
    );
    assert!(
        list.items[0].starts_with("Play or pause: "),
        "{}",
        list.items[0]
    );
    h.press(key(KeyCode::Down));
    let s = h.status_area();
    let line = h.row(s.y);
    let item = &list.items[1];
    let keys_end = item.find(". ").expect("keys, then the help");
    within_forty_cells("keyboard list item", &line, &item[..keys_end]);
    h.press(key(KeyCode::Esc));
}

/// Screen-reader mode with line numbers and the reading ruler on: every
/// line starts with text, the hints line is hidden, and the empty screen
/// starts at the edge (QW4, question 9).
#[test]
fn cell_one_holds_text_in_braille_first() {
    let mut s = screen_reader();
    s.display.show_line_numbers = true;
    let text: String = (1..=12).map(|i| format!("Row {i} words.\n")).collect();
    let mut h = launch(s, 80, 24, Some(&text));
    h.press(key(KeyCode::Down));
    h.press(h.tui.key_for(ActionId::RulerCycle));
    let body = h.tui.areas(h.term.backend().buffer().area).body;
    for y in body.y..body.y + 12 {
        let r = h.row(y);
        assert!(
            r.starts_with(|c: char| c.is_ascii_digit()),
            "row {y} starts with a blank or a mark: {r:?}"
        );
    }
    assert!(
        h.row(body.y + 1).starts_with("2  Row 2 words."),
        "{:?}",
        h.row(body.y + 1)
    );
    // The hints line: hidden by default with a screen reader.
    assert_eq!(h.bottom_row().trim(), "", "{:?}", h.bottom_row());
    // Shown when asked for, starting with a key.
    h.tui
        .app_mut()
        .update_settings(|s| s.display.hints = HintsLine::On)
        .unwrap();
    h.draw();
    let b = h.bottom_row();
    assert!(!b.starts_with(' ') && !b.trim().is_empty(), "{b:?}");

    // No document: the first line starts at the edge, the cursor on it.
    let mut s = screen_reader();
    s.accessibility.cursor = CursorPlacement::Follow;
    let h = launch(s, 80, 24, None);
    let r = h.row(2);
    assert!(r.starts_with(|c: char| !c.is_whitespace()), "{r:?}");
    let p = h.term.backend().cursor_position();
    assert_eq!((p.x, p.y), (0, 2));
}

/// Self-voicing keeps its hints line by default, with its blank first;
/// `hints = "off"` hides it.
#[test]
fn hints_follow_the_setting_when_self_voicing() {
    let mut h = launch(Settings::default(), 80, 24, Some("One line.\n"));
    let b = h.bottom_row();
    assert!(b.starts_with(' ') && !b.trim().is_empty(), "{b:?}");
    h.tui
        .app_mut()
        .update_settings(|s| s.display.hints = HintsLine::Off)
        .unwrap();
    h.draw();
    assert_eq!(h.bottom_row().trim(), "");
    // A prompt still uses the line.
    h.act(ActionId::Find);
    assert!(h.bottom_row().starts_with("Find: "), "{:?}", h.bottom_row());
}
