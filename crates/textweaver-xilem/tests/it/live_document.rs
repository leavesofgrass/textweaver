//! A live document in the window (beta 5 spike, N0): a scripted source
//! adds and corrects entries outside edit mode, and the screen reader
//! keeps its place: the text nodes that stay keep their ids and text, the
//! caret and the focus do not move, and the only live-region message is
//! the one the update asks for.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Instant;

use masonry::accesskit::{Live, Role, TreeUpdate};
use masonry::core::WidgetId;
use masonry_testing::{TestHarness, TestHarnessParams};
use textweaver_app::App;
use textweaver_app::Command;
use textweaver_app::a11y::{LogAnnouncer, Shared};
use textweaver_app::core::CharPos;
use textweaver_app::live::{Fields, LiveLog, LogLabels, LogOptions, LogView, entry_object};
use textweaver_xilem::gui::{self, ANNOUNCER, DOC};
use textweaver_xilem::setup::{self, Options};
use textweaver_xilem::theme::{self, Palette};
use textweaver_xilem::widgets::{Announcer, Message, Root};

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

    fn len(&self) -> usize {
        self.0.len()
    }
}

struct Rig {
    app: App,
    h: TestHarness<Root>,
    r: gui::Refresher,
    doc: WidgetId,
    said: Shared<LogAnnouncer>,
    /// Messages already passed to the live region.
    heard: usize,
    log: Log,
    view: LogView,
}

fn entry(i: usize, notes: &str) -> Fields {
    entry_object(&[
        ("callsign", &format!("K{i:04}X")),
        ("name", &format!("Ada{i}")),
        ("notes", notes),
    ])
}

fn rig(home: &std::path::Path, n: usize) -> Rig {
    let opts = Options {
        no_speech: true,
        home: Some(home.to_path_buf()),
        ..Options::default()
    };
    let said = Shared::new(LogAnnouncer::default());
    let (mut app, _) = setup::build_app(&opts, Box::new(said.clone()));
    let mut log = Log::default();
    for i in 0..n {
        log.upsert(entry(i, "Mobile, portable antenna."));
    }
    app.open_live("test", "Live log", &log.render(LogView::List));
    let p = Palette::galaxy();
    let tree = gui::build_tree(
        &p,
        Default::default(),
        Some(&app),
        Rc::new(Cell::new(0)),
        Default::default(),
    );
    let mut params = TestHarnessParams::default();
    params.window_size = (1100, 780).into();
    let h = TestHarness::create_with(theme::default_properties(&p), tree.root, params);
    let doc = h.get_widget(DOC).id();
    let heard = said.lock().log.len();
    let mut rig = Rig {
        app,
        h,
        r: gui::Refresher::default(),
        doc,
        said,
        heard,
        log,
        view: LogView::List,
    };
    rig.frame();
    rig.h.focus_on(Some(doc));
    rig.frame();
    rig
}

impl Rig {
    /// One turn of the window: refresh from the app, the app's new
    /// messages to the live region (as `Gui::refresh` does), and a redraw.
    fn frame(&mut self) -> TreeUpdate {
        let _ = self.r.refresh(&self.app, &mut self.h);
        let new: Vec<Message> = self.said.lock().log[self.heard..]
            .iter()
            .map(|(text, priority)| Message {
                text: text.clone(),
                priority: *priority,
            })
            .collect();
        self.heard += new.len();
        if !new.is_empty() {
            self.h
                .edit_widget(ANNOUNCER, |mut a| Announcer::say(&mut a, new));
        }
        self.h.redraw().1
    }

    /// The document's text-run nodes: id and text, in order.
    fn runs(&self) -> Vec<(String, String)> {
        self.h
            .access_node(self.doc)
            .expect("the document")
            .children()
            .map(|c| (format!("{:?}", c.id()), c.value().unwrap_or_default()))
            .collect()
    }

    /// The live nodes in the tree (id and label), and the focused node.
    fn live_and_focus(&self) -> (BTreeMap<String, String>, Option<String>) {
        let mut live = BTreeMap::new();
        let mut stack = vec![self.h.access_tree().state().root()];
        while let Some(n) = stack.pop() {
            stack.extend(n.children());
            if n.live() != Live::Off {
                live.insert(
                    format!("{:?}", n.id()),
                    n.label().or_else(|| n.value()).unwrap_or_default(),
                );
            }
        }
        let focus = self.h.focused_widget_id();
        (live, focus.map(|f| format!("{f:?}")))
    }

    fn caret(&self) -> CharPos {
        self.h.get_widget(DOC).inner().state().caret
    }

