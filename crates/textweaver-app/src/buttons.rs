//! The window's buttons, chosen by the user (B1-cb): what the header and
//! the toolbar hold, and the model of the "Customize buttons" list.
//!
//! - **One list.** `[gui] header_buttons` and `[gui] toolbar_buttons` hold
//!   command ids in order; their defaults are the store's
//!   [`DEFAULT_HEADER_BUTTONS`] and [`DEFAULT_TOOLBAR_BUTTONS`]. The
//!   window draws [`bar_actions`], and the terminal reader's key hints
//!   follow the toolbar's, so both frontends and the settings read the
//!   same list.
//! - **What is shown.** Any command with a short name (a `name-*`
//!   message, as the palette and the menus show it) can be a button. An
//!   id this version does not know, from an older or newer one, is kept in
//!   the setting but not shown, and a command already on a bar is not
//!   shown a second time (the header comes first).
//! - **The model.** [`add`], [`remove`], [`move_button`] and [`reset`]
//!   change the settings by the shown position and keep the unknown ids
//!   where they are; the app's methods ([`App::add_button`] and the rest)
//!   save the change and return what to say, in words. The
//!   "Customize buttons" command (View menu and palette) shows the same
//!   model as a list on the shared list model, which every frontend can
//!   show: Enter on a button offers Move up, Move down and Remove; Delete
//!   removes it; the last rows add a button to either bar or reset both.

use textweaver_keymap::ActionId;
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;
use textweaver_store::{DEFAULT_HEADER_BUTTONS, DEFAULT_TOOLBAR_BUTTONS, GuiSettings};

use crate::app::{App, ListKind};
use crate::command::Effect;

/// One of the window's two bars of buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bar {
    /// The header, above the document (Open, Font, Edit, Settings,
    /// Commands by default).
    Header,
    /// The toolbar, below the document (Play, Stop, the previous and next
    /// paragraph, Slower, Faster by default).
    Toolbar,
}

impl Bar {
    /// Both bars, header first.
    pub const ALL: [Bar; 2] = [Bar::Header, Bar::Toolbar];

    /// Its name in messages' selectors: `header` or `toolbar`.
    pub fn key(self) -> &'static str {
        match self {
            Bar::Header => "header",
            Bar::Toolbar => "toolbar",
        }
    }

    /// Its default buttons, as command ids.
    pub fn defaults(self) -> &'static [&'static str] {
        match self {
            Bar::Header => DEFAULT_HEADER_BUTTONS,
            Bar::Toolbar => DEFAULT_TOOLBAR_BUTTONS,
        }
    }

    fn ids(self, gui: &GuiSettings) -> &Vec<String> {
        match self {
            Bar::Header => &gui.header_buttons,
            Bar::Toolbar => &gui.toolbar_buttons,
        }
    }

    fn ids_mut(self, gui: &mut GuiSettings) -> &mut Vec<String> {
        match self {
            Bar::Header => &mut gui.header_buttons,
            Bar::Toolbar => &mut gui.toolbar_buttons,
        }
    }
}

/// True when `action` can be a button: it has a short name (a `name-*`
/// message), as the palette and the menus show it.
pub fn can_be_button(action: ActionId) -> bool {
    Catalog::english().has(&format!("name-{}", action.id().replace('_', "-")))
}

/// The commands `bar` shows, in order, with where each is stored in its
/// setting: unknown ids, commands with no short name, and commands already
/// shown (on this bar or, for the toolbar, on the header) are skipped.
fn shown(gui: &GuiSettings, bar: Bar) -> Vec<(ActionId, usize)> {
    let mut seen: Vec<ActionId> = Vec::new();
    let mut out = Vec::new();
    for b in Bar::ALL {
        for (i, id) in b.ids(gui).iter().enumerate() {
            let Some(a) = ActionId::from_id(id) else {
                continue;
            };
            if !can_be_button(a) || seen.contains(&a) {
                continue;
            }
            seen.push(a);
            if b == bar {
                out.push((a, i));
            }
        }
        if b == bar {
            break;
        }
    }
    out
}

