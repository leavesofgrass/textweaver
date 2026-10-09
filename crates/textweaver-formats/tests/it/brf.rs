//! Braille (BRF) files: the hand-made fixtures in `fixtures/r5` (two
//! 40-by-25 pages of contracted braille, in UEB and in EBAE, with a
//! centered heading, a print page change, a cell-5 heading, a paragraph
//! over a page break with a divided word, and braille page numbers), read
//! through liblouis when it is installed; volumes in a zip; and hostile
//! input.

use std::io::Write;
use std::path::{Path, PathBuf};

use textweaver_core::MarkerKind;
use textweaver_formats::brf::{TRANSLATION_PROPERTY, UNTRANSLATED};
use textweaver_formats::{BrfCode, LoadOptions, Registry, Source};
use textweaver_text::Document;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/r5")
        .join(name)
}

fn load(path: &Path, code: BrfCode) -> Document {
    let options = LoadOptions {
        brf_code: code,
        ..LoadOptions::default()
    };
    Registry::with_builtins()
        .load(&Source::Path(path.to_owned()), &options)
        .unwrap()
}

fn liblouis() -> bool {
    std::process::Command::new("lou_translate")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn texts(doc: &Document, kind: MarkerKind) -> Vec<String> {
    doc.markers()
        .iter()
        .filter(|m| m.kind == kind)
        .map(|m| doc.slice(m.range))
        .collect()
}

#[test]
fn a_ueb_book_reads_as_print_with_its_structure() {
    let doc = load(&fixture("river-ueb.brf"), BrfCode::Ueb);
    assert_eq!(doc.meta.format, "brf");
    let pages: Vec<Option<String>> = doc
        .markers()
        .iter()
        .filter(|m| m.kind == MarkerKind::PageBreak)
        .map(|m| m.label.clone())
        .collect();
    assert_eq!(pages, [Some("1".to_owned()), Some("2".to_owned())]);
    assert_eq!(texts(&doc, MarkerKind::Heading).len(), 2);
    assert!(doc.text().to_string().contains("Print page 2"));
    if !liblouis() {
        eprintln!("liblouis not installed; checked the braille fallback only");
        assert_eq!(
            doc.meta
                .properties
                .get(TRANSLATION_PROPERTY)
                .map(String::as_str),
            Some(UNTRANSLATED)
        );
        return;
    }
    assert_eq!(
        doc.meta
            .properties
            .get(TRANSLATION_PROPERTY)
            .map(String::as_str),
        Some("ueb")
    );
    assert_eq!(
        texts(&doc, MarkerKind::Heading),
        ["Chapter One The River", "The Road Home"]
    );
    assert_eq!(
        texts(&doc, MarkerKind::Paragraph),
        [
            "The river ran past the old mill, and the children followed it down to the sea every summer.",
            "They counted the boats and the birds, and wrote the names in a little book.",
            "Print page 2",
            "In the evening the light went gold over the water.",
            "Nobody was in a hurry. The children sang on the way home and the sky was full of stars. The summer ended too soon.",
        ]
    );
}

#[test]
fn an_ebae_book_reads_with_the_ebae_code() {
    let doc = load(&fixture("river-ebae.brf"), BrfCode::Ebae);
    if !liblouis() {
        eprintln!("liblouis not installed; skipped");
        return;
    }
    assert_eq!(
        doc.meta
            .properties
            .get(TRANSLATION_PROPERTY)
            .map(String::as_str),
        Some("ebae")
    );
    let text = doc.text().to_string();
    assert!(
        text.contains("followed it down to the sea every summer."),
        "{text}"
    );
    assert!(text.contains("The summer ended too soon."), "{text}");
}

#[test]
fn volumes_in_a_zip_are_listed_and_open() {
    let dir = tempfile::tempdir().unwrap();
    let zip_path = dir.path().join("book.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
    for name in ["volume1.brf", "volume2.brf"] {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&std::fs::read(fixture("river-ueb.brf")).unwrap())
            .unwrap();
    }
    zip.finish().unwrap();
    let listing = load(&zip_path, BrfCode::Ueb);
    let links: Vec<String> = listing
        .markers()
        .iter()
        .filter(|m| m.kind == MarkerKind::Link)
        .filter_map(|m| m.reference.clone())
        .collect();
    assert_eq!(links, ["book.zip!volume1.brf", "book.zip!volume2.brf"]);
    let volume = load(&dir.path().join("book.zip!volume2.brf"), BrfCode::Ueb);
    assert_eq!(volume.meta.format, "brf");
    assert_eq!(texts(&volume, MarkerKind::Heading).len(), 2);
    let pages = textweaver_formats::brf::original_pages(
        &Source::Path(dir.path().join("book.zip!volume2.brf"))
            .read()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(pages.len(), 2);
}

#[test]
fn hostile_braille_files_load_without_panicking() {
    let cases: Vec<Vec<u8>> = vec![
        Vec::new(),
        vec![0x0C; 5000],
        "#".repeat(100_000).into_bytes(),
        b"-----#".repeat(2000),
        (0u8..=255).cycle().take(10_000).collect(),
        format!("{}{}", " ".repeat(500), "#AB").into_bytes(),
        "  ,A\r\n".repeat(5000).into_bytes(),
    ];
    for data in cases {
        let source = Source::Bytes {
            data,
            hint: "brf".into(),
        };
        let _ = Registry::with_builtins().load(&source, &LoadOptions::default());
    }
}

#[test]
fn a_brf_file_that_is_not_braille_reads_as_text() {
    let source = Source::Bytes {
        data: "Caf\u{e9} cr\u{e8}me br\u{fb}l\u{e9}e, d\u{e9}j\u{e0} vu".into(),
        hint: "brf".into(),
    };
    let doc = Registry::with_builtins()
        .load(&source, &LoadOptions::default())
        .unwrap();
    assert_eq!(doc.meta.format, "text");
    assert!(
        textweaver_formats::warnings(&doc.meta)
            .contains(&textweaver_formats::brf::NOT_BRAILLE_WARNING.to_owned())
    );
}
