//! The Braille view: the current line or sentence drawn as braille dots
//! in a bar at the bottom of the app, for learning and teaching braille
//! and for checking spacing (the app core's [`textweaver_app::braille_view`]).
//!
//! - **Dots drawn, no font.** Each cell's dots are circles at the standard
//!   proportions (dots 2.5 mm apart, cells 6 mm apart, so a cell pitch of
//!   2.4 dot spacings), sized from the document text size, so spacing is
//!   exact at any size. Eight-dot cells get a fourth row when the braille
//!   uses dots 7 or 8.
//! - **Never color alone.** A raised dot is a filled disc; an empty dot
//!   position is a thin ring. Fill against outline, not a color.
//! - **Teaching options.** The print line puts the print each run of cells
//!   stands for under it; cell outlines draw each cell's box, so blank
//!   cells show.
//! - **Visual only, and why it is an image to screen readers.** The bar is
//!   one image node named "Braille view" with a description that says it
//!   is visual only; its braille is never in the tree, never live, never
//!   focused. Screen readers already send braille to a real display, so
//!   the cells read as text would only repeat it, and a change on every
//!   cursor move would be noise. Hiding the bar entirely would leave a
//!   screen reader user who turned it on (to teach a sighted student, say)
//!   no way to find that it is there; an image with a name and no content
//!   is how a drawing is given to assistive technology, quiet and
//!   findable.
//! - **Focus mode.** It fades with the bars and comes back with them on a
//!   mouse move, never on a key.

use std::ops::Range;

use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, BrushIndex, ChildrenIds, LayoutCtx, MeasureCtx, NoAction, PaintCtx, PropertiesRef,
    RegisterCtx, StyleProperty, Widget, WidgetMut, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Circle, Rect, RoundedRect, Size, Stroke, Vec2};
use masonry::layout::{LenReq, Length};
use masonry::parley::style::FontFamily;
use masonry::parley::{Alignment, AlignmentOptions, Layout};
use textweaver_app::App;
use textweaver_app::lexicon::args;

use crate::theme::{self, Palette};

/// Dot spacing in multiples of the document text size.
const PITCH_RATIO: f64 = 0.6;
/// Cell to cell, in dot spacings (6 mm over 2.5 mm).
const CELL_PITCH: f64 = 2.4;
/// A dot's radius, in dot spacings (a 1.5 mm dot).
const DOT_RADIUS: f64 = 0.3;
/// The smallest dot spacing, in logical pixels, when the window is narrow.
const MIN_PITCH: f64 = 3.0;
/// Room around the cells.
const MARGIN: f64 = 16.0;
/// The caption's size, before the platform text scale.
const CAPTION_SIZE: f32 = 13.0;

/// What the bar shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BrailleShown {
    /// The cells, dots as bits (bit 0 is dot 1).
    pub cells: Vec<u8>,
    /// Cells on the display: the bar is laid out for this many.
    pub width: usize,
    /// Under each run of cells, its print (empty while the print line is
    /// off).
    pub print: Vec<(Range<usize>, String)>,
    /// Draw each cell's outline.
    pub outlines: bool,
    /// Four rows of dots (the braille uses dots 7 or 8).
    pub eight: bool,
    /// The caption: the grade and the part shown ("Contracted braille, 1
    /// of 2"). Drawn only.
    pub caption: String,
    /// The document text size, in logical pixels.
    pub size: f32,
    /// The reading font's family list, for the print line.
    pub family: String,
    /// The bar's accessible name.
    pub name: String,
    /// The bar's accessible description: what it is, never its braille.
    pub description: String,
}

impl BrailleShown {
    /// What the bar shows for `app`, or `None` while the view is off.
    pub fn from_app(app: &App) -> Option<BrailleShown> {
        let v = app.braille_view()?;
        let c = app.catalog();
        let id = if v.fell_back {
            "gui-braille-caption-fell-back"
        } else if v.contracted {
            "gui-braille-caption-contracted"
        } else {
            "gui-braille-caption-uncontracted"
        };
        let font = crate::fonts::doc_font(&app.settings().reading_aids.font);
        Some(BrailleShown {
            eight: v.eight_dot(),
            print: if v.print_line {
                v.print
                    .iter()
                    .map(|g| (g.cells.clone(), g.print.clone()))
                    .collect()
            } else {
                Vec::new()
            },
            cells: v.cells,
            width: v.width,
            outlines: v.outlines,
            caption: c.fmt(id, &args!["n" => v.part + 1, "total" => v.parts]),
            size: font.size,
            family: font.family,
            name: c.tr("gui-braille-view"),
            description: c.tr("gui-braille-view-description"),
        })
    }
}

