//! The reader window: a menu bar whose items carry the keymap's accelerators,
//! a read-only multi-line text control holding the document, Play/Pause and
//! Stop buttons, a status bar, and a hidden live-region label for
//! announcements.
//!
//! State lives in one [`Gui`] behind `Rc<RefCell<..>>`, shared by the event
//! handlers. Handlers never hold the borrow across a modal dialog (a dialog
//! runs a nested event loop in which the speech timer keeps firing), and the
//! timer skips a tick when the state is busy.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use textweaver_app::a11y::Priority;
use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::keymap::{ActionId, KeyChord, Layer, Platform};
use textweaver_app::store::DocKey;
use textweaver_app::text::GoTo;
use textweaver_app::{App, Command, Effect, Playback, PromptPurpose};
use wxdragon::prelude::*;
use wxdragon::timer::Timer;

use crate::announce::LiveRegionAnnouncer;
use crate::keys::{self, Mods};
use crate::positions::Units;
use crate::setup::{self, Options};

/// How often the GUI drains speech status (and moves the caret with it).
const SPEECH_POLL_MS: i32 = 30;

/// Highlight moves slower than this are logged (large documents).
const SLOW_HIGHLIGHT_MS: u128 = 30;

/// Delay before opening the document, so the window is up and focused.
const STARTUP_DELAY_MS: i32 = 100;

/// First menu id; menu item `n` of [`MENU_ACTIONS`] (flattened) has id
/// `MENU_BASE + n`.
const MENU_BASE: i32 = ID_HIGHEST + 1;

/// The menu bar: titles and the actions under them, in order.
const MENUS: &[(&str, &[ActionId])] = &[
    ("&File", &[ActionId::Open, ActionId::Quit]),
    (
        "&Reading",
        &[
            ActionId::PlayPause,
            ActionId::Stop,
            ActionId::ReadFromCursor,
            ActionId::ReplaySentence,
            ActionId::ReadCurrentSentence,
            ActionId::ReadCurrentWord,
            ActionId::SayPosition,
        ],
    ),
    (
        "&Navigate",
        &[
            ActionId::NextSentence,
            ActionId::PreviousSentence,
            ActionId::NextParagraph,
            ActionId::PreviousParagraph,
            ActionId::NextHeading,
            ActionId::PreviousHeading,
            ActionId::HistoryBack,
            ActionId::HistoryForward,
            ActionId::GoTo,
            ActionId::Find,
            ActionId::FindNext,
        ],
    ),
    (
        "&Voice",
        &[
            ActionId::RateUp,
            ActionId::RateDown,
            ActionId::CycleSpeedPreset,
        ],
    ),
    ("&Help", &[ActionId::KeyboardHelp, ActionId::CommandPalette]),
];

/// How the document control gets its accessible name (a spike experiment;
/// see ADR-0014).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NameMode {
    /// A visible "Document" label placed just before the control, the
    /// Win32 convention screen readers and UI Automation use.
    #[default]
    Label,
    /// wxWidgets' accessible name (`set_accessibility_label`), which on
    /// Windows installs a `wxAccessible` (MSAA) object on the control.
    Accessible,
    /// Both.
    Both,
}

/// Everything the window needs to start.
#[derive(Debug, Default)]
pub struct GuiOptions {
    /// App options (speech, home).
    pub app: Options,
    /// Document to open.
    pub file: Option<PathBuf>,
    /// Start reading from the cursor once the document is open.
    pub read_on_start: bool,
    /// Close the window after this long (automated checks).
    pub exit_after: Option<Duration>,
    /// Log announcements and timings to standard error.
    pub log: bool,
    /// How the document control is named.
    pub name_mode: NameMode,
    /// Automated runs: no taskbar button, and the window is shown minimized
    /// without being activated, so it cannot take the foreground or a
    /// screen reader's focus from the person at the machine. Its controls
    /// stay in the UI Automation tree.
    pub background: bool,
}

