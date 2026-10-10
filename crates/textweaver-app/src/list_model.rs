//! List and prompt state, shared by every frontend (Wave 3, Agent W3a).
//!
//! Until Wave 3 the terminal reader kept the focused list item, first-letter
//! jumps, the "3 of 12" announcements, and the prompt's text, caret, and
//! history itself (`textweaver-tui`'s `widgets` and `ui`). They live here
//! now, so the terminal reader, the GUI's dialogs, and JSON-RPC share one
//! implementation and say the same things:
//!
//! - When the app shows a list ([`Effect::ShowList`]), it keeps a
//!   [`ListModel`]: the title, the items, and the focused item, which
//!   stays in place when the same list is shown again (after a delete).
//!   The focused item is announced after the list's introduction.
//! - A frontend sends list keys as [`Command::ListKey`]: arrows, Page Up
//!   and Down, Home and End move and announce "k of n, item" (or "Top of
//!   list."); a typed character filters a list that filters as you type,
//!   chooses by its accelerator (`s`, `d`, `c` in Save, Discard, Cancel),
//!   or jumps to the next item starting with it; Enter chooses; Escape
//!   closes; Delete, F2, and Space act on the item. A GUI list that moves
//!   its own focus reports it quietly with [`Command::ListFocus`].
//! - When the app opens a prompt ([`Effect::Prompt`]), it keeps a
//!   [`PromptModel`]: label, purpose, text, and caret. A frontend sends
//!   [`Command::PromptKey`]: characters, Backspace, Delete, the caret keys,
//!   the Emacs-style kills, Up and Down (earlier answers to the same
//!   prompt, or command palette matches), Tab (completes a command name or
//!   a file path; moves to the next field of a form), Shift+Tab (the
//!   previous field), Enter, and Escape. Typing is echoed as in edit mode. A
//!   GUI text field that edits its own text sends its whole text with
//!   [`PromptKey::SetText`] and Enter with [`PromptKey::Enter`].
//!
//! [`Effect::ShowList`]: crate::Effect::ShowList
//! [`Effect::Prompt`]: crate::Effect::Prompt
//! [`Command::ListKey`]: crate::Command::ListKey
//! [`Command::ListFocus`]: crate::Command::ListFocus
//! [`Command::PromptKey`]: crate::Command::PromptKey

use std::borrow::Cow;

use textweaver_a11y::Priority;
use textweaver_keymap::ActionId;
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;

use crate::app::App;
use crate::command::{Command, Effect, PromptPurpose};

/// Most recalled answers kept per prompt.
pub const PROMPT_HISTORY: usize = 50;

/// How far Page Up and Page Down move in a list.
pub const LIST_PAGE: usize = 10;

/// A list shown to the user: its title, items, and focused item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListModel {
    /// Title.
    pub title: String,
    /// Items, as shown (and spoken by a screen reader).
    pub items: Vec<String>,
    /// The focused item (0 in an empty list).
    pub selected: usize,
    /// In a list of commands (the keyboard shortcuts list, the command
    /// palette's list), each row's short name and key, for a frontend
    /// that draws the key at the right edge ("Find next" and "F3"; the key
    /// is empty for a command without one). Empty for other lists.
    pub columns: Vec<(String, String)>,
    /// In a list of commands, each row's long explanation, which F1 says
    /// and a frontend gives the row as its description. Empty for other
    /// lists.
    pub descriptions: Vec<String>,
    /// Items as textweaver's voice says them: keys named in an item
    /// ("Open a document: Ctrl+O") keep both forms (crate::help), so the
    /// voice says "Control O". Empty when no item names a key.
    said: Vec<String>,
    /// The position is said after the item ("notes.md, Markdown, 3 of
    /// 40"), so a Braille line starts with the name (the file browser).
    position_last: bool,
}

impl ListModel {
    /// A list focused on its first item.
    pub fn new(title: impl Into<String>, items: Vec<String>) -> Self {
        let marked = items
            .iter()
            .any(|i| matches!(crate::help::written_text(i), Cow::Owned(_)));
        let (items, said) = if marked {
            let shown = items
                .iter()
                .map(|i| crate::help::written_text(i).into_owned())
                .collect();
            (shown, items)
        } else {
            (items, Vec::new())
        };
        ListModel {
            title: title.into(),
            items,
            selected: 0,
            columns: Vec::new(),
            descriptions: Vec::new(),
            said,
            position_last: false,
        }
    }

