//! The main window: the widget tree, and the driver that connects it to
//! the app core.
//!
//! The layout follows Xilem's `to_do_mvc` example (a padded column of
//! rounded panels on a dark page): a header with the document's title, the
//! document, a toolbar with Play and Stop, and a status bar. Everything a
//! screen reader needs has a role and a name.
//!
//! The driver owns the [`App`]. A ticker thread wakes the event loop (every
//! 30 ms while reading, 250 ms otherwise) to poll speech; W3a's waker will
//! replace it (ADR-0023, "Waiting on W3a").

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use masonry::accesskit::Role;
use masonry::app::RenderRoot;
use masonry::core::{
    ErasedAction, NewWidget, PropertySet, StyleProperty, Widget, WidgetId, WidgetMut, WidgetTag,
};
use masonry::layout::Length;
use masonry::parley::style::{FontFamily, FontWeight};
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Background, Padding};
use masonry::widgets::{Flex, Label, TextArea, TextInput};
use masonry_winit::app::{
    AppDriver, DriverCtx, EventLoop, EventLoopProxy, MasonryState, MasonryUserEvent, NewWindow,
    WindowId,
};
use masonry_winit::winit::dpi::{LogicalSize, PhysicalPosition};
use masonry_winit::winit::window::Window as WinitWindow;
use textweaver_app::a11y::{Announcer as AppAnnouncer, Priority};
use textweaver_app::core::CharRange;
use textweaver_app::keymap::{ActionId, Platform};
use textweaver_app::store::DocKey;
use textweaver_app::{App, Command, Effect, Playback, PromptPurpose, extra_lookup};

use crate::dialog::{self, ChoiceList, DialogAction, Modal};
use crate::document::{DocAction, DocFont, DocModel, DocState, DocumentView};
use crate::keys;
use crate::setup::{self, Options};
use crate::theme::{self, Palette};
use crate::widgets::{
    ActionButton, Announcer, KeyAction, Message, MessageQueue, Pressed, Region, Root,
};
use crate::window::{self, TextWindow, WINDOW_CHARS};

/// The document view.
pub const DOC: WidgetTag<DocumentView> = WidgetTag::named("tw-document");
/// The live region.
pub const ANNOUNCER: WidgetTag<Announcer> = WidgetTag::named("tw-announcer");
/// The root.
pub const ROOT: WidgetTag<Root> = WidgetTag::named("tw-root");
/// The Play/Pause button.
pub const PLAY: WidgetTag<ActionButton> = WidgetTag::named("tw-play");
/// The status text.
pub const STATUS: WidgetTag<Label> = WidgetTag::named("tw-status");
/// The status bar region (named by its text).
pub const STATUS_BAR: WidgetTag<Region> = WidgetTag::named("tw-status-bar");
/// The position in the status bar.
pub const POSITION: WidgetTag<Label> = WidgetTag::named("tw-position");
/// The document's title in the header.
pub const TITLE: WidgetTag<Label> = WidgetTag::named("tw-title");
/// The field of an open prompt.
pub const PROMPT_FIELD: WidgetTag<TextArea<true>> = WidgetTag::named("tw-prompt-field");
/// The list of an open list dialog.
pub const LIST: WidgetTag<ChoiceList> = WidgetTag::named("tw-list");

/// How often speech is polled while reading.
const READING_TICK: Duration = Duration::from_millis(30);
/// How often it is polled otherwise.
const IDLE_TICK: Duration = Duration::from_millis(250);
/// Highlight moves slower than this are logged.
const SLOW_HIGHLIGHT_MS: f64 = 30.0;

/// Everything the window needs to start.
#[derive(Debug, Default, Clone)]
pub struct GuiOptions {
    /// App options (speech, home).
    pub app: Options,
    /// Document to open.
    pub file: Option<PathBuf>,
    /// Start reading once the document is open.
    pub read_on_start: bool,
    /// Close after this long (automated checks).
    pub exit_after: Option<Duration>,
    /// Log announcements, commands, keys, and timings.
    pub log: bool,
    /// Automated runs: never activated, off screen, no taskbar button.
    pub background: bool,
    /// Select the spoken word instead of putting the caret on it.
    pub select_spoken: bool,
    /// A theme to use instead of the saved one.
    pub theme: Option<String>,
}

/// Wakes the event loop.
#[derive(Debug)]
struct Tick;

