//! Edit a document's details by hand (Wave 7, W7m): its title, author,
//! DOI, and ISBN, from the open document (`edit_document_details`, in File
//! and the palette) or from the library list (F2 on a document).
//!
//! The form is the shared prompt model ([`crate::list_model`]), one field
//! at a time, so every frontend gets it: each field is a labeled line
//! ("Author, 2 of 4") holding its current value, and moving to it says the
//! label and the value ("Author: Ada Example", or "blank"). Tab and
//! Shift+Tab move between the fields, keeping what was typed; Enter saves
//! every field; Escape cancels and says so. A DOI or an ISBN that is not
//! one is said, and the form opens again on that field.
//!
//! Only the fields the owner changed are saved: on the bookshelf
//! (`library.json`, [`Library::record_edits`]) on the background writer,
//! and with sync on, as the record's hand-edited details
//! ([`crate::sync_engine::SyncRequest::EditDetails`]), newest wins per
//! field. The hand-edited values win over the document's own in the
//! library list, its filter, and `tw library search`. Clearing a field
//! removes the hand edit, so the document's own value shows again.
//!
//! [`Library::record_edits`]: textweaver_store::Library::record_edits

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use textweaver_a11y::Priority;
use textweaver_keymap::ActionId;
use textweaver_lexicon::args;
use textweaver_store::library::{DetailField, DocMetadata, LibraryItem};
use textweaver_store::{Library, Paths, Settings};

use crate::app::{App, ListKind};
use crate::command::{Effect, PromptPurpose};
use crate::library::LibraryList;
use crate::synced_library::SyncedLibrary;

/// The edit form, while it is open.
#[derive(Clone, Debug)]
pub(crate) struct DetailsForm {
    /// The document.
    path: PathBuf,
    /// What each field holds now, in [`DetailField::ALL`]'s order.
    values: [String; 4],
    /// What each field held when the form opened.
    original: [String; 4],
    /// The field being edited.
    focus: usize,
    /// The document is the one open in the reader.
    open_document: bool,
    /// The library list the form was opened from, and its focused row, to
    /// show again when the form closes.
    back: Option<(LibraryList, usize)>,
}

/// The open document's details, read on a helper thread (the sync folder
/// can be slow).
pub(crate) struct DetailsLoad {
    rx: Receiver<LibraryItem>,
}

impl std::fmt::Debug for DetailsLoad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DetailsLoad").finish_non_exhaustive()
    }
}

/// The message id of a field's label.
fn label_id(field: DetailField) -> &'static str {
    match field {
        DetailField::Title => "details-label-title",
        DetailField::Author => "details-label-author",
        DetailField::Doi => "details-label-doi",
        DetailField::Isbn => "details-label-isbn",
    }
}

/// What a field shows for `item`.
fn item_value(item: &LibraryItem, field: DetailField) -> String {
    match field {
        DetailField::Title => item.title.clone(),
        DetailField::Author => item.meta.author.clone().unwrap_or_default(),
        DetailField::Doi => item.meta.doi.clone().unwrap_or_default(),
        DetailField::Isbn => item.meta.isbn.clone().unwrap_or_default(),
    }
}

/// The open document's details as the library shows them: the bookshelf's
/// entry (with the owner's edits), else what the document says, then what
/// other computers know (with sync on). Reads files: run it off the input
/// thread.
fn load_item(
    path: &Path,
    title: String,
    meta: DocMetadata,
    paths: &Paths,
    settings: &Settings,
) -> LibraryItem {
    let lib = Library::load(&paths.library_file()).unwrap_or_default();
    let entry = lib.get(path);
    let mut item = LibraryItem {
        path: path.to_owned(),
        title: entry
            .map(|e| e.shown_title().to_owned())
            .filter(|t| !t.is_empty())
            .unwrap_or(title),
        folder: None,
        rel: None,
        pct: None,
        last_opened: None,
        source: textweaver_store::library::ItemSource::Recent,
        meta: entry.map_or(meta, textweaver_store::LibraryEntry::shown_meta),
        edited: entry.map(|e| e.edited.clone()).unwrap_or_default(),
    };
    if let Some(s) = SyncedLibrary::load(paths, settings) {
        s.enrich(std::slice::from_mut(&mut item), &|_| None);
    }
    item
}

