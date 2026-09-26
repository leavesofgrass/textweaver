//! Notes and highlights, and bookmark rename and delete.
//!
//! A note is text attached to a range (the selection, else the sentence at
//! the cursor), with `#tags` taken from its text (Star's tag rule: split on
//! commas and spaces, leading `#` dropped). A highlight is a colored range.
//! Both move with edits exactly like bookmarks (one `EditOutcome` for all),
//! are listed accessibly (Enter jumps, Delete removes, F2 edits), and are
//! saved with the document's state.
//!
//! Until Agent C2's typed notes land in `DocState`, they are stored in its
//! preserved extra keys `app_notes` and `app_highlights`, which cannot clash
//! with the fields C2 adds; the orchestrator moves them over at
//! integration.

use serde::{Deserialize, Serialize};
use textweaver_a11y::Verbosity;
use textweaver_core::{CharPos, CharRange, Direction, EditOutcome, Unit};
use textweaver_speech::Earcon;
use textweaver_store::DocState;
use textweaver_text::units::unit_at;

use crate::app::{App, ListKind};
use crate::command::{Effect, NoteCommand, PromptPurpose};
use crate::nav::ReadAfter;
use crate::text_util::{self, preview};

/// A note attached to a range of the document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    /// Short stable id (8 hex digits).
    pub id: String,
    /// The text the note is about.
    pub range: CharRange,
    /// That text when the note was written, whitespace collapsed, at most
    /// 120 characters (Star's `anchor`).
    pub anchor: String,
    /// The note.
    pub text: String,
    /// Tags from `#words` in the note.
    #[serde(default)]
    pub tags: Vec<String>,
    /// When it was written or last edited (Unix seconds, UTC).
    pub ts: i64,
}

/// A highlighted range.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserHighlight {
    /// The highlighted text.
    pub range: CharRange,
    /// Color name (Star's default is yellow).
    pub color: String,
    /// When it was added (Unix seconds, UTC).
    pub ts: i64,
}

/// `DocState` extra key holding the notes.
pub const NOTES_KEY: &str = "app_notes";
/// `DocState` extra key holding the highlights.
pub const HIGHLIGHTS_KEY: &str = "app_highlights";

/// Longest anchor kept, in characters (Star's limit).
const ANCHOR_CHARS: usize = 120;

/// The notes and highlights stored in `state`; malformed entries are
/// dropped with a warning.
pub fn load_notes(state: &DocState) -> (Vec<Note>, Vec<UserHighlight>) {
    fn get<T: serde::de::DeserializeOwned>(state: &DocState, key: &str) -> Vec<T> {
        state
            .extra
            .get(key)
            .and_then(|v| match serde_json::from_value(v.clone()) {
                Ok(v) => Some(v),
                Err(e) => {
                    log::warn!("ignoring stored {key}: {e}");
                    None
                }
            })
            .unwrap_or_default()
    }
    let mut notes: Vec<Note> = get(state, NOTES_KEY);
    notes.sort_by_key(|n| (n.range.start, n.range.end));
    let mut highlights: Vec<UserHighlight> = get(state, HIGHLIGHTS_KEY);
    highlights.sort_by_key(|h| (h.range.start, h.range.end));
    (notes, highlights)
}

/// Stores notes and highlights in `state` (an empty list removes the key,
/// as Star did).
pub fn store_notes(state: &mut DocState, notes: &[Note], highlights: &[UserHighlight]) {
    fn put<T: Serialize>(state: &mut DocState, key: &str, items: &[T]) {
        if items.is_empty() {
            state.extra.remove(key);
        } else if let Ok(v) = serde_json::to_value(items) {
            state.extra.insert(key.to_owned(), v);
        }
    }
    put(state, NOTES_KEY, notes);
    put(state, HIGHLIGHTS_KEY, highlights);
}

/// Tags in a note: words starting with `#`, without the `#`, lowercase,
/// deduplicated.
pub fn parse_tags(text: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for word in text.split([',', ' ', '\t', '\n']) {
        if let Some(t) = word.strip_prefix('#') {
            let t = t
                .trim_end_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            if !t.is_empty() && !tags.contains(&t) {
                tags.push(t);
            }
        }
    }
    tags
}

