//! The window's frame and its bars (W9b-n): the header and the toolbar,
//! the document between them, and the status bar, laid out so that no
//! control ever leaves the window.
//!
//! - [`Flow`]: a bar's buttons in a row that wraps onto more rows when the
//!   window is too narrow for one, instead of running past its edge.
//! - [`Frame`]: the window's column. At [`NARROW_WIDTH`] and wider, the
//!   header is above the document and the toolbar below it, as before.
//!   Narrower, the two fold into one bar above the document, and the
//!   buttons hide their keys on screen (the key property, which screen
//!   readers say, never changes). The children's order follows what is
//!   seen, so Tab, Shift+Tab and the screen reader's order match the
//!   screen in both layouts.
//! - Either bar can be hidden from the View menu (`[gui] header`,
//!   `[gui] toolbar`); a hidden bar leaves the Tab order and the
//!   accessibility tree, and its commands stay on their keys and menus.

use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx, PropertiesRef,
    RegisterCtx, Widget, WidgetMut, WidgetPod, WidgetTag,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LayoutSize, LenReq, Length};

use masonry::peniko::Color;
use masonry::properties::{Background, BorderWidth, BoxShadow, Padding};
use masonry::widgets::Label;

use crate::theme::{self, Palette};
use crate::widgets::{ActionButton, Region};

/// Below this window width, in logical pixels, the header and the toolbar
/// fold into one bar and the buttons hide their keys on screen (the GUI
/// audit's "about 800").
pub const NARROW_WIDTH: f64 = 800.0;

/// Below this window height, in logical pixels, the folded bar keeps to
/// two rows (one each for the header and the toolbar, or two for the one
/// shown): the buttons that do not fit are hidden, and the header's last
/// button, Commands, stays, so every command is still a list away.
pub const SHORT_HEIGHT: f64 = 480.0;

/// The space between bars, and between buttons in a bar.
const GAP: f64 = theme::GAP;

/// The window's column.
pub const FRAME: WidgetTag<Frame> = WidgetTag::named("tw-frame");

// --- Flow.

/// A bar's buttons, left to right, wrapping onto more rows when they do
/// not fit. With [`with_push_right`](Self::with_push_right), the buttons
/// from that one on sit at the right end while all fit on one row (the
/// toolbar's Slower and Faster).
pub struct Flow {
    children: Vec<WidgetPod<ActionButton>>,
    push_right: Option<usize>,
    compact: bool,
    /// At most this many rows ([`set_max_rows`](Self::set_max_rows)); the
    /// buttons that do not fit are hidden.
    max_rows: Option<usize>,
    /// The last button stays shown when rows are limited.
    keep_last: bool,
}

impl Flow {
    /// A flow of `buttons`.
    pub fn new(buttons: Vec<NewWidget<ActionButton>>) -> Self {
        Flow {
            children: buttons.into_iter().map(NewWidget::to_pod).collect(),
            push_right: None,
            compact: false,
            max_rows: None,
            keep_last: false,
        }
    }

    /// Puts the buttons from `index` on at the right end of a single row.
    /// When the row wraps, those buttons stay together on one row.
    pub fn with_push_right(mut self, index: usize) -> Self {
        self.push_right = Some(index);
        self
    }

    /// Keeps the last button shown when [`set_max_rows`](Self::set_max_rows)
    /// hides some (the header's Commands, which opens every command,
    /// those hidden too).
    pub fn with_keep_last(mut self) -> Self {
        self.keep_last = true;
        self
    }

    /// Limits the flow to `rows` rows (`None`: no limit). The buttons that
    /// do not fit are hidden: off the screen, out of the Tab order and out
    /// of the accessibility tree, so what is reached is what is seen.
    /// Their commands stay on their keys, in the menus and in Commands.
    pub fn set_max_rows(this: &mut WidgetMut<'_, Self>, rows: Option<usize>) {
        if this.widget.max_rows != rows {
            this.widget.max_rows = rows;
            this.ctx.request_layout();
        }
    }

    /// The buttons shown at `width`, by index, in order.
    fn shown(&self, widths: &[f64], width: f64) -> Vec<usize> {
        let all: Vec<usize> = (0..widths.len()).collect();
        let Some(max) = self.max_rows.filter(|&m| m > 0) else {
            return all;
        };
        let fits = |set: &[usize]| {
            let w: Vec<f64> = set.iter().map(|&i| widths[i]).collect();
            rows(&w, width, self.group(set)).len() <= max
        };
        if fits(&all) {
            return all;
        }
        let n = widths.len();
        for k in (1..n).rev() {
            let mut set: Vec<usize> = (0..k).collect();
            if self.keep_last && k < n {
                set.push(n - 1);
            }
            if fits(&set) {
                return set;
            }
        }
        vec![0]
    }

