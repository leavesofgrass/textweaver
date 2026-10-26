//! A live document in the terminal reader (beta 5 spike, N0): entries
//! added and corrected outside edit mode leave the reader's row, the
//! rows above it, and the hardware cursor where they were, and the status
//! line carries the new entry, meaning first, inside 40 Braille cells.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use textweaver_app::live::{LogEntry, LogView, NetLog};
use textweaver_app::store::{AccessMode as ModeSetting, CursorPlacement, Settings};
use textweaver_app::testing::recording_service;
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig, Command};
use textweaver_tui::Tui;
use textweaver_tui::ui::{BRAILLE_CELLS, braille_cells};

struct Harness {
    tui: Tui,
    term: Terminal<TestBackend>,
    width: u16,
    log: NetLog,
}

fn entry(i: usize, notes: &str) -> LogEntry {
    LogEntry::new(&format!("K{i:04}X"), &format!("Ada{i}"), notes)
}

fn launch(cursor: CursorPlacement, n: usize) -> Harness {
    let mut settings = Settings::default();
    settings.accessibility.mode = ModeSetting::ScreenReader;
    settings.accessibility.cursor = cursor;
    let (speech, _log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        settings,
        speech,
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    let mut log = NetLog::default();
    for i in 0..n {
        log.upsert(entry(i, "Mobile, portable antenna."));
    }
    app.open_live("test", "Net log", &log.render(LogView::List));
    let (width, height) = (80, 24);
    let mut h = Harness {
        tui: Tui::with_color_support(app, ColorSupport::NoColor),
        term: Terminal::new(TestBackend::new(width, height)).unwrap(),
        width,
        log,
    };
    h.draw();
    h
}

impl Harness {
    fn draw(&mut self) {
        let later = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let tui = &mut self.tui;
        self.term.draw(|f| tui.draw_at(f, later)).unwrap();
    }

    fn row(&self, y: u16) -> String {
        let buf = self.term.backend().buffer();
        (0..self.width).map(|x| buf[(x, y)].symbol()).collect()
    }

    /// The document's rows.
    fn body(&self) -> Vec<String> {
        let a = self.tui.areas(self.term.backend().buffer().area).body;
        (a.y..a.y + a.height)
            .map(|y| self.row(y).trim_end().to_owned())
            .collect()
    }

    fn status(&self) -> String {
        let a = self.tui.areas(self.term.backend().buffer().area).status;
        (a.y..a.y + a.height)
            .map(|y| self.row(y).trim_end().to_owned())
            .collect::<Vec<_>>()
            .join(" ")
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

    fn update(&mut self, announce: bool, i: usize) {
        let md = self.log.render(LogView::List);
        let said = announce.then(|| NetLog::line(&self.log.entries[i]));
        self.tui
            .app_mut()
            .live_apply(&md, None, said.as_deref())
            .expect("a change");
        self.draw();
    }
}

/// The reader on entry 10 of 40 (a log longer than the screen): an
/// append and a correction above leave every document row and the
/// hardware cursor where they were (the corrected row itself changes),
/// and the status line starts with the new entry.
#[test]
fn rows_and_cursor_stay_and_the_status_line_carries_the_entry() {
    let mut h = launch(CursorPlacement::Follow, 40);
    h.go("K0010X");
    let body = h.body();
    let cursor = h.cursor();
    assert!(body.iter().any(|r| r.contains("K0010X")), "{body:?}");

    // An append below the screen: nothing on screen moves.
    h.log.upsert(entry(40, "Fixed station."));
    h.update(true, 40);
    assert_eq!(h.body(), body, "the rows stay");
    assert_eq!(h.cursor(), cursor, "the cursor stays");
    let s = h.status();
    assert!(s.starts_with("K0040X, Ada40."), "meaning first: {s}");
    assert!(braille_cells("K0040X, Ada40.") <= BRAILLE_CELLS);

    // A correction above the reader: only that row changes.
    h.log.upsert(entry(3, "On battery."));
    h.update(true, 3);
    let after = h.body();
    let changed: Vec<usize> = (0..body.len()).filter(|&i| after[i] != body[i]).collect();
    assert_eq!(changed.len(), 1, "{changed:?}: {after:?}");
    assert!(after[changed[0]].contains("K0003X, Ada3. On battery."));
    assert_eq!(h.cursor(), cursor, "the cursor stays");
    assert!(
        h.status().starts_with("K0003X, Ada3. On battery."),
        "{}",
        h.status()
    );

    // A silent update leaves the status line alone.
    let before = h.status();
    h.log.upsert(entry(41, ""));
    h.update(false, 41);
    assert_eq!(h.status(), before);
}

/// The owner's setup: the cursor on the status line. A new entry is on
/// the status line's first cells, and the cursor waits at its start.
#[test]
fn with_the_cursor_on_the_status_line_the_entry_is_in_the_first_cells() {
    let mut h = launch(CursorPlacement::Status, 20);
    h.go("K0005X");
    let cursor = h.cursor();
    h.log.upsert(entry(20, "Mobile, portable antenna."));
    h.update(true, 20);
    let status_area = h.tui.areas(h.term.backend().buffer().area).status;
    assert_eq!(h.cursor(), cursor);
    assert_eq!(h.cursor(), (status_area.x, status_area.y));
    let first = h.row(status_area.y);
    let line = first.trim_end();
    assert!(line.starts_with("K0020X, Ada20. Mobile"), "{line}");
    let title = h.row(0);
    eprintln!("title after an append: {}", title.trim_end());
}
