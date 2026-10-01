//! Document identity with two or three test homes as computers and one
//! temporary folder as the sync folder (ADR-0049, "Recognizing the same
//! document"). Nothing touches a real sync service.

use std::path::{Path, PathBuf};

use textweaver_core::{CharPos, CharRange};
use textweaver_store::sync_ids::{SYNC_IDS_FILE, SyncIds};
use textweaver_store::{DocKey, Note, Paths};
use textweaver_sync::docid::{
    self, Details, Found, Identify, IdentityIndex, LIBRARY_ID_FILE, SharedIdentifier,
};
use textweaver_sync::{Clock, DocRecord, Identity, SyncFolder, local_names};

const VERSION: &str = "0.1.0-alpha.6";

/// One computer: a home (state and documents) and the shared sync folder.
struct Home {
    dir: tempfile::TempDir,
    paths: Paths,
    folder: SyncFolder,
    clock: Clock,
}

impl Home {
    fn new(label: &str, sync: &Path) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::under(dir.path());
        let (mut identity, _) = Identity::load_or_create(&paths.data_dir).unwrap();
        let opened = SyncFolder::open(sync, &mut identity, label, VERSION).unwrap();
        let clock = Clock::new(identity.device());
        Self {
            dir,
            paths,
            folder: opened.folder,
            clock,
        }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    /// Writes a document under this home.
    fn put(&self, rel: &str, body: &[u8]) -> PathBuf {
        let p = self.root().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, body).unwrap();
        p
    }

    fn job(&self, path: &Path, folders: &[PathBuf], details: Details) -> Identify {
        Identify {
            ids_file: self.paths.sync_ids_file(),
            path: path.to_owned(),
            library_folders: folders.to_vec(),
            details,
        }
    }

    fn index(&mut self) -> IdentityIndex {
        let (index, problems) = self.folder.identity_index();
        assert!(problems.is_empty(), "{problems:?}");
        index
    }

    /// Identifies `path` (its text is its bytes as UTF-8) with what the
    /// sync folder knows, and publishes the result there, as the reader
    /// would.
    fn open(&mut self, path: &Path, folders: &[PathBuf], details: Details) -> docid::Resolved {
        let index = self.index();
        let text = String::from_utf8_lossy(&std::fs::read(path).unwrap()).into_owned();
        let job = self.job(path, folders, details.clone());
        let r = job.run(Some([text.as_str()]), Some(&index)).unwrap();
        self.publish(&r, &details, 0);
        r
    }

    /// Merges the folder's record of `r`'s document, adds `notes` notes
    /// and the hashes when they changed, and writes this computer's record.
    fn publish(&mut self, r: &docid::Resolved, details: &Details, notes: usize) {
        let mut record = DocRecord::new(r.sync_id);
        self.folder.merge_doc(&mut record, &mut self.clock);
        if r.changed || record.identity.is_empty() {
            let stamp = self.clock.tick();
            record.publish_identity(stamp, &r.fingerprint, details);
        }
        for n in 0..notes {
            let stamp = self.clock.tick();
            let id = format!("n{n}");
            record.notes.set(id.clone(), stamp, note(&id));
        }
        self.folder.write_doc(&record).unwrap();
    }
}

fn note(id: &str) -> Note {
    Note {
        id: id.to_owned(),
        range: CharRange::new(CharPos(0), CharPos(5)),
        anchor: "Cells".to_owned(),
        note: "Mitosis".to_owned(),
        tags: Vec::new(),
        cite: String::new(),
        color: None,
        relations: Vec::new(),
        created: 1,
        ts: 1,
        extra: serde_json::Map::new(),
    }
}

fn body(words: usize, seed: &str) -> String {
    (0..words)
        .map(|n| format!("{seed} word {n}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every file under `dir`, with its text.
fn files(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_owned()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                let text = String::from_utf8_lossy(&std::fs::read(&p).unwrap()).into_owned();
                out.push((p, text));
            }
        }
    }
    out
}

