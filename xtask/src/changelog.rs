//! `cargo xtask changelog [--check]`: folds the changelog fragments in
//! `changelog.d/` into the `[Unreleased]` section of `CHANGELOG.md`.
//!
//! Each fragment (`changelog.d/<branch>.md`, see `changelog.d/README.md`)
//! is one area heading line, such as `### Security`, and plain bullets.
//! Fragments are folded in file-name order. A fragment whose area already
//! has a heading under `[Unreleased]` (compared without case or
//! surrounding spaces) adds its bullets at the end of that heading's list;
//! any other area gets a new heading at the end of the section. The folded
//! fragments are then removed with `git rm`, so the removal is part of the
//! commit that folds them. A fragment must be committed and unchanged
//! first; otherwise nothing is written.
//!
//! With `--check` it reads every fragment, says where each would go, and
//! changes nothing. A fragment with no heading or no bullets fails either
//! way, by name.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};

/// The usage line.
const USAGE: &str = "usage: cargo xtask changelog [--check]";

/// The section fragments fold into.
const UNRELEASED: &str = "## [Unreleased]";

/// One fragment: its area heading and its lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fragment {
    /// The area, such as `Security`.
    pub area: String,
    /// The bullets, as written, without leading or trailing blank lines.
    pub body: Vec<String>,
}

pub fn run() -> anyhow::Result<()> {
    let mut check = false;
    for a in std::env::args().skip(2) {
        match a.as_str() {
            "--check" => check = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            other => bail!("unknown option {other}. {USAGE}"),
        }
    }
    let root = crate::eci::root();
    let paths = fragment_paths(&root)?;
    if paths.is_empty() {
        println!("No changelog fragments to fold.");
        return Ok(());
    }
    let mut fragments = Vec::new();
    for p in &paths {
        let text =
            std::fs::read_to_string(p).with_context(|| format!("reading {}", rel(&root, p)))?;
        let f = parse_fragment(&text).with_context(|| rel(&root, p))?;
        fragments.push(f);
    }
    let changelog_path = root.join("CHANGELOG.md");
    let changelog = std::fs::read_to_string(&changelog_path).context("reading CHANGELOG.md")?;
    for (p, f) in paths.iter().zip(&fragments) {
        let how = if has_area(&changelog, &f.area) {
            "into its existing heading"
        } else {
            "as a new heading"
        };
        let verb = if check { "Would fold" } else { "Folding" };
        println!("{verb} {} under {} {how}.", rel(&root, p), f.area);
    }
    let folded = fold(&changelog, &fragments)?;
    if check {
        println!(
            "Check only: {} fragments read, nothing changed.",
            fragments.len()
        );
        return Ok(());
    }
    let names: Vec<String> = paths.iter().map(|p| rel(&root, p)).collect();
    let mut status = vec!["status", "--porcelain", "--"];
    status.extend(names.iter().map(String::as_str));
    let dirty = git(&root, &status)?;
    if !dirty.trim().is_empty() {
        bail!(
            "commit the fragments first; git reports them as new or changed:\n{}",
            dirty.trim_end()
        );
    }
    std::fs::write(&changelog_path, folded).context("writing CHANGELOG.md")?;
    let mut rm = vec!["rm", "--quiet", "--"];
    rm.extend(names.iter().map(String::as_str));
    git(&root, &rm)?;
    println!(
        "Folded {} fragments into CHANGELOG.md and removed them with git rm. Review and commit both.",
        fragments.len()
    );
    Ok(())
}

