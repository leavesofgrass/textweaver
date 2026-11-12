//! `cargo xtask tools [--check]`: the tool versions the checks rely on,
//! pinned in one place, `tools.toml` at the repository root.
//!
//! Without `--check` it lists each pin and the version installed here.
//! With `--check` it fails when an installed tool differs from its pin, in
//! words ("cargo-about is 0.9.2; the repository pins 0.9.3"), or when a
//! required tool is missing. A missing optional tool is a note, not a
//! failure.
//!
//! `tools.toml` is a small subset of TOML, read here without a TOML crate
//! (xtask's dependencies stay as they are): one `[name]` table per tool,
//! with `version` and `command` strings and an optional `required`
//! boolean, and `#` comments.

use std::process::Command;

use anyhow::{Context, bail};

/// The usage line.
const USAGE: &str = "usage: cargo xtask tools [--check]";

/// One pinned tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    /// Its name, such as `cargo-about`.
    pub name: String,
    /// The pinned version.
    pub version: String,
    /// The command that prints its version, split at spaces.
    pub command: Vec<String>,
    /// A missing required tool fails the check.
    pub required: bool,
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
    let path = crate::eci::root().join("tools.toml");
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let tools = parse(&text)?;
    let mut problems = 0;
    for t in &tools {
        let installed = installed_version(&t.command);
        let (line, bad) = verdict(t, installed.as_deref());
        println!("{line}");
        if bad {
            problems += 1;
        }
    }
    if check && problems > 0 {
        bail!(
            "{problems} of {} tools do not match tools.toml.",
            tools.len()
        );
    }
    if check {
        println!("Every installed tool matches tools.toml.");
    }
    Ok(())
}

/// One line about a tool, meaning first, and whether it fails the check.
pub fn verdict(t: &Tool, installed: Option<&str>) -> (String, bool) {
    match installed {
        Some(v) if v == t.version => (format!("Pass: {} is {v}, as pinned.", t.name), false),
        Some(v) => (
            format!(
                "Fail: {} is {v}; the repository pins {}.",
                t.name, t.version
            ),
            true,
        ),
        None if t.required => (
            format!(
                "Fail: {} is not installed; the repository pins {} and the checks need it.",
                t.name, t.version
            ),
            true,
        ),
        None => (
            format!(
                "Note: {} is not installed (optional; the repository pins {}).",
                t.name, t.version
            ),
            false,
        ),
    }
}

/// Runs `command` and reads the first version number it prints.
fn installed_version(command: &[String]) -> Option<String> {
    let (exe, args) = command.split_first()?;
    let out = Command::new(exe).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push('\n');
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    first_version(&text)
}

/// The first `N.N` or `N.N.N` (and so on) in `text`, without a leading
/// `v`.
pub fn first_version(text: &str) -> Option<String> {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let starts = b[i].is_ascii_digit() && (i == 0 || !b[i - 1].is_ascii_alphanumeric());
        if starts {
            let mut j = i;
            while j < b.len() && (b[j].is_ascii_digit() || b[j] == b'.') {
                j += 1;
            }
            let v = text[i..j].trim_end_matches('.');
            if v.contains('.') {
                return Some(v.to_owned());
            }
            i = j;
        } else {
            i += 1;
        }
    }
    None
}