fn collapse(s: &str, max: usize) -> String {
    let all = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if all.chars().count() <= max {
        all
    } else {
        let mut cut: String = all.chars().take(max).collect();
        cut.push('…');
        cut
    }
}

fn new_id(existing: &[Note], seed: usize) -> String {
    let base = u32::try_from(textweaver_store::now_ts() & 0xffff_ffff).unwrap_or(0);
    (0u32..)
        .map(|n| {
            format!(
                "{:08x}",
                base.wrapping_mul(2_654_435_761)
                    .wrapping_add(u32::try_from(seed).unwrap_or(0))
                    .wrapping_add(n)
            )
        })
        .find(|id| !existing.iter().any(|x| &x.id == id))
        .unwrap_or_default()
}

/// Moves notes and highlights across an edit; a note whose text was deleted
/// entirely stays, collapsed at the edit point (its anchor keeps the text).
pub(crate) fn shift_notes(
    notes: &mut [Note],
    highlights: &mut Vec<UserHighlight>,
    o: &EditOutcome,
) {
    for n in notes.iter_mut() {
        n.range = o.map_range(n.range);
    }
    for h in highlights.iter_mut() {
        h.range = o.map_range(h.range);
    }
    highlights.retain(|h| !h.range.is_empty());
}

impl App {
    /// Runs a notes, highlights, or bookmark management command.
    pub(crate) fn notes_command(&mut self, c: NoteCommand) -> Vec<Effect> {
        if self.session.is_none() {
            self.tell("No document is open. Press Control O to open one.");
            return vec![Effect::Redraw];
        }
        match c {
            NoteCommand::Add => return self.prompt(PromptPurpose::NoteText),
            NoteCommand::List => return self.list_notes(),
            NoteCommand::Next => self.note_step(Direction::Forward),
            NoteCommand::Previous => self.note_step(Direction::Backward),
            NoteCommand::ToggleHighlight => self.toggle_highlight(),
            NoteCommand::ListHighlights => return self.list_highlights(),
            NoteCommand::RenameBookmark => return self.bookmark_manage(false),
            NoteCommand::DeleteBookmark => return self.bookmark_manage(true),
        }
        vec![Effect::Redraw]
    }

    /// The selection, else the sentence at the cursor, else the word.
    fn note_target(&self) -> Option<CharRange> {
        let s = self.session.as_ref()?;
        if let Some(sel) = s.selection.filter(|r| !r.is_empty()) {
            return Some(sel);
        }
        let pos = self.reading_position()?;
        unit_at(&s.doc, pos, Unit::Sentence)
            .or_else(|| unit_at(&s.doc, pos, Unit::Word))
            .filter(|r| !r.is_empty())
    }

