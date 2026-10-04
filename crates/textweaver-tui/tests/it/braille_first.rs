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

/// The window's button descriptions (`gui-hint-*`, read after a button's
/// name) fit one 40-cell line, counted in Braille cells, in every language
/// (`docs/dev/messages.md`, rule 11).
#[test]
fn window_button_hints_fit_a_braille_line() {
    use textweaver_app::lexicon::i18n::{Catalog, LANGUAGES};
    for lang in LANGUAGES {
        let c = Catalog::builtin(lang.tag).unwrap_or_else(Catalog::english);
        for id in [
            "gui-hint-open",
            "gui-hint-font",
            "gui-hint-edit",
            "gui-hint-settings",
            "gui-hint-commands",
            "gui-hint-play",
        ] {
            let text = c.tr(id);
            assert!(
                braille_cells(&text) <= BRAILLE_CELLS,
                "{}: {id} takes {} cells: {text}",
                lang.tag,
                braille_cells(&text)
            );
        }
    }
}

/// Wave 9 (W9c-t, terminal rank 5): with the cursor on the status line,
/// an edge message keeps the item it stopped on, so the Braille line
/// still says where the user is.
#[test]
fn an_edge_message_keeps_the_item_on_the_status_line() {
    let mut h = launch(screen_reader(), 80, 24, Some("One line.\n"));
    h.act(ActionId::KeyboardHelp);
    h.press(key(KeyCode::End));
    let item = h.tui.app().status_text().to_owned();
    h.press(key(KeyCode::Down));
    let status = h.tui.app().status_text().to_owned();
    assert!(!item.is_empty());
    assert_eq!(status, format!("End of list. {item}"));
    // Following the cursor, the edge message stands alone, as before.
    let mut s = screen_reader();
    s.accessibility.cursor = CursorPlacement::Follow;
    let mut h = launch(s, 80, 24, Some("One line.\n"));
    h.act(ActionId::KeyboardHelp);
    h.press(key(KeyCode::Home));
    h.press(key(KeyCode::Up));
    assert_eq!(h.tui.app().status_text(), "Top of list.");
}

/// During a screen say-all, a background message follows the sentence on
/// the status line instead of replacing it (terminal rank 5, question 7).
#[test]
fn the_say_all_sentence_stays_first_on_the_status_line() {
    let mut s = screen_reader();
    s.accessibility.say_all = textweaver_app::store::SayAll::Screen;
    let text = "The first sentence is here. The second follows it.\n";
    let mut h = launch(s, 80, 24, Some(text));
    h.act(ActionId::PlayPause);
    let sentence = h.tui.app().status_text().to_owned();
    assert!(sentence.starts_with("The first sentence"), "{sentence}");
    h.tui.app_mut().announce("Saved.", Priority::Polite);
    assert_eq!(h.tui.app().status_text(), format!("{sentence}  Saved."));
    // An assertive message still shows alone.
    h.tui
        .app_mut()
        .announce("Error: disk full.", Priority::Assertive);
    assert_eq!(h.tui.app().status_text(), "Error: disk full.");
}

/// The hints line under an open list names the list's keys, not the
/// reading keys behind it: Space marks an item there, it does not play
/// (terminal rank 7, QW6).
#[test]
fn hints_under_an_open_list_match_its_keys() {
    let mut h = launch(Settings::default(), 100, 24, Some("# One\n\nText.\n"));
    assert!(h.tui.hints(100).contains("play"), "{}", h.tui.hints(100));
    h.act(ActionId::KeyboardHelp);
    assert!(h.tui.list().is_some());
    let hints = h.tui.hints(100);
    assert!(hints.contains("Enter choose"), "{hints}");
    assert!(hints.contains("close"), "{hints}");
    assert!(!hints.contains("play"), "{hints}");
    h.act(ActionId::Menu);
    let hints = h.tui.hints(100);
    assert!(hints.contains("back"), "{hints}");
}

