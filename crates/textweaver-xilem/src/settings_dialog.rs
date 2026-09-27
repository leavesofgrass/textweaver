//! The settings dialog, built from the app's settings schema (ADR-0024).
//!
//! The schema (`App::settings_schema`) lists every setting with its
//! section, label, help, and kind. The dialog shows the sections as a list
//! on the left and the chosen section's settings as a form on the right:
//!
//! - an on-or-off setting is a check box;
//! - a number is a slider, with its range, step, and unit;
//! - a choice is a combo box showing the chosen value;
//! - text and lists are edit fields, typed in a prompt (Enter);
//! - tables (pronunciations, abbreviations) show their size and are edited
//!   in `settings.toml`.
//!
//! The form ([`SettingsGrid`]) is one focusable widget whose settings are
//! AccessKit nodes, like the lists ([`ChoiceList`](crate::dialog::ChoiceList)):
//! the focused setting is the form's active descendant, so a screen reader
//! reads it with its role, value, and help. Up and Down move between
//! settings, Left and Right change one, Space turns a switch on or off,
//! Enter types a new value, Delete puts the default back, and Control Page
//! Down and Control Page Up move between sections. Every change goes
//! through `App::set_setting`, which checks it, puts it into effect, and
//! saves it on the writer thread. The screen reader hears the new value
//! from the control itself; the dialog only announces what the control
//! cannot show (a value out of range, a change that needs a restart).

use masonry::accesskit::{Action, ActionData, Node, NodeId, Role, Toggled};
use masonry::core::keyboard::{Key, KeyState, NamedKey};
use masonry::core::{
    AccessCtx, AccessEvent, BrushIndex, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, PaintCtx,
    PointerButtonEvent, PointerEvent, PointerScrollEvent, PropertiesMut, PropertiesRef,
    RegisterCtx, StyleProperty, TextEvent, Update, UpdateCtx, Widget, WidgetMut, render_text,
};
use masonry::dpi::{LogicalPosition, PhysicalPosition};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, BezPath, Circle, Point, Rect, RoundedRect, Size, Stroke, Vec2};
use masonry::layout::{LenReq, Length};
use masonry::parley::Layout;
use masonry::parley::style::{FontFamily, FontWeight, LineHeight};
use serde_json::Value;
use textweaver_app::{App, Setting, SettingKind, SettingsSchema};

use crate::theme::{self, Palette};

// --- The model.

/// How a setting is shown and changed in the form.
#[derive(Clone, Debug, PartialEq)]
pub enum RowKind {
    /// A check box, on or off.
    Toggle(bool),
    /// A slider.
    Number {
        /// The value.
        value: f64,
        /// Smallest value.
        min: f64,
        /// Largest value.
        max: f64,
        /// How much Left and Right change it.
        step: f64,
    },
    /// A combo box: Left and Right take the previous or next choice.
    Choice,
    /// An edit field, typed in a prompt.
    Text,
    /// A table, edited in `settings.toml`.
    Table,
}

/// One setting as the form shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct FormRow {
    /// The key's path in `settings.toml`.
    pub path: String,
    /// The label, shown and spoken.
    pub label: String,
    /// One or two sentences of help (the control's description).
    pub help: String,
    /// How it is shown and changed.
    pub kind: RowKind,
    /// The value as shown and said ("300 words per minute", "Galaxy").
    pub value_text: String,
}

impl FormRow {
    /// The row for `setting` with its current `value`.
    pub fn new(setting: &Setting, value: &Value) -> Self {
        let kind = match &setting.kind {
            SettingKind::Toggle => RowKind::Toggle(value.as_bool().unwrap_or(false)),
            SettingKind::Number { min, max, step, .. } => RowKind::Number {
                value: value.as_f64().unwrap_or(*min),
                min: *min,
                max: *max,
                step: *step,
            },
            SettingKind::Choice { .. } => RowKind::Choice,
            SettingKind::Text { .. } | SettingKind::List => RowKind::Text,
            SettingKind::Table => RowKind::Table,
        };
        FormRow {
            path: setting.path.clone(),
            label: setting.label.to_owned(),
            help: setting.help.to_owned(),
            kind,
            value_text: setting.describe(value),
        }
    }

