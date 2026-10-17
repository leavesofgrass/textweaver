//! `cargo xtask dist [--universal] [--no-screenshot] [--out DIR]`: the
//! release package for this platform (docs/dev/releasing.md). `cargo
//! xtask gui-dist` is another name for it.
//!
//! One package per OS and CPU architecture holds one program (beta 3):
//! `textweaver`, the app by default and the terminal program with `--tw`
//! (`gui_dist.rs`), with `tw` beside it: a link to it on Linux and macOS,
//! and on Windows `tw.exe`, a small console launcher
//! (`textweaver-launcher`) that runs `textweaver.exe --tw` and waits. It
//! builds them with the `dist` profile (the release profile with fat
//! LTO), and the combined 32-bit engine host on Windows, and stages them
//! with the pronunciation dictionaries, the licence, the copyright notice
//! (`NOTICE`), the third-party notices and licence files, and the
//! complete user guides in `target/dist/textweaver-VERSION-PLATFORM/`,
//! then archives the folder: a `.zip` on Windows and macOS (where the
//! program is in `textweaver.app`), a `.tar.gz` on Linux. It fails if a
//! program, an engine host, or a notice is missing from the staged
//! folder, if the Windows package holds a program it should not (an
//! earlier release's `textweaver-gui.exe`), or if the archive grew more
//! than 10 percent over the last release's without a note (`sizes.rs`). A user
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

