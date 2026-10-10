//! `cargo xtask release-assets`: the exact set of files a release carries.
//!
//! A release that silently lost a package, or gained a stray one, is a
//! release whose checks passed while blind (star's release chain audit
//! found 10 of 24 tags off the release commit). The release workflow
//! compares what was built, and then what was published, with the list
//! here, before and after upload:
//!
//! - `--dir DIR --platform P`: the package files `textweaver-*` in DIR
//!   (the build folder, `target/dist`) must be exactly platform P's, for
//!   this version. Folders and files not named `textweaver-*` are ignored.
//! - `--names FILE`: FILE (`-` for standard input) holds a published
//!   release's asset names, one per line (`gh release view --json assets
//!   --jq '.assets[].name'`); they must be exactly every platform's files
//!   plus `SHA256SUMS.txt`.
//!
//! `--version V` overrides the version, which is otherwise this
//! workspace's (the one `cargo xtask dist` names its packages with). Each
//! missing or extra file is one line, meaning first: `Missing: NAME` or
//! `Extra: NAME`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};

/// The checksums file the release workflow writes once, at the end.
pub const SUMS: &str = "SHA256SUMS.txt";

/// Each platform job's files, without `textweaver-VERSION-`: one package
/// per OS and CPU architecture holding both programs (B1-o2). The package
/// names match `xtask/package-sizes.toml`'s, plus the AppImages' `.zsync`
/// files for delta updates, and on Linux a copy of each under the `-gui`
/// name the app's older AppImages look for (`appimage.rs`).
pub const PLATFORMS: [(&str, &[&str]); 4] = [
    ("windows", &["windows-x86_64.zip"]),
    ("macos", &["macos-universal.zip"]),
    (
        "linux-x86_64",
        &[
            "linux-x86_64.AppImage",
            "linux-x86_64.AppImage.zsync",
            "linux-x86_64-gui.AppImage.zsync",
            "linux-x86_64.tar.gz",
        ],
    ),
    (
        "linux-aarch64",
        &[
            "linux-aarch64.AppImage",
            "linux-aarch64.AppImage.zsync",
            "linux-aarch64-gui.AppImage.zsync",
            "linux-aarch64.tar.gz",
        ],
    ),
];

/// The usage line.
const USAGE: &str =
    "usage: cargo xtask release-assets (--dir DIR --platform P | --names FILE) [--version V]";

/// Platform `platform`'s file names for `version`, or `None` for an
/// unknown platform.
pub fn expected_for(platform: &str, version: &str) -> Option<BTreeSet<String>> {
    PLATFORMS
        .iter()
        .find(|(p, _)| *p == platform)
        .map(|(_, files)| {
            files
                .iter()
                .map(|f| format!("textweaver-{version}-{f}"))
                .collect()
        })
}

/// Every file of a complete release for `version`, the checksums included.
pub fn expected_release(version: &str) -> BTreeSet<String> {
    let mut all: BTreeSet<String> = PLATFORMS
        .iter()
        .flat_map(|(_, files)| files.iter())
        .map(|f| format!("textweaver-{version}-{f}"))
        .collect();
    all.insert(SUMS.to_owned());
    all
}

/// The differences, one line each, meaning first: missing files, then
/// extra ones.
pub fn compare(expected: &BTreeSet<String>, actual: &BTreeSet<String>) -> Vec<String> {
    let mut out: Vec<String> = expected
        .difference(actual)
        .map(|n| format!("Missing: {n}"))
        .collect();
    out.extend(actual.difference(expected).map(|n| format!("Extra: {n}")));
    out
}

/// The package files in `dir`: regular files named `textweaver-*`.
pub fn files_in(dir: &Path) -> anyhow::Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("listing {}", dir.display()))? {
        let entry = entry.with_context(|| format!("listing {}", dir.display()))?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("textweaver-") {
            out.insert(name);
        }
    }
    Ok(out)
}

/// Asset names, one per line; blank lines are ignored.
pub fn names(text: &str) -> BTreeSet<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
enum Source {
    Dir { dir: PathBuf, platform: String },
    Names(String),
}

fn parse(args: &[String]) -> anyhow::Result<(Source, Option<String>)> {
    let (mut dir, mut platform, mut names, mut version) = (None, None, None, None);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let slot = match a.as_str() {
            "--dir" => &mut dir,
            "--platform" => &mut platform,
            "--names" => &mut names,
            "--version" => &mut version,
            _ => bail!("{USAGE}"),
        };
        let Some(v) = it.next() else {
            bail!("{USAGE}");
        };
        if slot.replace(v.clone()).is_some() {
            bail!("{USAGE}");
        }
    }
    let source = match (dir, platform, names) {
        (Some(dir), Some(platform), None) => Source::Dir {
            dir: PathBuf::from(dir),
            platform,
        },
        (None, None, Some(n)) => Source::Names(n),
        _ => bail!("{USAGE}"),
    };
    Ok((source, version))
}

