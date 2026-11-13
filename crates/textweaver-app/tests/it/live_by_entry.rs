//! A live log changed by one entry changes the document by one edit
//! (beta 6), with the same text and markers a full load of the view gives,
//! and the reader's place kept.

use std::time::{Duration, Instant};

use textweaver_app::a11y::{LogAnnouncer, Shared};
use textweaver_app::core::CharPos;
use textweaver_app::live::{LiveOptions, LogView, ScriptHandle, ScriptedSource, load_markdown};
use textweaver_app::{App, AppConfig, Command};

/// A small deterministic random source.
struct Lcg(u64);

impl Lcg {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(self.0 >> 33).unwrap_or(0) % n.max(1)
    }
}

struct Fed {
    app: App,
    feed: ScriptHandle,
}

fn fed() -> Fed {
    let mut app = App::new(AppConfig {
        announcer: Box::new(Shared::new(LogAnnouncer::default())),
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
    Fed { app, feed }
}

impl Fed {
    fn send(&mut self, json: &str) {
        self.feed.push_json("club/log", json);
        let _ = self.app.tick(Instant::now());
    }

    /// The open document is what loading the whole view gives: the same
    /// text and the same markers.
    fn check(&self, step: &str) {
        let view = self.app.live_view().unwrap();
        let md = self
            .app
            .live_log()
            .unwrap()
            .render(view, &self.app.live_labels());
        let full = load_markdown(&md);
        let doc = &self.app.session().unwrap().doc;
        assert_eq!(doc.text().to_string(), full.text().to_string(), "{step}");
        let (a, b) = (doc.markers(), full.markers());
        if let Some(i) = (0..a.len().max(b.len())).find(|&i| a.get(i) != b.get(i)) {
            panic!(
                "{step}: marker {i} differs: {:?} against a full load's {:?}, in {:?}",
                a.get(i),
                b.get(i),
                doc.text().to_string()
            );
        }
    }

    fn word_at_cursor(&self) -> String {
        let s = self.app.session().unwrap();
        let end = (s.cursor.0 + 6).min(s.doc.len_chars());
        s.doc
            .slice(textweaver_app::core::CharRange::new(s.cursor.0, end))
    }
}

/// One entry with a random mix of fields, some needing escapes, some
/// unknown (a table column of their own).
fn random_entry(r: &mut Lcg, call: usize) -> String {
    let notes = [
        "",
        "Mobile",
        "Short time, *portable* [antenna]",
        "Traffic: 2 | 3 _items_",
        "`code` and \\\\ slash",
        "caf\u{e9} \u{1F99D}",
    ];
    let mut fields = vec![format!(r#""callsign": "K{call:03}X""#)];
    if r.below(3) > 0 {
        fields.push(format!(r#""name": "Ada {}""#, r.below(50)));
    }
    let n = notes[r.below(notes.len())];
    if !n.is_empty() {
        fields.push(format!(r#""notes": "{n}""#));
    }
    if r.below(5) == 0 {
        fields.push(format!(r#""power": {}"#, r.below(100)));
    }
    if r.below(6) == 0 {
        fields.push(r#""traffic": true"#.to_owned());
    }
    format!("{{{}}}", fields.join(", "))
}

/// Random appends and corrections, in both views: after each message the
/// document equals a full load of the view, and an append never moves
/// the reader.
#[test]
fn entry_by_entry_changes_match_a_full_load() {
    for seed in 1..=6u64 {
        let mut r = Lcg(seed);
        let mut f = fed();
        let mut calls = 0usize;
        for _ in 0..3 {
            f.send(&random_entry(&mut r, calls));
            calls += 1;
        }
        f.check("start");
        if seed % 2 == 0 {
            f.app.live_set_view(LogView::Table);
            f.check("table");
        }
        for step in 0..60 {
            if r.below(10) == 0 {
                let v = f.app.live_view().unwrap().other();
                f.app.live_set_view(v);
                f.check(&format!("seed {seed} step {step}: view"));
                continue;
            }
            let append = r.below(3) > 0;
            let first = f.app.session().unwrap().doc.len_chars() / 2;
            f.app.dispatch(Command::SetCursor(CharPos(first)));
            let before = f.word_at_cursor();
            let call = if append {
                calls += 1;
                calls - 1
            } else {
                r.below(calls)
            };
            f.send(&random_entry(&mut r, call));
            f.check(&format!("seed {seed} step {step}"));
            if append {
                assert_eq!(f.word_at_cursor(), before, "seed {seed} step {step}");
            }
        }
    }
}

/// The time per message with a 10,000-entry log fed by a source: an
/// append and a correction. Printed; the bound is loose so slow machines
/// pass.
#[test]
fn ten_thousand_fed_entries_update_quickly() {
    let mut f = fed();
    let whole: Vec<String> = (0..10_000)
        .map(|i| format!(r#"{{"callsign": "K{i:05}X", "name": "Ada{i}"}}"#))
        .collect();
    f.send(&format!("[{}]", whole.join(",")));
    assert_eq!(f.app.live_log().unwrap().len(), 10_000);
    let mid = f.app.session().unwrap().doc.len_chars() / 2;
    f.app.dispatch(Command::SetCursor(CharPos(mid)));
    let before = f.word_at_cursor();
    let rounds = 10u32;
    let started = Instant::now();
    for i in 0..rounds {
        f.send(&format!(r#"{{"callsign": "N{i:04}X", "name": "New"}}"#));
    }
    let append = started.elapsed() / rounds;
    let started = Instant::now();
    for i in 0..rounds {
        f.send(&format!(
            r#"{{"callsign": "K00000X", "name": "Ada0", "notes": "Correction {i}"}}"#
        ));
    }
    let correct = started.elapsed() / rounds;
    eprintln!("fed log, 10,000 entries: append {append:?}, correction {correct:?}");
    f.check("after");
    assert_eq!(f.word_at_cursor(), before);
    assert!(append < Duration::from_secs(2) && correct < Duration::from_secs(2));
}
