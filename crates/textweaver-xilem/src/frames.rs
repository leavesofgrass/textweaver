//! `--measure-frames MOVES`: the frame-time probe (W8b-i), the gate for
//! Wave 9's visual work (the performance report's section C).
//!
//! It reads a document aloud on the silent `paced` backend (words timed
//! like a real engine, at 900 words per minute so the probe is quick),
//! with no window on screen, and each time the spoken word moves it does
//! what the window does per frame, over the whole widget tree:
//!
//! 1. the driver's refresh, through [`gui::Refresher`] (the same
//!    `refresh_host` the window runs), which sends the spoken word and
//!    sentence to the document view and slides the window's text;
//! 2. Masonry's passes over the whole tree: layout (the document view
//!    asks for it on every move, so its ancestors are laid out too),
//!    paint (the scene encoded, not rasterized), and the accessibility
//!    tree.
//!
//! Per move it records the time, the allocations made on this thread
//! (with the `alloc-count` feature, which `cargo xtask frames` builds),
//! and the accessibility nodes the document view sent. It reports the
//! median, 95th percentile, and worst time, and the allocations and nodes
//! per move, in words, and as JSON with `--frames-json`. Counts do not
//! move with the machine's load, so they are what the gate checks; times
//! are reported.
//!
//! Options: `--frames-aids` turns every reading aid on first (bionic
//! reading, difficult words, syllables, the ruler with its band, text
//! spacing, and RSVP); `--scale` sets the scale (1 is 100 percent).
//!
//! Not included: rasterizing on the GPU, which depends on the graphics
//! card, the UI thread's wait for it, and the platform's accessibility
//! adapter (the harness uses `accesskit_consumer`). Those are measured by
//! hand in a real window. It is built on the same headless harness as
//! `--screenshot` (`screenshot.rs`), with the bundled fonts.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use masonry_testing::{TestHarness, TestHarnessParams};
use serde_json::{Value, json};
use textweaver_app::Command;
use textweaver_app::a11y::LogAnnouncer;
use textweaver_app::core::Rate;
use textweaver_app::keymap::ActionId;
use textweaver_app::store::{Paths, SettingsStore};

use crate::gui::{self, DOC, Refresher};
pub use crate::log::quantile;
use crate::setup::{self, Options};
use crate::theme::{self, Palette};

/// The reading rate of the probe: the fastest, so a word arrives every
/// 67 ms.
pub const PROBE_WPM: u16 = Rate::MAX_WPM;

/// What to measure.
#[derive(Clone, Debug)]
pub struct FrameOptions {
    /// The document to read.
    pub file: PathBuf,
    /// Keep settings and state here (never the user's own folder). The
    /// probe writes its reading rate into its settings.
    pub home: PathBuf,
    /// Highlight moves to measure.
    pub moves: usize,
    /// Give up after this long, however many moves were measured.
    pub limit: Duration,
    /// Logical window size.
    pub size: (u32, u32),
    /// Scale factor (1.0 is 100 percent).
    pub scale: f64,
    /// Every reading aid on.
    pub aids: bool,
    /// Theme name (the saved theme otherwise).
    pub theme: Option<String>,
}

impl FrameOptions {
    /// 200 moves of `file` at 100 percent, aids off, with a two-minute
    /// limit.
    pub fn new(file: PathBuf, home: PathBuf) -> Self {
        FrameOptions {
            file,
            home,
            moves: 200,
            limit: Duration::from_secs(120),
            size: (1100, 780),
            scale: 1.0,
            aids: false,
            theme: None,
        }
    }
}

/// The frames measured.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameReport {
    /// Highlight moves measured.
    pub moves: usize,
    /// Moves on which the sentence band was set.
    pub with_sentence: usize,
    /// Whole move (refresh and passes), in milliseconds: median.
    pub median_ms: f64,
    /// The 95th percentile.
    pub p95_ms: f64,
    /// The worst move.
    pub worst_ms: f64,
    /// The median of the driver's refresh alone.
    pub refresh_median_ms: f64,
    /// The median of Masonry's passes alone (layout, paint, accessibility).
    pub passes_median_ms: f64,
    /// Allocations on the measuring thread over every move (`None`
    /// without the `alloc-count` feature).
    pub allocs: Option<u64>,
    /// Accessibility nodes the document view sent over every move.
    pub nodes: u64,
}

impl FrameReport {
    /// The report as JSON. The totals end in `_allocs` and `_nodes`, which
    /// the bench gate checks as counts; the times are reported.
    pub fn to_json(&self) -> Value {
        let mut v = json!({
            "moves": self.moves,
            "moves_with_sentence": self.with_sentence,
            "move_p50_ms": self.median_ms,
            "move_p95_ms": self.p95_ms,
            "move_worst_ms": self.worst_ms,
            "refresh_p50_ms": self.refresh_median_ms,
            "passes_p50_ms": self.passes_median_ms,
            "moves_nodes": self.nodes,
        });
        if let (Some(a), Some(o)) = (self.allocs, v.as_object_mut()) {
            o.insert("moves_allocs".into(), json!(a));
        }
        v
    }

