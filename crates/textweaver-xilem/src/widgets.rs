//! Small widgets the window is built from, each written for screen readers
//! first: an explicit accessible name on every control, the role a screen
//! reader expects, and nothing announced twice.
//!
//! - [`Root`]: the window's content. Keys nothing else handled go to the
//!   keymap from here, and it tells the document view when an
//!   accessibility pass rebuilds every node.
//! - [`Region`]: a panel with a role and a name (toolbar, status bar,
//!   header), drawn with Masonry's box properties.
//! - [`ActionButton`]: a button with its own name, keyboard shortcut, and
//!   description, which Masonry's `Button` cannot carry.
//! - [`Announcer`]: the live region. Each message is a fresh AccessKit node,
//!   so saying the same thing twice is still announced.

use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;

use masonry::accesskit::{Action, Live, Node, NodeId, Role};
use masonry::core::keyboard::{Key, KeyState, KeyboardEvent, NamedKey};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, NoAction,
    PaintCtx, PointerButtonEvent, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx,
    TextEvent, Update, UpdateCtx, Widget, WidgetMut, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LayoutSize, LenReq, Length, SizeDef};
use masonry::widgets::Label;
use textweaver_app::a11y::Priority;

// --- Root.

/// What the root sends the driver: a key nobody else handled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyAction(pub KeyboardEvent);

/// The window's content. See the module documentation.
pub struct Root {
    main: WidgetPod<Region>,
    dialog: Option<WidgetPod<dyn Widget>>,
    full_passes: Rc<Cell<u64>>,
}

impl Root {
    /// Wraps `main`. `full_passes` is shared with the document view.
    pub fn new(main: NewWidget<Region>, full_passes: Rc<Cell<u64>>) -> Self {
        Root {
            main: main.to_pod(),
            dialog: None,
            full_passes,
        }
    }

    /// Shows `dialog` over the window (or closes the open one with
    /// `None`). While a dialog is open, the window behind it is disabled
    /// and hidden from screen readers, so focus and reading stay inside.
    pub fn set_dialog(this: &mut WidgetMut<'_, Self>, dialog: Option<NewWidget<dyn Widget>>) {
        if let Some(old) = this.widget.dialog.take() {
            this.ctx.remove_child(old);
        }
        let open = dialog.is_some();
        this.widget.dialog = dialog.map(NewWidget::to_pod);
        {
            let mut main = this.ctx.get_mut(&mut this.widget.main);
            main.ctx.set_disabled(open);
            Region::set_hidden(&mut main, open);
        }
        this.ctx.children_changed();
        this.ctx.request_layout();
    }

    /// True while a dialog is open.
    pub fn has_dialog(&self) -> bool {
        self.dialog.is_some()
    }
}

impl Widget for Root {
    type Action = KeyAction;

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        if self.dialog.is_some() {
            return;
        }
        if let TextEvent::Keyboard(k) = event
            && k.state == KeyState::Down
            && !is_focus_key(k)
            && !k.is_composing
        {
            ctx.submit_action::<KeyAction>(KeyAction(k.clone()));
            ctx.set_handled();
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.main);
        if let Some(d) = &mut self.dialog {
            ctx.register_child(d);
        }
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        _len_req: LenReq,
        cross_length: Option<Length>,
    ) -> Length {
        ctx.redirect_measurement(&mut self.main, axis, cross_length)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.main, size);
        ctx.place_child(&mut self.main, Point::ORIGIN);
        if let Some(d) = &mut self.dialog {
            ctx.run_layout(d, size);
            ctx.place_child(d, Point::ORIGIN);
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
        // The root is only rebuilt when the whole tree is (the tree was
        // enabled, the window was resized): tell the document view.
        self.full_passes.set(self.full_passes.get().wrapping_add(1));
    }

    fn children_ids(&self) -> ChildrenIds {
        let mut ids = ChildrenIds::from_slice(&[self.main.id()]);
        if let Some(d) = &self.dialog {
            ids.push(d.id());
        }
        ids
    }
}

/// Tab and Shift+Tab move focus; the root leaves them to Masonry.
fn is_focus_key(k: &KeyboardEvent) -> bool {
    k.key == Key::Named(NamedKey::Tab) && !k.modifiers.ctrl() && !k.modifiers.alt()
}

// --- Region.

/// A panel with a role and a name. Its look comes from the box properties
/// set on it (background, border, corner radius, padding, shadow).
pub struct Region {
    child: WidgetPod<dyn Widget>,
    role: Role,
    label: String,
    hidden: bool,
}

