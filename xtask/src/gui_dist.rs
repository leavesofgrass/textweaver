//! `cargo xtask gui-dist [--out DIR]`: the Xilem GUI's own package
//! (ADR-0027, "Packaging").
//!
//! Builds `textweaver-xilem` with the `dist` profile and stages it as
//! `textweaver-gui` with the licence, the third-party notices, the font
//! licences, and the quick start, in
//! `target/dist/textweaver-gui-VERSION-PLATFORM/`, then packages it:
//!
//! - Windows: a `.zip`;
//! - macOS: `textweaver.app` (the binary in `Contents/MacOS`, an
//!   `Info.plist`, signed ad hoc as Apple silicon requires) inside a
//!   `.zip`;
//! - Linux: a `.tar.gz`, and an AppImage of its own when `appimagetool`
//!   and its runtime are found (as `cargo xtask appimage` finds them).
//!
//! The GUI needs no GTK or wxWidgets: winit and Vello use the system's
//! graphics stack (Direct3D 12 or Vulkan on Windows, Metal on macOS,
//! Vulkan or OpenGL on Linux), and the bundled fonts are compiled in.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};

use crate::eci;

/// The package the GUI is built from, and its binary.
const PACKAGE: (&str, &str) = ("textweaver-xilem", "textweaver-xilem");
/// The name the binary is installed under.
const INSTALLED: &str = "textweaver-gui";
/// The cargo profile (root `Cargo.toml`, `[profile.dist]`).
const PROFILE: &str = "dist";
/// Files copied into the package: (source, path in the package).
const FILES: [(&str, &str); 7] = [
    ("LICENSE", "LICENSE"),
    ("THIRD-PARTY-NOTICES.md", "THIRD-PARTY-NOTICES.md"),
    ("docs/quickstart.md", "QUICKSTART.md"),
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
    ("third_party/xilem/LICENSE", "licenses/xilem/LICENSE"),
];

#[derive(Debug, Default, PartialEq, Eq)]
struct Args {
    out: Option<PathBuf>,
}

fn parse(args: &[String]) -> anyhow::Result<Args> {
    let mut out = Args::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--out" => out.out = Some(PathBuf::from(it.next().context("--out needs a directory")?)),
            other => bail!("unknown argument {other} (usage: cargo xtask gui-dist [--out DIR])"),
        }
    }
    Ok(out)
}

