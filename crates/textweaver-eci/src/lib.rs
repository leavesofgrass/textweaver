//! ETI-Eloquence for textweaver (ADR-0007).
//!
//! Eloquence's engine is the ECI library: Code Factory's `eci.dll` on
//! Windows (32-bit only) or Voxin's `libibmeci.so` on Linux (64-bit). It is
//! loaded at run time, never linked, and always runs in a separate host
//! process, `textweaver-eci-host`, which synthesizes into a buffer and
//! reports each index mark with its sample offset over a framed pipe
//! protocol ([`protocol`]). [`EciBackend`] plays the audio in the main
//! process and turns those offsets into audio-clock word events, so the
//! highlight follows the exact word being heard.
//!
//! Wiring (for the application and the backend registry):
//! - [`backend_info`] describes the backend (id `"eci"`, the highest
//!   automatic priority, available when the library and host are found);
//! - [`factory`] builds it on the speech thread;
//! - [`NORMALIZES_NATIVELY`]: Eloquence expands numbers, dates, times,
//!   currency, and abbreviations itself, so the speech service should skip
//!   its own overlapping normalization transforms for this backend.
//!
//! Finding things:
//! - the ECI library: [`EciConfig::library`], else `TEXTWEAVER_ECI_LIBRARY`,
//!   else [`default_library_path`];
//! - the host: [`EciConfig::host`], else `TEXTWEAVER_ECI_HOST`, else next to
//!   the running executable (`textweaver-eci-host-i686.exe` first on
//!   Windows, which `cargo xtask eci-host` puts there, then
//!   `textweaver-eci-host[.exe]`), else a cargo `i686-pc-windows-msvc`
//!   build of it under the target directory (Windows development builds).

pub mod audio;
mod backend;
pub mod calibration;
pub mod dictionaries;
pub mod host;
pub mod language;
pub mod protocol;
pub mod voices;
pub mod wav;
pub mod words;

use std::path::PathBuf;

pub use audio::AudioOutput;
pub use backend::{EciBackend, STALL_TIMEOUT, Synthesis};
pub use dictionaries::Dictionaries;
use textweaver_speech::{BackendFactory, BackendId, BackendInfo};
pub use voices::VoiceParam;

/// The backend id.
pub const BACKEND_ID: BackendId = "eci";

/// Automatic-selection priority: above every other engine, so Eloquence is
/// chosen whenever it is installed (ADR-0007).
pub const PRIORITY: i32 = 1000;

/// Eloquence normalizes numbers, abbreviations, dates, times, and currency
/// itself ("9:30 a.m.", "Dr. Smith"); the speech service should skip its
/// overlapping transforms for this backend and keep punctuation and
/// pronunciation handling.
pub const NORMALIZES_NATIVELY: bool = true;

/// Environment variable naming the ECI library.
pub const LIBRARY_ENV: &str = "TEXTWEAVER_ECI_LIBRARY";
/// Environment variable naming the host executable.
pub const HOST_ENV: &str = "TEXTWEAVER_ECI_HOST";

/// Where the ECI library usually lives on this platform, if it is installed.
///
/// Windows: Code Factory "Eloquence for Windows" (`eci.dll`, 32-bit).
/// Linux: Voxin (`libibmeci.so`).
pub fn default_library_path() -> Option<PathBuf> {
    let candidates: &[&str] = if cfg!(windows) {
        &[r"C:\Program Files (x86)\Code Factory\Eloquence for Windows\eci.dll"]
    } else {
        &[
            "/opt/IBM/ibmtts/lib/libibmeci.so",
            "/usr/lib/libibmeci.so",
            "/opt/oralux/voxin/lib/libibmeci.so",
        ]
    };
    candidates.iter().map(PathBuf::from).find(|p| p.exists())
}

/// The ECI library to load: `TEXTWEAVER_ECI_LIBRARY` if set and non-empty,
/// else [`default_library_path`].
pub fn library_path() -> Option<PathBuf> {
    std::env::var_os(LIBRARY_ENV)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(default_library_path)
}

/// Backend configuration.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EciConfig {
    /// The ECI library; `None` uses [`library_path`].
    pub library: Option<PathBuf>,
    /// The host executable; `None` searches (see the crate docs).
    pub host: Option<PathBuf>,
    /// Engine sample rate in Hz (8000, 11025, or 22050); `None` keeps the
    /// engine's default (11025 Hz).
    pub sample_rate: Option<u32>,
    /// Where audio goes.
    pub output: AudioOutput,
    /// Run the host with its fake engine (tests only).
    pub fake_engine: bool,
    /// Community pronunciation dictionaries: on by default
    /// ([`Dictionaries::Auto`]), off, or from a directory.
    pub dictionaries: Dictionaries,
    /// How long the engine may go silent while it owes audio before it is
    /// treated as hung; `None` uses [`STALL_TIMEOUT`] (10 s).
    pub stall_timeout: Option<std::time::Duration>,
}

const HOST_NAME: &str = if cfg!(windows) {
    "textweaver-eci-host.exe"
} else {
    "textweaver-eci-host"
};

/// The 32-bit host `cargo xtask eci-host` installs next to the binaries.
pub const HOST_NAME_I686: &str = "textweaver-eci-host-i686.exe";

/// Host executables to try, in order (see the crate docs). Only existing
/// files are returned.
pub fn host_candidates(config: &EciConfig) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    if let Some(h) = &config.host {
        out.push(h.clone());
    }
    if let Some(h) = std::env::var_os(HOST_ENV).filter(|v| !v.is_empty()) {
        out.push(PathBuf::from(h));
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(PathBuf::from))
    {
        // The executable's directory, and its parent (test binaries live in
        // `target/<profile>/deps`).
        let dirs: Vec<PathBuf> = dir.ancestors().take(2).map(PathBuf::from).collect();
        for d in &dirs {
            if cfg!(windows) && !config.fake_engine {
                out.push(d.join(HOST_NAME_I686));
            }
            out.push(d.join(HOST_NAME));
        }
        if cfg!(windows) && !config.fake_engine {
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

/// Describes the backend for the registry. `available` means the library
/// and a host executable were found; the engine itself starts in
/// [`factory`], which reports [`SpeechError::Unavailable`](textweaver_speech::SpeechError)
/// if it cannot.
pub fn backend_info() -> BackendInfo {
    let config = EciConfig::default();
    BackendInfo {
        id: BACKEND_ID,
        name: "ETI-Eloquence",
        priority: PRIORITY,
        opt_in: false,
        available: library_path().is_some() && !host_candidates(&config).is_empty(),
    }
}

/// A factory that starts the host and creates an [`EciBackend`] on the
/// speech thread.
pub fn factory(config: EciConfig) -> BackendFactory {
    Box::new(move || Ok(Box::new(EciBackend::new(config)?) as _))
}
