//! Persistence for textweaver.
//!
//! - [`Settings`] in `settings.toml` and keymap overrides in `keymap.toml`,
//!   written only on explicit change, atomically, preserving unknown keys
//!   (Star rewrote `settings.json` on every set and dropped unknown keys).
//! - Per-document [`DocState`] in `state/<doc-key>.json`.
//! - [`Recent`] files.
//! - Wave 1: sidecar merge (`.textweaver/progress.json`, ported from
//!   `star/sync.py`) in [`sync`].
//!
//! This crate depends only on `textweaver-core`; keymap overrides are stored
//! as plain strings and interpreted by `textweaver-keymap`.
//!
//! Owner: Agent C.

mod atomic;
mod doc_state;
mod paths;
mod recent;
mod settings;
pub mod sync;

pub use atomic::atomic_write;
pub use doc_state::{Bookmark, DocKey, DocState, StateStore};
pub use paths::Paths;
pub use recent::{Recent, RecentEntry};
pub use settings::{
    DisplaySettings, EditingSettings, FootnoteMode, HighlightSettings, KeymapOverrides,
    LibrarySettings, NormalizationSettings, ReadingSettings, Settings, SettingsStore,
    SpeechSettings, TableMode,
};

/// Persistence failures.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// File system error.
    #[error("{path}: {source}")]
    Io {
        /// The file involved.
        path: std::path::PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// TOML could not be parsed.
    #[error("{path}: {message}")]
    Parse {
        /// The file involved.
        path: std::path::PathBuf,
        /// Parser message.
        message: String,
    },
    /// No platform configuration directory could be determined.
    #[error("no configuration directory available")]
    NoConfigDir,
}

/// Current time as Unix seconds (UTC). Star stored zone-less local time
/// strings; textweaver stores UTC seconds and converts on migration.
pub fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}