    fn per_move(&self, total: u64) -> f64 {
        total as f64 / self.moves.max(1) as f64
    }

    /// Plain lines, the meaning first, for the terminal and a Braille
    /// display.
    pub fn lines(&self) -> Vec<String> {
        let mut out = vec![
            format!(
                "Moves measured: {}, {} with the sentence band.",
                self.moves, self.with_sentence
            ),
            format!("Median move: {:.3} ms.", self.median_ms),
            format!("95th percentile: {:.3} ms.", self.p95_ms),
            format!("Worst move: {:.3} ms.", self.worst_ms),
            format!("Median refresh: {:.3} ms.", self.refresh_median_ms),
            format!("Median passes: {:.3} ms.", self.passes_median_ms),
            format!("Nodes sent per move: {:.1}.", self.per_move(self.nodes)),
        ];
        match self.allocs {
            Some(a) => out.push(format!("Allocations per move: {:.1}.", self.per_move(a))),
            None => out.push("Allocations: not counted in this build.".into()),
        }
        out
    }
}

fn sorted(mut v: Vec<f64>) -> Vec<f64> {
    v.sort_by(f64::total_cmp);
    v
}

/// The probe's settings: the defaults, at [`PROBE_WPM`], written into
/// `home` before the app starts (the paced backend takes its word time
/// from them).
fn prepare_home(home: &Path) -> Result<(), String> {
    let store = SettingsStore::new(Paths::under(home));
    let (mut settings, _) = store.load();
    if settings.speech.rate != Rate::Wpm(PROBE_WPM) {
        settings.speech.rate = Rate::Wpm(PROBE_WPM);
        store
            .save(&settings)
            .map_err(|e| format!("cannot write the probe's settings: {e}"))?;
    }
    Ok(())
}

/// Reads `opts.file` and measures `opts.moves` highlight moves.
pub fn measure(opts: &FrameOptions) -> Result<FrameReport, String> {
    prepare_home(&opts.home)?;
    let app_opts = Options {
        backend: Some(setup::PACED_BACKEND.into()),
        home: Some(opts.home.clone()),
        ..Options::default()
    };
    let (mut app, _) = setup::build_app(&app_opts, Box::new(LogAnnouncer::default()));
    app.open(&opts.file)
        .map_err(|e| format!("cannot open {}: {e}", opts.file.display()))?;
    if opts.aids {
        crate::screenshot::turn_on_aids(&mut app)?;
    }
    let palette = match opts.theme.as_deref() {
        Some(name) => Palette::named(name),
        None => Palette::from_theme(&app.reading_theme()),
    };
    let font = crate::fonts::doc_font(&app.settings().reading_aids.font);
    let tree = gui::build_tree(
        &palette,
        font,
        Some(&app),
        Rc::new(Cell::new(0)),
        Default::default(),
    );
    let (w, h) = opts.size;
    let scale = opts.scale.clamp(0.5, 4.0);
    let mut params = TestHarnessParams::default();
    params.window_size = ((f64::from(w) * scale) as u32, (f64::from(h) * scale) as u32).into();
    params.scale_factor = scale;
    params.background_color = theme::color(palette.background);
    params.root_padding = 0;
    let mut harness =
        TestHarness::create_with(theme::default_properties(&palette), tree.root, params);
    for blob in crate::fonts::bundled_blobs() {
        harness.register_fonts(blob);
    }
    let mut refresher = Refresher::default();
    refresher.refresh(&app, &mut harness);
    let doc_id = harness.get_widget(DOC).id();
    harness.focus_on(Some(doc_id));
    let _ = harness.redraw();

    let _ = app.dispatch(Command::Action(ActionId::ReadFromCursor));
    let start = Instant::now();
    let mut last = app.session().and_then(|s| s.spoken);
    let (mut whole, mut refresh, mut passes) = (Vec::new(), Vec::new(), Vec::new());
    let (mut with_sentence, mut nodes) = (0, 0u64);
    let mut allocs = counting::Window::default();
    while whole.len() < opts.moves && start.elapsed() < opts.limit {
        let _ = app.poll_speech();
        let spoken = app.session().and_then(|s| s.spoken);
        if spoken == last {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }
        last = spoken;
        if app.session().is_some_and(|s| s.spoken_sentence.is_some()) {
            with_sentence += 1;
        }
        harness.edit_widget(DOC, |d| d.widget.last_nodes_sent = 0);
        allocs.begin();
        let t = Instant::now();
        refresher.refresh(&app, &mut harness);
        let refreshed = t.elapsed();
        let _ = harness.redraw();
        let total = t.elapsed();
        allocs.end();
        nodes += harness.get_widget(DOC).inner().last_nodes_sent as u64;
        whole.push(total.as_secs_f64() * 1000.0);
        refresh.push(refreshed.as_secs_f64() * 1000.0);
        passes.push((total - refreshed).as_secs_f64() * 1000.0);
    }
    let _ = app.dispatch(Command::Action(ActionId::Stop));
    app.shutdown();
    let moves = whole.len();
    let whole = sorted(whole);
    Ok(FrameReport {
        moves,
        with_sentence,
        median_ms: quantile(&whole, 0.5),
        p95_ms: quantile(&whole, 0.95),
        worst_ms: whole.last().copied().unwrap_or(0.0),
        refresh_median_ms: quantile(&sorted(refresh), 0.5),
        passes_median_ms: quantile(&sorted(passes), 0.5),
        allocs: allocs.total(),
        nodes,
    })
}

