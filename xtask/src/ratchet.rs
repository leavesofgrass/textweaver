//! The bench gate's two-way ratchet (W8b-i, from abax's coverage ratchet).
//!
//! The baseline is a committed file, `xtask/bench-baseline.json`, with one
//! entry per platform (`linux-x86_64`, `windows-x86_64`, ...). Each entry
//! is a whole `cargo xtask bench --json` report plus why and when it was
//! written. The gate compares a run with its platform's entry:
//!
//! - **Memory** (peak heap, `*_peak_mb`, and allocation counts,
//!   `*_allocs`) fails when it grows more than `max_growth` (25 percent)
//!   over the baseline, and also when it falls more than
//!   `max_improvement` (25 percent) below it: a gain must be written into
//!   the baseline, or the floor goes stale and a later regression hides
//!   under it.
//! - **Times** (`*_ms`, and the median of sampled timings) fail when they
//!   grow more than `max_time_growth` over the baseline, but only for an
//!   entry measured on the machine type the gate runs on
//!   (`gate_times: true`), only above `time_floor_ms`, and only by more
//!   than `time_growth_floor_ms`; times that improve are reported, never
//!   failed, since a quiet runner is not a change in the code.
//!
//! A failed run never becomes the baseline. Only `cargo xtask bench
//! --update-baseline --reason TEXT` writes it, on the `main` branch, or on
//! any branch for a platform the file has no entry for yet.

// Without the feature only the tests use it.
#![cfg_attr(not(feature = "bench"), allow(dead_code))]

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use serde_json::{Map, Value, json};

/// The committed baseline file, relative to the workspace root.
pub(crate) const BASELINE_FILE: &str = "xtask/bench-baseline.json";

/// This machine's key in the baseline file.
pub(crate) fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// The limits, stored in the baseline file so they change with a commit
/// that says why.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Policy {
    /// Memory may grow this fraction over the baseline (0.25: 25 percent).
    pub max_growth: f64,
    /// Memory that falls more than this fraction below the baseline fails
    /// until the baseline is updated.
    pub max_improvement: f64,
    /// Times may grow this fraction over the baseline.
    pub max_time_growth: f64,
    /// Times below this in the baseline are not gated (too noisy).
    pub time_floor_ms: f64,
    /// Time growth smaller than this is not gated, whatever its ratio.
    pub time_growth_floor_ms: f64,
    /// Peak heap below this, in MB, and growth smaller than it, are not
    /// gated.
    pub peak_floor_mb: f64,
    /// Allocation counts below this, and growth smaller than it, are not
    /// gated.
    pub alloc_floor: f64,
    /// Accessibility node counts (`*_nodes`, the frame probe) below this,
    /// and growth smaller than it, are not gated.
    pub node_floor: f64,
    /// Keys containing any of these are reported, never gated (disk
    /// writes, process starts).
    pub report_only: Vec<String>,
}

impl Default for Policy {
    fn default() -> Self {
        Policy {
            max_growth: 0.25,
            max_improvement: 0.25,
            max_time_growth: 0.5,
            time_floor_ms: 5.0,
            time_growth_floor_ms: 2.0,
            peak_floor_mb: 1.0,
            alloc_floor: 5_000.0,
            node_floor: 100.0,
            report_only: vec![
                "startup".into(),
                "shutdown".into(),
                "save_".into(),
                "autosave".into(),
                "identify".into(),
                "pathological".into(),
                // Repository files change with the docs; the corpora do not.
                "README.md".into(),
                "docs/".into(),
            ],
        }
    }
}

