//! Drawing one video frame: the current sentence's lines on the theme's
//! page color, in the theme's text color, with the spoken word in bold and
//! underlined (shape as well as weight, never color alone).

use std::ops::Range;

use hayro::vello_cpu::color::{AlphaColor, Srgb};
use hayro::vello_cpu::kurbo::{Affine, Rect};
use hayro::vello_cpu::{Pixmap, RenderContext, Resources};

use super::VideoOptions;
use super::font::{Font, FontError};

/// Frame width in pixels.
pub const WIDTH: u16 = 1280;
/// Frame height in pixels.
pub const HEIGHT: u16 = 720;
/// Text size in pixels: large enough to read across a room.
const SIZE_PX: f64 = 44.0;
/// Space left and right of the text.
const MARGIN_X: f64 = 80.0;
/// Space above and below the text.
const MARGIN_Y: f64 = 60.0;
/// Line spacing, as a multiple of the font's own.
const LINE_SPACING: f64 = 1.25;

/// One word of the sentence, placed.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    /// The word as written (no spaces).
    pub text: String,
    /// Its chars within the sentence's caption text.
    pub chars: Range<usize>,
    /// Its line, from 0.
    pub line: usize,
    /// Its left edge in pixels.
    pub x: f64,
    /// Its width in pixels when bold (the slot it always takes, so the
    /// line never moves when the word is spoken).
    pub width: f64,
}

/// Draws frames; keeps the fonts, the canvas and the glyph outlines.
pub struct FrameRenderer {
    regular: Font<'static>,
    bold: Font<'static>,
    ctx: RenderContext,
    resources: Resources,
    pixmap: Pixmap,
    page: AlphaColor<Srgb>,
    text: AlphaColor<Srgb>,
}

impl FrameRenderer {
    /// A renderer with the bundled reading font in `opts`'s colors.
    pub fn new(opts: &VideoOptions) -> Result<Self, FontError> {
        let family = textweaver_fonts::bundled::default_text()
            .ok_or_else(|| FontError("this build has no bundled font".to_owned()))?;
        let regular = Font::parse(family.face(textweaver_fonts::Style::Regular).data)?;
        let bold = Font::parse(family.face(textweaver_fonts::Style::Bold).data)?;
        let color = |c: [u8; 3]| AlphaColor::<Srgb>::from_rgba8(c[0], c[1], c[2], 255);
        Ok(FrameRenderer {
            regular,
            bold,
            ctx: RenderContext::new(WIDTH, HEIGHT),
            resources: Resources::new(),
            pixmap: Pixmap::new(WIDTH, HEIGHT),
            page: color(opts.page),
            text: color(opts.text),
        })
    }

    /// The sentence's words in lines no wider than the frame's text area.
    pub fn layout(&self, text: &str) -> Vec<Placed> {
        let rs = self.regular.scale(SIZE_PX);
        let bs = self.bold.scale(SIZE_PX);
        let space = self.regular.advance(self.regular.glyph(' ')) * rs;
        let max = f64::from(WIDTH) - 2.0 * MARGIN_X;
        let mut out: Vec<Placed> = Vec::new();
        let (mut line, mut x) = (0usize, MARGIN_X);
        let mut at = 0usize; // chars
        for word in text.split(' ') {
            let n = word.chars().count();
            if n > 0 {
                let width = (self.regular.width(word) * rs).max(self.bold.width(word) * bs);
                if x > MARGIN_X && x + width > MARGIN_X + max {
                    line += 1;
                    x = MARGIN_X;
                }
                out.push(Placed {
                    text: word.to_owned(),
                    chars: at..at + n,
                    line,
                    x,
                    width,
                });
                x += width + space;
            }
            at += n + 1;
        }
        out
    }

    /// How many lines fit in the frame.
    pub fn lines_that_fit(&self) -> usize {
        let step = self.regular.line_height() * self.regular.scale(SIZE_PX) * LINE_SPACING;
        (((f64::from(HEIGHT) - 2.0 * MARGIN_Y) / step).floor() as usize).max(1)
    }

    /// Draws `text` (the caption text of the sentence) with the words that
    /// overlap `spoken` (caption chars) in bold and underlined, and returns
    /// the frame as RGBA bytes, row by row. `None` text draws an empty page.
    pub fn draw(&mut self, text: Option<&str>, spoken: Option<Range<usize>>) -> &[u8] {
        self.ctx.reset();
        self.ctx.set_paint(self.page);
        self.ctx
            .fill_rect(&Rect::new(0.0, 0.0, f64::from(WIDTH), f64::from(HEIGHT)));
        if let Some(text) = text {
            let placed = self.layout(text);
            self.draw_words(&placed, spoken);
        }
        self.ctx.flush();
        self.ctx
            .render_to_pixmap(&mut self.resources, &mut self.pixmap);
        self.pixmap.data_as_u8_slice()
    }

