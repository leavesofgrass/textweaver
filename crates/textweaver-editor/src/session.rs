//! The authoring session: read mode and edit mode for one document, the
//! Save / Discard / Cancel decisions, the save rule, Save As adoption,
//! stale-load protection, and autosave snapshots. It is Star's GUI edit
//! mode (docs/star-parity.md Part 3 §4.1, §4.2, §5) without a GUI, so the
//! app can drive it from any frontend and the tests can exercise it.
//!
//! Decisions are two-phase so a frontend can ask asynchronously: a call
//! that needs an answer returns [`LeaveOutcome::NeedsChoice`] or
//! [`LeaveOutcome::NeedsPath`], and the frontend calls again with the
//! user's [`Choice`] or path.
//!
//! Differences from Star: New Document does not change a setting (Star
//! turned the preview pane on permanently, §7 item 34); leaving edit mode
//! cleanly deletes the snapshot, and a quit with unsaved edits is reported
//! by [`EditSession::needs_save_prompt`] so the app can ask (Star closed
//! without asking, item 38).

use std::path::{Path, PathBuf};
use std::time::Instant;

use textweaver_core::{CoreError, EditOutcome};

use crate::autosave::{
    self, AutosavePolicy, RecoverySnapshot, SaveTarget, save_as_path, save_target,
};
use crate::find::{self, FindOptions};
use crate::markdown::{self, FormatError, MarkdownOp};
use crate::{Editor, Selection};

/// The document being read or edited.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocInfo {
    /// The per-document key (`DocKey` in the store), used for the snapshot.
    pub key: String,
    /// The file, or `None` for a new document.
    pub path: Option<PathBuf>,
    /// The loader that produced it (`markdown`, `text`, `html`, ...).
    pub loader_id: String,
    /// The title.
    pub title: String,
}

impl DocInfo {
    /// A new, unsaved Markdown document.
    pub fn untitled(key: impl Into<String>) -> Self {
        DocInfo {
            key: key.into(),
            path: None,
            loader_id: "markdown".to_owned(),
            title: "Untitled".to_owned(),
        }
    }
}

/// The answer to "Save changes before leaving edit mode?".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Save, then continue.
    Save,
    /// Throw the changes away, then continue.
    Discard,
    /// Stay in edit mode.
    Cancel,
}

/// What an attempt to leave edit mode (finish, open another document,
/// start a new one) did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeaveOutcome {
    /// Left edit mode, or was not in it. `rebuild` is true when a save
    /// happened during the session, so the reading view (word maps,
    /// markers) must be rebuilt from the saved text; false when nothing
    /// changed.
    Left {
        /// Rebuild the reading view.
        rebuild: bool,
    },
    /// The user cancelled, or the save failed; still editing.
    Stayed,
    /// There are unsaved changes: ask Save / Discard / Cancel and call
    /// again with the answer.
    NeedsChoice,
    /// Saving needs a file name: ask (suggesting `suggested`) and call
    /// again with `Choice::Save` and the path.
    NeedsPath {
        /// The suggested path.
        suggested: PathBuf,
    },
}

/// What [`EditSession::save`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveOutcome {
    /// Written. `adopted` is true when this was a Save As and the session
    /// now refers to the new file, so the next save writes in place.
    Saved {
        /// The file written.
        path: PathBuf,
        /// The document adopted the path.
        adopted: bool,
    },
    /// A file name is needed: ask and call [`EditSession::save`] with it.
    NeedsPath {
        /// The suggested path.
        suggested: PathBuf,
    },
}

