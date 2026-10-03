//! The package size budget (docs/dev/releasing.md, "Package sizes").
//!
//! [`FILE`] records the size of each package at the last release.
//! `cargo xtask dist`, `gui-dist`, and `appimage` call [`check`] on the
//! packages they write: a package more than [`MAX_GROWTH_PERCENT`] percent
//! over its recorded size fails the build, unless the file's `[notes]`
//! table says why. `cargo xtask release VERSION --sizes` rewrites the file
//! from the published release (see `release.rs`) and clears the notes.
//!
//! A package is named by its file name without `textweaver-VERSION-`
//! (`windows-x86_64.zip`, `linux-x86_64-gui.AppImage`), so the names stay
//! the same from one release to the next.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};

/// The committed size file, relative to the root.
pub(crate) const FILE: &str = "xtask/package-sizes.toml";
/// How much a package may grow over its recorded size without a note.
pub(crate) const MAX_GROWTH_PERCENT: u64 = 10;
/// The note key that covers every package.
const ALL: &str = "all";
/// The endings of package files; `.zsync` files and checksums are not
/// packages.
const PACKAGE_ENDINGS: [&str; 3] = [".zip", ".tar.gz", ".AppImage"];

/// The recorded sizes.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Recorded {
    /// The release the sizes are from.
    pub version: String,
    /// Package name to size in bytes.
    pub sizes: BTreeMap<String, u64>,
    /// Package name (or `all`) to why it may grow past the budget.
    pub notes: BTreeMap<String, String>,
}

/// The comment at the top of [`FILE`].
const HEADER: &str = "\
# The size of each release package, in bytes, at the last release.
#
# `cargo xtask dist`, `gui-dist`, and `appimage` compare each package they
# write with its line here, and fail when one grew more than 10 percent
# without a note below saying why. `cargo xtask release VERSION --sizes`
# rewrites this file from the published release and clears the notes
# (docs/dev/releasing.md, \"Package sizes\"). Written by that step; edit only
# the notes by hand.
";

/// The comment above the `[notes]` table.
const NOTES_HELP: &str = "\
# Why a package may grow more than 10 percent before the next release, one
# line per package: \"windows-x86_64.zip\" = \"why\". \"all\" covers every
# package.
";

/// A quoted TOML basic string's contents, with `\"` and `\\` unescaped.
fn unquote(s: &str) -> Option<String> {
    let inner = s.trim().strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                other => out.push(other),
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

/// `s` as a quoted TOML basic string.
fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Parses [`FILE`]: `version`, then the `[sizes]` and `[notes]` tables,
/// each line `"name" = value`. Comments and blank lines are skipped.
pub(crate) fn parse(text: &str) -> anyhow::Result<Recorded> {
    let mut out = Recorded::default();
    let mut table = "";
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = || format!("{FILE}, line {}", i + 1);
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            table = match name {
                "sizes" => "sizes",
                "notes" => "notes",
                other => bail!("{}: unknown table [{other}]", at()),
            };
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .with_context(|| format!("{}: expected name = value", at()))?;
        match table {
            "" if key.trim() == "version" => {
                out.version = unquote(value).with_context(|| format!("{}: version", at()))?;
            }
            "sizes" => {
                let key = unquote(key).with_context(|| format!("{}: package name", at()))?;
                let size = value
                    .trim()
                    .parse()
                    .with_context(|| format!("{}: size of {key}", at()))?;
                out.sizes.insert(key, size);
            }
            "notes" => {
                let key = unquote(key).with_context(|| format!("{}: package name", at()))?;
                let note = unquote(value).with_context(|| format!("{}: note for {key}", at()))?;
                if !note.trim().is_empty() {
                    out.notes.insert(key, note);
                }
            }
            _ => bail!("{}: unexpected line {line:?}", at()),
        }
    }
    Ok(out)
}