    /// What the prompt starts with when this setting is typed (Enter).
    pub fn is_typed(&self) -> bool {
        matches!(self.kind, RowKind::Number { .. } | RowKind::Text)
    }
}

/// A change asked for in the form.
#[derive(Clone, Debug, PartialEq)]
pub enum FormChange {
    /// One step: `true` is larger, the next choice, or on.
    Step(bool),
    /// The default value.
    Reset,
    /// A value typed in the prompt, or set by a screen reader.
    Text(String),
    /// A number set by a screen reader (the slider's `SetValue`).
    Number(f64),
}

/// The dialog's sections, in the schema's order, and their settings.
#[derive(Clone, Debug)]
pub struct SettingsForm {
    schema: SettingsSchema,
    /// Section titles ("Speech", "Display").
    pub sections: Vec<&'static str>,
}

impl SettingsForm {
    /// The form for `schema` (from `App::settings_schema`), leaving out the
    /// settings textweaver keeps for itself.
    pub fn new(schema: SettingsSchema) -> Self {
        let mut sections: Vec<&'static str> = Vec::new();
        for s in schema.visible() {
            if !sections.contains(&s.section) {
                sections.push(s.section);
            }
        }
        SettingsForm { schema, sections }
    }

    /// The settings in section `i`, in order.
    pub fn settings_in(&self, i: usize) -> Vec<&Setting> {
        let Some(title) = self.sections.get(i) else {
            return Vec::new();
        };
        self.schema
            .visible()
            .filter(|s| s.section == *title)
            .collect()
    }

    /// The setting shown at `row` of section `section`.
    pub fn setting(&self, section: usize, row: usize) -> Option<&Setting> {
        self.settings_in(section).get(row).copied()
    }

    /// The rows of section `i`, with `app`'s current values.
    pub fn rows(&self, i: usize, app: &App) -> Vec<FormRow> {
        self.settings_in(i)
            .into_iter()
            .map(|s| FormRow::new(s, &app.setting_value(&s.path).unwrap_or(Value::Null)))
            .collect()
    }

    /// The section list's items: each title with how many settings it has.
    pub fn section_items(&self) -> Vec<String> {
        (0..self.sections.len())
            .map(|i| {
                let n = self.settings_in(i).len();
                let noun = if n == 1 { "setting" } else { "settings" };
                format!("{}, {n} {noun}", self.sections[i])
            })
            .collect()
    }

    /// Where the setting at `path` is: its section and row.
    pub fn find(&self, path: &str) -> Option<(usize, usize)> {
        (0..self.sections.len()).find_map(|sec| {
            self.settings_in(sec)
                .iter()
                .position(|s| s.path == path)
                .map(|row| (sec, row))
        })
    }
}

/// Makes `change` to `setting` through the app. Returns what the app says
/// ("Rate, 300 words per minute."), or why it cannot be done.
pub fn apply(app: &mut App, setting: &Setting, change: FormChange) -> Result<String, String> {
    let now = app.setting_value(&setting.path).unwrap_or(Value::Null);
    let value = match change {
        FormChange::Step(forward) => setting.stepped(&now, forward).ok_or_else(|| match setting
            .kind
        {
            SettingKind::Table => {
                format!("{} is a table. Edit it in settings.toml.", setting.label)
            }
            _ => format!(
                "Press Enter to type a new {}.",
                setting.label.to_lowercase()
            ),
        })?,
        FormChange::Reset => Value::Null,
        FormChange::Text(t) => setting.parse(&t)?,
        FormChange::Number(n) => setting.parse(&n.to_string())?,
    };
    app.set_setting(&setting.path, value)
}

