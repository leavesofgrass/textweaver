//! The terminal reader with a live document open (beta 5, N8), for a
//! screen reader and a 40-cell Braille display: the status line carries
//! the newest entry, meaning first, inside one Braille line; the title
//! line's position does not change with every entry; a new entry draws
//! once and an idle reader draws nothing; the reader's row, the rows
//! above it and the hardware cursor stay; entries arrive within one pass
//! of the event loop; and a saved JSON log opens as a live log.
//!
//! A scripted source feeds the log: no network, no audio.

use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use textweaver_app::a11y::Priority;
use textweaver_app::live::{LiveOptions, ScriptHandle, ScriptedSource};
use textweaver_app::store::{AccessMode as ModeSetting, CursorPlacement, Settings};
use textweaver_app::testing::recording_service;
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig, Command};
use textweaver_tui::Tui;
use textweaver_tui::ui::{BRAILLE_CELLS, braille_cells, live_status_line, trim_to_cells};

struct Harness {
    tui: Tui,
    term: Terminal<TestBackend>,
    width: u16,
    feed: ScriptHandle,
}

fn app(cursor: CursorPlacement) -> App {
    let mut settings = Settings::default();
    settings.accessibility.mode = ModeSetting::ScreenReader;
    settings.accessibility.cursor = cursor;
    let (speech, _log) = recording_service().unwrap();
    App::new(AppConfig {
        settings,
        speech,
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    })
}

/// A live log fed by a scripted source, on an 80 by 24 screen.
fn launch(cursor: CursorPlacement) -> Harness {
    let mut app = app(cursor);
    // Each entry is sent at once here; the announcements would say
    // entries this close together as one.
    app.set_live_announce_burst(Duration::ZERO);
    let (source, feed) = ScriptedSource::new();
    app.open_live_source(
        Box::new(source),
        LiveOptions {
            id: "club".into(),
            title: "Club log".into(),
            ..LiveOptions::default()
        },
    );
    let (width, height) = (80, 24);
    let mut h = Harness {
        tui: Tui::with_color_support(app, ColorSupport::NoColor),
        term: Terminal::new(TestBackend::new(width, height)).unwrap(),
        width,
        feed,
    };
    h.draw();
    h
}

