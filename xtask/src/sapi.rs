//! `cargo xtask sapi-host [--release]`: builds the hosts SAPI 5 voices
//! run in (ADR-0009) for development, in the chosen profile, and places
//! them next to that profile's workspace binaries, where `textweaver-sapi`
//! looks for them:
//!
//! - `target/<profile>/textweaver-sapi-host.exe`: the separate x64 host,
//!   for test binaries and `cargo xtask bench`, which cannot run hosts
//!   themselves (the programs run theirs in host mode, `--engine-host
//!   sapi`, and ship no such file);
//! - `target/<profile>/textweaver-host-x86.exe`: the combined 32-bit host
//!   (built for `i686-pc-windows-msvc`, then copied), as the packages ship
//!   it.
//!
//! Both are built without default features (a host needs no audio
//! output). `cargo xtask hosts` (in [`crate::eci`]) builds the 32-bit
//! host for both profiles. Needs the `i686-pc-windows-msvc` target
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
    let hosts = [
        HostBuild {
            package: "textweaver-sapi",
            bin: "textweaver-sapi-host",
            target: None,
        },
        eci::X86_HOST,
    ];
    eci::build(&root, &hosts, release).context(format!(
        "building the SAPI hosts (is the target installed? rustup target add {})",
        eci::X86_TARGET
    ))?;
    for h in &hosts {
        let to = target_dir.join(profile).join(h.installed_name());
        eci::copy(&h.built(&target_dir, profile), &to)?;
        println!(
            "{} host: {}",
            h.target.map_or("x64", |_| "x86"),
            to.display()
        );
    }
    Ok(())
}