/// The app's announcer for the GUI: queues messages for the live region
/// (and the log). `muted` silences bookkeeping dispatches.
struct QueueAnnouncer {
    queue: MessageQueue,
    muted: Rc<Cell<bool>>,
    log: bool,
}

impl AppAnnouncer for QueueAnnouncer {
    fn announce(&mut self, text: &str, priority: Priority) {
        let muted = self.muted.get();
        if self.log {
            let state = if muted { " (muted)" } else { "" };
            crate::log::line(&format!("announce {priority:?}{state}: {text}"));
        }
        if !muted {
            self.queue.borrow_mut().push_back(Message {
                text: text.to_owned(),
                priority,
            });
        }
    }
}

/// What the window last showed, to update only what changed.
#[derive(Default)]
struct Shown {
    doc: Option<(DocKey, usize)>,
    window: Option<TextWindow>,
    state: DocState,
    status: String,
    position: String,
    reading: bool,
    title: String,
}

/// An open dialog.
enum OpenDialog {
    Prompt,
    List,
    /// The command palette: the actions its list shows, in order.
    Palette(Vec<ActionId>),
}

/// The widget tree's toolbar buttons and what they do.
struct Buttons {
    by_id: HashMap<WidgetId, ActionId>,
}

/// The driver: the app and the window's state.
pub struct Gui {
    app: App,
    queue: MessageQueue,
    muted: Rc<Cell<bool>>,
    window_id: WindowId,
    palette: Palette,
    shown: Shown,
    buttons: Buttons,
    dialog: Option<OpenDialog>,
    log: bool,
    started: bool,
    startup: Option<(Option<PathBuf>, bool, Vec<String>)>,
    exit_at: Option<Instant>,
    reading_flag: Arc<AtomicBool>,
    /// The ticker starts once the window exists (in `on_start`).
    ticker: Option<EventLoopProxy>,
    closed: bool,
    /// Load and highlight timings, for `--log` and the measurements.
    pub timings: Timings,
}

/// Measured times, in milliseconds.
#[derive(Clone, Debug, Default)]
pub struct Timings {
    /// Building the window's text and runs for a document.
    pub load_ms: Vec<(usize, f64)>,
    /// Moving the highlight.
    pub highlight_ms: Vec<f64>,
}

/// The pieces the tree is built from, shared by the window and the
/// screenshot harness.
pub struct Tree {
    /// The root widget.
    pub root: NewWidget<Root>,
    buttons: Buttons,
}

fn label(text: &str, size: f32, bold: bool) -> Label {
    let mut l = Label::new(text.to_owned())
        .with_style(StyleProperty::FontSize(size))
        .with_style(StyleProperty::FontFamily(FontFamily::Source(
            crate::fonts::DEFAULT_STACK.into(),
        )));
    if bold {
        l = l.with_style(StyleProperty::FontWeight(FontWeight::BOLD));
    }
    l
}

fn button(
    text: &str,
    action: ActionId,
    app: Option<&App>,
    ids: &mut HashMap<WidgetId, ActionId>,
) -> NewWidget<ActionButton> {
    styled_button(text, action, app, ids, None)
}

fn styled_button(
    text: &str,
    action: ActionId,
    app: Option<&App>,
    ids: &mut HashMap<WidgetId, ActionId>,
    text_color: Option<masonry::peniko::Color>,
) -> NewWidget<ActionButton> {
    let shortcut = app
        .map(|a| {
            a.keymap()
                .chords_in_mode(action, textweaver_app::keymap::Layer::Browse)
        })
        .and_then(|c| c.first().map(keys::shortcut_text))
        .unwrap_or_default();
    let mut b = ActionButton::new(text)
        .with_shortcut(shortcut)
        .with_description(action.help());
    if let Some(c) = text_color {
        b = b.with_text_color(c);
    }
    let w = NewWidget::new(b);
    ids.insert(w.id(), action);
    w
}

fn panel(p: &Palette, pad_v: f64, pad_h: f64) -> PropertySet {
    let (bg, border, bw, radius, shadow) = theme::panel_props(p);
    (
        bg,
        border,
        bw,
        radius,
        shadow,
        Padding::from_vh(Length::px(pad_v), Length::px(pad_h)),
    )
        .into()
}

