//! Tagged PDF writer: PDF/UA-1 validation by krilla while writing, the
//! structure tree, metadata, and outline in the file, and the text read back
//! with `pdftotext` when it is installed. Skipped when the system has none
//! of the fonts the writer looks for.

use crate::common;

use common::{options, sample, words};
use ropey::Rope;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, DocumentMeta, Marker};
use textweaver_writers::{Format, PdfOptions, WriteError, WriteOptions, WriteReport, write_to_vec};

/// Uncompressed content, so the test can read the structure.
fn plain() -> WriteOptions {
    WriteOptions {
        pdf: PdfOptions {
            compress: false,
            ..PdfOptions::default()
        },
        ..options()
    }
}

/// The PDF, or `None` when no font is installed.
fn pdf(doc: &Document, options: &WriteOptions) -> Option<(Vec<u8>, WriteReport)> {
    match write_to_vec(doc, Format::Pdf, options) {
        Ok(v) => Some(v),
        Err(WriteError::NoFont) => {
            eprintln!("no font for PDF output on this system; skipping");
            None
        }
        Err(e) => panic!("{e}"),
    }
}

/// Occurrences of `needle`, written with PDF's optional spaces before
/// `/`, `(`, and `[` (krilla writes them without).
fn count(hay: &[u8], needle: &str) -> usize {
    let needle = needle
        .replace(" /", "/")
        .replace(" (", "(")
        .replace(" [", "[");
    hay.windows(needle.len())
        .filter(|w| *w == needle.as_bytes())
        .count()
}

fn has(hay: &[u8], needle: &str) -> bool {
    count(hay, needle) > 0
}

#[test]
fn sample_is_tagged_and_passes_pdf_ua_validation() {
    let Some((bytes, report)) = pdf(&sample(), &plain()) else {
        return;
    };
    assert!(bytes.starts_with(b"%PDF-1.7"));
    assert!(report.warnings.is_empty(), "{report:?}");
    for needle in [
        "/StructTreeRoot",
        "/MarkInfo",
        "/Marked true",
        "/Lang (en-US)",
        "/DisplayDocTitle true",
        "pdfuaid:part",
        "/Outlines",
        "/FontFile2",
        "/S /Document",
        "/S /H1",
        "/S /H2",
        "/S /H3",
        "/T (Reading Guide)",
        "/S /P",
        "/S /L",
        "/ListNumbering /Decimal",
        "/ListNumbering /Disc",
        "/S /LI",
        "/S /Lbl",
        "/S /LBody",
        "/S /Table",
        "/S /TR",
        "/S /TH",
        "/Scope /Column",
        "/S /TD",
        "/S /Figure",
        "/Alt (Two coloured squares)",
        "/S /BlockQuote",
        "/S /Code",
        "/S /Note",
        "/S /Link",
        "/Annots",
        "/URI (https://example.org/textweaver)",
        "/Artifact",
    ] {
        assert!(has(&bytes, needle), "missing {needle}");
    }
    // Five headings, five outline entries.
    assert_eq!(count(&bytes, "/S /H"), 5);
    assert!(has(&bytes, "/Title (Reading Guide)"));
    assert!(has(&bytes, "/Author (Ada Example)"));
    // The one image is embedded.
    assert!(has(&bytes, "/Subtype /Image"));
}

#[test]
fn text_reads_back_in_order_with_pdftotext() {
    if !common::runs("pdftotext", "-v") {
        common::skip_or_fail("pdftotext", "pdftotext is not installed");
        return;
    }
    let doc = sample();
    let Some((bytes, _)) = pdf(&doc, &options()) else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sample.pdf");
    std::fs::write(&path, bytes).unwrap();
    // `-raw` keeps content stream order, which is reading order here.
    let out = std::process::Command::new("pdftotext")
        .args(["-raw", "-enc", "UTF-8"])
        .arg(&path)
        .arg("-")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    // Numbers aside (list labels and page numbers are drawn, not source
    // text), the words match; the footer "Page N of M" is an artifact.
    let digits = |w: &String| w.chars().all(|c| c.is_ascii_digit());
    let mut got: Vec<String> = words(&text).into_iter().filter(|w| !digits(w)).collect();
    let mut i = 0;
    while i + 1 < got.len() {
        if got[i] == "page" && got[i + 1] == "of" {
            got.drain(i..i + 2);
        } else {
            i += 1;
        }
    }
    // Image descriptions are alt text in the PDF, not drawn text.
    let expected: Vec<String> = words(&common::text_without(&doc, |m| m.kind == MarkerKind::Image))
        .into_iter()
        .filter(|w| !digits(w))
        .collect();
    assert_eq!(got, expected);
}