    /// Says the position after each item instead of before it: "notes.md,
    /// Markdown, 12 KB, 3 of 40", meaning first on a Braille line.
    pub fn with_position_last(mut self) -> Self {
        self.position_last = true;
        self
    }

    /// Moves the focus by `delta`, clamped; returns true when it moved.
    pub fn step(&mut self, delta: isize) -> bool {
        let last = self.items.len().saturating_sub(1);
        let next = self.selected.saturating_add_signed(delta).min(last);
        let moved = next != self.selected;
        self.selected = next;
        moved
    }

    /// The focused item's text.
    pub fn current(&self) -> Option<&str> {
        self.items.get(self.selected).map(String::as_str)
    }

    /// Moves the focus to the next item (after the focused one, wrapping)
    /// whose first letter or digit is `c`, ignoring case. Returns true when
    /// one was found.
    pub fn jump_to_letter(&mut self, c: char) -> bool {
        let want: Vec<char> = c.to_lowercase().collect();
        let n = self.items.len();
        let starts = |s: &str| {
            s.chars()
                .find(|ch| ch.is_alphanumeric())
                .is_some_and(|f| f.to_lowercase().eq(want.iter().copied()))
        };
        for step in 1..=n {
            let i = (self.selected + step) % n;
            if starts(&self.items[i]) {
                self.selected = i;
                return true;
            }
        }
        false
    }

    /// The focused item as spoken: its text and where it is in the list
    /// ("2 of 5, Chapter two").
    pub fn spoken_item(&self) -> Option<String> {
        self.spoken_item_text(&Catalog::english())
    }

    /// The focused item as spoken ([`spoken_item`](Self::spoken_item)), in
    /// the language of `c`.
    pub fn spoken_item_text(&self, c: &Catalog) -> Option<String> {
        let item = self
            .said
            .get(self.selected)
            .map(String::as_str)
            .or_else(|| self.current())?;
        let id = if self.position_last {
            "listmodel-item-position-last"
        } else {
            "listmodel-item-position"
        };
        Some(c.fmt(
            id,
            &args!["item" => item, "k" => self.selected + 1, "n" => self.items.len()],
        ))
    }
}

/// Effects as a frontend gets them: keys named in list items in their
/// written form only (crate::help). The list model keeps both.
pub(crate) fn without_key_marks(mut effects: Vec<Effect>) -> Vec<Effect> {
    for e in &mut effects {
        if let Effect::ShowList { items, .. } = e {
            for item in items.iter_mut() {
                if let Cow::Owned(plain) = crate::help::written_text(item) {
                    *item = plain;
                }
            }
        }
    }
    effects
}

/// A key pressed in a list ([`Command::ListKey`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListKey {
    /// Up: the previous item.
    Up,
    /// Down: the next item.
    Down,
    /// Page Up: [`LIST_PAGE`] items back.
    PageUp,
    /// Page Down: [`LIST_PAGE`] items on.
    PageDown,
    /// Home: the first item.
    Home,
    /// End: the last item.
    End,
    /// Left: in the settings list, a smaller value or the previous choice.
    Left,
    /// Right: in the settings list, a larger value or the next choice.
    Right,
    /// A typed character (no Control or Alt): filters a list that filters
    /// as you type, chooses by an accelerator, or jumps to the next item
    /// starting with it; Space marks an item (a favourite voice).
    Char(char),
    /// Backspace: removes the filter's last character, else closes the
    /// list.
    Backspace,
    /// Enter: chooses the focused item.
    Enter,
    /// Escape: closes the list.
    Escape,
    /// Delete: deletes the focused item (bookmarks, notes, highlights), or
    /// resets a setting to its default.
    Delete,
    /// F2: renames or edits the focused item.
    Rename,
    /// Says the list's introduction again (its title, how many items it
    /// has, and the keys it takes), then the focused item. The terminal
    /// sends it for F1.
    Introduce,
    /// The Say Status key: in the file browser, a preview of the focused
    /// row (crate::browse); in the voice list, a sample in the focused
    /// voice; in other lists, as [`Introduce`](Self::Introduce).
    Details,
    /// The file browser's Choose Folder key (Ctrl+Enter): the focused
    /// folder, or the one shown, goes to the command that asked for it.
    ChooseHere,
    /// The file browser's Sort key (Ctrl+R): name, date, size.
    Sort,
    /// The file browser's Show All key (Ctrl+A): every file, or readable
    /// ones only.
    ShowAll,
}