/// Builds the window's widget tree.
pub fn build_tree(
    palette: &Palette,
    font: DocFont,
    app: Option<&App>,
    full_passes: Rc<Cell<u64>>,
    select_spoken: bool,
) -> Tree {
    let mut ids = HashMap::new();
    let p = palette;

    // Header: the document's title and the commands.
    let title = NewWidget::new(label("textweaver", 18.0, true)).with_tag(TITLE);
    let header = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with(title, 1.0)
        .with_fixed(button("Open…", ActionId::Open, app, &mut ids))
        .with_fixed(button("Commands…", ActionId::CommandPalette, app, &mut ids));
    let header = NewWidget::new(Region::new(NewWidget::new(header), Role::Banner, ""))
        .with_props(panel(p, 10.0, 16.0));

    // The document.
    let doc = NewWidget::new(
        DocumentView::new(p.clone(), font, Rc::clone(&full_passes))
            .with_select_spoken(select_spoken),
    )
    .with_tag(DOC);

    // Toolbar: Play/Pause is the primary action.
    let play = styled_button(
        "Play",
        ActionId::PlayPause,
        app,
        &mut ids,
        Some(theme::color(p.on_accent)),
    )
    .with_tag(PLAY)
    .with_class("primary");
    let toolbar = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_fixed(play)
        .with_fixed(button("Stop", ActionId::Stop, app, &mut ids))
        .with_fixed(button(
            "Previous sentence",
            ActionId::PreviousSentence,
            app,
            &mut ids,
        ))
        .with_fixed(button(
            "Next sentence",
            ActionId::NextSentence,
            app,
            &mut ids,
        ))
        .with_spacer(1.0)
        .with_fixed(button("Slower", ActionId::RateDown, app, &mut ids))
        .with_fixed(button("Faster", ActionId::RateUp, app, &mut ids));
    let toolbar = NewWidget::new(Region::new(
        NewWidget::new(toolbar),
        Role::Toolbar,
        "Reading",
    ))
    .with_props(panel(p, 10.0, 12.0));

    // Status bar: the latest message and the position.
    let status_text = NewWidget::new(label("", theme::UI_TEXT, false).accessibility_hidden(true))
        .with_tag(STATUS);
    let position = NewWidget::new(label("", theme::UI_TEXT, false).accessibility_hidden(true))
        .with_tag(POSITION);
    let status = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with(status_text, 1.0)
        .with_fixed(position);
    let status = NewWidget::new(Region::new(NewWidget::new(status), Role::Status, ""))
        .with_tag(STATUS_BAR)
        .with_props(panel(p, 8.0, 16.0));

    let announcer = NewWidget::new(Announcer::new(Rc::clone(&full_passes))).with_tag(ANNOUNCER);

    let column = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(header)
        .with(doc, 1.0)
        .with_fixed(toolbar)
        .with_fixed(status)
        .with_fixed(announcer);
    let main = NewWidget::new(Region::new(
        NewWidget::new(column).with_props(Padding::all(Length::px(theme::PAD))),
        Role::GenericContainer,
        "",
    ))
    .with_props(Background::Color(theme::color(p.background)));
    let root = NewWidget::new(Root::new(main, full_passes)).with_tag(ROOT);
    Tree {
        root,
        buttons: Buttons { by_id: ids },
    }
}

/// A list dialog: its title, the list, and a hint, as a modal card.
/// Returns the dialog and the list's id (to focus it).
pub fn list_dialog(
    p: &Palette,
    title: &str,
    items: Vec<String>,
    selected: usize,
) -> (NewWidget<dyn Widget>, WidgetId) {
    let list = NewWidget::new(ChoiceList::new(title, items, p.clone()).with_selected(selected))
        .with_tag(LIST);
    let list_id = list.id();
    let card = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(NewWidget::new(
            label(title, 18.0, true).accessibility_hidden(true),
        ))
        .with_fixed(list)
        .with_fixed(NewWidget::new(
            label("Enter chooses, Escape closes.", theme::UI_TEXT, false)
                .accessibility_hidden(true),
        ));
    let card = NewWidget::new(card).with_props(dialog::card_props(p));
    let modal = NewWidget::new(Modal::new(card, title, p.clone())).erased();
    (modal, list_id)
}

