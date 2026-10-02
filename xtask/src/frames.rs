//! `cargo xtask frames`: the GUI frame-time probe (W8b-i), the gate for
//! Wave 9's visual work (the performance report's section C).
//!
//! It builds the Xilem GUI in release mode with the `alloc-count`
//! feature, writes the 1 MB benchmark corpus and the 1 MB one-line corpus,
//! and runs `textweaver-xilem --measure-frames 200` on each: plain and
//! with every reading aid on, at 100 and 200 percent (eight runs). Each
//! run reads with no window on screen and measures 200 moves of the spoken
//! word through the driver and the whole widget tree, with the sentence
//! band (see `crates/textweaver-xilem/src/frames.rs`). It prints, per run,
//! the median, 95th percentile, and worst move, and the allocations and
//! accessibility nodes per move.
//!
//! The gate: `--baseline xtask/frames-baseline.json` compares the counts
//! (allocations and nodes, which do not move with the machine's load)
//! with the committed baseline through the bench's two-way ratchet
//! (`ratchet.rs`); times are reported. `--update-baseline --reason TEXT`
//! writes this platform's entry. `--max-worst MS` fails any run whose
//! worst move is over a ceiling (ADR-0027's 30 ms is the hard one).
//!
//! Options: `--moves N` (default 200); `--quick` for the 1 MB corpus at
//! 100 percent, plain and with the aids, only; `--file PATH` to measure
//! one document instead of the corpora; `--json PATH` to keep the report;
//! `--bin PATH` to use a GUI already built.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};
use serde_json::{Map, Value};

/// The committed baseline for the frame counts, relative to the root.
pub(crate) const BASELINE_FILE: &str = "xtask/frames-baseline.json";

/// Parsed options.
#[derive(Debug, PartialEq)]
pub(crate) struct Args {
    pub moves: usize,
    pub quick: bool,
    pub file: Option<PathBuf>,
    pub json: Option<PathBuf>,
    pub bin: Option<PathBuf>,
    pub baseline: Option<PathBuf>,
    pub update_baseline: bool,
    pub reason: Option<String>,
    pub max_worst: Option<f64>,
}

impl Default for Args {
    fn default() -> Self {
        Args {
            moves: 200,
            quick: false,
            file: None,
            json: None,
            bin: None,
            baseline: None,
            update_baseline: false,
            reason: None,
            max_worst: None,
        }
    }
}

fn positive(name: &str, v: &str) -> anyhow::Result<f64> {
    v.parse()
        .ok()
        .filter(|n: &f64| n.is_finite() && *n > 0.0)
        .with_context(|| format!("{name} {v}: give a positive number"))
}

pub(crate) fn parse(args: &[String]) -> anyhow::Result<Args> {
    let mut a = Args::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = || {
            it.next()
                .cloned()
                .with_context(|| format!("{arg} needs a value"))
        };
        match arg.as_str() {
            "--moves" => {
                let v = value()?;
                a.moves = v
                    .parse()
                    .ok()
                    .filter(|n: &usize| *n > 0)
                    .with_context(|| format!("--moves {v}: give a whole number"))?;
            }
            "--quick" => a.quick = true,
            "--file" => a.file = Some(value()?.into()),
            "--json" => a.json = Some(value()?.into()),
            "--bin" => a.bin = Some(value()?.into()),
            "--baseline" => a.baseline = Some(value()?.into()),
            "--update-baseline" => a.update_baseline = true,
            "--reason" => a.reason = Some(value()?),
            "--max-worst" => a.max_worst = Some(positive(arg, &value()?)?),
            other => bail!(
                "unknown option {other} (cargo xtask frames [--moves N] [--quick] [--file PATH] [--json PATH] [--bin PATH] [--baseline FILE] [--update-baseline --reason TEXT] [--max-worst MS])"
            ),
        }
    }
    if a.update_baseline && a.reason.as_deref().is_none_or(|r| r.trim().is_empty()) {
        bail!("--update-baseline needs --reason TEXT: why the numbers moved");
    }
    Ok(a)
}

