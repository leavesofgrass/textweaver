//! Live documents (beta 5 spike, N0): a scripted source changes the open
//! document from outside edit mode, and every position the reader has
//! stays on the same text.

use std::time::Instant;

use textweaver_app::a11y::{LogAnnouncer, Shared};
use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::keymap::ActionId;
use textweaver_app::live::{Fields, LiveLog, LogLabels, LogOptions, LogView, entry_object};
use textweaver_app::{App, AppConfig, Command};

/// The scripted log: the app's log view's entries, in English.
#[derive(Clone)]
struct Log(LiveLog);

impl Default for Log {
    fn default() -> Self {
        Log(LiveLog::new(LogOptions {
            title: "Live log".into(),
            id_field: None,
        }))
    }
}

impl Log {
    fn upsert(&mut self, e: Fields) {
        self.0.upsert(e, "2026-10-26T20:10:00Z");
    }

    fn render(&self, view: LogView) -> String {
        self.0.render(view, &LogLabels::default())
    }

    fn line(&self, i: usize) -> String {
        self.0.line(i, &LogLabels::default())
    }
}

/// The scripted source: what a net logger would send, one step at a time.
struct Scripted {
    log: Log,
    view: LogView,
}

impl Scripted {
    fn new(n: usize) -> Self {
        let mut log = Log::default();
        for i in 0..n {
            log.upsert(entry(i, ""));
        }
        Scripted {
            log,
            view: LogView::List,
        }
    }

    fn md(&self) -> String {
        self.log.render(self.view)
    }
}

fn entry(i: usize, notes: &str) -> Fields {
    entry_object(&[
        ("callsign", &format!("K{i:04}X")),
        ("name", &format!("Ada{i}")),
        ("notes", notes),
    ])
}

fn rig(n: usize) -> (App, Shared<LogAnnouncer>, Scripted) {
    let said = Shared::new(LogAnnouncer::default());
    let mut app = App::new(AppConfig {
        announcer: Box::new(said.clone()),
        ..AppConfig::for_tests()
    });
    let src = Scripted::new(n);
    app.open_live("test", "Live log", &src.md());
    (app, said, src)
}

fn text(app: &App) -> String {
    app.session().unwrap().doc.text().to_string()
}

fn at(app: &App, needle: &str) -> CharPos {
    let t = text(app);
    let b = t
        .find(needle)
        .unwrap_or_else(|| panic!("{needle:?} in {t:?}"));
    CharPos(t[..b].chars().count())
}

fn slice(app: &App, r: CharRange) -> String {
    app.session().unwrap().doc.slice(r)
}

fn word_at(app: &App, p: CharPos) -> String {
    slice(
        app,
        CharRange::new(p.0, (p.0 + 5).min(text(app).chars().count())),
    )
}

#[test]
fn an_append_moves_nothing_the_reader_has() {
    let (mut app, said, mut src) = rig(3);
    assert!(app.session().unwrap().live.is_some());
    let _ = app.dispatch(Command::SetCursor(at(&app, "K0002X")));
    let _ = app.dispatch(Command::Action(ActionId::AddBookmark));
    let _ = app.dispatch(Command::Find("Ada1".into()));
    let _ = app.dispatch(Command::SetCursor(at(&app, "Ada1")));
    let sel = CharRange::new(at(&app, "K0000X").0, at(&app, "K0000X").0 + 6);
    let _ = app.dispatch(Command::Select(sel));
    let before = app.session().unwrap();
    let (cursor, rev) = (before.cursor, before.revision);
    let bookmark = before.bookmarks[0].pos;
    let hits = before.find.as_ref().unwrap().hits.clone();
    let history = before.history.entries().to_vec();
    let n_said = said.lock().log.len();

    src.log.upsert(entry(3, "Mobile."));
    let o = app.live_apply(&src.md(), None, None).expect("an edit");
    assert_eq!(o.len(), 1, "{o:?}");
    assert!(o[0].removed.is_empty(), "an append removes nothing: {o:?}");
    let s = app.session().unwrap();
    assert_ne!(s.revision, rev);
    assert!(
        text(&app).contains("K0003X, Ada3. Mobile."),
        "{}",
        text(&app)
    );
    assert_eq!(s.cursor, cursor);
    assert_eq!(s.selection, Some(sel));
    assert_eq!(slice(&app, sel), "K0000X");
    assert_eq!(s.bookmarks[0].pos, bookmark);
    assert_eq!(s.find.as_ref().unwrap().hits, hits);
    assert_eq!(s.history.entries(), history.as_slice());
    assert_eq!(said.lock().log.len(), n_said, "nothing said");
}

