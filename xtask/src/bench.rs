//! `cargo xtask bench`: timings and memory for the reading and authoring hot
//! paths, on generated Markdown corpora and the fixtures.
//!
//! `cargo xtask bench` rebuilds xtask in release mode with the `bench`
//! feature and runs `bench-run`, so the numbers are optimized-build numbers.
//! Options (passed through):
//!
//! - `--quick`: the 1 MB and 50k-line corpora and the fixtures only (a
//!   couple of minutes; what CI runs).
//! - `--file PATH` (repeatable): bench these files instead of the corpora.
//! - `--json PATH`: also write every measurement as JSON.
//! - `--only NAME`: run only the corpus whose name contains `NAME`.
//! - `--baseline FILE`: compare with an earlier `--json` file, and fail
//!   when a memory number grew more than `--max-ratio R` times (default
//!   2). Only peak heap and allocation counts are gated: they barely vary
//!   from run to run, while times depend on the machine and its load, so
//!   times are reported for information only. Numbers too small to matter
//!   (under 1 MB of peak heap, under 5,000 allocations) are not gated.
//! - `--no-startup`: skip the startup timings.
//!
//! Startup timings (also `cargo xtask startup` on its own): the release
//! `tw` is built and `tw --version`, `tw text`, `tw info` (both on the
//! 1 MB corpus), and `tw backends` are each run five times after one
//! warm-up run; the median, fastest, and slowest wall times are reported.
//!
//! What is measured, per document (each line of the report is one number):
//!
//! - load: `Registry::load` from the file; chars, lines, markers.
//! - plan: narration planning of the whole document, which is what reading
//!   from the top does.
//! - open to first speech: `App::open`, then Read from cursor, until a
//!   backend is handed the first document utterance.
//! - navigation: dispatch time and keystroke-to-speech latency for next
//!   word, sentence, paragraph, and heading, idle and while reading, from
//!   the middle of the document.
//! - highlight: cost of applying one word position (`poll_speech_step`).
//! - search: a common word, and a regular expression.
//! - edit: entering edit mode, typing one character (per keystroke),
//!   backspace, an autosave snapshot, and leaving edit mode.
//! - state: saving the reading position.
//!
//! Peak heap is measured with a counting global allocator for each phase.
//! The speech backend is a silent clock that records when it is asked to
//! speak; nothing is played.

// Without the feature only `run` is used; the generators are also tested.
#![cfg_attr(not(feature = "bench"), allow(dead_code))]

use std::path::{Path, PathBuf};
use std::process::{Command as Process, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use serde_json::{Value, json};

/// Parsed `bench` options.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Args {
    pub quick: bool,
    pub files: Vec<PathBuf>,
    pub json: Option<PathBuf>,
    pub only: Option<String>,
    pub baseline: Option<PathBuf>,
    pub max_ratio: f64,
    pub startup: bool,
    /// The `tw` binary for the startup timings (passed to `bench-run`).
    pub tw: Option<PathBuf>,
}

impl Default for Args {
    fn default() -> Self {
        Args {
            quick: false,
            files: Vec::new(),
            json: None,
            only: None,
            baseline: None,
            max_ratio: DEFAULT_MAX_RATIO,
            startup: true,
            tw: None,
        }
    }
}

/// How much a gated number may grow before the gate fails.
pub(crate) const DEFAULT_MAX_RATIO: f64 = 2.0;

impl Args {
    pub(crate) fn parse(args: &[String]) -> anyhow::Result<Args> {
        let mut out = Args::default();
        let mut it = args.iter();
        let value = |it: &mut std::slice::Iter<'_, String>, name: &str| {
            it.next()
                .cloned()
                .with_context(|| format!("{name} needs a value"))
        };
        while let Some(a) = it.next() {
            match a.as_str() {
                "--quick" => out.quick = true,
                "--file" => out.files.push(value(&mut it, "--file")?.into()),
                "--json" => out.json = Some(value(&mut it, "--json")?.into()),
                "--only" => out.only = Some(value(&mut it, "--only")?),
                "--baseline" => out.baseline = Some(value(&mut it, "--baseline")?.into()),
                "--max-ratio" => {
                    let v = value(&mut it, "--max-ratio")?;
                    out.max_ratio = v
                        .parse()
                        .ok()
                        .filter(|r: &f64| *r >= 1.0)
                        .with_context(|| format!("--max-ratio {v}: give a number of 1 or more"))?;
                }
                "--no-startup" => out.startup = false,
                "--tw" => out.tw = Some(value(&mut it, "--tw")?.into()),
                other => bail!(
                    "unknown option {other} (cargo xtask bench [--quick] [--file PATH] [--json PATH] [--only NAME] [--baseline FILE] [--max-ratio R] [--no-startup])"
                ),
            }
        }
        Ok(out)
    }
}

fn cargo() -> Process {
    Process::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
}

/// Builds the release `tw` (default features) and returns its path.
fn build_tw() -> anyhow::Result<PathBuf> {
    let status = cargo()
        .current_dir(root())
        .args([
            "build",
            "--quiet",
            "--release",
            "-p",
            "textweaver-cli",
            "--bin",
            "tw",
        ])
        .status()
        .context("running cargo")?;
    if !status.success() {
        bail!("building tw failed: {status}");
    }
    Ok(crate::eci::target_dir(&root())
        .join("release")
        .join(format!("tw{}", std::env::consts::EXE_SUFFIX)))
}

/// `cargo xtask bench`: re-runs xtask in release mode with the bench
/// feature.
pub fn run() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let parsed = Args::parse(&args)?;
    let mut extra = Vec::new();
    if parsed.startup && parsed.tw.is_none() {
        extra.push("--tw".to_owned());
        extra.push(build_tw()?.display().to_string());
    }
    let status = cargo()
        .args([
            "run",
            "--quiet",
            "--release",
            "--package",
            "xtask",
            "--features",
            "bench",
            "--",
            "bench-run",
        ])
        .args(&args)
        .args(&extra)
        .status()?;
    if !status.success() {
        bail!("bench failed: {status}");
    }
    Ok(())
}

