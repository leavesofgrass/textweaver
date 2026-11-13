//! A status-line transcript of the terminal reader (beta 6, the hearing
//! suite): the reader runs as the owner reads with a screen reader and
//! a 40-cell Braille display, in screen-reader mode with the cursor on
//! the status line, through a scripted walk. At every step the
//! transcript records one line in words: the step, the status text, its
//! length in characters (computer Braille is one cell per character),
//! where the step's key fact ends, the first 40 characters, where the
//! hardware cursor is, and whether the message repeats the one before.
//!
//! The transcript is compared with `snapshots/status_transcript.txt` on
//! every run, so a wording change, a cursor that lands somewhere else, or
//! a message said twice shows as one changed line in review. To accept a
//! change on purpose, run the test with `TEXTWEAVER_UPDATE_SNAPSHOTS=1`
//! and review the diff of the snapshot.
//!
//! Deterministic: the reader is drawn at times this test chooses (never
//! the real clock), a scripted source feeds the live entry, the speech
//! service only records, and the keymap is the Windows one on every
//! system, so keys named in messages read the same on macOS and Linux.
//! The settings sections follow the engine and the system, but the only
//! settings tied to a system belong to the Eloquence, SAPI and Apple
//! engines, which never apply to the recording engine used here, so the
//! section counts are the same everywhere.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use textweaver_app::keymap::{ActionId, Frontend, Keymap, Platform};
use textweaver_app::live::{LiveOptions, ScriptedSource};
use textweaver_app::store::{AccessMode as ModeSetting, CursorPlacement, Settings};
use textweaver_app::testing::recording_service;
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig};
use textweaver_tui::Tui;
use textweaver_tui::ui::BRAILLE_CELLS;

const WIDTH: u16 = 80;
const HEIGHT: u16 = 24;

/// The variable that writes the snapshot instead of comparing it.
const UPDATE: &str = "TEXTWEAVER_UPDATE_SNAPSHOTS";

fn snapshot_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("it")
        .join("snapshots")
        .join("status_transcript.txt")
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join("x")
        .join("essay.md")
}

struct Walk {
    tui: Tui,
    term: Terminal<TestBackend>,
    /// The time the next draw is given: the test's own clock.
    clock: Instant,
    /// True when a draw since the last step blanked a repeated message.
    blanked: bool,
    /// The status text recorded at the step before.
    last: String,
    lines: Vec<String>,
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

impl Walk {
    fn new() -> Self {
        let (speech, _log) = recording_service().unwrap();
        let mut settings = Settings::default();
        settings.accessibility.mode = ModeSetting::ScreenReader;
        settings.accessibility.cursor = CursorPlacement::Status;
        let app = App::new(AppConfig {
            settings,
            speech,
            self_voicing: true,
            backend_name: "test-recording".into(),
            // The same keys on every system: a message that names a key
            // reads the same in the snapshot on macOS and Linux.
            keymap: Keymap::defaults(Platform::Windows, Frontend::Terminal),
            ..AppConfig::for_tests()
        });
        let mut w = Walk {
            tui: Tui::with_color_support(app, ColorSupport::NoColor),
            term: Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap(),
            clock: Instant::now(),
            blanked: false,
            last: String::new(),
            lines: Vec::new(),
        };
        w.draw();
        w
    }

    /// One frame at the test's clock, which then moves on a second: past
    /// the blank a repeated message gets, so the next frame shows it.
    fn draw(&mut self) {
        let now = self.clock;
        let tui = &mut self.tui;
        self.term.draw(|f| tui.draw_at(f, now)).unwrap();
        self.blanked |= self.tui.status_blanked();
        self.clock += Duration::from_secs(1);
    }

    fn press(&mut self, k: KeyEvent) {
        self.tui.handle_key(k);
        self.draw();
    }

    fn act(&mut self, a: ActionId) {
        let k = self.tui.key_for(a);
        self.press(k);
    }

