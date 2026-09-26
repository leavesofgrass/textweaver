//! `cargo xtask eci-host`: builds `textweaver-eci-host` and installs it,
//! with the community pronunciation dictionaries, next to the workspace's
//! debug and release binaries, where the `eci` backend looks for them
//! (ADR-0007; `textweaver_eci::discovery`). Owner: Agent E.
//!
//! - Windows: the native x64 host (`textweaver-eci-host.exe`, for OpenEVV's
//!   x86_64 `eci.dll`) and the 32-bit host (`textweaver-eci-host-x86.exe`,
//!   built for `i686-pc-windows-msvc`, for Code Factory's 32-bit `eci.dll`).
//!   The 32-bit build needs `rustup target add i686-pc-windows-msvc`.
//! - Elsewhere: the native host (`textweaver-eci-host`, for Voxin).
//! - Everywhere: `third_party/ibmtts-dictionaries/` is copied to
//!   `ibmtts-dictionaries/` beside the hosts.
//!
//! The host needs no audio output, so it is built without default features.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};

const PACKAGE: &str = "textweaver-eci";
const BIN: &str = "textweaver-eci-host";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn target_dir(root: &Path) -> PathBuf {
    match std::env::var_os("CARGO_TARGET_DIR").filter(|v| !v.is_empty()) {
        Some(d) => {
            let d = PathBuf::from(d);
            if d.is_absolute() { d } else { root.join(d) }
        }
        None => root.join("target"),
    }
}

fn cargo_build(root: &Path, target: Option<&str>) -> anyhow::Result<()> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut cmd = Command::new(cargo);
    cmd.current_dir(root).args([
        "build",
        "--release",
        "--no-default-features",
        "-p",
        PACKAGE,
        "--bin",
        BIN,
    ]);
    if let Some(t) = target {
        cmd.args(["--target", t]);
    }
    let status = cmd.status().context("running cargo")?;
    if !status.success() {
        bail!(
            "building {BIN}{} failed{}",
            target.map(|t| format!(" for {t}")).unwrap_or_default(),
            if target.is_some() {
                format!(
                    " (is the target installed? rustup target add {})",
                    target.unwrap_or_default()
                )
            } else {
                String::new()
            }
        );
    }
    Ok(())
}

fn copy(from: &Path, to: &Path) -> anyhow::Result<()> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if from == to {
        return Ok(());
    }
    std::fs::copy(from, to)
        .with_context(|| format!("copying {} to {}", from.display(), to.display()))?;
    println!("installed {}", to.display());
    Ok(())
}

fn copy_dictionaries(root: &Path, dest: &Path) -> anyhow::Result<()> {
    let src = root.join("third_party").join("ibmtts-dictionaries");
    let dest = dest.join("ibmtts-dictionaries");
    std::fs::create_dir_all(&dest)?;
    let mut n = 0;
    for entry in std::fs::read_dir(&src).with_context(|| format!("reading {}", src.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            std::fs::copy(&path, dest.join(entry.file_name()))?;
            n += 1;
        }
    }
    println!("installed {} ({n} files)", dest.display());
    Ok(())
}

pub fn run() -> anyhow::Result<()> {
    let root = root();
    let target = target_dir(&root);
    let exe = std::env::consts::EXE_SUFFIX;
    let native_name = format!("{BIN}{exe}");

    cargo_build(&root, None)?;
    let native = target.join("release").join(&native_name);
    let mut installs: Vec<(PathBuf, String)> = vec![(native, native_name)];

    if cfg!(windows) {
        cargo_build(&root, Some("i686-pc-windows-msvc"))?;
        let x86 = target
            .join("i686-pc-windows-msvc")
            .join("release")
            .join(format!("{BIN}.exe"));
        installs.push((x86, format!("{BIN}-x86.exe")));
    }

    for profile in ["debug", "release"] {
        let dest = target.join(profile);
        for (from, name) in &installs {
            copy(from, &dest.join(name))?;
        }
        copy_dictionaries(&root, &dest)?;
    }
    Ok(())
}
