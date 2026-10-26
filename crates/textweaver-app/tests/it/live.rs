//! Live documents (beta 5 spike, N0): a scripted source changes the open
//! document from outside edit mode, and every position the reader has
//! stays on the same text.

use std::time::Instant;

use textweaver_app::a11y::{LogAnnouncer, Shared};
use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::keymap::ActionId;
use textweaver_app::live::{LogEntry, LogView, NetLog};
use textweaver_app::{App, AppConfig, Command};

/// The scripted source: what a net logger would send, one step at a time.
struct Scripted {
    log: NetLog,
    view: LogView,
}

impl Scripted {
    fn new(n: usize) -> Self {
        let mut log = NetLog::default();
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

fn entry(i: usize, notes: &str) -> LogEntry {
    LogEntry::new(&format!("K{i:04}X"), &format!("Ada{i}"), notes)
}

fn rig(n: usize) -> (App, Shared<LogAnnouncer>, Scripted) {
    let said = Shared::new(LogAnnouncer::default());
    let mut app = App::new(AppConfig {
        announcer: Box::new(said.clone()),
        ..AppConfig::for_tests()
    });
    let src = Scripted::new(n);
    app.open_live("test", "Net log", &src.md());
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
    let line = NetLog::line(&src.log.entries[1]);
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
    let mut log = NetLog::default();
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
    let place = NetLog::same_entry(
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
    let place = NetLog::same_entry(&old, LogView::Table, &new, LogView::List, s.cursor);
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
