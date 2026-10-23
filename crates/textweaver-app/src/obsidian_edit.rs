//! Obsidian editing aids in edit mode (beta 4): a wiki link chosen from
//! the vault, a task's status toggled or cycled, and a callout of a
//! chosen type. They reuse the list picker and the editor's pure
//! formatting functions (`textweaver_editor::markdown`); each change is one
//! undo step.
//!
//! - `insert_wiki_link`: the notes of the open note's vault (the vault
//!   index of `crate::vault`), by title and folder; Enter on one lists
//!   "The whole note" and its headings and blocks, and Enter there inserts
//!   `[[Note]]`, `[[Note#Heading]]` or `[[Note#^block]]` (a note with
//!   neither is linked at once). A selection becomes the link's shown text.
//!   The note is named by its file name, or by its path in the vault when
//!   another note has the same name; a link into the open note itself is
//!   `[[#Heading]]`.
//! - `toggle_task` and `cycle_task_status`: say the new status in words
//!   ("Checked.", "In progress.").
//! - `insert_callout`: the callout types in Obsidian's order; Enter inserts
//!   `> [!type]` with an empty body line for the text.

use std::path::{Path, PathBuf};

use textweaver_editor::MarkdownOp;
use textweaver_lexicon::args;
use textweaver_speech::Earcon;

use crate::app::{App, ListKind};
use crate::command::Effect;

/// Most headings and blocks offered for one note.
const MAX_ANCHORS: usize = 2_000;

/// What an editing-aid list's rows stand for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EditAidList {
    /// Notes to link to: Enter lists the note's headings and blocks.
    LinkNotes(Vec<PathBuf>),
    /// The link's note name and, per row, the anchor to add (`#Heading`,
    /// `#^block`), or nothing for the whole note.
    LinkAnchors {
        name: String,
        anchors: Vec<Option<String>>,
    },
    /// Callout types: Enter inserts one.
    Callouts,
}

/// A heading or block a link can point into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NoteAnchor {
    /// A heading: its level and text.
    Heading(u8, String),
    /// A block id (`^id` at a line's end) and the start of its text.
    Block(String, String),
}

/// Obsidian's callout types, without the aliases (`summary` is
/// `abstract`), in Obsidian's order.
pub(crate) fn callout_types() -> Vec<&'static str> {
    textweaver_formats::callout::TYPES
        .iter()
        .filter(|(written, styled)| written == styled)
        .map(|(written, _)| *written)
        .collect()
}

/// The headings and block ids of a note's Markdown, in order: front
/// matter and code blocks skipped.
pub(crate) fn note_anchors(text: &str) -> Vec<NoteAnchor> {
    let mut out = Vec::new();
    let mut lines = text.lines().peekable();
    if lines.peek().map(|l| l.trim_end()) == Some("---") {
        lines.next();
        for l in lines.by_ref() {
            if matches!(l.trim_end(), "---" | "...") {
                break;
            }
        }
    }
    let mut fence: Option<(char, usize)> = None;
    for line in lines {
        let t = line.trim_start();
        let ch = t.chars().next().unwrap_or(' ');
        if ch == '`' || ch == '~' {
            let n = t.chars().take_while(|&c| c == ch).count();
            if n >= 3 {
                match fence {
                    None => fence = Some((ch, n)),
                    Some((c, m)) if c == ch && n >= m => fence = None,
                    Some(_) => {}
                }
                continue;
            }
        }
        if fence.is_some() {
            continue;
        }
        let hashes = t.chars().take_while(|&c| c == '#').count();
        if (1..=6).contains(&hashes) && t[hashes..].starts_with(' ') {
            let heading = t[hashes..].trim().trim_end_matches('#').trim();
            let heading = link_safe(heading);
            if !heading.is_empty() {
                out.push(NoteAnchor::Heading(hashes as u8, heading));
            }
        } else if let Some((body, id)) = t.trim_end().rsplit_once(" ^") {
            let ok = !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
            if ok {
                let words: String = body.trim().chars().take(60).collect();
                out.push(NoteAnchor::Block(id.to_owned(), words));
            }
        }
        if out.len() >= MAX_ANCHORS {
            break;
        }
    }
    out
}

/// Heading text as a link can carry it: Obsidian leaves out the
/// characters that end or split a link (`[ ] | # ^`).
fn link_safe(heading: &str) -> String {
    heading
        .chars()
        .filter(|c| !matches!(c, '[' | ']' | '|' | '#' | '^'))
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The words of a task status for `edit-task-status`.
fn status_key(c: char) -> &'static str {
    match c {
        'x' | 'X' => "done",
        ' ' => "open",
        '/' => "in-progress",
        '-' => "cancelled",
        '>' => "forwarded",
        '!' => "important",
        '?' => "question",
        _ => "other",
    }
}

