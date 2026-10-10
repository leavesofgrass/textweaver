//! Edit mode: typing with echo, Markdown formatting, undo and redo, Save,
//! Save As, New, the Save / Discard / Cancel decisions, autosave, and
//! startup recovery, on Agent C's [`EditSession`].
//!
//! # The text being edited
//!
//! Edit mode shows and edits the document's **source**: a plain-text file's
//! own text (which is its canonical text, ADR-0002), a Markdown file's
//! Markdown, and for any other format the converted Markdown (star's rule;
//! saving it asks for a new `.md` name). While editing, the session's
//! document *is* that text, so the viewport, highlights, reading, and
//! navigation all work on what is on screen.
//!
//! Every edit, whatever made it (a keystroke, a formatting command, undo,
//! Replace All), is applied to the document with `Document::apply`, and the
//! same `EditOutcome`s move the cursor, bookmarks, notes, highlights, and
//! history. Entering edit mode maps those positions from the canonical text
//! into the source, and leaving maps them back into the canonical text
//! rebuilt from the saved file ([`crate::align`]); for plain text both maps
//! are the identity. Leaving without a save restores the reading document
//! as it was, and discarding restores the positions of the last save.
//!
//! # Differences from star
//!
//! - Typing is echoed (characters, completed words, deletions, the new line
//!   on a line move, capitals as configured); star had no echo.
//! - Edit mode on and off, saving, undo, redo, and every formatting command
//!   are announced; star showed most of them only on the status bar.
//! - Quitting with unsaved edits asks Save / Discard / Cancel (star closed
//!   silently and relied on the snapshot, bug 38).
//! - Saves are atomic and keep the file's BOM and line endings; converted
//!   formats are never overwritten with Markdown (bugs 28, 29).

use std::path::{Path, PathBuf};
use std::time::Instant;

use ropey::Rope;
use textweaver_a11y::{Priority, Verbosity};
use textweaver_core::{CharPos, CharRange, Direction, Edit, EditOutcome, Unit};
use textweaver_editor::autosave::{self, AutosavePolicy, RecoverySnapshot};
use textweaver_editor::echo::{self, EchoEvent, EchoPolicy};
use textweaver_editor::{Choice, DocInfo, EditSession, LeaveOutcome, MarkdownOp, Selection};
use textweaver_formats::Source;
use textweaver_keymap::ActionId;
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Arg;
use textweaver_speech::{Earcon, SayMode};
use textweaver_store::{Bookmark, DocKey};
use textweaver_text::{Document, History, NavOptions, navigate};

use crate::align::Aligner;
use crate::app::{App, ListKind, Mode, Session};
use crate::command::{CaretMove, Effect, PromptPurpose};
use crate::notes::{UserHighlight, shift_marks};
use crate::text_util;
use textweaver_store::Note;

mod preview;
pub(crate) use preview::leaves_preview;

/// Positions that move with the text.
#[derive(Clone, Debug, Default)]
pub(crate) struct Marks {
    cursor: CharPos,
    bookmarks: Vec<Bookmark>,
    notes: Vec<Note>,
    highlights: Vec<UserHighlight>,
    history: Vec<CharPos>,
}

impl Marks {
    fn of(s: &Session) -> Self {
        Marks {
            cursor: s.cursor,
            bookmarks: s.bookmarks.clone(),
            notes: s.notes.clone(),
            highlights: s.highlights.clone(),
            history: s.history.entries().to_vec(),
        }
    }

    fn map(&self, f: impl Fn(CharPos) -> CharPos) -> Self {
        let range = |r: CharRange| {
            let a = f(r.start);
            CharRange::new(a, f(r.end).max(a))
        };
        let mut bookmarks = self.bookmarks.clone();
        for b in &mut bookmarks {
            b.pos = f(b.pos);
        }
        bookmarks.sort_by_key(|b| b.pos);
        let mut notes = self.notes.clone();
        for n in &mut notes {
            n.range = range(n.range);
        }
        let mut highlights = self.highlights.clone();
        for h in &mut highlights {
            h.range = range(h.range);
        }
        highlights.retain(|h| !h.range.is_empty());
        Marks {
            cursor: f(self.cursor),
            bookmarks,
            notes,
            highlights,
            history: self.history.iter().map(|&p| f(p)).collect(),
        }
    }

    fn put(self, s: &mut Session) {
        let len = s.doc.len_chars();
        s.cursor = self.cursor.clamp_to(len);
        s.bookmarks = self.bookmarks;
        for b in &mut s.bookmarks {
            b.pos = b.pos.clamp_to(len);
        }
        s.notes = self.notes;
        s.highlights = self.highlights;
        let mut h = History::with_capacity(s.history.capacity());
        for p in self.history {
            h.record(p.clamp_to(len));
        }
        s.history = h;
    }
}

/// What to do once edit mode has been left.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AfterLeave {
    /// Nothing more (Ctrl+E).
    Finish,
    /// Open this file.
    Open(PathBuf),
    /// Start a new document.
    New,
    /// Quit the application.
    Quit,
    /// Recover the first offered snapshot.
    Recover,
    /// Offer the templates for a new document.
    Templates,
}

/// What the Save As prompt's answer is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SaveThen {
    /// A plain save; keep editing.
    Stay,
    /// Saving on the way out of edit mode.
    Leave(AfterLeave),
}

/// Edit mode state.
pub(crate) struct EditState {
    pub(crate) session: EditSession,
    /// The reading document as it was on entering.
    reading: Document,
    /// Positions (canonical) on entering.
    entry: Marks,
    /// Positions (source) at the last save.
    saved: Option<Marks>,
    /// An edit was applied since entering.
    changed: bool,
    /// The reading view shown in place of the source (`toggle_preview`).
    preview: Option<preview::Preview>,
}

/// The first edit that turns `before` into `after`, given the outcomes of
/// the edits applied in order: only the span they touched is compared.
fn span_edit(before_len: usize, outcomes: &[EditOutcome], after: &Rope) -> Option<Edit> {
    if outcomes.is_empty() {
        return None;
    }
    let mut prefix = usize::MAX;
    let mut suffix = usize::MAX;
    let mut len = before_len;
    for o in outcomes {
        prefix = prefix.min(o.removed.start.0);
        suffix = suffix.min(len.saturating_sub(o.removed.end.0));
        len = len
            .saturating_sub(o.removed.len())
            .saturating_add(o.inserted.len());
    }
    let after_len = after.len_chars();
    let end_before = before_len.saturating_sub(suffix).max(prefix);
    let end_after = after_len.saturating_sub(suffix).max(prefix).min(after_len);
    let text = after.slice(prefix.min(end_after)..end_after).to_string();
    Some(Edit::replace(CharRange::new(prefix, end_before), text))
}

/// Carries `marks` from `from` to `to`, where one is canonical text and the
/// other the Markdown source it was built from: into the source when
/// `into_source`, else out of it. Paired blocks carry them
/// ([`crate::structure::SourceMap`]); without any structure on one side,
/// the word alignment of the two texts does.
fn carry_marks(marks: &Marks, from: &Document, to: &Document, into_source: bool) -> Marks {
    let (canonical, source) = if into_source { (from, to) } else { (to, from) };
    let map = crate::structure::SourceMap::build(canonical, source);
    if map.is_empty() {
        let aligner = Aligner::new(from.text(), to.text());
        return marks.map(|p| aligner.map(p));
    }
    if into_source {
        marks.map(|p| map.to_source(canonical, source, p))
    } else {
        marks.map(|p| map.to_canonical(canonical, source, p))
    }
}

/// Parses a table size: `3 by 2`, `3x2`, `3 2` (columns, then rows).
fn parse_table_size(s: &str) -> Option<(u16, u16)> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        return Some((2, 2));
    }
    let nums: Vec<u16> = s
        .split(|c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.parse().ok())
        .collect();
    match nums.as_slice() {
        [c] => Some(((*c).clamp(1, 20), 2)),
        [c, r] => Some(((*c).clamp(1, 20), (*r).clamp(1, 100))),
        _ => None,
    }
}

