//! Dialogs inside the window: a modal card over a dimmed page, with the
//! `Dialog` role, a name, and focus kept inside it. Two kinds:
//!
//! - a prompt (find, go to, open, the command palette): a labelled text
//!   field with OK and Cancel;
//! - a list (bookmarks, help, voices, the library): a [`ChoiceList`] with
//!   the `ListBox` role, arrow keys, Home and End, Page Up and Page Down,
//!   first-letter search, Enter to choose, and Escape to close.
//!
//! In-window dialogs never take the foreground from another program, so
//! automated runs with `--background` can open them too.

use masonry::accesskit::{Action, ActionData, Node, NodeId, Role};
use masonry::core::keyboard::{Key, KeyState, NamedKey};
use masonry::core::{
    AccessCtx, AccessEvent, BrushIndex, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget,
    PaintCtx, PointerButtonEvent, PointerEvent, PointerScrollEvent, PropertiesMut, PropertiesRef,
    RegisterCtx, StyleProperty, TextEvent, Update, UpdateCtx, Widget, WidgetMut, WidgetPod,
    render_text,
};
use masonry::dpi::{LogicalPosition, PhysicalPosition};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Rect, RoundedRect, Size, Stroke, Vec2};
use masonry::layout::{LayoutSize, LenReq, Length};
use masonry::parley::Layout;
use masonry::parley::style::{FontFamily, LineHeight};
use masonry::peniko::Color;

use crate::theme::{self, Palette};

/// What a dialog tells the driver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DialogAction {
    /// The list's item at this index was chosen.
    Choose(usize),
    /// The dialog was closed without an answer.
    Cancel,
}

// --- Modal.

/// A modal card centred over the window, over a dimmed page. It measures
/// to the window's size, so the root can give it the whole window.
pub struct Modal {
    card: WidgetPod<dyn Widget>,
    label: String,
    palette: Palette,
    max_width: f64,
}

impl Modal {
    /// A dialog named `label` holding `card` (usually a column of
    /// controls, drawn as a panel).
    pub fn new(
        card: NewWidget<impl Widget + ?Sized>,
        label: impl Into<String>,
        palette: Palette,
    ) -> Self {
        Modal {
            card: card.erased().to_pod(),
            label: label.into(),
            palette,
            max_width: 600.0,
        }
    }
}

impl Widget for Modal {
    type Action = DialogAction;

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        if let TextEvent::Keyboard(k) = event
            && k.state == KeyState::Down
            && k.key == Key::Named(NamedKey::Escape)
        {
            ctx.submit_action::<DialogAction>(DialogAction::Cancel);
            ctx.set_handled();
        }
    }

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        // Clicks on the dimmed page do nothing, and go no further.
        if matches!(event, PointerEvent::Down(..) | PointerEvent::Up(..)) {
            ctx.set_handled();
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.card);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        _cross_length: Option<Length>,
    ) -> Length {
        match len_req {
            LenReq::FitContent(space) => space,
            _ => ctx.context_size().length(axis).unwrap_or(Length::px(600.0)),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let width = (size.width - 48.0).clamp(240.0, self.max_width);
        let avail = Size::new(width, (size.height - 48.0).max(120.0));
        // The card's own height at this width, up to the window's.
        let height = ctx.compute_length(
            &mut self.card,
            LenReq::MaxContent.into(),
            LayoutSize::maybe(Axis::Horizontal, Some(Length::px(width))),
            Axis::Vertical,
            Some(Length::px(width)),
        );
        let card = Size::new(width, height.get().min(avail.height));
        ctx.run_layout(&mut self.card, card);
        let x = ((size.width - card.width) / 2.0).max(0.0);
        // A little above the middle, where the eye expects a dialog.
        let y = ((size.height - card.height) * 0.38).max(24.0);
        ctx.place_child(&mut self.card, Point::new(x, y));
    }

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        let size = ctx.content_box().size();
        let dim = if self.palette.background.is_dark() {
            0.55
        } else {
            0.35
        };
        painter
            .fill(size.to_rect(), Color::BLACK.with_alpha(dim))
            .draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::Dialog
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_label(self.label.as_str());
        node.set_modal();
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.card.id()])
    }
}

// --- ChoiceList.

/// Rows kept laid out around the visible ones.
const ROW_PAD: f64 = 10.0;

/// A list of choices: one focusable `ListBox` whose options are AccessKit
/// nodes (not widgets), with the selected one reported as selected and
/// the list's active descendant.
pub struct ChoiceList {
    items: Vec<String>,
    selected: usize,
    label: String,
    palette: Palette,
    focused: bool,
    top: usize,
    row_h: f64,
    visible_rows: usize,
    layouts: Vec<Option<Layout<BrushIndex>>>,
    option_ids: Vec<NodeId>,
    width: f64,
}