impl Policy {
    fn from_json(v: Option<&Value>) -> Policy {
        let d = Policy::default();
        let Some(v) = v else { return d };
        let num = |k: &str, def: f64| v.get(k).and_then(Value::as_f64).unwrap_or(def);
        Policy {
            max_growth: num("max_growth", d.max_growth),
            max_improvement: num("max_improvement", d.max_improvement),
            max_time_growth: num("max_time_growth", d.max_time_growth),
            time_floor_ms: num("time_floor_ms", d.time_floor_ms),
            time_growth_floor_ms: num("time_growth_floor_ms", d.time_growth_floor_ms),
            peak_floor_mb: num("peak_floor_mb", d.peak_floor_mb),
            alloc_floor: num("alloc_floor", d.alloc_floor),
            node_floor: num("node_floor", d.node_floor),
            report_only: v.get("report_only").and_then(Value::as_array).map_or(
                d.report_only,
                |a| {
                    a.iter()
                        .filter_map(|s| s.as_str().map(str::to_owned))
                        .collect()
                },
            ),
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "max_growth": self.max_growth,
            "max_improvement": self.max_improvement,
            "max_time_growth": self.max_time_growth,
            "time_floor_ms": self.time_floor_ms,
            "time_growth_floor_ms": self.time_growth_floor_ms,
            "peak_floor_mb": self.peak_floor_mb,
            "alloc_floor": self.alloc_floor,
            "node_floor": self.node_floor,
            "report_only": self.report_only,
        })
    }
}

/// True when `v` is a baseline file rather than a plain report.
pub(crate) fn is_baseline(v: &Value) -> bool {
    v.get("platforms").is_some_and(Value::is_object)
}

/// What the ratchet found.
#[derive(Debug, Default)]
pub(crate) struct Verdict {
    /// Plain-sentence lines for the log and the job summary.
    pub lines: Vec<String>,
    /// Gated numbers out of bounds, one sentence each.
    pub failures: Vec<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Peak,
    Allocs,
    Nodes,
    Time,
}

/// A number's kind and value, or `None` for a number that is not gated
/// (sizes, counts of characters, rates).
fn classify(key: &str, v: &Value) -> Option<(Kind, f64)> {
    if key.ends_with("_peak_mb") {
        return v.as_f64().map(|n| (Kind::Peak, n));
    }
    if key.ends_with("_allocs") {
        return v.as_f64().map(|n| (Kind::Allocs, n));
    }
    if key.ends_with("_nodes") {
        return v.as_f64().map(|n| (Kind::Nodes, n));
    }
    if key.ends_with("_ms") {
        return v.as_f64().map(|n| (Kind::Time, n));
    }
    // Sampled timings: the median is steadier than the mean.
    v.get("p50_ms")
        .or_else(|| v.get("median_ms"))
        .and_then(Value::as_f64)
        .map(|n| (Kind::Time, n))
}

fn percent(ratio: f64) -> String {
    format!("{:.0} percent", (ratio * 100.0).abs())
}

fn amount(kind: Kind, n: f64) -> String {
    match kind {
        Kind::Peak => format!("{n:.1} MB"),
        Kind::Allocs | Kind::Nodes => format!("{n:.0}"),
        Kind::Time => format!("{n:.2} ms"),
    }
}

