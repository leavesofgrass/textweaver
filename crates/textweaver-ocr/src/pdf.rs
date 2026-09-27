//! Images of PDF pages, for OCR.
//!
//! A scanned page usually holds one image and nothing else. That image is
//! taken as it is ([`PageImageKind::Extracted`]): hayro's interpreter
//! decodes it, whatever its filter (JPEG, JPEG 2000, JBIG2, CCITT fax,
//! Flate) and colour space, and no page is rendered. A page with anything
//! more (several images, drawings, a rotated or mirrored image) is
//! rendered with hayro instead ([`PageImageKind::Rendered`]).
//!
//! Both steps run inside `catch_unwind`, so a malformed page is an error
//! for that page, never a crash.

use hayro::hayro_interpret::font::Glyph;
use hayro::hayro_interpret::{
    BlendMode, ClipPath, Context, Device, GlyphDrawMode, Image, ImageData, InterpreterCache,
    InterpreterSettings, Paint, PathDrawMode, SoftMask, TransformExt, interpret_page,
};
use hayro::hayro_syntax::Pdf;
use hayro::{RenderCache, RenderSettings, render};
use kurbo::{Affine, BezPath, Rect as KRect};

use crate::OcrError;
use crate::image::GrayImage;

/// How a page's image was obtained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageImageKind {
    /// The page's one scanned image, taken without rendering.
    Extracted,
    /// The page, rendered.
    Rendered,
}

/// A PDF opened for page images.
pub struct PdfPages {
    pdf: Pdf,
}

/// The resolution pages are rendered at, in dots per inch.
pub const RENDER_DPI: f32 = 200.0;

/// The longest side of a rendered page, in pixels.
pub const MAX_RENDER_SIDE: f32 = 3_400.0;

impl PdfPages {
    /// Opens a PDF (encrypted files with an empty password work too).
    pub fn open(bytes: Vec<u8>) -> Result<Self, OcrError> {
        let pdf = std::panic::catch_unwind(|| Pdf::new(bytes))
            .map_err(|_| OcrError::Image("the PDF could not be opened for recognition".into()))?
            .map_err(|e| {
                OcrError::Image(format!(
                    "the PDF could not be opened for recognition ({e:?})"
                ))
            })?;
        Ok(PdfPages { pdf })
    }

    /// The number of pages.
    pub fn len(&self) -> usize {
        self.pdf.pages().len()
    }

    /// True when the PDF has no pages.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The image of page `index` (from 0): its scanned image when that is
    /// all it holds, else the page rendered at [`RENDER_DPI`].
    pub fn page_image(&self, index: usize) -> Result<(GrayImage, PageImageKind), OcrError> {
        let pages = self.pdf.pages();
        let page = pages
            .get(index)
            .ok_or_else(|| OcrError::Image(format!("the PDF has no page {}", index + 1)))?;
        let extracted =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| single_image(page)))
                .ok()
                .flatten();
        if let Some(img) = extracted {
            return Ok((img, PageImageKind::Extracted));
        }
        let rendered = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| render_page(page)))
            .map_err(|_| OcrError::Image(format!("page {} could not be drawn", index + 1)))??;
        Ok((rendered, PageImageKind::Rendered))
    }
}

/// Counts what a page draws, and decodes its first image.
#[derive(Default)]
struct Probe {
    images: usize,
    glyphs: usize,
    paths: usize,
    first: Option<(GrayImage, Affine)>,
}