    fn cursor(&self) -> CharPos {
        self.app.session().unwrap().cursor
    }

    fn at(&self, needle: &str) -> CharPos {
        let t = self.app.session().unwrap().doc.text().to_string();
        let b = t
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} in {t:?}"));
        CharPos(t[..b].chars().count())
    }

    /// Sends the log to the app, and says `announce`.
    fn update(&mut self, announce: Option<&str>) -> TreeUpdate {
        let md = self.log.render(self.view);
        self.app
            .live_apply(&md, None, announce)
            .expect("the text changed");
        self.frame()
    }
}

fn text_runs(update: &TreeUpdate) -> usize {
    update
        .nodes
        .iter()
        .filter(|(_, n)| n.role() == Role::TextRun)
        .count()
}

fn live_sent(update: &TreeUpdate) -> Vec<String> {
    update
        .nodes
        .iter()
        .filter(|(_, n)| n.live().is_some_and(|l| l != Live::Off))
        .map(|(_, n)| n.label().unwrap_or_default().to_owned())
        .collect()
}

/// After an append and after an in-place correction, every run that was
/// there keeps its node id and its text, the caret and the focus stay, and
/// the only new live-region message is the one the update asked for.
#[test]
fn entries_added_and_corrected_keep_the_screen_readers_place() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = rig(dir.path(), 12);
    let p = r.at("K0006X");
    let _ = r.app.dispatch(Command::SetCursor(p));
    r.frame();
    assert_eq!(r.caret(), p);
    let before = r.runs();
    let (live_before, focus) = r.live_and_focus();
    assert_eq!(
        r.h.focused_widget_id(),
        Some(r.doc),
        "the document has the focus"
    );

    // An append, said as the whole entry.
    r.log.upsert(entry(12, "Fixed station."));
    let said = r.log.line(12);
    let update = r.update(Some(&said));
    let after = r.runs();
    assert!(after.len() > before.len(), "{after:?}");
    assert_eq!(
        &after[..before.len()],
        before.as_slice(),
        "the runs that stay"
    );
    assert_eq!(after.last().unwrap().1.trim_end(), said);
    eprintln!(
        "append: {} nodes sent, {} text runs, live {:?}",
        update.nodes.len(),
        text_runs(&update),
        live_sent(&update)
    );
    assert_eq!(text_runs(&update), after.len() - before.len());
    assert_eq!(r.cursor(), p);
    assert_eq!(r.caret(), p, "the caret stays");
    let (live_after, focus_after) = r.live_and_focus();
    assert_eq!(focus_after, focus, "the focus stays");
    let new_live: Vec<_> = live_after
        .iter()
        .filter(|(k, _)| !live_before.contains_key(*k))
        .map(|(_, v)| v.clone())
        .collect();
    assert_eq!(new_live, vec![said.clone()], "one message, the entry");

    // An in-place correction of an entry above the reader, said.
    let before = r.runs();
    let (live_before, _) = r.live_and_focus();
    r.log.upsert(entry(2, "Mobile, now on battery."));
    let said = r.log.line(2);
    let update = r.update(Some(&said));
    let after = r.runs();
    assert_eq!(after.len(), before.len());
    let changed: Vec<usize> = (0..after.len())
        .filter(|&i| after[i] != before[i])
        .collect();
    eprintln!(
        "correction: {} nodes sent, {} text runs, changed {changed:?}, live {:?}",
        update.nodes.len(),
        text_runs(&update),
        live_sent(&update)
    );
    assert_eq!(
        changed.len(),
        1,
        "only the corrected entry's run: {changed:?}"
    );
    assert_eq!(after[changed[0]].1.trim_end(), said);
    assert_eq!(text_runs(&update), 1);
    let q = r.at("K0006X");
    assert_eq!(r.cursor(), q, "the reader is on the same entry");
    assert_eq!(r.caret(), q, "the caret is on the same entry");
    let (live_after, focus_after) = r.live_and_focus();
    assert_eq!(focus_after, focus);
    let new_live: Vec<_> = live_after
        .iter()
        .filter(|(k, _)| !live_before.contains_key(*k))
        .map(|(_, v)| v.clone())
        .collect();
    assert_eq!(new_live, vec![said], "one message, the corrected entry");

    // A silent update: nothing live is sent at all.
    r.log.upsert(entry(13, ""));
    let update = r.update(None);
    assert!(live_sent(&update).is_empty(), "{:?}", live_sent(&update));
    assert_eq!(r.caret(), q);
}