/// The history a prompt's answers join: the text to find is one history,
/// whether typed for Find or for find and replace.
fn history_key(purpose: PromptPurpose) -> PromptPurpose {
    match purpose {
        PromptPurpose::ReplaceFind => PromptPurpose::Find,
        p => p,
    }
}

/// A key pressed in a prompt ([`Command::PromptKey`]).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PromptKey {
    /// A typed character, inserted at the caret.
    Char(char),
    /// Pasted text (control characters are dropped), inserted at the caret.
    Paste(String),
    /// The whole text, from a GUI text field that edits its own text; the
    /// caret goes to the end. Nothing is echoed (the field does that).
    SetText(String),
    /// Deletes the character before the caret.
    Backspace,
    /// Deletes the character at the caret.
    Delete,
    /// The caret one character left.
    Left,
    /// The caret one character right.
    Right,
    /// The caret to the start.
    Home,
    /// The caret to the end.
    End,
    /// Deletes from the start to the caret (Ctrl+U).
    KillToStart,
    /// Deletes from the caret to the end (Ctrl+K).
    KillToEnd,
    /// Deletes the word before the caret (Ctrl+W).
    DeleteWordBack,
    /// Up: an earlier answer to this prompt, or the previous command
    /// palette match.
    Up,
    /// Down: a later answer, or the next command palette match.
    Down,
    /// Tab: completes a command name or a file path; in a form (the edit
    /// details form), moves to the next field.
    Tab,
    /// Shift+Tab: in a form, moves to the previous field; elsewhere
    /// nothing.
    BackTab,
    /// Ctrl+L in the command palette: shows its matches as a list, to hear
    /// them in context; Enter runs one.
    ShowMatches,
    /// The browse key ([`crate::path_prompt::browse_key`], F4) in a prompt
    /// for a path: the file browser chooses it, and the path fills the
    /// prompt; elsewhere nothing (W8a-f).
    Browse,
    /// Enter: answers the prompt with its text.
    Enter,
    /// Escape: cancels the prompt.
    Escape,
}

/// A one-line prompt with a caret, history position, and (for the command
/// palette) completion candidates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptModel {
    /// Prompt label.
    pub label: String,
    /// What the answer is for.
    pub purpose: PromptPurpose,
    text: Vec<char>,
    caret: usize,
    /// Position in the prompt's history while recalling (`None` when
    /// editing fresh text).
    pub history_index: Option<usize>,
    /// Palette candidates for the current text: the action and what is
    /// said for it.
    pub candidates: Vec<(ActionId, String)>,
    /// Which candidate Up and Down are on.
    pub candidate: Option<usize>,
    /// Up and Down started from an empty palette, whose recent commands
    /// come first and are said as recent.
    candidate_from_empty: bool,
    /// Its text was chosen in the file browser (the browse key).
    pub(crate) browsed: bool,
}

impl PromptModel {
    /// An empty prompt.
    pub fn new(label: impl Into<String>, purpose: PromptPurpose) -> Self {
        PromptModel {
            label: label.into(),
            purpose,
            text: Vec::new(),
            caret: 0,
            history_index: None,
            candidates: Vec::new(),
            candidate: None,
            candidate_from_empty: false,
            browsed: false,
        }
    }

    /// True when the file browser filled this prompt (its browse key,
    /// [`PromptKey::Browse`]): a GUI shows it as typed, not through the
    /// system's chooser again.
    pub fn from_browser(&self) -> bool {
        self.browsed
    }

    /// The text typed so far.
    pub fn text(&self) -> String {
        self.text.iter().collect()
    }

    /// The text to draw: the text typed, or one star per character in a
    /// secret prompt ([`PromptPurpose::is_secret`]), so a token is never
    /// on the screen or the Braille display.
    pub fn shown_text(&self) -> String {
        if self.purpose.is_secret() {
            "*".repeat(self.text.len())
        } else {
            self.text()
        }
    }

    /// The caret, in chars from the start of the text.
    pub fn caret(&self) -> usize {
        self.caret
    }

    /// Replaces the text and puts the caret at its end.
    pub fn set_text(&mut self, s: &str) {
        self.text = s.chars().collect();
        self.caret = self.text.len();
    }