/// Where the driver edits widgets: the live window or the test harness.
pub trait Host {
    /// Edits the widget with `tag`.
    fn edit<W: Widget + masonry::core::FromDynWidget + ?Sized, R>(
        &mut self,
        tag: WidgetTag<W>,
        f: impl FnOnce(WidgetMut<'_, W>) -> R,
    ) -> R;
}

impl Host for RenderRoot {
    fn edit<W: Widget + masonry::core::FromDynWidget + ?Sized, R>(
        &mut self,
        tag: WidgetTag<W>,
        f: impl FnOnce(WidgetMut<'_, W>) -> R,
    ) -> R {
        self.edit_widget_with_tag(tag, f)
    }
}

#[cfg(any(test, feature = "screenshot"))]
impl<RW: Widget> Host for masonry_testing::TestHarness<RW> {
    fn edit<W: Widget + masonry::core::FromDynWidget + ?Sized, R>(
        &mut self,
        tag: WidgetTag<W>,
        f: impl FnOnce(WidgetMut<'_, W>) -> R,
    ) -> R {
        self.edit_widget(tag, f)
    }
}

/// The document view's model for the window `w` of the session's document.
pub fn model_for(app: &App, w: CharRange) -> Option<DocModel> {
    let s = app.session()?;
    Some(DocModel {
        paragraphs: window::window_paragraphs(&s.doc, w),
        spans: window::window_spans(&s.doc, w),
        doc_len: s.doc.len_chars(),
        title: s.title.clone(),
    })
}

/// The document view's state from the app.
pub fn state_for(app: &App) -> DocState {
    let Some(s) = app.session() else {
        return DocState::default();
    };
    let reading = app.playback() == Playback::Reading;
    let anchor = s.selection.and_then(|r| {
        if r.start == s.cursor {
            Some(r.end)
        } else if r.end == s.cursor {
            Some(r.start)
        } else {
            None
        }
    });
    DocState {
        caret: s.cursor,
        anchor,
        spoken: s.spoken,
        sentence: s.spoken_sentence,
        reading,
    }
}

/// Brings `host` up to date with `app`. Returns the load time in
/// milliseconds when the document's text (or window) was rebuilt.
fn refresh_host(app: &App, shown: &mut Shown, host: &mut impl Host, log: bool) -> Option<f64> {
    let mut loaded = None;
    match app.session() {
        None => {
            if shown.doc.take().is_some() {
                host.edit(DOC, |mut d| {
                    DocumentView::set_model(&mut d, DocModel::default())
                });
                shown.window = None;
            }
        }
        Some(s) => {
            let id = (s.key.clone(), s.doc.len_chars());
            let focus = match (app.playback(), s.spoken) {
                (Playback::Reading, Some(r)) => r.start,
                _ => s.cursor,
            };
            let rebuild =
                shown.doc.as_ref() != Some(&id) || shown.window.is_none_or(|w| w.needs_move(focus));
            if rebuild {
                let started = Instant::now();
                let w = TextWindow::around(&s.doc, focus, WINDOW_CHARS);
                if let Some(model) = model_for(app, w.range) {
                    host.edit(DOC, |mut d| DocumentView::set_model(&mut d, model));
                }
                let ms = started.elapsed().as_secs_f64() * 1000.0;
                if log {
                    crate::log::line(&format!(
                        "loaded {} chars: window {}..{} in {ms:.1} ms",
                        s.doc.len_chars(),
                        w.range.start.0,
                        w.range.end.0
                    ));
                }
                shown.doc = Some(id);
                shown.window = Some(w);
                loaded = Some(ms);
            }
        }
    }
    let state = state_for(app);
    if state != shown.state {
        let started = Instant::now();
        host.edit(DOC, |mut d| DocumentView::set_state(&mut d, state));
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        if log && ms >= SLOW_HIGHLIGHT_MS {
            crate::log::line(&format!("slow highlight: {ms:.1} ms at {:?}", state.spoken));
        }
        shown.state = state;
    }
    let reading = app.playback() == Playback::Reading;
    if reading != shown.reading {
        host.edit(PLAY, |mut b| {
            ActionButton::set_label(&mut b, if reading { "Pause" } else { "Play" });
        });
        shown.reading = reading;
    }
    let status = app.status_text().to_owned();
    if status != shown.status {
        host.edit(STATUS, |mut l| Label::set_text(&mut l, status.clone()));
        shown.status = status;
    }
    let position = app
        .session()
        .map(|s| format!("Line {}, {}%", s.line() + 1, s.percent()))
        .unwrap_or_default();
    if position != shown.position {
        host.edit(POSITION, |mut l| Label::set_text(&mut l, position.clone()));
        shown.position = position;
    }
    let bar = [shown.status.as_str(), shown.position.as_str()]
        .iter()
        .map(|s| s.trim().trim_end_matches('.'))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(". ");
    host.edit(STATUS_BAR, |mut r| Region::set_label(&mut r, bar));
    let title = app
        .session()
        .map_or_else(|| "textweaver".to_owned(), |s| s.title.clone());
    if title != shown.title {
        host.edit(TITLE, |mut l| Label::set_text(&mut l, title.clone()));
        shown.title = title;
    }
    loaded
}

impl Gui {
    fn refresh(&mut self, ctx: &mut DriverCtx<'_>) {
        let reading = self.app.playback() == Playback::Reading;
        self.reading_flag.store(reading, Ordering::Relaxed);
        let root = ctx.render_root(self.window_id);
        let before = self.shown.state;
        if let Some(ms) = refresh_host(&self.app, &mut self.shown, root, self.log) {
            let len = self.app.session().map_or(0, |s| s.doc.len_chars());
            self.timings.load_ms.push((len, ms));
        }
        if before.spoken != self.shown.state.spoken {
            // The widget's share is measured in its accessibility pass; the
            // edit itself is recorded here.
            self.timings.highlight_ms.push(0.0);
        }
        let messages: Vec<Message> = self.queue.borrow_mut().drain(..).collect();
        if !messages.is_empty() {
            root.edit_widget_with_tag(ANNOUNCER, |mut a| Announcer::say(&mut a, messages));
        }
        let title = self.app.session().map_or_else(
            || "textweaver".to_owned(),
            |s| format!("{} - textweaver", s.title),
        );
        ctx.window(self.window_id).handle().set_title(&title);
    }