/// Runs [`measure`], prints the report, and writes it as JSON to `json`
/// when given.
pub fn run(opts: &FrameOptions, json: Option<&Path>) -> Result<FrameReport, String> {
    let report = measure(opts)?;
    for l in report.lines() {
        println!("{l}");
    }
    if let Some(path) = json {
        let text = serde_json::to_string_pretty(&report.to_json())
            .map_err(|e| format!("cannot write the report: {e}"))?;
        std::fs::write(path, text + "\n")
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    }
    Ok(report)
}

/// Counting this thread's allocations, with the `alloc-count` feature.
#[cfg(feature = "alloc-count")]
#[allow(unsafe_code)]
mod counting {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    /// The system allocator, counting each thread's allocation calls.
    struct Counting;

    thread_local! {
        // A const `Cell` never allocates, so the allocator may touch it.
        static ALLOCS: Cell<u64> = const { Cell::new(0) };
    }

    fn counted() {
        let _ = ALLOCS.try_with(|n| n.set(n.get() + 1));
    }

    // SAFETY: every call forwards to the system allocator with the same
    // arguments; the counter never affects the returned pointers.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            counted();
            // SAFETY: forwarded unchanged.
            unsafe { System.alloc(layout) }
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            counted();
            // SAFETY: forwarded unchanged.
            unsafe { System.alloc_zeroed(layout) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            // SAFETY: forwarded unchanged; `ptr` came from this allocator.
            unsafe { System.dealloc(ptr, layout) }
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            counted();
            // SAFETY: forwarded unchanged; `ptr` came from this allocator.
            unsafe { System.realloc(ptr, layout, new_size) }
        }
    }

    #[global_allocator]
    static ALLOC: Counting = Counting;

    /// Allocations made between `begin` and `end`, summed.
    #[derive(Default)]
    pub struct Window {
        at: u64,
        total: u64,
    }

    impl Window {
        pub fn begin(&mut self) {
            self.at = ALLOCS.with(Cell::get);
        }
        pub fn end(&mut self) {
            self.total += ALLOCS.with(Cell::get).saturating_sub(self.at);
        }
        pub fn total(&self) -> Option<u64> {
            Some(self.total)
        }
    }
}

/// Without the `alloc-count` feature nothing is counted.
#[cfg(not(feature = "alloc-count"))]
mod counting {
    /// Counts nothing.
    #[derive(Default)]
    pub struct Window {
        _nothing: (),
    }

    impl Window {
        pub fn begin(&mut self) {}
        pub fn end(&mut self) {}
        pub fn total(&self) -> Option<u64> {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_report_reads_in_words_and_names_counts_for_the_gate() {
        let r = FrameReport {
            moves: 200,
            with_sentence: 200,
            median_ms: 0.5,
            p95_ms: 1.0,
            worst_ms: 3.0,
            refresh_median_ms: 0.1,
            passes_median_ms: 0.4,
            allocs: Some(40_000),
            nodes: 1_000,
        };
        let lines = r.lines();
        assert_eq!(lines[0], "Moves measured: 200, 200 with the sentence band.");
        assert!(lines.contains(&"Allocations per move: 200.0.".to_owned()));
        assert!(lines.contains(&"Nodes sent per move: 5.0.".to_owned()));
        let v = r.to_json();
        assert_eq!(v["moves_allocs"], 40_000);
        assert_eq!(v["moves_nodes"], 1_000);
        let none = FrameReport::default();
        assert!(none.to_json().get("moves_allocs").is_none());
        assert_eq!(quantile(&[1.0, 2.0, 3.0], 0.5), 2.0);
        assert_eq!(quantile(&[], 0.5), 0.0);
    }
}