    /// Inserts a char at the caret.
    pub fn insert(&mut self, c: char) {
        self.text.insert(self.caret, c);
        self.caret += 1;
    }

    /// Deletes the char before the caret.
    pub fn backspace(&mut self) -> Option<char> {
        if self.caret == 0 {
            return None;
        }
        self.caret -= 1;
        Some(self.text.remove(self.caret))
    }

    /// Deletes the char at the caret.
    pub fn delete(&mut self) -> Option<char> {
        (self.caret < self.text.len()).then(|| self.text.remove(self.caret))
    }

    /// Moves the caret one char left; false at the start.
    pub fn left(&mut self) -> bool {
        let moved = self.caret > 0;
        self.caret = self.caret.saturating_sub(1);
        moved
    }

    /// Moves the caret one char right; false at the end.
    pub fn right(&mut self) -> bool {
        let moved = self.caret < self.text.len();
        self.caret = (self.caret + 1).min(self.text.len());
        moved
    }

    /// Caret to the start.
    pub fn home(&mut self) {
        self.caret = 0;
    }

    /// Caret to the end.
    pub fn end(&mut self) {
        self.caret = self.text.len();
    }

    /// Deletes from the start to the caret; returns what was deleted.
    pub fn kill_to_start(&mut self) -> String {
        let gone: String = self.text.drain(..self.caret).collect();
        self.caret = 0;
        gone
    }

    /// Deletes from the caret to the end; returns what was deleted.
    pub fn kill_to_end(&mut self) -> String {
        self.text.drain(self.caret..).collect()
    }

    /// Deletes the word before the caret; returns it.
    pub fn delete_word_back(&mut self) -> String {
        let mut start = self.caret;
        while start > 0 && self.text[start - 1].is_whitespace() {
            start -= 1;
        }
        while start > 0 && !self.text[start - 1].is_whitespace() {
            start -= 1;
        }
        let gone: String = self.text.drain(start..self.caret).collect();
        self.caret = start;
        gone
    }

    /// The char at the caret, if any.
    pub fn char_at_caret(&self) -> Option<char> {
        self.text.get(self.caret).copied()
    }
}

/// The longest start shared by `ids`.
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

impl App {
    /// The list shown, with its focused item, if any. While one is shown,
    /// a frontend sends list keys as [`Command::ListKey`].
    pub fn list_model(&self) -> Option<&ListModel> {
        self.list_model.as_ref()
    }

    /// The prompt open, with its text and caret, if any. While one is open,
    /// a frontend sends prompt keys as [`Command::PromptKey`].
    pub fn prompt_model(&self) -> Option<&PromptModel> {
        self.prompt_model.as_ref()
    }

    /// Whether the app says a list's focused item when the list is shown,
    /// after the list's introduction ("1 of 5, Chapter one"). On by
    /// default. A GUI whose native list the screen reader announces when
    /// it opens turns this off, so the item is not heard twice. Moves made
    /// with [`Command::ListKey`] are always said.
    pub fn set_announce_list_focus(&mut self, on: bool) {
        self.announce_list_focus = on;
    }

    /// Earlier answers to prompts for `purpose`, oldest first (at most
    /// [`PROMPT_HISTORY`]).
    /// Find and the replace prompt's "find what" share one history.
    pub fn prompt_history(&self, purpose: PromptPurpose) -> &[String] {
        self.answers
            .get(&history_key(purpose))
            .map_or(&[], Vec::as_slice)
    }

