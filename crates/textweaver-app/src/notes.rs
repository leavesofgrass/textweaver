//! Notes and highlights, and bookmark rename and delete.
//!
//! A note is text attached to a range (the selection, else the sentence at
//! the cursor), with `#tags` taken from its text (star's tag rule: split on
//! commas and spaces, leading `#` dropped). A highlight is a colored range.
//! Both move with edits exactly like bookmarks (one `EditOutcome` for all,
//! through [`DocState::shift`]), are listed accessibly (Enter jumps, Delete
//! removes, F2 edits), and are saved with the document's state.
//!
//! They are the store's typed [`Note`] and [`Highlight`]
//! (`DocState::notes` and `DocState::highlights`), the one model the vault
//! (`textweaver-vault`), `tw marks`, and sidecar sync use too.
//!
//! With the `publish` feature, [`notes_references`] and [`export_notes`]
//! write them as reference records (BibTeX, BibLaTeX, RIS, CSL-JSON),
//! as star's notes export did (Agent W4g; `tw marks --export`).

use textweaver_a11y::Verbosity;
use textweaver_core::{CharPos, CharRange, Direction, EditOutcome, Unit};
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;
use textweaver_speech::Earcon;
use textweaver_store::notes::{self as store_notes, color_name, highlight_color};
use textweaver_store::{DocState, Highlight, MarkKind, Note};
use textweaver_text::Document;
use textweaver_text::units::unit_at;

use crate::app::{App, ListKind};
use crate::command::{Effect, NoteCommand, PromptPurpose};
use crate::nav::ReadAfter;
use crate::text_util::{self, preview};

/// A highlighted range: the store's [`Highlight`] (named for the app so it
/// does not clash with [`crate::Highlight`], a range drawn on screen).
pub type UserHighlight = Highlight;

