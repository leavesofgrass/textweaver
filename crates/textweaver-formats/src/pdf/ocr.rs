//! OCR for PDF pages with no text layer (feature `ocr`; ADR-0026).
//!
//! Each such page's image (its one scanned image, else the page rendered)
//! goes to the engine `textweaver_ocr::plan` picks for the language (the
//! `ocr_lang` setting, else the PDF's `/Lang`, else English). The words
//! that come back become glyphs on the page, placed where they were
//! printed, so the layout engine rebuilds lines, paragraphs, headings (by
//! size), columns, and reading order exactly as it does for born-digital
//! pages, and running heads and page numbers are removed.
//!
//! Recognized pages are kept in the cache folder (`ocr/`, keyed by the
//! page image, the engine, and the languages), so opening the book again
//! is instant. Progress is reported per page, and a cancel stops before the
//! next page; the pages not reached say so.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use textweaver_ocr::{Engine, OcrLine, OcrPage, OcrWord, Plan, Rect};
use textweaver_text::DocumentMeta;

use super::interp::{Glyph, NO_MCID, PageContent};
use crate::{LoadOptions, OcrEngineChoice};

/// How recognizing a PDF's blank pages went.
#[derive(Debug, Default)]
pub(super) struct Outcome {
    /// The engine used, when one could run.
    engine: Option<Engine>,
    /// A caution from the plan (English engine reading another language).
    caution: Option<String>,
    /// Why no engine could run.
    unavailable: Option<String>,
    /// Pages (from 0) that were recognized and had text.
    recognized: Vec<usize>,
    /// Pages whose recognition failed, and why.
    failed: Vec<(usize, String)>,
    /// Pages not reached because the user cancelled.
    cancelled: Vec<usize>,
}

pub(super) fn choice(c: OcrEngineChoice) -> textweaver_ocr::EngineChoice {
    match c {
        OcrEngineChoice::Auto => textweaver_ocr::EngineChoice::Auto,
        OcrEngineChoice::Ocrs => textweaver_ocr::EngineChoice::Ocrs,
        OcrEngineChoice::Tesseract => textweaver_ocr::EngineChoice::Tesseract,
        OcrEngineChoice::Paddle => textweaver_ocr::EngineChoice::Paddle,
    }
}

/// The language to recognize: the setting, else the document's.
pub(super) fn language<'a>(options: &'a LoadOptions, document: Option<&'a str>) -> &'a str {
    let set = options.ocr.lang.trim();
    if set.is_empty() {
        document.unwrap_or("")
    } else {
        set
    }
}

