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
use textweaver_app::a11y::{Announcer as AppAnnouncer, Importance, Priority};
use textweaver_app::core::CharRange;
use textweaver_app::keymap::{ActionId, Platform};
use textweaver_app::lexicon::args;
use textweaver_app::lexicon::i18n::Catalog;
use textweaver_app::store::DocKey;
use textweaver_app::{
    App, AppError, Command, DocWindow, Effect, Playback, PromptKey, PromptPurpose, WindowChange,
    extra_command,
};

use crate::dialog::{self, ChoiceList, DialogAction, Modal};
use crate::document::{CaretEcho, DocAction, DocAids, DocFont, DocModel, DocState, DocumentView};
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
/// How often the window looks again while startup messages wait for a
/// screen reader to ask for the tree.
const HOLD_TICK: Duration = Duration::from_millis(200);
/// How often the window looks again while a menu bar a key showed waits
/// to hide (`gui.auto_hide_menu`).
const MENU_TICK: Duration = Duration::from_millis(150);
/// Misspelled words are marked once typing pauses this long.
const SPELL_PAUSE: Duration = Duration::from_millis(500);
/// Misspelled words are marked in documents up to this many chars (about
/// 50 ms to check on the development machine; 10 million took 0.6 s).
const SPELL_LIMIT: usize = 1_000_000;

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
    /// The menus as a list in the window (F10), as on Linux, instead of the
    /// system's menu bar (`--list-menus`).
    pub list_menus: bool,
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
    /// The notes, bookmarks, highlights, and search matches drawn.
    marks: Vec<(CharRange, crate::document::DocMark)>,
    /// The caret keys the view leaves to the keymap in the app's mode.
    yielded: Option<Vec<textweaver_app::keymap::KeyChord>>,
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
    /// The Colors dialog's Reset all colors button.
    reset: Option<WidgetId>,
}

/// An open dialog.
enum OpenDialog {
    Prompt,
    /// The settings dialog.
    Settings(SettingsOpen),
    /// A prompt for a new value of the setting at this row of the settings
    /// dialog, which comes back when it closes.
    SettingEdit(SettingsOpen, usize),
    /// A list from the app, with the title it was shown with (a menu
    /// list shows a new title as a submenu opens).
    List(String),
    /// The command palette: the actions its list shows, in order.
    Palette(Vec<ActionId>),
    /// The system's file chooser, open on its own thread; the app's Open
    /// prompt (labelled with this) waits for its answer.
    FileChooser {
        /// The waiting prompt's label.
        label: String,
        /// What the prompt is for (every prompt for a path, W8a-f).
        purpose: PromptPurpose,
    },
    /// The system's folder chooser, open on its own thread, for the
    /// app's file browser choosing a folder (audio export, batch
    /// conversion, sync; W8a-f); the browser waits behind it.
    FolderChooser,
    /// The voice manager (W7v): the app's voice list, under this title,
    /// with its filter and action buttons.
    Voices {
        /// The title it was shown with.
        title: String,
        /// Each button's id and what it does.
        buttons: Vec<(WidgetId, crate::voices::VoiceButton)>,
    },
    /// A yes-or-no question from the app, with its two buttons.
    Question {
        /// The Yes button.
        yes: WidgetId,
        /// The No button.
        no: WidgetId,
    },
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
    /// No settings, keys, or state existed under the state folder when the
    /// window started: the welcome and the language list come first.
    first_run: bool,
    /// The startup questions (hybrid mode with a screen reader running)
    /// may be asked: not in automated runs (`--background`).
    startup_offers: bool,
    /// `--background`: on Windows the window is kept from ever taking the
    /// foreground ([`crate::background`]).
    background: bool,
    exit_at: Option<Instant>,
    /// The app's waker: speech statuses and finished background work post
    /// a tick at once (ADR-0024). Set when a tick is posted and not yet
    /// handled, so a burst of rings posts one.
    wake_pending: Arc<AtomicBool>,
    /// How long the ticker sleeps when nothing rings (`App::tick_interval`).
    tick_ms: Arc<AtomicU64>,
    /// Installed font families, for the font chooser.
    installed: crate::font_chooser::Installed,
    /// The theme and the highlight colors the palette was made from
    /// (`App::reading_theme_key`).
    theme_key: (String, String, Option<String>, Vec<String>),
    /// The system's high contrast colors the palette was made from, when
    /// the window follows them.
    system: Option<crate::system_colors::SystemColors>,
    /// `--theme` was given: the saved theme is not followed.
    fixed_theme: bool,
    /// Settings opens the app's list instead of the dialog.
    settings_list: bool,
    /// The menus are the in-window list, not the system's menu bar.
    list_menus: bool,
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
    /// Misspelled words marked for this document and text revision, and
    /// when a new revision was first seen (the marks wait for a pause in
    /// typing).
    spell_marked: Option<(DocKey, u64)>,
    spell_seen: Option<((DocKey, u64), Instant)>,
    /// The interface language the drawn labels were last written in
    /// (`[interface] language` changes them live, as in the terminal).
    lang: String,
    closed: bool,
    /// Where the focus was when the first dialog opened, to go back to.
    return_focus: Option<WidgetId>,
    /// How announcements reach the screen reader.
    announce: AnnounceMode,
    /// UI Automation notifications waiting for a screen reader to ask for
    /// the tree (`--announce uia`).
    held_notices: Vec<Message>,
    /// The window's Win32 handle, for UI Automation notifications (read
    /// on first use; 0 until then or when there is none).
    hwnd: isize,
    /// The native menus (Windows and macOS), once attached; `None` where
    /// the menu key opens the list menu instead (ADR-0046).
    native: Option<crate::menus::Native>,
    /// A command ran since the menus were last compared with the model:
    /// the next refresh builds the model's tree and updates them if it
    /// changed (a toggle, the language, the keys, a recent document).
    menu_dirty: bool,
    /// What the native menus were built from ([`crate::menus::Fingerprint`]).
    menu_key: Option<crate::menus::Fingerprint>,
    /// How the title bar and menus are drawn now (W8a-m).
    chrome: crate::dark_mode::Chrome,
    /// The font list's families while it is shown (the list itself is the
    /// app's; `App::take_frontend_choice` says which was chosen).
    font_choices: Option<Vec<crate::font_chooser::Choice>>,
    /// How many font downloads the app had finished when the downloaded
    /// fonts were last registered (`App::font_downloads`).
    font_downloads: u64,
    /// Reset all colors was asked from the Colors dialog on this section:
    /// the dialog opens again there once the question is answered.
    colors_after_question: Option<usize>,
    /// Load and highlight timings, for `--log` and the measurements.
    pub timings: Timings,
}

/// Measured times, in milliseconds.
#[derive(Clone, Debug, Default)]
pub struct Timings {
    /// Building the window's text and runs for a document.
    pub load_ms: Vec<(usize, f64)>,
    /// Moving the highlight: the driver's refresh for each move of the
    /// spoken word (the widget passes that follow are timed by
    /// `--measure-frames`). Kept for [`HIGHLIGHT_SUMMARY_MOVES`] moves,
    /// then summarized under `--log` and cleared.
    pub highlight_ms: Vec<f64>,
}

/// Highlight moves per `--log` summary line.
pub const HIGHLIGHT_SUMMARY_MOVES: usize = 200;

impl Timings {
    /// Records one highlight move; every [`HIGHLIGHT_SUMMARY_MOVES`] moves
    /// returns a summary line (median, 95th percentile, worst) and starts
    /// again, so the record never grows.
    pub fn highlight_moved(&mut self, ms: f64) -> Option<String> {
        self.highlight_ms.push(ms);
        if self.highlight_ms.len() < HIGHLIGHT_SUMMARY_MOVES {
            return None;
        }
        let mut v = std::mem::take(&mut self.highlight_ms);
        v.sort_by(f64::total_cmp);
        let at = |q: f64| v[((v.len() - 1) as f64 * q).round() as usize];
        Some(format!(
            "highlight: {} moves, driver refresh median {:.3} ms, 95th percentile {:.3} ms, worst {:.3} ms",
            v.len(),
            at(0.5),
            at(0.95),
            at(1.0)
        ))
    }
}

/// The pieces the tree is built from, shared by the window and the
/// screenshot harness.
pub struct Tree {
    /// The root widget.
    pub root: NewWidget<Root>,
    buttons: Buttons,
}