    /// Adds a note with `text` (from the note prompt).
    pub(crate) fn add_note(&mut self, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            self.note("Cancelled.");
            return;
        }
        let Some(range) = self.note_target() else {
            self.tell("Nothing here to attach a note to.");
            return;
        };
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let anchor = collapse(&s.doc.slice(range), ANCHOR_CHARS);
        let id = new_id(&s.notes, s.notes.len());
        let tags = parse_tags(text);
        s.notes.push(Note {
            id,
            range,
            anchor: anchor.clone(),
            text: text.to_owned(),
            tags: tags.clone(),
            ts: textweaver_store::now_ts(),
        });
        s.notes.sort_by_key(|n| (n.range.start, n.range.end));
        self.persist_marks();
        let on = collapse(&anchor, 40);
        let msg = if tags.is_empty() {
            format!("Note added on: {on}")
        } else {
            format!("Note added with tags {} on: {on}", tags.join(", "))
        };
        self.tell(&msg);
    }

    fn note_item(&self, i: usize) -> Option<String> {
        let s = self.session.as_ref()?;
        let n = s.notes.get(i)?;
        let line = text_util::line_of(&s.doc, n.range.start) + 1;
        Some(format!(
            "{}, line {line}. On: {}",
            n.text,
            collapse(&n.anchor, 60)
        ))
    }

    pub(crate) fn list_notes(&mut self) -> Vec<Effect> {
        let n = self.session.as_ref().map_or(0, |s| s.notes.len());
        if n == 0 {
            self.tell("No notes.");
            return vec![Effect::Redraw];
        }
        let items: Vec<String> = (0..n).filter_map(|i| self.note_item(i)).collect();
        self.list = Some(ListKind::Notes);
        self.tell(&format!(
            "Notes, {n} {}. Enter goes to a note, Delete deletes it, F2 edits it.",
            if n == 1 { "item" } else { "items" }
        ));
        vec![Effect::ShowList {
            title: "Notes".into(),
            items,
        }]
    }

    /// Jumps to note `i`, reading its text.
    pub(crate) fn go_to_note(&mut self, i: usize, wrapped: bool) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let Some(n) = s.notes.get(i) else {
            return;
        };
        let target = n.range.start;
        let content = format!("{}. On: {}", n.text, collapse(&n.anchor, 60));
        let label = format!("Note {} of {}", i + 1, s.notes.len());
        let mut msg = self.nav_message(Some(&label), target, &content);
        if wrapped {
            msg = format!("Wrapped. {msg}");
        }
        self.jump(target, true, ReadAfter::Follow, &msg);
    }

    fn note_step(&mut self, dir: Direction) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        if s.notes.is_empty() {
            self.tell("No notes.");
            return;
        }
        let cursor = s.cursor;
        let (i, wrapped) = match dir {
            Direction::Forward => match s.notes.iter().position(|n| n.range.start > cursor) {
                Some(i) => (i, false),
                None => (0, true),
            },
            Direction::Backward => match s.notes.iter().rposition(|n| n.range.start < cursor) {
                Some(i) => (i, false),
                None => (s.notes.len() - 1, true),
            },
        };
        if wrapped {
            self.speech.earcon(Earcon::Wrap);
        }
        self.go_to_note(i, wrapped);
    }

    pub(crate) fn delete_note(&mut self, i: usize) -> Vec<Effect> {
        let Some(s) = self.session.as_mut() else {
            return vec![Effect::Redraw];
        };
        if i >= s.notes.len() {
            return vec![Effect::Redraw];
        }
        let n = s.notes.remove(i);
        let left = s.notes.len();
        self.persist_marks();
        self.tell(&format!("Note deleted: {}.", collapse(&n.text, 40)));
        if left > 0 {
            // Keep the list open, one item shorter, on the next item.
            let mut effects = self.list_notes_quiet();
            effects.push(Effect::Redraw);
            return effects;
        }
        self.list = None;
        vec![Effect::Redraw]
    }

    /// Deletes the note at the reading position, else the highlight there
    /// (the `delete_note` action, after its confirmation).
    pub(crate) fn delete_note_here(&mut self) -> Vec<Effect> {
        let Some(pos) = self.reading_position() else {
            return vec![Effect::Redraw];
        };
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let at = |r: CharRange| r.contains(pos) || r.start == pos;
        if let Some(i) = s.notes.iter().position(|n| at(n.range)) {
            let mut effects = self.delete_note(i);
            self.list = None;
            effects.retain(|e| !matches!(e, Effect::ShowList { .. }));
            return effects;
        }
        if let Some(i) = s.highlights.iter().position(|h| at(h.range)) {
            let mut effects = self.delete_highlight(i);
            self.list = None;
            effects.retain(|e| !matches!(e, Effect::ShowList { .. }));
            return effects;
        }
        self.tell("No note or highlight here.");
        vec![Effect::Redraw]
    }

    fn list_notes_quiet(&mut self) -> Vec<Effect> {
        let n = self.session.as_ref().map_or(0, |s| s.notes.len());
        let items: Vec<String> = (0..n).filter_map(|i| self.note_item(i)).collect();
        self.list = Some(ListKind::Notes);
        vec![Effect::ShowList {
            title: "Notes".into(),
            items,
        }]
    }

    /// Replaces note `i`'s text (from the edit-note prompt); empty keeps it.
    pub(crate) fn edit_note(&mut self, i: usize, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            self.note("Note unchanged.");
            return;
        }
        let Some(n) = self.session.as_mut().and_then(|s| s.notes.get_mut(i)) else {
            return;
        };
        n.text = text.to_owned();
        n.tags = parse_tags(text);
        n.ts = textweaver_store::now_ts();
        self.persist_marks();
        self.tell("Note updated.");
    }

    fn toggle_highlight(&mut self) {
        let Some(range) = self.note_target() else {
            self.tell("Nothing here to highlight.");
            return;
        };
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let pos = range.start;
        // On an existing highlight (the same range, or one containing the
        // cursor when nothing is selected): remove it.
        let existing = s
            .highlights
            .iter()
            .position(|h| h.range == range)
            .or_else(|| {
                s.selection
                    .is_none()
                    .then(|| s.highlights.iter().position(|h| h.range.contains(s.cursor)))
                    .flatten()
            });
        if let Some(i) = existing {
            let h = s.highlights.remove(i);
            let text = preview(&s.doc, h.range, 8);
            self.persist_marks();
            self.tell(&format!("Highlight removed: {text}"));
            return;
        }
        let text = preview(&s.doc, range, 8);
        s.highlights.push(UserHighlight {
            range,
            color: "yellow".into(),
            ts: textweaver_store::now_ts(),
        });
        s.highlights.sort_by_key(|h| (h.range.start, h.range.end));
        s.selection = None;
        s.selection_anchor = None;
        let pct = text_util::percent(&s.doc, pos);
        self.persist_marks();
        let msg = match self.settings.speech.verbosity {
            Verbosity::High => format!("Highlighted at {pct} percent: {text}"),
            _ => format!("Highlighted: {text}"),
        };
        self.tell(&msg);
    }

    fn highlight_items(&self) -> Vec<String> {
        let Some(s) = self.session.as_ref() else {
            return Vec::new();
        };
        s.highlights
            .iter()
            .map(|h| {
                let line = text_util::line_of(&s.doc, h.range.start) + 1;
                format!("{}, line {line}, {}", preview(&s.doc, h.range, 10), h.color)
            })
            .collect()
    }

    pub(crate) fn list_highlights(&mut self) -> Vec<Effect> {
        let items = self.highlight_items();
        let n = items.len();
        if n == 0 {
            self.tell("No highlights.");
            return vec![Effect::Redraw];
        }
        self.list = Some(ListKind::Highlights);
        self.tell(&format!(
            "Highlights, {n} {}. Enter goes to one, Delete removes it.",
            if n == 1 { "item" } else { "items" }
        ));
        vec![Effect::ShowList {
            title: "Highlights".into(),
            items,
        }]
    }

    pub(crate) fn go_to_highlight(&mut self, i: usize) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let Some(h) = s.highlights.get(i) else {
            return;
        };
        let target = h.range.start;
        let content = preview(&s.doc, h.range, 10);
        let msg = self.nav_message(Some("Highlight"), target, &content);
        self.jump(target, true, ReadAfter::Follow, &msg);
    }

    pub(crate) fn delete_highlight(&mut self, i: usize) -> Vec<Effect> {
        let Some(s) = self.session.as_mut() else {
            return vec![Effect::Redraw];
        };
        if i >= s.highlights.len() {
            return vec![Effect::Redraw];
        }
        let h = s.highlights.remove(i);
        let text = preview(&s.doc, h.range, 8);
        self.persist_marks();
        self.tell(&format!("Highlight removed: {text}"));
        let items = self.highlight_items();
        if items.is_empty() {
            self.list = None;
            return vec![Effect::Redraw];
        }
        self.list = Some(ListKind::Highlights);
        vec![
            Effect::ShowList {
                title: "Highlights".into(),
                items,
            },
            Effect::Redraw,
        ]
    }

    /// Rename or delete: the bookmark at the cursor directly, else the list
    /// (Delete and F2 act on its items).
    fn bookmark_manage(&mut self, delete: bool) -> Vec<Effect> {
        let at = self.session.as_ref().and_then(|s| {
            let pos = text_util::word_start(&s.doc, s.cursor);
            s.bookmarks.iter().position(|b| b.pos == pos)
        });
        match (at, delete) {
            (Some(i), true) => self.delete_bookmark(i),
            (Some(i), false) => self.rename_bookmark_prompt(i),
            (None, _) => {
                let mut effects = self.list_bookmarks();
                if self.list == Some(ListKind::Bookmarks) {
                    let verb = if delete { "Delete" } else { "F2" };
                    self.tell(&format!(
                        "Choose a bookmark and press {verb}{}.",
                        if delete { "" } else { " to rename it" }
                    ));
                }
                effects.push(Effect::Redraw);
                effects
            }
        }
    }

    pub(crate) fn delete_bookmark(&mut self, i: usize) -> Vec<Effect> {
        let Some(s) = self.session.as_mut() else {
            return vec![Effect::Redraw];
        };
        if i >= s.bookmarks.len() {
            return vec![Effect::Redraw];
        }
        let b = s.bookmarks.remove(i);
        let left = s.bookmarks.len();
        self.persist_marks();
        self.tell(&format!("Bookmark {} deleted.", b.name));
        if left > 0 && self.list == Some(ListKind::Bookmarks) {
            let mut e = self.list_bookmarks_quiet();
            e.push(Effect::Redraw);
            return e;
        }
        self.list = None;
        vec![Effect::Redraw]
    }

    pub(crate) fn rename_bookmark_prompt(&mut self, i: usize) -> Vec<Effect> {
        let Some(name) = self
            .session
            .as_ref()
            .and_then(|s| s.bookmarks.get(i))
            .map(|b| b.name.clone())
        else {
            return vec![Effect::Redraw];
        };
        self.list = None;
        self.pending_item = Some(i);
        let mut e = self.prompt(PromptPurpose::RenameBookmark);
        self.tell(&format!("Renaming bookmark {name}."));
        e.push(Effect::Redraw);
        e
    }

    /// Renames bookmark `i` (from the rename prompt); empty keeps the name,
    /// and a name in use by another bookmark is refused.
    pub(crate) fn rename_bookmark(&mut self, i: usize, name: &str) {
        let name = name.trim();
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let Some(old) = s.bookmarks.get(i).map(|b| b.name.clone()) else {
            return;
        };
        if name.is_empty() || name == old {
            self.note("Bookmark unchanged.");
            return;
        }
        if s.bookmarks.iter().any(|b| b.name == name) {
            self.error(&format!(
                "There is already a bookmark called {name}. Bookmark {old} unchanged."
            ));
            return;
        }
        if let Some(b) = s.bookmarks.get_mut(i) {
            b.name = name.to_owned();
        }
        self.persist_marks();
        self.tell(&format!("Bookmark {old} renamed to {name}."));
    }

    /// Saves bookmarks, notes, and highlights now (not while editing, when
    /// positions are in the source text; they are saved on leaving).
    pub(crate) fn persist_marks(&mut self) {
        if let Err(e) = self.save_position() {
            log::warn!("cannot save marks: {e}");
        }
    }

    /// Position of note `i`, if any (tests and frontends).
    pub fn note_position(&self, i: usize) -> Option<CharPos> {
        self.session.as_ref()?.notes.get(i).map(|n| n.range.start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_come_from_hash_words() {
        assert_eq!(
            parse_tags("Check this #Exam, and #exam again #todo."),
            vec!["exam", "todo"]
        );
        assert!(parse_tags("no tags # here").is_empty());
    }

    #[test]
    fn notes_round_trip_through_doc_state() {
        let mut st = DocState::default();
        let notes = vec![Note {
            id: "0000abcd".into(),
            range: CharRange::new(3, 9),
            anchor: "anchor".into(),
            text: "text".into(),
            tags: vec![],
            ts: 1,
        }];
        let hl = vec![UserHighlight {
            range: CharRange::new(1, 2),
            color: "yellow".into(),
            ts: 2,
        }];
        store_notes(&mut st, &notes, &hl);
        assert_eq!(load_notes(&st), (notes, hl));
        store_notes(&mut st, &[], &[]);
        assert!(st.extra.is_empty());
    }

    #[test]
    fn anchors_are_collapsed_and_cut() {
        assert_eq!(collapse("a \n b", 10), "a b");
        assert_eq!(collapse("abcdef", 3), "abc…");
    }
}