/// The bar's measures at one width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    /// Dot spacing, center to center.
    pub pitch: f64,
    /// Cell to cell.
    pub cell: f64,
    /// Rows of dots: 3, or 4 for eight-dot braille.
    pub rows: usize,
    /// A cell's height (its outline).
    pub cell_height: f64,
    /// Where the cells start, from the bar's left and top.
    pub origin: (f64, f64),
    /// The print line's text size, or 0 when it is off.
    pub print_size: f32,
}

impl Metrics {
    /// The measures of `shown` in a bar `width` wide: the dot spacing from
    /// the text size, smaller if the display's cells would not fit.
    pub fn of(shown: &BrailleShown, width: f64) -> Metrics {
        let size = if shown.size > 0.0 { shown.size } else { 18.0 };
        let cells = shown.width.max(shown.cells.len()).max(1) as f64;
        let fit = (width - 2.0 * MARGIN) / (cells * CELL_PITCH);
        let pitch = (f64::from(size) * PITCH_RATIO).min(fit).max(MIN_PITCH);
        let rows = if shown.eight { 4 } else { 3 };
        let cell_height = (rows as f64 - 1.0) * pitch + 1.4 * pitch;
        let caption = f64::from(theme::ui_size(CAPTION_SIZE)) * 1.4;
        let print_on = !shown.print.is_empty();
        Metrics {
            pitch,
            cell: CELL_PITCH * pitch,
            rows,
            cell_height,
            origin: (MARGIN, MARGIN / 2.0 + caption),
            print_size: if print_on {
                (CELL_PITCH * pitch * 0.7) as f32
            } else {
                0.0
            },
        }
    }

    /// The bar's height.
    pub fn height(&self) -> f64 {
        let print = if self.print_size > 0.0 {
            f64::from(self.print_size) * 1.5
        } else {
            0.0
        };
        self.origin.1 + self.cell_height + print + MARGIN / 2.0
    }

    /// Cell `i`'s box.
    pub fn cell_box(&self, i: usize) -> Rect {
        let x = self.origin.0 + i as f64 * self.cell;
        Rect::new(
            x,
            self.origin.1,
            x + self.cell,
            self.origin.1 + self.cell_height,
        )
    }
}

/// One dot position to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dot {
    /// The cell, from 0.
    pub cell: usize,
    /// The dot's number, 1 to 8.
    pub number: u8,
    /// Its center.
    pub x: f64,
    /// Its center.
    pub y: f64,
    /// Raised (a filled disc) or not (a thin ring).
    pub raised: bool,
}

/// Every dot position of every cell, raised or not: dots 1, 2, 3 down the
/// left column, 4, 5, 6 down the right, and 7 and 8 on a fourth row.
pub fn dots(shown: &BrailleShown, m: &Metrics) -> Vec<Dot> {
    let mut out = Vec::with_capacity(shown.cells.len() * m.rows * 2);
    for (i, &d) in shown.cells.iter().enumerate() {
        let b = m.cell_box(i);
        let left = b.x0 + (m.cell - m.pitch) / 2.0;
        let top = b.y0 + 0.7 * m.pitch;
        for number in 1..=(m.rows as u8 * 2) {
            let (col, row) = match number {
                1..=3 => (0, number - 1),
                4..=6 => (1, number - 4),
                7 => (0, 3),
                _ => (1, 3),
            };
            out.push(Dot {
                cell: i,
                number,
                x: left + f64::from(col) * m.pitch,
                y: top + f64::from(row) * m.pitch,
                raised: d & (1 << (number - 1)) != 0,
            });
        }
    }
    out
}

/// The Braille view's bar. See the module documentation.
pub struct BrailleBar {
    palette: Palette,
    shown: Option<BrailleShown>,
    veiled: bool,
    caption: Option<Layout<BrushIndex>>,
    print: Vec<(Range<usize>, Layout<BrushIndex>)>,
}

impl std::fmt::Debug for BrailleBar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrailleBar")
            .field("shown", &self.shown)
            .field("veiled", &self.veiled)
            .finish_non_exhaustive()
    }
}

impl BrailleBar {
    /// A hidden bar.
    pub fn new(palette: Palette) -> Self {
        BrailleBar {
            palette,
            shown: None,
            veiled: false,
            caption: None,
            print: Vec::new(),
        }
    }

    /// Shows `shown`, or hides the bar with `None`.
    pub fn set_shown(this: &mut WidgetMut<'_, Self>, shown: Option<BrailleShown>) {
        if this.widget.shown == shown {
            return;
        }
        this.widget.shown = shown;
        this.widget.caption = None;
        this.widget.print.clear();
        // The texts are laid out in `layout`; showing or hiding the bar
        // also changes its height.
        this.ctx.request_layout();
        this.ctx.request_render();
        this.ctx.request_accessibility_update();
    }