/// A path relative to the root, with forward slashes, for messages.
fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Runs git in `root`, failing with its message.
fn git(root: &Path, args: &[&str]) -> anyhow::Result<String> {
    let out = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .with_context(|| format!("running git {}", args.join(" ")))?;
    if !out.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The fragments: every `.md` file in `changelog.d/` except the README, in
/// name order.
fn fragment_paths(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let dir = root.join("changelog.d");
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).context("listing changelog.d")? {
        let path = entry?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_file() && is_fragment_name(name) {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// A fragment's file name: `.md`, and not the README.
pub fn is_fragment_name(name: &str) -> bool {
    name.ends_with(".md") && !name.eq_ignore_ascii_case("README.md")
}

/// Reads a fragment: the first non-blank line is the heading (`### Area`;
/// any number of `#` is accepted), the rest are its lines.
pub fn parse_fragment(text: &str) -> anyhow::Result<Fragment> {
    let mut lines = text.lines().map(|l| l.trim_end());
    let heading = lines
        .by_ref()
        .find(|l| !l.trim().is_empty())
        .context("the fragment is empty; it needs a heading line such as ### Security")?;
    let area = heading.trim_start_matches('#');
    if area.len() == heading.len() || !area.starts_with(' ') || area.trim().is_empty() {
        bail!("the first line is not a heading; start with an area heading such as ### Security");
    }
    let mut body: Vec<String> = lines.map(str::to_owned).collect();
    while body.first().is_some_and(|l| l.trim().is_empty()) {
        body.remove(0);
    }
    while body.last().is_some_and(|l| l.trim().is_empty()) {
        body.pop();
    }
    if body.is_empty() {
        bail!("the fragment has a heading but no bullets");
    }
    if let Some(l) = body.iter().find(|l| l.starts_with('#')) {
        bail!("one area per fragment; a second heading was found: {l}");
    }
    Ok(Fragment {
        area: area.trim().to_owned(),
        body,
    })
}

/// The line range of `[Unreleased]`'s body: after its heading, up to the
/// next version heading.
fn unreleased(lines: &[String]) -> anyhow::Result<(usize, usize)> {
    let start = lines
        .iter()
        .position(|l| l.trim_end() == UNRELEASED)
        .context("CHANGELOG.md has no ## [Unreleased] section")?;
    let end = lines[start + 1..]
        .iter()
        .position(|l| l.starts_with("## "))
        .map_or(lines.len(), |i| start + 1 + i);
    Ok((start, end))
}

/// The `### ` heading of `area` in `lines[from..to]`.
fn find_area(lines: &[String], from: usize, to: usize, area: &str) -> Option<usize> {
    let want = area.trim().to_lowercase();
    (from..to).find(|&i| {
        lines[i]
            .strip_prefix("### ")
            .is_some_and(|h| h.trim().to_lowercase() == want)
    })
}

/// Whether `[Unreleased]` already has a heading for `area`.
fn has_area(changelog: &str, area: &str) -> bool {
    let lines: Vec<String> = changelog.lines().map(str::to_owned).collect();
    unreleased(&lines).is_ok_and(|(s, e)| find_area(&lines, s + 1, e, area).is_some())
}

/// Inserts `block` at `at`, then a blank line if the next line has text.
fn insert(lines: &mut Vec<String>, at: usize, block: Vec<String>) {
    let after = at + block.len();
    lines.splice(at..at, block);
    if lines.get(after).is_some_and(|l| !l.trim().is_empty()) {
        lines.insert(after, String::new());
    }
}

/// `changelog` with every fragment folded into `[Unreleased]`.
pub fn fold(changelog: &str, fragments: &[Fragment]) -> anyhow::Result<String> {
    let newline = if changelog.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut lines: Vec<String> = changelog.lines().map(str::to_owned).collect();
    for f in fragments {
        let (start, end) = unreleased(&lines)?;
        if let Some(h) = find_area(&lines, start + 1, end, &f.area) {
            let sub_end = (h + 1..end)
                .find(|&i| lines[i].starts_with("### "))
                .unwrap_or(end);
            let mut at = sub_end;
            while at > h + 1 && lines[at - 1].trim().is_empty() {
                at -= 1;
            }
            let mut block = Vec::new();
            if at == h + 1 {
                block.push(String::new());
            }
            block.extend(f.body.iter().cloned());
            insert(&mut lines, at, block);
        } else {
            let mut at = end;
            while at > start + 1 && lines[at - 1].trim().is_empty() {
                at -= 1;
            }
            let mut block = vec![String::new(), format!("### {}", f.area), String::new()];
            block.extend(f.body.iter().cloned());
            insert(&mut lines, at, block);
        }
    }
    let mut out = lines.join(newline);
    if changelog.ends_with('\n') {
        out.push_str(newline);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frag(area: &str, body: &[&str]) -> Fragment {
        Fragment {
            area: area.to_owned(),
            body: body.iter().map(|b| (*b).to_owned()).collect(),
        }
    }

    const EMPTY: &str =
        "# Changelog\n\nIntro.\n\n## [Unreleased]\n\n## [0.1.0] - 2026-01-01\n\n### Old\n\n- old\n";

    #[test]
    fn a_fragment_parses() {
        let f = parse_fragment("\n### Security\n\n- one\n  more\n- two\n\n").unwrap();
        assert_eq!(f, frag("Security", &["- one", "  more", "- two"]));
        assert_eq!(parse_fragment("## Area\n- a\n").unwrap().area, "Area");
    }

    #[test]
    fn bad_fragments_are_refused_in_words() {
        for (text, why) in [
            ("", "empty"),
            ("- a bullet\n", "not a heading"),
            ("###\n- a\n", "not a heading"),
            ("### Area\n\n", "no bullets"),
            ("### A\n- a\n### B\n- b\n", "second heading"),
        ] {
            let e = format!("{:#}", parse_fragment(text).unwrap_err());
            assert!(e.contains(why), "{text:?}: {e}");
        }
    }

    #[test]
    fn the_readme_is_not_a_fragment() {
        assert!(!is_fragment_name("README.md"));
        assert!(is_fragment_name("b6-devloop.md"));
        assert!(!is_fragment_name("notes.txt"));
    }

    #[test]
    fn an_empty_section_gets_a_new_heading() {
        let out = fold(EMPTY, &[frag("Security", &["- a"])]).unwrap();
        assert_eq!(
            out,
            "# Changelog\n\nIntro.\n\n## [Unreleased]\n\n### Security\n\n- a\n\n## [0.1.0] - 2026-01-01\n\n### Old\n\n- old\n"
        );
    }

    #[test]
    fn a_matching_heading_is_merged_into() {
        let first = fold(
            EMPTY,
            &[frag("Security", &["- a"]), frag("Other", &["- o"])],
        )
        .unwrap();
        let out = fold(&first, &[frag("  security ", &["- b", "  more"])]).unwrap();
        assert_eq!(
            out,
            "# Changelog\n\nIntro.\n\n## [Unreleased]\n\n### Security\n\n- a\n- b\n  more\n\n### Other\n\n- o\n\n## [0.1.0] - 2026-01-01\n\n### Old\n\n- old\n"
        );
    }

    #[test]
    fn released_headings_are_left_alone() {
        // "Old" exists only under 0.1.0, so [Unreleased] gets its own.
        let out = fold(EMPTY, &[frag("Old", &["- new"])]).unwrap();
        assert!(
            out.contains("## [Unreleased]\n\n### Old\n\n- new\n\n## [0.1.0]"),
            "{out}"
        );
        assert!(out.ends_with("### Old\n\n- old\n"));
    }

    #[test]
    fn unreleased_last_in_the_file_works() {
        let out = fold("# C\n\n## [Unreleased]\n", &[frag("A", &["- a"])]).unwrap();
        assert_eq!(out, "# C\n\n## [Unreleased]\n\n### A\n\n- a\n");
    }

    #[test]
    fn crlf_is_kept() {
        let out = fold(&EMPTY.replace('\n', "\r\n"), &[frag("A", &["- a"])]).unwrap();
        assert!(out.contains("## [Unreleased]\r\n\r\n### A\r\n\r\n- a\r\n\r\n## [0.1.0]"));
        assert!(!out.replace("\r\n", "").contains('\n'));
    }

    #[test]
    fn no_unreleased_section_is_an_error() {
        assert!(fold("# C\n", &[frag("A", &["- a"])]).is_err());
    }

    #[test]
    fn the_result_passes_the_release_heading_checks() {
        let out = fold(
            EMPTY,
            &[frag("Security", &["- a"]), frag("security", &["- b"])],
        )
        .unwrap();
        assert_eq!(out.matches("### Security").count(), 1, "{out}");
    }

    #[test]
    fn the_repository_fragments_parse() {
        let root = crate::eci::root();
        for p in fragment_paths(&root).unwrap() {
            let text = std::fs::read_to_string(&p).unwrap();
            parse_fragment(&text).unwrap_or_else(|e| panic!("{}: {e:#}", p.display()));
        }
    }
}
