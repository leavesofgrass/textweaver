//! PDF fixtures: the loaded document, its Markdown, and the Markdown read
//! back. The fixtures come from `fixtures/a/make_pdfs.py` (standard fonts,
//! hand-placed text) and `fixtures/a/browser.html` printed by a browser
//! (embedded subset fonts, tagged, justified two-column text).
#![cfg(feature = "pdf")]

use std::path::PathBuf;
use std::time::Instant;

use serde_json::{Value, json};
use textweaver_core::MarkerKind;
use textweaver_formats::{LoadOptions, Loader, MarkdownLoader, Registry, Source, to_markdown};
use textweaver_text::Document;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/a")
        .join(name)
}

fn load(name: &str) -> Document {
    let started = Instant::now();
    let doc = Registry::with_builtins()
        .load(&Source::Path(fixture(name)), &LoadOptions::default())
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    eprintln!("{name}: loaded in {:?}", started.elapsed());
    assert_eq!(doc.meta.format, "pdf");
    doc
}

/// The document as JSON without the path and without inline markers, which
/// the Markdown snapshot shows more readably.
fn view(doc: &Document) -> Value {
    let mut meta = doc.meta.clone();
    meta.path = None;
    let markers: Vec<Value> = doc
        .markers()
        .iter()
        .filter(|m| {
            !matches!(
                m.kind,
                MarkerKind::TableCell | MarkerKind::Bold | MarkerKind::Italic
            )
        })
        .map(|m| {
            let text = doc.slice(m.range);
            let short: String = text.chars().take(60).collect();
            let mut v = json!({
                "kind": m.kind,
                "range": [m.range.start.0, m.range.end.0],
                "text": short,
            });
            if m.level != 0 {
                v["level"] = json!(m.level);
            }
            if let Some(l) = &m.label {
                v["label"] = json!(l);
            }
            v
        })
        .collect();
    json!({ "meta": meta, "text": doc.text().to_string(), "markers": markers })
}

fn headings(doc: &Document) -> Vec<(u8, String)> {
    doc.marker_index()
        .iter(MarkerKind::Heading, None)
        .map(|m| (m.level, doc.slice(m.range)))
        .collect()
}

/// Markdown export read back by the Markdown loader keeps the headings,
/// list items, and table rows.
fn round_trip(doc: &Document) {
    let md = to_markdown(doc);
    let back = MarkdownLoader
        .load(
            &Source::Bytes {
                data: md.into_bytes(),
                hint: "md".into(),
            },
            &LoadOptions::default(),
        )
        .unwrap();
    assert_eq!(headings(&back), headings(doc));
    for kind in [
        MarkerKind::ListItem,
        MarkerKind::TableRow,
        MarkerKind::Paragraph,
    ] {
        assert_eq!(
            back.marker_index().count(kind, None),
            doc.marker_index().count(kind, None),
            "{kind:?}"
        );
    }
}

#[test]
fn single_column_structure() {
    let doc = load("single.pdf");
    insta::assert_json_snapshot!("pdf_single_document", view(&doc));
    insta::assert_snapshot!("pdf_single_markdown", to_markdown(&doc));
    assert_eq!(
        headings(&doc),
        [
            (1, "A Guide to Accessible Reading".to_owned()),
            (2, "1 Introduction".to_owned()),
            (3, "1.1 Background".to_owned()),
            (3, "Summary".to_owned()),
            (2, "2 Results".to_owned()),
        ]
    );
    let text = doc.text().to_string();
    assert!(text.contains("carries information that"), "de-hyphenated");
    assert!(text.contains("well-known compounds"), "real hyphens kept");
    assert_eq!(
        doc.meta.title.as_deref(),
        Some("A Guide to Accessible Reading")
    );
    assert_eq!(doc.meta.language.as_deref(), Some("en-US"));
    // Bookmarks become sections.
    let sections: Vec<String> = doc
        .marker_index()
        .iter(MarkerKind::SectionBreak, None)
        .filter_map(|m| m.label.clone())
        .collect();
    assert_eq!(sections, ["1 Introduction", "1.1 Background", "2 Results"]);
    round_trip(&doc);
}

