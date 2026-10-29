//! Images on the system clipboard, for Join from a QR image (beta 5,
//! phase 7c): a screenshot of a join QR code, or one copied from a
//! browser, is read without saving it first.
//!
//! A frontend's [`Clipboard`](crate::Clipboard) returns the image with
//! [`Clipboard::read_image`](crate::Clipboard::read_image).
//! [`read_system_image`] is the system's own reader, which the textweaver
//! app and the terminal reader use:
//!
//! - **Windows:** the clipboard's `PNG` format (browsers, the Snipping
//!   Tool) as a file's bytes, else its device-independent bitmap
//!   (`CF_DIB`: 24 or 32 bits a pixel, uncompressed), as RGBA pixels.
//! - **macOS and Linux:** none yet. Reading an image there needs the
//!   `image-data` feature of `arboard`, which brings new crates; until it
//!   is approved, an image is read from a file.
//!
//! Nothing here keeps the image: it is read when asked for, and dropped
//! once the code in it is read.

/// An image read from the clipboard.
#[derive(Clone, PartialEq, Eq)]
pub enum ClipboardImage {
    /// The bytes of an image file (PNG or JPEG), as a browser copies one.
    Encoded(Vec<u8>),
    /// Pixels: `width` by `height`, four bytes (red, green, blue, alpha) a
    /// pixel, rows from the top.
    Rgba {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
        /// The pixels.
        pixels: Vec<u8>,
    },
}

impl std::fmt::Debug for ClipboardImage {
    // The size only: the pixels may hold a join code.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClipboardImage::Encoded(b) => write!(f, "ClipboardImage::Encoded({} bytes)", b.len()),
            ClipboardImage::Rgba { width, height, .. } => {
                write!(f, "ClipboardImage::Rgba({width} by {height})")
            }
        }
    }
}

/// The largest side of a bitmap read from the clipboard, in pixels.
pub const MAX_SIDE: u32 = 16_384;

/// RGBA pixels from a device-independent bitmap (a `BITMAPINFOHEADER` or
/// a later header, then the pixels): 24 or 32 bits a pixel, uncompressed
/// or with the usual bit fields, rows from the bottom or (with a negative
/// height) the top. Every pixel is opaque, since a 32-bit bitmap's fourth
/// byte is often zero. `None` for any other bitmap.
pub fn dib_to_rgba(dib: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let u32_at = |o: usize| -> Option<u32> {
        dib.get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let u16_at =
        |o: usize| -> Option<u16> { dib.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]])) };
    let header = usize::try_from(u32_at(0)?).ok()?;
    if header < 40 {
        return None;
    }
    let width = u32_at(4)? as i32;
    let height = u32_at(8)? as i32;
    let bits = u16_at(14)?;
    let compression = u32_at(16)?;
    if width <= 0 || height == 0 || !matches!(bits, 24 | 32) {
        return None;
    }
    // BI_RGB, or BI_BITFIELDS (3) and BI_ALPHABITFIELDS (6) with the
    // masks every program writes (blue, green, red from the low byte).
    if !matches!(compression, 0 | 3 | 6) {
        return None;
    }
    let w = width.unsigned_abs();
    let h = height.unsigned_abs();
    if w > MAX_SIDE || h > MAX_SIDE {
        return None;
    }
    // With a 40-byte header, bit fields follow it as three masks (four
    // with alpha); later headers hold them.
    let masks = match (header, compression) {
        (40, 3) => 12,
        (40, 6) => 16,
        _ => 0,
    };
    let start = header + masks;
    let bytes_per_pixel = usize::from(bits / 8);
    let row = (w as usize * bytes_per_pixel).div_ceil(4) * 4;
    let data = dib.get(start..start + row * h as usize)?;
    let mut out = Vec::with_capacity(w as usize * h as usize * 4);
    for y in 0..h as usize {
        // Rows run from the bottom unless the height is negative.
        let source = if height > 0 { h as usize - 1 - y } else { y };
        let line = &data[source * row..source * row + w as usize * bytes_per_pixel];
        for px in line.chunks_exact(bytes_per_pixel) {
            out.extend_from_slice(&[px[2], px[1], px[0], 255]);
        }
    }
    Some((w, h, out))
}

