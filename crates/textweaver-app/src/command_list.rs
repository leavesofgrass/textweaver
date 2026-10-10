//! One list model and one row format for the command palette and the
//! keyboard shortcuts list (beta 1, from the owner's alpha.9 testing).
//!
//! Each row is the command's short name, then its key: "Find next, F3",
//! read in that order and drawn with the key at the right edge
//! ([`ListModel::columns`]). The long explanation is never in the row: F1
//! on the row says it, and it is the row's description
//! ([`ListModel::descriptions`]). Both frontends draw the same rows:
//!
//! - The keyboard shortcuts list (the Keyboard Help command, `?` in the
//!   terminal reader) filters as you type, like Settings, matching the
//!   command's name, its keys, its group, and its menu, and says how many
//!   match ("12 of 226 commands match."). It keeps its groups (Reading,
//!   Navigation, and so on): Page Down and Page Up move to the next and
//!   previous group, and moving into a group says its name and size first.
//! - The command palette's matches (and its list, Ctrl+L) use the same
//!   rows; F1 on one says what it does.
//!
//! [`ListModel::columns`]: crate::ListModel::columns
//! [`ListModel::descriptions`]: crate::ListModel::descriptions

use textweaver_a11y::Priority;
use textweaver_keymap::ActionId;
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;

use crate::app::{App, ListKind};
use crate::command::Effect;
use crate::help::{category_title, help_order, main_chord, mark_chord, marked_entry, palette_line};
use crate::list_model::ListKey;

/// Each row's short name and key ([`crate::ListModel::columns`]).
type Columns = Vec<(String, String)>;

/// One row of the command palette and the keyboard shortcuts list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandRow {
    /// The command.
    pub action: ActionId,
    /// Its short name: "Find next".
    pub name: String,
    /// Its main key, written ("F3"); `None` for a command without keys,
    /// which the command palette runs.
    pub key: Option<String>,
    /// Its group in the keyboard shortcuts list: "Search".
    pub group: String,
    /// The long explanation, for F1 and the row's description, with every
    /// key: "Find next: F3 or Ctrl+G. Find the next match. Search".
    pub help: String,
}

impl CommandRow {
    /// The row as shown and read: "Find next, F3" (the name alone without
    /// a key).
    pub fn text(&self, c: &Catalog) -> String {
        palette_line(c, self.action, self.key.clone(), false)
    }
}

impl App {
    /// The row for `a` in the command palette and the keyboard shortcuts
    /// list: its short name, main key, group, and long explanation.
    pub fn command_row(&self, a: ActionId) -> CommandRow {
        let c = self.cat();
        CommandRow {
            action: a,
            name: crate::menu::action_name(c, a),
            key: main_chord(&self.keymap, a).map(|ch| ch.to_string()),
            group: category_title(c, a.category()),
            help: crate::help::written_text(&marked_entry(c, &self.keymap, a)).into_owned(),
        }
    }

    /// True when `a` matches every word of `query` in its name (in the
    /// interface's language or English), any of its keys, its group, or
    /// its menu path.
    fn command_matches(&self, a: ActionId, query: &str) -> bool {
        let c = self.cat();
        let mut hay = vec![
            crate::menu::action_name(c, a),
            crate::menu::action_name(&Catalog::english(), a),
            category_title(c, a.category()),
        ];
        hay.extend(self.keymap.chords_for(a).iter().map(ToString::to_string));
        hay.extend(self.menu_path_of(a));
        crate::lists::matches(&hay.join("\n"), query)
    }

    /// The Keyboard Help command: every command, by group, each row its
    /// name and key; typing filters.
    pub(crate) fn keyboard_help(&mut self) -> Vec<Effect> {
        self.keys_filter.clear();
        let msg = self.msg_args("help-shortcuts-intro", &args!["n" => ActionId::ALL.len()]);
        self.tell(&msg);
        self.show_keys_list()
    }

    /// The keyboard shortcuts list as the filter leaves it.
    fn show_keys_list(&mut self) -> Vec<Effect> {
        let filter = self.keys_filter.clone();
        let actions: Vec<ActionId> = help_order()
            .into_iter()
            .filter(|&a| self.command_matches(a, &filter))
            .collect();
        let c = self.cat();
        // The key is marked: the list shows "Ctrl+O", the voice says
        // "Control O".
        let items = actions
            .iter()
            .map(|&a| {
                let key = main_chord(&self.keymap, a).map(|ch| mark_chord(c, &ch));
                palette_line(c, a, key, false)
            })
            .collect();
        let title = if filter.trim().is_empty() {
            self.msg("help-shortcuts-title")
        } else {
            self.msg_args(
                "help-shortcuts-title-matching",
                &args!["filter" => filter.trim()],
            )
        };
        self.list = Some(ListKind::Actions(actions));
        vec![Effect::ShowList { title, items }]
    }