/// The privacy scan: no home path, file name, or computer or user name
/// anywhere in the sync folder, in any file's name or text.
fn assert_private(sync: &Path, homes: &[&Home], names: &[&str]) {
    let mut forbidden: Vec<String> = local_names()
        .into_iter()
        .filter(|n| n.chars().count() >= 3)
        .collect();
    for h in homes {
        let full = h.root().to_string_lossy().into_owned();
        forbidden.push(full.clone());
        forbidden.push(full.replace('\\', "/"));
        forbidden.push(full.replace('\\', "\\\\"));
        if let Some(last) = h.root().file_name() {
            forbidden.push(last.to_string_lossy().into_owned());
        }
    }
    forbidden.extend(names.iter().map(|s| (*s).to_owned()));
    let all = files(sync);
    assert!(!all.is_empty());
    for (path, text) in all {
        let name = path
            .strip_prefix(sync)
            .unwrap()
            .to_string_lossy()
            .to_lowercase();
        let text = text.to_lowercase();
        for f in &forbidden {
            let f = f.to_lowercase();
            assert!(
                !name.contains(&f) && !text.contains(&f),
                "{f:?} found in {}",
                path.display()
            );
        }
        assert!(
            !text.contains(":\\") && !text.contains(":/"),
            "a path in {}",
            path.display()
        );
        assert!(!text.contains(LIBRARY_ID_FILE));
    }
}

#[test]
fn the_same_file_at_different_paths_in_two_homes_has_one_id() {
    let sync = tempfile::tempdir().unwrap();
    let mut laptop = Home::new("laptop", sync.path());
    let mut lab = Home::new("lab", sync.path());
    let bytes = body(500, "cells").into_bytes();

    let a = laptop.put("Papers/biology/unit-three.md", &bytes);
    let first = laptop.open(&a, &[], Details::default());
    assert_eq!(first.found, Found::New);
    assert!(first.changed);

    let b = lab.put("courses/copy-of-reading.md", &bytes);
    let second = lab.open(&b, &[], Details::default());
    assert_eq!(second.found, Found::Content);
    assert_eq!(second.sync_id, first.sync_id);
    assert_ne!(second.key, first.key, "the path keys differ");

    // Opened again, it is known, unchanged, and not published again.
    let again = lab.open(&b, &[], Details::default());
    assert_eq!(again.found, Found::Known);
    assert_eq!(again.sync_id, first.sync_id);
    assert!(!again.changed);

    assert_private(
        sync.path(),
        &[&laptop, &lab],
        &["unit-three", "copy-of-reading", "biology", "papers"],
    );
}

#[test]
fn a_renamed_file_is_found_by_content() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::under(home.path());
    let old = home.path().join("before.txt");
    std::fs::write(&old, body(300, "rename")).unwrap();
    let job = |p: &Path| Identify {
        ids_file: paths.sync_ids_file(),
        path: p.to_owned(),
        library_folders: Vec::new(),
        details: Details::default(),
    };
    let first = job(&old).run(None::<[&str; 0]>, None).unwrap();
    assert_eq!(first.found, Found::New);

    let new = home.path().join("after.txt");
    std::fs::rename(&old, &new).unwrap();
    let second = job(&new).run(None::<[&str; 0]>, None).unwrap();
    assert_eq!(second.found, Found::Content);
    assert_eq!(second.sync_id, first.sync_id);

    // Both path keys map to the one id; no state file was renamed.
    let ids = SyncIds::load(&paths.sync_ids_file());
    assert_eq!(ids.keys_of(&first.sync_id.to_string()).count(), 2);
    assert!(paths.data_dir.join(SYNC_IDS_FILE).is_file());
    assert!(!paths.state_dir().exists(), "nothing written under state/");
}

#[test]
fn the_same_text_saved_differently_is_found_by_its_text() {
    let sync = tempfile::tempdir().unwrap();
    let mut laptop = Home::new("laptop", sync.path());
    let mut lab = Home::new("lab", sync.path());
    let text = body(200, "membrane");
    let a = laptop.put("a.txt", text.as_bytes());
    let first = laptop.open(&a, &[], Details::default());

    // The same words with Windows line endings and wrapped differently.
    let crlf = text
        .replace(" word 1", "\r\nword 1")
        .replace(" word 5", "\r\n\r\n  word 5");
    assert_ne!(crlf, text);
    let b = lab.put("b.txt", crlf.as_bytes());
    let second = lab.open(&b, &[], Details::default());
    assert_eq!(second.found, Found::Text);
    assert_eq!(second.sync_id, first.sync_id);
}

