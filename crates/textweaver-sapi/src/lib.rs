//! Windows SAPI5 voices for textweaver (ADR-0009).
//!
//! Each voice runs in a host process, `textweaver-sapi-host`, built for the
//! voice's architecture: x64 for 64-bit voices (Microsoft David, Zira, and
//! the OneCore voices) and x86 for 32-bit-only voices (VW Paul, Kate,
//! James; eSpeak SAPI). The host synthesizes into a stream it forwards as it
//! is written, and reports each word boundary with its audio offset over a
//! framed pipe protocol ([`protocol`]). `SapiBackend` plays the audio in
//! the main process and turns the offsets into audio-clock word events, so
//! the highlight follows the word being heard.
//!
//! Wiring (for the application and the backend registry):
//! - [`backend_info`] describes the backend (id `"sapi"`, priority
//!   [`PRIORITY`], available when the 64-bit host is found);
//! - [`factory`] builds it on the speech thread;
//! - `SapiBackend::voice_details` / [`list_voices`] give each voice's
//!   architecture, family, vendor, and tags ([`voices::TAG_OPENEVV`] marks
//!   OpenEVV's Eloquence voices, which the app can prefer for Eloquence
//!   Reed; [`voices::TAG_NO_WORD_TIMING`] marks Code Factory's).
//!
//! Voice ids are `"<arch>:<SAPI token id>"` ([`voices::VoiceId`]).
//!
//! Finding the hosts:
//! - x64: [`SapiConfig::host_x64`], else `TEXTWEAVER_SAPI_HOST`, else
//!   `textweaver-sapi-host.exe` next to the running executable (or its
//!   parent directory, for test binaries in `target/<profile>/deps`);
//! - x86: [`SapiConfig::host_x86`], else `TEXTWEAVER_SAPI_HOST_X86`, else
//!   `textweaver-sapi-host-x86.exe` next to the executable (where
//!   `cargo xtask hosts` puts it), else a cargo
//!   `i686-pc-windows-msvc` build under the target directory.
//!
//! On other platforms the crate holds only its platform-neutral parts
//! (protocol, position mapping, voice metadata); [`available`] is false and
//! [`factory`] reports the backend unavailable.
//!
//! Owner: Agent G.

#[cfg(windows)]
pub mod audio;
#[cfg(windows)]
mod backend;
pub mod calibration;
pub mod host;
pub mod protocol;
pub mod voices;
pub mod wav;
pub mod words;

use std::path::PathBuf;

#[cfg(windows)]
pub use audio::AudioOutput;
#[cfg(windows)]
pub use backend::{SapiBackend, Synthesis};
use textweaver_speech::{BackendFactory, BackendId, BackendInfo, SpeechError};
use voices::{Arch, SapiVoice};

/// The backend id.
pub const BACKEND_ID: BackendId = "sapi";

/// Automatic-selection priority on Windows: below Eloquence (`eci`, 1000),
/// above engines without word timing.
pub const PRIORITY: i32 = 500;

/// SAPI voices normalize numbers, dates, and times themselves, but not
/// consistently (eSpeak reads "a.m." as "a"), so the speech service keeps
/// its own normalization for this backend.
pub const NORMALIZES_NATIVELY: bool = false;

/// Environment variable naming the 64-bit host.
pub const HOST_ENV: &str = "TEXTWEAVER_SAPI_HOST";
/// Environment variable naming the 32-bit host.
pub const HOST_ENV_X86: &str = "TEXTWEAVER_SAPI_HOST_X86";

/// File name of the 64-bit host.
pub const HOST_NAME: &str = "textweaver-sapi-host.exe";
/// File name `cargo xtask hosts` gives the 32-bit host.
pub const HOST_NAME_X86: &str = "textweaver-sapi-host-x86.exe";

/// True when this build can use SAPI5 (Windows only).
pub fn available() -> bool {
    cfg!(windows)
}

/// Backend configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct SapiConfig {
    /// The 64-bit host; `None` searches (see the crate docs).
    pub host_x64: Option<PathBuf>,
    /// The 32-bit host; `None` searches.
    pub host_x86: Option<PathBuf>,
    /// List OneCore voices (Microsoft Mark, David, Zira) too. They load
    /// through SAPI5 by token id (probed 2026-09-25).
    pub onecore: bool,
    /// Where audio goes.
    #[cfg(windows)]
    pub output: AudioOutput,
    /// Run the hosts with their fake engine (tests only).
    pub fake_engine: bool,
}

impl Default for SapiConfig {
    fn default() -> Self {
        SapiConfig {
            host_x64: None,
            host_x86: None,
            onecore: true,
            #[cfg(windows)]
            output: AudioOutput::default(),
            fake_engine: false,
        }
    }
}

/// The host file name for `arch` next to the binaries.
pub fn host_file_name(arch: Arch) -> &'static str {
    match arch {
        Arch::X64 => HOST_NAME,
        Arch::X86 => HOST_NAME_X86,
    }
}

/// The environment variable naming the host for `arch`.
pub fn host_env(arch: Arch) -> &'static str {
    match arch {
        Arch::X64 => HOST_ENV,
        Arch::X86 => HOST_ENV_X86,
    }
}

