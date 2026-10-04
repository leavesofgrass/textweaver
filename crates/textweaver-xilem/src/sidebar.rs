//! The panel beside the document (Wave 8d): the Contents (the headings,
//! as the outline lists them with Alt+O) or the Notes (as the notes list
//! shows them), closing the Star gap in `docs/star-gaps.md`.
//!
//! - **One list model.** The rows come from the app
//!   ([`App::panel_entries`]), with the outline's and the notes list's own
//!   labels, and Enter goes where choosing the row in that list would
//!   ([`App::go_to_panel_entry`]), so the panel and the list dialog never
//!   disagree. The list is a [`ChoiceList`]: a `ListBox` whose rows are
//!   AccessKit options with their place ("3 of 12"), laid out only around
//!   the visible ones.
//! - **A landmark.** The panel is a `Navigation` landmark named "Contents"
//!   or "Notes"; the row where the caret is has a bar and ", current" in
//!   its name, so the mark is never the bar alone.
//! - **Never takes the focus unasked.** Showing the panel from the setting
//!   (`[gui] sidebar`, at startup or from the settings) leaves the focus
//!   where it was. The panel's own key ([`toggle`]) shows it and goes to
//!   it; pressed in the panel, it closes it and goes back to the document.
//!   F6 and Shift+F6 move between the window's regions ([`next_region`]).
//!   In the list, Enter goes to the row and stays; Shift+Enter goes and
//!   returns to the document; Escape returns without moving.
//! - **No cost when closed.** [`sync`] reads one setting and returns; open,
//!   the rows are built again only when [`App::panel_key`] changes (the
//!   document, its revision, its headings, the notes), never on a caret or
//!   highlight move, which costs a binary search for the current row.

use masonry::accesskit::{Node, Role};
use masonry::core::keyboard::{Key, KeyState, NamedKey};
use masonry::core::{
    AccessCtx, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PropertiesMut,
    PropertiesRef, RegisterCtx, TextEvent, Widget, WidgetId, WidgetMut, WidgetPod, WidgetTag,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LayoutSize, LenReq, Length};
use masonry::properties::{LineBreaking, Padding};
use masonry::widgets::Label;
use textweaver_app::core::CharPos;
use textweaver_app::lexicon::args;
use textweaver_app::store::GuiSidebar;
use textweaver_app::{App, Effect, Panel};

use crate::dialog::ChoiceList;
use crate::document::DocumentView;
use crate::gui::Host;
use crate::theme::{self, Palette};

/// The document's row: the panel (when shown) and the document.
pub const SIDEBAR: WidgetTag<Sidebar> = WidgetTag::named("tw-sidebar");
/// The panel's list, while the panel is shown.
pub const SIDEBAR_LIST: WidgetTag<ChoiceList> = WidgetTag::named("tw-sidebar-list");

/// The panel's width, in logical pixels: about 30 characters of the
/// interface text, at most 45 percent of the row.
pub const PANEL_WIDTH: f64 = 300.0;
/// The space between the panel and the document.
const GAP: f64 = 12.0;

/// The panel `setting` names, if any.
pub fn panel_of(setting: GuiSidebar) -> Option<Panel> {
    match setting {
        GuiSidebar::Off => None,
        GuiSidebar::Contents => Some(Panel::Contents),
        GuiSidebar::Notes => Some(Panel::Notes),
    }
}

/// The setting that shows `panel` (`None` for none).
pub fn setting_of(panel: Option<Panel>) -> GuiSidebar {
    match panel {
        None => GuiSidebar::Off,
        Some(Panel::Contents) => GuiSidebar::Contents,
        Some(Panel::Notes) => GuiSidebar::Notes,
    }
}

/// The panel's name: the landmark's and the list's ("Contents", "Notes").
pub fn panel_name(app: &App, panel: Panel) -> String {
    app.catalog().tr(match panel {
        Panel::Contents => "gui-sidebar-contents",
        Panel::Notes => "gui-sidebar-notes",
    })
}

// --- The widgets.

/// What the panel tells the driver: a key its list left alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidebarAction {
    /// Back to the document: Escape (`go` false), or Shift+Enter, which
    /// first goes to the selected row (`go` true).
    Leave {
        /// Go to the selected row first.
        go: bool,
    },
}

/// The document's row: the panel beside the document when one is shown,
/// otherwise the document alone, laid out exactly as without a panel.
pub struct Sidebar {
    panel: Option<WidgetPod<PanelView>>,
    doc: WidgetPod<DocumentView>,
    /// The panel's list, while the panel is shown.
    list: Option<WidgetId>,
}