#[test]
fn long_tables_repeat_their_header_as_an_artifact() {
    let mut text = String::from("Name | Value\n");
    let mut markers =
        vec![Marker::new(MarkerKind::TableRow, CharRange::new(0, 12)).with_label("header")];
    for n in 0..120 {
        let start = text.chars().count();
        let row = format!("Row {n} | {}", n * 7);
        text.push_str(&row);
        markers.push(Marker::new(
            MarkerKind::TableRow,
            CharRange::new(start, start + row.chars().count()),
        ));
        text.push('\n');
    }
    let len = text.chars().count() - 1;
    markers.push(Marker::new(MarkerKind::Table, CharRange::new(0, len)).with_label("Values"));
    let doc = Document::new(
        DocumentMeta {
            title: Some("Long table".into()),
            ..DocumentMeta::default()
        },
        Rope::from_str(&text),
        markers,
    );
    let Some((bytes, _)) = pdf(&doc, &plain()) else {
        return;
    };
    // Every row is tagged once; the repeated headers are not.
    assert_eq!(count(&bytes, "/S /TR"), 121);
    assert_eq!(count(&bytes, "/S /TH"), 2);
    assert!(has(&bytes, "/S /Caption"));
    let pages = count(&bytes, "/Type /Page") - count(&bytes, "/Type /Pages");
    assert!(pages >= 3, "{pages} pages");
    // The header is drawn again on every page (as an artifact).
    if common::runs("pdftotext", "-v") {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("table.pdf");
        std::fs::write(&path, &bytes).unwrap();
        let out = std::process::Command::new("pdftotext")
            .args(["-raw", "-enc", "UTF-8"])
            .arg(&path)
            .arg("-")
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert_eq!(text.matches("Name Value").count(), pages, "{text}");
    }
}

#[test]
fn edge_cases_still_validate() {
    let cases = [
        Document::from_plain_text(""),
        Document::from_plain_text("Plain text\nwith lines.\n\nAnd a second paragraph."),
        Document::from_plain_text(&"Supercalifragilisticexpialidocious".repeat(12)),
        Document::from_plain_text(&"Many words make many pages. ".repeat(2000)),
        Document::from_plain_text("Tabs\tand ümlauts, Ελληνικά, and a private \u{E000} char."),
    ];
    for doc in cases {
        let Some((bytes, report)) = pdf(&doc, &plain()) else {
            return;
        };
        assert!(has(&bytes, "/StructTreeRoot"));
        assert!(has(&bytes, "/Title (Untitled document)"));
        if doc.text().to_string().contains('\u{E000}') {
            assert!(
                report.warnings.iter().any(|w| w.contains("U+E000")),
                "{report:?}"
            );
        }
    }
}

#[test]
fn page_size_and_validation_switch() {
    let doc = Document::from_plain_text("Hello.");
    let options = WriteOptions {
        pdf: PdfOptions {
            page_size: textweaver_writers::PageSize::A4,
            pdf_ua: false,
            compress: false,
            ..PdfOptions::default()
        },
        ..options()
    };
    let Some((bytes, _)) = pdf(&doc, &options) else {
        return;
    };
    assert!(has(&bytes, "/MediaBox [0 0 595.28 841.89]"));
    assert!(!has(&bytes, "pdfuaid:part"));
    assert!(has(&bytes, "/StructTreeRoot"));
}

#[test]
fn a_missing_font_file_is_an_error() {
    let options = WriteOptions {
        pdf: PdfOptions {
            font: Some("definitely/not/a/font.ttf".into()),
            ..PdfOptions::default()
        },
        ..options()
    };
    let err = write_to_vec(&Document::from_plain_text("x"), Format::Pdf, &options).unwrap_err();
    assert!(matches!(err, WriteError::Font(..)), "{err}");
}