/// Longest anchor kept, in characters (star's limit).
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
pub(crate) fn collapse(s: &str, max: usize) -> String {
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

/// The note id of a comment the document carries.
fn comment_note_id(c: &textweaver_formats::DocumentComment) -> String {
    format!("comment-{}", c.id)
}

/// Adds a note for each comment the document carries (Word comments and
/// OpenDocument annotations, with their replies) that `notes` does not
/// have yet, matched by id, so a comment becomes a note once and keeps any
/// edit made to it. The note reads "Comment by Ada Example: ..." and is
/// tagged `comment` (and `resolved` when it is), so reading passes it with
/// the note signal. Returns how many were added.
pub(crate) fn add_document_comments(notes: &mut Vec<Note>, doc: &Document) -> usize {
    add_document_comments_from(notes, doc, &textweaver_formats::comments(&doc.meta))
}

/// [`add_document_comments`] for `comments` (the changes list passes the
/// document's comments after an edit).
pub(crate) fn add_document_comments_from(
    notes: &mut Vec<Note>,
    doc: &Document,
    comments: &[textweaver_formats::DocumentComment],
) -> usize {
    if comments.is_empty() {
        return 0;
    }
    let len = doc.len_chars();
    let now = textweaver_store::now_ts();
    let mut added = 0;
    for c in comments {
        let id = comment_note_id(c);
        if notes.iter().any(|n| n.id == id) {
            continue;
        }
        let range = c.range.clamp_to(len);
        let mut tags = vec!["comment".to_owned()];
        if c.resolved {
            tags.push("resolved".to_owned());
        }
        notes.push(Note {
            id,
            range,
            anchor: store_notes::collapse(&doc.slice(range), ANCHOR_CHARS),
            note: c.spoken(),
            tags,
            created: now,
            ts: now,
            ..Note::default()
        });
        added += 1;
    }
    notes.sort_by_key(|n| (n.range.start, n.range.end));
    added
}

impl App {
    /// Runs a notes, highlights, or bookmark management command.
    pub(crate) fn notes_command(&mut self, c: NoteCommand) -> Vec<Effect> {
        if self.session.is_none() {
            let open = self.key(textweaver_keymap::ActionId::Open);
            let msg = self.msg_args("app-no-document-open", &args!["key" => open]);
            self.tell(&msg);
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
            NoteCommand::Links => return self.note_links_here(),
        }
        vec![Effect::Redraw]
    }

    /// The selection, else the sentence at the cursor, else the word.
    pub(crate) fn note_target(&self) -> Option<CharRange> {
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
            let msg = self.msg("common-cancelled");
            self.note(&msg);
            return;
        }
        let Some(range) = self.note_target() else {
            let msg = self.msg("notes-nothing-to-attach");
            self.tell(&msg);
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
            self.msg_args("notes-added", &args!["on" => on])
        } else {
            self.msg_args(
                "notes-added-with-tags",
                &args!["tags" => tags.join(", "), "on" => on],
            )
        };
        self.tell(&msg);
    }

    pub(crate) fn note_item(&self, i: usize) -> Option<String> {
        let s = self.session.as_ref()?;
        let n = s.notes.get(i)?;
        let line = text_util::line_of(&s.doc, n.range.start) + 1;
        let lost = if crate::relocate::is_marked(&n.extra) {
            "yes"
        } else {
            "no"
        };
        Some(self.msg_args(
            "notes-item",
            &args![
                "note" => n.note.as_str(),
                "line" => line,
                "anchor" => collapse(&n.anchor, 60),
                "lost" => lost
            ],
        ))
    }

    /// The notes list's rows: each note, with "Links: 2 out, 1 in" after
    /// a note that has links (crate::relations).
    fn note_items(&mut self) -> Vec<String> {
        let n = self.session.as_ref().map_or(0, |s| s.notes.len());
        let counts = self.relations_counts();
        (0..n)
            .filter_map(|i| {
                let item = self.note_item(i)?;
                Some(match counts.get(i).cloned().flatten() {
                    Some(links) => {
                        format!("{}. {links}", item.trim_end().trim_end_matches('.'))
                    }
                    None => item,
                })
            })
            .collect()
    }

    pub(crate) fn list_notes(&mut self) -> Vec<Effect> {
        let n = self.session.as_ref().map_or(0, |s| s.notes.len());
        if n == 0 {
            let key = self.key(textweaver_keymap::ActionId::AddNote);
            let msg = self.msg_args("notes-none", &args!["key" => key]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let items = self.note_items();
        self.list = Some(ListKind::Notes);
        let msg = self.msg_args("notes-list-intro", &args!["n" => n]);
        self.tell(&msg);
        vec![Effect::ShowList {
            title: self.msg("notes-list-title"),
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
        let content = self.msg_args(
            "notes-note-content",
            &args!["note" => n.note.as_str(), "anchor" => collapse(&n.anchor, 60)],
        );
        let label = self.msg_args(
            "notes-note-label",
            &args!["i" => i + 1, "n" => s.notes.len()],
        );
        let mut msg = self.nav_message(Some(&label), target, &content);
        if wrapped {
            msg = self.msg_args("nav-wrapped", &args!["message" => msg]);
        }
        self.jump(target, true, ReadAfter::Follow, &msg);
    }

    fn note_step(&mut self, dir: Direction) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        if s.notes.is_empty() {
            let key = self.key(textweaver_keymap::ActionId::AddNote);
            let msg = self.msg_args("notes-none", &args!["key" => key]);
            self.tell(&msg);
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
        s.saved.record_deletion(MarkKind::Note, &n.id);
        let left = s.notes.len();
        self.persist_marks();
        let msg = self.msg_args("notes-deleted", &args!["text" => collapse(&n.note, 40)]);
        self.tell(&msg);
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
        let msg = self.msg("notes-none-here");
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    fn list_notes_quiet(&mut self) -> Vec<Effect> {
        let items = self.note_items();
        self.list = Some(ListKind::Notes);
        vec![Effect::ShowList {
            title: self.msg("notes-list-title"),
            items,
        }]
    }

    /// Replaces note `i`'s text (from the edit-note prompt); empty keeps it.
    pub(crate) fn edit_note(&mut self, i: usize, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            let msg = self.msg("notes-unchanged");
            self.note(&msg);
            return;
        }
        let Some(n) = self.session.as_mut().and_then(|s| s.notes.get_mut(i)) else {
            return;
        };
        n.note = text.to_owned();
        n.tags = parse_tags(text);
        n.ts = textweaver_store::now_ts();
        self.persist_marks();
        let msg = self.msg("notes-updated");
        self.tell(&msg);
    }

    fn toggle_highlight(&mut self) {
        let Some(range) = self.note_target() else {
            let msg = self.msg("notes-nothing-to-highlight");
            self.tell(&msg);
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
            s.saved.record_deletion(MarkKind::Highlight, &h.id);
            let text = preview(&s.doc, h.range, 8);
            self.persist_marks();
            let msg = self.msg_args("notes-highlight-removed", &args!["text" => text]);
            self.tell(&msg);
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
            Verbosity::High => {
                self.msg_args("notes-highlighted-at", &args!["pct" => pct, "text" => text])
            }
            _ => self.msg_args("notes-highlighted", &args!["text" => text]),
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
                    "yes"
                } else {
                    "no"
                };
                self.msg_args(
                    "notes-highlight-item",
                    &args![
                        "text" => preview(&s.doc, h.range, 10),
                        "line" => line,
                        "color" => color_name(&h.color),
                        "lost" => lost
                    ],
                )
            })
            .collect()
    }

    pub(crate) fn list_highlights(&mut self) -> Vec<Effect> {
        let items = self.highlight_items();
        let n = items.len();
        if n == 0 {
            let key = self.key(textweaver_keymap::ActionId::HighlightSelection);
            let msg = self.msg_args("notes-no-highlights", &args!["key" => key]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        self.list = Some(ListKind::Highlights);
        let msg = self.msg_args("notes-highlights-intro", &args!["n" => n]);
        self.tell(&msg);
        vec![Effect::ShowList {
            title: self.msg("notes-highlights-title"),
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
        let label = self.msg("notes-highlight-label");
        let msg = self.nav_message(Some(&label), target, &content);
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
        s.saved.record_deletion(MarkKind::Highlight, &h.id);
        let text = preview(&s.doc, h.range, 8);
        self.persist_marks();
        let msg = self.msg_args("notes-highlight-removed", &args!["text" => text]);
        self.tell(&msg);
        let items = self.highlight_items();
        if items.is_empty() {
            self.list = None;
            return vec![Effect::Redraw];
        }
        self.list = Some(ListKind::Highlights);
        vec![
            Effect::ShowList {
                title: self.msg("notes-highlights-title"),
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
                    let msg = self.msg(if delete {
                        "notes-choose-bookmark-delete"
                    } else {
                        "notes-choose-bookmark-rename"
                    });
                    self.tell(&msg);
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
        s.saved.record_deletion(MarkKind::Bookmark, &b.id);
        let left = s.bookmarks.len();
        self.persist_marks();
        let msg = self.msg_args("notes-bookmark-deleted", &args!["name" => b.name.as_str()]);
        self.tell(&msg);
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
        let msg = self.msg_args("notes-renaming-bookmark", &args!["name" => name]);
        self.tell(&msg);
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
            let msg = self.msg("notes-bookmark-unchanged");
            self.note(&msg);
            return;
        }
        if s.bookmarks.iter().any(|b| b.name == name) {
            let msg = self.msg_args(
                "notes-bookmark-name-taken",
                &args!["name" => name, "old" => old],
            );
            self.error(&msg);
            return;
        }
        if let Some(b) = s.bookmarks.get_mut(i) {
            b.name = name.to_owned();
            b.ts = textweaver_store::now_ts();
        }
        self.persist_marks();
        let msg = self.msg_args(
            "notes-bookmark-renamed",
            &args!["old" => old, "name" => name],
        );
        self.tell(&msg);
    }

    /// Saves bookmarks, notes, and highlights now (not while editing, when
    /// positions are in the source text; they are saved on leaving).
    pub(crate) fn persist_marks(&mut self) {
        if let Err(e) = self.save_position() {
            log::warn!("cannot save marks: {e}");
        }
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
            let msg = self.msg_args("notes-signal", &args!["text" => text]);
            self.show(&msg);
        }
    }

    /// "Has a note: …", said after a caret move onto a note's passage (at
    /// normal verbosity and above), when there is one there.
    pub(crate) fn note_here_suffix(&self, pos: CharPos) -> Option<String> {
        if self.settings.speech.verbosity < Verbosity::Normal {
            return None;
        }
        self.note_at(pos)
            .map(|n| self.msg_args("notes-has-note", &args!["text" => collapse(&n.note, 60)]))
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
            let msg = self.msg("notes-nothing-to-export");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let date = crate::templates::local_date();
        let sheet = study_sheet(self.cat(), &s.doc, &s.title, &s.notes, &s.highlights, &date);
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
                let id = match (n, h) {
                    (_, 0) => "notes-study-sheet-saved-notes",
                    (0, _) => "notes-study-sheet-saved-highlights",
                    _ => "notes-study-sheet-saved-both",
                };
                let question = self.msg_args(
                    id,
                    &args![
                        "n" => n,
                        "h" => h,
                        "file" => crate::authoring_state::file_name(&out),
                        "folder" => folder.display().to_string()
                    ],
                );
                self.offer_open(out.display().to_string(), &question);
            }
            Err(e) => {
                let msg =
                    self.msg_args("notes-study-sheet-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
            }
        }
        vec![Effect::Redraw]
    }

    /// The palette's `self_test`: the study sheet's notes and highlights
    /// as prompts with hidden answers (crate::reveal).
    pub(crate) fn self_test(&mut self) -> Vec<Effect> {
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let items = self_test_items(self.cat(), &s.doc, &s.title, &s.notes, &s.highlights);
        if items.is_empty() {
            let msg = self.msg("reveal-nothing-to-test");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let title = self.msg_args(
            "reveal-self-test-title",
            &args!["title" => s.title.as_str()],
        );
        let list = crate::reveal::RevealList::new(title, items);
        let intro = self.msg_args("reveal-self-test-intro", &args!["n" => list.len()]);
        self.show_reveal_list(&intro, list)
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

/// The study sheet's contents: the headings (start, level, text), and the
/// notes and highlights in document order, each with its quoted passage
/// and the heading it falls under. The study sheet and the self-test are
/// made from it.
struct SheetItems<'a> {
    headings: Vec<(CharPos, u8, String)>,
    items: Vec<SheetItem<'a>>,
}

/// A note or highlight on the study sheet.
struct SheetItem<'a> {
    at: CharPos,
    /// The heading it falls under (None before the first).
    under: Option<usize>,
    /// The quoted passage.
    passage: String,
    /// The note, or None for a highlight.
    note: Option<&'a Note>,
    /// The highlight's color, for a highlight.
    color: Option<&'a str>,
}

fn sheet_items<'a>(
    doc: &Document,
    notes: &'a [Note],
    highlights: &'a [Highlight],
) -> SheetItems<'a> {
    use textweaver_core::MarkerKind;
    let headings: Vec<(CharPos, u8, String)> = doc
        .marker_index()
        .iter(MarkerKind::Heading, None)
        .map(|m| (m.range.start, m.level, collapse(&doc.slice(m.range), 120)))
        .collect();
    let under = |p: CharPos| headings.iter().rposition(|h| h.0 <= p);
    let mut items = Vec::new();
    for n in notes {
        let passage = collapse(&doc.slice(n.range), 400);
        let passage = if passage.is_empty() {
            collapse(&n.anchor, 400)
        } else {
            passage
        };
        items.push(SheetItem {
            at: n.range.start,
            under: under(n.range.start),
            passage,
            note: Some(n),
            color: None,
        });
    }
    for h in highlights {
        items.push(SheetItem {
            at: h.range.start,
            under: under(h.range.start),
            passage: collapse(&doc.slice(h.range), 400),
            note: None,
            color: Some(&h.color),
        });
    }
    items.sort_by_key(|i| i.at);
    SheetItems { headings, items }
}

/// The self-test's prompts from the study sheet: a note asks with its own
/// words and the passage it quotes is the answer; a highlight asks what
/// was highlighted under its heading. The section is the heading, or the
/// document's title before the first heading. Items with no passage are
/// left out (nothing to reveal).
pub(crate) fn self_test_items(
    c: &Catalog,
    doc: &Document,
    title: &str,
    notes: &[Note],
    highlights: &[Highlight],
) -> Vec<crate::reveal::RevealItem> {
    let sheet = sheet_items(doc, notes, highlights);
    sheet
        .items
        .into_iter()
        .filter(|i| !i.passage.is_empty())
        .map(|i| {
            let section = i
                .under
                .map_or(title, |h| sheet.headings[h].2.as_str())
                .to_owned();
            let prompt = match i.note.map(|n| n.note.trim()).filter(|t| !t.is_empty()) {
                Some(note) => c.fmt(
                    "reveal-prompt-note",
                    &args!["note" => note, "section" => section],
                ),
                None => c.fmt("reveal-prompt-highlight", &args!["section" => section]),
            };
            crate::reveal::RevealItem {
                prompt,
                answer: i.passage,
            }
        })
        .collect()
}

/// The study sheet: a title, then one section per heading that has notes
/// or highlights under it (in document order, at the heading's level less
/// one, so the sheet's own title stays level 1), each passage quoted with
/// its note after it.
pub(crate) fn study_sheet(
    c: &Catalog,
    doc: &Document,
    title: &str,
    notes: &[Note],
    highlights: &[Highlight],
    date: &str,
) -> String {
    let SheetItems { headings, items } = sheet_items(doc, notes, highlights);
    let items = items.into_iter().map(|i| {
        let entry = match i.note {
            Some(n) => {
                let mut entry = format!("- > {}\n\n  {}", i.passage, n.note.trim());
                if !n.tags.is_empty() {
                    entry.push_str(&format!(
                        " {}",
                        c.fmt("notes-sheet-tags", &args!["tags" => n.tags.join(", ")])
                    ));
                }
                entry
            }
            None => format!(
                "- > {}\n\n  {}",
                i.passage,
                c.fmt(
                    "notes-sheet-highlighted",
                    &args!["color" => color_name(i.color.unwrap_or_default())]
                )
            ),
        };
        (i.at, i.under, entry)
    });
    let mut out = format!(
        "# {}\n\n{}\n",
        c.fmt("notes-sheet-title", &args!["title" => title]),
        c.fmt("notes-sheet-exported", &args!["date" => date])
    );
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
                None => out.push_str(&format!(
                    "\n## {}\n",
                    c.tr("notes-sheet-before-first-heading")
                )),
            }
        }
        out.push('\n');
        out.push_str(&entry);
        out.push('\n');
    }
    out
}

/// What [`notes_references`] needs to know about the document.
#[cfg(feature = "publish")]
#[derive(Clone, Debug, Default)]
pub struct NotesRecords<'a> {
    /// The document's title: every record's title.
    pub title: &'a str,
    /// The document's author, when known.
    pub author: Option<&'a str>,
    /// The start of every key (`book` gives `book-note-1`); letters,
    /// digits, and dashes are kept, and `notes` is used when none are left.
    pub key: &'a str,
    /// The document's web address, when it came from the web.
    pub url: Option<&'a str>,
    /// The document's length in chars, for each record's percentage (in
    /// CSL-JSON's `textweaver-position`).
    pub doc_len: Option<usize>,
}