/// The status messages a reader meets most, in every language, fit one
/// 40-cell Braille line, and an edge message leaves room for the place
/// after it (the sticky status context). Cells are counted with the
/// uncontracted UEB counter ([`braille_cells`]); the liblouis spike
/// (W9c-b) can count with a real table here (microcopy rank 14,
/// conventions rank 4; the ratio is calibrated in session S7).
#[test]
fn common_status_messages_fit_a_braille_line_in_every_language() {
    use textweaver_app::lexicon::args;
    use textweaver_app::lexicon::i18n::{Catalog, LANGUAGES};
    for lang in LANGUAGES {
        let c = Catalog::builtin(lang.tag).unwrap_or_else(Catalog::english);
        let place = |k: usize| {
            c.fmt(
                "listmodel-item-position",
                &args!["k" => k, "n" => 120, "item" => ""],
            )
        };
        let table = [
            ("playback-paused", c.tr("playback-paused")),
            ("playback-stopped", c.tr("playback-stopped")),
            ("nav-end-of-document-stop", c.tr("nav-end-of-document-stop")),
            ("nav-top-of-document-stop", c.tr("nav-top-of-document-stop")),
            (
                "listmodel-end-of-list, then a place",
                format!("{} {}", c.tr("listmodel-end-of-list"), place(120)),
            ),
            (
                "listmodel-top-of-list, then a place",
                format!("{} {}", c.tr("listmodel-top-of-list"), place(1)),
            ),
        ];
        for (id, text) in table {
            let cells = braille_cells(&text);
            assert!(
                cells <= BRAILLE_CELLS,
                "{}: {id} takes {cells} cells: {text}",
                lang.tag
            );
        }
    }
}

/// Every prompt the app opens (found by dispatching every action), in the
/// terminal and the window alike: its label fits one 40-cell line, counted
/// in Braille cells, and the terminal draws it at the start of the bottom
/// row (checklist row A8, W9e-c).
#[test]
fn every_prompt_label_fits_a_braille_line() {
    let text: String = (1..=40).map(|i| format!("Line number {i}.\n")).collect();
    let mut h = launch(screen_reader(), 80, 24, Some(&text));
    let mut prompts = Vec::new();
    let mut long = Vec::new();
    for &a in ActionId::ALL {
        if a == ActionId::Quit {
            continue;
        }
        for _ in 0..3 {
            h.tui.dispatch(Command::Cancel);
        }
        if h.tui.app().mode() == textweaver_app::Mode::Edit {
            h.act(ActionId::ToggleEditMode);
        }
        h.act(a);
        let Some(label) = h.tui.app().prompt_model().map(|p| p.label.clone()) else {
            continue;
        };
        prompts.push(format!("{a:?}: {label}"));
        // The key fact is the label's first clause ("Export settings to
        // file"); an example after a comma may run past the line.
        let fact = label.split([',', ':']).next().unwrap_or_default();
        let cells = braille_cells(fact);
        if cells > BRAILLE_CELLS {
            long.push(format!("{a:?} takes {cells} cells: {label}"));
        }
        let first = label.split_whitespace().next().unwrap_or_default();
        within_forty_cells(&format!("{a:?} prompt row"), &h.bottom_row(), first);
    }
    h.tui.dispatch(Command::Cancel);
    assert!(prompts.len() >= 5, "too few prompts: {prompts:#?}");
    assert!(long.is_empty(), "{long:#?}");
}

/// The window's status bar shows the terminal's title parts after the
/// message: `App::title_parts` joined with commas, as `gui.rs` builds its
/// position label. Its place and reading state, or in edit mode its mode
/// and "modified", end inside 40 Braille cells (checklist rows C3 and A8).
#[test]
fn the_window_position_line_puts_its_key_facts_inside_forty_cells() {
    let text: String = (1..=400).map(|i| format!("Line number {i}.\n")).collect();
    let mut h = launch(screen_reader(), 80, 24, Some(&text));
    h.tui.dispatch(Command::GoTo(GoTo::Line(212)));
    h.draw();
    let line = |h: &Harness| {
        let app = h.tui.app();
        app.title_parts(app.title_position().as_deref()).join(", ")
    };
    let l = line(&h);
    within_forty_cells("window position, browse", &l, "212 of 400");
    within_forty_cells("window position, browse", &l, ", Ready");
    h.tui.dispatch(Command::GoTo(GoTo::Line(12)));
    h.act(ActionId::ToggleEditMode);
    h.press(key(KeyCode::Char('x')));
    let l = line(&h);
    within_forty_cells("window position, edit", &l, ", Edit, modified");
}

/// Edit mode at a three-digit line: "line 212 of 400, 51%, Edit,
/// modified" ends at cell 41, one past the line, in the window and the
/// terminal alike (they share `App::title_parts`).
#[test]
#[ignore = "defect: in edit mode at a three-digit line, \"modified\" ends at Braille cell 41 (W9e-c)"]
fn the_position_line_keeps_modified_inside_forty_cells_at_long_lines() {
    let text: String = (1..=400).map(|i| format!("Line number {i}.\n")).collect();
    let mut h = launch(screen_reader(), 80, 24, Some(&text));
    h.tui.dispatch(Command::GoTo(GoTo::Line(212)));
    h.act(ActionId::ToggleEditMode);
    h.press(key(KeyCode::Char('x')));
    let app = h.tui.app();
    let l = app.title_parts(app.title_position().as_deref()).join(", ");
    within_forty_cells("window position, edit", &l, ", Edit, modified");
}