/// The key of the startup timings in the JSON report.
pub(crate) const STARTUP_KEY: &str = "startup";
/// Timed runs of each startup command (after one warm-up run).
pub(crate) const STARTUP_RUNS: usize = 5;

/// The corpus the startup timings read (`md-1mb.md`), written if needed.
pub(crate) fn startup_corpus() -> anyhow::Result<PathBuf> {
    let dir = corpus_dir();
    std::fs::create_dir_all(&dir)?;
    let p = dir.join("md-1mb.md");
    let text = markdown_corpus(1 << 20, 1);
    if std::fs::read_to_string(&p).ok().as_deref() != Some(text.as_str()) {
        std::fs::write(&p, &text)?;
    }
    Ok(p)
}

/// Median, fastest, and slowest of `samples`, in milliseconds.
fn summary(samples: &[Duration]) -> Value {
    let mut ms: Vec<f64> = samples.iter().map(|d| d.as_secs_f64() * 1000.0).collect();
    ms.sort_by(f64::total_cmp);
    let median = ms.get(ms.len() / 2).copied().unwrap_or(0.0);
    json!({
        "median_ms": median,
        "min_ms": ms.first().copied().unwrap_or(0.0),
        "max_ms": ms.last().copied().unwrap_or(0.0),
        "n": ms.len(),
    })
}

/// Times `tw --version`, `tw text`, `tw info`, and `tw backends` (the last
/// three on `corpus`): one warm-up run, then `runs` timed runs each. A
/// command that fails is an error.
pub(crate) fn startup_timings(tw: &Path, corpus: &Path, runs: usize) -> anyhow::Result<Value> {
    let corpus_arg = corpus.display().to_string();
    let commands: [(&str, Vec<&str>); 4] = [
        ("tw --version", vec!["--version"]),
        ("tw text", vec!["text", &corpus_arg]),
        ("tw info", vec!["info", &corpus_arg]),
        ("tw backends", vec!["backends"]),
    ];
    println!("startup ({})", tw.display());
    let mut out = serde_json::Map::new();
    for (name, args) in commands {
        let mut samples = Vec::new();
        for i in 0..=runs {
            let t = Instant::now();
            let status = Process::new(tw)
                .args(&args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .with_context(|| format!("running {}", tw.display()))?;
            let d = t.elapsed();
            if !status.success() {
                bail!("{name} failed ({status})");
            }
            if i > 0 {
                samples.push(d);
            }
        }
        let s = summary(&samples);
        println!(
            "  {name}: median {:.1} ms, fastest {:.1} ms, slowest {:.1} ms",
            s["median_ms"].as_f64().unwrap_or(0.0),
            s["min_ms"].as_f64().unwrap_or(0.0),
            s["max_ms"].as_f64().unwrap_or(0.0)
        );
        out.insert(name.to_owned(), s);
    }
    println!();
    Ok(Value::Object(out))
}

/// `cargo xtask startup [--tw PATH] [--json PATH]`: the startup timings on
/// their own.
pub fn startup() -> anyhow::Result<()> {
    let args = Args::parse(&std::env::args().skip(2).collect::<Vec<_>>())?;
    let tw = match args.tw {
        Some(t) => t,
        None => build_tw()?,
    };
    let v = startup_timings(&tw, &startup_corpus()?, STARTUP_RUNS)?;
    let report = json!({ STARTUP_KEY: v });
    if let Some(p) = args.json {
        std::fs::write(&p, serde_json::to_string_pretty(&report)?)?;
        println!("wrote {}", p.display());
    }
    if let Some(base) = &args.baseline {
        gate(&report, base, args.max_ratio)?;
    }
    Ok(())
}

/// Numbers below these are not gated: too small to matter, and relatively
/// noisy.
const PEAK_FLOOR_MB: f64 = 1.0;
const ALLOC_FLOOR: f64 = 5_000.0;

/// What [`compare`] found.
#[derive(Debug, Default)]
pub(crate) struct Comparison {
    /// Plain-sentence lines for the log and the job summary.
    pub lines: Vec<String>,
    /// The gated numbers over the limit.
    pub failures: Vec<String>,
}

fn number(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.get("mean_ms").and_then(Value::as_f64))
        .or_else(|| v.get("median_ms").and_then(Value::as_f64))
}

