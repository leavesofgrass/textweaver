//! `cargo xtask sentinel record FILE` and `cargo xtask sentinel check FILE`:
//! proof that no test touched the real data folders.
//!
//! A test that reads or writes the user's own settings, state, or cache is
//! a test that can clobber them (star's and abax's tests both did). Tests
//! use a temporary folder (`Paths::under`, or `TEXTWEAVER_HOME` set to a
//! temporary folder for a child process). CI checks that rule around the
//! test run:
//!
//! 1. `record` lists every file under the watched folders, with its size
//!    and modification time, and writes the list to FILE.
//! 2. The tests run.
//! 3. `check` lists them again and compares. Any file added, removed, or
//!    changed fails the check, one line each, meaning first.
//!
//! The watched folders are textweaver's platform folders, exactly as
//! `textweaver-store` places them (the `directories` crate with the same
//! qualifier, organization, and application name), and the folder named by
//! `TEXTWEAVER_HOME` when it is set. CI sets `TEXTWEAVER_HOME` to an empty
//! sentinel folder for the test step, so a test that uses the default
//! folders without its own temporary one writes there, and is caught,
//! instead of writing to the runner's real profile. A test that sets its
//! own `TEXTWEAVER_HOME` for a child process is not affected.
//!
//! On a developer's machine the real folders may change while the tests
//! run, because textweaver itself is open; the check is meant for CI.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::{Context, bail};

/// The usage line.
const USAGE: &str = "usage: cargo xtask sentinel record FILE | cargo xtask sentinel check FILE";

/// One file's size and modification time, in nanoseconds since 1970.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp {
    /// Size in bytes.
    pub len: u64,
    /// Modification time.
    pub modified_ns: u128,
}

/// Every watched file, by full path. A watched folder that exists but is
/// empty is listed with a trailing separator and a zero stamp, so creating
/// a folder counts as a change too.
pub type Listing = BTreeMap<String, Stamp>;

/// The folders the check watches, each once.
pub fn watched() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    if let Some(dirs) = directories::ProjectDirs::from("org", "leavesofgrass", "textweaver") {
        for d in [
            dirs.config_dir(),
            dirs.data_dir(),
            dirs.data_local_dir(),
            dirs.cache_dir(),
            dirs.preference_dir(),
        ] {
            out.push(d.to_owned());
        }
    }
    if let Some(home) = std::env::var_os("TEXTWEAVER_HOME").filter(|v| !v.is_empty()) {
        out.push(PathBuf::from(home));
    }
    out.sort();
    out.dedup();
    out
}

/// Lists every file under `folders` (a missing folder lists nothing).
pub fn list(folders: &[PathBuf]) -> anyhow::Result<Listing> {
    let mut listing = Listing::new();
    for f in folders {
        walk(f, &mut listing)?;
    }
    Ok(listing)
}

fn walk(dir: &Path, listing: &mut Listing) -> anyhow::Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e).with_context(|| format!("listing {}", dir.display())),
    };
    let mut any = false;
    for entry in entries {
        any = true;
        let entry = entry.with_context(|| format!("listing {}", dir.display()))?;
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        if meta.is_dir() {
            walk(&path, listing)?;
        } else {
            let modified_ns = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_nanos());
            listing.insert(
                path.display().to_string(),
                Stamp {
                    len: meta.len(),
                    modified_ns,
                },
            );
        }
    }
    if !any {
        listing.insert(
            format!("{}{}", dir.display(), std::path::MAIN_SEPARATOR),
            Stamp {
                len: 0,
                modified_ns: 0,
            },
        );
    }
    Ok(())
}

/// The listing as text: one line per file, `len modified_ns path`.
pub fn to_text(folders: &[PathBuf], listing: &Listing) -> String {
    let mut out = String::new();
    for f in folders {
        let _ = writeln!(out, "folder {}", f.display());
    }
    for (path, s) in listing {
        let _ = writeln!(out, "file {} {} {path}", s.len, s.modified_ns);
    }
    out
}

/// Reads [`to_text`]'s output back.
pub fn from_text(text: &str) -> anyhow::Result<(Vec<PathBuf>, Listing)> {
    let mut folders = Vec::new();
    let mut listing = Listing::new();
    for (n, line) in text.lines().enumerate() {
        if let Some(f) = line.strip_prefix("folder ") {
            folders.push(PathBuf::from(f));
        } else if let Some(rest) = line.strip_prefix("file ") {
            let mut parts = rest.splitn(3, ' ');
            let (Some(len), Some(ns), Some(path)) = (parts.next(), parts.next(), parts.next())
            else {
                bail!("line {} of the record is not a file line", n + 1);
            };
            listing.insert(
                path.to_owned(),
                Stamp {
                    len: len.parse().context("a file size")?,
                    modified_ns: ns.parse().context("a modification time")?,
                },
            );
        } else if !line.trim().is_empty() {
            bail!("line {} of the record is not understood", n + 1);
        }
    }
    Ok((folders, listing))
}

