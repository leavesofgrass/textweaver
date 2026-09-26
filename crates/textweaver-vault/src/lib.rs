//! Obsidian vault import and export, ported from `star/obsidian.py`
//! (`docs/star-parity.md` Part 3 §4.8).
//!
//! - [`export_documents`] writes documents' notes and highlights into a
//!   vault: one Markdown note per note, with front matter and its relations
//!   as Dataview fields under `## Links`, plus one document note per
//!   document with its highlights (quotes with block ids) and links to its
//!   notes.
//! - [`read_vault`] and [`apply`] (together, [`import_vault`]) read a
//!   vault's notes as documents with their links, and bring textweaver's
//!   own exported notes and highlights back onto their documents, so a
//!   vault round-trips.
//!
//! The crate reads and writes notes only through [`AnnotationStore`];
//! [`MemoryStore`] serves tests and dry runs, and
//! [`StateStoreAnnotations`] reads and writes the typed notes and
//! highlights in the per-document state files, the same ones the reader
//! uses (`docs/vault.md` is the user guide).
//!
//! Owner: Agent J.

mod export;
mod frontmatter;
mod import;
mod links;
mod model;
mod names;
mod state;
mod walk;

pub use export::{
    ExportDocument, ExportOptions, ExportReport, document_id, export_documents, highlight_id,
};
pub use frontmatter::{FmValue, FrontMatter, split as split_front_matter};
pub use import::{
    ImportMode, ImportOptions, ImportReport, NoteKind, VaultNote, VaultRead, apply, import_vault,
    parse_note, read_vault,
};
pub use links::{Link, extract_links, first_line, inline_tags, strip_link_syntax};
pub use model::{
    AnnotationStore, DEFAULT_HIGHLIGHT_COLOR, DocAnnotations, Highlight, LibraryEntry, MemoryStore,
    Note, Relation, RelationType, color_name, derive_id,
};
pub use names::{MAX_NAME_CHARS, NameAllocator, sanitize as sanitize_name};
pub use state::{StateStoreAnnotations, save_library};
pub use walk::note_files;

/// The tag on a vault note's node note (Star's `_NODE_TAG`).
pub const NODE_TAG: &str = "obsidian-note";

/// The tag on the document notes textweaver writes.
pub const DOCUMENT_TAG: &str = "textweaver-document";

/// Vault failures.
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    /// A file or folder could not be read or written.
    #[error("{path}: {source}")]
    Io {
        /// The file or folder.
        path: std::path::PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// The vault path is not a folder.
    #[error("{0} is not a folder")]
    NotAFolder(std::path::PathBuf),
    /// Writing through the store failed.
    #[error(transparent)]
    Store(#[from] textweaver_store::StoreError),
    /// A document's saved notes could not be read.
    #[error("saved notes for {doc} could not be read: {message}")]
    State {
        /// The document.
        doc: std::path::PathBuf,
        /// What was wrong.
        message: String,
    },
}
