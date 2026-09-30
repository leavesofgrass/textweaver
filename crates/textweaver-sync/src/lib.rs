//! Sync for textweaver: notes, highlights, bookmarks, places, and reading
//! statistics carried between computers through a folder the user chooses,
//! with no account, no server, and no networking inside textweaver.
//!
//! - [`Clock`] and [`Stamp`]: the hybrid logical clock. A stamp is the wall
//!   time, never less than the newest time seen plus one; the computer's id
//!   breaks ties; a stamp more than a day ahead is flagged.
//! - [`merge`]: the merge types. Registers by id where the newest change
//!   wins and a deletion is a record that loses to a later edit, sets where
//!   adding wins with removal records, and per-computer counters that are
//!   summed. Merge order and repeats never change the result.
//! - [`DocRecord`]: one record per document (each computer's place, the
//!   bookmarks, notes, and highlights, and the statistics). Its merge lists
//!   what was added, replaced, removed, or brought back ([`MergeReport`]),
//!   so the app can say which note a newer edit replaced.
//! - [`SyncFolder`]: the folder format. `textweaver-sync/format.json`, and
//!   for each computer `devices/<device-id>/device.json` and
//!   `devices/<device-id>/docs/<sync-id>.json`. Each computer writes only
//!   its own files, by atomic replace, and never deletes or changes another
//!   computer's. Damaged files are skipped and reported; a newer format
//!   makes sync read-only.
//! - [`groups`]: the groups that are not about one document (portable
//!   settings, profiles, key overrides, the word list, the glossary and
//!   pronunciations, favorite voices), one file per group and computer.
//! - [`docid`]: document identity. Each document gets a random sync id,
//!   found by the file's SHA-256, then the SHA-256 of its text, then its
//!   library folder's id and path inside it; a DOI or ISBN is only
//!   suggested. The local `sync-ids.json` maps path keys to sync ids.
//! - [`Identity`]: this computer's random id and install token, in an
//!   install marker in the local state folder that also catches a copied
//!   state folder.
//!
//! The design is ADR-0049, "Sync beyond the place".
//!
//! No computer, user, or account name, and no path, is ever written to the
//! sync folder. Nothing is encrypted (the owner's decision).
//!
//! This crate depends only on `textweaver-core` and `textweaver-store`
//! among the workspace crates, and on sha2 for hashing (checked by
//! `cargo xtask deps --check`).

mod clock;
pub mod docid;
pub mod folder;
pub mod groups;
mod identity;
mod ids;
pub mod merge;
pub mod record;

pub use clock::{AHEAD_LIMIT_MS, Clock, ClockAhead, Stamp, wall_ms};
pub use docid::{Found, Identify, IdentityIndex, Resolved, Suggestion};
pub use folder::{DeviceInfo, DocRead, FileKind, Merged, Opened, Problem, ReadOnly, SyncFolder};
pub use groups::{GroupChange, GroupFile, GroupRecord};
pub use identity::{
    DEFAULT_LABEL_WORD, Identity, IdentityEvent, MARKER_FILE, MAX_LABEL_CHARS, check_label,
    default_label, local_names,
};
pub use ids::{DeviceId, ID_HEX_DIGITS, InstallToken, LibraryId, SyncId};
pub use merge::{AddWinsSet, ChangeKind, Counter, Maximum, Register, RegisterMap};
pub use record::{
    Change, DocIdentity, DocRecord, DocStatsRecord, FORMAT, ItemKind, MergeReport, Place, Previous,
    RecentHashes,
};

/// Errors from sync.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    /// A file could not be read or written.
    #[error("{path}: {source}")]
    Io {
        /// The file or folder.
        path: std::path::PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// A file is damaged or cut short.
    #[error("damaged: {0}")]
    Damaged(String),
    /// A file is larger than any textweaver writes.
    #[error("file too large")]
    TooLarge,
    /// A file is from a newer textweaver.
    #[error("format {found} is newer than this textweaver reads")]
    NewerFormat {
        /// Its format number.
        found: u32,
    },
    /// An id is not 32 lower-case hex digits.
    #[error("not a {what}")]
    BadId {
        /// Which kind of id.
        what: &'static str,
    },
    /// Two records for different documents were merged.
    #[error("the records are for different documents")]
    DifferentDocument,
    /// Sync is read-only.
    #[error("{0}")]
    ReadOnly(ReadOnly),
    /// The chosen folder does not exist (a USB stick not plugged in, say).
    #[error("sync folder missing")]
    FolderMissing,
    /// A label cannot be used.
    #[error("label {0}")]
    Label(&'static str),
}

impl From<textweaver_store::StoreError> for SyncError {
    fn from(e: textweaver_store::StoreError) -> Self {
        match e {
            textweaver_store::StoreError::Io { path, source } => SyncError::Io { path, source },
            other => SyncError::Damaged(other.to_string()),
        }
    }
}