/// The widgets the GUI updates. wxDragon widgets are copyable handles.
#[derive(Clone, Copy)]
struct Widgets {
    frame: Frame,
    text: TextCtrl,
    play: Button,
}

/// The GUI state shared by the event handlers.
struct Gui {
    app: App,
    w: Widgets,
    units: Units,
    muted: Rc<Cell<bool>>,
    log: bool,
    /// The document in the control: its key and length.
    loaded: Option<(DocKey, usize)>,
    /// The app cursor last shown in the control.
    last_cursor: Option<CharPos>,
    /// The spoken range last shown in the control.
    last_spoken: Option<CharRange>,
    /// The control position the GUI last put the caret at; a different
    /// position means the user moved it.
    last_set: Option<i64>,
    last_reading: bool,
    last_status: (String, String),
    closed: bool,
    _poll_timer: Option<Timer<Frame>>,
    _start_timer: Option<Timer<Frame>>,
    _exit_timer: Option<Timer<Frame>>,
}

type Shared = Rc<RefCell<Gui>>;

/// Builds and shows the window. Called from `wxdragon::main`.
pub fn build(opts: GuiOptions) {
    // Automated runs keep out of the way (see `GuiOptions::background`).
    let style = if opts.background {
        FrameStyle::Default | FrameStyle::NoTaskbar
    } else {
        FrameStyle::Default
    };
    let frame = Frame::builder()
        .with_title("textweaver")
        .with_size(Size::new(900, 700))
        .with_style(style)
        .build();
    let panel = Panel::builder(&frame).build();
    let sizer = BoxSizer::builder(Orientation::Vertical).build();

    // Announcements: a hidden, zero-size label (Paperback's arrangement).
    let live = StaticText::builder(&panel)
        .with_label("")
        .with_size(Size::new(0, 0))
        .build();
    live.show(false);
    let live_ok = live_region::set_live_region(&live);

    // The document: its label first, so it names the control (Win32 labels
    // an unnamed control by the static text before it in the tab order).
    if opts.name_mode != NameMode::Accessible {
        let label = StaticText::builder(&panel).with_label("&Document").build();
        sizer.add(
            &label,
            0,
            SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
            8,
        );
    }
    let text = TextCtrl::builder(&panel)
        .with_style(
            TextCtrlStyle::MultiLine
                | TextCtrlStyle::ReadOnly
                | TextCtrlStyle::Rich2
                | TextCtrlStyle::WordWrap
                | TextCtrlStyle::NoHideSel,
        )
        .build();
    if opts.name_mode != NameMode::Label {
        text.set_accessibility_label("Document");
    }
    sizer.add(&text, 1, SizerFlag::Expand | SizerFlag::All, 8);

    let buttons = BoxSizer::builder(Orientation::Horizontal).build();
    let play = Button::builder(&panel).with_label("&Play").build();
    let stop = Button::builder(&panel).with_label("&Stop").build();
    buttons.add(&play, 0, SizerFlag::Right, 8);
    buttons.add(&stop, 0, SizerFlag::Right, 8);
    sizer.add_sizer(&buttons, 0, SizerFlag::Left | SizerFlag::Bottom, 8);
    panel.set_sizer(sizer, true);

    // -1 is wxID_ANY (wxDragon exports it as i64).
    let status = frame.create_status_bar(2, 0, -1, "statusBar");
    status.set_status_widths(&[-3, -1]);

    let muted = Rc::new(Cell::new(false));
    let announcer = LiveRegionAnnouncer::new(live, Rc::clone(&muted), opts.log);
    let (app, mut messages) = setup::build_app(&opts.app, Box::new(announcer));
    if !live_ok {
        messages.push("Screen reader announcements are not available on this system.".into());
    }
    frame.set_menu_bar(menu_bar(&app));

    let gui = Rc::new(RefCell::new(Gui {
        app,
        w: Widgets { frame, text, play },
        units: Units::NATIVE,
        muted,
        log: opts.log,
        loaded: None,
        last_cursor: None,
        last_spoken: None,
        last_set: None,
        last_reading: false,
        last_status: (String::new(), String::new()),
        closed: false,
        _poll_timer: None,
        _start_timer: None,
        _exit_timer: None,
    }));

    bind_events(&gui, frame, text, play, stop);
    start_timers(&gui, frame, opts.exit_after);

    if opts.background {
        // Minimized before it is first shown: wxWidgets then shows it with
        // SW_MINIMIZE and skips bringing it to the top, so it is never
        // activated.
        frame.iconize(true);
        frame.show(true);
    } else {
        frame.show(true);
        frame.centre();
        text.set_focus();
    }

    // Open the document and speak the startup messages once the event loop
    // runs, so the screen reader has focused the window first. (A one-shot
    // timer: `wxdragon::call_after` needs a `Send` closure.)
    let file = opts.file;
    let read_on_start = opts.read_on_start;
    let st = Rc::clone(&gui);
    let start = Timer::new(&frame);
    start.on_tick(move |_| {
        let effects = {
            let Ok(mut g) = st.try_borrow_mut() else {
                return;
            };
            let mut effects = Vec::new();
            match &file {
                Some(path) => match g.app.open(path) {
                    Ok(e) => effects.extend(e),
                    Err(e) => {
                        let msg = format!("Could not open {}: {e}", path.display());
                        g.app.announce(&msg, Priority::Assertive);
                    }
                },
                None => g.app.announce(
                    "No document is open. Press Control O to open one.",
                    Priority::Polite,
                ),
            }
            for m in &messages {
                g.app.announce(m, Priority::Assertive);
            }
            g.refresh();
            if read_on_start && g.app.session().is_some() {
                effects.extend(g.app.dispatch(Command::Action(ActionId::ReadFromCursor)));
                g.refresh();
            }
            effects
        };
        run_effects(&st, effects);
    });
    start.start(STARTUP_DELAY_MS, true);
    gui.borrow_mut()._start_timer = Some(start);
}

