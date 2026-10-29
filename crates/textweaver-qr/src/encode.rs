//! Encoding text to a QR code, and writing it as PNG or SVG.

use std::fmt;
use std::fmt::Write as _;

use qrcode::types::{Color, QrError as EncodeError};
use qrcode::{EcLevel, QrCode};

use crate::QrError;

/// The light border around a code, in modules, on every side. The QR
/// standard asks for four; scanners find codes without it less reliably.
pub const QUIET_ZONE: usize = 4;

/// The default size of one module in a PNG file, in pixels: a typical link
/// makes an image of about 400 pixels a side, easy to scan from a screen.
pub const DEFAULT_MODULE_PIXELS: u32 = 8;

/// The largest module size [`QrMatrix::to_png`] accepts, in pixels.
pub const MAX_MODULE_PIXELS: u32 = 64;

/// A QR code as a square of modules, without its quiet zone.
///
/// `Debug` shows only the size: the modules spell out the text, which may
/// be a secret.
#[derive(Clone, PartialEq, Eq)]
pub struct QrMatrix {
    size: usize,
    dark: Vec<bool>,
}

impl fmt::Debug for QrMatrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QrMatrix")
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

/// Encodes `text` (a join link) as a QR code, at error correction level M
/// (about 15 percent of the code may be lost), the usual choice for codes
/// shown on a screen. The smallest version that holds the text is used.
pub fn encode(text: &str) -> Result<QrMatrix, QrError> {
    if text.is_empty() {
        return Err(QrError::Empty);
    }
    let code =
        QrCode::with_error_correction_level(text.as_bytes(), EcLevel::M).map_err(|e| match e {
            EncodeError::DataTooLong => QrError::TooLong,
            // Any other failure is a version or character set the
            // encoder chose itself; say so without the text.
            other => QrError::Write(other.to_string()),
        })?;
    let size = code.width();
    let dark = code
        .to_colors()
        .into_iter()
        .map(|c| c == Color::Dark)
        .collect();
    Ok(QrMatrix { size, dark })
}

impl QrMatrix {
    /// The number of modules on each side, without the quiet zone (21 for
    /// the smallest code, 177 for the largest).
    pub fn size(&self) -> usize {
        self.size
    }

    /// Whether the module at column `x`, row `y` is dark. Outside the code
    /// (the quiet zone and beyond) every module is light.
    pub fn is_dark(&self, x: usize, y: usize) -> bool {
        x < self.size && y < self.size && self.dark[y * self.size + x]
    }

    /// The modules row by row, top to bottom, `true` for dark.
    pub fn rows(&self) -> impl Iterator<Item = &[bool]> + '_ {
        self.dark.chunks_exact(self.size.max(1))
    }

    /// The number of modules on each side with the quiet zone.
    fn full_size(&self) -> usize {
        self.size + 2 * QUIET_ZONE
    }

    /// Whether the module at `x`, `y` counted from the outer corner of the
    /// quiet zone is dark.
    fn is_dark_framed(&self, x: usize, y: usize) -> bool {
        x >= QUIET_ZONE && y >= QUIET_ZONE && self.is_dark(x - QUIET_ZONE, y - QUIET_ZONE)
    }

    /// The code as a black and white PNG file (one bit a pixel), with its
    /// quiet zone, `module_pixels` pixels to a module (1 to
    /// [`MAX_MODULE_PIXELS`]). The file carries no text chunks.
    pub fn to_png(&self, module_pixels: u32) -> Result<Vec<u8>, QrError> {
        if module_pixels == 0 || module_pixels > MAX_MODULE_PIXELS {
            return Err(QrError::Write(format!(
                "a module must be 1 to {MAX_MODULE_PIXELS} pixels wide"
            )));
        }
        let scale = module_pixels as usize;
        let side = self.full_size() * scale;
        let side_u32 =
            u32::try_from(side).map_err(|_| QrError::Write("the image is too large".into()))?;
        let row_bytes = side.div_ceil(8);
        let mut pixels = vec![0u8; row_bytes * side];
        for (y, row) in pixels.chunks_exact_mut(row_bytes).enumerate() {
            let my = y / scale;
            for x in 0..side {
                // In a one-bit greyscale PNG, 1 is white.
                if !self.is_dark_framed(x / scale, my) {
                    row[x / 8] |= 0x80 >> (x % 8);
                }
            }
        }
        let bad = |e: png::EncodingError| QrError::Write(e.to_string());
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, side_u32, side_u32);
            enc.set_color(png::ColorType::Grayscale);
            enc.set_depth(png::BitDepth::One);
            let mut w = enc.write_header().map_err(bad)?;
            w.write_image_data(&pixels).map_err(bad)?;
            w.finish().map_err(bad)?;
        }
        Ok(out)
    }

    /// The code as an SVG file, with its quiet zone, black on white, one
    /// unit to a module, drawn at [`DEFAULT_MODULE_PIXELS`] pixels to a
    /// module.
    ///
    /// `title` is the image's accessible name (its `<title>` and
    /// `aria-label`), for example "QR code for joining the shared
    /// document". It is escaped; never pass the link itself, which holds
    /// the join code.
    pub fn to_svg(&self, title: &str) -> String {
        let n = self.full_size();
        let px = n * DEFAULT_MODULE_PIXELS as usize;
        let title = escape_xml(title);
        let mut path = String::new();
        for (y, row) in self.rows().enumerate() {
            let mut x = 0;
            while x < row.len() {
                if !row[x] {
                    x += 1;
                    continue;
                }
                let start = x;
                while x < row.len() && row[x] {
                    x += 1;
                }
                let _ = write!(
                    path,
                    "M{} {}h{}v1h-{}z",
                    start + QUIET_ZONE,
                    y + QUIET_ZONE,
                    x - start,
                    x - start
                );
            }
        }
        format!(
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
                "<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\" ",
                "width=\"{px}\" height=\"{px}\" viewBox=\"0 0 {n} {n}\" ",
                "role=\"img\" aria-label=\"{title}\" shape-rendering=\"crispEdges\">\n",
                "<title>{title}</title>\n",
                "<rect width=\"{n}\" height=\"{n}\" fill=\"#ffffff\"/>\n",
                "<path fill=\"#000000\" d=\"{path}\"/>\n",
                "</svg>\n"
            ),
            px = px,
            n = n,
            title = title,
            path = path
        )
    }
}

/// `s` with the five XML special characters escaped and control
/// characters (which XML 1.0 forbids) left out.
fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_titles() {
        assert_eq!(
            escape_xml("a<b & \"c\" 'd'>\u{7}"),
            "a&lt;b &amp; &quot;c&quot; &apos;d&apos;&gt;"
        );
    }

    #[test]
    fn debug_hides_the_modules() {
        let m = encode("twjoin://broker.example:8883/ABCD-EFGH-JKMN-PQRS").unwrap();
        let shown = format!("{m:?}");
        assert!(shown.starts_with("QrMatrix { size: "), "{shown}");
        assert!(!shown.contains("true"), "{shown}");
    }

    #[test]
    fn quiet_zone_is_light() {
        let m = encode("textweaver").unwrap();
        let n = m.full_size();
        for i in 0..n {
            for j in 0..QUIET_ZONE {
                assert!(!m.is_dark_framed(i, j));
                assert!(!m.is_dark_framed(j, i));
                assert!(!m.is_dark_framed(i, n - 1 - j));
                assert!(!m.is_dark_framed(n - 1 - j, i));
            }
        }
        // The top left finder pattern's corner is dark.
        assert!(m.is_dark_framed(QUIET_ZONE, QUIET_ZONE));
    }
}
