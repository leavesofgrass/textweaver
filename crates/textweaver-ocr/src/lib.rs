//! OCR for textweaver: recognizing the text of scanned pages (ADR-0023).
//!
//! - **ocrs, in process** (feature `ocrs`, on by default): Robert Knight's
//!   pure-Rust engine on the RTen runtime. It reads English (ASCII and the
//!   euro sign). Its models (12.2 MB, CC BY-SA 4.0) are downloaded only
//!   after the user agrees ([`models`]).
//! - **Tesseract, as a separate program** ([`tesseract`]): the fallback
//!   for other languages, and for English when the ocrs models are absent.
//! - **PaddleOCR's Latin model** (experimental): accented Latin-script text
//!   in process, with ocrs finding the lines ([`models::PADDLE_LATIN`]).
//!
//! [`plan`] picks the engine for a language (the `ocr_lang` setting, or the
//! document's own language), and [`recognize`] reads one page image.
//! [`pdf`] finds the image to read for a PDF page: the page's one scanned
//! image when that is all it holds (no rendering needed), else the page
//! rendered with hayro.
//!
//! Recognized pages come back as lines of words with their boxes, in
//! reading order; `textweaver-formats` rebuilds paragraphs, headings, and
//! columns from them with its PDF layout engine.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

pub mod image;
pub mod lang;
pub mod models;
#[cfg(feature = "ocrs")]
mod ocrs_engine;
#[cfg(feature = "ocrs")]
mod paddle;
pub mod pdf;
pub mod tesseract;

pub use image::GrayImage;
pub use models::{ModelSet, ModelStatus};

/// OCR failures, each a sentence that reads well aloud.
#[derive(Debug, thiserror::Error)]
pub enum OcrError {
    /// A model set is not downloaded (its title).
    #[error("{0} are not downloaded yet")]
    ModelsMissing(&'static str),
    /// A model file failed its size or SHA-256 check (its name).
    #[error("the model file {0} is damaged or incomplete; download it again")]
    Checksum(&'static str),
    /// Downloading a file failed (its name, and why).
    #[error("downloading {0} failed: {1}")]
    Download(&'static str, String),
    /// No data folder could be found for the models.
    #[error("there is no folder to keep the text recognition models in")]
    NoDataDir,
    /// Tesseract is not installed.
    #[error("Tesseract is not installed, so text in this language cannot be recognized")]
    TesseractMissing,
    /// Tesseract failed.
    #[error("{0}")]
    Tesseract(String),
    /// The engine failed.
    #[error("text recognition failed: {0}")]
    Engine(String),
    /// The image could not be read.
    #[error("{0}")]
    Image(String),
    /// A file could not be read or written.
    #[error("cannot use {0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),
    /// The user cancelled.
    #[error("text recognition was cancelled")]
    Cancelled,
}

/// A rectangle in image pixels (y grows downward).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x0: f32,
    /// Top edge.
    pub y0: f32,
    /// Right edge.
    pub x1: f32,
    /// Bottom edge.
    pub y1: f32,
}

impl Rect {
    /// The smallest rectangle holding both.
    pub fn union(self, o: Rect) -> Rect {
        Rect {
            x0: self.x0.min(o.x0),
            y0: self.y0.min(o.y0),
            x1: self.x1.max(o.x1),
            y1: self.y1.max(o.y1),
        }
    }

    /// Height.
    pub fn height(&self) -> f32 {
        self.y1 - self.y0
    }
}

/// One recognized word.
#[derive(Clone, Debug, PartialEq)]
pub struct OcrWord {
    /// Its text.
    pub text: String,
    /// Its box.
    pub rect: Rect,
    /// How sure the engine is, from 0 to 1, when it says.
    pub confidence: Option<f32>,
}

/// One line of words, left to right.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OcrLine {
    /// The words.
    pub words: Vec<OcrWord>,
    /// The box around them.
    pub rect: Rect,
}

impl OcrLine {
    /// Adds a word and grows the line's box.
    pub fn push(&mut self, w: OcrWord) {
        self.rect = if self.words.is_empty() {
            w.rect
        } else {
            self.rect.union(w.rect)
        };
        self.words.push(w);
    }

    /// The words joined by spaces.
    pub fn text(&self) -> String {
        self.words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// A recognized page: lines in reading order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OcrPage {
    /// The image's width in pixels.
    pub width: u32,
    /// The image's height in pixels.
    pub height: u32,
    /// The lines.
    pub lines: Vec<OcrLine>,
}

impl OcrPage {
    /// The page's text, one line per line.
    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(OcrLine::text)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// An OCR engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Engine {
    /// ocrs, in process (English).
    Ocrs,
    /// Tesseract, as a separate program (many languages).
    Tesseract,
    /// PaddleOCR's Latin model, in process (experimental).
    Paddle,
}

impl Engine {
    /// The engine's name for a person.
    pub fn name(self) -> &'static str {
        match self {
            Engine::Ocrs => "ocrs",
            Engine::Tesseract => "Tesseract",
            Engine::Paddle => "PaddleOCR",
        }
    }
}

/// Which engine the user asked for (the `ocr_engine` setting).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum EngineChoice {
    /// ocrs for English, Tesseract for other languages.
    #[default]
    Auto,
    /// Always ocrs.
    Ocrs,
    /// Always Tesseract.
    Tesseract,
    /// PaddleOCR's Latin model (experimental).
    Paddle,
}