#[test]
fn a_correction_in_place_keeps_every_place_on_its_text() {
    let (mut app, said, mut src) = rig(4);
    let _ = app.dispatch(Command::SetCursor(at(&app, "K0003X")));
    let _ = app.dispatch(Command::Action(ActionId::AddBookmark));
    let _ = app.dispatch(Command::SetCursor(at(&app, "Ada2")));
    let n_said = said.lock().log.len();
    // Entry 1 gets notes: text before entry 2 grows.
    src.log.upsert(entry(1, "Portable, low battery."));
    let line = src.log.line(1);
    app.live_apply(&src.md(), None, Some(&line))
        .expect("an edit");
    let s = app.session().unwrap();
    assert_eq!(word_at(&app, s.cursor), "Ada2.");
    assert_eq!(word_at(&app, s.bookmarks[0].pos), "K0003");
    let log = said.lock().log.clone();
    assert_eq!(log.len(), n_said + 1, "{log:?}");
    assert_eq!(
        log.last().unwrap().0,
        "K0001X, Ada1. Portable, low battery."
    );
}

#[test]
fn a_cursor_inside_the_corrected_entry_keeps_its_column() {
    let (mut app, _, mut src) = rig(3);
    src.log.upsert(entry(1, "Mobile, portable antenna."));
    app.live_apply(&src.md(), None, None);
    let p = at(&app, "portable");
    let _ = app.dispatch(Command::SetCursor(p));
    src.log.upsert(entry(1, "Mobile, portable beam."));
    app.live_apply(&src.md(), None, None);
    let s = app.session().unwrap();
    assert_eq!(s.cursor, p, "same column");
    assert_eq!(word_at(&app, s.cursor), "porta");
}

#[test]
fn a_whole_list_replaces_the_view_and_the_reader_stays_on_the_entry() {
    let (mut app, _, mut src) = rig(5);
    let _ = app.dispatch(Command::SetCursor(at(&app, "K0003X")));
    // The retained whole log, with a correction above the reader and two
    // new entries.
    let mut log = Log::default();
    for i in 0..7 {
        log.upsert(entry(i, if i == 0 { "Net control." } else { "" }));
    }
    src.log = log;
    app.live_apply(&src.md(), None, None);
    assert_eq!(word_at(&app, app.session().unwrap().cursor), "K0003");
}

#[test]
fn switching_list_to_table_keeps_the_reader_on_the_same_entry() {
    let (mut app, _, mut src) = rig(6);
    let _ = app.dispatch(Command::SetCursor(at(&app, "K0004X")));
    let old = app.session().unwrap().doc.clone();
    src.view = LogView::Table;
    let md = src.md();
    let new = textweaver_app::live::load_markdown(&md);
    let place = LiveLog::same_entry(
        &old,
        LogView::List,
        &new,
        LogView::Table,
        app.session().unwrap().cursor,
    );
    assert!(
        place.is_some(),
        "the entry is found in the table: {:?}",
        new.text().to_string()
    );
    app.live_apply(&md, place, None);
    let s = app.session().unwrap();
    assert!(text(&app).contains("Callsign"), "{}", text(&app));
    assert_eq!(word_at(&app, s.cursor), "K0004", "{:?}", text(&app));
    // And back.
    let old = s.doc.clone();
    src.view = LogView::List;
    let md = src.md();
    let new = textweaver_app::live::load_markdown(&md);
    let place = LiveLog::same_entry(&old, LogView::Table, &new, LogView::List, s.cursor);
    app.live_apply(&md, place, None);
    assert_eq!(word_at(&app, app.session().unwrap().cursor), "K0004");
}

#[test]
fn a_document_that_is_not_live_is_never_changed() {
    let mut app = App::new(AppConfig::for_tests());
    app.open_document(
        textweaver_app::text::Document::from_plain_text("Plain.\n"),
        textweaver_app::store::DocKey::untitled(1),
        "Plain".into(),
    );
    assert!(app.live_apply("# Other\n", None, None).is_none());
    assert_eq!(text(&app), "Plain.\n");
}

