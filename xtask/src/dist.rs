//! `cargo xtask dist [--universal] [--out DIR]`: a release package for
//! this platform (docs/dev/releasing.md).
//!
//! Builds `textweaver` and `tw` with the `dist` profile (the release
//! profile with fat LTO), the engine hosts for the platform (Windows and
//! Linux), and stages them with the pronunciation dictionaries, the
//! licence, the third-party notices and licence files, and the user guides
//! in `target/dist/textweaver-VERSION-PLATFORM/`, then archives the folder:
//! a `.zip` on Windows, a `.tar.gz` elsewhere. It fails if a notice is
//! missing from the staged folder.
//!
//! - Windows builds link the C runtime statically (`+crt-static`), so the
//!   package runs without the Visual C++ redistributable. The static build
//!   uses its own target directory (`target/dist-build`) so it does not
//!   invalidate everyday builds.
//! - `--universal` (macOS) builds for Apple silicon and Intel, joins the
//!   binaries with `lipo`, and signs the result ad hoc (`codesign -s -`),
//!   which Apple silicon requires. The package is not notarized.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};

use crate::eci::{self, HostBuild};

/// The user binaries: (package, binary).
const BINARIES: [(&str, &str); 2] = [("textweaver-tui", "textweaver"), ("textweaver-cli", "tw")];
/// Speech engines built into the user binaries on this platform. Nothing
/// extra is linked: Omnivox is a subprocess, speech-dispatcher a socket,
/// and espeak-ng is loaded at run time when it is installed. Linux gets all
/// three (the AppImage and the tarball); Windows and macOS keep Omnivox
/// and their own engines.
fn engines() -> &'static [&'static str] {
    if cfg!(target_os = "linux") {
        &["omnivox", "speechd", "espeak"]
    } else {
        &["omnivox"]
    }
}

/// The `--features` value for the user binaries.
pub(crate) fn features() -> String {
    BINARIES
        .iter()
        .flat_map(|(package, _)| engines().iter().map(move |e| format!("{package}/{e}")))
        .collect::<Vec<_>>()
        .join(",")
}
/// The cargo profile for packages (root `Cargo.toml`, `[profile.dist]`).
const PROFILE: &str = "dist";
/// Where the licence files of bundled data go in the package: (source,
/// path in the package). The notices file itself goes at the top.
const LICENCE_FILES: [(&str, &str); 7] = [
    (
        "third_party/fonts/atkinson-hyperlegible-next/OFL.txt",
        "licenses/fonts/atkinson-hyperlegible-next/OFL.txt",
    ),
    (
        "third_party/fonts/atkinson-hyperlegible-mono/OFL.txt",
        "licenses/fonts/atkinson-hyperlegible-mono/OFL.txt",
    ),
    (
        "third_party/fonts/opendyslexic/OFL.txt",
        "licenses/fonts/opendyslexic/OFL.txt",
    ),
    ("third_party/scowl/Copyright", "licenses/scowl/Copyright"),
    (
        "third_party/ibmtts-dictionaries/LICENSE.md",
        "licenses/ibmtts-dictionaries/LICENSE.md",
    ),
    (
        "third_party/lexicon/WORDNET-LICENSE",
        "licenses/lexicon/WORDNET-LICENSE",
    ),
    (
        "third_party/lexicon/CMUDICT-LICENSE",
        "licenses/lexicon/CMUDICT-LICENSE",
    ),
];
/// Data files copied into the package: (source, path in the package). The
/// define-word dictionary sits in `lexicon/` beside the programs, where
/// `textweaver_lexicon::data_file_candidates` looks.
const DATA_FILES: [(&str, &str); 1] = [(
    "third_party/lexicon/lexicon-en.twlex",
    "lexicon/lexicon-en.twlex",
)];
/// The two macOS targets joined by `--universal`.
const MAC_TARGETS: [&str; 2] = ["aarch64-apple-darwin", "x86_64-apple-darwin"];
/// Documents copied into the package: (source, name in the package).
const DOCS: [(&str, &str); 6] = [
    ("docs/quickstart.md", "QUICKSTART.md"),
    ("README.md", "README.md"),
    ("LICENSE", "LICENSE"),
    ("CHANGELOG.md", "CHANGELOG.md"),
    ("docs/install.md", "INSTALL.md"),
    ("docs/eloquence.md", "docs/eloquence.md"),
];
/// More guides copied when present, besides the user guides listed in the
/// documentation index (see [`user_guides`]).
const OPTIONAL_DOCS: [&str; 3] = ["docs/README.md", "docs/quickstart.md", "scripts/README.md"];
/// The documentation index; the guides linked under its [`USER_SECTION`]
/// are packaged.
const DOCS_INDEX: &str = "docs/README.md";
/// The heading of the user section in [`DOCS_INDEX`].
const USER_SECTION: &str = "## For users";
/// The offline interactive pages, packaged whole.
const SITE_DIR: &str = "docs/site";