/// The commands `bar` shows, in order (see the module notes).
pub fn bar_actions(gui: &GuiSettings, bar: Bar) -> Vec<ActionId> {
    shown(gui, bar).into_iter().map(|(a, _)| a).collect()
}

/// The bar that shows `action`, if any.
pub fn bar_of(gui: &GuiSettings, action: ActionId) -> Option<Bar> {
    Bar::ALL
        .into_iter()
        .find(|&b| bar_actions(gui, b).contains(&action))
}

/// Why a change to a bar was not made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonsRefusal {
    /// The command is already shown on this bar.
    AlreadyOn(Bar),
    /// The command has no short name, so it cannot be a button.
    NoName,
    /// No button at that position.
    NoSuchButton,
    /// The button is already first.
    AlreadyFirst,
    /// The button is already last.
    AlreadyLast,
}

/// Adds `action` at the end of `bar`; returns its shown position (from 0).
pub fn add(gui: &mut GuiSettings, bar: Bar, action: ActionId) -> Result<usize, ButtonsRefusal> {
    if !can_be_button(action) {
        return Err(ButtonsRefusal::NoName);
    }
    if let Some(on) = bar_of(gui, action) {
        return Err(ButtonsRefusal::AlreadyOn(on));
    }
    bar.ids_mut(gui).push(action.id().to_owned());
    Ok(bar_actions(gui, bar).len().saturating_sub(1))
}

/// Removes the button shown at `index` on `bar`; returns its command.
/// Unknown ids stay.
pub fn remove(gui: &mut GuiSettings, bar: Bar, index: usize) -> Result<ActionId, ButtonsRefusal> {
    let (a, at) = *shown(gui, bar)
        .get(index)
        .ok_or(ButtonsRefusal::NoSuchButton)?;
    bar.ids_mut(gui).remove(at);
    Ok(a)
}

/// Moves the button shown at `index` on `bar` one place up (towards the
/// start) or down; returns its new shown position. It swaps places with
/// its shown neighbor, so unknown ids between them stay where they are.
pub fn move_button(
    gui: &mut GuiSettings,
    bar: Bar,
    index: usize,
    up: bool,
) -> Result<usize, ButtonsRefusal> {
    let list = shown(gui, bar);
    let (_, at) = *list.get(index).ok_or(ButtonsRefusal::NoSuchButton)?;
    let to = if up {
        index.checked_sub(1).ok_or(ButtonsRefusal::AlreadyFirst)?
    } else {
        Some(index + 1)
            .filter(|&t| t < list.len())
            .ok_or(ButtonsRefusal::AlreadyLast)?
    };
    let (_, other) = list[to];
    bar.ids_mut(gui).swap(at, other);
    Ok(to)
}

/// Puts both bars back to their defaults (unknown ids go too).
pub fn reset(gui: &mut GuiSettings) {
    for bar in Bar::ALL {
        *bar.ids_mut(gui) = bar.defaults().iter().map(|&s| s.to_owned()).collect();
    }
}

/// What the "Customize buttons" list shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ButtonsList {
    /// Every button, header first, then the rows that add or reset.
    Main(Vec<Row>),
    /// What to do with the button at `index` on `bar`.
    Button {
        /// The bar.
        bar: Bar,
        /// Its shown position.
        index: usize,
    },
    /// The commands that can be added to `bar`, by name.
    Add {
        /// The bar.
        bar: Bar,
        /// The commands, in the order shown.
        actions: Vec<ActionId>,
    },
}

/// A row of the main list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Row {
    /// The button at a shown position on a bar.
    Button(Bar, usize),
    /// Add a button to a bar.
    Add(Bar),
    /// Reset both bars to the defaults.
    Reset,
}

/// The rows of a button's own list: Move up, Move down, Remove.
const BUTTON_ROWS: [&str; 3] = ["buttons-move-up", "buttons-move-down", "buttons-remove"];

impl App {
    /// The commands `bar` shows, in order: what the window draws.
    pub fn bar_buttons(&self, bar: Bar) -> Vec<ActionId> {
        bar_actions(&self.settings.gui, bar)
    }