/// `cargo xtask release-assets ...`.
pub fn run() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let (source, version) = parse(&args)?;
    let version = version.unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_owned());
    let (what, expected, actual) = match &source {
        Source::Dir { dir, platform } => {
            let Some(expected) = expected_for(platform, &version) else {
                let known: Vec<&str> = PLATFORMS.iter().map(|(p, _)| *p).collect();
                bail!(
                    "unknown platform {platform}; the platforms are {}",
                    known.join(", ")
                );
            };
            (
                format!("the {platform} packages in {}", dir.display()),
                expected,
                files_in(dir)?,
            )
        }
        Source::Names(file) => {
            let text = if file == "-" {
                std::io::read_to_string(std::io::stdin()).context("reading standard input")?
            } else {
                std::fs::read_to_string(file).with_context(|| format!("reading {file}"))?
            };
            (
                "the release's assets".to_owned(),
                expected_release(&version),
                names(&text),
            )
        }
    };
    let diff = compare(&expected, &actual);
    if diff.is_empty() {
        println!(
            "Pass: {what} are exactly the {} expected files for {version}",
            expected.len()
        );
        for n in &expected {
            println!("Present: {n}");
        }
        return Ok(());
    }
    println!(
        "Fail: {what} differ from the {} expected files for {version}",
        expected.len()
    );
    for line in &diff {
        println!("{line}");
    }
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(a: &[&str]) -> Vec<String> {
        a.iter().map(|x| (*x).to_owned()).collect()
    }

    #[test]
    fn every_package_in_the_size_budget_is_expected() {
        // The size file names every package of the last release; the list
        // here adds only the .zsync files. A package added or dropped on
        // one side and not the other fails here.
        let root = crate::eci::root();
        let recorded = crate::sizes::read(&root).unwrap();
        let packages: BTreeSet<String> = PLATFORMS
            .iter()
            .flat_map(|(_, f)| f.iter())
            .filter(|f| !f.ends_with(".zsync"))
            .map(|f| (*f).to_owned())
            .collect();
        let sized: BTreeSet<String> = recorded.sizes.keys().cloned().collect();
        assert_eq!(packages, sized);
    }

    #[test]
    fn a_complete_release_has_eleven_files() {
        let all = expected_release("1.2.3");
        assert_eq!(all.len(), 11);
        assert!(all.contains("SHA256SUMS.txt"));
        assert!(all.contains("textweaver-1.2.3-windows-x86_64.zip"));
        assert!(all.contains("textweaver-1.2.3-macos-universal.zip"));
        assert!(all.contains("textweaver-1.2.3-linux-aarch64-gui.AppImage.zsync"));
        // No separate app packages any more.
        assert!(!all.iter().any(|f| f.ends_with("-gui.zip")
            || f.ends_with("-gui.tar.gz")
            || f.ends_with("-gui.AppImage")));
    }

    #[test]
    fn missing_and_extra_files_are_named() {
        let expected = expected_for("windows", "1.0.0").unwrap();
        let mut actual = expected.clone();
        assert!(compare(&expected, &actual).is_empty());
        actual.remove("textweaver-1.0.0-windows-x86_64.zip");
        actual.insert("textweaver-1.0.0-windows-x86_64-gui.zip".into());
        assert_eq!(
            compare(&expected, &actual),
            [
                "Missing: textweaver-1.0.0-windows-x86_64.zip",
                "Extra: textweaver-1.0.0-windows-x86_64-gui.zip"
            ]
        );
        assert!(expected_for("solaris", "1.0.0").is_none());
    }

    #[test]
    fn a_build_folder_is_read_by_name() {
        let dir = tempfile::tempdir().unwrap();
        for f in expected_for("macos", "1.0.0").unwrap() {
            std::fs::write(dir.path().join(f), b"x").unwrap();
        }
        // A staging folder and an unrelated file are not packages.
        std::fs::create_dir(dir.path().join("textweaver-1.0.0-macos-universal")).unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"x").unwrap();
        let found = files_in(dir.path()).unwrap();
        assert!(compare(&expected_for("macos", "1.0.0").unwrap(), &found).is_empty());
    }

    #[test]
    fn asset_names_are_read_one_per_line() {
        let n = names("a\n\n b \r\nSHA256SUMS.txt\n");
        assert_eq!(n.len(), 3);
        assert!(n.contains("b"));
    }

    #[test]
    fn arguments() {
        assert_eq!(
            parse(&s(&["--dir", "d", "--platform", "macos"])).unwrap(),
            (
                Source::Dir {
                    dir: PathBuf::from("d"),
                    platform: "macos".into()
                },
                None
            )
        );
        assert_eq!(
            parse(&s(&["--names", "-", "--version", "1.0.0"])).unwrap(),
            (Source::Names("-".into()), Some("1.0.0".into()))
        );
        for bad in [
            &[][..],
            &["--dir", "d"],
            &["--names", "-", "--dir", "d", "--platform", "x"],
            &["--names"],
            &["--names", "a", "--names", "b"],
            &["--bogus", "x"],
        ] {
            assert_eq!(parse(&s(bad)).unwrap_err().to_string(), USAGE);
        }
    }
}