    fn draw_words(&mut self, placed: &[Placed], spoken: Option<Range<usize>>) {
        let lines = placed.last().map_or(0, |p| p.line + 1);
        let fit = self.lines_that_fit();
        let is_spoken = |p: &Placed| {
            spoken
                .as_ref()
                .is_some_and(|s| s.start < p.chars.end && p.chars.start < s.end)
        };
        // When the sentence is taller than the frame, show the lines
        // around the spoken word, a third of the way down.
        let spoken_line = placed.iter().find(|p| is_spoken(p)).map_or(0, |p| p.line);
        let first = if lines > fit {
            spoken_line.saturating_sub(fit / 3).min(lines - fit)
        } else {
            0
        };
        let shown = lines.min(fit);
        let rs = self.regular.scale(SIZE_PX);
        let step = self.regular.line_height() * rs * LINE_SPACING;
        let top = (f64::from(HEIGHT) - step * shown as f64) / 2.0;
        let ascent = self.regular.ascender() * rs;
        self.ctx.set_paint(self.text);
        for p in placed {
            if p.line < first || p.line >= first + shown {
                continue;
            }
            let baseline =
                top + step * (p.line - first) as f64 + ascent + (step - step / LINE_SPACING) / 2.0;
            let bold = is_spoken(p);
            let font = if bold {
                &mut self.bold
            } else {
                &mut self.regular
            };
            let scale = font.scale(SIZE_PX);
            let mut x = p.x;
            for c in p.text.chars() {
                let g = font.glyph(c);
                let advance = font.advance(g) * scale;
                let path = font.outline(g);
                if !path.elements().is_empty() {
                    self.ctx
                        .set_transform(Affine::new([scale, 0.0, 0.0, -scale, x, baseline]));
                    self.ctx.fill_path(path);
                }
                x += advance;
            }
            self.ctx.reset_transform();
            if bold {
                let (y, thick) = underline(baseline);
                self.ctx
                    .fill_rect(&Rect::new(p.x, y, p.x + p.width, y + thick));
            }
        }
    }
}

/// Where the spoken word's underline goes below `baseline`, and how thick
/// it is, in pixels.
pub fn underline(baseline: f64) -> (f64, f64) {
    (
        (baseline + SIZE_PX * 0.12).round(),
        (SIZE_PX * 0.08).round().max(3.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> VideoOptions {
        VideoOptions {
            page: [255, 255, 255],
            text: [0, 0, 0],
        }
    }

    #[test]
    fn layout_wraps_and_keeps_char_ranges() {
        let r = FrameRenderer::new(&opts()).unwrap();
        let text = "The quick brown fox jumps over the lazy dog, ".repeat(4);
        let text = text.trim_end();
        let placed = r.layout(text);
        assert_eq!(placed.len(), 36);
        assert!(placed.last().unwrap().line > 0, "a long sentence wraps");
        let fox = &placed[3];
        assert_eq!(fox.text, "fox");
        assert_eq!(fox.chars, 16..19);
        assert!(
            placed
                .iter()
                .all(|p| p.x + p.width <= f64::from(WIDTH) - MARGIN_X + 0.5)
        );
    }

    #[test]
    fn the_spoken_word_is_underlined() {
        let mut r = FrameRenderer::new(&opts()).unwrap();
        let text = "Read this aloud";
        let placed = r.layout(text);
        let this = placed[1].clone();
        let plain = r.draw(Some(text), None).to_vec();
        let spoken = r.draw(Some(text), Some(this.chars.clone())).to_vec();
        assert_eq!(plain.len(), usize::from(WIDTH) * usize::from(HEIGHT) * 4);
        // One line, centered: find the baseline the renderer used.
        let rs = r.regular.scale(SIZE_PX);
        let step = r.regular.line_height() * rs * LINE_SPACING;
        let top = (f64::from(HEIGHT) - step) / 2.0;
        let baseline = top + r.regular.ascender() * rs + (step - step / LINE_SPACING) / 2.0;
        let (y, thick) = underline(baseline);
        let row = (y + thick / 2.0) as usize;
        let dark = |frame: &[u8], x: usize| frame[(row * usize::from(WIDTH) + x) * 4] < 64;
        let middle = (this.x + this.width / 2.0) as usize;
        assert!(
            dark(&spoken, middle),
            "the underline is drawn under the word"
        );
        assert!(!dark(&plain, middle), "no underline when nothing is spoken");
        // The underline spans the word and stops at its edges.
        let left = this.x.ceil() as usize + 1;
        let right = (this.x + this.width).floor() as usize - 1;
        assert!((left..=right).all(|x| dark(&spoken, x)));
        assert!(!dark(&spoken, (this.x - 6.0) as usize));
        // Bold: more ink in the word's box than when it is not spoken.
        let ink = |frame: &[u8]| {
            let (x0, x1) = (this.x as usize, (this.x + this.width) as usize);
            let (y0, y1) = ((baseline - 40.0) as usize, baseline as usize);
            (y0..y1)
                .flat_map(|yy| (x0..x1).map(move |xx| (yy, xx)))
                .filter(|&(yy, xx)| frame[(yy * usize::from(WIDTH) + xx) * 4] < 128)
                .count()
        };
        assert!(ink(&spoken) > ink(&plain), "the spoken word is bold");
    }

    #[test]
    fn an_empty_page_is_the_page_color() {
        let mut r = FrameRenderer::new(&VideoOptions {
            page: [10, 20, 30],
            text: [200, 200, 200],
        })
        .unwrap();
        let f = r.draw(None, None);
        assert_eq!(&f[..4], &[10, 20, 30, 255]);
        assert!(f.chunks(4).all(|p| p == [10, 20, 30, 255]));
    }
}
