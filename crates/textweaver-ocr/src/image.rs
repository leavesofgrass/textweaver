//! Greyscale page images: decoding PNG and JPEG files, converting colour,
//! scaling, and encoding PNG (for Tesseract).
//!
//! Every decoder runs with limits, so a hostile file cannot claim a huge
//! canvas and exhaust memory: images wider or taller than
//! [`MAX_SIDE`] pixels, or larger than [`MAX_PIXELS`] in all, are refused.

use std::io::Cursor;

use crate::OcrError;

/// The widest or tallest image accepted (pixels).
pub const MAX_SIDE: u32 = 20_000;

/// The most pixels an image may have (a letter page at 1,200 dots per inch
/// is about 135 million; 600 is plenty for OCR).
pub const MAX_PIXELS: u64 = 150_000_000;

/// An 8-bit greyscale image, rows top to bottom (0 is black).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrayImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height` bytes.
    pub data: Vec<u8>,
}

/// The pixel count of a `width` by `height` image, if it is within the
/// limits.
fn checked_pixels(width: u32, height: u32) -> Result<usize, OcrError> {
    let n = u64::from(width) * u64::from(height);
    if width == 0 || height == 0 {
        return Err(OcrError::Image("the image is empty".into()));
    }
    if width > MAX_SIDE || height > MAX_SIDE || n > MAX_PIXELS {
        return Err(OcrError::Image(format!(
            "the image is too large to recognize ({width} by {height} pixels)"
        )));
    }
    usize::try_from(n).map_err(|_| OcrError::Image("the image is too large".into()))
}

/// Rec. 601 luma of an RGB pixel.
fn luma(r: u8, g: u8, b: u8) -> u8 {
    let y = (u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114 + 500) / 1000;
    u8::try_from(y.min(255)).unwrap_or(u8::MAX)
}

impl GrayImage {
    /// An image from greyscale bytes (`width * height` of them).
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Result<Self, OcrError> {
        let n = checked_pixels(width, height)?;
        if data.len() != n {
            return Err(OcrError::Image(format!(
                "expected {n} bytes of pixels, found {}",
                data.len()
            )));
        }
        Ok(GrayImage {
            width,
            height,
            data,
        })
    }

    /// An image from interleaved samples with `channels` per pixel: 1
    /// (grey), 2 (grey and alpha), 3 (RGB), or 4 (RGBA). Transparent
    /// pixels read as white paper.
    pub fn from_interleaved(
        width: u32,
        height: u32,
        channels: usize,
        samples: &[u8],
    ) -> Result<Self, OcrError> {
        let n = checked_pixels(width, height)?;
        if channels == 0 || channels > 4 || samples.len() < n * channels {
            return Err(OcrError::Image("the image data is incomplete".into()));
        }
        let over_white = |v: u8, a: u8| -> u8 {
            let v = u32::from(v) * u32::from(a) + 255 * (255 - u32::from(a));
            u8::try_from(v / 255).unwrap_or(u8::MAX)
        };
        let data = samples
            .chunks_exact(channels)
            .take(n)
            .map(|p| match p {
                [g] => *g,
                [g, a] => over_white(*g, *a),
                [r, g, b] => luma(*r, *g, *b),
                [r, g, b, a] => over_white(luma(*r, *g, *b), *a),
                _ => 255,
            })
            .collect();
        Ok(GrayImage {
            width,
            height,
            data,
        })
    }