    /// Adds `action` to the end of `bar`, saves, and returns what to say
    /// ("Find added, toolbar 7 of 7."), or why not.
    pub fn add_button(&mut self, bar: Bar, action: ActionId) -> String {
        let name = crate::menu::action_name(self.cat(), action);
        let mut gui = self.settings.gui.clone();
        match add(&mut gui, bar, action) {
            Ok(pos) => {
                let count = bar_actions(&gui, bar).len();
                let _ = self.update_settings(|s| s.gui = gui);
                self.msg_args(
                    "buttons-added",
                    &args!["name" => name, "bar" => bar.key(), "pos" => pos + 1, "count" => count],
                )
            }
            Err(r) => self.buttons_refusal(r, &name),
        }
    }

    /// Removes the button shown at `index` on `bar`, saves, and returns
    /// what to say.
    pub fn remove_button(&mut self, bar: Bar, index: usize) -> String {
        let mut gui = self.settings.gui.clone();
        match remove(&mut gui, bar, index) {
            Ok(a) => {
                let _ = self.update_settings(|s| s.gui = gui);
                let name = crate::menu::action_name(self.cat(), a);
                self.msg_args(
                    "buttons-removed",
                    &args!["name" => name, "bar" => bar.key()],
                )
            }
            Err(r) => self.buttons_refusal(r, ""),
        }
    }

    /// Moves the button shown at `index` on `bar` up or down one place,
    /// saves, and returns what to say ("Stop moved, 1 of 6.").
    pub fn move_bar_button(&mut self, bar: Bar, index: usize, up: bool) -> String {
        let name = bar_actions(&self.settings.gui, bar)
            .get(index)
            .map(|&a| crate::menu::action_name(self.cat(), a))
            .unwrap_or_default();
        let mut gui = self.settings.gui.clone();
        match move_button(&mut gui, bar, index, up) {
            Ok(to) => {
                let count = bar_actions(&gui, bar).len();
                let _ = self.update_settings(|s| s.gui = gui);
                self.msg_args(
                    "buttons-moved",
                    &args!["name" => name, "pos" => to + 1, "count" => count],
                )
            }
            Err(r) => self.buttons_refusal(r, &name),
        }
    }

    /// Puts both bars back to their defaults, saves, and returns what to
    /// say.
    pub fn reset_buttons(&mut self) -> String {
        let _ = self.update_settings(|s| reset(&mut s.gui));
        self.msg("buttons-reset-done")
    }

    fn buttons_refusal(&self, r: ButtonsRefusal, name: &str) -> String {
        match r {
            ButtonsRefusal::AlreadyOn(bar) => self.msg_args(
                "buttons-already",
                &args!["name" => name, "bar" => bar.key()],
            ),
            ButtonsRefusal::NoName => self.msg("buttons-no-name"),
            ButtonsRefusal::NoSuchButton => self.msg("buttons-no-button"),
            ButtonsRefusal::AlreadyFirst => self.msg_args("buttons-first", &args!["name" => name]),
            ButtonsRefusal::AlreadyLast => self.msg_args("buttons-last", &args!["name" => name]),
        }
    }

    /// The "Customize buttons" command: the list of every button, said
    /// after its introduction.
    pub(crate) fn customize_buttons(&mut self) -> Vec<Effect> {
        let n: usize = Bar::ALL.iter().map(|&b| self.bar_buttons(b).len()).sum();
        let intro = self.msg_args("buttons-intro", &args!["n" => n]);
        self.tell(&intro);
        self.show_buttons_list(0)
    }