pub(crate) fn label(text: &str, size: f32, bold: bool) -> Label {
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
    let help = button_description(&catalog_of(app), action);
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

/// The drawn label of the button for `action`, in the catalog's language
/// (W4d's six languages): "Open…", or "Pause" while reading, or "Finish
/// editing" in edit mode. The accessible name is the label without its
/// ellipsis ([`ActionButton::name`]).
pub fn button_label(c: &Catalog, action: ActionId, reading: bool, editing: bool) -> String {
    let id = match action {
        ActionId::Open => "gui-button-open",
        ActionId::ChooseFont => "gui-button-font",
        ActionId::ToggleEditMode if editing => "gui-button-finish-editing",
        ActionId::ToggleEditMode => "gui-button-edit",
        ActionId::Settings => "gui-button-settings",
        ActionId::CommandPalette => "gui-button-commands",
        ActionId::PlayPause if reading => "gui-button-pause",
        ActionId::PlayPause => "gui-button-play",
        ActionId::Stop => "gui-button-stop",
        ActionId::PreviousSentence => "gui-button-previous-sentence",
        ActionId::NextSentence => "gui-button-next-sentence",
        ActionId::RateDown => "gui-button-slower",
        ActionId::RateUp => "gui-button-faster",
        _ => return action.id().to_owned(),
    };
    c.tr(id)
}

/// The short description of the button for `action` (`gui-hint-*`, at
/// most 40 cells), which NVDA reads after the name: what the button does,
/// in words the name does not already say. Empty, so no description at all,
/// for Stop, Slower, Faster, and the sentence buttons, whose names say it.
/// The command's full help stays in F1, Shift+F1, and the palette.
pub fn button_description(c: &Catalog, action: ActionId) -> String {
    let id = match action {
        ActionId::Open => "gui-hint-open",
        ActionId::ChooseFont => "gui-hint-font",
        ActionId::ToggleEditMode => "gui-hint-edit",
        ActionId::Settings => "gui-hint-settings",
        ActionId::CommandPalette => "gui-hint-commands",
        ActionId::PlayPause => "gui-hint-play",
        _ => return String::new(),
    };
    c.tr(id)
}

/// The catalog for drawn labels: the app's, or English with no app (the
/// screenshot tool's tree before an app exists).
fn catalog_of(app: Option<&App>) -> std::sync::Arc<Catalog> {
    app.map_or_else(Catalog::english, App::catalog)
}

/// A control's shortcut for `action`, from the keymap (`named_key`), as
/// written ("Ctrl+O"): the button's keyboard shortcut and its text on
/// screen. The main key, which is the single key while single-key
/// shortcuts are on ("Space" for Play) and a chord while they are off.
///
/// A lone punctuation key (":" for Commands while single-key shortcuts are
/// on) is hard to see and is not the key people are told; the first chord
/// that works either way ("F2") is shown instead.
pub fn shortcut_for(app: &App, action: ActionId) -> String {
    let named = textweaver_app::named_key_in(&app.catalog(), app.keymap(), action);
    let written = textweaver_app::written_text(&named).into_owned();
    let lone_punctuation = {
        let mut chars = written.chars();
        matches!((chars.next(), chars.next()), (Some(ch), None) if ch.is_ascii_punctuation())
    };
    if lone_punctuation
        && let Some(chord) = app
            .keymap()
            .chords_for(action)
            .into_iter()
            .find(|ch| !ch.is_text_input())
    {
        return chord.to_string();
    }
    written
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
    let c = catalog_of(app);
    let l = |a| button_label(&c, a, false, false);

    // Header: the document's title and the commands.
    let title = NewWidget::new(label("textweaver", 18.0, true)).with_tag(TITLE);
    let header = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with(title, 1.0)
        .with_fixed(button(&l(ActionId::Open), ActionId::Open, app, &mut ids))
        .with_fixed(button(
            &l(ActionId::ChooseFont),
            ActionId::ChooseFont,
            app,
            &mut ids,
        ))
        .with_fixed(
            button(
                &l(ActionId::ToggleEditMode),
                ActionId::ToggleEditMode,
                app,
                &mut ids,
            )
            .with_tag(EDIT),
        )
        .with_fixed(button(
            &l(ActionId::Settings),
            ActionId::Settings,
            app,
            &mut ids,
        ))
        .with_fixed(button(
            &l(ActionId::CommandPalette),
            ActionId::CommandPalette,
            app,
            &mut ids,
        ));
    let header = NewWidget::new(Region::new(NewWidget::new(header), Role::Banner, ""))
        .with_tag(HEADER)
        .with_props(panel(p, 10.0, 16.0));

    // The document.
    let doc = NewWidget::new(
        DocumentView::new(p.clone(), font, Rc::clone(&full_passes))
            .with_select_spoken(experiments.select_spoken)
            .with_edit_role(experiments.edit_role)
            .with_label(c.tr("gui-document")),
    )
    .with_tag(DOC);
    // RSVP, hidden until it is turned on: its own strip under the document,
    // so the word never covers the text or the caret.
    let rsvp = NewWidget::new(RsvpView::new(p.clone())).with_tag(RSVP);

    // Toolbar: Play/Pause is the primary action.
    let play = styled_button(
        &l(ActionId::PlayPause),
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
        .with_fixed(button(&l(ActionId::Stop), ActionId::Stop, app, &mut ids))
        .with_fixed(button(
            &l(ActionId::PreviousSentence),
            ActionId::PreviousSentence,
            app,
            &mut ids,
        ))
        .with_fixed(button(
            &l(ActionId::NextSentence),
            ActionId::NextSentence,
            app,
            &mut ids,
        ))
        .with_spacer(1.0)
        .with_fixed(button(
            &l(ActionId::RateDown),
            ActionId::RateDown,
            app,
            &mut ids,
        ))
        .with_fixed(button(
            &l(ActionId::RateUp),
            ActionId::RateUp,
            app,
            &mut ids,
        ));
    let toolbar = NewWidget::new(Region::new(
        NewWidget::new(toolbar),
        Role::Toolbar,
        c.tr("gui-toolbar-reading"),
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
    c: &Catalog,
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
            label(&c.tr("gui-list-hint"), theme::UI_TEXT, false).accessibility_hidden(true),
        ));
    let card = NewWidget::new(card).with_props(dialog::card_props(p));
    let modal = NewWidget::new(Modal::new(card, title, p.clone())).erased();
    (modal, list_id)
}

/// A yes-or-no question from the app (a download, a removal, a file
/// changed on disk), built by [`question_dialog()`].
pub struct QuestionDialog {
    /// The dialog, for [`Root::set_dialog`].
    pub modal: NewWidget<dyn Widget>,
    /// The Yes button, which takes the focus.
    pub yes: WidgetId,
    /// The No button.
    pub no: WidgetId,
}

/// A question as a dialog: the question is the dialog's name (so a screen
/// reader says it when the focus moves in), then Yes and No buttons whose
/// keys are `Y` and `N`. Typing `y` or `n` anywhere in it answers, as in the
/// terminal; Escape is no; any other character asks again.
pub fn question_dialog(
    p: &Palette,
    c: &textweaver_app::lexicon::i18n::Catalog,
    question: &str,
) -> QuestionDialog {
    // The name is the question alone: "Quit textweaver?", not "... y or n";
    // the Yes and No buttons carry the keys.
    let question = textweaver_app::text_util::question_name(question);
    let yes = NewWidget::new(ActionButton::new(c.tr("gui-yes")).with_shortcut("Y"));
    let no = NewWidget::new(ActionButton::new(c.tr("gui-no")).with_shortcut("N"));
    let (yes_id, no_id) = (yes.id(), no.id());
    let buttons = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_spacer(1.0)
        .with_fixed(yes)
        .with_fixed(no);
    let card = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(NewWidget::new(
            label(question, 18.0, false).accessibility_hidden(true),
        ))
        .with_fixed_spacer(Length::px(10.0))
        .with_fixed(NewWidget::new(
            label(&c.tr("gui-question-hint"), theme::UI_TEXT, false).accessibility_hidden(true),
        ))
        .with_fixed_spacer(Length::px(14.0))
        .with_fixed(NewWidget::new(buttons));
    let card = NewWidget::new(card).with_props(dialog::card_props(p));
    let modal =
        NewWidget::new(Modal::new(card, question, p.clone()).with_answer_keys(true)).erased();
    QuestionDialog {
        modal,
        yes: yes_id,
        no: no_id,
    }
}

/// The settings dialog, built by [`settings_dialog()`].
pub struct SettingsDialog {
    /// The dialog, for [`Root::set_dialog`].
    pub modal: NewWidget<dyn Widget>,
    /// The form, which takes the focus.
    pub form: WidgetId,
    /// The Close button.
    pub close: WidgetId,
    /// The Colors dialog's Reset all colors button (`None` in the settings
    /// dialog).
    pub reset: Option<WidgetId>,
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
    let c = app.catalog();
    if form.is_colors() {
        return colors_dialog(p, form, app, row);
    }
    let settings_title = c.tr("settings-title");
    let sections = NewWidget::new(
        ChoiceList::new(
            c.tr("gui-settings-sections"),
            form.section_items(&c),
            p.clone(),
        )
        .with_selected(section)
        .with_focus_actions(true),
    )
    .with_tag(SECTIONS);
    let grid = NewWidget::new(
        SettingsGrid::new(
            form.form_label(section, &c),
            form.rows(section, app),
            p.clone(),
        )
        .with_help_text({
            // The form's own section keys (not keymap commands).
            use textweaver_app::keymap::{Key, KeyChord, Modifiers};
            let next = KeyChord::new(Key::PageDown, Modifiers::CTRL).to_string();
            let previous = KeyChord::new(Key::PageUp, Modifiers::CTRL).to_string();
            c.fmt(
                "gui-settings-form-help",
                &args!["next" => next, "previous" => previous],
            )
        })
        .with_selected(row),
    )
    .with_tag(FORM);
    let form_id = grid.id();
    // The dialog's own key (not a keymap command): Escape closes it.
    let escape = textweaver_app::keymap::KeyChord::new(
        textweaver_app::keymap::Key::Escape,
        textweaver_app::keymap::Modifiers::empty(),
    );
    let close = NewWidget::new(
        ActionButton::new(c.tr("gui-button-close"))
            .with_shortcut(escape.to_string())
            .with_description(c.tr("gui-settings-close-help")),
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
                label(&c.tr("gui-settings-saved-hint"), theme::UI_TEXT, false)
                    .accessibility_hidden(true),
            ),
            1.0,
        )
        .with_fixed(close);
    let card = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(NewWidget::new(
            label(&settings_title, 20.0, true).accessibility_hidden(true),
        ))
        .with_fixed_spacer(Length::px(14.0))
        .with_fixed(NewWidget::new(body))
        .with_fixed_spacer(Length::px(14.0))
        .with_fixed(NewWidget::new(footer));
    let card = NewWidget::new(card).with_props(dialog::card_props(p));
    let modal =
        NewWidget::new(Modal::new(card, settings_title, p.clone()).with_max_width(960.0)).erased();
    SettingsDialog {
        modal,
        form: form_id,
        close: close_id,
        reset: None,
    }
}

/// View, Colors (W6a6): every color setting in one form, the reading
/// aids' highlights first, each with a sample and its contrast said in
/// words; a Reset all colors button puts the theme's back; Close (Escape)
/// closes. The form's help names the keys and that color never carries
/// meaning alone.
fn colors_dialog(p: &Palette, form: &SettingsForm, app: &App, row: usize) -> SettingsDialog {
    let c = app.catalog();
    let title = form.section_title(0, &c);
    let grid = NewWidget::new(
        SettingsGrid::new(title.clone(), form.rows(0, app), p.clone())
            .with_help_text(c.tr("gui-colors-help"))
            .with_selected(row),
    )
    .with_tag(FORM);
    let form_id = grid.id();
    // The dialog's own key (not a keymap command): Escape closes it.
    let escape = textweaver_app::keymap::KeyChord::new(
        textweaver_app::keymap::Key::Escape,
        textweaver_app::keymap::Modifiers::empty(),
    );
    let reset = NewWidget::new(
        ActionButton::new(c.tr("gui-colors-reset-all"))
            .with_description(c.tr("gui-colors-reset-all-help")),
    );
    let reset_id = reset.id();
    let close = NewWidget::new(
        ActionButton::new(c.tr("gui-button-close"))
            .with_shortcut(escape.to_string())
            .with_description(c.tr("gui-settings-close-help")),
    );
    let close_id = close.id();
    let footer = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with(
            NewWidget::new(
                label(&c.tr("gui-settings-saved-hint"), theme::UI_TEXT, false)
                    .accessibility_hidden(true),
            ),
            1.0,
        )
        .with_fixed(reset)
        .with_fixed_spacer(Length::px(10.0))
        .with_fixed(close);
    let card = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(NewWidget::new(
            label(&title, 20.0, true).accessibility_hidden(true),
        ))
        .with_fixed_spacer(Length::px(14.0))
        .with_fixed(grid)
        .with_fixed_spacer(Length::px(14.0))
        .with_fixed(NewWidget::new(footer));
    let card = NewWidget::new(card).with_props(dialog::card_props(p));
    let modal = NewWidget::new(Modal::new(card, title, p.clone()).with_max_width(820.0)).erased();
    SettingsDialog {
        modal,
        form: form_id,
        close: close_id,
        reset: Some(reset_id),
    }
}

/// Recolors the window's own panels and the document for `p` (the
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

/// The notes, bookmarks, the reader's highlights, and the search matches in
/// `window`, as the document view draws them (the selection and the spoken
/// word and sentence are its own).
pub fn marks_in(app: &App, window: CharRange) -> Vec<(CharRange, crate::document::DocMark)> {
    use crate::document::DocMark;
    use textweaver_app::HighlightKind as K;
    app.highlights(window)
        .into_iter()
        .filter_map(|h| {
            let mark = match h.kind {
                K::UserHighlight => DocMark::Highlight,
                K::Note => DocMark::Note,
                K::Bookmark => DocMark::Bookmark,
                K::FindHit => DocMark::FindHit,
                K::CurrentFindHit => DocMark::CurrentFindHit,
                K::Selection | K::SpokenSentence | K::SpokenWord => return None,
            };
            Some((h.range, mark))
        })
        .collect()
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

/// The view moved its caret (or its selection): the app's cursor and
/// selection follow, so the keymap's Copy and Cut (and a note or highlight
/// on the selection) take what is selected on screen. A selection the app
/// held (from a command) is let go when the view moves the caret without
/// Shift, so typing goes to the caret. Returns the app's effects, or
/// `None` when the app already had this caret and selection. The window
/// mutes the app's announcer meanwhile: a screen reader reads the move.
pub fn sync_caret(
    app: &mut App,
    caret: textweaver_app::core::CharPos,
    selection: Option<CharRange>,
) -> Option<Vec<Effect>> {
    let selection = selection.filter(|r| !r.is_empty());
    let app_cursor = app.session().map(|s| s.cursor);
    let app_selection = app
        .session()
        .and_then(|s| s.selection)
        .filter(|r| !r.is_empty());
    let select = app_selection != selection;
    if app_cursor == Some(caret) && !select {
        return None;
    }
    if select {
        let range = selection.unwrap_or(CharRange::new(caret.0, caret.0));
        let _ = app.dispatch(Command::Select(range));
    }
    Some(app.dispatch(Command::SetCursor(caret)))
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
            // and recenters on jumps or when the text changed.
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
                        // An edit keeps the paragraphs around the change
                        // (their layouts and nodes); else a slide or a
                        // new model.
                        let model = if edited && !aids_changed {
                            match DocumentView::edit_model(&mut d, model) {
                                Ok(()) => return,
                                Err(model) => *model,
                            }
                        } else {
                            model
                        };
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
    // The reader's marks and the search matches in the window.
    let marks = shown
        .window
        .map(|w| marks_in(app, w.range()))
        .unwrap_or_default();
    if marks != shown.marks {
        host.edit(DOC, |mut d| DocumentView::set_marks(&mut d, marks.clone()));
        shown.marks = marks;
    }
    // Speech Cursor mode's line keys go to the keymap, not the caret.
    let math = app.math_exploring() && !app.confirmation_pending();
    let yielded =
        keys::yielded_caret_keys(app.keymap(), app.mode().layer(), math, Platform::current());
    if shown.yielded.as_ref() != Some(&yielded) {
        host.edit(DOC, |mut d| {
            DocumentView::set_yielded_keys(&mut d, yielded.clone())
        });
        shown.yielded = Some(yielded);
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
        let text = button_label(&app.catalog(), ActionId::PlayPause, reading, false);
        host.edit(PLAY, |mut b| ActionButton::set_label(&mut b, text));
        shown.reading = reading;
    }
    if editing != shown.edit_button {
        let text = button_label(&app.catalog(), ActionId::ToggleEditMode, false, editing);
        host.edit(EDIT, |mut b| ActionButton::set_label(&mut b, text));
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
        // The document's name is its node's name, so a screen reader says
        // it when the window or the document takes the focus (W8a).
        let label = document_label(app);
        host.edit(DOC, |mut d| DocumentView::set_label(&mut d, label));
        shown.title = title;
    }
    loaded
}

/// The document view's accessible name: the document's title first, then
/// what it is ("Reading check, document"), or "Document" with none open.
/// UI Automation's Name, which NVDA and JAWS say when the document takes
/// the focus (the legal report's item 6).
pub fn document_label(app: &App) -> String {
    let c = app.catalog();
    match app.session() {
        Some(s) if !s.title.trim().is_empty() => {
            c.fmt("gui-document-titled", &args!["title" => s.title.as_str()])
        }
        _ => c.tr("gui-document"),
    }
}

/// The window's title and its node's name: the document's title, then
/// textweaver ("Reading check - textweaver"), or "textweaver" alone.
pub fn window_title(app: &App) -> String {
    app.session().map_or_else(
        || "textweaver".to_owned(),
        |s| format!("{} - textweaver", s.title),
    )
}

impl Gui {
    /// `--announce uia`: raises a UI Automation Notification event on the
    /// window for each message (the live region's nodes stay, not live).
    fn notify(&mut self, ctx: &mut DriverCtx<'_>, messages: &[Message]) {
        if self.hwnd == 0 {
            self.hwnd = hwnd_of(ctx.window(self.window_id).handle());
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
        if self.app.font_downloads() != self.font_downloads {
            // A font finished downloading (Lexend): register it, and lay the
            // text out again so a family list naming it now finds it.
            self.font_downloads = self.app.font_downloads();
            let root = ctx.render_root(self.window_id);
            for blob in crate::fonts::downloaded_blobs(self.app.fonts_folder().as_deref()) {
                let _ = root.register_fonts(blob);
            }
            self.shown.font = None;
            if self.log {
                crate::log::line("downloaded fonts registered");
            }
        }
        let root = ctx.render_root(self.window_id);
        let before = self.shown.state;
        let started = Instant::now();
        let loaded = refresh_host(&self.app, &mut self.shown, root, self.log);
        let refreshed = started.elapsed().as_secs_f64() * 1000.0;
        if let Some(ms) = loaded {
            let len = self.app.session().map_or(0, |s| s.doc.len_chars());
            self.timings.load_ms.push((len, ms));
        } else if before.spoken != self.shown.state.spoken
            && let Some(line) = self.timings.highlight_moved(refreshed)
            && self.log
        {
            // The driver's share; the widget passes follow in Masonry.
            crate::log::line(&line);
        }
        // Copy and Cut (the keymap's): what the app copied goes on the
        // system clipboard, as the terminal sends it with OSC 52.
        if let Some(text) = self.app.take_clipboard() {
            if self.log {
                crate::log::line(&format!("clipboard: {} chars", text.chars().count()));
            }
            ctx.render_root(self.window_id)
                .edit_widget_with_tag(DOC, |mut d| d.ctx.set_clipboard(text));
        }
        let messages: Vec<Message> = self.queue.borrow_mut().drain(..).collect();
        let (tree_seen, release_due) = ctx
            .render_root(self.window_id)
            .get_widget_with_tag(ANNOUNCER)
            .map_or((true, false), |a| {
                (a.inner().tree_seen(), a.inner().release_due())
            });
        if self.announce == AnnounceMode::Uia {
            // Notifications, too, wait for a screen reader to ask for the
            // tree: raised before, no one hears them.
            self.held_notices.extend(messages.iter().cloned());
            if tree_seen && !self.held_notices.is_empty() {
                let notices = std::mem::take(&mut self.held_notices);
                self.notify(ctx, &notices);
            }
            let excess = self.held_notices.len().saturating_sub(12);
            self.held_notices.drain(..excess);
        }
        if !messages.is_empty() {
            let root = ctx.render_root(self.window_id);
            root.edit_widget_with_tag(ANNOUNCER, |mut a| Announcer::say(&mut a, messages));
        }
        if release_due {
            if self.log {
                crate::log::line("announcements held until the tree was asked for: released");
            }
            ctx.render_root(self.window_id)
                .edit_widget_with_tag(ANNOUNCER, |mut a| Announcer::release(&mut a));
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
        // The interface language changed: every drawn label the window
        // owns, in the new language (the app's own messages already are).
        let c = self.app.catalog();
        if c.lang() != self.lang {
            let first = self.lang.is_empty();
            self.lang = c.lang().to_owned();
            if !first {
                let (reading, editing) = (self.shown.reading, self.shown.edit_button);
                let root = ctx.render_root(self.window_id);
                for (id, action) in &self.buttons.by_id {
                    let text = button_label(&c, *action, reading, editing);
                    let help = button_description(&c, *action);
                    root.edit_widget(*id, |mut w| {
                        let mut b = w.downcast::<ActionButton>();
                        ActionButton::set_label(&mut b, text);
                        ActionButton::set_description(&mut b, help);
                    });
                }
                root.edit_widget_with_tag(TOOLBAR, |mut r| {
                    Region::set_label(&mut r, c.tr("gui-toolbar-reading"));
                });
                let doc_label = document_label(&self.app);
                root.edit_widget_with_tag(DOC, |mut d| {
                    DocumentView::set_label(&mut d, doc_label);
                });
                if self.log {
                    crate::log::line(&format!("labels: {}", self.lang));
                }
            }
        }
        // The theme changed (a key, the palette, or the settings), or the
        // system's high contrast mode was turned on or off.
        let theme_key = self.app.reading_theme_key();
        let system = self.system_colors();
        if (!self.fixed_theme && theme_key != self.theme_key) || system != self.system {
            self.theme_key = theme_key;
            self.system = system;
            self.palette = match &system {
                Some(c) => crate::system_colors::palette(c),
                None => Palette::from_theme(&self.app.reading_theme()),
            };
            let root = ctx.render_root(self.window_id);
            root.set_default_properties(Arc::new(theme::default_properties(&self.palette)));
            apply_palette(root, &self.palette);
            if self.log {
                crate::log::line(&format!("theme: {}", self.palette.name));
            }
        }
        self.sync_question(ctx);
        self.sync_misspellings(ctx);
        // The font list (the app's list, the window's families): a family
        // chosen applies; a list closed without one leaves the font alone.
        if let Some(i) = self.app.take_frontend_choice() {
            self.font_chosen(ctx, i);
        } else if self.font_choices.is_some() && self.app.list_model().is_none() {
            self.font_choices = None;
        }
        let title = window_title(&self.app);
        if title != self.window_title {
            ctx.window(self.window_id).handle().set_title(&title);
            // AT-SPI and macOS read the title from the window's node.
            ctx.render_root(self.window_id)
                .set_window_label(title.as_str());
            self.window_title = title;
        }
        self.sync_chrome(ctx);
        self.sync_menus();
    }

    /// The title bar and the menus follow the palette (W8a-m), and the
    /// menu bar hides or shows as `gui.auto_hide_menu` says: a bar a key
    /// showed hides again once its menu loop is over. Nothing is said.
    fn sync_chrome(&mut self, ctx: &mut DriverCtx<'_>) {
        let chrome = crate::dark_mode::for_palette(&self.palette);
        if chrome != self.chrome {
            self.chrome = chrome;
            ctx.window(self.window_id)
                .handle()
                .set_theme(chrome.window_theme());
            if let Some(n) = self.native.as_mut() {
                n.set_chrome(chrome);
            }
            if self.log {
                crate::log::line(&format!("frame: {}", chrome.name()));
            }
        }
        let auto_hide = self.app.settings().gui.auto_hide_menu;
        if let Some(n) = self.native.as_mut() {
            n.set_auto_hide(auto_hide);
            if n.settle() && self.log {
                crate::log::line("menu bar hidden");
            }
        }
    }

    /// Shows the hidden menu bar (`gui.auto_hide_menu`) before a key or
    /// the menu command enters it; the window then looks back soon to
    /// hide it again.
    fn reveal_menu_bar(&mut self) {
        if self.native.as_mut().is_some_and(|n| n.reveal()) {
            self.tick_ms.store(
                u64::try_from(MENU_TICK.as_millis()).unwrap_or(150),
                Ordering::Relaxed,
            );
            if self.log {
                crate::log::line("menu bar shown");
            }
        }
    }

    fn dispatch(&mut self, ctx: &mut DriverCtx<'_>, cmd: Command) {
        if self.log {
            crate::log::line(&format!("command {cmd:?}"));
        }
        self.menu_dirty = true;
        // A command, pressed or chosen from a menu (`RunCommand`, which the
        // app keeps as a recent command).
        // While a key is being described (Shift+F1), the app describes
        // it, the window's own commands too.
        let (command, from_menu) = window_command_of(&cmd, self.app.describing_next_key())
            .map_or((None, false), |(a, m)| (Some(a), m));
        if let Some(a) = command
            && self.window_command(ctx, a)
        {
            if from_menu {
                self.app.remember_command(a);
            }
            return;
        }
        // Open Path (a key, or chosen in the palette) is the app's Open,
        // answered by typing.
        self.typed_open = match &cmd {
            Command::Action(a) | Command::RunCommand(a) => *a == ActionId::OpenPath,
            Command::Answer(id) => id == ActionId::OpenPath.id(),
            _ => false,
        };
        let effects = self.app.dispatch(cmd);
        self.run_effects(ctx, effects);
        self.typed_open = false;
        self.refresh(ctx);
    }

    /// True for the commands the window runs itself rather than the app:
    /// the settings and colors dialogs, the text size, the font, and, where
    /// the menus are native, the menu key.
    fn is_window_command(&self, a: ActionId) -> bool {
        match a {
            ActionId::Settings | ActionId::ColorSettings => !self.settings_list,
            ActionId::TextLarger
            | ActionId::TextSmaller
            | ActionId::TextSizeReset
            | ActionId::ChooseFont => true,
            ActionId::Menu => self.native.is_some() && cfg!(windows),
            _ => false,
        }
    }

    /// Runs `a` if the window runs it itself ([`Self::is_window_command`]);
    /// returns false for the app's commands.
    fn window_command(&mut self, ctx: &mut DriverCtx<'_>, a: ActionId) -> bool {
        if !self.is_window_command(a) {
            return false;
        }
        match a {
            ActionId::Settings => self.open_settings(ctx, None),
            ActionId::ColorSettings => self.open_colors(ctx),
            ActionId::TextLarger => self.text_size(ctx, Step::Larger),
            ActionId::TextSmaller => self.text_size(ctx, Step::Smaller),
            ActionId::TextSizeReset => self.text_size(ctx, Step::Reset),
            ActionId::ChooseFont => self.open_fonts(ctx),
            ActionId::Menu => {
                // The native menu bar, entered as F10 enters it.
                self.reveal_menu_bar();
                let hwnd = self.window_handle(ctx);
                if !crate::menus::enter_menu_bar(hwnd, '\0') {
                    return false;
                }
            }
            _ => return false,
        }
        true
    }

    /// The window's Win32 handle (0 elsewhere).
    fn window_handle(&mut self, ctx: &mut DriverCtx<'_>) -> isize {
        if self.hwnd == 0 {
            self.hwnd = hwnd_of(ctx.window(self.window_id).handle());
        }
        self.hwnd
    }

    /// Builds the native menus from the app's model and attaches them to
    /// the window (Windows and macOS); elsewhere the menu key shows the
    /// list menu.
    fn attach_menus(&mut self, ctx: &mut DriverCtx<'_>) {
        if !crate::menus::NATIVE || self.list_menus {
            return;
        }
        let started = Instant::now();
        crate::menus::listen(self.proxy.clone(), self.window_id);
        let key = crate::menus::Fingerprint::of(&self.app);
        let tree = crate::menus::tree(&self.app);
        let modelled = started.elapsed();
        let auto_hide = self.app.settings().gui.auto_hide_menu;
        let window = ctx.window(self.window_id).handle();
        match crate::menus::Native::attach(window, tree, self.chrome, auto_hide) {
            Ok(n) => {
                if self.log {
                    crate::log::line(&format!(
                        "menus: native, model {:.1} ms, attached in {:.1} ms, frame: {}{}",
                        modelled.as_secs_f64() * 1000.0,
                        (started.elapsed() - modelled).as_secs_f64() * 1000.0,
                        self.chrome.name(),
                        if auto_hide { ", hidden until Alt" } else { "" }
                    ));
                    // What the system holds, as a screen reader will read
                    // it: each item's text, a tab, and its key.
                    for line in n.dump() {
                        crate::log::line(&format!("menu item: {line}"));
                    }
                }
                self.native = Some(n);
                self.menu_key = Some(key);
            }
            Err(e) => {
                if self.log {
                    crate::log::line(&format!("menus: not attached ({e}); the list menu instead"));
                }
            }
        }
    }

    /// After a command: the native menus again, if what they show changed.
    fn sync_menus(&mut self) {
        if !std::mem::take(&mut self.menu_dirty) {
            return;
        }
        let Some(native) = self.native.as_mut() else {
            return;
        };
        let key = crate::menus::Fingerprint::of(&self.app);
        if self.menu_key.as_ref() == Some(&key) {
            return;
        }
        self.menu_key = Some(key);
        let started = Instant::now();
        let tree = crate::menus::tree(&self.app);
        match native.update(tree) {
            Ok(true) if self.log => crate::log::line(&format!(
                "menus: rebuilt in {:.1} ms",
                started.elapsed().as_secs_f64() * 1000.0
            )),
            Err(e) if self.log => crate::log::line(&format!("menus: not rebuilt ({e})")),
            _ => {}
        }
    }

    /// A menu item was chosen: an open dialog closes first (as Escape
    /// would), then the command runs as a recent command, or the recent
    /// document opens.
    fn menu_picked(&mut self, ctx: &mut DriverCtx<'_>, id: &str) {
        let Some(pick) = crate::menus::Pick::from_id(id) else {
            return;
        };
        if self.log {
            crate::log::line(&format!("menu: {pick:?}"));
        }
        if matches!(
            self.dialog,
            Some(OpenDialog::FileChooser { .. } | OpenDialog::FolderChooser)
        ) {
            // The system's file chooser is modal to the window; nothing is
            // run behind it.
            return;
        }
        self.cancel_dialog(ctx);
        match pick {
            crate::menus::Pick::Command(a) => self.dispatch(ctx, Command::RunCommand(a)),
            crate::menus::Pick::Document(p) => self.dispatch(ctx, Command::Open(p)),
        }
    }

    /// Windows, with the native menu bar: the keys that enter it as in any
    /// Windows program. F10 (the menu key) and Alt alone are Windows' own
    /// (winit leaves their key-up to the system when a window has a menu);
    /// Alt with a menu's letter and Alt+Space reach the window as keys, so
    /// the window enters the menu or the system menu itself, unless the
    /// keymap binds the chord. Returns true when the key was taken.
    fn native_menu_key(
        &mut self,
        ctx: &mut DriverCtx<'_>,
        chord: &textweaver_app::keymap::KeyChord,
        action: Option<ActionId>,
    ) -> bool {
        use textweaver_app::keymap::{Key, Modifiers};
        if !cfg!(windows) || self.native.is_none() {
            return false;
        }
        if action == Some(ActionId::Menu) && *chord == keys::menu_bar_key() {
            // Windows enters the bar on the key's release; a hidden bar is
            // shown first, so there is one to enter.
            self.reveal_menu_bar();
            return true;
        }
        if action.is_some() || chord.mods != Modifiers::ALT {
            return false;
        }
        let letter = match chord.key {
            Key::Space => ' ',
            Key::Char(c)
                if self
                    .native
                    .as_ref()
                    .is_some_and(|n| crate::menus::access_letters(n.shown()).contains(&c)) =>
            {
                c
            }
            _ => return false,
        };
        if letter != ' ' {
            self.reveal_menu_bar();
        }
        let hwnd = self.window_handle(ctx);
        let entered = crate::menus::enter_menu_bar(hwnd, letter);
        if self.log {
            crate::log::line(&format!("menu bar entered with {letter:?}: {entered}"));
        }
        entered
    }

    /// The command on row `row` of the list menu, while the app shows the
    /// menus as a list and that row runs a command (rows are the menu's
    /// items without its separators, as the app lists them).
    fn menu_row_command(&self, row: usize) -> Option<ActionId> {
        crate::menus::list_row_command(&self.app, row)
    }

    /// Closes the open dialog as Escape would.
    fn cancel_dialog(&mut self, ctx: &mut DriverCtx<'_>) {
        match &self.dialog {
            None | Some(OpenDialog::FileChooser { .. } | OpenDialog::FolderChooser) => {}
            Some(OpenDialog::Question { .. }) => {
                self.answer_question(ctx, textweaver_app::Confirm::No);
            }
            Some(OpenDialog::Settings(_)) => {
                self.settings_dialog_action(ctx, &DialogAction::Cancel);
            }
            Some(OpenDialog::SettingEdit(..)) => {
                self.setting_edit_answer(ctx, None);
                self.settings_dialog_action(ctx, &DialogAction::Cancel);
            }
            Some(OpenDialog::Prompt) => self.prompt_answer(ctx, None),
            Some(OpenDialog::List(_) | OpenDialog::Palette(_) | OpenDialog::Voices { .. }) => {
                self.answer(ctx, Command::Cancel);
            }
        }
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
                // A prompt for a path (Open, Save As, Insert Image, import
                // and export of settings, profiles and references): the
                // system's file chooser, unless Open Path asked for the
                // typed prompt or the file browser just filled it (W8a-f).
                Effect::Prompt { label, purpose }
                    if file_chooser::uses_chooser(
                        purpose,
                        self.typed_open,
                        self.app
                            .prompt_model()
                            .is_some_and(textweaver_app::PromptModel::from_browser),
                    ) =>
                {
                    self.open_file_chooser(ctx, &label, purpose);
                }
                Effect::Prompt { label, purpose } => self.open_prompt(ctx, &label, purpose),
                // A command choosing a folder (W8a-f): the system's folder
                // chooser, with the file browser waiting behind it as the
                // fallback.
                Effect::ShowList { .. } if self.app.folder_choice().is_some() => {
                    self.open_folder_chooser(ctx);
                }
                Effect::ShowList { .. }
                    if matches!(self.dialog, Some(OpenDialog::FolderChooser)) => {}
                // The open list changed (filtered, a setting changed): show
                // it in place, keeping focus in the dialog.
                Effect::ShowList { .. }
                    if matches!(
                        self.dialog,
                        Some(OpenDialog::List(_) | OpenDialog::Voices { .. })
                    ) =>
                {
                    self.sync_list(ctx);
                }
                // The voice list: the voice manager, with its buttons.
                Effect::ShowList { title, items } if self.app.voice_list_open() => {
                    self.open_voices(ctx, &title, items);
                }
                Effect::ShowList { title, items } => self.open_list(ctx, &title, items),
            }
        }
    }

    /// A prompt from the app: its text and history are the app's
    /// `PromptModel`, which the field keeps in step with `PromptKey`s.
    fn open_prompt(&mut self, ctx: &mut DriverCtx<'_>, label_text: &str, purpose: PromptPurpose) {
        let paths = purpose.is_path();
        let c = self.app.catalog();
        let hint = if paths {
            // The browse key (F4) opens the file browser (W8a-f).
            let key = textweaver_app::path_prompt::browse_key().to_string();
            format!(
                "{} {}",
                c.tr("gui-prompt-path-hint"),
                c.fmt("gui-prompt-browse-hint", &args!["key" => key])
            )
        } else {
            c.tr("gui-prompt-hint")
        };
        let initial = self
            .app
            .prompt_model()
            .map(textweaver_app::PromptModel::text)
            .unwrap_or_default();
        // The edit details form (W7m): Tab and Shift+Tab move between its
        // fields, each the app's prompt.
        let fields = purpose == PromptPurpose::DocumentDetails;
        self.show_prompt(ctx, label_text, &hint, &initial, paths, fields);
        self.dialog = Some(OpenDialog::Prompt);
        if self.log {
            crate::log::line(&format!("dialog: prompt {label_text:?}"));
        }
    }

    /// The system's file chooser, on its own thread and modal to the
    /// window, for the app's prompt `purpose`: its title, filters, the
    /// name offered and the folder it starts in are the app's
    /// [`PathPromptSpec`](textweaver_app::path_prompt::PathPromptSpec) (a
    /// save dialog for Save As and the exports). Its answer comes back as
    /// a [`FileChosen`] action; until then the app's prompt waits,
    /// labelled `label_text`.
    fn open_file_chooser(
        &mut self,
        ctx: &mut DriverCtx<'_>,
        label_text: &str,
        purpose: PromptPurpose,
    ) {
        let Some(spec) = self.app.path_prompt_spec(purpose) else {
            self.open_prompt(ctx, label_text, purpose);
            return;
        };
        let c = self.app.catalog();
        let registry = textweaver_app::formats::Registry::with_builtins();
        let filters = file_chooser::spec_filters(
            &spec,
            &registry.extensions(),
            &c.tr("gui-open-documents"),
            &c.tr("gui-open-all-files"),
        );
        let folder = spec
            .folder
            .clone()
            .or_else(|| file_chooser::start_folder(self.app.session().map(|s| s.key.0.as_str())));
        let mut chooser = file_chooser::Chooser::new(
            ctx.window(self.window_id).handle(),
            &spec.title,
            &filters,
            folder.as_deref(),
        );
        if matches!(spec.kind, textweaver_app::path_prompt::PathKind::Write(_)) {
            chooser = chooser.saving(spec.file_name.as_deref().unwrap_or_default());
        }
        let proxy = self.proxy.clone();
        let window_id = self.window_id;
        chooser.show(move |chosen| {
            let _ = proxy.send_event(MasonryUserEvent::AsyncAction(window_id, Box::new(chosen)));
        });
        self.dialog = Some(OpenDialog::FileChooser {
            label: label_text.to_owned(),
            purpose,
        });
        if self.log {
            crate::log::line(&format!("dialog: system file chooser for {purpose:?}"));
        }
    }

    /// The file chooser's answer: the file opens through the app's Open
    /// prompt (so it joins the prompt's history), a cancel cancels it, and
    /// a chooser that never appeared gives way to the typed prompt. The
    /// focus goes back to the document either way.
    fn file_chosen(&mut self, ctx: &mut DriverCtx<'_>, chosen: &FileChosen) {
        if matches!(self.dialog, Some(OpenDialog::FolderChooser)) {
            self.folder_chosen(ctx, chosen);
            return;
        }
        let Some(OpenDialog::FileChooser {
            label: label_text,
            purpose,
        }) = self.dialog.take()
        else {
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
                // The system save dialog already asked before replacing.
                if purpose == PromptPurpose::SaveAs {
                    self.app.save_as_confirmed_by_system();
                }
                PromptKey::Enter
            }
            file_chooser::Outcome::Cancelled => PromptKey::Escape,
            file_chooser::Outcome::Failed => {
                let said = self.app.catalog().tr(if purpose == PromptPurpose::Open {
                    "gui-open-no-dialog"
                } else {
                    "gui-chooser-no-dialog"
                });
                self.app
                    .announce_as(&said, Priority::Assertive, Importance::Error);
                self.open_prompt(ctx, &label_text, purpose);
                self.refresh(ctx);
                return;
            }
        };
        let effects = self.app.dispatch(Command::PromptKey(key));
        self.run_effects(ctx, effects);
        self.refresh(ctx);
    }

    /// The system's folder chooser for the folder the app's file browser
    /// is choosing (W8a-f): titled with what the folder is for, starting
    /// in the document's folder. Its answer comes back as a
    /// [`FileChosen`] action, like the file chooser's.
    fn open_folder_chooser(&mut self, ctx: &mut DriverCtx<'_>) {
        let Some(choice) = self.app.take_folder_choice() else {
            return;
        };
        if self.dialog.is_some() {
            // The list it was chosen from (audio export's "another
            // folder") closes; focus returns after the chooser.
            self.close_dialog(ctx);
        }
        let chooser = file_chooser::Chooser::new(
            ctx.window(self.window_id).handle(),
            &choice.title,
            &[],
            choice.folder.as_deref(),
        )
        .folders();
        let proxy = self.proxy.clone();
        let window_id = self.window_id;
        chooser.show(move |chosen| {
            let _ = proxy.send_event(MasonryUserEvent::AsyncAction(window_id, Box::new(chosen)));
        });
        self.dialog = Some(OpenDialog::FolderChooser);
        if self.log {
            crate::log::line(&format!("dialog: system folder chooser {:?}", choice.title));
        }
    }

    /// The folder chooser's answer: the folder goes to the command waiting
    /// (`Command::PathChosen`), a cancel cancels it, and a chooser that
    /// never appeared gives way to the app's file browser, which chooses
    /// the folder from the keyboard. The focus goes back to the document,
    /// or to the next dialog the command opens.
    fn folder_chosen(&mut self, ctx: &mut DriverCtx<'_>, chosen: &FileChosen) {
        self.dialog = None;
        let outcome = file_chooser::outcome(chosen.path.clone(), chosen.elapsed);
        if self.log {
            crate::log::line(&format!(
                "folder chooser: {outcome:?} after {} ms",
                chosen.elapsed.as_millis()
            ));
        }
        self.close_dialog(ctx);
        let path = match outcome {
            file_chooser::Outcome::Chosen(path) => Some(path),
            file_chooser::Outcome::Cancelled => None,
            file_chooser::Outcome::Failed => {
                let said = self.app.catalog().tr("gui-folder-no-dialog");
                self.app
                    .announce_as(&said, Priority::Assertive, Importance::Error);
                if let Some(list) = self.app.list_model() {
                    let (title, items) = (list.title.clone(), list.items.clone());
                    self.open_list(ctx, &title, items);
                }
                self.refresh(ctx);
                return;
            }
        };
        let effects = self.app.dispatch(Command::PathChosen(path));
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
        tab_fields: bool,
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
            Modal::new(card, label_text, p.clone())
                .with_tab_completion(tab_completes)
                .with_tab_fields(tab_fields),
        )
        .erased();
        self.show_dialog(ctx, modal, field_id);
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
        self.show_settings(ctx, form, at);
    }

    /// View, Colors (W6a6): the Colors dialog, every color setting in one
    /// form, starting on the spoken word's highlight.
    fn open_colors(&mut self, ctx: &mut DriverCtx<'_>) {
        let form = SettingsForm::colors(self.app.settings_schema());
        self.show_settings(ctx, form, None);
    }

    /// Shows the settings dialog (or the Colors dialog) for `form`, on
    /// section and row `at`, or on the first section's first plain row.
    fn show_settings(
        &mut self,
        ctx: &mut DriverCtx<'_>,
        form: SettingsForm,
        at: Option<(usize, usize)>,
    ) {
        let (section, row) = at.unwrap_or_else(|| (0, form.first_plain_row(0)));
        let section = section.min(form.sections.len().saturating_sub(1));
        let d = settings_dialog(&self.palette, &form, &self.app, section, row);
        self.show_dialog(ctx, d.modal, d.form);
        if self.log {
            crate::log::line(&format!(
                "dialog: {}, {} sections, section {section}, row {row}",
                if form.is_colors() {
                    "colors"
                } else {
                    "settings"
                },
                form.sections.len()
            ));
        }
        self.dialog = Some(OpenDialog::Settings(SettingsOpen {
            form,
            section,
            close: d.close,
            reset: d.reset,
        }));
    }

    /// The settings dialog again, as it was (the Colors dialog stays the
    /// Colors dialog), on `section` and `row`.
    fn reopen_settings(
        &mut self,
        ctx: &mut DriverCtx<'_>,
        colors: bool,
        section: usize,
        row: usize,
    ) {
        let schema = self.app.settings_schema();
        let form = if colors {
            SettingsForm::colors(schema)
        } else {
            SettingsForm::new(schema)
        };
        self.show_settings(ctx, form, Some((section, row)));
    }

    /// Reset all colors (the Colors dialog's button): asks first ("Reset
    /// every color to the theme's? y or n"); a yes puts every color setting
    /// back to the theme's and says so once. The Colors dialog opens again
    /// on the same section after the answer.
    fn reset_all_colors(&mut self, ctx: &mut DriverCtx<'_>) {
        let section = match &self.dialog {
            Some(OpenDialog::Settings(open)) => open.section,
            _ => 0,
        };
        self.colors_after_question = Some(section);
        self.app.ask_reset_colors();
        if self.log {
            crate::log::line("colors: asked to reset all");
        }
        self.refresh(ctx);
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
                let c = self.app.catalog();
                let label = setting.label_in(&c);
                if matches!(setting.kind, textweaver_app::SettingKind::Table) {
                    self.app.announce_as(
                        &c.fmt("gui-settings-table", &args!["label" => label.as_str()]),
                        Priority::Polite,
                        Importance::Answer,
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
                let label_text = c.fmt("gui-setting-new-value", &args!["label" => label.as_str()]);
                let hint = if setting.help.is_empty() {
                    c.tr("gui-setting-value-hint")
                } else {
                    setting.help_in(&c)
                };
                let Some(OpenDialog::Settings(open)) = self.dialog.take() else {
                    return;
                };
                self.show_prompt(ctx, &label_text, &hint, &initial, false, false);
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
        self.menu_dirty = true;
        let theme_before = self.palette.name.clone();
        match settings_dialog::apply(&mut self.app, setting, change) {
            Ok(said) => {
                if self.log {
                    crate::log::line(&format!("setting: {said}"));
                }
                self.refresh(ctx);
                if self.palette.name != theme_before {
                    // A new theme: draw the dialog again in its colors.
                    let (section, colors) = match &self.dialog {
                        Some(OpenDialog::Settings(o)) => (o.section, o.form.is_colors()),
                        _ => (0, false),
                    };
                    self.reopen_settings(ctx, colors, section, row);
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
                    self.app
                        .announce_as(&note, Priority::Polite, Importance::Result);
                }
            }
            Err(why) => self
                .app
                .announce_as(&why, Priority::Assertive, Importance::Error),
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
        let c = self.app.catalog();
        let title = open.form.form_label(section, &c);
        let rows = open.form.rows(section, &self.app);
        let item = open.form.section_items(&c).get(section).cloned();
        let first = open.form.first_plain_row(section);
        let root = ctx.render_root(self.window_id);
        root.edit_widget_with_tag(FORM, |mut g| {
            SettingsGrid::set_section(&mut g, title, rows, first);
        });
        root.edit_widget_with_tag(SECTIONS, |mut l| ChoiceList::select(&mut l, section));
        if say && let Some(item) = item {
            self.app
                .announce_as(&format!("{item}."), Priority::Polite, Importance::Answer);
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
                let colors =
                    matches!(&self.dialog, Some(OpenDialog::Settings(o)) if o.form.is_colors());
                self.close_dialog(ctx);
                let said = self.app.catalog().tr(if colors {
                    "gui-colors-closed"
                } else {
                    "gui-settings-closed"
                });
                self.app
                    .announce_as(&said, Priority::Polite, Importance::Dialog);
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
        self.reopen_settings(ctx, open.form.is_colors(), section, row);
        if let (Some(text), Some(setting)) = (answer, setting) {
            self.change_setting(ctx, &setting, row, FormChange::Text(text));
        }
    }

    fn open_list(&mut self, ctx: &mut DriverCtx<'_>, title: &str, items: Vec<String>) {
        let count = items.len();
        let selected = self.app.list_model().map_or(0, |m| m.selected);
        let c = self.app.catalog();
        let (modal, list_id) = list_dialog(&self.palette, &c, title, items, selected, true);
        self.show_dialog(ctx, modal, list_id);
        self.dialog = Some(OpenDialog::List(title.to_owned()));
        if self.log {
            crate::log::line(&format!("dialog: list {title:?} with {count} items"));
        }
    }

    /// The voice manager (W7v): the app's voice list with the filter and
    /// action buttons beside it ([`crate::voices`]), focused on the list.
    fn open_voices(&mut self, ctx: &mut DriverCtx<'_>, title: &str, items: Vec<String>) {
        let count = items.len();
        let selected = self.app.list_model().map_or(0, |m| m.selected);
        let d = crate::voices::voice_dialog(&self.palette, &self.app, title, items, selected);
        self.show_dialog(ctx, d.modal, d.list);
        self.dialog = Some(OpenDialog::Voices {
            title: title.to_owned(),
            buttons: d.buttons,
        });
        if self.log {
            crate::log::line(&format!(
                "dialog: voice manager {title:?} with {count} voices"
            ));
        }
    }

    /// A button of the voice manager: the filters and Fetch run as the
    /// app's voice controls; the others act on the focused voice as their
    /// keys do in the list; Close is Escape.
    fn voice_button(&mut self, ctx: &mut DriverCtx<'_>, button: crate::voices::VoiceButton) {
        use crate::voices::VoiceButton as B;
        use textweaver_app::ListKey as L;
        use textweaver_app::voice_manager::VoiceControl as V;
        if self.log {
            crate::log::line(&format!("voice manager: {button:?}"));
        }
        let control = match button {
            B::Language => V::NextLanguage,
            B::Engine => V::NextEngine,
            B::Fetch => V::FetchCatalog,
            B::Use => return self.list_key(ctx, L::Enter),
            B::Preview => return self.list_key(ctx, L::Details),
            B::Favorite => return self.list_key(ctx, L::Char(' ')),
            B::Remove => return self.list_key(ctx, L::Delete),
            B::Close => return self.answer(ctx, Command::Cancel),
        };
        self.menu_dirty = true;
        let effects = self.app.dispatch(Command::VoiceControl(control));
        self.sync_list(ctx);
        self.run_effects(ctx, effects);
        self.refresh(ctx);
    }

    /// The command palette, the GUI's menu: a filter field over the list of
    /// every command with its keys. Typing filters (and says how many
    /// match); Tab reaches the list; Enter runs the selected command.
    fn open_palette(&mut self, ctx: &mut DriverCtx<'_>, label_text: &str) {
        let p = &self.palette;
        let (ids, items): (Vec<ActionId>, Vec<String>) =
            self.app.palette_candidates("").into_iter().unzip();
        let count = items.len();
        let c = self.app.catalog();
        let field = NewWidget::new(
            TextArea::new_editable("")
                .with_accessible_label(label_text.to_owned())
                .with_style(StyleProperty::FontSize(theme::UI_TEXT + 2.0)),
        )
        .with_tag(PROMPT_FIELD);
        let field_id = field.id();
        let input = NewWidget::new(
            TextInput::from_text_area(field).with_placeholder(c.tr("gui-palette-filter")),
        );
        let list = NewWidget::new(ChoiceList::new(c.tr("gui-palette-list"), items, p.clone()))
            .with_tag(LIST);
        let card = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(NewWidget::new(
                label(label_text, 18.0, true).accessibility_hidden(true),
            ))
            .with_fixed(input)
            .with_fixed(list)
            .with_fixed(NewWidget::new(
                label(&c.tr("gui-palette-hint"), theme::UI_TEXT, false).accessibility_hidden(true),
            ));
        let card = NewWidget::new(card).with_props(dialog::card_props(p));
        let modal = NewWidget::new(Modal::new(card, label_text, p.clone()).with_show_matches(true))
            .erased();
        self.show_dialog(ctx, modal, field_id);
        self.dialog = Some(OpenDialog::Palette(ids));
        if self.log {
            crate::log::line(&format!("dialog: command palette with {count} commands"));
        }
    }

    /// The font list (Ctrl+D, the Font button): the families, bundled
    /// first, starting on the current one.
    ///
    /// The list is the app's (`App::show_frontend_list`), so it moves,
    /// jumps by letter, and is announced as every other list is; the
    /// window keeps the families and applies the one chosen
    /// ([`Self::font_chosen`]).
    fn open_fonts(&mut self, ctx: &mut DriverCtx<'_>) {
        let c = self.app.catalog();
        let choices = crate::font_chooser::choices(
            &c,
            self.installed.names(),
            self.app.fonts_folder().as_deref(),
        );
        let current = crate::fonts::doc_font(&self.app.settings().reading_aids.font);
        let items: Vec<String> = choices.iter().map(|c| c.label.clone()).collect();
        let selected = choices
            .iter()
            .position(|c| current.family.starts_with(&format!("\"{}\"", c.family)))
            .unwrap_or(0);
        let title = c.tr("gui-font-list");
        let intro = c.fmt(
            "gui-font-list-intro",
            &args!["title" => title.as_str(), "n" => items.len()],
        );
        let n = items.len();
        self.font_choices = Some(choices);
        let effects = self.app.show_frontend_list(&title, &intro, items, selected);
        self.run_effects(ctx, effects);
        self.refresh(ctx);
        if self.log {
            crate::log::line(&format!("dialog: font list, {n} families"));
        }
    }

    /// A family was chosen in the font list: it applies at once, keeping
    /// the size, and is said ("Font: OpenDyslexic.").
    fn font_chosen(&mut self, ctx: &mut DriverCtx<'_>, i: usize) {
        let Some(family) = self
            .font_choices
            .take()
            .and_then(|c| c.get(i).map(|c| c.family.clone()))
        else {
            return;
        };
        let new = crate::font_chooser::with_family(&self.app.settings().reading_aids.font, &family);
        self.save_font(new);
        let said = crate::font_chooser::font_message(&self.app.catalog(), &family);
        self.app
            .announce_as(&said, Priority::Polite, Importance::Result);
        // Lexend, not downloaded yet: the app asks first (size and license).
        let asked = self.app.offer_font_download();
        self.run_effects(ctx, asked);
        self.refresh(ctx);
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
        self.app
            .announce_as(&said, Priority::Assertive, Importance::Result);
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
        let said = self
            .app
            .catalog()
            .fmt("gui-palette-count", &args!["n" => n]);
        // The count answers what was typed, as a search's count does.
        self.app
            .announce_as(&said, Priority::Polite, Importance::Answer);
        self.refresh(ctx);
    }

    /// A key in an app list: the app moves its focus, filters, or chooses;
    /// then the dialog shows the list as the app has it, or closes.
    fn list_key(&mut self, ctx: &mut DriverCtx<'_>, k: textweaver_app::ListKey) {
        if self.log {
            crate::log::line(&format!("list key {k:?}"));
        }
        self.menu_dirty = true;
        if k == textweaver_app::ListKey::Enter {
            // Enter on a window command in the list menu: the window runs
            // it, as it would from the palette.
            let selected = self.app.list_model().map_or(0, |m| m.selected);
            if self
                .menu_row_command(selected)
                .is_some_and(|a| self.is_window_command(a))
            {
                self.answer(ctx, Command::Choose(selected));
                return;
            }
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
        // The file browser's own keys (choose this folder, sort, show all),
        // asked of the app so the key and its name stay one (ADR-0045).
        if let Some(key) = self.app.browse_list_key_for(&chord) {
            if self.log {
                crate::log::line(&format!("list chord {chord} -> browser {key:?}"));
            }
            self.list_key(ctx, key);
            return;
        }
        let action = self
            .app
            .keymap()
            .lookup(&chord, textweaver_app::keymap::Layer::Global);
        if self.log {
            crate::log::line(&format!("list chord {chord} -> {action:?}"));
        }
        match action {
            // In the file browser, Say Status previews the focused row;
            // in the voice manager, the focused voice.
            Some(ActionId::SayStatus) if self.app.list_has_details() => {
                self.list_key(ctx, textweaver_app::ListKey::Details);
            }
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
        let (shown, voices_shown) = match &self.dialog {
            Some(OpenDialog::List(t)) => (t.clone(), false),
            Some(OpenDialog::Voices { title, .. }) => (title.clone(), true),
            _ => return,
        };
        let voices = self.app.voice_list_open();
        match self.app.list_model() {
            // A new list in its place (a submenu of the list menu, or the
            // voice manager from the list menu): shown under its own name,
            // so the screen reader says it. A command choosing a folder
            // gets the system's folder chooser instead (W8a-f).
            Some(_) if self.app.folder_choice().is_some() => self.open_folder_chooser(ctx),
            Some(m) if m.title != shown || voices != voices_shown => {
                let (title, items) = (m.title.clone(), m.items.clone());
                if voices {
                    self.open_voices(ctx, &title, items);
                } else {
                    self.open_list(ctx, &title, items);
                }
            }
            Some(m) => {
                let (items, selected) = (m.items.clone(), m.selected);
                let root = ctx.render_root(self.window_id);
                root.edit_widget_with_tag(LIST, |mut l| ChoiceList::sync(&mut l, &items, selected));
                // The filter buttons say what the list shows now.
                if let Some(c) = self.app.voice_controls() {
                    root.edit_widget_with_tag(crate::voices::VOICE_LANGUAGE, |mut b| {
                        ActionButton::set_label(&mut b, c.language.clone());
                    });
                    root.edit_widget_with_tag(crate::voices::VOICE_ENGINE, |mut b| {
                        ActionButton::set_label(&mut b, c.engine.clone());
                    });
                }
                if voices_shown {
                    self.sync_voice_remove(ctx);
                }
            }
            None => self.close_dialog(ctx),
        }
    }

    /// The voice manager's Remove button follows the focused voice: it is
    /// unavailable, with the reason, on a voice that cannot be removed.
    fn sync_voice_remove(&mut self, ctx: &mut DriverCtx<'_>) {
        if !matches!(self.dialog, Some(OpenDialog::Voices { .. })) {
            return;
        }
        let Some(row) = self.app.list_model().map(|m| m.selected) else {
            return;
        };
        let why = crate::voices::remove_unavailable(&self.app, row);
        ctx.render_root(self.window_id).edit_widget_with_tag(
            crate::voices::VOICE_REMOVE,
            |mut b| {
                ActionButton::set_unavailable(&mut b, why);
            },
        );
    }

    /// Edit mode: marks the misspelled words (Alt+M finds them), once the
    /// text has not changed for [`SPELL_PAUSE`], for documents up to
    /// [`SPELL_LIMIT`] chars (the check reads the whole document on the
    /// input thread).
    fn sync_misspellings(&mut self, ctx: &mut DriverCtx<'_>) {
        let key = self
            .app
            .session()
            .filter(|s| self.app.is_editing() && s.doc.len_chars() <= SPELL_LIMIT)
            .map(|s| (s.key.clone(), s.revision));
        let Some(key) = key else {
            if self.spell_marked.take().is_some() {
                ctx.render_root(self.window_id)
                    .edit_widget_with_tag(DOC, |mut d| {
                        DocumentView::set_misspelled(&mut d, Vec::new())
                    });
            }
            self.spell_seen = None;
            return;
        };
        if self.spell_marked.as_ref() == Some(&key) {
            return;
        }
        match &self.spell_seen {
            Some((k, at)) if *k == key && at.elapsed() >= SPELL_PAUSE => {}
            Some((k, _)) if *k == key => return,
            _ => {
                self.spell_seen = Some((key, Instant::now()));
                return;
            }
        }
        let started = Instant::now();
        let ranges = self.app.misspelled_ranges();
        if self.log {
            crate::log::line(&format!(
                "misspellings: {} in {:.1} ms",
                ranges.len(),
                started.elapsed().as_secs_f64() * 1000.0
            ));
        }
        ctx.render_root(self.window_id)
            .edit_widget_with_tag(DOC, |mut d| DocumentView::set_misspelled(&mut d, ranges));
        self.spell_marked = Some(key);
        self.spell_seen = None;
    }

    /// A caret key in the view, said by textweaver's own voice as the
    /// terminal says its caret keys: `App::echo` speaks only in the
    /// self-voicing mode (and not while reading), since a screen reader
    /// reads the caret and the selection itself.
    fn echo_caret(&mut self, echo: CaretEcho) {
        let c = self.app.catalog();
        let text = match echo {
            CaretEcho::Char(ch) => ch.to_string(),
            CaretEcho::LineEnd => c.tr("edit-end-of-line-content"),
            CaretEcho::DocEnd => c.tr("playback-end-of-document-content"),
            CaretEcho::Word(w) | CaretEcho::Line(w) => w,
            CaretEcho::Selection { text, selected } => {
                textweaver_app::text_util::selection_change_text(&c, &text, selected)
            }
        };
        if self.log {
            crate::log::line(&format!("caret echo: {text}"));
        }
        self.app.echo(&text);
    }

    /// The system's high contrast colors, when the window follows them:
    /// with no `--theme`, while `display.follow_os_theme` is on.
    fn system_colors(&self) -> Option<crate::system_colors::SystemColors> {
        if self.fixed_theme || !self.app.settings().display.follow_os_theme {
            return None;
        }
        crate::system_colors::high_contrast()
    }

    /// The window took the focus: the document's name and the window's, in
    /// textweaver's own voice only (`App::echo` speaks only in the
    /// self-voicing mode, and not while reading), since a screen reader says
    /// the window's title and the focused document itself.
    fn window_focused(&mut self) {
        let title = self
            .app
            .session()
            .map_or_else(|| "textweaver".to_owned(), |s| s.title.clone());
        let said = self
            .app
            .catalog()
            .fmt("gui-window-focused", &args!["title" => title]);
        if self.log {
            crate::log::line(&format!("window focused: {said}"));
        }
        // A window taking the focus is the interface's own news.
        if self.app.interface_allows(Importance::Dialog) {
            self.app.echo(&said);
        }
    }

    /// A yes-or-no question from the app shows as a dialog while it is
    /// open (the app has already said it), and goes when it is answered.
    fn sync_question(&mut self, ctx: &mut DriverCtx<'_>) {
        let pending = self.app.confirmation_pending();
        let showing = matches!(self.dialog, Some(OpenDialog::Question { .. }));
        if pending && !showing {
            let question = self.app.status_text().to_owned();
            let q = question_dialog(&self.palette, &self.app.catalog(), &question);
            self.show_dialog(ctx, q.modal, q.yes);
            self.dialog = Some(OpenDialog::Question {
                yes: q.yes,
                no: q.no,
            });
            if self.log {
                crate::log::line(&format!("dialog: question {question:?}"));
            }
        } else if !pending && showing {
            self.close_dialog(ctx);
        }
    }

    /// An answer to the open question: Yes, No, or ask again.
    fn answer_question(&mut self, ctx: &mut DriverCtx<'_>, answer: textweaver_app::Confirm) {
        if !matches!(self.dialog, Some(OpenDialog::Question { .. })) {
            return;
        }
        if answer != textweaver_app::Confirm::Repeat {
            self.close_dialog(ctx);
        }
        if self.log {
            crate::log::line(&format!("answer {answer:?}"));
        }
        self.dispatch(ctx, Command::Confirm(answer));
        if !self.app.confirmation_pending()
            && let Some(section) = self.colors_after_question.take()
        {
            self.menu_dirty = true;
            self.reopen_settings(ctx, true, section, 0);
        }
    }

    /// Shows `modal` over the window with the focus on `focus`. The first
    /// dialog over the window remembers where the focus was, so closing it
    /// puts the focus back there (a toolbar button, or the document).
    fn show_dialog(
        &mut self,
        ctx: &mut DriverCtx<'_>,
        modal: NewWidget<dyn Widget>,
        focus: WidgetId,
    ) {
        let root = ctx.render_root(self.window_id);
        let open = root
            .get_widget_with_tag(ROOT)
            .is_some_and(|r| r.inner().has_dialog());
        if !open {
            self.return_focus = root.focused_widget();
        }
        root.edit_widget_with_tag(ROOT, |mut r| Root::set_dialog(&mut r, Some(modal)));
        root.focus_on(Some(focus));
    }

    /// Closes the dialog; the focus goes back where it was before the
    /// dialog opened, or to the document.
    fn close_dialog(&mut self, ctx: &mut DriverCtx<'_>) {
        self.dialog = None;
        let root = ctx.render_root(self.window_id);
        root.edit_widget_with_tag(ROOT, |mut r| Root::set_dialog(&mut r, None));
        let back = self
            .return_focus
            .take()
            .filter(|id| root.get_widget(*id).is_some());
        let doc = root.get_widget_with_tag(DOC).map(|w| w.id());
        root.focus_on(back.or(doc));
    }

    fn answer(&mut self, ctx: &mut DriverCtx<'_>, cmd: Command) {
        // One answer per dialog: Escape reaches both the field and the
        // dialog, and only the first counts.
        if self.dialog.is_none() {
            return;
        }
        // Commands the window runs itself, chosen in the command palette or
        // in the list menu: Settings and Colors open their dialogs; the
        // text size and font keys. They join the recent commands too.
        let own = match (&cmd, &self.dialog) {
            (Command::Answer(id), Some(OpenDialog::Palette(_))) => {
                ActionId::from_id(id).filter(|a| self.is_window_command(*a))
            }
            (Command::Choose(i), Some(OpenDialog::List(_))) => self
                .menu_row_command(*i)
                .filter(|a| self.is_window_command(*a)),
            _ => None,
        };
        self.close_dialog(ctx);
        if let Some(a) = own {
            self.muted.set(true);
            let effects = self.app.dispatch(Command::Cancel);
            self.muted.set(false);
            self.run_effects(ctx, effects);
            self.dispatch(ctx, Command::RunCommand(a));
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
        self.menu_dirty = true;
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
        // Exploring a formula (Explore Math): the arrows, Home, End, Space,
        // Enter, and Escape move through it, as in the terminal; any other
        // key leaves it and does what it usually does.
        if self.app.math_exploring() && !self.app.confirmation_pending() {
            if let Some(mv) = keys::math_move(&chord) {
                if self.log {
                    crate::log::line(&format!("math key {chord} -> {mv:?}"));
                }
                self.dispatch(ctx, Command::MathStep(mv));
                return;
            }
            self.app.stop_math_exploring();
        }
        let layer = self.app.mode().layer();
        let action = (!keys::is_native(&chord, Platform::current()))
            .then(|| self.app.keymap().lookup(&chord, layer))
            .flatten();
        if self.log {
            crate::log::line(&format!("key {chord} -> {action:?}"));
        }
        // A key being described (Shift+F1) is the app's, even the menu
        // key, so it is described rather than entering the menus.
        let describing = action.is_some() && self.app.describing_next_key();
        if !describing && self.native_menu_key(ctx, &chord, action) {
            return;
        }
        if let Some(a) = action {
            self.dispatch(ctx, Command::Action(a));
        } else if let Some(cmd) = extra_command(self.app.keymap(), &chord, layer) {
            // The extra keys obey the single-key switch, as the keymap does.
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
        // The window was created, shown, and drawn before the first tick.
        startup_phase(self.log, "window shown, first tick");
        if self.background {
            // Before anything is opened or pressed: a window that UI
            // Automation brings to the front gives the foreground back (W7x).
            let hwnd = self.window_handle(ctx);
            let guarded = crate::background::guard(hwnd, self.log);
            if self.log && cfg!(windows) {
                crate::log::line(&format!(
                    "background: guarded (WS_EX_NOACTIVATE, foreground given back): {}",
                    if guarded { "yes" } else { "no" }
                ));
            }
        }
        let mut effects = Vec::new();
        // On a first run the welcome comes first, before the empty-window
        // hint, so "Welcome to textweaver" is the first thing heard.
        let mut welcomed = false;
        match &file {
            Some(path) => {
                let started = Instant::now();
                match self.app.open(path) {
                    Ok(e) => effects.extend(e),
                    Err(e) => {
                        let said = startup_open_message(&self.app.catalog(), path, &e);
                        self.app
                            .announce_as(&said, Priority::Assertive, Importance::Error);
                    }
                }
                if self.log {
                    crate::log::line(&format!(
                        "opened in {:.1} ms",
                        started.elapsed().as_secs_f64() * 1000.0
                    ));
                }
                startup_phase(self.log, "document open");
            }
            None => {
                // The key from the keymap, written for the screen reader
                // ("Ctrl+O") and spoken for textweaver's voice.
                let open = textweaver_app::named_key(self.app.keymap(), ActionId::Open);
                let said = self
                    .app
                    .catalog()
                    .fmt("gui-no-document", &args!["key" => open.as_str()]);
                if self.first_run && self.app.interface_allows(Importance::Tip) {
                    let welcome = setup::welcome_text(&self.app.catalog(), self.app.keymap());
                    self.app
                        .announce_as(&welcome, Priority::Polite, Importance::Tip);
                    self.app.announce_queued(&said, Priority::Polite);
                    welcomed = true;
                } else {
                    self.app
                        .announce_as(&said, Priority::Polite, Importance::Tip);
                }
            }
        }
        // As in the terminal reader: startup messages follow the opening
        // message instead of cutting it off (a settings or keymap warning
        // stays assertive, so it is heard even when reading starts at once).
        // They are warnings (a setting or key that could not be used), which
        // interface announcements never silence.
        for m in &messages {
            self.app.announce_queued(m, Priority::Assertive);
        }
        if self.first_run {
            // The first run: the welcome (the five keys that get a new user
            // reading), then the language list, the system's first. The
            // welcome is a tip: heard unless announcements are turned down.
            let welcome = setup::welcome_text(&self.app.catalog(), self.app.keymap());
            if !welcomed && self.app.interface_allows(Importance::Tip) {
                self.app.announce_queued(&welcome, Priority::Polite);
            }
            effects.extend(self.app.language_list());
            // Then, once nothing else is open, the optional components,
            // none chosen (W8a-d).
            self.app.offer_components_on_first_run();
        } else if self.startup_offers
            && self.app.hybrid_offer_due()
            && let Some(found) = textweaver_app::a11y::detect::detect()
        {
            // A screen reader is running and the mode was never chosen:
            // ask once whether to use hybrid mode (a yes-or-no dialog).
            let _ = self.app.offer_hybrid(&found);
        }
        // Unsaved work from an earlier run, one snapshot at a time.
        effects.extend(self.app.offer_recovery());
        self.run_effects(ctx, effects);
        self.refresh(ctx);
        let doc = ctx
            .render_root(self.window_id)
            .get_widget_with_tag(DOC)
            .map(|w| w.id());
        ctx.render_root(self.window_id).focus_on(doc);
        self.attach_menus(ctx);
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
            if crate::menus::is_alt_alone(&k) {
                // A hidden menu bar is shown while Alt is down, so Windows
                // enters it when Alt is let go (W8a-m).
                self.reveal_menu_bar();
            }
            self.on_key(ctx, &k);
        } else if let Some(DocAction::CaretMoved {
            caret,
            selection,
            echo,
        }) = action.downcast_ref::<DocAction>()
        {
            let (caret, selection) = (*caret, *selection);
            let echo = echo.clone();
            self.muted.set(true);
            let synced = sync_caret(&mut self.app, caret, selection);
            self.muted.set(false);
            if let Some(effects) = synced {
                if self.log {
                    crate::log::line(&format!("caret sync: {caret:?} {selection:?}"));
                }
                self.run_effects(ctx, effects);
                // The view already shows the caret and the selection; keep
                // the window in step, so a later change (a Cut) is sent.
                self.shown.state = state_for(&self.app);
                self.refresh(ctx);
            }
            if let Some(echo) = echo {
                self.echo_caret(echo);
            }
        } else if let Some(DocAction::WindowFocused) = action.downcast_ref::<DocAction>() {
            self.window_focused();
        } else if let Some(DocAction::TableCell { forward }) = action.downcast_ref::<DocAction>() {
            let a = if *forward {
                ActionId::NextTableCell
            } else {
                ActionId::PreviousTableCell
            };
            self.dispatch(ctx, Command::Action(a));
        } else if let Some(edit) = action.downcast_ref::<DocAction>() {
            // Edit mode: typing and deleting go through the app, which
            // keeps the undo history and echoes as the access mode says.
            let cmd = match edit.clone() {
                DocAction::Typed(text) => Command::Insert(text),
                DocAction::Delete { forward: true } => Command::DeleteForward,
                DocAction::Delete { forward: false } => Command::DeleteBack,
                DocAction::Replace { range, text } => Command::ReplaceRange { range, text },
                DocAction::CaretMoved { .. }
                | DocAction::TableCell { .. }
                | DocAction::WindowFocused => return,
            };
            if self.log {
                crate::log::line(&format!("edit: {cmd:?}"));
            }
            let effects = self.app.dispatch(cmd);
            self.run_effects(ctx, effects);
            self.refresh(ctx);
        } else if action.downcast_ref::<Pressed>().is_some() {
            if let Some(OpenDialog::Question { yes, no }) = &self.dialog {
                let answer = if *yes == widget_id {
                    Some(textweaver_app::Confirm::Yes)
                } else if *no == widget_id {
                    Some(textweaver_app::Confirm::No)
                } else {
                    None
                };
                if let Some(a) = answer {
                    self.answer_question(ctx, a);
                }
            } else if let Some(OpenDialog::Settings(open)) = &self.dialog
                && open.close == widget_id
            {
                self.settings_dialog_action(ctx, &DialogAction::Cancel);
            } else if let Some(OpenDialog::Settings(open)) = &self.dialog
                && open.reset == Some(widget_id)
            {
                self.reset_all_colors(ctx);
            } else if let Some(OpenDialog::Voices { buttons, .. }) = &self.dialog
                && let Some(b) = crate::voices::button_for(buttons, widget_id)
            {
                self.voice_button(ctx, b);
            } else if let Some(a) = self.buttons.by_id.get(&widget_id).copied() {
                self.dispatch(ctx, Command::Action(a));
            }
        } else if let Some(a) = action.downcast_ref::<FormAction>() {
            let a = a.clone();
            self.settings_form_action(ctx, a);
        } else if let Some(d) = action.downcast_ref::<DialogAction>() {
            if matches!(self.dialog, Some(OpenDialog::Question { .. })) {
                match d {
                    DialogAction::Answer(a) => self.answer_question(ctx, *a),
                    DialogAction::Cancel => {
                        self.answer_question(ctx, textweaver_app::Confirm::No);
                    }
                    _ => {}
                }
                return;
            }
            if self.settings_dialog_action(ctx, d) {
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
                    self.sync_voice_remove(ctx);
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
                                self.app.announce_as(
                                    &said,
                                    Priority::Assertive,
                                    Importance::Answer,
                                );
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
                DialogAction::Browse => {
                    if matches!(self.dialog, Some(OpenDialog::Prompt)) {
                        self.prompt_key(ctx, PromptKey::Browse);
                    }
                    return;
                }
                DialogAction::Field(next) => {
                    if matches!(self.dialog, Some(OpenDialog::Prompt)) {
                        let key = if *next {
                            PromptKey::Tab
                        } else {
                            PromptKey::BackTab
                        };
                        self.prompt_key(ctx, key);
                    }
                    return;
                }
                DialogAction::ShowMatches => {
                    // The palette's matches, in context: the focus moves
                    // to the list, whose selected row the screen reader
                    // reads with its place ("3 of 12").
                    if matches!(self.dialog, Some(OpenDialog::Palette(_))) {
                        let root = ctx.render_root(self.window_id);
                        let list = root.get_widget_with_tag(LIST).map(|w| w.id());
                        root.focus_on(list);
                        if self.log {
                            crate::log::line("palette: the list of matches");
                        }
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
                    | DialogAction::Browse
                    | DialogAction::Field(_)
                    | DialogAction::Chord(_)
                    | DialogAction::Answer(_)
                    | DialogAction::ShowMatches,
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
                    // In the palette, Enter runs the match Up and Down
                    // moved to and announced (the first after typing).
                    let answer = match &self.dialog {
                        Some(OpenDialog::Palette(ids)) => {
                            let selected = ctx
                                .render_root(self.window_id)
                                .get_widget_with_tag(LIST)
                                .map_or(0, |w| w.inner().selected());
                            palette_answer(ids, selected, text)
                        }
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
            self.menu_dirty = true;
            self.file_chosen(ctx, chosen);
            return;
        }
        if let Some(crate::menus::MenuPicked(id)) =
            action.downcast_ref::<crate::menus::MenuPicked>()
        {
            let id = id.clone();
            self.menu_picked(ctx, &id);
            return;
        }
        if action.downcast_ref::<Tick>().is_none() {
            return;
        }
        self.wake_pending.store(false, Ordering::Release);
        if self.background {
            crate::background::note_foreground();
        }
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
        let mut wait = self.app.tick_interval(Instant::now());
        if self.spell_seen.is_some() {
            // Come back when the pause is over, to mark the misspellings.
            wait = wait.min(SPELL_PAUSE / 2);
        }
        let holding = ctx
            .render_root(self.window_id)
            .get_widget_with_tag(ANNOUNCER)
            .is_some_and(|a| a.inner().holding());
        if holding {
            // Messages wait for a screen reader to ask for the tree: come
            // back soon, to say them once it has.
            wait = wait.min(HOLD_TICK);
        }
        if self.native.as_ref().is_some_and(|n| n.waiting()) {
            // A menu bar a key showed hides once its menu closes.
            wait = wait.min(MENU_TICK);
        }
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
        // Bundled fonts, and a Lexend downloaded in an earlier session.
        let mut blobs = crate::fonts::bundled_blobs();
        blobs.extend(crate::fonts::downloaded_blobs(
            self.app.fonts_folder().as_deref(),
        ));
        self.font_downloads = self.app.font_downloads();
        for root in state.roots() {
            for blob in &blobs {
                let _ = root.register_fonts(blob.clone());
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

/// When `run` began, for the startup phases `--log` writes.
static RUN_STARTED: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// Under `--log`, writes how long after `run` began a startup phase ended
/// (the performance audit's startup breakdown): "startup: settings and
/// app at 41.2 ms".
fn startup_phase(log: bool, phase: &str) {
    let started = *RUN_STARTED.get_or_init(Instant::now);
    if log {
        crate::log::line(&format!(
            "startup: {phase} at {:.1} ms",
            started.elapsed().as_secs_f64() * 1000.0
        ));
    }
}

/// Runs the GUI until the window closes.
pub fn run(opts: GuiOptions) -> Result<(), String> {
    let _ = RUN_STARTED.set(Instant::now());
    let queue: MessageQueue = Rc::new(RefCell::new(VecDeque::new()));
    let muted = Rc::new(Cell::new(false));
    let announcer = QueueAnnouncer {
        queue: Rc::clone(&queue),
        muted: Rc::clone(&muted),
        log: opts.log,
    };
    // Automated runs (`--background`) skip the first run's welcome and
    // language list, which would stand in front of what they check.
    let first_run = !opts.background && setup::is_first_run(&opts.app);
    let (mut app, mut messages) = setup::build_app(&opts.app, Box::new(announcer));
    startup_phase(opts.log, "settings and app");
    app.set_announce_list_focus(opts.experiments.app_list_announcements);
    let mut experiments = opts.experiments;
    let wanted = experiments
        .announce
        .unwrap_or_else(|| setup::announce_setting(app.settings()));
    let announce = wanted.effective();
    if announce != wanted {
        messages.push(app.catalog().tr("gui-uia-unavailable"));
    }
    experiments.announce = Some(announce);
    if opts.log {
        crate::log::line(&format!("announce: {}", announce.name()));
    }
    if opts.theme.is_none() {
        // The system's light, dark, or high-contrast setting, when the
        // settings ask to follow it (`display.follow_os_theme`).
        let _ = app.apply_startup_theme(textweaver_app::theme::os::probe());
        startup_phase(opts.log, "color-scheme probe");
    }
    // Windows High Contrast: the system's own colors, while the settings
    // follow the system (`display.follow_os_theme`) and no `--theme` was
    // given.
    let system = (opts.theme.is_none() && app.settings().display.follow_os_theme)
        .then(crate::system_colors::high_contrast)
        .flatten();
    let palette = match (&opts.theme, &system) {
        (Some(name), _) => Palette::named(name),
        (None, Some(c)) => crate::system_colors::palette(c),
        (None, None) => Palette::from_theme(&app.reading_theme()),
    };
    let theme_key = app.reading_theme_key();
    let font = crate::fonts::doc_font(&app.settings().reading_aids.font);
    let full_passes = Rc::new(Cell::new(0));
    let tree = build_tree(&palette, font, Some(&app), full_passes, experiments);
    startup_phase(opts.log, "widget tree");

    let mut attrs = WinitWindow::default_attributes()
        .with_title("textweaver")
        .with_inner_size(LogicalSize::new(1100.0, 780.0))
        .with_min_inner_size(LogicalSize::new(420.0, 320.0));
    // The title bar follows the palette from the start (W8a-m).
    let chrome = crate::dark_mode::for_palette(&palette);
    attrs = attrs.with_theme(chrome.window_theme());
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
    startup_phase(opts.log, "event loop");
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
        first_run,
        startup_offers: !opts.background,
        background: opts.background,
        exit_at: opts.exit_after.map(|d| Instant::now() + d),
        wake_pending: Arc::new(AtomicBool::new(false)),
        tick_ms: Arc::new(AtomicU64::new(
            u64::try_from(FIRST_TICK.as_millis()).unwrap_or(250),
        )),
        ticker: Some(proxy.clone()),
        proxy,
        typed_open: false,
        char_keys: None,
        lang: String::new(),
        spell_marked: None,
        spell_seen: None,
        fixed_theme: opts.theme.is_some(),
        theme_key,
        system,
        settings_list: experiments.settings_list,
        list_menus: experiments.list_menus,
        announce,
        held_notices: Vec::new(),
        return_focus: None,
        hwnd: 0,
        native: None,
        menu_dirty: false,
        menu_key: None,
        chrome,
        font_choices: None,
        font_downloads: 0,
        colors_after_question: None,
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
pub(crate) fn hwnd_of(window: &WinitWindow) -> isize {
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

/// What Enter answers in the command palette: the match at `selected`,
/// the one Up and Down moved to and announced (the first after typing,
/// since a new filter selects the first). With no matches, the typed
/// text, which the app answers as it answers any unknown command.
pub fn palette_answer(ids: &[ActionId], selected: usize, typed: &str) -> String {
    ids.get(selected)
        .or_else(|| ids.first())
        .map_or_else(|| typed.to_owned(), |a| a.id().to_owned())
}

/// The command the window runs itself for `cmd`, if any, and whether it
/// came from a menu. While Help, "What does this key do?" waits for a key
/// (`describing`), a key's command goes to the app, which describes it
/// instead of the window running it; a menu pick still runs, as in the
/// terminal.
pub fn window_command_of(cmd: &Command, describing: bool) -> Option<(ActionId, bool)> {
    match cmd {
        Command::Action(_) if describing => None,
        Command::Action(a) => Some((*a, false)),
        Command::RunCommand(a) => Some((*a, true)),
        _ => None,
    }
}

/// The message when the document named on the command line cannot be
/// opened: the same words as Ctrl+O's ("Could not open x.md: there is no
/// file named x.md in Notes. Check the name."), and the plain error only
/// when the failure was in saving state, not in the file.
pub fn startup_open_message(c: &Catalog, path: &std::path::Path, err: &AppError) -> String {
    match err {
        AppError::Load(e) => textweaver_app::open_failure_message_in(c, path, e),
        AppError::Store(e) => c.fmt(
            "gui-open-failed",
            &args![
                "name" => path.file_name().map_or_else(
                    || path.display().to_string(),
                    |n| n.to_string_lossy().into_owned()
                ),
                "error" => e.to_string()
            ],
        ),
    }
}

/// Refreshes a test harness or screenshot host from `app`.
pub fn refresh_for_tests(app: &App, host: &mut impl Host) {
    let mut shown = Shown::default();
    let _ = refresh_host(app, &mut shown, host, false);
}

/// Keeps a test harness in step with an app across refreshes, as the
/// window does: the document window slides or recenters, and only what
/// changed is sent.
#[derive(Default)]
pub struct Refresher {
    shown: Shown,
}

impl Refresher {
    /// The view moved its caret or selection, as the window takes it
    /// ([`sync_caret`]): the app follows, and the view is known to show it.
    pub fn caret_moved(
        &mut self,
        app: &mut App,
        caret: textweaver_app::core::CharPos,
        selection: Option<CharRange>,
    ) {
        let _ = sync_caret(app, caret, selection);
        self.shown.state = state_for(app);
    }

    /// Brings `host` up to date with `app`. Returns the document window's
    /// range when the view's text was replaced or slid.
    pub fn refresh(&mut self, app: &App, host: &mut impl Host) -> Option<CharRange> {
        refresh_host(app, &mut self.shown, host, false)?;
        self.shown.window.map(|w| w.range())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Enter in the palette runs the match Up and Down announced, not the
    /// first (walkthroughs QW1).
    #[test]
    fn palette_enter_runs_the_selected_match() {
        let ids = [ActionId::Open, ActionId::Settings, ActionId::ChooseFont];
        assert_eq!(palette_answer(&ids, 2, "f"), ActionId::ChooseFont.id());
        assert_eq!(palette_answer(&ids, 0, "f"), ActionId::Open.id());
        // A stale index falls back to the first match.
        assert_eq!(palette_answer(&ids, 9, "f"), ActionId::Open.id());
        // No match: the typed text goes to the app.
        assert_eq!(palette_answer(&[], 0, "zzz"), "zzz");
    }

    /// While Shift+F1 waits for a key, the window runs none of its own
    /// commands from a key; a menu pick still runs (walkthroughs QW4).
    #[test]
    fn a_described_key_is_not_run_by_the_window() {
        let key = Command::Action(ActionId::ChooseFont);
        let menu = Command::RunCommand(ActionId::ChooseFont);
        assert_eq!(
            window_command_of(&key, false),
            Some((ActionId::ChooseFont, false))
        );
        assert_eq!(window_command_of(&key, true), None);
        assert_eq!(
            window_command_of(&menu, true),
            Some((ActionId::ChooseFont, true))
        );
        assert_eq!(window_command_of(&Command::Cancel, false), None);
    }
}