impl ChoiceList {
    /// A list named `label`.
    pub fn new(label: impl Into<String>, items: Vec<String>, palette: Palette) -> Self {
        let n = items.len();
        ChoiceList {
            items,
            selected: 0,
            label: label.into(),
            palette,
            focused: false,
            top: 0,
            row_h: 36.0,
            visible_rows: 10,
            layouts: (0..n).map(|_| None).collect(),
            option_ids: Vec::new(),
            width: 0.0,
        }
    }

    /// Starts on item `i`.
    pub fn with_selected(mut self, i: usize) -> Self {
        self.selected = i.min(self.items.len().saturating_sub(1));
        self
    }

    /// The selected index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Replaces the items (a filtered list), selecting the first.
    pub fn set_items(this: &mut WidgetMut<'_, Self>, items: Vec<String>) {
        let w = &mut *this.widget;
        w.layouts = (0..items.len()).map(|_| None).collect();
        w.items = items;
        w.selected = 0;
        w.top = 0;
        this.ctx.request_layout();
        this.ctx.request_render();
    }

    /// Moves the selection (for the driver and tests).
    pub fn select(this: &mut WidgetMut<'_, Self>, i: usize) {
        this.widget.set_selected(i);
        this.ctx.request_render();
    }

    fn set_selected(&mut self, i: usize) {
        if self.items.is_empty() {
            return;
        }
        self.selected = i.min(self.items.len() - 1);
        if self.selected < self.top {
            self.top = self.selected;
        } else if self.selected >= self.top + self.visible_rows {
            self.top = self.selected + 1 - self.visible_rows;
        }
    }

    fn first_letter(&self, c: char) -> Option<usize> {
        let c = c.to_lowercase().next()?;
        let n = self.items.len();
        (1..=n).map(|k| (self.selected + k) % n).find(|&i| {
            self.items[i]
                .trim_start()
                .chars()
                .next()
                .and_then(|f| f.to_lowercase().next())
                == Some(c)
        })
    }
}

impl Widget for ChoiceList {
    type Action = DialogAction;

