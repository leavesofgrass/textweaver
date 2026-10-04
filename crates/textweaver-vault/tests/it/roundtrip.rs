//! Vault round trips on a temporary vault: export, edit in "Obsidian",
//! import, and export again.

use std::path::{Path, PathBuf};

use textweaver_core::{CharPos, CharRange};
use textweaver_vault::{
    AnnotationStore, DocAnnotations, ExportDocument, ExportOptions, Highlight, ImportOptions,
    MemoryStore, Note, Relation, RelationType, StateStoreAnnotations, export_documents,
    import_vault, read_vault,
};

const BIO: &str = "Cells are the unit of life. Mitochondria make energy for the cell. Ribosomes build proteins from amino acids.";
const CHEM: &str = "Atoms bond to form molecules. Proteins are large molecules.";

fn bio_path() -> PathBuf {
    PathBuf::from("/library/biology.md")
}

fn chem_path() -> PathBuf {
    PathBuf::from("/library/chemistry.md")
}

fn note(id: &str, pos: usize, anchor: &str, text: &str, tags: &[&str]) -> Note {
    Note {
        id: id.into(),
        range: CharRange::new(CharPos(pos), CharPos(pos)),
        anchor: anchor.into(),
        note: text.into(),
        tags: tags.iter().map(|t| (*t).to_owned()).collect(),
        created: 1_790_000_000,
        ts: 1_790_000_000,
        ..Note::default()
    }
}

fn rel(t: RelationType, doc: &Path, id: &str, note: &str) -> Relation {
    Relation {
        rel_type: t.as_str().into(),
        target_doc: doc.to_string_lossy().into_owned(),
        target_id: id.into(),
        note: note.into(),
    }
}

/// Two documents whose notes link to each other.
fn store() -> MemoryStore {
    let mut s = MemoryStore::new();
    let mut energy = note(
        "bio-1",
        28,
        "Mitochondria make energy",
        "Powerhouse of the cell.\nRemember for the exam.",
        &["exam", "cells"],
    );
    energy.cite = "Campbell, Biology, p. 112".into();
    let mut ribo = note(
        "bio-2",
        67,
        "Ribosomes build proteins",
        "Translation happens here.",
        &[],
    );
    ribo.relations.push(rel(
        RelationType::Supports,
        &chem_path(),
        "chem-1",
        "proteins are molecules",
    ));
    ribo.relations
        .push(rel(RelationType::SeeAlso, &bio_path(), "bio-1", ""));
    s.save(
        &bio_path(),
        &DocAnnotations {
            notes: vec![energy, ribo],
            highlights: vec![
                Highlight {
                    id: "hl-a".into(),
                    range: CharRange::new(CharPos(0), CharPos(27)),
                    color: "#ffff00".into(),
                    ts: 1_790_000_100,
                    ..Highlight::default()
                },
                Highlight {
                    id: "hl-b".into(),
                    range: CharRange::new(CharPos(67), CharPos(90)),
                    color: "#90ee90".into(),
                    ts: 1_790_000_200,
                    ..Highlight::default()
                },
            ],
        },
    )
    .unwrap();
    s.save(
        &chem_path(),
        &DocAnnotations {
            notes: vec![note(
                "chem-1",
                30,
                "Proteins are large molecules",
                "Polymers of amino acids.",
                &["chem"],
            )],
            highlights: vec![],
        },
    )
    .unwrap();
    s
}

fn export_all(vault: &Path, s: &MemoryStore) -> textweaver_vault::ExportReport {
    let bio = s.docs.get(&bio_path()).cloned().unwrap_or_default();
    let chem = s.docs.get(&chem_path()).cloned().unwrap_or_default();
    let bio_path = bio_path();
    let chem_path = chem_path();
    let docs = [
        ExportDocument {
            path: &bio_path,
            title: "Biology",
            text: Some(BIO),
            annotations: &bio,
        },
        ExportDocument {
            path: &chem_path,
            title: "Chemistry",
            text: Some(CHEM),
            annotations: &chem,
        },
    ];
    export_documents(vault, &docs, &ExportOptions::default()).unwrap()
}

/// Every file in the vault with its text, sorted.
fn snapshot(vault: &Path) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = textweaver_vault::note_files(vault)
        .unwrap()
        .into_iter()
        .map(|p| {
            let name = p
                .strip_prefix(vault)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            (name, std::fs::read_to_string(&p).unwrap())
        })
        .collect();
    out.sort();
    out
}

