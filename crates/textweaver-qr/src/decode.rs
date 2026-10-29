//! Finding and reading QR codes in images.

use std::borrow::Cow;
use std::io::Read;
use std::path::Path;

use crate::{GrayImage, QrError};

/// The longest side the detector works on, in pixels. Larger images (a
/// phone photo, a screenshot of several monitors) are scaled down to it
/// first, which keeps reading fast and still leaves a code that fills a
/// tenth of the picture several pixels to a module.
pub const MAX_DECODE_SIDE: u32 = 4096;

/// The largest image file [`decode_file`] reads, in bytes (64 MiB).
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// Images whose longer side is shorter than this are enlarged before a
/// second try, so a code drawn one or two pixels to a module still reads.
const SMALL_SIDE: u32 = 400;

/// Reads every QR code in the PNG or JPEG file at `path`.
///
/// See [`decode_image`] for what is returned.
pub fn decode_file(path: &Path) -> Result<Vec<String>, QrError> {
    let file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(QrError::FileTooLarge);
    }
    decode_bytes(&bytes)
}

/// Reads every QR code in a PNG or JPEG file's bytes (found by their
/// signature).
///
/// See [`decode_image`] for what is returned.
pub fn decode_bytes(bytes: &[u8]) -> Result<Vec<String>, QrError> {
    let image = GrayImage::decode(bytes)?;
    decode_image(&image)
}

/// Reads every QR code in `width` by `height` pixels of RGBA, four bytes
/// a pixel, rows top to bottom: the form clipboard images take. Transparent
/// pixels read as white.
///
/// See [`decode_image`] for what is returned.
pub fn decode_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<String>, QrError> {
    let image = GrayImage::from_interleaved(width, height, 4, rgba)?;
    decode_image(&image)
}

/// Reads every QR code in a greyscale image and returns the text of each,
/// in the order found, without repeats; the list is never empty.
///
/// Dark codes on light ground and light codes on dark ground (a dark
/// theme) are both read, at any rotation. The texts are returned as found:
/// check a join link with its parser before using it.
///
/// Errors: [`QrError::NotFound`] when there is no code, and
/// [`QrError::Unreadable`] when there is one that cannot be read.
pub fn decode_image(image: &GrayImage) -> Result<Vec<String>, QrError> {
    let image: Cow<'_, GrayImage> = if image.width.max(image.height) > MAX_DECODE_SIDE {
        Cow::Owned(image.fit_within(MAX_DECODE_SIDE))
    } else {
        Cow::Borrowed(image)
    };
    let mut seen_grid = false;
    for invert in [false, true] {
        let scan = scan(&image, invert);
        seen_grid |= scan.grids > 0;
        if !scan.texts.is_empty() {
            return Ok(scan.texts);
        }
    }
    // A code drawn one or two pixels to a module, as in a small PNG.
    if image.width.max(image.height) < SMALL_SIDE {
        let large = image.enlarged(4);
        for invert in [false, true] {
            let scan = scan(&large, invert);
            seen_grid |= scan.grids > 0;
            if !scan.texts.is_empty() {
                return Ok(scan.texts);
            }
        }
    }
    Err(if seen_grid {
        QrError::Unreadable
    } else {
        QrError::NotFound
    })
}

/// What one pass of the detector found.
struct Scan {
    /// Grids that look like QR codes.
    grids: usize,
    /// The texts of those that decoded, without repeats.
    texts: Vec<String>,
}

/// One pass of the detector over `image`, with light and dark swapped
/// when `invert` is set.
fn scan(image: &GrayImage, invert: bool) -> Scan {
    let w = image.width as usize;
    let h = image.height as usize;
    let data = &image.data;
    // The detector is third-party code reading untrusted pixels: a panic
    // in it is a failed read, not a crash of the program.
    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut prepared = rqrr::PreparedImage::prepare_from_greyscale(w, h, |x, y| {
            let v = data.get(y * w + x).copied().unwrap_or(255);
            if invert { 255 - v } else { v }
        });
        let grids = prepared.detect_grids();
        let mut texts: Vec<String> = Vec::new();
        for grid in &grids {
            if let Ok((_, text)) = grid.decode()
                && !text.is_empty()
                && !texts.contains(&text)
            {
                texts.push(text);
            }
        }
        Scan {
            grids: grids.len(),
            texts,
        }
    }));
    run.unwrap_or(Scan {
        grids: 0,
        texts: Vec::new(),
    })
}
