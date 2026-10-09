//! Relations between notes, the knowledge graph as lists (B1-g1).
//!
//! A note links to other notes with one of star's ten relation types
//! ([`RelationType`]); the links are kept on the note
//! ([`Note::relations`]), so they are saved and synced with it. Lists,
//! never a picture:
//!
//! - in the notes list, a note with links ends with "Links: 2 out, 1 in",
//!   and Space opens its links (the `note_links` command does the same for
//!   the note at the cursor);
//! - a note's links: one row per link, type first ("supports: Chapter 3
//!   note"), then "What links here" and "Add a link". Enter follows a link,
//!   F2 changes its type or target, Delete removes it (after a question),
//!   and typing filters by type;
//! - "What links here": the notes linking to this one, from the open
//!   document and every library document with notes
//!   ([`Backlinks`], read once per open document);
//! - adding a link: the type (the ten spoken names, type to filter), then
//!   the target, a note of the open document or of a library document.

use std::path::PathBuf;

use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;
use textweaver_store::notes::doc_key;
use textweaver_store::{
    Backlink, Backlinks, DocKey, Library, Note, NotedDoc, Relation, RelationType,
};

use crate::app::{App, ListKind};
use crate::command::Effect;
use crate::notes::collapse;

/// Longest note text in a list row, in characters.
const LABEL_CHARS: usize = 40;

/// Relation state kept between lists.
#[derive(Debug, Default)]
pub(crate) struct RelationsState {
    /// The type filter typed so far in a relations list.
    pub(crate) filter: String,
    /// The library documents with notes, read for the open document (its
    /// key), other than the open document itself.
    library: Option<(DocKey, Vec<NotedDoc>)>,
}

/// A row of a note's links list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LinkRow {
    /// The note's relation at this index.
    Out(usize),
    /// "What links here".
    Incoming,
    /// "Add a link".
    Add,
}

/// A relations list shown, with what its rows stand for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RelationsList {
    /// The links of note `id` of the open document.
    Note {
        /// The note.
        id: String,
        /// The rows shown.
        rows: Vec<LinkRow>,
    },
    /// What links to note `id`.
    Backlinks {
        /// The note.
        id: String,
        /// The rows shown.
        rows: Vec<Backlink>,
    },
    /// Adding (or changing link `editing` of) note `id`: the type.
    Types {
        /// The note.
        id: String,
        /// The link being changed, if any.
        editing: Option<usize>,
        /// The types shown.
        shown: Vec<RelationType>,
    },
    /// Adding: the target note, in document `doc` (empty: the open one).
    Targets {
        /// The note.
        id: String,
        /// The type chosen.
        rel: RelationType,
        /// The link being changed, if any.
        editing: Option<usize>,
        /// The target document's path, empty for the open document.
        doc: String,
        /// The target notes' ids, one per row; a last row "A note in
        /// another document" follows when `other` is set.
        ids: Vec<String>,
        /// The last row chooses another document.
        other: bool,
    },
    /// Adding: the library document holding the target.
    Documents {
        /// The note.
        id: String,
        /// The type chosen.
        rel: RelationType,
        /// The link being changed, if any.
        editing: Option<usize>,
        /// The documents, one per row.
        docs: Vec<PathBuf>,
    },
}

impl RelationsList {
    /// True for the lists that filter by type as you type.
    pub(crate) fn filterable(&self) -> bool {
        matches!(
            self,
            RelationsList::Note { .. }
                | RelationsList::Backlinks { .. }
                | RelationsList::Types { .. }
        )
    }
}

/// The catalog id of a relation type's spoken name.
fn type_id(t: RelationType) -> &'static str {
    match t {
        RelationType::ConflictsWith => "relations-type-conflicts-with",
        RelationType::Supports => "relations-type-supports",
        RelationType::IsExampleOf => "relations-type-is-example-of",
        RelationType::Cites => "relations-type-cites",
        RelationType::Contradicts => "relations-type-contradicts",
        RelationType::Defines => "relations-type-defines",
        RelationType::Extends => "relations-type-extends",
        RelationType::SeeAlso => "relations-type-see-also",
        RelationType::Precedes => "relations-type-precedes",
        RelationType::Follows => "relations-type-follows",
    }
}