/// `s` with its first letter capitalized.
fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// True when `a` and `b` name the same file (compared resolved when both
/// exist, else as written).
pub(crate) fn same_path(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// The heading level of a Markdown line (`## x` is 2), 0 for none.
fn heading_level(line: &str) -> u8 {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    let rest = &line[hashes.min(line.len())..];
    if (1..=6).contains(&hashes) && (rest.is_empty() || rest.starts_with(' ')) {
        u8::try_from(hashes).unwrap_or(0)
    } else {
        0
    }
}

impl App {
    /// True in edit mode.
    pub fn is_editing(&self) -> bool {
        self.edit.is_some()
    }

    /// True when editing with unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.edit.as_ref().is_some_and(|e| e.session.is_dirty())
    }

    /// The edit session while editing (its text, selection, undo state).
    pub fn edit_session(&self) -> Option<&EditSession> {
        self.edit.as_ref().map(|e| &e.session)
    }

    fn echo_policy(&self) -> EchoPolicy {
        let e = &self.settings.editing;
        EchoPolicy {
            characters: e.echo_characters,
            words: e.echo_words,
            lines_on_move: e.echo_lines_on_move,
            deletions: e.echo_deletions,
            caps: self.settings.speech.caps,
        }
    }

    fn autosave_policy(&self) -> AutosavePolicy {
        let e = &self.settings.editing;
        AutosavePolicy::new(
            e.autosave_recovery && self.paths.is_some(),
            e.autosave_interval_secs,
        )
    }

    /// The editable source of the open document: its own text for plain
    /// text (identical to the canonical text), the Markdown file for
    /// Markdown, else the document converted to Markdown.
    fn editable_source(doc: &Document) -> (String, bool) {
        // Pause markup was taken out of a plain-text file's canonical text:
        // the file itself is edited, so the markup is kept as written.
        let has_markup_pauses =
            !textweaver_formats::pause_markup::written_pauses(&doc.meta).is_empty();
        match doc.meta.format.as_str() {
            "text" if has_markup_pauses => {
                let from_file =
                    doc.meta.path.as_ref().and_then(|p| {
                        textweaver_formats::source_text(&Source::Path(p.clone())).ok()
                    });
                match from_file {
                    Some(t) => (t, false),
                    None => (doc.text().to_string(), true),
                }
            }
            "text" => (doc.text().to_string(), true),
            "markdown" => {
                let from_file =
                    doc.meta.path.as_ref().and_then(|p| {
                        textweaver_formats::source_text(&Source::Path(p.clone())).ok()
                    });
                match from_file {
                    Some(t) => (t, false),
                    None => (textweaver_formats::to_markdown(doc), false),
                }
            }
            _ => (textweaver_formats::to_markdown(doc), false),
        }
    }

    /// Enters edit mode on the open document (Ctrl+E). With `source`, the
    /// editor holds that text instead of the file's (recovered work).
    pub(crate) fn enter_edit(&mut self, source: Option<String>) {
        if self.edit.is_some() {
            return;
        }
        self.stop_speech();
        let policy = self.autosave_policy();
        let recovery_dir = self.paths.as_ref().map(|p| p.recovery_dir());
        let undo_limits = textweaver_editor::UndoLimits {
            steps: self.settings.editing.undo_steps.max(1),
            bytes: self
                .settings
                .editing
                .undo_memory_mb
                .max(1)
                .saturating_mul(1024 * 1024),
        };
        // The structure parsed in the background when the file opened, if
        // the file is unchanged (large Markdown files only).
        let prefetched = if source.is_none() {
            self.prefetched_markers()
        } else {
            None
        };
        let Some(s) = self.session.as_mut() else {
            let new = self.key(ActionId::NewDocument);
            let msg = self.msg_args("edit-no-document", &args!["key" => new]);
            self.tell(&msg);
            return;
        };
        let recovered = source.is_some();
        let (text, identity) = match source {
            Some(t) => (t, false),
            None => Self::editable_source(&s.doc),
        };
        // Line breaks are `\n` in the editor, as in the canonical text.
        let text = if identity {
            text
        } else if text.contains('\r') {
            text.replace("\r\n", "\n").replace('\r', "\n")
        } else {
            text
        };
        let info = DocInfo {
            key: s.key.0.clone(),
            path: s.doc.meta.path.clone(),
            loader_id: s.doc.meta.format.clone(),
            title: s.title.clone(),
        };
        let mut session = EditSession::new(info, text);
        session.set_undo_limits(undo_limits);
        // Snapshots are written and deleted by the background writer.
        session.set_deferred_io(true);
        if let Some(dir) = recovery_dir {
            session = session.with_autosave(policy, dir);
        }
        session.enter_edit();
        // The edited document shares the editor's rope, and a Markdown
        // source gets its structure at source positions, which also carry
        // the reading positions across (`structure`).
        let edit_doc = if identity {
            s.doc.clone()
        } else {
            let rope = session
                .editor()
                .map(|e| e.text().clone())
                .unwrap_or_else(|| Rope::from_str(session.document_text()));
            let markers = prefetched
                .unwrap_or_else(|| crate::structure::source_markers(session.document_text()));
            Document::new(s.doc.meta.clone(), rope, markers)
        };
        let entry = Marks::of(s);
        let mapped = if identity {
            entry.clone()
        } else {
            carry_marks(&entry, &s.doc, &edit_doc, true)
        };
        let reading = std::mem::replace(&mut s.doc, edit_doc);
        s.revision = crate::app::next_revision();
        mapped.put(s);
        s.selection = None;
        s.selection_anchor = None;
        s.find = None;
        s.spoken = None;
        s.spoken_sentence = None;
        s.speech_cursor_line = None;
        s.goal_column = None;
        self.authoring.structure = crate::authoring_state::Structure {
            markdown: !identity,
            ..Default::default()
        };
        let cursor = s.cursor;
        if let Some(ed) = session.editor_mut() {
            ed.set_selection(Selection::caret(cursor));
            if recovered {
                ed.mark_modified();
            }
        }
        self.edit = Some(EditState {
            session,
            reading,
            entry,
            saved: None,
            changed: false,
            preview: None,
        });
        self.mode = Mode::Edit;
        self.return_mode = Mode::Edit;
        self.scroll_to_cursor();
        if !recovered {
            let save = self.keys(ActionId::Save);
            let finish = self.keys(ActionId::ToggleEditMode);
            let line = self
                .session
                .as_ref()
                .map(|s| echo::line_echo(s.doc.text(), s.line()))
                .map(|(line, cut)| {
                    if cut {
                        self.with_line_continues(line)
                    } else {
                        line
                    }
                })
                .unwrap_or_default();
            let msg = match self.settings.speech.verbosity {
                Verbosity::Low => self.msg_args("edit-mode-on-brief", &args!["line" => line]),
                _ => self.msg_args(
                    "edit-mode-on",
                    &args!["save" => save, "finish" => finish, "line" => line],
                ),
            };
            self.tell(&msg);
        }
    }

    /// Ctrl+E: enter edit mode, or leave it (asking about unsaved changes).
    pub(crate) fn toggle_edit(&mut self) -> Vec<Effect> {
        if self.edit.is_some() {
            return self.leave_edit(None, None, AfterLeave::Finish);
        }
        let guide = self
            .session
            .as_ref()
            .and_then(|s| s.doc.meta.path.as_deref())
            .is_some_and(|p| self.is_bundled_guide(p));
        if self.session.is_none() {
            let new = self.key(ActionId::NewDocument);
            let msg = self.msg_args("edit-no-document", &args!["key" => new]);
            self.tell(&msg);
        } else if guide {
            // The guides packaged with textweaver are read only.
            let msg = self.msg("docs-read-only");
            self.tell(&msg);
        } else {
            self.enter_edit(None);
        }
        vec![Effect::Redraw]
    }

    /// Tries to leave edit mode, then does `after`. Asks Save / Discard /
    /// Cancel (a list) when there are unsaved changes, and a file name when
    /// saving needs one.
    pub(crate) fn leave_edit(
        &mut self,
        choice: Option<Choice>,
        save_as: Option<PathBuf>,
        after: AfterLeave,
    ) -> Vec<Effect> {
        self.end_preview(false);
        if self.is_dirty() && choice == Some(Choice::Save) {
            // Written on the writer; edit mode is left when it reports
            // (crate::writes), or the question about a file changed on
            // disk is asked then.
            return self.start_save(save_as, SaveThen::Leave(after));
        }
        let Some(edit) = self.edit.as_mut() else {
            return self.continue_after(after);
        };
        let dirty = edit.session.is_dirty();
        let result = edit.session.finish_editing(choice, save_as.as_deref());
        match result {
            Ok(LeaveOutcome::Left { rebuild }) => {
                let discarded = dirty && choice == Some(Choice::Discard);
                // The file's stamp was kept when the writer saved it.
                self.finish_leave(rebuild, discarded);
                self.continue_after(after)
            }
            Ok(LeaveOutcome::Stayed) => {
                let msg = self.msg("edit-still-editing");
                self.tell(&msg);
                vec![Effect::Redraw]
            }
            Ok(LeaveOutcome::NeedsChoice) => {
                let title = self.edit_title();
                self.list = Some(ListKind::SaveChoice(after));
                let msg = self.msg_args("edit-unsaved-question", &args!["title" => title.as_str()]);
                self.tell(&msg);
                vec![Effect::ShowList {
                    title: self
                        .msg_args("edit-save-changes-title", &args!["title" => title.as_str()]),
                    items: vec![
                        self.msg("edit-choice-save"),
                        self.msg("edit-choice-discard"),
                        self.msg("edit-choice-cancel"),
                    ],
                }]
            }
            Ok(LeaveOutcome::NeedsPath { suggested }) => {
                self.ask_save_path(suggested, SaveThen::Leave(after))
            }
            Err(e) => {
                let msg = self.msg_args("edit-save-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
                vec![Effect::Redraw]
            }
        }
    }

    fn edit_title(&self) -> String {
        self.edit
            .as_ref()
            .map(|e| e.session.doc().title.clone())
            .unwrap_or_default()
    }

    pub(crate) fn ask_save_path(&mut self, suggested: PathBuf, then: SaveThen) -> Vec<Effect> {
        let suggested = match (&suggested, self.edit.as_ref()) {
            // A new document: suggest a file in the working directory.
            (p, Some(e))
                if e.session.doc().path.is_none()
                    && p.parent().is_none_or(|d| d.as_os_str().is_empty()) =>
            {
                std::env::current_dir()
                    .map(|d| d.join(p))
                    .unwrap_or_else(|_| p.clone())
            }
            _ => suggested,
        };
        self.save_then = Some(then);
        let label = self.msg_args(
            "edit-save-as-label",
            &args!["path" => suggested.display().to_string()],
        );
        self.suggested_path = Some(suggested);
        if !self.mode.is_prompt() {
            self.return_mode = self.mode;
        }
        self.mode = Mode::Prompt;
        self.prompt_purpose = PromptPurpose::SaveAs;
        let said = self.path_prompt_said(PromptPurpose::SaveAs, &label);
        self.tell(&said);
        vec![Effect::Prompt {
            label,
            purpose: PromptPurpose::SaveAs,
        }]
    }

    /// The answer to the Save As prompt.
    pub(crate) fn answer_save_as(&mut self, text: &str) -> Vec<Effect> {
        let then = self.save_then.take().unwrap_or(SaveThen::Stay);
        let suggested = self.suggested_path.take();
        let confirmed = std::mem::take(&mut self.save_as_replace_confirmed);
        let trimmed = text.trim().trim_matches('"');
        let path = if trimmed.is_empty() {
            suggested
        } else {
            let p = PathBuf::from(trimmed);
            Some(match (p.is_relative(), &suggested) {
                (true, Some(s)) => s.parent().map_or(p.clone(), |d| d.join(&p)),
                _ => p,
            })
        };
        let Some(path) = path else {
            let msg = self.msg("common-cancelled");
            self.note(&msg);
            return vec![Effect::Redraw];
        };
        self.save_as_to(path, then, confirmed)
    }

    /// Marks the next Save As answer as already confirmed by a system save
    /// dialog, which asks before replacing a file itself. The mark applies to
    /// that one answer only.
    pub fn save_as_confirmed_by_system(&mut self) {
        self.save_as_replace_confirmed = true;
    }

    /// Saves under `path` (a Save As answer). Another file already there is
    /// overwritten only after a yes (`confirmed`); the question is asked
    /// first ("notes.md already exists. Replace it? y or n").
    pub(crate) fn save_as_to(
        &mut self,
        path: PathBuf,
        then: SaveThen,
        confirmed: bool,
    ) -> Vec<Effect> {
        // The file actually written (a converted extension becomes `.md`).
        let dest = autosave::save_as_path(&path);
        let current = self
            .edit
            .as_ref()
            .and_then(|e| e.session.doc().path.clone());
        let same_file = current.as_deref().is_some_and(|c| same_path(c, &dest));
        if !confirmed && !same_file && dest.exists() {
            let name = dest.file_name().map_or_else(
                || dest.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            );
            self.pending_disk = Some(crate::disk::DiskQuestion::SaveAsOver {
                path: path.clone(),
                then,
            });
            self.list = None;
            let msg = self.msg_args("edit-file-exists-question", &args!["name" => name]);
            self.ask(&msg);
            return vec![Effect::Redraw];
        }
        match then {
            SaveThen::Stay => self.save(Some(path)),
            SaveThen::Leave(after) => self.leave_edit(Some(Choice::Save), Some(path), after),
        }
    }

    /// The answer to the Save / Discard / Cancel list.
    pub(crate) fn answer_save_choice(&mut self, n: usize, after: AfterLeave) -> Vec<Effect> {
        let choice = match n {
            0 => Choice::Save,
            1 => Choice::Discard,
            _ => Choice::Cancel,
        };
        self.leave_edit(Some(choice), None, after)
    }

    /// Tears edit mode down after the session left it: rebuilds the reading
    /// document from the saved file when a save happened, else restores it,
    /// and maps every position back.
    pub(crate) fn finish_leave(&mut self, rebuild: bool, discarded: bool) {
        self.end_preview(false);
        // The structure must match the text before positions are carried
        // from it; discarding goes back to positions that need none.
        if !discarded {
            self.refresh_structure(true);
        }
        let markdown = self.authoring.structure.markdown;
        self.authoring.structure = crate::authoring_state::Structure::default();
        let Some(mut state) = self.edit.take() else {
            return;
        };
        // The snapshot's deletion, queued when the session left edit mode.
        for op in state.session.take_snapshot_ops() {
            self.writer.send(crate::writer::Job::Snapshot(op));
        }
        self.stop_speech();
        let saved_text = state.session.document_text().to_owned();
        let path = state.session.doc().path.clone();
        let loader = state.session.doc().loader_id.clone();
        let Some(s) = self.session.as_mut() else {
            return;
        };
        // Source positions that match `saved_text` (or the entry text).
        let current = Marks::of(s);
        let source_marks = if discarded {
            state.saved.clone()
        } else {
            Some(current)
        };
        let rebuilt = if rebuild {
            let loaded = path.as_ref().map(|p| {
                self.registry
                    .load(&Source::Path(p.clone()), &self.load_options())
            });
            match loaded {
                Some(Ok(doc)) => Some(doc),
                _ => {
                    let hint = if loader == "text" { "txt" } else { "md" };
                    self.registry
                        .load(
                            &Source::Bytes {
                                data: saved_text.clone().into_bytes(),
                                hint: hint.into(),
                            },
                            &self.load_options(),
                        )
                        .ok()
                }
            }
        } else {
            None
        };
        let Some(s) = self.session.as_mut() else {
            return;
        };
        match (rebuilt, source_marks) {
            (Some(mut doc), Some(marks)) => {
                if let Some(p) = &path {
                    doc.meta.path = Some(p.clone());
                }
                // `marks` are positions in the saved text: the edited
                // document itself when nothing was changed after the save.
                let saved_doc = (s.doc.text() != saved_text.as_str()).then(|| {
                    let markers = if markdown {
                        crate::structure::source_markers(&saved_text)
                    } else {
                        Vec::new()
                    };
                    Document::new(s.doc.meta.clone(), Rope::from_str(&saved_text), markers)
                });
                let source = saved_doc.as_ref().unwrap_or(&s.doc);
                let mapped = carry_marks(&marks, source, &doc, false);
                s.doc = doc;
                mapped.put(s);
            }
            (Some(mut doc), None) => {
                // Discarded without a save of this session's positions:
                // should not happen (a rebuild means a save); keep entry.
                if let Some(p) = &path {
                    doc.meta.path = Some(p.clone());
                }
                s.doc = doc;
                state.entry.clone().put(s);
            }
            (None, marks) => {
                let edited = std::mem::replace(&mut s.doc, state.reading);
                match marks {
                    Some(m) if state.changed && !discarded => {
                        carry_marks(&m, &edited, &s.doc, false).put(s);
                    }
                    _ => state.entry.put(s),
                }
            }
        }
        s.cursor = text_util::word_start(&s.doc, s.cursor);
        s.selection = None;
        s.selection_anchor = None;
        s.find = None;
        s.goal_column = None;
        s.text_stamp = Some(crate::relocate::text_stamp(&s.doc));
        s.revision = crate::app::next_revision();
        self.mode = Mode::Browse;
        self.return_mode = Mode::Browse;
        self.scroll_to_cursor();
        if let Err(e) = self.save_position() {
            log::warn!("cannot save position: {e}");
        }
        let msg = self.msg(if discarded {
            "edit-mode-off-discarded"
        } else {
            "edit-mode-off"
        });
        self.tell(&msg);
        // Ready for the next Ctrl+E on a large Markdown file.
        self.prefetch_structure();
    }

    fn continue_after(&mut self, after: AfterLeave) -> Vec<Effect> {
        match after {
            AfterLeave::Finish => vec![Effect::Redraw],
            AfterLeave::Open(path) => self.dispatch_open(&path),
            AfterLeave::New => self.new_document(),
            AfterLeave::Quit => {
                self.shutdown();
                vec![Effect::Quit]
            }
            AfterLeave::Recover => self.recover_first(),
            AfterLeave::Templates => self.new_from_template(),
        }
    }

    /// File > New: a blank Markdown document in edit mode (after resolving
    /// unsaved edits).
    pub(crate) fn new_document(&mut self) -> Vec<Effect> {
        if self.edit.is_some() {
            return self.leave_edit(None, None, AfterLeave::New);
        }
        self.untitled += 1;
        let n = std::process::id()
            .wrapping_mul(100)
            .wrapping_add(self.untitled);
        let mut doc = Document::from_plain_text("");
        doc.meta.format = "markdown".into();
        let untitled = self.msg("edit-untitled");
        self.open_document(doc, DocKey::untitled(n), untitled);
        self.enter_edit(Some(String::new()));
        if let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) {
            // A new document is clean until typed into.
            ed.mark_saved();
        }
        let msg = self.msg("edit-new-document");
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    /// Ctrl+S: saves in place, or asks for a name (new and converted
    /// documents). Stays in edit mode.
    pub(crate) fn save(&mut self, save_as: Option<PathBuf>) -> Vec<Effect> {
        if self.edit.is_none() {
            let k = self.keys(ActionId::ToggleEditMode);
            let msg = self.msg_args("edit-nothing-to-save", &args!["key" => k]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        // Written on the writer; "Saved" is said when it reports.
        self.start_save(save_as, SaveThen::Stay)
    }

    /// Save As (Alt+S): always asks for a name.
    pub(crate) fn save_as(&mut self) -> Vec<Effect> {
        let Some(edit) = self.edit.as_ref() else {
            return self.save(None);
        };
        let suggested = edit
            .session
            .doc()
            .path
            .as_ref()
            .map(|p| autosave::save_as_path(p))
            .unwrap_or_else(|| {
                let text = edit
                    .session
                    .editor()
                    .map(|e| e.text().to_string())
                    .unwrap_or_default();
                PathBuf::from(autosave::suggest_file_name(&text))
            });
        self.ask_save_path(suggested, SaveThen::Stay)
    }

    /// Bookkeeping after a successful save: positions of the saved text, and
    /// on Save As the new key, title, and recent entry.
    pub(crate) fn after_save(&mut self, path: &Path, adopted: bool) {
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        let Some(s) = self.session.as_mut() else {
            return;
        };
        edit.saved = Some(Marks::of(s));
        if adopted {
            let key = DocKey::for_path(path);
            edit.session.doc_mut().key = key.0.clone();
            let title = path.file_name().map_or_else(
                || edit.session.doc().title.clone(),
                |n| n.to_string_lossy().into(),
            );
            edit.session.doc_mut().title = title.clone();
            s.key = key;
            s.title = title.clone();
            s.doc.meta.path = Some(path.to_owned());
            s.doc.meta.format = edit.session.doc().loader_id.clone();
            s.saved = textweaver_store::DocState::default();
            let format = s.doc.meta.format.clone();
            // The recent list and the bookshelf, on the writer; and its sync
            // id, under the new name.
            self.record_library_open(path, &title, &format);
        }
    }

    /// Applies what an editor operation did: the document, every position,
    /// the cursor and selection, and the view.
    pub(crate) fn after_edit(&mut self, before: &Rope, outcomes: &[EditOutcome]) {
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        let Some(ed) = edit.session.editor() else {
            return;
        };
        let sel = ed.selection();
        let span = span_edit(before.len_chars(), outcomes, ed.text());
        let Some(s) = self.session.as_mut() else {
            return;
        };
        // In a large Markdown text, shifting (and sorting) every marker on
        // each keystroke would cost milliseconds: the markers are dropped
        // at the first edit and parsed again when typing pauses, or at once
        // when a command needs them (`structure`).
        if span.is_some()
            && self.authoring.structure.markdown
            && s.doc.len_chars() > crate::structure::INLINE_PARSE_LIMIT
            && !s.doc.markers().is_empty()
        {
            s.doc = Document::new(s.doc.meta.clone(), s.doc.text().clone(), Vec::new());
        }
        if let Some(e) = span
            && let Err(err) = s.doc.apply(&e)
        {
            log::warn!("document out of step with the editor: {err}");
            let meta = s.doc.meta.clone();
            s.doc = Document::from_plain_text(&ed.text().to_string());
            s.doc.meta = meta;
        }
        shift_marks(&mut s.notes, &mut s.highlights, &mut s.bookmarks, outcomes);
        for o in outcomes {
            s.history.shift(o);
        }
        let changed = !outcomes.is_empty();
        if changed {
            edit.changed = true;
            s.revision = crate::app::next_revision();
            s.find = None;
            s.spoken = None;
            s.spoken_sentence = None;
        }
        s.cursor = sel.head;
        s.goal_column = None;
        let r = sel.range();
        s.selection = (!r.is_empty()).then_some(r);
        s.selection_anchor = s.selection.map(|_| sel.anchor);
        self.scroll_to_cursor();
        if changed {
            self.structure_edited();
        }
    }

    /// Speaks echo events when self-voicing (a screen reader echoes typing
    /// itself otherwise, and in the screen-reader and hybrid modes).
    fn speak_echo(&mut self, events: Vec<EchoEvent>) {
        if !self.route(textweaver_a11y::Channel::Echo).speak {
            return;
        }
        for (i, e) in events.into_iter().enumerate() {
            let mode = if i == 0 {
                SayMode::Interrupt
            } else {
                SayMode::Queue
            };
            match e {
                EchoEvent::Typed(c) if !c.is_whitespace() => self.speech.speak_char(c, None),
                EchoEvent::Typed(c) => self
                    .speech
                    .say(text_util::char_name_text(self.cat(), c), mode),
                EchoEvent::WordCompleted(w) => self.speech.say(w, mode),
                EchoEvent::Deleted(t) => {
                    let mut chars = t.chars();
                    match (chars.next(), chars.next()) {
                        (Some(c), None) if !c.is_whitespace() => self.speech.speak_char(c, None),
                        (Some(c), None) => self
                            .speech
                            .say(text_util::char_name_text(self.cat(), c), mode),
                        _ => self.speech.say(t, mode),
                    }
                }
                EchoEvent::CursorMoved(line) => {
                    // The echo holds at most the line's first sentence.
                    let long = self.session.as_ref().is_some_and(|s| {
                        let l = s.line();
                        l < s.doc.text().len_lines()
                            && s.doc.text().line(l).len_chars() > echo::LINE_ECHO_MAX + 1
                    });
                    let line = if long {
                        self.with_line_continues(line)
                    } else {
                        line
                    };
                    self.speech.say(line, mode);
                }
            }
        }
    }

    /// Speaks what the user asked to hear while editing (the character or
    /// word reached), when self-voicing.
    fn speak_edit_feedback(&mut self, text: &str) {
        if self.route(textweaver_a11y::Channel::Echo).speak && !text.is_empty() {
            self.speech.say(text, SayMode::Interrupt);
        }
    }

    /// Says how to turn on edit mode to do `what`, a key of the message
    /// edit-not-editing ("type", "cut-text", "format", ...).
    pub(crate) fn not_editing(&mut self, what: &str) -> Vec<Effect> {
        let k = self.keys(ActionId::ToggleEditMode);
        let msg = self.msg_args("edit-not-editing", &args!["key" => k, "what" => what]);
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    /// Typed text: one character is a keystroke (coalesced for undo and
    /// echoed); more is a paste (one undo step, announced).
    pub(crate) fn insert(&mut self, text: &str) -> Vec<Effect> {
        self.insert_said(text, None)
    }

    /// [`insert`](Self::insert), saying `said` for a paste instead of how
    /// much was pasted and how it starts.
    pub(crate) fn insert_said(&mut self, text: &str, said: Option<String>) -> Vec<Effect> {
        if self.edit.is_none() {
            return self.not_editing("type");
        }
        if self.previewing() {
            return self.preview_read_only();
        }
        if text.is_empty() {
            return vec![Effect::Redraw];
        }
        self.stop_speech();
        if text == "\n"
            && let Some(effects) = self.continue_list()
        {
            return effects;
        }
        let policy = self.echo_policy();
        let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) else {
            return vec![Effect::Redraw];
        };
        let before = ed.text().clone();
        let mut chars = text.chars();
        let single = match (chars.next(), chars.next()) {
            (Some(c), None) => Some(c),
            _ => None,
        };
        let result = match single {
            Some(c) => ed.type_char(c),
            None => ed.insert_text(text),
        };
        match result {
            Ok(o) => {
                let edit = Edit::replace(o.removed, text);
                self.after_edit(&before, &[o]);
                match single {
                    Some(_) => {
                        let ev = echo::for_edit(&policy, &before, &edit);
                        let echoing = policy.characters || policy.words;
                        match self.markdown_echo().filter(|_| echoing) {
                            // Markup just typed is said as what it means.
                            Some(said) => {
                                self.show(&said);
                                self.speak_edit_feedback(&said);
                            }
                            None => {
                                let ev = self.heading_word_echo(ev);
                                self.speak_echo(ev);
                            }
                        }
                    }
                    None => {
                        let msg = said.unwrap_or_else(|| self.pasted_message(text));
                        self.show(&msg);
                        self.speak_edit_feedback(&msg);
                    }
                }
            }
            Err(e) => {
                let msg = self.msg_args("edit-insert-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
            }
        }
        vec![Effect::Redraw]
    }

    /// What a paste says: how much (lines when there are several, else
    /// characters) and how it starts.
    fn pasted_message(&self, text: &str) -> String {
        let n = text.chars().count();
        let lines = text.trim_end_matches('\n').lines().count();
        let words: Vec<&str> = text.split_whitespace().take(6).collect();
        let more = text.split_whitespace().nth(6).is_some();
        let start = format!("{}{}", words.join(" "), if more { "…" } else { "" });
        if start.is_empty() {
            self.msg_args("edit-pasted", &args!["n" => n])
        } else if lines > 1 {
            self.msg_args(
                "edit-pasted-lines",
                &args!["n" => lines, "start" => start.as_str()],
            )
        } else {
            self.msg_args(
                "edit-pasted-start",
                &args!["n" => n, "start" => start.as_str()],
            )
        }
    }

    /// [`Command::ReplaceRange`](crate::Command::ReplaceRange): an edit a
    /// native text control made. Quiet: the control and the screen reader
    /// echo it. One character typed at the caret joins the typing undo
    /// step; anything else is one step.
    pub(crate) fn replace_range(&mut self, range: CharRange, text: &str) -> Vec<Effect> {
        if self.previewing() {
            return self.preview_read_only();
        }
        let Some(len) = self
            .edit
            .as_ref()
            .and_then(|e| e.session.editor())
            .map(|ed| ed.text().len_chars())
        else {
            return self.not_editing("change-text");
        };
        if range.start.0 > len || range.end.0 > len || range.start > range.end {
            let msg = self.msg_args(
                "edit-range-out-of-text",
                &args!["start" => range.start.0, "end" => range.end.0, "len" => len],
            );
            self.error(&msg);
            return vec![Effect::Redraw];
        }
        if range.is_empty() && text.is_empty() {
            return vec![Effect::Redraw];
        }
        self.stop_speech();
        let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) else {
            return vec![Effect::Redraw];
        };
        let before = ed.text().clone();
        ed.set_selection(Selection::new(range.start, range.end));
        let mut chars = text.chars();
        let result = match (chars.next(), chars.next()) {
            (Some(c), None) if range.is_empty() => ed.type_char(c),
            _ => ed.insert_text(text),
        };
        match result {
            Ok(o) => self.after_edit(&before, &[o]),
            Err(e) => {
                let msg = self.msg_args("edit-change-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
            }
        }
        vec![Effect::Redraw]
    }

    /// The source line holding the editor's caret and the caret's char
    /// offset in it.
    fn editor_caret_line(&self) -> Option<(String, usize, CharRange)> {
        let ed = self.edit.as_ref()?.session.editor()?;
        let rope = ed.text();
        let head = ed.selection().head.0.min(rope.len_chars());
        // The rope's own lines: the empty line after a final newline is a
        // line here (the caret can be on it).
        let line = rope.char_to_line(head);
        let start = rope.line_to_char(line);
        let text = rope.line(line).to_string();
        let text = text.trim_end_matches(['\n', '\r']).to_owned();
        let r = CharRange::new(start, start + text.chars().count());
        Some((text, head - start, r))
    }

    /// What Markdown typed just before the caret means ("heading level 2",
    /// "bullet", "numbered item 3"), when a character completed it.
    fn markdown_echo(&self) -> Option<String> {
        let (text, col, _) = self.editor_caret_line()?;
        let before: String = text.chars().take(col).collect();
        let said = crate::mdline::markdown_echo(self.cat(), &before)?;
        let mut chars = said.chars();
        chars
            .next()
            .map(|f| f.to_uppercase().chain(chars).collect::<String>())
    }

    /// The first word completed on a heading line is said with the heading:
    /// `## Methods` as "Heading level 2, Methods".
    fn heading_word_echo(&self, mut events: Vec<EchoEvent>) -> Vec<EchoEvent> {
        let Some((text, col, _)) = self.editor_caret_line() else {
            return events;
        };
        let level = crate::mdline::heading_level(&text);
        if level == 0 {
            return events;
        }
        let before: String = text.chars().take(col).collect();
        let content = before.trim_start().trim_start_matches('#');
        let heading = self.msg_args("nav-label-heading-level", &args!["level" => level]);
        for e in &mut events {
            if let EchoEvent::WordCompleted(w) = e {
                let first = content
                    .split_whitespace()
                    .next()
                    .is_some_and(|f| f.trim_matches(|c: char| !c.is_alphanumeric()) == w.as_str());
                if first {
                    *w = format!("{heading}, {w}");
                }
            }
        }
        events
    }

    /// Enter in a list item (edit mode): continues the list with the next
    /// bullet or number (a task item gets an empty box); Enter on an empty
    /// item removes its marker, ending the list. `None` when the caret is
    /// not after a list item's marker, so Enter types a plain line break.
    fn continue_list(&mut self) -> Option<Vec<Effect>> {
        let (text, col, range) = self.editor_caret_line()?;
        let ed = self.edit.as_ref()?.session.editor()?;
        if !ed.selection().is_caret() {
            return None;
        }
        let m = crate::mdline::list_marker(&text)?;
        let byte_col = text.char_indices().nth(col).map_or(text.len(), |(b, _)| b);
        if byte_col < m.content_start.min(text.len()) {
            return None;
        }
        let content_empty = text[m.content_start.min(text.len())..].trim().is_empty();
        // What to say, before the editor is borrowed.
        let ended = self.msg("edit-list-ended");
        let prefix = m.next_prefix();
        let next = crate::mdline::list_marker(prefix.trim_end())
            .or_else(|| crate::mdline::list_marker(&prefix))
            .map_or_else(|| self.msg("edit-bullet"), |n| n.spoken(self.cat()));
        let ed = self.edit.as_mut()?.session.editor_mut()?;
        let before = ed.text().clone();
        let (result, said) = if content_empty {
            ed.set_selection(Selection::new(range.start, range.end));
            (
                ed.backspace().map(|o| o.into_iter().collect::<Vec<_>>()),
                ended,
            )
        } else {
            (
                ed.insert_text(&format!("\n{prefix}")).map(|o| vec![o]),
                capitalize_first(&next),
            )
        };
        match result {
            Ok(outcomes) => {
                self.after_edit(&before, &outcomes);
                self.show(&said);
                self.speak_edit_feedback(&said);
            }
            Err(e) => {
                let msg = self.msg_args("edit-insert-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
            }
        }
        Some(vec![Effect::Redraw])
    }

    /// Deletes the editor's selection without echo (Cut says what it took).
    pub(crate) fn delete_quietly(&mut self) {
        let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) else {
            return;
        };
        let before = ed.text().clone();
        match ed.backspace() {
            Ok(Some(o)) => self.after_edit(&before, &[o]),
            Ok(None) => {}
            Err(e) => {
                let msg = self.msg_args("edit-delete-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
            }
        }
    }

    /// A source line as said on a caret move in edit mode: its structure
    /// first, then its text without the markup ("heading level 2,
    /// Methods", "bullet, milk", "row 2, Ada | 36").
    pub(crate) fn spoken_source_line(&self, line: usize) -> Option<String> {
        let s = self.session.as_ref()?;
        // A long line is read only as far as the first sentence or the
        // first 200 characters; its structure shows in its start (W6u).
        let limit = 4 * echo::LINE_ECHO_MAX;
        let long = line < s.doc.text().len_lines() && s.doc.text().line(line).len_chars() > limit;
        let text = if long {
            s.doc.text().line(line).chars().take(limit).collect()
        } else {
            text_util::line_text(&s.doc, line)
        };
        let said = self.spoken_line_text(line, text)?;
        let (said, cut) = echo::cap_text(&said);
        Some(if cut || long {
            self.with_line_continues(said)
        } else {
            said
        })
    }

    /// `said` followed by "line continues" in the interface's language.
    pub(crate) fn with_line_continues(&self, said: String) -> String {
        let more = self.msg("edit-line-continues");
        format!("{said} {more}")
    }

    /// [`spoken_source_line`](Self::spoken_source_line) for the line's
    /// `text` (or its start).
    fn spoken_line_text(&self, line: usize, text: String) -> Option<String> {
        let s = self.session.as_ref()?;
        if text.trim().is_empty() {
            return Some(self.msg("nav-blank"));
        }
        let level = crate::mdline::heading_level(&text);
        if level > 0 {
            let body = text.trim_start().trim_start_matches('#').trim();
            let body = body.trim_end_matches('#').trim_end();
            let heading = self.msg_args("nav-line-heading", &args!["level" => level]);
            return Some(format!("{heading}, {body}"));
        }
        if let Some(m) = crate::mdline::list_marker(&text) {
            let body = text[m.content_start.min(text.len())..].trim();
            let blank = self.msg("nav-blank");
            let body = if body.is_empty() {
                blank.as_str()
            } else {
                body
            };
            return Some(format!("{}, {body}", m.spoken(self.cat())));
        }
        if crate::mdline::is_table_row(&text) {
            if crate::mdline::is_table_delimiter(&text) {
                return Some(self.msg("edit-table-divider"));
            }
            let mut first = line;
            while first > 0 && crate::mdline::is_table_row(&text_util::line_text(&s.doc, first - 1))
            {
                first -= 1;
            }
            let row = (first..=line)
                .filter(|&l| !crate::mdline::is_table_delimiter(&text_util::line_text(&s.doc, l)))
                .count();
            let cells: Vec<String> = crate::mdline::table_cells(&text)
                .into_iter()
                .map(|r| {
                    let c = &text[r];
                    if c.is_empty() {
                        self.msg("nav-blank")
                    } else {
                        c.to_owned()
                    }
                })
                .collect();
            let row = self.msg_args("nav-line-row", &args!["n" => row]);
            return Some(format!("{row}, {}", cells.join(", ")));
        }
        match crate::mdline::structure_of(self.cat(), &text) {
            Some(kind) => {
                let body = text.trim_start().trim_start_matches(['>', ' ']);
                Some(format!("{kind}, {body}"))
            }
            None => Some(text),
        }
    }

    /// Backspace and Delete.
    pub(crate) fn delete(&mut self, forward: bool) -> Vec<Effect> {
        if self.edit.is_none() {
            return self.not_editing("delete-text");
        }
        if self.previewing() {
            return self.preview_read_only();
        }
        self.stop_speech();
        let policy = self.echo_policy();
        let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) else {
            return vec![Effect::Redraw];
        };
        let before = ed.text().clone();
        let result = if forward {
            ed.delete_forward()
        } else {
            ed.backspace()
        };
        match result {
            Ok(Some(o)) => {
                let edit = Edit::delete(o.removed);
                self.after_edit(&before, &[o]);
                let ev = echo::for_edit(&policy, &before, &edit);
                // A long deletion's summary in the interface's language.
                let removed = before
                    .slice(edit.range.clamp_to(before.len_chars()).to_range())
                    .to_string();
                let ev = ev
                    .into_iter()
                    .map(|e| match e {
                        echo::EchoEvent::Deleted(t) => echo::EchoEvent::Deleted(
                            text_util::summary_text(self.cat(), &removed, "deleted").unwrap_or(t),
                        ),
                        other => other,
                    })
                    .collect();
                self.speak_echo(ev);
            }
            Ok(None) => {
                self.speech.earcon(Earcon::Boundary);
                let msg = self.msg(if forward {
                    "nav-end-of-document-stop"
                } else {
                    "nav-top-of-document-stop"
                });
                self.tell(&msg);
            }
            Err(e) => {
                let msg = self.msg_args("edit-delete-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
            }
        }
        vec![Effect::Redraw]
    }

    /// Moves the editor caret (arrows, Home, End, Page keys), echoing the
    /// character, word, or line reached.
    pub(crate) fn move_caret(
        &mut self,
        by: CaretMove,
        dir: Direction,
        extend: bool,
    ) -> Vec<Effect> {
        if self.edit.is_none() || self.previewing() {
            // Outside edit mode, and in its preview, the caret moves are
            // the browse actions.
            let a = match (by, dir) {
                (CaretMove::Char | CaretMove::Word, Direction::Forward) => ActionId::CaretNextWord,
                (CaretMove::Char | CaretMove::Word, Direction::Backward) => {
                    ActionId::CaretPreviousWord
                }
                (CaretMove::Line | CaretMove::LineEdge, Direction::Forward) => {
                    ActionId::CaretNextLine
                }
                (CaretMove::Line | CaretMove::LineEdge, Direction::Backward) => {
                    ActionId::CaretPreviousLine
                }
                (CaretMove::Page, Direction::Forward) => ActionId::PageDown,
                (CaretMove::Page, Direction::Backward) => ActionId::PageUp,
                (CaretMove::DocumentEdge, Direction::Forward) => ActionId::DocumentEnd,
                (CaretMove::DocumentEdge, Direction::Backward) => ActionId::DocumentStart,
            };
            return self.action(a);
        }
        self.stop_speech();
        let policy = self.echo_policy();
        let page = usize::from(self.view.height.max(2)) - 1;
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let Some(ed) = self.edit.as_ref().and_then(|e| e.session.editor()) else {
            return vec![Effect::Redraw];
        };
        let doc = &s.doc;
        let sel = ed.selection();
        let head = sel.head;
        let len = doc.len_chars();
        let line = text_util::line_of(doc, head);
        let line_r = text_util::line_range(doc, line);
        let goal = s
            .goal_column
            .unwrap_or(head.0.saturating_sub(line_r.start.0));
        // Collapsing a selection with Left or Right goes to its edge.
        let collapse_to = (!extend && !sel.is_caret() && by == CaretMove::Char).then(|| {
            let r = sel.range();
            match dir {
                Direction::Forward => r.end,
                Direction::Backward => r.start,
            }
        });
        let line_target = |lines: usize| -> Option<CharPos> {
            let count = text_util::line_count(doc);
            let target = match dir {
                Direction::Forward if line + 1 < count => (line + lines).min(count - 1),
                Direction::Backward if line > 0 => line.saturating_sub(lines),
                _ => return None,
            };
            let r = text_util::line_range(doc, target);
            Some(CharPos(r.start.0 + goal.min(r.len())))
        };
        let target = match collapse_to {
            Some(p) => Some(p),
            None => match by {
                CaretMove::Char => match dir {
                    Direction::Forward => (head.0 < len).then(|| {
                        navigate(doc, head, Unit::Grapheme, dir, NavOptions::default())
                            .map_or(CharPos(head.0 + 1), |t| t.range.start)
                            .max(CharPos(head.0 + 1))
                    }),
                    Direction::Backward => (head.0 > 0).then(|| {
                        navigate(doc, head, Unit::Grapheme, dir, NavOptions::default())
                            .map_or(CharPos(head.0 - 1), |t| t.range.start)
                            .min(CharPos(head.0 - 1))
                    }),
                },
                CaretMove::Word => match dir {
                    Direction::Forward => {
                        navigate(doc, head, Unit::Word, dir, NavOptions::default())
                            .map(|t| t.range.start)
                            .or_else(|| (head.0 < len).then_some(doc.end()))
                    }
                    Direction::Backward => {
                        let inside = text_util::word_containing(doc, head)
                            .filter(|w| w.start < head)
                            .map(|w| w.start);
                        inside.or_else(|| {
                            navigate(doc, head, Unit::Word, dir, NavOptions::default())
                                .map(|t| t.range.start)
                                .or_else(|| (head.0 > 0).then_some(CharPos::ZERO))
                        })
                    }
                },
                CaretMove::Line => line_target(1),
                CaretMove::Page => line_target(page),
                CaretMove::LineEdge => Some(match dir {
                    Direction::Forward => line_r.end,
                    Direction::Backward => line_r.start,
                })
                .filter(|&p| p != head),
                CaretMove::DocumentEdge => Some(match dir {
                    Direction::Forward => doc.end(),
                    Direction::Backward => CharPos::ZERO,
                })
                .filter(|&p| p != head),
            },
        };
        let Some(target) = target else {
            self.speech.earcon(Earcon::Boundary);
            let msg = self.msg(match (by, dir) {
                (CaretMove::LineEdge, Direction::Forward) => "edit-end-of-line-stop",
                (CaretMove::LineEdge, Direction::Backward) => "edit-start-of-line-stop",
                (_, Direction::Forward) => "nav-end-of-document-stop",
                (_, Direction::Backward) => "nav-top-of-document-stop",
            });
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        let keep_goal = matches!(by, CaretMove::Line | CaretMove::Page);
        let new_sel = if extend {
            Selection::new(sel.anchor, target)
        } else {
            Selection::caret(target)
        };
        // What to say, computed before the borrow ends.
        let feedback = if extend {
            let changed = CharRange::new(head, target);
            let text = doc.slice(changed);
            let grew = new_sel.range().contains_range(changed);
            Some(text_util::selection_change_text(self.cat(), &text, grew))
        } else {
            match by {
                CaretMove::Char | CaretMove::LineEdge => Some(match doc.char_at(target) {
                    Some('\n') | None if target == doc.end() => {
                        self.msg("playback-end-of-document-content")
                    }
                    Some('\n') => self.msg("edit-end-of-line-content"),
                    Some(c) => text_util::char_name(c),
                    None => self.msg("playback-end-of-document-content"),
                }),
                CaretMove::Word => Some(
                    text_util::word_containing(doc, target)
                        .map(|w| doc.slice(w))
                        .unwrap_or_else(|| {
                            doc.char_at(target).map_or_else(
                                || self.msg("playback-end-of-document-content"),
                                text_util::char_name,
                            )
                        }),
                ),
                CaretMove::Line | CaretMove::Page | CaretMove::DocumentEdge => {
                    let ev = echo::for_move(&policy, doc.text(), head, target);
                    match ev.into_iter().next() {
                        // Structure first: "heading level 2, Methods".
                        Some(EchoEvent::CursorMoved(l)) => self
                            .spoken_source_line(text_util::line_of(doc, target))
                            .or(Some(l)),
                        _ => None,
                    }
                }
            }
        };
        if let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) {
            ed.set_selection(new_sel);
        }
        self.after_edit(&Rope::new(), &[]);
        if keep_goal && let Some(s) = self.session.as_mut() {
            s.goal_column = Some(goal);
        }
        if let Some(f) = feedback {
            if extend {
                self.tell(&f);
            } else {
                self.speak_edit_feedback(&f);
            }
        }
        vec![Effect::Redraw]
    }

    /// Moves the editor caret to the session cursor after a navigation
    /// action (Ctrl+Home, find, go to) in edit mode.
    pub(crate) fn sync_editor_caret(&mut self) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let cursor = s.cursor;
        let sel = s.selection;
        let anchor = s.selection_anchor;
        let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) else {
            return;
        };
        let want = match (sel, anchor) {
            (Some(_), Some(a)) => Selection::new(a, cursor),
            _ => Selection::caret(cursor),
        };
        if ed.selection() != want {
            ed.set_selection(want);
        }
    }

    /// Runs an editing action (formatting, undo, redo, insert table or
    /// image, replace).
    pub(crate) fn edit_action(&mut self, a: ActionId) -> Vec<Effect> {
        use ActionId as A;
        if self.edit.is_none() {
            let what = match a {
                A::Undo => "undo",
                A::Redo => "redo",
                A::Replace => "replace-text",
                A::InsertTable | A::InsertImage | A::HorizontalRule | A::AddTableRow => "insert",
                _ => "format",
            };
            return self.not_editing(what);
        }
        self.stop_speech();
        match a {
            A::Undo => return self.undo_redo(true),
            A::Redo => return self.undo_redo(false),
            A::InsertTable => return self.prompt(PromptPurpose::TableSize),
            A::InsertImage => return self.prompt(PromptPurpose::ImagePath),
            A::Replace => return self.prompt(PromptPurpose::ReplaceFind),
            _ => {}
        }
        // Keys of the edit-format-* messages.
        let (op, what) = match a {
            A::Bold => (MarkdownOp::Bold, "bold"),
            A::Italic => (MarkdownOp::Italic, "italic"),
            A::Underline => (MarkdownOp::Underline, "underline"),
            A::Strikethrough => (MarkdownOp::Strikethrough, "strikethrough"),
            A::InlineCode => (MarkdownOp::InlineCode, "code"),
            A::CodeBlock => (MarkdownOp::CodeBlock, "code-block"),
            A::InsertLink => (MarkdownOp::Link, "link"),
            A::BulletList => (MarkdownOp::BulletList, "bulleted-list"),
            A::NumberedList => (MarkdownOp::NumberedList, "numbered-list"),
            A::BlockQuote => (MarkdownOp::Quote, "block-quote"),
            A::HorizontalRule => (MarkdownOp::HorizontalRule, "horizontal-rule"),
            A::AddTableRow => (MarkdownOp::AddTableRow, "table-row"),
            A::Heading => return self.cycle_heading(),
            _ => return vec![Effect::Redraw],
        };
        self.format(op, &args!["what" => what])
    }

    /// Applies a formatting command and announces it: "Bold." when markup
    /// was added, "Bold removed." when the command toggled it off. `args`
    /// are the values of the edit-format-* messages: `what` names the
    /// command, with `level`, `cols` and `rows` where it needs them.
    fn format(&mut self, op: MarkdownOp, args: &[(&str, Arg)]) -> Vec<Effect> {
        let Some(edit) = self.edit.as_mut() else {
            return vec![Effect::Redraw];
        };
        let Some(before) = edit.session.editor().map(|e| e.text().clone()) else {
            return vec![Effect::Redraw];
        };
        match edit.session.format(op) {
            Ok(outcomes) => {
                if outcomes.is_empty() {
                    let msg = self.msg_args("edit-format-unchanged", args);
                    self.tell(&msg);
                    return vec![Effect::Redraw];
                }
                self.after_edit(&before, &outcomes);
                let after_len = self
                    .edit
                    .as_ref()
                    .and_then(|e| e.session.editor())
                    .map_or(0, |e| e.text().len_chars());
                let removed = after_len < before.len_chars();
                let selected = self
                    .session
                    .as_ref()
                    .and_then(|s| s.selection.map(|r| text_util::preview(&s.doc, r, 6)));
                let mut msg = if removed {
                    self.msg_args("edit-format-removed", args)
                } else {
                    self.msg_args("edit-format-done", args)
                };
                if let Some(sel) = selected.filter(|t| !t.is_empty()) {
                    let selected = self.msg_args("edit-format-selected", &args!["text" => sel]);
                    msg.push(' ');
                    msg.push_str(&selected);
                }
                self.tell(&msg);
            }
            Err(e) => {
                self.speech.earcon(Earcon::Error);
                self.error(&format!("{e}."));
            }
        }
        vec![Effect::Redraw]
    }

    /// Heading: none, level 1, 2, ... 6, none again.
    fn cycle_heading(&mut self) -> Vec<Effect> {
        let level = self
            .session
            .as_ref()
            .map(|s| heading_level(&text_util::line_text(&s.doc, s.line())))
            .unwrap_or(0);
        let next = if level == 0 { 1 } else { (level + 1).min(6) };
        let what = if level == 6 {
            "heading"
        } else {
            "heading-level"
        };
        let effects = self.format(
            MarkdownOp::Heading(next),
            &args!["what" => what, "level" => next],
        );
        if level > 0 && level < 6 {
            // Changing the level shortens nothing; say the new level only.
            let msg = self.msg_args("edit-heading-level-now", &args!["level" => next]);
            self.tell(&msg);
        }
        effects
    }

    fn undo_redo(&mut self, undo: bool) -> Vec<Effect> {
        let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) else {
            return vec![Effect::Redraw];
        };
        let before = ed.text().clone();
        let outcomes = if undo { ed.undo() } else { ed.redo() };
        match outcomes {
            Some(o) => {
                self.after_edit(&before, &o);
                let line = self
                    .session
                    .as_ref()
                    .map(|s| echo::line_echo(s.doc.text(), s.line()))
                    .map(|(line, cut)| {
                        if cut {
                            self.with_line_continues(line)
                        } else {
                            line
                        }
                    })
                    .unwrap_or_default();
                let what = if undo { "undo" } else { "redo" };
                let msg = match self.settings.speech.verbosity {
                    Verbosity::Low => self.msg_args("edit-undo-redo", &args!["what" => what]),
                    _ => self.msg_args(
                        "edit-undo-redo-line",
                        &args!["what" => what, "line" => line],
                    ),
                };
                self.tell(&msg);
            }
            None => {
                self.speech.earcon(Earcon::Boundary);
                let msg = self.msg(if undo {
                    "edit-nothing-to-undo"
                } else {
                    "edit-nothing-to-redo"
                });
                self.tell(&msg);
            }
        }
        vec![Effect::Redraw]
    }

    /// The table-size prompt's answer.
    pub(crate) fn answer_table(&mut self, text: &str) -> Vec<Effect> {
        match parse_table_size(text) {
            Some((cols, rows)) => self.format(
                MarkdownOp::InsertTable { rows, cols },
                &args!["what" => "table", "cols" => cols, "rows" => rows],
            ),
            None => {
                let msg = self.msg_args("edit-not-a-table-size", &args!["text" => text]);
                self.error(&msg);
                vec![Effect::Redraw]
            }
        }
    }

    /// The image prompt's answer.
    pub(crate) fn answer_image(&mut self, text: &str) -> Vec<Effect> {
        let path = text.trim().trim_matches('"');
        if path.is_empty() {
            let msg = self.msg("common-cancelled");
            self.note(&msg);
            return vec![Effect::Redraw];
        }
        let path = PathBuf::from(path);
        let Some(edit) = self.edit.as_mut() else {
            return vec![Effect::Redraw];
        };
        let Some(before) = edit.session.editor().map(|e| e.text().clone()) else {
            return vec![Effect::Redraw];
        };
        match edit.session.insert_image(&path) {
            Ok(outcomes) => {
                self.after_edit(&before, &outcomes);
                let name = path.file_name().map_or_else(
                    || path.display().to_string(),
                    |n| n.to_string_lossy().into(),
                );
                let msg = self.msg_args("edit-image-inserted", &args!["name" => name]);
                self.tell(&msg);
            }
            Err(e) => {
                let msg = self.msg_args("edit-image-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
            }
        }
        vec![Effect::Redraw]
    }

    /// The replace prompts' answers: the text to find, then its
    /// replacement (every match, one undo step).
    pub(crate) fn answer_replace(&mut self, text: &str, with: bool) -> Vec<Effect> {
        if !with {
            if text.is_empty() {
                let msg = self.msg("common-cancelled");
                self.note(&msg);
                return vec![Effect::Redraw];
            }
            if self.refuse_bad_pattern(text) {
                return vec![Effect::Redraw];
            }
            let opts = self.search;
            let n = self
                .edit
                .as_ref()
                .and_then(|e| e.session.editor())
                .map_or(0, |ed| crate::search_options::count(ed.text(), text, opts));
            if n == 0 {
                self.speech.earcon(Earcon::Error);
                let msg = self.msg_args("common-no-matches", &args!["query" => text]);
                self.tell(&msg);
                return vec![Effect::Redraw];
            }
            self.replace_query = Some(text.to_owned());
            let mut e = self.prompt(PromptPurpose::ReplaceWith);
            let msg = self.msg_args("edit-replace-with", &args!["n" => n, "query" => text]);
            self.tell(&msg);
            e.push(Effect::Redraw);
            return e;
        }
        let Some(query) = self.replace_query.take() else {
            return vec![Effect::Redraw];
        };
        // One match at a time: replace, skip, or replace all the rest
        // (`crate::replace`).
        self.start_replace(query, text.to_owned())
    }

    /// Autosave while editing (from [`App::tick`]).
    pub(crate) fn autosave_tick(&mut self, now: Instant) {
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        match edit.session.autosave_tick(now) {
            Err(e) => {
                let failures = edit.session.snapshot_failures();
                log::warn!("autosave failed ({failures} in a row): {e}");
                // Said once per run of failures; the session backs off
                // between attempts, and the log keeps every one.
                if failures == 1 {
                    let msg = self.msg_args(
                        "common-recovery-write-failed",
                        &args!["error" => e.to_string()],
                    );
                    self.say_at(&msg, Verbosity::Low, Priority::Polite);
                }
                self.snapshot_trouble = true;
            }
            Ok(true) if std::mem::take(&mut self.snapshot_trouble) => {
                let msg = self.msg("common-recovery-writing-again");
                self.note(&msg);
            }
            Ok(_) => {}
        }
    }

    /// Before the process ends unexpectedly (a panic, a signal, the
    /// terminal closing): writes the recovery snapshot of unsaved edits at
    /// once, saves the reading position and settings, and stops speech.
    /// Never asks anything; safe to call more than once.
    pub fn emergency_save(&mut self) {
        if let Some(edit) = self.edit.as_mut() {
            match edit.session.snapshot_now() {
                Ok(true) => log::warn!("wrote a recovery snapshot before exiting"),
                Ok(false) => {}
                Err(e) => log::error!("could not write the recovery snapshot: {e}"),
            }
        }
        self.shutdown();
    }

    /// Offers unsaved work found at startup (star's recovery prompt), one
    /// snapshot at a time. Returns the list effect, or nothing when there is
    /// none or recovery is off.
    pub fn offer_recovery(&mut self) -> Vec<Effect> {
        self.entry(|app| {
            if !app.settings.editing.autosave_recovery {
                return Vec::new();
            }
            let Some(paths) = &app.paths else {
                return Vec::new();
            };
            app.recovery = autosave::scan_snapshots(&paths.recovery_dir());
            app.show_recovery_offer()
        })
    }

    fn show_recovery_offer(&mut self) -> Vec<Effect> {
        let Some((_, snap)) = self.recovery.first() else {
            return Vec::new();
        };
        let title = snap.display_title();
        let when = textweaver_store::time::human(snap.ts);
        self.list = Some(ListKind::Recovery);
        let msg = self.msg_args(
            "edit-recovery-offer",
            &args!["title" => title.as_str(), "when" => when],
        );
        self.tell(&msg);
        vec![Effect::ShowList {
            title: self.msg_args("edit-recovery-title", &args!["title" => title.as_str()]),
            items: vec![
                self.msg_args("edit-recovery-yes", &args!["title" => title.as_str()]),
                self.msg("edit-recovery-no"),
            ],
        }]
    }

    /// The answer to the recovery offer: 0 recovers, 1 discards.
    pub(crate) fn answer_recovery(&mut self, n: usize) -> Vec<Effect> {
        if n == 0 {
            if self.edit.is_some() {
                return self.leave_edit(None, None, AfterLeave::Recover);
            }
            return self.recover_first();
        }
        if self.recovery.is_empty() {
            return vec![Effect::Redraw];
        }
        let (file, snap) = self.recovery.remove(0);
        if let Err(e) = autosave::delete_snapshot(&file) {
            log::warn!("cannot delete {}: {e}", file.display());
        }
        let msg = self.msg_args(
            "edit-recovery-discarded",
            &args!["title" => snap.display_title()],
        );
        self.tell(&msg);
        let mut e = self.show_recovery_offer();
        e.push(Effect::Redraw);
        e
    }

    fn recover_first(&mut self) -> Vec<Effect> {
        if self.recovery.is_empty() {
            return vec![Effect::Redraw];
        }
        let (file, snap) = self.recovery.remove(0);
        if let Err(e) = autosave::delete_snapshot(&file) {
            log::warn!("cannot delete {}: {e}", file.display());
        }
        self.recover(&snap);
        let mut e = self.show_recovery_offer();
        e.push(Effect::Redraw);
        e
    }

    /// Opens the snapshot's document (or a new one) in edit mode holding the
    /// recovered text, marked unsaved.
    fn recover(&mut self, snap: &RecoverySnapshot) {
        let title = snap.display_title();
        let loaded = snap.path.as_ref().filter(|p| p.is_file()).and_then(|p| {
            self.registry
                .load(&Source::Path(p.clone()), &self.load_options())
                .ok()
                .map(|d| (d, DocKey::for_path(p)))
        });
        let (doc, key) = match loaded {
            Some(x) => x,
            None => {
                let mut d = Document::from_plain_text(&snap.text);
                d.meta.format = "markdown".into();
                d.meta.path = snap.path.clone();
                (d, DocKey(snap.doc_key.clone()))
            }
        };
        let doc_title = snap
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map_or_else(|| title.clone(), |n| n.to_string_lossy().into_owned());
        self.open_document(doc, key, doc_title);
        self.enter_edit(Some(snap.text.clone()));
        let msg = self.msg_args("edit-recovered", &args!["title" => title.as_str()]);
        self.tell(&msg);
    }

    /// Cancelling the recovery list keeps the snapshots for next time.
    pub(crate) fn postpone_recovery(&mut self) {
        let n = self.recovery.len();
        self.recovery.clear();
        if n > 0 {
            let msg = self.msg("edit-recovery-postponed");
            self.tell(&msg);
        }
    }

    /// Quitting: asks about unsaved edits first.
    pub(crate) fn quit(&mut self) -> Vec<Effect> {
        if self.is_dirty() {
            return self.leave_edit(None, None, AfterLeave::Quit);
        }
        // A save during this session changed the file: rebuild the reading
        // view from it, so the positions saved on the way out match what
        // will be loaded next time (the September 2026 audit, finding D1).
        if let Some(rebuild) = self.edit.as_ref().map(|e| e.session.maps_stale()) {
            self.finish_leave(rebuild, false);
        }
        self.shutdown();
        vec![Effect::Quit]
    }

    /// Opening a file: asks about unsaved edits first.
    pub(crate) fn open_command(&mut self, path: PathBuf) -> Vec<Effect> {
        if self.edit.is_some() {
            return self.leave_edit(None, None, AfterLeave::Open(path));
        }
        self.dispatch_open(&path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_sizes_parse() {
        assert_eq!(parse_table_size("3 by 2"), Some((3, 2)));
        assert_eq!(parse_table_size("4x5"), Some((4, 5)));
        assert_eq!(parse_table_size(""), Some((2, 2)));
        assert_eq!(parse_table_size("7"), Some((7, 2)));
        assert_eq!(parse_table_size("99 by 999"), Some((20, 100)));
        assert_eq!(parse_table_size("wide"), None);
    }

    #[test]
    fn heading_levels() {
        assert_eq!(heading_level("## Two"), 2);
        assert_eq!(heading_level("#hashtag"), 0);
        assert_eq!(heading_level("plain"), 0);
        assert_eq!(heading_level("######"), 6);
    }

    #[test]
    fn span_edit_covers_every_change() {
        let before = Rope::from_str("abc def ghi");
        let mut after = before.clone();
        let e1 = Edit::replace(CharRange::new(0, 3), "X");
        let (o1, _) = e1.apply_to_rope(&mut after).unwrap();
        let e2 = Edit::insert(6, "YY");
        let (o2, _) = e2.apply_to_rope(&mut after).unwrap();
        let span = span_edit(before.len_chars(), &[o1, o2], &after).unwrap();
        let mut check = before.clone();
        span.apply_to_rope(&mut check).unwrap();
        assert_eq!(check, after);
        assert_eq!(span.range, CharRange::new(0, 8));
    }
}