/// The user guides linked under "For users" in the documentation index,
/// as paths relative to the root (`docs/reading.md`, `scripts/README.md`).
/// Only local Markdown links count; anchors are dropped.
fn user_guides(index: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_section = false;
    for line in index.lines() {
        if line.starts_with("## ") {
            in_section = line.trim_end() == USER_SECTION;
            continue;
        }
        if !in_section {
            continue;
        }
        let mut rest = line;
        while let Some(start) = rest.find("](") {
            let after = &rest[start + 2..];
            let Some(end) = after.find(')') else {
                break;
            };
            let target = after[..end].split('#').next().unwrap_or("");
            if target.ends_with(".md") && !target.contains("://") {
                let path = normalize(&format!("docs/{target}"));
                if !out.contains(&path) {
                    out.push(path);
                }
            }
            rest = &after[end..];
        }
    }
    out
}

/// Resolves `.` and `..` in a relative path with forward slashes.
fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for p in path.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            p => parts.push(p),
        }
    }
    parts.join("/")
}

/// Copies every file under `src` into `dest`, keeping the layout.
pub(crate) fn copy_tree(src: &Path, dest: &Path) -> anyhow::Result<()> {
    let mut files = Vec::new();
    collect(src, &mut files)?;
    for f in files {
        eci::copy(&f, &dest.join(f.strip_prefix(src)?))?;
    }
    Ok(())
}

/// Parsed arguments.
#[derive(Debug, Default, PartialEq, Eq)]
struct Args {
    universal: bool,
    out: Option<PathBuf>,
}

fn parse(args: &[String]) -> anyhow::Result<Args> {
    let mut out = Args::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--universal" => out.universal = true,
            "--out" => out.out = Some(PathBuf::from(it.next().context("--out needs a directory")?)),
            other => bail!(
                "unknown argument {other} (usage: cargo xtask dist [--universal] [--out DIR])"
            ),
        }
    }
    if out.universal && !cfg!(target_os = "macos") {
        bail!("--universal is for macOS");
    }
    Ok(out)
}

/// The platform part of the package name.
fn platform(universal: bool) -> String {
    let os = std::env::consts::OS;
    let arch = if universal {
        "universal"
    } else {
        std::env::consts::ARCH
    };
    format!("{os}-{arch}")
}

/// The package folder name.
fn package_name(version: &str, platform: &str) -> String {
    format!("textweaver-{version}-{platform}")
}

/// `cargo xtask dist`.
pub fn run() -> anyhow::Result<()> {
    let args = parse(&std::env::args().skip(2).collect::<Vec<_>>())?;
    let staged = stage(args.out.clone(), args.universal)?;
    let archive = archive(&staged)?;
    println!("package {}", archive.display());
    Ok(())
}

/// A staged package folder.
pub(crate) struct Staged {
    /// The package name (`textweaver-VERSION-PLATFORM`).
    pub name: String,
    /// The staged folder, `out/name`.
    pub dir: PathBuf,
    /// The output folder.
    pub out: PathBuf,
}