    /// Keeps the list and prompt models in step with effects returned to
    /// the frontend: a prompt replaces any list, a list any prompt; the
    /// same list shown again keeps its focus. The focused item is said
    /// after the list's introduction, without interrupting it (the first
    /// item was never heard unless the user pressed Up,
    /// the September 2026 audit, finding A4).
    ///
    /// `fresh_message` says a message was said while the effects were
    /// made: for a list shown, its introduction, kept for
    /// [`ListKey::Introduce`].
    pub(crate) fn adopt(&mut self, effects: &[Effect], fresh_message: bool) {
        for e in effects {
            match e {
                Effect::Prompt { label, purpose } => {
                    self.list_model = None;
                    let mut model = PromptModel::new(label.clone(), *purpose);
                    if let Some(text) = self.pending_prompt_text.take() {
                        model.set_text(&text);
                    }
                    model.browsed = std::mem::take(&mut self.browse.prompt_filled);
                    self.prompt_model = Some(model);
                }
                Effect::ShowList { title, items } => {
                    self.prompt_model = None;
                    let keep = self
                        .list_model
                        .as_ref()
                        .filter(|l| &l.title == title)
                        .map(|l| l.selected);
                    let same_list = self.list_model.as_ref().is_some_and(|l| &l.title == title);
                    // The same list again after a change just said: the
                    // item and the introduction stay as they were.
                    let quiet = std::mem::take(&mut self.list_reshow_quiet);
                    if quiet {
                        // Kept.
                    } else if fresh_message {
                        self.list_intro = self.last_message.clone();
                    } else if !same_list {
                        self.list_intro = None;
                    }
                    let mut view = ListModel::new(title.clone(), items.clone());
                    if self.list == Some(crate::app::ListKind::Browse) {
                        view = view.with_position_last();
                    }
                    if let Some((columns, descriptions)) = self.command_columns() {
                        view.columns = columns;
                        view.descriptions = descriptions;
                    }
                    // A focus the app asked for wins over the one kept
                    // (the file browser keeps its row when sorted).
                    if let Some(i) = self.pending_list_focus.take().or(keep) {
                        view.selected = i.min(view.items.len().saturating_sub(1));
                    }
                    let item = (self.announce_list_focus && !quiet)
                        .then(|| view.spoken_item_text(self.cat()))
                        .flatten();
                    // Shown first, so a message held for it belongs to it
                    // (crate::announce drops it if the list closes).
                    self.list_model = Some(view);
                    if let Some(item) = item {
                        self.announce_queued(&item, Priority::Polite);
                    }
                }
                Effect::Redraw | Effect::Quit => {}
            }
        }
    }

    /// Before a command runs: the list or prompt it closes.
    pub(crate) fn close_models_for(&mut self, cmd: &Command) {
        match cmd {
            Command::Choose(_) | Command::Cancel => {
                self.list_model = None;
                self.prompt_model = None;
            }
            Command::Answer(_) => self.prompt_model = None,
            _ => {}
        }
    }

    /// After a command acting on a list item (delete, rename, mark): the
    /// list stays only if the app showed it again (or asked for a name).
    pub(crate) fn close_list_unless_reshown(&mut self, effects: &[Effect]) {
        let reshown = effects
            .iter()
            .any(|e| matches!(e, Effect::ShowList { .. } | Effect::Prompt { .. }));
        if !reshown {
            self.list_model = None;
        }
    }

