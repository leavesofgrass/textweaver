//! Dialogs inside the window: a modal card over a dimmed page, with the
//! `Dialog` role, a name, and focus kept inside it. Two kinds:
//!
//! - a prompt (find, go to, open, the command palette): a labelled text
//!   field, Enter to accept;
//! - a list (bookmarks, help, voices, the library): a [`ChoiceList`] with
//!   the `ListBox` role, arrow keys, Home and End, Page Up and Page Down,
//!   first-letter search, Enter to choose, and Escape to close.
//!
//! Every kind has one style: a 20 px bold title ([`TITLE_SIZE`]), a hint
//! and a Close button in its footer, so a mouse user can always leave it,
//! Escape always closes it, and the focus goes back where it was.
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
use textweaver_app::keymap::Platform;

use crate::theme::{self, Palette};

/// What a dialog tells the driver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DialogAction {
    /// The list's item at this index was chosen.
    Choose(usize),
    /// The dialog was closed without an answer.
    Cancel,
    /// A key for the app's list model (`Command::ListKey`): the list is
    /// the app's, which moves its focus, filters it, and chooses from it.
    Key(textweaver_app::ListKey),
    /// The pointer moved the list's focus (`Command::ListFocus`, quiet).
    Focus(usize),
    /// Up (`true`) or Down in a prompt's field: an earlier or later answer
    /// (`PromptKey::Up` and `PromptKey::Down`).
    Recall(bool),
    /// Tab in a prompt for a path: complete it (`PromptKey::Tab`).
    Complete,
    /// The browse key (F4) in a prompt for a path: the app's file browser
    /// chooses it (`PromptKey::Browse`, W8a-f).
    Browse,
    /// Tab (`true`) or Shift+Tab in a form of fields (the edit details
    /// form, W7m): the next or previous field (`PromptKey::Tab` and
    /// `PromptKey::BackTab`), which the app opens as a prompt of its own.
    Field(bool),
    /// A chord with Ctrl or Alt, or a function key, in an app list: the
    /// driver looks it up in the keymap, so the Help and Say Status keys
    /// repeat the list's introduction (`ListKey::Introduce`) and the
    /// Repeat Message key says the last message, as in the terminal.
    Chord(textweaver_app::keymap::KeyChord),
    /// Ctrl+L in the command palette (`Modal::with_show_matches`): the
    /// focus moves to the list of matches.
    ShowMatches,
    /// A key typed in a question (`Modal::with_answer_keys`): `y` is yes,
    /// `n` and `a` are no, any other character asks again, as in the
    /// terminal (`Command::Confirm`).
    Answer(textweaver_app::Confirm),
}

// --- One dialog style (W9b-d).

/// Every dialog's title as drawn: 20 px, bold. The dialog's name says it,
/// so the drawn title is hidden from screen readers.
pub const TITLE_SIZE: f32 = 20.0;

/// The gap above a dialog's footer (its hint and buttons).
pub const FOOTER_GAP: f64 = 8.0;

/// The widest a dialog of text is: a prompt, a list, a question.
pub const WIDTH_TEXT: f64 = 600.0;

/// The widest a dialog of forms is: Settings, Colors, the reading form,
/// the voice manager. One rule: a form is this wide, the rest
/// [`WIDTH_TEXT`].
pub const WIDTH_FORM: f64 = 960.0;

// --- Modal.

