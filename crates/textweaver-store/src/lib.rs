//! Persistence for textweaver.
//!
//! - [`Settings`] in `settings.toml` and keymap overrides in `keymap.toml`,
//!   written only on explicit change, atomically, preserving unknown keys,
//!   storing only non-default values (star rewrote `settings.json` on every
//!   set, wrote every default, and dropped unknown keys).
//! - Per-document [`DocState`] in `state/<doc-key>.json` (position,
//!   history, bookmarks, [`notes`] and highlights), with position saves
//!   coalesced by [`StateStore`] and flushed on demand and on drop. Since
//!   state format 2 ([`STATE_FORMAT`]) bookmarks carry ids, deletions are
//!   recorded, and notes replaced by another computer's edit are kept.
//! - [`Recent`] files and the [`library`]: folders, the bookshelf in
//!   `library.json`, sidecar sync wired to the folders, and search through
//!   [`fulltext::FullTextIndex`].
//! - [`settings_io`]: settings and key overrides exported to and imported
//!   from one JSON (or TOML) document, validated before anything is
//!   written, with backups.
//! - [`profiles`]: named settings profiles in `profiles.toml`, and
//!   [`stats`]: reading statistics in `stats.json` (Agent W3e).
//! - [`migrate`]: importing star's settings, positions, bookmarks, notes,
//!   highlights, recents, library, keybindings, and sidecars.
//! - Folder sidecars (`<folder>/.textweaver/progress.json`) and their merge
//!   rules, ported from `star/sync.py`, in [`sync`].
//! - [`sync_ids`]: `sync-ids.json`, each document's sync id for sync
//!   between computers (ADR-0049), keyed by the local path key.
//! - [`sync_scope`]: every setting marked portable (it syncs) or machine
//!   (it never does), and the portable values taken out of and put into
//!   settings.
//!
//! - [`cards`]: study cards made from notes, highlights, and headings,
//!   with the grades given them, in `cards/<doc-key>.json` (B1-f1).
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
pub mod cards;
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
pub mod sync_ids;
pub mod sync_scope;
pub mod time;

pub use atomic::atomic_write;
pub use cards::{Card, CardDeck, CardKind, CardSource, CardStore, Grade, Review};
pub use doc_state::{
    Anchor, Bookmark, ClockStamp, DEFAULT_DEBOUNCE, Deletion, Deletions, DocKey, DocState,
    LEGACY_STATE_FORMAT, MarkKind, MergeReport, NOTE_BACKUPS_MAX, NoteBackup, STATE_FORMAT,
    StateStore, TextStamp, percent,
};
pub use fulltext::{FullTextIndex, SearchHit, SimpleIndex};
pub use library::{Library, LibraryEntry, LibraryItem, LibrarySync, NotedDoc, ScannedDoc};
pub use notes::{
    Annotation, Backlink, Backlinks, Highlight, Note, NotesExport, Relation, RelationType,
};
pub use paths::{COMPONENTS_DIR, MEDICAL_OVERLAY_FILE, Paths, components_dir, find_in_components};
pub use profiles::{ProfileError, ProfileImport, Profiles};
pub use recent::{Recent, RecentEntry};
pub use settings::{
    AccessMode, AccessibilitySettings, AppleBackend, AppleSettings, AudioExportFormat,
    BrailleSettings, BrailleTableFormat, BrfCode, CitationReading, ColorSettings,
    CommunityLexiconSettings, ComponentsSettings, CursorPlacement, DEFAULT_HEADER_BUTTONS, DEFAULT_TOOLBAR_BUTTONS, DectalkSettings,
    DictationSettings, DigitRow, DisplaySettings, EciDictionaries, EciSettings, EditingSettings,
    EspeakHelper, EspeakSettings, ExportSettings, FootnoteMode, GuiAnnounce, GuiSettings,
    GuiSidebar, GuiWindow, HighlightSettings, HighlightShape, HintsLine, InterfaceAnnouncements,
    InterfaceSettings, KeyboardSettings, KeymapOverrides, KeymapPreset, LexiconSettings,
    LibrarySettings, MathBrailleCode, MathDisplay, MathEngine, MedicalLexiconSettings,
    NormalizationSettings, OcrEngine, PALETTE_MAX, PaletteEntry, PiperPhonemizer, PiperSettings,
    PositionPolicy, PreviewSettings, QuietScreen, REMOVED_SETTINGS, RESERVED_SETTINGS,
    ReadingSettings, RememberedVoice, RevisionReading, RtlDisplay, SapiSettings, SayAll, Settings,
    SettingsLoad, SettingsStore, SpeechSettings, StatsSettings, StopAt, SubtitleFormat,
    SubtitleKaraoke, SummarySettings, SyncSettings, TableMode, default_palette,
    drop_removed_settings,
};
pub use settings_io::{
    Applied, Change, ChangeArea, ExportFormat, ExportOptions, ImportMode, ImportPlan,
    SettingsIoError,
};
pub use stats::{DocStats, ReadingStats, StatsDelta};
pub use sync::{ConflictPolicy, SidecarStore};
pub use sync_ids::{SyncIdEntry, SyncIds};
pub use sync_scope::SettingScope;

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

/// Current time as Unix seconds (UTC). star stored zone-less local time
/// strings; textweaver stores UTC seconds and converts on migration.
pub fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}
