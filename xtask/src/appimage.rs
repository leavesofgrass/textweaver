//! `cargo xtask appimage [--out DIR] [--docker]`: the Linux release
//! packages (docs/releasing.md).
//!
//! Stages the Linux package exactly as `cargo xtask dist` does (the `dist`
//! profile; `textweaver` and `tw` with Omnivox, speech-dispatcher, and
//! espeak-ng loaded at run time; the ECI and DECtalk hosts; the
//! dictionaries, notices, licences, and guides; the menu entry and icon),
//! writes the plain tarball, then wraps the same folder in an AppImage:
//!
//! - `textweaver-VERSION-linux-ARCH.tar.gz`: the fallback for systems
//!   without FUSE;
//! - `textweaver-VERSION-linux-ARCH.AppImage`: one file that runs on most
//!   distributions. Its `AppRun` (`scripts/linux/AppRun`) starts
//!   `textweaver`, or `tw` when started through a link named `tw` or with
//!   `--tw`, and offers `--install` and `--uninstall`;
//! - `textweaver-VERSION-linux-ARCH.AppImage.zsync`: for delta updates
//!   (the AppImage carries `gh-releases-zsync` update information).
//!
//! The AppDir keeps the package folder whole under `usr/lib/textweaver/`,
//! so the programs find the engine hosts and dictionaries beside them as
//! they do in the tarball.
//!
//! Tools: `appimagetool` and the type 2 runtime, downloaded and checked by
//! `docker/appimage/fetch-tools.sh`. The task finds them through
//! `APPIMAGETOOL` and `APPIMAGE_RUNTIME`, or on `PATH` (the runtime as
//! `runtime-ARCH` beside appimagetool).
//!
//! Build on an old glibc so the result runs on older systems: the
//! `docker/appimage` image is Ubuntu 22.04 (glibc 2.35). `--docker` builds
//! that image and runs this task inside it, from any OS with Docker; the
//! packages land in `target/dist`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};

use crate::dist;
use crate::eci;

/// The GitHub owner and repository the update information points at.
const GITHUB: (&str, &str) = ("leavesofgrass", "textweaver");
/// The Docker image `--docker` builds and runs.
const IMAGE: &str = "textweaver-appimage:latest";
/// Named volumes shared with the dev container (compose project
/// `textweaver`): build output and the cargo registry.
const TARGET_VOLUME: &str = "textweaver_textweaver-target";
const REGISTRY_VOLUME: &str = "textweaver_textweaver-cargo-registry";

/// Parsed arguments.
#[derive(Debug, Default, PartialEq, Eq)]
struct Args {
    out: Option<PathBuf>,
    docker: bool,
}

fn parse(args: &[String]) -> anyhow::Result<Args> {
    let mut out = Args::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--docker" => out.docker = true,
            "--out" => out.out = Some(PathBuf::from(it.next().context("--out needs a directory")?)),
            other => bail!(
                "unknown argument {other} (usage: cargo xtask appimage [--out DIR] [--docker])"
            ),
        }
    }
    Ok(out)
}

/// The machine name the AppImage tools use (`x86_64`, `aarch64`).
fn arch() -> &'static str {
    std::env::consts::ARCH
}

/// The update information embedded in the AppImage: the newest release or
/// pre-release on GitHub (every release before 1.0 is a pre-release).
fn update_information(arch: &str) -> String {
    format!(
        "gh-releases-zsync|{}|{}|latest-all|textweaver-*-linux-{arch}.AppImage.zsync",
        GITHUB.0, GITHUB.1
    )
}