    /// A key in the list shown.
    pub(crate) fn list_key(&mut self, key: ListKey) -> Vec<Effect> {
        if self.list_model.is_none() {
            return vec![Effect::Redraw];
        }
        if let Some(effects) = self.menu_list_key(key) {
            return effects;
        }
        if let Some(effects) = self.browse_list_key(key) {
            return effects;
        }
        if self.settings_screen.is_some()
            && let Some(effects) = self.settings_list_key(key)
        {
            return effects;
        }
        if let Some(effects) = self.changes_list_key(key) {
            return effects;
        }
        if let Some(effects) = self.command_list_key(key) {
            return effects;
        }
        // The Say Status key previews the focused voice (crate::voice).
        if key == ListKey::Details && self.list == Some(crate::app::ListKind::Voices) {
            let n = self.list_model.as_ref().map_or(0, |l| l.selected);
            return self.preview_voice_row(n);
        }
        // Lists that filter as you type (the outline, the citation picker):
        // characters and Space add to the filter, Backspace removes one.
        if let Some(filter) = self.list_filter().map(str::to_owned) {
            match key {
                ListKey::Char(c) if !c.is_control() => {
                    return self.dispatch_inner(Command::FilterList(format!("{filter}{c}")));
                }
                ListKey::Backspace if !filter.is_empty() => {
                    let mut q = filter;
                    q.pop();
                    return self.dispatch_inner(Command::FilterList(q));
                }
                _ => {}
            }
        }
        let n = self.list_model.as_ref().map_or(0, |l| l.selected);
        let delta: isize = match key {
            ListKey::Char(c) if c.is_alphanumeric() => {
                // An accelerator, else the next item starting with it.
                if let Some(i) = self.list_accelerator(c) {
                    return self.choose_closing(i);
                }
                let found = self.list_model.as_mut().map(|l| l.jump_to_letter(c));
                match found {
                    Some(true) => {
                        let text = self
                            .list_model
                            .as_ref()
                            .and_then(|l| l.spoken_item_text(self.cat()))
                            .unwrap_or_default();
                        self.announce(&text, Priority::Assertive);
                    }
                    _ => {
                        let msg = self.msg_args(
                            "listmodel-no-item-starts",
                            &args!["letter" => c.to_string()],
                        );
                        self.announce(&msg, Priority::Polite);
                    }
                }
                return vec![Effect::Redraw];
            }
            ListKey::Char(' ') => return self.list_item_command(Command::MarkItem(n)),
            ListKey::Char(_) | ListKey::Left | ListKey::Right => return vec![Effect::Redraw],
            ListKey::Enter => return self.choose_closing(n),
            ListKey::Escape | ListKey::Backspace => {
                self.list_model = None;
                return self.dispatch_inner(Command::Cancel);
            }
            ListKey::Delete => return self.list_item_command(Command::DeleteItem(n)),
            ListKey::Introduce | ListKey::Details => return self.repeat_list_introduction(),
            // The file browser's own keys, elsewhere nothing.
            ListKey::ChooseHere | ListKey::Sort | ListKey::ShowAll => return vec![Effect::Redraw],
            ListKey::Rename => return self.list_item_command(Command::RenameItem(n)),
            ListKey::Up => -1,
            ListKey::Down => 1,
            ListKey::PageUp => -(LIST_PAGE as isize),
            ListKey::PageDown => LIST_PAGE as isize,
            ListKey::Home => isize::MIN / 2,
            ListKey::End => isize::MAX / 2,
        };
        let Some(list) = self.list_model.as_mut() else {
            return vec![Effect::Redraw];
        };
        let moved = list.step(delta);
        let text = self
            .list_model
            .as_ref()
            .and_then(|l| l.spoken_item_text(self.cat()))
            .unwrap_or_default();
        if moved {
            self.announce(&text, Priority::Assertive);
        } else {
            let edge = self.msg(if delta < 0 {
                "listmodel-top-of-list"
            } else {
                "listmodel-end-of-list"
            });
            // With the cursor on the status line, the Braille line is the
            // status line: keep the item there after the edge message, so
            // the display still says what the user is on.
            let sticky = self.cursor_placement() == textweaver_a11y::CursorPlacement::Status;
            let edge = if sticky && !text.is_empty() {
                format!("{edge} {text}")
            } else {
                edge
            };
            self.announce(&edge, Priority::Polite);
        }
        vec![Effect::Redraw]
    }

    /// Chooses item `n` of the list shown, closing it.
    fn choose_closing(&mut self, n: usize) -> Vec<Effect> {
        self.list_model = None;
        self.dispatch_inner(Command::Choose(n))
    }

    /// Runs a command on a list item; the list stays open only if the app
    /// shows it again.
    fn list_item_command(&mut self, cmd: Command) -> Vec<Effect> {
        let effects = self.dispatch_inner(cmd);
        self.close_list_unless_reshown(&effects);
        effects
    }

    /// Moves the list's focus to item `n` quietly (a GUI list that moved
    /// its own focus, which the screen reader already announced).
    pub(crate) fn list_focus(&mut self, n: usize) -> Vec<Effect> {
        if let Some(l) = self.list_model.as_mut() {
            l.selected = n.min(l.items.len().saturating_sub(1));
        }
        vec![Effect::Redraw]
    }