/// Host executables to try for `arch`, in order (see the crate docs). Only
/// existing files are returned.
pub fn host_candidates(config: &SapiConfig, arch: Arch) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let configured = match arch {
        Arch::X64 => &config.host_x64,
        Arch::X86 => &config.host_x86,
    };
    if let Some(h) = configured {
        out.push(h.clone());
    }
    if let Some(h) = std::env::var_os(host_env(arch)).filter(|v| !v.is_empty()) {
        out.push(PathBuf::from(h));
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(PathBuf::from))
    {
        for d in dir.ancestors().take(2) {
            out.push(d.join(host_file_name(arch)));
        }
        if arch == Arch::X86 {
            // A development build: target/i686-pc-windows-msvc/<profile>/.
            for d in dir.ancestors().take(4) {
                for profile in ["debug", "release"] {
                    out.push(d.join("i686-pc-windows-msvc").join(profile).join(HOST_NAME));
                }
            }
        }
    }
    let mut seen: Vec<PathBuf> = Vec::new();
    out.retain(|p| {
        let keep = p.is_file() && !seen.contains(p);
        if keep {
            seen.push(p.clone());
        }
        keep
    });
    out
}

/// Every installed voice, from both hosts' registries (x64 first; a voice
/// registered for both is listed once, as x64), OneCore voices included
/// when [`SapiConfig::onecore`] is set. Reads registry tokens only: no
/// engine is loaded. Fails only when no host can list anything.
#[cfg(windows)]
pub fn list_voices(config: &SapiConfig) -> Result<Vec<SapiVoice>, SpeechError> {
    let mut lists = Vec::new();
    let mut errors = Vec::new();
    for arch in Arch::ALL {
        let mut categories = vec![host::CATEGORY_SAPI];
        if arch == Arch::X64 && config.onecore {
            categories.push(host::CATEGORY_ONECORE);
        }
        let Some(path) = host_candidates(config, arch).into_iter().next() else {
            errors.push(format!("{} not found", host_file_name(arch)));
            continue;
        };
        let mut tokens = Vec::new();
        for category in categories {
            match backend::list_tokens(&path, config.fake_engine, arch, category) {
                Ok(t) => tokens.extend(t),
                Err(e) => errors.push(format!("{arch} {category}: {e}")),
            }
        }
        if !tokens.is_empty() {
            lists.push((arch, tokens));
        }
    }
    if lists.is_empty() {
        return Err(SpeechError::Unavailable(BACKEND_ID, errors.join("; ")));
    }
    for e in &errors {
        log::info!("sapi: {e}");
    }
    let code_factory = voices::code_factory_enabled();
    Ok(voices::merge(&lists)
        .iter()
        .map(|(arch, t)| voices::describe(t, *arch))
        .filter(|v| voices::is_listed(v, code_factory))
        .collect())
}

/// Every installed voice (Windows only; empty elsewhere).
#[cfg(not(windows))]
pub fn list_voices(config: &SapiConfig) -> Result<Vec<SapiVoice>, SpeechError> {
    let _ = config;
    Err(SpeechError::Unavailable(BACKEND_ID, "Windows only".into()))
}

/// Describes the backend for the registry. `available` means this is
/// Windows and the 64-bit host was found; the hosts start in [`factory`],
/// which reports [`SpeechError::Unavailable`] if they cannot.
pub fn backend_info() -> BackendInfo {
    BackendInfo {
        id: BACKEND_ID,
        name: "Windows SAPI5 voices",
        priority: PRIORITY,
        opt_in: false,
        available: available() && !host_candidates(&SapiConfig::default(), Arch::X64).is_empty(),
    }
}

/// A factory that creates a `SapiBackend` (and its 64-bit host) on the
/// speech thread.
pub fn factory(config: SapiConfig) -> BackendFactory {
    #[cfg(windows)]
    {
        Box::new(move || Ok(Box::new(SapiBackend::new(config)?) as _))
    }
    #[cfg(not(windows))]
    {
        let _ = config;
        Box::new(|| {
            Err(SpeechError::Unavailable(
                BACKEND_ID,
                "SAPI5 is only available on Windows".into(),
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_describes_the_backend() {
        let i = backend_info();
        assert_eq!(i.id, "sapi");
        assert_eq!(i.priority, PRIORITY);
        assert!(!i.opt_in);
        if !cfg!(windows) {
            assert!(!i.available);
        }
    }

    #[test]
    fn configured_hosts_come_first_and_must_exist() {
        let dir = tempfile::tempdir().unwrap();
        let h = dir.path().join("host.exe");
        std::fs::write(&h, b"").unwrap();
        let config = SapiConfig {
            host_x86: Some(h.clone()),
            host_x64: Some(dir.path().join("missing.exe")),
            ..SapiConfig::default()
        };
        assert_eq!(host_candidates(&config, Arch::X86).first(), Some(&h));
        assert!(!host_candidates(&config, Arch::X64).contains(&dir.path().join("missing.exe")));
    }
}
