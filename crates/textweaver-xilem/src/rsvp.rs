//! The RSVP panel: one word at a time, drawn large under the document
//! (ADR-0022; the GUI's design in ADR-0028).
//!
//! - **Never covers the caret.** The panel is its own strip in the window's
//!   column, between the document and the toolbar, so it can never lie over
//!   the text or the caret. RSVP's nine positions (`[reading_aids.rsvp]
//!   position`) move the word left, centre, or right within the strip.
//! - **Quiet for screen readers.** The flashing word is its own node,
//!   hidden, never live, and never focused: a screen reader following it
//!   would speak every word over the reading. Beside it, a status node
//!   ("RSVP paused, word 120 of 900") with its live setting off, which a
//!   screen reader finds by object navigation or review and which never
//!   interrupts. The app announces what RSVP does (on, off, paused) through
//!   the announcer, once.
//! - **Never colour alone.** The pivot letter (the optimal recognition
//!   point) is bold and underlined as well as coloured.

use masonry::accesskit::{Live, Node, NodeId, Role};
use masonry::core::{
    AccessCtx, BrushIndex, ChildrenIds, LayoutCtx, MeasureCtx, NoAction, PaintCtx, PropertiesRef,
    RegisterCtx, StyleProperty, Widget, WidgetMut, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, RoundedRect, Size, Stroke, Vec2};
use masonry::layout::{LenReq, Length};
use masonry::parley::style::{FontFamily, FontWeight};
use masonry::parley::{Alignment, AlignmentOptions, Layout};
use textweaver_app::App;
use textweaver_app::aids::RsvpPosition;
use textweaver_app::aids::rsvp::PlayState;
use textweaver_app::lexicon::args;

use crate::theme::{self, Palette};

/// The panel's height while RSVP shows, in logical pixels.
const HEIGHT: f64 = 104.0;
/// The word's size.
const WORD_SIZE: f32 = 40.0;
/// The context words' size.
const CONTEXT_SIZE: f32 = 18.0;

/// What the panel shows: the word, split at its pivot, the words around
/// it when RSVP shows them, and the status.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RsvpShown {
    /// The word before the pivot.
    pub before: String,
    /// The pivot grapheme.
    pub pivot: String,
    /// The word after the pivot.
    pub after: String,
    /// The previous word, when shown.
    pub previous: Option<String>,
    /// The next word, when shown.
    pub next: Option<String>,
    /// The status node's text: "RSVP paused, word 120 of 900".
    pub status: String,
    /// The panel's name ("RSVP"), in the interface language.
    pub name: String,
    /// Where the word sits across the panel, 0 (left) to 1 (right).
    pub x: f64,
}

impl RsvpShown {
    /// What the panel shows for the app's RSVP, or `None` when it is off.
    pub fn from_app(app: &App) -> Option<RsvpShown> {
        let r = app.rsvp()?;
        let f = r.frame()?;
        let id = match r.state() {
            PlayState::Playing { .. } => "gui-rsvp-playing",
            PlayState::Paused => "gui-rsvp-paused",
            PlayState::Finished => "gui-rsvp-finished",
        };
        let c = app.catalog();
        let position = RsvpPosition::from(app.settings().reading_aids.rsvp.position);
        Some(RsvpShown {
            before: f.before.to_owned(),
            pivot: f.pivot.to_owned(),
            after: f.after.to_owned(),
            previous: f.previous.map(str::to_owned),
            next: f.next.map(str::to_owned),
            status: c.fmt(id, &args!["n" => f.index + 1, "total" => f.total]),
            name: c.tr("gui-rsvp"),
            x: position.fractions().0,
        })
    }

    /// The whole word.
    pub fn word(&self) -> String {
        format!("{}{}{}", self.before, self.pivot, self.after)
    }
}

/// The RSVP panel widget. See the module documentation.
pub struct RsvpView {
    palette: Palette,
    shown: Option<RsvpShown>,
    word: Option<Layout<BrushIndex>>,
    previous: Option<Layout<BrushIndex>>,
    next: Option<Layout<BrushIndex>>,
    word_id: NodeId,
    status_id: NodeId,
}

impl std::fmt::Debug for RsvpView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RsvpView")
            .field("shown", &self.shown)
            .finish_non_exhaustive()
    }
}

// Brushes: the text, the pivot, the context words.
const B_TEXT: usize = 0;
const B_PIVOT: usize = 1;
const B_CONTEXT: usize = 2;

impl RsvpView {
    /// A hidden panel.
    pub fn new(palette: Palette) -> Self {
        RsvpView {
            palette,
            shown: None,
            word: None,
            previous: None,
            next: None,
            word_id: AccessCtx::next_node_id(),
            status_id: AccessCtx::next_node_id(),
        }
    }

    /// Shows `shown`, or hides the panel with `None`.
    pub fn set_shown(this: &mut WidgetMut<'_, Self>, shown: Option<RsvpShown>) {
        if this.widget.shown == shown {
            return;
        }
        this.widget.shown = shown;
        this.widget.word = None;
        this.widget.previous = None;
        this.widget.next = None;
        // The words are laid out in `layout`; showing or hiding the panel
        // also changes its height.
        this.ctx.request_layout();
        this.ctx.request_render();
    }

    /// Changes the colours.
    pub fn set_palette(this: &mut WidgetMut<'_, Self>, palette: Palette) {
        this.widget.palette = palette;
        this.ctx.request_render();
    }

    /// What the panel shows, if anything.
    pub fn shown(&self) -> Option<&RsvpShown> {
        self.shown.as_ref()
    }