    /// A key in the prompt open.
    pub(crate) fn prompt_key(&mut self, key: PromptKey) -> Vec<Effect> {
        let Some(mb) = self.prompt_model.as_mut() else {
            return vec![Effect::Redraw];
        };
        let mut echo: Option<String> = None;
        let secret = mb.purpose.is_secret();
        match key {
            PromptKey::Enter => {
                let answer = mb.text();
                let purpose = mb.purpose;
                if !secret {
                    self.remember_answer(purpose, &answer);
                }
                self.prompt_model = None;
                return self.dispatch_inner(Command::Answer(answer));
            }
            PromptKey::Escape => {
                self.prompt_model = None;
                return self.dispatch_inner(Command::Cancel);
            }
            PromptKey::Char(c) if !c.is_control() => {
                mb.insert(c);
                mb.candidate = None;
                mb.candidate_from_empty = false;
                echo = Some(c.to_string());
            }
            PromptKey::Char(_) => {}
            PromptKey::Paste(text) => {
                for c in text.chars().filter(|c| !c.is_control()) {
                    mb.insert(c);
                }
                mb.candidate = None;
                echo = Some(text);
            }
            PromptKey::SetText(text) => {
                mb.set_text(&text);
                mb.candidate = None;
            }
            PromptKey::KillToStart => echo = Some(mb.kill_to_start()),
            PromptKey::KillToEnd => echo = Some(mb.kill_to_end()),
            PromptKey::DeleteWordBack => echo = Some(mb.delete_word_back()),
            PromptKey::Backspace => echo = mb.backspace().map(|c| c.to_string()),
            PromptKey::Delete => echo = mb.delete().map(|c| c.to_string()),
            PromptKey::Left => {
                mb.left();
                echo = mb.char_at_caret().map(|c| c.to_string());
            }
            PromptKey::Right => {
                mb.right();
                echo = mb.char_at_caret().map(|c| c.to_string());
            }
            PromptKey::Home => mb.home(),
            PromptKey::End => mb.end(),
            PromptKey::Tab | PromptKey::BackTab if mb.purpose == PromptPurpose::DocumentDetails => {
                let text = mb.text();
                let step = if key == PromptKey::Tab { 1 } else { -1 };
                return self.details_move(text, step);
            }
            PromptKey::Tab => {
                self.complete_prompt();
                return vec![Effect::Redraw];
            }
            PromptKey::BackTab => return vec![Effect::Redraw],
            PromptKey::Browse => return self.browse_for_prompt(),
            PromptKey::ShowMatches => {
                if mb.purpose == PromptPurpose::CommandPalette {
                    let query = mb.text();
                    self.prompt_model = None;
                    self.leave_prompt();
                    return self.palette_list(&query);
                }
                return vec![Effect::Redraw];
            }
            PromptKey::Up | PromptKey::Down if secret => return vec![Effect::Redraw],
            PromptKey::Up => {
                self.recall(-1);
                return vec![Effect::Redraw];
            }
            PromptKey::Down => {
                self.recall(1);
                return vec![Effect::Redraw];
            }
        }
        if secret {
            // Nothing typed into a secret prompt is said.
            echo = None;
        }
        if let Some(e) = echo.filter(|e| !e.is_empty()) {
            self.echo(&e);
        }
        vec![Effect::Redraw]
    }

    /// Records an answer in the prompt's history (blank answers are not
    /// kept; a repeated answer moves to the end).
    pub(crate) fn remember_answer(&mut self, purpose: PromptPurpose, answer: &str) {
        if answer.trim().is_empty() {
            return;
        }
        let hist = self.answers.entry(history_key(purpose)).or_default();
        hist.retain(|a| a != answer);
        hist.push(answer.to_owned());
        if hist.len() > PROMPT_HISTORY {
            hist.remove(0);
        }
    }

    /// Up and Down: palette candidates, or earlier answers to this prompt.
    fn recall(&mut self, delta: isize) {
        let Some(purpose) = self.prompt_model.as_ref().map(|m| m.purpose) else {
            return;
        };
        if purpose == PromptPurpose::CommandPalette {
            let Some(mb) = self.prompt_model.as_ref() else {
                return;
            };
            let fresh = mb.candidate.is_none().then(|| mb.text());
            let cands = fresh.map(|t| self.palette_candidates(&t));
            let Some(mb) = self.prompt_model.as_mut() else {
                return;
            };
            if let Some(c) = cands {
                mb.candidates = c;
            }
            if mb.candidates.is_empty() {
                let msg = self.msg("listmodel-no-matching-commands");
                self.announce(&msg, Priority::Polite);
                return;
            }
            let n = mb.candidates.len();
            let i = match mb.candidate {
                None if delta > 0 => 0,
                None => n - 1,
                Some(i) => (i as isize + delta).rem_euclid(n as isize) as usize,
            };
            mb.candidate = Some(i);
            let action = mb.candidates[i].0;
            let recent = mb.text().is_empty() || mb.candidate_from_empty;
            mb.candidate_from_empty = recent;
            mb.set_text(action.id());
            let recent = recent && self.recent_commands().contains(&action);
            let said = self.palette_said(action, recent);
            self.announce(&said, Priority::Assertive);
            return;
        }
        let hist = self
            .answers
            .get(&history_key(purpose))
            .cloned()
            .unwrap_or_default();
        let Some(mb) = self.prompt_model.as_mut() else {
            return;
        };
        if hist.is_empty() {
            let msg = self.msg("listmodel-no-earlier-entries");
            self.announce(&msg, Priority::Polite);
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
            self.msg("nav-blank")
        } else {
            text
        };
        self.announce(&spoken, Priority::Assertive);
    }

