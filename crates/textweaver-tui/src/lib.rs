//! The textweaver terminal frontend: a self-voicing ratatui reader.
//!
//! Screen, top to bottom: a title line (document, reading state, rate,
//! backend, line and percentage, mode); the document viewport over a window
//! slice of the text; the status line, which shows every announcement and is
//! the app's `StatusLineAnnouncer`; and a line of key hints that becomes the
//! minibuffer when a prompt is open. Lists (keyboard help, bookmarks, help)
//! appear over the document.
//!
//! Accessibility: the hardware cursor always sits where attention is (the
//! prompt caret, the focused list item, the Speech Cursor line, the spoken
//! word while reading, else the caret), so screen readers and magnifiers
//! follow it; every state change is announced through the app; highlights
//! never rely on color alone; speech is self-voiced through the speech
//! service unless `--no-speech` is given.
//!
//! Edit mode (Ctrl+E) shows the document's source: bound chords (the Edit
//! layer, then Global) run their actions, and every other key types,
//! deletes, or moves the caret (arrows, Ctrl+arrows by word, Home, End,
//! Page keys, Shift to select), with echo through the app. Pasted text
//! (bracketed paste) is one undo step. In lists, Enter chooses, Delete
//! deletes the item (bookmarks, notes, highlights), and F2 renames or edits
//! it. Keys for notes and highlights come from
//! `textweaver_app::extra_bindings` until the keymap has actions for them.
//!
//! Reading aids (ADR-0022): RSVP shows one word at a time in a box over the
//! document (Alt+Shift+R; Alt+Shift+P plays), bionic reading bolds the
//! start of each word (Alt+Shift+B), the reading ruler marks the current
//! line or a band with a gutter mark and underline (Alt+Shift+U), and
//! `[reading_aids.spacing]` adds blank rows and wider word spaces.
//!
//! Owner: Agent D.

pub mod layout;
pub mod setup;
pub mod signals;
pub mod theme;
pub mod ui;
pub mod widgets;

use std::path::Path;
use std::time::{Duration, Instant};

use ratatui::backend::Backend;
use ratatui::crossterm::event::{self, DisableBracketedPaste, EnableBracketedPaste};
use ratatui::crossterm::execute;
use ratatui::{DefaultTerminal, Terminal};
use textweaver_app::a11y::Priority;

pub use setup::{Options, build_app, build_app_with};
pub use theme::{Theme, theme_help};
pub use ui::{Tui, chord};

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
/// (docs/audit-2026-09.md, finding R3).
pub fn frame<B: Backend>(terminal: &mut Terminal<B>, tui: &mut Tui) -> Result<Duration, B::Error> {
    tui.tick();
    // Apple's AVSpeechSynthesizer delivers audio and words through the
    // main thread's run loop (ADR-0008); a no-op on other platforms.
    textweaver_app::apple::pump_main_loop(Duration::ZERO);
    terminal.draw(|f| tui.draw(f))?;
    Ok(input_wait(tui.app(), Instant::now()))
}

/// Runs the event loop until the user quits or a signal asks it to stop
/// ([`signals`]): apply speech status and housekeeping, draw, wait briefly
/// for input.
pub fn run(terminal: &mut DefaultTerminal, tui: &mut Tui) -> anyhow::Result<()> {
    while !tui.should_quit() && !signals::requested() {
        let wait = frame(terminal, tui)?;
        if event::poll(wait)? {
            let ev = event::read()?;
            tui.handle_event(&ev);
        }
    }
    Ok(())
}

/// The whole terminal reader: builds the app from `opts`, opens `file`,
/// offers unsaved work from a previous run, runs until the user quits, and
/// saves on the way out. Used by the `textweaver` binary and `tw open`.
pub fn launch(opts: &Options, file: Option<&Path>) -> anyhow::Result<()> {
    let log_message = setup::start_log(opts);
    let (app, mut messages) = build_app(opts);
    messages.extend(log_message);
    let mut tui = Tui::new(app);
    match file {
        Some(file) => {
            if let Err(e) = tui.app_mut().open(file) {
                let msg = format!("Could not open {}: {e}", file.display());
                tui.app_mut().announce(&msg, Priority::Assertive);
            }
        }
        None => tui.app_mut().announce(
            "No document is open. Press Control O to open one, Control N for a new one, or F1 for help.",
            Priority::Polite,
        ),
    }
    for m in messages {
        tui.app_mut().announce(&m, Priority::Assertive);
    }
    tui.offer_recovery();
    if let Some(msg) = signals::install() {
        log::warn!("{msg}");
    }
    let mut terminal = ratatui::init();
    // Pasted text arrives as one event (one undo step), not as keystrokes.
    let _ = execute!(std::io::stdout(), EnableBracketedPaste);
    install_panic_hook();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run(&mut terminal, &mut tui)
    }));
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