    /// Where the buttons from [`with_push_right`](Self::with_push_right) on
    /// start among `shown`, when all of them are shown: they are kept
    /// together on one row.
    fn group(&self, shown: &[usize]) -> Option<usize> {
        let p = self.push_right?;
        let n = self.children.len();
        (p < n && (p..n).all(|i| shown.contains(&i)))
            .then(|| shown.iter().position(|&i| i == p))
            .flatten()
    }

    /// Hides (`true`) or shows the keys drawn on the buttons.
    pub fn set_compact(this: &mut WidgetMut<'_, Self>, compact: bool) {
        if this.widget.compact == compact {
            return;
        }
        this.widget.compact = compact;
        for i in 0..this.widget.children.len() {
            let mut b = this.ctx.get_mut(&mut this.widget.children[i]);
            ActionButton::set_show_key(&mut b, !compact);
            // Less padding, still well over 24 by 24 (WCAG 2.5.8).
            if compact {
                b.insert_prop(Padding::from_vh(Length::px(5.0), Length::px(12.0)));
            } else {
                b.remove_prop::<Padding>();
            }
        }
        this.ctx.request_layout();
    }

    /// True while the keys are hidden.
    pub fn is_compact(&self) -> bool {
        self.compact
    }

    /// Each button's natural width and height.
    fn natural(&mut self, ctx: &mut MeasureCtx<'_>) -> Vec<(f64, f64)> {
        let mut out = Vec::with_capacity(self.children.len());
        for c in &mut self.children {
            out.push(natural_size(ctx, c));
        }
        out
    }
}

/// A child's natural (one-line) size.
fn natural_size<W: Widget + ?Sized>(
    ctx: &mut MeasureCtx<'_>,
    child: &mut WidgetPod<W>,
) -> (f64, f64) {
    let w = ctx.compute_length(
        child,
        LenReq::MaxContent.into(),
        LayoutSize::maybe(Axis::Vertical, None),
        Axis::Horizontal,
        None,
    );
    let h = ctx.compute_length(
        child,
        LenReq::MaxContent.into(),
        LayoutSize::maybe(Axis::Horizontal, Some(w)),
        Axis::Vertical,
        Some(w),
    );
    (w.get(), h.get())
}

/// Splits items of `widths` into rows no wider than `width` (at least one
/// item a row): the index each row starts at. The items from `group` on
/// start a new row together when they do not all fit on the current one
/// (so Faster is never left alone on a row without Slower).
fn rows(widths: &[f64], width: f64, group: Option<usize>) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut x = 0.0;
    for (i, &w) in widths.iter().enumerate() {
        if i == 0 {
            starts.push(0);
            x = w;
            continue;
        }
        let w_needed = if group == Some(i) {
            let rest = &widths[i..];
            let together = rest.iter().sum::<f64>() + GAP * (rest.len() - 1) as f64;
            // Only when the group fits on a row of its own.
            if together <= width + 0.5 { together } else { w }
        } else {
            w
        };
        if x + GAP + w_needed > width + 0.5 {
            starts.push(i);
            x = w;
        } else {
            x += GAP + w;
        }
    }
    starts
}

/// The rows' heights, for items of `sizes` starting rows at `starts`.
fn row_heights(sizes: &[(f64, f64)], starts: &[usize]) -> Vec<f64> {
    let mut out = Vec::with_capacity(starts.len());
    for (r, &s) in starts.iter().enumerate() {
        let end = starts.get(r + 1).copied().unwrap_or(sizes.len());
        out.push(sizes[s..end].iter().map(|x| x.1).fold(0.0, f64::max));
    }
    out
}

fn total(heights: &[f64]) -> f64 {
    if heights.is_empty() {
        return 0.0;
    }
    heights.iter().sum::<f64>() + GAP * (heights.len() - 1) as f64
}