    /// Shows the main list focused on row `focus`.
    fn show_buttons_list(&mut self, focus: usize) -> Vec<Effect> {
        let mut rows = Vec::new();
        let mut items = Vec::new();
        for bar in Bar::ALL {
            let actions = self.bar_buttons(bar);
            let count = actions.len();
            for (i, a) in actions.into_iter().enumerate() {
                let name = crate::menu::action_name(self.cat(), a);
                items.push(self.msg_args(
                    "buttons-row",
                    &args!["name" => name, "bar" => bar.key(), "pos" => i + 1, "count" => count],
                ));
                rows.push(Row::Button(bar, i));
            }
        }
        for (row, id) in [
            (Row::Add(Bar::Header), "buttons-add-header"),
            (Row::Add(Bar::Toolbar), "buttons-add-toolbar"),
            (Row::Reset, "buttons-reset"),
        ] {
            items.push(self.msg(id));
            rows.push(row);
        }
        self.pending_list_focus = Some(focus.min(rows.len().saturating_sub(1)));
        self.list = Some(ListKind::Buttons(ButtonsList::Main(rows)));
        vec![Effect::ShowList {
            title: self.msg("buttons-title"),
            items,
        }]
    }

    /// The main list again after a change, which is said first, focused
    /// on the button at `index` on `bar` (or the nearest row).
    fn reshow_buttons(&mut self, said: &str, bar: Bar, index: usize) -> Vec<Effect> {
        self.tell(said);
        let before = match bar {
            Bar::Header => 0,
            Bar::Toolbar => self.bar_buttons(Bar::Header).len(),
        };
        self.list_reshow_quiet = true;
        self.show_buttons_list(before + index)
    }

    /// Enter in one of the lists.
    pub(crate) fn choose_buttons(&mut self, list: ButtonsList, n: usize) -> Vec<Effect> {
        match list {
            ButtonsList::Main(rows) => match rows.get(n).copied() {
                Some(Row::Button(bar, index)) => {
                    let name = self
                        .bar_buttons(bar)
                        .get(index)
                        .map(|&a| crate::menu::action_name(self.cat(), a))
                        .unwrap_or_default();
                    let items = BUTTON_ROWS.iter().map(|id| self.msg(id)).collect();
                    self.list = Some(ListKind::Buttons(ButtonsList::Button { bar, index }));
                    vec![Effect::ShowList { title: name, items }]
                }
                Some(Row::Add(bar)) => self.show_add_list(bar),
                Some(Row::Reset) => {
                    let said = self.reset_buttons();
                    self.reshow_buttons(&said, Bar::Header, 0)
                }
                None => vec![Effect::Redraw],
            },
            ButtonsList::Button { bar, index } => {
                if n >= 2 {
                    let said = self.remove_button(bar, index);
                    return self.reshow_buttons(&said, bar, index);
                }
                let up = n == 0;
                let before = self.bar_buttons(bar);
                let said = self.move_bar_button(bar, index, up);
                let moved = self.bar_buttons(bar) != before;
                let at = match (moved, up) {
                    (false, _) => index,
                    (true, true) => index.saturating_sub(1),
                    (true, false) => index + 1,
                };
                self.reshow_buttons(&said, bar, at)
            }
            ButtonsList::Add { bar, actions } => match actions.get(n) {
                Some(&a) => {
                    let said = self.add_button(bar, a);
                    let at = self.bar_buttons(bar).len().saturating_sub(1);
                    self.reshow_buttons(&said, bar, at)
                }
                None => vec![Effect::Redraw],
            },
        }
    }

