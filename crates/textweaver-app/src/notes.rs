//! Notes and highlights, and bookmark rename and delete.
//!
//! A note is text attached to a range (the selection, else the sentence at
//! the cursor), with `#tags` taken from its text (Star's tag rule: split on
//! commas and spaces, leading `#` dropped). A highlight is a colored range.
//! Both move with edits exactly like bookmarks (one `EditOutcome` for all,
//! through [`DocState::shift`]), are listed accessibly (Enter jumps, Delete
//! removes, F2 edits), and are saved with the document's state.
//!
//! They are the store's typed [`Note`] and [`Highlight`]
//! (`DocState::notes` and `DocState::highlights`), the one model the vault
//! (`textweaver-vault`), `tw marks`, and sidecar sync use too.

use textweaver_a11y::Verbosity;
use textweaver_core::{CharPos, CharRange, Direction, EditOutcome, Unit};
use textweaver_speech::Earcon;
use textweaver_store::notes::{self as store_notes, color_name, highlight_color};
use textweaver_store::{DocState, Highlight, Note};
use textweaver_text::Document;
use textweaver_text::units::unit_at;

use crate::app::{App, ListKind};
use crate::command::{Effect, NoteCommand, PromptPurpose};
use crate::nav::ReadAfter;
use crate::text_util::{self, preview};

/// A highlighted range: the store's [`Highlight`] (named for the app so it
/// does not clash with [`crate::Highlight`], a range drawn on screen).
pub type UserHighlight = Highlight;

/// Longest anchor kept, in characters (Star's limit).
const ANCHOR_CHARS: usize = store_notes::ANCHOR_MAX_CHARS;

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

/// Whitespace collapsed, cut to `max` chars with an ellipsis (for lists
/// and announcements).
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

/// An id no note or highlight in `notes` and `highlights` uses.
fn fresh_id(notes: &[Note], highlights: &[Highlight]) -> String {
    loop {
        let id = store_notes::new_id();
        if !notes.iter().any(|n| n.id == id) && !highlights.iter().any(|h| h.id == id) {
            return id;
        }
    }
}