/// The menu bar, with each item's first accelerator-safe chord after a tab
/// (wxWidgets turns it into an accelerator) and a document-only key, if
/// that is all the action has, in parentheses.
fn menu_bar(app: &App) -> MenuBar {
    let platform = Platform::current();
    let mut bar = MenuBar::builder();
    let mut n = 0;
    for (title, actions) in MENUS {
        let mut menu = Menu::builder();
        for &action in *actions {
            let chords = app.keymap().chords_in_mode(action, Layer::Browse);
            let accel = chords
                .iter()
                .filter(|c| keys::is_accelerator(c))
                .find_map(|c| keys::accelerator_text(c, platform));
            let mut label = menu_label(action);
            match accel {
                Some(a) => {
                    label.push('\t');
                    label.push_str(&a);
                }
                None => {
                    if let Some(c) = chords.iter().find(|c| !keys::is_native(c)) {
                        label.push_str(&format!(" ({})", c.spoken()));
                    }
                }
            }
            menu = menu.append_item(MENU_BASE + n, &label, action.help());
            n += 1;
        }
        bar = bar.append(menu.build(), title);
    }
    bar.build()
}

/// The menu item text for `action`.
fn menu_label(action: ActionId) -> String {
    match action {
        ActionId::Open => "&Open...".to_owned(),
        ActionId::Quit => "E&xit".to_owned(),
        ActionId::PlayPause => "&Play/Pause".to_owned(),
        ActionId::GoTo => "&Go to...".to_owned(),
        ActionId::Find => "&Find...".to_owned(),
        ActionId::CommandPalette => "&Command palette...".to_owned(),
        ActionId::KeyboardHelp => "&Keyboard shortcuts".to_owned(),
        _ => {
            let name = action.palette_name();
            let mut chars = name.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().chain(chars).collect(),
                None => name,
            }
        }
    }
}

