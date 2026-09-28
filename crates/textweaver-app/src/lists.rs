//! The lists the authoring features show: the outline (Alt+O), the
//! citation picker (Alt+C), spelling suggestions (Alt+J), the find and
//! replace choices, and templates.
//!
//! The outline and the citation picker **filter as you type**: the
//! frontend sends the filter with [`Command::FilterList`] while
//! [`App::list_filter`] says the shown list filters, and the list is shown
//! again with the items holding every word typed. The other lists have
//! letter keys that choose at once ([`accelerator`]).
//!
//! [`Command::FilterList`]: crate::Command::FilterList

use textweaver_core::MarkerKind;
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;
use textweaver_text::Document;

use crate::app::{App, ListKind};
use crate::authoring_state::{AuthoringList, OutlineItem, SpellChoice};
use crate::command::Effect;
use crate::nav::ReadAfter;

/// True when `text` holds every word of `query`, ignoring case.
pub(crate) fn matches(text: &str, query: &str) -> bool {
    let text = text.to_lowercase();
    query
        .split_whitespace()
        .all(|w| text.contains(&w.to_lowercase()))
}

/// The item a letter chooses at once in `list`, if any: in the find and
/// replace choices, `r` replaces, `s` skips, `a` replaces all, `c` and `w`
/// switch match case and whole words.
pub(crate) fn accelerator(list: &AuthoringList, c: char) -> Option<usize> {
    match (list, c.to_ascii_lowercase()) {
        (AuthoringList::Replace, 'r') => Some(0),
        (AuthoringList::Replace, 's') => Some(1),
        (AuthoringList::Replace, 'a') => Some(2),
        (AuthoringList::Replace, 'c') => Some(3),
        (AuthoringList::Replace, 'w') => Some(4),
        _ => None,
    }
}

/// Whitespace collapsed, at most `max` chars (with an ellipsis).
pub(crate) fn one_line(text: &str, max: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let joined = words.join(" ");
    if joined.chars().count() <= max {
        return joined;
    }
    let cut: String = joined.chars().take(max).collect();
    format!("{}…", cut.trim_end())
}

/// The headings of `doc`, in order.
pub(crate) fn headings(doc: &Document) -> Vec<OutlineItem> {
    doc.marker_index()
        .iter(MarkerKind::Heading, None)
        .map(|m| OutlineItem {
            pos: m.range.start,
            level: m.level,
            text: one_line(&doc.slice(m.range), 100),
        })
        .collect()
}

fn outline_label(c: &Catalog, h: &OutlineItem) -> String {
    let text = if h.text.is_empty() {
        c.tr("nav-blank")
    } else {
        h.text.clone()
    };
    c.fmt(
        "lists-outline-item",
        &args!["text" => text, "level" => h.level],
    )
}

impl App {
    /// The filter typed so far when the shown list filters as you type
    /// (the outline, the citation picker); `None` for other lists. Letters,
    /// digits, Space, and Backspace then change the filter
    /// ([`Command::FilterList`](crate::Command::FilterList)) instead of
    /// jumping or closing.
    pub fn list_filter(&self) -> Option<&str> {
        match &self.list {
            Some(ListKind::Authoring(l)) if l.filterable() => Some(&self.authoring.filter),
            Some(ListKind::Settings) => self.settings_filter(),
            _ => None,
        }
    }

    /// Shows `list`: its title and items, for the frontend.
    pub(crate) fn show_authoring_list(&mut self, list: AuthoringList) -> Vec<Effect> {
        let filter = self.authoring.filter.clone();
        let (title, items) = match &list {
            AuthoringList::Outline { items, shown } => {
                let n = items.len();
                let title = if filter.is_empty() {
                    self.msg_args("lists-outline-title", &args!["n" => n])
                } else {
                    self.msg_args(
                        "lists-outline-title-filtered",
                        &args!["shown" => shown.len(), "n" => n, "filter" => filter.as_str()],
                    )
                };
                (
                    title,
                    shown
                        .iter()
                        .map(|&i| outline_label(self.cat(), &items[i]))
                        .collect(),
                )
            }
            AuthoringList::Citations { entries, shown } => {
                let n = entries.len();
                let title = if filter.is_empty() {
                    self.msg_args("lists-citations-title", &args!["n" => n])
                } else {
                    self.msg_args(
                        "lists-citations-title-filtered",
                        &args!["shown" => shown.len(), "n" => n, "filter" => filter.as_str()],
                    )
                };
                (
                    title,
                    shown.iter().map(|&i| entries[i].label.clone()).collect(),
                )
            }
            AuthoringList::Spelling { word, choices, .. } => (
                self.msg_args("lists-spelling-title", &args!["word" => word]),
                choices
                    .iter()
                    .map(|c| match c {
                        SpellChoice::Replace(w) => w.clone(),
                        SpellChoice::Add => {
                            self.msg_args("lists-spelling-add", &args!["word" => word])
                        }
                        SpellChoice::Ignore => self.msg("lists-leave-as-is"),
                    })
                    .collect(),
            ),
            AuthoringList::Grammar { words, fixes, .. } => (
                self.msg_args(
                    "lists-grammar-title",
                    &args!["words" => words.split_whitespace().collect::<Vec<_>>().join(" ")],
                ),
                fixes
                    .iter()
                    .map(|f| crate::authoring::grammar_fix_label(self.cat(), f))
                    .chain([self.msg("lists-leave-as-is")])
                    .collect(),
            ),
            AuthoringList::Replace => (self.replace_title(), self.replace_items()),
            AuthoringList::Templates(t) => (
                self.msg_args("lists-templates-title", &args!["n" => t.len()]),
                t.iter().map(|t| t.label(self.cat())).collect(),
            ),
        };
        self.list = Some(ListKind::Authoring(list));
        vec![Effect::ShowList { title, items }]
    }

