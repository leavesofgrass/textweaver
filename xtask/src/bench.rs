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
//! - `--baseline FILE`: compare with a baseline. With the committed
//!   baseline file, `xtask/bench-baseline.json`, this is the two-way
//!   ratchet the CI gate runs (`ratchet.rs`): memory may move 25 percent
//!   either way, times may grow 50 percent where the entry was measured on
//!   the gate's own machine type. With an earlier `--json` report it is an
//!   A/B comparison that fails when a memory number grew more than
//!   `--max-ratio R` times (default 2), for trying a change locally.
//!   Numbers too small to matter (under 1 MB of peak heap, under 5,000
//!   allocations) are not gated, and neither is growth too small to matter
//!   (less than 1 MB, or fewer than 5,000 allocations), however large its
//!   ratio. Allocation counts are the measuring thread's own; peak heap is
//!   the whole process's.
//! - `--update-baseline --reason TEXT`: write this run into this
//!   platform's entry of `xtask/bench-baseline.json` (on `main`, or on any
//!   branch for a platform with no entry yet). `--gate-times` marks the
//!   entry's times as gated: only for a run on the CI runner type.
//! - `--no-startup`: skip the startup timings.
//! - `--no-pathological`: skip the pathological inputs.
//! - `--ceiling-s S`: the time ceiling per pathological input (default
//!   10 seconds).
//! - `--engine ID` (repeatable): also time stop-to-first-audio with a real
//!   engine, playing to a silent output: `piper` (needs
//!   `TEXTWEAVER_PIPER_VOICES` naming a folder of installed voices; never
//!   downloads) or `sapi` (Windows). With `TEXTWEAVER_PIPER_VOICES` set,
//!   Piper is timed without asking.
//! - `--profile-plan`: only load each document and plan its narration
//!   three times, for a profiler (the nightly profile job).
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
//! - segmentation: every sentence and every word of the document through
//!   `Units`, on their own (`sentences_ms`, `words_ms`, with peak heap and
//!   allocations).
//! - normalization: the first 2,000 utterances of the plan through the
//!   default pipeline (`normalize_ms`).
//! - edit on the loaded document: one insert in the middle with the
//!   markers kept (`apply_ms`), and the blank-line table rebuilt after it
//!   (`blank_lines_ms`).
//! - stop to speak: Stop, then Read from cursor, until the backend is
//!   handed the first utterance (`stop_to_speak`, 20 times).
//! - stop to first audio: the same on the recording backend playing in
//!   real time, until its first audio starts, as the speech service stamps
//!   it (`stop_to_first_audio`; `stop_to_first_audio_service` is the
//!   speech thread's share). With `--engine`, the same with real engines,
//!   in the `first-audio` entry.
//! - highlight: cost of applying one word position (`poll_speech_step`).
//! - search: a common word, and a regular expression.
//! - edit: entering edit mode, typing one character (per keystroke),
//!   backspace, an autosave snapshot, and leaving edit mode.
//! - state: saving the reading position.
//! - identity (`sync-id-100mb.pdf`, a 100 MB file that starts like a
//!   PDF): finding a document's sync id (ADR-0049) the first time, when the
//!   whole file is hashed, and again, when it is unchanged; its time, the
//!   hashing rate, and peak heap, which stays near the read buffer's size.
//!
//! - pathological inputs (`pathological.rs`): one generated file per
//!   loader with a 512 KB token, lists nested 200 deep, a 2,000-row
//!   table, and a 1 MB line, loaded and planned within a time ceiling.
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
    /// Write this run into the committed baseline file.
    pub update_baseline: bool,
    /// Why the baseline moves (required with `update_baseline`).
    pub reason: Option<String>,
    /// The written entry gates times (a run on the CI runner type).
    pub gate_times: bool,
    /// Run the pathological inputs.
    pub pathological: bool,
    /// Time ceiling per pathological input, in seconds.
    pub ceiling_s: f64,
    /// Real engines to time stop-to-first-audio with.
    pub engines: Vec<String>,
    /// Only load and plan, for a profiler.
    pub profile_plan: bool,
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
            update_baseline: false,
            reason: None,
            gate_times: false,
            pathological: true,
            ceiling_s: crate::pathological::DEFAULT_CEILING_S,
            engines: Vec::new(),
            profile_plan: false,
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
                "--update-baseline" => out.update_baseline = true,
                "--reason" => out.reason = Some(value(&mut it, "--reason")?),
                "--gate-times" => out.gate_times = true,
                "--no-pathological" => out.pathological = false,
                "--ceiling-s" => {
                    let v = value(&mut it, "--ceiling-s")?;
                    out.ceiling_s = v
                        .parse()
                        .ok()
                        .filter(|s: &f64| s.is_finite() && *s > 0.0)
                        .with_context(|| format!("--ceiling-s {v}: give a number of seconds"))?;
                }
                "--engine" => out.engines.push(value(&mut it, "--engine")?),
                "--profile-plan" => out.profile_plan = true,
                other => bail!(
                    "unknown option {other} (cargo xtask bench [--quick] [--file PATH] [--json PATH] [--only NAME] [--baseline FILE] [--max-ratio R] [--update-baseline --reason TEXT [--gate-times]] [--no-startup] [--no-pathological] [--ceiling-s S] [--engine ID] [--profile-plan])"
                ),
            }
        }
        if out.update_baseline && out.reason.as_deref().is_none_or(|r| r.trim().is_empty()) {
            bail!("--update-baseline needs --reason TEXT: why the numbers moved");
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
    if parsed.startup && parsed.tw.is_none() && !parsed.profile_plan {
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

/// Numbers below these, and growth below these, are not gated: too small
/// to matter, and relatively noisy. A fixed cost of a few megabytes added
/// to every document would otherwise fail every tiny one.
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
                    if ratio > max_ratio && now - then >= floor {
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
    if crate::ratchet::is_baseline(&base) {
        return crate::ratchet::gate(report, &base, baseline);
    }
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

/// The start of [`pdf_like_corpus`]: enough for a file to look like a PDF
/// by its first bytes. Hashing it for a sync id reads bytes only.
const PDF_HEADER: &[u8] = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n";

/// `bytes` bytes that start like a PDF and go on as deterministic noise:
/// the size of a large scanned textbook, for the document identity bench
/// (ADR-0049). Not a readable PDF; identity hashes the file's bytes.
pub fn pdf_like_corpus(bytes: usize, seed: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes);
    out.extend_from_slice(&PDF_HEADER[..PDF_HEADER.len().min(bytes)]);
    let mut r = Lcg(seed);
    while out.len() < bytes {
        let v = r.next().to_le_bytes();
        let n = (bytes - out.len()).min(4);
        out.extend_from_slice(&v[..n]);
    }
    out
}

/// Writes [`pdf_like_corpus`] to `path` unless a file of that size with
/// that start is there already (writing 100 MB on every run is slow).
pub(crate) fn write_pdf_like_corpus(path: &Path, bytes: usize, seed: u64) -> anyhow::Result<()> {
    let same = std::fs::metadata(path).is_ok_and(|m| m.len() == bytes as u64)
        && std::fs::File::open(path).is_ok_and(|mut f| {
            use std::io::Read as _;
            let mut head = vec![0u8; 4096.min(bytes)];
            f.read_exact(&mut head).is_ok() && head == pdf_like_corpus(head.len(), seed)
        });
    if !same {
        std::fs::write(path, pdf_like_corpus(bytes, seed))?;
    }
    Ok(())
}

#[cfg(feature = "bench")]
pub use inner::run_inner;

#[cfg(feature = "bench")]
#[allow(unsafe_code)]
pub mod alloc {
    //! A counting global allocator: current and peak heap bytes, for the
    //! whole process, and allocation calls, for the calling thread only.
    //! Threads left over from earlier work (a previous document's readers
    //! and parsers) would otherwise add their allocations to whatever is
    //! being measured, so the counts drifted from run to run.

    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;
    use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

    /// Wraps the system allocator and counts live bytes.
    pub struct Counting;

    static CURRENT: AtomicUsize = AtomicUsize::new(0);
    static PEAK: AtomicUsize = AtomicUsize::new(0);
    thread_local! {
        /// This thread's allocation calls (alloc, alloc_zeroed, realloc)
        /// since the start. A const `Cell` never allocates, so it is safe
        /// to touch from inside the allocator.
        static ALLOCS: Cell<usize> = const { Cell::new(0) };
        /// [`ALLOCS`] at this thread's last [`reset_peak`].
        static ALLOCS_AT_RESET: Cell<usize> = const { Cell::new(0) };
    }

    /// Allocation calls on every thread, for [`wait_quiet`] only.
    static ANY_ALLOCS: AtomicUsize = AtomicUsize::new(0);

    fn counted() {
        ANY_ALLOCS.fetch_add(1, Relaxed);
        // `try_with`: a thread being torn down may still free and allocate.
        let _ = ALLOCS.try_with(|n| n.set(n.get() + 1));
    }

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
                counted();
            }
            p
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            // SAFETY: forwarded unchanged.
            let p = unsafe { System.alloc_zeroed(layout) };
            if !p.is_null() {
                grew(layout.size());
                counted();
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
                counted();
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
        let now = ALLOCS.with(Cell::get);
        ALLOCS_AT_RESET.with(|r| r.set(now));
    }

    /// The highest live heap since the last [`reset_peak`].
    pub fn peak() -> usize {
        PEAK.load(Relaxed)
    }

    /// The calling thread's allocation calls since its last
    /// [`reset_peak`].
    pub fn allocs() -> usize {
        ALLOCS
            .with(Cell::get)
            .saturating_sub(ALLOCS_AT_RESET.with(Cell::get))
    }

    /// Waits until no thread has allocated for `quiet`, or `limit` has
    /// passed; true when it went quiet. Peak heap counts every thread, so
    /// work a previous document left running (its speech and structure
    /// threads) would otherwise be counted against the next one.
    pub fn wait_quiet(quiet: std::time::Duration, limit: std::time::Duration) -> bool {
        let start = std::time::Instant::now();
        let mut seen = ANY_ALLOCS.load(Relaxed);
        let mut since = std::time::Instant::now();
        while start.elapsed() < limit {
            std::thread::sleep(std::time::Duration::from_millis(10));
            let now = ANY_ALLOCS.load(Relaxed);
            if now != seen {
                seen = now;
                since = std::time::Instant::now();
            } else if since.elapsed() >= quiet {
                return true;
            }
        }
        false
    }
}

#[cfg(feature = "bench")]
mod inner {
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use serde_json::{Value, json};
    use textweaver_app::a11y::LogAnnouncer;
    use textweaver_app::core::{
        CharPos, Direction, Edit, PunctuationLevel, Unit, Utterance, UtteranceKind,
    };
    use textweaver_app::formats::{LoadOptions, Registry, Source};
    use textweaver_app::keymap::{ActionId, Frontend, Keymap, Platform};
    use textweaver_app::speech::backend::BackendFactory;
    use textweaver_app::speech::{
        Caps, EventSink, FirstAudio, NormalizeConfig, Pipeline, RawEvent, RecordingBackend,
        RecordingMode, ServiceConfig, SpeechBackend, SpeechError, SpeechService, Voice,
        VoiceParams,
    };
    use textweaver_app::store::{Paths, Settings};
    use textweaver_app::text::Document;
    use textweaver_app::text::narrate::NarrationPolicy;
    use textweaver_app::text::units::Units;
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
        let sample: Vec<Utterance> = plan.iter().take(NORMALIZE_SAMPLE).cloned().collect();
        drop(plan);
        segmentation(&mut r, &doc);
        normalization(&mut r, sample);
        edit_in_place(&mut r, doc);

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
        // Opening starts background work (the structure parse of a large
        // document); it finishes before the steady-state numbers below, or
        // its allocations land in whichever step it overlaps (one step's
        // count varied 30 times between two runs of the same code).
        alloc::wait_quiet(Duration::from_millis(300), Duration::from_secs(30));

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
        // Stop to speak: Stop, then Read from cursor, until the backend is
        // handed the first utterance (the service's share of a restart).
        app.dispatch(Command::GoTo(middle));
        let mut restarts = Vec::new();
        for _ in 0..20 {
            app.dispatch(Command::Action(ActionId::Stop));
            app.poll_speech();
            let (_, spoke) = keystroke(
                &mut app,
                &clock,
                Command::Action(ActionId::ReadFromCursor),
                true,
            );
            restarts.extend(spoke);
        }
        r.samples(
            "stop_to_speak",
            "stop, then read from cursor, to speech",
            &restarts,
        );
        app.dispatch(Command::Action(ActionId::Stop));
        app.poll_speech();

        // Stop to first audio, on a backend that plays in real time.
        first_audio(&mut r, path, home);

        // Highlight updates: one word position applied at a time.
        highlight(&mut r, path, home);

        // Search. Background work (the structure parse, the writer) is
        // finished first: the peak heap counts every thread.
        app.wait_for_background(Duration::from_secs(60));
        app.wait_for_writes();
        alloc::wait_quiet(Duration::from_millis(300), Duration::from_secs(30));
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
                .map_or(0, |f| f.total);
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
            // Edit-mode Replace: the prompt's count, then replacing every
            // match (one undo step), then undoing it.
            app.dispatch(Command::Action(ActionId::Replace));
            alloc::reset_peak();
            let t = Instant::now();
            app.dispatch(Command::Answer("the".into()));
            r.time("replace_count_ms", "replace: count \"the\"", t.elapsed());
            let t = Instant::now();
            app.dispatch(Command::Answer("THE".into()));
            r.time(
                "replace_first_ms",
                "replace: the first match asked about",
                t.elapsed(),
            );
            let t = Instant::now();
            app.dispatch(Command::Choose(1));
            r.time(
                "replace_skip_ms",
                "replace: skip to the next match",
                t.elapsed(),
            );
            // "Replace all the rest" (one undo step).
            let t = Instant::now();
            app.dispatch(Command::Choose(2));
            r.time(
                "replace_all_ms",
                "replace all the rest of \"the\" (one undo step)",
                t.elapsed(),
            );
            r.peak("replace_peak_mb", "replace");
            app.dispatch(Command::Action(ActionId::Undo));
            alloc::reset_peak();
            let mut autosaves = Vec::new();
            let mut written = Vec::new();
            let mut now = Instant::now();
            for _ in 0..5 {
                app.dispatch(Command::Insert("x".into()));
                // Each tick is an interval later, so a snapshot is due.
                now += Duration::from_secs(60);
                let t = Instant::now();
                app.tick(now);
                autosaves.push(t.elapsed());
                // The snapshot is written on the writer thread; wait for
                // it outside the measured tick.
                app.wait_for_writes();
                written.push(t.elapsed());
            }
            r.samples(
                "autosave",
                "autosave tick on the input thread (queues the recovery snapshot)",
                &autosaves,
            );
            r.samples(
                "autosave_written",
                "autosave, snapshot written by the writer thread",
                &written,
            );
            r.peak("autosave_peak_mb", "autosave");
            alloc::wait_quiet(Duration::from_millis(300), Duration::from_secs(30));
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
        // Saving (Ctrl+S) a copy under the bench home, so the corpus and
        // the fixtures are never written.
        let copy = home.join(path.file_name().unwrap_or_default());
        if std::fs::copy(path, &copy).is_ok() && app.open(&copy).is_ok() {
            app.dispatch(Command::Action(ActionId::ToggleEditMode));
            let mut dispatch = Vec::new();
            let mut written = Vec::new();
            for _ in 0..5 {
                app.dispatch(Command::Insert("x".into()));
                let t = Instant::now();
                app.dispatch(Command::Action(ActionId::Save));
                dispatch.push(t.elapsed());
                app.wait_for_writes();
                written.push(t.elapsed());
            }
            r.samples(
                "save_dispatch",
                "save (Ctrl+S) on the input thread",
                &dispatch,
            );
            r.samples(
                "save_written",
                "save (Ctrl+S), file written by the writer thread",
                &written,
            );
        }
        let t = Instant::now();
        app.shutdown();
        r.time("shutdown_ms", "quit (waits for the writer)", t.elapsed());
        r
    }

    /// Document identity (ADR-0049) on a large file, as the writer does it
    /// when a document opens: the first time, when the whole file is
    /// hashed through a small buffer, and again, when the file is
    /// unchanged and is not hashed. Peak heap stays near the buffer's size
    /// whatever the file's.
    fn bench_identity(name: &str, path: &Path, home: &Path) -> Report {
        let mut r = Report {
            name: name.to_owned(),
            values: Vec::new(),
        };
        let bytes = std::fs::metadata(path).map_or(0, |m| m.len());
        println!("{name} ({:.2} MB, document identity)", mb(bytes as usize));
        r.values.push(("bytes".into(), json!(bytes)));
        let job = textweaver_sync::Identify {
            ids_file: Paths::under(home).sync_ids_file(),
            path: path.to_owned(),
            library_folders: Vec::new(),
            details: textweaver_sync::docid::Details::default(),
        };
        for (key, label) in [
            ("identify_first", "identify, whole file hashed"),
            ("identify_again", "identify again, file unchanged"),
        ] {
            alloc::reset_peak();
            let t = Instant::now();
            if let Err(e) = job.run(None::<[&str; 0]>, None) {
                println!("  cannot identify: {e}");
                return r;
            }
            let d = t.elapsed();
            r.time(&format!("{key}_ms"), label, d);
            r.peak(&format!("{key}_peak_mb"), label);
            if key == "identify_first" && d.as_secs_f64() > 0.0 {
                let rate = mb(bytes as usize) / d.as_secs_f64();
                r.put(
                    "identify_mb_per_s",
                    json!(rate),
                    format!("hashing rate: {rate:.0} MB per second"),
                );
            }
        }
        r
    }

    /// Utterances of the plan normalized for `normalize_ms`.
    const NORMALIZE_SAMPLE: usize = 2_000;

    /// Every sentence, then every word, through `Units`: what the plan
    /// and navigation pay for segmentation, on their own.
    fn segmentation(r: &mut Report, doc: &Document) {
        for (unit, name, label) in [
            (Unit::Sentence, "sentences", "every sentence"),
            (Unit::Word, "words", "every word"),
        ] {
            alloc::reset_peak();
            let t = Instant::now();
            let n = Units::new(doc, unit, CharPos(0), Direction::Forward).count();
            let d = t.elapsed();
            r.time(
                &format!("{name}_ms"),
                &format!("{label} through Units ({n})"),
                d,
            );
            r.peak(&format!("{name}_peak_mb"), label);
        }
    }

    /// The first utterances of the plan through the default normalization
    /// pipeline, as the speech thread normalizes them before speaking.
    fn normalization(r: &mut Report, sample: Vec<Utterance>) {
        let t = Instant::now();
        let pipeline = Pipeline::for_settings(
            &NormalizeConfig::default(),
            PunctuationLevel::default(),
            false,
            false,
        );
        r.time(
            "normalize_pipeline_ms",
            "build the normalization pipeline",
            t.elapsed(),
        );
        let n = sample.len();
        alloc::reset_peak();
        let t = Instant::now();
        let mut bytes = 0;
        for u in sample {
            bytes += pipeline.apply(u).text.len();
        }
        let d = t.elapsed();
        std::hint::black_box(bytes);
        r.time(
            "normalize_ms",
            &format!("normalize the first {n} utterances"),
            d,
        );
        r.peak("normalize_peak_mb", "normalize");
    }

    /// One insert in the middle of the loaded document with its markers
    /// kept, then the blank-line table rebuilt, as reading on after an
    /// edit does.
    fn edit_in_place(r: &mut Report, mut doc: Document) {
        let _ = doc.blank_lines();
        let middle = CharPos(doc.len_chars() / 2);
        alloc::reset_peak();
        let t = Instant::now();
        let applied = doc.apply(&Edit::insert(middle, "x"));
        let d = t.elapsed();
        if let Err(e) = applied {
            println!("  edit: the insert failed: {e}");
            return;
        }
        r.time(
            "apply_ms",
            "edit: one insert in the middle, markers kept",
            d,
        );
        r.peak("apply_peak_mb", "edit");
        let t = Instant::now();
        let blanks = doc.blank_lines().len();
        r.time(
            "blank_lines_ms",
            &format!("edit: the blank-line table rebuilt ({blanks} blank lines)"),
            t.elapsed(),
        );
    }

    /// Reads from the middle, then `n` times: Stop, Read from cursor, and
    /// the time to the reading's first audio as the speech service stamps
    /// it. The first restart warms up (a voice's first run is slower) and
    /// is not counted. Returns the key-to-audio times and the speech
    /// thread's share of each.
    fn restarts(
        app: &mut App,
        probe: &FirstAudio,
        n: usize,
        limit: Duration,
    ) -> (Vec<Duration>, Vec<Duration>) {
        app.dispatch(Command::GoTo(parse_go_to("50%").expect("50%")));
        let (mut key, mut service) = (Vec::new(), Vec::new());
        for i in 0..=n {
            app.dispatch(Command::Action(ActionId::Stop));
            app.poll_speech();
            let last = probe.latest().map_or(0, |s| s.generation);
            let t0 = Instant::now();
            app.dispatch(Command::Action(ActionId::ReadFromCursor));
            let Some(stamp) = probe.wait_for(last + 1, limit) else {
                println!("  no audio within {} s", limit.as_secs());
                break;
            };
            if i > 0 {
                key.push(stamp.started.saturating_duration_since(t0));
                service.push(stamp.latency());
            }
            // A moment of listening before the next key.
            std::thread::sleep(Duration::from_millis(30));
            app.poll_speech();
        }
        app.dispatch(Command::Action(ActionId::Stop));
        app.poll_speech();
        (key, service)
    }

    /// Stop to first audio on the recording backend playing in real time
    /// (an audio-clock engine whose words arrive as their audio starts).
    fn first_audio(r: &mut Report, path: &Path, home: &Path) {
        let (backend, _handle) = RecordingBackend::with(
            RecordingMode::Timed { ms_per_word: 300 },
            RecordingBackend::DEFAULT_CAPS,
        );
        let Ok(speech) = SpeechService::spawn(backend.into_factory(), ServiceConfig::default())
        else {
            return;
        };
        let probe = speech.first_audio();
        let mut app = new_app(home, speech);
        if app.open(path).is_err() {
            return;
        }
        let (key, service) = restarts(&mut app, &probe, 20, Duration::from_secs(5));
        r.samples(
            "stop_to_first_audio",
            "stop, then read from cursor, to first audio (recording backend)",
            &key,
        );
        r.samples(
            "stop_to_first_audio_service",
            "the speech thread's share of it",
            &service,
        );
        app.shutdown();
    }

    /// The backend factory for a real engine playing to a silent output
    /// that takes samples in real time, or why it cannot run here.
    fn engine_factory(id: &str) -> Result<BackendFactory, String> {
        use textweaver_enginehost::AudioOutput;
        let silent = AudioOutput::Null { speed: 1.0 };
        match id {
            "piper" => {
                let Some(dir) =
                    std::env::var_os("TEXTWEAVER_PIPER_VOICES").filter(|v| !v.is_empty())
                else {
                    return Err(
                        "TEXTWEAVER_PIPER_VOICES does not name a folder of installed voices (nothing is downloaded)"
                            .into(),
                    );
                };
                let voices = PathBuf::from(dir);
                let mut config = textweaver_app::piper_config(&Settings::default());
                if let Some(data) = voices.parent() {
                    config.espeak_data = data.join("espeak-ng-data");
                }
                config.voices_dir = voices;
                config.output = silent;
                if !textweaver_piper::probe(&config) {
                    return Err(format!(
                        "no usable Piper voice in {}",
                        config.voices_dir.display()
                    ));
                }
                Ok(Box::new(textweaver_piper::factory(config)))
            }
            #[cfg(windows)]
            "sapi" => {
                let mut config = textweaver_app::sapi_config(&Settings::default());
                config.output = silent;
                if !textweaver_sapi::probe(&config) {
                    return Err(
                        "SAPI 5 has no engine host here (cargo xtask sapi-host --release)".into(),
                    );
                }
                Ok(textweaver_sapi::factory(config))
            }
            #[cfg(not(windows))]
            "sapi" => Err("SAPI 5 is Windows only".into()),
            other => Err(format!("{other}: only piper and sapi are timed here")),
        }
    }

    /// Stop to first audio with each real engine in `ids`, on `path`. An
    /// engine that cannot run here is reported and skipped.
    fn engines_first_audio(ids: &[String], path: &Path, home: &Path) -> Report {
        let mut r = Report {
            name: "first-audio".into(),
            values: Vec::new(),
        };
        println!(
            "first audio with real engines, silent output ({})",
            path.display()
        );
        for id in ids {
            let factory = match engine_factory(id) {
                Ok(f) => f,
                Err(why) => {
                    println!("  {id}: skipped, {why}");
                    continue;
                }
            };
            let speech = match SpeechService::spawn(factory, ServiceConfig::default()) {
                Ok(s) => s,
                Err(e) => {
                    println!("  {id}: skipped, it did not start: {e}");
                    continue;
                }
            };
            let probe = speech.first_audio();
            let mut app = new_app(&home.join(id), speech);
            if let Err(e) = app.open(path) {
                println!("  {id}: cannot open {}: {e}", path.display());
                continue;
            }
            let (key, service) = restarts(&mut app, &probe, 20, Duration::from_secs(10));
            r.samples(
                &format!("{id}_stop_to_first_audio"),
                &format!("{id}: stop, then read from cursor, to first audio"),
                &key,
            );
            r.samples(
                &format!("{id}_stop_to_first_audio_service"),
                &format!("{id}: the speech thread's share of it"),
                &service,
            );
            app.shutdown();
        }
        r
    }

    /// Loads and plans each pathological input (`pathological.rs`).
    /// Returns the report and one sentence per input over the ceiling or
    /// that panicked.
    fn pathological(dir: &Path, ceiling: Duration) -> anyhow::Result<(Report, Vec<String>)> {
        let dir = dir.join("pathological");
        std::fs::create_dir_all(&dir)?;
        let mut r = Report {
            name: "pathological".into(),
            values: Vec::new(),
        };
        let mut over = Vec::new();
        let registry = Registry::with_builtins();
        println!(
            "pathological inputs, a ceiling of {:.0} seconds each ({})",
            ceiling.as_secs_f64(),
            dir.display()
        );
        for input in crate::pathological::inputs() {
            let p = dir.join(input.name);
            if std::fs::read(&p).ok().as_deref() != Some(input.bytes.as_slice()) {
                std::fs::write(&p, &input.bytes)?;
            }
            let key = input.loader;
            let t = Instant::now();
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match registry
                .load(&Source::Path(p.clone()), &LoadOptions::default())
            {
                Ok(doc) => {
                    let loaded = t.elapsed();
                    let plan = textweaver_app::text::plan(
                        &doc,
                        doc.full_range(),
                        &NarrationPolicy::default(),
                    );
                    Ok((loaded, doc.len_chars(), plan.len()))
                }
                Err(e) => Err(e.to_string()),
            }));
            let total = t.elapsed();
            r.values.push((format!("{key}_total_ms"), json!(ms(total))));
            match outcome {
                Ok(Ok((loaded, chars, utterances))) => {
                    r.values.push((format!("{key}_load_ms"), json!(ms(loaded))));
                    println!(
                        "  {key} ({}): {:.0} ms in all, loaded in {:.0} ms; {chars} chars, {utterances} utterances",
                        input.name,
                        ms(total),
                        ms(loaded)
                    );
                }
                Ok(Err(e)) => println!(
                    "  {key} ({}): refused in {:.0} ms: {e}",
                    input.name,
                    ms(total)
                ),
                Err(_) => {
                    println!("  {key} ({}): the loader panicked", input.name);
                    over.push(format!("{key} ({}): the loader panicked", input.name));
                }
            }
            if total > ceiling {
                over.push(format!(
                    "{key} ({}): took {:.1} seconds, over the ceiling of {:.0}",
                    input.name,
                    total.as_secs_f64(),
                    ceiling.as_secs_f64()
                ));
            }
        }
        println!();
        Ok((r, over))
    }

    /// `--profile-plan`: loads each document and plans it three times,
    /// and nothing else, so a profiler sees loading and planning.
    fn profile_plan(docs: &[(String, PathBuf)]) {
        let registry = Registry::with_builtins();
        for (name, path) in docs {
            let t = Instant::now();
            let doc = match registry.load(&Source::Path(path.clone()), &LoadOptions::default()) {
                Ok(d) => d,
                Err(e) => {
                    println!("{name}: cannot load: {e}");
                    continue;
                }
            };
            println!("{name}: loaded in {:.0} ms", ms(t.elapsed()));
            for i in 1..=3 {
                let t = Instant::now();
                let plan =
                    textweaver_app::text::plan(&doc, doc.full_range(), &NarrationPolicy::default());
                println!(
                    "{name}: plan {i} of 3, {} utterances, {:.0} ms",
                    plan.len(),
                    ms(t.elapsed())
                );
            }
        }
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
            for f in ["fixtures/sample.md", "README.md", "docs/reading.md"] {
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
        if args.profile_plan {
            profile_plan(&docs);
            return Ok(());
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
            // The previous document's threads finish before this one is
            // measured (under load they ran on for seconds and multiplied
            // some allocation counts by up to seven).
            if !alloc::wait_quiet(Duration::from_millis(300), Duration::from_secs(30)) {
                println!("(other threads were still allocating after 30 s)");
            }
            let _ = std::fs::remove_dir_all(&home);
            let rep = bench_doc(name, path, &home);
            println!();
            all.insert(
                rep.name.clone(),
                Value::Object(rep.values.into_iter().collect()),
            );
        }
        // Document identity on a 100 MB file (ADR-0049), with the corpora.
        let identity = "sync-id-100mb.pdf";
        if args.files.is_empty() && only.as_ref().is_none_or(|o| identity.contains(o.as_str())) {
            let p = dir.join(identity);
            super::write_pdf_like_corpus(&p, 100 << 20, 5)?;
            if !alloc::wait_quiet(Duration::from_millis(300), Duration::from_secs(30)) {
                println!("(other threads were still allocating after 30 s)");
            }
            let _ = std::fs::remove_dir_all(&home);
            let rep = bench_identity(identity, &p, &home);
            println!();
            all.insert(
                rep.name.clone(),
                Value::Object(rep.values.into_iter().collect()),
            );
        }
        // Pathological inputs, one per loader, each within a ceiling.
        let mut over = Vec::new();
        if args.pathological
            && args.files.is_empty()
            && only
                .as_ref()
                .is_none_or(|o| "pathological".contains(o.as_str()))
        {
            let (rep, o) = pathological(&dir, Duration::from_secs_f64(args.ceiling_s))?;
            over = o;
            all.insert(
                rep.name.clone(),
                Value::Object(rep.values.into_iter().collect()),
            );
        }
        // Stop to first audio with real engines (Piper whenever a voices
        // folder is named), on the 1 MB corpus.
        let mut engines = args.engines.clone();
        if std::env::var_os("TEXTWEAVER_PIPER_VOICES").is_some_and(|v| !v.is_empty())
            && !engines.iter().any(|e| e == "piper")
        {
            engines.push("piper".into());
        }
        if !engines.is_empty() {
            let corpus = super::startup_corpus()?;
            let rep = engines_first_audio(&engines, &corpus, &home);
            println!();
            if !rep.values.is_empty() {
                all.insert(
                    rep.name.clone(),
                    Value::Object(rep.values.into_iter().collect()),
                );
            }
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
        for o in &over {
            println!("Pathological input out of bounds: {o}");
        }
        if args.update_baseline {
            if !over.is_empty() {
                anyhow::bail!(
                    "the baseline is not written: {} pathological inputs are out of bounds",
                    over.len()
                );
            }
            let root = super::root();
            crate::ratchet::update(
                &all,
                &crate::ratchet::Update {
                    path: &root.join(crate::ratchet::BASELINE_FILE),
                    root: &root,
                    reason: args.reason.as_deref(),
                    quick,
                    gate_times: args.gate_times,
                },
            )?;
        } else if let Some(base) = &args.baseline {
            super::gate(&all, base, args.max_ratio)?;
        }
        if !over.is_empty() {
            anyhow::bail!(
                "{} pathological inputs are out of bounds (a panic, or over the ceiling)",
                over.len()
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pdf_like_corpus_is_sized_and_stable() {
        let a = pdf_like_corpus(10_000, 5);
        assert_eq!(a.len(), 10_000);
        assert!(a.starts_with(b"%PDF-1.7"));
        assert_eq!(a, pdf_like_corpus(10_000, 5));
        assert_ne!(a, pdf_like_corpus(10_000, 6));
        assert_eq!(pdf_like_corpus(3, 5), b"%PD");
    }

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
        // Growth below the floor is not gated either: 0.2 MB to 1.1 MB is
        // over the ratio, but only 0.9 MB more.
        let small = json!({"md-1mb.md": {"plan_all_peak_mb": 1.1}});
        assert!(compare(&small, &base, 2.0).failures.is_empty());
        // Growth over the floor is: 0.2 MB to 4.2 MB.
        let fixed = json!({"md-1mb.md": {"plan_all_peak_mb": 4.2}});
        assert_eq!(compare(&fixed, &base, 2.0).failures.len(), 1);
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