impl Sidebar {
    /// The row around `doc`, with no panel.
    pub fn new(doc: NewWidget<DocumentView>) -> Self {
        Sidebar {
            panel: None,
            doc: doc.to_pod(),
            list: None,
        }
    }

    /// Shows a panel named `name`, with `list` (its rows) and `hint` (its
    /// keys, drawn only) under the title, replacing the one shown.
    pub fn open(
        this: &mut WidgetMut<'_, Self>,
        name: &str,
        list: NewWidget<ChoiceList>,
        hint: &str,
        palette: &Palette,
    ) {
        if let Some(old) = this.widget.panel.take() {
            this.ctx.remove_child(old);
        }
        this.widget.list = Some(list.id());
        let panel = PanelView::new(name, list, hint);
        let (bg, border, bw, radius, shadow) = theme::panel_props(palette);
        let panel = NewWidget::new(panel).with_props((
            bg,
            border,
            bw,
            radius,
            shadow,
            Padding::from_vh(Length::px(10.0), Length::px(8.0)),
        ));
        this.widget.panel = Some(panel.to_pod());
        this.ctx.children_changed();
        this.ctx.request_layout();
    }

    /// Closes the panel; the document takes the whole row again.
    pub fn close(this: &mut WidgetMut<'_, Self>) {
        if let Some(old) = this.widget.panel.take() {
            this.ctx.remove_child(old);
            this.widget.list = None;
            this.ctx.children_changed();
            this.ctx.request_layout();
        }
    }

    /// Recolors the open panel's box for `palette` (its list is recolored
    /// through [`SIDEBAR_LIST`]).
    pub fn set_palette(this: &mut WidgetMut<'_, Self>, palette: &Palette) {
        let Some(pod) = this.widget.panel.as_mut() else {
            return;
        };
        let mut panel = this.ctx.get_mut(pod);
        let (bg, border, bw, radius, shadow) = theme::panel_props(palette);
        panel.insert_prop(bg);
        panel.insert_prop(border);
        panel.insert_prop(bw);
        panel.insert_prop(radius);
        panel.insert_prop(shadow);
    }

    /// True while a panel is shown.
    pub fn is_open(&self) -> bool {
        self.panel.is_some()
    }

    /// The panel's list, while a panel is shown.
    pub fn list_id(&self) -> Option<WidgetId> {
        self.list
    }

    /// The document view.
    pub fn doc_id(&self) -> WidgetId {
        self.doc.id()
    }
}

impl Widget for Sidebar {
    type Action = SidebarAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        if let Some(p) = &mut self.panel {
            ctx.register_child(p);
        }
        ctx.register_child(&mut self.doc);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<Length>,
    ) -> Length {
        let doc = ctx.redirect_measurement(&mut self.doc, axis, cross_length);
        match (axis, &self.panel, len_req) {
            (Axis::Horizontal, Some(_), LenReq::FitContent(space)) => space,
            (Axis::Horizontal, Some(_), _) => Length::px(doc.get() + PANEL_WIDTH + GAP),
            _ => doc,
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let Some(panel) = &mut self.panel else {
            ctx.run_layout(&mut self.doc, size);
            ctx.place_child(&mut self.doc, Point::ORIGIN);
            return;
        };
        let width = PANEL_WIDTH.min(size.width * 0.45).max(0.0);
        ctx.run_layout(panel, Size::new(width, size.height));
        ctx.place_child(panel, Point::ORIGIN);
        let x = width + GAP;
        ctx.run_layout(
            &mut self.doc,
            Size::new((size.width - x).max(0.0), size.height),
        );
        ctx.place_child(&mut self.doc, Point::new(x, 0.0));
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
        let mut ids = ChildrenIds::new();
        if let Some(p) = &self.panel {
            ids.push(p.id());
        }
        ids.push(self.doc.id());
        ids
    }
}

/// The panel: a `Navigation` landmark with its title (drawn only: the
/// landmark and the list carry the name), the list, and its keys (drawn
/// only). Escape and Shift+Enter, which the list leaves alone, are its.
pub struct PanelView {
    name: String,
    title: WidgetPod<Label>,
    list: WidgetPod<ChoiceList>,
    hint: WidgetPod<Label>,
}