/// What to announce after `said` (from [`apply`]) for `row` as it now is:
/// only what the control does not show itself, such as "Out of range, so
/// the nearest value is used." `None` when the control says it all.
pub fn extra_note(said: &str, row: &FormRow) -> Option<String> {
    let plain = format!("{}, {}.", row.label, row.value_text);
    let rest = said.strip_prefix(plain.as_str()).unwrap_or(said).trim();
    (!rest.is_empty()).then(|| rest.to_owned())
}

// --- The form widget.

/// What the form asks the driver to do.
#[derive(Clone, Debug, PartialEq)]
pub enum FormAction {
    /// Change the setting at `row`.
    Change {
        /// The row.
        row: usize,
        /// The change.
        change: FormChange,
    },
    /// Type a new value for the setting at `row` (a prompt), starting
    /// with `text`.
    Edit {
        /// The row.
        row: usize,
        /// The prompt's first text.
        text: String,
    },
    /// Move to the next (1) or previous (-1) section.
    Section(isize),
}

const ROW_H: f64 = 46.0;
const HELP_H: f64 = 58.0;
const ROW_PAD: f64 = 14.0;
/// Rows shown at most before the form scrolls.
const MAX_ROWS: usize = 9;

/// The settings of one section as a form. See the module documentation.
pub struct SettingsGrid {
    rows: Vec<FormRow>,
    title: String,
    selected: usize,
    top: usize,
    visible_rows: usize,
    focused: bool,
    palette: Palette,
    node_ids: Vec<NodeId>,
    labels: Vec<Option<Layout<BrushIndex>>>,
    values: Vec<Option<Layout<BrushIndex>>>,
    help: Option<(usize, Layout<BrushIndex>)>,
    width: f64,
}

impl SettingsGrid {
    /// The settings of the section `title`.
    pub fn new(title: impl Into<String>, rows: Vec<FormRow>, palette: Palette) -> Self {
        let n = rows.len();
        SettingsGrid {
            rows,
            title: title.into(),
            selected: 0,
            top: 0,
            visible_rows: MAX_ROWS,
            focused: false,
            palette,
            node_ids: Vec::new(),
            labels: vec![None; n],
            values: vec![None; n],
            help: None,
            width: 0.0,
        }
    }

    /// Starts on row `i`.
    pub fn with_selected(mut self, i: usize) -> Self {
        self.set_selected(i);
        self
    }

    /// The focused row.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// The rows shown.
    pub fn rows(&self) -> &[FormRow] {
        &self.rows
    }

    /// Shows new values for the same settings (after a change), keeping
    /// the focus; only the rows whose value changed are laid out again.
    pub fn update_rows(this: &mut WidgetMut<'_, Self>, rows: Vec<FormRow>) {
        let w = &mut *this.widget;
        if rows.len() != w.rows.len() {
            w.labels = vec![None; rows.len()];
            w.values = vec![None; rows.len()];
        } else {
            for (i, (old, new)) in w.rows.iter().zip(&rows).enumerate() {
                if old.value_text != new.value_text {
                    w.values[i] = None;
                }
                if old.label != new.label {
                    w.labels[i] = None;
                }
            }
        }
        w.rows = rows;
        let selected = w.selected;
        w.set_selected(selected);
        this.ctx.request_layout();
        this.ctx.request_render();
        this.ctx.request_accessibility_update();
    }

    /// Shows another section's rows, focused on `selected`.
    pub fn set_section(
        this: &mut WidgetMut<'_, Self>,
        title: impl Into<String>,
        rows: Vec<FormRow>,
        selected: usize,
    ) {
        let w = &mut *this.widget;
        w.title = title.into();
        w.labels = vec![None; rows.len()];
        w.values = vec![None; rows.len()];
        w.help = None;
        w.rows = rows;
        w.top = 0;
        w.set_selected(selected);
        this.ctx.request_layout();
        this.ctx.request_render();
        this.ctx.request_accessibility_update();
    }

