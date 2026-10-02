//! `cargo xtask frames`: the GUI frame-time probe (W8b-i), the gate for
//! Wave 9's visual work.
//!
//! It builds the Xilem GUI in release mode, writes the 1 MB benchmark
//! corpus, and runs `textweaver-xilem --measure-frames SECONDS`: the
//! document read aloud on the silent paced backend with no window on
//! screen, timing each frame in which the spoken word moved (the widget
//! tree brought up to date, then layout, paint, and the accessibility
//! tree; see `crates/textweaver-xilem/src/frames.rs`). It prints the
//! median, 95th percentile, and worst frame.
//!
//! Options: `--seconds N` (default 30); `--file PATH` instead of the
//! corpus; `--json PATH` to keep the report; `--bin PATH` to use a GUI
//! already built; `--max-median MS` and `--max-worst MS` to fail above a
//! ceiling (for CI, once the spread is known).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};
use serde_json::Value;

/// Parsed options.
#[derive(Debug, PartialEq)]
pub(crate) struct Args {
    pub seconds: f64,
    pub file: Option<PathBuf>,
    pub json: Option<PathBuf>,
    pub bin: Option<PathBuf>,
    pub max_median: Option<f64>,
    pub max_worst: Option<f64>,
}

impl Default for Args {
    fn default() -> Self {
        Args {
            seconds: 30.0,
            file: None,
            json: None,
            bin: None,
            max_median: None,
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
            "--seconds" => a.seconds = positive(arg, &value()?)?,
            "--file" => a.file = Some(value()?.into()),
            "--json" => a.json = Some(value()?.into()),
            "--bin" => a.bin = Some(value()?.into()),
            "--max-median" => a.max_median = Some(positive(arg, &value()?)?),
            "--max-worst" => a.max_worst = Some(positive(arg, &value()?)?),
            other => bail!(
                "unknown option {other} (cargo xtask frames [--seconds N] [--file PATH] [--json PATH] [--bin PATH] [--max-median MS] [--max-worst MS])"
            ),
        }
    }
    Ok(a)
}

/// Builds the GUI in release mode and returns its path.
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

/// The ceilings `report` breaks, one sentence each.
pub(crate) fn over_ceilings(report: &Value, a: &Args) -> Vec<String> {
    let mut out = Vec::new();
    let get = |k: &str| report.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    if report.get("frames").and_then(Value::as_u64).unwrap_or(0) == 0 {
        out.push("No frame was measured: the spoken word never moved.".into());
    }
    if let Some(max) = a.max_median
        && get("frame_median_ms") > max
    {
        out.push(format!(
            "The median frame took {:.2} ms, over the ceiling of {max} ms.",
            get("frame_median_ms")
        ));
    }
    if let Some(max) = a.max_worst
        && get("frame_worst_ms") > max
    {
        out.push(format!(
            "The worst frame took {:.2} ms, over the ceiling of {max} ms.",
            get("frame_worst_ms")
        ));
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
    let file = match &a.file {
        Some(f) => f.clone(),
        None => crate::bench::startup_corpus()?,
    };
    let dir = crate::bench::corpus_dir();
    // Settings and positions go here, never into the user's own folder.
    let home = dir.join("frames-home");
    let out = a.json.clone().unwrap_or_else(|| dir.join("frames.json"));
    println!(
        "Frame times: reading {} for {} seconds, no window on screen.",
        file.display(),
        a.seconds
    );
    let status = Command::new(&bin)
        .arg("--measure-frames")
        .arg(a.seconds.to_string())
        .arg("--home")
        .arg(&home)
        .arg("--frames-json")
        .arg(&out)
        .arg(&file)
        .status()
        .with_context(|| format!("running {}", bin.display()))?;
    if !status.success() {
        bail!("the frame-time probe failed: {status}");
    }
    let report = read(&out)?;
    for (key, label) in [
        ("frames", "Frames measured"),
        ("frame_median_ms", "Median frame, ms"),
        ("frame_p95_ms", "95th percentile, ms"),
        ("frame_worst_ms", "Worst frame, ms"),
        ("refresh_median_ms", "Median refresh, ms"),
        ("passes_median_ms", "Median passes, ms"),
    ] {
        if let Some(v) = report.get(key) {
            println!("{label}: {v}");
        }
    }
    println!("Wrote {}.", out.display());
    let over = over_ceilings(&report, &a);
    for o in &over {
        println!("Out of bounds: {o}");
    }
    if !over.is_empty() {
        bail!("the frame times are out of bounds");
    }
    Ok(())
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
            "--seconds",
            "5",
            "--max-median",
            "4",
            "--json",
            "f.json",
        ]))
        .unwrap();
        assert!((a.seconds - 5.0).abs() < f64::EPSILON);
        assert_eq!(a.max_median, Some(4.0));
        assert_eq!(a.json, Some(PathBuf::from("f.json")));
        assert_eq!(parse(&[]).unwrap(), Args::default());
        assert!(parse(&s(&["--seconds", "0"])).is_err());
        assert!(parse(&s(&["--seconds", "lots"])).is_err());
        assert!(parse(&s(&["--bogus"])).is_err());
    }

    #[test]
    fn ceilings_fail_in_words() {
        let r = json!({"frames": 90, "frame_median_ms": 3.0, "frame_worst_ms": 40.0});
        let mut a = Args::default();
        assert!(over_ceilings(&r, &a).is_empty());
        a.max_median = Some(2.0);
        a.max_worst = Some(50.0);
        let over = over_ceilings(&r, &a);
        assert_eq!(over.len(), 1);
        assert!(over[0].starts_with("The median frame took 3.00 ms"));
        let none = json!({"frames": 0});
        assert_eq!(over_ceilings(&none, &Args::default()).len(), 1);
    }
}