/// A stored relation type's spoken name in the interface language; a type
/// textweaver does not know reads as stored ("likes").
pub(crate) fn type_name(c: &Catalog, rel_type: &str) -> String {
    let r = Relation {
        rel_type: rel_type.to_owned(),
        ..Relation::default()
    };
    match r.relation_type() {
        Some(t) => c.tr(type_id(t)),
        None => r.spoken_type(),
    }
}

/// True when `rel_type` passes the type filter `query` (every word in its
/// spoken name, in the interface language or in English).
fn type_matches(c: &Catalog, rel_type: &str, query: &str) -> bool {
    let english = Relation {
        rel_type: rel_type.to_owned(),
        ..Relation::default()
    }
    .spoken_type();
    crate::lists::matches(&type_name(c, rel_type), query) || crate::lists::matches(&english, query)
}

/// A note as a short label: its text, else its passage.
fn note_label(c: &Catalog, n: &Note) -> String {
    let text = if n.note.trim().is_empty() {
        n.anchor.as_str()
    } else {
        n.note.as_str()
    };
    if text.trim().is_empty() {
        c.tr("relations-empty-note")
    } else {
        collapse(text, LABEL_CHARS)
    }
}

impl App {
    /// The open document's path as relations name it (empty when it has no
    /// file).
    fn relations_here(&self) -> String {
        self.session
            .as_ref()
            .and_then(|s| s.doc.meta.path.as_ref())
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// Reads the library's documents with notes once per open document.
    fn relations_library(&mut self) -> &[NotedDoc] {
        let key = self.session.as_ref().map(|s| s.key.clone());
        let fresh = self.relations.library.as_ref().map(|(k, _)| k) == key.as_ref();
        if !fresh {
            let here = doc_key(&self.relations_here());
            let docs = match (&self.paths, self.state_store()) {
                (Some(paths), Some(states)) => Library::load(&paths.library_file())
                    .unwrap_or_default()
                    .noted_documents(&states)
                    .into_iter()
                    .filter(|d| here.is_empty() || doc_key(&d.path.to_string_lossy()) != here)
                    .collect(),
                _ => Vec::new(),
            };
            self.relations.library = key.map(|k| (k, docs));
        }
        self.relations
            .library
            .as_ref()
            .map_or(&[], |(_, d)| d.as_slice())
    }

    /// The reverse index over the open document and the library.
    fn relations_index(&mut self) -> Backlinks {
        self.relations_library();
        let here = self.relations_here();
        let mut docs: Vec<(String, &[Note])> = Vec::new();
        if let Some(s) = self.session.as_ref() {
            docs.push((here, s.notes.as_slice()));
        }
        if let Some((_, lib)) = &self.relations.library {
            for d in lib {
                docs.push((d.path.to_string_lossy().into_owned(), d.notes.as_slice()));
            }
        }
        Backlinks::build(docs.iter().map(|(p, n)| (p.as_str(), *n)))
    }

    /// "Links: 2 out, 1 in" for each note of the open document, or `None`
    /// for a note without links (the notes list).
    pub(crate) fn relations_counts(&mut self) -> Vec<Option<String>> {
        let index = self.relations_index();
        let here = self.relations_here();
        let Some(s) = self.session.as_ref() else {
            return Vec::new();
        };
        s.notes
            .iter()
            .map(|n| {
                let (out, inn) = (n.relations.len(), index.backlinks(&here, &n.id).len());
                (out + inn > 0).then(|| {
                    self.cat()
                        .fmt("relations-count", &args!["out" => out, "in" => inn])
                })
            })
            .collect()
    }

    /// A note's label: the open document's note `id`, or a library
    /// document's (its label with ", in Title").
    fn relations_find(&self, doc: &str, id: &str) -> Option<String> {
        let here = self.relations_here();
        if doc.is_empty() || doc_key(doc) == doc_key(&here) {
            let n = self.session.as_ref()?.notes.iter().find(|n| n.id == id)?;
            return Some(note_label(self.cat(), n));
        }
        let (_, lib) = self.relations.library.as_ref()?;
        let d = lib
            .iter()
            .find(|d| doc_key(&d.path.to_string_lossy()) == doc_key(doc))?;
        let n = d.notes.iter().find(|n| n.id == id)?;
        Some(self.msg_args(
            "relations-target-in",
            &args!["note" => note_label(self.cat(), n), "doc" => d.title.as_str()],
        ))
    }

    /// The note `id` of the open document.
    fn relations_note(&self, id: &str) -> Option<&Note> {
        self.session.as_ref()?.notes.iter().find(|n| n.id == id)
    }

    /// Says that the note is gone and shows nothing.
    fn relations_gone(&mut self) -> Vec<Effect> {
        let msg = self.msg("relations-note-gone");
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    /// Space in the notes list: the links of note `i`.
    pub(crate) fn note_links(&mut self, i: usize) -> Vec<Effect> {
        let Some(id) = self
            .session
            .as_ref()
            .and_then(|s| s.notes.get(i))
            .map(|n| n.id.clone())
        else {
            return vec![Effect::Redraw];
        };
        self.relations.filter.clear();
        self.show_note_links(&id, true)
    }

    /// The `note_links` command: the links of the note at the cursor.
    pub(crate) fn note_links_here(&mut self) -> Vec<Effect> {
        let at = self.reading_position().and_then(|pos| {
            let notes = &self.session.as_ref()?.notes;
            notes
                .iter()
                .position(|n| n.range.contains(pos) || n.range.start == pos)
        });
        match at {
            Some(i) => self.note_links(i),
            None => {
                let key = self.key(textweaver_keymap::ActionId::AddNote);
                let msg = self.msg_args("relations-no-note-here", &args!["key" => key]);
                self.tell(&msg);
                vec![Effect::Redraw]
            }
        }
    }

    /// The links of note `id`: its relations (those passing the type
    /// filter), "What links here", and "Add a link". `intro` says the
    /// list's introduction.
    fn show_note_links(&mut self, id: &str, intro: bool) -> Vec<Effect> {
        let index = self.relations_index();
        let here = self.relations_here();
        let Some(note) = self.relations_note(id).cloned() else {
            return self.relations_gone();
        };
        let filter = self.relations.filter.clone();
        let incoming = index.backlinks(&here, id).len();
        let mut rows = Vec::new();
        let mut items = Vec::new();
        for (i, r) in note.relations.iter().enumerate() {
            if !type_matches(self.cat(), &r.rel_type, &filter) {
                continue;
            }
            let target = self
                .relations_find(&r.target_doc, &r.target_id)
                .unwrap_or_else(|| self.msg("relations-target-missing"));
            rows.push(LinkRow::Out(i));
            items.push(self.msg_args(
                "relations-out-item",
                &args!["type" => type_name(self.cat(), &r.rel_type), "target" => target],
            ));
        }
        rows.push(LinkRow::Incoming);
        items.push(self.msg_args("relations-incoming-row", &args!["n" => incoming]));
        rows.push(LinkRow::Add);
        items.push(self.msg("relations-add-row"));
        let label = note_label(self.cat(), &note);
        if intro {
            let msg = self.msg_args(
                "relations-note-intro",
                &args!["note" => label.as_str(), "out" => note.relations.len(), "in" => incoming],
            );
            self.tell(&msg);
        }
        let title = self.relations_title("relations-note-title", &label);
        self.list = Some(ListKind::Relations(RelationsList::Note {
            id: id.to_owned(),
            rows,
        }));
        vec![Effect::ShowList { title, items }]
    }

    /// A list title with the note, and the filter when there is one.
    fn relations_title(&self, id: &str, note: &str) -> String {
        let title = self.msg_args(id, &args!["note" => note]);
        if self.relations.filter.trim().is_empty() {
            title
        } else {
            self.msg_args(
                "relations-title-filtered",
                &args!["title" => title, "filter" => self.relations.filter.trim()],
            )
        }
    }

    /// "What links here" for note `id`.
    fn show_backlinks(&mut self, id: &str, intro: bool) -> Vec<Effect> {
        let index = self.relations_index();
        let here = self.relations_here();
        let Some(label) = self.relations_note(id).map(|n| note_label(self.cat(), n)) else {
            return self.relations_gone();
        };
        let all = index.backlinks(&here, id).to_vec();
        if all.is_empty() {
            let msg = self.msg("relations-none-in");
            self.tell(&msg);
            return self.show_note_links(id, false);
        }
        let filter = self.relations.filter.clone();
        let rows: Vec<Backlink> = all
            .into_iter()
            .filter(|b| type_matches(self.cat(), &b.rel_type, &filter))
            .collect();
        let items = rows
            .iter()
            .map(|b| {
                let from = self
                    .relations_find(&b.from_doc, &b.from_id)
                    .unwrap_or_else(|| self.msg("relations-target-missing"));
                self.msg_args(
                    "relations-backlink-item",
                    &args!["type" => type_name(self.cat(), &b.rel_type), "note" => from],
                )
            })
            .collect();
        if intro {
            let msg = self.msg_args(
                "relations-backlinks-intro",
                &args!["note" => label.as_str(), "n" => rows.len()],
            );
            self.tell(&msg);
        }
        let title = self.relations_title("relations-backlinks-title", &label);
        self.list = Some(ListKind::Relations(RelationsList::Backlinks {
            id: id.to_owned(),
            rows,
        }));
        vec![Effect::ShowList { title, items }]
    }

    /// The ten types, for adding a link or changing link `editing`.
    fn show_relation_types(
        &mut self,
        id: &str,
        editing: Option<usize>,
        intro: bool,
    ) -> Vec<Effect> {
        let Some(note) = self.relations_note(id).cloned() else {
            return self.relations_gone();
        };
        let filter = self.relations.filter.clone();
        let shown: Vec<RelationType> = RelationType::ALL
            .into_iter()
            .filter(|t| type_matches(self.cat(), t.as_str(), &filter))
            .collect();
        let items = shown.iter().map(|t| self.msg(type_id(*t))).collect();
        // Changing a link: the focus starts on its type.
        if intro
            && let Some(cur) = editing
                .and_then(|i| note.relations.get(i))
                .and_then(Relation::relation_type)
        {
            self.pending_list_focus = shown.iter().position(|t| *t == cur);
        }
        let label = note_label(self.cat(), &note);
        if intro {
            let msg = self.msg("relations-types-intro");
            self.tell(&msg);
        }
        let title = self.relations_title("relations-types-title", &label);
        self.list = Some(ListKind::Relations(RelationsList::Types {
            id: id.to_owned(),
            editing,
            shown,
        }));
        vec![Effect::ShowList { title, items }]
    }

    /// The notes a link can go to: those of document `doc` (empty: the
    /// open one, then "A note in another document").
    fn show_relation_targets(
        &mut self,
        id: &str,
        rel: RelationType,
        editing: Option<usize>,
        doc: &str,
    ) -> Vec<Effect> {
        self.relations_library();
        let here = self.relations_here();
        let same = doc.is_empty() || doc_key(doc) == doc_key(&here);
        let notes: Vec<Note> = if same {
            self.session
                .as_ref()
                .map(|s| s.notes.iter().filter(|n| n.id != id).cloned().collect())
                .unwrap_or_default()
        } else {
            self.relations
                .library
                .as_ref()
                .and_then(|(_, lib)| {
                    lib.iter()
                        .find(|d| doc_key(&d.path.to_string_lossy()) == doc_key(doc))
                })
                .map(|d| d.notes.clone())
                .unwrap_or_default()
        };
        let mut items: Vec<String> = notes.iter().map(|n| note_label(self.cat(), n)).collect();
        if same {
            items.push(self.msg("relations-other-document-row"));
        }
        let msg = self.msg_args("relations-targets-intro", &args!["n" => notes.len()]);
        self.tell(&msg);
        let title = self.msg_args(
            "relations-targets-title",
            &args!["type" => self.msg(type_id(rel))],
        );
        let doc = if same { String::new() } else { doc.to_owned() };
        self.list = Some(ListKind::Relations(RelationsList::Targets {
            id: id.to_owned(),
            rel,
            editing,
            doc,
            ids: notes.into_iter().map(|n| n.id).collect(),
            other: same,
        }));
        vec![Effect::ShowList { title, items }]
    }

    /// The library documents with notes, to choose a link's target from.
    fn show_relation_documents(
        &mut self,
        id: &str,
        rel: RelationType,
        editing: Option<usize>,
    ) -> Vec<Effect> {
        let docs: Vec<(PathBuf, String, usize)> = self
            .relations_library()
            .iter()
            .map(|d| (d.path.clone(), d.title.clone(), d.notes.len()))
            .collect();
        if docs.is_empty() {
            let msg = self.msg("relations-no-other-documents");
            self.tell(&msg);
            return self.show_relation_targets(id, rel, editing, "");
        }
        let items = docs
            .iter()
            .map(|(_, title, n)| {
                self.msg_args(
                    "relations-document-item",
                    &args!["title" => title.as_str(), "n" => *n],
                )
            })
            .collect();
        let msg = self.msg_args("relations-documents-intro", &args!["n" => docs.len()]);
        self.tell(&msg);
        self.list = Some(ListKind::Relations(RelationsList::Documents {
            id: id.to_owned(),
            rel,
            editing,
            docs: docs.into_iter().map(|(p, _, _)| p).collect(),
        }));
        vec![Effect::ShowList {
            title: self.msg("relations-documents-title"),
            items,
        }]
    }

    /// Enter in a relations list.
    pub(crate) fn choose_relation(&mut self, list: RelationsList, n: usize) -> Vec<Effect> {
        match list {
            RelationsList::Note { id, rows } => match rows.get(n) {
                Some(LinkRow::Out(i)) => {
                    let Some(r) = self.relations_note(&id).and_then(|x| x.relations.get(*i)) else {
                        return self.relations_gone();
                    };
                    let (doc, target) = (r.target_doc.clone(), r.target_id.clone());
                    self.follow_relation(&doc, &target)
                }
                Some(LinkRow::Incoming) => {
                    self.relations.filter.clear();
                    self.show_backlinks(&id, true)
                }
                Some(LinkRow::Add) => {
                    self.relations.filter.clear();
                    self.show_relation_types(&id, None, true)
                }
                None => vec![Effect::Redraw],
            },
            RelationsList::Backlinks { rows, .. } => match rows.get(n) {
                Some(b) => {
                    let (doc, from) = (b.from_doc.clone(), b.from_id.clone());
                    self.follow_relation(&doc, &from)
                }
                None => vec![Effect::Redraw],
            },
            RelationsList::Types { id, editing, shown } => match shown.get(n) {
                Some(&rel) => {
                    self.relations.filter.clear();
                    self.show_relation_targets(&id, rel, editing, "")
                }
                None => vec![Effect::Redraw],
            },
            RelationsList::Targets {
                id,
                rel,
                editing,
                doc,
                ids,
                other,
            } => match ids.get(n) {
                Some(target) => self.set_relation(&id, rel, editing, &doc, target),
                None if other => self.show_relation_documents(&id, rel, editing),
                None => vec![Effect::Redraw],
            },
            RelationsList::Documents {
                id,
                rel,
                editing,
                docs,
            } => match docs.get(n) {
                Some(p) => {
                    let doc = p.to_string_lossy().into_owned();
                    self.show_relation_targets(&id, rel, editing, &doc)
                }
                None => vec![Effect::Redraw],
            },
        }
    }

    /// Goes to note `id` of document `doc`: in the open document, a jump;
    /// in another, the document opens and the cursor goes to the note.
    fn follow_relation(&mut self, doc: &str, id: &str) -> Vec<Effect> {
        let index_of = |app: &App| {
            app.session
                .as_ref()
                .and_then(|s| s.notes.iter().position(|n| n.id == id))
        };
        let here = self.relations_here();
        if doc.is_empty() || doc_key(doc) == doc_key(&here) {
            return match index_of(self) {
                Some(i) => {
                    self.go_to_note(i, false);
                    vec![Effect::Redraw]
                }
                None => self.relations_gone(),
            };
        }
        let path = PathBuf::from(doc);
        if !path.exists() {
            let msg = self.msg_args(
                "relations-document-missing",
                &args!["file" => crate::authoring_state::file_name(&path)],
            );
            self.error(&msg);
            return vec![Effect::Redraw];
        }
        let effects = self.open_command(path);
        // shortcut: the jump happens when the document opened at once, as
        // following a file link does (crate::links); a document opening in
        // the background, or behind a question about unsaved edits, opens
        // at its reading position. Upgrade with an after-open hook.
        let opened = self
            .session
            .as_ref()
            .and_then(|s| s.doc.meta.path.as_deref())
            .is_some_and(|p| doc_key(&p.to_string_lossy()) == doc_key(doc));
        if opened && let Some(i) = index_of(self) {
            self.go_to_note(i, false);
        }
        effects
    }

    /// Adds a link from note `id` (or changes its link `editing`) to note
    /// `target` of document `doc` (empty: the open one), then shows the
    /// note's links again.
    fn set_relation(
        &mut self,
        id: &str,
        rel: RelationType,
        editing: Option<usize>,
        doc: &str,
        target: &str,
    ) -> Vec<Effect> {
        let here = self.relations_here();
        let target_doc = if doc.is_empty() || doc_key(doc) == doc_key(&here) {
            String::new()
        } else {
            doc.to_owned()
        };
        let label = self
            .relations_find(&target_doc, target)
            .unwrap_or_else(|| self.msg("relations-target-missing"));
        let type_said = self.msg(type_id(rel));
        let points_at = if target_doc.is_empty() {
            here.clone()
        } else {
            target_doc.clone()
        };
        let Some(note) = self
            .session
            .as_mut()
            .and_then(|s| s.notes.iter_mut().find(|n| n.id == id))
        else {
            return self.relations_gone();
        };
        let taken = note.relations.iter().enumerate().any(|(i, r)| {
            Some(i) != editing
                && r.relation_type() == Some(rel)
                && r.points_to(&here, &points_at, target)
        });
        if taken {
            let msg = self.msg_args(
                "relations-already",
                &args!["type" => type_said, "target" => label],
            );
            self.tell(&msg);
            return self.show_note_links(id, false);
        }
        let mut relation = Relation {
            rel_type: rel.as_str().to_owned(),
            target_doc,
            target_id: target.to_owned(),
            note: String::new(),
        };
        let at = match editing.filter(|&i| i < note.relations.len()) {
            Some(i) => {
                relation.note = std::mem::take(&mut note.relations[i].note);
                note.relations[i] = relation;
                i
            }
            None => {
                note.relations.push(relation);
                note.relations.len() - 1
            }
        };
        note.ts = textweaver_store::now_ts();
        self.persist_marks();
        let id_msg = if editing.is_some() {
            "relations-changed"
        } else {
            "relations-linked"
        };
        let msg = self.msg_args(id_msg, &args!["type" => type_said, "target" => label]);
        self.tell(&msg);
        self.relations.filter.clear();
        self.pending_list_focus = Some(at);
        self.show_note_links(id, false)
    }

    /// Delete in a note's links list: asks before removing the link.
    pub(crate) fn delete_relation_item(&mut self, list: RelationsList, n: usize) -> Vec<Effect> {
        if let RelationsList::Note { rows, .. } = &list
            && matches!(rows.get(n), Some(LinkRow::Out(_)))
        {
            let question = self.msg("relations-remove-question");
            self.list = None;
            self.pending_list_delete = Some((ListKind::Relations(list), n));
            self.ask(&question);
            return vec![Effect::Redraw];
        }
        let msg = self.msg("relations-nothing-to-remove");
        self.tell(&msg);
        self.list = Some(ListKind::Relations(list));
        vec![Effect::Redraw]
    }

    /// Removes the link in row `n` (after the question), then shows the
    /// note's links again.
    pub(crate) fn remove_relation(&mut self, list: &RelationsList, n: usize) -> Vec<Effect> {
        let RelationsList::Note { id, rows } = list else {
            return vec![Effect::Redraw];
        };
        let Some(LinkRow::Out(i)) = rows.get(n) else {
            return vec![Effect::Redraw];
        };
        let Some(r) = self
            .relations_note(id)
            .and_then(|x| x.relations.get(*i))
            .cloned()
        else {
            return self.relations_gone();
        };
        let label = self
            .relations_find(&r.target_doc, &r.target_id)
            .unwrap_or_else(|| self.msg("relations-target-missing"));
        let type_said = type_name(self.cat(), &r.rel_type);
        if let Some(note) = self
            .session
            .as_mut()
            .and_then(|s| s.notes.iter_mut().find(|x| &x.id == id))
        {
            note.relations.remove(*i);
            note.ts = textweaver_store::now_ts();
        }
        self.persist_marks();
        let msg = self.msg_args(
            "relations-removed",
            &args!["type" => type_said, "target" => label],
        );
        self.tell(&msg);
        self.show_note_links(id, false)
    }

    /// The note's links again, after No to the remove question.
    pub(crate) fn reshow_relations(&mut self, list: &RelationsList) -> Vec<Effect> {
        match list {
            RelationsList::Note { id, .. } => {
                let id = id.clone();
                self.show_note_links(&id, false)
            }
            _ => vec![Effect::Redraw],
        }
    }

    /// F2 in a note's links list: change the link's type, then its target.
    pub(crate) fn edit_relation_item(&mut self, list: RelationsList, n: usize) -> Vec<Effect> {
        if let RelationsList::Note { id, rows } = &list
            && let Some(LinkRow::Out(i)) = rows.get(n)
        {
            self.relations.filter.clear();
            let (id, i) = (id.clone(), *i);
            return self.show_relation_types(&id, Some(i), true);
        }
        let msg = self.msg("study-nothing-to-rename");
        self.tell(&msg);
        self.list = Some(ListKind::Relations(list));
        vec![Effect::Redraw]
    }

    /// The type filter changed to `query`: the list is shown again with the
    /// rows whose type holds every word.
    pub(crate) fn filter_relations(&mut self, query: String) -> Vec<Effect> {
        let Some(ListKind::Relations(list)) = self.list.clone() else {
            return vec![Effect::Redraw];
        };
        self.relations.filter = query.clone();
        let effects = match &list {
            RelationsList::Note { id, .. } => self.show_note_links(id, false),
            RelationsList::Backlinks { id, .. } => self.show_backlinks(id, false),
            RelationsList::Types { id, editing, .. } => {
                self.show_relation_types(id, *editing, false)
            }
            _ => return vec![Effect::Redraw],
        };
        let n = match &self.list {
            Some(ListKind::Relations(RelationsList::Note { rows, .. })) => {
                rows.iter().filter(|r| matches!(r, LinkRow::Out(_))).count()
            }
            Some(ListKind::Relations(RelationsList::Backlinks { rows, .. })) => rows.len(),
            Some(ListKind::Relations(RelationsList::Types { shown, .. })) => shown.len(),
            _ => 0,
        };
        let msg = if query.trim().is_empty() {
            self.msg_args("relations-filter-cleared", &args!["n" => n])
        } else if n == 0 {
            self.msg_args("relations-filter-none", &args!["filter" => query.trim()])
        } else {
            self.msg_args(
                "relations-filter-matched",
                &args!["n" => n, "filter" => query.trim()],
            )
        };
        self.tell(&msg);
        effects
    }
}