/// A key made of `s`'s letters and digits, lowercase, dashes between
/// words.
#[cfg(feature = "publish")]
fn key_part(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_owned();
    if out.is_empty() {
        "notes".to_owned()
    } else {
        out
    }
}

/// Notes, then highlights, as reference records of the citation crate
/// (type `document`, BibTeX `@misc`, RIS `GEN`), one per note or
/// highlight, in document order: the document's title and author, the
/// noted passage as the abstract, the note as the note (with the key it
/// cites), the tags as keywords, and the date it was made.
#[cfg(feature = "publish")]
pub fn notes_references(
    notes: &[Note],
    highlights: &[Highlight],
    opts: &NotesRecords<'_>,
) -> Vec<textweaver_cite::Reference> {
    use textweaver_cite::{CslDate, Name, Reference};
    let prefix = key_part(opts.key);
    let base = |id: String, ts: i64, start: CharPos| {
        let mut r = Reference::new(&id, "document");
        r.title = (!opts.title.trim().is_empty()).then(|| opts.title.trim().to_owned());
        r.author = opts
            .author
            .filter(|a| !a.trim().is_empty())
            .map(|a| vec![Name::parse(a)])
            .unwrap_or_default();
        r.url = opts.url.map(str::to_owned);
        if ts > 0 {
            let day: String = textweaver_store::time::rfc3339(ts)
                .chars()
                .take(10)
                .collect();
            r.issued = CslDate::parse(&day);
        }
        if let Some(len) = opts.doc_len {
            r.extra.insert(
                "textweaver-position".into(),
                format!("{} percent", textweaver_store::percent(start, len)).into(),
            );
        }
        r
    };
    let mut out = Vec::with_capacity(notes.len() + highlights.len());
    for (i, n) in notes.iter().enumerate() {
        let mut r = base(
            format!("{prefix}-note-{}", i + 1),
            n.created.max(n.ts),
            n.range.start,
        );
        let passage = collapse(&n.anchor, 2000);
        r.abstract_text = (!passage.is_empty()).then_some(passage);
        let mut note = n.note.trim().to_owned();
        if !n.cite.trim().is_empty() {
            if !note.is_empty() {
                note.push(' ');
            }
            note.push_str(&format!("Cites {}.", n.cite.trim()));
        }
        r.note = (!note.is_empty()).then_some(note);
        r.keyword = (!n.tags.is_empty()).then(|| n.tags.join(", "));
        out.push(r);
    }
    for (i, h) in highlights.iter().enumerate() {
        let mut r = base(format!("{prefix}-highlight-{}", i + 1), h.ts, h.range.start);
        let passage = collapse(&h.text, 2000);
        r.abstract_text = (!passage.is_empty()).then_some(passage);
        r.note = Some(format!("Highlighted, {}.", color_name(&h.color)));
        out.push(r);
    }
    out
}

