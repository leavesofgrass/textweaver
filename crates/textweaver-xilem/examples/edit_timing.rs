//! The cost of one typed key in edit mode, as the window pays it: the app's
//! edit, the window's refresh (the document view's model), and the passes
//! Masonry runs for the next frame (layout, paint's plan, and the
//! accessibility tree update a screen reader receives). Rendering to the
//! GPU is left out; so are the platform adapters, which get the same tree
//! update.
//!
//! It prints, for typing and for Backspace, the median and slowest time of
//! each part, and how many AccessKit nodes each key sent (each is work for
//! UI Automation and AT-SPI, and events for NVDA, JAWS, and Orca).
//!
//! No window, no speech, no audio.
//!
//! ```text
//! cargo run --release -p textweaver-xilem --example edit_timing -- \
//!     --home SCRATCH --file target/bench-corpus/md-1mb.md [--keys 40]
//! ```

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

use masonry_testing::{TestHarness, TestHarnessParams};
use textweaver_app::Command;
use textweaver_app::a11y::LogAnnouncer;
use textweaver_app::core::CharPos;
use textweaver_app::keymap::ActionId;
use textweaver_xilem::gui::{self, DOC};
use textweaver_xilem::setup::{self, Options};
use textweaver_xilem::theme::{self, Palette};

fn main() {
    let mut file: Option<PathBuf> = None;
    let mut home: Option<PathBuf> = None;
    let mut keys = 40usize;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--file" => file = args.next().map(PathBuf::from),
            "--home" => home = args.next().map(PathBuf::from),
            "--keys" => keys = args.next().and_then(|s| s.parse().ok()).unwrap_or(keys),
            other => {
                eprintln!("edit_timing: unknown argument {other}");
                std::process::exit(2);
            }
        }
    }
    let (Some(file), Some(home)) = (file, home) else {
        eprintln!("edit_timing: --file PATH and --home SCRATCH are required");
        std::process::exit(2);
    };
    let opts = Options {
        no_speech: true,
        home: Some(home),
        ..Options::default()
    };
    let (mut app, _) = setup::build_app(&opts, Box::new(LogAnnouncer::default()));
    if let Err(e) = app.open(&file) {
        eprintln!("edit_timing: cannot open {}: {e}", file.display());
        std::process::exit(1);
    }
    let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
    let len = app.session().map_or(0, |s| s.doc.len_chars());
    // The middle of the document, at the end of a line (typing there is
    // the common case: text goes on after what is written).
    let middle = app.session().map_or(0, |s| {
        let text = s
            .doc
            .slice(textweaver_app::core::CharRange::new(len / 2, len));
        len / 2 + text.find('\n').map_or(0, |b| text[..b].chars().count())
    });
    let _ = app.dispatch(Command::SetCursor(CharPos(middle)));

    let p = Palette::galaxy();
    let tree = gui::build_tree(
        &p,
        textweaver_xilem::fonts::doc_font(&app.settings().reading_aids.font),
        Some(&app),
        Rc::new(Cell::new(0)),
        Default::default(),
    );
    let mut params = TestHarnessParams::default();
    params.window_size = (1100, 780).into();
    let mut h = TestHarness::create_with(theme::default_properties(&p), tree.root, params);
    for b in textweaver_xilem::fonts::bundled_blobs() {
        h.register_fonts(b);
    }
    let mut r = gui::Refresher::default();
    let t = Instant::now();
    let _ = r.refresh(&app, &mut h);
    let (_, first) = h.redraw();
    println!(
        "{} chars, edit mode, caret at {middle}; first frame {:.1} ms, {} nodes",
        len,
        t.elapsed().as_secs_f64() * 1000.0,
        first.nodes.len()
    );
    let doc = h.get_widget(DOC).id();
    h.focus_on(Some(doc));
    let _ = h.redraw();

    for (name, cmd) in [
        ("typing", Command::Insert("x".into())),
        ("backspace", Command::DeleteBack),
    ] {
        let mut edit = Vec::new();
        let mut refresh = Vec::new();
        let mut passes = Vec::new();
        let mut total = Vec::new();
        let mut nodes = Vec::new();
        for i in 0..keys + 3 {
            let t0 = Instant::now();
            let _ = app.dispatch(cmd.clone());
            let t1 = Instant::now();
            let _ = r.refresh(&app, &mut h);
            let t2 = Instant::now();
            let (_, update) = h.redraw();
            let t3 = Instant::now();
            // The first keys warm the caches.
            if i >= 3 {
                edit.push(ms(t1 - t0));
                refresh.push(ms(t2 - t1));
                passes.push(ms(t3 - t2));
                total.push(ms(t3 - t0));
                nodes.push(update.nodes.len() as f64);
            }
        }
        println!(
            "{name}, {keys} keys: total {}; app edit {}; window refresh {}; \
             layout and accessibility {}; nodes sent {}",
            stats(&mut total),
            stats(&mut edit),
            stats(&mut refresh),
            stats(&mut passes),
            stats(&mut nodes)
        );
    }
}

fn ms(d: std::time::Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// "median M, slowest S".
fn stats(v: &mut [f64]) -> String {
    v.sort_by(f64::total_cmp);
    let median = v.get(v.len() / 2).copied().unwrap_or(0.0);
    let max = v.last().copied().unwrap_or(0.0);
    format!("median {median:.2}, slowest {max:.2}")
}
