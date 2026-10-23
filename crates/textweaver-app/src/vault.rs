//! Obsidian vaults in the reader: a note's properties, the notes that link
//! to it, its links to notes that don't exist, and tags, as lists.
//!
//! The vault index (`textweaver_vault::VaultIndex`) is built the first time
//! a command needs it, for the open note's vault (the folder holding
//! `.obsidian`, or the note's own folder without one), and brought up to
//! date before each use, so notes added, changed, or removed since are
//! seen. It is bounded in size; a partial index says so.
//!
//! - `note_properties` (`f`): the front matter, "name: value" per row.
//!   Opening a note with properties says "Properties: 3." after its title.
//! - `backlinks`: "Notes that link here"; Enter opens one.
//! - `unresolved_links`: "Links to notes that don't exist" in this note.
//! - `note_tags`: the tags in this note; Enter lists the notes with one.
//! - `vault_tags`: every tag in the vault with its count; Enter lists the
//!   notes with it, and Enter there opens one.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use textweaver_lexicon::args;
use textweaver_vault::{LinkScope, Resolved, VaultIndex};

use crate::app::{App, ListKind};
use crate::command::Effect;

/// The vault index and what has been said about it.
#[derive(Debug, Default)]
pub(crate) struct VaultState {
    /// The index of the last vault a command needed.
    index: Option<VaultIndex>,
    /// The link names already said to match several notes.
    said_same_name: HashSet<String>,
}

/// What a vault list's rows stand for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum VaultList {
    /// Notes: Enter opens one (backlinks, the notes with a tag).
    Notes(Vec<PathBuf>),
    /// Tags: Enter lists the notes carrying one.
    Tags(Vec<String>),
    /// Rows to read (properties, unresolved links): Enter does nothing.
    Read,
}

impl App {
    /// Makes the index the one for `note`'s vault, up to date, and
    /// returns it.
    fn index_for(&mut self, note: &Path) -> &VaultIndex {
        let scope = LinkScope::for_note(note);
        let current = self
            .vault
            .index
            .as_ref()
            .is_some_and(|i| *i.scope() == scope);
        if current {
            if let Some(i) = self.vault.index.as_mut() {
                i.refresh();
            }
        } else {
            self.vault.index = Some(VaultIndex::build(scope));
        }
        self.vault
            .index
            .get_or_insert_with(|| VaultIndex::for_note(note))
    }

    /// The note a link names by its title or an alias (after its path and
    /// name were not found), for a note at `from`.
    pub(crate) fn vault_resolve(&mut self, from: &Path, target: &str) -> Option<Resolved> {
        self.index_for(from).resolve(from, target)
    }

    /// When a link's name matched `count` notes, "2 notes named X; the
    /// nearest opened.", the first time for that name only.
    pub(crate) fn same_name_once(&mut self, name: &str, count: usize) -> Option<String> {
        if count < 2 || !self.vault.said_same_name.insert(name.to_lowercase()) {
            return None;
        }
        Some(self.msg_args("vault-same-name", &args!["count" => count, "name" => name]))
    }

    /// "Properties: 3." for an opened document with front matter.
    pub(crate) fn properties_count_message(&self) -> Option<String> {
        let n = textweaver_formats::properties::front_matter_properties(
            &self.session.as_ref()?.doc.meta,
        )
        .len();
        (n > 0).then(|| self.msg_args("vault-properties-count", &args!["n" => n]))
    }

    /// The open document's path, or "This document has not been saved"
    /// said and `None`.
    fn note_path(&mut self) -> Option<PathBuf> {
        let path = self.session.as_ref()?.doc.meta.path.clone();
        if path.is_none() {
            let msg = self.msg("vault-no-file");
            self.tell(&msg);
        }
        path
    }

    /// Shows a vault list.
    fn show_vault_list(
        &mut self,
        title_id: &str,
        intro: String,
        items: Vec<String>,
        kind: VaultList,
    ) -> Vec<Effect> {
        self.tell(&intro);
        let title = self.msg(title_id);
        self.list = Some(ListKind::Vault(kind));
        vec![Effect::ShowList { title, items }]
    }

    /// " Only part of the vault was searched; it is very large." when the
    /// index hit a limit, else nothing.
    fn partial_note(&self) -> String {
        let partial = self
            .vault
            .index
            .as_ref()
            .is_some_and(VaultIndex::is_partial);
        if partial {
            format!(" {}", self.msg("vault-partial"))
        } else {
            String::new()
        }
    }