impl<'a> Device<'a> for Probe {
    fn set_soft_mask(&mut self, _: Option<SoftMask<'a>>) {}
    fn set_blend_mode(&mut self, _: BlendMode) {}
    fn draw_path(&mut self, _: &BezPath, _: Affine, _: &Paint<'a>, _: &PathDrawMode) {
        self.paths += 1;
    }
    fn push_clip_path(&mut self, _: &ClipPath) {}
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'a>>, _: BlendMode) {}
    fn draw_glyph(
        &mut self,
        _: &Glyph<'a>,
        _: Affine,
        _: Affine,
        _: &Paint<'a>,
        _: &GlyphDrawMode,
    ) {
        self.glyphs += 1;
    }
    fn draw_image(&mut self, image: Image<'a, '_>, transform: Affine) {
        self.images += 1;
        if self.images > 1 {
            // Several images: the page is rendered, so the first is dropped.
            self.first = None;
            return;
        }
        if let Image::Raster(r) = image {
            let mut decoded = None;
            r.with_rgba(
                |data, _alpha| {
                    decoded = match data {
                        ImageData::Luma(l) => {
                            GrayImage::from_interleaved(l.width, l.height, 1, &l.data).ok()
                        }
                        ImageData::Rgb(c) => {
                            GrayImage::from_interleaved(c.width, c.height, 3, &c.data).ok()
                        }
                    };
                },
                None,
            );
            self.first = decoded.map(|d| (d, transform));
        }
    }
    fn pop_clip_path(&mut self) {}
    fn pop_transparency_group(&mut self) {}
}

/// The page's one image, when the page is a plain scan: one upright image
/// covering most of the page, no text, at most a background rectangle.
fn single_image(page: &hayro::hayro_syntax::page::Page<'_>) -> Option<GrayImage> {
    let (w, h) = page.render_dimensions();
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let initial = page.initial_transform(true).to_kurbo();
    let cache = InterpreterCache::new();
    let mut ctx = Context::new(
        initial,
        KRect::new(0.0, 0.0, f64::from(w), f64::from(h)),
        &cache,
        page.xref(),
        InterpreterSettings::default(),
    );
    let mut probe = Probe::default();
    interpret_page(page, &mut ctx, &mut probe);
    if probe.images != 1 || probe.glyphs > 0 || probe.paths > 2 {
        return None;
    }
    let (img, t) = probe.first?;
    // A tiny image (a thumbnail, a logo) is not a scan worth reading.
    if img.width < 200 || img.height < 200 {
        return None;
    }
    // hayro gives the transform from image pixels (row 0 at the top) to
    // the page as displayed (y down): an upright image has positive scales
    // and no shear. A mirrored image is flipped back; a turned one (a
    // rotated page, a sideways scan) is left to the renderer.
    let [a, b, c, d, _, _] = t.as_coeffs();
    let axis_aligned = b.abs() <= 1e-6 * a.abs() && c.abs() <= 1e-6 * d.abs();
    let area = (a * d - b * c).abs() * f64::from(img.width) * f64::from(img.height);
    if !axis_aligned || area < 0.5 * f64::from(w) * f64::from(h) {
        return None;
    }
    let img = if a < 0.0 { img.flip(true) } else { img };
    Some(if d < 0.0 { img.flip(false) } else { img })
}

/// The page rendered at [`RENDER_DPI`] (less for very large pages).
fn render_page(page: &hayro::hayro_syntax::page::Page<'_>) -> Result<GrayImage, OcrError> {
    let (w, h) = page.render_dimensions();
    if !(w > 0.0 && h > 0.0) {
        return Err(OcrError::Image("the page has no size".into()));
    }
    let mut scale = RENDER_DPI / 72.0;
    let long = w.max(h) * scale;
    if long > MAX_RENDER_SIDE {
        scale *= MAX_RENDER_SIDE / long;
    }
    let settings = RenderSettings {
        x_scale: scale,
        y_scale: scale,
        bg_color: hayro::vello_cpu::color::palette::css::WHITE,
        ..RenderSettings::default()
    };
    let cache = RenderCache::new();
    let pixmap = render(page, &cache, &InterpreterSettings::default(), &settings);
    let (pw, ph) = (u32::from(pixmap.width()), u32::from(pixmap.height()));
    let rgba: Vec<u8> = pixmap
        .take_unpremultiplied()
        .into_iter()
        .flat_map(|p| [p.r, p.g, p.b, p.a])
        .collect();
    GrayImage::from_interleaved(pw, ph, 4, &rgba)
}
