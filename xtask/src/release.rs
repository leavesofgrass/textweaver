//! `cargo xtask release X.Y.Z [--dry-run] [--no-checks]`: prepare a release
//! commit and tag (docs/dev/releasing.md).
//!
//! 1. Checks that the working tree is clean and on `main`; that the
//!    listening checklist in the release guide is dated, for this version,
//!    within the last 14 days; and that the changelog's `[Unreleased]`
//!    section is grouped by area, with no agent's heading left in it.
//! 2. Sets `version` in `[workspace.package]` and runs `cargo update -w`.
//! 3. Turns `## [Unreleased]` in `CHANGELOG.md` into
//!    `## [X.Y.Z] - YYYY-MM-DD`, dated from this machine's clock in local
//!    time, keeps an empty `[Unreleased]` above it, and adds the link.
//! 4. Updates the version examples in the release guide, the install guide,
//!    and the workflows.
//! 5. Runs the checks CI runs (fmt, clippy, tests, the keyboard and
//!    dependency checks), unless `--no-checks`.
//! 6. Commits and makes the annotated tag `vX.Y.Z`. Nothing is pushed.
//!
//! `--dry-run` changes nothing: it prints what the release would do and the
//! date it would use, and reports a dirty tree, another branch, or an
//! undated listening check without stopping.
//!
//! `cargo xtask release X.Y.Z --listened` records the listening check
//! instead: it writes today's date, from the machine, and the version on
//! the "Last listening check" line of the release guide, and does nothing
//! else. Run it after listening, and commit the guide.
//!
//! The date is never typed in or guessed: it comes from the machine
//! (`date` or PowerShell's `Get-Date`), and the weekday is computed from it.

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, bail};

/// Files whose version examples follow the release, relative to the root.
const EXAMPLE_FILES: [&str; 4] = [
    "docs/dev/releasing.md",
    "docs/install.md",
    ".github/workflows/scripts.yml",
    ".github/workflows/release.yml",
];

/// The repository, for the changelog link.
const REPO_URL: &str = "https://github.com/leavesofgrass/textweaver";

/// The release guide, which holds the listening checklist.
const GUIDE: &str = "docs/dev/releasing.md";
/// The line in the release guide that records the last listening check.
const LISTENED: &str = "**Last listening check:**";
/// How old a listening check may be, in days, for a release.
const LISTENED_MAX_DAYS: i64 = 14;

/// Parsed arguments.
#[derive(Debug, PartialEq, Eq)]
struct Args {
    version: String,
    dry_run: bool,
    checks: bool,
    listened: bool,
}

const USAGE: &str =
    "usage: cargo xtask release X.Y.Z[-PRE] [--dry-run] [--no-checks] | X.Y.Z[-PRE] --listened";

fn parse(args: &[String]) -> anyhow::Result<Args> {
    let mut version = None;
    let mut dry_run = false;
    let mut checks = true;
    let mut listened = false;
    for a in args {
        match a.as_str() {
            "--dry-run" => dry_run = true,
            "--no-checks" => checks = false,
            "--listened" => listened = true,
            s if s.starts_with('-') => bail!("unknown option {s} ({USAGE})"),
            s if version.is_none() => version = Some(s.trim_start_matches('v').to_owned()),
            s => bail!("unexpected argument {s} ({USAGE})"),
        }
    }
    let version = version.context(USAGE)?;
    if !valid_version(&version) {
        bail!("{version} is not a version like 0.2.0 or 0.2.0-alpha.1");
    }
    if listened && (dry_run || !checks) {
        bail!("--listened only records the listening check; run it on its own ({USAGE})");
    }
    Ok(Args {
        version,
        dry_run,
        checks,
        listened,
    })
}