#[test]
fn a_library_folder_is_one_folder_on_every_computer() {
    let sync = tempfile::tempdir().unwrap();
    let mut laptop = Home::new("laptop", sync.path());
    let mut lab = Home::new("lab", sync.path());

    let lib_a = laptop.root().join("shelfalpha");
    let a = laptop.put("shelfalpha/unit/reading.md", b"first draft of the reading");
    let first = laptop.open(&a, std::slice::from_ref(&lib_a), Details::default());
    let id_file = docid::library_id_file(&lib_a);
    assert!(id_file.is_file(), "made on first use");

    // The folder reaches the lab (its id file with it), where the reading
    // was since changed: different bytes, different text, same place.
    let lib_b = lab.root().join("shared").join("shelfbeta");
    let b = lab.put(
        "shared/shelfbeta/unit/reading.md",
        b"a quite different second draft",
    );
    std::fs::create_dir_all(docid::library_id_file(&lib_b).parent().unwrap()).unwrap();
    std::fs::copy(&id_file, docid::library_id_file(&lib_b)).unwrap();
    assert_eq!(
        docid::library_id(&lib_a).unwrap(),
        docid::library_id(&lib_b).unwrap()
    );
    let second = lab.open(&b, std::slice::from_ref(&lib_b), Details::default());
    assert_eq!(second.found, Found::Library);
    assert_eq!(second.sync_id, first.sync_id);

    // Another path in the same folder is another document.
    let c = lab.put("shared/shelfbeta/unit/other.md", b"something else entirely");
    let third = lab.open(&c, std::slice::from_ref(&lib_b), Details::default());
    assert_eq!(third.found, Found::New);

    assert_private(
        sync.path(),
        &[&laptop, &lab],
        &["reading.md", "shelfalpha", "shelfbeta", "unit/"],
    );
}

#[test]
fn an_edited_file_keeps_its_id_and_publishes_its_new_hash() {
    let sync = tempfile::tempdir().unwrap();
    let mut laptop = Home::new("laptop", sync.path());
    let mut lab = Home::new("lab", sync.path());
    let original = body(100, "draft");
    let a = laptop.put("essay.md", original.as_bytes());
    let first = laptop.open(&a, &[], Details::default());

    // Edited and saved: longer, so size and time both change.
    let edited = format!("{original} and one more sentence.");
    std::fs::write(&a, &edited).unwrap();
    let second = laptop.open(&a, &[], Details::default());
    assert_eq!(second.found, Found::Known);
    assert_eq!(second.sync_id, first.sync_id);
    assert!(second.changed, "new hashes to publish");
    assert_ne!(
        second.fingerprint.content_sha256,
        first.fingerprint.content_sha256
    );
    let ids = SyncIds::load(&laptop.paths.sync_ids_file());
    let entry = ids.get(&DocKey::for_path(&a)).unwrap();
    assert_eq!(entry.content_sha256, second.fingerprint.content_sha256);

    // The lab recognizes the edited copy by its new hash, and an older
    // copy by the hash before it.
    let b = lab.put("new.md", edited.as_bytes());
    let by_new = lab.open(&b, &[], Details::default());
    assert_eq!(
        (by_new.found, by_new.sync_id),
        (Found::Content, first.sync_id)
    );
    let c = lab.put("old.md", original.as_bytes());
    let by_old = lab.open(&c, &[], Details::default());
    assert_eq!(by_old.sync_id, first.sync_id);
}

