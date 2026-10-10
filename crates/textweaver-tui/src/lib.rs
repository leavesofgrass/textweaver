//! The textweaver terminal frontend: a self-voicing ratatui reader.
//!
//! Screen, top to bottom: a title line (document, then line and
//! percentage, mode and "modified" outside browse mode, reading state,
//! access mode, rate, engine; the position first in screen-reader and
//! hybrid modes); the document viewport over a window slice of the text;
//! the status area, which shows every announcement in full and is the
//! app's `StatusLineAnnouncer`; and a line of key hints (`[display]
//! hints`) that becomes the minibuffer when a prompt is open. Lists
//! (keyboard help, bookmarks, help) appear over the document.
//!
//! Accessibility: the hardware cursor sits where attention is (the prompt
//! caret, the focused list item, the Speech Cursor line, the spoken word
//! while reading, else the caret), so screen readers and magnifiers follow
//! it, or on the status line with `[accessibility] cursor = "status"`;
//! every state change is announced through the app; highlights never rely
//! on color alone. The accessibility mode (`[accessibility] mode`,
//! `--mode`, Alt+Shift+A) decides what textweaver speaks and what it
//! leaves to a screen reader through the status line; `--no-speech` is
//! screen-reader mode without a voice. With `quiet_screen`, the title
//! line's position stays still while reading continuously. On the first
//! run with a screen reader, textweaver asks once whether to use hybrid
//! mode.
//!
//! Edit mode (Ctrl+E) shows the document's source: bound chords (the Edit
//! layer, then Global) run their actions, and every other key types,
//! deletes, or moves the caret (arrows, Ctrl+arrows by word, Home, End,
//! Page keys, Shift to select), with echo through the app. Pasted text
//! (bracketed paste) is one undo step. In lists, Enter chooses, Delete
//! deletes the item (bookmarks, notes, highlights), and F2 renames or edits
//! it. Most note and highlight keys are keymap
//! actions; `Shift+Y` (list highlights) comes from
//! `textweaver_app::extra_bindings`.
//!
//! Browse keys follow NVDA's and JAWS's quick navigation. The digit row
//! (`1` to `6`, heading levels) is matched by the physical key
//! ([`physical`]): on Windows the event loop peeks at the console's input
//! records before crossterm reads them. While math exploration is on
//! (Alt+Shift+X), the arrows, Home, End, Space, Enter, and Escape move
//! through the formula.
//!
//! Reading aids (ADR-0022): RSVP shows one word at a time in a box over the
//! document (Alt+Shift+R; Alt+Shift+P plays), bionic reading bolds the
//! start of each word (Alt+Shift+B), the reading ruler marks the current
//! line or a band with a gutter mark and underline (Alt+Shift+U), and
//! `[reading_aids.spacing]` adds blank rows and wider word spaces.
//!
//! Owner: Agent D.

pub mod bidi;
pub mod clipboard;
#[cfg(feature = "highlight")]
pub mod highlight;
pub mod layout;
pub mod paths;
pub mod physical;
pub mod setup;
pub mod signals;
pub mod terminal_info;
pub mod theme;
pub mod ui;
pub mod widgets;

use std::path::Path;
use std::time::{Duration, Instant};

use ratatui::backend::Backend;
use ratatui::crossterm::event::{self, DisableBracketedPaste, EnableBracketedPaste};
use ratatui::crossterm::execute;
use ratatui::crossterm::style::Print;
use ratatui::crossterm::terminal::SetTitle;

/// Pushes the window title onto the terminal's title stack (xterm, CSI 22;2 t).
const TITLE_PUSH: &str = "\x1b[22;2t";
/// Pops it back (CSI 23;2 t).
const TITLE_POP: &str = "\x1b[23;2t";
use ratatui::{DefaultTerminal, Terminal};
use textweaver_app::a11y::Priority;
use textweaver_app::lexicon::args;

pub use setup::{Options, build_app, build_app_with};
pub use textweaver_app::a11y::AccessMode;
pub use theme::{Theme, theme_help};
pub use ui::{REDRAW_AT_LEAST, Tui, chord, key_event};

/// How long the loop waits for a key while reading: a highlight step is
/// drawn at most this long after its word is heard.
pub const READING_POLL: Duration = Duration::from_millis(10);

/// How long the loop waits for a key otherwise.
pub const IDLE_POLL: Duration = Duration::from_millis(40);