    /// Tab: in the command palette, completes to the longest common prefix
    /// of the matching command ids and says what matches; in a file prompt
    /// (Open, Save As, Insert image), completes the path.
    fn complete_prompt(&mut self) {
        let Some(mb) = self.prompt_model.as_ref() else {
            return;
        };
        let purpose = mb.purpose;
        let typed = mb.text();
        if purpose.is_path() {
            let cwd = std::env::current_dir().unwrap_or_default();
            let (done, spoken) = crate::path_complete::complete_in(self.cat(), &typed, &cwd);
            if let (Some(text), Some(mb)) = (done, self.prompt_model.as_mut()) {
                mb.set_text(&text);
            }
            self.announce(&spoken, Priority::Assertive);
            return;
        }
        if purpose != PromptPurpose::CommandPalette {
            return;
        }
        let cands = self.palette_candidates(&typed);
        match cands.as_slice() {
            [] => {
                let msg = self.msg("listmodel-no-matching-commands");
                self.announce(&msg, Priority::Polite);
            }
            [(a, _)] => {
                if let Some(mb) = self.prompt_model.as_mut() {
                    mb.set_text(a.id());
                }
                let said = self.palette_said(*a, false);
                self.announce(&said, Priority::Assertive);
            }
            many => {
                let ids: Vec<&str> = many.iter().map(|(a, _)| a.id()).collect();
                let prefix = common_prefix(&ids);
                if prefix.len() > typed.len()
                    && ids.iter().all(|i| i.starts_with(&prefix))
                    && let Some(mb) = self.prompt_model.as_mut()
                {
                    mb.set_text(&prefix);
                }
                let first: Vec<String> = many
                    .iter()
                    .take(5)
                    .map(|(a, _)| crate::menu::action_name(self.cat(), *a))
                    .collect();
                let msg = self.msg_args(
                    "listmodel-command-matches",
                    &args!["n" => many.len(), "names" => first.join(", ")],
                );
                self.announce(&msg, Priority::Assertive);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_editing() {
        let mut m = PromptModel::new("Find", PromptPurpose::Find);
        for c in "hello world".chars() {
            m.insert(c);
        }
        assert_eq!(m.delete_word_back(), "world");
        assert_eq!(m.text(), "hello ");
        m.home();
        m.right();
        assert_eq!(m.kill_to_end(), "ello ");
        assert_eq!(m.backspace(), Some('h'));
        assert_eq!(m.backspace(), None);
        m.set_text("abc");
        m.left();
        assert_eq!(m.kill_to_start(), "ab");
        assert_eq!(m.text(), "c");
    }

    #[test]
    fn list_steps_clamp() {
        let mut l = ListModel::new("t", vec!["a".into(), "b".into()]);
        assert!(!l.step(-1));
        assert!(l.step(5));
        assert_eq!(l.current(), Some("b"));
        assert_eq!(l.spoken_item().as_deref(), Some("2 of 2, b"));
        assert_eq!(ListModel::new("t", Vec::new()).spoken_item(), None);
    }

    #[test]
    fn first_letter_jumps_wrap_and_ignore_case() {
        let mut l = ListModel::new(
            "t",
            vec![
                "Apple".into(),
                "banana".into(),
                "Avocado".into(),
                "\u{201c}Cherry\u{201d}".into(),
            ],
        );
        assert!(l.jump_to_letter('a'));
        assert_eq!(l.current(), Some("Avocado"));
        assert!(l.jump_to_letter('A'));
        assert_eq!(l.current(), Some("Apple"));
        assert!(l.jump_to_letter('c'));
        assert_eq!(l.selected, 3);
        assert!(!l.jump_to_letter('z'));
        assert_eq!(l.selected, 3);
        assert!(!ListModel::new("t", Vec::new()).jump_to_letter('a'));
    }

    #[test]
    fn prefix() {
        assert_eq!(common_prefix(&["next_list", "next_link"]), "next_li");
        assert_eq!(common_prefix(&["a"]), "a");
    }
}