/// The action for a menu id.
fn action_for_menu_id(id: i32) -> Option<ActionId> {
    let n = usize::try_from(id - MENU_BASE).ok()?;
    MENUS.iter().flat_map(|(_, a)| a.iter()).nth(n).copied()
}

fn bind_events(gui: &Shared, frame: Frame, text: TextCtrl, play: Button, stop: Button) {
    let st = Rc::clone(gui);
    frame.on_menu(move |ev| {
        if let Some(a) = action_for_menu_id(ev.get_id()) {
            dispatch(&st, Command::Action(a));
        }
    });

    let st = Rc::clone(gui);
    play.on_click(move |_| dispatch(&st, Command::Action(ActionId::PlayPause)));
    let st = Rc::clone(gui);
    stop.on_click(move |_| dispatch(&st, Command::Action(ActionId::Stop)));

    let platform = Platform::current();
    let st = Rc::clone(gui);
    text.on_key_down(move |ev| {
        if let WindowEventData::Keyboard(k) = &ev {
            let Some(code) = k.get_key_code() else {
                return;
            };
            if let Some(chord) = keys::chord_from_key_down(code, mods(k), platform) {
                handle_chord(&st, &chord, &ev);
            }
        }
    });
    let st = Rc::clone(gui);
    text.on_char(move |ev| {
        if let WindowEventData::Keyboard(k) = &ev {
            let Some(unicode) = k.get_unicode_key() else {
                return;
            };
            if let Some(chord) = keys::chord_from_char(unicode, mods(k), platform) {
                handle_chord(&st, &chord, &ev);
            }
        }
    });

    let st = Rc::clone(gui);
    frame.on_close(move |ev| {
        if let Ok(mut g) = st.try_borrow_mut()
            && !g.closed
        {
            g.closed = true;
            if let Some(t) = &g._poll_timer {
                t.stop();
            }
            g.app.shutdown();
        }
        ev.skip(true);
    });
}

fn mods(k: &wxdragon::event::window_events::KeyboardEvent) -> Mods {
    Mods {
        ctrl: k.control_down(),
        alt: k.alt_down(),
        shift: k.shift_down(),
        meta: k.meta_down(),
    }
}

/// Runs the action bound to `chord` in the current mode, consuming the key;
/// leaves caret keys and unbound keys to the control.
fn handle_chord(st: &Shared, chord: &KeyChord, ev: &WindowEventData) {
    let native = keys::is_native(chord);
    let action = {
        let Ok(g) = st.try_borrow() else {
            return;
        };
        let action = (!native)
            .then(|| g.app.keymap().lookup(chord, g.app.mode().layer()))
            .flatten();
        if g.log {
            let note = if native { " (native)" } else { "" };
            crate::log::line(&format!("key {chord} -> {action:?}{note}"));
        }
        action
    };
    if let Some(a) = action {
        ev.skip(false);
        dispatch(st, Command::Action(a));
    }
}

fn start_timers(gui: &Shared, frame: Frame, exit_after: Option<Duration>) {
    let poll = Timer::new(&frame);
    let st = Rc::clone(gui);
    poll.on_tick(move |_| {
        let effects = {
            let Ok(mut g) = st.try_borrow_mut() else {
                return;
            };
            if g.closed {
                return;
            }
            let e = g.app.poll_speech();
            g.refresh();
            e
        };
        run_effects(&st, effects);
    });
    poll.start(SPEECH_POLL_MS, false);

    let exit = exit_after.map(|d| {
        let t = Timer::new(&frame);
        t.on_tick(move |_| frame.close(true));
        t.start(i32::try_from(d.as_millis()).unwrap_or(i32::MAX), true);
        t
    });
    let mut g = gui.borrow_mut();
    g._poll_timer = Some(poll);
    g._exit_timer = exit;
}