    fn dispatch(&mut self, ctx: &mut DriverCtx<'_>, cmd: Command) {
        if self.log {
            crate::log::line(&format!("command {cmd:?}"));
        }
        let effects = self.app.dispatch(cmd);
        self.run_effects(ctx, effects);
        self.refresh(ctx);
    }

    fn run_effects(&mut self, ctx: &mut DriverCtx<'_>, effects: Vec<Effect>) {
        let mut queue: VecDeque<Effect> = effects.into();
        while let Some(e) = queue.pop_front() {
            match e {
                Effect::Redraw => {}
                Effect::Quit => {
                    self.close(ctx);
                    return;
                }
                Effect::Prompt {
                    label,
                    purpose: PromptPurpose::CommandPalette,
                } => self.open_palette(ctx, &label),
                Effect::Prompt { label, purpose } => self.open_prompt(ctx, &label, purpose),
                Effect::ShowList { title, items } => self.open_list(ctx, &title, items),
            }
        }
    }

    fn open_prompt(&mut self, ctx: &mut DriverCtx<'_>, label_text: &str, purpose: PromptPurpose) {
        let p = &self.palette;
        let hint = match purpose {
            PromptPurpose::Open => "Type the full path of a document, then press Enter.",
            _ => "Press Enter to accept, or Escape to cancel.",
        };
        let field = NewWidget::new(
            TextArea::new_editable("")
                .with_accessible_label(label_text.to_owned())
                .with_style(StyleProperty::FontSize(theme::UI_TEXT + 2.0)),
        )
        .with_tag(PROMPT_FIELD);
        let field_id = field.id();
        let input = NewWidget::new(TextInput::from_text_area(field).with_placeholder(hint));
        let card = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(NewWidget::new(
                label(label_text, 18.0, true).accessibility_hidden(true),
            ))
            .with_fixed(input)
            .with_fixed(NewWidget::new(
                label(hint, theme::UI_TEXT, false).accessibility_hidden(true),
            ));
        let card = NewWidget::new(card).with_props(dialog::card_props(p));
        let modal = NewWidget::new(Modal::new(card, label_text, p.clone())).erased();
        let root = ctx.render_root(self.window_id);
        root.edit_widget_with_tag(ROOT, |mut r| Root::set_dialog(&mut r, Some(modal)));
        root.focus_on(Some(field_id));
        self.dialog = Some(OpenDialog::Prompt);
        if self.log {
            crate::log::line(&format!("dialog: prompt {label_text:?}"));
        }
    }

    fn open_list(&mut self, ctx: &mut DriverCtx<'_>, title: &str, items: Vec<String>) {
        let count = items.len();
        let (modal, list_id) = list_dialog(&self.palette, title, items, 0);
        let root = ctx.render_root(self.window_id);
        root.edit_widget_with_tag(ROOT, |mut r| Root::set_dialog(&mut r, Some(modal)));
        root.focus_on(Some(list_id));
        self.dialog = Some(OpenDialog::List);
        if self.log {
            crate::log::line(&format!("dialog: list {title:?} with {count} items"));
        }
    }

