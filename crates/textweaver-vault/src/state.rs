//! An [`AnnotationStore`] over `textweaver-store`'s per-document state
//! files: the typed notes and highlights in `DocState`, shared with the
//! reader, so a vault export sees what was written while reading and an
//! import lands where the reader finds it.
//!
//! The vault works on the store's own [`Note`](textweaver_store::Note) and
//! [`Highlight`](textweaver_store::Highlight), so loading and saving pass
//! them through whole: ranges, created times, colors, and unknown keys
//! survive a round trip. Library registrations are collected in memory;
//! the caller saves them to the library with [`save_library`].

use std::path::Path;

use textweaver_store::{DocKey, StateStore};

use crate::VaultError;
use crate::model::{AnnotationStore, DocAnnotations, LibraryEntry, record_library};

/// Notes and highlights kept in the per-document state files.
#[derive(Debug)]
pub struct StateStoreAnnotations {
    store: StateStore,
    library: Vec<LibraryEntry>,
}

impl StateStoreAnnotations {
    /// A bridge over `store`.
    pub fn new(store: StateStore) -> Self {
        StateStoreAnnotations {
            store,
            library: Vec::new(),
        }
    }

    /// Documents registered so far (not persisted by this bridge).
    pub fn library(&self) -> &[LibraryEntry] {
        &self.library
    }

    /// The underlying state store.
    pub fn store(&self) -> &StateStore {
        &self.store
    }
}

impl AnnotationStore for StateStoreAnnotations {
    fn load(&mut self, doc: &Path) -> Result<DocAnnotations, VaultError> {
        let Some(state) = self.store.load(&DocKey::for_path(doc)) else {
            return Ok(DocAnnotations::default());
        };
        Ok(DocAnnotations {
            notes: state.notes,
            highlights: state.highlights,
        })
    }

    fn save(&mut self, doc: &Path, annotations: &DocAnnotations) -> Result<(), VaultError> {
        let key = DocKey::for_path(doc);
        let mut state = self.store.load(&key).unwrap_or_default();
        state.notes.clone_from(&annotations.notes);
        state.highlights.clone_from(&annotations.highlights);
        self.store.save(&key, &state)?;
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

/// Adds the documents an import registered to the library file
/// (`library.json`), keeping every entry already there. Returns how many
/// documents were new to the library. An unreadable library file is an
/// error, and is left as it is.
pub fn save_library(entries: &[LibraryEntry], file: &Path) -> Result<usize, VaultError> {
    if entries.is_empty() {
        return Ok(0);
    }
    let mut library = textweaver_store::Library::load(file)?;
    let now = textweaver_store::now_ts();
    let mut added = 0;
    for e in entries {
        if library.get(&e.path).is_none() {
            added += 1;
        }
        library.record_open_at(&e.path, &e.title, &e.format, now);
    }
    library.save(file)?;
    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Highlight, Note};
    use textweaver_core::{CharPos, CharRange};

    #[test]
    fn registered_documents_are_saved_to_the_library() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("library.json");
        let mut existing = textweaver_store::Library::default();
        existing.record_open_at(&dir.path().join("book.md"), "Book", "markdown", 5);
        existing.save(&file).unwrap();
        let entries = vec![
            LibraryEntry {
                path: dir.path().join("vault/Cell.md"),
                title: "Cell".into(),
                format: "markdown".into(),
            },
            LibraryEntry {
                path: dir.path().join("book.md"),
                title: "Book".into(),
                format: "markdown".into(),
            },
        ];
        assert_eq!(save_library(&entries, &file).unwrap(), 1);
        let back = textweaver_store::Library::load(&file).unwrap();
        assert_eq!(back.entries.len(), 2);
        assert_eq!(
            back.get(&dir.path().join("vault/Cell.md")).unwrap().title,
            "Cell"
        );
        // Again: nothing new.
        assert_eq!(save_library(&entries, &file).unwrap(), 0);
        // A damaged library is not overwritten.
        std::fs::write(&file, "{ not json").unwrap();
        assert!(save_library(&entries, &file).is_err());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "{ not json");
    }

    #[test]
    fn keeps_notes_beside_the_position_and_bookmarks() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::new(dir.path().to_owned());
        let doc = dir.path().join("book.md");
        let key = DocKey::for_path(&doc);
        let mut state = textweaver_store::DocState::default();
        state.set_position(CharPos(10), 100);
        state.add_bookmark(Some("here"), CharPos(5), 100);
        store.save(&key, &state).unwrap();

        let mut bridge = StateStoreAnnotations::new(store.clone());
        assert!(bridge.load(&doc).unwrap().is_empty());
        let ann = DocAnnotations {
            notes: vec![Note {
                id: "n".into(),
                range: CharRange::new(CharPos(3), CharPos(3)),
                note: "hello".into(),
                ..Note::default()
            }],
            highlights: vec![Highlight {
                id: "h".into(),
                range: CharRange::new(CharPos(1), CharPos(4)),
                color: "#ffff00".into(),
                ts: 1,
                ..Highlight::default()
            }],
        };
        bridge.save(&doc, &ann).unwrap();
        assert_eq!(bridge.load(&doc).unwrap(), ann);
        let back = store.load(&key).unwrap();
        assert_eq!(back.position, CharPos(10));
        assert_eq!(back.bookmarks.len(), 1);

        bridge.save(&doc, &DocAnnotations::default()).unwrap();
        let empty = store.load(&key).unwrap();
        assert!(empty.notes.is_empty() && empty.highlights.is_empty());
    }

    #[test]
    fn note_ranges_and_unknown_fields_survive_a_vault_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::new(dir.path().to_owned());
        let doc = dir.path().join("x.md");
        let key = DocKey::for_path(&doc);
        let mut state = textweaver_store::DocState::default();
        let mut n = textweaver_store::Note {
            id: "n1".into(),
            range: CharRange::new(CharPos(3), CharPos(9)),
            note: "old".into(),
            created: 7,
            ..textweaver_store::Note::default()
        };
        n.extra
            .insert("sr_state".into(), serde_json::json!({"due": 1}));
        state.notes.push(n);
        store.save(&key, &state).unwrap();

        let mut bridge = StateStoreAnnotations::new(store.clone());
        let mut ann = bridge.load(&doc).unwrap();
        assert_eq!(ann.notes[0].range.start, CharPos(3));
        ann.notes[0].note = "new".into();
        bridge.save(&doc, &ann).unwrap();

        let back = store.load(&key).unwrap();
        assert_eq!(back.notes[0].note, "new");
        assert_eq!(back.notes[0].range, CharRange::new(CharPos(3), CharPos(9)));
        assert_eq!(back.notes[0].created, 7);
        assert!(back.notes[0].extra.contains_key("sr_state"));
    }
}
