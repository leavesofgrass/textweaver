//! `cargo xtask gui-dist [--universal] [--out DIR]`: the Xilem GUI's own package
//! (ADR-0027, "Packaging"; docs/dev/releasing.md).
//!
//! Builds `textweaver-xilem` with the `dist` profile, as `cargo xtask dist`
//! builds the terminal programs (the static C runtime on Windows, in the
//! same build folder, so the two share their compiled dependencies), and
//! stages it as `textweaver-gui` in
//! `target/dist/textweaver-VERSION-PLATFORM-gui/` with what it needs to
//! speak and to be shared:
//!
//! - the engine hosts for the platform (Eloquence, SAPI 5, and DECtalk on
//!   Windows; Eloquence for Voxin and DECtalk on Linux) and the IBMTTS
//!   community dictionaries, beside the program, where the engines look;
//! - the define-word dictionary (`lexicon/`);
//! - the licence, the copyright notice (`NOTICE`), the third-party notices,
//!   and every data licence file the terminal package carries (the check
//!   fails if one is missing), and the vendored Xilem's licence;
//! - the quick start and the window's guide (`GUI.md`) at the top;
//! - the complete user documentation in `docs/`, in the same layout as the
//!   terminal package (`dist::stage_user_docs`): the index, every guide it
//!   lists for users, and the offline pages. A missing guide is named in a
//!   warning and the package still builds.
//!
//! The GUI is built with the speech engines `cargo xtask dist` builds into
//! the terminal programs for the platform (espeak-ng, speech-dispatcher,
//! and Omnivox on Linux; Omnivox elsewhere), for each engine feature the GUI
//! crate declares. A feature the crate does not declare yet is named in the
//! output and left out, so the package still builds. `--no-screenshot`
//! builds it without the screenshot harness (see [`SCREENSHOT`] for why
//! the package keeps it for now).
//!
//! Then it packages the folder:
//!
//! - Windows: a `.zip`;
//! - macOS: `textweaver.app` (the binary in `Contents/MacOS`, an
//!   `Info.plist`, signed ad hoc as Apple silicon requires) inside a
//!   `.zip`, for this Mac's architecture, or with `--universal` for Apple
//!   silicon and Intel in one binary (joined with `lipo`, as
//!   `cargo xtask dist --universal` does);
//! - Linux: a `.tar.gz`, and an AppImage of its own (with its `.zsync`)
//!   when `appimagetool` and its pinned runtime are found, as
//!   `cargo xtask appimage` finds them.
//!
//! Each package is checked against the size budget (`sizes.rs`), as the
//! terminal packages are.
//!
//! The names end in `-gui` (`textweaver-0.1.0-linux-x86_64-gui.AppImage`)
//! so that no pattern for the terminal packages matches them: not the
//! release workflow's, not the install scripts', and not the update
//! information inside AppImages already released
//! (`textweaver-*-linux-ARCH.AppImage.zsync`).
//!
//! The GUI needs no GTK or wxWidgets: winit and Vello use the system's
//! graphics stack (Direct3D 12 or Vulkan on Windows, Metal on macOS,
//! Vulkan or OpenGL on Linux), and the bundled fonts are compiled in.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};

use crate::{appimage, dist, eci};

/// The package the GUI is built from, and its binary.
const PACKAGE: (&str, &str) = ("textweaver-xilem", "textweaver-xilem");
/// The name the binary is installed under.
const INSTALLED: &str = "textweaver-gui";
/// Files copied into the package besides the notices and data licences
/// (`dist::stage_notices`): (source, path in the package).
const FILES: [(&str, &str); 4] = [
    ("LICENSE", "LICENSE"),
    ("docs/quickstart.md", "QUICKSTART.md"),
    // How to start the window, its keys, and its settings.
    ("docs/gui.md", "GUI.md"),
    ("third_party/xilem/LICENSE", "licenses/xilem/LICENSE"),
];
/// The GitHub owner and repository the AppImage's update information
/// points at.
const GITHUB: (&str, &str) = ("leavesofgrass", "textweaver");

/// The GUI crate's manifest, read for the engine features it declares.
const MANIFEST: &str = "crates/textweaver-xilem/Cargo.toml";