impl PanelView {
    fn new(name: &str, list: NewWidget<ChoiceList>, hint: &str) -> Self {
        let title = crate::gui::label(name, theme::UI_TEXT + 2.0, true).accessibility_hidden(true);
        let hint = crate::gui::label(hint, theme::UI_TEXT - 1.0, false).accessibility_hidden(true);
        PanelView {
            name: name.to_owned(),
            title: NewWidget::new(title).to_pod(),
            list: list.to_pod(),
            hint: NewWidget::new(hint)
                .with_props(LineBreaking::WordWrap)
                .to_pod(),
        }
    }
}

impl Widget for PanelView {
    type Action = SidebarAction;

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        let TextEvent::Keyboard(k) = event else {
            return;
        };
        if k.state != KeyState::Down || k.is_composing {
            return;
        }
        let m = k.modifiers;
        let plain = !m.ctrl() && !m.alt() && !m.meta();
        let go = match &k.key {
            Key::Named(NamedKey::Escape) if plain && !m.shift() => false,
            Key::Named(NamedKey::Enter) if plain && m.shift() => true,
            _ => return,
        };
        ctx.submit_action::<SidebarAction>(SidebarAction::Leave { go });
        ctx.set_handled();
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.title);
        ctx.register_child(&mut self.list);
        ctx.register_child(&mut self.hint);
    }

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        _cross_length: Option<Length>,
    ) -> Length {
        match (axis, len_req) {
            (_, LenReq::FitContent(space)) => space,
            (Axis::Horizontal, _) => Length::px(PANEL_WIDTH),
            (Axis::Vertical, _) => Length::px(240.0),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let w = size.width.max(0.0);
        let height_of = |ctx: &mut LayoutCtx<'_>, pod: &mut WidgetPod<Label>| {
            ctx.compute_length(
                pod,
                LenReq::MaxContent.into(),
                LayoutSize::maybe(Axis::Horizontal, Some(Length::px(w))),
                Axis::Vertical,
                Some(Length::px(w)),
            )
            .get()
        };
        let title_h = height_of(ctx, &mut self.title);
        let hint_h = height_of(ctx, &mut self.hint);
        ctx.run_layout(&mut self.title, Size::new(w, title_h));
        ctx.place_child(&mut self.title, Point::new(4.0_f64.min(w), 0.0));
        let top = title_h + 6.0;
        let list_h = (size.height - top - hint_h - 6.0).max(0.0);
        ctx.run_layout(&mut self.list, Size::new(w, list_h));
        ctx.place_child(&mut self.list, Point::new(0.0, top));
        ctx.run_layout(&mut self.hint, Size::new(w, hint_h));
        ctx.place_child(
            &mut self.hint,
            Point::new(0.0, (size.height - hint_h).max(top)),
        );
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _p: &mut Painter<'_>) {
    }

    fn accessibility_role(&self) -> Role {
        Role::Navigation
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_label(self.name.as_str());
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.title.id(), self.list.id(), self.hint.id()])
    }
}

// --- The driver's side.

/// Where the window moves the focus, and reads it: the live window's
/// render root, or the test harness.
pub trait FocusHost: Host {
    /// The focused widget.
    fn focused(&self) -> Option<WidgetId>;
    /// Moves the focus to `id`.
    fn focus(&mut self, id: Option<WidgetId>);
}

impl FocusHost for masonry::app::RenderRoot {
    fn focused(&self) -> Option<WidgetId> {
        self.focused_widget()
    }

    fn focus(&mut self, id: Option<WidgetId>) {
        let _ = self.focus_on(id);
    }
}

#[cfg(any(test, feature = "screenshot"))]
impl<RW: Widget> FocusHost for masonry_testing::TestHarness<RW> {
    fn focused(&self) -> Option<WidgetId> {
        self.focused_widget_id()
    }

    fn focus(&mut self, id: Option<WidgetId>) {
        self.focus_on(id);
    }
}

/// What the window last showed in the panel, to change only what changed.
#[derive(Debug, Default)]
pub struct SidebarShown {
    /// The panel shown.
    panel: Option<Panel>,
    /// Its rows' key ([`App::panel_key`]).
    key: Option<u64>,
    /// Where each row goes.
    positions: Vec<CharPos>,
    /// Each row's text.
    labels: Vec<String>,
    /// The row marked current.
    current: Option<usize>,
    /// The theme the panel was drawn in.
    palette: String,
    /// The interface language the panel was named in.
    lang: String,
}

