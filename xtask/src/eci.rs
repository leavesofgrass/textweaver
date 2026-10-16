//! Engine hosts (ADR-0012): building them and installing them next to the
//! workspace's binaries, where the backends look for them.
//!
//! Since Friday, October 16, 2026 (ADR-0007 addendum) the programs run
//! the native-architecture hosts as a copy of themselves
//! (`--engine-host <engine>`), so there is nothing separate to build for
//! them. The one host program left is the combined 32-bit host for
//! Windows, `textweaver-host-x86.exe` (the `textweaver-host` binary built
//! for `i686-pc-windows-msvc`), which runs every engine's 32-bit library:
//! Code Factory's `eci.dll`, the usual `DECtalk.dll`, 32-bit SAPI 5
//! voices, and a 32-bit libespeak-ng.
//!
//! - `cargo xtask hosts [--dest DIR]` builds it (on Windows; elsewhere
//!   there is no host to build) and is what CI and the release job call. It
//!   is a release build without default features (a host needs no audio
//!   output), installed with the community pronunciation dictionaries
//!   (`ibmtts-dictionaries/`, which the Eloquence host reads beside the
//!   program that runs it) into both `target/debug` and `target/release`,
//!   and into `DIR` when given (for packaging).
//! - `cargo xtask eci-host` is another name for `hosts` (the Wave 1 task,
//!   kept).
//!
//! The 32-bit build needs `rustup target add i686-pc-windows-msvc`.
//!
//! Wiring (orchestrator, `xtask/src/main.rs`): the match arm
//! `"hosts" => eci::hosts(),` and `hosts` in the usage line.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};

/// The combined host's package and binary.
pub(crate) const HOST_PACKAGE: &str = "textweaver-host";
const HOST_BIN: &str = "textweaver-host";
/// The 32-bit Windows target.
pub(crate) const X86_TARGET: &str = "i686-pc-windows-msvc";

/// The workspace root.
pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// Cargo's target directory (`CARGO_TARGET_DIR`, else `target/`).
pub(crate) fn target_dir(root: &Path) -> PathBuf {
    match std::env::var_os("CARGO_TARGET_DIR").filter(|v| !v.is_empty()) {
        Some(d) => {
            let d = PathBuf::from(d);
            if d.is_absolute() { d } else { root.join(d) }
        }
        None => root.join("target"),
    }
}

/// One host binary to build: package, binary, and target triple (`None`
/// for the native target).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HostBuild {
    pub package: &'static str,
    pub bin: &'static str,
    pub target: Option<&'static str>,
}

impl HostBuild {
    /// Where cargo puts the binary for `profile`.
    pub fn built(&self, target_dir: &Path, profile: &str) -> PathBuf {
        let dir = match self.target {
            Some(t) => target_dir.join(t).join(profile),
            None => target_dir.join(profile),
        };
        dir.join(format!("{}{}", self.bin, exe_suffix(self.target)))
    }

    /// The file name the backends look for: 32-bit hosts get `-x86`.
    pub fn installed_name(&self) -> String {
        match self.target {
            Some(t) if t == X86_TARGET => format!("{}-x86.exe", self.bin),
            t => format!("{}{}", self.bin, exe_suffix(t)),
        }
    }
}

fn exe_suffix(target: Option<&str>) -> &'static str {
    match target {
        Some(t) if t.contains("windows") => ".exe",
        Some(_) => "",
        None => std::env::consts::EXE_SUFFIX,
    }
}

/// The combined 32-bit host, `textweaver-host-x86.exe`.
pub(crate) const X86_HOST: HostBuild = HostBuild {
    package: HOST_PACKAGE,
    bin: HOST_BIN,
    target: Some(X86_TARGET),
};

/// Every host program this platform ships: the combined 32-bit host on
/// Windows, none elsewhere (the programs are their own native hosts).
pub(crate) fn all_hosts() -> Vec<HostBuild> {
    if cfg!(windows) {
        vec![X86_HOST]
    } else {
        Vec::new()
    }
}