    /// Changes the colors.
    pub fn set_palette(this: &mut WidgetMut<'_, Self>, palette: Palette) {
        this.widget.palette = palette;
        this.ctx.request_render();
    }

    /// Focus mode's veil: drawn as nothing while it is down, like the
    /// bars. Its node stays.
    pub fn set_veiled(this: &mut WidgetMut<'_, Self>, veiled: bool) {
        if this.widget.veiled != veiled {
            this.widget.veiled = veiled;
            this.ctx.request_render();
        }
    }

    /// What the bar shows, if anything.
    pub fn shown(&self) -> Option<&BrailleShown> {
        self.shown.as_ref()
    }

    /// True while focus mode's veil hides it.
    pub fn is_veiled(&self) -> bool {
        self.veiled
    }

    fn text(ctx: &mut LayoutCtx<'_>, family: &str, text: &str, size: f32) -> Layout<BrushIndex> {
        let (fcx, lcx) = ctx.text_contexts();
        let mut b = lcx.ranged_builder(fcx, text, 1.0, true);
        b.push_default(StyleProperty::FontFamily(FontFamily::Source(
            family.to_owned().into(),
        )));
        b.push_default(StyleProperty::FontSize(size));
        b.push_default(StyleProperty::Brush(BrushIndex(0)));
        let mut layout = b.build(text);
        layout.break_all_lines(None);
        layout.align(Alignment::Start, AlignmentOptions::default());
        layout
    }
}

