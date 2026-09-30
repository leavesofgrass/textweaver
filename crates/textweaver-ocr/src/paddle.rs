//! PaddleOCR's PP-OCRv5 Latin recognition model on RTen (experimental).
//!
//! ocrs finds the words and lines; each line is cut out as ocrs would for
//! its own recognizer, scaled to the 48-pixel height the Paddle model was
//! trained on, and read with greedy CTC decoding over the model's alphabet
//! (836 characters, with accented Latin letters), which comes from the
//! model's configuration file.

use std::sync::{Arc, Mutex};

use rten_imageproc::BoundingRect;
use rten_tensor::prelude::*;
use rten_tensor::{NdTensor, Tensor};

use crate::image::GrayImage;
use crate::models::{self, ModelStatus};
use crate::ocrs_engine::{self, load_model};
use crate::{OcrError, OcrLine, OcrPage, OcrWord, Rect};

/// The model's input height.
const HEIGHT: usize = 48;

/// The widest line image given to the model.
const MAX_WIDTH: usize = 3_200;

/// The loaded recognizer.
pub(crate) struct Paddle {
    model: rten::Model,
    /// Index `i` of the model's output is `alphabet[i - 1]` (0 is blank).
    alphabet: Vec<String>,
}

static ENGINE: Mutex<Option<Arc<Paddle>>> = Mutex::new(None);

fn engine_error(e: impl std::fmt::Display) -> OcrError {
    OcrError::Engine(e.to_string())
}

/// The Paddle recognizer (loaded on first use).
pub(crate) fn engine() -> Result<Arc<Paddle>, OcrError> {
    let mut slot = ENGINE.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(e) = slot.as_ref() {
        return Ok(Arc::clone(e));
    }
    if models::PADDLE_LATIN.status() != ModelStatus::Present {
        return Err(OcrError::ModelsMissing(models::PADDLE_LATIN.title));
    }
    let mut files = models::PADDLE_LATIN.load_verified()?.into_iter();
    let (Some(model), Some(config)) = (files.next(), files.next()) else {
        return Err(OcrError::ModelsMissing(models::PADDLE_LATIN.title));
    };
    let alphabet = parse_alphabet(&String::from_utf8_lossy(&config));
    if alphabet.is_empty() {
        return Err(OcrError::Engine(
            "the PaddleOCR configuration has no alphabet".into(),
        ));
    }
    let p = Arc::new(Paddle {
        model: load_model(model)?,
        alphabet,
    });
    *slot = Some(Arc::clone(&p));
    Ok(p)
}

/// The `character_dict` list from the model's YAML configuration (plain,
/// single-quoted, or double-quoted scalars, one per `- ` item).
pub(crate) fn parse_alphabet(yaml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in yaml.lines() {
        let trimmed = line.trim_start();
        if !inside {
            inside = trimmed.starts_with("character_dict:");
            continue;
        }
        let Some(item) = trimmed.strip_prefix("- ") else {
            if trimmed == "-" {
                continue;
            }
            break;
        };
        let item = item.trim_end_matches(['\r']);
        let value = if let Some(s) = item.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
            s.replace("''", "'")
        } else if let Some(s) = item.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
            s.replace("\\\"", "\"").replace("\\\\", "\\")
        } else {
            item.to_owned()
        };
        out.push(value);
    }
    out
}

/// Scales a line image (`h` rows of `w` floats) to [`HEIGHT`] rows,
/// keeping its shape, and returns the new width and the pixels.
fn resize_line(src: &[f32], w: usize, h: usize) -> (usize, Vec<f32>) {
    let nw = ((w * HEIGHT) as f32 / h.max(1) as f32).round() as usize;
    let nw = nw.clamp(8, MAX_WIDTH);
    let mut out = vec![0.0f32; nw * HEIGHT];
    for y in 0..HEIGHT {
        let sy = ((y as f32 + 0.5) * h as f32 / HEIGHT as f32 - 0.5).clamp(0.0, (h - 1) as f32);
        let (y0, fy) = (sy.floor() as usize, sy.fract());
        let y1 = (y0 + 1).min(h - 1);
        for x in 0..nw {
            let sx = ((x as f32 + 0.5) * w as f32 / nw as f32 - 0.5).clamp(0.0, (w - 1) as f32);
            let (x0, fx) = (sx.floor() as usize, sx.fract());
            let x1 = (x0 + 1).min(w - 1);
            let top = src[y0 * w + x0] * (1.0 - fx) + src[y0 * w + x1] * fx;
            let bottom = src[y1 * w + x0] * (1.0 - fx) + src[y1 * w + x1] * fx;
            out[y * nw + x] = top * (1.0 - fy) + bottom * fy;
        }
    }
    (nw, out)
}