    /// `note_properties`: the note's front matter as a list, name first.
    pub(crate) fn list_note_properties(&mut self) -> Vec<Effect> {
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let props = textweaver_formats::properties::front_matter_properties(&s.doc.meta);
        if props.is_empty() {
            let msg = self.msg("vault-no-properties");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let items = props
            .iter()
            .map(|(name, value)| {
                let value = match value.to_ascii_lowercase().as_str() {
                    "true" => self.msg("vault-property-checked"),
                    "false" => self.msg("vault-property-not-checked"),
                    _ => value.clone(),
                };
                self.msg_args(
                    "vault-property-row",
                    &args!["name" => name.as_str(), "value" => value],
                )
            })
            .collect();
        let intro = self.msg_args("vault-properties-count", &args!["n" => props.len()]);
        self.show_vault_list("vault-properties-title", intro, items, VaultList::Read)
    }

    /// A note's row: its title, then the folder it is in.
    fn note_row(&self, path: &Path, links: Option<usize>) -> String {
        let (title, rel) = match &self.vault.index {
            Some(index) => (index.title_of(path), index.scope().relative(path)),
            None => (crate::authoring_state::file_name(path), String::new()),
        };
        let folder = rel
            .rsplit_once('/')
            .map_or_else(|| self.msg("vault-top-folder"), |(f, _)| f.to_owned());
        match links {
            Some(n) => self.msg_args(
                "vault-backlink-row",
                &args!["title" => title, "links" => n, "folder" => folder],
            ),
            None => self.msg_args(
                "vault-note-row",
                &args!["title" => title, "folder" => folder],
            ),
        }
    }

    /// `backlinks`: the notes that link to or embed this one.
    pub(crate) fn list_backlinks(&mut self) -> Vec<Effect> {
        let Some(path) = self.note_path() else {
            return vec![Effect::Redraw];
        };
        let back = self.index_for(&path).backlinks(&path);
        let partial = self.partial_note();
        if back.is_empty() {
            let msg = format!("{}{partial}", self.msg("vault-no-backlinks"));
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let items = back
            .iter()
            .map(|b| self.note_row(&b.from, Some(b.links)))
            .collect();
        let intro = format!(
            "{}{partial}",
            self.msg_args("vault-backlinks-intro", &args!["n" => back.len()])
        );
        let paths = back.into_iter().map(|b| b.from).collect();
        self.show_vault_list(
            "vault-backlinks-title",
            intro,
            items,
            VaultList::Notes(paths),
        )
    }

    /// `unresolved_links`: this note's links to notes that don't exist.
    pub(crate) fn list_unresolved_links(&mut self) -> Vec<Effect> {
        let Some(path) = self.note_path() else {
            return vec![Effect::Redraw];
        };
        let missing = self.index_for(&path).unresolved(Some(&path));
        let partial = self.partial_note();
        if missing.is_empty() {
            let msg = format!("{}{partial}", self.msg("vault-no-unresolved"));
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let items = missing.iter().map(|u| u.target.clone()).collect();
        let intro = format!(
            "{}{partial}",
            self.msg_args("vault-unresolved-intro", &args!["n" => missing.len()])
        );
        self.show_vault_list("vault-unresolved-title", intro, items, VaultList::Read)
    }

    /// `note_tags`: the tags in this note (front matter and `#tags`).
    pub(crate) fn list_note_tags(&mut self) -> Vec<Effect> {
        let Some(path) = self.note_path() else {
            return vec![Effect::Redraw];
        };
        let tags = self
            .index_for(&path)
            .note(&path)
            .map(|n| n.tags.clone())
            .unwrap_or_default();
        if tags.is_empty() {
            let msg = self.msg("vault-no-note-tags");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let intro = self.msg_args("vault-note-tags-intro", &args!["n" => tags.len()]);
        let items = tags.clone();
        self.show_vault_list("vault-note-tags-title", intro, items, VaultList::Tags(tags))
    }

    /// `vault_tags`: every tag in the vault, with how many notes carry it.
    pub(crate) fn list_vault_tags(&mut self) -> Vec<Effect> {
        let Some(path) = self.note_path() else {
            return vec![Effect::Redraw];
        };
        let tags = self.index_for(&path).tags();
        let partial = self.partial_note();
        if tags.is_empty() {
            let msg = format!("{}{partial}", self.msg("vault-no-tags"));
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let items = tags
            .iter()
            .map(|t| {
                self.msg_args(
                    "vault-tag-row",
                    &args!["tag" => t.tag.as_str(), "n" => t.notes],
                )
            })
            .collect();
        let intro = format!(
            "{}{partial}",
            self.msg_args("vault-tags-intro", &args!["n" => tags.len()])
        );
        let names = tags.into_iter().map(|t| t.tag).collect();
        self.show_vault_list("vault-tags-title", intro, items, VaultList::Tags(names))
    }

    /// The notes carrying `tag`; Enter opens one.
    fn list_tag_notes(&mut self, tag: &str) -> Vec<Effect> {
        let Some(path) = self.note_path() else {
            return vec![Effect::Redraw];
        };
        let notes = self.index_for(&path).notes_with_tag(tag);
        if notes.is_empty() {
            let msg = self.msg_args("vault-tag-no-notes", &args!["tag" => tag]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let items = notes.iter().map(|p| self.note_row(p, None)).collect();
        let intro = self.msg_args(
            "vault-tag-notes-intro",
            &args!["tag" => tag, "n" => notes.len()],
        );
        self.show_vault_list(
            "vault-tag-notes-title",
            intro,
            items,
            VaultList::Notes(notes),
        )
    }

    /// Enter on row `n` of a vault list.
    pub(crate) fn choose_vault(&mut self, list: VaultList, n: usize) -> Vec<Effect> {
        match list {
            VaultList::Notes(paths) => match paths.get(n) {
                Some(p) => self.open_command(p.clone()),
                None => vec![Effect::Redraw],
            },
            VaultList::Tags(tags) => match tags.get(n) {
                Some(t) => self.list_tag_notes(&t.clone()),
                None => vec![Effect::Redraw],
            },
            VaultList::Read => vec![Effect::Redraw],
        }
    }
}