#[test]
fn export_then_import_into_an_empty_store_restores_everything() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("Study Vault");
    let original = store();
    let report = export_all(&vault, &original);
    assert_eq!(report.notes, 3);
    assert_eq!(report.documents, 2);
    assert_eq!(report.highlights, 2);
    assert_eq!(report.unresolved, 0);

    let mut restored = MemoryStore::new();
    let imported = import_vault(&vault, &ImportOptions::default(), &mut restored).unwrap();
    assert_eq!(imported.notes_added, 3);
    assert_eq!(imported.highlights_added, 2);
    assert_eq!(imported.unresolved, 0);
    // Exported notes are not registered as library documents or nodes.
    assert_eq!(imported.documents, 0);
    assert_eq!(imported.nodes, 0);
    assert!(restored.library.is_empty());

    for path in [bio_path(), chem_path()] {
        let mut want = original.docs[&path].clone();
        let mut got = restored.docs[&path].clone();
        for a in [&mut want, &mut got] {
            a.notes.sort_by(|x, y| x.id.cmp(&y.id));
            a.highlights.sort_by(|x, y| x.id.cmp(&y.id));
        }
        assert_eq!(got, want, "{}", path.display());
    }
}

#[test]
fn links_to_notes_left_out_of_the_vault_survive_an_import() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("Bio only");
    let mut s = store();
    // Export only Biology: bio-2's link to chem-1 cannot be written.
    let bio = s.docs.get(&bio_path()).cloned().unwrap();
    let bio_path = bio_path();
    let report = export_documents(
        &vault,
        &[ExportDocument {
            path: &bio_path,
            title: "Biology",
            text: Some(BIO),
            annotations: &bio,
        }],
        &ExportOptions::default(),
    )
    .unwrap();
    assert_eq!(report.unresolved, 1, "{report:?}");
    let before = s.docs[&bio_path].notes[1].relations.clone();
    assert_eq!(before.len(), 2);

    // Importing the vault keeps the link the vault could not hold, and
    // the one it did hold.
    import_vault(&vault, &ImportOptions::default(), &mut s).unwrap();
    let after = &s.docs[&bio_path].notes[1].relations;
    for r in &before {
        assert!(after.contains(r), "{r:?} lost: {after:?}");
    }
    assert_eq!(after.len(), 2, "{after:?}");

    // A link removed in Obsidian to a note that is in the vault is removed.
    let file = textweaver_vault::note_files(&vault)
        .unwrap()
        .into_iter()
        .find(|p| std::fs::read_to_string(p).unwrap().contains("SEE_ALSO::"))
        .unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    let edited: String = text
        .lines()
        .filter(|l| !l.contains("SEE_ALSO::"))
        .map(|l| format!("{l}\n"))
        .collect();
    std::fs::write(&file, edited).unwrap();
    import_vault(&vault, &ImportOptions::default(), &mut s).unwrap();
    let after = &s.docs[&bio_path].notes[1].relations;
    assert_eq!(after.len(), 1, "{after:?}");
    assert_eq!(after[0].target_id, "chem-1");
}