/// Time per update with a 500-entry log: an append, and a correction of
/// the first entry. Printed for the spike report; the bound is loose so
/// slow machines pass.
#[test]
fn five_hundred_entries_update_quickly() {
    let (mut app, _, mut src) = rig(500);
    let _ = app.dispatch(Command::SetCursor(at(&app, "K0250X")));
    let rounds = 20;
    let started = Instant::now();
    for i in 0..rounds {
        src.log.upsert(entry(500 + i, "New."));
        app.live_apply(&src.md(), None, None).expect("an edit");
    }
    let append = started.elapsed() / rounds as u32;
    let started = Instant::now();
    for i in 0..rounds {
        src.log.upsert(entry(0, &format!("Correction {i}.")));
        app.live_apply(&src.md(), None, None).expect("an edit");
    }
    let replace = started.elapsed() / rounds as u32;
    // The whole log resent (a retained list): a correction at the top and
    // five new entries, which takes the line comparison.
    let started = Instant::now();
    for i in 0..rounds {
        src.log.upsert(entry(0, &format!("Resent {i}.")));
        for k in 0..5 {
            src.log.upsert(entry(1000 + i * 5 + k, "New."));
        }
        app.live_apply(&src.md(), None, None).expect("an edit");
    }
    let resend = started.elapsed() / rounds as u32;
    eprintln!(
        "live_apply, {} chars: append {append:?}, replace first {replace:?}, resend {resend:?}",
        text(&app).chars().count()
    );
    assert_eq!(word_at(&app, app.session().unwrap().cursor), "K0250");
    assert!(append.as_millis() < 500 && replace.as_millis() < 500 && resend.as_millis() < 2000);
}

// Beta 5, N1, N2 and N7: a source feeding the log view through the tick,
// the three message shapes, corrections, the views, the unread count,
// pausing, the JSON log, and what a live session refuses.

mod fed {
    use std::path::Path;
    use std::time::{Duration, Instant};

    use textweaver_app::a11y::{LogAnnouncer, Shared};
    use textweaver_app::core::CharPos;
    use textweaver_app::keymap::ActionId;
    use textweaver_app::live::{
        LiveEvent, LiveOptions, LivePayload, LogView, PAUSE_QUEUE_CAP, ScriptHandle, ScriptedSource,
    };
    use textweaver_app::store::Paths;
    use textweaver_app::{App, AppConfig, Command};

    struct Rig {
        app: App,
        said: Shared<LogAnnouncer>,
        feed: ScriptHandle,
    }

    fn rig_with(paths: Option<Paths>, id: &str) -> Rig {
        let said = Shared::new(LogAnnouncer::default());
        let mut app = App::new(AppConfig {
            announcer: Box::new(said.clone()),
            paths,
            ..AppConfig::for_tests()
        });
        let (source, feed) = ScriptedSource::new();
        app.open_live_source(
            Box::new(source),
            LiveOptions {
                id: id.into(),
                title: "Club log".into(),
                ..LiveOptions::default()
            },
        );
        Rig { app, said, feed }
    }

    fn rig() -> Rig {
        rig_with(None, "club")
    }

    impl Rig {
        fn send(&mut self, json: &str) {
            self.feed.push_json("club/log", json);
            let _ = self.app.tick(Instant::now());
        }

        fn text(&self) -> String {
            self.app.session().unwrap().doc.text().to_string()
        }

        fn at(&self, needle: &str) -> CharPos {
            let t = self.text();
            let b = t
                .find(needle)
                .unwrap_or_else(|| panic!("{needle:?} in {t:?}"));
            CharPos(t[..b].chars().count())
        }

        fn word(&self) -> String {
            let s = self.app.session().unwrap();
            let end = (s.cursor.0 + 6).min(s.doc.len_chars());
            s.doc
                .slice(textweaver_app::core::CharRange::new(s.cursor.0, end))
        }

        fn last_said(&self) -> String {
            self.said
                .lock()
                .log
                .last()
                .map(|l| l.0.clone())
                .unwrap_or_default()
        }

        fn said_any(&self, part: &str) -> bool {
            self.said.lock().log.iter().any(|(t, _)| t.contains(part))
        }

        fn go(&mut self, needle: &str) {
            let p = self.at(needle);
            let _ = self.app.dispatch(Command::SetCursor(p));
        }
    }