impl Region {
    /// A region around `child`.
    pub fn new(
        child: NewWidget<impl Widget + ?Sized>,
        role: Role,
        label: impl Into<String>,
    ) -> Self {
        Region {
            child: child.erased().to_pod(),
            role,
            label: label.into(),
            hidden: false,
        }
    }

    /// Hides the region and everything in it from screen readers (while a
    /// dialog is open over it).
    pub fn set_hidden(this: &mut WidgetMut<'_, Self>, hidden: bool) {
        if this.widget.hidden != hidden {
            this.widget.hidden = hidden;
            this.ctx.request_accessibility_update();
        }
    }

    /// Changes the name (a status bar's name is its text).
    pub fn set_label(this: &mut WidgetMut<'_, Self>, label: impl Into<String>) {
        let label = label.into();
        if this.widget.label != label {
            this.widget.label = label;
            this.ctx.request_accessibility_update();
        }
    }

    /// The child.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

impl Widget for Region {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<Length>,
    ) -> Length {
        let context = LayoutSize::maybe(axis.cross(), cross_length);
        ctx.compute_length(&mut self.child, len_req.into(), context, axis, cross_length)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.child, size);
        ctx.place_child(&mut self.child, Point::ORIGIN);
        ctx.derive_baselines(&self.child);
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _p: &mut Painter<'_>) {
    }

    fn accessibility_role(&self) -> Role {
        self.role
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        if !self.label.is_empty() {
            node.set_label(self.label.as_str());
        }
        if self.hidden {
            node.set_hidden();
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}

// --- ActionButton.

/// An [`ActionButton`] was pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pressed;

/// A button with an explicit accessible name, shortcut, and description.
/// The visible text is a label child hidden from screen readers, so the
/// name is said once.
pub struct ActionButton {
    child: WidgetPod<Label>,
    label: String,
    shortcut: String,
    description: String,
}

impl ActionButton {
    /// A button showing and named `label`.
    pub fn new(label: impl Into<String>) -> Self {
        let label = label.into();
        ActionButton {
            child: NewWidget::new(Label::new(label.clone()).accessibility_hidden(true)).to_pod(),
            label,
            shortcut: String::new(),
            description: String::new(),
        }
    }

    /// Draws the text in `color` (the primary button's text on the
    /// accent).
    pub fn with_text_color(mut self, color: masonry::peniko::Color) -> Self {
        let label = self.label.clone();
        self.child = NewWidget::new(Label::new(label).accessibility_hidden(true))
            .with_props(masonry::properties::ContentColor::new(color))
            .to_pod();
        self
    }

    /// Adds the keyboard shortcut screen readers read after the name.
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = shortcut.into();
        self
    }

    /// Adds a description (read after a pause, or on request).
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Changes the text and name (Play becomes Pause).
    pub fn set_label(this: &mut WidgetMut<'_, Self>, label: impl Into<String>) {
        let label = label.into();
        if this.widget.label == label {
            return;
        }
        this.widget.label = label.clone();
        {
            let mut child = this.ctx.get_mut(&mut this.widget.child);
            Label::set_text(&mut child, label);
        }
        this.ctx.request_accessibility_update();
    }

    /// The name.
    pub fn label(&self) -> &str {
        &self.label
    }
}