#[test]
fn two_chapters_with_one_isbn_are_not_matched_without_asking() {
    let sync = tempfile::tempdir().unwrap();
    let mut laptop = Home::new("laptop", sync.path());
    let mut lab = Home::new("lab", sync.path());
    let isbn = Some("9780306406157".to_owned());
    let one = Details {
        title: Some("Cells".into()),
        doi: None,
        isbn: isbn.clone(),
        ..Details::default()
    };
    let a = laptop.put("ch1.md", body(100, "chapter one").as_bytes());
    let first = laptop.open(&a, &[], one.clone());
    laptop.publish(&first, &one, 4);

    let two = Details {
        title: Some("Tissues".into()),
        doi: None,
        isbn,
        ..Details::default()
    };
    let b = lab.put("ch2.md", body(100, "chapter two").as_bytes());
    let second = lab.open(&b, &[], two.clone());
    assert_eq!(second.found, Found::New);
    assert_ne!(
        second.sync_id, first.sync_id,
        "never matched on the ISBN alone"
    );
    assert_eq!(second.suggestions.len(), 1);
    let s = &second.suggestions[0];
    assert_eq!(s.sync_id, first.sync_id);
    assert_eq!(s.title.as_deref(), Some("Cells"));
    assert_eq!(s.notes, 4);
    assert_eq!(s.shared, SharedIdentifier::Isbn);
    assert_eq!(s.devices, vec![laptop.folder.device()]);

    // "No": not asked again.
    let ids_file = lab.paths.sync_ids_file();
    docid::decline(&ids_file, &second.key, first.sync_id).unwrap();
    let again = lab.open(&b, &[], two.clone());
    assert_eq!(again.sync_id, second.sync_id);
    assert!(again.suggestions.is_empty());

    // A third chapter, answered "yes": it takes the laptop's id.
    let c = lab.put("ch3.md", body(100, "chapter three").as_bytes());
    let third = lab.open(&c, &[], two.clone());
    assert!(third.suggestions.iter().any(|s| s.sync_id == first.sync_id));
    docid::accept(&ids_file, &third.key, first.sync_id).unwrap();
    let accepted = lab.open(&c, &[], two);
    assert_eq!(
        (accepted.found, accepted.sync_id),
        (Found::Known, first.sync_id)
    );
    assert!(accepted.suggestions.is_empty(), "the laptop has this id");
}

#[test]
fn a_doi_is_suggested_too() {
    let sync = tempfile::tempdir().unwrap();
    let mut laptop = Home::new("laptop", sync.path());
    let mut lab = Home::new("lab", sync.path());
    let d = Details {
        title: Some("Signal pathways".into()),
        doi: Some("10.1000/xyz123".into()),
        isbn: None,
        ..Details::default()
    };
    let a = laptop.put("paper.md", b"the preprint");
    let first = laptop.open(&a, &[], d.clone());
    let b = lab.put("paper-final.md", b"the published version");
    let second = lab.open(&b, &[], d);
    assert_ne!(second.sync_id, first.sync_id);
    assert_eq!(second.suggestions[0].shared, SharedIdentifier::Doi);
}

#[test]
fn a_damaged_library_id_file_is_left_alone() {
    let lib = tempfile::tempdir().unwrap();
    let file = docid::library_id_file(lib.path());
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "{\"format\":1,\"libr").unwrap();
    assert!(docid::library_id(lib.path()).is_err());
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "{\"format\":1,\"libr"
    );
    // The document is still identified, without a library key.
    let home = tempfile::tempdir().unwrap();
    let doc = lib.path().join("a.md");
    std::fs::write(&doc, "text").unwrap();
    let r = Identify {
        ids_file: Paths::under(home.path()).sync_ids_file(),
        path: doc,
        library_folders: vec![lib.path().to_owned()],
        details: Details::default(),
    }
    .run(None::<[&str; 0]>, None)
    .unwrap();
    assert!(r.fingerprint.library_key.is_none());
}

#[test]
fn a_missing_file_is_an_error_not_a_new_id() {
    let home = tempfile::tempdir().unwrap();
    let r = Identify {
        ids_file: Paths::under(home.path()).sync_ids_file(),
        path: home.path().join("gone.md"),
        library_folders: Vec::new(),
        details: Details::default(),
    }
    .run(None::<[&str; 0]>, None);
    assert!(r.is_err());
    assert!(!Paths::under(home.path()).sync_ids_file().exists());
}

#[test]
fn hashes_ignore_white_space_but_not_words() {
    let a = docid::text_sha256(["Cells divide.\r\n\r\n", "  Mitosis   follows."]);
    let b = docid::text_sha256(["Cells divide. Mitosis follows.\n"]);
    let c = docid::text_sha256(["Cells divide. Meiosis follows."]);
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(a.len(), 64);
    // The same text in other pieces.
    let d = docid::text_sha256(["Cel", "ls divide. Mi", "tosis follows."]);
    assert_eq!(a, d);
}
