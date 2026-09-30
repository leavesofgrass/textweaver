//! Image files (PNG and JPEG) read by OCR (feature `images`; ADR-0026): a
//! photographed handout, a screenshot of a slide, a scanned page saved as
//! a picture.
//!
//! The image is recognized as one page, and the PDF layout engine rebuilds
//! its paragraphs, headings, and columns. When no engine can run, or the
//! image holds no text, the document is one sentence saying so.

use std::collections::HashMap;

use ropey::Rope;
use textweaver_text::Document;

use super::interp::PageContent;
use super::ocr;
use crate::{LoadError, LoadOptions, Loader, Source, meta_for, title_from_path};

/// Loads images by recognizing their text.
#[derive(Clone, Copy, Debug, Default)]
pub struct ImageLoader;

impl Loader for ImageLoader {
    fn id(&self) -> &'static str {
        "image"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["png", "jpg", "jpeg"]
    }

    fn scan_extensions(&self) -> &'static [&'static str] {
        &[]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let bytes = source.read()?;
        let image = textweaver_ocr::GrayImage::decode(&bytes)
            .map_err(|e| LoadError::Parse(e.to_string()))?;
        let mut meta = meta_for(source, self.id());
        meta.title = title_from_path(source);
        meta.properties.insert(
            "image.size".into(),
            format!("{} by {} pixels", image.width, image.height),
        );
        let say = |meta, text: String| Ok(Document::new(meta, Rope::from_str(&text), Vec::new()));
        if !options.ocr.enabled {
            return say(
                meta,
                "This is a picture. Text recognition (OCR) is turned off, so any text in it cannot be read.".into(),
            );
        }
        let plan = match textweaver_ocr::plan(
            ocr::choice(options.ocr.engine),
            ocr::language(options, None),
        ) {
            Ok(p) => p,
            Err(u) => {
                return say(
                    meta,
                    format!(
                        "This is a picture. Its text must be recognized (OCR) before it can be read aloud. {}",
                        u.message
                    ),
                );
            }
        };
        options
            .progress
            .report(0, 1, "Recognizing the text in the picture.");
        let page = ocr::recognize_cached(&image, &plan, options).map_err(|e| {
            LoadError::Unsupported(format!("the picture's text could not be recognized: {e}"))
        })?;
        options.progress.report(1, 1, "Text recognition finished.");
        // Points at 96 pixels per inch: sizes only matter relative to each
        // other.
        let mut content = PageContent {
            width: image.width as f32 * 0.75,
            height: image.height as f32 * 0.75,
            ..PageContent::default()
        };
        if !ocr::place(&page, &mut content) {
            return say(meta, "No text was found in this picture.".into());
        }
        meta.properties
            .insert("ocr".into(), plan.engine.name().into());
        crate::add_warning(
            &mut meta,
            &format!(
                "This picture's text was recognized with {} (OCR). Recognized text can contain mistakes.",
                plan.engine.name()
            ),
        );
        if let Some(c) = &plan.caution {
            crate::add_warning(&mut meta, c);
        }
        let pages = vec![super::layout::layout(&content, &HashMap::new())];
        let (text, markers, _) = super::build(pages, &HashMap::new(), &[], &[]);
        if meta.title.is_none() {
            meta.title = Some("Picture".into());
        }
        Ok(Document::new(meta, Rope::from_str(&text), markers))
    }
}