#[test]
fn two_columns_read_in_order() {
    let doc = load("columns.pdf");
    insta::assert_snapshot!("pdf_columns_markdown", to_markdown(&doc));
    let text = doc.text().to_string();
    let order = [
        "Two Column Layout",
        "The left column begins",
        "in the left column.",
        "The right column continues",
        "in the right column.",
        "Figure 1.",
        "the left column resumes",
        "This is the last sentence.",
    ];
    let mut at = 0;
    for part in order {
        let found = text[at..]
            .find(part)
            .unwrap_or_else(|| panic!("{part:?} out of order"));
        at += found + part.len();
    }
    assert_eq!(doc.marker_index().count(MarkerKind::Table, None), 0);
    round_trip(&doc);
}

#[test]
fn running_heads_removed_and_pages_marked() {
    let doc = load("running.pdf");
    insta::assert_json_snapshot!("pdf_running_document", view(&doc));
    let text = doc.text().to_string();
    assert!(!text.contains("Journal of Reading Examples"));
    let pages: Vec<(String, String)> = doc
        .marker_index()
        .iter(MarkerKind::PageBreak, None)
        .map(|m| (m.label.clone().unwrap_or_default(), doc.slice(m.range)))
        .collect();
    assert_eq!(pages.len(), 3);
    // Printed page labels from /PageLabels: a roman first page, then 1, 2.
    let labels: Vec<&str> = pages.iter().map(|(l, _)| l.as_str()).collect();
    assert_eq!(labels, ["i", "1", "2"]);
    // Page numbers are not text; a paragraph continues across a page.
    for (_, t) in &pages {
        assert!(
            !t.lines()
                .any(|l| !l.trim().is_empty() && l.trim().chars().all(|c| c.is_ascii_digit()))
        );
    }
    let crossing = doc
        .marker_index()
        .iter(MarkerKind::Paragraph, None)
        .filter(|p| {
            pages.len() > 1
                && doc
                    .marker_index()
                    .iter(MarkerKind::PageBreak, None)
                    .any(|pg| pg.range.start > p.range.start && pg.range.start < p.range.end)
        })
        .count();
    assert!(crossing >= 1, "a paragraph continues onto the next page");
    round_trip(&doc);
}

#[test]
fn browser_printed_two_columns() {
    let doc = load("browser.pdf");
    insta::assert_snapshot!("pdf_browser_markdown", to_markdown(&doc));
    assert_eq!(
        headings(&doc)
            .iter()
            .map(|(l, t)| format!("{l} {t}"))
            .collect::<Vec<_>>(),
        [
            "1 Printed Article Sample",
            "2 Introduction",
            "2 Methods",
            "2 Results",
            "2 Discussion"
        ]
    );
    let text = doc.text().to_string();
    assert!(text.contains("accessibility, internationalization, and characterization."));
    assert!(text.contains("A bar chart of reading speed by method"));
    assert_eq!(doc.marker_index().count(MarkerKind::ListItem, None), 6);
    assert_eq!(doc.marker_index().count(MarkerKind::TableRow, None), 4);
    round_trip(&doc);
}

#[test]
fn footnotes_do_not_interrupt_a_paragraph_across_pages() {
    let doc = load("notes.pdf");
    insta::assert_json_snapshot!("pdf_notes_document", view(&doc));
    let text = doc.text().to_string();
    assert!(
        text.contains("which is printed in smaller type, and ends on the second page"),
        "{text}"
    );
    let note = doc
        .marker_index()
        .iter(MarkerKind::Footnote, Some(1))
        .map(|m| doc.slice(m.range))
        .collect::<Vec<_>>();
    assert_eq!(
        note,
        ["1 The footnote sits at the foot of page one, in smaller type."]
    );
    // The note is read after the paragraph it interrupted on the page.
    assert!(text.find("second page").unwrap_or(0) < text.find("The footnote").unwrap_or(0));
}