/// Why a session operation failed.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// The operation needs edit mode.
    #[error("Turn on edit mode to {action}")]
    NotEditing {
        /// What the user tried ("format text", "replace text", "undo").
        action: &'static str,
    },
    /// A formatting command could not run.
    #[error("{0}")]
    Format(#[from] FormatError),
    /// An edit was out of range (a bug in the caller).
    #[error("{0}")]
    Edit(#[from] CoreError),
    /// A file could not be written.
    #[error("{path}: {source}")]
    Io {
        /// The file.
        path: PathBuf,
        /// The error.
        #[source]
        source: std::io::Error,
    },
}

/// One document in read or edit mode.
#[derive(Clone, Debug)]
pub struct EditSession {
    doc: DocInfo,
    /// The document's text as last loaded or saved (Star's `doc.markdown`).
    doc_text: String,
    editor: Option<Editor>,
    maps_stale: bool,
    generation: u64,
    autosave: AutosavePolicy,
    recovery_dir: Option<PathBuf>,
    last_snapshot: Option<Instant>,
    /// Held while this session writes snapshots, so another instance does
    /// not offer or overwrite them (shared by clones).
    snapshot_lock: Option<std::sync::Arc<autosave::SnapshotLock>>,
}

impl EditSession {
    /// A session reading `doc`, whose editable text is `text`.
    pub fn new(doc: DocInfo, text: impl Into<String>) -> Self {
        EditSession {
            doc,
            doc_text: text.into(),
            editor: None,
            maps_stale: false,
            generation: 0,
            autosave: AutosavePolicy {
                enabled: false,
                ..AutosavePolicy::default()
            },
            recovery_dir: None,
            last_snapshot: None,
            snapshot_lock: None,
        }
    }

    /// Enables autosave snapshots into `dir` under `policy`.
    pub fn with_autosave(mut self, policy: AutosavePolicy, dir: impl Into<PathBuf>) -> Self {
        self.autosave = policy;
        self.recovery_dir = Some(dir.into());
        self
    }

    /// The document.
    pub fn doc(&self) -> &DocInfo {
        &self.doc
    }

    /// The document, for updating its key or title.
    pub fn doc_mut(&mut self) -> &mut DocInfo {
        &mut self.doc
    }

    /// The document's text as last loaded or saved.
    pub fn document_text(&self) -> &str {
        &self.doc_text
    }

    /// The text being edited, or the document's text in read mode (Star's
    /// `_qt_live_markdown`).
    pub fn live_text(&self) -> String {
        match &self.editor {
            Some(ed) => ed.text().to_string(),
            None => self.doc_text.clone(),
        }
    }

    /// True in edit mode.
    pub fn is_editing(&self) -> bool {
        self.editor.is_some()
    }

    /// True when editing with unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.editor.as_ref().is_some_and(Editor::is_dirty)
    }

    /// True when quitting now would lose edits; the app should ask Save /
    /// Discard / Cancel and call [`finish_editing`](Self::finish_editing).
    pub fn needs_save_prompt(&self) -> bool {
        self.is_dirty()
    }

    /// The editor, in edit mode.
    pub fn editor(&self) -> Option<&Editor> {
        self.editor.as_ref()
    }

    /// The editor, in edit mode, for typing and moving.
    pub fn editor_mut(&mut self) -> Option<&mut Editor> {
        self.editor.as_mut()
    }

    /// True when a save changed the text since the reading view was built.
    pub fn maps_stale(&self) -> bool {
        self.maps_stale
    }

    /// The load generation: bumped by every load and new document, so a
    /// slow background load that started earlier can be dropped.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The snapshot file for this document, when autosave is configured.
    pub fn snapshot_path(&self) -> Option<PathBuf> {
        self.recovery_dir
            .as_ref()
            .map(|d| autosave::snapshot_file(d, &self.doc.key))
    }

    fn clear_snapshot(&mut self) {
        if let Some(p) = self.snapshot_path() {
            let _ = autosave::delete_snapshot(&p);
        }
        self.last_snapshot = None;
        self.snapshot_lock = None;
    }

    /// Enters edit mode on the document's text (Ctrl+E). Undo history
    /// starts empty and the editor is clean. Does nothing when already
    /// editing.
    pub fn enter_edit(&mut self) {
        if self.editor.is_none() {
            self.editor = Some(Editor::new(&self.doc_text));
            self.last_snapshot = None;
        }
    }

    fn teardown(&mut self) -> LeaveOutcome {
        let was_editing = self.editor.take().is_some();
        if was_editing {
            self.clear_snapshot();
        }
        let rebuild = self.maps_stale;
        self.maps_stale = false;
        LeaveOutcome::Left { rebuild }
    }

    /// Saves the text being edited and stays in edit mode (Ctrl+S).
    ///
    /// Markdown and plain-text sources are written in place, keeping their
    /// BOM and line endings. Anything else (a converted format, a new
    /// document) needs a path: without `save_as` this returns
    /// [`SaveOutcome::NeedsPath`]; with it, the file is written (a
    /// converted-format extension becomes `.md`) and the session adopts it,
    /// so the next save writes in place without asking.
    pub fn save(&mut self, save_as: Option<&Path>) -> Result<SaveOutcome, SessionError> {
        let Some(editor) = &self.editor else {
            return Err(SessionError::NotEditing { action: "save" });
        };
        let text = editor.text().to_string();
        let target = match &self.doc.path {
            Some(p) => save_target(p, &self.doc.loader_id),
            None => SaveTarget::SaveAsMarkdown {
                suggested: PathBuf::from("document.md"),
            },
        };
        let (dest, adopted) = match (target, save_as) {
            (SaveTarget::InPlace(p), None) => (p, false),
            (SaveTarget::InPlace(_), Some(chosen))
            | (SaveTarget::SaveAsMarkdown { .. }, Some(chosen)) => (save_as_path(chosen), true),
            (SaveTarget::SaveAsMarkdown { suggested }, None) => {
                return Ok(SaveOutcome::NeedsPath { suggested });
            }
        };
        autosave::save_text(&dest, &text).map_err(|source| SessionError::Io {
            path: dest.clone(),
            source,
        })?;
        self.doc_text = text;
        if let Some(ed) = &mut self.editor {
            ed.mark_saved();
        }
        self.maps_stale = true;
        self.clear_snapshot();
        if adopted {
            let is_text = dest
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("txt") || e.eq_ignore_ascii_case("text"));
            self.doc.loader_id = if is_text { "text" } else { "markdown" }.to_owned();
            self.doc.title = dest
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| self.doc.title.clone());
            self.doc.path = Some(dest.clone());
        }
        Ok(SaveOutcome::Saved {
            path: dest,
            adopted,
        })
    }

    /// Leaves edit mode (Ctrl+E while editing, Star's
    /// `_qt_finish_editing`). Clean: leaves at once. Dirty: needs a
    /// [`Choice`]; Cancel stays, Discard leaves without saving, Save saves
    /// (asking for a path through [`LeaveOutcome::NeedsPath`] if needed)
    /// and leaves only if the save succeeded. Not editing: `Left`.
    pub fn finish_editing(
        &mut self,
        choice: Option<Choice>,
        save_as: Option<&Path>,
    ) -> Result<LeaveOutcome, SessionError> {
        if !self.is_editing() {
            return Ok(self.teardown());
        }
        if !self.is_dirty() {
            return Ok(self.teardown());
        }
        match choice {
            None => Ok(LeaveOutcome::NeedsChoice),
            Some(Choice::Cancel) => Ok(LeaveOutcome::Stayed),
            Some(Choice::Discard) => Ok(self.teardown()),
            Some(Choice::Save) => match self.save(save_as) {
                Ok(SaveOutcome::Saved { .. }) => Ok(self.teardown()),
                Ok(SaveOutcome::NeedsPath { suggested }) => {
                    Ok(LeaveOutcome::NeedsPath { suggested })
                }
                // A failed save leaves the user in edit mode with the error.
                Err(e) => Err(e),
            },
        }
    }

    /// Before opening another document or starting a new one (Star's
    /// `_qt_confirm_leave_edit_for_replace`): the same decisions as
    /// [`finish_editing`](Self::finish_editing). Proceed only on `Left`.
    pub fn confirm_leave(
        &mut self,
        choice: Option<Choice>,
        save_as: Option<&Path>,
    ) -> Result<LeaveOutcome, SessionError> {
        self.finish_editing(choice, save_as)
    }

    /// Starts a background load: returns the ticket to pass to
    /// [`finish_load`](Self::finish_load).
    pub fn begin_load(&mut self) -> u64 {
        self.generation += 1;
        self.generation
    }

    /// Applies a loaded document if `ticket` is still current; a load that
    /// started before a newer load or a new document is dropped. Returns
    /// whether it was applied.
    pub fn finish_load(&mut self, ticket: u64, doc: DocInfo, text: impl Into<String>) -> bool {
        if ticket != self.generation {
            return false;
        }
        self.apply_load(doc, text.into());
        true
    }

    /// Replaces the document at once. As a safety net this tears edit mode
    /// down without saving (callers ask [`confirm_leave`](Self::confirm_leave)
    /// first) and clears the stale-maps flag.
    pub fn load(&mut self, doc: DocInfo, text: impl Into<String>) {
        self.generation += 1;
        self.apply_load(doc, text.into());
    }

    fn apply_load(&mut self, doc: DocInfo, text: String) {
        if self.editor.is_some() {
            self.clear_snapshot();
        }
        self.editor = None;
        self.maps_stale = false;
        self.doc = doc;
        self.doc_text = text;
        self.last_snapshot = None;
    }

    /// File > New (Ctrl+N): after the leave decision, a blank Markdown
    /// document with key `key`, in edit mode. Returns the leave outcome;
    /// the new document exists only when it is `Left`.
    pub fn new_document(
        &mut self,
        key: impl Into<String>,
        choice: Option<Choice>,
        save_as: Option<&Path>,
    ) -> Result<LeaveOutcome, SessionError> {
        let outcome = self.confirm_leave(choice, save_as)?;
        if let LeaveOutcome::Left { .. } = outcome {
            self.load(DocInfo::untitled(key), String::new());
            self.enter_edit();
        }
        Ok(outcome)
    }

    fn editing(&mut self, action: &'static str) -> Result<&mut Editor, SessionError> {
        self.editor
            .as_mut()
            .ok_or(SessionError::NotEditing { action })
    }

    /// Applies a formatting command to the selection as one undo step.
    /// Outside edit mode nothing changes and the error says "Turn on edit
    /// mode to format text".
    pub fn format(&mut self, op: MarkdownOp) -> Result<Vec<EditOutcome>, SessionError> {
        let ed = self.editing("format text")?;
        let f = markdown::try_apply(ed.text(), ed.selection(), op)?;
        Ok(ed.apply_formatted(&f)?)
    }

    /// Inserts an image reference relative to the document (Star's Insert
    /// Image), selecting the alt text.
    pub fn insert_image(&mut self, image: &Path) -> Result<Vec<EditOutcome>, SessionError> {
        let doc_path = self.doc.path.clone();
        let ed = self.editing("insert an image")?;
        let f = markdown::insert_image(ed.text(), ed.selection(), doc_path.as_deref(), image);
        Ok(ed.apply_formatted(&f)?)
    }

    /// Replaces the match at the caret (see [`find::replace_one`]).
    pub fn replace_one(
        &mut self,
        query: &str,
        replacement: &str,
        opts: FindOptions,
    ) -> Result<bool, SessionError> {
        let ed = self.editing("replace text")?;
        Ok(find::replace_one(ed, query, replacement, opts)?.is_some())
    }

    /// Replaces every match as one undo step; returns the count.
    pub fn replace_all(
        &mut self,
        query: &str,
        replacement: &str,
        opts: FindOptions,
    ) -> Result<usize, SessionError> {
        let ed = self.editing("replace text")?;
        Ok(find::replace_all(ed, query, replacement, opts)?)
    }

    /// Undoes the last step; `Ok(false)` when there is nothing to undo.
    pub fn undo(&mut self) -> Result<bool, SessionError> {
        Ok(self.editing("undo")?.undo().is_some())
    }

    /// Redoes the last undone step; `Ok(false)` when there is nothing.
    pub fn redo(&mut self) -> Result<bool, SessionError> {
        Ok(self.editing("redo")?.redo().is_some())
    }

    /// Selects `sel` in the editor (clamped).
    pub fn select(&mut self, sel: Selection) -> Result<(), SessionError> {
        self.editing("select text")?.set_selection(sel);
        Ok(())
    }

    /// Writes the live text (unsaved edits included) to `path` as Markdown,
    /// without changing the document (Star's Export as Markdown).
    pub fn export(&self, path: &Path) -> Result<(), SessionError> {
        autosave::write_atomic(path, self.live_text().as_bytes()).map_err(|source| {
            SessionError::Io {
                path: path.to_owned(),
                source,
            }
        })
    }

    /// The autosave timer fired: writes a snapshot when autosave is on, the
    /// session is editing with unsaved changes, and the interval has passed.
    /// Returns whether a snapshot was written.
    pub fn autosave_tick(&mut self, now: Instant) -> std::io::Result<bool> {
        let Some(dir) = self.recovery_dir.clone() else {
            return Ok(false);
        };
        let since = self.last_snapshot.map(|t| now.saturating_duration_since(t));
        if !self.autosave.due(self.is_dirty(), since) {
            return Ok(false);
        }
        if self.snapshot_lock.is_none() {
            match autosave::SnapshotLock::acquire(&dir, &self.doc.key)? {
                Some(lock) => self.snapshot_lock = Some(std::sync::Arc::new(lock)),
                // Another instance is editing this document and owns its
                // snapshot; do not overwrite it.
                None => return Ok(false),
            }
        }
        let snap = RecoverySnapshot {
            doc_key: self.doc.key.clone(),
            path: self.doc.path.clone(),
            text: self.live_text(),
            ts: autosave::now_ts(),
            title: Some(self.doc.title.clone()),
        };
        autosave::write_snapshot(&dir, &snap)?;
        self.last_snapshot = Some(now);
        Ok(true)
    }

    /// A session editing recovered text (Star's `_autosave_recover`): the
    /// snapshot's path and title (or "Recovered document"), in edit mode,
    /// marked unsaved.
    pub fn from_recovery(snapshot: &RecoverySnapshot) -> Self {
        let doc = DocInfo {
            key: snapshot.doc_key.clone(),
            path: snapshot.path.clone(),
            loader_id: "markdown".to_owned(),
            title: snapshot.display_title(),
        };
        let mut s = EditSession::new(doc, snapshot.text.clone());
        s.enter_edit();
        if let Some(ed) = &mut s.editor {
            ed.mark_modified();
        }
        s
    }
}

/// Answers a startup recovery offer for the snapshot in `file` (Star's
/// Yes / No prompt). Either way the snapshot file is deleted. On yes,
/// returns a session editing the recovered text, marked unsaved; announce
/// "Recovered unsaved work. Remember to save.".
pub fn resolve_recovery(
    file: &Path,
    snapshot: &RecoverySnapshot,
    accept: bool,
) -> std::io::Result<Option<EditSession>> {
    autosave::delete_snapshot(file)?;
    Ok(accept.then(|| EditSession::from_recovery(snapshot)))
}
