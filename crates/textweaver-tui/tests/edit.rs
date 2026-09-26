//! Scripted edit-mode test on ratatui's `TestBackend` (Agent D2's
//! acceptance test): open a Markdown file, enter edit mode with its real
//! key, type, select and format, undo, save, leave, quit, relaunch, and
//! reopen; with the hardware cursor on the caret and every step on the
//! status line. Also notes through their keys and list Delete.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use textweaver_app::core::CharPos;
use textweaver_app::store::{Paths, SettingsStore};
use textweaver_app::testing::{SpeechLog, recording_service};
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig, Mode};
use textweaver_tui::Tui;

const WIDTH: u16 = 80;
const HEIGHT: u16 = 20;
const SOURCE: &str = "# Notes\n\nPlain words here.\n";

struct Harness {
    tui: Tui,
    term: Terminal<TestBackend>,
    log: SpeechLog,
}

fn launch(home: &Path) -> Harness {
    let paths = Paths::under(home);
    let (settings, _) = SettingsStore::new(paths.clone()).load();
    let (speech, log) = recording_service().unwrap();
    let app = App::new(AppConfig {
        settings,
        speech,
        paths: Some(paths),
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    let mut h = Harness {
        tui: Tui::with_color_support(app, ColorSupport::TrueColor),
        term: Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap(),
        log,
    };
    h.draw();
    h
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

fn shift(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::SHIFT)
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
            let mods = if c.is_ascii_uppercase() {
                KeyModifiers::SHIFT
            } else {
                KeyModifiers::NONE
            };
            self.press(KeyEvent::new(KeyCode::Char(c), mods));
        }
    }

    fn app(&self) -> &App {
        self.tui.app()
    }

    fn text(&self) -> String {
        self.app().session().unwrap().doc.text().to_string()
    }

    fn row_text(&self, y: u16) -> String {
        let buf = self.term.backend().buffer();
        (0..WIDTH).map(|x| buf[(x, y)].symbol()).collect()
    }

    fn screen(&self) -> String {
        (0..HEIGHT)
            .map(|y| self.row_text(y))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn status(&self) -> String {
        let size = self.term.backend().buffer().area;
        let a = self.tui.areas(size).status;
        (a.y..a.y + a.height)
            .map(|y| self.row_text(y))
            .collect::<Vec<_>>()
            .join("")
    }

    /// The hardware cursor is on the editor caret: the screen text from
    /// the cursor starts with the document text from the caret.
    fn assert_cursor_on_caret(&self) {
        let s = self.app().session().unwrap();
        let caret = s.cursor;
        let p = self.term.backend().cursor_position();
        let row = self.row_text(p.y);
        let shown: String = row.chars().skip(usize::from(p.x)).collect();
        let line = textweaver_app::text_util::line_of(&s.doc, caret);
        let end = textweaver_app::text_util::line_range(&s.doc, line).end;
        let expected = s
            .doc
            .slice(textweaver_app::core::CharRange::new(caret, end));
        assert!(
            shown.starts_with(expected.trim_end()),
            "cursor {p:?} shows {shown:?}, caret text {expected:?}"
        );
    }

    /// Applies speech status until the speech thread has recorded what
    /// `done` waits for (or ten seconds pass). A fixed 200 ms wait failed on
    /// a busy machine.
    fn settle(&mut self, done: impl Fn(&[String]) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            self.tui.tick();
            if done(&self.log.texts()) || Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        self.draw();
    }
}

fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("notes.md");
    std::fs::write(&file, SOURCE).unwrap();
    let home = dir.path().join("home");
    (dir, file, home)
}