/// How long the event loop may wait for input before it must apply speech
/// status and draw again: [`READING_POLL`] while reading, else
/// [`IDLE_POLL`], and never past the next RSVP word.
pub fn input_wait(app: &textweaver_app::App, now: Instant) -> Duration {
    let base = if app.playback() == textweaver_app::Playback::Reading {
        READING_POLL
    } else {
        IDLE_POLL
    };
    app.rsvp_wait(now).map_or(base, |w| w.min(base))
}

/// One pass of the event loop without the input: apply speech status and
/// housekeeping, then draw. Returns how long to wait for input next.
///
/// Status is applied before drawing, so a highlight step is on screen (and
/// the hardware cursor, which screen readers and magnifiers follow, is on
/// its word) as soon as it arrives, not after the next wait for a key
/// (the September 2026 audit, finding R3).
///
/// It draws only when something could have changed (W6u; the loop drew
/// every pass, 25 times a second while idle and 100 while reading): a key
/// or command, speech status or a background job, a resize, a change in
/// what the screen shows ([`Tui::view_signature`]), a blanked status
/// message coming back, RSVP playing, and at least once a second.
pub fn frame<B: Backend>(terminal: &mut Terminal<B>, tui: &mut Tui) -> Result<Duration, B::Error> {
    let changed = tui.tick();
    // Apple's AVSpeechSynthesizer delivers audio and words through the
    // main thread's run loop (ADR-0008); a no-op on other platforms.
    textweaver_app::apple::pump_main_loop(Duration::ZERO);
    let now = Instant::now();
    if tui.wants_draw(changed, now) {
        terminal.draw(|f| tui.draw(f))?;
    }
    let now = Instant::now();
    Ok(input_wait(tui.app(), now).min(tui.draw_due_in(now).max(Duration::from_millis(1))))
}

/// Runs the event loop until the user quits or a signal asks it to stop
/// ([`signals`]): apply speech status and housekeeping, draw, wait briefly
/// for input.
pub fn run(terminal: &mut DefaultTerminal, tui: &mut Tui) -> anyhow::Result<()> {
    while !tui.should_quit() && !signals::requested() {
        let wait = frame(terminal, tui)?;
        // The window's title names the document: Alt+Tab and the screen
        // reader's title key then say it. Written once per open.
        if let Some(title) = tui.take_window_title() {
            let _ = execute!(std::io::stdout(), SetTitle(title));
        }
        // On Windows the loop waits on the console itself, so it can peek
        // at the physical keys before crossterm reads them (the digit row
        // on any layout; see `physical`).
        let ready = match physical::peek_console(wait) {
            Some((ready, keys)) => {
                tui.digit_keys_mut().set_pending(keys);
                ready && event::poll(Duration::ZERO)?
            }
            None => event::poll(wait)?,
        };
        if ready {
            let ev = event::read()?;
            tui.handle_event(&ev);
            if let Some(seq) = tui.take_clipboard_sequence() {
                use std::io::Write as _;
                let mut out = std::io::stdout();
                let _ = out.write_all(seq.as_bytes());
                let _ = out.flush();
            }
        }
    }
    Ok(())
}

