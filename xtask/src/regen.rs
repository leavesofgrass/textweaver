//! `cargo xtask regen [--check]`: rebuilds every generated file, in order.
//!
//! 1. `THIRD-PARTY-NOTICES.md` (as `cargo xtask notices`; needs
//!    `cargo-about`; without it the step is reported as skipped, and
//!    fails under `--require-all`).
//! 2. `docs/settings-reference.md` (as `cargo xtask settings-doc`; builds
//!    the app crate's `settings_reference` test).
//! 3. `docs/keyboard.md` (as `cargo xtask keyboard`).
//! 4. The data in `docs/site/*.html` (`tools/gen_site_data.py`, run with
//!    `py -3` on Windows and `python3` elsewhere). It reads
//!    `docs/keyboard.md`, so it comes after step 3.
//! 5. The crate counts in `docs/README.md` and `docs/dev/architecture.md`,
//!    then the rest of `cargo xtask docs --check` (the ADR lists, the See
//!    also sections, the index links), which are written by hand and only
//!    reported.
//!
//! With `--check` nothing is written: every step runs, even after one
//! fails, and each reports one line, meaning first, such as
//! `notices: pass` or `site data: FAIL, out of date; run cargo xtask
//! regen`. The exit status is non-zero when any step failed. A step whose
//! tool is not installed is reported as "SKIPPED, not checked" and, by
//! default, does not fail the run; the summary names it.
//!
//! `--require-all` (for CI, where every tool is installed) turns a skip
//! into a failure: "notices: FAIL, not checked: cargo-about is not
//! installed". A gate that skips silently is a gate that is never run.

use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::bail;

/// Whether to write the files or only check them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Rewrite every generated file.
    Write,
    /// Change nothing; report which files are out of date.
    Check,
}

/// What one step found or did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Checked, and current.
    Pass,
    /// Written.
    Written,
    /// Checked, and out of date.
    Stale,
    /// Not run, for the reason given (a tool is missing).
    Skipped(String),
    /// Not run although `--require-all` asked for every step: a failure.
    Required(String),
    /// Failed, for the reason given.
    Failed(String),
}

impl Outcome {
    fn failed(&self) -> bool {
        matches!(
            self,
            Outcome::Stale | Outcome::Failed(_) | Outcome::Required(_)
        )
    }
}

/// The usage line.
const USAGE: &str = "usage: cargo xtask regen [--check] [--require-all]";

/// What the arguments after `regen` ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// Write or check.
    pub mode: Mode,
    /// A step skipped for a missing tool fails the run (CI).
    pub require_all: bool,
}

/// The options the arguments after `regen` ask for.
pub fn parse_args(args: &[String]) -> anyhow::Result<Options> {
    let mut options = Options {
        mode: Mode::Write,
        require_all: false,
    };
    let mut seen_check = false;
    for a in args {
        match a.as_str() {
            "--check" if !seen_check => {
                seen_check = true;
                options.mode = Mode::Check;
            }
            "--require-all" if !options.require_all => options.require_all = true,
            _ => bail!("{USAGE}"),
        }
    }
    Ok(options)
}

/// A skip becomes a failure when every step is required.
pub fn require(outcome: Outcome, require_all: bool) -> Outcome {
    match outcome {
        Outcome::Skipped(why) if require_all => Outcome::Required(why),
        other => other,
    }
}

/// One step's report line, meaning first.
pub fn line(label: &str, outcome: &Outcome) -> String {
    match outcome {
        Outcome::Pass => format!("{label}: pass"),
        Outcome::Written => format!("{label}: written"),
        Outcome::Stale => format!("{label}: FAIL, out of date; run cargo xtask regen"),
        Outcome::Skipped(why) => format!("{label}: SKIPPED, not checked: {why}"),
        Outcome::Required(why) => format!("{label}: FAIL, not checked: {why}"),
        Outcome::Failed(why) => format!("{label}: FAIL, {why}"),
    }
}