/// The image on the system clipboard, if there is one this build reads
/// (see the module documentation).
pub fn read_system_image() -> Option<ClipboardImage> {
    imp::read()
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod imp {
    use std::ffi::c_void;

    use super::ClipboardImage;

    windows_core::link!("user32.dll" "system" fn OpenClipboard(owner: isize) -> i32);
    windows_core::link!("user32.dll" "system" fn CloseClipboard() -> i32);
    windows_core::link!("user32.dll" "system" fn GetClipboardData(format: u32) -> isize);
    windows_core::link!("user32.dll" "system" fn IsClipboardFormatAvailable(format: u32) -> i32);
    windows_core::link!("user32.dll" "system" fn RegisterClipboardFormatW(name: *const u16) -> u32);
    windows_core::link!("kernel32.dll" "system" fn GlobalLock(mem: isize) -> *mut c_void);
    windows_core::link!("kernel32.dll" "system" fn GlobalUnlock(mem: isize) -> i32);
    windows_core::link!("kernel32.dll" "system" fn GlobalSize(mem: isize) -> usize);

    /// A device-independent bitmap.
    const CF_DIB: u32 = 8;

    /// The largest clipboard block read (256 MiB).
    const MAX_BLOCK: usize = 256 * 1024 * 1024;

    pub fn read() -> Option<ClipboardImage> {
        let wide: Vec<u16> = "PNG".encode_utf16().chain([0]).collect();
        // SAFETY: `wide` is a NUL-terminated UTF-16 string that outlives
        // the call.
        let png = unsafe { RegisterClipboardFormatW(wide.as_ptr()) };
        // SAFETY: the clipboard is opened with no owner window and closed
        // before returning; each handle is read only while it is open.
        unsafe {
            let mut opened = false;
            for _ in 0..5 {
                if OpenClipboard(0) != 0 {
                    opened = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            if !opened {
                return None;
            }
            let image = (png != 0)
                .then(|| block(png))
                .flatten()
                .map(ClipboardImage::Encoded)
                .or_else(|| {
                    block(CF_DIB).and_then(|b| super::dib_to_rgba(&b)).map(
                        |(width, height, pixels)| ClipboardImage::Rgba {
                            width,
                            height,
                            pixels,
                        },
                    )
                });
            CloseClipboard();
            image
        }
    }

    /// A copy of the clipboard's block in `format`, while it is open.
    ///
    /// # Safety
    ///
    /// The clipboard must be open.
    unsafe fn block(format: u32) -> Option<Vec<u8>> {
        // SAFETY: the caller holds the clipboard open; the block is locked
        // while it is copied and its size is the system's.
        unsafe {
            if IsClipboardFormatAvailable(format) == 0 {
                return None;
            }
            let mem = GetClipboardData(format);
            if mem == 0 {
                return None;
            }
            let size = GlobalSize(mem);
            if size == 0 || size > MAX_BLOCK {
                return None;
            }
            let ptr = GlobalLock(mem).cast::<u8>();
            if ptr.is_null() {
                return None;
            }
            let bytes = std::slice::from_raw_parts(ptr, size).to_vec();
            GlobalUnlock(mem);
            Some(bytes)
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::ClipboardImage;

    pub fn read() -> Option<ClipboardImage> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 40-byte header for a `w` by `h` bitmap of `bits` a pixel.
    fn header(w: i32, h: i32, bits: u16, compression: u32) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&40u32.to_le_bytes());
        b.extend_from_slice(&w.to_le_bytes());
        b.extend_from_slice(&h.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&bits.to_le_bytes());
        b.extend_from_slice(&compression.to_le_bytes());
        b.resize(40, 0);
        b
    }

    #[test]
    fn a_bottom_up_24_bit_bitmap_reads_top_row_first_and_opaque() {
        // 2 by 2: bottom row blue, white; top row black, red. Rows pad to
        // four bytes (6 bytes of pixels, 2 of padding).
        let mut dib = header(2, 2, 24, 0);
        dib.extend_from_slice(&[255, 0, 0, 255, 255, 255, 0, 0]);
        dib.extend_from_slice(&[0, 0, 0, 0, 0, 255, 0, 0]);
        let (w, h, px) = dib_to_rgba(&dib).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(
            px,
            [
                0, 0, 0, 255, 255, 0, 0, 255, // top: black, red
                0, 0, 255, 255, 255, 255, 255, 255, // bottom: blue, white
            ]
        );
    }

    #[test]
    fn a_top_down_32_bit_bitmap_with_bit_fields_reads_in_order() {
        let mut dib = header(1, -2, 32, 3);
        // The three masks a 40-byte header carries with BI_BITFIELDS.
        dib.extend_from_slice(&[0, 0, 255, 0, 0, 255, 0, 0, 255, 0, 0, 0]);
        // Alpha zero, as many programs write it: read as opaque.
        dib.extend_from_slice(&[10, 20, 30, 0, 40, 50, 60, 0]);
        let (w, h, px) = dib_to_rgba(&dib).unwrap();
        assert_eq!((w, h), (1, 2));
        assert_eq!(px, [30, 20, 10, 255, 60, 50, 40, 255]);
    }

    #[test]
    fn other_bitmaps_and_short_data_are_refused() {
        assert_eq!(dib_to_rgba(&header(2, 2, 8, 0)), None, "palette");
        assert_eq!(dib_to_rgba(&header(2, 2, 24, 1)), None, "RLE");
        assert_eq!(dib_to_rgba(&header(2, 2, 24, 0)), None, "no pixels");
        assert_eq!(dib_to_rgba(&header(0, 2, 24, 0)), None, "no width");
        assert_eq!(
            dib_to_rgba(&header(MAX_SIDE as i32 + 1, 1, 24, 0)),
            None,
            "too wide"
        );
        assert_eq!(dib_to_rgba(&[1, 2, 3]), None);
    }

    #[test]
    fn the_debug_form_gives_the_size_only() {
        let image = ClipboardImage::Rgba {
            width: 3,
            height: 4,
            pixels: vec![0; 48],
        };
        assert_eq!(format!("{image:?}"), "ClipboardImage::Rgba(3 by 4)");
    }
}
