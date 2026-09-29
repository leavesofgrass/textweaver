//! The main window: the widget tree, and the driver that connects it to
//! the app core.
//!
//! The layout follows Xilem's `to_do_mvc` example (a padded column of
//! rounded panels on a dark page): a header with the document's title, the
//! document, a toolbar with Play and Stop, and a status bar. Everything a
//! screen reader needs has a role and a name.
//!
//! The driver owns the [`App`]. The app's waker (ADR-0024) posts a tick to
//! the event loop whenever speech or background work has something to
//! apply; a ticker thread covers the app's own timers, sleeping as long as
//! `App::tick_interval` allows.

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
use masonry::widgets::{Flex, Label, SizedBox, TextArea, TextInput};
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
use textweaver_app::{
    App, Command, DocWindow, Effect, Playback, PromptKey, PromptPurpose, WindowChange, extra_lookup,
};

use crate::dialog::{self, ChoiceList, DialogAction, Modal};
use crate::document::{DocAction, DocAids, DocFont, DocModel, DocState, DocumentView};
use crate::file_chooser::{self, FileChosen};
use crate::font_chooser::Step;
use crate::keys;
use crate::rsvp::{RsvpShown, RsvpView};
use crate::settings_dialog::{self, FormAction, FormChange, SettingsForm, SettingsGrid};
use crate::setup::{self, Options};
use crate::theme::{self, Palette};
use crate::widgets::{
    ActionButton, AnnounceMode, Announcer, KeyAction, Message, MessageQueue, Pressed, Region, Root,
};
use crate::window::{self, WINDOW_UNITS};

/// The document view.
pub const DOC: WidgetTag<DocumentView> = WidgetTag::named("tw-document");
/// The live region.
pub const ANNOUNCER: WidgetTag<Announcer> = WidgetTag::named("tw-announcer");
/// The root.
pub const ROOT: WidgetTag<Root> = WidgetTag::named("tw-root");
/// The Play/Pause button.
pub const PLAY: WidgetTag<ActionButton> = WidgetTag::named("tw-play");
/// The Edit button (edit mode on or off).
pub const EDIT: WidgetTag<ActionButton> = WidgetTag::named("tw-edit");
/// The status text.
pub const STATUS: WidgetTag<Label> = WidgetTag::named("tw-status");
/// The status bar region (named by its text).
pub const STATUS_BAR: WidgetTag<Region> = WidgetTag::named("tw-status-bar");
/// The position in the status bar.
pub const POSITION: WidgetTag<Label> = WidgetTag::named("tw-position");
/// The document's title in the header.
pub const TITLE: WidgetTag<Label> = WidgetTag::named("tw-title");
/// The header panel.
pub const HEADER: WidgetTag<Region> = WidgetTag::named("tw-header");
/// The toolbar panel.
pub const TOOLBAR: WidgetTag<Region> = WidgetTag::named("tw-toolbar");
/// The page behind the panels.
pub const MAIN: WidgetTag<Region> = WidgetTag::named("tw-main");
/// The field of an open prompt.
pub const PROMPT_FIELD: WidgetTag<TextArea<true>> = WidgetTag::named("tw-prompt-field");
/// The list of an open list dialog.
pub const LIST: WidgetTag<ChoiceList> = WidgetTag::named("tw-list");
/// The settings dialog's list of sections.
pub const SECTIONS: WidgetTag<ChoiceList> = WidgetTag::named("tw-settings-sections");
/// The settings dialog's form.
pub const FORM: WidgetTag<SettingsGrid> = WidgetTag::named("tw-settings-form");
/// The RSVP panel, under the document.
pub const RSVP: WidgetTag<RsvpView> = WidgetTag::named("tw-rsvp");

/// The first wait between ticks, before the app says (`App::tick_interval`).
const FIRST_TICK: Duration = Duration::from_millis(250);
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
    /// Listening-session experiments.
    pub experiments: Experiments,
    /// A theme to use instead of the saved one.
    pub theme: Option<String>,
}

/// Choices to compare by ear in a listening session (ADR-0027).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Experiments {
    /// While reading, select the spoken word instead of placing the caret
    /// at its start. Off by default: session 1 kept the background color
    /// (ADR-0028); this stays as an option.
    pub select_spoken: bool,
    /// Expose the document as a read-only multi-line edit instead of a
    /// Document.
    pub edit_role: bool,
    /// Let the app announce each list item as the focus moves, as the
    /// terminal reader does. Off by default: the list's options are
    /// AccessKit nodes the screen reader follows itself (the list's active
    /// descendant is its focus), and both would say each item twice.
    pub app_list_announcements: bool,
    /// Settings opens the app's settings list (as the terminal reader
    /// shows it) instead of the settings dialog.
    pub settings_list: bool,
    /// How announcements reach the screen reader (`--announce`); `None`
    /// follows the `[gui] announce` setting, and then the live region.
    pub announce: Option<AnnounceMode>,
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
    /// The document shown: its key and text revision.
    doc: Option<(DocKey, u64)>,
    window: Option<DocWindow>,
    state: DocState,
    status: String,
    position: String,
    reading: bool,
    title: String,
    /// The reading aids' settings the model's spans were built with.
    aid_spans: Option<AidSpansKey>,
    aids: DocAids,
    rsvp: Option<RsvpShown>,
    /// The document font, from `[reading_aids.font]`.
    font: Option<DocFont>,
    /// Edit mode, as the view shows it.
    editing: bool,
    /// Edit mode, as the Edit button says it.
    edit_button: bool,
}

/// What the reading aids' spans depend on: bionic reading (and its
/// options), difficult words, and syllables (and their options).
type AidSpansKey = (
    bool,
    textweaver_app::store::reading_aids::BionicOptions,
    bool,
    bool,
    textweaver_app::store::reading_aids::SyllableOptions,
);

fn aid_spans_key(app: &App) -> AidSpansKey {
    let a = &app.settings().reading_aids;
    (
        a.bionic,
        a.bionic_options.clone(),
        a.difficult_words,
        a.syllables,
        a.syllable_options.clone(),
    )
}

/// The settings dialog while it is open.
struct SettingsOpen {
    form: SettingsForm,
    section: usize,
    /// The Close button.
    close: WidgetId,
}