/// Dispatches `cmd` (after moving the app's cursor to the native caret) and
/// carries out the effects.
fn dispatch(st: &Shared, cmd: Command) {
    let effects = {
        let Ok(mut g) = st.try_borrow_mut() else {
            return;
        };
        if g.log {
            crate::log::line(&format!("command {cmd:?}"));
        }
        g.sync_caret();
        let e = g.app.dispatch(cmd);
        g.refresh();
        e
    };
    run_effects(st, effects);
}

/// Carries out effects: prompts and lists become native dialogs whose
/// answers are dispatched in turn.
fn run_effects(st: &Shared, effects: Vec<Effect>) {
    let mut queue: VecDeque<Effect> = effects.into();
    while let Some(effect) = queue.pop_front() {
        let Some(frame) = st.try_borrow().ok().map(|g| g.w.frame) else {
            return;
        };
        let cmd = match effect {
            Effect::Redraw => {
                if let Ok(mut g) = st.try_borrow_mut() {
                    g.refresh();
                }
                continue;
            }
            Effect::Quit => {
                frame.close(true);
                return;
            }
            Effect::Prompt { label, purpose } => {
                ask(&frame, &label, purpose).map_or(Command::Cancel, Command::Answer)
            }
            Effect::ShowList { title, items } => {
                choose(&frame, &title, &items).map_or(Command::Cancel, Command::Choose)
            }
        };
        let Ok(mut g) = st.try_borrow_mut() else {
            return;
        };
        let more = g.app.dispatch(cmd);
        g.refresh();
        queue.extend(more);
    }
}

/// Asks for a prompt's answer: a file dialog to open a document, a text
/// entry dialog otherwise.
fn ask(frame: &Frame, label: &str, purpose: PromptPurpose) -> Option<String> {
    if purpose == PromptPurpose::Open {
        let dialog = FileDialog::builder(frame)
            .with_message("Open document")
            .with_wildcard(
                "Documents (*.txt;*.md;*.markdown;*.html;*.htm;*.epub;*.docx)|*.txt;*.md;*.markdown;*.html;*.htm;*.epub;*.docx|All files (*.*)|*.*",
            )
            .with_style(FileDialogStyle::Open | FileDialogStyle::FileMustExist)
            .build();
        return (dialog.show_modal() == ID_OK)
            .then(|| dialog.get_path())
            .flatten();
    }
    let dialog = TextEntryDialog::builder(frame, label, label).build();
    (dialog.show_modal() == ID_OK)
        .then(|| dialog.get_value())
        .flatten()
}

/// Shows a list and returns the chosen index.
fn choose(frame: &Frame, title: &str, items: &[String]) -> Option<usize> {
    let choices: Vec<&str> = items.iter().map(String::as_str).collect();
    let dialog = SingleChoiceDialog::builder(frame, title, title, &choices).build();
    if dialog.show_modal() != ID_OK {
        return None;
    }
    usize::try_from(dialog.get_selection()).ok()
}

impl Gui {
    /// Moves the app's cursor to the native caret if the user moved it
    /// (arrow keys, mouse) since the GUI last placed it. Quiet: the screen
    /// reader already spoke the caret movement.
    ///
    /// Uses `GoTo(Char)`, which records history; a quiet cursor command is a
    /// contract change request (ADR-0014).
    fn sync_caret(&mut self) {
        if self.app.playback() == Playback::Reading {
            return;
        }
        let native = self.w.text.get_insertion_point();
        if self.last_set == Some(native) {
            return;
        }
        self.last_set = Some(native);
        let Some(s) = self.app.session() else {
            return;
        };
        let pos = self.units.to_doc(&s.doc, native);
        if pos == s.cursor {
            return;
        }
        if self.log {
            crate::log::line(&format!("caret sync: control {native} -> {pos:?}"));
        }
        self.muted.set(true);
        let _ = self.app.dispatch(Command::GoTo(GoTo::Char(pos)));
        self.muted.set(false);
        self.last_cursor = self.app.session().map(|s| s.cursor);
    }

