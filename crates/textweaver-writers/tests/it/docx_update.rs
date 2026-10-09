//! Writing a review back into a Word-made document in place (task B1-t2),
//! on `fixtures/t1/word-review.docx`: insertions, deletions, a named move,
//! a deleted paragraph, formatting changes, an inserted row, a change in
//! the header, and threaded comments.

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use textweaver_formats::{changes, comments, revision_count};
use textweaver_writers::docx_update::{DocxUpdate, update_docx};

fn fixture() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/t1/word-review.docx");
    std::fs::read(path).unwrap()
}

/// Every part of a package, uncompressed, by name.
fn parts(pkg: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut z = zip::ZipArchive::new(Cursor::new(pkg)).unwrap();
    (0..z.len())
        .map(|i| {
            let mut f = z.by_index(i).unwrap();
            let mut b = Vec::new();
            f.read_to_end(&mut b).unwrap();
            (f.name().to_owned(), b)
        })
        .collect()
}

fn load(pkg: &[u8]) -> textweaver_text::Document {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("review.docx");
    std::fs::write(&path, pkg).unwrap();
    textweaver_formats::load_path(&path).unwrap()
}

fn decide_all(accept: bool) -> Vec<u8> {
    let update = DocxUpdate {
        rest: Some(accept),
        ..DocxUpdate::default()
    };
    update_docx(&fixture(), &update).unwrap().0
}

/// The parts a decision may change; every other part must come through
/// byte for byte.
const REVISED: [&str; 2] = ["word/document.xml", "word/header1.xml"];

fn others_unchanged(before: &BTreeMap<String, Vec<u8>>, after: &BTreeMap<String, Vec<u8>>) {
    assert_eq!(
        before.keys().collect::<Vec<_>>(),
        after.keys().collect::<Vec<_>>(),
        "the same parts"
    );
    for (name, bytes) in before {
        if !REVISED.contains(&name.as_str()) {
            assert!(after[name] == *bytes, "{name} changed");
        }
    }
}

fn no_revision_markup(parts: &BTreeMap<String, Vec<u8>>) {
    for name in REVISED {
        let xml = String::from_utf8(parts[name].clone()).unwrap();
        for mark in [
            "<w:ins ",
            "<w:del ",
            "<w:moveFrom",
            "<w:moveTo",
            "PrChange",
            "delText",
        ] {
            assert!(!xml.contains(mark), "{mark} left in {name}");
        }
    }
}

#[test]
fn accept_all_leaves_no_revisions_and_every_other_part_as_it_was() {
    let before = parts(&fixture());
    let out = decide_all(true);
    let after = parts(&out);
    others_unchanged(&before, &after);
    no_revision_markup(&after);
    let doc = load(&out);
    assert!(changes(&doc.meta).is_empty());
    assert_eq!(revision_count(&doc.meta), 0);
    assert_eq!(comments(&doc.meta).len(), 2, "the comments stay");
    let text = doc.text().to_string();
    assert!(text.contains("go home tomorrow after review."), "{text}");
    assert!(!text.contains("drain"), "{text}");
    assert!(text.contains("Then check the labs first."), "{text}");
    assert!(!text.contains("Check the labs first. Give"), "{text}");
    assert!(text.contains("Day two"), "{text}");
    let header = String::from_utf8(after["word/header1.xml"].clone()).unwrap();
    assert!(header.contains(">draft<"), "{header}");
}

#[test]
fn reject_all_gives_back_the_original_text() {
    let before = parts(&fixture());
    let out = decide_all(false);
    let after = parts(&out);
    others_unchanged(&before, &after);
    no_revision_markup(&after);
    let doc = load(&out);
    assert!(changes(&doc.meta).is_empty());
    let text = doc.text().to_string();
    assert!(text.contains("may go home after two review."), "{text}");
    assert!(text.contains("Keep the drain in place."), "{text}");
    assert!(
        text.contains("Check the labs first. Give fluids."),
        "{text}"
    );
    assert!(!text.contains("Day two"), "{text}");
    let document = String::from_utf8(after["word/document.xml"].clone()).unwrap();
    assert!(
        document.contains(r#"<w:jc w:val="left"/>"#),
        "the old alignment"
    );
    assert!(
        document.contains("<w:rPr><w:i/></w:rPr>"),
        "the old run format"
    );
    let header = String::from_utf8(after["word/header1.xml"].clone()).unwrap();
    assert!(!header.contains("draft"), "{header}");
}

#[test]
fn deciding_nothing_writes_the_package_unchanged() {
    let before = parts(&fixture());
    let (out, _) = update_docx(&fixture(), &DocxUpdate::default()).unwrap();
    assert_eq!(parts(&out), before);
}