    /// Decodes a PNG or JPEG file (found by its signature).
    pub fn decode(bytes: &[u8]) -> Result<Self, OcrError> {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            decode_png(bytes)
        } else if bytes.starts_with(&[0xFF, 0xD8]) {
            decode_jpeg(bytes)
        } else {
            Err(OcrError::Image(
                "only PNG and JPEG images can be recognized".into(),
            ))
        }
    }

    /// The image scaled down (box filter) so that its longer side is at
    /// most `max_side` pixels; unchanged when already small enough.
    pub fn fit_within(&self, max_side: u32) -> GrayImage {
        let long = self.width.max(self.height);
        if long <= max_side || max_side == 0 {
            return self.clone();
        }
        let scale = f64::from(max_side) / f64::from(long);
        let w = ((f64::from(self.width) * scale).round() as u32).max(1);
        let h = ((f64::from(self.height) * scale).round() as u32).max(1);
        self.resize(w, h)
    }

    /// The image scaled up by `factor` (bilinear): small scans read better
    /// larger.
    pub fn enlarged(&self, factor: u32) -> GrayImage {
        let factor = factor.max(1);
        let w = self.width.saturating_mul(factor).min(MAX_SIDE);
        let h = self.height.saturating_mul(factor).min(MAX_SIDE);
        let (sw, sh) = (self.width as usize, self.height as usize);
        let (dw, dh) = (w as usize, h as usize);
        let mut out = Vec::with_capacity(dw * dh);
        for y in 0..dh {
            let sy = ((y as f32 + 0.5) * sh as f32 / dh as f32 - 0.5).clamp(0.0, (sh - 1) as f32);
            let (y0, fy) = (sy as usize, sy.fract());
            let y1 = (y0 + 1).min(sh - 1);
            for x in 0..dw {
                let sx =
                    ((x as f32 + 0.5) * sw as f32 / dw as f32 - 0.5).clamp(0.0, (sw - 1) as f32);
                let (x0, fx) = (sx as usize, sx.fract());
                let x1 = (x0 + 1).min(sw - 1);
                let p = |yy: usize, xx: usize| f32::from(self.data[yy * sw + xx]);
                let top = p(y0, x0) * (1.0 - fx) + p(y0, x1) * fx;
                let bottom = p(y1, x0) * (1.0 - fx) + p(y1, x1) * fx;
                out.push((top * (1.0 - fy) + bottom * fy).round().clamp(0.0, 255.0) as u8);
            }
        }
        GrayImage {
            width: w,
            height: h,
            data: out,
        }
    }

    /// The image resampled to `w` by `h` (area averaging when shrinking,
    /// nearest pixel when growing).
    pub fn resize(&self, w: u32, h: u32) -> GrayImage {
        let (sw, sh) = (self.width as usize, self.height as usize);
        let (w, h) = (w.max(1), h.max(1));
        let (dw, dh) = (w as usize, h as usize);
        let mut out = vec![255u8; dw * dh];
        for y in 0..dh {
            let y0 = y * sh / dh;
            let y1 = ((y + 1) * sh / dh).max(y0 + 1).min(sh);
            for x in 0..dw {
                let x0 = x * sw / dw;
                let x1 = ((x + 1) * sw / dw).max(x0 + 1).min(sw);
                let mut sum = 0u64;
                for yy in y0..y1 {
                    let row = &self.data[yy * sw + x0..yy * sw + x1];
                    sum += row.iter().map(|&v| u64::from(v)).sum::<u64>();
                }
                let count = ((y1 - y0) * (x1 - x0)) as u64;
                out[y * dw + x] = u8::try_from(sum / count.max(1)).unwrap_or(u8::MAX);
            }
        }
        GrayImage {
            width: w,
            height: h,
            data: out,
        }
    }

    /// The image turned by a multiple of 90 degrees clockwise.
    pub fn rotate(&self, quarter_turns: u8) -> GrayImage {
        let (w, h) = (self.width as usize, self.height as usize);
        match quarter_turns % 4 {
            0 => self.clone(),
            2 => GrayImage {
                width: self.width,
                height: self.height,
                data: self.data.iter().rev().copied().collect(),
            },
            turns => {
                let mut data = vec![255u8; w * h];
                for y in 0..h {
                    for x in 0..w {
                        // New image is h wide and w tall.
                        let (nx, ny) = if turns == 1 {
                            (h - 1 - y, x)
                        } else {
                            (y, w - 1 - x)
                        };
                        data[ny * h + nx] = self.data[y * w + x];
                    }
                }
                GrayImage {
                    width: self.height,
                    height: self.width,
                    data,
                }
            }
        }
    }

    /// The image mirrored left to right (`horizontal`) or top to bottom.
    pub fn flip(&self, horizontal: bool) -> GrayImage {
        let w = self.width as usize;
        let data = if horizontal {
            self.data
                .chunks_exact(w)
                .flat_map(|row| row.iter().rev().copied())
                .collect()
        } else {
            self.data
                .chunks_exact(w)
                .rev()
                .flat_map(|row| row.iter().copied())
                .collect()
        };
        GrayImage {
            width: self.width,
            height: self.height,
            data,
        }
    }

    /// The image as a greyscale PNG file.
    pub fn to_png(&self) -> Result<Vec<u8>, OcrError> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, self.width, self.height);
            enc.set_color(png::ColorType::Grayscale);
            enc.set_depth(png::BitDepth::Eight);
            enc.set_compression(png::Compression::Fast);
            let mut w = enc
                .write_header()
                .map_err(|e| OcrError::Image(format!("cannot encode PNG: {e}")))?;
            w.write_image_data(&self.data)
                .map_err(|e| OcrError::Image(format!("cannot encode PNG: {e}")))?;
        }
        Ok(out)
    }
}

