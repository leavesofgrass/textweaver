//! The terminal frontend: key handling and drawing over an [`App`].

use std::collections::HashMap;
use std::time::Instant;

use ratatui::Frame;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};
use textweaver_app::a11y::{CursorPlacement, Priority};
use textweaver_app::core::{CharPos, CharRange, Direction, Unit};
use textweaver_app::keymap::{ActionId, Key, KeyChord, Layer, Modifiers};
use textweaver_app::text_util::line_count;
use textweaver_app::{
    App, CaretMove, Command, Confirm, Effect, ListKey, Mode, PromptKey, chords_text, extra_lookup,
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

/// The reading aids drawn on a row.
struct RowAids<'a> {
    /// Bionic reading: ranges drawn bold.
    bold: &'a [CharRange],
    /// Difficult words: ranges underlined.
    difficult: &'a [CharRange],
    /// Syllable breaks: the separator is drawn before these positions.
    breaks: &'a [CharPos],
    /// The syllable separator.
    sep: &'a str,
    /// Ranges drawn as other text (Unicode math), in order.
    shown: &'a [(CharRange, String)],
    /// Code blocks and their tokens: the style under highlights, in
    /// order, not overlapping.
    code: &'a [(CharRange, Style)],
}

/// The math exploration move for a key, if it is one: arrows, Home, End,
/// Space, Enter, and Escape, without modifiers.
fn math_move(k: &KeyEvent) -> Option<textweaver_app::MathMove> {
    use textweaver_app::MathMove as M;
    if k.modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT)
    {
        return None;
    }
    Some(match k.code {
        KeyCode::Right => M::Next,
        KeyCode::Left => M::Previous,
        KeyCode::Down => M::Enter,
        KeyCode::Up => M::Exit,
        KeyCode::Home => M::First,
        KeyCode::End => M::Last,
        KeyCode::Char(' ') | KeyCode::Enter => M::Repeat,
        KeyCode::Esc => M::Leave,
        _ => return None,
    })
}

/// How long the status line stays blank before a repeated message comes
/// back, so a screen reader that speaks the status line when it changes
/// hears the message again ("No next heading." twice in a row).
pub const REPEAT_BLANK: std::time::Duration = std::time::Duration::from_millis(150);

/// The terminal frontend's state around the app.
pub struct Tui {
    app: App,
    quit: bool,
    /// The terminal's color level, detected once.
    support: ColorSupport,
    /// The styles of the theme in effect, rebuilt only when it changes.
    theme: Theme,
    /// The theme name and highlight colours `theme` was built for.
    theme_key: (String, String, Option<String>),
    /// The status announcement last drawn: its sequence number and text.
    status_shown: (u64, String),
    /// While set, the status line is drawn blank until then (a repeated
    /// message).
    status_blank_until: Option<Instant>,
    /// Copied text as an OSC 52 sequence, waiting to be written to the
    /// terminal ([`Tui::take_clipboard_sequence`]).
    clipboard_out: Option<String>,
    /// Where copied text goes ([`crate::clipboard`]).
    clipboard_route: crate::clipboard::Route,
    /// The system clipboard, for terminals that cannot take OSC 52.
    system_clipboard: crate::clipboard::SystemClipboard,
    /// Set once the listener was told that the system clipboard is used.
    system_clipboard_said: bool,
    /// The title line's position while it is frozen (`[accessibility]
    /// quiet_screen` during continuous reading).
    frozen_position: Option<String>,
    /// Physical keys peeked from the Windows console, for the digit row.
    digits: crate::physical::DigitKeys,
    /// Code block tokens already found ([`crate::highlight`]).
    #[cfg(feature = "highlight")]
    code_cache: std::cell::RefCell<crate::highlight::Cache>,
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
    /// Wraps an app, with the terminal's color level and clipboard route
    /// detected ([`crate::clipboard::detect`]).
    pub fn new(app: App) -> Self {
        let mut tui = Self::with_color_support(app, ColorSupport::detect());
        tui.clipboard_route = crate::clipboard::detect_here();
        tui
    }

