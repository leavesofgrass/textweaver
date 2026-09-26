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
//! - [`profiles`]: named settings profiles in `profiles.toml`, and
//!   [`stats`]: reading statistics in `stats.json` (Agent W3e).
//! - [`migrate`]: importing Star's settings, positions, bookmarks, notes,
//!   highlights, recents, library, keybindings, and sidecars.
//! - Folder sidecars (`<folder>/.textweaver/progress.json`) and their merge
//!   rules, ported from `star/sync.py`, in [`sync`].
//!
//! - [`reading_aids`]: the saved form of the `[reading_aids]` settings,
//!   which `textweaver-aids` converts into its working types.
//!
//! Timestamps are UTC: Unix seconds in textweaver's own files, RFC 3339
//! strings in the sidecar ([`time`]).
//!
//! This crate depends only on `textweaver-core` among the workspace crates
//! (ADR-0001, checked by `cargo xtask deps --check`): keymap overrides are
//! stored as plain strings and interpreted by `textweaver-keymap`, and the
//! reading-aid settings are plain data that `textweaver-aids` converts.
//!
//! Owner: Agent C.

mod atomic;
mod doc_state;
pub mod fulltext;
pub mod library;
pub mod migrate;
pub mod notes;
mod paths;
pub mod profiles;
pub mod reading_aids;
mod recent;
mod settings;
pub mod settings_io;
pub mod stats;
pub mod sync;
pub mod time;

pub use atomic::atomic_write;
pub use doc_state::{
    Anchor, Bookmark, DEFAULT_DEBOUNCE, DocKey, DocState, StateStore, TextStamp, percent,
};
pub use fulltext::{FullTextIndex, SearchHit, SimpleIndex};
pub use library::{Library, LibraryEntry, LibraryItem, LibrarySync, ScannedDoc};
pub use notes::{Annotation, Highlight, Note, NotesExport, Relation, RelationType};
pub use paths::Paths;
pub use profiles::{ProfileError, ProfileImport, Profiles};
pub use recent::{Recent, RecentEntry};
pub use settings::{
    AccessMode, AccessibilitySettings, AppleBackend, AppleSettings, CitationReading,
    CommunityLexiconSettings, CursorPlacement, DigitRow, DisplaySettings, EciDictionaries,
    EciSettings, EditingSettings, ExportSettings, FootnoteMode, HighlightSettings,
    InterfaceSettings, KeyboardSettings, KeymapOverrides, KeymapPreset, LexiconSettings,
    LibrarySettings, NormalizationSettings, PreviewSettings, RESERVED_SETTINGS, ReadingSettings,
    SapiSettings, SayAll, Settings, SettingsLoad, SettingsStore, SpeechSettings, StatsSettings,
    SubtitleFormat, TableMode,
};
pub use settings_io::{
    Applied, Change, ChangeArea, ExportFormat, ExportOptions, ImportMode, ImportPlan,
    SettingsIoError,
};
pub use stats::{DocStats, ReadingStats, StatsDelta};
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
