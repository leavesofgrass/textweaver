//! The knowledge graph through a vault (B1-g2): export to a temporary
//! vault, import into an empty store, and the graph's relations compare
//! equal, a type textweaver does not know included.

use std::path::PathBuf;

use textweaver_core::{CharPos, CharRange};
use textweaver_store::{Graph, GraphFormat, NotedDoc};
use textweaver_vault::{
    AnnotationStore, DocAnnotations, ExportDocument, ExportOptions, ImportOptions, MemoryStore,
    Note, Relation, export_documents, import_vault,
};

fn note(id: &str, pos: usize, text: &str, rels: Vec<Relation>) -> Note {
    Note {
        id: id.into(),
        range: CharRange::new(CharPos(pos), CharPos(pos)),
        anchor: text.into(),
        note: text.into(),
        relations: rels,
        created: 1_790_000_000,
        ts: 1_790_000_000,
        ..Note::default()
    }
}

fn rel(t: &str, doc: &str, id: &str, why: &str) -> Relation {
    Relation {
        rel_type: t.into(),
        target_doc: doc.into(),
        target_id: id.into(),
        note: why.into(),
    }
}

/// The library's documents with notes, as the graph reads them.
fn noted(s: &MemoryStore, docs: &[(PathBuf, &str)]) -> Vec<NotedDoc> {
    docs.iter()
        .map(|(path, title)| {
            let mut notes = s.docs.get(path).cloned().unwrap_or_default().notes;
            notes.sort_by(|a, b| a.id.cmp(&b.id));
            NotedDoc {
                path: path.clone(),
                title: (*title).to_owned(),
                notes,
            }
        })
        .collect()
}

#[test]
fn graph_relations_survive_a_vault_round_trip_with_an_unknown_type() {
    let bio = PathBuf::from("/library/biology.md");
    let chem = PathBuf::from("/library/chemistry.md");
    let chem_s = chem.to_string_lossy().into_owned();
    let bio_s = bio.to_string_lossy().into_owned();
    let mut original = MemoryStore::new();
    let bio_notes = DocAnnotations {
        notes: vec![
            note(
                "bio-1",
                0,
                "Mitochondria make energy",
                vec![
                    rel("SUPPORTS", &bio_s, "bio-2", "same chapter"),
                    rel("CITES", &chem_s, "chem-1", ""),
                ],
            ),
            note("bio-2", 30, "Cells need energy", vec![]),
        ],
        highlights: vec![],
    };
    let chem_notes = DocAnnotations {
        notes: vec![note(
            "chem-1",
            0,
            "ATP stores energy",
            vec![rel("LIKES", &bio_s, "bio-1", "a newer type")],
        )],
        highlights: vec![],
    };
    original.save(&bio, &bio_notes).unwrap();
    original.save(&chem, &chem_notes).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("graph-vault");
    let docs = [
        ExportDocument {
            path: &bio,
            title: "Biology",
            text: None,
            annotations: &bio_notes,
        },
        ExportDocument {
            path: &chem,
            title: "Chemistry",
            text: None,
            annotations: &chem_notes,
        },
    ];
    let report = export_documents(&vault, &docs, &ExportOptions::default()).unwrap();
    assert_eq!(report.unresolved, 0);
    let mut restored = MemoryStore::new();
    let imported = import_vault(&vault, &ImportOptions::default(), &mut restored).unwrap();
    assert_eq!(imported.unresolved, 0);

    let titles = [(bio, "Biology"), (chem, "Chemistry")];
    let before = Graph::build(&noted(&original, &titles));
    let after = Graph::build(&noted(&restored, &titles));
    assert_eq!(before.edges.len(), 3);
    assert!(
        after
            .edges
            .iter()
            .any(|e| e.rel_type == "LIKES" && e.note == "a newer type"),
        "{:?}",
        after.edges
    );
    assert_eq!(after.edges, before.edges);
    assert_eq!(
        after.render(GraphFormat::Json),
        before.render(GraphFormat::Json)
    );
}