    fn three(r: &mut Rig) {
        r.send(
            r#"{"callsign": "N0CALL", "name": "Ada Example", "notes": "Mobile, portable antenna"}"#,
        );
        r.send(r#"{"call": "N1TEST", "name": "Grace Sample"}"#);
        r.send(r#"{"call": "N2TEST", "name": "Lin Example", "remarks": "Short time"}"#);
    }

    #[test]
    fn a_source_feeds_the_log_through_the_tick_and_its_waker() {
        let mut r = rig();
        let (waker, rx) = textweaver_app::wake::channel_waker();
        r.app.set_waker(Some(waker));
        r.feed.set_waker(r.app.live_waker());
        assert!(r.app.live_receiving());
        assert!(r.text().contains("No entries yet."), "{}", r.text());
        r.feed.push_json(
            "club/log",
            r#"{"callsign": "N0CALL", "name": "Ada Example", "notes": "Mobile, portable antenna"}"#,
        );
        assert!(rx.try_recv().is_ok(), "the waker rang");
        let _ = r.app.tick(Instant::now());
        assert!(
            r.text()
                .contains("N0CALL, Ada Example. Mobile, portable antenna."),
            "{}",
            r.text()
        );
        assert_eq!(
            r.last_said(),
            "N0CALL, Ada Example. Mobile, portable antenna."
        );
        assert!(r.feed.polls() > 0);
    }

    #[test]
    fn opening_another_document_or_closing_stops_the_source() {
        let mut r = rig();
        three(&mut r);
        r.app.open_document(
            textweaver_app::text::Document::from_plain_text("Plain.\n"),
            textweaver_app::store::DocKey::untitled(1),
            "Plain".into(),
        );
        assert!(r.feed.stopped());
        assert!(!r.app.live_receiving());
        assert!(r.app.live_view().is_none());

        let mut r = rig();
        three(&mut r);
        let _ = r.app.live_close();
        assert!(r.feed.stopped());
        let s = r.app.session().unwrap();
        assert_eq!(s.live.as_ref().map(|t| t.receiving), Some(false));
        assert!(r.text().contains("N2TEST"));
        // The view still switches on a stopped log.
        let _ = r.app.live_toggle_view();
        assert_eq!(r.app.live_view(), Some(LogView::Table));
    }

    #[test]
    fn one_object_adds_and_a_matching_one_corrects_in_place() {
        let mut r = rig();
        three(&mut r);
        r.go("N2TEST");
        let rev = r.app.session().unwrap().revision;
        // By callsign, written differently.
        r.send(r#"{"call": "n1 test", "name": "Grace Sample", "notes": "Traffic, one message"}"#);
        assert_ne!(r.app.session().unwrap().revision, rev);
        assert_eq!(r.app.live_log().unwrap().len(), 3);
        assert!(r.text().contains("Grace Sample. Traffic, one message."));
        assert_eq!(r.word(), "N2TEST", "the reader stays");
        assert_eq!(
            r.last_said(),
            "n1 test, Grace Sample. Traffic, one message."
        );
        // By id.
        r.send(r#"{"id": 9, "call": "N3TEST"}"#);
        r.send(r#"{"id": 9, "call": "N3TEST", "name": "Corrected"}"#);
        assert_eq!(r.app.live_log().unwrap().len(), 4);
        assert!(r.text().contains("N3TEST, Corrected."));
    }

    #[test]
    fn a_whole_list_replaces_the_log_and_keeps_the_reader_on_the_entry() {
        let mut r = rig();
        three(&mut r);
        r.go("Grace");
        // The retained whole log, reordered, with a new entry first.
        r.send(
            r#"[{"call": "N9NEW", "name": "Zed"},
                {"call": "N2TEST", "name": "Lin Example"},
                {"call": "N1TEST", "name": "Grace Sample"},
                {"call": "N0CALL", "name": "Ada Example"}]"#,
        );
        assert_eq!(r.app.live_log().unwrap().len(), 4);
        assert_eq!(r.word(), "Grace ", "same entry, same offset");
        // An object wrapping the list.
        r.send(
            r#"{"net": "Example", "checkins": [{"call": "N1TEST", "name": "Grace Sample"}, {"call": "N0CALL", "name": "Ada"}]}"#,
        );
        assert_eq!(r.app.live_log().unwrap().len(), 2);
        assert_eq!(r.word(), "Grace ");
        assert!(!r.text().contains("N9NEW"));
    }

    #[test]
    fn text_and_binary_messages_are_never_dropped_silently() {
        let mut r = rig();
        r.send("plain words, not JSON");
        assert!(r.text().contains("plain words, not JSON."), "{}", r.text());
        r.feed.push(LiveEvent::Message {
            topic: "club/log".into(),
            payload: LivePayload::Binary { len: 12 },
            retained: false,
        });
        let _ = r.app.tick(Instant::now());
        assert!(
            r.said_any("it is not text (12 bytes)"),
            "{:?}",
            r.last_said()
        );
        r.feed.push(LiveEvent::Quiet {
            topic: "club/log".into(),
            seconds: 60,
        });
        let _ = r.app.tick(Instant::now());
        assert!(r.last_said().starts_with("No messages yet on club/log."));
    }

    #[test]
    fn switching_list_and_table_keeps_the_reader_on_the_entry() {
        let mut r = rig();
        three(&mut r);
        r.go("N1TEST");
        let _ = r.app.live_toggle_view();
        assert_eq!(r.app.live_view(), Some(LogView::Table));
        assert!(r.text().contains("Callsign"), "{}", r.text());
        assert_eq!(r.word(), "N1TEST");
        assert_eq!(r.last_said(), "Table view.");
        // New entries arrive in the table too.
        r.send(r#"{"call": "N4TEST", "name": "Kim"}"#);
        assert_eq!(r.word(), "N1TEST");
        let _ = r.app.live_toggle_view();
        assert_eq!(r.word(), "N1TEST");
        assert_eq!(r.last_said(), "List view.");
    }

    #[test]
    fn unread_counts_from_the_furthest_entry_read() {
        let mut r = rig();
        three(&mut r);
        assert_eq!(r.app.live_unread(), Some(3), "the reader is on the heading");
        assert_eq!(
            r.app.title_position(),
            Some(vec!["3 unread".to_owned()]),
            "the position is frozen"
        );
        r.go("N1TEST");
        assert_eq!(r.app.live_unread(), Some(1));
        let _ = r.app.tick(Instant::now());
        // Going back up keeps the furthest read.
        r.go("N0CALL");
        assert_eq!(r.app.live_unread(), Some(1));
        r.send(r#"{"call": "N4TEST"}"#);
        assert_eq!(r.app.live_unread(), Some(2));
        let _ = r.app.live_jump_newest();
        assert_eq!(r.word(), "N4TEST");
        assert_eq!(r.app.live_unread(), Some(0));
        assert_eq!(r.app.title_position(), Some(vec!["none unread".to_owned()]));
        assert_eq!(r.app.live_newest_line().as_deref(), Some("N4TEST."));
    }

    #[test]
    fn paused_updates_wait_and_resume_as_one_change() {
        let mut r = rig();
        three(&mut r);
        r.go("N1TEST");
        let _ = r.app.live_set_paused(true);
        assert!(r.app.live_paused());
        let before = r.text();
        let rev = r.app.session().unwrap().revision;
        r.send(r#"{"call": "N4TEST"}"#);
        r.send(r#"{"call": "N1TEST", "name": "Grace Corrected"}"#);
        assert_eq!(r.text(), before, "nothing changes while paused");
        assert_eq!(r.app.live_waiting(), 2);
        let pos = r.app.title_position().unwrap();
        assert!(pos.contains(&"updates paused".to_owned()), "{pos:?}");
        let _ = r.app.live_toggle_pause();
        let s = r.app.session().unwrap();
        assert_ne!(s.revision, rev);
        assert!(r.text().contains("N4TEST") && r.text().contains("Grace Corrected"));
        assert_eq!(r.word(), "N1TEST");
        assert_eq!(r.last_said(), "Updates resumed, 2 updates applied.");

        // The queue is capped, and what was left out is said.
        let _ = r.app.live_set_paused(true);
        for i in 0..PAUSE_QUEUE_CAP + 5 {
            r.feed
                .push_json("club/log", &format!(r#"{{"call": "Q{i}"}}"#));
        }
        let _ = r.app.tick(Instant::now());
        assert_eq!(r.app.live_waiting(), PAUSE_QUEUE_CAP);
        let _ = r.app.live_set_paused(false);
        assert!(r.last_said().contains("5 older updates were left out"));
        // A whole log replaces everything queued before it.
        let _ = r.app.live_set_paused(true);
        r.send(r#"{"call": "W1"}"#);
        r.send(r#"[{"call": "W2"}]"#);
        assert_eq!(r.app.live_waiting(), 1);
    }

    #[test]
    fn a_live_session_refuses_edits_and_keeps_out_of_the_library() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::under(dir.path());
        let mut r = rig_with(Some(paths.clone()), "club");
        three(&mut r);
        let _ = r.app.dispatch(Command::Action(ActionId::ToggleEditMode));
        assert!(!r.app.is_editing());
        assert!(
            r.said_any("This live log is read only; it cannot be edited."),
            "{}",
            r.last_said()
        );
        r.go("N1TEST");
        let _ = r.app.dispatch(Command::Action(ActionId::AddBookmark));
        let _ = r.app.wait_for_writes();
        let state_files = std::fs::read_dir(paths.state_dir())
            .map(|d| d.count())
            .unwrap_or(0);
        assert_eq!(state_files, 0, "no reading position is kept");
        assert!(!paths.recent_file().exists(), "no recent entry");
        // Save writes the JSON log.
        let _ = r.app.dispatch(Command::Action(ActionId::Save));
        let _ = r.app.wait_for_writes();
        assert!(
            r.said_any("Log saved as club-live-log-"),
            "{}",
            r.last_said()
        );
    }

    fn logs_in(dir: &Path) -> Vec<std::path::PathBuf> {
        std::fs::read_dir(dir)
            .map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).collect())
            .unwrap_or_default()
    }

    #[test]
    fn the_json_log_is_written_as_it_runs_and_reopens_read_only() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::under(dir.path());
        let logs = paths.data_dir.join("logs");
        let mut r = rig_with(Some(paths.clone()), "");
        let _ = r.app.tick(Instant::now());
        let _ = r.app.wait_for_writes();
        assert!(!logs.exists(), "nothing written before the first entry");
        let start = Instant::now();
        r.feed
            .push_json("club/log", r#"{"call": "N0CALL", "name": "Ada Example"}"#);
        let _ = r.app.tick(start);
        let _ = r.app.wait_for_writes();
        let files = logs_in(&logs);
        assert_eq!(files.len(), 1, "{files:?}");
        let path = files[0].clone();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            name.starts_with("live-log-") && name.ends_with(".json") && name.len() == 29,
            "{name}"
        );
        // Written again only after the interval.
        r.feed
            .push_json("club/log", r#"{"call": "N1TEST", "name": "Grace Sample"}"#);
        let _ = r.app.tick(start + Duration::from_secs(1));
        let _ = r.app.wait_for_writes();
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(!saved.contains("N1TEST"));
        let _ = r.app.tick(start + Duration::from_secs(11));
        let _ = r.app.wait_for_writes();
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(
            saved.contains("N1TEST") && saved.contains("\"received\""),
            "{saved}"
        );
        assert_eq!(r.app.live_log_path().as_deref(), Some(path.as_path()));
        // A correction, then closing writes at once.
        r.feed.push_json(
            "club/log",
            r#"{"call": "N1TEST", "name": "Grace Corrected"}"#,
        );
        let _ = r.app.tick(start + Duration::from_secs(12));
        r.app.shutdown();
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(
            saved.contains("Grace Corrected") && saved.contains("\"updated\""),
            "{saved}"
        );
        drop(r);

        // Reopened: the same view, read only, not live.
        let said = Shared::new(LogAnnouncer::default());
        let mut app = App::new(AppConfig {
            announcer: Box::new(said.clone()),
            paths: Some(paths.clone()),
            ..AppConfig::for_tests()
        });
        let _ = app.dispatch(Command::Open(path.clone()));
        let s = app.session().expect("opened");
        let text = s.doc.text().to_string();
        assert!(text.contains("N0CALL, Ada Example."), "{text}");
        assert!(text.contains("N1TEST, Grace Corrected."), "{text}");
        assert_eq!(s.live.as_ref().map(|t| t.receiving), Some(false));
        assert!(!app.live_receiving());
        assert_eq!(app.title_position(), Some(vec!["2 entries".to_owned()]));
        let _ = app.live_toggle_view();
        assert!(
            app.session()
                .unwrap()
                .doc
                .text()
                .to_string()
                .contains("Callsign")
        );
        let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
        assert!(!app.is_editing());
        let _ = app.dispatch(Command::Action(ActionId::Save));
        let last = said.lock().log.last().unwrap().0.clone();
        assert!(last.starts_with("This log was opened from"), "{last}");
    }
}
