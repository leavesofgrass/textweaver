//! `cargo xtask sapi-host [--release]`: builds both SAPI5 hosts (ADR-0009)
//! and places them next to the workspace binaries, where
//! `textweaver-sapi` looks for them:
//!
//! - `target/<profile>/textweaver-sapi-host.exe` (x64, built in place);
//! - `target/<profile>/textweaver-sapi-host-x86.exe` (built for
//!   `i686-pc-windows-msvc` without the playback feature, then copied).
//!
//! Needs the `i686-pc-windows-msvc` target
//! (`rustup target add i686-pc-windows-msvc`). On other platforms it does
//! nothing: SAPI5 is Windows-only.
//!
//! Owner: Agent G. Wiring (orchestrator, `xtask/src/main.rs`):
//! `mod sapi;` and the match arm `"sapi-host" => sapi::run(),`.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, bail};

const HOST: &str = "textweaver-sapi-host";
const X86_TARGET: &str = "i686-pc-windows-msvc";

/// Runs the task; `--release` builds release hosts.
pub fn run() -> anyhow::Result<()> {
    if !cfg!(windows) {
        println!("sapi-host: SAPI5 is Windows-only; nothing to build here");
        return Ok(());
    }
    let release = std::env::args().skip(2).any(|a| a == "--release");
    let profile = if release { "release" } else { "debug" };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("xtask has no parent directory")?
        .to_path_buf();
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| root.join("target"));
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());

    let build = |extra: &[&str]| -> anyhow::Result<()> {
        let mut cmd = Command::new(&cargo);
        cmd.current_dir(&root)
            .args(["build", "-p", "textweaver-sapi", "--bin", HOST])
            .args(extra);
        if release {
            cmd.arg("--release");
        }
        let status = cmd.status().context("cannot run cargo")?;
        if !status.success() {
            bail!("cargo build {} failed ({status})", extra.join(" "));
        }
        Ok(())
    };

    build(&[])?;
    build(&["--target", X86_TARGET, "--no-default-features"]).context(format!(
        "building the 32-bit host (is the target installed? rustup target add {X86_TARGET})"
    ))?;

    let x64 = target_dir.join(profile).join(format!("{HOST}.exe"));
    let x86_built = target_dir
        .join(X86_TARGET)
        .join(profile)
        .join(format!("{HOST}.exe"));
    let x86 = target_dir.join(profile).join(format!("{HOST}-x86.exe"));
    std::fs::copy(&x86_built, &x86)
        .with_context(|| format!("copying {} to {}", x86_built.display(), x86.display()))?;
    println!("x64 host: {}", x64.display());
    println!("x86 host: {}", x86.display());
    Ok(())
}