/// An open dialog.
enum OpenDialog {
    Prompt,
    /// The settings dialog.
    Settings(SettingsOpen),
    /// A prompt for a new value of the setting at this row of the settings
    /// dialog, which comes back when it closes.
    SettingEdit(SettingsOpen, usize),
    List,
    /// The command palette: the actions its list shows, in order.
    Palette(Vec<ActionId>),
    /// The font list.
    FontFamily(Vec<crate::font_chooser::Choice>),
    /// The system's file chooser, open on its own thread; the app's Open
    /// prompt (labelled with this) waits for its answer.
    FileChooser(String),
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
    /// The app's waker: speech statuses and finished background work post
    /// a tick at once (ADR-0024). Set when a tick is posted and not yet
    /// handled, so a burst of rings posts one.
    wake_pending: Arc<AtomicBool>,
    /// How long the ticker sleeps when nothing rings (`App::tick_interval`).
    tick_ms: Arc<AtomicU64>,
    /// Installed font families, for the font chooser.
    installed: crate::font_chooser::Installed,
    /// `--theme` was given: the saved theme is not followed.
    fixed_theme: bool,
    /// Settings opens the app's list instead of the dialog.
    settings_list: bool,
    /// The window title last set.
    window_title: String,
    /// The ticker starts once the window exists (in `on_start`).
    ticker: Option<EventLoopProxy>,
    /// Posts answers from other threads (the file chooser's).
    proxy: EventLoopProxy,
    /// The next Open prompt is the typed one (the Open Path key), not the
    /// system's file chooser.
    typed_open: bool,
    /// Whether single-key shortcuts were on when the buttons' shortcuts
    /// were last shown (F9 changes which key each button names).
    char_keys: Option<bool>,
    closed: bool,
    /// How announcements reach the screen reader.
    announce: AnnounceMode,
    /// The window's Win32 handle, for UI Automation notifications (read
    /// on first use; 0 until then or when there is none).
    hwnd: isize,
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
    let shortcut = app.map(|a| shortcut_for(a, action)).unwrap_or_default();
    let help = app.map_or_else(
        || action.help().to_owned(),
        |a| textweaver_app::action_help(&a.catalog(), action),
    );
    let mut b = ActionButton::new(text)
        .with_shortcut(shortcut)
        .with_description(help);
    if let Some(c) = text_color {
        b = b.with_text_color(c);
    }
    let w = NewWidget::new(b);
    ids.insert(w.id(), action);
    w
}