impl Widget for ActionButton {
    type Action = Pressed;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down(..) => {
                ctx.request_focus();
                ctx.capture_pointer();
                ctx.request_paint_only();
            }
            PointerEvent::Up(PointerButtonEvent { .. }) => {
                if ctx.is_active() && ctx.is_hovered() {
                    ctx.submit_action::<Pressed>(Pressed);
                }
                ctx.request_paint_only();
            }
            _ => {}
        }
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        if let TextEvent::Keyboard(k) = event {
            let activate = matches!(&k.key, Key::Character(c) if c == " ")
                || k.key == Key::Named(NamedKey::Enter);
            if activate && k.modifiers.is_empty() {
                if k.state == KeyState::Up {
                    ctx.submit_action::<Pressed>(Pressed);
                }
                // Down and up both belong to the button, not the keymap.
                ctx.set_handled();
            }
        }
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        if event.action == Action::Click {
            ctx.submit_action::<Pressed>(Pressed);
            ctx.set_handled();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::FocusChanged(_) | Update::HoveredChanged(_)) {
            ctx.request_paint_only();
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<Length>,
    ) -> Length {
        let context = LayoutSize::maybe(axis.cross(), cross_length);
        let child =
            ctx.compute_length(&mut self.child, len_req.into(), context, axis, cross_length);
        match axis {
            Axis::Horizontal => child.max(Length::px(64.0)),
            Axis::Vertical => child.max(Length::px(22.0)),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let child_size = ctx.compute_size(&mut self.child, SizeDef::fit(size), size.into());
        ctx.run_layout(&mut self.child, child_size);
        let origin = ((size - child_size).to_vec2() * 0.5).to_point();
        ctx.place_child(&mut self.child, origin);
        ctx.derive_baselines(&self.child);
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _p: &mut Painter<'_>) {
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_label(self.label.as_str());
        if !self.shortcut.is_empty() {
            node.set_keyboard_shortcut(self.shortcut.as_str());
        }
        if !self.description.is_empty() {
            node.set_description(self.description.as_str());
        }
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }

    fn propagates_pointer_interaction(&self) -> bool {
        false
    }

    fn accepts_focus(&self) -> bool {
        true
    }
}

// --- Announcer.

/// A message waiting to be announced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// What to say.
    pub text: String,
    /// Polite waits for the current speech; assertive interrupts.
    pub priority: Priority,
}

/// Messages from the app's [`Announcer`](textweaver_app::a11y::Announcer)
/// to the [`Announcer`] widget, shared on the main thread.
pub type MessageQueue = Rc<std::cell::RefCell<VecDeque<Message>>>;

/// How many recent messages stay in the tree. Screen readers read a live
/// region's change when it happens; old messages are kept only so a quick
/// burst is not cut short by the next update.
const KEEP: usize = 3;

/// The live region: an invisible widget whose children are the latest
/// messages, each a new AccessKit node with `live` set, so every message
/// raises UI Automation's LiveRegionChanged and AT-SPI's Announcement, even
/// when the same words are repeated.
pub struct Announcer {
    pending: VecDeque<Message>,
    shown: VecDeque<(NodeId, Message)>,
    full_passes: Rc<Cell<u64>>,
    seen_full: u64,
    /// Every message announced, for the log and the tests.
    pub announced: usize,
}

impl Announcer {
    /// An announcer with nothing to say. `full_passes` is the root's count
    /// of passes that rebuild every node: on those, earlier messages are
    /// dropped rather than sent (and announced) again.
    pub fn new(full_passes: Rc<Cell<u64>>) -> Self {
        let seen_full = full_passes.get();
        Announcer {
            pending: VecDeque::new(),
            shown: VecDeque::new(),
            full_passes,
            seen_full,
            announced: 0,
        }
    }

    /// Queues `messages` to be announced on the next accessibility pass.
    pub fn say(this: &mut WidgetMut<'_, Self>, messages: impl IntoIterator<Item = Message>) {
        let before = this.widget.pending.len();
        this.widget.pending.extend(messages);
        if this.widget.pending.len() != before {
            this.ctx.request_accessibility_update();
        }
    }

    /// The messages currently in the tree, oldest first.
    pub fn shown(&self) -> impl Iterator<Item = &Message> {
        self.shown.iter().map(|(_, m)| m)
    }
}

fn message_node(m: &Message) -> Node {
    let mut n = Node::new(Role::Label);
    n.set_label(m.text.as_str());
    n.set_live(match m.priority {
        Priority::Polite => Live::Polite,
        Priority::Assertive => Live::Assertive,
    });
    n
}

impl Widget for Announcer {
    type Action = NoAction;

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        _axis: Axis,
        _len_req: LenReq,
        _cross_length: Option<Length>,
    ) -> Length {
        Length::ZERO
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _p: &mut Painter<'_>) {
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
        if self.full_passes.get() != self.seen_full {
            self.seen_full = self.full_passes.get();
            self.shown.clear();
        }
        while let Some(m) = self.pending.pop_front() {
            let id = AccessCtx::next_node_id();
            self.shown.push_back((id, m));
            self.announced += 1;
            while self.shown.len() > KEEP {
                self.shown.pop_front();
            }
        }
        // Every shown node goes in every update: unchanged ones raise no
        // event, and a rebuilt tree still finds each child.
        for (id, m) in &self.shown {
            ctx.tree_update().nodes.push((*id, message_node(m)));
        }
        node.set_children(self.shown.iter().map(|(id, _)| *id).collect::<Vec<_>>());
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