/// A modal card centred over the window, over a dimmed page. It measures
/// to the window's size, so the root can give it the whole window.
pub struct Modal {
    card: WidgetPod<dyn Widget>,
    label: String,
    palette: Palette,
    max_width: f64,
    /// Tab completes the field (a path) instead of moving the focus.
    tab_completes: bool,
    /// Tab and Shift+Tab move between the app's form fields instead of the
    /// dialog's controls.
    tab_fields: bool,
    /// A question: typed characters answer it.
    answer_keys: bool,
    /// The command palette: the command key with L moves to its list of
    /// matches, as Ctrl+L shows them as a list in the terminal.
    show_matches: bool,
    /// Chords (Ctrl, Alt, or a function key) that no control took go to
    /// the app as [`DialogAction::Chord`], so the Help, Say Status, and
    /// Repeat Message keys work with the focus on a button too.
    app_chords: bool,
    /// Whose command modifier the dialog follows ([`crate::keys`]).
    platform: Platform,
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
            max_width: WIDTH_TEXT,
            tab_completes: false,
            tab_fields: false,
            answer_keys: false,
            show_matches: false,
            app_chords: false,
            platform: Platform::current(),
        }
    }

    /// The command palette: the command key with L (Ctrl+L, Command+L on
    /// macOS) moves to the list of matches, as in the terminal's palette.
    pub fn with_show_matches(mut self, on: bool) -> Self {
        self.show_matches = on;
        self
    }

    /// Chords no control took (Ctrl, Alt, or a function key, as the app
    /// list sends them) go to the app as [`DialogAction::Chord`]: the
    /// voice manager's keys work on its buttons as in its list.
    pub fn with_app_chords(mut self, on: bool) -> Self {
        self.app_chords = on;
        self
    }

    /// Follows `platform`'s command modifier instead of this system's.
    pub fn with_platform(mut self, platform: Platform) -> Self {
        self.platform = platform;
        self
    }

    /// A yes-or-no question: `y` answers yes, `n` (or `a`) no, and any
    /// other character asks again, as in the terminal. Escape is no.
    pub fn with_answer_keys(mut self, on: bool) -> Self {
        self.answer_keys = on;
        self
    }

    /// Tab completes the prompt's text (a path) instead of moving the focus.
    pub fn with_tab_completion(mut self, on: bool) -> Self {
        self.tab_completes = on;
        self
    }

    /// Tab and Shift+Tab move to the app's next and previous form field
    /// (the edit details form) instead of the dialog's next control.
    pub fn with_tab_fields(mut self, on: bool) -> Self {
        self.tab_fields = on;
        self
    }

    /// The card's widest width, in logical pixels ([`WIDTH_TEXT`] by
    /// default, [`WIDTH_FORM`] for a form).
    pub fn with_max_width(mut self, width: f64) -> Self {
        self.max_width = width;
        self
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
        let TextEvent::Keyboard(k) = event else {
            return;
        };
        let command = crate::keys::is_command(k.modifiers, self.platform);
        if k.state == KeyState::Down
            && command
            && self.show_matches
            && !k.modifiers.shift()
            && matches!(&k.key, Key::Character(s) if s.eq_ignore_ascii_case("l"))
        {
            ctx.submit_action::<DialogAction>(DialogAction::ShowMatches);
            ctx.set_handled();
            return;
        }
        if self.app_chords
            && k.state == KeyState::Down
            && !k.is_composing
            && (command || matches!(&k.key, Key::Named(n) if crate::keys::is_function_key(n)))
            && let Some(chord) = crate::keys::chord(k, self.platform)
        {
            ctx.submit_action::<DialogAction>(DialogAction::Chord(chord));
            ctx.set_handled();
            return;
        }
        if k.state != KeyState::Down || command {
            return;
        }
        // Keys the dialog's controls left alone.
        let action = match &k.key {
            Key::Named(NamedKey::Escape) => DialogAction::Cancel,
            Key::Named(NamedKey::ArrowUp) if !k.modifiers.shift() => DialogAction::Recall(true),
            Key::Named(NamedKey::ArrowDown) if !k.modifiers.shift() => DialogAction::Recall(false),
            Key::Named(NamedKey::Tab) if self.tab_fields => {
                DialogAction::Field(!k.modifiers.shift())
            }
            Key::Named(NamedKey::Tab) if self.tab_completes && !k.modifiers.shift() => {
                DialogAction::Complete
            }
            Key::Named(_)
                if self.tab_completes
                    && crate::keys::chord(k, self.platform)
                        == Some(textweaver_app::path_prompt::browse_key()) =>
            {
                DialogAction::Browse
            }
            Key::Character(s) if self.answer_keys => match s.chars().next() {
                Some(ch) if !ch.is_control() && !ch.is_whitespace() => {
                    DialogAction::Answer(textweaver_app::Confirm::from_char(ch))
                }
                _ => return,
            },
            _ => return,
        };
        ctx.submit_action::<DialogAction>(action);
        ctx.set_handled();
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

/// The longest start of `text` that `fits` with an ellipsis after it,
/// ending at a word where one is in the second half ("Introduction to
/// the…"), else mid-word. `text` itself when it fits; "…" at the least.
pub fn ellipsize(text: &str, mut fits: impl FnMut(&str) -> bool) -> String {
    if fits(text) {
        return text.to_owned();
    }
    let cuts: Vec<usize> = text.char_indices().map(|(i, _)| i).skip(1).collect();
    let with = |n: usize| format!("{}…", text[..n].trim_end());
    // Binary search for the longest prefix that fits.
    let (mut lo, mut hi) = (0usize, cuts.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if fits(&with(cuts[mid - 1])) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    if lo == 0 {
        return "…".to_owned();
    }
    let end = cuts[lo - 1];
    let prefix = &text[..end];
    match prefix.rfind(char::is_whitespace) {
        Some(w) if w >= end / 2 && w > 0 => with(w),
        _ => with(end),
    }
}

/// The rows of a list of commands for a [`ChoiceList`]: the text drawn
/// (each command's short name), the names for screen readers ("Find next,
/// F3"), the keys drawn at the right edge, and the descriptions (each
/// command's long explanation).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rows {
    /// The text drawn on each row.
    pub items: Vec<String>,
    /// Each row's name for screen readers.
    pub names: Vec<String>,
    /// Each row's key, drawn at the right edge.
    pub keys: Vec<String>,
    /// Each row's description for screen readers.
    pub descriptions: Vec<String>,
}

impl Rows {
    /// The rows of the app's list model when it is a list of commands
    /// (`ListModel::columns`); `None` for other lists.
    pub fn from_model(m: &textweaver_app::ListModel) -> Option<Rows> {
        if m.columns.len() != m.items.len() || m.items.is_empty() {
            return None;
        }
        Some(Rows {
            items: m.columns.iter().map(|(n, _)| n.clone()).collect(),
            names: m.items.clone(),
            keys: m.columns.iter().map(|(_, k)| k.clone()).collect(),
            descriptions: m.descriptions.clone(),
        })
    }

    /// The rows of the command palette for `actions`, from the app's
    /// command rows ([`App::command_row`](textweaver_app::App::command_row)):
    /// `names` are the palette's own lines ("Find next, F3, recent").
    pub fn commands(
        app: &textweaver_app::App,
        actions: &[textweaver_app::keymap::ActionId],
        names: Vec<String>,
    ) -> Rows {
        let rows: Vec<textweaver_app::CommandRow> =
            actions.iter().map(|&a| app.command_row(a)).collect();
        Rows {
            items: rows.iter().map(|r| r.name.clone()).collect(),
            names,
            keys: rows
                .iter()
                .map(|r| r.key.clone().unwrap_or_default())
                .collect(),
            descriptions: rows.into_iter().map(|r| r.help).collect(),
        }
    }
}

/// A list of choices: one focusable `ListBox` whose options are AccessKit
/// nodes (not widgets), with the selected one reported as selected and
/// the list's active descendant.
pub struct ChoiceList {
    items: Vec<String>,
    /// Each row's accessible name, when it says more than the row's text
    /// (the settings' sections: "Speech, 31 settings" for the screen
    /// reader, "Speech" on screen). Empty: the text is the name.
    names: Vec<String>,
    /// Each row's key, drawn at the right edge (the command palette and
    /// the keyboard shortcuts list: "F3" beside "Find next"). The row's
    /// name says it too ("Find next, F3"). Empty: no key column.
    keys: Vec<String>,
    key_layouts: Vec<Option<Layout<BrushIndex>>>,
    /// Each row's description for screen readers (a command's long
    /// explanation). Empty: none.
    descriptions: Vec<String>,
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
    /// Keys go to the app's list model instead of moving the focus here.
    app_keys: bool,
    /// Every move is reported ([`DialogAction::Focus`]), for a list whose
    /// focus changes what the dialog shows (the settings' sections).
    focus_actions: bool,
    /// Whose command modifier the list follows ([`crate::keys`]).
    platform: Platform,
    /// The row where the reader is (the Contents panel's heading at the
    /// caret): drawn with a bar and named with its own label (", current").
    current: Option<(usize, String)>,
    /// Shift+Enter is left to the widget around the list (the panel's "go
    /// there and return to the document").
    shift_enter_to_parent: bool,
}

impl ChoiceList {
    /// A list named `label`.
    pub fn new(label: impl Into<String>, items: Vec<String>, palette: Palette) -> Self {
        let n = items.len();
        ChoiceList {
            items,
            names: Vec::new(),
            keys: Vec::new(),
            key_layouts: Vec::new(),
            descriptions: Vec::new(),
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
            app_keys: false,
            focus_actions: false,
            platform: Platform::current(),
            current: None,
            shift_enter_to_parent: false,
        }
    }

    /// Leaves Shift+Enter to the widget around the list, which reads
    /// [`selected`](Self::selected).
    pub fn with_shift_enter_to_parent(mut self, on: bool) -> Self {
        self.shift_enter_to_parent = on;
        self
    }

    /// Marks row `i` as the one where the reader is: a bar before it, and
    /// `label` as its name for screen readers ("Methods, level 2,
    /// current"), so the mark is never the bar alone. `None` marks none.
    pub fn set_current(this: &mut WidgetMut<'_, Self>, current: Option<(usize, String)>) {
        if this.widget.current != current {
            this.widget.current = current;
            this.ctx.request_render();
        }
    }

    /// True while the list has the keyboard focus.
    pub fn has_focus(&self) -> bool {
        self.focused
    }

    /// Recolors the list for `palette` (a theme change while it stays
    /// open, as the window's panel does).
    pub fn set_palette(this: &mut WidgetMut<'_, Self>, palette: Palette) {
        this.widget.palette = palette;
        this.ctx.request_render();
    }

    /// The row marked current, if any.
    pub fn current(&self) -> Option<usize> {
        self.current.as_ref().map(|(i, _)| *i)
    }

    /// The rows.
    pub fn items(&self) -> &[String] {
        &self.items
    }

    /// Follows `platform`'s command modifier instead of this system's.
    pub fn with_platform(mut self, platform: Platform) -> Self {
        self.platform = platform;
        self
    }

    /// Sends keys to the app's list model ([`DialogAction::Key`]); the
    /// driver then shows the model's items and focus with [`sync`](Self::sync).
    pub fn with_app_keys(mut self, on: bool) -> Self {
        self.app_keys = on;
        self
    }

    /// Reports every move of the focus with [`DialogAction::Focus`].
    pub fn with_focus_actions(mut self, on: bool) -> Self {
        self.focus_actions = on;
        self
    }

    /// Shows `items` with `selected` focused (the app's list model),
    /// keeping the layouts when the items did not change.
    pub fn sync(this: &mut WidgetMut<'_, Self>, items: &[String], selected: usize) {
        let w = &mut *this.widget;
        if w.items != items {
            w.layouts = (0..items.len()).map(|_| None).collect();
            w.items = items.to_vec();
            w.names.clear();
            w.keys.clear();
            w.key_layouts.clear();
            w.descriptions.clear();
            w.top = 0;
            this.ctx.request_layout();
        }
        w.set_selected(selected);
        this.ctx.request_render();
    }

    /// Names each row for screen readers with `names` (same order as the
    /// items) while the rows show their shorter text.
    pub fn with_names(mut self, names: Vec<String>) -> Self {
        self.names = names;
        self
    }

    /// Draws each row's key at the right edge (`keys`, same order as the
    /// items; an empty key draws none) and gives each row a description
    /// for screen readers (`descriptions`; may be empty).
    pub fn with_keys(mut self, keys: Vec<String>, descriptions: Vec<String>) -> Self {
        self.key_layouts = (0..keys.len()).map(|_| None).collect();
        self.keys = keys;
        self.descriptions = descriptions;
        self
    }

    /// Row `i`'s key, drawn at the right edge, if any.
    pub fn key(&self, i: usize) -> Option<&str> {
        self.keys
            .get(i)
            .map(String::as_str)
            .filter(|k| !k.is_empty())
    }

    /// Row `i`'s description for screen readers, if any.
    pub fn description(&self, i: usize) -> Option<&str> {
        self.descriptions
            .get(i)
            .map(String::as_str)
            .filter(|d| !d.is_empty())
    }

    /// Shows the rows of a list of commands from the app's list model:
    /// `items` drawn, `names` for screen readers, `keys` at the right
    /// edge, and `descriptions`, with `selected` focused.
    pub fn sync_rows(this: &mut WidgetMut<'_, Self>, rows: Rows, selected: usize) {
        let w = &mut *this.widget;
        if w.items != rows.items || w.keys != rows.keys || w.names != rows.names {
            w.layouts = (0..rows.items.len()).map(|_| None).collect();
            w.key_layouts = (0..rows.keys.len()).map(|_| None).collect();
            w.items = rows.items;
            w.names = rows.names;
            w.keys = rows.keys;
            w.top = 0;
            this.ctx.request_layout();
        }
        w.descriptions = rows.descriptions;
        w.set_selected(selected);
        this.ctx.request_render();
    }

    /// Replaces the rows (a filtered list of commands), selecting the
    /// first.
    pub fn set_rows(this: &mut WidgetMut<'_, Self>, rows: Rows) {
        Self::set_named_items(this, rows.items, rows.names);
        let w = &mut *this.widget;
        w.key_layouts = (0..rows.keys.len()).map(|_| None).collect();
        w.keys = rows.keys;
        w.descriptions = rows.descriptions;
    }

    /// Row `i`'s accessible name: its own name, or its text.
    pub fn name(&self, i: usize) -> Option<&str> {
        self.names
            .get(i)
            .or_else(|| self.items.get(i))
            .map(String::as_str)
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
        Self::set_named_items(this, items, Vec::new());
    }

    /// Replaces the items and their accessible names
    /// ([`with_names`](Self::with_names)), selecting the first.
    pub fn set_named_items(this: &mut WidgetMut<'_, Self>, items: Vec<String>, names: Vec<String>) {
        let w = &mut *this.widget;
        w.layouts = (0..items.len()).map(|_| None).collect();
        w.items = items;
        w.names = names;
        w.keys.clear();
        w.key_layouts.clear();
        w.descriptions.clear();
        w.selected = 0;
        w.top = 0;
        this.ctx.request_layout();
        this.ctx.request_render();
    }

    /// Moves the selection (for the driver and tests).
    pub fn select(this: &mut WidgetMut<'_, Self>, i: usize) {
        this.widget.set_selected(i);
        if this.widget.needs_text() {
            this.ctx.request_layout();
        }
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

    /// True when a row in view has no text layout yet: the list scrolled,
    /// and the next layout pass must build them, or the rows are drawn
    /// blank (text layouts are built in `layout`, not in `paint`).
    fn needs_text(&self) -> bool {
        let last = (self.top + self.visible_rows + 1).min(self.items.len());
        self.layouts
            .get(self.top..last)
            .is_some_and(|rows| rows.iter().any(Option::is_none))
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
        if k.state != KeyState::Down {
            return;
        }
        let command = crate::keys::is_command(k.modifiers, self.platform);
        let function = matches!(&k.key, Key::Named(n) if *n != NamedKey::F2 && crate::keys::is_function_key(n));
        if self.app_keys
            && (command || function)
            && let Some(chord) = crate::keys::chord(k, self.platform)
        {
            ctx.submit_action::<DialogAction>(DialogAction::Chord(chord));
            ctx.set_handled();
            return;
        }
        if command {
            return;
        }
        if self.app_keys {
            use textweaver_app::ListKey as L;
            let key = match &k.key {
                Key::Named(NamedKey::ArrowDown) => L::Down,
                Key::Named(NamedKey::ArrowUp) => L::Up,
                Key::Named(NamedKey::ArrowLeft) => L::Left,
                Key::Named(NamedKey::ArrowRight) => L::Right,
                Key::Named(NamedKey::Home) => L::Home,
                Key::Named(NamedKey::End) => L::End,
                Key::Named(NamedKey::PageDown) => L::PageDown,
                Key::Named(NamedKey::PageUp) => L::PageUp,
                Key::Named(NamedKey::Enter) => L::Enter,
                Key::Named(NamedKey::Escape) => L::Escape,
                Key::Named(NamedKey::Backspace) => L::Backspace,
                Key::Named(NamedKey::Delete) => L::Delete,
                Key::Named(NamedKey::F2) => L::Rename,
                Key::Character(s) => match s.chars().next() {
                    Some(c) if !c.is_control() => L::Char(c),
                    _ => return,
                },
                _ => return,
            };
            ctx.submit_action::<DialogAction>(DialogAction::Key(key));
            ctx.set_handled();
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
            Key::Named(NamedKey::Enter) if self.shift_enter_to_parent && k.modifiers.shift() => {
                return;
            }
            Key::Named(NamedKey::Enter) => {
                ctx.submit_action::<DialogAction>(DialogAction::Choose(self.selected));
                ctx.set_handled();
                return;
            }
            Key::Character(s) if s != " " => s.chars().next().and_then(|c| self.first_letter(c)),
            _ => return,
        };
        if let Some(i) = new {
            let moved = i != self.selected;
            self.set_selected(i);
            if self.needs_text() {
                ctx.request_layout();
            }
            ctx.request_render();
            if moved && self.focus_actions {
                ctx.submit_action::<DialogAction>(DialogAction::Focus(self.selected));
            }
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
                    let double = row == self.selected && state.count >= 2;
                    self.set_selected(row);
                    ctx.request_render();
                    if self.app_keys || self.focus_actions {
                        ctx.submit_action::<DialogAction>(DialogAction::Focus(row));
                        if double {
                            ctx.submit_action::<DialogAction>(DialogAction::Key(
                                textweaver_app::ListKey::Enter,
                            ));
                        }
                    }
                    if !self.app_keys && double {
                        ctx.submit_action::<DialogAction>(DialogAction::Choose(row));
                    }
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
                if self.needs_text() {
                    ctx.request_layout();
                }
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
        // Screen readers choose an option by clicking or focusing it (the
        // action names the option's node), or set the list's value.
        let row = match (event.node, &event.data) {
            (Some(node), _) => match self.option_ids.iter().position(|id| *id == node) {
                Some(i) if i < self.items.len() => i,
                _ => return,
            },
            (None, Some(ActionData::NumericValue(v))) if event.action == Action::SetValue => {
                *v as usize
            }
            _ => return,
        };
        if !matches!(
            event.action,
            Action::Focus | Action::Click | Action::ScrollIntoView | Action::SetValue
        ) {
            return;
        }
        self.set_selected(row);
        if self.needs_text() {
            ctx.request_layout();
        }
        ctx.request_render();
        if self.app_keys || self.focus_actions {
            ctx.submit_action::<DialogAction>(DialogAction::Focus(self.selected));
        }
        ctx.set_handled();
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
            self.key_layouts.iter_mut().for_each(|l| *l = None);
        }
        let last = (self.top + self.visible_rows + 1).min(self.items.len());
        let (fcx, lcx) = ctx.text_contexts();
        for i in self.top..last {
            if self.layouts[i].is_none() {
                let mut build = |text: &str| {
                    let mut b = lcx.ranged_builder(fcx, text, 1.0, true);
                    b.push_default(StyleProperty::FontFamily(FontFamily::Source(
                        crate::fonts::DEFAULT_STACK.into(),
                    )));
                    b.push_default(StyleProperty::FontSize(theme::ui_size(
                        theme::UI_TEXT + 1.0,
                    )));
                    b.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(1.3)));
                    b.push_default(StyleProperty::Brush(BrushIndex(0)));
                    let mut l = b.build(text);
                    l.break_all_lines(None);
                    l
                };
                // The key, at the right edge, takes its width (and a gap)
                // from the row's text.
                let mut key_w = 0.0;
                if let Some(key) = self.keys.get(i).filter(|k| !k.is_empty()) {
                    let l = build(key);
                    key_w = l.width() + 24.0;
                    self.key_layouts[i] = Some(l);
                }
                // A row too long for the list ends with an ellipsis at a
                // word, never a hard cut; its option keeps the whole text
                // as its name (W9b-n).
                let room = ((size.width - 2.0 * ROW_PAD - 8.0) as f32 - key_w).max(0.0);
                let text = self.items[i].as_str();
                let mut l = build(text);
                if l.width() > room {
                    let shown = ellipsize(text, |t| build(t).width() <= room);
                    l = build(&shown);
                }
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
                if !self.focused {
                    // A hairline, so the selection shows where the raised
                    // colour is the page's (high contrast).
                    painter
                        .stroke(
                            RoundedRect::from_rect(row.inset(-2.0), theme::RADIUS),
                            &Stroke::new(1.0),
                            theme::color(p.border),
                        )
                        .draw();
                }
            }
            let text_fg = if selected && !self.focused {
                p.text
            } else {
                fg
            };
            if self.current.as_ref().is_some_and(|(c, _)| *c == i) {
                // The row where the reader is: a 3 px bar at its start, in
                // the row's text color, so it shows on any fill.
                painter
                    .fill(
                        RoundedRect::new(1.0, y + 6.0, 4.0, y + self.row_h - 6.0, 1.5),
                        theme::color(text_fg),
                    )
                    .draw();
            }
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
            if let Some(Some(k)) = self.key_layouts.get(i) {
                let ty = y + (self.row_h - f64::from(k.height())) / 2.0;
                let x = size.width - ROW_PAD - 4.0 - f64::from(k.width());
                render_text(
                    painter,
                    Affine::translate(Vec2::new(x, ty)),
                    k,
                    &[theme::color(text_fg).into()],
                    true,
                );
            }
        }
        // A scroll hint when there are more items than rows.
        let n = self.items.len();
        if n > self.visible_rows {
            let track = Rect::new(size.width - 3.0, 4.0, size.width - 1.0, size.height - 4.0);
            let h = track.height() * (self.visible_rows as f64 / n as f64);
            let top = track.y0 + track.height() * (self.top as f64 / n as f64);
            painter
                .fill(
                    RoundedRect::new(track.x0, top, track.x1, top + h, 1.0),
                    theme::with_alpha(p.dim_text, 0.6),
                )
                .draw();
        }
        if self.focused {
            // The double ring (design system D), inside the list's edge.
            theme::paint_ring_inside(painter, size.to_rect(), theme::RADIUS, p);
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
        // The list clips its painting, and Masonry tells AccessKit so, but
        // then AccessKit's filter leaves out the options scrolled out of the
        // box (all but the first one past each edge), and a screen reader's
        // object navigation cannot reach them. Every option is a node with
        // its scrolled bounds, so the list does not claim to clip them.
        node.clear_clips_children();
        while self.option_ids.len() < self.items.len() {
            self.option_ids.push(AccessCtx::next_node_id());
        }
        let n = self.items.len();
        for (i, item) in self.items.iter().enumerate() {
            let mut o = Node::new(Role::ListBoxOption);
            match &self.current {
                Some((c, label)) if *c == i => o.set_label(label.as_str()),
                _ => o.set_label(self.names.get(i).unwrap_or(item).as_str()),
            }
            if let Some(d) = self.descriptions.get(i).filter(|d| !d.is_empty()) {
                o.set_description(d.as_str());
            }
            o.set_selected(i == self.selected);
            o.add_action(Action::Focus);
            o.add_action(Action::Click);
            o.add_action(Action::ScrollIntoView);
            // AccessKit's position is zero-based (the adapters add one), and
            // the size belongs on the list, not on each option.
            o.set_position_in_set(i);
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
        node.set_size_of_set(n);
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

#[cfg(test)]
mod tests {
    use super::ellipsize;

    #[test]
    fn a_long_row_ends_with_an_ellipsis_at_a_word() {
        let fits = |n: usize| move |t: &str| t.chars().count() <= n;
        assert_eq!(ellipsize("Short", fits(10)), "Short");
        assert_eq!(
            ellipsize("Introduction to the history of reading", fits(22)),
            "Introduction to the…"
        );
        // One long word is cut inside it, still with the ellipsis.
        assert_eq!(ellipsize("Antidisestablishment", fits(8)), "Antidis…");
        assert_eq!(ellipsize("Word", fits(0)), "…");
    }
}