    fn set_selected(&mut self, i: usize) {
        if self.rows.is_empty() {
            self.selected = 0;
            return;
        }
        self.selected = i.min(self.rows.len() - 1);
        let visible = self.visible_rows.max(1);
        if self.selected < self.top {
            self.top = self.selected;
        } else if self.selected >= self.top + visible {
            self.top = self.selected + 1 - visible;
        }
    }

    fn first_letter(&self, c: char) -> Option<usize> {
        let c = c.to_lowercase().next()?;
        let n = self.rows.len();
        (1..=n).map(|k| (self.selected + k) % n).find(|&i| {
            self.rows[i]
                .label
                .chars()
                .next()
                .and_then(|f| f.to_lowercase().next())
                == Some(c)
        })
    }

    /// Enter (or a click) on `row`: a switch or choice steps; a number or
    /// text is typed.
    fn activate(&self, row: usize) -> Option<FormAction> {
        let r = self.rows.get(row)?;
        Some(match r.kind {
            RowKind::Toggle(_) | RowKind::Choice => FormAction::Change {
                row,
                change: FormChange::Step(true),
            },
            RowKind::Number { value, step, .. } => FormAction::Edit {
                row,
                text: number_text(value, step),
            },
            RowKind::Text | RowKind::Table => FormAction::Edit {
                row,
                text: String::new(),
            },
        })
    }

    fn row_of_node(&self, node: NodeId) -> Option<usize> {
        self.node_ids
            .iter()
            .position(|id| *id == node)
            .filter(|&i| i < self.rows.len())
    }

    fn text_layout(
        ctx: &mut LayoutCtx<'_>,
        text: &str,
        size: f32,
        bold: bool,
        width: Option<f32>,
    ) -> Layout<BrushIndex> {
        let (fcx, lcx) = ctx.text_contexts();
        let mut b = lcx.ranged_builder(fcx, text, 1.0, true);
        b.push_default(StyleProperty::FontFamily(FontFamily::Source(
            crate::fonts::DEFAULT_STACK.into(),
        )));
        b.push_default(StyleProperty::FontSize(size));
        b.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(1.3)));
        b.push_default(StyleProperty::Brush(BrushIndex(0)));
        if bold {
            b.push_default(StyleProperty::FontWeight(FontWeight::SEMI_BOLD));
        }
        let mut l = b.build(text);
        l.break_all_lines(width);
        l
    }
}

/// A number as typed: no trailing zeros, whole for whole steps.
fn number_text(value: f64, step: f64) -> String {
    if step.fract() == 0.0 && value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        let s = format!("{value:.3}");
        s.trim_end_matches('0').trim_end_matches('.').to_owned()
    }
}