    fn brushes(&self) -> [masonry::peniko::Brush; 3] {
        let p = &self.palette;
        [
            theme::color(p.text).into(),
            theme::color(p.accent).into(),
            theme::color(p.dim_text).into(),
        ]
    }

    fn build(
        ctx: &mut LayoutCtx<'_>,
        text: &str,
        size: f32,
        brush: usize,
        pivot: Option<std::ops::Range<usize>>,
    ) -> Layout<BrushIndex> {
        let (fcx, lcx) = ctx.text_contexts();
        let mut b = lcx.ranged_builder(fcx, text, 1.0, true);
        b.push_default(StyleProperty::FontFamily(FontFamily::Source(
            crate::fonts::DEFAULT_STACK.into(),
        )));
        b.push_default(StyleProperty::FontSize(size));
        b.push_default(StyleProperty::Brush(BrushIndex(brush)));
        if let Some(r) = pivot {
            b.push(StyleProperty::Brush(BrushIndex(B_PIVOT)), r.clone());
            b.push(StyleProperty::FontWeight(FontWeight::BOLD), r.clone());
            b.push(StyleProperty::Underline(true), r);
        }
        let mut layout = b.build(text);
        layout.break_all_lines(None);
        layout.align(Alignment::Start, AlignmentOptions::default());
        layout
    }
}

impl Widget for RsvpView {
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
        _cross: Option<Length>,
    ) -> Length {
        match (axis, self.shown.is_some()) {
            (Axis::Vertical, true) => Length::px(HEIGHT),
            (Axis::Vertical, false) => Length::ZERO,
            (Axis::Horizontal, _) => match len_req {
                LenReq::FitContent(space) => space,
                _ => Length::px(200.0),
            },
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {
        let Some(s) = self.shown.clone() else {
            return;
        };
        let word = s.word();
        let a = s.before.len();
        let pivot = a..a + s.pivot.len();
        self.word = Some(Self::build(ctx, &word, WORD_SIZE, B_TEXT, Some(pivot)));
        self.previous = s
            .previous
            .as_deref()
            .map(|t| Self::build(ctx, t, CONTEXT_SIZE, B_CONTEXT, None));
        self.next = s
            .next
            .as_deref()
            .map(|t| Self::build(ctx, t, CONTEXT_SIZE, B_CONTEXT, None));
    }

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        let (Some(s), Some(word)) = (&self.shown, &self.word) else {
            return;
        };
        let p = &self.palette;
        let size = ctx.content_box().size();
        let card = RoundedRect::from_rect(size.to_rect().inset(-4.0), theme::PANEL_RADIUS);
        painter.fill(card, theme::color(p.surface)).draw();
        painter
            .stroke(card, &Stroke::new(1.0), theme::color(p.border))
            .draw();
        let brushes = self.brushes();
        // The word: its pivot at the chosen place across the panel, as RSVP
        // readers expect the eye to rest on the same spot.
        let before_w = f64::from(Self::prefix_width(word, s.before.len()).unwrap_or(0.0));
        let pivot_w =
            f64::from(Self::prefix_width(word, s.before.len() + s.pivot.len()).unwrap_or(0.0))
                - before_w;
        let w = f64::from(word.width());
        let margin = 24.0;
        let anchor = margin + (size.width - 2.0 * margin) * s.x.clamp(0.0, 1.0);
        let x = (anchor - before_w - pivot_w / 2.0)
            .clamp(margin, (size.width - margin - w).max(margin));
        let y = (size.height - f64::from(word.height())) / 2.0;
        render_text(
            painter,
            Affine::translate(Vec2::new(x, y)),
            word,
            &brushes,
            false,
        );
        let cy = |l: &Layout<BrushIndex>| (size.height - f64::from(l.height())) / 2.0;
        if let Some(prev) = &self.previous {
            let px = (x - 20.0 - f64::from(prev.width())).max(margin / 2.0);
            if px + f64::from(prev.width()) < x - 8.0 {
                render_text(
                    painter,
                    Affine::translate(Vec2::new(px, cy(prev))),
                    prev,
                    &brushes,
                    false,
                );
            }
        }
        if let Some(next) = &self.next {
            let nx = x + w + 20.0;
            if nx + f64::from(next.width()) < size.width - margin / 2.0 {
                render_text(
                    painter,
                    Affine::translate(Vec2::new(nx, cy(next))),
                    next,
                    &brushes,
                    false,
                );
            }
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        let Some(s) = &self.shown else {
            node.set_hidden();
            node.set_children(Vec::<NodeId>::new());
            return;
        };
        // The flashing word: hidden, never live, never focusable.
        let mut word = Node::new(Role::Label);
        word.set_value(s.word());
        word.set_hidden();
        word.set_live(Live::Off);
        // The quiet status: found by review, never spoken by itself.
        let mut status = Node::new(Role::Status);
        status.set_label(s.status.as_str());
        status.set_live(Live::Off);
        ctx.tree_update().nodes.push((self.word_id, word));
        ctx.tree_update().nodes.push((self.status_id, status));
        node.set_label(s.name.as_str());
        node.set_children(vec![self.status_id, self.word_id]);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn get_debug_text(&self) -> Option<String> {
        self.shown.as_ref().map(|s| s.status.clone())
    }
}

impl RsvpView {
    /// The width of the first `bytes` of the laid-out word.
    fn prefix_width(layout: &Layout<BrushIndex>, bytes: usize) -> Option<f32> {
        let c = masonry::parley::Cursor::from_byte_index(
            layout,
            bytes,
            masonry::parley::Affinity::Downstream,
        );
        Some(c.geometry(layout, 1.0).x0 as f32)
    }
}