impl Widget for BrailleBar {
    type Action = NoAction;

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross: Option<Length>,
    ) -> Length {
        match (axis, &self.shown) {
            (Axis::Vertical, Some(s)) => {
                let w = cross.map_or(800.0, Length::get);
                Length::px(Metrics::of(s, w).height())
            }
            (Axis::Vertical, None) => Length::ZERO,
            (Axis::Horizontal, _) => match len_req {
                LenReq::FitContent(space) => space,
                _ => Length::px(200.0),
            },
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let Some(s) = self.shown.clone() else {
            return;
        };
        let m = Metrics::of(&s, size.width);
        let family = if s.family.is_empty() {
            crate::fonts::DEFAULT_STACK
        } else {
            s.family.as_str()
        };
        self.caption = Some(Self::text(
            ctx,
            crate::fonts::DEFAULT_STACK,
            &s.caption,
            theme::ui_size(CAPTION_SIZE),
        ));
        self.print.clear();
        for (cells, text) in &s.print {
            if text.trim().is_empty() {
                continue;
            }
            // Smaller where a run's print is wider than its cells.
            let room = cells.len() as f64 * m.cell;
            let mut layout = Self::text(ctx, family, text, m.print_size);
            let w = f64::from(layout.width());
            if w > room && w > 0.0 {
                let fitted = (f64::from(m.print_size) * room / w).max(6.0) as f32;
                layout = Self::text(ctx, family, text, fitted);
            }
            self.print.push((cells.clone(), layout));
        }
    }

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        let Some(s) = &self.shown else {
            return;
        };
        if self.veiled {
            return;
        }
        let p = &self.palette;
        let size = ctx.content_box().size();
        let hc = p.kind == textweaver_theme::ThemeKind::HighContrast;
        let edge = if hc { 2.0 } else { 1.0 };
        // A panel like RSVP's.
        let card = RoundedRect::from_rect(size.to_rect(), theme::PANEL_RADIUS);
        painter.fill(card, theme::color(p.surface)).draw();
        painter
            .stroke(
                RoundedRect::from_rect(size.to_rect().inset(-edge / 2.0), theme::PANEL_RADIUS),
                &Stroke::new(edge),
                theme::color(p.border),
            )
            .draw();
        let text: masonry::peniko::Brush = theme::color(p.text).into();
        let dim: masonry::peniko::Brush = theme::color(p.dim_text).into();
        if let Some(caption) = &self.caption {
            render_text(
                painter,
                Affine::translate(Vec2::new(MARGIN, MARGIN / 4.0)),
                caption,
                std::slice::from_ref(&dim),
                false,
            );
        }
        let m = Metrics::of(s, size.width);
        if s.outlines {
            for i in 0..s.cells.len() {
                let b = m.cell_box(i).inset(-m.pitch * 0.1);
                painter
                    .stroke(
                        RoundedRect::from_rect(b, m.pitch * 0.3),
                        &Stroke::new(edge),
                        theme::color(p.border),
                    )
                    .draw();
            }
        }
        let r = DOT_RADIUS * m.pitch;
        // An empty position: a ring, thinner in the usual themes.
        let ring = if hc {
            1.5
        } else {
            (m.pitch * 0.06).clamp(0.75, 1.5)
        };
        for d in dots(s, &m) {
            let circle = Circle::new((d.x, d.y), r);
            if d.raised {
                painter.fill(circle, theme::color(p.text)).draw();
            } else {
                painter
                    .stroke(
                        Circle::new((d.x, d.y), r - ring / 2.0),
                        &Stroke::new(ring),
                        theme::color(p.dim_text),
                    )
                    .draw();
            }
        }
        let y = m.origin.1 + m.cell_height + f64::from(m.print_size) * 0.15;
        for (cells, layout) in &self.print {
            let x0 = m.origin.0 + cells.start as f64 * m.cell;
            let room = cells.len() as f64 * m.cell;
            let x = x0 + (room - f64::from(layout.width())).max(0.0) / 2.0;
            render_text(
                painter,
                Affine::translate(Vec2::new(x, y)),
                layout,
                std::slice::from_ref(&text),
                false,
            );
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        let Some(s) = &self.shown else {
            node.set_hidden();
            return;
        };
        // What it is, never its braille: see the module documentation.
        node.set_label(s.name.as_str());
        node.set_description(s.description.as_str());
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn get_debug_text(&self) -> Option<String> {
        self.shown.as_ref().map(|s| s.caption.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown(cells: Vec<u8>) -> BrailleShown {
        BrailleShown {
            cells,
            width: 40,
            size: 20.0,
            ..BrailleShown::default()
        }
    }

    /// The drawing model, without a display: six positions per cell in two
    /// columns of three, raised where the bits say, the rest empty rings.
    #[test]
    fn dots_are_placed_by_number_and_raised_by_bit() {
        // Cell 0: dots 1 and 5 (e). Cell 1: blank.
        let s = shown(vec![0b01_0001, 0]);
        let m = Metrics::of(&s, 2000.0);
        assert_eq!(m.rows, 3);
        assert!((m.pitch - 20.0 * PITCH_RATIO).abs() < 1e-9);
        assert!((m.cell - CELL_PITCH * m.pitch).abs() < 1e-9);
        let d = dots(&s, &m);
        assert_eq!(d.len(), 12);
        let raised: Vec<(usize, u8)> = d
            .iter()
            .filter(|d| d.raised)
            .map(|d| (d.cell, d.number))
            .collect();
        assert_eq!(raised, vec![(0, 1), (0, 5)]);
        let by = |cell: usize, n: u8| *d.iter().find(|d| d.cell == cell && d.number == n).unwrap();
        // Dot 4 is right of dot 1, dot 2 below it.
        assert!((by(0, 4).x - by(0, 1).x - m.pitch).abs() < 1e-9);
        assert!((by(0, 2).y - by(0, 1).y - m.pitch).abs() < 1e-9);
        assert_eq!(by(0, 3).x, by(0, 1).x);
        // The next cell is one cell pitch on: spacing is exact.
        assert!((by(1, 1).x - by(0, 1).x - m.cell).abs() < 1e-9);
        assert!(d.iter().filter(|d| d.cell == 1).all(|d| !d.raised));
    }

    #[test]
    fn eight_dot_cells_get_a_fourth_row() {
        let mut s = shown(vec![0b1100_0000]);
        s.eight = true;
        let m = Metrics::of(&s, 2000.0);
        assert_eq!(m.rows, 4);
        let d = dots(&s, &m);
        assert_eq!(d.len(), 8);
        let seven = d.iter().find(|d| d.number == 7).unwrap();
        let three = d.iter().find(|d| d.number == 3).unwrap();
        assert!(seven.raised && !three.raised);
        assert_eq!(seven.x, three.x);
        assert!((seven.y - three.y - m.pitch).abs() < 1e-9);
        assert!(m.cell_height > Metrics::of(&shown(vec![0]), 2000.0).cell_height);
    }

    /// The text size sets the dots; a narrow bar shrinks them so the
    /// display's cells fit; the print line adds height.
    #[test]
    fn metrics_scale_with_the_text_and_fit_the_width() {
        let small = Metrics::of(
            &BrailleShown {
                size: 10.0,
                ..shown(vec![1])
            },
            4000.0,
        );
        let large = Metrics::of(
            &BrailleShown {
                size: 40.0,
                ..shown(vec![1])
            },
            4000.0,
        );
        assert!((large.pitch / small.pitch - 4.0).abs() < 1e-9);
        let narrow = Metrics::of(&shown(vec![1]), 500.0);
        assert!(narrow.origin.0 + 40.0 * narrow.cell <= 500.0 - MARGIN + 1e-9);
        let mut with_print = shown(vec![1]);
        with_print.print = vec![(0..1, "a".into())];
        let wide = Metrics::of(&shown(vec![1]), 2000.0);
        assert!(Metrics::of(&with_print, 2000.0).height() > wide.height());
    }
}