#[test]
fn type_format_undo_save_and_reopen_through_keys() {
    let (_dir, file, home) = setup();
    let mut h = launch(&home);
    h.tui.app_mut().open(&file).unwrap();
    h.draw();
    // Reading view: no Markdown markup on screen.
    assert!(!h.screen().contains("# Notes"));

    // Ctrl+E: the source, the Edit mode in the title, the status line.
    h.press(ctrl('e'));
    assert_eq!(h.app().mode(), Mode::Edit);
    assert!(h.screen().contains("# Notes"), "{}", h.screen());
    assert!(h.row_text(0).contains("Edit"));
    assert!(h.status().contains("Edit mode on"), "{}", h.status());
    // The cursor was on "Notes"; in the source that is after "# ".
    assert_eq!(h.app().session().unwrap().cursor, CharPos(2));
    h.assert_cursor_on_caret();

    // Down twice to "Plain words here.", End, then type a sentence.
    h.press(key(KeyCode::Down));
    h.press(key(KeyCode::Down));
    h.log.clear();
    h.press(key(KeyCode::End));
    h.typed(" More text");
    h.settle(|spoken| spoken.iter().any(|t| t == "M") && spoken.iter().any(|t| t == "More"));
    assert_eq!(h.text(), "# Notes\n\nPlain words here. More text\n");
    assert!(h.row_text(0).contains("modified"), "{}", h.row_text(0));
    h.assert_cursor_on_caret();
    let spoken = h.log.texts();
    assert!(spoken.iter().any(|t| t == "M"), "{spoken:?}");
    assert!(spoken.iter().any(|t| t == "More"), "{spoken:?}");

    // Shift+Ctrl+Left selects "text"; Ctrl+B bolds it.
    h.press(KeyEvent::new(
        KeyCode::Left,
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    ));
    assert!(h.status().contains("text selected"), "{}", h.status());
    h.press(ctrl('b'));
    assert_eq!(h.text(), "# Notes\n\nPlain words here. More **text**\n");
    assert!(h.status().starts_with("Bold."), "{}", h.status());
    assert!(h.screen().contains("More **text**"));

    // Ctrl+Z undoes the bold; Ctrl+Y redoes it; Ctrl+Z again.
    h.press(ctrl('z'));
    assert_eq!(h.text(), "# Notes\n\nPlain words here. More text\n");
    assert!(h.status().starts_with("Undo."));
    h.press(ctrl('y'));
    assert!(h.text().contains("**text**"));
    h.press(ctrl('z'));

    // Backspace and Enter type as expected.
    h.press(key(KeyCode::End));
    h.press(key(KeyCode::Backspace));
    h.press(key(KeyCode::Enter));
    h.typed("Last.");
    assert_eq!(h.text(), "# Notes\n\nPlain words here. More tex\nLast.\n");

    // Ctrl+S saves in place and keeps editing.
    h.press(ctrl('s'));
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "# Notes\n\nPlain words here. More tex\nLast.\n"
    );
    assert!(h.status().contains("Saved notes.md. Still editing."));
    assert!(!h.row_text(0).contains("modified"));

    // Ctrl+E leaves; the reading view shows the new text without markup.
    h.press(ctrl('e'));
    assert_eq!(h.app().mode(), Mode::Browse);
    assert!(h.status().contains("Edit mode off."));
    assert!(h.screen().contains("More tex"));
    assert!(!h.screen().contains("# Notes"));

    // Quit and relaunch: the saved text is what opens.
    h.press(ctrl('q'));
    assert!(!h.tui.should_quit());
    h.press(key(KeyCode::Char('y')));
    assert!(h.tui.should_quit());
    let mut h = launch(&home);
    h.tui.app_mut().open(&file).unwrap();
    h.draw();
    assert!(h.screen().contains("Plain words here. More tex"));
    assert!(h.screen().contains("Last."));
}

#[test]
fn quitting_with_unsaved_edits_asks_in_a_list() {
    let (_dir, file, home) = setup();
    let mut h = launch(&home);
    h.tui.app_mut().open(&file).unwrap();
    h.press(ctrl('e'));
    h.typed("x");
    h.press(ctrl('q'));
    h.press(key(KeyCode::Char('y')));
    assert!(!h.tui.should_quit());
    let list = h.tui.list().expect("save choice list");
    assert_eq!(list.items.len(), 3);
    assert!(h.screen().contains("Save changes to Notes?"));
    // Down to "Discard", Enter: quits without saving.
    h.press(key(KeyCode::Down));
    assert!(h.status().contains("Discard"), "{}", h.status());
    h.press(key(KeyCode::Enter));
    assert!(h.tui.should_quit());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), SOURCE);
}

#[test]
fn notes_by_key_and_delete_from_the_list() {
    let (_dir, file, home) = setup();
    let mut h = launch(&home);
    h.tui.app_mut().open(&file).unwrap();
    h.draw();
    // "a" opens the note prompt; the answer adds the note.
    h.typed("a");
    assert!(h.tui.minibuffer().is_some());
    h.typed("Remember this");
    h.press(key(KeyCode::Enter));
    assert!(h.status().starts_with("Note added"), "{}", h.status());
    assert_eq!(h.app().session().unwrap().notes.len(), 1);
    // "A" lists the notes; Delete asks, and y removes the focused one and
    // closes the emptied list.
    h.typed("A");
    assert!(h.tui.list().is_some());
    assert!(h.screen().contains("Remember this"));
    h.press(key(KeyCode::Delete));
    assert!(
        h.status().contains("Delete this note? y or n"),
        "{}",
        h.status()
    );
    assert_eq!(h.app().session().unwrap().notes.len(), 1);
    h.typed("y");
    assert!(h.tui.list().is_none());
    assert!(h.status().starts_with("Note deleted"), "{}", h.status());
    assert!(h.app().session().unwrap().notes.is_empty());
}

#[test]
fn bracketed_paste_inserts_as_one_step() {
    let (_dir, file, home) = setup();
    let mut h = launch(&home);
    h.tui.app_mut().open(&file).unwrap();
    h.press(ctrl('e'));
    h.tui.handle_event(&Event::Paste("Pasted\r\nlines ".into()));
    h.draw();
    // The caret was on "Notes" (after "# "); CRLF became one line break.
    assert_eq!(h.text(), "# Pasted\nlines Notes\n\nPlain words here.\n");
    assert!(
        h.status().contains("Pasted 13 characters: Pasted lines"),
        "{}",
        h.status()
    );
    h.press(ctrl('z'));
    assert_eq!(h.text(), SOURCE);
    // Shift+Left in edit mode selects a character.
    h.press(shift(KeyCode::Left));
    assert!(h.status().contains("selected"), "{}", h.status());
}
