//! Persistence for textweaver.
//!
//! - [`Settings`] in `settings.toml` and keymap overrides in `keymap.toml`,
//!   written only on explicit change, atomically, preserving unknown keys,
//!   storing only non-default values (Star rewrote `settings.json` on every
//!   set, wrote every default, and dropped unknown keys).
//! - Per-document [`DocState`] in `state/<doc-key>.json` (position,
//!   history, bookmarks, [`notes`] and highlights), with position saves
//!   coalesced by [`StateStore`] and flushed on demand and on drop.
//! - [`Recent`] files and the [`library`]: folders, the bookshelf in
//!   `library.json`, sidecar sync wired to the folders, and search through
//!   [`fulltext::FullTextIndex`].
//! - [`settings_io`]: settings and key overrides exported to and imported
//!   from one JSON (or TOML) document, validated before anything is
//!   written, with backups.
//! - [`migrate`]: importing Star's settings, positions, bookmarks, notes,
//!   highlights, recents, library, keybindings, and sidecars.
//! - Folder sidecars (`<folder>/.textweaver/progress.json`) and their merge
//!   rules, ported from `star/sync.py`, in [`sync`].
//!
//! Timestamps are UTC: Unix seconds in textweaver's own files, RFC 3339
//! strings in the sidecar ([`time`]).
//!
//! This crate depends only on `textweaver-core`; keymap overrides are stored
//! as plain strings and interpreted by `textweaver-keymap`.
//!
//! Owner: Agent C.

mod atomic;
mod doc_state;
pub mod fulltext;
pub mod library;
pub mod migrate;
pub mod notes;
mod paths;
mod recent;
mod settings;
pub mod settings_io;
pub mod sync;
pub mod time;

pub use atomic::atomic_write;
pub use doc_state::{Bookmark, DEFAULT_DEBOUNCE, DocKey, DocState, StateStore, percent};
pub use fulltext::{FullTextIndex, SearchHit, SimpleIndex};
pub use library::{Library, LibraryEntry, LibraryItem, LibrarySync, ScannedDoc};
pub use notes::{Annotation, Highlight, Note, NotesExport, Relation};
pub use paths::Paths;
pub use recent::{Recent, RecentEntry};
pub use settings::{
    AppleBackend, AppleSettings, CommunityLexiconSettings, DisplaySettings, EciDictionaries,
    EciSettings, EditingSettings, ExportSettings, FootnoteMode, HighlightSettings,
    KeyboardSettings, KeymapOverrides, LibrarySettings, NormalizationSettings, ReadingSettings,
    SapiSettings, Settings, SettingsLoad, SettingsStore, SpeechSettings, SubtitleFormat, TableMode,
};
pub use settings_io::{
    Applied, Change, ChangeArea, ExportFormat, ExportOptions, ImportMode, ImportPlan,
    SettingsIoError,
};
pub use sync::{ConflictPolicy, SidecarStore};

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