impl SidebarShown {
    /// The panel shown, if any.
    pub fn panel(&self) -> Option<Panel> {
        self.panel
    }

    /// How many rows the panel lists (0 for none to list).
    pub fn rows(&self) -> usize {
        self.positions.len()
    }

    /// The row marked current.
    pub fn current(&self) -> Option<usize> {
        self.current
    }
}

/// What [`sync`] changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidebarChange {
    /// The panel was shown, with this many rows.
    Opened(Panel, usize),
    /// The panel was closed.
    Closed(Panel),
}

/// The row a cursor at `cursor` is in: the last starting at or before it.
fn current_row(positions: &[CharPos], cursor: CharPos) -> Option<usize> {
    positions.partition_point(|p| *p <= cursor).checked_sub(1)
}

/// Brings the panel in step with `[gui] sidebar` and the app: shows,
/// closes, or refills it, and marks the current row. Never moves the
/// focus, except back to the document when the panel holding it closes.
/// Closed, it reads one setting and returns.
pub fn sync(
    app: &App,
    palette: &Palette,
    shown: &mut SidebarShown,
    host: &mut impl FocusHost,
) -> Option<SidebarChange> {
    let want = panel_of(app.settings().gui.sidebar);
    let Some(panel) = want else {
        let was = shown.panel.take()?;
        let (list, doc) = host.edit(SIDEBAR, |s| (s.widget.list_id(), s.widget.doc_id()));
        let had_focus = list.is_some() && host.focused() == list;
        host.edit(SIDEBAR, |mut s| Sidebar::close(&mut s));
        if had_focus || host.focused().is_none() {
            host.focus(Some(doc));
        }
        *shown = SidebarShown::default();
        return Some(SidebarChange::Closed(was));
    };
    let key = app.panel_key(panel);
    let new_panel = shown.panel != Some(panel);
    if !new_panel && shown.palette != palette.name {
        // A new theme: the same panel and list, recolored, so a list with
        // the focus keeps it.
        host.edit(SIDEBAR, |mut s| Sidebar::set_palette(&mut s, palette));
        host.edit(SIDEBAR_LIST, |mut l| {
            ChoiceList::set_palette(&mut l, palette.clone());
        });
        shown.palette.clone_from(&palette.name);
    }
    let lang = app.catalog().lang().to_owned();
    // A new interface language names the panel again: it is built anew,
    // and keeps the focus if it had it.
    let rename = !new_panel && shown.lang != lang;
    let mut change = None;
    if new_panel || rename || shown.key != key {
        let entries = app.panel_entries(panel);
        shown.positions = entries.iter().map(|e| e.pos).collect();
        shown.labels = entries.into_iter().map(|e| e.label).collect();
        let items = if shown.labels.is_empty() {
            vec![app.catalog().tr(match panel {
                Panel::Contents => "gui-sidebar-no-headings",
                Panel::Notes => "gui-sidebar-no-notes",
            })]
        } else {
            shown.labels.clone()
        };
        let cursor = app.session().map_or(CharPos(0), |s| s.cursor);
        let here = current_row(&shown.positions, cursor).unwrap_or(0);
        if new_panel || rename {
            let old = host.edit(SIDEBAR, |s| s.widget.list_id());
            let had_focus = old.is_some() && host.focused() == old;
            let name = panel_name(app, panel);
            let list = NewWidget::new(
                ChoiceList::new(name.clone(), items, palette.clone())
                    .with_selected(here)
                    .with_shift_enter_to_parent(true),
            )
            .with_tag(SIDEBAR_LIST);
            // The panel's own key, written as the keymap writes keys.
            let leave = textweaver_app::keymap::KeyChord::new(
                textweaver_app::keymap::Key::Enter,
                textweaver_app::keymap::Modifiers::SHIFT,
            );
            let hint = app.catalog().fmt(
                "gui-sidebar-hint",
                &args!["leave" => crate::keys::shortcut_text(&leave)],
            );
            host.edit(SIDEBAR, |mut s| {
                Sidebar::open(&mut s, &name, list, &hint, palette);
            });
            if had_focus {
                let new = host.edit(SIDEBAR, |s| s.widget.list_id());
                host.focus(new);
            }
            if new_panel {
                change = Some(SidebarChange::Opened(panel, shown.positions.len()));
            }
        } else {
            host.edit(SIDEBAR_LIST, |mut l| {
                // The reader's place in the list stays while it is there.
                let at = if l.widget.has_focus() {
                    l.widget.selected().min(items.len().saturating_sub(1))
                } else {
                    here
                };
                ChoiceList::sync(&mut l, &items, at);
            });
        }
        shown.panel = Some(panel);
        shown.key = key;
        shown.palette.clone_from(&palette.name);
        shown.lang = lang;
        shown.current = None;
        mark_current(app, shown, host, true);
        return change;
    }
    mark_current(app, shown, host, false);
    change
}