/// The package folder name.
fn package_name(version: &str) -> String {
    format!(
        "textweaver-gui-{version}-{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH
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

/// The AppImage's entry point: starts the GUI beside it.
const APPRUN: &str = "#!/bin/sh\nhere=\"$(dirname \"$(readlink -f \"$0\")\")\"\nexec \"$here/usr/bin/textweaver-gui\" \"$@\"\n";

fn run_tool(cmd: &mut Command) -> anyhow::Result<()> {
    let status = cmd
        .status()
        .with_context(|| format!("running {:?}", cmd.get_program()))?;
    if !status.success() {
        bail!("{:?} failed ({status})", cmd.get_program());
    }
    Ok(())
}

/// `cargo xtask gui-dist`.
pub fn run() -> anyhow::Result<()> {
    let args = parse(&std::env::args().skip(2).collect::<Vec<_>>())?;
    let root = eci::root();
    let version = env!("CARGO_PKG_VERSION");
    let target = eci::target_dir(&root);
    let out = args.out.unwrap_or_else(|| target.join("dist"));
    let name = package_name(version);
    let stage = out.join(&name);
    if stage.exists() {
        fs::remove_dir_all(&stage).with_context(|| format!("clearing {}", stage.display()))?;
    }
    fs::create_dir_all(&stage)?;

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    run_tool(Command::new(cargo).current_dir(&root).args([
        "build",
        "--locked",
        "--profile",
        PROFILE,
        "-p",
        PACKAGE.0,
        "--bin",
        PACKAGE.1,
    ]))
    .context("building the GUI")?;
    let exe = std::env::consts::EXE_SUFFIX;
    let built = target.join(PROFILE).join(format!("{}{exe}", PACKAGE.1));
    let bin = stage.join(format!("{INSTALLED}{exe}"));
    eci::copy(&built, &bin)?;
    for (src, dest) in FILES {
        eci::copy(&root.join(src), &stage.join(dest))?;
    }

    if cfg!(target_os = "macos") {
        let app = stage.join("textweaver.app");
        let contents = app.join("Contents");
        fs::create_dir_all(contents.join("MacOS"))?;
        fs::rename(&bin, contents.join("MacOS").join(INSTALLED))?;
        fs::write(contents.join("Info.plist"), info_plist(version))?;
        run_tool(
            Command::new("codesign")
                .args(["--force", "--deep", "--sign", "-"])
                .arg(&app),
        )?;
        let zip = out.join(format!("{name}.zip"));
        run_tool(
            Command::new("ditto")
                .args(["-c", "-k", "--keepParent"])
                .arg(&stage)
                .arg(&zip),
        )?;
        println!("package {}", zip.display());
    } else if cfg!(windows) {
        let zip = out.join(format!("{name}.zip"));
        run_tool(
            Command::new("tar")
                .arg("-a")
                .arg("-cf")
                .arg(&zip)
                .arg("-C")
                .arg(&out)
                .arg(&name),
        )?;
        println!("package {}", zip.display());
    } else {
        let tgz = out.join(format!("{name}.tar.gz"));
        run_tool(
            Command::new("tar")
                .arg("-czf")
                .arg(&tgz)
                .arg("-C")
                .arg(&out)
                .arg(&name),
        )?;
        println!("package {}", tgz.display());
        appimage(&root, &stage, &out, &name)?;
    }
    Ok(())
}

/// The GUI's own AppImage, when appimagetool is available.
fn appimage(root: &Path, stage: &Path, out: &Path, name: &str) -> anyhow::Result<()> {
    let Some(tool) = std::env::var_os("APPIMAGETOOL").map(PathBuf::from) else {
        println!("no AppImage: set APPIMAGETOOL (docker/appimage/fetch-tools.sh fetches it)");
        return Ok(());
    };
    let appdir = out.join("textweaver-gui.AppDir");
    if appdir.exists() {
        fs::remove_dir_all(&appdir)?;
    }
    crate::dist::copy_tree(
        stage,
        &appdir.join("usr").join("share").join("textweaver-gui"),
    )?;
    eci::copy(
        &stage.join(INSTALLED),
        &appdir.join("usr").join("bin").join(INSTALLED),
    )?;
    fs::write(appdir.join("AppRun"), APPRUN)?;
    fs::write(appdir.join("textweaver-gui.desktop"), desktop_entry())?;
    eci::copy(
        &root.join("scripts/linux/textweaver.svg"),
        &appdir.join("textweaver.svg"),
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for f in [
            appdir.join("AppRun"),
            appdir.join("usr/bin").join(INSTALLED),
        ] {
            fs::set_permissions(&f, fs::Permissions::from_mode(0o755))?;
        }
    }
    let image = out.join(format!("{name}.AppImage"));
    let mut cmd = Command::new(tool);
    if let Some(rt) = std::env::var_os("APPIMAGE_RUNTIME") {
        cmd.arg("--runtime-file").arg(rt);
    }
    run_tool(
        cmd.env("ARCH", std::env::consts::ARCH)
            .arg(&appdir)
            .arg(&image),
    )?;
    println!("package {}", image.display());
    Ok(())
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
        assert!(package_name("0.1.0").starts_with("textweaver-gui-0.1.0-"));
        assert!(desktop_entry().contains("Exec=textweaver-gui %f"));
        assert!(desktop_entry().contains("Terminal=false"));
        assert!(APPRUN.contains("usr/bin/textweaver-gui"));
    }

    #[test]
    fn packaged_files_exist() {
        let root = eci::root();
        for (src, _) in FILES {
            assert!(root.join(src).is_file(), "{src}");
        }
    }
}