#[derive(Debug, Default, PartialEq, Eq)]
struct Args {
    universal: bool,
    out: Option<PathBuf>,
    /// Leave the screenshot harness ([`SCREENSHOT`]) out of the program.
    no_screenshot: bool,
}

/// The GUI feature that draws `--screenshot` and `--review-screenshots`
/// with Vello's CPU renderer (`masonry_testing`, with `image`, PNG, and
/// `oxipng`). It is a default feature, and the package keeps it for now:
/// the release workflow's GUI checks draw a `--screenshot` with the
/// packaged program on every platform, the only check that it renders
/// where the runners have no usable GPU. `--no-screenshot` leaves it out.
const SCREENSHOT: &str = "screenshot";

fn parse(args: &[String]) -> anyhow::Result<Args> {
    let mut out = Args::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--universal" => out.universal = true,
            "--no-screenshot" => out.no_screenshot = true,
            "--out" => out.out = Some(PathBuf::from(it.next().context("--out needs a directory")?)),
            other => bail!(
                "unknown argument {other} (usage: cargo xtask gui-dist [--universal] [--no-screenshot] [--out DIR])"
            ),
        }
    }
    if out.universal && !cfg!(target_os = "macos") {
        bail!("--universal is for macOS");
    }
    Ok(out)
}

/// The feature names in a manifest's `[features]` table.
fn declared_features(manifest: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_features = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_features = line == "[features]";
            continue;
        }
        if !in_features || line.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = line.split_once('=') {
            let name = name.trim();
            if !name.is_empty() && !name.contains(char::is_whitespace) {
                out.push(name.to_owned());
            }
        }
    }
    out
}

/// The features in a manifest's `default = [...]` line (on one line).
fn default_features(manifest: &str) -> Vec<String> {
    let mut in_features = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') && !line.starts_with("[\"") {
            in_features = line == "[features]";
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if in_features && name.trim() == "default" {
            return value
                .trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .map(|f| f.trim().trim_matches('"').to_owned())
                .filter(|f| !f.is_empty())
                .collect();
        }
    }
    Vec::new()
}

/// The `--features` list and whether to pass `--no-default-features`:
/// the engine features, and with `no_screenshot` the default features but
/// [`SCREENSHOT`], so the package keeps every other default.
fn gui_features(
    defaults: &[String],
    engines: &[String],
    no_screenshot: bool,
) -> (Vec<String>, bool) {
    if !no_screenshot {
        return (engines.to_vec(), false);
    }
    let mut out: Vec<String> = defaults
        .iter()
        .filter(|f| *f != SCREENSHOT)
        .map(|f| format!("{}/{f}", PACKAGE.0))
        .collect();
    out.extend(engines.iter().cloned());
    (out, true)
}

/// The platform's engines (as in the terminal package) split into those
/// the GUI crate declares, as `--features` entries, and those it does not.
fn engine_features(declared: &[String], engines: &[&str]) -> (Vec<String>, Vec<String>) {
    let (on, missing): (Vec<&str>, Vec<&str>) = engines
        .iter()
        .copied()
        .partition(|e| declared.iter().any(|d| d == e));
    (
        on.iter().map(|e| format!("{}/{e}", PACKAGE.0)).collect(),
        missing.iter().map(|e| (*e).to_owned()).collect(),
    )
}

/// The package folder name: the terminal package's name with `-gui` at the
/// end, so the terminal packages' patterns never match it.
fn package_name(version: &str, universal: bool) -> String {
    let arch = if universal {
        "universal"
    } else {
        std::env::consts::ARCH
    };
    format!("textweaver-{version}-{}-{arch}-gui", std::env::consts::OS)
}

/// The update information embedded in the GUI's AppImage: the newest
/// release or pre-release's GUI AppImage for this architecture.
fn update_information(arch: &str) -> String {
    format!(
        "gh-releases-zsync|{}|{}|latest-all|textweaver-*-linux-{arch}-gui.AppImage.zsync",
        GITHUB.0, GITHUB.1
    )
}

/// The macOS `Info.plist` for the `.app`.
fn info_plist(version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>textweaver</string>
  <key>CFBundleDisplayName</key><string>textweaver</string>
  <key>CFBundleIdentifier</key><string>org.textweaver.gui</string>
  <key>CFBundleExecutable</key><string>{INSTALLED}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>{version}</string>
  <key>CFBundleVersion</key><string>{version}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSSupportsAutomaticGraphicsSwitching</key><true/>
</dict>
</plist>
"#
    )
}

