//! The ocrs engine, loaded once per process from the downloaded models
//! (each file checked by SHA-256 first).

use std::sync::{Arc, Mutex};

use ocrs::{ImageSource, OcrEngine, OcrEngineParams, TextItem};

use crate::image::GrayImage;
use crate::models::{self, ModelStatus};
use crate::{OcrError, OcrLine, OcrPage, OcrWord, Rect};

/// The loaded engine, shared by every document.
static ENGINE: Mutex<Option<Arc<OcrEngine>>> = Mutex::new(None);

fn engine_error(e: impl std::fmt::Display) -> OcrError {
    OcrError::Engine(e.to_string())
}

/// Loads an rten model from checked bytes.
pub(crate) fn load_model(bytes: Vec<u8>) -> Result<rten::Model, OcrError> {
    rten::Model::load(bytes).map_err(engine_error)
}

/// The ocrs engine (loaded on first use).
pub(crate) fn engine() -> Result<Arc<OcrEngine>, OcrError> {
    let mut slot = ENGINE.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(e) = slot.as_ref() {
        return Ok(Arc::clone(e));
    }
    if models::OCRS.status() != ModelStatus::Present {
        return Err(OcrError::ModelsMissing(models::OCRS.title));
    }
    let mut files = models::OCRS.load_verified()?.into_iter();
    let (Some(detection), Some(recognition)) = (files.next(), files.next()) else {
        return Err(OcrError::ModelsMissing(models::OCRS.title));
    };
    let engine = OcrEngine::new(OcrEngineParams {
        detection_model: Some(load_model(detection)?),
        recognition_model: Some(load_model(recognition)?),
        ..OcrEngineParams::default()
    })
    .map_err(engine_error)?;
    let engine = Arc::new(engine);
    *slot = Some(Arc::clone(&engine));
    Ok(engine)
}

/// Converts an ocrs rectangle to ours.
pub(crate) fn rect(r: rten_imageproc::Rect) -> Rect {
    Rect {
        x0: r.left() as f32,
        y0: r.top() as f32,
        x1: r.right() as f32,
        y1: r.bottom() as f32,
    }
}

/// Recognizes `image` with ocrs.
pub(crate) fn recognize(image: &GrayImage) -> Result<OcrPage, OcrError> {
    let engine = engine()?;
    let source =
        ImageSource::from_bytes(&image.data, (image.width, image.height)).map_err(engine_error)?;
    let input = engine.prepare_input(source).map_err(engine_error)?;
    let words = engine.detect_words(&input).map_err(engine_error)?;
    let lines = engine.find_text_lines(&input, &words);
    let recognized = engine
        .recognize_text(&input, &lines)
        .map_err(engine_error)?;
    let mut page = OcrPage {
        width: image.width,
        height: image.height,
        lines: Vec::new(),
        turned: 0,
    };
    for line in recognized.into_iter().flatten() {
        let mut out = OcrLine::default();
        for w in line.words() {
            let text = w.to_string();
            if text.trim().is_empty() {
                continue;
            }
            out.push(OcrWord {
                text,
                rect: rect(w.bounding_rect()),
                confidence: None,
            });
        }
        if !out.words.is_empty() {
            page.lines.push(out);
        }
    }
    Ok(page)
}
