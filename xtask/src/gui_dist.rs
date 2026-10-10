//! The app's part of the single package (B1-o2; ADR-0027, "Packaging";
//! docs/dev/releasing.md).
//!
//! Since beta 1 each release has one package per OS and CPU architecture
//! holding both programs, the app (`textweaver-gui`) and the terminal
//! program (`tw`, with `textweaver` as a second name), with the complete
//! docs. `cargo xtask dist` builds it; `cargo xtask gui-dist` is kept as
//! another name for the same task, so older notes and scripts still work.
//!
//! This module builds `textweaver-xilem` with the `dist` profile into the
//! same build folder as `tw` (the static C runtime on Windows), so the two
//! share their compiled dependencies, and stages it as `textweaver-gui`
//! with its guide (`GUI.md`) and the vendored Xilem's licence
//! ([`stage_gui`]). On macOS [`make_app`] wraps it in `textweaver.app` (an
//! `Info.plist`, signed ad hoc as Apple silicon requires), beside `tw` in
//! the same zip; with `--universal` the app joins the Apple silicon and
//! Intel builds with `lipo`, as `tw` does.
//!
//! The app is built with the speech engines `cargo xtask dist` builds into
//! `tw` for the platform (espeak-ng, speech-dispatcher, and Omnivox on
//! Linux; Omnivox elsewhere), for each engine feature the GUI crate
//! declares. A feature the crate does not declare yet is named in the
//! output and left out, so the package still builds. `--no-screenshot`
//! builds it without the screenshot harness (see [`SCREENSHOT`] for why
//! the package keeps it for now).
//!
//! The GUI needs no GTK or wxWidgets: winit and Vello use the system's
//! graphics stack (Direct3D 12 or Vulkan on Windows, Metal on macOS,
//! Vulkan or OpenGL on Linux), and the bundled fonts are compiled in.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context;

use crate::{dist, eci};

/// The package the GUI is built from, and its binary.
const PACKAGE: (&str, &str) = ("textweaver-xilem", "textweaver-xilem");
/// The name the binary is installed under.
pub(crate) const INSTALLED: &str = "textweaver-gui";
/// The app's own files in the package: (source, path in the package). The
/// rest (the licence, notices, quick start, guides, hosts, dictionaries)
/// is the package's, staged once by `cargo xtask dist`.
const FILES: [(&str, &str); 2] = [
    // How to start the app, its keys, and its settings.
    ("docs/gui.md", "GUI.md"),
    ("third_party/xilem/LICENSE", "licenses/xilem/LICENSE"),
];

/// The GUI crate's manifest, read for the engine features it declares.
const MANIFEST: &str = "crates/textweaver-xilem/Cargo.toml";

/// The GUI feature that draws `--screenshot` and `--review-screenshots`
/// with Vello's CPU renderer (`masonry_testing`, with `image`, PNG, and
/// `oxipng`). It is a default feature, and the package keeps it for now:
/// the release workflow's checks draw a `--screenshot` with the packaged
/// app on every platform, the only check that it renders where the
/// runners have no usable GPU. `--no-screenshot` leaves it out.
pub(crate) const SCREENSHOT: &str = "screenshot";

/// The macOS app bundle's folder in the package.
pub(crate) const APP: &str = "textweaver.app";

/// `cargo xtask gui-dist`: another name for `cargo xtask dist`, which
/// builds the one package holding the app and `tw`.
pub fn run() -> anyhow::Result<()> {
    println!("gui-dist builds the single package, the same as cargo xtask dist.");
    dist::run()
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

/// Builds the app and stages it into `stage` as [`INSTALLED`], with its
/// own [`FILES`]. `universal` (macOS) joins the Apple silicon and Intel
/// builds with `lipo`.
pub(crate) fn stage_gui(
    root: &Path,
    build_dir: &Path,
    stage: &Path,
    universal: bool,
    no_screenshot: bool,
) -> anyhow::Result<()> {
    let manifest =
        fs::read_to_string(root.join(MANIFEST)).with_context(|| format!("reading {MANIFEST}"))?;
    let (features, missing) = engine_features(&declared_features(&manifest), dist::engines());
    if !features.is_empty() {
        println!("App engines: {}", features.join(", "));
    }
    if !missing.is_empty() {
        println!(
            "Not in the app yet, left out: {} ({MANIFEST} declares no such feature)",
            missing.join(", ")
        );
    }
    let (features, no_defaults) =
        gui_features(&default_features(&manifest), &features, no_screenshot);
    if no_screenshot {
        println!("Left out: the screenshot harness (--no-screenshot)");
    }
    let build = |target: Option<&str>| -> anyhow::Result<()> {
        let mut cmd = dist::cargo(root, build_dir);
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
        dist::run_tool(&mut cmd).context("building the app")
    };
    let exe = std::env::consts::EXE_SUFFIX;
    let bin = stage.join(format!("{INSTALLED}{exe}"));
    if universal {
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
    for (src, dest) in FILES {
        eci::copy(&root.join(src), &stage.join(dest))?;
    }
    Ok(())
}

/// macOS: moves the staged app into `textweaver.app` (the binary in
/// `Contents/MacOS`, an `Info.plist`) and signs the bundle ad hoc. `tw`
/// and everything else stay beside it in the package folder.
pub(crate) fn make_app(stage: &Path, version: &str) -> anyhow::Result<()> {
    let app = stage.join(APP);
    let contents = app.join("Contents");
    fs::create_dir_all(contents.join("MacOS"))?;
    fs::rename(
        stage.join(INSTALLED),
        contents.join("MacOS").join(INSTALLED),
    )
    .context("moving the app into textweaver.app")?;
    fs::write(contents.join("Info.plist"), info_plist(version))?;
    dist::run_tool(
        Command::new("codesign")
            .args(["--force", "--deep", "--sign", "-"])
            .arg(&app),
    )
}

/// Where the app is in a staged package on this platform.
pub(crate) fn app_path() -> String {
    if cfg!(target_os = "macos") {
        format!("{APP}/Contents/MacOS/{INSTALLED}")
    } else {
        format!("{INSTALLED}{}", std::env::consts::EXE_SUFFIX)
    }
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

/// The app's Linux menu entry, in the tarball and the AppImage.
pub(crate) fn desktop_entry() -> &'static str {
    "[Desktop Entry]\nType=Application\nName=textweaver\nGenericName=Talking document reader\n\
     Comment=Read documents aloud, with a highlight that follows the spoken word\n\
     Exec=textweaver-gui %f\nTerminal=false\nIcon=textweaver\n\
     Categories=Utility;Accessibility;TextTools;\n"
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }

    #[test]
    fn the_app_bundle_names_its_executable() {
        let p = info_plist("1.2.3");
        assert!(p.contains("<key>CFBundleExecutable</key><string>textweaver-gui</string>"));
        assert!(p.contains("<string>1.2.3</string>"));
        assert!(p.contains("NSHighResolutionCapable"));
    }

    #[test]
    fn the_desktop_entry_starts_the_app() {
        assert!(desktop_entry().contains("Exec=textweaver-gui %f"));
        assert!(desktop_entry().contains("Terminal=false"));
    }

    #[test]
    fn packaged_files_exist() {
        let root = eci::root();
        for (src, _) in FILES {
            assert!(root.join(src).is_file(), "{src}");
        }
    }

    #[test]
    fn the_app_is_found_where_the_platform_puts_it() {
        let p = app_path();
        assert!(p.contains(INSTALLED), "{p}");
        assert_eq!(p.starts_with(APP), cfg!(target_os = "macos"), "{p}");
    }
}
