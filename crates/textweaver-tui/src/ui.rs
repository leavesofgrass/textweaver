//! The terminal frontend: key handling and drawing over an [`App`].

use std::collections::HashMap;
use std::time::Instant;

use ratatui::Frame;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};
use textweaver_app::a11y::Priority;
use textweaver_app::core::{CharPos, CharRange, Direction, Unit};
use textweaver_app::keymap::{ActionId, Key, KeyChord, Modifiers};
use textweaver_app::text_util::line_count;
use textweaver_app::{
    App, CaretMove, Command, Confirm, Effect, Mode, Playback, PromptPurpose, chords_text,
    extra_lookup,
};

use crate::layout::{self, Cells, Row};
use crate::theme::Theme;
use crate::widgets::{ListView, Minibuffer};
use ratatui::style::Modifier;
use textweaver_app::aids::rsvp::{Area as RsvpArea, SegmentRole, TuiBoxOptions, tui_box};
use textweaver_app::aids::{RowMark, RulerMode, TermStyle as RulerStyle, ViewRow, ruler_rows};
use textweaver_theme::ColorSupport;

/// Converts a crossterm key event into a keymap chord.
pub fn chord(k: &KeyEvent) -> Option<KeyChord> {
    let key = match k.code {
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::Null => return Some(KeyChord::new(Key::Space, Modifiers::CTRL)),
        KeyCode::F(n) => Key::F(n),
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Escape,
        KeyCode::Tab => Key::Tab,
        KeyCode::BackTab => return Some(KeyChord::new(Key::Tab, Modifiers::SHIFT)),
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

/// The character a key press types, if it types one: a character key
/// without Control or Alt, or with both when the character is not an ASCII
/// letter or digit.
///
/// On Windows, AltGr (the right Alt key on German, French, Nordic, Polish,
/// and many other layouts) arrives from crossterm as Control plus Alt, so
/// `@ [ ] { } | ~` and characters such as `€` and `ą` came with both
/// modifiers and could not be typed. Such a key reaches this only when no
/// binding claims its chord, and a real Control+Alt shortcut is an ASCII
/// letter or digit.
pub fn typed_char(k: &KeyEvent) -> Option<char> {
    let KeyCode::Char(c) = k.code else {
        return None;
    };
    if c.is_control() {
        return None;
    }
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let alt = k.modifiers.contains(KeyModifiers::ALT);
    match (ctrl, alt) {
        (false, false) => Some(c),
        (true, true) if !c.is_ascii_alphanumeric() => Some(c),
        _ => None,
    }
}

/// Most recalled answers kept per prompt.
const PROMPT_HISTORY: usize = 50;

/// How long the status line stays blank before a repeated message comes
/// back, so a screen reader that speaks the status line when it changes
/// hears the message again ("No next heading." twice in a row).
pub const REPEAT_BLANK: std::time::Duration = std::time::Duration::from_millis(150);

/// The terminal frontend's state around the app.
pub struct Tui {
    app: App,
    minibuffer: Option<Minibuffer>,
    list: Option<ListView>,
    answers: HashMap<PromptPurpose, Vec<String>>,
    quit: bool,
    /// The terminal's color level, detected once.
    support: ColorSupport,
    /// The styles of the theme in effect, rebuilt only when it changes.
    theme: Theme,
    /// The status announcement last drawn: its sequence number and text.
    status_shown: (u64, String),
    /// While set, the status line is drawn blank until then (a repeated
    /// message).
    status_blank_until: Option<Instant>,
}

/// Screen areas of the last draw.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Areas {
    /// Title line.
    pub title: Rect,
    /// Document text.
    pub body: Rect,
    /// Status line (one to three rows).
    pub status: Rect,
    /// Key hints or the minibuffer.
    pub bottom: Rect,
}

impl Tui {
    /// Wraps an app.
    pub fn new(app: App) -> Self {
        Self::with_color_support(app, ColorSupport::detect())
    }

    /// Wraps an app, drawing at a given color level (tests; at run time
    /// [`ColorSupport::detect`] also honors `TEXTWEAVER_COLOR` and
    /// `NO_COLOR`).
    pub fn with_color_support(app: App, support: ColorSupport) -> Self {
        let theme = Theme::from_theme(app.current_theme(), support);
        Tui {
            app,
            minibuffer: None,
            list: None,
            answers: Default::default(),
            quit: false,
            support,
            theme,
            status_shown: (0, String::new()),
            status_blank_until: None,
        }
    }

    /// The app.
    pub fn app(&self) -> &App {
        &self.app
    }

    /// The app, mutably (for opening files and tests).
    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    /// True once the user has quit.
    pub fn should_quit(&self) -> bool {
        self.quit
    }

    /// The open prompt, if any.
    pub fn minibuffer(&self) -> Option<&Minibuffer> {
        self.minibuffer.as_ref()
    }

    /// The open list, if any.
    pub fn list(&self) -> Option<&ListView> {
        self.list.as_ref()
    }

    /// The styles of the theme in effect, cached by theme name and rebuilt
    /// only when the theme changes.
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Rebuilds the cached styles when the app's theme changed.
    fn refresh_theme(&mut self) {
        let current = self.app.current_theme();
        if current.meta.name != self.theme.name {
            self.theme = Theme::from_theme(current, self.support);
        }
    }

    /// Applies speech status and runs the app's housekeeping (autosave,
    /// periodic position saves); true when a redraw is due.
    pub fn tick(&mut self) -> bool {
        let redraw = !self.app.poll_speech().is_empty();
        let effects = self.app.tick(Instant::now());
        let more = !effects.is_empty();
        self.apply(effects);
        redraw || more
    }

    /// Offers unsaved work from a previous run (call once at startup).
    pub fn offer_recovery(&mut self) {
        let effects = self.app.offer_recovery();
        self.apply(effects);
    }

    /// Dispatches a command and acts on its effects.
    pub fn dispatch(&mut self, cmd: Command) {
        let effects = self.app.dispatch(cmd);
        self.apply(effects);
    }

    fn apply(&mut self, effects: Vec<Effect>) {
        for e in effects {
            match e {
                Effect::Redraw => {}
                Effect::Quit => self.quit = true,
                Effect::Prompt { label, purpose } => {
                    self.list = None;
                    self.minibuffer = Some(Minibuffer::new(label, purpose));
                }
                Effect::ShowList { title, items } => {
                    self.minibuffer = None;
                    // The same list again (after a delete): stay in place.
                    let keep = self
                        .list
                        .as_ref()
                        .filter(|l| l.title == title)
                        .map(|l| l.selected);
                    let mut view = ListView::new(title, items);
                    if let Some(i) = keep {
                        view.selected = i.min(view.items.len().saturating_sub(1));
                    }
                    // Say the focused item after the app's introduction
                    // ("Bookmarks, 3 items. ..."), without interrupting
                    // it: the first item was never heard unless the user
                    // pressed Up (docs/audit-2026-09.md, finding A4).
                    if let Some(item) = view.spoken_item() {
                        self.app.announce_queued(&item, Priority::Polite);
                    }
                    self.list = Some(view);
                }
            }
        }
    }

    /// Handles one terminal event.
    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::Key(k) if matches!(k.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
                self.handle_key(*k);
            }
            Event::Paste(text) => self.paste(text),
            _ => {}
        }
    }

    /// Pasted text: into the prompt when one is open, else into the
    /// document in edit mode.
    pub fn paste(&mut self, text: &str) {
        if let Some(mb) = self.minibuffer.as_mut() {
            for c in text.chars().filter(|c| !c.is_control()) {
                mb.insert(c);
            }
            self.app.echo(text);
            return;
        }
        if self.list.is_none() {
            let text = text.replace("\r\n", "\n").replace('\r', "\n");
            self.dispatch(Command::Insert(text));
        }
    }

    /// Handles one key press.
    pub fn handle_key(&mut self, k: KeyEvent) {
        if self.app.confirmation_pending() {
            let answer = match k.code {
                KeyCode::Esc => Confirm::No,
                KeyCode::Char(c)
                    if !k
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    Confirm::from_char(c)
                }
                // Modifier keys alone are not answers.
                KeyCode::Modifier(_) => return,
                _ => Confirm::Repeat,
            };
            self.dispatch(Command::Confirm(answer));
            return;
        }
        if self.list.is_some() {
            self.list_key(k);
        } else if self.minibuffer.is_some() {
            self.minibuffer_key(k);
        } else if self.app.mode() == Mode::Edit {
            self.edit_key(k);
        } else {
            self.browse_key(k);
        }
    }

    /// Edit mode: bound chords (the Edit layer, then Global) run their
    /// actions; everything else types, deletes, or moves the caret.
    fn edit_key(&mut self, k: KeyEvent) {
        let Some(c) = chord(&k) else { return };
        if let Some(action) = self.app.keymap().lookup(&c, Mode::Edit.layer()) {
            self.dispatch(Command::Action(action));
            return;
        }
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let alt = k.modifiers.contains(KeyModifiers::ALT);
        let extend = k.modifiers.contains(KeyModifiers::SHIFT);
        let caret = |by, direction| Command::MoveCaret {
            by,
            direction,
            extend,
        };
        let cmd = match k.code {
            KeyCode::Char(ch) if typed_char(&k).is_some() => Command::Insert(ch.to_string()),
            KeyCode::Enter if !ctrl && !alt => Command::Insert("\n".into()),
            KeyCode::Tab if !ctrl && !alt => Command::Insert("\t".into()),
            KeyCode::Backspace => Command::DeleteBack,
            KeyCode::Delete => Command::DeleteForward,
            KeyCode::Left if ctrl => caret(CaretMove::Word, Direction::Backward),
            KeyCode::Right if ctrl => caret(CaretMove::Word, Direction::Forward),
            KeyCode::Left => caret(CaretMove::Char, Direction::Backward),
            KeyCode::Right => caret(CaretMove::Char, Direction::Forward),
            KeyCode::Up => caret(CaretMove::Line, Direction::Backward),
            KeyCode::Down => caret(CaretMove::Line, Direction::Forward),
            KeyCode::Home if ctrl => caret(CaretMove::DocumentEdge, Direction::Backward),
            KeyCode::End if ctrl => caret(CaretMove::DocumentEdge, Direction::Forward),
            KeyCode::Home => caret(CaretMove::LineEdge, Direction::Backward),
            KeyCode::End => caret(CaretMove::LineEdge, Direction::Forward),
            KeyCode::PageUp => caret(CaretMove::Page, Direction::Backward),
            KeyCode::PageDown => caret(CaretMove::Page, Direction::Forward),
            _ => return,
        };
        self.dispatch(cmd);
    }

    fn browse_key(&mut self, k: KeyEvent) {
        let Some(c) = chord(&k) else { return };
        let layer = self.app.mode().layer();
        if let Some(action) = self.app.keymap().lookup(&c, layer) {
            self.dispatch(Command::Action(action));
            return;
        }
        // The extra single keys obey the single-key switch like the keymap.
        let allowed = self.app.keymap().character_keys() || !c.is_text_input();
        if allowed && let Some(cmd) = extra_lookup(&c, layer) {
            self.dispatch(cmd);
            return;
        }
        // Shift+arrows select when nothing else claims them.
        if c.mods == Modifiers::SHIFT {
            let sel = match c.key {
                Key::Right => Some((Unit::Word, Direction::Forward)),
                Key::Left => Some((Unit::Word, Direction::Backward)),
                Key::Down => Some((Unit::Line, Direction::Forward)),
                Key::Up => Some((Unit::Line, Direction::Backward)),
                _ => None,
            };
            if let Some((unit, dir)) = sel {
                self.dispatch(Command::ExtendSelection(unit, dir));
            }
        }
    }

    fn list_key(&mut self, k: KeyEvent) {
        let Some(list) = self.list.as_mut() else {
            return;
        };
        let page = 10;
        let moved = match k.code {
            KeyCode::Up | KeyCode::Char('k') => list.step(-1),
            KeyCode::Down | KeyCode::Char('j') => list.step(1),
            KeyCode::PageUp => list.step(-page),
            KeyCode::PageDown => list.step(page),
            KeyCode::Home => list.step(isize::MIN / 2),
            KeyCode::End => list.step(isize::MAX / 2),
            KeyCode::Enter => {
                let n = list.selected;
                self.list = None;
                self.dispatch(Command::Choose(n));
                return;
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Backspace => {
                self.list = None;
                self.dispatch(Command::Cancel);
                return;
            }
            KeyCode::Delete => {
                let n = list.selected;
                self.list_action(Command::DeleteItem(n));
                return;
            }
            KeyCode::F(2) => {
                let n = list.selected;
                self.list_action(Command::RenameItem(n));
                return;
            }
            _ => return,
        };
        let text = list.spoken_item().unwrap_or_default();
        if moved {
            self.app.announce(&text, Priority::Assertive);
        } else {
            let edge = if matches!(
                k.code,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::PageUp | KeyCode::Home
            ) {
                "Top of list."
            } else {
                "End of list."
            };
            self.app.announce(edge, Priority::Polite);
        }
    }

    /// Runs a command on a list item; the list stays open only if the app
    /// shows it again.
    fn list_action(&mut self, cmd: Command) {
        let effects = self.app.dispatch(cmd);
        let reshown = effects
            .iter()
            .any(|e| matches!(e, Effect::ShowList { .. } | Effect::Prompt { .. }));
        if !reshown {
            self.list = None;
        }
        self.apply(effects);
    }

    fn minibuffer_key(&mut self, k: KeyEvent) {
        let Some(mb) = self.minibuffer.as_mut() else {
            return;
        };
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let mut echo: Option<String> = None;
        match k.code {
            KeyCode::Enter => {
                let answer = mb.text();
                let purpose = mb.purpose;
                if !answer.trim().is_empty() {
                    let hist = self.answers.entry(purpose).or_default();
                    hist.retain(|a| a != &answer);
                    hist.push(answer.clone());
                    if hist.len() > PROMPT_HISTORY {
                        hist.remove(0);
                    }
                }
                self.minibuffer = None;
                self.dispatch(Command::Answer(answer));
                return;
            }
            KeyCode::Esc => {
                self.minibuffer = None;
                self.dispatch(Command::Cancel);
                return;
            }
            KeyCode::Char('g') if ctrl => {
                self.minibuffer = None;
                self.dispatch(Command::Cancel);
                return;
            }
            KeyCode::Char('a') if ctrl => mb.home(),
            KeyCode::Char('e') if ctrl => mb.end(),
            KeyCode::Char('u') if ctrl => echo = Some(mb.kill_to_start()),
            KeyCode::Char('k') if ctrl => echo = Some(mb.kill_to_end()),
            KeyCode::Char('w') if ctrl => echo = Some(mb.delete_word_back()),
            KeyCode::Char(c) if typed_char(&k).is_some() => {
                mb.insert(c);
                mb.candidate = None;
                echo = Some(c.to_string());
            }
            KeyCode::Backspace => echo = mb.backspace().map(|c| c.to_string()),
            KeyCode::Delete => echo = mb.delete().map(|c| c.to_string()),
            KeyCode::Left => {
                mb.left();
                echo = mb.char_at_caret().map(|c| c.to_string());
            }
            KeyCode::Right => {
                mb.right();
                echo = mb.char_at_caret().map(|c| c.to_string());
            }
            KeyCode::Home => mb.home(),
            KeyCode::End => mb.end(),
            KeyCode::Tab => {
                self.complete();
                return;
            }
            KeyCode::Up => {
                self.recall(-1);
                return;
            }
            KeyCode::Down => {
                self.recall(1);
                return;
            }
            _ => return,
        }
        if let Some(e) = echo.filter(|e| !e.is_empty()) {
            self.app.echo(&e);
        }
    }

    /// Up and Down: palette candidates, or earlier answers to this prompt.
    fn recall(&mut self, delta: isize) {
        let Some(mb) = self.minibuffer.as_mut() else {
            return;
        };
        if mb.purpose == PromptPurpose::CommandPalette {
            if mb.candidate.is_none() {
                mb.candidates = self.app.palette_candidates(&mb.text());
            }
            if mb.candidates.is_empty() {
                self.app.announce("No matching commands.", Priority::Polite);
                return;
            }
            let n = mb.candidates.len();
            let i = match mb.candidate {
                None if delta > 0 => 0,
                None => n - 1,
                Some(i) => (i as isize + delta).rem_euclid(n as isize) as usize,
            };
            mb.candidate = Some(i);
            let (action, desc) = mb.candidates[i].clone();
            mb.set_text(action.id());
            self.app.announce(&desc, Priority::Assertive);
            return;
        }
        let empty = Vec::new();
        let hist = self.answers.get(&mb.purpose).unwrap_or(&empty);
        if hist.is_empty() {
            self.app.announce("No earlier entries.", Priority::Polite);
            return;
        }
        let n = hist.len();
        let i = match (mb.history_index, delta < 0) {
            (None, true) => Some(n - 1),
            (None, false) => None,
            (Some(i), true) => Some(i.saturating_sub(1)),
            (Some(i), false) if i + 1 < n => Some(i + 1),
            (Some(_), false) => None,
        };
        mb.history_index = i;
        let text = i.map_or_else(String::new, |i| hist[i].clone());
        mb.set_text(&text);
        let spoken = if text.is_empty() {
            "blank".to_owned()
        } else {
            text
        };
        self.app.announce(&spoken, Priority::Assertive);
    }

    /// Tab in the command palette: complete to the longest common prefix of
    /// the matching command ids, and say what matches.
    fn complete(&mut self) {
        let Some(mb) = self.minibuffer.as_mut() else {
            return;
        };
        if mb.purpose != PromptPurpose::CommandPalette {
            return;
        }
        let cands = self.app.palette_candidates(&mb.text());
        match cands.as_slice() {
            [] => self.app.announce("No matching commands.", Priority::Polite),
            [(a, desc)] => {
                mb.set_text(a.id());
                let desc = desc.clone();
                self.app.announce(&desc, Priority::Assertive);
            }
            many => {
                let ids: Vec<&str> = many.iter().map(|(a, _)| a.id()).collect();
                let prefix = common_prefix(&ids);
                if prefix.len() > mb.text().len() && ids.iter().all(|i| i.starts_with(&prefix)) {
                    mb.set_text(&prefix);
                }
                let first: Vec<String> = ids.iter().take(5).map(|i| i.replace('_', " ")).collect();
                let msg = format!("{} matches: {}.", many.len(), first.join(", "));
                self.app.announce(&msg, Priority::Assertive);
            }
        }
    }

    /// What the status line shows: the pending question while one waits for
    /// a yes or no (so it stays in view whatever else is announced), else
    /// the latest announcement.
    pub fn status_line(&self) -> String {
        match self
            .app
            .pending_confirmation()
            .and_then(ActionId::confirmation_prompt)
        {
            Some(question) if self.app.status_text() != question => {
                format!("{question}  {}", self.app.status_text())
                    .trim_end()
                    .to_owned()
            }
            _ => self.app.status_text().to_owned(),
        }
    }

    /// The status line for this frame. A message announced again with the
    /// same text is drawn blank for [`REPEAT_BLANK`] first: terminal screen
    /// readers speak the status line only when it changes, so the second
    /// "No next heading." was silent (docs/audit-2026-09.md, finding A5).
    fn status_to_draw(&mut self, now: Instant) -> String {
        let seq = self.app.status().seq;
        let text = self.status_line();
        if let Some(until) = self.status_blank_until {
            if now < until {
                return String::new();
            }
            self.status_blank_until = None;
        } else if seq != self.status_shown.0 && text == self.status_shown.1 && !text.is_empty() {
            self.status_blank_until = Some(now + REPEAT_BLANK);
            self.status_shown.0 = seq;
            return String::new();
        }
        self.status_shown = (seq, text.clone());
        text
    }

    /// True while a repeated status message is blanked (the loop should
    /// draw again soon).
    pub fn status_blanked(&self) -> bool {
        self.status_blank_until.is_some()
    }

    /// Screen areas for a frame of `area`.
    pub fn areas(&self, area: Rect) -> Areas {
        let status_rows = {
            let w = usize::from(area.width.max(1));
            let len = Span::raw(self.status_line()).width();
            u16::try_from(len.div_ceil(w).clamp(1, 3)).unwrap_or(1)
        };
        let [title, body, status, bottom] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(status_rows),
            Constraint::Length(1),
        ])
        .areas(area);
        Areas {
            title,
            body,
            status,
            bottom,
        }
    }

    /// Columns for line numbers, if shown.
    fn number_width(&self) -> u16 {
        if !self.app.settings().display.show_line_numbers {
            return 0;
        }
        let lines = self.app.session().map_or(1, |s| line_count(&s.doc));
        u16::try_from(lines.to_string().len() + 1).unwrap_or(6)
    }

    /// Columns left of the text: line numbers, and one for the reading
    /// ruler's mark when the ruler is on.
    fn gutter_width(&self) -> u16 {
        self.number_width() + u16::from(self.app.ruler().mode != RulerMode::Off)
    }

    fn text_width(&self, body: Rect) -> u16 {
        let w = body.width.saturating_sub(self.gutter_width()).max(1);
        match self.app.settings().display.wrap_width {
            0 => w,
            ww => ww.min(w),
        }
    }

    /// Draws the whole screen and parks the hardware cursor.
    pub fn draw(&mut self, f: &mut Frame<'_>) {
        self.refresh_theme();
        let theme = self.theme.clone();
        let areas = self.areas(f.area());
        let text_width = self.text_width(areas.body);
        let text_height = self.layout_height(areas.body.height);
        let vp = self.app.viewport();
        if vp.width != text_width || vp.height != text_height {
            self.app.dispatch(Command::Resize {
                width: text_width,
                height: text_height,
            });
        }
        self.draw_title(f, areas.title, &theme);
        let cursor = self.draw_body(f, areas.body, &theme);
        self.draw_rsvp(f, areas.body, &theme, cursor.map(|p| p.y));
        let status = self.status_to_draw(Instant::now());
        f.render_widget(
            Paragraph::new(status)
                .wrap(ratatui::widgets::Wrap { trim: false })
                .style(theme.status),
            areas.status,
        );
        let cursor = self.draw_bottom(f, areas.bottom, &theme).or(cursor);
        let cursor = self.draw_list(f, areas.body, &theme).or(cursor);
        f.set_cursor_position(cursor.unwrap_or(Position::new(areas.body.x, areas.body.y)));
    }

    fn draw_title(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let app = &self.app;
        let title = app.session().map_or("no document", |s| s.title.as_str());
        let left = format!(" textweaver: {title}");
        let state = match app.playback() {
            Playback::Reading => "Reading",
            Playback::Paused { .. } => "Paused",
            Playback::Idle => "Stopped",
        };
        // Most important first; trailing parts are dropped when narrow.
        let mut parts = Vec::new();
        if app.mode() != Mode::Browse {
            parts.push(app.mode().name().to_owned());
        }
        if app.is_dirty() {
            parts.push("modified".to_owned());
        }
        parts.push(state.to_owned());
        if let Some(s) = app.session() {
            parts.push(format!(
                "line {} of {}, {}%",
                s.line() + 1,
                line_count(&s.doc),
                s.percent()
            ));
        }
        parts.push(format!("{} wpm", app.settings().speech.rate.wpm()));
        parts.push(app.backend_name().to_owned());
        let width = usize::from(area.width);
        let lw = Span::raw(&left).width();
        let mut right = format!("{} ", parts.join(", "));
        while parts.len() > 1 && lw + Span::raw(&right).width() + 2 > width {
            parts.pop();
            right = format!("{} ", parts.join(", "));
        }
        let rw = Span::raw(&right).width();
        let line = if lw + rw < width {
            Line::from(vec![
                Span::raw(left),
                Span::raw(" ".repeat(width - lw - rw)),
                Span::raw(right),
            ])
        } else {
            Line::from(format!("{left}  {right}"))
        };
        f.render_widget(Paragraph::new(line).style(theme.title), area);
    }

    /// How chars are measured: tab width and word spacing.
    fn cells(&self) -> Cells {
        Cells {
            tab: usize::from(self.app.settings().display.tab_width),
            word_extra: usize::from(self.app.terminal_spacing().extra_word_spaces),
        }
    }

    /// Document rows that fit in `height` screen rows, given the blank
    /// rows text spacing adds between lines.
    fn layout_height(&self, height: u16) -> u16 {
        let between = self.app.terminal_spacing().rows_between_lines;
        (height.saturating_add(between) / (1 + between)).max(1)
    }

    /// Draws the document window; returns where the hardware cursor
    /// belongs (the focus position) when it is visible.
    ///
    /// Reading aids drawn here: bionic reading (the start of each word in
    /// bold), the reading ruler and current line (a gutter mark plus
    /// underline, never colour alone; dim only for the opt-in mask), and
    /// terminal text spacing (blank rows between lines and paragraphs,
    /// wider spaces between words).
    fn draw_body(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) -> Option<Position> {
        f.render_widget(Block::new().style(theme.text), area);
        let Some(s) = self.app.session() else {
            let k = |a| chords_text(self.app.keymap(), a);
            let lines = vec![
                Line::from(""),
                Line::from(" No document is open."),
                Line::from(format!(" Open one: {}.", k(ActionId::Open))),
                Line::from(format!(" Help: {}.", k(ActionId::Help))),
                Line::from(format!(" Quit: {}.", k(ActionId::Quit))),
            ];
            f.render_widget(Paragraph::new(lines).style(theme.text), area);
            return Some(Position::new(area.x + 1, area.y + 1));
        };
        let doc = &s.doc;
        let settings = self.app.settings();
        let cells = self.cells();
        let spacing = self.app.terminal_spacing();
        let numbers = self.number_width();
        let ruler = self.app.ruler();
        let ruler_col = u16::from(ruler.mode != RulerMode::Off);
        let gutter = numbers + ruler_col;
        let width = usize::from(self.text_width(area));
        let height = usize::from(area.height);
        let rows_wanted = usize::from(self.layout_height(area.height));
        let margin =
            usize::from(settings.display.scroll_margin).min(rows_wanted.saturating_sub(1) / 2);
        let focus = self.app.focus();
        let top = self.app.viewport().top_line;
        let rows = layout::window_cells(doc, top, width, rows_wanted, cells, focus, margin);
        let window = match (rows.first(), rows.last()) {
            (Some(a), Some(b)) => CharRange::new(a.range.start, b.range.end.saturating_add(1)),
            _ => CharRange::empty(0),
        };
        let highlights = self.app.highlights(window);
        let bold = self.app.bionic_ranges(window);
        let view_rows: Vec<ViewRow> = rows
            .iter()
            .map(|r| ViewRow {
                range: r.range,
                line: r.line,
            })
            .collect();
        let marks = focus.map_or_else(
            || vec![RowMark::Normal; rows.len()],
            |p| ruler_rows(&view_rows, p, &ruler),
        );
        let mut lines: Vec<Line<'static>> = Vec::with_capacity(height);
        let mut cursor = None;
        for (i, row) in rows.iter().enumerate() {
            if lines.len() >= height {
                break;
            }
            let mark = RulerStyle::recommended(marks.get(i).copied().unwrap_or_default());
            let mut spans = Vec::new();
            if numbers > 0 {
                let label = if row.first {
                    format!("{:>w$} ", row.line + 1, w = usize::from(numbers) - 1)
                } else {
                    " ".repeat(usize::from(numbers))
                };
                spans.push(Span::styled(label, theme.gutter));
            }
            if ruler_col > 0 {
                let c = mark.gutter.unwrap_or(' ');
                spans.push(Span::styled(c.to_string(), theme.gutter));
            }
            let mut text = self.row_spans(doc, row, &highlights, &bold, theme, cells);
            let extra = ruler_modifier(mark);
            if !extra.is_empty() {
                for sp in &mut text {
                    sp.style = sp.style.add_modifier(extra);
                }
            }
            spans.extend(text);
            if let Some(fp) = focus.filter(|&p| row.holds(p))
                && cursor.is_none()
            {
                let col = layout::column_cells(doc, row, fp, cells).min(width.saturating_sub(1));
                let x = area.x + gutter + u16::try_from(col).unwrap_or(0);
                let y = area.y + u16::try_from(lines.len()).unwrap_or(0);
                cursor = Some(Position::new(x, y));
            }
            lines.push(Line::from(spans));
            // Text spacing: blank rows after each row, and more after a
            // paragraph's blank line.
            let mut blank = usize::from(spacing.rows_between_lines);
            if row.range.is_empty() && row.first && row.last {
                blank += usize::from(spacing.rows_between_paragraphs.saturating_sub(1));
            }
            for _ in 0..blank {
                if lines.len() < height {
                    lines.push(Line::from(""));
                }
            }
        }
        f.render_widget(Paragraph::new(lines).style(theme.text), area);
        cursor
    }

    /// Draws the RSVP word in a box over the document (never over the
    /// cursor's row). Only the glyphs change from word to word: the box
    /// keeps its place and size, so nothing flashes.
    fn draw_rsvp(&self, f: &mut Frame<'_>, body: Rect, theme: &Theme, avoid: Option<u16>) {
        let Some(rsvp) = self.app.rsvp() else {
            return;
        };
        let Some(frame) = rsvp.frame() else {
            return;
        };
        let settings = rsvp.settings();
        let opts = TuiBoxOptions {
            position: settings.position,
            width: 32,
            show_previous: settings.show_previous,
            show_next: settings.show_next,
            avoid_row: avoid,
        };
        let area = RsvpArea::new(body.x, body.y, body.width, body.height);
        let Some(bx) = tui_box(&frame, area, &opts) else {
            return;
        };
        let rect = Rect::new(bx.area.x, bx.area.y, bx.area.width, bx.area.height);
        f.render_widget(Clear, rect);
        f.render_widget(Block::new().style(theme.list), rect);
        for (y, segments) in &bx.rows {
            for seg in segments {
                let style = match seg.role {
                    SegmentRole::Word => theme.list.add_modifier(Modifier::BOLD),
                    SegmentRole::Pivot => theme
                        .list
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    SegmentRole::Context | SegmentRole::Clipped => theme.list,
                };
                let w = u16::try_from(Span::raw(&seg.text).width()).unwrap_or(0);
                let right = rect.x + rect.width;
                let w = w.min(right.saturating_sub(seg.col));
                if w == 0 {
                    continue;
                }
                f.render_widget(
                    Paragraph::new(Span::styled(seg.text.clone(), style)),
                    Rect::new(seg.col, *y, w, 1),
                );
            }
        }
    }

    fn row_spans(
        &self,
        doc: &textweaver_app::text::Document,
        row: &Row,
        highlights: &[textweaver_app::Highlight],
        bold: &[CharRange],
        theme: &Theme,
        cells: Cells,
    ) -> Vec<Span<'static>> {
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut run = String::new();
        let mut run_style: Option<Style> = None;
        let chars = doc.text().slice(row.range.to_range()).chars();
        let mut b = bold.partition_point(|r| r.end <= row.range.start);
        for (i, c) in chars.enumerate() {
            let pos = CharPos(row.range.start.0 + i);
            let mut style = highlights
                .iter()
                .filter(|h| h.range.contains(pos))
                .fold(theme.text, |st, h| st.patch(theme.highlight(h.kind)));
            while b < bold.len() && bold[b].end <= pos {
                b += 1;
            }
            if bold.get(b).is_some_and(|r| r.contains(pos)) {
                style = style.add_modifier(Modifier::BOLD);
            }
            if run_style != Some(style) {
                if let Some(st) = run_style {
                    spans.push(Span::styled(std::mem::take(&mut run), st));
                }
                run_style = Some(style);
            }
            run.push_str(&cells.text(c));
        }
        if let Some(st) = run_style {
            spans.push(Span::styled(run, st));
        }
        spans
    }

    /// The minibuffer or the key hints; returns the caret position when a
    /// prompt is open.
    fn draw_bottom(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) -> Option<Position> {
        if let Some(mb) = &self.minibuffer {
            let label = format!("{}: ", mb.label);
            let text = mb.text();
            let before: String = text.chars().take(mb.caret()).collect();
            let col = Span::raw(&label).width() + Span::raw(&before).width();
            f.render_widget(
                Paragraph::new(format!("{label}{text}")).style(theme.minibuffer),
                area,
            );
            let x = area.x
                + u16::try_from(col)
                    .unwrap_or(0)
                    .min(area.width.saturating_sub(1));
            return Some(Position::new(x, area.y));
        }
        f.render_widget(
            Paragraph::new(self.hints(area.width)).style(theme.hints),
            area,
        );
        None
    }

    /// Key hints for the current mode, from the keymap, fitted to `width`.
    pub fn hints(&self, width: u16) -> String {
        if self.app.confirmation_pending() {
            return " y yes  n or a no  Escape no".to_owned();
        }
        let rsvp_hints: &[(ActionId, &str)] = &[
            (ActionId::RsvpPlayPause, "play"),
            (ActionId::NextSentence, "sentence"),
            (ActionId::RsvpFaster, "faster"),
            (ActionId::RsvpSlower, "slower"),
            (ActionId::RsvpToggle, "close RSVP"),
            (ActionId::Quit, "quit"),
        ];
        let hints: &[(ActionId, &str)] = match self.app.mode() {
            _ if self.app.rsvp().is_some() => rsvp_hints,
            Mode::Edit => &[
                (ActionId::Save, "save"),
                (ActionId::ToggleEditMode, "finish"),
                (ActionId::Undo, "undo"),
                (ActionId::Bold, "bold"),
                (ActionId::Heading, "heading"),
                (ActionId::KeyboardHelp, "keys"),
                (ActionId::Quit, "quit"),
            ],
            Mode::SpeechCursor => &[
                (ActionId::SpeechCursorNextLine, "next line"),
                (ActionId::SpeechCursorPreviousLine, "previous line"),
                (ActionId::SpeechCursorRereadLine, "again"),
                (ActionId::SpeechCursorExitAndRead, "read on"),
                (ActionId::SpeechCursorToggle, "leave"),
            ],
            _ => &[
                (ActionId::PlayPause, "play"),
                (ActionId::NextSentence, "sentence"),
                (ActionId::NextParagraph, "paragraph"),
                (ActionId::SkipNextHeading, "heading"),
                (ActionId::Find, "find"),
                (ActionId::AddBookmark, "mark"),
                (ActionId::SpeechCursorToggle, "lines"),
                (ActionId::KeyboardHelp, "keys"),
                (ActionId::Quit, "quit"),
            ],
        };
        let keymap = self.app.keymap();
        let parts: Vec<String> = hints
            .iter()
            .filter_map(|(a, label)| {
                let chords = keymap.chords_for(*a);
                let best = chords
                    .iter()
                    .find(|c| c.is_text_input() || c.mods.is_empty())
                    .or(chords.first())?;
                Some(format!("{best} {label}"))
            })
            .collect();
        // Keep what fits, always ending with the last two (help and quit
        // in browse mode, the way out in Speech Cursor mode).
        let width = usize::from(width).saturating_sub(1);
        let n = parts.len();
        let mut keep = vec![false; n];
        let mut used = 0;
        let order = (n.saturating_sub(2)..n).chain(0..n.saturating_sub(2));
        for i in order {
            let w = Span::raw(&parts[i]).width() + 2;
            if used + w <= width + 2 {
                keep[i] = true;
                used += w;
            }
        }
        let shown: Vec<&str> = parts
            .iter()
            .zip(keep)
            .filter_map(|(p, k)| k.then_some(p.as_str()))
            .collect();
        format!(" {}", shown.join("  "))
    }

    /// Draws the list overlay; returns the focused item's position.
    fn draw_list(&self, f: &mut Frame<'_>, body: Rect, theme: &Theme) -> Option<Position> {
        let list = self.list.as_ref()?;
        let area = if body.width > 10 && body.height > 4 {
            Rect::new(body.x + 2, body.y + 1, body.width - 4, body.height - 2)
        } else {
            body
        };
        f.render_widget(Clear, area);
        let block = Block::bordered()
            .title(format!(
                " {} ({} of {}) ",
                list.title,
                list.selected + 1,
                list.items.len()
            ))
            .style(theme.list);
        let inner = block.inner(area);
        f.render_widget(block, area);
        let width = usize::from(inner.width.max(1));
        let height = usize::from(inner.height.max(1));
        // Wrap each item; keep the focused one fully visible.
        let mut rows: Vec<(usize, String)> = Vec::new();
        let mut first_row = 0;
        for (i, item) in list.items.iter().enumerate() {
            if i == list.selected {
                first_row = rows.len();
            }
            let chars: Vec<char> = item.chars().collect();
            for (a, b) in layout::wrap(&chars, width, 4) {
                rows.push((i, chars[a..b].iter().collect()));
            }
        }
        let selected_rows = rows.iter().filter(|(i, _)| *i == list.selected).count();
        let skip = (first_row + selected_rows)
            .saturating_sub(height)
            .min(first_row);
        let lines: Vec<Line<'_>> = rows
            .iter()
            .skip(skip)
            .take(height)
            .map(|(i, t)| {
                let style = if *i == list.selected {
                    theme.list_selected
                } else {
                    theme.list
                };
                Line::from(Span::styled(format!("{t:<width$}"), style))
            })
            .collect();
        f.render_widget(Paragraph::new(lines), inner);
        let y = inner.y + u16::try_from(first_row - skip).unwrap_or(0);
        Some(Position::new(inner.x, y))
    }
}

