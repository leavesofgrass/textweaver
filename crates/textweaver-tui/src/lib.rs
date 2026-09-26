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
pub mod theme;
pub mod ui;
pub mod widgets;

use std::path::Path;
use std::time::Duration;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, DisableBracketedPaste, EnableBracketedPaste};
use ratatui::crossterm::execute;
use textweaver_app::a11y::Priority;

pub use setup::{Options, build_app, build_app_with};
pub use theme::{Theme, theme_help};
pub use ui::{Tui, chord};

/// Runs the event loop until the user quits: draw, apply speech status and
/// housekeeping, wait briefly for input.
pub fn run(terminal: &mut DefaultTerminal, tui: &mut Tui) -> anyhow::Result<()> {
    while !tui.should_quit() {
        terminal.draw(|f| tui.draw(f))?;
        tui.tick();
        // Apple's AVSpeechSynthesizer delivers audio and words through the
        // main thread's run loop (ADR-0008); a no-op on other platforms.
        textweaver_app::apple::pump_main_loop(Duration::ZERO);
        // Wake in time for the next RSVP word, else every 40 ms.
        let idle = Duration::from_millis(40);
        let wait = tui
            .app()
            .rsvp_wait(std::time::Instant::now())
            .map_or(idle, |w| w.min(idle));
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
    let (app, messages) = build_app(opts);
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
    let mut terminal = ratatui::init();
    // Pasted text arrives as one event (one undo step), not as keystrokes.
    let _ = execute!(std::io::stdout(), EnableBracketedPaste);
    let result = run(&mut terminal, &mut tui);
    let _ = execute!(std::io::stdout(), DisableBracketedPaste);
    ratatui::restore();
    tui.app_mut().shutdown();
    result
}