/// The desktop entry inside the AppImage: the template without `TryExec`
/// (the program is inside the image, not on `PATH`).
fn appimage_desktop(template: &str) -> String {
    let mut out = String::new();
    for line in template.lines() {
        if line.starts_with("TryExec=") {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Finds a program: the environment variable, else `PATH`.
fn find_program(var: &str, name: &str) -> Option<PathBuf> {
    if let Some(p) = std::env::var_os(var).filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(p));
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

/// `cargo xtask appimage`.
pub fn run() -> anyhow::Result<()> {
    let args = parse(&std::env::args().skip(2).collect::<Vec<_>>())?;
    if args.docker {
        return in_docker(&args);
    }
    if !cfg!(target_os = "linux") {
        bail!(
            "the AppImage is built on Linux; on this system run `cargo xtask appimage --docker` (it needs Docker)"
        );
    }
    let tool = find_program("APPIMAGETOOL", "appimagetool").context(
        "appimagetool was not found: run docker/appimage/fetch-tools.sh DIR and set APPIMAGETOOL=DIR/appimagetool, or use --docker",
    )?;
    let runtime = match std::env::var_os("APPIMAGE_RUNTIME").filter(|v| !v.is_empty()) {
        Some(r) => PathBuf::from(r),
        None => tool
            .parent()
            .map(|d| d.join(format!("runtime-{}", arch())))
            .filter(|p| p.is_file())
            .context("the AppImage runtime was not found: set APPIMAGE_RUNTIME, or keep runtime-ARCH beside appimagetool (docker/appimage/fetch-tools.sh)")?,
    };

    let root = eci::root();
    let staged = dist::stage(args.out.clone(), false)?;
    let tarball = dist::archive(&staged)?;
    println!("package {}", tarball.display());

    let appdir = staged.out.join("textweaver.AppDir");
    build_appdir(&root, &staged.dir, &appdir)?;

    let file = staged.out.join(format!("{}.AppImage", staged.name));
    let _ = fs::remove_file(&file);
    let zsync = staged.out.join(format!("{}.AppImage.zsync", staged.name));
    let _ = fs::remove_file(&zsync);
    let status = Command::new(&tool)
        .arg("--no-appstream")
        .arg("--runtime-file")
        .arg(&runtime)
        .arg("--updateinformation")
        .arg(update_information(arch()))
        .arg(&appdir)
        .arg(&file)
        .current_dir(&staged.out)
        .env("ARCH", arch())
        .env("VERSION", env!("CARGO_PKG_VERSION"))
        // appimagetool is itself an AppImage; without FUSE (containers)
        // it extracts itself and runs.
        .env("APPIMAGE_EXTRACT_AND_RUN", "1")
        .status()
        .with_context(|| format!("running {}", tool.display()))?;
    if !status.success() {
        bail!("appimagetool failed ({status})");
    }
    if !file.is_file() {
        bail!("appimagetool did not write {}", file.display());
    }
    println!("package {}", file.display());
    if zsync.is_file() {
        println!("package {}", zsync.display());
    } else {
        println!(
            "note: no {} (appimagetool writes it when zsyncmake is installed)",
            zsync.display()
        );
    }
    Ok(())
}

/// Lays out the AppDir: the package under `usr/lib/textweaver`, `AppRun`,
/// the desktop entry, and the icon.
fn build_appdir(root: &Path, package: &Path, appdir: &Path) -> anyhow::Result<()> {
    if appdir.exists() {
        fs::remove_dir_all(appdir).with_context(|| format!("clearing {}", appdir.display()))?;
    }
    dist::copy_tree(package, &appdir.join("usr/lib/textweaver"))?;

    let apprun = appdir.join("AppRun");
    eci::copy(&root.join("scripts/linux/AppRun"), &apprun)?;
    executable(&apprun)?;

    let template = fs::read_to_string(root.join("scripts/linux/textweaver.desktop"))
        .context("reading scripts/linux/textweaver.desktop")?;
    let desktop = appimage_desktop(&template);
    for dest in [
        appdir.join("textweaver.desktop"),
        appdir.join("usr/share/applications/textweaver.desktop"),
    ] {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, &desktop).with_context(|| format!("writing {}", dest.display()))?;
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

#[cfg(unix)]
fn executable(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .with_context(|| format!("making {} executable", path.display()))
}

#[cfg(not(unix))]
fn executable(_: &Path) -> anyhow::Result<()> {
    Ok(())
}

/// Builds the `docker/appimage` image and runs this task inside it, with
/// the packages written to `target/dist` in this checkout.
fn in_docker(args: &Args) -> anyhow::Result<()> {
    let root = eci::root();
    let docker = std::env::var_os("DOCKER").unwrap_or_else(|| "docker".into());
    let status = Command::new(&docker)
        .args(["build", "-t", IMAGE])
        .arg(root.join("docker").join("appimage"))
        .status()
        .context("running docker (is Docker installed and running?)")?;
    if !status.success() {
        bail!("building the {IMAGE} image failed ({status})");
    }
    if args.out.is_some() {
        println!("note: --out is ignored with --docker; the packages go to target/dist");
    }
    let mut cmd = Command::new(&docker);
    cmd.args(["run", "--rm"])
        .arg("-v")
        .arg(format!("{}:/work", root.display()))
        .args(["-v", &format!("{TARGET_VOLUME}:/target")])
        .args([
            "-v",
            &format!("{REGISTRY_VOLUME}:/usr/local/cargo/registry"),
        ])
        .args(["-e", "CARGO_TARGET_DIR=/target/appimage"])
        .args(["-w", "/work"]);
    if cfg!(unix)
        && let (Some(uid), Some(gid)) = (id("-u"), id("-g"))
    {
        cmd.args(["--user", &format!("{uid}:{gid}")]);
    }
    cmd.args([
        IMAGE,
        "cargo",
        "xtask",
        "appimage",
        "--out",
        "/work/target/dist",
    ]);
    let status = cmd.status().context("running docker")?;
    if !status.success() {
        bail!("the AppImage build in Docker failed ({status})");
    }
    println!(
        "The packages are in {}",
        eci::target_dir(&root).join("dist").display()
    );
    Ok(())
}

/// `id -u` or `id -g`.
fn id(flag: &str) -> Option<String> {
    let out = Command::new("id").arg(flag).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments() {
        assert_eq!(parse(&[]).unwrap(), Args::default());
        let a = parse(&["--docker".into(), "--out".into(), "o".into()]).unwrap();
        assert!(a.docker);
        assert_eq!(a.out, Some(PathBuf::from("o")));
        assert!(parse(&["--out".into()]).is_err());
        assert!(parse(&["--bogus".into()]).is_err());
    }

    #[test]
    fn update_information_points_at_the_releases() {
        assert_eq!(
            update_information("x86_64"),
            "gh-releases-zsync|leavesofgrass|textweaver|latest-all|textweaver-*-linux-x86_64.AppImage.zsync"
        );
    }

    #[test]
    fn the_desktop_entry_drops_tryexec() {
        let root = eci::root();
        let template = fs::read_to_string(root.join("scripts/linux/textweaver.desktop")).unwrap();
        let d = appimage_desktop(&template);
        assert!(!d.contains("TryExec="));
        assert!(d.lines().any(|l| l == "Terminal=true"));
        assert!(d.lines().any(|l| l.starts_with("Exec=textweaver")));
        assert!(d.lines().any(|l| l == "Icon=textweaver"));
    }

    #[test]
    fn the_appdir_holds_the_package_and_the_entry_point() {
        let root = eci::root();
        let tmp = std::env::temp_dir().join(format!("tw-appdir-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let package = tmp.join("pkg");
        fs::create_dir_all(package.join("docs")).unwrap();
        fs::write(package.join("tw"), "tw").unwrap();
        fs::write(package.join("docs/reading.md"), "# Reading").unwrap();
        let appdir = tmp.join("textweaver.AppDir");
        build_appdir(&root, &package, &appdir).unwrap();
        for f in [
            "AppRun",
            "textweaver.desktop",
            "textweaver.svg",
            ".DirIcon",
            "usr/lib/textweaver/tw",
            "usr/lib/textweaver/docs/reading.md",
            "usr/share/applications/textweaver.desktop",
            "usr/share/icons/hicolor/scalable/apps/textweaver.svg",
        ] {
            assert!(appdir.join(f).is_file(), "{f} is missing");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(appdir.join("AppRun"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o111, 0o111);
        }
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn apprun_dispatches_on_the_name_and_first_argument() {
        let apprun = fs::read_to_string(eci::root().join("scripts/linux/AppRun")).unwrap();
        assert!(apprun.starts_with("#!/bin/sh\n"));
        for needle in [
            "ARGV0",
            "tw | tw.*) program=tw",
            "--tw)",
            "--install | --uninstall)",
            "usr/lib/textweaver",
        ] {
            assert!(apprun.contains(needle), "AppRun lacks {needle:?}");
        }
    }
}