/// Compares a bench report with a baseline report. Peak heap (`*_peak_mb`)
/// and allocation counts (`*_allocs`) fail above `max_ratio` times the
/// baseline; times are only reported.
pub(crate) fn compare(current: &Value, baseline: &Value, max_ratio: f64) -> Comparison {
    let mut c = Comparison::default();
    let Some(docs) = current.as_object() else {
        c.lines.push("The report is empty.".into());
        return c;
    };
    for (doc, cur) in docs {
        let Some(base) = baseline.get(doc) else {
            c.lines
                .push(format!("{doc}: no baseline, so nothing to compare."));
            continue;
        };
        let Some(cur) = cur.as_object() else {
            continue;
        };
        let mut gated = 0;
        let mut worst: Option<(f64, String)> = None;
        let mut slowest: Option<(f64, String)> = None;
        for (key, v) in cur {
            let (Some(now), Some(then)) = (number(v), base.get(key).and_then(number)) else {
                continue;
            };
            let ratio = if then > 0.0 {
                now / then
            } else {
                f64::INFINITY
            };
            let floor = if key.ends_with("_peak_mb") {
                Some(PEAK_FLOOR_MB)
            } else if key.ends_with("_allocs") {
                Some(ALLOC_FLOOR)
            } else {
                None
            };
            match floor {
                Some(floor) => {
                    if now < floor {
                        continue;
                    }
                    gated += 1;
                    if worst.as_ref().is_none_or(|(r, _)| ratio > *r) {
                        worst = Some((ratio, key.clone()));
                    }
                    if ratio > max_ratio {
                        c.failures.push(format!(
                            "{doc}: {key} grew from {then:.1} to {now:.1}, {ratio:.2} times the baseline (the limit is {max_ratio})."
                        ));
                    }
                }
                None if (key.ends_with("_ms") || v.is_object())
                    && then >= 0.05
                    && slowest.as_ref().is_none_or(|(r, _)| ratio > *r) =>
                {
                    slowest = Some((ratio, key.clone()));
                }
                None => {}
            }
        }
        let mut line = format!("{doc}: {gated} memory numbers checked");
        if let Some((r, k)) = worst {
            line.push_str(&format!(", the largest change {r:.2} times ({k})"));
        }
        if let Some((r, k)) = slowest {
            line.push_str(&format!(
                "; for information, the largest time change {r:.2} times ({k})"
            ));
        }
        line.push('.');
        c.lines.push(line);
    }
    c
}

/// Compares `report` with the baseline file and fails when a gated number
/// is over the limit. Writes the result to the GitHub job summary when
/// there is one.
pub(crate) fn gate(report: &Value, baseline: &Path, max_ratio: f64) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(baseline)
        .with_context(|| format!("reading the baseline {}", baseline.display()))?;
    let base: Value = serde_json::from_str(&text)
        .with_context(|| format!("parsing the baseline {}", baseline.display()))?;
    let c = compare(report, &base, max_ratio);
    println!("Compared with {}:", baseline.display());
    for l in &c.lines {
        println!("  {l}");
    }
    for f in &c.failures {
        println!("  Over the limit: {f}");
    }
    if let Some(summary) = std::env::var_os("GITHUB_STEP_SUMMARY") {
        use std::io::Write as _;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(summary)
        {
            let _ = writeln!(f, "## Benchmark gate\n");
            for l in c.lines.iter() {
                let _ = writeln!(f, "- {l}");
            }
            for l in c.failures.iter() {
                let _ = writeln!(f, "- Over the limit: {l}");
            }
            if c.failures.is_empty() {
                let _ = writeln!(f, "\nNo memory number grew more than {max_ratio} times.");
            }
        }
    }
    if !c.failures.is_empty() {
        bail!(
            "{} memory numbers grew more than {max_ratio} times the baseline",
            c.failures.len()
        );
    }
    println!("No memory number grew more than {max_ratio} times.");
    Ok(())
}

/// The workspace root (xtask's parent directory).
pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// Where generated corpora go: `<target>/bench-corpus`.
pub(crate) fn corpus_dir() -> PathBuf {
    let target =
        std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root().join("target"), PathBuf::from);
    target.join("bench-corpus")
}

/// A small deterministic random source (no dependency; reproducible corpora).
pub(crate) struct Lcg(pub(crate) u64);

impl Lcg {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    pub(crate) fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next()).unwrap_or(0) % n.max(1)
    }
}

const WORDS: &[&str] = &[
    "the",
    "reader",
    "speech",
    "highlight",
    "document",
    "students",
    "accessible",
    "voice",
    "sentence",
    "paragraph",
    "heading",
    "markdown",
    "keyboard",
    "screen",
    "quickly",
    "reliable",
    "position",
    "library",
    "chapter",
    "notes",
    "table",
    "list",
    "link",
    "code",
    "writer",
    "and",
    "of",
    "to",
    "in",
    "is",
    "for",
    "with",
    "on",
    "as",
    "by",
    "every",
    "word",
    "never",
    "drifts",
    "Dr.",
    "e.g.",
    "3.5",
    "2026",
    "café",
    "naïve",
    "résumé",
    "O'Brien",
    "well-known",
    "U.S.",
];

fn sentence(r: &mut Lcg) -> String {
    let n = 6 + r.below(18);
    let mut s = String::new();
    for i in 0..n {
        let w = WORDS[r.below(WORDS.len())];
        if i == 0 {
            let mut c = w.chars();
            if let Some(f) = c.next() {
                s.extend(f.to_uppercase());
                s.push_str(c.as_str());
            }
        } else {
            s.push(' ');
            match r.below(40) {
                0 => {
                    s.push_str("**");
                    s.push_str(w);
                    s.push_str("**");
                }
                1 => {
                    s.push('`');
                    s.push_str(w);
                    s.push('`');
                }
                2 => {
                    s.push('[');
                    s.push_str(w);
                    s.push_str("](https://example.org/");
                    s.push_str(w);
                    s.push(')');
                }
                _ => s.push_str(w),
            }
        }
    }
    s.push(match r.below(10) {
        0 => '?',
        1 => '!',
        _ => '.',
    });
    s
}