impl EngineChoice {
    /// Parses `auto`, `ocrs`, `tesseract`, or `paddle` (any case).
    pub fn parse(s: &str) -> Option<EngineChoice> {
        match s.trim().to_ascii_lowercase().as_str() {
            "" | "auto" => Some(EngineChoice::Auto),
            "ocrs" => Some(EngineChoice::Ocrs),
            "tesseract" => Some(EngineChoice::Tesseract),
            "paddle" | "paddleocr" => Some(EngineChoice::Paddle),
            _ => None,
        }
    }
}

/// The engine and languages chosen for some pages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// The engine.
    pub engine: Engine,
    /// Tesseract language codes joined by `+` (`eng`, `fra+eng`).
    pub langs: String,
    /// The Tesseract program, for [`Engine::Tesseract`].
    pub tesseract: Option<PathBuf>,
    /// A caution for the reader, when the choice is a compromise (ocrs
    /// reading a language it does not know).
    pub caution: Option<String>,
}

/// Why no engine can run, with what the user can do about it: sentences
/// that read well aloud.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unavailable {
    /// What is missing and what to do.
    pub message: String,
    /// True when downloading the ocrs models would help (the app can offer
    /// the download).
    pub offer_download: bool,
}

fn models_present(set: &ModelSet) -> bool {
    cfg!(feature = "ocrs") && set.status() == ModelStatus::Present
}

/// What to say when the ocrs models are missing.
fn download_hint() -> String {
    format!(
        "The English text recognition models ({}, {}) are not downloaded. To download them, run tw ocr download, or answer yes when textweaver offers them.",
        models::OCRS.size_text(),
        models::OCRS.licence
    )
}

/// Picks the engine for `lang` (the `ocr_lang` setting or the document's
/// language tag; empty means English) under `choice`.
///
/// With [`EngineChoice::Auto`], English goes to ocrs (Tesseract when the
/// ocrs models are missing), and other languages go to Tesseract (then
/// PaddleOCR's Latin model for Latin-script languages, then ocrs with a
/// caution that accented letters will be misread).
pub fn plan(choice: EngineChoice, lang: &str) -> Result<Plan, Unavailable> {
    let langs = lang::tesseract_codes(lang).unwrap_or_else(|| "eng".to_owned());
    let english = langs.split('+').all(|c| c == "eng");
    let latin = lang::is_latin(&langs);
    let ocrs = models_present(&models::OCRS);
    let paddle = ocrs && models_present(&models::PADDLE_LATIN);
    let tess = tesseract::find();
    let make = |engine: Engine, caution: Option<String>| Plan {
        engine,
        langs: langs.clone(),
        tesseract: tess.clone(),
        caution,
    };
    let no_tesseract = || Unavailable {
        message: format!(
            "Tesseract is not installed, so text in this language ({langs}) cannot be recognized. Install Tesseract with the language's data, or set ocr_lang to eng for English."
        ),
        offer_download: false,
    };
    match choice {
        EngineChoice::Ocrs if ocrs => Ok(make(Engine::Ocrs, None)),
        EngineChoice::Ocrs | EngineChoice::Paddle if !ocrs => Err(Unavailable {
            message: download_hint(),
            offer_download: true,
        }),
        EngineChoice::Paddle if paddle => Ok(make(Engine::Paddle, None)),
        EngineChoice::Paddle => Err(Unavailable {
            message: format!(
                "The PaddleOCR Latin model ({}) is not downloaded. To download it, run tw ocr download paddle-latin.",
                models::PADDLE_LATIN.size_text()
            ),
            offer_download: false,
        }),
        EngineChoice::Tesseract => match tess {
            Some(_) => Ok(make(Engine::Tesseract, None)),
            None => Err(no_tesseract()),
        },
        EngineChoice::Auto if english && ocrs => Ok(make(Engine::Ocrs, None)),
        EngineChoice::Auto if english => match tess {
            Some(_) => Ok(make(Engine::Tesseract, None)),
            None => Err(Unavailable {
                message: format!(
                    "{} Tesseract, which could read it instead, is not installed either.",
                    download_hint()
                ),
                offer_download: true,
            }),
        },
        EngineChoice::Auto if tess.is_some() => Ok(make(Engine::Tesseract, None)),
        EngineChoice::Auto if paddle && latin => Ok(make(Engine::Paddle, None)),
        EngineChoice::Auto if ocrs => Ok(make(
            Engine::Ocrs,
            Some(format!(
                "This text is in another language ({langs}), but Tesseract is not installed, so it was read with the English engine. Accented letters and other scripts will be misread."
            )),
        )),
        EngineChoice::Auto | EngineChoice::Ocrs => Err(no_tesseract()),
    }
}