    /// The filter of the shown list changed to `query`.
    pub(crate) fn filter_list(&mut self, query: String) -> Vec<Effect> {
        if self.list == Some(ListKind::Settings) {
            return self.filter_settings(query);
        }
        let Some(ListKind::Authoring(mut list)) = self.list.clone() else {
            let msg = self.msg("lists-no-filter");
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        if !list.filterable() {
            let msg = self.msg("lists-no-filter");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        // The messages for this list: filter cleared, nothing matches, and
        // how many match.
        let (n, ids) = match &mut list {
            AuthoringList::Outline { items, shown } => {
                *shown = (0..items.len())
                    .filter(|&i| matches(&items[i].text, &query))
                    .collect();
                (
                    shown.len(),
                    (
                        "lists-filter-cleared-headings",
                        "lists-filter-none-headings",
                        "lists-filter-matched-headings",
                    ),
                )
            }
            AuthoringList::Citations { entries, shown } => {
                *shown = (0..entries.len())
                    .filter(|&i| matches(&entries[i].label, &query))
                    .collect();
                (
                    shown.len(),
                    (
                        "lists-filter-cleared-references",
                        "lists-filter-none-references",
                        "lists-filter-matched-references",
                    ),
                )
            }
            _ => (
                0,
                (
                    "lists-filter-cleared-items",
                    "lists-filter-none-items",
                    "lists-filter-matched-items",
                ),
            ),
        };
        self.authoring.filter = query.clone();
        let msg = if query.trim().is_empty() {
            self.msg_args(ids.0, &args!["n" => n])
        } else if n == 0 {
            self.msg_args(ids.1, &args!["query" => query.as_str()])
        } else {
            self.msg_args(ids.2, &args!["n" => n])
        };
        self.tell(&msg);
        self.show_authoring_list(list)
    }

    /// Enter on item `n` of an authoring list.
    pub(crate) fn choose_authoring(&mut self, list: AuthoringList, n: usize) -> Vec<Effect> {
        self.authoring.filter.clear();
        match list {
            AuthoringList::Outline { items, shown } => {
                if let Some(h) = shown.get(n).and_then(|&i| items.get(i)) {
                    self.go_to_heading(h);
                }
                vec![Effect::Redraw]
            }
            AuthoringList::Citations { entries, shown } => {
                match shown.get(n).and_then(|&i| entries.get(i)) {
                    Some(e) => self.citation_chosen(e.key.clone()),
                    None => vec![Effect::Redraw],
                }
            }
            AuthoringList::Spelling {
                word,
                range,
                choices,
            } => match choices.get(n) {
                Some(c) => self.spelling_chosen(&word, range, c.clone()),
                None => vec![Effect::Redraw],
            },
            AuthoringList::Grammar { range, fixes, .. } => {
                self.grammar_fix_action(range, &fixes, n)
            }
            AuthoringList::Replace => self.replace_choice(n),
            AuthoringList::Templates(t) => match t.get(n) {
                Some(t) => self.template_chosen(t.clone()),
                None => vec![Effect::Redraw],
            },
        }
    }

    /// Escape on an authoring list.
    pub(crate) fn cancel_authoring_list(&mut self, list: AuthoringList) {
        self.authoring.filter.clear();
        match list {
            AuthoringList::Replace => self.replace_stopped(),
            _ => {
                let msg = self.msg("common-cancelled");
                self.note(&msg);
            }
        }
    }

    /// Alt+O: the outline, in reading and editing.
    pub(crate) fn outline(&mut self) -> Vec<Effect> {
        self.refresh_structure(false);
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let items = headings(&s.doc);
        if items.is_empty() {
            let msg = self.msg("lists-no-headings");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let cursor = s.cursor;
        let here = items
            .iter()
            .rev()
            .find(|h| h.pos <= cursor)
            .map(|h| h.text.clone());
        let n = items.len();
        let mut intro = self.msg_args("lists-outline-intro", &args!["n" => n]);
        if let Some(h) = here {
            intro.push(' ');
            intro.push_str(&self.msg_args("lists-outline-here", &args!["heading" => h]));
        }
        self.authoring.filter.clear();
        self.tell(&intro);
        let shown = (0..n).collect();
        self.show_authoring_list(AuthoringList::Outline { items, shown })
    }

    /// Jumps to a heading chosen in the outline (recorded in history).
    fn go_to_heading(&mut self, h: &OutlineItem) {
        let label = self.msg_args("nav-label-heading-level", &args!["level" => h.level]);
        let msg = self.nav_message(Some(&label), h.pos, &h.text);
        self.jump(h.pos, true, ReadAfter::Follow, &msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_need_every_word() {
        assert!(matches("Methods and results", "res meth"));
        assert!(!matches("Methods", "results"));
        assert!(matches("Anything", "  "));
        assert_eq!(one_line("a\n  b   c", 10), "a b c");
        assert_eq!(one_line("abcdefghij klm", 5), "abcde…");
    }
}
