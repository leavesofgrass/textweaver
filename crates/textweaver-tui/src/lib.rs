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
//! Owner: Agent D.

pub mod layout;
pub mod setup;
pub mod theme;
pub mod ui;
pub mod widgets;

use std::time::Duration;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event;

pub use setup::{Options, build_app};
pub use theme::Theme;
pub use ui::{Tui, chord};

/// Runs the event loop until the user quits: draw, apply speech status,
/// wait briefly for input.
pub fn run(terminal: &mut DefaultTerminal, tui: &mut Tui) -> anyhow::Result<()> {
    while !tui.should_quit() {
        terminal.draw(|f| tui.draw(f))?;
        tui.tick();
        if event::poll(Duration::from_millis(40))? {
            let ev = event::read()?;
            tui.handle_event(&ev);
        }
    }
    Ok(())
}
