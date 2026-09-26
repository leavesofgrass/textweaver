//! `cargo xtask sapi-host [--release]`: builds both SAPI5 hosts (ADR-0009)
//! in the chosen profile and places them next to that profile's workspace
//! binaries, where `textweaver-sapi` looks for them:
//!
//! - `target/<profile>/textweaver-sapi-host.exe` (x64, built in place);
//! - `target/<profile>/textweaver-sapi-host-x86.exe` (built for
//!   `i686-pc-windows-msvc`, then copied).
//!
//! Both are built without default features (a host needs no audio
//! output). `cargo xtask hosts` (in [`crate::eci`]) builds every host,
//! these included, for both profiles; this task remains for a quick
//! debug-profile rebuild. Needs the `i686-pc-windows-msvc` target
//! (`rustup target add i686-pc-windows-msvc`). On other platforms it does
//! nothing: SAPI5 is Windows-only.

use anyhow::Context;

use crate::eci::{self, HostBuild};

/// Runs the task; `--release` builds release hosts.
pub fn run() -> anyhow::Result<()> {
    if !cfg!(windows) {
        println!("sapi-host: SAPI5 is Windows-only; nothing to build here");
        return Ok(());
    }
    let release = std::env::args().skip(2).any(|a| a == "--release");
    let profile = if release { "release" } else { "debug" };
    let root = eci::root();
    let target_dir = eci::target_dir(&root);
    let hosts: Vec<HostBuild> = eci::all_hosts()
        .into_iter()
        .filter(|h| h.package == "textweaver-sapi")
        .collect();
    eci::build(&root, &hosts, release).context(format!(
        "building the SAPI hosts (is the target installed? rustup target add {})",
        eci::X86_TARGET
    ))?;
    for h in &hosts {
        let to = target_dir.join(profile).join(h.installed_name());
        eci::copy(&h.built(&target_dir, profile), &to)?;
        println!("{} host: {}", h.target.map_or("x64", |_| "x86"), to.display());
    }
    Ok(())
}