/// Compares `report` with one platform's entry of the baseline.
pub(crate) fn compare(report: &Value, base: &Value, policy: &Policy, gate_times: bool) -> Verdict {
    let mut out = Verdict::default();
    let Some(docs) = report.as_object() else {
        out.lines.push("The report is empty.".into());
        return out;
    };
    for (doc, cur) in docs {
        let Some(cur) = cur.as_object() else { continue };
        let Some(then_doc) = base.get(doc) else {
            out.lines.push(format!(
                "{doc}: not in the baseline, so nothing to compare."
            ));
            continue;
        };
        let (mut memory, mut times) = (0, 0);
        let mut largest: Option<(f64, String)> = None;
        let mut largest_time: Option<(f64, String)> = None;
        for (key, v) in cur {
            let Some((kind, now)) = classify(key, v) else {
                continue;
            };
            let Some((_, then)) = then_doc.get(key).and_then(|t| classify(key, t)) else {
                continue;
            };
            let report_only = policy
                .report_only
                .iter()
                .any(|p| key.contains(p.as_str()) || doc.contains(p.as_str()));
            let change = if then > 0.0 { now / then - 1.0 } else { 0.0 };
            let what = format!(
                "{doc}: {key} {} {}, from {} to {}",
                if change >= 0.0 { "grew" } else { "fell" },
                percent(change),
                amount(kind, then),
                amount(kind, now)
            );
            match kind {
                Kind::Peak | Kind::Allocs | Kind::Nodes => {
                    let floor = match kind {
                        Kind::Peak => policy.peak_floor_mb,
                        Kind::Nodes => policy.node_floor,
                        _ => policy.alloc_floor,
                    };
                    if now.max(then) < floor || report_only {
                        continue;
                    }
                    memory += 1;
                    if largest.as_ref().is_none_or(|(c, _)| change.abs() > c.abs()) {
                        largest = Some((change, key.clone()));
                    }
                    if change > policy.max_growth && now - then >= floor {
                        out.failures.push(format!(
                            "{what}; the limit is {}.",
                            percent(policy.max_growth)
                        ));
                    } else if -change > policy.max_improvement && then - now >= floor {
                        out.failures.push(format!(
                            "{what}, a gain of more than {}: write it into the baseline (cargo xtask bench --update-baseline --reason ...) so the floor follows it.",
                            percent(policy.max_improvement)
                        ));
                    }
                }
                Kind::Time => {
                    if then < policy.time_floor_ms || report_only {
                        continue;
                    }
                    times += 1;
                    if largest_time
                        .as_ref()
                        .is_none_or(|(c, _)| change.abs() > c.abs())
                    {
                        largest_time = Some((change, key.clone()));
                    }
                    if gate_times
                        && change > policy.max_time_growth
                        && now - then >= policy.time_growth_floor_ms
                    {
                        out.failures.push(format!(
                            "{what}; the limit is {}.",
                            percent(policy.max_time_growth)
                        ));
                    }
                }
            }
        }
        let mut line = format!("{doc}: {memory} memory numbers checked");
        if let Some((c, k)) = largest {
            line.push_str(&format!(
                ", the largest change {} {} ({k})",
                if c >= 0.0 { "up" } else { "down" },
                percent(c)
            ));
        }
        line.push_str(&format!(
            "; {times} times {}",
            if gate_times {
                "checked"
            } else {
                "reported, not gated"
            }
        ));
        if let Some((c, k)) = largest_time {
            line.push_str(&format!(
                ", the largest change {} {} ({k})",
                if c >= 0.0 { "up" } else { "down" },
                percent(c)
            ));
        }
        line.push('.');
        out.lines.push(line);
    }
    out
}

/// Runs the ratchet for this platform: prints the verdict, writes the
/// GitHub job summary when there is one, and fails on any failure.
pub(crate) fn gate(report: &Value, file: &Value, path: &Path) -> anyhow::Result<()> {
    let policy = Policy::from_json(file.get("policy"));
    let key = platform();
    let Some(entry) = file.get("platforms").and_then(|p| p.get(&key)) else {
        let line = format!(
            "{} has no baseline for {key}, so nothing is gated; write one with cargo xtask bench --update-baseline --reason TEXT.",
            path.display()
        );
        println!("{line}");
        summary(&[line], &[]);
        return Ok(());
    };
    let gate_times = entry
        .get("gate_times")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let base = entry.get("report").cloned().unwrap_or(Value::Null);
    let v = compare(report, &base, &policy, gate_times);
    println!(
        "Compared with the {key} baseline in {} ({}):",
        path.display(),
        entry
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("no reason given")
    );
    for l in &v.lines {
        println!("  {l}");
    }
    for f in &v.failures {
        println!("  Out of bounds: {f}");
    }
    summary(&v.lines, &v.failures);
    if !v.failures.is_empty() {
        bail!(
            "{} numbers are out of bounds against the baseline",
            v.failures.len()
        );
    }
    println!("Every gated number is within bounds.");
    Ok(())
}

fn summary(lines: &[String], failures: &[String]) {
    let Some(file) = std::env::var_os("GITHUB_STEP_SUMMARY") else {
        return;
    };
    use std::io::Write as _;
    let Ok(mut f) = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(file)
    else {
        return;
    };
    let _ = writeln!(f, "## Benchmark gate\n");
    for l in lines {
        let _ = writeln!(f, "- {l}");
    }
    for l in failures {
        let _ = writeln!(f, "- Out of bounds: {l}");
    }
    if failures.is_empty() {
        let _ = writeln!(f, "\nEvery gated number is within bounds.");
    }
}