/// The text of [`FILE`] for `r`.
pub(crate) fn render(r: &Recorded) -> String {
    let mut out = String::from(HEADER);
    let _ = write!(out, "\nversion = {}\n\n[sizes]\n", quote(&r.version));
    for (name, size) in &r.sizes {
        let _ = writeln!(out, "{} = {size}", quote(name));
    }
    out.push('\n');
    out.push_str(NOTES_HELP);
    out.push_str("[notes]\n");
    for (name, note) in &r.notes {
        let _ = writeln!(out, "{} = {}", quote(name), quote(note));
    }
    out
}

/// Reads [`FILE`] under `root`.
pub(crate) fn read(root: &Path) -> anyhow::Result<Recorded> {
    let path = root.join(FILE);
    let text = fs::read_to_string(&path).with_context(|| format!("reading {FILE}"))?;
    parse(&text)
}

/// The package name of a file named `textweaver-VERSION-REST`: `REST`,
/// when it ends like a package. `None` for anything else (a `.zsync` file,
/// the checksums, another version).
pub(crate) fn package_name(file_name: &str, version: &str) -> Option<String> {
    let rest = file_name.strip_prefix(&format!("textweaver-{version}-"))?;
    PACKAGE_ENDINGS
        .iter()
        .any(|e| rest.ends_with(e))
        .then(|| rest.to_owned())
}

/// What the budget says about one package.
#[derive(Debug, PartialEq)]
pub(crate) enum Verdict {
    /// No size is recorded for this package.
    New,
    /// Within the budget: the change in percent (negative when smaller).
    Within(f64),
    /// Over the budget, with the note that explains it.
    Explained(f64, String),
    /// Over the budget with no note: the build fails.
    Over(f64),
}

/// The budget's verdict on `name` at `size` bytes.
pub(crate) fn judge(r: &Recorded, name: &str, size: u64) -> Verdict {
    let Some(&before) = r.sizes.get(name) else {
        return Verdict::New;
    };
    let change = if before == 0 {
        0.0
    } else {
        (size as f64 - before as f64) * 100.0 / before as f64
    };
    // Integer arithmetic for the limit, so 10.0 percent exactly passes.
    if u128::from(size) * 100 <= u128::from(before) * u128::from(100 + MAX_GROWTH_PERCENT) {
        return Verdict::Within(change);
    }
    match r.notes.get(name).or_else(|| r.notes.get(ALL)) {
        Some(note) => Verdict::Explained(change, note.clone()),
        None => Verdict::Over(change),
    }
}

/// `bytes` in MB (1,048,576 bytes), one decimal, as the release workflow
/// writes them.
pub(crate) fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1_048_576.0)
}

/// One line for the build output, meaning first.
fn line(r: &Recorded, name: &str, size: u64, verdict: &Verdict) -> String {
    let now = megabytes(size);
    let before = r
        .sizes
        .get(name)
        .copied()
        .map(megabytes)
        .unwrap_or_default();
    let v = &r.version;
    match verdict {
        Verdict::New => format!("Size: {name}, {now}; no size recorded for it in {FILE}"),
        Verdict::Within(c) => {
            format!("Size within budget: {name}, {now}, {c:+.1} percent from {before} in {v}")
        }
        Verdict::Explained(c, note) => format!(
            "Size over budget, with a note: {name}, {now}, {c:+.1} percent from {before} in {v}. Note: {note}"
        ),
        Verdict::Over(c) => format!(
            "Size over budget: {name}, {now}, {c:+.1} percent from {before} in {v}, more than {MAX_GROWTH_PERCENT} percent"
        ),
    }
}

/// Checks each package file in `files` against [`FILE`] under `root`,
/// prints one line per package, and fails when one grew past the budget
/// without a note. Files that are not packages are skipped.
pub(crate) fn check(root: &Path, files: &[PathBuf]) -> anyhow::Result<()> {
    let recorded = read(root)?;
    let version = env!("CARGO_PKG_VERSION");
    let mut over = Vec::new();
    for file in files {
        let Some(name) = file
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| package_name(n, version))
        else {
            continue;
        };
        let size = fs::metadata(file)
            .with_context(|| format!("measuring {}", file.display()))?
            .len();
        let verdict = judge(&recorded, &name, size);
        println!("{}", line(&recorded, &name, size, &verdict));
        if let Verdict::Over(_) = verdict {
            over.push(name);
        }
    }
    if !over.is_empty() {
        bail!(
            "over the size budget: {}. Find what grew, or say why under [notes] in {FILE}, as \"{}\" = \"why\"",
            over.join(", "),
            over[0]
        );
    }
    Ok(())
}

