//! Finding libespeak-ng and the helper program that can run it.
//!
//! The library is searched in the order the in-process backend loads it
//! (`textweaver_speech::backends::espeak`): textweaver's components
//! folder first, then `TEXTWEAVER_ESPEAK_LIBRARY`, then the usual install
//! paths. The first file that exists is the library, and its header says
//! its architecture ([`Arch`]). The helper must match it: the native host
//! (`textweaver-espeak-host`) for a library of this program's own
//! architecture, `textweaver-espeak-host-x86.exe` for a 32-bit one,
//! `textweaver-espeak-host-x64.exe` for an x86-64 library under an ARM64
//! program (Windows runs it under emulation), and so on.

use std::path::PathBuf;

pub use textweaver_enginehost::arch::{Arch, library_arch};
use textweaver_speech::backends::espeak;

use crate::EspeakHostConfig;

/// The native host's file name.
pub const HOST_NAME: &str = if cfg!(windows) {
    "textweaver-espeak-host.exe"
} else {
    "textweaver-espeak-host"
};

/// The 32-bit host's file name (`cargo xtask hosts` installs it next to
/// the binaries on Windows).
pub const HOST_NAME_X86: &str = "textweaver-espeak-host-x86.exe";

/// The library to load, and its architecture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryChoice {
    /// The file.
    pub path: PathBuf,
    /// Its architecture, when the header could be read.
    pub arch: Option<Arch>,
}

/// The files the library is looked for in, in order: the components
/// folder's copy, `TEXTWEAVER_ESPEAK_LIBRARY`, then the usual install
/// paths. Bare names (found through the system's search path) are left
/// out: a file found that way loads in this process or not at all.
pub fn library_candidates() -> Vec<PathBuf> {
    let env = std::env::var_os(espeak::LIBRARY_ENV)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from);
    espeak::component_library()
        .into_iter()
        .chain(env)
        .chain(
            espeak::library_candidates()
                .iter()
                .map(PathBuf::from)
                .filter(|p| p.is_absolute()),
        )
        .collect()
}

/// The first of `candidates` that exists, with its architecture.
pub fn choose_library(candidates: &[PathBuf]) -> Option<LibraryChoice> {
    candidates
        .iter()
        .find(|p| p.is_file())
        .map(|p| LibraryChoice {
            arch: library_arch(p),
            path: p.clone(),
        })
}

/// The host's file name for a library of architecture `arch` when this
/// program is `program`: the native host for the same architecture (or
/// an unknown one), else the host named for the library's.
pub fn host_name(arch: Option<Arch>, program: Arch) -> String {
    match arch {
        Some(a) if a != program && a != Arch::Other => {
            let suffix = match a {
                Arch::X86 => "x86",
                Arch::X64 => "x64",
                _ => "arm64",
            };
            format!("textweaver-espeak-host-{suffix}.exe")
        }
        _ => HOST_NAME.to_owned(),
    }
}

/// Host executables that can run a library of architecture `arch`, in
/// order: [`EspeakHostConfig::host`], `TEXTWEAVER_ESPEAK_HOST`, then the
/// host for that architecture beside the running executable (and in its
/// parent folder, for test binaries in `deps/`), and on Windows a cargo
/// `i686-pc-windows-msvc` build under the target folder for a 32-bit
/// library. Only existing files are returned.
pub fn host_candidates(config: &EspeakHostConfig, arch: Option<Arch>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    if let Some(h) = &config.host {
        out.push(h.clone());
    }
    if let Some(h) = std::env::var_os(crate::HOST_ENV).filter(|v| !v.is_empty()) {
        out.push(PathBuf::from(h));
    }
    let arch = if config.fake_engine { None } else { arch };
    let name = host_name(arch, Arch::current());
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(PathBuf::from))
    {
        for d in dir.ancestors().take(2) {
            out.push(d.join(&name));
        }
        if cfg!(windows) && arch == Some(Arch::X86) && Arch::current() != Arch::X86 {
            for d in dir.ancestors().take(4) {
                for profile in ["release", "debug"] {
                    out.push(d.join("i686-pc-windows-msvc").join(profile).join(HOST_NAME));
                }
            }
        }
    }
    let mut seen = Vec::new();
    out.retain(|p| {
        let keep = p.is_file() && !seen.contains(p);
        if keep {
            seen.push(p.clone());
        }
        keep
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_host_follows_the_library_architecture() {
        assert_eq!(host_name(None, Arch::X64), HOST_NAME);
        assert_eq!(host_name(Some(Arch::X64), Arch::X64), HOST_NAME);
        assert_eq!(host_name(Some(Arch::Other), Arch::X64), HOST_NAME);
        assert_eq!(host_name(Some(Arch::X86), Arch::X64), HOST_NAME_X86);
        assert_eq!(
            host_name(Some(Arch::X64), Arch::Arm64),
            "textweaver-espeak-host-x64.exe"
        );
        assert_eq!(
            host_name(Some(Arch::Arm64), Arch::X64),
            "textweaver-espeak-host-arm64.exe"
        );
    }

    #[test]
    fn the_first_existing_library_wins_with_its_architecture() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.dll");
        let lib = dir.path().join("libespeak-ng.dll");
        let mut pe = vec![0u8; 0x80];
        pe[..2].copy_from_slice(b"MZ");
        pe[0x3C] = 0x40;
        pe[0x40..0x44].copy_from_slice(b"PE\0\0");
        pe[0x44..0x46].copy_from_slice(&0x014Cu16.to_le_bytes());
        std::fs::write(&lib, pe).unwrap();
        let got = choose_library(&[missing.clone(), lib.clone()]).unwrap();
        assert_eq!(got.path, lib);
        assert_eq!(got.arch, Some(Arch::X86));
        assert_eq!(choose_library(&[missing]), None);
    }

    #[test]
    fn hosts_named_in_the_options_come_first_and_must_exist() {
        let dir = tempfile::tempdir().unwrap();
        let host = dir.path().join("my-host.exe");
        std::fs::write(&host, b"").unwrap();
        let config = EspeakHostConfig {
            host: Some(host.clone()),
            ..EspeakHostConfig::default()
        };
        assert_eq!(host_candidates(&config, None).first(), Some(&host));
        let config = EspeakHostConfig {
            host: Some(dir.path().join("gone.exe")),
            ..EspeakHostConfig::default()
        };
        assert!(!host_candidates(&config, None).contains(&dir.path().join("gone.exe")));
    }
}