/// [`notes_references`] written in `format`.
///
/// # Errors
///
/// When CSL-JSON cannot be written (it always can for these records).
#[cfg(feature = "publish")]
pub fn export_notes(
    notes: &[Note],
    highlights: &[Highlight],
    opts: &NotesRecords<'_>,
    format: textweaver_cite::Format,
) -> textweaver_cite::Result<String> {
    textweaver_cite::formats::write(&notes_references(notes, highlights, opts), format)
}

#[cfg(test)]
mod tests {
    use textweaver_core::Edit;

    use super::*;

    #[test]
    fn document_comments_become_notes_once() {
        use textweaver_formats::{COMMENTS_PROPERTY, CommentReply, DocumentComment};
        let mut doc = Document::from_plain_text("Read chapter two first.");
        let comments = vec![DocumentComment {
            id: "0".into(),
            range: CharRange::new(5, 16),
            author: "Ada Example".into(),
            text: "Check this date.".into(),
            replies: vec![CommentReply {
                author: "Bo Example".into(),
                text: "Fixed.".into(),
                ..CommentReply::default()
            }],
            resolved: true,
            ..DocumentComment::default()
        }];
        doc.meta.properties.insert(
            COMMENTS_PROPERTY.to_owned(),
            serde_json::to_string(&comments).unwrap_or_default(),
        );
        let mut notes = vec![Note {
            id: "mine".into(),
            range: CharRange::new(0, 4),
            note: "My own note".into(),
            ..Note::default()
        }];
        assert_eq!(add_document_comments(&mut notes, &doc), 1);
        assert_eq!(notes.len(), 2);
        let n = &notes[1];
        assert_eq!(n.id, "comment-0");
        assert_eq!(n.anchor, "chapter two");
        assert_eq!(
            n.note,
            "Comment by Ada Example: Check this date. Reply by Bo Example: Fixed. Resolved."
        );
        assert_eq!(n.tags, vec!["comment", "resolved"]);
        // Opening again adds nothing, and an edited comment note is kept.
        notes[1].note = "Edited".into();
        assert_eq!(add_document_comments(&mut notes, &doc), 0);
        assert_eq!(notes[1].note, "Edited");
    }

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

