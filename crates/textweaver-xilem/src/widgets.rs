//! Small widgets the window is built from, each written for screen readers
//! first: an explicit accessible name on every control, the role a screen
//! reader expects, and nothing announced twice.
//!
//! - [`Root`]: the window's content. Keys nothing else handled go to the
//!   keymap from here, and its [`FullPassProbe`] tells the document view
//!   and the announcer when an accessibility pass rebuilds every node.
//! - [`Region`]: a panel with a role and a name (toolbar, status bar,
//!   header), drawn with Masonry's box properties.
//! - [`ActionButton`]: a button with its own name, keyboard shortcut, and
//!   description, which Masonry's `Button` cannot carry.
//! - [`Announcer`]: the live region. Each message is a fresh AccessKit node,
//!   so saying the same thing twice is still announced.
//! - [`AnnounceMode`] and [`notify`]: the second way to announce on
//!   Windows, a UI Automation Notification event raised directly
//!   (`--announce uia`), for comparing by ear with the live region.

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
    probe: WidgetPod<FullPassProbe>,
    main: WidgetPod<Region>,
    dialog: Option<WidgetPod<dyn Widget>>,
}

impl Root {
    /// Wraps `main`. `full_passes` is shared with the document view.
    pub fn new(main: NewWidget<Region>, full_passes: Rc<Cell<u64>>) -> Self {
        Root {
            probe: NewWidget::new(FullPassProbe { full_passes }).to_pod(),
            main: main.to_pod(),
            dialog: None,
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
        ctx.register_child(&mut self.probe);
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
        ctx.run_layout(&mut self.probe, Size::ZERO);
        ctx.place_child(&mut self.probe, Point::ORIGIN);
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
    }

    fn children_ids(&self) -> ChildrenIds {
        let mut ids = ChildrenIds::from_slice(&[self.probe.id(), self.main.id()]);
        if let Some(d) = &self.dialog {
            ids.push(d.id());
        }
        ids
    }
}

/// Counts the accessibility passes that rebuild every node (a screen reader
/// asked for the tree, or asked again after a restart), for the document
/// view and the announcer, which then send everything they hold again.
///
/// It is the root's first child: zero-sized at the origin, it is never laid
/// out again or moved, so Masonry rebuilds its node only when it rebuilds
/// them all. The root itself cannot tell: it is laid out again, and its
/// node rebuilt, whenever anything in the window asks for layout, so every
/// caret move and every key in edit mode looked like a full pass.
pub struct FullPassProbe {
    full_passes: Rc<Cell<u64>>,
}

impl Widget for FullPassProbe {
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
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_hidden();
        self.full_passes.set(self.full_passes.get().wrapping_add(1));
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

/// Tab and Shift+Tab move focus, and so do Ctrl+Tab and Ctrl+Shift+Tab
/// (which leave a multi-line edit, where Tab types, as in Windows' own
/// edits); the root leaves them to Masonry.
fn is_focus_key(k: &KeyboardEvent) -> bool {
    k.key == Key::Named(NamedKey::Tab) && !k.modifiers.alt()
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
///
/// The shortcut is the node's keyboard shortcut property (UI Automation's
/// AcceleratorKey, AT-SPI's), which NVDA and JAWS say when their "report
/// shortcut keys" setting is on, and it is on screen, written ("Open…
/// (Ctrl+O)"). The name is the label only ("Open"): the owner found the key
/// in the name wordy (Monday, September 28, 2026).
pub struct ActionButton {
    child: WidgetPod<Label>,
    label: String,
    /// The shortcut as written ("Ctrl+O").
    shortcut: String,
    description: String,
}

impl ActionButton {
    /// A button showing and named `label`.
    pub fn new(label: impl Into<String>) -> Self {
        let label = label.into();
        ActionButton {
            child: NewWidget::new(button_text(label.clone())).to_pod(),
            label,
            shortcut: String::new(),
            description: String::new(),
        }
    }

    /// The text on screen: the label, then the shortcut as written.
    pub fn shown_text(&self) -> String {
        if self.shortcut.is_empty() {
            self.label.clone()
        } else {
            format!("{} ({})", self.label, self.shortcut)
        }
    }

    /// The accessible name: the label without a trailing ellipsis ("Open").
    pub fn name(&self) -> String {
        self.label.trim_end_matches('…').trim_end().to_owned()
    }

    /// Draws the text in `color` (the primary button's text on the
    /// accent).
    pub fn with_text_color(mut self, color: masonry::peniko::Color) -> Self {
        let text = self.shown_text();
        self.child = NewWidget::new(button_text(text))
            .with_props(masonry::properties::ContentColor::new(color))
            .to_pod();
        self
    }

    /// Changes the text colour (a new theme).
    pub fn set_text_color(this: &mut WidgetMut<'_, Self>, color: masonry::peniko::Color) {
        let mut child = this.ctx.get_mut(&mut this.widget.child);
        child.insert_prop(masonry::properties::ContentColor::new(color));
    }

    /// Adds the keyboard shortcut, as written ("Ctrl+O"): the node's
    /// keyboard shortcut, and on screen. Call it before
    /// [`with_text_color`](Self::with_text_color).
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = shortcut.into();
        self.child = NewWidget::new(button_text(self.shown_text())).to_pod();
        self
    }

    /// Changes the shortcut (single-key shortcuts turned on or off).
    pub fn set_shortcut(this: &mut WidgetMut<'_, Self>, shortcut: impl Into<String>) {
        let shortcut = shortcut.into();
        if this.widget.shortcut == shortcut {
            return;
        }
        this.widget.shortcut = shortcut;
        let text = this.widget.shown_text();
        {
            let mut child = this.ctx.get_mut(&mut this.widget.child);
            Label::set_text(&mut child, text);
        }
        this.ctx.request_layout();
        this.ctx.request_accessibility_update();
    }

    /// The shortcut as written, or empty.
    pub fn shortcut(&self) -> &str {
        &self.shortcut
    }

    /// Adds a description (read after a pause, or on request).
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Changes the description (the interface language changed).
    pub fn set_description(this: &mut WidgetMut<'_, Self>, description: impl Into<String>) {
        this.widget.description = description.into();
        this.ctx.request_accessibility_update();
    }

    /// Changes the text and name (Play becomes Pause).
    pub fn set_label(this: &mut WidgetMut<'_, Self>, label: impl Into<String>) {
        let label = label.into();
        if this.widget.label == label {
            return;
        }
        this.widget.label = label;
        let text = this.widget.shown_text();
        {
            let mut child = this.ctx.get_mut(&mut this.widget.child);
            Label::set_text(&mut child, text);
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
        node.set_label(self.name());
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

/// A button's visible text: the interface font, hidden from screen
/// readers (the button carries the name).
fn button_text(text: String) -> Label {
    use masonry::core::StyleProperty;
    use masonry::parley::style::FontFamily;
    Label::new(text)
        .with_style(StyleProperty::FontFamily(FontFamily::Source(
            crate::fonts::DEFAULT_STACK.into(),
        )))
        .with_style(StyleProperty::FontSize(crate::theme::UI_TEXT))
        .accessibility_hidden(true)
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

/// How announcements reach the screen reader: `--announce live|uia`, or
/// the `announce` key of the `[gui]` settings table.
///
/// Both keep the messages in the tree as the announcer's children, so a
/// screen reader's object navigation and the UI Automation report find
/// them. Only the event that makes a screen reader speak differs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnnounceMode {
    /// The live region (the default): each message is a new node with its
    /// live setting, and AccessKit raises UI Automation's LiveRegionChanged
    /// (AT-SPI's Announcement on Linux). NVDA speaks it; JAWS has been
    /// inconsistent.
    #[default]
    Live,
    /// Windows only: a UI Automation Notification event, raised directly
    /// with `UiaRaiseNotificationEvent` on the window, for each message
    /// (Windows 10 1709 or later). The message nodes stay in the tree with
    /// their live setting off, so the screen reader is not told twice. On
    /// other systems this is the same as [`Live`](Self::Live).
    Uia,
}

impl AnnounceMode {
    /// The names `--announce` and the setting take.
    pub const NAMES: [&'static str; 2] = ["live", "uia"];

    /// Reads a name: `live` or `uia`, in any case.
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "live" => Some(AnnounceMode::Live),
            "uia" => Some(AnnounceMode::Uia),
            _ => None,
        }
    }

    /// The name, as [`parse`](Self::parse) reads it.
    pub fn name(self) -> &'static str {
        match self {
            AnnounceMode::Live => "live",
            AnnounceMode::Uia => "uia",
        }
    }

    /// The mode this system can use: [`Uia`](Self::Uia) is Windows only,
    /// and becomes [`Live`](Self::Live) elsewhere.
    pub fn effective(self) -> Self {
        if cfg!(windows) {
            self
        } else {
            AnnounceMode::Live
        }
    }

    /// True when message nodes carry a live setting.
    pub fn uses_live_region(self) -> bool {
        self.effective() == AnnounceMode::Live
    }
}

/// The live region: an invisible widget whose children are the latest
/// messages, each a new AccessKit node with `live` set, so every message
/// raises UI Automation's LiveRegionChanged and AT-SPI's Announcement, even
/// when the same words are repeated. With [`AnnounceMode::Uia`] the nodes
/// are kept with their live setting off, and the driver raises a
/// Notification event for each message instead ([`notify`]).
pub struct Announcer {
    pending: VecDeque<Message>,
    shown: VecDeque<(NodeId, Message)>,
    full_passes: Rc<Cell<u64>>,
    seen_full: u64,
    mode: AnnounceMode,
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
            mode: AnnounceMode::Live,
            announced: 0,
        }
    }