/// Builds the binaries and hosts and stages the package folder (everything
/// but the archive). `out` defaults to `target/dist`.
pub(crate) fn stage(out: Option<PathBuf>, universal: bool) -> anyhow::Result<Staged> {
    let root = eci::root();
    let version = env!("CARGO_PKG_VERSION");
    let build_dir = if cfg!(windows) {
        eci::target_dir(&root).join("dist-build")
    } else {
        eci::target_dir(&root)
    };
    let out = out.unwrap_or_else(|| eci::target_dir(&root).join("dist"));
    let name = package_name(version, &platform(universal));
    let stage = out.join(&name);
    if stage.exists() {
        fs::remove_dir_all(&stage).with_context(|| format!("clearing {}", stage.display()))?;
    }
    fs::create_dir_all(&stage)?;

    if universal {
        for t in MAC_TARGETS {
            build_binaries(&root, &build_dir, Some(t))?;
        }
        for (_, bin) in BINARIES {
            let parts: Vec<PathBuf> = MAC_TARGETS
                .iter()
                .map(|t| build_dir.join(t).join(PROFILE).join(bin))
                .collect();
            let dest = stage.join(bin);
            run_tool(
                Command::new("lipo")
                    .arg("-create")
                    .args(&parts)
                    .arg("-output")
                    .arg(&dest),
            )?;
            run_tool(
                Command::new("codesign")
                    .args(["--force", "--sign", "-"])
                    .arg(&dest),
            )?;
            println!("installed {} (universal)", dest.display());
        }
    } else {
        build_binaries(&root, &build_dir, None)?;
        for (_, bin) in BINARIES {
            let file = format!("{bin}{}", std::env::consts::EXE_SUFFIX);
            eci::copy(&build_dir.join(PROFILE).join(&file), &stage.join(&file))?;
        }
    }

    // Engine hosts: Eloquence (ECI) and SAPI5 on Windows, ECI for Voxin on
    // Linux. macOS speaks through Apple's own voices in process.
    if !cfg!(target_os = "macos") {
        let hosts = eci::all_hosts();
        build_hosts(&root, &build_dir, &hosts)?;
        for h in &hosts {
            eci::copy(
                &h.built(&build_dir, PROFILE),
                &stage.join(h.installed_name()),
            )?;
        }
        eci::copy_dictionaries(&root, &stage)?;
    }

    for (src, dest) in DOCS.iter().chain(DATA_FILES.iter()) {
        eci::copy(&root.join(src), &stage.join(dest))?;
    }
    stage_notices(&root, &stage)?;
    // The helper scripts for this platform (doctor, speech check, update).
    let ext = if cfg!(windows) { "ps1" } else { "sh" };
    if let Ok(entries) = fs::read_dir(root.join("scripts")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == ext) {
                eci::copy(&path, &stage.join("scripts").join(entry.file_name()))?;
            }
        }
    }
    // The user guides from the documentation index, and the offline pages.
    let index = fs::read_to_string(root.join(DOCS_INDEX)).unwrap_or_default();
    let guides = user_guides(&index);
    for src in OPTIONAL_DOCS.iter().map(|s| (*s).to_owned()).chain(guides) {
        let path = root.join(&src);
        if path.is_file() {
            eci::copy(&path, &stage.join(&src))?;
        }
    }
    let site = root.join(SITE_DIR);
    if site.is_dir() {
        copy_tree(&site, &stage.join(SITE_DIR))?;
    }

    // Linux: the menu entry and the icon, for the tarball's users and the
    // AppImage (scripts/linux/).
    if cfg!(target_os = "linux") {
        for (src, dest) in LINUX_DESKTOP_FILES {
            eci::copy(&root.join(src), &stage.join(dest))?;
        }
    }

    check_notices(&stage)?;
    Ok(Staged {
        name,
        dir: stage,
        out,
    })
}

/// Archives a staged package: a `.zip` on Windows, a `.tar.gz` elsewhere.
pub(crate) fn archive(staged: &Staged) -> anyhow::Result<PathBuf> {
    let Staged { name, dir, out } = staged;
    if cfg!(windows) {
        let zip = out.join(format!("{name}.zip"));
        zip_dir(dir, name, &zip)?;
        Ok(zip)
    } else {
        let tgz = out.join(format!("{name}.tar.gz"));
        run_tool(
            Command::new("tar")
                .arg("-czf")
                .arg(&tgz)
                .arg("-C")
                .arg(out)
                .arg(name),
        )?;
        Ok(tgz)
    }
}

/// The Linux menu entry and icon: (source, path in the package).
pub(crate) const LINUX_DESKTOP_FILES: [(&str, &str); 2] = [
    (
        "scripts/linux/textweaver.desktop",
        "share/applications/textweaver.desktop",
    ),
    (
        "scripts/linux/textweaver.svg",
        "share/icons/hicolor/scalable/apps/textweaver.svg",
    ),
];