/// Package names and sizes from `gh release view TAG --json assets`.
pub(crate) fn from_release_json(
    json: &str,
    version: &str,
) -> anyhow::Result<BTreeMap<String, u64>> {
    let v: serde_json::Value = serde_json::from_str(json).context("reading gh's JSON")?;
    let assets = v["assets"]
        .as_array()
        .context("gh's JSON has no assets list")?;
    let mut out = BTreeMap::new();
    for a in assets {
        let (Some(name), Some(size)) = (a["name"].as_str(), a["size"].as_u64()) else {
            continue;
        };
        if let Some(p) = package_name(name, version) {
            out.insert(p, size);
        }
    }
    Ok(out)
}

/// Package names and sizes from the package files in `dir`.
pub(crate) fn from_dir(dir: &Path, version: &str) -> anyhow::Result<BTreeMap<String, u64>> {
    let mut out = BTreeMap::new();
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        let Some(name) = entry
            .file_name()
            .to_str()
            .and_then(|n| package_name(n, version))
        else {
            continue;
        };
        if entry.file_type()?.is_file() {
            out.insert(name, entry.metadata()?.len());
        }
    }
    Ok(out)
}

/// The "Package sizes" subsection for the release notes.
pub(crate) fn notes_section(version: &str, sizes: &BTreeMap<String, u64>) -> String {
    let mut out = String::from("### Package sizes\n\n");
    for (name, size) in sizes {
        let _ = writeln!(
            out,
            "- `textweaver-{version}-{name}`: {} ({} bytes)",
            megabytes(*size),
            grouped(*size)
        );
    }
    out
}