/// The summary: the count line first, then one line per step. The second
/// value is true when any step failed.
pub fn summary(mode: Mode, results: &[(&str, Outcome)]) -> (Vec<String>, bool) {
    let failed: Vec<&str> = results
        .iter()
        .filter(|(_, o)| o.failed())
        .map(|(l, _)| *l)
        .collect();
    let skipped: Vec<&str> = results
        .iter()
        .filter(|(_, o)| matches!(o, Outcome::Skipped(_)))
        .map(|(l, _)| *l)
        .collect();
    let skipped_n = skipped.len();
    let total = results.len();
    let mut head = if failed.is_empty() {
        match mode {
            Mode::Check => format!("Pass: {} of {total} checks passed", total - skipped_n),
            Mode::Write => format!(
                "Done: {} of {total} steps rebuilt their files",
                total - skipped_n
            ),
        }
    } else {
        format!(
            "FAIL: {} of {total} {} failed: {}",
            failed.len(),
            if failed.len() == 1 { "step" } else { "steps" },
            failed.join(", ")
        )
    };
    if skipped_n > 0 {
        head.push_str(&format!(
            ", {skipped_n} SKIPPED, not checked: {}",
            skipped.join(", ")
        ));
    }
    head.push('.');
    let mut lines = vec![head];
    lines.extend(results.iter().map(|(l, o)| line(l, o)));
    (lines, !failed.is_empty())
}

/// One step: it checks or rebuilds one generated file or set of files.
type Step = fn(&Path, Mode) -> Outcome;

/// The steps, in the order they run.
const STEPS: [(&str, Step); 5] = [
    ("notices", notices),
    ("settings reference", settings_reference),
    ("keyboard", keyboard),
    ("site data", site_data),
    ("docs", docs),
];

/// `cargo xtask regen [--check]`.
pub fn run() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let Options { mode, require_all } = parse_args(&args)?;
    let root = crate::eci::root();
    let mut results = Vec::new();
    for (label, step) in STEPS {
        println!(
            "{label}: {}",
            if mode == Mode::Check {
                "checking"
            } else {
                "rebuilding"
            }
        );
        let outcome = require(step(&root, mode), require_all);
        println!("{}", line(label, &outcome));
        results.push((label, outcome));
    }
    let (lines, failed) = summary(mode, &results);
    println!();
    println!("Summary");
    for l in &lines {
        println!("{l}");
    }
    if failed {
        std::process::exit(1);
    }
    Ok(())
}

/// A step's error, as one line.
/// The whole error goes to stderr first, since its later lines (a tool's
/// own output) may be what explains it.
fn failed(e: &anyhow::Error) -> Outcome {
    let text = format!("{e:#}");
    if text.lines().nth(1).is_some() {
        eprintln!("{text}");
    }
    Outcome::Failed(first_line(&text))
}

/// An error's first line, without a trailing colon, and a pointer to the
/// rest when there is more.
fn first_line(text: &str) -> String {
    let first = text
        .lines()
        .next()
        .unwrap_or("")
        .trim_end()
        .trim_end_matches(':');
    if text.lines().skip(1).any(|l| !l.trim().is_empty()) {
        format!("{first}; the details are above")
    } else {
        first.to_owned()
    }
}

fn notices(root: &Path, mode: Mode) -> Outcome {
    if !crate::notices::cargo_about_installed() {
        return Outcome::Skipped(
            "cargo-about is not installed (cargo install --locked cargo-about)".into(),
        );
    }
    let result = match mode {
        Mode::Check => crate::notices::is_current(root).map(pass_or_stale),
        Mode::Write => crate::notices::write(root).map(|()| Outcome::Written),
    };
    result.unwrap_or_else(|e| failed(&e))
}

