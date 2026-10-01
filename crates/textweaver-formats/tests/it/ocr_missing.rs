//! OCR when the ocrs models are not downloaded: the document says what is
//! missing and how to get it (its own test binary, because the model
//! folder is chosen per process).

#![cfg(feature = "ocr")]

use std::path::Path;

use textweaver_formats::{LoadOptions, OcrEngineChoice, OcrOptions, Registry, Source};

#[test]
fn missing_models_are_announced_with_the_way_to_get_them() {
    let empty = tempfile::tempdir().unwrap();
    textweaver_ocr::models::set_flat_dir(Some(empty.path().to_owned()));
    textweaver_formats::set_cache_dir(Some(empty.path().join("cache")));
    let options = LoadOptions {
        ocr: OcrOptions {
            engine: OcrEngineChoice::Ocrs,
            ..OcrOptions::default()
        },
        ..LoadOptions::default()
    };
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/w3d");
    let load = |name: &str| {
        Registry::with_builtins()
            .load(&Source::Path(fixtures.join(name)), &options)
            .unwrap()
            .text()
            .to_string()
    };
    let pdf = load("scan-en.pdf");
    assert!(
        pdf.starts_with(textweaver_formats::pdf::NO_TEXT_LAYER),
        "{pdf}"
    );
    assert!(pdf.contains("12.2 MB, CC BY-SA 4.0"), "{pdf}");
    assert!(pdf.contains("tw ocr download"), "{pdf}");
    let png = load("scan-fr.png");
    assert!(png.starts_with("This is a picture."), "{png}");
    assert!(png.contains("tw ocr download"), "{png}");
}