/// A minimal PDF with one page and the given content stream.
fn one_page_pdf(content: &str) -> Vec<u8> {
    let objs = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len() + 1),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n{o}\nendobj\n", i + 1).bytes());
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).bytes());
    for off in offsets {
        out.extend(format!("{off:010} 00000 n \n").bytes());
    }
    out.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objs.len() + 1
        )
        .bytes(),
    );
    out
}

#[test]
fn scans_and_odd_streams_load_gracefully() {
    let load = |content: &str| {
        textweaver_formats::PdfLoader
            .load(
                &Source::Bytes {
                    data: one_page_pdf(content),
                    hint: "pdf".into(),
                },
                &LoadOptions::default(),
            )
            .unwrap()
    };
    // No text layer: one sentence saying so.
    let scan = load("q 100 0 0 100 0 0 cm Q");
    assert_eq!(
        scan.text().to_string(),
        textweaver_formats::pdf::NO_TEXT_LAYER
    );
    // Odd operands, unknown fonts and operators, unbalanced Q: no panic.
    let odd = load("Q Q BT /Nope 12 Tf 10 10 Td (Hello) Tj [(wor) -250 (ld)] TJ 5 Tj ET EMC xyz");
    assert!(
        odd.text().to_string().starts_with("Hello"),
        "{:?}",
        odd.text().to_string()
    );
    // Text in an Artifact is not read; ActualText replaces what it covers.
    let tagged = load(
        "/Artifact BMC BT /F1 10 Tf 10 190 Td (Header) Tj ET EMC /Span << /ActualText (fifty) >> BDC BT /F1 10 Tf 10 100 Td (50) Tj ET EMC",
    );
    assert_eq!(tagged.text().to_string(), "fifty");
}

#[test]
fn broken_and_protected_pdfs_fail_clearly() {
    let err = Registry::with_builtins()
        .load(
            &Source::Bytes {
                data: b"%PDF-1.4 not really".to_vec(),
                hint: "pdf".into(),
            },
            &LoadOptions::default(),
        )
        .unwrap_err();
    assert!(err.to_string().contains("not a readable PDF"), "{err}");
}

/// Load times (fastest of five, from memory) of the PDF fixtures and of any
/// PDFs named in `TW_PDF_BENCH` (`;`-separated; for example a large one
/// made with `python fixtures/a/make_pdfs.py big big.pdf 300`); run with
/// `cargo test --release -p textweaver-formats --test it -- pdf:: --ignored --nocapture`.
#[test]
#[ignore = "benchmark"]
fn pdf_load_times() {
    let mut paths: Vec<PathBuf> = ["single.pdf", "columns.pdf", "running.pdf", "browser.pdf"]
        .iter()
        .map(|n| fixture(n))
        .collect();
    if let Some(extra) = std::env::var_os("TW_PDF_BENCH") {
        paths.extend(std::env::split_paths(&extra));
    }
    let loader = textweaver_formats::PdfLoader;
    for path in paths {
        let data = std::fs::read(&path).expect("readable");
        let source = Source::Bytes {
            data,
            hint: "pdf".into(),
        };
        let mut best = std::time::Duration::MAX;
        let mut doc = None;
        for _ in 0..5 {
            let started = Instant::now();
            let d = loader
                .load(&source, &LoadOptions::default())
                .expect("loads");
            best = best.min(started.elapsed());
            doc = Some(d);
        }
        let doc = doc.expect("loaded");
        eprintln!(
            "{}: {} pages, {} chars, {:?}",
            path.display(),
            doc.meta.properties.get("pages").map_or("?", String::as_str),
            doc.len_chars(),
            best
        );
    }
}