    /// The command palette, the GUI's menu: a filter field over the list of
    /// every command with its keys. Typing filters (and says how many
    /// match); Tab reaches the list; Enter runs the selected command.
    fn open_palette(&mut self, ctx: &mut DriverCtx<'_>, label_text: &str) {
        let p = &self.palette;
        let (ids, items): (Vec<ActionId>, Vec<String>) =
            self.app.palette_candidates("").into_iter().unzip();
        let count = items.len();
        let field = NewWidget::new(
            TextArea::new_editable("")
                .with_accessible_label(label_text.to_owned())
                .with_style(StyleProperty::FontSize(theme::UI_TEXT + 2.0)),
        )
        .with_tag(PROMPT_FIELD);
        let field_id = field.id();
        let input = NewWidget::new(
            TextInput::from_text_area(field).with_placeholder("Type to filter the commands"),
        );
        let list = NewWidget::new(ChoiceList::new("Commands", items, p.clone())).with_tag(LIST);
        let card = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(NewWidget::new(
                label(label_text, 18.0, true).accessibility_hidden(true),
            ))
            .with_fixed(input)
            .with_fixed(list)
            .with_fixed(NewWidget::new(
                label(
                    "Enter runs the first match; Tab moves to the list.",
                    theme::UI_TEXT,
                    false,
                )
                .accessibility_hidden(true),
            ));
        let card = NewWidget::new(card).with_props(dialog::card_props(p));
        let modal = NewWidget::new(Modal::new(card, label_text, p.clone())).erased();
        let root = ctx.render_root(self.window_id);
        root.edit_widget_with_tag(ROOT, |mut r| Root::set_dialog(&mut r, Some(modal)));
        root.focus_on(Some(field_id));
        self.dialog = Some(OpenDialog::Palette(ids));
        if self.log {
            crate::log::line(&format!("dialog: command palette with {count} commands"));
        }
    }

    /// The palette's filter changed: show the matches and say how many.
    fn filter_palette(&mut self, ctx: &mut DriverCtx<'_>, query: &str) {
        let (ids, items): (Vec<ActionId>, Vec<String>) =
            self.app.palette_candidates(query).into_iter().unzip();
        let n = items.len();
        ctx.render_root(self.window_id)
            .edit_widget_with_tag(LIST, |mut l| ChoiceList::set_items(&mut l, items));
        self.dialog = Some(OpenDialog::Palette(ids));
        let said = match n {
            0 => "No commands match.".to_owned(),
            1 => "1 command.".to_owned(),
            n => format!("{n} commands."),
        };
        self.app.announce(&said, Priority::Polite);
        self.refresh(ctx);
    }

    fn close_dialog(&mut self, ctx: &mut DriverCtx<'_>) {
        self.dialog = None;
        let root = ctx.render_root(self.window_id);
        root.edit_widget_with_tag(ROOT, |mut r| Root::set_dialog(&mut r, None));
        let doc = root.get_widget_with_tag(DOC).map(|w| w.id());
        root.focus_on(doc);
    }

    fn answer(&mut self, ctx: &mut DriverCtx<'_>, cmd: Command) {
        // One answer per dialog: Escape reaches both the field and the
        // dialog, and only the first counts.
        if self.dialog.is_none() {
            return;
        }
        self.close_dialog(ctx);
        self.dispatch(ctx, cmd);
    }

    fn on_key(&mut self, ctx: &mut DriverCtx<'_>, k: &masonry::core::keyboard::KeyboardEvent) {
        let Some(chord) = keys::chord(k, Platform::current()) else {
            return;
        };
        let layer = self.app.mode().layer();
        let action = (!keys::is_native(&chord))
            .then(|| self.app.keymap().lookup(&chord, layer))
            .flatten();
        if self.log {
            crate::log::line(&format!("key {chord} -> {action:?}"));
        }
        if let Some(a) = action {
            self.dispatch(ctx, Command::Action(a));
        } else if let Some(cmd) = extra_lookup(&chord, layer) {
            self.dispatch(ctx, cmd);
        }
    }

    fn close(&mut self, ctx: &mut DriverCtx<'_>) {
        if !self.closed {
            self.closed = true;
            self.app.shutdown();
            if self.log {
                crate::log::line("closed");
            }
        }
        ctx.exit();
    }

