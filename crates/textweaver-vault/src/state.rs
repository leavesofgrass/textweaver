//! An [`AnnotationStore`] over `textweaver-store`'s per-document state
//! files: the typed notes and highlights in `DocState` (Agent C2), shared
//! with the reader, so a vault export sees what was written while reading
//! and an import lands where the reader finds it.
//!
//! The vault model differs slightly from the store's: a vault note is
//! attached at a position, a store note to a range (empty for a point).
//! Saving keeps an existing note's range when its start has not moved, and
//! keeps fields the vault does not know (created time, color, unknown
//! keys). Library registrations are collected in memory for the caller to
//! report or hand to the library.

use std::path::{Path, PathBuf};

use textweaver_core::CharRange;
use textweaver_store::{DocKey, StateStore};

use crate::VaultError;
use crate::model::{
    AnnotationStore, DocAnnotations, Highlight, LibraryEntry, Note, Relation, RelationType,
    record_library,
};

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

fn note_from_store(n: &textweaver_store::Note) -> Note {
    Note {
        id: n.id.clone(),
        pos: n.range.start,
        anchor: n.anchor.clone(),
        text: n.note.clone(),
        tags: n.tags.clone(),
        cite: n.cite.clone(),
        ts: n.ts,
        relations: n
            .relations
            .iter()
            .filter_map(|r| {
                Some(Relation {
                    rel_type: RelationType::parse(&r.rel_type)?,
                    target_doc: PathBuf::from(&r.target_doc),
                    target_id: r.target_id.clone(),
                    note: r.note.clone(),
                })
            })
            .collect(),
    }
}

fn note_to_store(n: &Note, old: Option<&textweaver_store::Note>) -> textweaver_store::Note {
    let mut out = old.cloned().unwrap_or_default();
    if old.is_none_or(|o| o.range.start != n.pos) {
        out.range = CharRange::new(n.pos, n.pos);
    }
    if out.created == 0 {
        out.created = n.ts;
    }
    out.id.clone_from(&n.id);
    out.anchor.clone_from(&n.anchor);
    out.note.clone_from(&n.text);
    out.tags.clone_from(&n.tags);
    out.cite.clone_from(&n.cite);
    out.ts = n.ts;
    out.relations = n
        .relations
        .iter()
        .map(|r| textweaver_store::Relation {
            rel_type: r.rel_type.as_str().to_owned(),
            target_doc: r.target_doc.to_string_lossy().into_owned(),
            target_id: r.target_id.clone(),
            note: r.note.clone(),
        })
        .collect();
    out
}

fn highlight_from_store(h: &textweaver_store::Highlight) -> Highlight {
    Highlight {
        id: h.id.clone(),
        range: h.range,
        color: h.color.clone(),
        ts: h.ts,
    }
}

fn highlight_to_store(
    h: &Highlight,
    old: Option<&textweaver_store::Highlight>,
) -> textweaver_store::Highlight {
    let mut out = old.cloned().unwrap_or_default();
    out.id.clone_from(&h.id);
    out.range = h.range;
    out.color.clone_from(&h.color);
    out.ts = h.ts;
    out
}

impl AnnotationStore for StateStoreAnnotations {
    fn load(&mut self, doc: &Path) -> Result<DocAnnotations, VaultError> {
        let Some(state) = self.store.load(&DocKey::for_path(doc)) else {
            return Ok(DocAnnotations::default());
        };
        Ok(DocAnnotations {
            notes: state.notes.iter().map(note_from_store).collect(),
            highlights: state.highlights.iter().map(highlight_from_store).collect(),
        })
    }

    fn save(&mut self, doc: &Path, annotations: &DocAnnotations) -> Result<(), VaultError> {
        let key = DocKey::for_path(doc);
        let mut state = self.store.load(&key).unwrap_or_default();
        state.notes = annotations
            .notes
            .iter()
            .map(|n| note_to_store(n, state.notes.iter().find(|o| o.id == n.id)))
            .collect();
        state.highlights = annotations
            .highlights
            .iter()
            .map(|h| highlight_to_store(h, state.highlights.iter().find(|o| o.id == h.id)))
            .collect();
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
        assert_eq!(ann.notes[0].pos, CharPos(3));
        ann.notes[0].text = "new".into();
        bridge.save(&doc, &ann).unwrap();

        let back = store.load(&key).unwrap();
        assert_eq!(back.notes[0].note, "new");
        assert_eq!(back.notes[0].range, CharRange::new(CharPos(3), CharPos(9)));
        assert_eq!(back.notes[0].created, 7);
        assert!(back.notes[0].extra.contains_key("sr_state"));
    }
}