    #[cfg(feature = "publish")]
    #[test]
    fn notes_become_reference_records() {
        use textweaver_cite::Format;
        let notes = vec![Note {
            id: "n1".into(),
            range: CharRange::new(10, 20),
            anchor: "the  noted\npassage".into(),
            note: "Look here".into(),
            tags: vec!["exam".into(), "ch2".into()],
            cite: "doe2020".into(),
            created: 1_790_344_987,
            ts: 1_790_344_987,
            ..Note::default()
        }];
        let highlights = vec![Highlight {
            id: "h1".into(),
            range: CharRange::new(30, 40),
            color: "#ffff00".into(),
            text: "bright words".into(),
            ts: 1_790_344_987,
            ..Highlight::default()
        }];
        let opts = NotesRecords {
            title: "Cell Biology",
            author: Some("Ada Lovelace"),
            key: "Cell Biology (2nd ed.)",
            url: None,
            doc_len: Some(100),
        };
        let refs = notes_references(&notes, &highlights, &opts);
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].id, "cell-biology-2nd-ed-note-1");
        assert_eq!(refs[1].id, "cell-biology-2nd-ed-highlight-1");
        assert_eq!(refs[0].abstract_text.as_deref(), Some("the noted passage"));
        assert_eq!(refs[0].note.as_deref(), Some("Look here Cites doe2020."));
        assert_eq!(refs[0].keyword.as_deref(), Some("exam, ch2"));
        assert_eq!(refs[1].note.as_deref(), Some("Highlighted, yellow."));
        assert_eq!(
            refs[0].issued.as_ref().and_then(|d| d.iso()).as_deref(),
            Some("2026-09-25")
        );

        let bib = export_notes(&notes, &highlights, &opts, Format::BibTex).unwrap();
        assert!(bib.contains("@misc{cell-biology-2nd-ed-note-1,"), "{bib}");
        assert!(bib.contains("Look here Cites doe2020."), "{bib}");
        let ris = export_notes(&notes, &highlights, &opts, Format::Ris).unwrap();
        assert!(ris.contains("TY  - GEN"), "{ris}");
        assert!(ris.contains("KW  - exam"), "{ris}");
        assert!(ris.contains("N1  - Highlighted, yellow."), "{ris}");
        let json = export_notes(&notes, &highlights, &opts, Format::CslJson).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v[0]["type"], "document");
        assert_eq!(v[0]["title"], "Cell Biology");
        assert_eq!(v[0]["textweaver-position"], "10 percent");
        // Each format reads back as the same records.
        for f in [Format::BibTex, Format::Ris, Format::CslJson] {
            let text = export_notes(&notes, &highlights, &opts, f).unwrap();
            let back = textweaver_cite::formats::parse(&text, f).unwrap();
            assert_eq!(back.len(), 2, "{f:?}");
            assert_eq!(back[0].note.as_deref(), Some("Look here Cites doe2020."));
        }
        assert_eq!(key_part("  "), "notes");
    }

    #[test]
    fn anchors_are_collapsed_and_cut() {
        assert_eq!(collapse("a \n b", 10), "a b");
        assert_eq!(collapse("abcdef", 3), "abc…");
    }
}