/// What changed between two listings, one line each, meaning first.
pub fn changes(before: &Listing, after: &Listing) -> Vec<String> {
    let mut out = Vec::new();
    for (path, s) in after {
        match before.get(path) {
            None => out.push(format!("Added: {path}")),
            Some(b) if b != s => out.push(format!("Changed: {path}")),
            Some(_) => {}
        }
    }
    for path in before.keys() {
        if !after.contains_key(path) {
            out.push(format!("Removed: {path}"));
        }
    }
    out
}

/// `cargo xtask sentinel record|check FILE`.
pub fn run() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let [verb, file] = args.as_slice() else {
        bail!("{USAGE}");
    };
    let file = PathBuf::from(file);
    match verb.as_str() {
        "record" => {
            let folders = watched();
            let listing = list(&folders)?;
            std::fs::write(&file, to_text(&folders, &listing))
                .with_context(|| format!("writing {}", file.display()))?;
            println!(
                "Recorded: {} files in {} watched folders",
                listing.len(),
                folders.len()
            );
            for f in &folders {
                println!("Watching: {}", f.display());
            }
            Ok(())
        }
        "check" => {
            let text = std::fs::read_to_string(&file)
                .with_context(|| format!("reading {} (run record first)", file.display()))?;
            let (folders, before) = from_text(&text)?;
            if folders.is_empty() {
                bail!("the record names no folder to watch");
            }
            let after = list(&folders)?;
            let changed = changes(&before, &after);
            if changed.is_empty() {
                println!(
                    "Pass: no test touched the real data folders ({} watched)",
                    folders.len()
                );
                for f in &folders {
                    println!("Watched: {}", f.display());
                }
                return Ok(());
            }
            println!(
                "Fail: the tests changed {} files in the real data folders",
                changed.len()
            );
            for line in &changed {
                println!("{line}");
            }
            std::process::exit(1);
        }
        _ => bail!("{USAGE}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp(len: u64, modified_ns: u128) -> Stamp {
        Stamp { len, modified_ns }
    }

    #[test]
    fn the_record_reads_back() {
        let folders = vec![PathBuf::from("/x/config"), PathBuf::from("/x/data dir")];
        let mut listing = Listing::new();
        listing.insert("/x/config/settings.toml".into(), stamp(12, 34));
        listing.insert("/x/data dir/a b.json".into(), stamp(0, 1));
        let text = to_text(&folders, &listing);
        let (f, l) = from_text(&text).unwrap();
        assert_eq!(f, folders);
        assert_eq!(l, listing);
        assert!(from_text("nonsense").is_err());
        assert!(from_text("file 1 2").is_err());
    }

    #[test]
    fn every_change_is_named_meaning_first() {
        let mut before = Listing::new();
        before.insert("a".into(), stamp(1, 1));
        before.insert("b".into(), stamp(1, 1));
        before.insert("c".into(), stamp(1, 1));
        let mut after = before.clone();
        assert!(changes(&before, &after).is_empty());
        after.remove("a");
        after.insert("b".into(), stamp(2, 1));
        after.insert("c".into(), stamp(1, 2));
        after.insert("d".into(), stamp(0, 0));
        assert_eq!(
            changes(&before, &after),
            ["Changed: b", "Changed: c", "Added: d", "Removed: a"]
        );
    }

    #[test]
    fn a_write_into_a_watched_folder_is_caught() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let folders = vec![home.clone()];
        // A missing folder lists nothing; creating it empty is a change.
        let before = list(&folders).unwrap();
        assert!(before.is_empty());
        std::fs::create_dir_all(home.join("config")).unwrap();
        let empty = list(&folders).unwrap();
        assert_eq!(changes(&before, &empty).len(), 1, "{empty:?}");
        // A file written by a test is caught.
        std::fs::write(home.join("config").join("settings.toml"), "x = 1\n").unwrap();
        let after = list(&folders).unwrap();
        let c = changes(&empty, &after);
        assert!(c.iter().any(|l| l.starts_with("Added: ")), "{c:?}");
        assert!(c.iter().any(|l| l.starts_with("Removed: ")), "{c:?}");
    }

    #[test]
    fn the_folders_are_textweavers_own() {
        let w = watched();
        assert!(
            w.iter().all(|p| p.to_string_lossy().contains("textweaver")
                || std::env::var_os("TEXTWEAVER_HOME").is_some_and(|h| p == Path::new(&h))),
            "{w:?}"
        );
    }
}