/// The checked-out branch, or `None` when it cannot be told (a detached
/// head outside GitHub Actions).
fn branch(root: &Path) -> Option<String> {
    if let Ok(r) = std::env::var("GITHUB_REF")
        && let Some(b) = r.strip_prefix("refs/heads/")
    {
        return Some(b.to_owned());
    }
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .filter(|b| b != "HEAD")
}

fn commit(root: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--short=10", "HEAD"])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// Today's date from the machine clock, as `YYYY-MM-DD` (UTC).
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    // Howard Hinnant's civil-from-days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Options for [`update`].
pub(crate) struct Update<'a> {
    pub path: &'a Path,
    pub root: &'a Path,
    pub reason: Option<&'a str>,
    pub quick: bool,
    /// Gate times against this entry (only for a run on the machine type
    /// the gate itself runs on).
    pub gate_times: bool,
}

/// Writes `report` as this platform's entry of the baseline file, keeping
/// the other platforms and the policy. Refuses without a reason, and off
/// `main` when the platform already has an entry.
pub(crate) fn update(report: &Value, u: &Update<'_>) -> anyhow::Result<PathBuf> {
    let Some(reason) = u.reason.map(str::trim).filter(|r| !r.is_empty()) else {
        bail!("--update-baseline needs --reason TEXT: why the numbers moved");
    };
    let mut file: Value = match std::fs::read_to_string(u.path) {
        Ok(text) => {
            serde_json::from_str(&text).with_context(|| format!("parsing {}", u.path.display()))?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({}),
        Err(e) => return Err(e).with_context(|| format!("reading {}", u.path.display())),
    };
    let key = platform();
    let exists = file.get("platforms").and_then(|p| p.get(&key)).is_some();
    let on = branch(u.root);
    if exists && on.as_deref() != Some("main") {
        bail!(
            "the {key} baseline is written only on main (this is {}); a new platform's first entry may be written from any branch",
            on.as_deref().unwrap_or("a detached head")
        );
    }
    let obj = file
        .as_object_mut()
        .context("the baseline file is not a JSON object")?;
    obj.insert(
        "about".into(),
        json!(
            "The bench gate's baseline (xtask/src/ratchet.rs). Written only by cargo xtask bench --update-baseline, in a commit that says why."
        ),
    );
    let policy = Policy::from_json(obj.get("policy"));
    obj.insert("policy".into(), policy.to_json());
    let platforms = obj
        .entry("platforms")
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(platforms) = platforms.as_object_mut() else {
        bail!("platforms in the baseline file is not an object");
    };
    let cpus = std::thread::available_parallelism().map_or(0, std::num::NonZero::get);
    platforms.insert(
        key.clone(),
        json!({
            "date": today(),
            "commit": commit(u.root),
            "branch": on,
            "reason": reason,
            "quick": u.quick,
            "cpus": cpus,
            "gate_times": u.gate_times,
            "report": report,
        }),
    );
    let text = serde_json::to_string_pretty(&file)? + "\n";
    std::fs::write(u.path, text).with_context(|| format!("writing {}", u.path.display()))?;
    println!(
        "Wrote the {key} baseline into {} ({reason}).",
        u.path.display()
    );
    Ok(u.path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Value {
        json!({
            "md-1mb.md": {
                "load_ms": 20.0,
                "load_peak_mb": 20.0,
                "load_allocs": 100_000,
                "plan_all_peak_mb": 0.2,
                "tiny_allocs": 900,
                "nav_idle_word_dispatch": {"mean_ms": 0.1, "p50_ms": 0.1, "max_ms": 0.2, "n": 20},
                "open_to_first_speech_ms": 40.0,
                "save_written": {"mean_ms": 30.0, "p50_ms": 30.0, "max_ms": 50.0, "n": 5},
            },
        })
    }

    #[test]
    fn small_changes_pass_both_ways() {
        let now = json!({"md-1mb.md": {
            "load_ms": 24.0, "load_peak_mb": 23.0, "load_allocs": 90_000,
            "open_to_first_speech_ms": 44.0,
        }, "new.md": {"load_allocs": 1}});
        let v = compare(&now, &base(), &Policy::default(), true);
        assert!(v.failures.is_empty(), "{:?}", v.failures);
        assert!(
            v.lines
                .iter()
                .any(|l| l.starts_with("new.md: not in the baseline"))
        );
        assert!(
            v.lines
                .iter()
                .any(|l| l.contains("2 memory numbers checked") && l.contains("2 times checked")),
            "{:?}",
            v.lines
        );
    }

    #[test]
    fn memory_fails_on_growth_and_on_an_unrecorded_gain() {
        let grew = json!({"md-1mb.md": {"load_allocs": 130_000}});
        let v = compare(&grew, &base(), &Policy::default(), false);
        assert_eq!(v.failures.len(), 1, "{:?}", v.failures);
        assert!(v.failures[0].contains("load_allocs grew 30 percent"));
        let fell = json!({"md-1mb.md": {"load_peak_mb": 10.0}});
        let v = compare(&fell, &base(), &Policy::default(), false);
        assert_eq!(v.failures.len(), 1, "{:?}", v.failures);
        assert!(v.failures[0].contains("fell 50 percent"));
        assert!(v.failures[0].contains("--update-baseline"));
        // Below the floors nothing is gated, however large the change.
        let tiny = json!({"md-1mb.md": {"plan_all_peak_mb": 0.9, "tiny_allocs": 4000}});
        assert!(
            compare(&tiny, &base(), &Policy::default(), false)
                .failures
                .is_empty()
        );
    }

    #[test]
    fn times_fail_only_on_growth_and_only_when_gated() {
        let slow = json!({"md-1mb.md": {"open_to_first_speech_ms": 70.0, "load_ms": 31.0}});
        let v = compare(&slow, &base(), &Policy::default(), true);
        assert_eq!(v.failures.len(), 2, "{:?}", v.failures);
        // The same run against an entry from another machine: reported only.
        let v = compare(&slow, &base(), &Policy::default(), false);
        assert!(v.failures.is_empty());
        assert!(v.lines[0].contains("reported, not gated"));
        // Faster is never a failure; a sampled time uses its median; the
        // tiny dispatch time is under the floor; disk writes are report-only.
        let fast = json!({"md-1mb.md": {
            "open_to_first_speech_ms": 5.0,
            "nav_idle_word_dispatch": {"mean_ms": 9.0, "p50_ms": 9.0, "max_ms": 9.0, "n": 20},
            "save_written": {"mean_ms": 300.0, "p50_ms": 300.0, "max_ms": 500.0, "n": 5},
        }});
        assert!(
            compare(&fast, &base(), &Policy::default(), true)
                .failures
                .is_empty()
        );
    }

    #[test]
    fn node_counts_are_gated_like_allocations() {
        let base = json!({"run": {"moves_nodes": 1000, "few_nodes": 20}});
        let more = json!({"run": {"moves_nodes": 1400, "few_nodes": 90}});
        let v = compare(&more, &base, &Policy::default(), false);
        assert_eq!(v.failures.len(), 1, "{:?}", v.failures);
        assert!(v.failures[0].contains("moves_nodes grew 40 percent, from 1000 to 1400"));
    }

    #[test]
    fn the_policy_reads_back_and_fills_gaps() {
        let p = Policy::default();
        assert_eq!(Policy::from_json(Some(&p.to_json())), p);
        let partial = json!({"max_growth": 0.1});
        assert!((Policy::from_json(Some(&partial)).max_growth - 0.1).abs() < 1e-9);
        assert_eq!(Policy::from_json(None), p);
        assert!(is_baseline(&json!({"platforms": {}})));
        assert!(!is_baseline(&base()));
    }

    #[test]
    fn today_is_a_date() {
        let t = today();
        assert_eq!(t.len(), 10);
        assert!(t.starts_with("20"));
        assert_eq!(&t[4..5], "-");
    }
}