    fn start(&mut self, ctx: &mut DriverCtx<'_>) {
        let Some((file, read, messages)) = self.startup.take() else {
            return;
        };
        let mut effects = Vec::new();
        match &file {
            Some(path) => {
                let started = Instant::now();
                match self.app.open(path) {
                    Ok(e) => effects.extend(e),
                    Err(e) => self.app.announce(
                        &format!("Could not open {}: {e}", path.display()),
                        Priority::Assertive,
                    ),
                }
                if self.log {
                    crate::log::line(&format!(
                        "opened in {:.1} ms",
                        started.elapsed().as_secs_f64() * 1000.0
                    ));
                }
            }
            None => self.app.announce(
                "No document is open. Press Control O to open one.",
                Priority::Polite,
            ),
        }
        for m in &messages {
            self.app.announce(m, Priority::Assertive);
        }
        self.run_effects(ctx, effects);
        self.refresh(ctx);
        let doc = ctx
            .render_root(self.window_id)
            .get_widget_with_tag(DOC)
            .map(|w| w.id());
        ctx.render_root(self.window_id).focus_on(doc);
        if read && self.app.session().is_some() {
            self.dispatch(ctx, Command::Action(ActionId::ReadFromCursor));
        }
    }
}

impl AppDriver for Gui {
    fn on_action(
        &mut self,
        _window_id: WindowId,
        ctx: &mut DriverCtx<'_>,
        widget_id: WidgetId,
        action: ErasedAction,
    ) {
        if let Some(KeyAction(k)) = action.downcast_ref::<KeyAction>() {
            let k = k.clone();
            self.on_key(ctx, &k);
        } else if let Some(DocAction::CaretMoved { caret, .. }) = action.downcast_ref::<DocAction>()
        {
            let caret = *caret;
            let app_cursor = self.app.session().map(|s| s.cursor);
            if app_cursor != Some(caret) {
                if self.log {
                    crate::log::line(&format!("caret sync: {caret:?}"));
                }
                self.muted.set(true);
                let effects = self.app.dispatch(Command::SetCursor(caret));
                self.muted.set(false);
                self.run_effects(ctx, effects);
                // The view already shows the caret; keep the window in step.
                self.shown.state.caret = caret;
                self.refresh(ctx);
            }
        } else if action.downcast_ref::<Pressed>().is_some() {
            if let Some(a) = self.buttons.by_id.get(&widget_id).copied() {
                self.dispatch(ctx, Command::Action(a));
            }
        } else if let Some(d) = action.downcast_ref::<DialogAction>() {
            let cmd = match (d, &self.dialog) {
                (DialogAction::Choose(i), Some(OpenDialog::Palette(ids))) => match ids.get(*i) {
                    Some(a) => Command::Answer(a.id().to_owned()),
                    None => Command::Cancel,
                },
                (DialogAction::Choose(i), _) => Command::Choose(*i),
                (DialogAction::Cancel, _) => Command::Cancel,
            };
            self.answer(ctx, cmd);
        } else if let Some(t) = action.downcast_ref::<masonry::widgets::TextAction>() {
            match t {
                masonry::widgets::TextAction::Entered(text) => {
                    // In the palette, Enter runs the first match.
                    let answer = match &self.dialog {
                        Some(OpenDialog::Palette(ids)) => ids
                            .first()
                            .map_or_else(|| text.clone(), |a| a.id().to_owned()),
                        _ => text.clone(),
                    };
                    self.answer(ctx, Command::Answer(answer));
                }
                masonry::widgets::TextAction::Cancelled => self.answer(ctx, Command::Cancel),
                masonry::widgets::TextAction::Changed(q) => {
                    if matches!(self.dialog, Some(OpenDialog::Palette(_))) {
                        let q = q.clone();
                        self.filter_palette(ctx, &q);
                    }
                }
            }
        }
    }

    fn on_async_action(
        &mut self,
        _window_id: WindowId,
        ctx: &mut DriverCtx<'_>,
        action: ErasedAction,
    ) {
        if action.downcast_ref::<Tick>().is_none() || self.closed {
            return;
        }
        if !self.started {
            self.started = true;
            self.start(ctx);
            return;
        }
        if let Some(at) = self.exit_at
            && Instant::now() >= at
        {
            if self.log {
                crate::log::line("exit-after: closing the window");
            }
            self.close(ctx);
            return;
        }
        let mut effects = self.app.poll_speech();
        effects.extend(self.app.tick(Instant::now()));
        self.run_effects(ctx, effects);
        self.refresh(ctx);
    }