fn entry(call: &str, name: &str, notes: &str) -> String {
    format!(r#"{{"callsign":"{call}","name":"{name}","notes":"{notes}"}}"#)
}

impl Harness {
    /// Draws a moment later, so a repeated message is not blank.
    fn draw(&mut self) {
        let later = Instant::now() + Duration::from_secs(2);
        let tui = &mut self.tui;
        self.term.draw(|f| tui.draw_at(f, later)).unwrap();
    }

    /// One message from the source, taken in by a tick, then drawn.
    fn send(&mut self, json: &str) {
        self.feed.push_json("club/log", json);
        self.tui.tick();
        self.draw();
    }

    fn row(&self, y: u16) -> String {
        let buf = self.term.backend().buffer();
        (0..self.width).map(|x| buf[(x, y)].symbol()).collect()
    }

    fn title(&self) -> String {
        self.row(0).trim_end().to_owned()
    }

    fn body(&self) -> Vec<String> {
        let a = self.tui.areas(self.term.backend().buffer().area).body;
        (a.y..a.y + a.height)
            .map(|y| self.row(y).trim_end().to_owned())
            .collect()
    }

    /// The status area's first row: what a 40-cell display shows first.
    fn status_first_row(&self) -> String {
        let a = self.tui.areas(self.term.backend().buffer().area).status;
        self.row(a.y).trim_end().to_owned()
    }

    fn cursor(&self) -> (u16, u16) {
        let p = self.term.backend().cursor_position();
        (p.x, p.y)
    }

    fn go(&mut self, needle: &str) {
        let t = self.tui.app().session().unwrap().doc.text().to_string();
        let b = t.find(needle).unwrap();
        let p = textweaver_app::core::CharPos(t[..b].chars().count());
        self.tui.dispatch(Command::SetCursor(p));
        self.draw();
    }

    fn press(&mut self, code: KeyCode) {
        self.tui.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
        self.draw();
    }
}

/// Asserts `line` fits one Braille line.
fn fits(line: &str) {
    let cells = braille_cells(line);
    assert!(
        cells <= BRAILLE_CELLS,
        "{cells} cells, past {BRAILLE_CELLS}: {line:?}"
    );
}

/// A long entry is cut at a word boundary to 40 cells, meaning first,
/// with "..." where words were left out; a short one is whole, with the
/// unread count after it.
#[test]
fn the_status_line_carries_the_newest_entry_in_forty_cells() {
    let mut h = launch(CursorPlacement::Status);
    h.send(&entry(
        "N0CALL",
        "Ada Example",
        "Mobile, portable antenna, running on battery power tonight.",
    ));
    let s = h.status_first_row();
    assert!(s.starts_with("N0CALL, Ada Example. Mobile"), "{s}");
    assert!(s.ends_with("..."), "the cut is marked: {s}");
    assert!(!s.contains("tonight"), "{s}");
    fits(&s);
    // The cursor waits at the start of the status line.
    let a = h.tui.areas(h.term.backend().buffer().area).status;
    assert_eq!(h.cursor(), (a.x, a.y));

    // A short entry is whole, and the unread count follows where it fits.
    h.send(&entry("N1CALL", "Bo", ""));
    let s = h.status_first_row();
    assert!(s.starts_with("N1CALL, Bo."), "{s}");
    assert!(s.ends_with("2 unread"), "{s}");
    fits(&s);
}

/// A correction (the same callsign again) leads the status line, and the
/// log keeps one entry for it.
#[test]
fn a_correction_leads_the_status_line() {
    let mut h = launch(CursorPlacement::Status);
    h.send(&entry("N0CALL", "Ada", "Mobile."));
    h.send(&entry("N1CALL", "Bo", "Fixed."));
    h.send(&entry("N0CALL", "Ada", "Portable."));
    assert_eq!(h.tui.app().live_log().unwrap().len(), 2);
    let s = h.status_first_row();
    assert!(s.starts_with("N0CALL, Ada. Portable."), "{s}");
    fits(&s);
    let body = h.body().join("\n");
    assert!(body.contains("N0CALL, Ada. Portable."), "{body}");
    assert!(!body.contains("Mobile."), "{body}");
}

/// The status line is the reader's when they use it: another message
/// stays until the next entry, and a pending message is not trimmed.
#[test]
fn another_message_is_left_alone_until_the_next_entry() {
    let mut h = launch(CursorPlacement::Status);
    h.send(&entry("N0CALL", "Ada", "Mobile."));
    let msg = "A message of the reader's own, long enough to need more than forty cells.";
    h.tui.app_mut().announce(msg, Priority::Polite);
    h.draw();
    assert!(
        h.tui.status_line().starts_with(msg),
        "{}",
        h.tui.status_line()
    );
    // A tick with nothing new leaves it.
    h.tui.tick();
    h.draw();
    assert!(h.tui.status_line().starts_with(msg));
    h.send(&entry("N1CALL", "Bo", "Fixed."));
    assert!(h.status_first_row().starts_with("N1CALL, Bo. Fixed."));
}

/// The unread count on the status line is taken when the entry arrives:
/// moving onto the entries does not change the line (a terminal screen
/// reader would read it again).
#[test]
fn the_status_line_does_not_change_as_the_reader_moves() {
    let mut h = launch(CursorPlacement::Status);
    h.send(&entry("N0CALL", "Ada", ""));
    h.send(&entry("N1CALL", "Bo", ""));
    let s = h.status_first_row();
    assert!(s.ends_with("2 unread"), "{s}");
    h.go("N1CALL");
    assert_eq!(h.tui.app().live_unread(), Some(0));
    assert_eq!(h.status_first_row(), s);
}

/// The title line's position does not change as entries arrive; the
/// reader's own key brings the unread count up to date.
#[test]
fn the_title_position_is_frozen_while_entries_arrive() {
    let mut h = launch(CursorPlacement::Status);
    h.send(&entry("N0CALL", "Ada", "Mobile."));
    // A key draws the title with the current count.
    h.press(KeyCode::Home);
    let title = h.title();
    assert!(title.starts_with("1 unread"), "{title}");
    for i in 1..=5 {
        h.send(&entry(&format!("N{i}ABC"), "Bo", "Fixed."));
        assert_eq!(h.title(), title, "the title changed with entry {i}");
    }
    assert_eq!(h.tui.app().live_unread(), Some(6));
    h.press(KeyCode::Home);
    assert!(h.title().starts_with("6 unread"), "{}", h.title());
}

/// No redraw storm: an idle live log draws nothing, each entry draws
/// once, and the next idle passes draw nothing again. Entries arrive in
/// the pass after they are sent: the loop ticks on every pass, at most
/// [`textweaver_tui::IDLE_POLL`] apart, so no waker is needed.
#[test]
fn each_entry_draws_once_and_idle_draws_nothing() {
    let mut h = launch(CursorPlacement::Status);
    let mut term = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let tui = &mut h.tui;
    textweaver_tui::frame(&mut term, tui).unwrap();
    textweaver_tui::frame(&mut term, tui).unwrap();
    let first = tui.draws();
    for _ in 0..20 {
        textweaver_tui::frame(&mut term, tui).unwrap();
    }
    assert_eq!(tui.draws(), first, "idle frames drew");
    let wait = textweaver_tui::input_wait(tui.app(), Instant::now());
    assert!(wait <= textweaver_tui::IDLE_POLL, "{wait:?}");
    for i in 0..5 {
        h.feed
            .push_json("club/log", &entry(&format!("N{i}XYZ"), "Ada", "Mobile."));
        let before = tui.draws();
        let wait = textweaver_tui::frame(&mut term, tui).unwrap();
        assert!(wait <= textweaver_tui::IDLE_POLL, "{wait:?}");
        assert_eq!(tui.draws(), before + 1, "entry {i} drew once");
        let shown = tui.app().session().unwrap().doc.text().to_string();
        assert!(shown.contains(&format!("N{i}XYZ")), "entry {i} arrived");
        for _ in 0..10 {
            textweaver_tui::frame(&mut term, tui).unwrap();
        }
        assert_eq!(tui.draws(), before + 1, "entry {i} drew again");
    }
}

/// A short log grows into the empty rows below it: the rows down to the
/// reader's, and every row there was, stay; the new entries draw below;
/// the hardware cursor stays on the reader's word.
#[test]
fn a_short_log_grows_below_the_reader() {
    let mut h = launch(CursorPlacement::Follow);
    for (c, n) in [("N0CALL", "Ada"), ("N1CALL", "Bo"), ("N2CALL", "Cy")] {
        h.send(&entry(c, n, "Mobile."));
    }
    h.go("N1CALL");
    let body = h.body();
    let cursor = h.cursor();
    let used = body.iter().rposition(|r| !r.is_empty()).unwrap();
    assert!(used + 5 < body.len(), "room below: {body:?}");
    for (c, n) in [("N3CALL", "Di"), ("N4CALL", "Ed"), ("N5CALL", "Flo")] {
        h.send(&entry(c, n, "Fixed."));
        let after = h.body();
        assert_eq!(after[..=used], body[..=used], "rows moved: {after:?}");
        assert!(
            after.iter().any(|r| r.contains(c)),
            "{c} not drawn: {after:?}"
        );
        assert_eq!(h.cursor(), cursor, "the cursor moved");
    }
    let after = h.body();
    let rows: Vec<usize> = ["N3CALL", "N4CALL", "N5CALL"]
        .iter()
        .map(|c| after.iter().position(|r| r.contains(c)).unwrap())
        .collect();
    assert!(rows.iter().all(|&r| r > used), "{rows:?} {used}");
}

/// The reader on the newest entry stays on it when another arrives.
#[test]
fn the_reader_on_the_newest_entry_stays_there() {
    let mut h = launch(CursorPlacement::Follow);
    h.send(&entry("N0CALL", "Ada", "Mobile."));
    h.send(&entry("N1CALL", "Bo", "Fixed."));
    h.go("N1CALL");
    let cursor = h.cursor();
    let at = h.tui.app().session().unwrap().cursor;
    h.send(&entry("N2CALL", "Cy", "Portable."));
    assert_eq!(h.cursor(), cursor);
    assert_eq!(h.tui.app().session().unwrap().cursor, at);
}

/// The trim itself: whole when it fits, cut between words, "..." added,
/// and a clause's comma not left before it.
#[test]
fn trimming_keeps_whole_words_and_marks_the_cut() {
    assert_eq!(trim_to_cells("Short entry.", 40), "Short entry.");
    let long = "N0CALL, Ada Example. Mobile, portable antenna, battery.";
    let t = trim_to_cells(long, 40);
    fits(&t);
    assert!(t.ends_with("..."), "{t}");
    assert!(long.starts_with(t.trim_end_matches('.')), "{t}");
    assert!(!t.contains(",..."), "{t}");
    let word = "x".repeat(60);
    let t = trim_to_cells(&word, 40);
    fits(&t);
    assert!(t.starts_with("xxxx"), "{t}");
    // The unread count only where it fits.
    assert_eq!(
        live_status_line("N0CALL, Ada.", Some("3 unread")),
        "N0CALL, Ada.  3 unread"
    );
    let full = live_status_line(long, Some("3 unread"));
    assert!(!full.contains("unread"), "{full}");
    fits(&full);
}

/// A JSON log textweaver saved opens as a live log from the terminal
/// reader's open paths (the file named on the command line, and the open
/// prompt's command); any other JSON file opens as before.
#[test]
fn a_saved_json_log_opens_as_a_live_log() {
    use textweaver_app::live::{LiveLog, LogOptions, entry_object, json_log};
    let dir = tempfile::tempdir().unwrap();
    let mut log = LiveLog::new(LogOptions {
        title: "Club log".into(),
        id_field: None,
    });
    log.upsert(
        entry_object(&[
            ("callsign", "N0CALL"),
            ("name", "Ada"),
            ("notes", "Mobile."),
        ]),
        "2026-10-26T20:10:00Z",
    );
    log.upsert(
        entry_object(&[("callsign", "N1CALL"), ("name", "Bo"), ("notes", "")]),
        "2026-10-26T20:11:00Z",
    );
    let saved = dir.path().join("club-2026-10-26-2010.json");
    std::fs::write(
        &saved,
        json_log::to_json(&log, "club", "2026-10-26T20:10:00Z"),
    )
    .unwrap();
    let other = dir.path().join("data.json");
    std::fs::write(&other, r#"{"entries": [1, 2], "callsign": "N0CALL"}"#).unwrap();

    // The command line's file: what `tw FILE` does.
    let mut tui = Tui::with_color_support(app(CursorPlacement::Status), ColorSupport::NoColor);
    tui.app_mut().open(&saved).unwrap();
    assert_eq!(tui.app().live_log().map(LiveLog::len), Some(2));
    assert!(!tui.app().live_receiving());
    let text = tui.app().session().unwrap().doc.text().to_string();
    assert!(text.contains("N0CALL, Ada. Mobile."), "{text}");
    let mut term = Terminal::new(TestBackend::new(80, 24)).unwrap();
    term.draw(|f| tui.draw(f)).unwrap();
    let buf = term.backend().buffer();
    let title: String = (0..80).map(|x| buf[(x, 0)].symbol()).collect();
    assert!(title.starts_with("2 entries"), "{title}");

    // Another JSON file opens as a document, not a log.
    tui.app_mut().open(&other).unwrap();
    assert!(tui.app().live_log().is_none());
    assert!(tui.app().session().unwrap().live.is_none());

    // The open prompt's command.
    tui.dispatch(Command::Open(saved.clone()));
    for _ in 0..200 {
        if tui.app().live_log().is_some() {
            break;
        }
        tui.tick();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(tui.app().live_log().map(LiveLog::len), Some(2));
    tui.dispatch(Command::Open(other.clone()));
    for _ in 0..200 {
        if tui.app().live_log().is_none() && !tui.app().opening() {
            break;
        }
        tui.tick();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(tui.app().live_log().is_none());
}