/// A control's shortcut for `action`, from the keymap (`named_key`), as
/// written ("Ctrl+O"): the button's keyboard shortcut and its text on
/// screen. The main key, which is the single key while single-key
/// shortcuts are on ("Space" for Play) and a chord while they are off.
pub fn shortcut_for(app: &App, action: ActionId) -> String {
    let named = textweaver_app::named_key_in(&app.catalog(), app.keymap(), action);
    textweaver_app::written_text(&named).into_owned()
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
    experiments: Experiments,
) -> Tree {
    let mut ids = HashMap::new();
    let p = palette;

    // Header: the document's title and the commands.
    let title = NewWidget::new(label("textweaver", 18.0, true)).with_tag(TITLE);
    let header = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with(title, 1.0)
        .with_fixed(button("Open…", ActionId::Open, app, &mut ids))
        .with_fixed(button("Font…", ActionId::ChooseFont, app, &mut ids))
        .with_fixed(button("Edit", ActionId::ToggleEditMode, app, &mut ids).with_tag(EDIT))
        .with_fixed(button("Settings…", ActionId::Settings, app, &mut ids))
        .with_fixed(button("Commands…", ActionId::CommandPalette, app, &mut ids));
    let header = NewWidget::new(Region::new(NewWidget::new(header), Role::Banner, ""))
        .with_tag(HEADER)
        .with_props(panel(p, 10.0, 16.0));

    // The document.
    let doc = NewWidget::new(
        DocumentView::new(p.clone(), font, Rc::clone(&full_passes))
            .with_select_spoken(experiments.select_spoken)
            .with_edit_role(experiments.edit_role),
    )
    .with_tag(DOC);
    // RSVP, hidden until it is turned on: its own strip under the document,
    // so the word never covers the text or the caret.
    let rsvp = NewWidget::new(RsvpView::new(p.clone())).with_tag(RSVP);

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
    .with_tag(TOOLBAR)
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

    let mode = experiments.announce.unwrap_or_default().effective();
    let announcer =
        NewWidget::new(Announcer::new(Rc::clone(&full_passes)).with_mode(mode)).with_tag(ANNOUNCER);

    let column = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(header)
        .with(doc, 1.0)
        .with_fixed(rsvp)
        .with_fixed(toolbar)
        .with_fixed(status)
        .with_fixed(announcer);
    let main = NewWidget::new(Region::new(
        NewWidget::new(column).with_props(Padding::all(Length::px(theme::PAD))),
        Role::GenericContainer,
        "",
    ))
    .with_tag(MAIN)
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
    app_keys: bool,
) -> (NewWidget<dyn Widget>, WidgetId) {
    let list = NewWidget::new(
        ChoiceList::new(title, items, p.clone())
            .with_selected(selected)
            .with_app_keys(app_keys),
    )
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

/// The settings dialog, built by [`settings_dialog()`].
pub struct SettingsDialog {
    /// The dialog, for [`Root::set_dialog`].
    pub modal: NewWidget<dyn Widget>,
    /// The form, which takes the focus.
    pub form: WidgetId,
    /// The Close button.
    pub close: WidgetId,
}

/// The settings dialog: the sections on the left, the chosen section's
/// settings as a form on the right, and a Close button. The form starts on
/// `row` of `section`.
pub fn settings_dialog(
    p: &Palette,
    form: &SettingsForm,
    app: &App,
    section: usize,
    row: usize,
) -> SettingsDialog {
    let title = form.sections.get(section).copied().unwrap_or("Settings");
    let sections = NewWidget::new(
        ChoiceList::new("Sections", form.section_items(), p.clone())
            .with_selected(section)
            .with_focus_actions(true),
    )
    .with_tag(SECTIONS);
    let grid = NewWidget::new(
        SettingsGrid::new(title, form.rows(section, app), p.clone()).with_selected(row),
    )
    .with_tag(FORM);
    let form_id = grid.id();
    // The dialog's own key (not a keymap command): Escape closes it.
    let escape = textweaver_app::keymap::KeyChord::new(
        textweaver_app::keymap::Key::Escape,
        textweaver_app::keymap::Modifiers::empty(),
    );
    let close = NewWidget::new(
        ActionButton::new("Close")
            .with_shortcut(escape.to_string())
            .with_description("Close the settings. Every change is already saved."),
    );
    let close_id = close.id();
    let body = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_fixed(NewWidget::new(
            SizedBox::new(sections).width(Length::px(250.0)),
        ))
        .with_fixed_spacer(Length::px(20.0))
        .with(grid, 1.0);
    let footer = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with(
            NewWidget::new(
                label(
                    "Changes take effect and are saved at once.",
                    theme::UI_TEXT,
                    false,
                )
                .accessibility_hidden(true),
            ),
            1.0,
        )
        .with_fixed(close);
    let card = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(NewWidget::new(
            label("Settings", 20.0, true).accessibility_hidden(true),
        ))
        .with_fixed_spacer(Length::px(14.0))
        .with_fixed(NewWidget::new(body))
        .with_fixed_spacer(Length::px(14.0))
        .with_fixed(NewWidget::new(footer));
    let card = NewWidget::new(card).with_props(dialog::card_props(p));
    let modal =
        NewWidget::new(Modal::new(card, "Settings", p.clone()).with_max_width(960.0)).erased();
    SettingsDialog {
        modal,
        form: form_id,
        close: close_id,
    }
}

/// Recolours the window's own panels and the document for `p` (the
/// default properties are replaced separately).
pub fn apply_palette(host: &mut impl Host, p: &Palette) {
    for tag in [HEADER, TOOLBAR, STATUS_BAR] {
        host.edit(tag, |mut r| {
            let (bg, border, bw, radius, shadow) = theme::panel_props(p);
            r.insert_prop(bg);
            r.insert_prop(border);
            r.insert_prop(bw);
            r.insert_prop(radius);
            r.insert_prop(shadow);
        });
    }
    host.edit(MAIN, |mut r| {
        r.insert_prop(Background::Color(theme::color(p.background)));
    });
    host.edit(PLAY, |mut b| {
        ActionButton::set_text_color(&mut b, theme::color(p.on_accent));
    });
    host.edit(DOC, |mut d| DocumentView::set_palette(&mut d, p.clone()));
    host.edit(RSVP, |mut r| RsvpView::set_palette(&mut r, p.clone()));
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
    let mut spans = window::window_spans(&s.doc, w);
    spans.extend(window::aid_spans(app, w));
    Some(DocModel {
        paragraphs: window::window_paragraphs(&s.doc, w),
        spans,
        doc_len: s.doc.len_chars(),
        title: s.title.clone(),
        // Syllables: drawn only, at the app's break positions (always
        // between two chars of the document), as the terminal draws them.
        breaks: app.syllable_breaks(w),
        separator: app.syllable_separator().to_owned(),
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
            let id = (s.key.clone(), s.revision);
            let focus = match (app.playback(), s.spoken) {
                (Playback::Reading, Some(r)) => r.start,
                _ => s.cursor,
            };
            let started = Instant::now();
            let new_doc = shown.doc.as_ref() != Some(&id);
            // Edit mode: the same document with its text changed (a key,
            // undo, a command): the paragraphs that stay keep their nodes,
            // as in a slide, so the screen reader keeps its place.
            let edited = app.is_editing()
                && shown.editing
                && shown.doc.as_ref().is_some_and(|(k, _)| *k == s.key);
            let mut w = match shown.window {
                Some(w) if !new_doc => w,
                _ => DocWindow::with_budget(&s.doc, focus, WINDOW_UNITS),
            };
            // The app's window follows the focus: it slides while reading
            // and recentres on jumps or when the text changed.
            let change = w.follow_session(s, focus);
            // Bionic reading or difficult words turned on or off: the same
            // text with new spans.
            let key = aid_spans_key(app);
            let aids_changed = shown.aid_spans.as_ref() != Some(&key);
            if new_doc || aids_changed || change != WindowChange::Unchanged {
                // A slide keeps the runs that stay (and the screen reader's
                // place on them); a new document or a jump replaces them.
                let slide = edited
                    || (!new_doc
                        && matches!(
                            change,
                            WindowChange::Forward { .. }
                                | WindowChange::Backward { .. }
                                | WindowChange::Unchanged
                        ));
                shown.aid_spans = Some(key);
                if let Some(model) = model_for(app, w.range()) {
                    host.edit(DOC, |mut d| {
                        if slide {
                            DocumentView::slide_model(&mut d, model);
                        } else {
                            DocumentView::set_model(&mut d, model);
                        }
                    });
                }
                let ms = started.elapsed().as_secs_f64() * 1000.0;
                if log {
                    crate::log::line(&format!(
                        "loaded {} chars: window {}..{} ({change:?}) in {ms:.1} ms",
                        s.doc.len_chars(),
                        w.range().start.0,
                        w.range().end.0
                    ));
                }
                shown.doc = Some(id);
                loaded = Some(ms);
            }
            shown.window = Some(w);
        }
    }
    // Edit mode: a multi-line edit that takes typing.
    let editing = app.is_editing();
    if editing != shown.editing {
        host.edit(DOC, |mut d| DocumentView::set_editing(&mut d, editing));
        shown.editing = editing;
    }
    // The font and size: the keys, the font list, or the settings dialog.
    let font = crate::fonts::doc_font(&app.settings().reading_aids.font);
    if shown.font.as_ref() != Some(&font) {
        host.edit(DOC, |mut d| DocumentView::set_font(&mut d, font.clone()));
        shown.font = Some(font);
    }
    // Text spacing and the ruler, drawn by the view.
    let aids = DocAids {
        spacing: (&app.settings().reading_aids.spacing).into(),
        ruler: app.ruler(),
    };
    if aids != shown.aids {
        host.edit(DOC, |mut d| DocumentView::set_aids(&mut d, aids));
        shown.aids = aids;
    }
    // RSVP: the panel under the document.
    let rsvp = RsvpShown::from_app(app);
    if rsvp != shown.rsvp {
        host.edit(RSVP, |mut r| RsvpView::set_shown(&mut r, rsvp.clone()));
        shown.rsvp = rsvp;
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
    if editing != shown.edit_button {
        host.edit(EDIT, |mut b| {
            ActionButton::set_label(&mut b, if editing { "Finish editing" } else { "Edit" });
        });
        shown.edit_button = editing;
    }
    let status = app.status_text().to_owned();
    if status != shown.status {
        host.edit(STATUS, |mut l| Label::set_text(&mut l, status.clone()));
        shown.status = status;
    }
    // The terminal's title line, from the app: the mode, the reading
    // state, "line 3 of 40, 7%", the access mode, the rate, the engine.
    let position = app.title_parts(app.title_position().as_deref()).join(", ");
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
    /// `--announce uia`: raises a UI Automation Notification event on the
    /// window for each message (the live region's nodes stay, not live).
    fn notify(&mut self, ctx: &mut DriverCtx<'_>, messages: &[Message]) {
        if self.hwnd == 0 {
            self.hwnd = window_hwnd(ctx.window(self.window_id).handle());
        }
        for m in messages {
            let result = crate::widgets::notify::raise(self.hwnd, m);
            if self.log {
                match result {
                    Ok(()) => crate::log::line(&format!("notify uia: {}", m.text)),
                    Err(e) => crate::log::line(&format!("notify uia failed ({e}): {}", m.text)),
                }
            }
        }
    }

    fn refresh(&mut self, ctx: &mut DriverCtx<'_>) {
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
            if self.announce == AnnounceMode::Uia {
                self.notify(ctx, &messages);
            }
            let root = ctx.render_root(self.window_id);
            root.edit_widget_with_tag(ANNOUNCER, |mut a| Announcer::say(&mut a, messages));
        }
        // Single-key shortcuts turned on or off: each button names its key.
        let char_keys = self.app.keymap().character_keys();
        if self.char_keys != Some(char_keys) {
            self.char_keys = Some(char_keys);
            let root = ctx.render_root(self.window_id);
            for (id, action) in &self.buttons.by_id {
                let shortcut = shortcut_for(&self.app, *action);
                root.edit_widget(*id, |mut w| {
                    let mut b = w.downcast::<ActionButton>();
                    ActionButton::set_shortcut(&mut b, shortcut);
                });
            }
        }
        // The theme changed (a key, the palette, or the settings).
        if !self.fixed_theme && self.app.current_theme().name() != self.palette.name {
            self.palette = Palette::from_theme(self.app.current_theme());
            let root = ctx.render_root(self.window_id);
            root.set_default_properties(Arc::new(theme::default_properties(&self.palette)));
            apply_palette(root, &self.palette);
            if self.log {
                crate::log::line(&format!("theme: {}", self.palette.name));
            }
        }
        let title = self.app.session().map_or_else(
            || "textweaver".to_owned(),
            |s| format!("{} - textweaver", s.title),
        );
        if title != self.window_title {
            ctx.window(self.window_id).handle().set_title(&title);
            // AT-SPI and macOS read the title from the window's node.
            ctx.render_root(self.window_id)
                .set_window_label(title.as_str());
            self.window_title = title;
        }
    }

    fn dispatch(&mut self, ctx: &mut DriverCtx<'_>, cmd: Command) {
        if self.log {
            crate::log::line(&format!("command {cmd:?}"));
        }
        if !self.settings_list && cmd == Command::Action(ActionId::Settings) {
            self.open_settings(ctx, None);
            return;
        }
        // The window's own commands: the text size and the font.
        let step = match &cmd {
            Command::Action(ActionId::TextLarger) => Some(Step::Larger),
            Command::Action(ActionId::TextSmaller) => Some(Step::Smaller),
            Command::Action(ActionId::TextSizeReset) => Some(Step::Reset),
            _ => None,
        };
        if let Some(step) = step {
            self.text_size(ctx, step);
            return;
        }
        if cmd == Command::Action(ActionId::ChooseFont) {
            self.open_fonts(ctx);
            return;
        }
        // Open Path (a key, or chosen in the palette) is the app's Open,
        // answered by typing.
        self.typed_open = match &cmd {
            Command::Action(a) => *a == ActionId::OpenPath,
            Command::Answer(id) => id == ActionId::OpenPath.id(),
            _ => false,
        };
        let effects = self.app.dispatch(cmd);
        self.run_effects(ctx, effects);
        self.typed_open = false;
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
                // Open: the system's file chooser, unless Open Path asked
                // for the typed prompt.
                Effect::Prompt {
                    label,
                    purpose: PromptPurpose::Open,
                } if !self.typed_open => self.open_file_chooser(ctx, &label),
                Effect::Prompt { label, purpose } => self.open_prompt(ctx, &label, purpose),
                // The open list changed (filtered, a setting changed): show
                // it in place, keeping focus in the dialog.
                Effect::ShowList { .. } if matches!(self.dialog, Some(OpenDialog::List)) => {
                    self.sync_list(ctx);
                }
                Effect::ShowList { title, items } => self.open_list(ctx, &title, items),
            }
        }
    }

    /// A prompt from the app: its text and history are the app's
    /// `PromptModel`, which the field keeps in step with `PromptKey`s.
    fn open_prompt(&mut self, ctx: &mut DriverCtx<'_>, label_text: &str, purpose: PromptPurpose) {
        let paths = matches!(
            purpose,
            PromptPurpose::Open | PromptPurpose::SaveAs | PromptPurpose::ImagePath
        );
        let hint = if paths {
            "Type the path of a document, then press Enter. Tab completes it; Up and Down recall earlier ones."
        } else {
            "Press Enter to accept, or Escape to cancel. Up and Down recall earlier answers."
        };
        let initial = self
            .app
            .prompt_model()
            .map(textweaver_app::PromptModel::text)
            .unwrap_or_default();
        self.show_prompt(ctx, label_text, hint, &initial, paths);
        self.dialog = Some(OpenDialog::Prompt);
        if self.log {
            crate::log::line(&format!("dialog: prompt {label_text:?}"));
        }
    }

    /// Open: the system's file chooser, on its own thread and modal to the
    /// window. Its answer comes back as a [`FileChosen`] action; until
    /// then the app's Open prompt waits, labelled `label_text`.
    fn open_file_chooser(&mut self, ctx: &mut DriverCtx<'_>, label_text: &str) {
        let c = self.app.catalog();
        let registry = textweaver_app::formats::Registry::with_builtins();
        let filters = file_chooser::filters(
            &registry.extensions(),
            &c.tr("gui-open-documents"),
            &c.tr("gui-open-all-files"),
        );
        let folder = file_chooser::start_folder(self.app.session().map(|s| s.key.0.as_str()));
        let chooser = file_chooser::Chooser::new(
            ctx.window(self.window_id).handle(),
            &c.tr("gui-open-title"),
            &filters,
            folder.as_deref(),
        );
        let proxy = self.proxy.clone();
        let window_id = self.window_id;
        chooser.show(move |chosen| {
            let _ = proxy.send_event(MasonryUserEvent::AsyncAction(window_id, Box::new(chosen)));
        });
        self.dialog = Some(OpenDialog::FileChooser(label_text.to_owned()));
        if self.log {
            crate::log::line("dialog: system file chooser");
        }
    }

    /// The file chooser's answer: the file opens through the app's Open
    /// prompt (so it joins the prompt's history), a cancel cancels it, and
    /// a chooser that never appeared gives way to the typed prompt. The
    /// focus goes back to the document either way.
    fn file_chosen(&mut self, ctx: &mut DriverCtx<'_>, chosen: &FileChosen) {
        let Some(OpenDialog::FileChooser(label_text)) = self.dialog.take() else {
            return;
        };
        let outcome = file_chooser::outcome(chosen.path.clone(), chosen.elapsed);
        if self.log {
            crate::log::line(&format!(
                "file chooser: {outcome:?} after {} ms",
                chosen.elapsed.as_millis()
            ));
        }
        self.close_dialog(ctx);
        let key = match outcome {
            file_chooser::Outcome::Chosen(path) => {
                let text = path.to_string_lossy().into_owned();
                let _ = self
                    .app
                    .dispatch(Command::PromptKey(PromptKey::SetText(text)));
                PromptKey::Enter
            }
            file_chooser::Outcome::Cancelled => PromptKey::Escape,
            file_chooser::Outcome::Failed => {
                let said = self.app.catalog().tr("gui-open-no-dialog");
                self.app.announce(&said, Priority::Assertive);
                self.open_prompt(ctx, &label_text, PromptPurpose::Open);
                self.refresh(ctx);
                return;
            }
        };
        let effects = self.app.dispatch(Command::PromptKey(key));
        self.run_effects(ctx, effects);
        self.refresh(ctx);
    }

    /// Shows a one-field prompt labelled `label_text`, starting with
    /// `initial`, and focuses its field.
    fn show_prompt(
        &mut self,
        ctx: &mut DriverCtx<'_>,
        label_text: &str,
        hint: &str,
        initial: &str,
        tab_completes: bool,
    ) {
        let p = &self.palette;
        let field = NewWidget::new(
            TextArea::new_editable(initial)
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
        let modal = NewWidget::new(
            Modal::new(card, label_text, p.clone()).with_tab_completion(tab_completes),
        )
        .erased();
        let root = ctx.render_root(self.window_id);
        root.edit_widget_with_tag(ROOT, |mut r| Root::set_dialog(&mut r, Some(modal)));
        root.focus_on(Some(field_id));
    }

    /// A key for the app's prompt: the app changes its text (history,
    /// completion), and the field shows it.
    fn prompt_key(&mut self, ctx: &mut DriverCtx<'_>, key: PromptKey) {
        if self.log {
            crate::log::line(&format!("prompt key {key:?}"));
        }
        let effects = self.app.dispatch(Command::PromptKey(key));
        if let Some(text) = self
            .app
            .prompt_model()
            .map(textweaver_app::PromptModel::text)
        {
            ctx.render_root(self.window_id)
                .edit_widget_with_tag(PROMPT_FIELD, |mut f| TextArea::reset_text(&mut f, &text));
        }
        self.run_effects(ctx, effects);
        self.refresh(ctx);
    }

    /// The Settings command: the settings dialog, from the app's schema.
    fn open_settings(&mut self, ctx: &mut DriverCtx<'_>, at: Option<(usize, usize)>) {
        let form = SettingsForm::new(self.app.settings_schema());
        let (section, row) = at.unwrap_or((0, 0));
        let section = section.min(form.sections.len().saturating_sub(1));
        let d = settings_dialog(&self.palette, &form, &self.app, section, row);
        let root = ctx.render_root(self.window_id);
        root.edit_widget_with_tag(ROOT, |mut r| Root::set_dialog(&mut r, Some(d.modal)));
        root.focus_on(Some(d.form));
        if self.log {
            crate::log::line(&format!(
                "dialog: settings, {} sections, section {section}, row {row}",
                form.sections.len()
            ));
        }
        self.dialog = Some(OpenDialog::Settings(SettingsOpen {
            form,
            section,
            close: d.close,
        }));
    }

    /// A change, an edit, or a section move from the settings form.
    fn settings_form_action(&mut self, ctx: &mut DriverCtx<'_>, a: FormAction) {
        let Some(OpenDialog::Settings(open)) = &self.dialog else {
            return;
        };
        let section = open.section;
        match a {
            FormAction::Section(delta) => {
                let n = open.form.sections.len() as isize;
                if n == 0 {
                    return;
                }
                let next = (section as isize + delta).rem_euclid(n) as usize;
                self.show_section(ctx, next, true);
            }
            FormAction::Edit { row, text } => {
                let Some(setting) = open.form.setting(section, row).cloned() else {
                    return;
                };
                if matches!(setting.kind, textweaver_app::SettingKind::Table) {
                    self.app.announce(
                        &format!("{} is a table. Edit it in settings.toml.", setting.label),
                        Priority::Polite,
                    );
                    self.refresh(ctx);
                    return;
                }
                let now = self.app.setting_value(&setting.path).unwrap_or_default();
                let initial = if text.is_empty() {
                    setting.edit_text(&now)
                } else {
                    text
                };
                let label_text = format!("New value for {}", setting.label);
                let hint = if setting.help.is_empty() {
                    "Press Enter to accept, or Escape to go back."
                } else {
                    setting.help
                };
                let Some(OpenDialog::Settings(open)) = self.dialog.take() else {
                    return;
                };
                self.show_prompt(ctx, &label_text, hint, &initial, false);
                self.dialog = Some(OpenDialog::SettingEdit(open, row));
                if self.log {
                    crate::log::line(&format!("dialog: {label_text}"));
                }
            }
            FormAction::Change { row, change } => {
                let Some(setting) = open.form.setting(section, row).cloned() else {
                    return;
                };
                self.change_setting(ctx, &setting, row, change);
            }
        }
    }

    /// Changes a setting through the app and shows its new value; says only
    /// what the form does not show.
    fn change_setting(
        &mut self,
        ctx: &mut DriverCtx<'_>,
        setting: &textweaver_app::Setting,
        row: usize,
        change: FormChange,
    ) {
        if self.log {
            crate::log::line(&format!("setting {} {change:?}", setting.path));
        }
        let theme_before = self.palette.name.clone();
        match settings_dialog::apply(&mut self.app, setting, change) {
            Ok(said) => {
                if self.log {
                    crate::log::line(&format!("setting: {said}"));
                }
                self.refresh(ctx);
                if self.palette.name != theme_before {
                    // A new theme: draw the dialog again in its colours.
                    let section = match &self.dialog {
                        Some(OpenDialog::Settings(o)) => o.section,
                        _ => 0,
                    };
                    self.open_settings(ctx, Some((section, row)));
                }
                let Some(OpenDialog::Settings(open)) = &self.dialog else {
                    return;
                };
                let rows = open.form.rows(open.section, &self.app);
                let note = rows
                    .get(row)
                    .and_then(|r| settings_dialog::extra_note(&said, r));
                ctx.render_root(self.window_id)
                    .edit_widget_with_tag(FORM, |mut g| SettingsGrid::update_rows(&mut g, rows));
                if let Some(note) = note {
                    self.app.announce(&note, Priority::Polite);
                }
            }
            Err(why) => self.app.announce(&why, Priority::Assertive),
        }
        self.refresh(ctx);
    }

    /// Shows section `section` in the form; `say` announces it (a move
    /// made from the form, where the section list is not heard).
    fn show_section(&mut self, ctx: &mut DriverCtx<'_>, section: usize, say: bool) {
        let Some(OpenDialog::Settings(open)) = &mut self.dialog else {
            return;
        };
        if open.section == section && !say {
            return;
        }
        open.section = section;
        let title = open
            .form
            .sections
            .get(section)
            .copied()
            .unwrap_or("Settings");
        let rows = open.form.rows(section, &self.app);
        let item = open.form.section_items().get(section).cloned();
        let root = ctx.render_root(self.window_id);
        root.edit_widget_with_tag(FORM, |mut g| {
            SettingsGrid::set_section(&mut g, title, rows, 0);
        });
        root.edit_widget_with_tag(SECTIONS, |mut l| ChoiceList::select(&mut l, section));
        if say && let Some(item) = item {
            self.app.announce(&format!("{item}."), Priority::Polite);
            self.refresh(ctx);
        }
    }

    /// The settings dialog's own actions: the section list moved or chose,
    /// the dialog was closed. Returns false when the settings dialog is not
    /// open.
    fn settings_dialog_action(&mut self, ctx: &mut DriverCtx<'_>, d: &DialogAction) -> bool {
        if !matches!(self.dialog, Some(OpenDialog::Settings(_))) {
            return false;
        }
        match d {
            DialogAction::Focus(i) => self.show_section(ctx, *i, false),
            DialogAction::Choose(i) => {
                self.show_section(ctx, *i, false);
                let root = ctx.render_root(self.window_id);
                let form = root.get_widget_with_tag(FORM).map(|w| w.id());
                root.focus_on(form);
            }
            DialogAction::Cancel => {
                self.close_dialog(ctx);
                self.app.announce("Settings closed.", Priority::Polite);
                self.refresh(ctx);
            }
            _ => {}
        }
        true
    }

    /// The answer to a setting's prompt: the new value, then the settings
    /// dialog again on the same setting.
    fn setting_edit_answer(&mut self, ctx: &mut DriverCtx<'_>, answer: Option<String>) {
        let Some(OpenDialog::SettingEdit(open, row)) = self.dialog.take() else {
            return;
        };
        let section = open.section;
        let setting = open.form.setting(section, row).cloned();
        self.open_settings(ctx, Some((section, row)));
        if let (Some(text), Some(setting)) = (answer, setting) {
            self.change_setting(ctx, &setting, row, FormChange::Text(text));
        }
    }

    fn open_list(&mut self, ctx: &mut DriverCtx<'_>, title: &str, items: Vec<String>) {
        let count = items.len();
        let selected = self.app.list_model().map_or(0, |m| m.selected);
        let (modal, list_id) = list_dialog(&self.palette, title, items, selected, true);
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

    /// The font list (Ctrl+D, the Font button): the families, bundled
    /// first, starting on the current one.
    fn open_fonts(&mut self, ctx: &mut DriverCtx<'_>) {
        let choices = crate::font_chooser::choices(self.installed.names());
        let current = crate::fonts::doc_font(&self.app.settings().reading_aids.font);
        let items: Vec<String> = choices.iter().map(|c| c.label.clone()).collect();
        let selected = choices
            .iter()
            .position(|c| current.family.starts_with(&format!("\"{}\"", c.family)))
            .unwrap_or(0);
        let title = self.app.catalog().tr("gui-font-list");
        let (modal, list_id) = list_dialog(&self.palette, &title, items, selected, false);
        let root = ctx.render_root(self.window_id);
        root.edit_widget_with_tag(ROOT, |mut r| Root::set_dialog(&mut r, Some(modal)));
        root.focus_on(Some(list_id));
        self.dialog = Some(OpenDialog::FontFamily(choices));
        if self.log {
            crate::log::line("dialog: font list");
        }
    }

    /// The font list's answer: the family applies at once, keeping the
    /// size, and is said ("Font: OpenDyslexic."). Returns false when the
    /// open dialog is not the font list.
    fn font_answer(&mut self, ctx: &mut DriverCtx<'_>, d: &DialogAction) -> bool {
        match (&self.dialog, d) {
            (Some(OpenDialog::FontFamily(_)), DialogAction::Cancel) => {
                self.close_dialog(ctx);
                let said = self.app.catalog().tr("gui-font-unchanged");
                self.app.announce(&said, Priority::Polite);
                self.refresh(ctx);
                true
            }
            (Some(OpenDialog::FontFamily(choices)), DialogAction::Choose(i)) => {
                let Some(family) = choices.get(*i).map(|c| c.family.clone()) else {
                    return true;
                };
                self.close_dialog(ctx);
                let new = crate::font_chooser::with_family(
                    &self.app.settings().reading_aids.font,
                    &family,
                );
                self.save_font(new);
                let said = crate::font_chooser::font_message(&self.app.catalog(), &family);
                self.app.announce(&said, Priority::Polite);
                self.refresh(ctx);
                true
            }
            _ => false,
        }
    }

    /// Ctrl+Plus, Ctrl+Minus, Ctrl+0: the next text size, saved and said
    /// ("Text size 18 points."). The next refresh lays the text out again.
    fn text_size(&mut self, ctx: &mut DriverCtx<'_>, step: Step) {
        let now = self.app.settings().reading_aids.font.clone();
        let (size, limit) = crate::font_chooser::stepped(now.size_pt, step);
        self.save_font(crate::font_chooser::with_size(&now, size));
        if self.log {
            crate::log::line(&format!("text size {step:?}: {size} points"));
        }
        let said = crate::font_chooser::size_message(&self.app.catalog(), size, limit);
        // Assertive: a held key says only the latest size.
        self.app.announce(&said, Priority::Assertive);
        self.refresh(ctx);
    }

    /// Saves the document font in `[reading_aids.font]` (the writer thread
    /// saves it; a failure is said on a later tick).
    fn save_font(&mut self, new: textweaver_app::store::reading_aids::FontSettings) {
        let _ = self.app.update_settings(|s| s.reading_aids.font = new);
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

    /// A key in an app list: the app moves its focus, filters, or chooses;
    /// then the dialog shows the list as the app has it, or closes.
    fn list_key(&mut self, ctx: &mut DriverCtx<'_>, k: textweaver_app::ListKey) {
        if self.log {
            crate::log::line(&format!("list key {k:?}"));
        }
        let effects = self.app.dispatch(Command::ListKey(k));
        self.sync_list(ctx);
        self.run_effects(ctx, effects);
        self.refresh(ctx);
    }

    /// A chord in an app list: the Help and Say Status keys repeat the
    /// list's introduction, and the Repeat Message key says the last
    /// message, as in the terminal reader. Other chords do nothing there.
    fn list_chord(&mut self, ctx: &mut DriverCtx<'_>, chord: textweaver_app::keymap::KeyChord) {
        let action = self
            .app
            .keymap()
            .lookup(&chord, textweaver_app::keymap::Layer::Global);
        if self.log {
            crate::log::line(&format!("list chord {chord} -> {action:?}"));
        }
        match action {
            Some(ActionId::Help | ActionId::SayStatus) => {
                self.list_key(ctx, textweaver_app::ListKey::Introduce);
            }
            Some(ActionId::RepeatMessage) => {
                self.dispatch(ctx, Command::Action(ActionId::RepeatMessage));
            }
            _ => {}
        }
    }

    /// Shows the app's list model in the open list dialog, or closes the
    /// dialog when the app's list is gone.
    fn sync_list(&mut self, ctx: &mut DriverCtx<'_>) {
        if !matches!(self.dialog, Some(OpenDialog::List)) {
            return;
        }
        match self.app.list_model() {
            Some(m) => {
                let (items, selected) = (m.items.clone(), m.selected);
                ctx.render_root(self.window_id)
                    .edit_widget_with_tag(LIST, |mut l| ChoiceList::sync(&mut l, &items, selected));
            }
            None => self.close_dialog(ctx),
        }
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
        // Commands the window runs itself, chosen in the command palette:
        // Settings opens the dialog; the text size and font keys.
        let palette = matches!(self.dialog, Some(OpenDialog::Palette(_)));
        self.close_dialog(ctx);
        let own = match &cmd {
            Command::Answer(id) if palette => ActionId::from_id(id).filter(|a| {
                (*a == ActionId::Settings && !self.settings_list)
                    || (a.is_window_only() && *a != ActionId::OpenPath)
            }),
            _ => None,
        };
        if let Some(a) = own {
            self.muted.set(true);
            let effects = self.app.dispatch(Command::Cancel);
            self.muted.set(false);
            self.run_effects(ctx, effects);
            self.dispatch(ctx, Command::Action(a));
            self.refresh(ctx);
            return;
        }
        self.dispatch(ctx, cmd);
    }

    /// The prompt's answer (Enter) or cancel (Escape), through the app's
    /// prompt model so the answer joins the prompt's history.
    fn prompt_answer(&mut self, ctx: &mut DriverCtx<'_>, text: Option<String>) {
        if !matches!(self.dialog, Some(OpenDialog::Prompt)) {
            return;
        }
        self.close_dialog(ctx);
        if self.app.prompt_model().is_none() {
            // The app has no prompt open (it was closed under us): answer
            // directly.
            let cmd = text.map_or(Command::Cancel, Command::Answer);
            self.dispatch(ctx, cmd);
            return;
        }
        let key = match text {
            Some(t) => {
                let _ = self.app.dispatch(Command::PromptKey(PromptKey::SetText(t)));
                PromptKey::Enter
            }
            None => PromptKey::Escape,
        };
        if self.log {
            crate::log::line(&format!("prompt key {key:?}"));
        }
        let effects = self.app.dispatch(Command::PromptKey(key));
        self.run_effects(ctx, effects);
        self.refresh(ctx);
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
            None => {
                // The key from the keymap, written for the screen reader
                // ("Ctrl+O") and spoken for textweaver's voice.
                let open = textweaver_app::named_key(self.app.keymap(), ActionId::Open);
                self.app.announce(
                    &format!("No document is open. Press {open} to open one."),
                    Priority::Polite,
                );
            }
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
            // In edit mode, a selection the app holds (from a command) is
            // let go when the view moves the caret, so typing goes to the
            // caret; the view keeps its own selection and sends it with
            // each edit.
            let app_selection =
                self.app.is_editing() && self.app.session().is_some_and(|s| s.selection.is_some());
            if app_cursor != Some(caret) || app_selection {
                if self.log {
                    crate::log::line(&format!("caret sync: {caret:?}"));
                }
                self.muted.set(true);
                if app_selection {
                    let _ = self
                        .app
                        .dispatch(Command::Select(CharRange::new(caret.0, caret.0)));
                }
                let effects = self.app.dispatch(Command::SetCursor(caret));
                self.muted.set(false);
                self.run_effects(ctx, effects);
                // The view already shows the caret; keep the window in step.
                self.shown.state.caret = caret;
                self.refresh(ctx);
            }
        } else if let Some(edit) = action.downcast_ref::<DocAction>() {
            // Edit mode: typing and deleting go through the app, which
            // keeps the undo history and echoes as the access mode says.
            let cmd = match edit.clone() {
                DocAction::Typed(text) => Command::Insert(text),
                DocAction::Delete { forward: true } => Command::DeleteForward,
                DocAction::Delete { forward: false } => Command::DeleteBack,
                DocAction::Replace { range, text } => Command::ReplaceRange { range, text },
                DocAction::CaretMoved { .. } => return,
            };
            if self.log {
                crate::log::line(&format!("edit: {cmd:?}"));
            }
            let effects = self.app.dispatch(cmd);
            self.run_effects(ctx, effects);
            self.refresh(ctx);
        } else if action.downcast_ref::<Pressed>().is_some() {
            if let Some(OpenDialog::Settings(open)) = &self.dialog
                && open.close == widget_id
            {
                self.settings_dialog_action(ctx, &DialogAction::Cancel);
            } else if let Some(a) = self.buttons.by_id.get(&widget_id).copied() {
                self.dispatch(ctx, Command::Action(a));
            }
        } else if let Some(a) = action.downcast_ref::<FormAction>() {
            let a = a.clone();
            self.settings_form_action(ctx, a);
        } else if let Some(d) = action.downcast_ref::<DialogAction>() {
            if self.settings_dialog_action(ctx, d) || self.font_answer(ctx, d) {
                return;
            }
            match d {
                DialogAction::Key(k) => {
                    let k = *k;
                    self.list_key(ctx, k);
                    return;
                }
                DialogAction::Focus(i) => {
                    let i = *i;
                    self.dispatch(ctx, Command::ListFocus(i));
                    return;
                }
                DialogAction::Recall(up) => {
                    let up = *up;
                    match &self.dialog {
                        Some(OpenDialog::Prompt) => {
                            let key = if up { PromptKey::Up } else { PromptKey::Down };
                            self.prompt_key(ctx, key);
                        }
                        // In the palette's filter, Up and Down move through
                        // the matches, and say each.
                        Some(OpenDialog::Palette(ids)) if !ids.is_empty() => {
                            let n = ids.len();
                            let root = ctx.render_root(self.window_id);
                            let now = root
                                .get_widget_with_tag(LIST)
                                .map_or(0, |w| w.inner().selected());
                            let next = if up {
                                now.checked_sub(1).unwrap_or(n - 1)
                            } else {
                                (now + 1) % n
                            };
                            root.edit_widget_with_tag(LIST, |mut l| {
                                ChoiceList::select(&mut l, next)
                            });
                            let said = self
                                .app
                                .palette_candidates("")
                                .into_iter()
                                .find(|(a, _)| Some(a) == ids.get(next))
                                .map(|(_, d)| d);
                            if let Some(said) = said {
                                self.app.announce(&said, Priority::Assertive);
                            }
                            self.refresh(ctx);
                        }
                        _ => {}
                    }
                    return;
                }
                DialogAction::Complete => {
                    if matches!(self.dialog, Some(OpenDialog::Prompt)) {
                        self.prompt_key(ctx, PromptKey::Tab);
                    }
                    return;
                }
                DialogAction::Chord(c) => {
                    let c = *c;
                    self.list_chord(ctx, c);
                    return;
                }
                DialogAction::Cancel
                    if matches!(self.dialog, Some(OpenDialog::SettingEdit(..))) =>
                {
                    self.setting_edit_answer(ctx, None);
                    return;
                }
                DialogAction::Cancel if matches!(self.dialog, Some(OpenDialog::Prompt)) => {
                    self.prompt_answer(ctx, None);
                    return;
                }
                _ => {}
            }
            let cmd = match (d, &self.dialog) {
                (DialogAction::Choose(i), Some(OpenDialog::Palette(ids))) => match ids.get(*i) {
                    Some(a) => Command::Answer(a.id().to_owned()),
                    None => Command::Cancel,
                },
                (DialogAction::Choose(i), _) => Command::Choose(*i),
                (DialogAction::Cancel, _) => Command::Cancel,
                // Handled above.
                (
                    DialogAction::Key(_)
                    | DialogAction::Focus(_)
                    | DialogAction::Recall(_)
                    | DialogAction::Complete
                    | DialogAction::Chord(_),
                    _,
                ) => return,
            };
            self.answer(ctx, cmd);
        } else if let Some(t) = action.downcast_ref::<masonry::widgets::TextAction>() {
            use masonry::widgets::TextAction;
            match (t, &self.dialog) {
                (TextAction::Entered(text), Some(OpenDialog::SettingEdit(..))) => {
                    let text = text.clone();
                    self.setting_edit_answer(ctx, Some(text));
                }
                (TextAction::Cancelled, Some(OpenDialog::SettingEdit(..))) => {
                    self.setting_edit_answer(ctx, None);
                }
                (TextAction::Entered(text), Some(OpenDialog::Prompt)) => {
                    let text = text.clone();
                    self.prompt_answer(ctx, Some(text));
                }
                (TextAction::Cancelled, Some(OpenDialog::Prompt)) => self.prompt_answer(ctx, None),
                (TextAction::Changed(q), Some(OpenDialog::Prompt)) => {
                    // The app's prompt model follows the field (no echo:
                    // the field says what was typed).
                    let q = q.clone();
                    let _ = self.app.dispatch(Command::PromptKey(PromptKey::SetText(q)));
                }
                (TextAction::Entered(text), _) => {
                    // In the palette, Enter runs the first match.
                    let answer = match &self.dialog {
                        Some(OpenDialog::Palette(ids)) => ids
                            .first()
                            .map_or_else(|| text.clone(), |a| a.id().to_owned()),
                        _ => text.clone(),
                    };
                    self.answer(ctx, Command::Answer(answer));
                }
                (TextAction::Cancelled, _) => self.answer(ctx, Command::Cancel),
                (TextAction::Changed(q), Some(OpenDialog::Palette(_))) => {
                    let q = q.clone();
                    self.filter_palette(ctx, &q);
                }
                (TextAction::Changed(_), _) => {}
            }
        }
    }

    fn on_async_action(
        &mut self,
        _window_id: WindowId,
        ctx: &mut DriverCtx<'_>,
        action: ErasedAction,
    ) {
        if self.closed {
            return;
        }
        if let Some(chosen) = action.downcast_ref::<FileChosen>() {
            self.file_chosen(ctx, chosen);
            return;
        }
        if action.downcast_ref::<Tick>().is_none() {
            return;
        }
        self.wake_pending.store(false, Ordering::Release);
        if !self.started {
            self.started = true;
            if self.log {
                crate::log::line("first tick");
            }
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
        let now = Instant::now();
        let mut effects = self.app.poll_speech();
        effects.extend(self.app.tick(now));
        self.run_effects(ctx, effects);
        self.refresh(ctx);
        let wait = self.app.tick_interval(Instant::now());
        self.tick_ms.store(
            u64::try_from(wait.as_millis()).unwrap_or(u64::MAX).max(10),
            Ordering::Relaxed,
        );
    }

    fn on_start(&mut self, state: &mut MasonryState) {
        if self.log {
            crate::log::line("window created; starting");
        }
        if let Some(proxy) = self.ticker.take() {
            // The app rings from other threads; post one tick per burst.
            let pending = Arc::clone(&self.wake_pending);
            let window_id = self.window_id;
            let waker_proxy = std::sync::Mutex::new(proxy.clone());
            self.app.set_waker(Some(Arc::new(move || {
                if !pending.swap(true, Ordering::AcqRel)
                    && let Ok(p) = waker_proxy.lock()
                {
                    let _ = p.send_event(MasonryUserEvent::AsyncAction(window_id, Box::new(Tick)));
                }
            })));
            spawn_ticker(proxy, self.window_id, Arc::clone(&self.tick_ms));
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
    let (mut app, mut messages) = setup::build_app(&opts.app, Box::new(announcer));
    app.set_announce_list_focus(opts.experiments.app_list_announcements);
    let mut experiments = opts.experiments;
    let wanted = experiments
        .announce
        .unwrap_or_else(|| setup::announce_setting(app.settings()));
    let announce = wanted.effective();
    if announce != wanted {
        messages.push(
            "UI Automation notifications exist only on Windows; using the live region.".into(),
        );
    }
    experiments.announce = Some(announce);
    if opts.log {
        crate::log::line(&format!("announce: {}", announce.name()));
    }
    if opts.theme.is_none() {
        // The system's light, dark, or high-contrast setting, when the
        // settings ask to follow it (`display.follow_os_theme`).
        let _ = app.apply_startup_theme(textweaver_app::theme::os::probe());
    }
    let palette = match &opts.theme {
        Some(name) => Palette::named(name),
        None => Palette::from_theme(app.current_theme()),
    };
    let font = crate::fonts::doc_font(&app.settings().reading_aids.font);
    let full_passes = Rc::new(Cell::new(0));
    let tree = build_tree(&palette, font, Some(&app), full_passes, experiments);

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
        wake_pending: Arc::new(AtomicBool::new(false)),
        tick_ms: Arc::new(AtomicU64::new(
            u64::try_from(FIRST_TICK.as_millis()).unwrap_or(250),
        )),
        ticker: Some(proxy.clone()),
        proxy,
        typed_open: false,
        char_keys: None,
        fixed_theme: opts.theme.is_some(),
        settings_list: experiments.settings_list,
        announce,
        hwnd: 0,
        installed: crate::font_chooser::Installed::scan_in_background(),
        window_title: String::new(),
        closed: false,
        timings: Timings::default(),
    };
    let default_props = theme::default_properties(&gui.palette);
    masonry_winit::app::run_with(event_loop, vec![window], gui, default_props)
        .map_err(|e| format!("the event loop failed: {e}"))
}

/// The window's Win32 handle, or 0 (another system, or no handle yet).
fn window_hwnd(window: &WinitWindow) -> isize {
    use masonry_winit::winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match window.window_handle().map(|h| h.as_raw()) {
        Ok(RawWindowHandle::Win32(h)) => h.hwnd.get(),
        _ => 0,
    }
}

/// Ticks for the app's own timers (autosave, the position save, RSVP),
/// sleeping as long as the app says it may; speech and background work
/// ring the waker instead.
fn spawn_ticker(proxy: EventLoopProxy, window_id: WindowId, tick_ms: Arc<AtomicU64>) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(tick_ms.load(Ordering::Relaxed)));
            if proxy
                .send_event(MasonryUserEvent::AsyncAction(window_id, Box::new(Tick)))
                .is_err()
            {
                return;
            }
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

/// Keeps a test harness in step with an app across refreshes, as the
/// window does: the document window slides or recentres, and only what
/// changed is sent.
#[derive(Default)]
pub struct Refresher {
    shown: Shown,
}

impl Refresher {
    /// Brings `host` up to date with `app`. Returns the document window's
    /// range when the view's text was replaced or slid.
    pub fn refresh(&mut self, app: &App, host: &mut impl Host) -> Option<CharRange> {
        refresh_host(app, &mut self.shown, host, false)?;
        self.shown.window.map(|w| w.range())
    }
}