    /// Wraps an app, drawing at a given color level (tests; at run time
    /// [`ColorSupport::detect`] also honors `TEXTWEAVER_COLOR` and
    /// `NO_COLOR`).
    pub fn with_color_support(app: App, support: ColorSupport) -> Self {
        let theme = Theme::from_theme(&app.reading_theme(), support);
        let theme_key = app.reading_theme_key();
        Tui {
            app,
            quit: false,
            support,
            theme,
            theme_key,
            status_shown: (0, String::new()),
            status_blank_until: None,
            clipboard_out: None,
            clipboard_route: crate::clipboard::Route::Osc52,
            system_clipboard: crate::clipboard::SystemClipboard::default(),
            system_clipboard_said: false,
            frozen_position: None,
            digits: crate::physical::DigitKeys::default(),
            #[cfg(feature = "highlight")]
            code_cache: std::cell::RefCell::default(),
        }
    }

    /// The physical keys the event loop peeked from the console (Windows),
    /// matched with the next character events.
    pub fn digit_keys_mut(&mut self) -> &mut crate::physical::DigitKeys {
        &mut self.digits
    }

    /// The chord for a browse-mode key press, with the digit row matched by
    /// the physical key ([`crate::physical`]): `1` to `6` and Shift with
    /// them reach the heading levels on any layout.
    pub fn browse_chord(&mut self, k: &KeyEvent) -> Option<KeyChord> {
        let c = chord(k)?;
        let KeyCode::Char(ch) = k.code else {
            return Some(c);
        };
        if k.modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
        {
            return Some(c);
        }
        use textweaver_app::keymap::digits::{DigitRow, digit_row_chord, from_typed};
        match self.digits.take(ch) {
            // The console named the key: a digit-row key is its digit.
            Some(Some((d, shift))) => return digit_row_chord(d, shift).or(Some(c)),
            Some(None) => return Some(c),
            None => {}
        }
        let row = textweaver_app::digit_row(self.app.settings().keyboard.digit_row);
        let layer = self.app.mode().layer();
        let unbound =
            self.app.keymap().lookup(&c, layer).is_none() && extra_lookup(&c, layer).is_none();
        match row {
            DigitRow::Azerty => from_typed(ch, row).or(Some(c)),
            DigitRow::Auto if unbound => from_typed(ch, row).or(Some(c)),
            DigitRow::Auto => Some(c),
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

    /// The open prompt, if any: the app's prompt model (Wave 3), shared
    /// with the GUI and JSON-RPC.
    pub fn minibuffer(&self) -> Option<&Minibuffer> {
        self.app.prompt_model()
    }

    /// The open list, if any: the app's list model (Wave 3), shared with
    /// the GUI and JSON-RPC.
    pub fn list(&self) -> Option<&ListView> {
        self.app.list_model()
    }

    /// The styles of the theme in effect (with the reader's highlight
    /// colours), cached and rebuilt only when the theme or those colours
    /// change.
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Rebuilds the cached styles when the app's theme changed.
    fn refresh_theme(&mut self) {
        let key = self.app.reading_theme_key();
        if key != self.theme_key {
            self.theme = Theme::from_theme(&self.app.reading_theme(), self.support);
            self.theme_key = key;
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

    /// The OSC 52 sequence for text copied or cut since the last call; the
    /// event loop writes it to the terminal, which puts the text on the
    /// system clipboard (over SSH too).
    pub fn take_clipboard_sequence(&mut self) -> Option<String> {
        self.clipboard_out.take()
    }

    /// Where copied text goes: OSC 52 (the default for a `Tui` made with
    /// [`Tui::with_color_support`]), the system clipboard, or both.
    pub fn set_clipboard_route(&mut self, route: crate::clipboard::Route) {
        self.clipboard_route = route;
    }

    /// Where copied text goes.
    pub fn clipboard_route(&self) -> crate::clipboard::Route {
        self.clipboard_route
    }

    /// Sends copied text on its route. The first use of the system
    /// clipboard is said once; if it fails, the text goes to the terminal
    /// instead, and the failure is said.
    fn send_to_clipboard(&mut self, text: &str) {
        use crate::clipboard::Route;
        let route = self.clipboard_route;
        let mut osc = route != Route::System;
        if route != Route::Osc52 {
            match self.system_clipboard.set_text(text) {
                Ok(()) => {
                    if route == Route::System && !self.system_clipboard_said {
                        self.system_clipboard_said = true;
                        self.app.announce_queued(
                            "Copied with the system clipboard, because this terminal cannot take copied text.",
                            Priority::Polite,
                        );
                    }
                }
                Err(e) => {
                    osc = true;
                    log::warn!("system clipboard: {e}");
                    self.app.announce_queued(
                        &format!(
                            "Could not copy with the system clipboard: {e}. Sent to the terminal instead."
                        ),
                        Priority::Assertive,
                    );
                }
            }
        }
        if osc {
            self.clipboard_out = Some(textweaver_app::osc52(text));
        }
    }

    fn apply(&mut self, effects: Vec<Effect>) {
        if let Some(text) = self.app.take_clipboard() {
            self.send_to_clipboard(&text);
        }
        // Prompts and lists are the app's own models (Wave 3): the app
        // adopted them, and said a list's focused item, before returning
        // these effects.
        if effects.contains(&Effect::Quit) {
            self.quit = true;
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
        if self.app.prompt_model().is_some() {
            self.dispatch(Command::PromptKey(PromptKey::Paste(text.to_owned())));
            return;
        }
        if self.app.list_model().is_none() {
            let text = text.replace("\r\n", "\n").replace('\r', "\n");
            self.dispatch(Command::Insert(text));
        }
    }

    /// Handles one key press.
    pub fn handle_key(&mut self, k: KeyEvent) {
        if self.app.math_exploring() && !self.app.confirmation_pending() {
            if let Some(mv) = math_move(&k) {
                self.dispatch(Command::MathStep(mv));
                return;
            }
            if !matches!(k.code, KeyCode::Modifier(_)) {
                // Any other key leaves math exploration and does what it
                // usually does.
                self.app.stop_math_exploring();
            }
        }
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
        if self.app.list_model().is_some() {
            self.list_key(k);
        } else if self.app.prompt_model().is_some() {
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
        let Some(c) = self.browse_chord(&k) else {
            return;
        };
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

    /// A key in a list: the app's list model handles it (Wave 3), so the
    /// terminal, the GUI, and JSON-RPC behave the same.
    fn list_key(&mut self, k: KeyEvent) {
        let plain = !k
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        // The Help and Say Status keys repeat the list's introduction; the
        // Repeat Message key says the last message. Only chords and
        // function keys: plain keys move, filter, and jump in the list, and
        // F2 renames.
        if (!plain || matches!(k.code, KeyCode::F(_))) && k.code != KeyCode::F(2) {
            match chord(&k).and_then(|c| self.app.keymap().lookup(&c, Layer::Global)) {
                Some(ActionId::Help | ActionId::SayStatus) => {
                    self.dispatch(Command::ListKey(ListKey::Introduce));
                    return;
                }
                Some(ActionId::RepeatMessage) => {
                    self.dispatch(Command::Action(ActionId::RepeatMessage));
                    return;
                }
                _ => {}
            }
        }
        let key = match k.code {
            KeyCode::Char(c) if plain && !c.is_control() => ListKey::Char(c),
            KeyCode::Up => ListKey::Up,
            KeyCode::Down => ListKey::Down,
            KeyCode::PageUp => ListKey::PageUp,
            KeyCode::PageDown => ListKey::PageDown,
            KeyCode::Home => ListKey::Home,
            KeyCode::End => ListKey::End,
            KeyCode::Left => ListKey::Left,
            KeyCode::Right => ListKey::Right,
            KeyCode::Enter => ListKey::Enter,
            KeyCode::Esc => ListKey::Escape,
            KeyCode::Backspace => ListKey::Backspace,
            KeyCode::Delete => ListKey::Delete,
            KeyCode::F(2) => ListKey::Rename,
            _ => return,
        };
        self.dispatch(Command::ListKey(key));
    }

    /// A key in the prompt: the app's prompt model handles it (Wave 3).
    fn minibuffer_key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let key = match k.code {
            KeyCode::Enter => PromptKey::Enter,
            KeyCode::Esc => PromptKey::Escape,
            KeyCode::Char('g') if ctrl => PromptKey::Escape,
            KeyCode::Char('a') if ctrl => PromptKey::Home,
            KeyCode::Char('e') if ctrl => PromptKey::End,
            KeyCode::Char('u') if ctrl => PromptKey::KillToStart,
            KeyCode::Char('k') if ctrl => PromptKey::KillToEnd,
            KeyCode::Char('w') if ctrl => PromptKey::DeleteWordBack,
            KeyCode::Char(c) if typed_char(&k).is_some() => PromptKey::Char(c),
            KeyCode::Backspace => PromptKey::Backspace,
            KeyCode::Delete => PromptKey::Delete,
            KeyCode::Left => PromptKey::Left,
            KeyCode::Right => PromptKey::Right,
            KeyCode::Home => PromptKey::Home,
            KeyCode::End => PromptKey::End,
            KeyCode::Tab => PromptKey::Tab,
            KeyCode::Up => PromptKey::Up,
            KeyCode::Down => PromptKey::Down,
            _ => return,
        };
        self.dispatch(Command::PromptKey(key));
    }

    /// What the status line shows: the pending question while one waits for
    /// a yes or no (so it stays in view whatever else is announced), else
    /// the latest announcement.
    pub fn status_line(&self) -> String {
        match self.app.pending_question() {
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
    /// "No next heading." was silent (docs/history/audit-2026-09.md, finding A5).
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
        self.draw_at(f, Instant::now());
    }

    /// [`draw`](Self::draw) as of `now`, which decides whether a repeated
    /// status message is still blanked (tests pass their own times).
    pub fn draw_at(&mut self, f: &mut Frame<'_>, now: Instant) {
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
        let position = self.title_position();
        self.draw_title(f, areas.title, &theme, position.as_deref());
        let cursor = self.draw_body(f, areas.body, &theme);
        self.draw_rsvp(f, areas.body, &theme, cursor.map(|p| p.y));
        let status = self.status_to_draw(now);
        f.render_widget(
            Paragraph::new(status)
                .wrap(ratatui::widgets::Wrap { trim: false })
                .style(theme.status),
            areas.status,
        );
        let prompt = self.draw_bottom(f, areas.bottom, &theme);
        let cursor = prompt.or(cursor);
        let cursor = self.draw_list(f, areas.body, &theme).or(cursor);
        // `[accessibility] cursor = "status"`: the cursor waits at the start
        // of the status line (as in Star), except at a prompt's caret, where
        // typing needs it.
        let cursor = if prompt.is_none() && self.app.cursor_placement() == CursorPlacement::Status {
            Some(Position::new(areas.status.x, areas.status.y))
        } else {
            cursor
        };
        f.set_cursor_position(cursor.unwrap_or(Position::new(areas.body.x, areas.body.y)));
    }

    /// The title line's "line 3 of 40, 7%": frozen while
    /// [`App::quiet_screen_active`], so a screen reader that reads the
    /// changing screen does not hear it tick over as textweaver reads.
    fn title_position(&mut self) -> Option<String> {
        let now = self.app.title_position()?;
        if self.app.quiet_screen_active() {
            Some(self.frozen_position.get_or_insert(now).clone())
        } else {
            self.frozen_position = None;
            Some(now)
        }
    }

    fn draw_title(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme, position: Option<&str>) {
        let app = &self.app;
        let title = app.session().map_or("no document", |s| s.title.as_str());
        let left = format!(" textweaver: {title}");
        // Most important first; trailing parts are dropped when narrow.
        // The app gives them ("Ready" until the first reading, then
        // "Stopped"), so Say Status speaks the same parts.
        let mut parts = app.title_parts(position);
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
    /// underline, never colour alone; dim only for the opt-in mask),
    /// terminal text spacing (blank rows between lines and paragraphs,
    /// wider spaces between words), syllables (a separator drawn between
    /// syllables, at the app's break positions, so highlights and the
    /// cursor keep the document's positions), and difficult words
    /// (underlined).
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
        let syllables = settings.reading_aids.syllables;
        let line_breaks = |r: CharRange| {
            if syllables {
                self.app.syllable_breaks(r)
            } else {
                Vec::new()
            }
        };
        // Unicode math (`[reading] math_display`): the formulas from the
        // top line to as far as the layout may go.
        let shown = {
            let lines = textweaver_app::text_util::line_count(doc);
            let last = top
                .saturating_add(rows_wanted.saturating_mul(4).max(512))
                .min(lines.saturating_sub(1));
            let from = textweaver_app::text_util::line_range(doc, top.min(last)).start;
            let to = textweaver_app::text_util::line_range(doc, last).end;
            self.app
                .math_display(CharRange::new(from, to.saturating_add(1)))
        };
        let decor = (syllables || !shown.is_empty()).then(|| layout::Decor {
            breaks: &line_breaks,
            text: if syllables {
                self.app.syllable_separator().to_owned()
            } else {
                String::new()
            },
            shown: &shown,
        });
        let sep_width = decor.as_ref().map_or(0, layout::Decor::width);
        let rows = layout::window_decor(
            doc,
            top,
            width,
            rows_wanted,
            cells,
            decor.as_ref(),
            focus,
            margin,
        );
        let window = match (rows.first(), rows.last()) {
            (Some(a), Some(b)) => CharRange::new(a.range.start, b.range.end.saturating_add(1)),
            _ => CharRange::empty(0),
        };
        let highlights = self.app.highlights(window);
        let code = self.code_styles(doc, window, theme);
        let bold = self.app.bionic_ranges(window);
        let difficult = self.app.difficult_ranges(window);
        // The syllable breaks of each line shown, as the layout used them.
        let mut breaks: HashMap<usize, Vec<CharPos>> = HashMap::new();
        if syllables {
            for r in &rows {
                breaks.entry(r.line).or_insert_with(|| {
                    line_breaks(textweaver_app::text_util::line_range(doc, r.line))
                });
            }
        }
        let sep = self.app.syllable_separator().to_owned();
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
            let row_breaks = breaks.get(&row.line).map_or(&[][..], Vec::as_slice);
            let aids = RowAids {
                bold: &bold,
                difficult: &difficult,
                breaks: row_breaks,
                sep: &sep,
                shown: &shown,
                code: &code,
            };
            let mut text = self.row_spans(doc, row, &highlights, &aids, theme, cells);
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
                let col = layout::column_shown(doc, row, fp, cells, row_breaks, sep_width, &shown)
                    .min(width.saturating_sub(1));
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

    /// The styles of the code blocks in `window` (the `highlight`
    /// feature): each block in the theme's code colors, and its tokens in
    /// theirs, as ranges in order that do not overlap. Empty without the
    /// feature.
    fn code_styles(
        &self,
        doc: &textweaver_app::text::Document,
        window: CharRange,
        theme: &Theme,
    ) -> Vec<(CharRange, Style)> {
        #[cfg(feature = "highlight")]
        {
            use crate::highlight::Token;
            use textweaver_app::core::MarkerKind;
            let styles = &theme.code;
            let mut out = Vec::new();
            let mut blocks: Vec<&textweaver_app::text::Marker> = doc
                .markers()
                .iter()
                .filter(|m| {
                    m.kind == MarkerKind::Code
                        && m.level == 1
                        && m.range.start < window.end
                        && window.start < m.range.end
                })
                .collect();
            blocks.sort_by_key(|m| m.range.start);
            let mut cache = self.code_cache.borrow_mut();
            for m in blocks {
                let start = m.range.start.0;
                let tokens = m
                    .label
                    .as_deref()
                    .filter(|l| !l.is_empty())
                    .and_then(|lang| cache.tokens(lang, &doc.slice(m.range)));
                let mut at = start;
                for &(a, b, t) in tokens.iter().flat_map(|t| t.iter()) {
                    let (a, b) = (start + a, start + b);
                    if a > at {
                        out.push((CharRange::new(at, a), styles.plain));
                    }
                    let st = match t {
                        Token::Plain => styles.plain,
                        Token::Comment => styles.comment,
                        Token::Keyword => styles.keyword,
                        Token::String => styles.string,
                        Token::Number => styles.number,
                        Token::Function => styles.function,
                        Token::Type => styles.kind,
                    };
                    out.push((CharRange::new(a, b), st));
                    at = b;
                }
                if at < m.range.end.0 {
                    out.push((CharRange::new(at, m.range.end.0), styles.plain));
                }
            }
            out.retain(|(r, _)| !r.is_empty());
            out
        }
        #[cfg(not(feature = "highlight"))]
        {
            let _ = (doc, window, theme);
            Vec::new()
        }
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
        aids: &RowAids<'_>,
        theme: &Theme,
        cells: Cells,
    ) -> Vec<Span<'static>> {
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut run = String::new();
        let mut run_style: Option<Style> = None;
        let chars = doc.text().slice(row.range.to_range()).chars();
        let bold = aids.bold;
        let difficult = aids.difficult;
        let mut b = bold.partition_point(|r| r.end <= row.range.start);
        let mut d = difficult.partition_point(|r| r.end <= row.range.start);
        let mut k = aids.code.partition_point(|(r, _)| r.end <= row.range.start);
        for (i, c) in chars.enumerate() {
            let pos = CharPos(row.range.start.0 + i);
            while k < aids.code.len() && aids.code[k].0.end <= pos {
                k += 1;
            }
            let base = match aids.code.get(k) {
                Some((r, st)) if r.contains(pos) => *st,
                _ => theme.text,
            };
            let mut style = highlights
                .iter()
                .filter(|h| h.range.contains(pos))
                .fold(base, |st, h| st.patch(theme.highlight(h.kind)));
            while b < bold.len() && bold[b].end <= pos {
                b += 1;
            }
            if bold.get(b).is_some_and(|r| r.contains(pos)) {
                style = style.add_modifier(Modifier::BOLD);
            }
            while d < difficult.len() && difficult[d].end <= pos {
                d += 1;
            }
            if difficult.get(d).is_some_and(|r| r.contains(pos)) {
                style = style.add_modifier(Modifier::UNDERLINED);
            }
            if run_style != Some(style) {
                if let Some(st) = run_style {
                    spans.push(Span::styled(std::mem::take(&mut run), st));
                }
                run_style = Some(style);
            }
            // A syllable separator takes the style of the char after it,
            // so a highlight over a word covers its separators too.
            if i > 0 && aids.breaks.binary_search(&pos).is_ok() {
                run.push_str(aids.sep);
            }
            match layout::shown_at(aids.shown, pos) {
                Some(Some(text)) => run.push_str(text),
                Some(None) => {}
                None => run.push_str(&cells.text(c)),
            }
        }
        if let Some(st) = run_style {
            spans.push(Span::styled(run, st));
        }
        spans
    }

    /// The minibuffer or the key hints; returns the caret position when a
    /// prompt is open.
    fn draw_bottom(&self, f: &mut Frame<'_>, area: Rect, theme: &Theme) -> Option<Position> {
        if let Some(mb) = self.minibuffer() {
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
                (ActionId::CommandPalette, "commands"),
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
        // Only keys that work here: in this mode's layers, and with
        // single-key shortcuts as F9 left them (a hint for a key that does
        // nothing misleads).
        let layer = self.app.mode().layer();
        let parts: Vec<String> = hints
            .iter()
            .filter_map(|(a, label)| {
                let chords = keymap.chords_in_mode(*a, layer);
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
        let list = self.list()?;
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
}