    fn accepts_focus(&self) -> bool {
        true
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if let Update::FocusChanged(f) = event {
            self.focused = *f;
            ctx.request_render();
        }
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        let TextEvent::Keyboard(k) = event else {
            return;
        };
        if k.state != KeyState::Down || k.modifiers.ctrl() || k.modifiers.alt() {
            return;
        }
        let n = self.items.len();
        if n == 0 {
            return;
        }
        let page = self.visible_rows.max(2) - 1;
        let new = match &k.key {
            Key::Named(NamedKey::ArrowDown) => Some((self.selected + 1).min(n - 1)),
            Key::Named(NamedKey::ArrowUp) => Some(self.selected.saturating_sub(1)),
            Key::Named(NamedKey::Home) => Some(0),
            Key::Named(NamedKey::End) => Some(n - 1),
            Key::Named(NamedKey::PageDown) => Some((self.selected + page).min(n - 1)),
            Key::Named(NamedKey::PageUp) => Some(self.selected.saturating_sub(page)),
            Key::Named(NamedKey::Enter) => {
                ctx.submit_action::<DialogAction>(DialogAction::Choose(self.selected));
                ctx.set_handled();
                return;
            }
            Key::Character(s) if s != " " => s.chars().next().and_then(|c| self.first_letter(c)),
            _ => return,
        };
        if let Some(i) = new {
            self.set_selected(i);
            ctx.request_render();
        }
        ctx.set_handled();
    }

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down(PointerButtonEvent { state, .. }) => {
                ctx.request_focus();
                let local = ctx.local_position(state.position);
                let row = self.top + (local.y / self.row_h).max(0.0) as usize;
                if row < self.items.len() {
                    if row == self.selected && state.count >= 2 {
                        ctx.submit_action::<DialogAction>(DialogAction::Choose(row));
                    }
                    self.set_selected(row);
                    ctx.request_render();
                }
                ctx.set_handled();
            }
            PointerEvent::Scroll(PointerScrollEvent { delta, .. }) => {
                let scale = ctx.scale_factor();
                let px = delta.to_pixel_delta(
                    PhysicalPosition::new(self.row_h * scale, self.row_h * scale),
                    PhysicalPosition::new(self.row_h * 8.0 * scale, self.row_h * 8.0 * scale),
                );
                let LogicalPosition { y, .. } = px.to_logical::<f64>(scale);
                let rows = (-y / self.row_h).round() as isize;
                let max_top = self.items.len().saturating_sub(self.visible_rows);
                self.top = (self.top as isize + rows).clamp(0, max_top as isize) as usize;
                ctx.request_render();
                ctx.set_handled();
            }
            _ => {}
        }
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        // Screen readers choose an option by clicking or focusing it.
        if let Some(ActionData::NumericValue(v)) = &event.data
            && event.action == Action::SetValue
        {
            self.set_selected(*v as usize);
            ctx.request_render();
        }
    }

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        _cross: Option<Length>,
    ) -> Length {
        match axis {
            Axis::Horizontal => match len_req {
                LenReq::FitContent(space) => space,
                _ => Length::px(480.0),
            },
            Axis::Vertical => {
                let rows = self.items.len().clamp(1, 12) as f64;
                Length::px(rows * self.row_h)
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        self.visible_rows = ((size.height / self.row_h).floor() as usize).max(1);
        if (size.width - self.width).abs() > 0.5 {
            self.width = size.width;
            self.layouts.iter_mut().for_each(|l| *l = None);
        }
        let last = (self.top + self.visible_rows + 1).min(self.items.len());
        let (fcx, lcx) = ctx.text_contexts();
        for i in self.top..last {
            if self.layouts[i].is_none() {
                let text = self.items[i].as_str();
                let mut b = lcx.ranged_builder(fcx, text, 1.0, true);
                b.push_default(StyleProperty::FontFamily(FontFamily::Source(
                    crate::fonts::DEFAULT_STACK.into(),
                )));
                b.push_default(StyleProperty::FontSize(theme::UI_TEXT + 1.0));
                b.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(1.3)));
                b.push_default(StyleProperty::Brush(BrushIndex(0)));
                let mut l = b.build(text);
                l.break_all_lines(None);
                self.layouts[i] = Some(l);
            }
        }
        ctx.set_clip_path(size.to_rect());
    }

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        let p = &self.palette;
        let size = ctx.content_box().size();
        let last = (self.top + self.visible_rows + 1).min(self.items.len());
        for i in self.top..last {
            let y = (i - self.top) as f64 * self.row_h;
            let row = Rect::new(0.0, y, size.width, y + self.row_h);
            let selected = i == self.selected;
            let fg = if selected { p.on_accent } else { p.text };
            if selected {
                let fill = if self.focused { p.accent } else { p.raised };
                let fg_fill = theme::color(fill);
                painter
                    .fill(
                        RoundedRect::from_rect(row.inset(-2.0), theme::RADIUS),
                        fg_fill,
                    )
                    .draw();
            }
            let text_fg = if selected && !self.focused {
                p.text
            } else {
                fg
            };
            if let Some(l) = &self.layouts[i] {
                let ty = y + (self.row_h - f64::from(l.height())) / 2.0;
                render_text(
                    painter,
                    Affine::translate(Vec2::new(ROW_PAD + 4.0, ty)),
                    l,
                    &[theme::color(text_fg).into()],
                    true,
                );
            }
        }
        if self.focused {
            painter
                .stroke(
                    RoundedRect::from_rect(size.to_rect().inset(-1.0), theme::RADIUS),
                    &Stroke::new(theme::FOCUS_WIDTH),
                    theme::color(p.focus),
                )
                .draw();
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::ListBox
    }

    fn accessibility(
        &mut self,
        ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_label(self.label.as_str());
        while self.option_ids.len() < self.items.len() {
            self.option_ids.push(AccessCtx::next_node_id());
        }
        let n = self.items.len();
        for (i, item) in self.items.iter().enumerate() {
            let mut o = Node::new(Role::ListBoxOption);
            o.set_label(item.as_str());
            o.set_selected(i == self.selected);
            o.set_position_in_set(i + 1);
            o.set_size_of_set(n);
            let y = (i as f64 - self.top as f64) * self.row_h;
            o.set_bounds(masonry::accesskit::Rect::new(
                0.0,
                y,
                self.width,
                y + self.row_h,
            ));
            ctx.tree_update().nodes.push((self.option_ids[i], o));
        }
        node.set_children(self.option_ids[..n].to_vec());
        if let Some(id) = self.option_ids.get(self.selected) {
            node.set_active_descendant(*id);
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

/// A column laid out as a dialog's card: the panel look from `palette`.
pub fn card_props(p: &Palette) -> impl Into<masonry::core::PropertySet> {
    use masonry::properties::Padding;
    let (bg, border, bw, radius, shadow) = theme::panel_props(p);
    let shadow = if p.background.is_dark() {
        masonry::properties::BoxShadow::new(Color::BLACK.with_alpha(0.5), (0.0, 8.0))
            .blur(Length::px(24.0))
    } else {
        shadow
    };
    (
        bg,
        border,
        bw,
        radius,
        shadow,
        Padding::all(Length::px(22.0)),
    )
}