/// Page numbers as a person says them: "page 3", "pages 3 to 5", "pages
/// 1, 4, and 6 to 9".
pub(super) fn page_list(pages: &[usize]) -> String {
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for &p in pages {
        match runs.last_mut() {
            Some((_, end)) if *end + 1 == p => *end = p,
            _ => runs.push((p, p)),
        }
    }
    let parts: Vec<String> = runs
        .iter()
        .map(|&(a, b)| {
            if a == b {
                (a + 1).to_string()
            } else {
                format!("{} to {}", a + 1, b + 1)
            }
        })
        .collect();
    let word = if pages.len() == 1 { "page" } else { "pages" };
    let list = match parts.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [a, b] => format!("{a} and {b}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    };
    format!("{word} {list}")
}

impl Outcome {
    /// Adds the outcome's warnings and properties to `meta`. Returns the
    /// sentence the document reads as when no page has text.
    pub(super) fn report(
        &self,
        meta: &mut DocumentMeta,
        total: usize,
        blank: &[usize],
    ) -> Option<String> {
        if let Some(engine) = self.engine
            && !self.recognized.is_empty()
        {
            meta.properties.insert("ocr".into(), engine.name().into());
            meta.properties
                .insert("ocr.pages".into(), self.recognized.len().to_string());
            let which = if self.recognized.len() == total {
                "This PDF has no text layer, so its text was recognized".to_owned()
            } else {
                let pages = page_list(&self.recognized);
                let mut s = pages[..1].to_uppercase();
                s.push_str(&pages[1..]);
                format!("{s} had no text layer, so their text was recognized")
            };
            crate::add_warning(
                meta,
                &format!(
                    "{which} with {} (OCR). Recognized text can contain mistakes.",
                    engine.name()
                ),
            );
        }
        if let Some(c) = &self.caution {
            crate::add_warning(meta, c);
        }
        for (page, why) in &self.failed {
            crate::add_warning(
                meta,
                &format!("Page {} could not be recognized: {why}.", page + 1),
            );
        }
        if !self.cancelled.is_empty() {
            crate::add_warning(
                meta,
                &format!(
                    "Text recognition was canceled, so {} were not read.",
                    page_list(&self.cancelled)
                ),
            );
        }
        if let Some(why) = &self.unavailable {
            if blank.len() < total {
                crate::add_warning(
                    meta,
                    &format!(
                        "{} {} no text layer and could not be recognized. {why}",
                        capitalized(&page_list(blank)),
                        if blank.len() == 1 { "has" } else { "have" }
                    ),
                );
            }
            return Some(format!("{} {why}", super::NO_TEXT_LAYER));
        }
        if self.recognized.is_empty() && self.engine.is_some() && self.cancelled.is_empty() {
            return Some(
                "This PDF has no text layer, and no text was found when its pages were recognized. It may hold only pictures.".into(),
            );
        }
        if !self.cancelled.is_empty() && self.recognized.is_empty() {
            return Some(format!(
                "{} Text recognition was canceled before any page was read.",
                super::NO_TEXT_LAYER
            ));
        }
        None
    }
}

fn capitalized(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// Recognizes `pages` (from 0) of the PDF in `bytes`, writing their words
/// into `contents` as glyphs.
pub(super) fn recognize_pages(
    bytes: &[u8],
    contents: &mut [PageContent],
    pages: &[usize],
    doc_lang: Option<&str>,
    options: &LoadOptions,
) -> Outcome {
    let mut out = Outcome::default();
    let plan = match textweaver_ocr::plan(choice(options.ocr.engine), language(options, doc_lang)) {
        Ok(p) => p,
        Err(u) => {
            out.unavailable = Some(u.message);
            return out;
        }
    };
    out.engine = Some(plan.engine);
    out.caution.clone_from(&plan.caution);
    let pdf = match textweaver_ocr::pdf::PdfPages::open(bytes.to_vec()) {
        Ok(p) => p,
        Err(e) => {
            out.failed = pages.iter().map(|&p| (p, e.to_string())).collect();
            return out;
        }
    };
    let progress = &options.progress;
    let total = pages.len();
    for (k, &i) in pages.iter().enumerate() {
        if progress.is_cancelled() {
            out.cancelled = pages[k..].to_vec();
            break;
        }
        progress.report(
            k,
            total,
            format!("Recognizing text on page {} ({} of {total}).", i + 1, k + 1),
        );
        match recognize_one(&pdf, i, &plan, options) {
            Ok(page) => {
                if place(&page, &mut contents[i]) {
                    out.recognized.push(i);
                }
            }
            Err(textweaver_ocr::OcrError::Cancelled) => {
                out.cancelled = pages[k..].to_vec();
                break;
            }
            Err(e) => out.failed.push((i, e.to_string())),
        }
    }
    progress.report(total, total, "Text recognition finished.");
    out
}

fn recognize_one(
    pdf: &textweaver_ocr::pdf::PdfPages,
    index: usize,
    plan: &Plan,
    options: &LoadOptions,
) -> Result<OcrPage, textweaver_ocr::OcrError> {
    let (image, _) = pdf.page_image(index)?;
    recognize_cached(&image, plan, options)
}

/// Recognizes an image, from the cache when it was recognized before with
/// the same engine and languages.
pub(super) fn recognize_cached(
    image: &textweaver_ocr::GrayImage,
    plan: &Plan,
    options: &LoadOptions,
) -> Result<OcrPage, textweaver_ocr::OcrError> {
    let key = cache_file(image, plan);
    if let Some(page) = key.as_ref().and_then(read_cached) {
        return Ok(page);
    }
    let page = textweaver_ocr::recognize(plan, image, options.progress.cancel_flag())?;
    if let Some(path) = key {
        write_cached(&path, &page);
    }
    Ok(page)
}

/// The cache file for an image's recognition.
fn cache_file(image: &textweaver_ocr::GrayImage, plan: &Plan) -> Option<PathBuf> {
    let mut h = crate::cache::fnv1a64(&image.data);
    for part in [
        // Pages recognized before sideways pages were turned are read
        // again.
        "turned",
        plan.engine.name(),
        &plan.langs,
        &image.width.to_string(),
        &image.height.to_string(),
    ] {
        h ^= crate::cache::fnv1a64(part.as_bytes()).rotate_left(17);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    crate::cache_dir().map(|d| d.join("ocr").join(format!("{h:016x}.json")))
}

#[derive(Serialize, Deserialize)]
struct Cached {
    width: u32,
    height: u32,
    /// Lines of words: text and box (x0, y0, x1, y1).
    lines: Vec<Vec<(String, [f32; 4])>>,
    /// Quarter turns the image was read at.
    #[serde(default)]
    turned: u8,
}

fn read_cached(path: &PathBuf) -> Option<OcrPage> {
    let bytes = std::fs::read(path).ok()?;
    let c: Cached = serde_json::from_slice(&bytes).ok()?;
    let mut page = OcrPage {
        width: c.width,
        height: c.height,
        lines: Vec::new(),
        turned: c.turned % 4,
    };
    for words in c.lines {
        let mut line = OcrLine::default();
        for (text, [x0, y0, x1, y1]) in words {
            line.push(OcrWord {
                text,
                rect: Rect { x0, y0, x1, y1 },
                confidence: None,
            });
        }
        page.lines.push(line);
    }
    Some(page)
}

fn write_cached(path: &PathBuf, page: &OcrPage) {
    let c = Cached {
        width: page.width,
        height: page.height,
        turned: page.turned,
        lines: page
            .lines
            .iter()
            .map(|l| {
                l.words
                    .iter()
                    .map(|w| (w.text.clone(), [w.rect.x0, w.rect.y0, w.rect.x1, w.rect.y1]))
                    .collect()
            })
            .collect(),
    };
    let Ok(json) = serde_json::to_vec(&c) else {
        return;
    };
    if let Some(dir) = path.parent()
        && std::fs::create_dir_all(dir).is_ok()
    {
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}

/// Writes a recognized page's words into `content` as glyphs, scaled from
/// image pixels to the page's points. Returns false when there were none.
///
/// A page read turned ([`OcrPage::turned`]: sideways or upside down) is
/// turned the same way here, so its lines run across and read in order.
pub(super) fn place(page: &OcrPage, content: &mut PageContent) -> bool {
    if page.width == 0 || page.height == 0 {
        return false;
    }
    content.ocr = true;
    if content.width > 0.0 && content.height > 0.0 {
        content.turn(page.turned);
    }
    if content.width <= 0.0 || content.height <= 0.0 {
        // A page that could not be interpreted: assume 200 dots per inch.
        content.width = page.width as f32 * 72.0 / 200.0;
        content.height = page.height as f32 * 72.0 / 200.0;
    }
    let sx = content.width / page.width as f32;
    let sy = content.height / page.height as f32;
    // Line boxes vary with the letters in them (a line without descenders
    // is shorter), so lines within a third of the page's usual height are
    // all given that height: body text stays one size, and only real
    // headings stand out.
    let mut heights: Vec<f32> = page.lines.iter().map(|l| l.rect.height()).collect();
    heights.sort_by(f32::total_cmp);
    let usual = heights.get(heights.len() / 2).copied().unwrap_or(0.0);
    let mut any = false;
    for line in &page.lines {
        let h = line.rect.height();
        let h = if usual > 0.0 && (h - usual).abs() <= usual / 3.0 {
            usual
        } else {
            h
        };
        let size = (h * sy).max(1.0);
        // Most words have no descender, so the middle word's bottom is the
        // baseline.
        let mut bottoms: Vec<f32> = line.words.iter().map(|w| w.rect.y1).collect();
        bottoms.sort_by(f32::total_cmp);
        let bottom = bottoms
            .get(bottoms.len() / 2)
            .copied()
            .unwrap_or(line.rect.y1);
        let baseline = bottom * sy;
        for w in &line.words {
            let text = w.text.trim();
            if text.is_empty() {
                continue;
            }
            let start = u32::try_from(content.text.len()).unwrap_or(u32::MAX);
            content.text.push_str(text);
            content.glyphs.push(Glyph {
                x: w.rect.x0 * sx,
                y: baseline,
                w: ((w.rect.x1 - w.rect.x0) * sx).max(0.1),
                size,
                start,
                len: u32::try_from(text.len()).unwrap_or(0),
                style: 0,
                mcid: NO_MCID,
            });
            any = true;
        }
    }
    any
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_lists_read_well() {
        assert_eq!(page_list(&[2]), "page 3");
        assert_eq!(page_list(&[0, 1, 2]), "pages 1 to 3");
        assert_eq!(page_list(&[0, 3]), "pages 1 and 4");
        assert_eq!(page_list(&[0, 3, 5, 6, 7]), "pages 1, 4, and 6 to 8");
    }

    #[test]
    fn words_become_glyphs_on_the_page() {
        let mut line = OcrLine::default();
        for (t, x0, x1) in [("Hello", 100.0, 200.0), ("world", 220.0, 320.0)] {
            line.push(OcrWord {
                text: t.into(),
                rect: Rect {
                    x0,
                    y0: 100.0,
                    x1,
                    y1: 140.0,
                },
                confidence: None,
            });
        }
        let page = OcrPage {
            width: 1000,
            height: 2000,
            lines: vec![line.clone()],
            turned: 0,
        };
        let mut content = PageContent {
            width: 500.0,
            height: 1000.0,
            ..PageContent::default()
        };
        assert!(place(&page, &mut content));
        assert!(content.ocr);
        assert_eq!(content.glyphs.len(), 2);
        assert_eq!(content.glyph_text(&content.glyphs[1]), "world");
        assert!((content.glyphs[0].size - 20.0).abs() < 1e-3);
        assert!((content.glyphs[0].y - 70.0).abs() < 1e-3);
        assert!((content.glyphs[1].x - 110.0).abs() < 1e-3);

        // A sideways scan read a quarter turn clockwise: the page turns
        // with it (landscape becomes portrait), and so does the way from
        // the page's own space to the words.
        let turned = OcrPage {
            width: 1000,
            height: 2000,
            lines: vec![line],
            turned: 1,
        };
        let mut content = PageContent {
            width: 1000.0,
            height: 500.0,
            to_page: Some([1.0, 0.0, 0.0, -1.0, 0.0, 500.0]),
            ..PageContent::default()
        };
        assert!(place(&turned, &mut content));
        assert_eq!((content.width, content.height), (500.0, 1000.0));
        assert!((content.glyphs[1].x - 110.0).abs() < 1e-3);
        // The page's top-left corner (0, 500 in PDF space) is now at the
        // top right.
        let (x0, y0, ..) = content.rect_on_page([0.0, 500.0, 0.0, 500.0]).unwrap();
        assert!((x0 - 500.0).abs() < 1e-3 && y0.abs() < 1e-3, "{x0} {y0}");
    }
}