/// Builds `hosts` in `profile` without default features, one cargo run per
/// target triple.
pub(crate) fn build(root: &Path, hosts: &[HostBuild], release: bool) -> anyhow::Result<()> {
    let mut targets: Vec<Option<&str>> = Vec::new();
    for h in hosts {
        if !targets.contains(&h.target) {
            targets.push(h.target);
        }
    }
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    for target in targets {
        let mut cmd = Command::new(&cargo);
        cmd.current_dir(root)
            .args(["build", "--no-default-features"]);
        if release {
            cmd.arg("--release");
        }
        for h in hosts.iter().filter(|h| h.target == target) {
            cmd.args(["-p", h.package, "--bin", h.bin]);
        }
        if let Some(t) = target {
            cmd.args(["--target", t]);
        }
        let status = cmd.status().context("running cargo")?;
        if !status.success() {
            match target {
                Some(t) => bail!(
                    "building the hosts for {t} failed (is the target installed? rustup target add {t})"
                ),
                None => bail!("building the hosts failed"),
            }
        }
    }
    Ok(())
}

/// Copies one file, creating the directory.
pub(crate) fn copy(from: &Path, to: &Path) -> anyhow::Result<()> {
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

/// Copies `third_party/ibmtts-dictionaries/` to `dest/ibmtts-dictionaries/`.
pub(crate) fn copy_dictionaries(root: &Path, dest: &Path) -> anyhow::Result<()> {
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

/// Builds `hosts` (release) and installs them with the dictionaries into
/// `target/debug`, `target/release`, and each of `extra`.
pub(crate) fn build_and_install(hosts: &[HostBuild], extra: &[PathBuf]) -> anyhow::Result<()> {
    let root = root();
    let target = target_dir(&root);
    build(&root, hosts, true)?;
    let mut dests = vec![target.join("debug"), target.join("release")];
    dests.extend(extra.iter().cloned());
    for dest in &dests {
        for h in hosts {
            copy(&h.built(&target, "release"), &dest.join(h.installed_name()))?;
        }
        copy_dictionaries(&root, dest)?;
    }
    Ok(())
}

/// `--dest DIR` values from the task's arguments.
fn dest_args(args: &[String]) -> anyhow::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--dest" => out.push(PathBuf::from(
                it.next().context("--dest needs a directory")?,
            )),
            other => bail!("unknown argument {other} (usage: cargo xtask hosts [--dest DIR])"),
        }
    }
    Ok(out)
}

/// `cargo xtask hosts [--dest DIR]`: every host for this platform.
pub fn hosts() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let extra = dest_args(&args)?;
    build_and_install(&all_hosts(), &extra)
}

/// `cargo xtask eci-host`: another name for `hosts` (every engine's
/// 32-bit host is one program now).
pub fn run() -> anyhow::Result<()> {
    build_and_install(&all_hosts(), &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One host program on Windows, the combined 32-bit host, under the
    /// name the backends look for; none elsewhere.
    #[test]
    fn hosts_cover_the_platform() {
        let names: Vec<String> = all_hosts().iter().map(HostBuild::installed_name).collect();
        if cfg!(windows) {
            assert_eq!(names, ["textweaver-host-x86.exe"]);
        } else {
            assert!(names.is_empty(), "{names:?}");
        }
    }

    #[test]
    fn built_paths_follow_cargo_layout() {
        let t = Path::new("t");
        assert_eq!(
            X86_HOST.built(t, "release"),
            t.join(X86_TARGET)
                .join("release")
                .join("textweaver-host.exe")
        );
        let native = HostBuild {
            target: None,
            ..X86_HOST
        };
        assert_eq!(
            native.built(t, "debug"),
            t.join("debug")
                .join(format!("textweaver-host{}", std::env::consts::EXE_SUFFIX))
        );
    }

    #[test]
    fn dest_arguments_parse() {
        assert!(dest_args(&[]).unwrap().is_empty());
        assert_eq!(
            dest_args(&["--dest".into(), "pkg".into()]).unwrap(),
            [PathBuf::from("pkg")]
        );
        assert!(dest_args(&["--dest".into()]).is_err());
        assert!(dest_args(&["--bogus".into()]).is_err());
    }
}