/// One run of the matrix.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Run {
    /// The report's key: "md-1mb.md, aids, 200 percent".
    pub name: String,
    pub file: PathBuf,
    pub aids: bool,
    pub scale: f64,
}

/// The runs: each document, plain and with the aids, at each scale.
pub(crate) fn matrix(files: &[(String, PathBuf)], scales: &[f64]) -> Vec<Run> {
    let mut out = Vec::new();
    for (label, file) in files {
        for aids in [false, true] {
            for &scale in scales {
                out.push(Run {
                    name: format!(
                        "{label}, {}, {:.0} percent",
                        if aids { "aids" } else { "plain" },
                        scale * 100.0
                    ),
                    file: file.clone(),
                    aids,
                    scale,
                });
            }
        }
    }
    out
}

/// Builds the GUI in release mode with allocation counting, and returns
/// its path.
fn build() -> anyhow::Result<PathBuf> {
    let root = crate::bench::root();
    let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .current_dir(&root)
        .args([
            "build",
            "--quiet",
            "--release",
            "-p",
            "textweaver-xilem",
            "--bin",
            "textweaver-xilem",
            "--features",
            "alloc-count",
        ])
        .status()
        .context("running cargo")?;
    if !status.success() {
        bail!("building textweaver-xilem failed: {status}");
    }
    Ok(crate::eci::target_dir(&root)
        .join("release")
        .join(format!("textweaver-xilem{}", std::env::consts::EXE_SUFFIX)))
}

/// The worst moves over `max`, one sentence each.
pub(crate) fn over_ceiling(report: &Value, max: Option<f64>) -> Vec<String> {
    let mut out = Vec::new();
    let Some(runs) = report.as_object() else {
        return out;
    };
    for (name, r) in runs {
        if r.get("moves").and_then(Value::as_u64).unwrap_or(0) == 0 {
            out.push(format!(
                "{name}: no move was measured; the spoken word never moved."
            ));
        }
        let worst = r
            .get("move_worst_ms")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        if let Some(max) = max
            && worst > max
        {
            out.push(format!(
                "{name}: the worst move took {worst:.2} ms, over the ceiling of {max} ms."
            ));
        }
    }
    out
}

/// `cargo xtask frames`.
pub fn run() -> anyhow::Result<()> {
    let a = parse(&std::env::args().skip(2).collect::<Vec<_>>())?;
    let bin = match &a.bin {
        Some(b) => b.clone(),
        None => build()?,
    };
    let dir = crate::bench::corpus_dir();
    let files: Vec<(String, PathBuf)> = match &a.file {
        Some(f) => vec![(f.display().to_string(), f.clone())],
        None => {
            let mut v = vec![("md-1mb.md".to_owned(), crate::bench::startup_corpus()?)];
            if !a.quick {
                let p = dir.join("one-line-1mb.txt");
                let text = crate::bench::one_line_corpus(1 << 20, 3);
                if std::fs::read_to_string(&p).ok().as_deref() != Some(text.as_str()) {
                    std::fs::write(&p, &text)?;
                }
                v.push(("one-line-1mb.txt".to_owned(), p));
            }
            v
        }
    };
    let scales: &[f64] = if a.quick { &[1.0] } else { &[1.0, 2.0] };
    let mut report = Map::new();
    for (i, r) in matrix(&files, scales).into_iter().enumerate() {
        // Settings and positions go here, never into the user's own
        // folder; one per run, so the aids of one never carry to the next.
        let home = dir.join(format!("frames-home-{i}"));
        let out = dir.join(format!("frames-{i}.json"));
        println!("{}: {} moves, no window on screen.", r.name, a.moves);
        let mut cmd = Command::new(&bin);
        cmd.arg("--measure-frames")
            .arg(a.moves.to_string())
            .arg("--scale")
            .arg(r.scale.to_string())
            .arg("--home")
            .arg(&home)
            .arg("--frames-json")
            .arg(&out);
        if r.aids {
            cmd.arg("--frames-aids");
        }
        let status = cmd
            .arg(&r.file)
            .status()
            .with_context(|| format!("running {}", bin.display()))?;
        if !status.success() {
            bail!("the frame-time probe failed on {}: {status}", r.name);
        }
        let v = read(&out)?;
        println!("  {}", summary(&v, a.moves));
        report.insert(r.name, v);
    }
    let report = Value::Object(report);
    let json = a.json.clone().unwrap_or_else(|| dir.join("frames.json"));
    std::fs::write(&json, serde_json::to_string_pretty(&report)? + "\n")?;
    println!("Wrote {}.", json.display());
    let over = over_ceiling(&report, a.max_worst);
    for o in &over {
        println!("Out of bounds: {o}");
    }
    let root = crate::bench::root();
    if a.update_baseline {
        if !over.is_empty() {
            bail!("the baseline is not written: some runs are out of bounds");
        }
        crate::ratchet::update(
            &report,
            &crate::ratchet::Update {
                path: &root.join(BASELINE_FILE),
                root: &root,
                reason: a.reason.as_deref(),
                quick: a.quick,
                gate_times: false,
            },
        )?;
    } else if let Some(base) = &a.baseline {
        crate::bench::gate(&report, base, crate::bench::DEFAULT_MAX_RATIO)?;
    }
    if !over.is_empty() {
        bail!("the frame times are out of bounds");
    }
    Ok(())
}

