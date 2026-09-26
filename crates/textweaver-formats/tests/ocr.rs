//! OCR of scanned PDFs and pictures (ADR-0025), on the scans in
//! `fixtures/w3d` (made by `make_scans.py`).
//!
//! The ocrs tests need the models in `.cache/models` at the workspace root
//! (downloaded once for tests; git ignores the folder) and are skipped,
//! with a note, without them. The Tesseract tests need Tesseract with the
//! French data, and are skipped without it.

#![cfg(feature = "ocr")]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use textweaver_core::MarkerKind;
use textweaver_formats::{
    LoadOptions, OcrEngineChoice, OcrOptions, Progress, Registry, Source, warnings,
};
use textweaver_text::Document;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(name: &str) -> PathBuf {
    root().join("fixtures/w3d").join(name)
}

/// Points the OCR crate at the test models (and the caches at a temporary
/// folder); false when the models are not there.
fn models() -> bool {
    let dir = root().join(".cache/models");
    textweaver_ocr::models::set_flat_dir(Some(dir));
    let cache = std::env::temp_dir().join(format!("textweaver-ocr-test-{}", std::process::id()));
    textweaver_formats::set_cache_dir(Some(cache));
    let ok = textweaver_ocr::models::OCRS.status() == textweaver_ocr::ModelStatus::Present;
    if !ok {
        eprintln!("skipped: the ocrs models are not in .cache/models");
    }
    ok
}

fn options(engine: OcrEngineChoice, lang: &str) -> LoadOptions {
    LoadOptions {
        ocr: OcrOptions {
            engine,
            lang: lang.into(),
            ..OcrOptions::default()
        },
        ..LoadOptions::default()
    }
}

fn load(path: &Path, options: &LoadOptions) -> Document {
    Registry::with_builtins()
        .load(&Source::Path(path.to_owned()), options)
        .unwrap()
}

/// The share of the truth's words that appear in the text (a rough
/// accuracy, independent of order).
fn words_found(truth: &str, text: &str) -> f64 {
    let got: std::collections::HashSet<&str> = text.split_whitespace().collect();
    let words: Vec<&str> = truth.split_whitespace().collect();
    words.iter().filter(|w| got.contains(*w)).count() as f64 / words.len() as f64
}

#[test]
fn scanned_pdfs_are_read_with_ocrs() {
    if !models() {
        return;
    }
    let reports = Arc::new(Mutex::new(Vec::new()));
    let r = Arc::clone(&reports);
    let opts = LoadOptions {
        progress: Progress::new(move |p| r.lock().unwrap().push(p.message.clone())),
        ..options(OcrEngineChoice::Ocrs, "")
    };
    let doc = load(&fixture("scan-en.pdf"), &opts);
    let text = doc.text().to_string();
    let truth = std::fs::read_to_string(fixture("scan-en.txt")).unwrap();
    let found = words_found(&truth, &text);
    assert!(found > 0.8, "only {found} of the words were read:\n{text}");
    // The layout engine rebuilt the structure: the title is a heading, the
    // lines of a paragraph are joined.
    let first = doc
        .marker_index()
        .iter(MarkerKind::Heading, None)
        .next()
        .map(|m| (doc.slice(m.range).to_string(), m.level));
    assert_eq!(first, Some(("Reading Scanned Pages Aloud".into(), 1)));
    let para = text
        .split("\n\n")
        .find(|p| p.contains("picture of text"))
        .unwrap_or_default();
    // (The layout engine ends a paragraph after a short line, so the ragged
    // third line of this one splits it; born-digital PDFs do the same.)
    assert!(
        para.contains("optical character") && !para.contains('\n'),
        "the first paragraph's lines are joined:\n{text}"
    );
    let pages: Vec<Option<String>> = doc
        .marker_index()
        .iter(MarkerKind::PageBreak, None)
        .map(|m| m.label.clone())
        .collect();
    assert_eq!(pages, [Some("1".into()), Some("2".into())]);
    assert_eq!(
        doc.meta.properties.get("ocr").map(String::as_str),
        Some("ocrs")
    );
    assert!(
        warnings(&doc.meta)[0]
            .starts_with("This PDF has no text layer, so its text was recognized with ocrs"),
        "{:?}",
        warnings(&doc.meta)
    );
    let reports = reports.lock().unwrap().clone();
    assert_eq!(reports[0], "Recognizing text on page 1 (1 of 2).");
    assert_eq!(
        reports.last().map(String::as_str),
        Some("Text recognition finished.")
    );
    // Opening it again reads the cached recognition: the same text.
    let again = load(&fixture("scan-en.pdf"), &options(OcrEngineChoice::Ocrs, ""));
    assert_eq!(again.text().to_string(), text);
}

#[test]
fn pictures_are_read_with_ocrs() {
    if !models() {
        return;
    }
    let doc = load(
        &fixture("scan-small.png"),
        &options(OcrEngineChoice::Ocrs, "en"),
    );
    assert_eq!(doc.meta.format, "image");
    let text = doc.text().to_string();
    assert!(text.contains("Scanned"), "{text}");
    assert_eq!(
        doc.meta.properties.get("ocr").map(String::as_str),
        Some("ocrs")
    );
}

#[test]
fn cancelling_stops_before_the_next_page() {
    if !models() {
        return;
    }
    let progress = Progress::new(|_| {});
    progress.cancel();
    let opts = LoadOptions {
        progress,
        ..options(OcrEngineChoice::Ocrs, "")
    };
    let doc = load(&fixture("scan-en.pdf"), &opts);
    assert!(
        doc.text()
            .to_string()
            .ends_with("Text recognition was cancelled before any page was read."),
        "{}",
        doc.text()
    );
    assert!(
        warnings(&doc.meta)
            .iter()
            .any(|w| w == "Text recognition was cancelled, so pages 1 to 2 were not read.")
    );
}

#[test]
fn turning_ocr_off_keeps_the_old_sentence() {
    let opts = LoadOptions {
        ocr: OcrOptions {
            enabled: false,
            ..OcrOptions::default()
        },
        ..LoadOptions::default()
    };
    let doc = load(&fixture("scan-en.pdf"), &opts);
    assert_eq!(
        doc.text().to_string(),
        textweaver_formats::pdf::NO_TEXT_LAYER
    );
    let pic = load(&fixture("scan-fr.png"), &opts);
    assert!(pic.text().to_string().contains("turned off"));
}

#[test]
fn french_goes_to_tesseract() {
    let Some(exe) = textweaver_ocr::tesseract::find() else {
        eprintln!("skipped: Tesseract is not installed");
        return;
    };
    if !textweaver_ocr::tesseract::languages(&exe)
        .iter()
        .any(|l| l == "fra")
    {
        eprintln!("skipped: Tesseract has no French data");
        return;
    }
    models();
    let doc = load(
        &fixture("scan-fr.png"),
        &options(OcrEngineChoice::Auto, "fr"),
    );
    let text = doc.text().to_string();
    assert_eq!(
        doc.meta.properties.get("ocr").map(String::as_str),
        Some("Tesseract")
    );
    for word in ["numérisées", "élèves", "déjà", "Noël", "améliorent"] {
        assert!(text.contains(word), "{word} missing from:\n{text}");
    }
}