    /// Delete in the main list: removes the button.
    pub(crate) fn delete_buttons_item(&mut self, list: ButtonsList, n: usize) -> Vec<Effect> {
        if let ButtonsList::Main(rows) = &list
            && let Some(Row::Button(bar, index)) = rows.get(n).copied()
        {
            let said = self.remove_button(bar, index);
            return self.reshow_buttons(&said, bar, index);
        }
        self.list = Some(ListKind::Buttons(list));
        let msg = self.msg("study-nothing-to-delete");
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    /// The commands that can be added to `bar`: every command with a short
    /// name that is on neither bar, by name.
    fn show_add_list(&mut self, bar: Bar) -> Vec<Effect> {
        let gui = &self.settings.gui;
        let mut actions: Vec<(String, ActionId)> = ActionId::ALL
            .iter()
            .copied()
            .filter(|&a| can_be_button(a) && bar_of(gui, a).is_none())
            .map(|a| (crate::menu::action_name(self.cat(), a), a))
            .collect();
        if actions.is_empty() {
            let msg = self.msg("buttons-none-to-add");
            return self.reshow_buttons(&msg, bar, 0);
        }
        actions.sort_by_key(|(name, _)| name.to_lowercase());
        let (items, actions): (Vec<String>, Vec<ActionId>) = actions.into_iter().unzip();
        let title = self.msg(match bar {
            Bar::Header => "buttons-add-header",
            Bar::Toolbar => "buttons-add-toolbar",
        });
        let intro = self.msg_args("buttons-add-intro", &args!["n" => items.len()]);
        self.tell(&intro);
        self.list = Some(ListKind::Buttons(ButtonsList::Add { bar, actions }));
        vec![Effect::ShowList { title, items }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|&s| s.to_owned()).collect()
    }

    #[test]
    fn the_defaults_are_the_bars_buttons() {
        let gui = GuiSettings::default();
        assert_eq!(
            bar_actions(&gui, Bar::Header),
            [
                ActionId::Open,
                ActionId::ChooseFont,
                ActionId::ToggleEditMode,
                ActionId::Settings,
                ActionId::CommandPalette,
            ]
        );
        // Paragraph steps, not sentences, by the owner's choice.
        assert_eq!(
            bar_actions(&gui, Bar::Toolbar),
            [
                ActionId::PlayPause,
                ActionId::Stop,
                ActionId::PreviousParagraph,
                ActionId::NextParagraph,
                ActionId::RateDown,
                ActionId::RateUp,
            ]
        );
        // Every default is a known command with a short name.
        for bar in Bar::ALL {
            assert_eq!(bar_actions(&gui, bar).len(), bar.defaults().len());
        }
    }

    #[test]
    fn unknown_ids_are_kept_but_not_shown() {
        let mut gui = GuiSettings {
            toolbar_buttons: ids(&["stop", "from_the_future", "play_pause"]),
            ..GuiSettings::default()
        };
        assert_eq!(
            bar_actions(&gui, Bar::Toolbar),
            [ActionId::Stop, ActionId::PlayPause]
        );
        // Moving and removing work by shown position and keep the id.
        assert_eq!(move_button(&mut gui, Bar::Toolbar, 1, true), Ok(0));
        assert_eq!(
            gui.toolbar_buttons,
            ids(&["play_pause", "from_the_future", "stop"])
        );
        assert_eq!(remove(&mut gui, Bar::Toolbar, 1), Ok(ActionId::Stop));
        assert_eq!(gui.toolbar_buttons, ids(&["play_pause", "from_the_future"]));
        assert_eq!(add(&mut gui, Bar::Toolbar, ActionId::Find), Ok(1));
        assert_eq!(
            gui.toolbar_buttons,
            ids(&["play_pause", "from_the_future", "find"])
        );
    }

    #[test]
    fn no_command_is_shown_twice() {
        let mut gui = GuiSettings {
            header_buttons: ids(&["open", "stop", "open"]),
            toolbar_buttons: ids(&["play_pause", "stop", "open"]),
            ..GuiSettings::default()
        };
        assert_eq!(
            bar_actions(&gui, Bar::Header),
            [ActionId::Open, ActionId::Stop]
        );
        assert_eq!(bar_actions(&gui, Bar::Toolbar), [ActionId::PlayPause]);
        assert_eq!(
            add(&mut gui, Bar::Toolbar, ActionId::Stop),
            Err(ButtonsRefusal::AlreadyOn(Bar::Header))
        );
        assert_eq!(
            add(&mut gui, Bar::Header, ActionId::PlayPause),
            Err(ButtonsRefusal::AlreadyOn(Bar::Toolbar))
        );
    }

    #[test]
    fn each_operation_changes_the_setting() {
        let mut gui = GuiSettings::default();
        assert_eq!(
            move_button(&mut gui, Bar::Toolbar, 0, true),
            Err(ButtonsRefusal::AlreadyFirst)
        );
        assert_eq!(
            move_button(&mut gui, Bar::Toolbar, 5, false),
            Err(ButtonsRefusal::AlreadyLast)
        );
        assert_eq!(
            move_button(&mut gui, Bar::Toolbar, 9, false),
            Err(ButtonsRefusal::NoSuchButton)
        );
        assert_eq!(move_button(&mut gui, Bar::Toolbar, 2, false), Ok(3));
        assert_eq!(
            bar_actions(&gui, Bar::Toolbar)[3],
            ActionId::PreviousParagraph
        );
        assert_eq!(remove(&mut gui, Bar::Header, 1), Ok(ActionId::ChooseFont));
        assert_eq!(bar_actions(&gui, Bar::Header).len(), 4);
        assert_eq!(
            remove(&mut gui, Bar::Header, 9),
            Err(ButtonsRefusal::NoSuchButton)
        );
        assert_eq!(add(&mut gui, Bar::Header, ActionId::ChooseFont), Ok(4));
        reset(&mut gui);
        assert_eq!(gui, GuiSettings::default());
    }

    /// The "Customize buttons" list: each change is said in words, saved,
    /// and the list comes back on the button.
    #[test]
    fn the_list_moves_removes_adds_and_resets() {
        use crate::AppConfig;
        use crate::command::Command;
        use crate::list_model::ListKey;
        let mut app = App::new(AppConfig::for_tests());
        app.dispatch(Command::Action(ActionId::CustomizeButtons));
        let items = app.list_model().unwrap().items.clone();
        assert_eq!(items.len(), 5 + 6 + 3, "{items:?}");
        assert_eq!(items[0], "Open, header 1 of 5");
        assert_eq!(items[7], "Previous paragraph, toolbar 3 of 6");
        assert_eq!(items[13], "Reset both bars to their defaults");
        // Enter on Stop, then Move up.
        app.dispatch(Command::Choose(6));
        assert_eq!(
            app.list_model().unwrap().items,
            ["Move up", "Move down", "Remove"]
        );
        app.dispatch(Command::Choose(0));
        assert_eq!(app.status_text(), "Stop moved, 1 of 6.");
        assert_eq!(app.bar_buttons(Bar::Toolbar)[0], ActionId::Stop);
        assert_eq!(app.list_model().unwrap().selected, 5);
        // Already first: said, nothing changes.
        app.dispatch(Command::Choose(5));
        app.dispatch(Command::Choose(0));
        assert_eq!(app.status_text(), "Stop is already first.");
        // Delete removes the focused button.
        app.dispatch(Command::ListKey(ListKey::Delete));
        assert_eq!(app.status_text(), "Stop removed from the toolbar.");
        assert_eq!(app.bar_buttons(Bar::Toolbar).len(), 5);
        assert!(
            !app.settings()
                .gui
                .toolbar_buttons
                .contains(&"stop".to_owned())
        );
        // Add a button to the toolbar: Stop is offered again, by name.
        let add_row = 5 + 5 + 1;
        app.dispatch(Command::Choose(add_row));
        let offered = app.list_model().unwrap().items.clone();
        let stop = offered.iter().position(|i| i == "Stop").unwrap();
        assert!(!offered.contains(&"Open".to_owned()), "{offered:?}");
        app.dispatch(Command::Choose(stop));
        assert_eq!(app.status_text(), "Stop added, toolbar 6 of 6.");
        assert_eq!(app.list_model().unwrap().selected, 10);
        // Reset both bars.
        app.dispatch(Command::Choose(13));
        assert_eq!(app.status_text(), "Buttons reset to their defaults.");
        assert_eq!(app.settings().gui, GuiSettings::default());
        // The model's own refusals are said in words too.
        assert_eq!(
            app.add_button(Bar::Header, ActionId::Stop),
            "Stop is already on the toolbar."
        );
    }

    #[test]
    fn only_commands_with_a_short_name_can_be_buttons() {
        assert!(can_be_button(ActionId::NextParagraph));
        let nameless = ActionId::ALL.iter().copied().find(|&a| !can_be_button(a));
        if let Some(a) = nameless {
            let mut gui = GuiSettings::default();
            assert_eq!(add(&mut gui, Bar::Toolbar, a), Err(ButtonsRefusal::NoName));
        }
    }
}
