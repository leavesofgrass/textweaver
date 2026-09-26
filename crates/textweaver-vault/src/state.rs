//! An [`AnnotationStore`] over `textweaver-store`'s per-document state
//! files, until Agent C2's notes and highlights land.
//!
//! The Phase 0 `DocState` has no notes or highlights, but it preserves
//! unknown keys (`DocState::extra`), so this bridge keeps them there under
//! `notes` and `highlights`. At integration the orchestrator replaces this
//! with an implementation over C2's typed fields; the JSON shape here is
//! this crate's [`Note`](crate::Note) and [`Highlight`](crate::Highlight).
//! Library registrations are collected in memory for the caller to report
//! or hand to C2's library.

use std::path::Path;

use textweaver_store::{DocKey, StateStore};

use crate::VaultError;
use crate::model::{
    AnnotationStore, DocAnnotations, Highlight, LibraryEntry, Note, record_library,
};

/// The `DocState::extra` key holding notes.
pub const NOTES_KEY: &str = "notes";
/// The `DocState::extra` key holding highlights.
pub const HIGHLIGHTS_KEY: &str = "highlights";

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

fn decode<T: serde::de::DeserializeOwned>(
    doc: &Path,
    key: &str,
    value: Option<&serde_json::Value>,
) -> Result<Vec<T>, VaultError> {
    match value {
        None | Some(serde_json::Value::Null) => Ok(Vec::new()),
        Some(v) => serde_json::from_value(v.clone()).map_err(|e| VaultError::State {
            doc: doc.to_owned(),
            message: format!("{key}: {e}"),
        }),
    }
}

fn encode<T: serde::Serialize>(doc: &Path, items: &[T]) -> Result<serde_json::Value, VaultError> {
    serde_json::to_value(items).map_err(|e| VaultError::State {
        doc: doc.to_owned(),
        message: e.to_string(),
    })
}

impl AnnotationStore for StateStoreAnnotations {
    fn load(&mut self, doc: &Path) -> Result<DocAnnotations, VaultError> {
        let Some(state) = self.store.load(&DocKey::for_path(doc)) else {
            return Ok(DocAnnotations::default());
        };
        Ok(DocAnnotations {
            notes: decode::<Note>(doc, NOTES_KEY, state.extra.get(NOTES_KEY))?,
            highlights: decode::<Highlight>(doc, HIGHLIGHTS_KEY, state.extra.get(HIGHLIGHTS_KEY))?,
        })
    }

    fn save(&mut self, doc: &Path, annotations: &DocAnnotations) -> Result<(), VaultError> {
        let key = DocKey::for_path(doc);
        let mut state = self.store.load(&key).unwrap_or_default();
        if annotations.notes.is_empty() {
            state.extra.remove(NOTES_KEY);
        } else {
            state
                .extra
                .insert(NOTES_KEY.to_owned(), encode(doc, &annotations.notes)?);
        }
        if annotations.highlights.is_empty() {
            state.extra.remove(HIGHLIGHTS_KEY);
        } else {
            state.extra.insert(
                HIGHLIGHTS_KEY.to_owned(),
                encode(doc, &annotations.highlights)?,
            );
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use textweaver_core::{CharPos, CharRange};

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
                pos: CharPos(3),
                text: "hello".into(),
                ..Note::default()
            }],
            highlights: vec![Highlight {
                id: "h".into(),
                range: CharRange::new(CharPos(1), CharPos(4)),
                color: "#ffff00".into(),
                ts: 1,
            }],
        };
        bridge.save(&doc, &ann).unwrap();
        assert_eq!(bridge.load(&doc).unwrap(), ann);
        let back = store.load(&key).unwrap();
        assert_eq!(back.position, CharPos(10));
        assert_eq!(back.bookmarks.len(), 1);

        bridge.save(&doc, &DocAnnotations::default()).unwrap();
        assert!(store.load(&key).unwrap().extra.is_empty());
    }

    #[test]
    fn corrupt_notes_are_an_error_not_silence() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::new(dir.path().to_owned());
        let doc = dir.path().join("x.md");
        let mut state = textweaver_store::DocState::default();
        state
            .extra
            .insert(NOTES_KEY.into(), serde_json::json!("not a list"));
        store.save(&DocKey::for_path(&doc), &state).unwrap();
        let mut bridge = StateStoreAnnotations::new(store);
        assert!(matches!(bridge.load(&doc), Err(VaultError::State { .. })));
    }
}
