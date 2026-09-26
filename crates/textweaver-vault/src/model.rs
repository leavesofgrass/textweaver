//! The notes and highlights this crate reads and writes, and the store it
//! reads them from and writes them back to.
//!
//! There is one notes model: the store's [`Note`], [`Highlight`], and
//! [`Relation`] (`textweaver_store::notes`), the same values the reader,
//! `tw marks`, and sidecar sync use. A vault note is attached where its
//! range starts; an imported note gets an empty range at its position.
//! [`StateStoreAnnotations`](crate::StateStoreAnnotations) reads and writes
//! them in the per-document state files.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
pub use textweaver_store::notes::{DEFAULT_HIGHLIGHT_COLOR, RelationType, color_name};
pub use textweaver_store::{Highlight, Note, Relation};

use crate::VaultError;

/// Everything annotated on one document.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DocAnnotations {
    /// Notes, in any order.
    pub notes: Vec<Note>,
    /// Highlights, in any order.
    pub highlights: Vec<Highlight>,
}

impl DocAnnotations {
    /// True when there are no notes and no highlights.
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty() && self.highlights.is_empty()
    }

    /// The note with `id`.
    pub fn note(&self, id: &str) -> Option<&Note> {
        self.notes.iter().find(|n| n.id == id)
    }

    /// The note with `id`, mutably.
    pub fn note_mut(&mut self, id: &str) -> Option<&mut Note> {
        self.notes.iter_mut().find(|n| n.id == id)
    }

    /// Notes in document order (by position, then id).
    pub fn sorted_notes(&self) -> Vec<&Note> {
        let mut v: Vec<&Note> = self.notes.iter().collect();
        v.sort_by(|a, b| {
            a.range
                .start
                .cmp(&b.range.start)
                .then_with(|| a.id.cmp(&b.id))
        });
        v
    }

    /// Highlights in document order (by start, then id).
    pub fn sorted_highlights(&self) -> Vec<&Highlight> {
        let mut v: Vec<&Highlight> = self.highlights.iter().collect();
        v.sort_by(|a, b| {
            a.range
                .start
                .cmp(&b.range.start)
                .then_with(|| a.id.cmp(&b.id))
        });
        v
    }
}

/// Where notes and highlights live, and where imported documents are
/// registered. The vault reads and writes only through this, so the app,
/// the CLI, and tests can each plug in their own store.
pub trait AnnotationStore {
    /// The notes and highlights of the document at `doc` (empty when none).
    fn load(&mut self, doc: &Path) -> Result<DocAnnotations, VaultError>;

    /// Replaces the notes and highlights of the document at `doc`.
    fn save(&mut self, doc: &Path, annotations: &DocAnnotations) -> Result<(), VaultError>;

    /// Adds the document at `doc` to the library (Star's `library[path]`
    /// entry: title and format), or refreshes its entry.
    fn register_document(
        &mut self,
        doc: &Path,
        title: &str,
        format: &str,
    ) -> Result<(), VaultError>;
}

/// A library entry recorded by [`MemoryStore`] or
/// [`StateStoreAnnotations`](crate::StateStoreAnnotations).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LibraryEntry {
    /// The document.
    pub path: PathBuf,
    /// Its title.
    pub title: String,
    /// Its format (`markdown` for vault notes).
    pub format: String,
}

/// An in-memory [`AnnotationStore`], for tests and dry runs.
#[derive(Clone, Debug, Default)]
pub struct MemoryStore {
    /// Annotations per document.
    pub docs: BTreeMap<PathBuf, DocAnnotations>,
    /// Registered documents, in registration order, one entry per path.
    pub library: Vec<LibraryEntry>,
}

impl MemoryStore {
    /// An empty store.
    pub fn new() -> Self {
        MemoryStore::default()
    }
}

impl AnnotationStore for MemoryStore {
    fn load(&mut self, doc: &Path) -> Result<DocAnnotations, VaultError> {
        Ok(self.docs.get(doc).cloned().unwrap_or_default())
    }

    fn save(&mut self, doc: &Path, annotations: &DocAnnotations) -> Result<(), VaultError> {
        if annotations.is_empty() {
            self.docs.remove(doc);
        } else {
            self.docs.insert(doc.to_owned(), annotations.clone());
        }
        Ok(())
    }

    fn register_document(
        &mut self,
        doc: &Path,
        title: &str,
        format: &str,
    ) -> Result<(), VaultError> {
        record_library(&mut self.library, doc, title, format);
        Ok(())
    }
}

/// Adds or refreshes a library entry in a list.
pub(crate) fn record_library(list: &mut Vec<LibraryEntry>, doc: &Path, title: &str, format: &str) {
    let entry = LibraryEntry {
        path: doc.to_owned(),
        title: title.to_owned(),
        format: format.to_owned(),
    };
    match list.iter_mut().find(|e| e.path == doc) {
        Some(e) => *e = entry,
        None => list.push(entry),
    }
}

/// A short stable id derived from `seed` (64-bit FNV-1a, 12 hex digits).
///
/// Star assigned random 8-hex-digit ids. Deriving ids from a stable seed
/// (a vault note's path, a highlight's range and color) makes re-imports
/// and re-exports idempotent without remembering anything between runs.
pub fn derive_id(seed: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in seed.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{:012x}", h >> 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_ids_are_stable_and_short() {
        let a = derive_id("/vault/a.md");
        assert_eq!(a, derive_id("/vault/a.md"));
        assert_ne!(a, derive_id("/vault/b.md"));
        assert_eq!(a.len(), 12);
    }

    #[test]
    fn memory_store_round_trips_and_forgets_empty() {
        let mut s = MemoryStore::new();
        let p = Path::new("/d.md");
        let mut a = DocAnnotations::default();
        a.notes.push(Note {
            id: "x".into(),
            ..Note::default()
        });
        s.save(p, &a).unwrap();
        assert_eq!(s.load(p).unwrap(), a);
        s.save(p, &DocAnnotations::default()).unwrap();
        assert!(s.docs.is_empty());
        s.register_document(p, "D", "markdown").unwrap();
        s.register_document(p, "D2", "markdown").unwrap();
        assert_eq!(s.library.len(), 1);
        assert_eq!(s.library[0].title, "D2");
    }
}