/// The whole log sent again (a retained list) with one change above the
/// reader and new entries below: the runs of unchanged entries keep their
/// nodes, and the reader stays on the same entry.
#[test]
fn a_whole_log_resent_keeps_unchanged_entries_nodes() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = rig(dir.path(), 10);
    let p = r.at("K0005X");
    let _ = r.app.dispatch(Command::SetCursor(p));
    r.frame();
    let before = r.runs();
    let mut log = Log::default();
    for i in 0..13 {
        log.upsert(entry(
            i,
            if i == 1 {
                "Net control."
            } else {
                "Mobile, portable antenna."
            },
        ));
    }
    r.log = log;
    let update = r.update(None);
    let after = r.runs();
    let kept = before.iter().filter(|b| after.contains(b)).count();
    eprintln!(
        "resent: kept {kept} of {} runs, {} text runs sent",
        before.len(),
        text_runs(&update)
    );
    assert_eq!(kept, before.len() - 1, "all but the corrected entry");
    assert_eq!(r.caret(), r.at("K0005X"));
}

/// Switching the list to a table keeps the reader on the same entry (the
/// view changes, so the nodes are new).
#[test]
fn list_to_table_keeps_the_reader_on_the_same_entry() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = rig(dir.path(), 8);
    let _ = r.app.dispatch(Command::SetCursor(r.at("K0004X")));
    r.frame();
    let old = r.app.session().unwrap().doc.clone();
    r.view = LogView::Table;
    let md = r.log.render(LogView::Table);
    let new = textweaver_app::live::load_markdown(&md);
    let place = LiveLog::same_entry(&old, LogView::List, &new, LogView::Table, r.cursor());
    r.app.live_apply(&md, place, None).expect("a change");
    r.frame();
    assert_eq!(r.caret(), r.at("K0004X"));
    assert_eq!(r.cursor(), r.at("K0004X"));
}

/// Time per update in the window with a 500-entry log: the app's apply,
/// the refresh, and the accessibility pass. Printed for the spike report.
#[test]
fn five_hundred_entries_in_the_window() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = rig(dir.path(), 500);
    let _ = r.app.dispatch(Command::SetCursor(r.at("K0250X")));
    r.frame();
    let rounds = 10u32;
    let mut apply = std::time::Duration::ZERO;
    let mut window = std::time::Duration::ZERO;
    let mut sent = 0;
    for i in 0..rounds as usize {
        r.log.upsert(entry(500 + i, "Mobile, portable antenna."));
        let md = r.log.render(LogView::List);
        let t = Instant::now();
        r.app.live_apply(&md, None, None).expect("a change");
        apply += t.elapsed();
        let t = Instant::now();
        let u = r.frame();
        window += t.elapsed();
        sent += text_runs(&u);
    }
    let before = r.runs();
    let t = Instant::now();
    r.log.upsert(entry(3, "Correction."));
    let u = r.update(None);
    let correction = t.elapsed();
    let after = r.runs();
    let changed = before.iter().zip(&after).filter(|(a, b)| a != b).count();
    eprintln!(
        "500 entries, {} chars: apply {:?}, window {:?} per append ({} runs sent in {rounds}); correction {correction:?}, {} runs sent, {changed} changed",
        r.app.session().unwrap().doc.len_chars(),
        apply / rounds,
        window / rounds,
        sent,
        text_runs(&u),
    );
    assert_eq!(r.caret(), r.at("K0250X"));
    assert_eq!(changed, 1);
}

/// A log longer than the window (about 120,000 UTF-16 units): the window
/// is cut again around the reader on every update. Printed for the spike
/// report: how many of the runs shown keep their nodes on an append, with
/// the reader near the end and near the start.
#[test]
fn a_log_longer_than_the_window() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = rig(dir.path(), 3500);
    for (name, at) in [("near the end", "K3450X"), ("near the start", "K0100X")] {
        let p = r.at(at);
        let _ = r.app.dispatch(Command::SetCursor(p));
        r.frame();
        let before = r.runs();
        let n = r.log.len();
        r.log.upsert(entry(n, "Mobile, portable antenna."));
        let t = Instant::now();
        let u = r.update(None);
        let after = r.runs();
        let kept = before.iter().filter(|b| after.contains(b)).count();
        eprintln!(
            "{name}: {} chars, kept {kept} of {} runs, {} sent, {:?}",
            r.app.session().unwrap().doc.len_chars(),
            before.len(),
            text_runs(&u),
            t.elapsed()
        );
        assert_eq!(r.caret(), p, "{name}");
    }
}