fn settings_reference(_root: &Path, mode: Mode) -> Outcome {
    match crate::docs_check::run_settings_test(mode == Mode::Check) {
        Ok(true) if mode == Mode::Check => Outcome::Pass,
        Ok(true) => Outcome::Written,
        Ok(false) if mode == Mode::Check => Outcome::Failed(
            "out of date, or the app crate did not build; run cargo xtask regen".into(),
        ),
        Ok(false) => Outcome::Failed("the app crate's settings_reference test failed".into()),
        Err(e) => failed(&e),
    }
}

fn keyboard(_root: &Path, mode: Mode) -> Outcome {
    let result = match mode {
        Mode::Check => crate::keyboard::is_current().map(pass_or_stale),
        Mode::Write => crate::keyboard::write().map(|()| Outcome::Written),
    };
    result.unwrap_or_else(|e| failed(&e))
}

fn site_data(root: &Path, mode: Mode) -> Outcome {
    let Some((program, pre)) = python() else {
        return Outcome::Skipped("Python 3 is not installed".into());
    };
    let mut cmd = Command::new(program);
    cmd.current_dir(root)
        .args(pre)
        .arg(root.join("tools").join("gen_site_data.py"));
    if mode == Mode::Check {
        cmd.arg("--check");
    }
    match cmd.status() {
        Ok(s) if s.success() => match mode {
            Mode::Check => Outcome::Pass,
            Mode::Write => Outcome::Written,
        },
        Ok(s) if mode == Mode::Check && s.code() == Some(1) => Outcome::Stale,
        Ok(s) => Outcome::Failed(format!(
            "tools/gen_site_data.py stopped with exit code {}",
            s.code()
                .map_or_else(|| "none".to_owned(), |c| c.to_string())
        )),
        Err(e) => Outcome::Failed(format!("could not run Python: {e}")),
    }
}

fn docs(root: &Path, mode: Mode) -> Outcome {
    if mode == Mode::Write {
        match crate::docs_check::write_crate_counts(root) {
            Ok(changed) => {
                for file in changed {
                    println!("wrote the crate count in {file}");
                }
            }
            Err(e) => return failed(&e),
        }
    }
    let problems = match crate::docs_check::check(root) {
        Ok(p) => p,
        Err(e) => return failed(&e),
    };
    if problems.is_empty() {
        return match mode {
            Mode::Check => Outcome::Pass,
            Mode::Write => Outcome::Written,
        };
    }
    for p in &problems {
        eprintln!("{p}");
    }
    let counts_only = problems
        .iter()
        .all(|p| p.contains(" crates, but crates/ has "));
    if counts_only && mode == Mode::Check {
        return Outcome::Stale;
    }
    let n = problems.len();
    Outcome::Failed(format!(
        "{n} {} to fix by hand, listed above",
        if n == 1 { "problem" } else { "problems" }
    ))
}

fn pass_or_stale(current: bool) -> Outcome {
    if current {
        Outcome::Pass
    } else {
        Outcome::Stale
    }
}