impl Widget for Flow {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for c in &mut self.children {
            ctx.register_child(c);
        }
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<Length>,
    ) -> Length {
        let sizes = self.natural(ctx);
        let widest = sizes.iter().map(|s| s.0).fold(0.0, f64::max);
        let one_row = if sizes.is_empty() {
            0.0
        } else {
            sizes.iter().map(|s| s.0).sum::<f64>() + GAP * (sizes.len() - 1) as f64
        };
        match axis {
            Axis::Horizontal => Length::px(match len_req {
                LenReq::MinContent => widest,
                LenReq::MaxContent => one_row,
                LenReq::FitContent(space) => one_row.min(space.get()).max(widest),
            }),
            Axis::Vertical => {
                let width = cross_length.map_or(one_row, Length::get);
                let widths: Vec<f64> = sizes.iter().map(|s| s.0).collect();
                let shown = self.shown(&widths, width);
                let sizes: Vec<(f64, f64)> = shown.iter().map(|&i| sizes[i]).collect();
                let widths: Vec<f64> = sizes.iter().map(|s| s.0).collect();
                let starts = rows(&widths, width, self.group(&shown));
                Length::px(total(&row_heights(&sizes, &starts)))
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let mut sizes = Vec::with_capacity(self.children.len());
        for c in &mut self.children {
            let w = ctx.compute_length(
                c,
                LenReq::MaxContent.into(),
                LayoutSize::maybe(Axis::Vertical, None),
                Axis::Horizontal,
                None,
            );
            // Never wider than the bar: a button wider than the window
            // is narrowed rather than pushed past its edge.
            let w = w.get().min(size.width.max(0.0));
            let h = ctx.compute_length(
                c,
                LenReq::MaxContent.into(),
                LayoutSize::maybe(Axis::Horizontal, Some(Length::px(w))),
                Axis::Vertical,
                Some(Length::px(w)),
            );
            sizes.push((w, h.get()));
        }
        let widths: Vec<f64> = sizes.iter().map(|s| s.0).collect();
        let shown = self.shown(&widths, size.width);
        for (i, child) in self.children.iter_mut().enumerate() {
            ctx.set_stashed(child, !shown.contains(&i));
        }
        let group = self.group(&shown);
        let sizes: Vec<(f64, f64)> = shown.iter().map(|&i| sizes[i]).collect();
        let widths: Vec<f64> = sizes.iter().map(|s| s.0).collect();
        let starts = rows(&widths, size.width, group);
        let heights = row_heights(&sizes, &starts);
        let single = starts.len() == 1;
        let mut y = 0.0;
        for (r, &s) in starts.iter().enumerate() {
            let end = starts.get(r + 1).copied().unwrap_or(sizes.len());
            let row_h = heights[r];
            let mut x: f64 = 0.0;
            let push_at = group.filter(|_| single);
            let right_width: f64 = match push_at {
                Some(p) if p < end => {
                    sizes[p..end].iter().map(|s| s.0).sum::<f64>()
                        + GAP * (end - p).saturating_sub(1) as f64
                }
                _ => 0.0,
            };
            for (k, &(w, _)) in sizes.iter().enumerate().take(end).skip(s) {
                if push_at == Some(k) {
                    x = x.max(size.width - right_width);
                }
                let child = &mut self.children[shown[k]];
                ctx.run_layout(child, Size::new(w, row_h));
                ctx.place_child(child, Point::new(x, y));
                x += w + GAP;
            }
            y += row_h + GAP;
        }
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _p: &mut Painter<'_>) {
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        self.children.iter().map(WidgetPod::id).collect()
    }
}

// --- Frame.

/// The window's column: the header, the document's row, the RSVP strip,
/// the toolbar, the status bar, and the live region. See the module
/// documentation.
pub struct Frame {
    header: WidgetPod<Region>,
    body: WidgetPod<dyn Widget>,
    rsvp: WidgetPod<dyn Widget>,
    toolbar: WidgetPod<Region>,
    status: WidgetPod<dyn Widget>,
    announcer: WidgetPod<dyn Widget>,
    folded: bool,
    /// Folded and shorter than [`SHORT_HEIGHT`]: the bars' rows are
    /// limited.
    short: bool,
    show_header: bool,
    show_toolbar: bool,
    /// The colors the bars' cards are drawn in.
    palette: Palette,
}

impl Frame {
    /// The column, wide (header above the document, toolbar below) until
    /// its first layout says otherwise.
    pub fn new(
        header: NewWidget<Region>,
        body: NewWidget<impl Widget + ?Sized>,
        rsvp: NewWidget<impl Widget + ?Sized>,
        toolbar: NewWidget<Region>,
        status: NewWidget<impl Widget + ?Sized>,
        announcer: NewWidget<impl Widget + ?Sized>,
        palette: &Palette,
    ) -> Self {
        Frame {
            header: header.to_pod(),
            body: body.erased().to_pod(),
            rsvp: rsvp.erased().to_pod(),
            toolbar: toolbar.to_pod(),
            status: status.erased().to_pod(),
            announcer: announcer.erased().to_pod(),
            folded: false,
            short: false,
            show_header: true,
            show_toolbar: true,
            palette: palette.clone(),
        }
    }

    /// True while the header and the toolbar are folded into one bar.
    pub fn is_folded(&self) -> bool {
        self.folded
    }

    /// True while the header is shown.
    pub fn header_shown(&self) -> bool {
        self.show_header
    }

    /// True while the toolbar is shown.
    pub fn toolbar_shown(&self) -> bool {
        self.show_toolbar
    }

    /// Folds the bars into one (a narrow window) or unfolds them. The
    /// buttons hide their keys on screen while folded.
    pub fn set_folded(this: &mut WidgetMut<'_, Self>, folded: bool) {
        if this.widget.folded == folded {
            return;
        }
        this.widget.folded = folded;
        for bar in [&mut this.widget.header, &mut this.widget.toolbar] {
            let mut region = this.ctx.get_mut(bar);
            let mut child = Region::child_mut(&mut region);
            if let Some(mut flow) = child.try_downcast::<Flow>() {
                Flow::set_compact(&mut flow, folded);
            }
        }
        Self::restyle(this);
        // The order of the children is the order on screen.
        this.ctx.children_changed();
        this.ctx.request_layout();
    }

    /// Recolors the bars for `palette` (a theme change).
    pub fn set_palette(this: &mut WidgetMut<'_, Self>, palette: &Palette) {
        this.widget.palette = palette.clone();
        Self::restyle(this);
    }

    /// The bars' look: cards at normal widths; folded, the header and the
    /// toolbar go flat and every padding shrinks, so the document keeps
    /// room in a small window.
    fn restyle(this: &mut WidgetMut<'_, Self>) {
        let folded = this.widget.folded;
        let p = this.widget.palette.clone();
        let pad = if folded { 8.0 } else { theme::PAD };
        this.insert_prop(Padding::all(Length::px(pad)));
        let bars = [
            (&mut this.widget.header, (10.0, 16.0)),
            (&mut this.widget.toolbar, (10.0, 12.0)),
        ];
        for (bar, (v, h)) in bars {
            let mut r = this.ctx.get_mut(bar);
            if folded {
                r.insert_prop(Background::Color(Color::TRANSPARENT));
                r.insert_prop(BorderWidth {
                    width: Length::px(0.0),
                });
                r.insert_prop(BoxShadow::new(Color::TRANSPARENT, (0.0, 0.0)));
                r.insert_prop(Padding::all(Length::px(0.0)));
            } else {
                let (bg, border, bw, radius, shadow) = theme::panel_props(&p);
                r.insert_prop(bg);
                r.insert_prop(border);
                r.insert_prop(bw);
                r.insert_prop(radius);
                r.insert_prop(shadow);
                r.insert_prop(Padding::from_vh(Length::px(v), Length::px(h)));
            }
        }
        let mut status = this.ctx.get_mut(&mut this.widget.status);
        let (bg, border, bw, radius, shadow) = theme::panel_props(&p);
        status.insert_prop(bg);
        status.insert_prop(border);
        status.insert_prop(bw);
        status.insert_prop(radius);
        status.insert_prop(shadow);
        let (v, h) = if folded { (4.0, 8.0) } else { (8.0, 16.0) };
        status.insert_prop(Padding::from_vh(Length::px(v), Length::px(h)));
    }

    /// Shows or hides the header and the toolbar (`[gui] header` and
    /// `[gui] toolbar`).
    pub fn set_bars(this: &mut WidgetMut<'_, Self>, header: bool, toolbar: bool) {
        if this.widget.show_header == header && this.widget.show_toolbar == toolbar {
            return;
        }
        this.widget.show_header = header;
        this.widget.show_toolbar = toolbar;
        Self::limit_rows(this);
        this.ctx.request_layout();
    }

    /// True while the folded bar is kept to two rows (a short window).
    pub fn is_short(&self) -> bool {
        self.short
    }

    /// Keeps the folded bar to two rows (`true`, a short window) or lets
    /// it wrap freely.
    pub fn set_short(this: &mut WidgetMut<'_, Self>, short: bool) {
        if this.widget.short != short {
            this.widget.short = short;
            Self::limit_rows(this);
            this.ctx.request_layout();
        }
    }

    /// Each bar's row limit: one each while both are shown, two for the
    /// one shown alone, none unless short.
    fn limit_rows(this: &mut WidgetMut<'_, Self>) {
        let w = &*this.widget;
        let (short, both) = (w.short, w.show_header && w.show_toolbar);
        let limit = short.then_some(if both { 1 } else { 2 });
        for bar in [&mut this.widget.header, &mut this.widget.toolbar] {
            let mut region = this.ctx.get_mut(bar);
            let mut child = Region::child_mut(&mut region);
            if let Some(mut flow) = child.try_downcast::<Flow>() {
                Flow::set_max_rows(&mut flow, limit);
            }
        }
    }

    fn height_at<W: Widget + ?Sized>(
        ctx: &mut LayoutCtx<'_>,
        child: &mut WidgetPod<W>,
        width: f64,
    ) -> f64 {
        ctx.compute_length(
            child,
            LenReq::MaxContent.into(),
            LayoutSize::maybe(Axis::Horizontal, Some(Length::px(width))),
            Axis::Vertical,
            Some(Length::px(width)),
        )
        .get()
    }

    fn width_of<W: Widget + ?Sized>(ctx: &mut LayoutCtx<'_>, child: &mut WidgetPod<W>) -> f64 {
        ctx.compute_length(
            child,
            LenReq::MaxContent.into(),
            LayoutSize::maybe(Axis::Vertical, None),
            Axis::Horizontal,
            None,
        )
        .get()
    }
}

impl Widget for Frame {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.header);
        ctx.register_child(&mut self.body);
        ctx.register_child(&mut self.rsvp);
        ctx.register_child(&mut self.toolbar);
        ctx.register_child(&mut self.status);
        ctx.register_child(&mut self.announcer);
    }

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        _axis: Axis,
        len_req: LenReq,
        _cross_length: Option<Length>,
    ) -> Length {
        // The window gives the frame its size.
        match len_req {
            LenReq::FitContent(space) => space,
            _ => Length::ZERO,
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let w = size.width.max(0.0);
        let narrow = w < NARROW_WIDTH;
        let short = narrow && size.height < SHORT_HEIGHT;
        if short != self.short {
            ctx.mutate_self_later(move |mut this| {
                if let Some(mut frame) = this.try_downcast::<Frame>() {
                    Frame::set_short(&mut frame, short);
                }
            });
        }
        if narrow != self.folded {
            // The order of the children changes with the fold, which only
            // a mutation may do; this pass lays out the current order and
            // the next one the folded (or unfolded) one.
            ctx.mutate_self_later(move |mut this| {
                if let Some(mut frame) = this.try_downcast::<Frame>() {
                    Frame::set_folded(&mut frame, narrow);
                }
            });
        }
        ctx.set_stashed(&mut self.header, !self.show_header);
        ctx.set_stashed(&mut self.toolbar, !self.show_toolbar);

        let mut y = 0.0;
        // The top band: the header, and while folded the toolbar too.
        if self.folded {
            let both = self.show_header && self.show_toolbar;
            let side_by_side = both && {
                let hw = Self::width_of(ctx, &mut self.header);
                let tw = Self::width_of(ctx, &mut self.toolbar);
                hw + GAP + tw <= w
            };
            if side_by_side {
                let hw = Self::width_of(ctx, &mut self.header).min(w);
                let tw = (w - hw - GAP).max(0.0);
                let hh = Self::height_at(ctx, &mut self.header, hw);
                let th = Self::height_at(ctx, &mut self.toolbar, tw);
                let band = hh.max(th);
                ctx.run_layout(&mut self.header, Size::new(hw, band));
                ctx.place_child(&mut self.header, Point::ORIGIN);
                ctx.run_layout(&mut self.toolbar, Size::new(tw, band));
                ctx.place_child(&mut self.toolbar, Point::new(hw + GAP, 0.0));
                y = band + GAP;
            } else {
                if self.show_header {
                    let h = Self::height_at(ctx, &mut self.header, w);
                    ctx.run_layout(&mut self.header, Size::new(w, h));
                    ctx.place_child(&mut self.header, Point::new(0.0, y));
                    y += h + GAP;
                }
                if self.show_toolbar {
                    let h = Self::height_at(ctx, &mut self.toolbar, w);
                    ctx.run_layout(&mut self.toolbar, Size::new(w, h));
                    ctx.place_child(&mut self.toolbar, Point::new(0.0, y));
                    y += h + GAP;
                }
            }
        } else if self.show_header {
            let h = Self::height_at(ctx, &mut self.header, w);
            ctx.run_layout(&mut self.header, Size::new(w, h));
            ctx.place_child(&mut self.header, Point::ORIGIN);
            y = h + GAP;
        }

        // The bottom, from the window's foot up: the live region (no
        // size), the status bar, the toolbar while unfolded, the RSVP strip.
        let announcer_h = Self::height_at(ctx, &mut self.announcer, w);
        let status_h = Self::height_at(ctx, &mut self.status, w);
        let toolbar_h = if !self.folded && self.show_toolbar {
            Self::height_at(ctx, &mut self.toolbar, w)
        } else {
            0.0
        };
        let rsvp_h = Self::height_at(ctx, &mut self.rsvp, w);
        let gap_after = |h: f64| if h > 0.0 { GAP } else { 0.0 };
        let bottom = rsvp_h
            + gap_after(rsvp_h)
            + toolbar_h
            + gap_after(toolbar_h)
            + status_h
            + gap_after(announcer_h)
            + announcer_h;
        let body_h = (size.height - y - bottom - GAP).max(0.0);
        ctx.run_layout(&mut self.body, Size::new(w, body_h));
        ctx.place_child(&mut self.body, Point::new(0.0, y));
        y += body_h + GAP;
        ctx.run_layout(&mut self.rsvp, Size::new(w, rsvp_h));
        ctx.place_child(&mut self.rsvp, Point::new(0.0, y));
        y += rsvp_h + gap_after(rsvp_h);
        if !self.folded && self.show_toolbar {
            ctx.run_layout(&mut self.toolbar, Size::new(w, toolbar_h));
            ctx.place_child(&mut self.toolbar, Point::new(0.0, y));
            y += toolbar_h + GAP;
        }
        ctx.run_layout(&mut self.status, Size::new(w, status_h));
        ctx.place_child(&mut self.status, Point::new(0.0, y));
        y += status_h + gap_after(announcer_h);
        ctx.run_layout(&mut self.announcer, Size::new(w, announcer_h));
        ctx.place_child(&mut self.announcer, Point::new(0.0, y));
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _p: &mut Painter<'_>) {
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        let ids = if self.folded {
            [
                self.header.id(),
                self.toolbar.id(),
                self.body.id(),
                self.rsvp.id(),
                self.status.id(),
                self.announcer.id(),
            ]
        } else {
            [
                self.header.id(),
                self.body.id(),
                self.rsvp.id(),
                self.toolbar.id(),
                self.status.id(),
                self.announcer.id(),
            ]
        };
        ChildrenIds::from_slice(&ids)
    }
}