/// Reads `tools.toml`.
pub fn parse(text: &str) -> anyhow::Result<Vec<Tool>> {
    let mut tools: Vec<Tool> = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let at = || format!("tools.toml line {}", n + 1);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            tools.push(Tool {
                name: name.trim().to_owned(),
                version: String::new(),
                command: Vec::new(),
                required: false,
            });
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .with_context(|| format!("{}: expected key = value", at()))?;
        let tool = tools
            .last_mut()
            .with_context(|| format!("{}: a key before the first [tool]", at()))?;
        let value = value.trim();
        let string = || -> anyhow::Result<String> {
            value
                .strip_prefix('"')
                .and_then(|v| v.split_once('"'))
                .map(|(v, _)| v.to_owned())
                .with_context(|| format!("{}: expected a quoted string", at()))
        };
        match key.trim() {
            "version" => tool.version = string()?,
            "command" => tool.command = string()?.split_whitespace().map(str::to_owned).collect(),
            "required" => {
                tool.required = match value.split('#').next().unwrap_or("").trim() {
                    "true" => true,
                    "false" => false,
                    _ => bail!("{}: required is true or false", at()),
                }
            }
            other => bail!("{}: unknown key {other}", at()),
        }
    }
    for t in &tools {
        if t.version.is_empty() || t.command.is_empty() {
            bail!("tools.toml: {} needs a version and a command", t.name);
        }
    }
    Ok(tools)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
# A comment.
[cargo-about]
version = "0.9.3"
command = "cargo about --version"
required = true # the notices

[actionlint]
version = "1.7.12"
command = "actionlint --version"
"#;

    #[test]
    fn the_sample_parses() {
        let t = parse(SAMPLE).unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].name, "cargo-about");
        assert_eq!(t[0].command, ["cargo", "about", "--version"]);
        assert!(t[0].required);
        assert!(!t[1].required);
    }

    #[test]
    fn a_tool_without_a_version_is_refused() {
        assert!(parse("[x]\ncommand = \"x --version\"\n").is_err());
        assert!(parse("version = \"1.0\"\n").is_err());
        assert!(parse("[x]\nversion = 1.0\ncommand = \"x\"\n").is_err());
    }

    #[test]
    fn versions_are_read_from_tool_output() {
        assert_eq!(first_version("cargo-about 0.9.3").as_deref(), Some("0.9.3"));
        assert_eq!(
            first_version("cargo-nextest 0.9.148 (cd1d6d546 2026-10-07)\nrelease: 0.9.148")
                .as_deref(),
            Some("0.9.148")
        );
        assert_eq!(
            first_version("ShellCheck - shell script analysis tool\nversion: 0.11.0").as_deref(),
            Some("0.11.0")
        );
        assert_eq!(first_version("1.7.12\n").as_deref(), Some("1.7.12"));
        assert_eq!(first_version("x86_64 v2").as_deref(), None);
    }

    #[test]
    fn verdicts_say_the_fact_first() {
        let t = parse(SAMPLE).unwrap();
        assert_eq!(
            verdict(&t[0], Some("0.9.2")),
            (
                "Fail: cargo-about is 0.9.2; the repository pins 0.9.3.".to_owned(),
                true
            )
        );
        assert!(!verdict(&t[0], Some("0.9.3")).1);
        assert!(verdict(&t[0], None).1, "a required tool");
        let (line, bad) = verdict(&t[1], None);
        assert!(!bad && line.starts_with("Note:"), "{line}");
    }

    #[test]
    fn the_repository_file_parses() {
        let text = std::fs::read_to_string(crate::eci::root().join("tools.toml")).unwrap();
        let tools = parse(&text).unwrap();
        let about = tools.iter().find(|t| t.name == "cargo-about").unwrap();
        assert_eq!(about.version, "0.9.3");
        assert!(about.required);
    }

    #[test]
    fn the_other_cargo_about_pins_match() {
        // These name the version too (see the top of tools.toml).
        let text = std::fs::read_to_string(crate::eci::root().join("tools.toml")).unwrap();
        let about = parse(&text)
            .unwrap()
            .into_iter()
            .find(|t| t.name == "cargo-about")
            .unwrap();
        let pin = format!("cargo-about@{}", about.version);
        for file in [
            "xtask/src/notices.rs",
            ".github/workflows/ci.yml",
            "docs/dev/releasing.md",
        ] {
            let text = std::fs::read_to_string(crate::eci::root().join(file)).unwrap();
            assert!(
                text.contains(&pin),
                "{file} and tools.toml disagree on cargo-about"
            );
        }
    }
}