/// The whole terminal reader: builds the app from `opts`, opens `file`,
/// offers unsaved work from a previous run, runs until the user quits, and
/// saves on the way out. Used by `tw` with no command (and `textweaver`,
/// its second name), and by `tw open`.
pub fn launch(opts: &Options, file: Option<&Path>) -> anyhow::Result<()> {
    let log_message = setup::start_log(opts);
    let (app, mut messages) = build_app(opts);
    messages.extend(log_message);
    let mut tui = Tui::new(app);
    // On a first run the welcome comes first, before the empty-screen hint,
    // so "Welcome to textweaver" is the first thing heard.
    // The first run (W9b-f): the system's language, when built in, is put
    // into effect quietly first, so the welcome is said in it.
    let first_run =
        setup::first_run_message(&tui.app().catalog(), opts, tui.app().keymap()).is_some();
    let language_due = first_run && tui.app_mut().first_run_language(None);
    let catalog = tui.app().catalog();
    let welcome = setup::first_run_message(&catalog, opts, tui.app().keymap());
    let mut welcomed = false;
    match file {
        Some(file) => {
            if let Err(e) = tui.app_mut().open(file) {
                let msg = match e {
                    textweaver_app::AppError::Load(e) => {
                        textweaver_app::open_failure_message_in(&tui.app().catalog(), file, &e)
                    }
                    other => tui.app().catalog().fmt(
                        "tui-could-not-open",
                        &args![
                            "name" => file.display().to_string(),
                            "error" => other.to_string()
                        ],
                    ),
                };
                tui.app_mut().announce(&msg, Priority::Assertive);
            }
        }
        None => {
            let msg = setup::no_document_text(&tui.app().catalog(), tui.app().keymap());
            if let Some(welcome) = &welcome {
                tui.app_mut().announce(welcome, Priority::Polite);
                tui.app_mut().announce_queued(&msg, Priority::Polite);
                welcomed = true;
            } else {
                tui.app_mut().announce(&msg, Priority::Polite);
            }
        }
    }
    // Startup messages follow the opening message instead of cutting it
    // off: each is queued after the one before, and the status line shows
    // them together. A settings or keymap warning stays assertive, so it is
    // spoken even when the document starts reading at once.
    for m in messages {
        tui.app_mut().announce_queued(&m, Priority::Assertive);
    }
    if let Some(welcome) = welcome.filter(|_| !welcomed) {
        tui.app_mut().announce_queued(&welcome, Priority::Polite);
    }
    // A screen reader, looked for only when the mode was never chosen.
    let screen_reader = setup::screen_reader_to_infer(tui.app(), opts);
    if first_run {
        // At most three skippable steps, one dialog at a time: the
        // language list only when the system's language is not built in,
        // hybrid mode inferred and said with a screen reader, then the
        // optional components, none chosen (W8a-d). `tw` never shows them.
        tui.first_run_steps(language_due, screen_reader.as_ref());
    } else {
        // With a screen reader and no mode chosen: hybrid mode, said once.
        tui.app_mut()
            .startup_screen_reader_step(screen_reader.as_ref());
    }
    tui.offer_recovery();
    if let Some(msg) = signals::install() {
        log::warn!("{msg}");
    }
    let mut terminal = ratatui::init();
    // Pasted text arrives as one event (one undo step), not as keystrokes.
    let _ = execute!(std::io::stdout(), EnableBracketedPaste);
    // Save the window's title (xterm's title stack, CSI 22 t), so it comes
    // back on the way out; terminals without the stack ignore both.
    let _ = execute!(std::io::stdout(), Print(TITLE_PUSH));
    install_panic_hook();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run(&mut terminal, &mut tui)
    }));
    let _ = execute!(std::io::stdout(), Print(TITLE_POP));
    let _ = execute!(std::io::stdout(), DisableBracketedPaste);
    ratatui::restore();
    let result = finish(&mut tui, outcome);
    signals::finished();
    result
}

/// What happens after the event loop ends, however it ended (the terminal
/// is already restored): a normal quit saves as usual; a stop asked for by
/// a signal, or a panic, saves without asking anything, writing unsaved
/// edits to the recovery snapshot. A panic becomes an error that says the
/// work was kept.
pub fn finish(
    tui: &mut Tui,
    outcome: std::thread::Result<anyhow::Result<()>>,
) -> anyhow::Result<()> {
    match outcome {
        Ok(result) => {
            if signals::requested() && !tui.should_quit() {
                log::warn!("stopping on a signal");
                save_after_trouble(tui);
            } else {
                tui.app_mut().shutdown();
            }
            result
        }
        Err(panic) => {
            save_after_trouble(tui);
            let what = panic_text(panic.as_ref());
            log::error!("the terminal reader stopped after an internal error: {what}");
            Err(anyhow::anyhow!(
                "textweaver stopped after an internal error ({what}). Your place was saved, and unsaved edits will be offered for recovery next time."
            ))
        }
    }
}

/// Saves what can be saved after a panic or a signal: the recovery
/// snapshot, the position, and the settings. A second panic while saving
/// is caught and logged.
fn save_after_trouble(tui: &mut Tui) {
    let saved = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tui.app_mut().emergency_save();
    }));
    if saved.is_err() {
        log::error!("saving after an internal error failed as well");
    }
}

/// The message a panic carried.
fn panic_text(panic: &(dyn std::any::Any + Send)) -> String {
    panic
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "no message".to_owned())
}

/// Adds to the panic hook (after `ratatui::init`, whose hook restores the
/// terminal): bracketed paste is turned off first and the panic is logged,
/// so the shell is usable and the log says what happened.
fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(std::io::stdout(), DisableBracketedPaste);
        log::error!("panic: {info}");
        previous(info);
    }));
}