impl Widget for SettingsGrid {
    type Action = FormAction;

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
        if k.state != KeyState::Down || k.modifiers.alt() || k.modifiers.meta() {
            return;
        }
        if k.modifiers.ctrl() {
            let delta = match &k.key {
                Key::Named(NamedKey::PageDown) => 1,
                Key::Named(NamedKey::PageUp) => -1,
                Key::Named(NamedKey::Tab) if k.modifiers.shift() => -1,
                Key::Named(NamedKey::Tab) => 1,
                _ => return,
            };
            ctx.submit_action::<FormAction>(FormAction::Section(delta));
            ctx.set_handled();
            return;
        }
        let n = self.rows.len();
        if n == 0 {
            return;
        }
        let page = self.visible_rows.max(2) - 1;
        let row = self.selected;
        let kind = self.rows[row].kind.clone();
        let steps = !matches!(kind, RowKind::Text | RowKind::Table);
        let action = match &k.key {
            Key::Named(NamedKey::ArrowDown) => {
                self.set_selected((row + 1).min(n - 1));
                None
            }
            Key::Named(NamedKey::ArrowUp) => {
                self.set_selected(row.saturating_sub(1));
                None
            }
            Key::Named(NamedKey::Home) => {
                self.set_selected(0);
                None
            }
            Key::Named(NamedKey::End) => {
                self.set_selected(n - 1);
                None
            }
            Key::Named(NamedKey::PageDown) => {
                self.set_selected((row + page).min(n - 1));
                None
            }
            Key::Named(NamedKey::PageUp) => {
                self.set_selected(row.saturating_sub(page));
                None
            }
            Key::Named(NamedKey::ArrowRight) if steps => Some(FormAction::Change {
                row,
                change: FormChange::Step(true),
            }),
            Key::Named(NamedKey::ArrowLeft) if steps => Some(FormAction::Change {
                row,
                change: FormChange::Step(false),
            }),
            Key::Named(NamedKey::Delete) => Some(FormAction::Change {
                row,
                change: FormChange::Reset,
            }),
            Key::Named(NamedKey::Enter) => self.activate(row),
            Key::Named(NamedKey::F2) => match kind {
                RowKind::Number { value, step, .. } => Some(FormAction::Edit {
                    row,
                    text: number_text(value, step),
                }),
                _ => Some(FormAction::Edit {
                    row,
                    text: String::new(),
                }),
            },
            Key::Character(s) if s == " " => match kind {
                RowKind::Toggle(_) | RowKind::Choice => Some(FormAction::Change {
                    row,
                    change: FormChange::Step(true),
                }),
                _ => None,
            },
            Key::Character(s) => {
                if let Some(i) = s.chars().next().and_then(|c| self.first_letter(c)) {
                    self.set_selected(i);
                }
                None
            }
            _ => return,
        };
        if let Some(a) = action {
            ctx.submit_action::<FormAction>(a);
        }
        ctx.request_render();
        ctx.request_accessibility_update();
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
                let row = self.top + (local.y / ROW_H).max(0.0) as usize;
                if row < self.rows.len() && local.y < self.visible_rows as f64 * ROW_H {
                    let again = row == self.selected;
                    self.set_selected(row);
                    // A click on the value, or a second click on the row,
                    // changes it.
                    if (again || local.x > self.width * 0.55)
                        && let Some(a) = self.activate(row)
                    {
                        ctx.submit_action::<FormAction>(a);
                    }
                    ctx.request_render();
                    ctx.request_accessibility_update();
                }
                ctx.set_handled();
            }
            PointerEvent::Scroll(PointerScrollEvent { delta, .. }) => {
                let scale = ctx.scale_factor();
                let px = delta.to_pixel_delta(
                    PhysicalPosition::new(ROW_H * scale, ROW_H * scale),
                    PhysicalPosition::new(ROW_H * 6.0 * scale, ROW_H * 6.0 * scale),
                );
                let LogicalPosition { y, .. } = px.to_logical::<f64>(scale);
                let rows = (-y / ROW_H).round() as isize;
                let max_top = self.rows.len().saturating_sub(self.visible_rows);
                self.top = (self.top as isize + rows).clamp(0, max_top as isize) as usize;
                ctx.request_render();
                ctx.request_accessibility_update();
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
        // Actions on a setting's node arrive with the node named (the
        // vendored Masonry's patch); actions on the form act on the
        // focused setting.
        let row = match event.node {
            Some(node) => match self.row_of_node(node) {
                Some(r) => r,
                None => return,
            },
            None => self.selected,
        };
        if row >= self.rows.len() {
            return;
        }
        let action = match (event.action, &event.data) {
            (Action::Focus, _) => {
                self.set_selected(row);
                None
            }
            (Action::Click, _) => {
                self.set_selected(row);
                self.activate(row)
            }
            (Action::Increment, _) => Some(FormAction::Change {
                row,
                change: FormChange::Step(true),
            }),
            (Action::Decrement, _) => Some(FormAction::Change {
                row,
                change: FormChange::Step(false),
            }),
            (Action::SetValue, Some(ActionData::NumericValue(v))) => Some(FormAction::Change {
                row,
                change: FormChange::Number(*v),
            }),
            (Action::SetValue, Some(ActionData::Value(v))) => Some(FormAction::Change {
                row,
                change: FormChange::Text(v.to_string()),
            }),
            (Action::ScrollIntoView, _) => {
                self.set_selected(row);
                None
            }
            _ => return,
        };
        if let Some(a) = action {
            ctx.submit_action::<FormAction>(a);
        }
        ctx.request_render();
        ctx.request_accessibility_update();
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
                _ => Length::px(560.0),
            },
            Axis::Vertical => {
                let rows = self.rows.len().clamp(4, MAX_ROWS) as f64;
                Length::px(rows * ROW_H + HELP_H)
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        self.visible_rows = (((size.height - HELP_H) / ROW_H).floor() as usize).max(1);
        if (size.width - self.width).abs() > 0.5 {
            self.width = size.width;
            self.labels.iter_mut().for_each(|l| *l = None);
            self.values.iter_mut().for_each(|l| *l = None);
            self.help = None;
        }
        let selected = self.selected;
        self.set_selected(selected);
        let last = (self.top + self.visible_rows + 1).min(self.rows.len());
        for i in self.top..last {
            if self.labels[i].is_none() {
                let l =
                    Self::text_layout(ctx, &self.rows[i].label, theme::UI_TEXT + 1.0, true, None);
                self.labels[i] = Some(l);
            }
            if self.values[i].is_none() {
                let text = match self.rows[i].kind {
                    RowKind::Toggle(_) => String::new(),
                    _ => self.rows[i].value_text.clone(),
                };
                let max = (size.width * 0.42) as f32;
                let l = Self::text_layout(ctx, &text, theme::UI_TEXT + 1.0, false, Some(max));
                self.values[i] = Some(l);
            }
        }
        let help_row = self.selected;
        if self.help.as_ref().is_none_or(|(r, _)| *r != help_row)
            && let Some(r) = self.rows.get(help_row)
        {
            let w = (size.width - 2.0 * ROW_PAD).max(40.0) as f32;
            let l = Self::text_layout(ctx, &r.help, theme::UI_TEXT, false, Some(w));
            self.help = Some((help_row, l));
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
        let last = (self.top + self.visible_rows).min(self.rows.len());
        let rows_h = self.visible_rows as f64 * ROW_H;
        for i in self.top..last {
            let y = (i - self.top) as f64 * ROW_H;
            let row = Rect::new(0.0, y + 2.0, size.width, y + ROW_H - 2.0);
            let selected = i == self.selected;
            if selected {
                painter
                    .fill(
                        RoundedRect::from_rect(row, theme::RADIUS),
                        theme::color(p.raised),
                    )
                    .draw();
                // The focus ring, or (unfocused) a hairline, so the row stays
                // visible where the raised colour is the page's (high
                // contrast).
                let (width, colour) = if self.focused {
                    (theme::FOCUS_WIDTH, p.focus)
                } else {
                    (1.0, p.border)
                };
                painter
                    .stroke(
                        RoundedRect::from_rect(row.inset(-1.0), theme::RADIUS),
                        &Stroke::new(width),
                        theme::color(colour),
                    )
                    .draw();
            } else if i + 1 < last {
                // A hairline between settings.
                let line_y = y + ROW_H - 0.5;
                painter
                    .stroke(
                        masonry::kurbo::Line::new(
                            (ROW_PAD, line_y),
                            (size.width - ROW_PAD, line_y),
                        ),
                        &Stroke::new(1.0),
                        theme::with_alpha(p.border, 0.6),
                    )
                    .draw();
            }
            let mid = y + ROW_H / 2.0;
            if let Some(l) = &self.labels[i] {
                let ty = mid - f64::from(l.height()) / 2.0;
                render_text(
                    painter,
                    Affine::translate(Vec2::new(ROW_PAD, ty)),
                    l,
                    &[theme::color(p.text).into()],
                    true,
                );
            }
            let right = size.width - ROW_PAD;
            match &self.rows[i].kind {
                RowKind::Toggle(on) => paint_switch(painter, p, Point::new(right, mid), *on),
                kind => {
                    let arrows = matches!(kind, RowKind::Number { .. } | RowKind::Choice);
                    let inset = match kind {
                        _ if arrows => 22.0,
                        RowKind::Text => 10.0,
                        _ => 0.0,
                    };
                    if let Some(l) = &self.values[i] {
                        let w = f64::from(l.width());
                        let x = right - inset - w;
                        let ty = mid - f64::from(l.height()) / 2.0;
                        if matches!(kind, RowKind::Text) {
                            // An edit field's box around the value.
                            let field = Rect::new(
                                (x - 12.0).min(right - 120.0),
                                mid - 15.0,
                                right,
                                mid + 15.0,
                            );
                            painter
                                .fill(
                                    RoundedRect::from_rect(field, theme::RADIUS),
                                    theme::color(p.background),
                                )
                                .draw();
                            painter
                                .stroke(
                                    RoundedRect::from_rect(field, theme::RADIUS),
                                    &Stroke::new(1.0),
                                    theme::color(p.border),
                                )
                                .draw();
                        }
                        let fg = if matches!(kind, RowKind::Table) {
                            p.dim_text
                        } else {
                            p.text
                        };
                        render_text(
                            painter,
                            Affine::translate(Vec2::new(x, ty)),
                            l,
                            &[theme::color(fg).into()],
                            true,
                        );
                        if arrows {
                            paint_chevron(painter, p, Point::new(x - 14.0, mid), false);
                            paint_chevron(painter, p, Point::new(right - 7.0, mid), true);
                        }
                    }
                }
            }
        }
        // The focused setting's help, under the form.
        if let Some((_, l)) = &self.help {
            let y = rows_h + 8.0;
            painter
                .stroke(
                    masonry::kurbo::Line::new((0.0, rows_h + 2.0), (size.width, rows_h + 2.0)),
                    &Stroke::new(1.0),
                    theme::color(p.border),
                )
                .draw();
            render_text(
                painter,
                Affine::translate(Vec2::new(ROW_PAD, y)),
                l,
                &[theme::color(p.dim_text).into()],
                true,
            );
        }
        // A scroll hint when there is more above or below.
        if self.rows.len() > self.visible_rows {
            let track = Rect::new(size.width - 3.0, 4.0, size.width - 1.0, rows_h - 4.0);
            let n = self.rows.len() as f64;
            let h = track.height() * (self.visible_rows as f64 / n);
            let top = track.y0 + track.height() * (self.top as f64 / n);
            painter
                .fill(
                    RoundedRect::new(track.x0, top, track.x1, top + h, 1.0),
                    theme::with_alpha(p.dim_text, 0.6),
                )
                .draw();
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Group
    }

    fn accessibility(
        &mut self,
        ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_label(format!("{} settings", self.title));
        node.set_description(
            "Up and Down move between settings. Left and Right change one. \
             Enter types a new value. Delete puts the default back. \
             Control Page Down and Control Page Up change the section.",
        );
        node.add_action(Action::Increment);
        node.add_action(Action::Decrement);
        while self.node_ids.len() < self.rows.len() {
            self.node_ids.push(AccessCtx::next_node_id());
        }
        let n = self.rows.len();
        for (i, r) in self.rows.iter().enumerate() {
            let mut o = match &r.kind {
                RowKind::Toggle(on) => {
                    let mut o = Node::new(Role::CheckBox);
                    o.set_toggled(if *on { Toggled::True } else { Toggled::False });
                    o.add_action(Action::Click);
                    o
                }
                RowKind::Number {
                    value,
                    min,
                    max,
                    step,
                } => {
                    let mut o = Node::new(Role::Slider);
                    o.set_numeric_value(*value);
                    o.set_min_numeric_value(*min);
                    o.set_max_numeric_value(*max);
                    o.set_numeric_value_step(*step);
                    o.set_value(r.value_text.as_str());
                    o.add_action(Action::Increment);
                    o.add_action(Action::Decrement);
                    o.add_action(Action::SetValue);
                    o
                }
                RowKind::Choice => {
                    let mut o = Node::new(Role::ComboBox);
                    o.set_value(r.value_text.as_str());
                    o.add_action(Action::Click);
                    o.add_action(Action::Increment);
                    o.add_action(Action::Decrement);
                    o
                }
                RowKind::Text => {
                    let mut o = Node::new(Role::TextInput);
                    o.set_value(r.value_text.as_str());
                    o.add_action(Action::Click);
                    o.add_action(Action::SetValue);
                    o
                }
                RowKind::Table => {
                    let mut o = Node::new(Role::Label);
                    o.set_value(r.value_text.as_str());
                    o.set_read_only();
                    o
                }
            };
            o.set_label(r.label.as_str());
            if !r.help.is_empty() {
                o.set_description(r.help.as_str());
            }
            o.add_action(Action::Focus);
            o.add_action(Action::ScrollIntoView);
            o.set_position_in_set(i + 1);
            o.set_size_of_set(n);
            o.set_selected(i == self.selected);
            let y = (i as f64 - self.top as f64) * ROW_H;
            o.set_bounds(masonry::accesskit::Rect::new(0.0, y, self.width, y + ROW_H));
            ctx.tree_update().nodes.push((self.node_ids[i], o));
        }
        node.set_children(self.node_ids[..n].to_vec());
        if let Some(id) = self.node_ids.get(self.selected).filter(|_| n > 0) {
            node.set_active_descendant(*id);
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

/// A switch: a pill, filled with the accent when on, with its knob.
fn paint_switch(painter: &mut Painter<'_>, p: &Palette, right_mid: Point, on: bool) {
    let (w, h) = (42.0, 24.0);
    let pill = RoundedRect::new(
        right_mid.x - w,
        right_mid.y - h / 2.0,
        right_mid.x,
        right_mid.y + h / 2.0,
        h / 2.0,
    );
    if on {
        painter.fill(pill, theme::color(p.accent)).draw();
    } else {
        painter.fill(pill, theme::color(p.background)).draw();
        painter
            .stroke(pill, &Stroke::new(1.5), theme::color(p.dim_text))
            .draw();
    }
    let r = h / 2.0 - 4.0;
    let cx = if on {
        right_mid.x - h / 2.0
    } else {
        right_mid.x - w + h / 2.0
    };
    let knob = if on { p.on_accent } else { p.dim_text };
    painter
        .fill(Circle::new((cx, right_mid.y), r), theme::color(knob))
        .draw();
}

/// A small chevron pointing right (or left), centred on `at`.
fn paint_chevron(painter: &mut Painter<'_>, p: &Palette, at: Point, right: bool) {
    let d = if right { 1.0 } else { -1.0 };
    let mut path = BezPath::new();
    path.move_to((at.x - 3.0 * d, at.y - 5.0));
    path.line_to((at.x + 2.0 * d, at.y));
    path.line_to((at.x - 3.0 * d, at.y + 5.0));
    painter
        .stroke(&path, &Stroke::new(1.8), theme::color(p.dim_text))
        .draw();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(label: &str, value_text: &str) -> FormRow {
        FormRow {
            path: "x".into(),
            label: label.into(),
            help: String::new(),
            kind: RowKind::Choice,
            value_text: value_text.into(),
        }
    }

    #[test]
    fn only_notes_the_control_cannot_show_are_announced() {
        let r = row("Rate", "300 words per minute");
        assert_eq!(extra_note("Rate, 300 words per minute.", &r), None);
        assert_eq!(
            extra_note(
                "Rate, 300 words per minute. Out of range, so the nearest value is used.",
                &r
            )
            .as_deref(),
            Some("Out of range, so the nearest value is used.")
        );
        // Something else entirely is said whole.
        assert_eq!(
            extra_note("Rate cannot be that.", &r).as_deref(),
            Some("Rate cannot be that.")
        );
    }

    #[test]
    fn numbers_are_typed_without_trailing_zeros() {
        assert_eq!(number_text(300.0, 10.0), "300");
        assert_eq!(number_text(1.5, 0.1), "1.5");
        assert_eq!(number_text(0.3, 0.1), "0.3");
    }
}