/// Realistic Markdown of about `bytes` bytes: sections with headings,
/// paragraphs, lists, tables, code blocks, quotes, and footnotes.
pub fn markdown_corpus(bytes: usize, seed: u64) -> String {
    let mut r = Lcg(seed);
    let mut out = String::with_capacity(bytes + 4096);
    out.push_str("---\ntitle: Benchmark corpus\nauthor: xtask\n---\n\n# Benchmark corpus\n\n");
    let mut section = 0;
    while out.len() < bytes {
        section += 1;
        out.push_str(&format!("## Section {section}\n\n"));
        for _ in 0..(2 + r.below(4)) {
            let n = 2 + r.below(6);
            let p: Vec<String> = (0..n).map(|_| sentence(&mut r)).collect();
            out.push_str(&p.join(" "));
            if r.below(8) == 0 {
                out.push_str(&format!("[^{section}]"));
            }
            out.push_str("\n\n");
        }
        match r.below(6) {
            0 => {
                for i in 0..(3 + r.below(6)) {
                    out.push_str(&format!("- Item {i}: {}\n", sentence(&mut r)));
                    if r.below(4) == 0 {
                        out.push_str(&format!("  1. Nested {}\n", sentence(&mut r)));
                    }
                }
                out.push('\n');
            }
            1 => {
                out.push_str("| Name | Role | Score |\n|---|:---:|---:|\n");
                for i in 0..(2 + r.below(8)) {
                    let w = WORDS[r.below(WORDS.len())];
                    out.push_str(&format!("| {w} | reader {i} | {} |\n", r.below(100)));
                }
                out.push('\n');
            }
            2 => {
                out.push_str("```rust\nfn main() {\n    println!(\"hello\");\n}\n```\n\n");
            }
            3 => {
                out.push_str(&format!(
                    "> {}\n> {}\n\n",
                    sentence(&mut r),
                    sentence(&mut r)
                ));
            }
            4 => {
                out.push_str(&format!(
                    "### Subsection {section}\n\n{}\n\n",
                    sentence(&mut r)
                ));
            }
            _ => {
                out.push_str(&format!("[^{section}]: {}\n\n", sentence(&mut r)));
            }
        }
    }
    out
}

/// `lines` lines of one tight list: no blank line anywhere, so the whole
/// document is one paragraph (the worst case for paragraph-scoped
/// segmentation).
pub fn tight_list_corpus(lines: usize, seed: u64) -> String {
    let mut r = Lcg(seed);
    let mut out = String::from("# Long list\n\n");
    for i in 0..lines {
        out.push_str(&format!("- Line {i}: {}\n", sentence(&mut r)));
    }
    out
}

/// A document with one line of about `bytes` bytes (minified or unwrapped
/// text).
pub fn one_line_corpus(bytes: usize, seed: u64) -> String {
    let mut r = Lcg(seed);
    let mut out = String::with_capacity(bytes + 256);
    while out.len() < bytes {
        out.push_str(&sentence(&mut r));
        out.push(' ');
    }
    out.push('\n');
    out
}

#[cfg(feature = "bench")]
pub use inner::run_inner;

#[cfg(feature = "bench")]
#[allow(unsafe_code)]
pub mod alloc {
    //! A counting global allocator: current and peak heap bytes.

    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

    /// Wraps the system allocator and counts live bytes.
    pub struct Counting;

    static CURRENT: AtomicUsize = AtomicUsize::new(0);
    static PEAK: AtomicUsize = AtomicUsize::new(0);
    /// Allocation calls (alloc, alloc_zeroed, realloc) since the start.
    static ALLOCS: AtomicUsize = AtomicUsize::new(0);
    /// [`ALLOCS`] at the last [`reset_peak`].
    static ALLOCS_AT_RESET: AtomicUsize = AtomicUsize::new(0);

    fn grew(by: usize) {
        let now = CURRENT.fetch_add(by, Relaxed) + by;
        PEAK.fetch_max(now, Relaxed);
    }

    // SAFETY: every call forwards to the system allocator with the same
    // arguments; the counters are plain atomics and never affect the
    // returned pointers.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            // SAFETY: forwarded unchanged (the caller upholds `alloc`'s contract).
            let p = unsafe { System.alloc(layout) };
            if !p.is_null() {
                grew(layout.size());
                ALLOCS.fetch_add(1, Relaxed);
            }
            p
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            // SAFETY: forwarded unchanged.
            let p = unsafe { System.alloc_zeroed(layout) };
            if !p.is_null() {
                grew(layout.size());
                ALLOCS.fetch_add(1, Relaxed);
            }
            p
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            // SAFETY: forwarded unchanged; `ptr` came from this allocator.
            unsafe { System.dealloc(ptr, layout) };
            CURRENT.fetch_sub(layout.size(), Relaxed);
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            // SAFETY: forwarded unchanged; `ptr` came from this allocator.
            let q = unsafe { System.realloc(ptr, layout, new_size) };
            if !q.is_null() {
                ALLOCS.fetch_add(1, Relaxed);
                if new_size >= layout.size() {
                    grew(new_size - layout.size());
                } else {
                    CURRENT.fetch_sub(layout.size() - new_size, Relaxed);
                }
            }
            q
        }
    }

    /// Live heap bytes.
    pub fn current() -> usize {
        CURRENT.load(Relaxed)
    }

    /// Starts a new peak measurement at the current level, and a new count
    /// of allocations.
    pub fn reset_peak() {
        PEAK.store(CURRENT.load(Relaxed), Relaxed);
        ALLOCS_AT_RESET.store(ALLOCS.load(Relaxed), Relaxed);
    }

    /// The highest live heap since the last [`reset_peak`].
    pub fn peak() -> usize {
        PEAK.load(Relaxed)
    }

    /// Allocation calls since the last [`reset_peak`].
    pub fn allocs() -> usize {
        ALLOCS
            .load(Relaxed)
            .saturating_sub(ALLOCS_AT_RESET.load(Relaxed))
    }
}

#[cfg(feature = "bench")]
mod inner {
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use serde_json::{Value, json};
    use textweaver_app::a11y::LogAnnouncer;
    use textweaver_app::core::{Utterance, UtteranceKind};
    use textweaver_app::formats::{LoadOptions, Registry, Source};
    use textweaver_app::keymap::{ActionId, Frontend, Keymap, Platform};
    use textweaver_app::speech::{
        Caps, EventSink, RawEvent, ServiceConfig, SpeechBackend, SpeechError, SpeechService, Voice,
        VoiceParams,
    };
    use textweaver_app::store::{Paths, Settings};
    use textweaver_app::text::narrate::NarrationPolicy;
    use textweaver_app::{App, AppConfig, Command, parse_go_to};