/// The Linux desktop entry for the AppImage.
fn desktop_entry() -> &'static str {
    "[Desktop Entry]\nType=Application\nName=textweaver\nGenericName=Talking document reader\n\
     Comment=Read documents aloud, with a highlight that follows the spoken word\n\
     Exec=textweaver-gui %f\nTerminal=false\nIcon=textweaver\n\
     Categories=Utility;Accessibility;TextTools;\n"
}

/// Where the package folder sits inside the AppImage. The program runs
/// from there, so it finds the engine hosts and dictionaries beside it, as
/// in the tarball.
const APPDIR_LIB: &str = "usr/lib/textweaver-gui";

/// The AppImage's entry point: starts the GUI in its package folder.
const APPRUN: &str = "#!/bin/sh\nhere=\"$(dirname \"$(readlink -f \"$0\")\")\"\nexec \"$here/usr/lib/textweaver-gui/textweaver-gui\" \"$@\"\n";

/// `cargo xtask gui-dist`.
pub fn run() -> anyhow::Result<()> {
    let args = parse(&std::env::args().skip(2).collect::<Vec<_>>())?;
    let root = eci::root();
    let version = env!("CARGO_PKG_VERSION");
    let build_dir = dist::build_dir(&root);
    let out = dist::absolute(
        args.out
            .unwrap_or_else(|| eci::target_dir(&root).join("dist")),
    )?;
    let name = package_name(version, args.universal);
    let stage = out.join(&name);
    if stage.exists() {
        fs::remove_dir_all(&stage).with_context(|| format!("clearing {}", stage.display()))?;
    }
    fs::create_dir_all(&stage)?;

    let manifest =
        fs::read_to_string(root.join(MANIFEST)).with_context(|| format!("reading {MANIFEST}"))?;
    let (features, missing) = engine_features(&declared_features(&manifest), dist::engines());
    if !features.is_empty() {
        println!("GUI engines: {}", features.join(", "));
    }
    if !missing.is_empty() {
        println!(
            "Not in the GUI yet, left out: {} ({MANIFEST} declares no such feature)",
            missing.join(", ")
        );
    }
    let (features, no_defaults) =
        gui_features(&default_features(&manifest), &features, args.no_screenshot);
    if args.no_screenshot {
        println!("Left out: the screenshot harness (--no-screenshot)");
    }
    let build = |target: Option<&str>| -> anyhow::Result<()> {
        let mut cmd = dist::cargo(&root, &build_dir);
        cmd.args([
            "build",
            "--locked",
            "--profile",
            dist::PROFILE,
            "-p",
            PACKAGE.0,
            "--bin",
            PACKAGE.1,
        ]);
        if no_defaults {
            cmd.arg("--no-default-features");
        }
        if !features.is_empty() {
            cmd.arg("--features").arg(features.join(","));
        }
        if let Some(t) = target {
            cmd.args(["--target", t]);
        }
        dist::run_tool(&mut cmd).context("building the GUI")
    };
    let exe = std::env::consts::EXE_SUFFIX;
    let bin = stage.join(format!("{INSTALLED}{exe}"));
    if args.universal {
        for t in dist::MAC_TARGETS {
            build(Some(t))?;
        }
        let parts: Vec<PathBuf> = dist::MAC_TARGETS
            .iter()
            .map(|t| build_dir.join(t).join(dist::PROFILE).join(PACKAGE.1))
            .collect();
        dist::run_tool(
            Command::new("lipo")
                .arg("-create")
                .args(&parts)
                .arg("-output")
                .arg(&bin),
        )?;
        println!("installed {} (universal)", bin.display());
    } else {
        build(None)?;
        let built = build_dir
            .join(dist::PROFILE)
            .join(format!("{}{exe}", PACKAGE.1));
        eci::copy(&built, &bin)?;
    }

    // The engine hosts and the dictionaries, as in the terminal package:
    // the engines look for them beside the program. macOS speaks through
    // Apple's own voices in process.
    if !cfg!(target_os = "macos") {
        let hosts = eci::all_hosts();
        dist::build_hosts(&root, &build_dir, &hosts)?;
        for h in &hosts {
            eci::copy(
                &h.built(&build_dir, dist::PROFILE),
                &stage.join(h.installed_name()),
            )?;
        }
        eci::copy_dictionaries(&root, &stage)?;
    }
    for (src, dest) in FILES.iter().chain(dist::DATA_FILES.iter()) {
        eci::copy(&root.join(src), &stage.join(dest))?;
    }
    dist::report_doc_warnings(&dist::stage_user_docs(&root, &stage)?);
    dist::stage_notices(&root, &stage)?;
    dist::check_notices(&stage)?;

    let packages = if cfg!(target_os = "macos") {
        let app = stage.join("textweaver.app");
        let contents = app.join("Contents");
        fs::create_dir_all(contents.join("MacOS"))?;
        fs::rename(&bin, contents.join("MacOS").join(INSTALLED))?;
        fs::write(contents.join("Info.plist"), info_plist(version))?;
        dist::run_tool(
            Command::new("codesign")
                .args(["--force", "--deep", "--sign", "-"])
                .arg(&app),
        )?;
        let zip = out.join(format!("{name}.zip"));
        let _ = fs::remove_file(&zip);
        dist::run_tool(
            Command::new("ditto")
                .args(["-c", "-k", "--keepParent"])
                .arg(&stage)
                .arg(&zip),
        )?;
        println!("package {}", zip.display());
        vec![zip]
    } else if cfg!(windows) {
        let zip = out.join(format!("{name}.zip"));
        dist::zip_dir(&stage, &name, &zip)?;
        println!("package {}", zip.display());
        vec![zip]
    } else {
        let tgz = out.join(format!("{name}.tar.gz"));
        dist::run_tool(
            Command::new("tar")
                .arg("-czf")
                .arg(&tgz)
                .arg("-C")
                .arg(&out)
                .arg(&name),
        )?;
        println!("package {}", tgz.display());
        let image = build_appimage(&root, &stage, &out, &name)?;
        std::iter::once(tgz).chain(image).collect()
    };
    crate::sizes::check(&root, &packages)
}