/// The terminal program's name: on Windows the launcher, elsewhere a link
/// to the program.
const TW: &str = "tw";
/// The Windows launcher's package and binary, staged as `tw.exe`.
const LAUNCHER: (&str, &str) = ("textweaver-launcher", "tw-launcher");
/// Programs of earlier releases that a Windows package must not hold:
/// the app before beta 3 (now `textweaver.exe` itself).
const RETIRED: [&str; 1] = ["textweaver-gui"];
/// Speech engines built into the program on this platform. Nothing
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
/// Example files the user guides describe, packaged with them at their
/// paths from the root.
const DOC_EXAMPLES: [&str; 1] = ["docs/examples/developer-profile.toml"];
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
/// documentation index, every guide linked under its [`USER_SECTION`], the
/// examples the guides describe ([`DOC_EXAMPLES`]), and the offline pages
/// in [`SITE_DIR`], each at its path from the root.
///
/// A missing file never stops the build: it is left out and named in the
/// returned warnings, one per file, for [`report_doc_warnings`].
pub(crate) fn stage_user_docs(root: &Path, stage: &Path) -> anyhow::Result<Vec<String>> {
    let mut warnings = Vec::new();
    let guides = fs::read_to_string(root.join(DOCS_INDEX))
        .map(|index| user_guides(&index))
        .unwrap_or_default();
    let examples = DOC_EXAMPLES.iter().map(|s| (*s).to_owned());
    for src in std::iter::once(DOCS_INDEX.to_owned())
        .chain(guides)
        .chain(examples)
    {
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

/// Copies every file under `src` into `dest`, keeping the layout. A link
/// (the `textweaver` link to `tw`) is made again as a link, never copied
/// as a second program.
pub(crate) fn copy_tree(src: &Path, dest: &Path) -> anyhow::Result<()> {
    let mut files = Vec::new();
    collect(src, &mut files)?;
    for f in files {
        let to = dest.join(f.strip_prefix(src)?);
        if fs::symlink_metadata(&f)?.file_type().is_symlink() {
            copy_link(&f, &to)?;
        } else {
            eci::copy(&f, &to)?;
        }
    }
    Ok(())
}

/// Makes `to` a link with the same target as the link `from`.
#[cfg(unix)]
fn copy_link(from: &Path, to: &Path) -> anyhow::Result<()> {
    let target = fs::read_link(from).with_context(|| format!("reading {}", from.display()))?;
    if let Some(dir) = to.parent() {
        fs::create_dir_all(dir)?;
    }
    std::os::unix::fs::symlink(&target, to).with_context(|| format!("linking {}", to.display()))
}

/// Windows packages hold no links; a link met here is copied as its file.
#[cfg(not(unix))]
fn copy_link(from: &Path, to: &Path) -> anyhow::Result<()> {
    eci::copy(from, to)
}

/// Parsed arguments.
#[derive(Debug, Default, PartialEq, Eq)]
struct Args {
    universal: bool,
    out: Option<PathBuf>,
    /// Leave the app's screenshot harness out (`gui_dist::SCREENSHOT`).
    no_screenshot: bool,
}

fn parse(args: &[String]) -> anyhow::Result<Args> {
    let mut out = Args::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--universal" => out.universal = true,
            "--no-screenshot" => out.no_screenshot = true,
            "--out" => out.out = Some(PathBuf::from(it.next().context("--out needs a directory")?)),
            other => bail!(
                "unknown argument {other} (usage: cargo xtask dist [--universal] [--no-screenshot] [--out DIR])"
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
    let staged = stage(args.out.clone(), args.universal, args.no_screenshot)?;
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

/// Builds the programs and hosts and stages the package folder (everything
/// but the archive). `out` defaults to `target/dist`.
pub(crate) fn stage(
    out: Option<PathBuf>,
    universal: bool,
    no_screenshot: bool,
) -> anyhow::Result<Staged> {
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

    // The one program, textweaver.
    crate::gui_dist::stage_gui(&root, &build_dir, &stage, universal, no_screenshot)?;
    // tw: the launcher on Windows; a link elsewhere, made once the
    // program is where it stays (inside textweaver.app on macOS).
    if cfg!(windows) {
        build_launcher(&root, &build_dir)?;
        let exe = std::env::consts::EXE_SUFFIX;
        eci::copy(
            &build_dir.join(PROFILE).join(format!("{}{exe}", LAUNCHER.1)),
            &stage.join(format!("{TW}{exe}")),
        )?;
    }

    // Engine hosts: the programs run them as a copy of themselves; the
    // one host program is the combined 32-bit host on Windows. The
    // Eloquence dictionaries go beside the programs on Windows and Linux
    // (Voxin). macOS speaks through Apple's own voices in process.
    if !cfg!(target_os = "macos") {
        let hosts = eci::all_hosts();
        if !hosts.is_empty() {
            build_hosts(&root, &build_dir, &hosts)?;
        }
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
    if cfg!(target_os = "macos") {
        crate::gui_dist::make_app(&stage, version)?;
    }
    link_names(&stage)?;

    check_programs(&stage)?;
    check_notices(&stage)?;
    Ok(Staged {
        name,
        dir: stage,
        out,
    })
}

/// Where release builds go: `target/dist-build` on Windows, where the
/// static C runtime would otherwise invalidate everyday builds, else the
/// target directory. Every build of the package (the program, the
/// launcher, the hosts) goes there.
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

/// Archives a staged package: a `.zip` on Windows and macOS, a `.tar.gz`
/// on Linux. The tarball and the macOS zip (made with `ditto`) keep the
/// `tw` link.
pub(crate) fn archive(staged: &Staged) -> anyhow::Result<PathBuf> {
    let Staged { name, dir, out } = staged;
    if cfg!(windows) {
        let zip = out.join(format!("{name}.zip"));
        zip_dir(dir, name, &zip)?;
        Ok(zip)
    } else if cfg!(target_os = "macos") {
        let zip = out.join(format!("{name}.zip"));
        let _ = fs::remove_file(&zip);
        run_tool(
            Command::new("ditto")
                .args(["-c", "-k", "--keepParent"])
                .arg(dir)
                .arg(&zip),
        )?;
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

/// The Linux menu entries, the app's and the terminal reader's, and the
/// icon: (source, path in the package).
pub(crate) const LINUX_DESKTOP_FILES: [(&str, &str); 3] = [
    (
        "scripts/linux/textweaver.desktop",
        "share/applications/textweaver.desktop",
    ),
    (
        "scripts/linux/textweaver-terminal.desktop",
        "share/applications/textweaver-terminal.desktop",
    ),
    (
        "scripts/linux/textweaver.svg",
        "share/icons/hicolor/scalable/apps/textweaver.svg",
    ),
];

/// The links in a Linux or macOS package, (name, target): `tw` beside the
/// program on Linux; on macOS `tw` and `textweaver` beside
/// `textweaver.app`, pointing at the program inside it.
#[cfg(unix)]
fn links() -> Vec<(&'static str, String)> {
    if cfg!(target_os = "macos") {
        let app = crate::gui_dist::app_path();
        vec![(TW, app.clone()), (crate::gui_dist::INSTALLED, app)]
    } else {
        vec![(TW, crate::gui_dist::INSTALLED.to_owned())]
    }
}

/// Makes the [`links`] in `stage`, on Linux and macOS. On Windows the
/// launcher is staged as `tw.exe` instead.
#[cfg(unix)]
fn link_names(stage: &Path) -> anyhow::Result<()> {
    for (name, target) in links() {
        let link = stage.join(name);
        std::os::unix::fs::symlink(&target, &link)
            .with_context(|| format!("linking {} to {target}", link.display()))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn link_names(_: &Path) -> anyhow::Result<()> {
    Ok(())
}

/// The programs a package must hold, as paths in it: `textweaver`, `tw`
/// (the launcher on Windows, a link elsewhere), and the combined 32-bit
/// engine host on Windows.
pub(crate) fn required_programs() -> Vec<String> {
    let exe = std::env::consts::EXE_SUFFIX;
    let mut out = vec![crate::gui_dist::app_path(), format!("{TW}{exe}")];
    if cfg!(target_os = "macos") {
        out.push(crate::gui_dist::INSTALLED.to_owned());
    } else {
        out.extend(eci::all_hosts().iter().map(|h| h.installed_name()));
    }
    out
}

/// The programs at the top of a Windows package: every `.exe` there.
fn top_programs(stage: &Path) -> anyhow::Result<Vec<String>> {
    let mut out: Vec<String> = fs::read_dir(stage)?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.to_ascii_lowercase().ends_with(".exe"))
        .collect();
    out.sort();
    Ok(out)
}

/// Fails unless `stage` holds every program in [`required_programs`] (a
/// link counts when it is a link, whatever it points at), and, on
/// Windows, unless those are the only programs at its top: an earlier
/// release's `textweaver-gui.exe`, or any other stray program, fails.
pub(crate) fn check_programs(stage: &Path) -> anyhow::Result<()> {
    let required = required_programs();
    let missing: Vec<&String> = required
        .iter()
        .filter(|p| fs::symlink_metadata(stage.join(p)).is_err())
        .collect();
    if !missing.is_empty() {
        let names: Vec<&str> = missing.iter().map(|s| s.as_str()).collect();
        bail!("the package lacks {}", names.join(", "));
    }
    if cfg!(windows) {
        let extra: Vec<String> = top_programs(stage)?
            .into_iter()
            .filter(|p| !required.iter().any(|r| r.eq_ignore_ascii_case(p)))
            .collect();
        if !extra.is_empty() {
            bail!(
                "the package holds programs it should not: {} (it holds textweaver.exe, tw.exe, and the hosts; {} retired in beta 3)",
                extra.join(", "),
                RETIRED.join(", ")
            );
        }
    }
    Ok(())
}

/// Copies textweaver's copyright notice, the third-party notices, and the
/// data licence files into `stage`.
pub(crate) fn stage_notices(root: &Path, stage: &Path) -> anyhow::Result<()> {
    eci::copy(
        &root.join(crate::notices::NOTICE),
        &stage.join(crate::notices::NOTICE),
    )?;
    eci::copy(
        &root.join(crate::notices::NOTICES),
        &stage.join(crate::notices::NOTICES),
    )?;
    for (src, dest) in LICENCE_FILES {
        eci::copy(&root.join(src), &stage.join(dest))?;
    }
    Ok(())
}

/// Fails unless `stage` holds the licence, the copyright notice, the
/// notices, and every data licence file: a package must never ship without
/// them.
pub(crate) fn check_notices(stage: &Path) -> anyhow::Result<()> {
    let required = ["LICENSE", crate::notices::NOTICE, crate::notices::NOTICES]
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

/// The cargo arguments that build the Windows launcher, `tw.exe`, in a run
/// of its own: it depends on nothing but the standard library.
fn launcher_build_args() -> Vec<String> {
    [
        "build",
        "--locked",
        "--profile",
        PROFILE,
        "-p",
        LAUNCHER.0,
        "--bin",
        LAUNCHER.1,
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect()
}

fn build_launcher(root: &Path, build_dir: &Path) -> anyhow::Result<()> {
    let mut cmd = cargo(root, build_dir);
    cmd.args(launcher_build_args());
    run_tool(&mut cmd).context("building tw.exe, the launcher")
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
        // A link is listed as itself, not followed.
        if fs::symlink_metadata(&path)?.file_type().is_dir() {
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
        assert!(parse(&["--no-screenshot".into()]).unwrap().no_screenshot);
        assert!(parse(&["--bogus".into()]).is_err());
        assert_eq!(
            parse(&["--universal".into()]).is_ok(),
            cfg!(target_os = "macos")
        );
    }

    /// The launcher is built in a cargo run of its own package, so it
    /// links nothing but the standard library.
    #[test]
    fn the_launcher_is_built_on_its_own() {
        let args = launcher_build_args();
        let after = |flag: &str| -> Vec<String> {
            args.windows(2)
                .filter(|w| w[0] == flag)
                .map(|w| w[1].clone())
                .collect()
        };
        assert_eq!(after("-p"), [LAUNCHER.0], "{args:?}");
        assert_eq!(after("--bin"), [LAUNCHER.1], "{args:?}");
        assert!(args.iter().any(|a| a == PROFILE), "{args:?}");
        assert!(!args.iter().any(|a| a == "--features"), "{args:?}");
    }

    /// The launcher's manifest names no dependency, so `tw.exe` stays
    /// small.
    #[test]
    fn the_launcher_depends_on_nothing() {
        let toml =
            fs::read_to_string(eci::root().join("crates/textweaver-launcher/Cargo.toml")).unwrap();
        let deps: Vec<&str> = toml
            .lines()
            .skip_while(|l| l.trim() != "[dependencies]")
            .skip(1)
            .take_while(|l| !l.trim_start().starts_with('['))
            .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
            .collect();
        assert!(deps.is_empty(), "{deps:?}");
    }

    /// A package without textweaver, `tw`, or a host fails the check,
    /// naming what is missing; on Windows so does one holding an earlier
    /// release's app.
    #[test]
    fn a_package_must_hold_the_program_tw_and_the_hosts() {
        let stage = std::env::temp_dir().join(format!("tw-dist-programs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&stage);
        fs::create_dir_all(&stage).unwrap();
        let err = check_programs(&stage).unwrap_err().to_string();
        assert!(err.contains(TW), "{err}");
        assert!(err.contains(crate::gui_dist::INSTALLED), "{err}");
        if cfg!(windows) {
            assert!(err.contains("textweaver-host-x86.exe"), "{err}");
            assert_eq!(
                required_programs(),
                ["textweaver.exe", "tw.exe", "textweaver-host-x86.exe"]
            );
        }
        for p in required_programs() {
            let path = stage.join(p);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "x").unwrap();
        }
        check_programs(&stage).unwrap();
        if cfg!(windows) {
            fs::write(stage.join("textweaver-gui.exe"), "x").unwrap();
            let err = check_programs(&stage).unwrap_err().to_string();
            assert!(err.contains("textweaver-gui.exe"), "{err}");
        }
        let _ = fs::remove_dir_all(&stage);
    }

    /// On Linux and macOS, `tw` is a link to the program, and copying the
    /// package (into the AppImage's folder) keeps it a link.
    #[cfg(unix)]
    #[test]
    fn tw_is_a_link_and_stays_one() {
        let tmp = std::env::temp_dir().join(format!("tw-dist-link-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let stage = tmp.join("stage");
        fs::create_dir_all(&stage).unwrap();
        link_names(&stage).unwrap();
        let copy = tmp.join("copy");
        copy_tree(&stage, &copy).unwrap();
        let link = copy.join(TW);
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::read_link(&link).unwrap(),
            Path::new(&crate::gui_dist::app_path())
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn linux_desktop_files_exist() {
        let root = eci::root();
        for (src, _) in LINUX_DESKTOP_FILES {
            assert!(root.join(src).is_file(), "{src} is packaged but missing");
        }
        let line = |file: &str, want: &str| {
            let desktop = fs::read_to_string(root.join(file)).unwrap();
            assert!(
                desktop.lines().any(|l| l.trim_end() == want),
                "{file} lacks {want}"
            );
        };
        // The app's entry, then the terminal reader's.
        let (app, terminal) = (LINUX_DESKTOP_FILES[0].0, LINUX_DESKTOP_FILES[1].0);
        line(app, "Terminal=false");
        line(app, "Exec=textweaver %f");
        line(app, "Icon=textweaver");
        line(terminal, "Terminal=true");
        line(terminal, "Exec=tw %f");
        line(terminal, "Icon=textweaver");
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
        fs::create_dir_all(root.join("docs/examples")).unwrap();
        fs::write(root.join(DOC_EXAMPLES[0]), "textweaver_profiles = 1\n").unwrap();

        let warnings = stage_user_docs(&root, &stage).unwrap();
        assert_eq!(
            warnings,
            ["Warning: the package lacks the guide docs/reading.md."]
        );
        for f in [
            "docs/README.md",
            "docs/notes.md",
            "docs/site/index.html",
            DOC_EXAMPLES[0],
        ] {
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
        assert!(stage.join("NOTICE").is_file());
        assert!(stage.join("licenses/scowl/Copyright").is_file());
        assert!(stage.join("licenses/lexicon/WORDNET-LICENSE").is_file());
        assert!(stage.join("licenses/fonts/opendyslexic/OFL.txt").is_file());
        let _ = fs::remove_dir_all(&stage);
    }
}
