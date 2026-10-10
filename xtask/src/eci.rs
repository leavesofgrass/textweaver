//! Engine hosts (ADR-0012): building them and installing them next to the
//! workspace's binaries, where the backends look for them.
//!
//! - `cargo xtask hosts [--dest DIR]` builds every host for this platform
//!   and is what CI and the release job call: on Windows the ECI hosts
//!   (`textweaver-eci-host.exe` for OpenEVV's x86_64 `eci.dll`,
//!   `textweaver-eci-host-x86.exe` for Code Factory's 32-bit `eci.dll`),
//!   the SAPI5 hosts (`textweaver-sapi-host.exe`,
//!   `textweaver-sapi-host-x86.exe`), the DECtalk hosts, and the eSpeak NG
//!   hosts (`textweaver-espeak-host.exe`, `textweaver-espeak-host-x86.exe`);
//!   elsewhere the native ECI host
//!   (`textweaver-eci-host`, for Voxin). All are release builds without
//!   default features (a host needs no audio output), installed with the
//!   community pronunciation dictionaries (`ibmtts-dictionaries/`) into
//!   both `target/debug` and `target/release`, and into `DIR` when given
//!   (for packaging).
//! - `cargo xtask eci-host` builds and installs the ECI hosts only (the
//!   Wave 1 task, kept).
//!
//! The 32-bit builds need `rustup target add i686-pc-windows-msvc`.
//!
//! Wiring (orchestrator, `xtask/src/main.rs`): the match arm
//! `"hosts" => eci::hosts(),` and `hosts` in the usage line.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};

const ECI_PACKAGE: &str = "textweaver-eci";
const ECI_BIN: &str = "textweaver-eci-host";
const SAPI_PACKAGE: &str = "textweaver-sapi";
const SAPI_BIN: &str = "textweaver-sapi-host";
const DECTALK_PACKAGE: &str = "textweaver-dectalk";
const DECTALK_BIN: &str = "textweaver-dectalk-host";
const ESPEAK_PACKAGE: &str = "textweaver-espeak";
const ESPEAK_BIN: &str = "textweaver-espeak-host";
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

/// Every host this platform uses.
pub(crate) fn all_hosts() -> Vec<HostBuild> {
    let mut v = vec![HostBuild {
        package: ECI_PACKAGE,
        bin: ECI_BIN,
        target: None,
    }];
    if cfg!(windows) {
        v.push(HostBuild {
            package: ECI_PACKAGE,
            bin: ECI_BIN,
            target: Some(X86_TARGET),
        });
        v.extend([None, Some(X86_TARGET)].map(|target| HostBuild {
            package: SAPI_PACKAGE,
            bin: SAPI_BIN,
            target,
        }));
    }
    // DECtalk (ADR-0021): the native host everywhere, and on Windows the
    // 32-bit host for the usual 32-bit DECtalk.dll.
    v.push(HostBuild {
        package: DECTALK_PACKAGE,
        bin: DECTALK_BIN,
        target: None,
    });
    if cfg!(windows) {
        v.push(HostBuild {
            package: DECTALK_PACKAGE,
            bin: DECTALK_BIN,
            target: Some(X86_TARGET),
        });
        // eSpeak NG's helper, on Windows only: the x64 host for the x64
        // installer's library, the x86 host for a 32-bit one. Linux and
        // macOS keep eSpeak NG in process.
        v.extend([None, Some(X86_TARGET)].map(|target| HostBuild {
            package: ESPEAK_PACKAGE,
            bin: ESPEAK_BIN,
            target,
        }));
    }
    v
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

/// `cargo xtask eci-host`: the ECI hosts only.
pub fn run() -> anyhow::Result<()> {
    let eci: Vec<HostBuild> = all_hosts()
        .into_iter()
        .filter(|h| h.package == ECI_PACKAGE)
        .collect();
    build_and_install(&eci, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_cover_the_platform() {
        let all = all_hosts();
        assert!(all.contains(&HostBuild {
            package: ECI_PACKAGE,
            bin: ECI_BIN,
            target: None
        }));
        if cfg!(windows) {
            let names: Vec<String> = all.iter().map(HostBuild::installed_name).collect();
            assert_eq!(
                names,
                [
                    "textweaver-eci-host.exe",
                    "textweaver-eci-host-x86.exe",
                    "textweaver-sapi-host.exe",
                    "textweaver-sapi-host-x86.exe",
                    "textweaver-dectalk-host.exe",
                    "textweaver-dectalk-host-x86.exe",
                    "textweaver-espeak-host.exe",
                    "textweaver-espeak-host-x86.exe",
                ]
            );
        } else {
            let names: Vec<String> = all.iter().map(HostBuild::installed_name).collect();
            assert_eq!(names, ["textweaver-eci-host", "textweaver-dectalk-host"]);
        }
    }

    #[test]
    fn built_paths_follow_cargo_layout() {
        let t = Path::new("t");
        let x86 = HostBuild {
            package: SAPI_PACKAGE,
            bin: SAPI_BIN,
            target: Some(X86_TARGET),
        };
        assert_eq!(
            x86.built(t, "release"),
            t.join(X86_TARGET)
                .join("release")
                .join("textweaver-sapi-host.exe")
        );
        let native = HostBuild {
            target: None,
            ..x86
        };
        assert_eq!(
            native.built(t, "debug"),
            t.join("debug").join(format!(
                "textweaver-sapi-host{}",
                std::env::consts::EXE_SUFFIX
            ))
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
