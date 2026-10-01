//! PDF links, comments, form fields, captions, and turned and tabular
//! scans (Agent W6c5, ADR-0048), on the fixtures in `fixtures/c5` (made by
//! `make_fixtures.py`).
//!
//! The scan tests need the ocrs models in `.cache/models` at the workspace
//! root, like `tests/ocr.rs`, and are skipped, with a note, without them.

#![cfg(feature = "pdf")]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use lopdf::{Object, Stream, dictionary};
use textweaver_core::{CharPos, MarkerKind};
use textweaver_formats::{LoadOptions, Registry, Source, comments};
use textweaver_text::Document;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(name: &str) -> PathBuf {
    root().join("fixtures/c5").join(name)
}

fn load(path: &Path) -> Document {
    Registry::with_builtins()
        .load(&Source::Path(path.to_owned()), &LoadOptions::default())
        .unwrap()
}

fn load_bytes(bytes: Vec<u8>) -> Document {
    Registry::with_builtins()
        .load(
            &Source::Bytes {
                data: bytes,
                hint: "pdf".into(),
            },
            &LoadOptions::default(),
        )
        .unwrap()
}

fn slice(doc: &Document, start: usize, end: usize) -> String {
    doc.text().chars().skip(start).take(end - start).collect()
}

#[test]
fn comments_become_notes_on_the_text_they_mark() {
    let doc = load(&fixture("comments.pdf"));
    let cs = comments(&doc.meta);
    let said: Vec<String> = cs.iter().map(|c| c.spoken()).collect();
    assert_eq!(
        said,
        [
            "Comment by Ada Example: Good opening sentence.",
            "Comment by Ada Example: Cite a source for this. Reply by Bo Example: Added a citation. Resolved.",
            "Comment by Ada Example: Struck out",
        ],
        "{cs:#?}"
    );
    // The sticky note sits at the start of the line beside it.
    let note = &cs[0];
    assert!(note.range.is_empty());
    assert_eq!(
        slice(&doc, note.range.start.0, note.range.start.0 + 9),
        "The river"
    );
    // The highlight covers the words it marks.
    let hl = &cs[1];
    assert_eq!(
        slice(&doc, hl.range.start.0, hl.range.end.0),
        "falls as rain in the spring"
    );
    assert_eq!(hl.date, "2026-09-01T10:30:00Z");
    assert!(hl.resolved);
    assert_eq!(
        slice(&doc, cs[2].range.start.0, cs[2].range.end.0),
        "This sentence was a draft"
    );
    // The hidden note is not read.
    assert!(!said.iter().any(|s| s.contains("Hidden")));
}