// --- StatusRow.

/// The status bar's two texts: the latest message on the left and the
/// position on the right while both fit on one line; otherwise the message
/// above the position, each wrapping at the bar's width, so the two never
/// overlap. The bar's name carries both in full (the texts are hidden
/// from screen readers).
pub struct StatusRow {
    message: WidgetPod<Label>,
    position: WidgetPod<Label>,
}

/// Where the status texts go at one width.
struct StatusPlan {
    one_line: bool,
    message: Size,
    position: Size,
}

impl StatusPlan {
    fn height(&self) -> f64 {
        if self.one_line {
            self.message.height.max(self.position.height)
        } else {
            self.message.height + self.position.height
        }
    }
}

impl StatusRow {
    /// The row of `message` and `position`, which should wrap
    /// (`LineBreaking::WordWrap`).
    pub fn new(message: NewWidget<Label>, position: NewWidget<Label>) -> Self {
        StatusRow {
            message: message.to_pod(),
            position: position.to_pod(),
        }
    }
}

/// Measures a child, the same way in measure and in layout.
trait LengthCtx {
    fn length_of<W: Widget + ?Sized>(
        &mut self,
        child: &mut WidgetPod<W>,
        axis: Axis,
        cross: Option<f64>,
    ) -> f64;
}

macro_rules! length_ctx {
    ($ty:ty) => {
        impl LengthCtx for $ty {
            fn length_of<W: Widget + ?Sized>(
                &mut self,
                child: &mut WidgetPod<W>,
                axis: Axis,
                cross: Option<f64>,
            ) -> f64 {
                let cross = cross.map(|c| Length::px(c.max(0.0)));
                self.compute_length(
                    child,
                    LenReq::MaxContent.into(),
                    LayoutSize::maybe(axis.cross(), cross),
                    axis,
                    cross,
                )
                .get()
            }
        }
    };
}
length_ctx!(MeasureCtx<'_>);
length_ctx!(LayoutCtx<'_>);

/// The status texts' layout at `width`.
fn status_plan(
    ctx: &mut impl LengthCtx,
    message: &mut WidgetPod<Label>,
    position: &mut WidgetPod<Label>,
    width: f64,
) -> StatusPlan {
    let mw = ctx.length_of(message, Axis::Horizontal, None);
    let pw = ctx.length_of(position, Axis::Horizontal, None);
    let one_line = mw + 2.0 * GAP + pw <= width;
    let (mw, pw) = if one_line {
        (mw, pw)
    } else {
        (mw.min(width), pw.min(width))
    };
    let mh = if mw > 0.0 {
        ctx.length_of(message, Axis::Vertical, Some(mw))
    } else {
        0.0
    };
    let ph = ctx.length_of(position, Axis::Vertical, Some(pw));
    StatusPlan {
        one_line,
        message: Size::new(mw, mh),
        position: Size::new(pw, ph),
    }
}

impl Widget for StatusRow {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.message);
        ctx.register_child(&mut self.position);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<Length>,
    ) -> Length {
        let mw = ctx.length_of(&mut self.message, Axis::Horizontal, None);
        let pw = ctx.length_of(&mut self.position, Axis::Horizontal, None);
        let one_line = mw + 2.0 * GAP + pw;
        match axis {
            Axis::Horizontal => Length::px(match len_req {
                LenReq::MinContent => 0.0,
                LenReq::MaxContent => one_line,
                LenReq::FitContent(space) => one_line.min(space.get()),
            }),
            Axis::Vertical => {
                let width = cross_length.map_or(one_line, Length::get);
                let plan = status_plan(ctx, &mut self.message, &mut self.position, width);
                Length::px(plan.height())
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let plan = status_plan(ctx, &mut self.message, &mut self.position, size.width);
        ctx.run_layout(&mut self.message, plan.message);
        ctx.run_layout(&mut self.position, plan.position);
        if plan.one_line {
            let h = plan.height();
            ctx.place_child(
                &mut self.message,
                Point::new(0.0, (h - plan.message.height) * 0.5),
            );
            ctx.place_child(
                &mut self.position,
                Point::new(
                    size.width - plan.position.width,
                    (h - plan.position.height) * 0.5,
                ),
            );
        } else {
            ctx.place_child(&mut self.message, Point::ORIGIN);
            ctx.place_child(&mut self.position, Point::new(0.0, plan.message.height));
        }
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _p: &mut Painter<'_>) {
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.message.id(), self.position.id()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_wrap_only_what_does_not_fit() {
        assert_eq!(rows(&[100.0, 100.0, 100.0], 400.0, None), vec![0]);
        assert_eq!(rows(&[100.0, 100.0, 100.0], 250.0, None), vec![0, 2]);
        // A lone item wider than the bar still gets its own row.
        assert_eq!(rows(&[500.0, 100.0], 300.0, None), vec![0, 1]);
        assert!(rows(&[], 300.0, None).is_empty());
    }

    #[test]
    fn a_group_wraps_together() {
        // Four buttons, the last two a group (Slower and Faster): the
        // third would fit on the first row, but not with the fourth.
        let w = [100.0, 100.0, 100.0, 100.0];
        assert_eq!(rows(&w, 330.0, None), vec![0, 3]);
        assert_eq!(rows(&w, 330.0, Some(2)), vec![0, 2]);
        // A group wider than a row wraps item by item.
        assert_eq!(rows(&w, 150.0, Some(2)), vec![0, 1, 2, 3]);
    }

    #[test]
    fn rows_are_as_tall_as_their_tallest_button() {
        let sizes = [(10.0, 30.0), (10.0, 40.0), (10.0, 20.0)];
        assert_eq!(row_heights(&sizes, &[0, 2]), vec![40.0, 20.0]);
        assert_eq!(total(&[40.0, 20.0]), 60.0 + GAP);
    }
}