/// Marks the row at the caret as current (a bar and ", current"); while
/// the list does not have the focus, it is also the selected row, so going
/// to the panel starts where the reader is. `force` marks it even when it
/// did not change.
fn mark_current(app: &App, shown: &mut SidebarShown, host: &mut impl FocusHost, force: bool) {
    let cursor = app.session().map_or(CharPos(0), |s| s.cursor);
    let now = current_row(&shown.positions, cursor);
    if now == shown.current && !force {
        return;
    }
    shown.current = now;
    let marked = now.and_then(|i| {
        let item = shown.labels.get(i)?;
        let label = app
            .catalog()
            .fmt("gui-sidebar-current", &args!["item" => item.as_str()]);
        Some((i, label))
    });
    host.edit(SIDEBAR_LIST, |mut l| {
        if let Some(i) = now
            && !l.widget.has_focus()
        {
            ChoiceList::select(&mut l, i);
        }
        ChoiceList::set_current(&mut l, marked);
    });
}

/// What the panel key did ([`toggle`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggled {
    /// The panel was shown, with this many rows, and has the focus.
    Opened(Panel, usize),
    /// The panel was already shown: the focus went to it.
    Focused(Panel),
    /// The panel was closed from inside it: the focus is on the document.
    Closed(Panel),
}

/// The panel key for `panel`: shows the panel and goes to it; in a panel
/// already shown, goes to it; pressed inside it, closes it and goes back
/// to the document. The choice is saved in `[gui] sidebar`.
pub fn toggle(
    app: &mut App,
    panel: Panel,
    palette: &Palette,
    shown: &mut SidebarShown,
    host: &mut impl FocusHost,
) -> Toggled {
    let (list, doc) = host.edit(SIDEBAR, |s| (s.widget.list_id(), s.widget.doc_id()));
    let in_list = list.is_some() && host.focused() == list;
    if shown.panel == Some(panel) && in_list {
        let _ = app.update_settings(|s| s.gui.sidebar = GuiSidebar::Off);
        let _ = sync(app, palette, shown, host);
        host.focus(Some(doc));
        return Toggled::Closed(panel);
    }
    let opened = shown.panel != Some(panel);
    if opened {
        let _ = app.update_settings(|s| s.gui.sidebar = setting_of(Some(panel)));
        let _ = sync(app, palette, shown, host);
    }
    let list = host.edit(SIDEBAR, |s| s.widget.list_id());
    host.focus(list);
    if opened {
        Toggled::Opened(panel, shown.positions.len())
    } else {
        Toggled::Focused(panel)
    }
}

/// What the window says for a panel shown or closed by its key.
pub fn toggled_message(app: &App, t: Toggled) -> Option<String> {
    let c = app.catalog();
    match t {
        Toggled::Opened(p, n) => Some(c.fmt(
            "gui-sidebar-open",
            &args!["panel" => panel_name(app, p), "n" => n],
        )),
        Toggled::Closed(p) => {
            Some(c.fmt("gui-sidebar-closed", &args!["panel" => panel_name(app, p)]))
        }
        // The screen reader says the list and its row as the focus moves.
        Toggled::Focused(_) => None,
    }
}

/// Enter (or Shift+Enter, `leave`) on row `index`: the document goes
/// there, as choosing it in the list dialog does; with `leave`, the focus
/// returns to the document, else it stays in the list. Returns the app's
/// effects; nothing for a row that is not a place (the "No headings" row).
pub fn go(
    app: &mut App,
    shown: &SidebarShown,
    index: usize,
    leave: bool,
    host: &mut impl FocusHost,
) -> Vec<Effect> {
    let mut effects = Vec::new();
    if let Some(panel) = shown.panel
        && index < shown.positions.len()
    {
        effects = app.go_to_panel_entry(panel, index);
    }
    if leave {
        let doc = host.edit(SIDEBAR, |s| s.widget.doc_id());
        host.focus(Some(doc));
    }
    effects
}