/// The longest side, in pixels, of an image given to the in-process
/// engines (larger scans are scaled down; about 200 dots per inch on a
/// letter page).
pub const MAX_SIDE_IN_PROCESS: u32 = 2_400;

/// Pages whose longer side is shorter than this (in pixels) are enlarged
/// twice before the in-process engines read them.
pub const MIN_SIDE_IN_PROCESS: u32 = 1_500;

/// The longest side, in pixels, of an image given to Tesseract (about 300
/// dots per inch on a letter page).
pub const MAX_SIDE_TESSERACT: u32 = 3_400;

/// Recognizes one page image with the planned engine. Word boxes are in
/// `image`'s pixels, whatever size the engine worked at.
pub fn recognize(plan: &Plan, image: &GrayImage, cancel: &AtomicBool) -> Result<OcrPage, OcrError> {
    let max = match plan.engine {
        Engine::Tesseract => MAX_SIDE_TESSERACT,
        _ => MAX_SIDE_IN_PROCESS,
    };
    let small = if plan.engine != Engine::Tesseract
        && image.width.max(image.height) < MIN_SIDE_IN_PROCESS
    {
        // A low-resolution page (a 100 dots per inch scan, a phone
        // screenshot): the in-process models read it far better at twice
        // the size (27% of words wrong on a test page, against 15%).
        image.enlarged(2)
    } else {
        image.fit_within(max)
    };
    let page = match plan.engine {
        Engine::Tesseract => {
            let exe = plan
                .tesseract
                .clone()
                .or_else(tesseract::find)
                .ok_or(OcrError::TesseractMissing)?;
            tesseract::recognize(&exe, &small, &plan.langs, cancel)?
        }
        #[cfg(feature = "ocrs")]
        Engine::Ocrs => guarded(|| ocrs_engine::recognize(&small))?,
        #[cfg(feature = "ocrs")]
        Engine::Paddle => guarded(|| paddle::recognize(&small))?,
        #[cfg(not(feature = "ocrs"))]
        Engine::Ocrs | Engine::Paddle => {
            return Err(OcrError::ModelsMissing(models::OCRS.title));
        }
    };
    Ok(scale_page(page, image.width, image.height))
}

/// Runs an in-process engine, turning a panic inside it into an error.
#[cfg(feature = "ocrs")]
fn guarded(f: impl FnOnce() -> Result<OcrPage, OcrError>) -> Result<OcrPage, OcrError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
        .unwrap_or_else(|_| Err(OcrError::Engine("the engine stopped unexpectedly".into())))
}

/// `page`'s boxes scaled to a `width` by `height` image.
fn scale_page(mut page: OcrPage, width: u32, height: u32) -> OcrPage {
    if page.width == width && page.height == height || page.width == 0 || page.height == 0 {
        page.width = width;
        page.height = height;
        return page;
    }
    let sx = width as f32 / page.width as f32;
    let sy = height as f32 / page.height as f32;
    let scale = |r: Rect| Rect {
        x0: r.x0 * sx,
        y0: r.y0 * sy,
        x1: r.x1 * sx,
        y1: r.y1 * sy,
    };
    for line in &mut page.lines {
        line.rect = scale(line.rect);
        for w in &mut line.words {
            w.rect = scale(w.rect);
        }
    }
    page.width = width;
    page.height = height;
    page
}

/// Loads the in-process engines' models now (checking their SHA-256), so
/// the first page does not wait; a no-op for Tesseract.
pub fn warm_up(plan: &Plan) -> Result<(), OcrError> {
    match plan.engine {
        #[cfg(feature = "ocrs")]
        Engine::Ocrs => ocrs_engine::engine().map(|_| ()),
        #[cfg(feature = "ocrs")]
        Engine::Paddle => paddle::engine().map(|_| ()),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_choices_parse() {
        assert_eq!(EngineChoice::parse("Auto"), Some(EngineChoice::Auto));
        assert_eq!(EngineChoice::parse(""), Some(EngineChoice::Auto));
        assert_eq!(
            EngineChoice::parse("TESSERACT"),
            Some(EngineChoice::Tesseract)
        );
        assert_eq!(EngineChoice::parse("paddleocr"), Some(EngineChoice::Paddle));
        assert_eq!(EngineChoice::parse("gpt"), None);
    }

    #[test]
    fn pages_scale_to_the_original_image() {
        let mut line = OcrLine::default();
        line.push(OcrWord {
            text: "a".into(),
            rect: Rect {
                x0: 10.0,
                y0: 10.0,
                x1: 20.0,
                y1: 20.0,
            },
            confidence: None,
        });
        let page = OcrPage {
            width: 100,
            height: 50,
            lines: vec![line],
        };
        let big = scale_page(page, 200, 100);
        assert_eq!(big.lines[0].rect.x1, 40.0);
        assert_eq!(big.lines[0].words[0].rect.y0, 20.0);
        assert_eq!(big.text(), "a");
    }
}