/// Copies the third-party notices and the data licence files into `stage`.
fn stage_notices(root: &Path, stage: &Path) -> anyhow::Result<()> {
    eci::copy(
        &root.join(crate::notices::NOTICES),
        &stage.join(crate::notices::NOTICES),
    )?;
    for (src, dest) in LICENCE_FILES {
        eci::copy(&root.join(src), &stage.join(dest))?;
    }
    Ok(())
}

/// Fails unless `stage` holds the licence, the notices, and every data
/// licence file: a package must never ship without them.
fn check_notices(stage: &Path) -> anyhow::Result<()> {
    let required = ["LICENSE", crate::notices::NOTICES]
        .into_iter()
        .chain(LICENCE_FILES.iter().map(|(_, dest)| *dest));
    let missing: Vec<&str> = required.filter(|f| !stage.join(f).is_file()).collect();
    if !missing.is_empty() {
        bail!("the package lacks {}", missing.join(", "));
    }
    Ok(())
}

/// A cargo command for release builds into `build_dir`, with the static C
/// runtime on Windows.
fn cargo(root: &Path, build_dir: &Path) -> Command {
    let mut cmd = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    cmd.current_dir(root).env("CARGO_TARGET_DIR", build_dir);
    if cfg!(windows) {
        let mut flags = std::env::var("RUSTFLAGS").unwrap_or_default();
        if !flags.contains("crt-static") {
            flags.push_str(" -C target-feature=+crt-static");
        }
        cmd.env("RUSTFLAGS", flags.trim());
    }
    cmd
}

fn build_binaries(root: &Path, build_dir: &Path, target: Option<&str>) -> anyhow::Result<()> {
    let mut cmd = cargo(root, build_dir);
    cmd.args(["build", "--locked", "--profile", PROFILE, "--features"])
        .arg(features());
    for (package, bin) in BINARIES {
        cmd.args(["-p", package, "--bin", bin]);
    }
    if let Some(t) = target {
        cmd.args(["--target", t]);
    }
    run_tool(&mut cmd).context("building textweaver and tw")
}

fn build_hosts(root: &Path, build_dir: &Path, hosts: &[HostBuild]) -> anyhow::Result<()> {
    let mut targets: Vec<Option<&str>> = Vec::new();
    for h in hosts {
        if !targets.contains(&h.target) {
            targets.push(h.target);
        }
    }
    for target in targets {
        let mut cmd = cargo(root, build_dir);
        cmd.args([
            "build",
            "--locked",
            "--profile",
            PROFILE,
            "--no-default-features",
        ]);
        for h in hosts.iter().filter(|h| h.target == target) {
            cmd.args(["-p", h.package, "--bin", h.bin]);
        }
        if let Some(t) = target {
            cmd.args(["--target", t]);
        }
        run_tool(&mut cmd).with_context(|| match target {
            Some(t) => format!("building the engine hosts for {t} (rustup target add {t})"),
            None => "building the engine hosts".to_owned(),
        })?;
    }
    Ok(())
}

fn run_tool(cmd: &mut Command) -> anyhow::Result<()> {
    let status = cmd
        .status()
        .with_context(|| format!("running {:?}", cmd.get_program()))?;
    if !status.success() {
        bail!("{:?} failed ({status})", cmd.get_program());
    }
    Ok(())
}

