//! Archives other than zip (feature `archives`): tar, tar.gz, and 7z
//! (`fixtures/w3d/course.7z`, made with 7-Zip, solid, LZMA2), listed and
//! opened by `archive!member` paths.

#![cfg(feature = "archives")]

use std::path::{Path, PathBuf};

use textweaver_core::MarkerKind;
use textweaver_formats::{LoadOptions, Registry, Source};
use textweaver_text::Document;

fn load(path: &Path) -> Document {
    Registry::with_builtins()
        .load(&Source::Path(path.to_owned()), &LoadOptions::default())
        .unwrap()
}

fn links(doc: &Document) -> Vec<String> {
    doc.marker_index()
        .iter(MarkerKind::Link, None)
        .filter_map(|m| m.reference.clone())
        .collect()
}

fn tar_bytes() -> Vec<u8> {
    let mut b = tar::Builder::new(Vec::new());
    for (name, body) in [
        ("./course/notes.md", &b"# Tar notes\n\nHello from tar."[..]),
        ("course/marks.csv", b"Name,Mark\nBo,7\n"),
    ] {
        let mut h = tar::Header::new_gnu();
        h.set_size(body.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        b.append_data(&mut h, name, body).unwrap();
    }
    b.into_inner().unwrap()
}

#[test]
fn seven_z_lists_and_opens_members() {
    let archive = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/w3d/course.7z");
    let doc = load(&archive);
    let mut l = links(&doc);
    l.sort();
    assert_eq!(l, ["course.7z!marks.csv", "course.7z!week1/notes.md"]);
    let notes = load(&archive.with_file_name("course.7z!week1/notes.md"));
    assert_eq!(notes.text().to_string(), "Week one\n\nRead chapter two.");
    let marks = load(&archive.with_file_name("course.7z!marks.csv"));
    assert_eq!(marks.text().to_string(), "Name | Mark\nAnn | 9");
}

#[test]
fn tar_and_tar_gz_list_and_open_members() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let tar = dir.path().join("course.tar");
    std::fs::write(&tar, tar_bytes()).unwrap();
    let tgz: PathBuf = dir.path().join("course.tgz");
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(&tar_bytes()).unwrap();
    std::fs::write(&tgz, gz.finish().unwrap()).unwrap();
    for (archive, base) in [(&tar, "course.tar"), (&tgz, "course.tgz")] {
        let doc = load(archive);
        assert_eq!(
            links(&doc),
            [
                format!("{base}!course/notes.md"),
                format!("{base}!course/marks.csv")
            ]
        );
        let notes = load(&dir.path().join(format!("{base}!course/notes.md")));
        assert_eq!(notes.text().to_string(), "Tar notes\n\nHello from tar.");
    }
}
