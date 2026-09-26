//! Scripted TUI test on ratatui's `TestBackend` (the Agent D acceptance
//! test): open a fixture, navigate by each unit with real keys, read with a
//! recording speech backend while checking that every highlight drawn on
//! screen is exactly the range the speech service reported and lies inside
//! what was spoken, use the Speech Cursor, find, the palette, help, and
//! themes, quit, relaunch with the same state directory, and check the
//! restored position.
//!
//! Expected positions are computed with the text crate's own navigation
//! functions on the loaded document, so the test keeps passing when Agent A
//! replaces the unit rules.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Position;
use textweaver_app::core::{CharPos, CharRange, Direction, Unit};
use textweaver_app::store::{Paths, SettingsStore};
use textweaver_app::testing::{SpeechLog, recording_service};
use textweaver_app::text::units::unit_at;
use textweaver_app::text::{NavOptions, navigate};
use textweaver_app::text_util::first_word_at_or_after;
use textweaver_app::{App, AppConfig, Mode, Playback};
use textweaver_tui::{Theme, Tui};

const WIDTH: u16 = 80;
const HEIGHT: u16 = 24;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/d/reading.txt")
}

struct Harness {
    tui: Tui,
    term: Terminal<TestBackend>,
    log: SpeechLog,
}

fn launch(home: &Path) -> Harness {
    let paths = Paths::under(home);
    let (settings, msg) = SettingsStore::new(paths.clone()).load();
    assert!(msg.is_none());
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
        tui: Tui::new(app),
        term: Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap(),
        log,
    };
    h.tui.app_mut().open(&fixture()).unwrap();
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

    fn typed(&mut self, s: &str) {
        for c in s.chars() {
            self.press(ch(c));
        }
    }

    fn app(&self) -> &App {
        self.tui.app()
    }

    fn doc(&self) -> &textweaver_app::text::Document {
        &self.app().session().unwrap().doc
    }

    fn cursor(&self) -> CharPos {
        self.app().session().unwrap().cursor
    }

    fn row_text(&self, y: u16) -> String {
        let buf = self.term.backend().buffer();
        (0..WIDTH).map(|x| buf[(x, y)].symbol()).collect()
    }

    fn areas(&self) -> textweaver_tui::ui::Areas {
        let size = self.term.backend().buffer().area;
        self.tui.areas(size)
    }

    /// The status line's text (it grows to three rows for long messages).
    fn status(&self) -> String {
        let a = self.areas().status;
        (a.y..a.y + a.height)
            .map(|y| self.row_text(y))
            .collect::<Vec<_>>()
            .join("")
    }

    fn screen_cursor(&self) -> Position {
        self.term.backend().cursor_position()
    }

    /// The hardware cursor sits on the char at `pos`, and the screen text
    /// from there starts with the word at `pos`.
    fn assert_cursor_on(&self, pos: CharPos) {
        let p = self.screen_cursor();
        let row = self.row_text(p.y);
        let from_cursor: String = row.chars().skip(usize::from(p.x)).collect();
        let word = unit_at(self.doc(), pos, Unit::Word)
            .filter(|w| w.start == pos)
            .map(|w| self.doc().slice(w))
            .unwrap_or_else(|| {
                self.doc()
                    .char_at(pos)
                    .map(String::from)
                    .unwrap_or_default()
            });
        assert!(
            from_cursor.starts_with(&word),
            "cursor at {p:?} shows {from_cursor:?}, expected {word:?} (pos {pos})"
        );
    }

    fn nav(&self, from: CharPos, unit: Unit, dir: Direction) -> CharPos {
        navigate(self.doc(), from, unit, dir, NavOptions::default())
            .unwrap()
            .range
            .start
    }

    /// The words of every body cell drawn in the spoken-word style, in
    /// reading order (rows joined by a space, whitespace collapsed).
    fn spoken_cells(&self) -> String {
        let theme = self.tui.theme();
        let bg = theme.spoken_word.bg;
        let buf = self.term.backend().buffer();
        let body = self.areas().body;
        let mut rows = Vec::new();
        for y in body.y..body.y + body.height {
            let row: String = (body.x..body.x + body.width)
                .filter(|&x| Some(buf[(x, y)].bg) == bg)
                .map(|x| buf[(x, y)].symbol())
                .collect();
            rows.push(row);
        }
        rows.join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Applies speech status one update at a time, redrawing after each,
    /// and checks the screen at every highlight step. Returns the ranges
    /// highlighted.
    fn follow_speech(&mut self) -> Vec<CharRange> {
        let mut seen = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match self.tui.app_mut().poll_speech_step() {
                Some(true) => {
                    self.draw();
                    let spoken = self.app().session().and_then(|s| s.spoken);
                    if let Some(r) = spoken {
                        seen.push(r);
                        // Drawn exactly: the styled cells are the range.
                        let expected = self
                            .doc()
                            .slice(r)
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ");
                        assert_eq!(self.spoken_cells(), expected, "highlight {r}");
                        // The hardware cursor follows the spoken word.
                        self.assert_cursor_on(r.start);
                    }
                }
                Some(false) => {}
                None if self.app().playback() == Playback::Idle => break,
                None => {
                    assert!(Instant::now() < deadline, "speech never finished");
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
        }
        self.draw();
        seen
    }

    /// Every highlight lies inside a range the backend was asked to speak,
    /// and every spoken range was highlighted.
    fn assert_highlights_match_speech(&self, highlights: &[CharRange], since: usize) {
        let spoken: Vec<CharRange> = self.log.spoken_ranges().into_iter().skip(since).collect();
        assert!(!highlights.is_empty());
        for h in highlights {
            assert!(
                spoken.iter().any(|s| s.contains_range(*h)),
                "highlight {h} not inside any spoken range {spoken:?}"
            );
        }
        for s in &spoken {
            assert!(
                highlights.iter().any(|h| s.contains_range(*h)),
                "spoken {s} never highlighted"
            );
        }
    }
}

#[test]
fn scripted_session_with_restore() {
    let home = tempfile::tempdir().unwrap();
    let mut h = launch(home.path());

    // Opened: title, status, and the cursor on the first word.
    assert!(h.row_text(0).contains("textweaver: reading.txt"));
    assert!(h.status().contains("Opened reading.txt."), "{}", h.status());
    assert_eq!(h.cursor(), CharPos(0));
    h.assert_cursor_on(CharPos(0));
    assert!(h.row_text(HEIGHT - 1).contains("quit"));

    // Word by word with the arrow keys; each word is spoken.
    let second = h.nav(CharPos(0), Unit::Word, Direction::Forward);
    h.press(key(KeyCode::Right));
    assert_eq!(h.cursor(), second);
    h.assert_cursor_on(second);
    assert!(h.status().starts_with("Quiet"));
    h.press(key(KeyCode::Left));
    assert_eq!(h.cursor(), CharPos(0));

    // Line by line: blank lines are skipped by the caret.
    h.press(key(KeyCode::Down));
    let alice = h.nav(CharPos(0), Unit::Paragraph, Direction::Forward);
    assert_eq!(h.cursor(), alice);
    h.assert_cursor_on(alice);

    // Sentences, with the key the keymap binds.
    let s2 = h.nav(alice, Unit::Sentence, Direction::Forward);
    h.press(ch('.'));
    assert_eq!(h.cursor(), s2);
    h.assert_cursor_on(s2);
    let s3 = h.nav(s2, Unit::Sentence, Direction::Forward);
    h.press(ch('.'));
    assert_eq!(h.cursor(), s3);
    // Previous at a sentence start goes to the previous sentence.
    h.press(ch(','));
    assert_eq!(h.cursor(), s2);

    // Paragraphs.
    let p3 = h.nav(s2, Unit::Paragraph, Direction::Forward);
    h.press(ch('p'));
    assert_eq!(h.cursor(), p3);
    h.assert_cursor_on(p3);
    h.press(ch('P'));
    assert_eq!(h.cursor(), alice);

    // Headings: plain text has none, and says so.
    h.press(ch('h'));
    assert_eq!(h.cursor(), alice);
    assert!(h.status().starts_with("No next heading."), "{}", h.status());

    // Read the current word, sentence, and line in place, checking the
    // highlight on screen at every step.
    let mut read_since = h.log.spoken_ranges().len();
    for c in ['w', 's', 'l'] {
        h.press(ch(c));
        let seen = h.follow_speech();
        h.assert_highlights_match_speech(&seen, read_since);
        read_since = h.log.spoken_ranges().len();
        assert_eq!(h.cursor(), alice, "reading in place does not move");
    }
    // The character at the cursor is spoken as a character.
    h.press(ch('c'));
    let deadline = Instant::now() + Duration::from_secs(10);
    while !h.log.texts().iter().any(|t| t == "A") {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }

    // Read on from the cursor to the end.
    let since = h.log.spoken_ranges().len();
    h.press(key(KeyCode::Enter));
    assert_eq!(h.app().playback(), Playback::Reading);
    let seen = h.follow_speech();
    h.assert_highlights_match_speech(&seen, since);
    assert!(seen.windows(2).all(|w| w[0].start <= w[1].start));
    let last = *seen.last().unwrap();
    assert_eq!(h.cursor(), last.start, "the cursor followed speech");
    h.assert_cursor_on(last.start);

    // Back to the top.
    h.press(key(KeyCode::Home));
    assert_eq!(h.cursor(), CharPos(0));
    h.assert_cursor_on(CharPos(0));

    // Speech Cursor: Tab enters, Down reads the next line ("blank").
    h.press(key(KeyCode::Tab));
    assert_eq!(h.app().mode(), Mode::SpeechCursor);
    assert!(h.row_text(0).contains("Speech Cursor"));
    assert!(h.row_text(HEIGHT - 1).contains("next line"));
    h.follow_speech();
    h.press(key(KeyCode::Down));
    let deadline = Instant::now() + Duration::from_secs(10);
    while !h.log.texts().iter().any(|t| t == "blank") {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    h.press(key(KeyCode::Down));
    h.follow_speech();
    assert_eq!(h.app().session().unwrap().speech_cursor_line, Some(2));
    // The hardware cursor is on the Speech Cursor line's start.
    h.assert_cursor_on(alice);
    h.press(key(KeyCode::Tab));
    assert_eq!(h.app().mode(), Mode::Browse);

    // Find through the minibuffer.
    h.press(ch('/'));
    assert!(h.row_text(HEIGHT - 1).starts_with("Find: "));
    h.typed("books");
    assert!(h.row_text(HEIGHT - 1).starts_with("Find: books"));
    assert_eq!(h.screen_cursor(), Position::new(11, HEIGHT - 1));
    h.press(key(KeyCode::Enter));
    let books = CharPos(h.doc().text().to_string().find("books").unwrap());
    assert_eq!(h.cursor(), books);
    h.assert_cursor_on(books);
    assert!(h.status().starts_with("Match 1 of 1"), "{}", h.status());
    let hit = h.term.backend().buffer()[(h.screen_cursor().x, h.screen_cursor().y)].bg;
    assert_eq!(Some(hit), h.tui.theme().current_hit.bg);

    // A bookmark here.
    h.press(ch('m'));
    assert!(h.status().starts_with("Bookmark mark1 set at"));

    // The command palette runs a command by name.
    h.press(key(KeyCode::F(2)));
    assert!(h.row_text(HEIGHT - 1).starts_with("Command: "));
    h.typed("next sentence");
    h.press(key(KeyCode::Enter));
    let after = h.nav(books, Unit::Sentence, Direction::Forward);
    assert_eq!(h.cursor(), after);

    // Keyboard help from the keymap, navigable and announced.
    h.press(ch('?'));
    assert!(h.tui.list().is_some());
    assert!(
        (1..HEIGHT).any(|y| h.row_text(y).contains("Keyboard shortcuts")),
        "help overlay title"
    );
    h.press(key(KeyCode::Down));
    let item = h.tui.list().unwrap().current().unwrap().to_owned();
    assert!(h.status().starts_with(&item[..20]), "{}", h.status());
    let p = h.screen_cursor();
    assert!(h.row_text(p.y).contains(&item[..20]), "cursor on the item");
    h.press(key(KeyCode::Esc));
    assert!(h.tui.list().is_none());

    // Themes change the drawing.
    h.press(key(KeyCode::F(5)));
    assert_eq!(h.app().settings().display.theme, "light");
    let title_bg = h.term.backend().buffer()[(0, 0)].bg;
    assert_eq!(Some(title_bg), Theme::light().title.bg);

    // Quit saves the position.
    let saved = h.cursor();
    h.press(ch('q'));
    assert!(h.tui.should_quit());
    drop(h);

    // Relaunch with the same state directory: the position comes back.
    let mut h = launch(home.path());
    let expected = first_word_at_or_after(h.doc(), saved);
    assert_eq!(h.cursor(), expected);
    h.assert_cursor_on(expected);
    assert!(h.status().contains("Resumed at"), "{}", h.status());
    assert_eq!(h.app().settings().display.theme, "light", "settings saved");
    assert_eq!(h.app().session().unwrap().bookmarks.len(), 1);
    // The bookmark is drawn.
    h.press(ch('B'));
    assert_eq!(h.cursor(), books);
}

#[test]
fn prompts_cancel_and_recall() {
    let home = tempfile::tempdir().unwrap();
    let mut h = launch(home.path());
    h.press(ch('/'));
    h.typed("zzz");
    h.press(key(KeyCode::Esc));
    assert!(h.tui.minibuffer().is_none());
    assert!(h.status().starts_with("Cancelled."));
    h.press(ch('/'));
    h.typed("light");
    h.press(key(KeyCode::Enter));
    // Up recalls the last answer.
    h.press(ch('/'));
    h.press(key(KeyCode::Up));
    assert_eq!(h.tui.minibuffer().unwrap().text(), "light");
    h.press(key(KeyCode::Esc));
    // Go to a line through the prompt (Ctrl+G).
    h.press(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL));
    assert!(h.row_text(HEIGHT - 1).starts_with("Go to"));
    h.typed("7");
    h.press(key(KeyCode::Enter));
    let line7 = CharPos(h.doc().text().line_to_char(6));
    assert_eq!(h.cursor(), line7);
    h.assert_cursor_on(line7);
    // Palette completion with Tab.
    h.press(key(KeyCode::F(2)));
    h.typed("toggle_line");
    h.press(key(KeyCode::Tab));
    assert_eq!(h.tui.minibuffer().unwrap().text(), "toggle_line_numbers");
    h.press(key(KeyCode::Enter));
    assert!(h.app().settings().display.show_line_numbers);
    // With line numbers the text shifts right past the gutter.
    assert!(
        h.row_text(1).starts_with("1 The Quiet Library"),
        "{}",
        h.row_text(1)
    );
}

#[test]
fn narrow_terminal_wraps_and_keeps_the_cursor_visible() {
    let home = tempfile::tempdir().unwrap();
    let mut h = launch(home.path());
    h.term.backend_mut().resize(30, 8);
    h.draw();
    for _ in 0..6 {
        h.press(ch('.'));
        let p = h.screen_cursor();
        let body = h.areas().body;
        assert!(
            p.y >= body.y && p.y < body.y + body.height,
            "cursor {p:?} inside the body {body:?}"
        );
        let c = h.cursor();
        let row: String = {
            let buf = h.term.backend().buffer();
            (p.x..30).map(|x| buf[(x, p.y)].symbol()).collect()
        };
        let word = unit_at(h.doc(), c, Unit::Word)
            .map(|w| h.doc().slice(w))
            .unwrap();
        assert!(row.starts_with(&word), "{row:?} vs {word:?}");
    }
}
