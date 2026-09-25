//! `textweaver`: the self-voicing terminal reader.
//!
//! Owner: Agent D. Phase 0 shows the document, maps keys through the keymap,
//! and dispatches to the app; the full layout (title, viewport over a window
//! slice, status line, key hints, minibuffer, themes, palette, help) comes
//! in wave 1.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph, Wrap};
use textweaver_app::keymap::{Key, KeyChord, Modifiers};
use textweaver_app::{App, AppConfig, Command, Effect};

/// Read documents aloud in the terminal.
#[derive(Parser, Debug)]
#[command(name = "textweaver", version, about)]
struct Args {
    /// Document to open.
    file: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut app = App::new(AppConfig::for_tests());
    if let Some(f) = &args.file {
        app.open(f)?;
    }
    let terminal = ratatui::init();
    let result = run(terminal, &mut app);
    ratatui::restore();
    result
}

/// Converts a crossterm key event into a keymap chord.
fn chord(k: &KeyEvent) -> Option<KeyChord> {
    let key = match k.code {
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::F(n) => Key::F(n),
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Escape,
        KeyCode::Tab => Key::Tab,
        KeyCode::BackTab => {
            return Some(KeyChord::new(Key::Tab, Modifiers::SHIFT));
        }
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        KeyCode::Insert => Key::Insert,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        _ => return None,
    };
    let mut mods = Modifiers::empty();
    if k.modifiers.contains(KeyModifiers::CONTROL) {
        mods |= Modifiers::CTRL;
    }
    if k.modifiers.contains(KeyModifiers::ALT) {
        mods |= Modifiers::ALT;
    }
    if k.modifiers.contains(KeyModifiers::SHIFT) {
        mods |= Modifiers::SHIFT;
    }
    if k.modifiers.contains(KeyModifiers::SUPER) {
        mods |= Modifiers::META;
    }
    Some(KeyChord::new(key, mods))
}

fn run(mut terminal: DefaultTerminal, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| {
            let [body, status] =
                Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(f.area());
            let text = app
                .session()
                .map(|s| s.doc.text().to_string())
                .unwrap_or_else(|| "No document. Run: textweaver FILE".to_owned());
            f.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(Block::bordered().title("textweaver")),
                body,
            );
            let pos = app.session().map(|s| s.cursor.0).unwrap_or(0);
            f.render_widget(
                Line::from(format!("{:?}  char {pos}  q quits", app.mode())),
                status,
            );
        })?;
        app.poll_speech();
        if !event::poll(Duration::from_millis(50))? {
            continue;
        }
        if let Event::Key(k) = event::read()? {
            if k.kind != KeyEventKind::Press {
                continue;
            }
            let Some(c) = chord(&k) else { continue };
            let Some(action) = app.keymap().lookup(&c, app.mode().layer()) else {
                continue;
            };
            if app
                .dispatch(Command::Action(action))
                .contains(&Effect::Quit)
            {
                return Ok(());
            }
        }
    }
}