    use super::alloc;

    /// When the backend was asked to speak what.
    #[derive(Clone, Default)]
    struct Clock(Arc<Mutex<Vec<(Instant, UtteranceKind)>>>);

    impl Clock {
        fn len(&self) -> usize {
            self.0.lock().map_or(0, |v| v.len())
        }

        /// Waits (up to `limit`) for a speak call after index `from` whose
        /// kind matches; returns when it happened.
        fn wait_after(&self, from: usize, text_only: bool, limit: Duration) -> Option<Instant> {
            let deadline = Instant::now() + limit;
            while Instant::now() < deadline {
                if let Ok(v) = self.0.lock()
                    && let Some((t, _)) = v
                        .iter()
                        .skip(from)
                        .find(|(_, k)| !text_only || *k == UtteranceKind::Text)
                {
                    return Some(*t);
                }
                std::thread::yield_now();
            }
            None
        }
    }

    /// A silent backend that records when it is asked to speak and reports
    /// the first word of the utterance it is "playing", then keeps speaking
    /// it (never finishes), like an engine in the middle of a long sentence.
    /// Queued lookahead utterances get no events until played, as with real
    /// engines.
    struct ClockBackend {
        clock: Clock,
        params: VoiceParams,
        playing: bool,
    }

    impl SpeechBackend for ClockBackend {
        fn id(&self) -> &'static str {
            "bench-clock"
        }
        fn capabilities(&self) -> Caps {
            Caps::WORD_EVENTS | Caps::PAUSE | Caps::LIVE_RATE | Caps::PITCH | Caps::VOLUME
        }
        fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
            Ok(vec![Voice {
                id: "bench".into(),
                name: "Bench".into(),
                ..Voice::default()
            }])
        }
        fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
            self.params = params.clone();
            Ok(())
        }
        fn effective_wpm(&self) -> u16 {
            self.params.rate.wpm()
        }
        fn speak(&mut self, u: &Utterance, sink: &mut dyn EventSink) -> Result<(), SpeechError> {
            // Only the utterance that would start playing counts: lookahead
            // handed over with it is not heard yet.
            if self.playing {
                return Ok(());
            }
            self.playing = true;
            if let Ok(mut v) = self.clock.0.lock() {
                v.push((Instant::now(), u.kind));
            }
            sink.emit(u.id, RawEvent::Started);
            if let Some(end) = u.text.find(' ') {
                sink.emit(
                    u.id,
                    RawEvent::Word {
                        byte_range: 0..u32::try_from(end).unwrap_or(0),
                        audio_ms: None,
                    },
                );
            }
            Ok(())
        }
        fn stop(&mut self) {
            self.playing = false;
        }
    }

    fn clock_service() -> (SpeechService, Clock) {
        let clock = Clock::default();
        let c = clock.clone();
        let service = SpeechService::spawn(
            Box::new(move || {
                Ok(Box::new(ClockBackend {
                    clock: c,
                    params: VoiceParams::default(),
                    playing: false,
                }) as _)
            }),
            ServiceConfig::default(),
        )
        .expect("the bench backend cannot fail");
        (service, clock)
    }

    fn new_app(home: &Path, speech: SpeechService) -> App {
        let mut app = App::new(AppConfig {
            settings: Settings::default(),
            keymap: Keymap::defaults(Platform::current(), Frontend::Terminal),
            speech,
            paths: Some(Paths::under(home)),
            announcer: Box::new(LogAnnouncer::default()),
            self_voicing: true,
            backend_name: "bench".into(),
        });
        app.dispatch(Command::Resize {
            width: 100,
            height: 30,
        });
        app
    }

    fn ms(d: Duration) -> f64 {
        d.as_secs_f64() * 1000.0
    }

    fn mb(bytes: usize) -> f64 {
        bytes as f64 / (1024.0 * 1024.0)
    }

    /// Collected measurements for one document.
    struct Report {
        name: String,
        values: Vec<(String, Value)>,
    }

    impl Report {
        fn put(&mut self, key: &str, v: Value, line: String) {
            println!("  {line}");
            self.values.push((key.to_owned(), v));
        }

        fn time(&mut self, key: &str, label: &str, d: Duration) {
            self.put(key, json!(ms(d)), format!("{label}: {:.2} ms", ms(d)));
        }

        fn samples(&mut self, key: &str, label: &str, v: &[Duration]) {
            if v.is_empty() {
                return;
            }
            let mut s: Vec<f64> = v.iter().map(|d| ms(*d)).collect();
            s.sort_by(f64::total_cmp);
            let mean = s.iter().sum::<f64>() / s.len() as f64;
            let p50 = s[s.len() / 2];
            let max = s[s.len() - 1];
            self.put(
                key,
                json!({"mean_ms": mean, "p50_ms": p50, "max_ms": max, "n": s.len()}),
                format!("{label}: mean {mean:.3} ms, median {p50:.3} ms, max {max:.3} ms"),
            );
        }

        /// Records the peak heap (`key`, ending in `_peak_mb`) and the
        /// allocation count (the same key ending in `_allocs`) since the
        /// last `alloc::reset_peak`.
        fn peak(&mut self, key: &str, label: &str) {
            let p = alloc::peak();
            let n = alloc::allocs();
            let count_key = format!("{}_allocs", key.trim_end_matches("_peak_mb"));
            self.put(
                key,
                json!(mb(p)),
                format!("{label}: {:.1} MB peak heap, {n} allocations", mb(p)),
            );
            self.values.push((count_key, json!(n)));
        }
    }

    /// Dispatches `cmd` and waits for the backend to be asked to speak.
    /// Returns (dispatch time, keystroke-to-speech latency).
    fn keystroke(
        app: &mut App,
        clock: &Clock,
        cmd: Command,
        text_only: bool,
    ) -> (Duration, Option<Duration>) {
        let before = clock.len();
        let t0 = Instant::now();
        app.dispatch(cmd);
        let d = t0.elapsed();
        let spoke = clock
            .wait_after(before, text_only, Duration::from_millis(500))
            .map(|t| t - t0);
        app.poll_speech();
        (d, spoke)
    }

    fn bench_doc(name: &str, path: &Path, home: &Path) -> Report {
        let mut r = Report {
            name: name.to_owned(),
            values: Vec::new(),
        };
        let bytes = std::fs::metadata(path).map_or(0, |m| m.len());
        println!(
            "{name} ({:.2} MB, {})",
            bytes as f64 / 1_048_576.0,
            path.display()
        );
        r.values.push(("bytes".into(), json!(bytes)));

        // Load.
        let registry = Registry::with_builtins();
        alloc::reset_peak();
        let base = alloc::current();
        let t = Instant::now();
        let doc = match registry.load(&Source::Path(path.to_owned()), &LoadOptions::default()) {
            Ok(d) => d,
            Err(e) => {
                println!("  cannot load: {e}");
                return r;
            }
        };
        let load = t.elapsed();
        r.time("load_ms", "load", load);
        r.put(
            "doc",
            json!({"chars": doc.len_chars(), "lines": doc.line_count(), "markers": doc.markers().len()}),
            format!(
                "document: {} chars, {} lines, {} markers, {:.1} MB retained",
                doc.len_chars(),
                doc.line_count(),
                doc.markers().len(),
                mb(alloc::current().saturating_sub(base))
            ),
        );
        r.peak("load_peak_mb", "load");

        // Narration plan of the whole document (what reading from the top does).
        alloc::reset_peak();
        let t = Instant::now();
        let plan = textweaver_app::text::plan(&doc, doc.full_range(), &NarrationPolicy::default());
        let d = t.elapsed();
        r.time(
            "plan_all_ms",
            &format!("plan the whole document ({} utterances)", plan.len()),
            d,
        );
        r.peak("plan_all_peak_mb", "plan");
        drop(plan);
        drop(doc);

        // Open to first speech.
        let (speech, clock) = clock_service();
        let mut app = new_app(home, speech);
        alloc::reset_peak();
        let t0 = Instant::now();
        if let Err(e) = app.open(path) {
            println!("  cannot open: {e}");
            return r;
        }
        let opened = t0.elapsed();
        let before = clock.len();
        app.dispatch(Command::Action(ActionId::ReadFromCursor));
        let dispatched = t0.elapsed();
        let first = clock
            .wait_after(before, true, Duration::from_secs(60))
            .map(|t| t - t0);
        r.time("open_ms", "App::open", opened);
        r.time(
            "open_and_read_dispatch_ms",
            "open plus Read from cursor dispatch",
            dispatched,
        );
        match first {
            Some(d) => r.time("open_to_first_speech_ms", "open to first speech", d),
            None => println!("  open to first speech: no speech within 60 s"),
        }
        r.peak("open_peak_mb", "open and read");
        app.dispatch(Command::Action(ActionId::Stop));
        app.poll_speech();

        // Navigation from the middle, idle then reading.
        let middle = parse_go_to("50%").expect("50% is a go-to target");
        let nav = [
            ("word", ActionId::CaretNextWord),
            ("sentence", ActionId::NextSentence),
            ("paragraph", ActionId::NextParagraph),
            ("heading", ActionId::NextHeading),
        ];
        for reading in [false, true] {
            let state = if reading { "reading" } else { "idle" };
            for (unit, action) in nav {
                app.dispatch(Command::Action(ActionId::Stop));
                app.dispatch(Command::GoTo(middle));
                if reading {
                    app.dispatch(Command::Action(ActionId::ReadFromCursor));
                    std::thread::sleep(Duration::from_millis(20));
                    app.poll_speech();
                }
                alloc::reset_peak();
                let mut dispatch = Vec::new();
                let mut speak = Vec::new();
                for _ in 0..20 {
                    let (d, s) = keystroke(&mut app, &clock, Command::Action(action), false);
                    if std::env::var_os("TW_BENCH_DEBUG").is_some() {
                        println!(
                            "    {unit} ({state}): {:?}, {:?}, spoke {s:?}",
                            app.playback(),
                            app.status_text()
                        );
                    }
                    dispatch.push(d);
                    match s {
                        Some(s) => speak.push(s),
                        // The end of the document: nothing more to say
                        // (while reading, "No next ..." is not spoken).
                        None => break,
                    }
                }
                r.samples(
                    &format!("nav_{state}_{unit}_dispatch"),
                    &format!("next {unit} ({state}), dispatch"),
                    &dispatch,
                );
                r.samples(
                    &format!("nav_{state}_{unit}_to_speech"),
                    &format!("next {unit} ({state}), keystroke to speech"),
                    &speak,
                );
                r.peak(
                    &format!("nav_{state}_{unit}_peak_mb"),
                    &format!("next {unit} ({state})"),
                );
            }
        }
        app.dispatch(Command::Action(ActionId::Stop));
        app.poll_speech();

        // Highlight updates: one word position applied at a time.
        highlight(&mut r, path, home);

        // Search.
        app.dispatch(Command::GoTo(parse_go_to("start").expect("start")));
        for (key, label, pattern) in [
            ("find_word", "find \"the\"", "the"),
            ("find_rare", "find a rare word", "zyzzyva"),
        ] {
            alloc::reset_peak();
            let t = Instant::now();
            app.dispatch(Command::Find(pattern.into()));
            let d = t.elapsed();
            let hits = app
                .session()
                .and_then(|s| s.find.as_ref())
                .map_or(0, |f| f.hits.len());
            r.time(&format!("{key}_ms"), &format!("{label} ({hits} hits)"), d);
            r.peak(&format!("{key}_peak_mb"), label);
            let t = Instant::now();
            app.dispatch(Command::Action(ActionId::FindNext));
            r.time(
                &format!("{key}_next_ms"),
                &format!("{label}, find next"),
                t.elapsed(),
            );
        }
        app.dispatch(Command::Action(ActionId::Stop));

        // State saving (the first save also creates the state directory).
        app.dispatch(Command::GoTo(middle));
        let mut saves = Vec::new();
        for i in 0..6 {
            app.dispatch(Command::Action(ActionId::CaretNextWord));
            let t = Instant::now();
            let saved = app.save_position();
            if i > 0 {
                saves.push(t.elapsed());
            }
            if let Err(e) = saved {
                println!("  save position failed: {e}");
                break;
            }
        }
        r.samples("save_position", "save reading position", &saves);

        // Editing.
        alloc::reset_peak();
        let t = Instant::now();
        app.dispatch(Command::Action(ActionId::ToggleEditMode));
        r.time("edit_enter_ms", "enter edit mode", t.elapsed());
        r.peak("edit_enter_peak_mb", "enter edit mode");
        if app.is_editing() {
            let mut typing = Vec::new();
            let mut echo = Vec::new();
            alloc::reset_peak();
            for c in "Typing speed test ".chars() {
                let (d, s) = keystroke(&mut app, &clock, Command::Insert(c.to_string()), false);
                typing.push(d);
                echo.extend(s);
            }
            r.samples(
                "edit_type_dispatch",
                "type one character, dispatch",
                &typing,
            );
            r.samples(
                "edit_type_to_echo",
                "type one character, keystroke to echo",
                &echo,
            );
            r.peak("edit_type_peak_mb", "typing");
            let mut back = Vec::new();
            for _ in 0..10 {
                let (d, _) = keystroke(&mut app, &clock, Command::DeleteBack, false);
                back.push(d);
            }
            r.samples("edit_backspace_dispatch", "backspace, dispatch", &back);
            let mut line = Vec::new();
            for _ in 0..10 {
                let (d, _) = keystroke(
                    &mut app,
                    &clock,
                    Command::MoveCaret {
                        by: textweaver_app::CaretMove::Line,
                        direction: textweaver_app::core::Direction::Forward,
                        extend: false,
                    },
                    false,
                );
                line.push(d);
            }
            r.samples(
                "edit_line_down_dispatch",
                "caret down one line, dispatch",
                &line,
            );
            alloc::reset_peak();
            let mut autosaves = Vec::new();
            let mut now = Instant::now();
            for _ in 0..5 {
                app.dispatch(Command::Insert("x".into()));
                // Each tick is an interval later, so a snapshot is due.
                now += Duration::from_secs(60);
                let t = Instant::now();
                app.tick(now);
                autosaves.push(t.elapsed());
            }
            r.samples(
                "autosave",
                "autosave tick (writes the recovery snapshot)",
                &autosaves,
            );
            r.peak("autosave_peak_mb", "autosave");
            alloc::reset_peak();
            let t = Instant::now();
            app.dispatch(Command::Action(ActionId::ToggleEditMode));
            // Leaving with changes asks what to do: discard (item 1).
            if app.is_editing() {
                app.dispatch(Command::Choose(1));
            }
            r.time(
                "edit_leave_ms",
                "leave edit mode (discard, rebuild)",
                t.elapsed(),
            );
            r.peak("edit_leave_peak_mb", "leave edit mode");
        }
        app.shutdown();
        r
    }

    /// Cost of following speech: word positions applied one at a time.
    fn highlight(r: &mut Report, path: &Path, home: &Path) {
        let Ok((speech, log)) = textweaver_app::testing::recording_service() else {
            return;
        };
        let mut app = new_app(home, speech);
        if app.open(path).is_err() {
            return;
        }
        app.dispatch(Command::GoTo(parse_go_to("50%").expect("50%")));
        app.dispatch(Command::Action(ActionId::ReadFromCursor));
        // The recording backend reports every word of the lookahead chunks
        // at once; let them arrive.
        let deadline = Instant::now() + Duration::from_secs(5);
        while log.utterances().is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        std::thread::sleep(Duration::from_millis(50));
        let mut steps = Vec::new();
        for _ in 0..2000 {
            let t = Instant::now();
            match app.poll_speech_step() {
                Some(true) => steps.push(t.elapsed()),
                Some(false) => {}
                None => {
                    if steps.len() >= 200 {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
        }
        r.samples(
            "highlight_step",
            "apply one word position (highlight)",
            &steps,
        );
        app.shutdown();
    }

    /// Runs the benchmarks (the `bench-run` task).
    pub fn run_inner() -> anyhow::Result<()> {
        let args = super::Args::parse(&std::env::args().skip(2).collect::<Vec<_>>())?;
        let super::Args {
            quick,
            files,
            json: json_out,
            only,
            ..
        } = args.clone();
        let dir = super::corpus_dir();
        std::fs::create_dir_all(&dir)?;
        let mut docs: Vec<(String, PathBuf)> = Vec::new();
        if files.is_empty() {
            let root = super::root();
            for f in ["fixtures/sample.md", "README.md", "docs/star-parity.md"] {
                docs.push((f.to_owned(), root.join(f)));
            }
            let mut add = |name: &str, text: String| -> anyhow::Result<()> {
                let p = dir.join(name);
                if std::fs::read_to_string(&p).ok().as_deref() != Some(text.as_str()) {
                    std::fs::write(&p, &text)?;
                }
                docs.push((name.to_owned(), p));
                Ok(())
            };
            add("md-1mb.md", super::markdown_corpus(1 << 20, 1))?;
            add("list-50k-lines.md", super::tight_list_corpus(50_000, 2))?;
            add("one-line-1mb.txt", super::one_line_corpus(1 << 20, 3))?;
            if !quick {
                add("md-10mb.md", super::markdown_corpus(10 << 20, 4))?;
            }
        } else {
            for f in files {
                docs.push((f.display().to_string(), f));
            }
        }
        if let Some(o) = &only {
            docs.retain(|(n, _)| n.contains(o.as_str()));
        }
        let home = dir.join(format!("home-{}", std::process::id()));
        let mut all = serde_json::Map::new();
        println!(
            "textweaver bench ({} {}), {} documents\n",
            std::env::consts::OS,
            std::env::consts::ARCH,
            docs.len()
        );
        for (name, path) in &docs {
            let _ = std::fs::remove_dir_all(&home);
            let rep = bench_doc(name, path, &home);
            println!();
            all.insert(
                rep.name.clone(),
                Value::Object(rep.values.into_iter().collect()),
            );
        }
        let _ = std::fs::remove_dir_all(&home);
        if let Some(tw) = &args.tw {
            let corpus = super::startup_corpus()?;
            all.insert(
                super::STARTUP_KEY.to_owned(),
                super::startup_timings(tw, &corpus, super::STARTUP_RUNS)?,
            );
        }
        let all = Value::Object(all);
        if let Some(p) = json_out {
            std::fs::write(&p, serde_json::to_string_pretty(&all)?)?;
            println!("wrote {}", p.display());
        }
        if let Some(base) = &args.baseline {
            super::gate(&all, base, args.max_ratio)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpora_have_the_requested_shape() {
        let md = markdown_corpus(20_000, 1);
        assert!(md.len() >= 20_000);
        assert!(md.starts_with("---\n"));
        assert!(md.contains("\n## Section 1\n"));
        assert_eq!(md, markdown_corpus(20_000, 1), "deterministic");
        let list = tight_list_corpus(100, 2);
        assert_eq!(list.lines().count(), 102);
        assert!(!list.lines().skip(2).any(str::is_empty));
        let one = one_line_corpus(5_000, 3);
        assert_eq!(one.lines().count(), 1);
    }

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| (*x).to_owned()).collect()
    }

    #[test]
    fn options_parse() {
        let a = Args::parse(&s(&[
            "--quick",
            "--json",
            "b.json",
            "--baseline",
            "main.json",
            "--max-ratio",
            "2.5",
            "--no-startup",
        ]))
        .unwrap();
        assert!(a.quick && !a.startup);
        assert_eq!(a.json, Some(PathBuf::from("b.json")));
        assert_eq!(a.baseline, Some(PathBuf::from("main.json")));
        assert!((a.max_ratio - 2.5).abs() < f64::EPSILON);
        assert_eq!(Args::parse(&[]).unwrap(), Args::default());
        assert!(Args::parse(&s(&["--max-ratio", "0.5"])).is_err());
        assert!(Args::parse(&s(&["--max-ratio", "lots"])).is_err());
        assert!(Args::parse(&s(&["--baseline"])).is_err());
        assert!(Args::parse(&s(&["--bogus"])).is_err());
    }

    #[test]
    fn the_gate_fails_on_memory_growth_only() {
        let base = json!({
            "md-1mb.md": {
                "load_ms": 10.0,
                "load_peak_mb": 20.0,
                "load_allocs": 100000,
                "plan_all_peak_mb": 0.2,
                "nav_idle_word_dispatch": {"mean_ms": 0.1, "p50_ms": 0.1, "max_ms": 0.2, "n": 20},
            },
            "gone.md": {"load_peak_mb": 5.0},
        });
        // Twice as slow, a little more memory: passes.
        let ok = json!({
            "md-1mb.md": {
                "load_ms": 25.0,
                "load_peak_mb": 30.0,
                "load_allocs": 160000,
                "plan_all_peak_mb": 0.9,
                "nav_idle_word_dispatch": {"mean_ms": 0.5, "p50_ms": 0.5, "max_ms": 0.9, "n": 20},
            },
            "new.md": {"load_peak_mb": 50.0},
        });
        let c = compare(&ok, &base, 2.0);
        assert!(c.failures.is_empty(), "{:?}", c.failures);
        assert!(c.lines.iter().any(|l| l.starts_with("new.md: no baseline")));
        assert!(
            c.lines
                .iter()
                .any(|l| l.contains("2 memory numbers checked") && l.contains("load_allocs")),
            "{:?}",
            c.lines
        );
        assert!(c.lines.iter().any(|l| l.contains("nav_idle_word_dispatch")));

        // Allocation count more than doubled: fails, and says which.
        let bad = json!({"md-1mb.md": {"load_peak_mb": 20.0, "load_allocs": 250000}});
        let c = compare(&bad, &base, 2.0);
        assert_eq!(c.failures.len(), 1, "{:?}", c.failures);
        assert!(c.failures[0].contains("load_allocs"));
        // A higher limit lets it through.
        assert!(compare(&bad, &base, 3.0).failures.is_empty());
        // Below the floor nothing is gated, however large the ratio.
        let tiny = json!({"md-1mb.md": {"plan_all_peak_mb": 0.9}});
        assert!(compare(&tiny, &base, 2.0).failures.is_empty());
    }

    #[test]
    fn startup_summaries_take_the_median() {
        let v = summary(&[
            Duration::from_millis(30),
            Duration::from_millis(10),
            Duration::from_millis(20),
        ]);
        assert_eq!(v["median_ms"].as_f64(), Some(20.0));
        assert_eq!(v["min_ms"].as_f64(), Some(10.0));
        assert_eq!(v["max_ms"].as_f64(), Some(30.0));
        assert_eq!(number(&v), Some(20.0));
    }
}