impl App {
    /// Edit details (`edit_document_details`): the open document's title,
    /// author, DOI, and ISBN. Its details are read on a helper thread; the
    /// form opens when they are in ([`App::details_tick`]).
    pub(crate) fn edit_document_details(&mut self) -> Vec<Effect> {
        let Some(s) = self.session.as_ref() else {
            let key = self.key(ActionId::Open);
            let msg = self.msg_args("app-no-document-open", &args!["key" => key]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        let (Some(path), Some(paths)) = (s.doc.meta.path.clone(), self.paths.clone()) else {
            let msg = self.msg("details-no-file");
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        if self.details_load.is_some() {
            return vec![Effect::Redraw];
        }
        let title = crate::library::stated_title(&s.doc).unwrap_or_else(|| s.title.clone());
        let meta = crate::library::document_metadata(&s.doc);
        let settings = self.settings.clone();
        let (tx, rx) = mpsc::channel();
        let wake = self.waker_slot();
        // The bookshelf entry queued when the document opened is written
        // first; the helper waits, not the keys.
        let written = self.writer.barrier();
        let spawned = std::thread::Builder::new()
            .name("textweaver-details".into())
            .spawn(move || {
                if let Some(w) = written {
                    let _ = w.recv_timeout(std::time::Duration::from_secs(5));
                }
                let _ = tx.send(load_item(&path, title, meta, &paths, &settings));
                wake.wake();
            });
        if let Err(e) = spawned {
            let msg = self.msg_args("details-save-failed", &args!["error" => e.to_string()]);
            self.error(&msg);
            return vec![Effect::Redraw];
        }
        self.details_load = Some(DetailsLoad { rx });
        vec![Effect::Redraw]
    }

    /// Opens the form once the open document's details are read (from
    /// [`App::tick`]).
    pub(crate) fn details_tick(&mut self) -> Vec<Effect> {
        let Some(load) = &self.details_load else {
            return Vec::new();
        };
        let item = match load.rx.try_recv() {
            Ok(item) => item,
            Err(TryRecvError::Empty) => return Vec::new(),
            Err(TryRecvError::Disconnected) => {
                self.details_load = None;
                return Vec::new();
            }
        };
        self.details_load = None;
        // The document may have closed, or a prompt or list opened, while
        // the details were read: then the form waits for the next ask.
        let still_open = self
            .session
            .as_ref()
            .and_then(|s| s.doc.meta.path.as_deref())
            == Some(item.path.as_path());
        if !still_open || self.mode.is_prompt() || self.list.is_some() {
            return Vec::new();
        }
        self.open_details_form(&item, true, None)
    }

    /// F2 on row `n` of the library list: edits that document's details,
    /// then shows the list again.
    pub(crate) fn edit_library_details(&mut self, list: LibraryList, n: usize) -> Vec<Effect> {
        let Some(item) = list.item_at(n).cloned() else {
            self.list = Some(ListKind::Library(list));
            return vec![Effect::Redraw];
        };
        let open = self
            .session
            .as_ref()
            .and_then(|s| s.doc.meta.path.as_deref())
            .is_some_and(|p| {
                textweaver_store::library::resolve_path(p)
                    == textweaver_store::library::resolve_path(&item.path)
            });
        self.open_details_form(&item, open, Some((list, n)))
    }

    fn open_details_form(
        &mut self,
        item: &LibraryItem,
        open_document: bool,
        back: Option<(LibraryList, usize)>,
    ) -> Vec<Effect> {
        let values = DetailField::ALL.map(|f| item_value(item, f));
        let name = if item.title.trim().is_empty() {
            item.path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        } else {
            item.title.clone()
        };
        self.details_form = Some(DetailsForm {
            path: item.path.clone(),
            original: values.clone(),
            values,
            focus: 0,
            open_document,
            back,
        });
        let intro = self.msg_args("details-intro", &args!["name" => name]);
        self.tell(&intro);
        self.details_field_prompt(false)
    }

    /// The prompt for the focused field, its label and value said: queued
    /// after the form's introduction when `interrupt` is false, said at
    /// once when moving between fields.
    fn details_field_prompt(&mut self, interrupt: bool) -> Vec<Effect> {
        let Some(form) = &self.details_form else {
            return vec![Effect::Redraw];
        };
        let field = DetailField::ALL[form.focus];
        let value = form.values[form.focus].clone();
        let n = form.focus + 1;
        let label_text = self.msg(label_id(field));
        let label = self.msg_args(
            "details-prompt-label",
            &args!["label" => label_text.as_str(), "n" => n, "total" => DetailField::ALL.len()],
        );
        let shown = if value.trim().is_empty() {
            self.msg("nav-blank")
        } else {
            value.clone()
        };
        let said = self.msg_args(
            "details-field",
            &args!["label" => label_text, "value" => shown],
        );
        if interrupt {
            self.announce(&said, Priority::Assertive);
        } else {
            self.announce_queued(&said, Priority::Polite);
        }
        if !self.mode.is_prompt() {
            self.return_mode = self.mode;
        }
        self.mode = crate::app::Mode::Prompt;
        self.prompt_purpose = PromptPurpose::DocumentDetails;
        self.pending_prompt_text = Some(value);
        vec![Effect::Prompt {
            label,
            purpose: PromptPurpose::DocumentDetails,
        }]
    }

    /// Tab (`step` 1) or Shift+Tab (`step` -1) in the form: keeps what the
    /// field holds and moves to the next or previous field, wrapping.
    pub(crate) fn details_move(&mut self, text: String, step: isize) -> Vec<Effect> {
        let Some(form) = self.details_form.as_mut() else {
            return vec![Effect::Redraw];
        };
        form.values[form.focus] = text;
        let n = DetailField::ALL.len() as isize;
        form.focus = (form.focus as isize + step).rem_euclid(n) as usize;
        self.details_field_prompt(true)
    }

    /// Enter in the form: saves the fields that changed.
    pub(crate) fn answer_details(&mut self, text: &str) -> Vec<Effect> {
        let Some(mut form) = self.details_form.take() else {
            return vec![Effect::Redraw];
        };
        form.values[form.focus] = text.to_owned();
        let mut edits = Vec::new();
        for (i, field) in DetailField::ALL.into_iter().enumerate() {
            let value = match field.clean(&form.values[i]) {
                Ok(v) => v,
                Err(bad) => {
                    let id = if field == DetailField::Doi {
                        "details-not-a-doi"
                    } else {
                        "details-not-an-isbn"
                    };
                    let msg = self.msg_args(id, &args!["text" => bad]);
                    self.error(&msg);
                    form.focus = i;
                    self.details_form = Some(form);
                    return self.details_field_prompt(false);
                }
            };
            let before = field.clean(&form.original[i]).ok().flatten();
            if value != before {
                edits.push((field, value));
            }
        }
        if edits.is_empty() {
            let msg = self.msg("details-unchanged");
            self.tell(&msg);
            return self.details_closed(form, &[]);
        }
        self.save_details(&form, &edits);
        let names: Vec<String> = edits.iter().map(|(f, _)| self.msg(label_id(*f))).collect();
        let msg = self.msg_args("details-saved", &args!["fields" => names.join(", ")]);
        self.tell(&msg);
        self.details_closed(form, &edits)
    }

    /// Escape in the form: nothing is saved, and the owner hears so.
    pub(crate) fn cancel_details(&mut self) -> Vec<Effect> {
        let Some(form) = self.details_form.take() else {
            return vec![Effect::Redraw];
        };
        let msg = self.msg("details-cancelled");
        self.tell(&msg);
        self.details_closed(form, &[])
    }

    /// The form closed: the library list it came from is shown again, with
    /// the row updated.
    fn details_closed(
        &mut self,
        form: DetailsForm,
        edits: &[(DetailField, Option<String>)],
    ) -> Vec<Effect> {
        self.leave_prompt();
        if form.open_document
            && let Some((_, Some(title))) = edits.iter().find(|(f, _)| *f == DetailField::Title)
            && let Some(s) = self.session.as_mut()
        {
            s.title.clone_from(title);
        }
        let Some((mut list, n)) = form.back else {
            return vec![Effect::Redraw];
        };
        list.apply_edits(n, edits);
        self.pending_list_focus = Some(n);
        self.reshow_library(list)
    }

    /// Queues the edits: the bookshelf on the writer, then the sync folder.
    fn save_details(&mut self, form: &DetailsForm, edits: &[(DetailField, Option<String>)]) {
        let Some(paths) = &self.paths else {
            return;
        };
        self.writer.send(crate::writer::Job::EditDetails {
            library_file: paths.library_file(),
            path: form.path.clone(),
            edits: edits.to_vec(),
            at_ms: textweaver_sync::wall_ms(),
        });
        if !self.sync_enabled() {
            return;
        }
        let details = if form.open_document {
            self.session
                .as_ref()
                .map(|s| crate::library::sync_details(&s.doc))
                .unwrap_or_default()
        } else {
            textweaver_sync::docid::Details::default()
        };
        let identify = textweaver_sync::Identify {
            ids_file: paths.sync_ids_file(),
            path: form.path.clone(),
            library_folders: self.settings.library.folders.clone(),
            details,
        };
        let edits = edits
            .iter()
            .map(|(f, v)| textweaver_sync::DetailEdit {
                name: f.name(),
                value: v.clone(),
            })
            .collect();
        self.sync_request_edit(identify, edits);
    }
}