/// Moves notes, highlights, and bookmarks across an edit with the store's
/// one rule ([`DocState::shift`]): a note whose text was deleted stays,
/// collapsed at the edit point (its anchor keeps the text); a highlight
/// whose text was deleted is dropped.
pub(crate) fn shift_marks(
    notes: &mut Vec<Note>,
    highlights: &mut Vec<Highlight>,
    bookmarks: &mut Vec<textweaver_store::Bookmark>,
    outcomes: &[EditOutcome],
) {
    let mut state = DocState {
        notes: std::mem::take(notes),
        highlights: std::mem::take(highlights),
        bookmarks: std::mem::take(bookmarks),
        ..DocState::default()
    };
    for o in outcomes {
        state.shift(o);
    }
    *notes = state.notes;
    *highlights = state.highlights;
    *bookmarks = state.bookmarks;
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
        let anchor = store_notes::collapse(&s.doc.slice(range), ANCHOR_CHARS);
        let id = fresh_id(&s.notes, &s.highlights);
        let tags = parse_tags(text);
        let now = textweaver_store::now_ts();
        s.notes.push(Note {
            id,
            range,
            anchor: anchor.clone(),
            note: text.to_owned(),
            tags: tags.clone(),
            created: now,
            ts: now,
            ..Note::default()
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
        let lost = if crate::relocate::is_marked(&n.extra) {
            " Not found after the file changed."
        } else {
            ""
        };
        Some(format!(
            "{}, line {line}. On: {}{lost}",
            n.note,
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
        let content = format!("{}. On: {}", n.note, collapse(&n.anchor, 60));
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
        self.tell(&format!("Note deleted: {}.", collapse(&n.note, 40)));
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
        n.note = text.to_owned();
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
        let id = fresh_id(&s.notes, &s.highlights);
        s.highlights.push(Highlight {
            id,
            range,
            color: highlight_color("yellow"),
            text: store_notes::collapse(&s.doc.slice(range), store_notes::HIGHLIGHT_TEXT_MAX_CHARS),
            ts: textweaver_store::now_ts(),
            extra: serde_json::Map::new(),
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
                let lost = if crate::relocate::is_marked(&h.extra) {
                    ", not found after the file changed"
                } else {
                    ""
                };
                format!(
                    "{}, line {line}, {}{lost}",
                    preview(&s.doc, h.range, 10),
                    color_name(&h.color)
                )
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

    /// The note whose passage holds `pos`, if any.
    fn note_at(&self, pos: CharPos) -> Option<&Note> {
        let notes = &self.session.as_ref()?.notes;
        // Sorted by start: the last one starting at or before `pos` that
        // still covers it.
        let upto = notes.partition_point(|n| n.range.start <= pos);
        notes[..upto].iter().rev().find(|n| {
            n.range.start <= pos && pos < n.range.end.max(n.range.start.saturating_add(1))
        })
    }

    /// While reading, signals the passage of a note once as speech reaches
    /// it: a short two-tone earcon, and at normal verbosity and above the
    /// note on the status line ("Note: check this for the exam"). The
    /// reading itself is never interrupted.
    pub(crate) fn note_signal(&mut self, spoken: CharRange) {
        let Some(n) = self.note_at(spoken.start) else {
            self.authoring.note_signalled = None;
            return;
        };
        if self.authoring.note_signalled.as_deref() == Some(n.id.as_str()) {
            return;
        }
        let (id, text) = (n.id.clone(), collapse(&n.note, 80));
        self.authoring.note_signalled = Some(id);
        // The earcon only where textweaver's voice is reading.
        if self.route(textweaver_a11y::Channel::Reading).speak {
            self.speech.tone(880.0, 40);
            self.speech.tone(1320.0, 60);
        }
        if self.settings.speech.verbosity >= Verbosity::Normal {
            self.show(&format!("Note: {text}"));
        }
    }

    /// "Has a note: …", said after a caret move onto a note's passage (at
    /// normal verbosity and above), when there is one there.
    pub(crate) fn note_here_suffix(&self, pos: CharPos) -> Option<String> {
        if self.settings.speech.verbosity < Verbosity::Normal {
            return None;
        }
        self.note_at(pos)
            .map(|n| format!("Has a note: {}", collapse(&n.note, 60)))
    }

    /// Where files made from a document that has no file yet go (exports,
    /// study sheets): the folder textweaver was started in, as Save As
    /// suggests; in a session that keeps no files (tests), the system's
    /// temporary folder.
    pub(crate) fn loose_folder(&self) -> std::path::PathBuf {
        if self.paths.is_some() {
            std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir())
        } else {
            std::env::temp_dir()
        }
    }

    /// The palette's `export_study_sheet`: the notes and highlights as
    /// Markdown, grouped under the headings they fall under, written next
    /// to the document as `NAME-study-sheet.md`.
    pub(crate) fn export_study_sheet(&mut self) -> Vec<Effect> {
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        if s.notes.is_empty() && s.highlights.is_empty() {
            self.tell("No notes or highlights to export.");
            return vec![Effect::Redraw];
        }
        let date = crate::templates::local_date();
        let sheet = study_sheet(&s.doc, &s.title, &s.notes, &s.highlights, &date);
        let path = s.doc.meta.path.clone();
        let folder = path
            .as_ref()
            .and_then(|p| p.parent().map(std::path::Path::to_owned))
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| self.loose_folder());
        let stem = path
            .as_ref()
            .and_then(|p| p.file_stem().map(|x| x.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "notes".to_owned());
        let out = folder.join(format!("{stem}-study-sheet.md"));
        match textweaver_store::atomic_write(&out, sheet.as_bytes()) {
            Ok(()) => {
                let (n, h) = (s_count(&self.session, true), s_count(&self.session, false));
                let what = match (n, h) {
                    (n, 0) => format!("{n} {}", plural_word(n, "note")),
                    (0, h) => format!("{h} {}", plural_word(h, "highlight")),
                    (n, h) => format!(
                        "{n} {} and {h} {}",
                        plural_word(n, "note"),
                        plural_word(h, "highlight")
                    ),
                };
                self.offer_open(
                    out.display().to_string(),
                    &format!(
                        "Study sheet with {what} saved as {} in {}. Open it? y or n.",
                        crate::authoring_state::file_name(&out),
                        folder.display()
                    ),
                );
            }
            Err(e) => self.error(&format!("Could not write the study sheet: {e}")),
        }
        vec![Effect::Redraw]
    }
}

fn s_count(s: &Option<crate::app::Session>, notes: bool) -> usize {
    s.as_ref().map_or(0, |s| {
        if notes {
            s.notes.len()
        } else {
            s.highlights.len()
        }
    })
}

fn plural_word(n: usize, one: &str) -> String {
    if n == 1 {
        one.to_owned()
    } else {
        format!("{one}s")
    }
}

/// The study sheet: a title, then one section per heading that has notes
/// or highlights under it (in document order, at the heading's level less
/// one, so the sheet's own title stays level 1), each passage quoted with
/// its note after it.
pub(crate) fn study_sheet(
    doc: &Document,
    title: &str,
    notes: &[Note],
    highlights: &[Highlight],
    date: &str,
) -> String {
    use textweaver_core::MarkerKind;
    let headings: Vec<(CharPos, u8, String)> = doc
        .marker_index()
        .iter(MarkerKind::Heading, None)
        .map(|m| (m.range.start, m.level, collapse(&doc.slice(m.range), 120)))
        .collect();
    // Every item with the heading it falls under (None before the first).
    let mut items: Vec<(CharPos, Option<usize>, String)> = Vec::new();
    let under = |p: CharPos| headings.iter().rposition(|h| h.0 <= p);
    for n in notes {
        let passage = collapse(&doc.slice(n.range), 400);
        let passage = if passage.is_empty() {
            collapse(&n.anchor, 400)
        } else {
            passage
        };
        let mut entry = format!("- > {passage}\n\n  {}", n.note.trim());
        if !n.tags.is_empty() {
            entry.push_str(&format!(" (tags: {})", n.tags.join(", ")));
        }
        items.push((n.range.start, under(n.range.start), entry));
    }
    for h in highlights {
        let passage = collapse(&doc.slice(h.range), 400);
        let color = color_name(&h.color);
        items.push((
            h.range.start,
            under(h.range.start),
            format!("- > {passage}\n\n  Highlighted, {color}."),
        ));
    }
    items.sort_by_key(|i| i.0);
    let mut out = format!("# Study sheet: {title}\n\nExported from textweaver on {date}.\n");
    let mut current: Option<Option<usize>> = None;
    for (_, h, entry) in items {
        if current != Some(h) {
            current = Some(h);
            match h {
                Some(i) => {
                    let (_, level, text) = &headings[i];
                    let hashes = "#".repeat(usize::from((*level).clamp(1, 5)) + 1);
                    out.push_str(&format!("\n{hashes} {text}\n"));
                }
                None => out.push_str("\n## Before the first heading\n"),
            }
        }
        out.push('\n');
        out.push_str(&entry);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use textweaver_core::Edit;

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
    fn marks_shift_with_the_store_rule() {
        let mut notes = vec![Note {
            id: "n".into(),
            range: CharRange::new(10, 20),
            ..Note::default()
        }];
        let mut highlights = vec![Highlight {
            id: "h".into(),
            range: CharRange::new(30, 40),
            ..Highlight::default()
        }];
        let mut bookmarks = Vec::new();
        shift_marks(
            &mut notes,
            &mut highlights,
            &mut bookmarks,
            &[
                Edit::insert(0, "abc").outcome(),
                Edit::delete(33..43).outcome(),
            ],
        );
        assert_eq!(notes[0].range, CharRange::new(13, 23));
        assert!(highlights.is_empty(), "deleted text drops its highlight");
    }

    #[test]
    fn anchors_are_collapsed_and_cut() {
        assert_eq!(collapse("a \n b", 10), "a b");
        assert_eq!(collapse("abcdef", 3), "abc…");
    }
}
