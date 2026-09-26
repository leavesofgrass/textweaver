//! ETI-Eloquence for textweaver (ADR-0007).
//!
//! Eloquence's engine is the ECI library: Code Factory's `eci.dll` on
//! Windows (32-bit only) or Voxin's `libibmeci.so` on Linux. It is loaded at
//! run time, never linked, and always runs in a separate host process
//! (`textweaver-eci-host`), which synthesizes into a buffer and reports each
//! index mark with its sample offset. This crate's backend plays the audio in
//! the main process and turns those offsets into audio-clock word events.
//!
//! Owner: Agent E. Phase 0 holds only library discovery.

use std::path::PathBuf;

/// Where the ECI library usually lives on this platform, if it is installed.
///
/// Windows: Code Factory "Eloquence for Windows" (`eci.dll`, 32-bit).
/// Linux: Voxin (`libibmeci.so`).
pub fn default_library_path() -> Option<PathBuf> {
    let candidates: &[&str] = if cfg!(windows) {
        &[r"C:\Program Files (x86)\Code Factory\Eloquence for Windows\eci.dll"]
    } else {
        &["/opt/IBM/ibmtts/lib/libibmeci.so", "/usr/lib/libibmeci.so"]
    };
    candidates.iter().map(PathBuf::from).find(|p| p.exists())
}