#[test]
fn edits_made_in_obsidian_come_back_and_re_export_is_stable() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().to_owned();
    let mut s = store();
    export_all(&vault, &s);
    let before = snapshot(&vault);

    // Importing an unchanged export changes nothing.
    let unchanged = import_vault(&vault, &ImportOptions::default(), &mut s).unwrap();
    assert_eq!(unchanged.notes_updated, 0);
    assert_eq!(unchanged.notes_added, 0);
    assert_eq!(unchanged.highlights_added, 0);
    assert_eq!(unchanged.highlights_updated, 0);
    assert_eq!(s.docs, store().docs);

    // The student edits a note in Obsidian: new text, a new inline tag, and
    // a new typed link to a note in the other document.
    let file = vault.join("Mitochondria make energy.md");
    let text = std::fs::read_to_string(&file).unwrap();
    let edited = text.replace(
        "Powerhouse of the cell.\nRemember for the exam.",
        "Powerhouse of the cell. #review\n\n- defines:: [[Proteins are large molecules]]",
    );
    assert_ne!(edited, text);
    std::fs::write(&file, edited).unwrap();

    let report = import_vault(&vault, &ImportOptions::default(), &mut s).unwrap();
    assert_eq!(report.notes_updated, 1);
    let bio = s.load(&bio_path()).unwrap();
    let energy = bio.note("bio-1").unwrap();
    assert_eq!(
        energy.note,
        // The typed link became a relation; its line leaves the text.
        "Powerhouse of the cell. #review"
    );
    assert_eq!(energy.tags, vec!["exam", "cells", "review"]);
    assert_eq!(energy.cite, "Campbell, Biology, p. 112");
    assert_eq!(energy.relations.len(), 1);
    assert_eq!(
        energy.relations[0].relation_type(),
        Some(RelationType::Defines)
    );
    assert_eq!(
        energy.relations[0].target_doc,
        chem_path().to_string_lossy()
    );
    assert_eq!(energy.relations[0].target_id, "chem-1");
    // The untouched notes keep their relations, comments included.
    let ribo = bio.note("bio-2").unwrap();
    assert_eq!(
        ribo.relations,
        store().docs[&bio_path()].note("bio-2").unwrap().relations
    );

    // Exporting again writes the edit back out, and a second export is
    // byte-for-byte the same as the first.
    export_all(&vault, &s);
    let after = snapshot(&vault);
    assert_eq!(after.len(), before.len());
    let energy_file = &after
        .iter()
        .find(|(n, _)| n == "Mitochondria make energy.md")
        .unwrap()
        .1;
    assert!(energy_file.contains("- DEFINES:: [[Proteins are large molecules]]\n"));
    assert!(energy_file.contains("tags: [exam, cells, review]\n"));
    export_all(&vault, &s);
    assert_eq!(snapshot(&vault), after);
    // And importing that export is again a no-op.
    let again = import_vault(&vault, &ImportOptions::default(), &mut s).unwrap();
    assert_eq!(again.notes_updated, 0);
}

#[test]
fn highlights_moved_in_the_document_note_are_updated() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().to_owned();
    let mut s = store();
    export_all(&vault, &s);
    let file = vault.join("Biology.md");
    let text = std::fs::read_to_string(&file).unwrap();
    std::fs::write(
        &file,
        text.replace(
            "id=hl-a start=0 end=27 color=#ffff00",
            "id=hl-a start=0 end=10 color=#ff0000",
        ),
    )
    .unwrap();
    let report = import_vault(&vault, &ImportOptions::default(), &mut s).unwrap();
    assert_eq!(report.highlights_updated, 1);
    let bio = s.load(&bio_path()).unwrap();
    let h = bio.highlights.iter().find(|h| h.id == "hl-a").unwrap();
    assert_eq!(h.range, CharRange::new(CharPos(0), CharPos(10)));
    assert_eq!(h.color, "#ff0000");
}

#[test]
fn state_file_bridge_round_trips_through_a_vault() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault");
    let state_dir = dir.path().join("state");
    let original = store();
    export_all(&vault, &original);

    let mut bridge = StateStoreAnnotations::new(textweaver_store::StateStore::new(state_dir));
    import_vault(&vault, &ImportOptions::default(), &mut bridge).unwrap();
    let mut got = bridge.load(&chem_path()).unwrap();
    got.notes.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(got, original.docs[&chem_path()]);
}

#[test]
fn star_exported_vault_imports_as_a_graph() {
    // A vault as star's `export_vault` wrote it: `star_id`, unquoted front
    // matter, `## Links` with Dataview fields.
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().to_owned();
    std::fs::write(
        vault.join("Cell theory.md"),
        "---\nstar_id: 1a2b3c4d\ntitle: Cell theory\nsource: /docs/bio.md\ntags: [exam]\n---\n\nAll living things are made of cells.\n\n## Links\n\n- SUPPORTS:: [[Microscopes]]\n",
    )
    .unwrap();
    std::fs::write(
        vault.join("Microscopes.md"),
        "---\nstar_id: 5e6f7a8b\ntitle: Microscopes\nsource: /docs/bio.md\ntags: []\n---\n\nHooke saw cells.\n",
    )
    .unwrap();
    let read = read_vault(&vault, &ImportOptions::default()).unwrap();
    assert_eq!(read.relations, 1);
    let mut s = MemoryStore::new();
    let report = textweaver_vault::apply(&read, &mut s).unwrap();
    assert_eq!(report.nodes, 2);
    let cell = s.load(&vault.join("Cell theory.md")).unwrap();
    assert_eq!(cell.notes[0].id, "1a2b3c4d");
    assert_eq!(cell.notes[0].note, "All living things are made of cells.");
    assert_eq!(cell.notes[0].relations[0].target_id, "5e6f7a8b");
    assert_eq!(cell.notes[0].relations[0].rel_type, "SUPPORTS");
}