    /// The same announcer, announcing the `mode` way.
    pub fn with_mode(mut self, mode: AnnounceMode) -> Self {
        self.mode = mode;
        self
    }

    /// How messages are announced.
    pub fn mode(&self) -> AnnounceMode {
        self.mode
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

fn message_node(m: &Message, live: bool) -> Node {
    // A Label's name comes from its value in AccessKit.
    let mut n = Node::new(Role::Label);
    n.set_value(m.text.as_str());
    n.set_live(match (live, m.priority) {
        (false, _) => Live::Off,
        (true, Priority::Polite) => Live::Polite,
        (true, Priority::Assertive) => Live::Assertive,
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
        let live = self.mode.uses_live_region();
        for (id, m) in &self.shown {
            ctx.tree_update().nodes.push((*id, message_node(m, live)));
        }
        node.set_children(self.shown.iter().map(|(id, _)| *id).collect::<Vec<_>>());
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

// --- UI Automation notifications.

/// UI Automation Notification events, raised directly on the window
/// ([`AnnounceMode::Uia`]). AccessKit raises only LiveRegionChanged, which
/// JAWS has handled inconsistently; NVDA (2018.1 and later) and JAWS both
/// handle Notification events. The owner compares the two by ear in the
/// listening session (ADR-0028).
pub mod notify {
    use super::Message;
    #[cfg(windows)]
    use textweaver_app::a11y::Priority;

    /// The activity id sent with each notification. Screen readers may use
    /// it to group or configure an application's notifications.
    pub const ACTIVITY: &str = "textweaver.announcement";

    /// Raises one Notification event on the window `hwnd` (a Win32 window
    /// handle) saying `message`. Assertive messages are sent as "important,
    /// most recent" (they may cut off the one before); polite ones as "all"
    /// (queued). The kind is "other".
    ///
    /// # Errors
    /// The error from UI Automation, or, on other systems, that there is no
    /// UI Automation.
    pub fn raise(hwnd: isize, message: &Message) -> Result<(), String> {
        #[cfg(windows)]
        {
            let important = message.priority == Priority::Assertive;
            imp::raise(hwnd, &message.text, important)
        }
        #[cfg(not(windows))]
        {
            let _ = (hwnd, message);
            Err("UI Automation notifications exist only on Windows".into())
        }
    }

    #[cfg(windows)]
    #[allow(unsafe_code)]
    mod imp {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::System::Variant::VARIANT;
        use windows::Win32::UI::Accessibility::{
            IRawElementProviderSimple, IRawElementProviderSimple_Impl, NotificationKind_Other,
            NotificationProcessing_All, NotificationProcessing_ImportantMostRecent,
            ProviderOptions, ProviderOptions_ServerSideProvider, ProviderOptions_UseComThreading,
            UIA_PATTERN_ID, UIA_PROPERTY_ID, UiaHostProviderFromHwnd, UiaRaiseNotificationEvent,
        };
        use windows::core::{BSTR, Error, IUnknown, Result, implement};

        /// The window as a server-side provider with no properties of its
        /// own: UI Automation merges it with the window's element (whose
        /// provider is AccessKit's), so the event comes from the window.
        /// The host provider on its own is a client-side provider, and UI
        /// Automation does not pass on events raised on it.
        #[implement(IRawElementProviderSimple)]
        struct WindowSource {
            hwnd: isize,
        }

        impl IRawElementProviderSimple_Impl for WindowSource_Impl {
            fn ProviderOptions(&self) -> Result<ProviderOptions> {
                Ok(ProviderOptions_ServerSideProvider | ProviderOptions_UseComThreading)
            }

            fn GetPatternProvider(&self, _pattern: UIA_PATTERN_ID) -> Result<IUnknown> {
                Err(Error::empty())
            }

            fn GetPropertyValue(&self, _property: UIA_PROPERTY_ID) -> Result<VARIANT> {
                Ok(VARIANT::default())
            }

            fn HostRawElementProvider(&self) -> Result<IRawElementProviderSimple> {
                // SAFETY: the handle is this process's own live window (see
                // `raise`); the call only reads it.
                unsafe { UiaHostProviderFromHwnd(HWND(self.hwnd as *mut core::ffi::c_void)) }
            }
        }

        pub(super) fn raise(
            hwnd: isize,
            text: &str,
            important: bool,
        ) -> std::result::Result<(), String> {
            if hwnd == 0 {
                return Err("no window handle".into());
            }
            let processing = if important {
                NotificationProcessing_ImportantMostRecent
            } else {
                NotificationProcessing_All
            };
            let display = BSTR::from(text);
            let activity = BSTR::from(super::ACTIVITY);
            let source: IRawElementProviderSimple = WindowSource { hwnd }.into();
            // SAFETY: `hwnd` is the handle of this process's own window,
            // read from winit on the event loop's thread, and the window
            // outlives this call (the driver owns it). The function only
            // reads its arguments: the provider is a reference-counted COM
            // object that the `windows` crate releases when it drops, and
            // the two BSTRs live until the call returns. The call is made on
            // the window's own thread, which winit initialized for COM (OLE).
            unsafe {
                UiaRaiseNotificationEvent(
                    &source,
                    NotificationKind_Other,
                    processing,
                    &display,
                    &activity,
                )
                .map_err(|e| format!("UiaRaiseNotificationEvent: {e}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use masonry::core::WidgetTag;
    use masonry_testing::TestHarness;

    #[test]
    fn announce_modes_have_names() {
        for name in AnnounceMode::NAMES {
            let mode = AnnounceMode::parse(name).expect("known name");
            assert_eq!(mode.name(), name);
        }
        assert_eq!(AnnounceMode::parse(" UIA "), Some(AnnounceMode::Uia));
        assert_eq!(AnnounceMode::parse("aria"), None);
        assert_eq!(AnnounceMode::default(), AnnounceMode::Live);
        assert!(AnnounceMode::Live.uses_live_region());
        assert_eq!(AnnounceMode::Uia.uses_live_region(), !cfg!(windows));
    }

    /// With UI Automation notifications, the messages stay in the tree
    /// (object navigation and the report find them) but are not live, so a
    /// screen reader is not told twice.
    #[test]
    fn uia_mode_keeps_messages_without_a_live_setting() {
        let p = crate::theme::Palette::galaxy();
        let tag: WidgetTag<Announcer> = WidgetTag::named("ann");
        let announcer = Announcer::new(Rc::new(Cell::new(0))).with_mode(AnnounceMode::Uia);
        let mut h = TestHarness::create(
            crate::theme::default_properties(&p),
            NewWidget::new(announcer).with_tag(tag),
        );
        h.edit_root_widget(|mut a| {
            Announcer::say(
                &mut a,
                [Message {
                    text: "Paused.".into(),
                    priority: Priority::Assertive,
                }],
            );
        });
        let _ = h.redraw();
        let node = h.access_node(h.root_id()).expect("the announcer");
        let last = node.children().last().expect("a message node");
        assert_eq!(last.value().as_deref(), Some("Paused."));
        let expected = if cfg!(windows) {
            Live::Off
        } else {
            Live::Assertive
        };
        assert_eq!(last.live(), expected);
    }

    #[test]
    fn a_notification_needs_a_window() {
        let m = Message {
            text: "Stopped.".into(),
            priority: Priority::Polite,
        };
        assert!(notify::raise(0, &m).is_err());
    }
}
