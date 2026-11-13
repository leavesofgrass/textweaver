//! A whole live log sent again (beta 6): the guide promises it "replaces
//! what you have and keeps you on the same entry". With 10,000 entries and
//! the reader in the middle, a resend, as it was or with one entry removed
//! before the reader and one changed after, leaves the reader, the
//! selection and the view on the same entry, and says nothing already
//! heard again.

use std::time::Instant;

use textweaver_app::a11y::{LogAnnouncer, Shared};
use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::live::{BURST, LiveLog, LiveOptions, ScriptHandle, ScriptedSource};
use textweaver_app::{App, AppConfig, Command};

const N: usize = 10_000;
/// The reader's entry, one removed before it, one changed after it.
const READER: usize = 5_000;
const REMOVED: usize = 1_000;
const CHANGED: usize = 9_000;

struct Rig {
    app: App,
    said: Shared<LogAnnouncer>,
    feed: ScriptHandle,
}

fn rig() -> Rig {
    let said = Shared::new(LogAnnouncer::default());
    let mut app = App::new(AppConfig {
        announcer: Box::new(said.clone()),
        ..AppConfig::for_tests()
    });
    let (source, feed) = ScriptedSource::new();
    app.open_live_source(
        Box::new(source),
        LiveOptions {
            id: "club".into(),
            title: "Club log".into(),
            ..LiveOptions::default()
        },
    );
    Rig { app, said, feed }
}

fn entry(i: usize) -> String {
    format!(r#"{{"callsign": "K{i:05}X", "name": "Ada{i}"}}"#)
}

fn whole(entries: impl Iterator<Item = String>) -> String {
    format!("[{}]", entries.collect::<Vec<_>>().join(","))
}

/// Where the reader is, relative to their entry: the entry's callsign,
/// the cursor's offset into it, the selection's offsets and text, and how
/// many lines the view's top is above the cursor's line.
#[derive(Debug, PartialEq)]
struct Place {
    call: String,
    offset: usize,
    selection: Option<(isize, isize, String)>,
    top_above: isize,
}

impl Rig {
    fn send(&mut self, json: &str) {
        self.feed.push_json("club/log", json);
        let _ = self.app.tick(Instant::now());
    }

    /// Lets the announcement throttle say everything it holds.
    fn flush(&mut self) {
        let _ = self.app.tick(Instant::now() + BURST * 4);
        let _ = self.app.tick(Instant::now() + BURST * 8);
    }

    fn said(&self) -> Vec<String> {
        self.said.lock().log.iter().map(|l| l.0.clone()).collect()
    }

    fn entry_range(&self, i: usize) -> CharRange {
        let view = self.app.live_view().unwrap();
        LiveLog::entry_ranges(&self.app.session().unwrap().doc, view)[i]
    }

    fn place(&self) -> Place {
        let s = self.app.session().unwrap();
        let view = self.app.live_view().unwrap();
        let i = LiveLog::entry_at(&s.doc, view, s.cursor).expect("the reader is on an entry");
        let start = self.entry_range(i).start.0;
        let off = |p: usize| p as isize - start as isize;
        let call = s.doc.slice(CharRange::new(start, start + 7));
        let selection = s
            .selection
            .map(|r| (off(r.start.0), off(r.end.0), s.doc.slice(r)));
        let line = s.doc.line_of(s.cursor);
        let top_above = line as isize - self.app.viewport().top_line as isize;
        Place {
            call,
            offset: s.cursor.0 - start,
            selection,
            top_above,
        }
    }
}

#[test]
fn a_whole_log_sent_again_keeps_the_reader_and_says_nothing_heard() {
    let mut r = rig();
    r.send(&whole((0..N).map(entry)));
    assert_eq!(r.app.live_log().unwrap().len(), N);
    r.flush();
    // The reader in the middle: a word of their entry selected, the cursor
    // at its end.
    let e = r.entry_range(READER);
    let sel = CharRange::new(e.start.0 + 8, e.start.0 + 12);
    let _ = r.app.dispatch(Command::Select(sel));
    // The view shows the reader's entry.
    let _ = r.app.dispatch(Command::Resize {
        width: 80,
        height: 24,
    });
    let before = r.place();
    assert_eq!(before.call, format!("K{READER:05}X"));
    assert!((0..24).contains(&before.top_above), "{before:?}");
    let unread = r.app.live_unread();
    let heard = r.said().len();

    // The same log again: nothing changes, nothing is said.
    let rev = r.app.session().unwrap().revision;
    r.send(&whole((0..N).map(entry)));
    r.flush();
    assert_eq!(r.app.session().unwrap().revision, rev, "the same text");
    assert_eq!(r.place(), before, "the same log");
    assert_eq!(r.said()[heard..], [] as [String; 0], "the same log");
    assert_eq!(r.app.live_unread(), unread);

    // One entry removed before the reader, one changed after.
    let changed = (0..N).filter(|&i| i != REMOVED).map(|i| {
        if i == CHANGED {
            format!(r#"{{"callsign": "K{i:05}X", "name": "Corrected"}}"#)
        } else {
            entry(i)
        }
    });
    r.send(&whole(changed));
    r.flush();
    assert_eq!(r.app.live_log().unwrap().len(), N - 1);
    assert_eq!(r.place(), before, "one removed, one changed");
    let new: Vec<String> = r.said()[heard..].to_vec();
    assert!(
        new.iter().all(|t| !t.contains("Ada")),
        "only the correction is new: {new:?}"
    );
    assert!(
        new.iter().any(|t| t.contains("Corrected")),
        "the correction is said: {new:?}"
    );
    // The entry removed is read; the count of unread entries is the same.
    assert_eq!(r.app.live_unread(), unread);
    assert_eq!(
        r.app.session().unwrap().cursor,
        CharPos(r.entry_range(READER - 1).start.0 + before.offset)
    );
}