    /// True while a list of commands is shown: F1 explains its focused
    /// row there, so the Say Status key repeats the introduction
    /// (`ListKey::Details`).
    pub(crate) fn command_list_shown(&self) -> bool {
        matches!(
            self.list,
            Some(ListKind::Actions(_)) | Some(ListKind::Palette(_))
        )
    }

    /// The keyboard shortcuts list's filter, while it is shown.
    pub(crate) fn keys_filter(&self) -> Option<&str> {
        matches!(self.list, Some(ListKind::Actions(_))).then_some(self.keys_filter.as_str())
    }

    /// The keyboard shortcuts list's filter changed to `query`: the rows
    /// that match, and how many ("12 of 226 commands match.").
    pub(crate) fn filter_keys(&mut self, query: String) -> Vec<Effect> {
        self.keys_filter = query;
        let effects = self.show_keys_list();
        let n = match &self.list {
            Some(ListKind::Actions(a)) => a.len(),
            _ => 0,
        };
        let query = self.keys_filter.trim().to_owned();
        let msg = if query.is_empty() {
            self.msg_args("help-shortcuts-filter-cleared", &args!["n" => n])
        } else if n == 0 {
            self.msg_args("help-shortcuts-filter-none", &args!["query" => query])
        } else {
            self.msg_args(
                "help-shortcuts-filter-match",
                &args!["n" => n, "total" => ActionId::ALL.len()],
            )
        };
        self.tell(&msg);
        effects
    }

    /// The name and key columns and the descriptions of the rows of a
    /// command list (the keyboard shortcuts list, the palette's list), for
    /// its list model; `None` for other lists.
    pub(crate) fn command_columns(&self) -> Option<(Columns, Vec<String>)> {
        let (Some(ListKind::Actions(actions)) | Some(ListKind::Palette(actions))) = &self.list
        else {
            return None;
        };
        let rows: Vec<CommandRow> = actions.iter().map(|&a| self.command_row(a)).collect();
        Some((
            rows.iter()
                .map(|r| (r.name.clone(), r.key.clone().unwrap_or_default()))
                .collect(),
            rows.into_iter().map(|r| r.help).collect(),
        ))
    }

    /// Keys a command list handles itself: F1 says the focused row's long
    /// explanation; in the keyboard shortcuts list, Page Down and Page Up
    /// move by group, and Up and Down say a group's name on entering it.
    /// `None` leaves the key to the list.
    pub(crate) fn command_list_key(&mut self, key: ListKey) -> Option<Vec<Effect>> {
        let (grouped, actions) = match &self.list {
            Some(ListKind::Actions(a)) => (true, a.clone()),
            Some(ListKind::Palette(a)) => (false, a.clone()),
            _ => return None,
        };
        let now = self.list_model.as_ref().map_or(0, |l| l.selected);
        if key == ListKey::Introduce {
            let Some(&a) = actions.get(now) else {
                return Some(self.repeat_list_introduction());
            };
            let help = marked_entry(self.cat(), &self.keymap, a);
            self.tell(&help);
            return Some(vec![Effect::Redraw]);
        }
        if !grouped || actions.is_empty() {
            return None;
        }
        let group_of = |i: usize| actions[i].category();
        let target = match key {
            ListKey::Down if now + 1 < actions.len() && group_of(now + 1) != group_of(now) => {
                now + 1
            }
            ListKey::Up if now > 0 && group_of(now - 1) != group_of(now) => now - 1,
            ListKey::PageDown => {
                (now + 1..actions.len()).find(|&i| group_of(i) != group_of(now))?
            }
            ListKey::PageUp => {
                let start = group_start(&actions, now);
                if start < now {
                    start
                } else if start == 0 {
                    return None;
                } else {
                    group_start(&actions, start - 1)
                }
            }
            _ => return None,
        };
        if let Some(l) = self.list_model.as_mut() {
            l.selected = target;
        }
        let cat = group_of(target);
        let size = actions.iter().filter(|a| a.category() == cat).count();
        let item = self
            .list_model
            .as_ref()
            .and_then(|l| l.spoken_item_text(self.cat()))
            .unwrap_or_default();
        let msg = self.msg_args(
            "help-shortcuts-group-item",
            &args![
                "group" => category_title(self.cat(), cat),
                "n" => size,
                "item" => item
            ],
        );
        self.announce(&msg, Priority::Assertive);
        Some(vec![Effect::Redraw])
    }
}

/// The first row of the group holding row `i`.
fn group_start(actions: &[ActionId], i: usize) -> usize {
    let cat = actions[i].category();
    (0..=i)
        .rev()
        .take_while(|&j| actions[j].category() == cat)
        .last()
        .unwrap_or(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_group_starts_where_its_category_does() {
        let a = [
            ActionId::PlayPause,
            ActionId::Stop,
            ActionId::NextSentence,
            ActionId::PreviousSentence,
        ];
        assert_eq!(a[0].category(), a[1].category());
        assert_ne!(a[1].category(), a[2].category());
        assert_eq!(group_start(&a, 1), 0);
        assert_eq!(group_start(&a, 3), 2);
        assert_eq!(group_start(&a, 2), 2);
    }
}
