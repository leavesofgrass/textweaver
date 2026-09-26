//! Pandoc as a fallback for formats textweaver has no native reader for
//! (for example ODT, RTF, reStructuredText, Org, LaTeX, DocBook).
//!
//! Pandoc runs as a subprocess and writes Markdown to stdout, which is
//! decoded as UTF-8 explicitly (Star decoded it with the Windows ANSI code
//! page and corrupted non-ASCII text). The Markdown then goes through the
//! same path as a native Markdown source.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use textweaver_render::Flavor;

/// Extensions Pandoc reads that textweaver may lack a native loader for.
pub const EXTENSIONS: &[&str] = &[
    "docx",
    "odt",
    "rtf",
    "epub",
    "rst",
    "org",
    "tex",
    "latex",
    "ltx",
    "dbk",
    "docbook",
    "textile",
    "mediawiki",
    "wiki",
    "ipynb",
    "opml",
    "fb2",
    "typ",
    "djot",
    "muse",
    "t2t",
    "jira",
    "bib",
    "csv",
    "tsv",
];

/// True when a `pandoc` executable answers on PATH (checked once).
pub fn available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("pandoc")
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    })
}

/// Converts `path` to Markdown in `flavor` (GFM unless Pandoc's own
/// Markdown is asked for).
pub fn to_markdown(path: &Path, flavor: Flavor) -> Result<String, String> {
    let to = match flavor {
        Flavor::Pandoc => "markdown",
        Flavor::CommonMark => "commonmark",
        Flavor::Gfm | Flavor::Obsidian => "gfm",
    };
    let out = Command::new("pandoc")
        .arg("--to")
        .arg(to)
        .arg("--wrap=none")
        .arg("--")
        .arg(path)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("cannot run pandoc: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("pandoc failed: {}", err.trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}