/// `X.Y.Z` or `X.Y.Z-PRE`, where PRE is dot-separated ASCII letters, digits,
/// and hyphens.
fn valid_version(v: &str) -> bool {
    let (core, pre) = match v.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (v, None),
    };
    let nums: Vec<&str> = core.split('.').collect();
    let core_ok = nums.len() == 3
        && nums.iter().all(|n| {
            !n.is_empty()
                && n.bytes().all(|b| b.is_ascii_digit())
                && (n.len() == 1 || !n.starts_with('0'))
        });
    let pre_ok = pre.is_none_or(|p| {
        p.split('.').all(|part| {
            !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
    });
    core_ok && pre_ok
}

/// `cargo xtask release`.
pub fn run() -> anyhow::Result<()> {
    let args = parse(&std::env::args().skip(2).collect::<Vec<_>>())?;
    let root = crate::eci::root();
    let old = env!("CARGO_PKG_VERSION");
    let date = local_date()?;
    let (y, m, d) = parse_date(&date).with_context(|| format!("the machine's date {date:?}"))?;
    let spoken = spoken_date(y, m, d);
    let tag = format!("v{}", args.version);

    if args.listened {
        let path = root.join(GUIDE);
        let text = fs::read_to_string(&path).with_context(|| format!("reading {GUIDE}"))?;
        let text = record_listening(&text, &args.version, &date, &spoken)?;
        fs::write(&path, text).with_context(|| format!("writing {GUIDE}"))?;
        println!(
            "Recorded: listening check for {} on {spoken}, in {GUIDE}. Commit it, then run cargo xtask release {}.",
            args.version, args.version
        );
        return Ok(());
    }

    println!(
        "release {} (from {old}), dated {date} ({spoken})",
        args.version
    );

    // 1. Clean tree on main.
    let mut problems = Vec::new();
    let status = git_output(&root, &["status", "--porcelain"])?;
    if !status.trim().is_empty() {
        problems.push(format!(
            "the working tree has changes:\n{}",
            status.trim_end()
        ));
    }
    let branch = git_output(&root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    if branch.trim() != "main" {
        problems.push(format!("on branch {}, not main", branch.trim()));
    }
    if git_output(&root, &["tag", "--list", &tag])?.trim() == tag {
        problems.push(format!("the tag {tag} exists already"));
    }
    if args.version == old {
        problems.push(format!("the workspace is already at {old}"));
    }
    let guide = fs::read_to_string(root.join(GUIDE)).with_context(|| format!("reading {GUIDE}"))?;
    if let Err(e) = check_listening(&guide, &args.version, (y, m, d)) {
        problems.push(e);
    }
    let headings = agent_headings(&fs::read_to_string(root.join("CHANGELOG.md"))?);
    if !headings.is_empty() {
        problems.push(format!(
            "the [Unreleased] section of CHANGELOG.md still has agents' headings; group their lines by area first: {}",
            headings.join("; ")
        ));
    }
    if !problems.is_empty() {
        if args.dry_run {
            for p in &problems {
                println!("would stop: {p}");
            }
        } else {
            bail!("cannot release: {}", problems.join("; "));
        }
    }

    // 2 to 4: the new file contents.
    let cargo_toml = root.join("Cargo.toml");
    let manifest = fs::read_to_string(&cargo_toml)?;
    let manifest = set_workspace_version(&manifest, &args.version)?;
    let changelog_path = root.join("CHANGELOG.md");
    let changelog = fs::read_to_string(&changelog_path)?;
    let changelog = date_changelog(&changelog, &args.version, &date)?;
    let mut writes = vec![
        (cargo_toml.clone(), manifest),
        (changelog_path.clone(), changelog),
    ];
    for rel in EXAMPLE_FILES {
        let path = root.join(rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let new = text.replace(old, &args.version);
        if new != text {
            writes.push((path, new));
        }
    }

    if args.dry_run {
        for (path, _) in &writes {
            println!("would update {}", rel(&root, path));
        }
        println!("would run: cargo update -w");
        if args.checks {
            for c in checks() {
                println!("would run: cargo {}", c.join(" "));
            }
        }
        println!("would commit \"Release {}\" and tag {tag}", args.version);
        return Ok(());
    }

    for (path, text) in &writes {
        fs::write(path, text).with_context(|| format!("writing {}", path.display()))?;
        println!("updated {}", rel(&root, path));
    }
    cargo(&root, &["update", "-w"])?;

    // 5. Checks.
    if args.checks {
        for c in checks() {
            let c: Vec<&str> = c.iter().map(String::as_str).collect();
            cargo(&root, &c)?;
        }
    }

    // 6. Commit and tag.
    git(&root, &["add", "Cargo.toml", "Cargo.lock", "CHANGELOG.md"])?;
    for (path, _) in &writes {
        git(&root, &["add", &rel(&root, path)])?;
    }
    git(
        &root,
        &["commit", "-m", &format!("Release {}", args.version)],
    )?;
    git(
        &root,
        &[
            "tag",
            "-a",
            &tag,
            "-m",
            &format!("textweaver {}", args.version),
        ],
    )?;
    println!("committed and tagged {tag}. Push with:");
    println!("  git push origin main");
    println!("  git push origin {tag}");
    Ok(())
}

/// The checks CI runs, as cargo arguments.
fn checks() -> Vec<Vec<String>> {
    let features = if cfg!(target_os = "linux") {
        "--all-features"
    } else {
        "--features=textweaver-speech/omnivox"
    };
    let v = |s: &[&str]| s.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
    vec![
        v(&["fmt", "--all", "--check"]),
        v(&[
            "clippy",
            "--workspace",
            "--exclude",
            "textweaver-xilem",
            "--all-targets",
            "--locked",
            features,
            "--",
            "-D",
            "warnings",
        ]),
        v(&[
            "test",
            "--workspace",
            "--exclude",
            "textweaver-xilem",
            "--locked",
            features,
        ]),
        v(&["xtask", "keyboard", "--check"]),
        v(&["xtask", "deps", "--check"]),
    ]
}

/// `path` relative to `root`, with forward slashes.
fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Sets `version = "..."` in `[workspace.package]`.
fn set_workspace_version(manifest: &str, version: &str) -> anyhow::Result<String> {
    let mut out = String::with_capacity(manifest.len());
    let mut section = "";
    let mut done = false;
    for line in manifest.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            section = if trimmed == "[workspace.package]" {
                "package"
            } else {
                ""
            };
        }
        if section == "package" && !done && trimmed.starts_with("version") {
            let eol = if line.ends_with("\r\n") {
                "\r\n"
            } else if line.ends_with('\n') {
                "\n"
            } else {
                ""
            };
            out.push_str(&format!("version = \"{version}\"{eol}"));
            done = true;
        } else {
            out.push_str(line);
        }
    }
    if !done {
        bail!("no version in [workspace.package] in Cargo.toml");
    }
    Ok(out)
}

/// Dates the `[Unreleased]` section as `[version] - date`, keeps an empty
/// `[Unreleased]` heading above it, and adds the release link.
fn date_changelog(text: &str, version: &str, date: &str) -> anyhow::Result<String> {
    const HEADING: &str = "## [Unreleased]";
    if text.contains(&format!("## [{version}]")) {
        bail!("CHANGELOG.md already has a section for {version}");
    }
    let at = text
        .find(HEADING)
        .context("CHANGELOG.md has no \"## [Unreleased]\" heading")?;
    let rest = &text[at + HEADING.len()..];
    let body_end = rest.find("\n## ").unwrap_or(rest.len());
    if rest[..body_end].trim().is_empty() {
        bail!("the [Unreleased] section of CHANGELOG.md is empty");
    }
    let mut out = String::with_capacity(text.len() + 128);
    out.push_str(&text[..at]);
    out.push_str(&format!("{HEADING}\n\n## [{version}] - {date}"));
    out.push_str(rest);
    // The link, above the other release links (or at the end).
    let link = format!("[{version}]: {REPO_URL}/releases/tag/v{version}\n");
    match first_link_line(&out) {
        Some(pos) => out.insert_str(pos, &link),
        None => {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
            out.push_str(&link);
        }
    }
    Ok(out)
}

/// Checks the "Last listening check" line of the release guide: it must
/// hold a date no later than `today` and at most [`LISTENED_MAX_DAYS`] old,
/// and name `version`. The error says what to do, meaning first.
fn check_listening(guide: &str, version: &str, today: (i64, u32, u32)) -> Result<(), String> {
    let how = format!(
        "listen through the checklist in {GUIDE}, then run cargo xtask release {version} --listened"
    );
    let Some(rest) = guide
        .lines()
        .find_map(|l| l.trim_start().strip_prefix(LISTENED))
    else {
        return Err(format!(
            "listening check missing: {GUIDE} has no \"{LISTENED}\" line"
        ));
    };
    let rest = rest.trim();
    let Some((y, m, d)) = rest.get(..10).and_then(parse_date) else {
        return Err(format!("listening check not dated: {how}"));
    };
    let age = days_from_civil(today.0, today.1, today.2) - days_from_civil(y, m, d);
    if age < 0 {
        return Err(format!(
            "listening check dated in the future ({}): {how}",
            &rest[..10]
        ));
    }
    if age > LISTENED_MAX_DAYS {
        return Err(format!(
            "listening check is {age} days old, more than {LISTENED_MAX_DAYS}: {how}"
        ));
    }
    let listened_for = rest
        .rsplit_once(" for ")
        .map(|(_, v)| v.trim().trim_end_matches('.').trim_start_matches('v'));
    if listened_for != Some(version) {
        return Err(format!(
            "listening check was for {}, not {version}: {how}",
            listened_for.unwrap_or("no version")
        ));
    }
    Ok(())
}

/// Rewrites the "Last listening check" line with `date` (and its spoken
/// form) and `version`.
fn record_listening(
    guide: &str,
    version: &str,
    date: &str,
    spoken: &str,
) -> anyhow::Result<String> {
    let mut out = String::with_capacity(guide.len() + 64);
    let mut done = false;
    for line in guide.split_inclusive('\n') {
        if !done && line.trim_start().starts_with(LISTENED) {
            let eol = if line.ends_with("\r\n") {
                "\r\n"
            } else if line.ends_with('\n') {
                "\n"
            } else {
                ""
            };
            out.push_str(&format!(
                "{LISTENED} {date} ({spoken}), for {version}.{eol}"
            ));
            done = true;
        } else {
            out.push_str(line);
        }
    }
    if !done {
        bail!("{GUIDE} has no \"{LISTENED}\" line to record the listening check on");
    }
    Ok(out)
}

/// Headings in the `[Unreleased]` section that name an agent rather than
/// an area: `### W4c2: documents`, `### Agent ...`, `### ... (Wave 4, Agent
/// W4h)`, `### The Cloud Agent ...`. Agents write under their own heading;
/// before a release their lines are grouped by area.
fn agent_headings(changelog: &str) -> Vec<String> {
    let Some(at) = changelog.find("## [Unreleased]") else {
        return Vec::new();
    };
    let rest = &changelog[at + "## [Unreleased]".len()..];
    let body = &rest[..rest.find("\n## ").unwrap_or(rest.len())];
    body.lines()
        .filter_map(|l| l.strip_prefix("### "))
        .filter(|h| {
            let b = h.as_bytes();
            let wave_agent = b.len() > 1 && b[0] == b'W' && b[1].is_ascii_digit();
            wave_agent || h.contains("Agent")
        })
        .map(str::to_owned)
        .collect()
}

/// The byte offset of the first `[x.y.z]: http...` link definition line.
fn first_link_line(text: &str) -> Option<usize> {
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if line.starts_with('[') && line.contains("]: http") {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

/// Today's date on this machine, in local time, as `YYYY-MM-DD`.
fn local_date() -> anyhow::Result<String> {
    let out = if cfg!(windows) {
        Command::new("powershell")
            .args(["-NoProfile", "-Command", "Get-Date -Format yyyy-MM-dd"])
            .output()
    } else {
        Command::new("date").arg("+%Y-%m-%d").output()
    }
    .context("reading the date from the machine")?;
    if !out.status.success() {
        bail!("reading the date from the machine failed");
    }
    let date = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    parse_date(&date).with_context(|| format!("the machine gave the date {date:?}"))?;
    Ok(date)
}

/// Splits `YYYY-MM-DD` into year, month, and day, checking the day exists.
fn parse_date(s: &str) -> Option<(i64, u32, u32)> {
    let mut parts = s.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return None;
    }
    let (y, m, d): (i64, u32, u32) = (y.parse().ok()?, m.parse().ok()?, d.parse().ok()?);
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    ((1..=12).contains(&m) && d >= 1 && d <= days[(m - 1) as usize]).then_some((y, m, d))
}

/// Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(m);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The weekday, computed (1970-01-01 was a Thursday).
fn weekday(y: i64, m: u32, d: u32) -> &'static str {
    const NAMES: [&str; 7] = [
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
    ];
    NAMES[days_from_civil(y, m, d).rem_euclid(7) as usize]
}

/// "Saturday, September 26, 2026".
fn spoken_date(y: i64, m: u32, d: u32) -> String {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    format!(
        "{}, {} {d}, {y}",
        weekday(y, m, d),
        MONTHS[(m - 1) as usize]
    )
}

fn git_output(root: &Path, args: &[&str]) -> anyhow::Result<String> {
    let out = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .context("running git")?;
    if !out.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn git(root: &Path, args: &[&str]) -> anyhow::Result<()> {
    let status = Command::new("git")
        .current_dir(root)
        .args(args)
        .status()
        .context("running git")?;
    if !status.success() {
        bail!("git {} failed ({status})", args.join(" "));
    }
    Ok(())
}

fn cargo(root: &Path, args: &[&str]) -> anyhow::Result<()> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    println!("cargo {}", args.join(" "));
    let status = Command::new(cargo)
        .current_dir(root)
        .args(args)
        .status()
        .context("running cargo")?;
    if !status.success() {
        bail!("cargo {} failed ({status})", args.join(" "));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments() {
        let a = parse(&["v0.2.0".into(), "--dry-run".into()]).unwrap();
        assert_eq!(
            a,
            Args {
                version: "0.2.0".into(),
                dry_run: true,
                checks: true,
                listened: false,
            }
        );
        assert!(
            parse(&["0.2.0".into(), "--listened".into()])
                .unwrap()
                .listened
        );
        assert!(parse(&["0.2.0".into(), "--listened".into(), "--dry-run".into()]).is_err());
        assert!(
            !parse(&["0.2.0".into(), "--no-checks".into()])
                .unwrap()
                .checks
        );
        assert!(parse(&[]).is_err());
        assert!(parse(&["0.2".into()]).is_err());
        assert!(parse(&["0.2.0".into(), "0.3.0".into()]).is_err());
        assert!(parse(&["0.2.0".into(), "--push".into()]).is_err());
    }

    #[test]
    fn versions() {
        for ok in ["0.1.0", "1.20.3", "0.1.0-alpha.4", "2.0.0-rc.1"] {
            assert!(valid_version(ok), "{ok}");
        }
        for bad in [
            "0.1",
            "01.1.0",
            "0.1.0-",
            "0.1.0-alpha..1",
            "a.b.c",
            "0.1.0 ",
        ] {
            assert!(!valid_version(bad), "{bad}");
        }
    }

    #[test]
    fn weekdays_are_computed() {
        // The dates the global date rule was written about.
        assert_eq!(spoken_date(2026, 9, 24), "Thursday, September 24, 2026");
        assert_eq!(spoken_date(2026, 9, 26), "Saturday, September 26, 2026");
        assert_eq!(weekday(1970, 1, 1), "Thursday");
        assert_eq!(weekday(2000, 2, 29), "Tuesday");
        assert_eq!(weekday(2024, 12, 31), "Tuesday");
    }

    #[test]
    fn dates_are_checked() {
        assert_eq!(parse_date("2026-09-26"), Some((2026, 9, 26)));
        assert_eq!(parse_date("2028-02-29"), Some((2028, 2, 29)));
        assert_eq!(parse_date("2026-02-29"), None);
        assert_eq!(parse_date("2026-9-26"), None);
        assert_eq!(parse_date("2026-13-01"), None);
        assert_eq!(parse_date("26/09/2026"), None);
    }

    #[test]
    fn the_machine_date_parses() {
        let d = local_date().unwrap();
        assert!(parse_date(&d).is_some(), "{d}");
    }

    #[test]
    fn workspace_version_is_set() {
        let m = "[package]\nversion = \"9.9.9\"\n\n[workspace.package]\nversion = \"0.1.0-alpha.3\"\nedition = \"2024\"\n\n[x]\nversion = \"1\"\n";
        let out = set_workspace_version(m, "0.2.0").unwrap();
        assert_eq!(
            out,
            "[package]\nversion = \"9.9.9\"\n\n[workspace.package]\nversion = \"0.2.0\"\nedition = \"2024\"\n\n[x]\nversion = \"1\"\n"
        );
        assert!(set_workspace_version("[workspace]\n", "0.2.0").is_err());
    }

    #[test]
    fn changelog_is_dated() {
        let text = "# Changelog\n\nIntro.\n\n## [Unreleased]\n\n### Added\n\n- Things.\n\n## [0.1.0] - 2026-09-25\n\n- Old.\n\n[0.1.0]: https://example/releases/tag/v0.1.0\n";
        let out = date_changelog(text, "0.2.0", "2026-09-26").unwrap();
        assert_eq!(
            out,
            format!(
                "# Changelog\n\nIntro.\n\n## [Unreleased]\n\n## [0.2.0] - 2026-09-26\n\n### Added\n\n- Things.\n\n## [0.1.0] - 2026-09-25\n\n- Old.\n\n[0.2.0]: {REPO_URL}/releases/tag/v0.2.0\n[0.1.0]: https://example/releases/tag/v0.1.0\n"
            )
        );
        // Empty, missing, and repeated sections are refused.
        assert!(date_changelog("## [Unreleased]\n\n## [0.1.0]\n", "0.2.0", "2026-09-26").is_err());
        assert!(date_changelog("# Changelog\n", "0.2.0", "2026-09-26").is_err());
        assert!(date_changelog(text, "0.1.0", "2026-09-26").is_err());
    }

    #[test]
    fn the_listening_check_is_dated_recent_and_for_this_version() {
        let today = (2026, 9, 28);
        let line = |rest: &str| format!("# Guide\n\n{LISTENED} {rest}\n\nMore.\n");
        let ok = line("2026-09-27 (Sunday, September 27, 2026), for 0.1.0-alpha.4.");
        assert_eq!(check_listening(&ok, "0.1.0-alpha.4", today), Ok(()));
        // Not dated, missing, from the future, too old, another version.
        let e = check_listening(&line("not yet recorded."), "0.1.0-alpha.4", today).unwrap_err();
        assert!(e.starts_with("listening check not dated"), "{e}");
        assert!(e.contains("--listened"), "{e}");
        let e = check_listening("# Guide\n", "0.1.0-alpha.4", today).unwrap_err();
        assert!(e.starts_with("listening check missing"), "{e}");
        let e = check_listening(
            &line("2026-09-29, for 0.1.0-alpha.4."),
            "0.1.0-alpha.4",
            today,
        )
        .unwrap_err();
        assert!(e.contains("future"), "{e}");
        let e = check_listening(
            &line("2026-09-13, for 0.1.0-alpha.4."),
            "0.1.0-alpha.4",
            today,
        )
        .unwrap_err();
        assert!(e.starts_with("listening check is 15 days old"), "{e}");
        assert_eq!(
            check_listening(
                &line("2026-09-14, for 0.1.0-alpha.4."),
                "0.1.0-alpha.4",
                today
            ),
            Ok(())
        );
        let e = check_listening(
            &line("2026-09-27, for 0.1.0-alpha.3."),
            "0.1.0-alpha.4",
            today,
        )
        .unwrap_err();
        assert!(
            e.starts_with("listening check was for 0.1.0-alpha.3"),
            "{e}"
        );
    }

    #[test]
    fn recording_the_listening_check_writes_the_machine_date() {
        let guide = format!("# Guide\r\n\r\n{LISTENED} not yet recorded.\r\n\r\nMore.\r\n");
        let out = record_listening(
            &guide,
            "0.1.0-alpha.4",
            "2026-09-28",
            "Monday, September 28, 2026",
        )
        .unwrap();
        assert_eq!(
            out,
            format!(
                "# Guide\r\n\r\n{LISTENED} 2026-09-28 (Monday, September 28, 2026), for 0.1.0-alpha.4.\r\n\r\nMore.\r\n"
            )
        );
        assert_eq!(
            check_listening(&out, "0.1.0-alpha.4", (2026, 9, 28)),
            Ok(())
        );
        assert!(record_listening("# Guide\n", "0.1.0-alpha.4", "2026-09-28", "x").is_err());
    }

    #[test]
    fn the_real_guide_has_the_listening_line() {
        let guide = fs::read_to_string(crate::eci::root().join(GUIDE)).unwrap();
        let e = check_listening(&guide, "99.0.0", (2026, 9, 28)).unwrap_err();
        assert!(!e.starts_with("listening check missing"), "{e}");
    }

    #[test]
    fn agents_headings_are_found_in_unreleased_only() {
        let text = "# Changelog\n\n## [Unreleased]\n\n### Reading and speech\n\n- A.\n\n### W4a3: GUI edit mode\n\n- B.\n\n### Terminal polish (Wave 4, Agent W4h)\n\n### The Cloud Agent (pull request 1)\n\n### Windows\n\n## [0.1.0] - 2026-09-25\n\n### W3a: old\n";
        assert_eq!(
            agent_headings(text),
            [
                "W4a3: GUI edit mode",
                "Terminal polish (Wave 4, Agent W4h)",
                "The Cloud Agent (pull request 1)"
            ]
        );
        assert!(agent_headings("## [Unreleased]\n\n### Added\n").is_empty());
    }

    #[test]
    fn the_real_changelog_can_be_dated() {
        let text = fs::read_to_string(crate::eci::root().join("CHANGELOG.md")).unwrap();
        // Right after a release, `[Unreleased]` is empty on purpose, and
        // `cargo xtask release` runs this very test in its checks; dating
        // an empty section is refused, as it should be.
        let out = match date_changelog(&text, "99.0.0", "2026-09-26") {
            Ok(out) => out,
            Err(e) if e.to_string().contains("is empty") => return,
            Err(e) => panic!("{e}"),
        };
        assert!(out.contains("## [Unreleased]\n\n## [99.0.0] - 2026-09-26\n"));
        assert!(out.contains("[99.0.0]: "));
    }
}