#[test]
fn links_go_to_addresses_headings_and_pages() {
    let doc = load(&fixture("comments.pdf"));
    let links: Vec<(String, String)> = doc
        .marker_index()
        .iter(MarkerKind::Link, None)
        .map(|m| {
            (
                slice(&doc, m.range.start.0, m.range.end.0),
                m.reference.clone().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        links,
        [
            (
                "the course page".to_owned(),
                "https://example.org/course".to_owned()
            ),
            ("see Methods".to_owned(), "#methods".to_owned()),
            ("see page 3".to_owned(), "#page=3".to_owned()),
        ]
    );
    // The page anchor finds page 3, which has no heading.
    let at = textweaver_formats::pdf::page_anchor(&doc, "page=3").unwrap();
    assert_eq!(slice(&doc, at.0, at.0 + 8), "Week one");
    assert!(textweaver_formats::pdf::is_page_anchor("#page=3"));
    assert_eq!(textweaver_formats::pdf::page_anchor(&doc, "page=9"), None);
    assert_ne!(at, CharPos(0));
}

#[test]
fn form_fields_are_read_label_first_then_value() {
    let doc = load(&fixture("form.pdf"));
    let text = doc.text().to_string();
    for line in [
        "Name: Ada Example",
        "Student ID, required: empty",
        "Date: empty",
        "I agree to the terms of the trip: checked",
        "Payment: Credit card",
        "Course you are taking: Biology 101",
        "Signature: not signed",
    ] {
        assert!(
            text.lines().any(|l| l == line),
            "missing {line:?} in:\n{text}"
        );
    }
    // Printed labels are not heard twice; the lines to write on and the
    // Submit button are gone.
    assert_eq!(text.matches("Name").count(), 1, "{text}");
    assert!(!text.contains("___"), "{text}");
    assert!(!text.contains("Submit"), "{text}");
    // The options' own words stay.
    assert!(text.lines().any(|l| l == "Check"), "{text}");
}

#[test]
fn captions_label_their_table_and_describe_their_figure() {
    let doc = load(&fixture("captions.pdf"));
    let index = doc.marker_index();
    let table = index.iter(MarkerKind::Table, None).next().unwrap();
    assert_eq!(table.label.as_deref(), Some("Table 1: Scores by student"));
    let images: Vec<String> = index
        .iter(MarkerKind::Image, None)
        .map(|m| slice(&doc, m.range.start.0, m.range.end.0))
        .collect();
    assert_eq!(
        images,
        ["Figure 1. The water cycle, from sea to cloud to rain."]
    );
    // Neither caption became a heading.
    let headings: Vec<String> = index
        .iter(MarkerKind::Heading, None)
        .map(|m| slice(&doc, m.range.start.0, m.range.end.0))
        .collect();
    assert_eq!(headings, ["Results"]);
}

/// A one-page PDF with `annots` on the page and `catalog` entries added.
fn hostile(
    build: impl FnOnce(&mut lopdf::Document, lopdf::ObjectId) -> (Vec<Object>, lopdf::Dictionary),
) -> Vec<u8> {
    let mut pdf = lopdf::Document::with_version("1.7");
    let pages_id = pdf.new_object_id();
    let font = pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let content = pdf.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 12 Tf 72 700 Td (Some text on the page.) Tj ET".to_vec(),
    ));
    let page_id = pdf.new_object_id();
    let (annots, extra) = build(&mut pdf, page_id);
    pdf.objects.insert(
        page_id,
        Object::Dictionary(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
            "Contents" => content,
            "Annots" => annots,
        }),
    );
    pdf.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1,
        }),
    );
    let mut catalog = dictionary! { "Type" => "Catalog", "Pages" => pages_id };
    for (k, v) in extra {
        catalog.set(k, v);
    }
    let catalog = pdf.add_object(catalog);
    pdf.trailer.set("Root", catalog);
    let mut out = Vec::new();
    pdf.save_to(&mut out).unwrap();
    out
}