    fn on_start(&mut self, state: &mut MasonryState) {
        if let Some(proxy) = self.ticker.take() {
            spawn_ticker(proxy, self.window_id, Arc::clone(&self.reading_flag));
        }
        for root in state.roots() {
            for blob in crate::fonts::bundled_blobs() {
                let _ = root.register_fonts(blob);
            }
        }
    }

    fn on_close_requested(&mut self, _window_id: WindowId, ctx: &mut DriverCtx<'_>) {
        self.close(ctx);
    }
}

/// How long `--exit-after` waits for the window to close before ending the
/// process itself.
const EXIT_GRACE: Duration = Duration::from_secs(15);

/// Runs the GUI until the window closes.
pub fn run(opts: GuiOptions) -> Result<(), String> {
    let queue: MessageQueue = Rc::new(RefCell::new(VecDeque::new()));
    let muted = Rc::new(Cell::new(false));
    let announcer = QueueAnnouncer {
        queue: Rc::clone(&queue),
        muted: Rc::clone(&muted),
        log: opts.log,
    };
    let (app, messages) = setup::build_app(&opts.app, Box::new(announcer));
    let palette = match &opts.theme {
        Some(name) => Palette::named(name),
        None => Palette::from_theme(app.current_theme()),
    };
    let font = crate::fonts::doc_font(&app.settings().reading_aids.font);
    let full_passes = Rc::new(Cell::new(0));
    let tree = build_tree(&palette, font, Some(&app), full_passes, opts.select_spoken);

    let mut attrs = WinitWindow::default_attributes()
        .with_title("textweaver")
        .with_inner_size(LogicalSize::new(1100.0, 780.0))
        .with_min_inner_size(LogicalSize::new(420.0, 320.0));
    if opts.background {
        // Never activated, off screen, and (on Windows) no taskbar button:
        // it cannot take focus from the person at the machine, and its
        // controls stay in the accessibility tree for the checks.
        attrs = attrs
            .with_active(false)
            .with_position(PhysicalPosition::new(-30_000, -30_000));
        #[cfg(windows)]
        {
            use masonry_winit::winit::platform::windows::WindowAttributesExtWindows;
            attrs = attrs.with_skip_taskbar(true);
        }
    }
    let window =
        NewWindow::new(attrs, tree.root.erased()).with_base_color(theme::color(palette.background));
    let window_id = window.id;

    let event_loop = EventLoop::with_user_event()
        .build()
        .map_err(|e| format!("cannot start the event loop: {e}"))?;
    let proxy = event_loop.create_proxy();
    let reading_flag = Arc::new(AtomicBool::new(false));
    if let Some(after) = opts.exit_after {
        exit_watchdog(after);
    }

    let gui = Gui {
        app,
        queue,
        muted,
        window_id,
        palette,
        shown: Shown::default(),
        buttons: tree.buttons,
        dialog: None,
        log: opts.log,
        started: false,
        startup: Some((opts.file.clone(), opts.read_on_start, messages)),
        exit_at: opts.exit_after.map(|d| Instant::now() + d),
        reading_flag,
        ticker: Some(proxy),
        closed: false,
        timings: Timings::default(),
    };
    let default_props = theme::default_properties(&gui.palette);
    masonry_winit::app::run_with(event_loop, vec![window], gui, default_props)
        .map_err(|e| format!("the event loop failed: {e}"))
}

fn spawn_ticker(proxy: EventLoopProxy, window_id: WindowId, reading: Arc<AtomicBool>) {
    static TICKS: AtomicU64 = AtomicU64::new(0);
    std::thread::spawn(move || {
        loop {
            let every = if reading.load(Ordering::Relaxed) {
                READING_TICK
            } else {
                IDLE_TICK
            };
            TICKS.fetch_add(1, Ordering::Relaxed);
            if proxy
                .send_event(MasonryUserEvent::AsyncAction(window_id, Box::new(Tick)))
                .is_err()
            {
                return;
            }
            std::thread::sleep(every);
        }
    });
}

/// For `--exit-after`: if the process is still running long after the
/// window should have closed, say so and exit with status 3.
fn exit_watchdog(after: Duration) {
    std::thread::spawn(move || {
        std::thread::sleep(after + EXIT_GRACE);
        crate::log::line(&format!(
            "exit-after: the window did not close within {} s",
            EXIT_GRACE.as_secs()
        ));
        std::process::exit(3);
    });
}

/// Refreshes a test harness or screenshot host from `app`.
pub fn refresh_for_tests(app: &App, host: &mut impl Host) {
    let mut shown = Shown::default();
    let _ = refresh_host(app, &mut shown, host, false);
}