/// One line per run: the meaning first.
fn summary(v: &Value, moves: usize) -> String {
    let f = |k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    let per = |k: &str| {
        v.get(k)
            .and_then(Value::as_f64)
            .map(|n| format!("{:.1}", n / moves.max(1) as f64))
            .unwrap_or_else(|| "not counted".into())
    };
    format!(
        "median {:.3} ms, 95th percentile {:.3} ms, worst {:.3} ms; allocations per move {}, nodes per move {}.",
        f("move_p50_ms"),
        f("move_p95_ms"),
        f("move_worst_ms"),
        per("moves_allocs"),
        per("moves_nodes")
    )
}

fn read(path: &Path) -> anyhow::Result<Value> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| (*x).to_owned()).collect()
    }

    #[test]
    fn options_parse() {
        let a = parse(&s(&[
            "--moves",
            "50",
            "--max-worst",
            "30",
            "--json",
            "f.json",
        ]))
        .unwrap();
        assert_eq!(a.moves, 50);
        assert_eq!(a.max_worst, Some(30.0));
        assert_eq!(a.json, Some(PathBuf::from("f.json")));
        assert_eq!(parse(&[]).unwrap(), Args::default());
        assert!(parse(&s(&["--moves", "0"])).is_err());
        assert!(parse(&s(&["--update-baseline"])).is_err());
        assert!(parse(&s(&["--bogus"])).is_err());
    }

    #[test]
    fn the_matrix_covers_aids_and_scales() {
        let files = vec![
            ("a.md".to_owned(), PathBuf::from("a.md")),
            ("b.txt".to_owned(), PathBuf::from("b.txt")),
        ];
        let m = matrix(&files, &[1.0, 2.0]);
        assert_eq!(m.len(), 8);
        assert_eq!(m[0].name, "a.md, plain, 100 percent");
        assert_eq!(m[3].name, "a.md, aids, 200 percent");
        assert!(m[3].aids && (m[3].scale - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ceilings_fail_in_words() {
        let r = json!({"a.md, plain, 100 percent": {"moves": 200, "move_worst_ms": 40.0}});
        assert!(over_ceiling(&r, None).is_empty());
        let over = over_ceiling(&r, Some(30.0));
        assert_eq!(over.len(), 1);
        assert!(over[0].starts_with("a.md, plain, 100 percent: the worst move took 40.00 ms"));
        let none = json!({"b": {"moves": 0}});
        assert_eq!(over_ceiling(&none, None).len(), 1);
        assert!(summary(&json!({"moves_nodes": 1000.0}), 200).contains("nodes per move 5.0"));
    }
}