fn common_prefix(ids: &[&str]) -> String {
    let Some(first) = ids.first() else {
        return String::new();
    };
    let mut len = first.len();
    for id in &ids[1..] {
        len = first
            .bytes()
            .zip(id.bytes())
            .take(len)
            .take_while(|(a, b)| a == b)
            .count();
    }
    first[..len].to_owned()
}

/// The text attributes a reading-ruler mark adds (colours stay the
/// theme's; the mark never relies on colour).
fn ruler_modifier(m: RulerStyle) -> Modifier {
    let mut out = Modifier::empty();
    if m.bold {
        out |= Modifier::BOLD;
    }
    if m.underline {
        out |= Modifier::UNDERLINED;
    }
    if m.reverse {
        out |= Modifier::REVERSED;
    }
    if m.dim {
        out |= Modifier::DIM;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chords_from_crossterm() {
        let k = |code, mods| chord(&KeyEvent::new(code, mods)).unwrap().to_string();
        assert_eq!(k(KeyCode::Char('P'), KeyModifiers::SHIFT), "Shift+P");
        assert_eq!(k(KeyCode::Char('.'), KeyModifiers::ALT), "Alt+.");
        assert_eq!(k(KeyCode::Char(' '), KeyModifiers::CONTROL), "Ctrl+Space");
        assert_eq!(k(KeyCode::Null, KeyModifiers::NONE), "Ctrl+Space");
        assert_eq!(k(KeyCode::BackTab, KeyModifiers::SHIFT), "Shift+Tab");
        assert_eq!(k(KeyCode::F(2), KeyModifiers::NONE), "F2");
    }

    #[test]
    fn prefix() {
        assert_eq!(common_prefix(&["next_list", "next_link"]), "next_li");
        assert_eq!(common_prefix(&["a"]), "a");
    }
}