/// The Python 3 to run: `py -3` on Windows (where `python` may be the
/// Microsoft Store stub), else `python3`, else `python`. Each is tried
/// with `--version` first.
fn python() -> Option<(OsString, Vec<&'static str>)> {
    let candidates: &[(&str, &[&'static str])] = if cfg!(windows) {
        &[("py", &["-3"]), ("python3", &[])]
    } else {
        &[("python3", &[]), ("python", &[])]
    };
    candidates.iter().find_map(|(program, pre)| {
        let ok = Command::new(program)
            .args(*pre)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        ok.then(|| (OsString::from(program), pre.to_vec()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn arguments_choose_the_mode() {
        let opts = |mode, require_all| Options { mode, require_all };
        assert_eq!(parse_args(&args(&[])).unwrap(), opts(Mode::Write, false));
        assert_eq!(
            parse_args(&args(&["--check"])).unwrap(),
            opts(Mode::Check, false)
        );
        assert_eq!(
            parse_args(&args(&["--check", "--require-all"])).unwrap(),
            opts(Mode::Check, true)
        );
        assert_eq!(
            parse_args(&args(&["--require-all", "--check"])).unwrap(),
            opts(Mode::Check, true)
        );
        for bad in [
            &["--chek"][..],
            &["--check", "--check"],
            &["notices"],
            &["--require-all", "--require-all"],
        ] {
            let e = parse_args(&args(bad)).unwrap_err();
            assert_eq!(e.to_string(), USAGE);
        }
    }

    #[test]
    fn require_all_turns_a_skip_into_a_failure() {
        let skip = || Outcome::Skipped("cargo-about is not installed".into());
        assert_eq!(require(skip(), false), skip());
        let required = require(skip(), true);
        assert!(required.failed());
        assert_eq!(
            line("notices", &required),
            "notices: FAIL, not checked: cargo-about is not installed"
        );
        assert_eq!(require(Outcome::Pass, true), Outcome::Pass);
        let (lines, failed) = summary(Mode::Check, &[("notices", required)]);
        assert!(failed);
        assert_eq!(lines[0], "FAIL: 1 of 1 step failed: notices.");
    }

    #[test]
    fn each_line_puts_the_meaning_first() {
        assert_eq!(line("notices", &Outcome::Pass), "notices: pass");
        assert_eq!(line("keyboard", &Outcome::Written), "keyboard: written");
        assert_eq!(
            line("site data", &Outcome::Stale),
            "site data: FAIL, out of date; run cargo xtask regen"
        );
        assert_eq!(
            line(
                "notices",
                &Outcome::Skipped("cargo-about is not installed".into())
            ),
            "notices: SKIPPED, not checked: cargo-about is not installed"
        );
        assert_eq!(
            line("docs", &Outcome::Failed("2 problems".into())),
            "docs: FAIL, 2 problems"
        );
    }

    #[test]
    fn errors_become_one_line() {
        assert_eq!(first_line("reading x: not found"), "reading x: not found");
        assert_eq!(
            first_line("cargo about failed (install it):\nERROR something\n"),
            "cargo about failed (install it); the details are above"
        );
        assert_eq!(first_line("stale:\n\n"), "stale");
    }

    #[test]
    fn every_failure_is_counted_and_named() {
        let results = [
            ("notices", Outcome::Pass),
            ("keyboard", Outcome::Stale),
            ("site data", Outcome::Failed("no Python".into())),
            ("docs", Outcome::Pass),
        ];
        let (lines, failed) = summary(Mode::Check, &results);
        assert!(failed);
        assert_eq!(lines[0], "FAIL: 2 of 4 steps failed: keyboard, site data.");
        assert_eq!(lines.len(), 5);
        assert_eq!(
            lines[2],
            "keyboard: FAIL, out of date; run cargo xtask regen"
        );
    }

    #[test]
    fn a_clean_run_passes_and_skips_do_not_fail() {
        let results = [
            (
                "notices",
                Outcome::Skipped("cargo-about is not installed".into()),
            ),
            ("keyboard", Outcome::Pass),
        ];
        let (lines, failed) = summary(Mode::Check, &results);
        assert!(!failed);
        assert_eq!(
            lines[0],
            "Pass: 1 of 2 checks passed, 1 SKIPPED, not checked: notices."
        );
        let (lines, failed) = summary(Mode::Write, &[("keyboard", Outcome::Written)]);
        assert!(!failed);
        assert_eq!(lines[0], "Done: 1 of 1 steps rebuilt their files.");
    }

    #[test]
    fn the_site_data_runs_after_the_keyboard_reference() {
        let order: Vec<&str> = STEPS.iter().map(|(l, _)| *l).collect();
        let pos = |l| order.iter().position(|x| *x == l).unwrap();
        assert!(pos("keyboard") < pos("site data"));
        assert_eq!(order.len(), 5);
    }
}
