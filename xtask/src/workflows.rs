//! Rules for the CI workflows themselves, checked by `cargo test -p xtask`.
//!
//! A report-only step (`continue-on-error: true`) is a gate that cannot
//! fail. Each one must say, in a comment just above it, why it is
//! report-only and when that is reviewed (a line with `Review:`), so none
//! stays report-only by accident (docs/dev/testing.md, "Gates and what
//! breaks them").

use std::path::Path;

/// How many lines above `continue-on-error` the comment may start.
const WINDOW: usize = 8;

/// The 1-based line numbers of report-only steps in `text` with no
/// `Report-only` comment and no `Review:` line in the [`WINDOW`] lines
/// above them.
pub fn unexplained_report_only(text: &str) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if !t.starts_with("continue-on-error:") || t.ends_with("false") {
            continue;
        }
        let above = &lines[i.saturating_sub(WINDOW)..i];
        let says_why = above
            .iter()
            .any(|l| l.trim_start().starts_with('#') && l.contains("Report-only"));
        let says_when = above
            .iter()
            .any(|l| l.trim_start().starts_with('#') && l.contains("Review:"));
        if !(says_why && says_when) {
            out.push(i + 1);
        }
    }
    out
}

/// Every workflow file, by name, with its text.
pub fn workflows(root: &Path) -> anyhow::Result<Vec<(String, String)>> {
    let dir = root.join(".github").join("workflows");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "yml" || e == "yaml") {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            out.push((name, std::fs::read_to_string(&path)?));
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_report_only_step_says_why_and_when_it_is_reviewed() {
        let root = crate::eci::root();
        let mut problems = Vec::new();
        let mut count = 0;
        for (name, text) in workflows(&root).unwrap() {
            count += text
                .lines()
                .filter(|l| l.trim() == "continue-on-error: true")
                .count();
            for n in unexplained_report_only(&text) {
                problems.push(format!(
                    "Fail: {name} line {n} is report-only (continue-on-error) with no \
                     Report-only comment and Review: line just above it"
                ));
            }
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
        // The workflows were read: a path mistake must not pass as "none".
        assert!(
            count > 0,
            "no report-only step found; was .github/workflows read?"
        );
    }

    #[test]
    fn a_bare_report_only_step_is_caught() {
        let bare = "steps:\n  - name: x\n    continue-on-error: true\n";
        assert_eq!(unexplained_report_only(bare), [3]);
        let why_only = "  # Report-only. Why: flaky.\n  continue-on-error: true\n";
        assert_eq!(unexplained_report_only(why_only), [2]);
        let explained =
            "  # Report-only. Why: flaky.\n  # Review: at alpha.9.\n  continue-on-error: true\n";
        assert!(unexplained_report_only(explained).is_empty());
        let off = "  continue-on-error: false\n";
        assert!(unexplained_report_only(off).is_empty());
    }
}
