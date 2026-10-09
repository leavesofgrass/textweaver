//! `cargo xtask dist [--universal] [--out DIR]`: a release package for
//! this platform (docs/dev/releasing.md).
//!
//! Builds `textweaver` and `tw` with the `dist` profile (the release
//! profile with fat LTO), each in a cargo run of its own so the reader
//! gets only its own features, the engine hosts for the platform (Windows
//! and Linux), and stages them with the pronunciation dictionaries, the
//! licence, the third-party notices and licence files, and the user guides
//! in `target/dist/textweaver-VERSION-PLATFORM/`, then archives the folder:
//! a `.zip` on Windows, a `.tar.gz` elsewhere. It fails if a notice is
//! missing from the staged folder, or if the archive grew more than 10
//! percent over the last release's without a note (`sizes.rs`). A user
//! guide missing from the package only warns (`stage_user_docs`): the
//! build prints one warning line per missing file, adds it to the GitHub
//! job summary, and goes on.
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
pub(crate) fn engines() -> &'static [&'static str] {
    if cfg!(target_os = "linux") {
        &["omnivox", "speechd", "espeak"]
    } else {
        &["omnivox"]
    }
}

/// The `--features` value for one user binary's package: the platform's
/// engines, for that package only.
pub(crate) fn features(package: &str) -> String {
    engines()
        .iter()
        .map(|e| format!("{package}/{e}"))
        .collect::<Vec<_>>()
        .join(",")
}
/// The cargo profile for packages (root `Cargo.toml`, `[profile.dist]`).
pub(crate) const PROFILE: &str = "dist";
/// Where the licence files of bundled data go in the package: (source,
/// path in the package). The notices file itself goes at the top.
const LICENCE_FILES: [(&str, &str); 9] = [
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
    (
        "third_party/fonts/lexend/OFL.txt",
        "licenses/fonts/lexend/OFL.txt",
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
    ("third_party/lame/COPYING", "licenses/lame/COPYING"),
];
/// Data files copied into the package: (source, path in the package). The
/// define-word dictionary sits in `lexicon/` beside the programs, where
/// `textweaver_lexicon::data_file_candidates` looks.
pub(crate) const DATA_FILES: [(&str, &str); 1] = [(
    "third_party/lexicon/lexicon-en.twlex",
    "lexicon/lexicon-en.twlex",
)];
/// The two macOS targets joined by `--universal`.
pub(crate) const MAC_TARGETS: [&str; 2] = ["aarch64-apple-darwin", "x86_64-apple-darwin"];
/// Documents copied into the package: (source, name in the package).
const DOCS: [(&str, &str); 6] = [
    ("docs/quickstart.md", "QUICKSTART.md"),
    ("README.md", "README.md"),
    ("LICENSE", "LICENSE"),
    ("CHANGELOG.md", "CHANGELOG.md"),
    ("docs/install.md", "INSTALL.md"),
    ("docs/eloquence.md", "docs/eloquence.md"),
];
/// The documentation index; it and the guides linked under its
/// [`USER_SECTION`] are packaged (see [`stage_user_docs`]).
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

/// The warning for a guide the package lacks.
fn missing_guide(path: &str) -> String {
    format!("Warning: the package lacks the guide {path}.")
}

/// Stages the complete user documentation into `stage`, in the same layout
/// for every package (`cargo xtask dist`, `appimage`, and `gui-dist`): the
/// documentation index, every guide linked under its [`USER_SECTION`], and
/// the offline pages in [`SITE_DIR`], each at its path from the root.
///
/// A missing file never stops the build: it is left out and named in the
/// returned warnings, one per file, for [`report_doc_warnings`].
pub(crate) fn stage_user_docs(root: &Path, stage: &Path) -> anyhow::Result<Vec<String>> {
    let mut warnings = Vec::new();
    let guides = fs::read_to_string(root.join(DOCS_INDEX))
        .map(|index| user_guides(&index))
        .unwrap_or_default();
    for src in std::iter::once(DOCS_INDEX.to_owned()).chain(guides) {
        let path = root.join(&src);
        if path.is_file() {
            eci::copy(&path, &stage.join(&src))?;
        } else {
            warnings.push(missing_guide(&src));
        }
    }
    let site = root.join(SITE_DIR);
    if site.is_dir() {
        copy_tree(&site, &stage.join(SITE_DIR))?;
    } else {
        warnings.push(format!(
            "Warning: the package lacks the offline pages {SITE_DIR}."
        ));
    }
    Ok(warnings)
}

/// Prints each documentation warning on a line of its own, and adds them
/// to the job summary when `GITHUB_STEP_SUMMARY` names one, so the release
/// workflow shows them. Never fails: a summary that cannot be written is
/// noted and the build goes on.
pub(crate) fn report_doc_warnings(warnings: &[String]) {
    for w in warnings {
        println!("{w}");
    }
    if let Some(summary) = std::env::var_os("GITHUB_STEP_SUMMARY")
        && let Err(e) = append_summary(Path::new(&summary), warnings)
    {
        println!("Warning: the job summary could not be written: {e}.");
    }
}

/// Appends `warnings` to a Markdown job summary, one list item each.
fn append_summary(path: &Path, warnings: &[String]) -> std::io::Result<()> {
    use std::io::Write as _;
    if warnings.is_empty() {
        return Ok(());
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(f)?;
    for w in warnings {
        writeln!(f, "- {w}")?;
    }
    Ok(())
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
    crate::sizes::check(&eci::root(), &[archive])
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
    let build_dir = build_dir(&root);
    let out = absolute(out.unwrap_or_else(|| eci::target_dir(&root).join("dist")))?;
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
    report_doc_warnings(&stage_user_docs(&root, &stage)?);

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

/// Where release builds go: `target/dist-build` on Windows, where the
/// static C runtime would otherwise invalidate everyday builds, else the
/// target directory. `cargo xtask gui-dist` builds there too, so the two
/// packages share their compiled dependencies.
pub(crate) fn build_dir(root: &Path) -> PathBuf {
    if cfg!(windows) {
        eci::target_dir(root).join("dist-build")
    } else {
        eci::target_dir(root)
    }
}

/// `path` made absolute against the current folder. An `--out` folder is
/// used from tools that run in another folder (appimagetool runs in the
/// output folder), so a relative one would name the wrong place: the GUI
/// workflow's `--out gui-dist` became `gui-dist/gui-dist/...`.
pub(crate) fn absolute(path: PathBuf) -> anyhow::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()
            .context("reading the current folder")?
            .join(path))
    }
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
pub(crate) fn stage_notices(root: &Path, stage: &Path) -> anyhow::Result<()> {
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
pub(crate) fn check_notices(stage: &Path) -> anyhow::Result<()> {
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
pub(crate) fn cargo(root: &Path, build_dir: &Path) -> Command {
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

/// The cargo arguments that build one user binary. Each binary gets a
/// cargo run of its own: built together, cargo would unify their features
/// and give the reader `tw`'s, such as `textweaver-formats`' `url` and
/// `textweaver-ocr`'s `download`, which the reader leaves out.
fn binary_build_args(package: &str, bin: &str, target: Option<&str>) -> Vec<String> {
    let mut args: Vec<String> = [
        "build",
        "--locked",
        "--profile",
        PROFILE,
        "-p",
        package,
        "--bin",
        bin,
        "--features",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    args.push(features(package));
    if let Some(t) = target {
        args.extend(["--target".to_owned(), t.to_owned()]);
    }
    args
}

fn build_binaries(root: &Path, build_dir: &Path, target: Option<&str>) -> anyhow::Result<()> {
    for (package, bin) in BINARIES {
        let mut cmd = cargo(root, build_dir);
        cmd.args(binary_build_args(package, bin, target));
        run_tool(&mut cmd).with_context(|| format!("building {bin}"))?;
    }
    Ok(())
}

pub(crate) fn build_hosts(
    root: &Path,
    build_dir: &Path,
    hosts: &[HostBuild],
) -> anyhow::Result<()> {
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

pub(crate) fn run_tool(cmd: &mut Command) -> anyhow::Result<()> {
    let status = cmd
        .status()
        .with_context(|| format!("running {:?}", cmd.get_program()))?;
    if !status.success() {
        bail!("{:?} failed ({status})", cmd.get_program());
    }
    Ok(())
}

/// Zips `dir` so its files sit under `prefix/` in the archive.
pub(crate) fn zip_dir(dir: &Path, prefix: &str, zip_path: &Path) -> anyhow::Result<()> {
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

    /// Packages keep full optimization whatever the everyday `release`
    /// profile becomes: `[profile.dist]` in the root `Cargo.toml` sets fat
    /// LTO and one codegen unit itself, not by inheritance.
    #[test]
    fn the_dist_profile_sets_full_optimization_itself() {
        let toml = fs::read_to_string(eci::root().join("Cargo.toml")).unwrap();
        let section: Vec<&str> = toml
            .lines()
            .skip_while(|l| l.trim() != format!("[profile.{PROFILE}]"))
            .skip(1)
            .take_while(|l| !l.trim_start().starts_with('['))
            .map(str::trim)
            .collect();
        assert!(!section.is_empty(), "no [profile.{PROFILE}] in Cargo.toml");
        for want in ["lto = \"fat\"", "codegen-units = 1"] {
            assert!(
                section.contains(&want),
                "[profile.{PROFILE}] lacks `{want}`: {section:?}"
            );
        }
    }

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
    fn output_folders_are_made_absolute() {
        let a = absolute(PathBuf::from("gui-dist")).unwrap();
        assert!(a.is_absolute(), "{}", a.display());
        assert!(a.ends_with("gui-dist"));
        let here = std::env::current_dir().unwrap();
        assert_eq!(absolute(here.clone()).unwrap(), here);
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
        let f = features("textweaver-tui");
        assert!(f.contains("textweaver-tui/omnivox"), "{f}");
        assert!(!f.contains("textweaver-cli"), "{f}");
        let linux = cfg!(target_os = "linux");
        assert_eq!(f.contains("textweaver-tui/espeak"), linux, "{f}");
        assert_eq!(
            features("textweaver-cli").contains("textweaver-cli/speechd"),
            linux
        );
    }

    /// Each user binary is built in a cargo run of its own, with only its
    /// own package and features, so the reader never gets `tw`'s.
    #[test]
    fn each_binary_is_built_on_its_own() {
        for (package, bin) in BINARIES {
            let args = binary_build_args(package, bin, Some("x86_64-apple-darwin"));
            let packages: Vec<&str> = args
                .windows(2)
                .filter(|w| w[0] == "-p")
                .map(|w| w[1].as_str())
                .collect();
            assert_eq!(packages, [package], "{args:?}");
            let at = args.iter().position(|a| a == "--features").unwrap();
            for (other, _) in BINARIES.iter().filter(|(p, _)| *p != package) {
                assert!(!args[at + 1].contains(other), "{args:?}");
            }
            assert!(args.ends_with(&["--target".into(), "x86_64-apple-darwin".into()]));
            assert!(args.iter().any(|a| a == PROFILE), "{args:?}");
        }
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

    /// A guide missing from the index's list warns, in words naming the
    /// file, and the package still builds with everything else.
    #[test]
    fn a_missing_guide_warns_and_the_build_goes_on() {
        let tmp = std::env::temp_dir().join(format!("tw-dist-docs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let (root, stage) = (tmp.join("root"), tmp.join("stage"));
        fs::create_dir_all(root.join("docs/site")).unwrap();
        fs::write(
            root.join(DOCS_INDEX),
            "# Docs

## For users

- [Reading](reading.md)
- [Notes](notes.md)

## For contributors

- [Dev](dev/x.md)
",
        )
        .unwrap();
        fs::write(root.join("docs/notes.md"), "notes").unwrap();
        fs::write(root.join("docs/site/index.html"), "<p>site</p>").unwrap();

        let warnings = stage_user_docs(&root, &stage).unwrap();
        assert_eq!(
            warnings,
            ["Warning: the package lacks the guide docs/reading.md."]
        );
        for f in ["docs/README.md", "docs/notes.md", "docs/site/index.html"] {
            assert!(stage.join(f).is_file(), "{f} is not staged");
        }
        assert!(!stage.join("docs/dev/x.md").exists());

        let summary = tmp.join("summary.md");
        append_summary(&summary, &warnings).unwrap();
        append_summary(&summary, &[]).unwrap();
        assert_eq!(
            fs::read_to_string(&summary).unwrap(),
            "
- Warning: the package lacks the guide docs/reading.md.
"
        );

        // With nothing missing, no warning.
        fs::write(root.join("docs/reading.md"), "reading").unwrap();
        assert!(stage_user_docs(&root, &stage).unwrap().is_empty());
        let _ = fs::remove_dir_all(&tmp);
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
