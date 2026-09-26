//! Images referenced by a document: found on disk, sniffed, and measured.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use textweaver_text::Document;

use crate::{WriteOptions, WriteReport};

/// Images larger than this are not embedded.
const MAX_IMAGE_BYTES: u64 = 50 * 1024 * 1024;

/// Image formats the writers can embed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ImageKind {
    Png,
    Jpeg,
    Gif,
    Webp,
    Svg,
}

impl ImageKind {
    pub(crate) fn media_type(self) -> &'static str {
        match self {
            ImageKind::Png => "image/png",
            ImageKind::Jpeg => "image/jpeg",
            ImageKind::Gif => "image/gif",
            ImageKind::Webp => "image/webp",
            ImageKind::Svg => "image/svg+xml",
        }
    }

    pub(crate) fn extension(self) -> &'static str {
        match self {
            ImageKind::Png => "png",
            ImageKind::Jpeg => "jpg",
            ImageKind::Gif => "gif",
            ImageKind::Webp => "webp",
            ImageKind::Svg => "svg",
        }
    }

    fn sniff(bytes: &[u8]) -> Option<ImageKind> {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(ImageKind::Png)
        } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some(ImageKind::Jpeg)
        } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            Some(ImageKind::Gif)
        } else if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            Some(ImageKind::Webp)
        } else {
            let head = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]).to_lowercase();
            head.contains("<svg").then_some(ImageKind::Svg)
        }
    }
}

/// An image read from disk.
#[derive(Debug)]
pub(crate) struct Resource {
    pub(crate) bytes: Vec<u8>,
    pub(crate) kind: ImageKind,
    /// Pixel width and height, when the format header gives them.
    pub(crate) size: Option<(u32, u32)>,
}

/// Loads and caches the images a document references.
pub(crate) struct Resources {
    base: Option<PathBuf>,
    enabled: bool,
    cache: HashMap<String, Option<Rc<Resource>>>,
}

impl Resources {
    pub(crate) fn new(doc: &Document, options: &WriteOptions) -> Self {
        let base = options.resource_dir.clone().or_else(|| {
            doc.meta
                .path
                .as_ref()
                .and_then(|p| p.parent())
                .map(Path::to_path_buf)
        });
        Resources {
            base,
            enabled: options.embed_images,
            cache: HashMap::new(),
        }
    }

    /// The image at `src`, or `None` (with a warning) when it cannot be
    /// embedded.
    pub(crate) fn get(&mut self, src: &str, report: &mut WriteReport) -> Option<Rc<Resource>> {
        if !self.enabled || src.trim().is_empty() {
            return None;
        }
        if let Some(hit) = self.cache.get(src) {
            return hit.clone();
        }
        let loaded = self.load(src, report).map(Rc::new);
        self.cache.insert(src.to_owned(), loaded.clone());
        loaded
    }

    fn load(&self, src: &str, report: &mut WriteReport) -> Option<Resource> {
        let name = display_name(src);
        let lower = src.to_ascii_lowercase();
        if lower.starts_with("http:") || lower.starts_with("https:") || lower.starts_with("data:") {
            report.warn(format!(
                "The image {name} is not a local file, so its description was written instead."
            ));
            return None;
        }
        let rel = percent_decode(src.strip_prefix("file://").unwrap_or(src));
        let rel = rel.split(['?', '#']).next().unwrap_or("").to_owned();
        let path = Path::new(&rel);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            match &self.base {
                Some(b) => b.join(path),
                None => path.to_path_buf(),
            }
        };
        let too_big = std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_IMAGE_BYTES);
        if too_big {
            report.warn(format!(
                "The image {name} is larger than 50 megabytes, so its description was written instead."
            ));
            return None;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            report.warn(format!(
                "The image {name} was not found, so its description was written instead."
            ));
            return None;
        };
        let Some(kind) = ImageKind::sniff(&bytes) else {
            report.warn(format!(
                "The image {name} is not a PNG, JPEG, GIF, WebP, or SVG image, so its description was written instead."
            ));
            return None;
        };
        let size = dimensions(kind, &bytes);
        Some(Resource { bytes, kind, size })
    }
}

/// The file name of a source, for messages.
fn display_name(src: &str) -> String {
    let s = src.split(['?', '#']).next().unwrap_or(src);
    let name = s.rsplit(['/', '\\']).next().unwrap_or(s);
    let name = if name.is_empty() { s } else { name };
    let short: String = name.chars().take(60).collect();
    if short.is_empty() {
        "without a name".to_owned()
    } else {
        short
    }
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Pixel dimensions from the image header.
pub(crate) fn dimensions(kind: ImageKind, b: &[u8]) -> Option<(u32, u32)> {
    let be32 =
        |i: usize| -> Option<u32> { Some(u32::from_be_bytes(b.get(i..i + 4)?.try_into().ok()?)) };
    let be16 = |i: usize| -> Option<u32> {
        Some(u32::from(u16::from_be_bytes(
            b.get(i..i + 2)?.try_into().ok()?,
        )))
    };
    let le16 = |i: usize| -> Option<u32> {
        Some(u32::from(u16::from_le_bytes(
            b.get(i..i + 2)?.try_into().ok()?,
        )))
    };
    let size = match kind {
        ImageKind::Png => (be32(16)?, be32(20)?),
        ImageKind::Gif => (le16(6)?, le16(8)?),
        ImageKind::Jpeg => {
            let mut i = 2;
            loop {
                if *b.get(i)? != 0xFF {
                    return None;
                }
                let marker = *b.get(i + 1)?;
                let len = be16(i + 2)? as usize;
                // SOF0..SOF15 except DHT (C4), JPG (C8), DAC (CC).
                if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
                    break (be16(i + 7)?, be16(i + 5)?);
                }
                i += 2 + len;
            }
        }
        ImageKind::Webp | ImageKind::Svg => return None,
    };
    (size.0 > 0 && size.1 > 0).then_some(size)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A valid 2 by 1 pixel PNG (red, blue).
    pub(crate) const TINY_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x7B,
        0x40, 0xE8, 0xDD, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0xF8,
        0xCF, 0x00, 0x04, 0xFF, 0x01, 0x07, 0x00, 0x01, 0xFF, 0x3D, 0x7D, 0x8C, 0x49, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn sniffs_and_measures() {
        assert_eq!(ImageKind::sniff(TINY_PNG), Some(ImageKind::Png));
        assert_eq!(dimensions(ImageKind::Png, TINY_PNG), Some((2, 1)));
        let gif = b"GIF89a\x03\x00\x04\x00rest";
        assert_eq!(dimensions(ImageKind::Gif, gif), Some((3, 4)));
        // SOI, APP0 (length 4), SOF0 with height 5 and width 6.
        let jpeg = [
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00,
            0x05, 0x00, 0x06,
        ];
        assert_eq!(dimensions(ImageKind::Jpeg, &jpeg), Some((6, 5)));
        assert_eq!(percent_decode("my%20cat.png"), "my cat.png");
        assert_eq!(display_name("img/a%20b.png?x=1"), "a%20b.png");
    }
}