impl Paddle {
    /// Reads one line image (values from -0.5 for black to 0.5 for white,
    /// `h` rows of `w`): characters with the fraction of the line's width
    /// at which each was read.
    fn read_line(&self, src: &[f32], w: usize, h: usize) -> Result<Vec<(String, f32)>, OcrError> {
        if w == 0 || h == 0 {
            return Ok(Vec::new());
        }
        let (nw, pixels) = resize_line(src, w, h);
        // Paddle's normalization maps pixels to -1..1; ocrs gives -0.5..0.5.
        let mut data = Vec::with_capacity(3 * nw * HEIGHT);
        for _ in 0..3 {
            data.extend(pixels.iter().map(|v| v * 2.0));
        }
        let input = NdTensor::from_data([1, 3, HEIGHT, nw], data);
        let output: Tensor<f32> = self
            .model
            .run_one(input.view().into(), None)
            .map_err(engine_error)?
            .try_into()
            .map_err(engine_error)?;
        let shape = output.shape().to_vec();
        let [_, steps, classes] = shape[..] else {
            return Err(OcrError::Engine(format!(
                "unexpected PaddleOCR output shape {shape:?}"
            )));
        };
        let data = output.to_vec();
        let space = classes == self.alphabet.len() + 2;
        let mut out = Vec::new();
        let mut last = 0usize;
        for t in 0..steps {
            let row = &data[t * classes..(t + 1) * classes];
            let best = row
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map_or(0, |(i, _)| i);
            if best != 0 && best != last {
                let ch = if space && best == classes - 1 {
                    Some(" ".to_owned())
                } else {
                    self.alphabet.get(best - 1).cloned()
                };
                if let Some(ch) = ch {
                    out.push((ch, (t as f32 + 0.5) / steps as f32));
                }
            }
            last = best;
        }
        Ok(out)
    }
}

/// Recognizes `image`: ocrs finds the lines, Paddle reads them.
pub(crate) fn recognize(image: &GrayImage) -> Result<OcrPage, OcrError> {
    let ocrs = ocrs_engine::engine()?;
    let paddle = engine()?;
    let source = ocrs::ImageSource::from_bytes(&image.data, (image.width, image.height))
        .map_err(engine_error)?;
    let input = ocrs.prepare_input(source).map_err(engine_error)?;
    let words = ocrs.detect_words(&input).map_err(engine_error)?;
    let lines = ocrs.find_text_lines(&input, &words);
    let mut page = OcrPage {
        width: image.width,
        height: image.height,
        lines: Vec::new(),
        turned: 0,
    };
    for line in &lines {
        let Some(bounds) = line
            .iter()
            .map(|r| ocrs_engine::rect(r.bounding_rect().integral_bounding_rect()))
            .reduce(Rect::union)
        else {
            continue;
        };
        let img = ocrs
            .prepare_recognition_input(&input, line)
            .map_err(engine_error)?;
        let [h, w] = img.shape();
        let pixels = img.to_vec();
        let chars = paddle.read_line(&pixels, w, h)?;
        page.lines.extend(words_of(&chars, bounds));
    }
    Ok(page)
}

/// Splits a line's characters into words at spaces, placing each word by
/// where its characters were read.
fn words_of(chars: &[(String, f32)], bounds: Rect) -> Option<OcrLine> {
    let width = bounds.x1 - bounds.x0;
    let mut line = OcrLine::default();
    let mut word = String::new();
    let mut span: Option<(f32, f32)> = None;
    let flush = |word: &mut String, span: &mut Option<(f32, f32)>, line: &mut OcrLine| {
        if let Some((a, b)) = span.take()
            && !word.trim().is_empty()
        {
            // Half a character's width either side of the first and last.
            let pad = 0.01;
            line.push(OcrWord {
                text: std::mem::take(word),
                rect: Rect {
                    x0: bounds.x0 + width * (a - pad).max(0.0),
                    y0: bounds.y0,
                    x1: bounds.x0 + width * (b + pad).min(1.0),
                    y1: bounds.y1,
                },
                confidence: None,
            });
        }
        word.clear();
    };
    for (ch, at) in chars {
        if ch == " " {
            flush(&mut word, &mut span, &mut line);
            continue;
        }
        word.push_str(ch);
        span = Some(span.map_or((*at, *at), |(a, _)| (a, *at)));
    }
    flush(&mut word, &mut span, &mut line);
    (!line.words.is_empty()).then_some(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alphabet_items_unquote() {
        let yaml = "PostProcess:\n  name: CTCLabelDecode\n  character_dict:\n  - '0'\n  - A\n  - ''''\n  - '\"'\n  - é\nOther: 1\n";
        assert_eq!(parse_alphabet(yaml), vec!["0", "A", "'", "\"", "é"]);
    }

    #[test]
    fn line_resize_keeps_the_shape() {
        let src = vec![0.5f32; 96 * 24];
        let (w, px) = resize_line(&src, 96, 24);
        assert_eq!(w, 192);
        assert_eq!(px.len(), 192 * HEIGHT);
        assert!(px.iter().all(|v| (*v - 0.5).abs() < 1e-6));
    }

    #[test]
    fn words_split_at_spaces() {
        let chars: Vec<(String, f32)> = [("a", 0.1), ("b", 0.2), (" ", 0.3), ("c", 0.8)]
            .iter()
            .map(|(c, t)| ((*c).to_owned(), *t))
            .collect();
        let bounds = Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 10.0,
        };
        let line = words_of(&chars, bounds).unwrap();
        assert_eq!(line.text(), "ab c");
        assert!(line.words[1].rect.x0 > 70.0);
    }
}