    /// Brings the window up to date with the app: the document text, the
    /// caret (on the spoken word while reading, else on the cursor), the
    /// status bar, the title, and the Play/Pause button.
    fn refresh(&mut self) {
        let reading = self.app.playback() == Playback::Reading;
        let Gui {
            app,
            w,
            units,
            log,
            loaded,
            last_cursor,
            last_spoken,
            last_set,
            last_reading,
            last_status,
            ..
        } = self;
        let text = w.text;
        let position = match app.session() {
            None => {
                if loaded.take().is_some() {
                    text.set_value("");
                    w.frame.set_title("textweaver");
                }
                String::new()
            }
            Some(s) => {
                let id = (s.key.clone(), s.doc.len_chars());
                if loaded.as_ref() != Some(&id) {
                    let started = Instant::now();
                    let content = s.doc.text().to_string();
                    text.freeze();
                    text.set_value(&content);
                    text.thaw();
                    let set_ms = started.elapsed().as_millis();
                    let expected = units.len(&s.doc);
                    let actual = text.get_last_position();
                    if *log || actual != expected {
                        crate::log::line(&format!(
                            "loaded {} chars ({expected} control units) in {set_ms} ms; control reports {actual}",
                            s.doc.len_chars()
                        ));
                    }
                    w.frame.set_title(&format!("{} - textweaver", s.title));
                    *loaded = Some(id);
                    *last_cursor = None;
                    *last_spoken = None;
                }
                if reading {
                    if let Some(r) = s.spoken
                        && *last_spoken != Some(r)
                    {
                        let started = Instant::now();
                        let a = units.to_ctrl(&s.doc, r.start);
                        let b = units.to_ctrl(&s.doc, r.end);
                        text.set_selection(a, b);
                        text.show_position(a);
                        let ms = started.elapsed().as_millis();
                        if *log && ms >= SLOW_HIGHLIGHT_MS {
                            crate::log::line(&format!(
                                "slow highlight: {ms} ms at control position {a}"
                            ));
                        }
                        *last_spoken = Some(r);
                        *last_set = Some(text.get_insertion_point());
                    }
                } else if *last_cursor != Some(s.cursor) || *last_reading {
                    let p = units.to_ctrl(&s.doc, s.cursor);
                    text.set_insertion_point(p);
                    text.show_position(p);
                    *last_set = Some(text.get_insertion_point());
                    *last_spoken = None;
                }
                *last_cursor = Some(s.cursor);
                format!("Line {}, {}%", s.line() + 1, s.percent())
            }
        };
        if *last_reading != reading {
            w.play.set_label(if reading { "&Pause" } else { "&Play" });
            *last_reading = reading;
        }
        let status = app.status_text();
        if last_status.0 != status {
            w.frame.set_status_text(status, 0);
            last_status.0 = status.to_owned();
        }
        if last_status.1 != position {
            w.frame.set_status_text(&position, 1);
            last_status.1 = position;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_ids_round_trip() {
        let all: Vec<ActionId> = MENUS.iter().flat_map(|(_, a)| a.iter().copied()).collect();
        for (n, a) in all.iter().enumerate() {
            let id = MENU_BASE + i32::try_from(n).unwrap();
            assert_eq!(action_for_menu_id(id), Some(*a));
        }
        assert_eq!(action_for_menu_id(MENU_BASE - 1), None);
        assert_eq!(action_for_menu_id(MENU_BASE + all.len() as i32), None);
    }

    #[test]
    fn menu_labels_read_well() {
        assert_eq!(menu_label(ActionId::NextSentence), "Next sentence");
        assert_eq!(menu_label(ActionId::PlayPause), "&Play/Pause");
    }
}