/// Lays out the GUI's AppDir: the package under [`APPDIR_LIB`], `AppRun`,
/// the desktop entry, and the icon.
fn build_appdir(root: &Path, package: &Path, appdir: &Path) -> anyhow::Result<()> {
    if appdir.exists() {
        fs::remove_dir_all(appdir).with_context(|| format!("clearing {}", appdir.display()))?;
    }
    dist::copy_tree(package, &appdir.join(APPDIR_LIB))?;
    let apprun = appdir.join("AppRun");
    fs::write(&apprun, APPRUN)?;
    appimage::executable(&apprun)?;
    appimage::executable(&appdir.join(APPDIR_LIB).join(INSTALLED))?;
    for dest in [
        appdir.join("textweaver-gui.desktop"),
        appdir.join("usr/share/applications/textweaver-gui.desktop"),
    ] {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, desktop_entry()).with_context(|| format!("writing {}", dest.display()))?;
    }
    let icon = root.join("scripts/linux/textweaver.svg");
    for dest in [
        appdir.join("textweaver.svg"),
        appdir.join(".DirIcon"),
        appdir.join("usr/share/icons/hicolor/scalable/apps/textweaver.svg"),
    ] {
        eci::copy(&icon, &dest)?;
    }
    Ok(())
}

/// The GUI's own AppImage, when appimagetool is available (the release
/// workflow builds in the `docker/appimage` image, which has it). Returns
/// the AppImage, or `None` when appimagetool was not found.
fn build_appimage(
    root: &Path,
    stage: &Path,
    out: &Path,
    name: &str,
) -> anyhow::Result<Option<PathBuf>> {
    let Some(tool) = appimage::find_tool() else {
        println!(
            "no AppImage: set APPIMAGETOOL (docker/appimage/fetch-tools.sh fetches it and its runtime)"
        );
        return Ok(None);
    };
    let runtime = appimage::find_runtime(&tool)?;
    let appdir = out.join("textweaver-gui.AppDir");
    build_appdir(root, stage, &appdir)?;
    let image = out.join(format!("{name}.AppImage"));
    let _ = fs::remove_file(&image);
    let zsync = out.join(format!("{name}.AppImage.zsync"));
    let _ = fs::remove_file(&zsync);
    let arch = appimage::arch();
    dist::run_tool(
        Command::new(&tool)
            .arg("--no-appstream")
            .arg("--runtime-file")
            .arg(&runtime)
            .arg("--updateinformation")
            .arg(update_information(arch))
            .arg(&appdir)
            .arg(&image)
            .current_dir(out)
            .env("ARCH", arch)
            .env("VERSION", env!("CARGO_PKG_VERSION"))
            // appimagetool is itself an AppImage; without FUSE
            // (containers) it extracts itself and runs.
            .env("APPIMAGE_EXTRACT_AND_RUN", "1"),
    )
    .context("appimagetool")?;
    if !image.is_file() {
        bail!("appimagetool did not write {}", image.display());
    }
    println!("package {}", image.display());
    if zsync.is_file() {
        println!("package {}", zsync.display());
    }
    Ok(Some(image))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments() {
        assert_eq!(parse(&[]).unwrap(), Args::default());
        let a = parse(&["--out".into(), "x".into()]).unwrap();
        assert_eq!(a.out, Some(PathBuf::from("x")));
        assert!(parse(&["--nope".into()]).is_err());
        assert_eq!(
            parse(&["--universal".into()]).is_ok(),
            cfg!(target_os = "macos")
        );
    }

    #[test]
    fn engine_features_follow_what_the_gui_declares() {
        let manifest = "[package]\nname = \"x\"\n\n[features]\n# a comment\ndefault = [\"a\"]\nespeak = [\"textweaver-app/espeak\"]\nomnivox = []\n\n[dependencies]\nspeechd = \"1\"\n";
        let declared = declared_features(manifest);
        assert_eq!(declared, ["default", "espeak", "omnivox"]);
        let (on, missing) = engine_features(&declared, &["omnivox", "speechd", "espeak"]);
        assert_eq!(on, ["textweaver-xilem/omnivox", "textweaver-xilem/espeak"]);
        assert_eq!(missing, ["speechd"]);
        let (on, missing) = engine_features(&[], &["omnivox"]);
        assert!(on.is_empty());
        assert_eq!(missing, ["omnivox"]);
    }

    /// The real manifest parses, and its default features are found.
    #[test]
    fn the_gui_manifest_declares_features() {
        let manifest = fs::read_to_string(eci::root().join(MANIFEST)).unwrap();
        let declared = declared_features(&manifest);
        assert!(declared.iter().any(|f| f == "default"));
        let defaults = default_features(&manifest);
        for f in [SCREENSHOT, "renderer-vello", "publish"] {
            assert!(defaults.iter().any(|d| d == f), "{f} not in {defaults:?}");
        }
        // Every default is a declared feature, so the list without the
        // harness names only real features.
        for d in &defaults {
            assert!(declared.contains(d), "{d} is not declared");
        }
    }

    #[test]
    fn the_screenshot_harness_can_be_left_out() {
        let defaults: Vec<String> = ["screenshot", "renderer-vello", "opus"]
            .map(String::from)
            .to_vec();
        let engines = vec!["textweaver-xilem/omnivox".to_owned()];
        // By default the package keeps the defaults, the harness included.
        assert_eq!(
            gui_features(&defaults, &engines, false),
            (engines.clone(), false)
        );
        let (f, no_defaults) = gui_features(&defaults, &engines, true);
        assert!(no_defaults);
        assert_eq!(
            f,
            [
                "textweaver-xilem/renderer-vello",
                "textweaver-xilem/opus",
                "textweaver-xilem/omnivox"
            ]
        );
        assert_eq!(
            default_features("[features]\ndefault = [\"a\", \"b\"]\n[dependencies]\ndefault = 1\n"),
            ["a", "b"]
        );
        assert!(default_features("[dependencies]\ndefault = [\"a\"]\n").is_empty());
        assert!(parse(&["--no-screenshot".into()]).unwrap().no_screenshot);
    }

    #[test]
    fn the_app_bundle_names_its_executable() {
        let p = info_plist("1.2.3");
        assert!(p.contains("<key>CFBundleExecutable</key><string>textweaver-gui</string>"));
        assert!(p.contains("<string>1.2.3</string>"));
        assert!(p.contains("NSHighResolutionCapable"));
    }

    #[test]
    fn names_and_entries() {
        let name = package_name("0.1.0", false);
        assert!(name.starts_with("textweaver-0.1.0-"), "{name}");
        assert!(name.ends_with("-gui"), "{name}");
        assert!(package_name("0.1.0", true).ends_with("-universal-gui"));
        assert!(desktop_entry().contains("Exec=textweaver-gui %f"));
        assert!(desktop_entry().contains("Terminal=false"));
        assert!(APPRUN.contains("usr/lib/textweaver-gui/textweaver-gui"));
        assert!(
            update_information("aarch64")
                .ends_with("textweaver-*-linux-aarch64-gui.AppImage.zsync")
        );
    }

    /// A shell or glob pattern with `*` wildcards only.
    fn glob(pattern: &str, name: &str) -> bool {
        let parts: Vec<&str> = pattern.split('*').collect();
        let (first, last) = (parts[0], parts[parts.len() - 1]);
        if !name.starts_with(first)
            || !name.ends_with(last)
            || name.len() < first.len() + last.len()
        {
            return false;
        }
        let mut rest = &name[first.len()..name.len() - last.len()];
        for p in &parts[1..parts.len() - 1] {
            match rest.find(p) {
                Some(i) => rest = &rest[i + p.len()..],
                None => return false,
            }
        }
        true
    }

    /// The patterns that pick the terminal packages (the release workflow,
    /// the distribution check, the update information in released
    /// AppImages) never pick a GUI package.
    #[test]
    fn terminal_patterns_never_match_the_gui_packages() {
        let terminal = [
            "textweaver-*-windows-x86_64.zip",
            "textweaver-*-macos-universal.tar.gz",
            "textweaver-*-linux-x86_64.AppImage",
            "textweaver-*-linux-x86_64.AppImage.zsync",
            "textweaver-*-linux-x86_64.tar.gz",
            "textweaver-*-linux-aarch64.AppImage",
            "textweaver-*-linux-aarch64.tar.gz",
        ];
        let v = "0.1.0-alpha.5";
        let gui = [
            format!("textweaver-{v}-windows-x86_64-gui.zip"),
            format!("textweaver-{v}-macos-aarch64-gui.zip"),
            format!("textweaver-{v}-macos-universal-gui.zip"),
            format!("textweaver-{v}-linux-x86_64-gui.AppImage"),
            format!("textweaver-{v}-linux-x86_64-gui.AppImage.zsync"),
            format!("textweaver-{v}-linux-x86_64-gui.tar.gz"),
            format!("textweaver-{v}-linux-aarch64-gui.AppImage"),
            format!("textweaver-{v}-linux-aarch64-gui.tar.gz"),
        ];
        for t in terminal {
            for g in &gui {
                assert!(!glob(t, g), "{t} matches {g}");
            }
            // The pattern still picks its own package.
            let own = t.replace('*', v);
            assert!(glob(t, &own), "{t} misses {own}");
        }
        // The checksums cover both: tools/release-upload.sh sums textweaver-*.
        for g in &gui {
            assert!(glob("textweaver-*", g));
        }
        assert!(glob(
            "textweaver-*-linux-x86_64-gui.AppImage.zsync",
            &gui[4]
        ));
    }

    #[test]
    fn packaged_files_exist() {
        let root = eci::root();
        for (src, _) in FILES.iter().chain(dist::DATA_FILES.iter()) {
            assert!(root.join(src).is_file(), "{src}");
        }
    }

    #[test]
    fn the_appdir_runs_the_program_beside_its_hosts() {
        let root = eci::root();
        let tmp = std::env::temp_dir().join(format!("tw-gui-appdir-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let package = tmp.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join(INSTALLED), "gui").unwrap();
        fs::write(package.join("eci-host"), "host").unwrap();
        let appdir = tmp.join("textweaver-gui.AppDir");
        build_appdir(&root, &package, &appdir).unwrap();
        for f in [
            "AppRun",
            "textweaver-gui.desktop",
            "textweaver.svg",
            ".DirIcon",
            "usr/lib/textweaver-gui/textweaver-gui",
            "usr/lib/textweaver-gui/eci-host",
            "usr/share/applications/textweaver-gui.desktop",
        ] {
            assert!(appdir.join(f).is_file(), "{f} is missing");
        }
        let _ = fs::remove_dir_all(&tmp);
    }
}