/// `n` with commas between thousands.
fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recorded() -> Recorded {
        Recorded {
            version: "0.1.0-alpha.7".into(),
            sizes: [("windows-x86_64.zip".to_owned(), 1000)].into(),
            notes: BTreeMap::new(),
        }
    }

    /// The committed file parses, and writing it back gives the same text,
    /// so the release step's rewrite changes only the numbers.
    #[test]
    fn the_committed_file_round_trips() {
        let text = fs::read_to_string(crate::eci::root().join(FILE)).unwrap();
        let r = parse(&text).unwrap();
        assert!(!r.version.is_empty());
        assert!(r.sizes.contains_key("windows-x86_64.zip"), "{r:?}");
        assert!(r.sizes.contains_key("windows-x86_64-gui.zip"), "{r:?}");
        assert_eq!(render(&r).replace("\r\n", "\n"), text.replace("\r\n", "\n"));
    }

    #[test]
    fn notes_round_trip_with_quotes() {
        let mut r = recorded();
        r.notes
            .insert("all".into(), "the \"Opus\" encoder \\ in process".into());
        assert_eq!(parse(&render(&r)).unwrap(), r);
        assert!(parse("[other]\n").is_err());
        assert!(parse("[sizes]\n\"a.zip\" = many\n").is_err());
        // An empty note is no note.
        let r = parse("[notes]\n\"a.zip\" = \"\"\n").unwrap();
        assert!(r.notes.is_empty());
    }

    #[test]
    fn the_budget_allows_ten_percent() {
        let mut r = recorded();
        assert_eq!(judge(&r, "windows-x86_64.zip", 1100), Verdict::Within(10.0));
        assert_eq!(judge(&r, "windows-x86_64.zip", 900), Verdict::Within(-10.0));
        assert!(matches!(
            judge(&r, "windows-x86_64.zip", 1101),
            Verdict::Over(_)
        ));
        assert_eq!(judge(&r, "macos-universal.tar.gz", 5), Verdict::New);
        r.notes.insert("all".into(), "fonts".into());
        assert!(matches!(
            judge(&r, "windows-x86_64.zip", 2000),
            Verdict::Explained(c, ref n) if (c - 100.0).abs() < 1e-9 && n == "fonts"
        ));
        r.notes
            .insert("windows-x86_64.zip".into(), "the reader's own".into());
        assert!(matches!(
            judge(&r, "windows-x86_64.zip", 2000),
            Verdict::Explained(_, ref n) if n == "the reader's own"
        ));
    }

    #[test]
    fn lines_lead_with_the_verdict() {
        let r = recorded();
        let over = line(&r, "windows-x86_64.zip", 2_000, &Verdict::Over(100.0));
        assert!(
            over.starts_with("Size over budget: windows-x86_64.zip"),
            "{over}"
        );
        let ok = line(&r, "windows-x86_64.zip", 1_050, &Verdict::Within(5.0));
        assert!(ok.starts_with("Size within budget:"), "{ok}");
        assert!(ok.contains("+5.0 percent"), "{ok}");
    }

    #[test]
    fn package_names_drop_the_version() {
        let v = "0.1.0-alpha.8";
        assert_eq!(
            package_name("textweaver-0.1.0-alpha.8-windows-x86_64.zip", v).as_deref(),
            Some("windows-x86_64.zip")
        );
        assert_eq!(
            package_name("textweaver-0.1.0-alpha.8-linux-x86_64-gui.AppImage", v).as_deref(),
            Some("linux-x86_64-gui.AppImage")
        );
        assert_eq!(
            package_name("textweaver-0.1.0-alpha.8-linux-x86_64.AppImage.zsync", v),
            None
        );
        assert_eq!(package_name("SHA256SUMS.txt", v), None);
        assert_eq!(
            package_name("textweaver-0.1.0-alpha.7-windows-x86_64.zip", v),
            None
        );
    }

    #[test]
    fn sizes_come_from_the_release() {
        let json = r#"{"assets":[
            {"name":"SHA256SUMS.txt","size":1855},
            {"name":"textweaver-0.1.0-alpha.7-windows-x86_64.zip","size":69108008},
            {"name":"textweaver-0.1.0-alpha.7-linux-x86_64.AppImage.zsync","size":211845},
            {"name":"textweaver-0.1.0-alpha.7-linux-x86_64-gui.AppImage","size":40380920}
        ]}"#;
        let s = from_release_json(json, "0.1.0-alpha.7").unwrap();
        assert_eq!(s.len(), 2, "{s:?}");
        assert_eq!(s["windows-x86_64.zip"], 69_108_008);
        let notes = notes_section("0.1.0-alpha.7", &s);
        assert!(notes.starts_with("### Package sizes\n\n"), "{notes}");
        assert!(
            notes.contains(
                "- `textweaver-0.1.0-alpha.7-windows-x86_64.zip`: 65.9 MB (69,108,008 bytes)"
            ),
            "{notes}"
        );
        assert!(from_release_json("{}", "x").is_err());
    }

    #[test]
    fn sizes_come_from_a_folder() {
        let dir = std::env::temp_dir().join(format!("tw-sizes-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("textweaver-1.0.0-windows-x86_64.zip"), [0u8; 10]).unwrap();
        fs::write(
            dir.join("textweaver-1.0.0-windows-x86_64.zip.part"),
            [0u8; 3],
        )
        .unwrap();
        fs::create_dir_all(dir.join("textweaver-1.0.0-windows-x86_64-gui.zip")).unwrap();
        let s = from_dir(&dir, "1.0.0").unwrap();
        assert_eq!(s.len(), 1, "{s:?}");
        assert_eq!(s["windows-x86_64.zip"], 10);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn thousands_are_grouped() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(1_000), "1,000");
        assert_eq!(grouped(69_108_008), "69,108,008");
    }
}