    fn typed(&mut self, s: &str) {
        for c in s.chars() {
            self.press(key(KeyCode::Char(c)));
        }
    }

    /// Where the hardware cursor is, in words: the area, then the row
    /// and column inside it, counted from 1.
    fn cursor(&self) -> String {
        let p = self.term.backend().cursor_position();
        let a = self.tui.areas(self.term.backend().buffer().area);
        let named = [
            ("status line", a.status),
            ("title line", a.title),
            ("document", a.body),
            ("Braille row", a.braille),
            ("bottom line", a.bottom),
        ];
        for (name, r) in named {
            if p.y >= r.y && p.y < r.y + r.height && p.x >= r.x && p.x < r.x + r.width {
                return format!("{name}, row {}, column {}", p.y - r.y + 1, p.x - r.x + 1);
            }
        }
        format!("outside every area, row {}, column {}", p.y + 1, p.x + 1)
    }

    /// What the keys go to, in words: a list by its title, a prompt, or
    /// the document.
    fn open(&self) -> String {
        if let Some(l) = self.tui.list() {
            format!("the list {:?}", l.title)
        } else if self.tui.minibuffer().is_some() {
            "a prompt".to_owned()
        } else {
            "the document".to_owned()
        }
    }

    /// Records step `n`: `what` was done, and where `key` (the step's key
    /// fact) ends in the status text.
    fn step(&mut self, n: usize, what: &str, key: &str) {
        // One more frame, so a repeated message drawn blank shows again.
        self.draw();
        let text = self.tui.status_line();
        let len = text.chars().count();
        let first: String = text.chars().take(BRAILLE_CELLS).collect();
        let fact = match (key, text.find(key)) {
            ("", _) => "No key fact checked".to_owned(),
            (_, Some(at)) => {
                let end = text[..at + key.len()].chars().count();
                let place = if end <= BRAILLE_CELLS {
                    "within 40"
                } else {
                    "past 40"
                };
                format!("Key fact {key:?} ends at character {end}, {place}")
            }
            (_, None) => format!("Key fact {key:?} missing"),
        };
        let repeat = if text == self.last && !text.is_empty() {
            if self.blanked {
                "repeat, blanked first"
            } else {
                "repeat, not blanked"
            }
        } else {
            "new"
        };
        self.lines.push(format!(
            "{n}. {what}. Status: {text:?}. {len} characters. {fact}. \
             First 40: {first:?}. Cursor: {}. On: {}. Message: {repeat}.",
            self.cursor(),
            self.open()
        ));
        self.last = text;
        self.blanked = false;
    }
}

/// The walk, as one transcript.
fn transcript() -> String {
    let mut w = Walk::new();
    let mut n = 0;
    let mut next = || {
        n += 1;
        n
    };

    // 1. Open a fixture.
    w.tui.app_mut().open(&fixture()).unwrap();
    w.draw();
    w.step(next(), "Open the essay fixture", "Opened");

    // 2. Move by heading, twice.
    w.act(ActionId::SkipNextHeading);
    w.step(next(), "Next heading", "Chapter one");
    w.act(ActionId::SkipNextHeading);
    w.step(next(), "Next heading again", "Chapter two");

    // 3. Open the outline, move in it, and close it.
    w.act(ActionId::Outline);
    w.step(next(), "Open the outline", "Outline");
    w.press(key(KeyCode::Down));
    w.step(next(), "Down in the outline", " of 6");
    w.press(key(KeyCode::Esc));
    w.step(next(), "Close the outline with Escape", "");

    // 4. Find a word, then the next match.
    w.act(ActionId::Find);
    w.step(next(), "Open Find", "Find");
    w.typed("chapter");
    w.press(key(KeyCode::Enter));
    w.step(next(), "Type chapter, then Enter", "Match");
    w.act(ActionId::FindNext);
    w.step(next(), "Find next", "Match");

    // 5. Open Settings and leave.
    w.act(ActionId::Settings);
    w.step(next(), "Open Settings", "Settings");
    w.press(key(KeyCode::Down));
    w.step(next(), "Down in Settings", " of ");
    w.press(key(KeyCode::Esc));
    w.step(next(), "Leave Settings with Escape", "Settings closed");

    // 6. A list: add two bookmarks, list them, and move in the list.
    w.act(ActionId::AddBookmark);
    w.step(next(), "Add a bookmark", "Bookmark");
    w.act(ActionId::SkipNextHeading);
    w.act(ActionId::AddBookmark);
    w.step(next(), "Next heading, then add a bookmark", "Bookmark");
    w.act(ActionId::ListBookmarks);
    w.step(next(), "List bookmarks", "Bookmarks");
    w.press(key(KeyCode::Down));
    w.step(next(), "Down in the bookmarks", "2 of 2");
    w.press(key(KeyCode::Esc));
    w.step(next(), "Close the bookmarks with Escape", "");

    // 7. A live entry from the scripted source.
    let (source, feed) = ScriptedSource::new();
    w.tui.app_mut().set_live_announce_burst(Duration::ZERO);
    w.tui.app_mut().open_live_source(
        Box::new(source),
        LiveOptions {
            id: "club".into(),
            title: "Club log".into(),
            ..LiveOptions::default()
        },
    );
    w.draw();
    w.step(next(), "Open a scripted live log", "Club log");
    feed.push_json(
        "club/log",
        r#"{"callsign":"AB1CDE","name":"Ada Example","notes":"Mobile."}"#,
    );
    w.tui.tick();
    w.draw();
    w.step(next(), "A live entry arrives", "AB1CDE");

    // 8. Repeat the last message.
    w.act(ActionId::RepeatMessage);
    w.step(next(), "Repeat the last message", "AB1CDE");

    let mut out = String::from(
        "Status-line transcript of the terminal reader: screen-reader mode, \
         cursor on the status line, 80 by 24.\n\
         One line per step. Written by tests/it/status_transcript.rs; \
         update with TEXTWEAVER_UPDATE_SNAPSHOTS=1.\n\n",
    );
    for l in &w.lines {
        out.push_str(l);
        out.push('\n');
    }
    out
}

/// Line endings do not matter (Git may check the file out with CRLF).
fn normalized(s: &str) -> String {
    s.replace("\r\n", "\n")
}

/// Compares `fresh` with the snapshot at `path`, or writes it when
/// [`UPDATE`] is 1; fails with the first line that differs.
fn compare(path: &Path, fresh: &str) {
    if std::env::var_os(UPDATE).is_some_and(|v| v == "1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, fresh).expect("the snapshot is writable");
        return;
    }
    let current = normalized(&std::fs::read_to_string(path).unwrap_or_default());
    if current == fresh {
        return;
    }
    let (n, (old, new)) = current
        .lines()
        .chain(std::iter::repeat(""))
        .zip(fresh.lines().chain(std::iter::repeat("")))
        .enumerate()
        .find(|(_, (a, b))| a != b)
        .unwrap_or((0, ("", "")));
    panic!(
        "the snapshot {} differs from the walk; if the change is meant, run \
         with {UPDATE}=1 and review the diff.\nFirst difference, line {}:\n  \
         snapshot: {old}\n  now:      {new}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        n + 1
    );
}

/// The walk gives the transcript in the snapshot, line for line.
#[test]
fn the_status_line_transcript_matches_its_snapshot() {
    let fresh = transcript();
    compare(&snapshot_path(), &fresh);
}

/// The walk is the same twice in one run: nothing in it depends on the
/// clock or on the order tests run in.
#[test]
fn the_status_line_transcript_is_deterministic() {
    assert_eq!(transcript(), transcript());
}

/// The transcript holds no machine path: the fixture is named by its
/// title, never by where the repository is.
#[test]
fn the_status_line_transcript_holds_no_machine_path() {
    let t = transcript();
    assert!(!t.contains(env!("CARGO_MANIFEST_DIR")), "{t}");
    assert!(!t.contains("fixtures"), "{t}");
}