#[test]
fn hostile_annotations_and_forms_load_quickly() {
    let bytes = hostile(|pdf, page| {
        // Replies that answer each other.
        let a = pdf.new_object_id();
        let b = pdf.new_object_id();
        let rect = || vec![72.into(), 690.into(), 200.into(), 712.into()];
        pdf.objects.insert(
            a,
            Object::Dictionary(dictionary! {
                "Subtype" => "Text", "Rect" => rect(), "IRT" => b,
                "Contents" => Object::string_literal("A"),
            }),
        );
        pdf.objects.insert(
            b,
            Object::Dictionary(dictionary! {
                "Subtype" => "Text", "Rect" => rect(), "IRT" => a,
                "Contents" => Object::string_literal("B"),
            }),
        );
        // A name tree whose kid is itself, and a link through it.
        let tree = pdf.new_object_id();
        pdf.objects.insert(
            tree,
            Object::Dictionary(dictionary! { "Kids" => vec![tree.into()] }),
        );
        let link = pdf.add_object(dictionary! {
            "Subtype" => "Link", "Rect" => rect(),
            "Dest" => Object::string_literal("nowhere"),
        });
        // Huge highlight quadrilaterals and impossible rectangles.
        let quads: Vec<Object> = (0..40_000).map(|i| Object::Real(i as f32)).collect();
        let hl = pdf.add_object(dictionary! {
            "Subtype" => "Highlight", "QuadPoints" => quads,
            "Rect" => vec![Object::Real(-1e30), 0.into(), Object::Real(1e30), 1.into()],
        });
        let launch = pdf.add_object(dictionary! {
            "Subtype" => "Link", "Rect" => rect(),
            "A" => dictionary! { "S" => "Launch", "F" => Object::string_literal("calc.exe") },
        });
        // A form whose field is its own kid, and a field with a huge value.
        let field = pdf.new_object_id();
        pdf.objects.insert(
            field,
            Object::Dictionary(dictionary! {
                "FT" => "Tx", "T" => Object::string_literal("loop"),
                "Kids" => vec![field.into()],
            }),
        );
        let big = pdf.add_object(dictionary! {
            "FT" => "Tx", "T" => Object::string_literal("big"), "P" => page,
            "V" => Object::string_literal("x ".repeat(100_000)),
            "Rect" => vec![300.into(), 690.into(), 400.into(), 712.into()],
        });
        let annots = vec![
            a.into(),
            b.into(),
            link.into(),
            hl.into(),
            launch.into(),
            big.into(),
        ];
        let extra = dictionary! {
            "Names" => dictionary! { "Dests" => tree },
            "AcroForm" => dictionary! { "Fields" => vec![field.into(), big.into()] },
        };
        (annots, extra)
    });
    let start = Instant::now();
    let doc = load_bytes(bytes);
    assert!(start.elapsed() < Duration::from_secs(20));
    assert!(doc.text().to_string().contains("Some text on the page."));
    // No link from the launch action or the missing name.
    assert_eq!(doc.marker_index().iter(MarkerKind::Link, None).count(), 0);
    // The huge value is cut.
    let text = doc.text().to_string();
    let line = text.lines().find(|l| l.starts_with("Big: ")).unwrap();
    assert!(line.chars().count() < 400, "{}", line.len());
}

#[cfg(feature = "ocr")]
mod scans {
    use super::*;

    fn models() -> bool {
        textweaver_ocr::models::set_flat_dir(Some(root().join(".cache/models")));
        // Cargo's own temporary folder, inside the build folder, so the
        // cache never lands in the system's temporary folder.
        let cache = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("textweaver-c5-test-{}", std::process::id()));
        textweaver_formats::set_cache_dir(Some(cache));
        let ok = textweaver_ocr::models::OCRS.status() == textweaver_ocr::ModelStatus::Present;
        if !ok {
            eprintln!("skipped: the ocrs models are not in .cache/models");
        }
        ok
    }

    #[test]
    fn the_sideways_scan_is_detected_without_an_engine() {
        let bytes = std::fs::read(fixture("sideways-scan.pdf")).unwrap();
        let pages = textweaver_ocr::pdf::PdfPages::open(bytes).unwrap();
        let (image, _) = pages.page_image(0).unwrap();
        assert_eq!(
            textweaver_ocr::orient::detect(&image),
            textweaver_ocr::orient::Orientation::Turn(1)
        );
    }

    #[test]
    fn a_sideways_scan_reads_in_order() {
        if !models() {
            return;
        }
        let doc = load(&fixture("sideways-scan.pdf"));
        let text = doc.text().to_string();
        let truth = std::fs::read_to_string(root().join("fixtures/w3d/scan-en.txt")).unwrap();
        let first = truth.lines().next().unwrap();
        assert!(
            text.contains(first.split_whitespace().next().unwrap()),
            "{text}"
        );
        let a = text.find("Why quality").unwrap();
        let b = text.find("What helps").unwrap();
        assert!(a < b, "{text}");
    }

    #[test]
    fn a_scanned_table_keeps_its_rows_and_columns() {
        if !models() {
            return;
        }
        let doc = load(&fixture("table-scan.pdf"));
        let index = doc.marker_index();
        let table = index.iter(MarkerKind::Table, None).next().unwrap();
        assert!(table.label.as_deref().unwrap_or("").starts_with("Table 2"));
        let rows = index
            .iter_within(MarkerKind::TableRow, None, table.range)
            .count();
        assert_eq!(rows, 5);
    }
}