fn decode_png(bytes: &[u8]) -> Result<GrayImage, OcrError> {
    let limits = png::Limits {
        bytes: usize::try_from(MAX_PIXELS * 8).unwrap_or(usize::MAX),
    };
    let mut dec = png::Decoder::new_with_limits(Cursor::new(bytes), limits);
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let bad = |e: png::DecodingError| OcrError::Image(format!("not a readable PNG image: {e}"));
    let mut reader = dec.read_info().map_err(bad)?;
    let (w, h) = {
        let info = reader.info();
        (info.width, info.height)
    };
    checked_pixels(w, h)?;
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| OcrError::Image("the PNG image is too large".into()))?;
    let mut buf = vec![0; size];
    let frame = reader.next_frame(&mut buf).map_err(bad)?;
    let channels = frame.color_type.samples();
    GrayImage::from_interleaved(
        frame.width,
        frame.height,
        channels,
        &buf[..frame.buffer_size()],
    )
}

fn decode_jpeg(bytes: &[u8]) -> Result<GrayImage, OcrError> {
    use zune_jpeg::zune_core::colorspace::ColorSpace;
    use zune_jpeg::zune_core::options::DecoderOptions;
    let options = DecoderOptions::default()
        .jpeg_set_out_colorspace(ColorSpace::Luma)
        .set_max_width(MAX_SIDE as usize)
        .set_max_height(MAX_SIDE as usize);
    let run = std::panic::catch_unwind(|| {
        let mut dec = zune_jpeg::JpegDecoder::new_with_options(Cursor::new(bytes), options);
        let pixels = dec.decode().map_err(|e| format!("{e:?}"))?;
        let (w, h) = dec.dimensions().ok_or("no dimensions")?;
        Ok::<_, String>((pixels, w, h))
    });
    let (pixels, w, h) = match run {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => return Err(OcrError::Image(format!("not a readable JPEG image: {e}"))),
        Err(_) => return Err(OcrError::Image("not a readable JPEG image".into())),
    };
    let (w, h) = (
        u32::try_from(w).unwrap_or(u32::MAX),
        u32::try_from(h).unwrap_or(u32::MAX),
    );
    let n = checked_pixels(w, h)?;
    let channels = pixels.len() / n.max(1);
    GrayImage::from_interleaved(w, h, channels.clamp(1, 4), &pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> GrayImage {
        // 3 wide, 2 tall: 0 1 2 / 3 4 5
        GrayImage::new(3, 2, vec![0, 1, 2, 3, 4, 5]).unwrap()
    }

    #[test]
    fn rotates_and_flips() {
        let r = sample().rotate(1);
        assert_eq!((r.width, r.height), (2, 3));
        assert_eq!(r.data, vec![3, 0, 4, 1, 5, 2]);
        assert_eq!(sample().rotate(3).data, vec![2, 5, 1, 4, 0, 3]);
        assert_eq!(sample().rotate(2).data, vec![5, 4, 3, 2, 1, 0]);
        assert_eq!(sample().flip(true).data, vec![2, 1, 0, 5, 4, 3]);
        assert_eq!(sample().flip(false).data, vec![3, 4, 5, 0, 1, 2]);
    }

    #[test]
    fn png_round_trips_and_limits_hold() {
        let img = sample();
        let png = img.to_png().unwrap();
        assert_eq!(GrayImage::decode(&png).unwrap(), img);
        assert!(GrayImage::new(MAX_SIDE + 1, 1, vec![0; MAX_SIDE as usize + 1]).is_err());
        assert!(GrayImage::new(0, 0, vec![]).is_err());
        assert!(GrayImage::decode(b"GIF89a").is_err());
        assert!(GrayImage::decode(&[0xFF, 0xD8, 0xFF, 0x00]).is_err());
    }

    #[test]
    fn colour_and_alpha_become_grey_on_white() {
        let img = GrayImage::from_interleaved(2, 1, 4, &[255, 0, 0, 255, 0, 0, 0, 0]).unwrap();
        assert_eq!(img.data, vec![76, 255]);
    }

    #[test]
    fn shrinks_by_averaging() {
        let img = GrayImage::new(4, 2, vec![0, 255, 0, 255, 0, 255, 0, 255]).unwrap();
        let small = img.fit_within(2);
        assert_eq!((small.width, small.height), (2, 1));
        assert_eq!(small.data, vec![127, 127]);
        assert_eq!(img.fit_within(10), img);
    }
}