/// The window's regions in F6 order: the header, the panel (when shown),
/// the document, and the toolbar. Each holds the controls the focus may
/// land on, the first being where it lands.
#[derive(Clone, Debug, Default)]
pub struct Regions {
    /// The header's buttons.
    pub header: Vec<WidgetId>,
    /// The panel's list, when shown.
    pub sidebar: Option<WidgetId>,
    /// The document.
    pub document: Option<WidgetId>,
    /// The toolbar's buttons.
    pub toolbar: Vec<WidgetId>,
    /// The header and the toolbar are folded into one bar above the
    /// document (a narrow window, [`crate::bars::Frame`]): the toolbar
    /// comes second, as on screen.
    pub folded: bool,
}

impl Regions {
    /// The regions in order on screen, and where the document is.
    fn ordered(&self) -> ([Vec<WidgetId>; 4], usize) {
        let header = self.header.clone();
        let sidebar = self.sidebar.into_iter().collect();
        let document = self.document.into_iter().collect();
        let toolbar = self.toolbar.clone();
        if self.folded {
            ([header, toolbar, sidebar, document], 3)
        } else {
            ([header, sidebar, document, toolbar], 2)
        }
    }
}

/// F6 (`forward`) or Shift+F6 from the control with the focus: the first
/// control of the next region that has one, wrapping around. With the focus
/// nowhere known, it counts from the document.
pub fn next_region(
    regions: &Regions,
    focused: Option<WidgetId>,
    forward: bool,
) -> Option<WidgetId> {
    let (all, doc) = regions.ordered();
    let at = focused
        .and_then(|f| all.iter().position(|r| r.contains(&f)))
        .unwrap_or(doc);
    let n = all.len();
    (1..=n)
        .map(|k| {
            if forward {
                (at + k) % n
            } else {
                (at + n - k) % n
            }
        })
        .find_map(|i| all[i].first().copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(n: usize) -> Vec<WidgetId> {
        (0..n)
            .map(|_| NewWidget::new(Label::new("x")).id())
            .collect()
    }

    #[test]
    fn f6_cycles_the_regions_in_order() {
        let header = ids(2);
        let toolbar = ids(3);
        let doc = ids(1)[0];
        let list = ids(1)[0];
        let mut r = Regions {
            header: header.clone(),
            sidebar: None,
            document: Some(doc),
            toolbar: toolbar.clone(),
            folded: false,
        };
        // No panel: document, toolbar, header, document.
        assert_eq!(next_region(&r, Some(doc), true), Some(toolbar[0]));
        assert_eq!(next_region(&r, Some(toolbar[2]), true), Some(header[0]));
        assert_eq!(next_region(&r, Some(header[1]), true), Some(doc));
        assert_eq!(next_region(&r, Some(doc), false), Some(header[0]));
        // With the panel: header, panel, document, toolbar.
        r.sidebar = Some(list);
        assert_eq!(next_region(&r, Some(header[0]), true), Some(list));
        assert_eq!(next_region(&r, Some(list), true), Some(doc));
        assert_eq!(next_region(&r, Some(doc), false), Some(list));
        assert_eq!(next_region(&r, Some(list), false), Some(header[0]));
        // Focus unknown: from the document.
        assert_eq!(next_region(&r, None, true), Some(toolbar[0]));
        // Folded (a narrow window): header, toolbar, panel, document, as
        // on screen; a hidden bar has no buttons and is skipped.
        r.folded = true;
        assert_eq!(next_region(&r, Some(header[0]), true), Some(toolbar[0]));
        assert_eq!(next_region(&r, Some(toolbar[1]), true), Some(list));
        assert_eq!(next_region(&r, Some(doc), true), Some(header[0]));
        assert_eq!(next_region(&r, None, true), Some(header[0]));
        r.header.clear();
        assert_eq!(next_region(&r, Some(doc), true), Some(toolbar[0]));
    }

    #[test]
    fn the_current_row_is_found_by_position() {
        let p = [CharPos(0), CharPos(10), CharPos(20)];
        assert_eq!(current_row(&p, CharPos(5)), Some(0));
        assert_eq!(current_row(&p, CharPos(20)), Some(2));
        assert_eq!(current_row(&[CharPos(3)], CharPos(1)), None);
    }

    #[test]
    fn the_setting_names_each_panel() {
        for p in [None, Some(Panel::Contents), Some(Panel::Notes)] {
            assert_eq!(panel_of(setting_of(p)), p);
        }
    }
}