/// Zips `dir` so its files sit under `prefix/` in the archive.
fn zip_dir(dir: &Path, prefix: &str, zip_path: &Path) -> anyhow::Result<()> {
    use std::io::Write as _;
    use zip::write::SimpleFileOptions;

    let file =
        fs::File::create(zip_path).with_context(|| format!("creating {}", zip_path.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut files = Vec::new();
    collect(dir, &mut files)?;
    files.sort();
    for path in files {
        let rel = path.strip_prefix(dir)?;
        let name = format!("{prefix}/{}", rel.to_string_lossy().replace('\\', "/"));
        zip.start_file(name, opts)?;
        zip.write_all(&fs::read(&path)?)?;
    }
    zip.finish()?;
    Ok(())
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(
            package_name("0.1.0-alpha.3", "windows-x86_64"),
            "textweaver-0.1.0-alpha.3-windows-x86_64"
        );
        assert!(platform(false).starts_with(std::env::consts::OS));
        assert!(platform(true).ends_with("-universal"));
    }

    #[test]
    fn arguments() {
        let a = parse(&["--out".into(), "x".into()]).unwrap();
        assert_eq!(a.out, Some(PathBuf::from("x")));
        assert!(!a.universal);
        assert!(parse(&["--bogus".into()]).is_err());
        assert_eq!(
            parse(&["--universal".into()]).is_ok(),
            cfg!(target_os = "macos")
        );
    }

    #[test]
    fn engines_follow_the_platform() {
        let f = features();
        assert!(f.contains("textweaver-tui/omnivox"), "{f}");
        assert!(f.contains("textweaver-cli/omnivox"), "{f}");
        let linux = cfg!(target_os = "linux");
        assert_eq!(f.contains("textweaver-cli/speechd"), linux, "{f}");
        assert_eq!(f.contains("textweaver-tui/espeak"), linux, "{f}");
    }

    #[test]
    fn linux_desktop_files_exist() {
        let root = eci::root();
        for (src, _) in LINUX_DESKTOP_FILES {
            assert!(root.join(src).is_file(), "{src} is packaged but missing");
        }
        let desktop = fs::read_to_string(root.join(LINUX_DESKTOP_FILES[0].0)).unwrap();
        assert!(desktop.lines().any(|l| l.trim_end() == "Terminal=true"));
        assert!(desktop.lines().any(|l| l.trim_end() == "Icon=textweaver"));
    }

    #[test]
    fn packaged_documents_exist() {
        let root = eci::root();
        for (src, _) in DOCS {
            assert!(root.join(src).is_file(), "{src} is packaged but missing");
        }
        assert!(root.join(crate::notices::NOTICES).is_file());
        for (src, _) in LICENCE_FILES {
            assert!(root.join(src).is_file(), "{src} is packaged but missing");
        }
    }

    #[test]
    fn user_guides_come_from_the_index() {
        let index = "# Docs\n\n- [Quick start](quickstart.md)\n\n## For users\n\n### Reading\n\n- [Reading](reading.md): moving around; see [keys](keyboard.md#browse).\n- [Scripts](../scripts/README.md): install.\n- [Site](https://example.org/x.md)\n- [Reading again](reading.md)\n\n## For contributors\n\n- [Architecture](architecture.md)\n";
        assert_eq!(
            user_guides(index),
            ["docs/reading.md", "docs/keyboard.md", "scripts/README.md"]
        );
        assert_eq!(normalize("docs/./a/../b.md"), "docs/b.md");
    }

    #[test]
    fn the_real_index_lists_existing_guides() {
        let root = eci::root();
        let Ok(index) = fs::read_to_string(root.join(DOCS_INDEX)) else {
            return;
        };
        let guides = user_guides(&index);
        assert!(guides.len() >= 10, "{guides:?}");
        for g in &guides {
            assert!(
                root.join(g).is_file(),
                "{g} is linked from {DOCS_INDEX} but missing"
            );
        }
        assert!(guides.iter().any(|g| g == "docs/reading.md"));
        assert!(!guides.iter().any(|g| g == "docs/dev/architecture.md"));
    }

    #[test]
    fn every_font_licence_is_packaged() {
        let fonts = eci::root().join("third_party").join("fonts");
        for entry in fs::read_dir(&fonts).unwrap().flatten() {
            if entry.path().join("OFL.txt").is_file() {
                let src = format!(
                    "third_party/fonts/{}/OFL.txt",
                    entry.file_name().to_string_lossy()
                );
                assert!(
                    LICENCE_FILES.iter().any(|(s, _)| *s == src),
                    "{src} is not packaged"
                );
            }
        }
    }

    #[test]
    fn staged_packages_carry_the_notices() {
        let stage = std::env::temp_dir().join(format!("tw-dist-notices-{}", std::process::id()));
        let _ = fs::remove_dir_all(&stage);
        fs::create_dir_all(&stage).unwrap();
        assert!(check_notices(&stage).is_err());
        let root = eci::root();
        eci::copy(&root.join("LICENSE"), &stage.join("LICENSE")).unwrap();
        stage_notices(&root, &stage).unwrap();
        check_notices(&stage).unwrap();
        assert!(stage.join("licenses/scowl/Copyright").is_file());
        assert!(stage.join("licenses/lexicon/WORDNET-LICENSE").is_file());
        assert!(stage.join("licenses/fonts/opendyslexic/OFL.txt").is_file());
        let _ = fs::remove_dir_all(&stage);
    }
}