impl App {
    /// `insert_wiki_link`: lists the vault's notes to link to.
    pub(crate) fn insert_wiki_link_start(&mut self) -> Vec<Effect> {
        let Some(path) = self.note_path() else {
            return vec![Effect::Redraw];
        };
        let index = self.index_for(&path);
        let mut notes: Vec<(String, PathBuf)> = index
            .notes()
            .iter()
            .map(|n| (index.title_of(&n.path).to_lowercase(), n.path.clone()))
            .collect();
        notes.sort();
        let partial = self.partial_note();
        if notes.is_empty() {
            let msg = format!("{}{partial}", self.msg("edit-wikilink-no-notes"));
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let paths: Vec<PathBuf> = notes.into_iter().map(|(_, p)| p).collect();
        let items = paths.iter().map(|p| self.note_row(p, None)).collect();
        let intro = format!(
            "{}{partial}",
            self.msg_args("edit-wikilink-intro", &args!["n" => paths.len()])
        );
        self.show_edit_aid_list(
            "edit-wikilink-title",
            intro,
            items,
            EditAidList::LinkNotes(paths),
        )
    }

    fn show_edit_aid_list(
        &mut self,
        title_id: &str,
        intro: String,
        items: Vec<String>,
        kind: EditAidList,
    ) -> Vec<Effect> {
        self.tell(&intro);
        let title = self.msg(title_id);
        self.list = Some(ListKind::EditAid(kind));
        vec![Effect::ShowList { title, items }]
    }

    /// How a link from the open note names the note at `path`: its file
    /// name, its path in the vault when another note shares the name, or
    /// nothing for the open note itself (`[[#Heading]]`).
    fn link_name(&mut self, path: &Path) -> String {
        let here = self.session.as_ref().and_then(|s| s.doc.meta.path.clone());
        let Some(here) = here else {
            return String::new();
        };
        let same_file = here == path
            || matches!(
                (std::fs::canonicalize(&here), std::fs::canonicalize(path)),
                (Ok(a), Ok(b)) if a == b
            );
        if same_file {
            return String::new();
        }
        let index = self.index_for(&here);
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let same = index
            .notes()
            .iter()
            .filter(|n| n.name.eq_ignore_ascii_case(&stem))
            .count();
        if same <= 1 {
            return stem;
        }
        let rel = index.scope().relative(path);
        rel.strip_suffix(".md").unwrap_or(&rel).to_owned()
    }

    /// A note was chosen: its headings and blocks, or the link at once.
    fn wiki_link_note_chosen(&mut self, path: &Path) -> Vec<Effect> {
        let name = self.link_name(path);
        let anchors = std::fs::read_to_string(path)
            .map(|t| note_anchors(&t))
            .unwrap_or_default();
        if anchors.is_empty() {
            return self.insert_wiki_link_target(&name);
        }
        let title = self.vault_title(path).unwrap_or_else(|| name.clone());
        let mut items = vec![self.msg("edit-wikilink-whole")];
        let mut targets = vec![None];
        for a in anchors {
            match a {
                NoteAnchor::Heading(level, text) => {
                    items.push(self.msg_args(
                        "edit-wikilink-heading",
                        &args!["level" => level, "text" => text.as_str()],
                    ));
                    targets.push(Some(format!("#{text}")));
                }
                NoteAnchor::Block(id, text) => {
                    items.push(self.msg_args(
                        "edit-wikilink-block",
                        &args!["id" => id.as_str(), "text" => text.as_str()],
                    ));
                    targets.push(Some(format!("#^{id}")));
                }
            }
        }
        let intro = self.msg_args(
            "edit-wikilink-anchor-intro",
            &args!["note" => title.as_str()],
        );
        let list_title = self.msg_args(
            "edit-wikilink-anchor-title",
            &args!["note" => title.as_str()],
        );
        self.tell(&intro);
        self.list = Some(ListKind::EditAid(EditAidList::LinkAnchors {
            name,
            anchors: targets,
        }));
        vec![Effect::ShowList {
            title: list_title,
            items,
        }]
    }

    /// The note's title in the vault index, when indexed.
    fn vault_title(&mut self, path: &Path) -> Option<String> {
        let here = self.session.as_ref()?.doc.meta.path.clone()?;
        Some(self.index_for(&here).title_of(path))
    }

    /// Inserts `[[target]]` at the caret.
    fn insert_wiki_link_target(&mut self, target: &str) -> Vec<Effect> {
        let Some(edit) = self.edit.as_mut() else {
            return self.not_editing("insert");
        };
        let Some(before) = edit.session.editor().map(|e| e.text().clone()) else {
            return vec![Effect::Redraw];
        };
        match edit.session.insert_wiki_link(target) {
            Ok(outcomes) => {
                self.after_edit(&before, &outcomes);
                let msg = self.msg_args("edit-wikilink-inserted", &args!["target" => target]);
                self.tell(&msg);
            }
            Err(e) => {
                self.speech.earcon(Earcon::Error);
                self.error(&format!("{e}."));
            }
        }
        vec![Effect::Redraw]
    }

    /// `insert_callout`: lists the callout types.
    pub(crate) fn insert_callout_start(&mut self) -> Vec<Effect> {
        let items: Vec<String> = callout_types().iter().map(|t| (*t).to_owned()).collect();
        let intro = self.msg_args("edit-callout-intro", &args!["n" => items.len()]);
        self.show_edit_aid_list("edit-callout-title", intro, items, EditAidList::Callouts)
    }

    fn insert_callout_kind(&mut self, kind: &str) -> Vec<Effect> {
        let Some(edit) = self.edit.as_mut() else {
            return self.not_editing("insert");
        };
        let Some(before) = edit.session.editor().map(|e| e.text().clone()) else {
            return vec![Effect::Redraw];
        };
        match edit.session.insert_callout(kind) {
            Ok(outcomes) => {
                self.after_edit(&before, &outcomes);
                let msg = self.msg_args("edit-callout-inserted", &args!["type" => kind]);
                self.tell(&msg);
            }
            Err(e) => {
                self.speech.earcon(Earcon::Error);
                self.error(&format!("{e}."));
            }
        }
        vec![Effect::Redraw]
    }

    /// `toggle_task` and `cycle_task_status`: changes the task on each
    /// selected line and says the caret line's new status ("Checked.",
    /// "In progress."), "New task, not checked." for a line that became
    /// one, or how many lines changed.
    pub(crate) fn task_command(&mut self, op: MarkdownOp) -> Vec<Effect> {
        let Some(edit) = self.edit.as_mut() else {
            return self.not_editing("format");
        };
        let Some((before, was)) = edit.session.editor().map(|e| {
            let t = e.text().clone();
            let line = t.char_to_line(e.selection().head.0.min(t.len_chars()));
            let was = textweaver_editor::markdown::task_status(&t.line(line).to_string());
            (t, was)
        }) else {
            return vec![Effect::Redraw];
        };
        match edit.session.format(op) {
            Ok(outcomes) => {
                let lines = outcomes.len();
                let now = edit.session.editor().and_then(|e| {
                    let t = e.text();
                    let line = t.char_to_line(e.selection().head.0.min(t.len_chars()));
                    textweaver_editor::markdown::task_status(&t.line(line).to_string())
                });
                self.after_edit(&before, &outcomes);
                let msg = if lines > 1 {
                    self.msg_args("edit-tasks-changed", &args!["n" => lines])
                } else {
                    match (was, now) {
                        (None, Some(_)) => self.msg("edit-task-new"),
                        (_, Some(c)) => self.msg_args(
                            "edit-task-status",
                            &args!["status" => status_key(c), "mark" => c.to_string()],
                        ),
                        (_, None) => self.msg("edit-task-new"),
                    }
                };
                self.tell(&msg);
            }
            Err(e) => {
                self.speech.earcon(Earcon::Error);
                self.error(&format!("{e}."));
            }
        }
        vec![Effect::Redraw]
    }

    /// Enter on row `n` of an editing-aid list.
    pub(crate) fn choose_edit_aid(&mut self, list: EditAidList, n: usize) -> Vec<Effect> {
        match list {
            EditAidList::LinkNotes(paths) => match paths.get(n) {
                Some(p) => self.wiki_link_note_chosen(&p.clone()),
                None => vec![Effect::Redraw],
            },
            EditAidList::LinkAnchors { name, anchors } => match anchors.get(n) {
                Some(anchor) => {
                    let target = match anchor {
                        Some(a) => format!("{name}{a}"),
                        None if name.is_empty() => {
                            // The whole open note: its own file name.
                            let here = self.session.as_ref().and_then(|s| {
                                s.doc.meta.path.as_ref().and_then(|p| {
                                    p.file_stem().map(|s| s.to_string_lossy().into_owned())
                                })
                            });
                            here.unwrap_or_default()
                        }
                        None => name.clone(),
                    };
                    self.insert_wiki_link_target(&target)
                }
                None => vec![Effect::Redraw],
            },
            EditAidList::Callouts => match callout_types().get(n) {
                Some(kind) => self.insert_callout_kind(kind),
                None => vec![Effect::Redraw],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchors_are_headings_and_blocks_outside_code_and_front_matter() {
        let text = "---\ntitle: X\n# not a heading\n---\n# Cell\n\nIntro.\n\n## Parts [of] the cell #\n\n```\n# code\nline ^not-a-block\n```\n\nCells make ATP here. ^atp-fact\n\n#tag only\n";
        assert_eq!(
            note_anchors(text),
            [
                NoteAnchor::Heading(1, "Cell".into()),
                NoteAnchor::Heading(2, "Parts of the cell".into()),
                NoteAnchor::Block("atp-fact".into(), "Cells make ATP here.".into()),
            ]
        );
    }

    #[test]
    fn callout_types_are_the_canonical_ones() {
        let t = callout_types();
        assert_eq!(t.len(), 13);
        assert_eq!(t[0], "note");
        assert!(t.contains(&"warning") && !t.contains(&"summary"));
    }
}
