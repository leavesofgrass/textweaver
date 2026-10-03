//! The app's side of the background writer ([`crate::writer`]): what it
//! queues, and what it does with each result on its next tick.
//!
//! Nothing here waits on the disk, except where waiting is the point:
//! quitting (bounded by [`Writer::QUIT_WAIT`], with an announcement when it
//! takes a moment), leaving edit mode to quit, and opening a document
//! (its saved state must be on disk before it is read back).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use textweaver_a11y::{Priority, Verbosity};
use textweaver_editor::{SaveOutcome, SaveStart};
use textweaver_lexicon::args;
use textweaver_speech::Earcon;

use crate::app::App;
use crate::command::Effect;
use crate::disk::FileStamp;
use crate::edit::{AfterLeave, SaveThen};
use crate::text_util;
use crate::writer::{Job, Report, SaveFailure, StateNote, Writer};

/// How long quitting waits quietly before saying that it is still writing.
const QUIET_WAIT: Duration = Duration::from_millis(300);

impl App {
    /// Applies what the background writer has finished: announces saves
    /// and failures, asks before overwriting a file that changed on disk,
    /// and offers to reload one. [`tick`](Self::tick) calls this; frontends
    /// that do not tick call it from their loop.
    pub fn poll_writes(&mut self) -> Vec<Effect> {
        self.entry(|app| {
            app.send_snapshot_ops();
            let mut effects = Vec::new();
            for report in app.writer.reports() {
                effects.extend(app.apply_report(report));
            }
            effects
        })
    }

    /// Waits until every file queued so far is written (at most ten
    /// seconds), then applies the results as [`poll_writes`](Self::poll_writes)
    /// does. For tests, and for callers that need the disk to be current.
    pub fn wait_for_writes(&mut self) -> Vec<Effect> {
        self.entry(Self::settle_writes)
    }

    /// [`wait_for_writes`](Self::wait_for_writes)'s work.
    fn settle_writes(&mut self) -> Vec<Effect> {
        let mut effects = Vec::new();
        // A result can queue more work (a save that leaves edit mode saves
        // the position): a few rounds settle it.
        for _ in 0..4 {
            self.send_snapshot_ops();
            if !self.writer.flush(Writer::QUIT_WAIT) {
                log::warn!("the writer did not finish in time");
            }
            let reports = self.writer.reports();
            if reports.is_empty() {
                break;
            }
            for report in reports {
                effects.extend(self.apply_report(report));
            }
        }
        effects
    }

    /// Waits up to `timeout` until every file queued so far is written,
    /// without applying the results: the next [`tick`](Self::tick) does,
    /// as in the event loop. True when the writer finished in time. For
    /// frontends' tests that drive their own loop.
    pub fn flush_writes(&mut self, timeout: Duration) -> bool {
        self.send_snapshot_ops();
        self.writer.flush(timeout)
    }

    /// Before the app ends: waits for the writer, saying so when it takes
    /// more than a moment, and reports what could not be written.
    pub(crate) fn finish_writes(&mut self) {
        self.send_snapshot_ops();
        let started = Instant::now();
        if !self.writer.flush(QUIET_WAIT) {
            let msg = self.msg("writes-still-saving");
            self.tell(&msg);
            let left = Writer::QUIT_WAIT.saturating_sub(started.elapsed());
            if !self.writer.flush(left) {
                let msg = self.msg("writes-not-written-in-time");
                self.error(&msg);
            }
        }
        for report in self.writer.reports() {
            let _ = self.apply_report(report);
        }
    }

    /// Hands the edit session's queued snapshot writes and deletes to the
    /// writer, in order.
    pub(crate) fn send_snapshot_ops(&mut self) {
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        for op in edit.session.take_snapshot_ops() {
            self.writer.send(Job::Snapshot(op));
        }
    }

    /// Queues a save of the open document's state: reading position,
    /// history, bookmarks, notes, and highlights (not while editing, when
    /// positions are in the source text; leaving edit mode saves them).
    /// Returns false when there is nothing to save or nowhere to save it.
    pub(crate) fn save_state(&mut self, note: StateNote) -> bool {
        let Some(store) = self.state_store() else {
            return false;
        };
        let Some(state) = self.state_now() else {
            return false;
        };
        let sync = self.library_sync.clone();
        // With sync on, places go to the sync folder, and a library folder's
        // old progress file is only read (ADR-0049, "The old sidecar"), so
        // Star-style folders are still honored but not written twice.
        let sidecar = !(self.sync_enabled() && self.settings.sync.places);
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let key = s.key.clone();
        let path = s.doc.meta.path.clone().filter(|_| sidecar);
        let pos = state.position;
        s.saved = state.clone();
        self.last_position_save = Some((Instant::now(), pos));
        self.writer.send(Job::State {
            store,
            key,
            state: Box::new(state),
            sync: path.map(|p| (sync, p)),
            note,
        });
        // Published to the sync folder soon (crate::sync).
        self.sync_mark_changed();
        true
    }

    /// The open document's state as a save writes it now: reading position,
    /// history, bookmarks, notes, and highlights. `None` while editing
    /// (positions are in the source text then) or with no document.
    pub(crate) fn state_now(&self) -> Option<textweaver_store::DocState> {
        if self.edit.is_some() {
            return None;
        }
        let pos = self.reading_position()?;
        let s = self.session.as_ref()?;
        let pos = text_util::word_start(&s.doc, pos);
        let mut state = s.saved.clone();
        state.position = pos;
        state.pct = text_util::percent(&s.doc, pos);
        state.ts = textweaver_store::now_ts();
        state.anchor = Some(text_util::anchor_at(&s.doc, pos));
        state.history = s.history.entries().to_vec();
        state.bookmarks = s.bookmarks.clone();
        // Bookmarks move with edits: their anchors follow the text now.
        for b in &mut state.bookmarks {
            b.anchor = Some(text_util::anchor_at(&s.doc, b.pos));
        }
        state.notes = s.notes.clone();
        state.highlights = s.highlights.clone();
        // What the text was, so positions can be found again if the file
        // changes outside textweaver (crate::relocate).
        state.text = s.text_stamp.clone();
        Some(state)
    }

    /// Starts saving the text being edited on the writer; `then` says what
    /// follows once it is written. Saving on the way out to quit waits for
    /// it here, so the quit happens in this call.
    pub(crate) fn start_save(&mut self, save_as: Option<PathBuf>, then: SaveThen) -> Vec<Effect> {
        let force = std::mem::take(&mut self.overwrite_confirmed);
        // In place, the writer checks the file is still the version this
        // app knows before writing over it.
        let expect = if save_as.is_none() && !force {
            self.session.as_ref().and_then(|s| s.disk)
        } else {
            None
        };
        let Some(edit) = self.edit.as_mut() else {
            return vec![Effect::Redraw];
        };
        match edit.session.begin_save(save_as.as_deref()) {
            Ok(SaveStart::Write(req)) => {
                let id = self.writer.next_id();
                let quitting = then == SaveThen::Leave(AfterLeave::Quit);
                self.pending_saves.push((id, then));
                let dest = req.dest.clone();
                self.writer.send(Job::Save { id, req, expect });
                // After the save, in order on the writer: the file's new
                // hash for sync; its id stays (ADR-0049).
                self.sync_document_saved(&dest);
                if quitting {
                    let effects = self.wait_for_writes();
                    if !effects.is_empty() {
                        return effects;
                    }
                }
                vec![Effect::Redraw]
            }
            Ok(SaveStart::NeedsPath { suggested }) => self.ask_save_path(suggested, then),
            Err(e) => {
                let msg = self.msg_args("writes-save-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
                vec![Effect::Redraw]
            }
        }
    }

    /// Asks the writer for the open file's stamp (the change-on-disk check).
    pub(crate) fn request_disk_check(&mut self, path: PathBuf) {
        if !self.disk_check_pending {
            self.disk_check_pending = true;
            self.writer.send(Job::DiskCheck { path });
        }
    }

    fn apply_report(&mut self, report: Report) -> Vec<Effect> {
        match report {
            Report::State { note, result } => {
                self.state_saved(note, result);
                Vec::new()
            }
            Report::Snapshot { doc_key, result } => {
                self.snapshot_saved(&doc_key, &result);
                Vec::new()
            }
            Report::Saved { id, req, result } => self.text_saved(id, req, result),
            Report::Disk { path, stamp } => {
                self.disk_check_pending = false;
                self.disk_checked(path, stamp)
            }
            Report::ProfilesFailed(e) => {
                let m = self.msg_args(
                    "profiles-save-failed",
                    &textweaver_lexicon::args!["error" => e],
                );
                self.error(&m);
                Vec::new()
            }
            Report::Settings { result } => {
                match result {
                    Err(e) => {
                        let msg =
                            self.msg_args("settings-save-failed", &args!["error" => e.to_string()]);
                        self.error(&msg);
                    }
                    Ok(true) if !self.settings_outside_said => {
                        self.settings_outside_said = true;
                        let msg = self.msg("settings-outside-kept");
                        self.tell(&msg);
                    }
                    Ok(_) => {}
                }
                vec![Effect::Redraw]
            }
            Report::Sync(response) => self.sync_response(response),
            Report::Sidecar(result) => {
                self.sidecar_reported(result);
                vec![Effect::Redraw]
            }
            Report::DetailsFailed(e) => {
                let msg = self.msg_args("details-save-failed", &args!["error" => e]);
                self.error(&msg);
                vec![Effect::Redraw]
            }
        }
    }

    fn state_saved(&mut self, note: StateNote, result: Result<(), String>) {
        match (note, result) {
            (StateNote::Bookmark { name, pct }, Ok(())) => {
                let msg =
                    self.msg_args("writes-bookmark-set", &args!["name" => name, "pct" => pct]);
                self.tell(&msg);
            }
            (StateNote::Bookmark { name, .. }, Err(e)) => {
                // Kept for this session and saved again with the position;
                // the user must not believe it is safe on disk.
                log::warn!("cannot save bookmarks: {e}");
                self.speech.earcon(Earcon::Error);
                let msg = self.msg_args(
                    "writes-bookmark-not-saved",
                    &args!["name" => name, "error" => e],
                );
                self.error(&msg);
            }
            (_, Err(e)) => log::warn!("cannot save the reading position: {e}"),
            (_, Ok(())) => {}
        }
    }

    fn snapshot_saved(&mut self, doc_key: &str, result: &std::io::Result<bool>) {
        let Some(edit) = self
            .edit
            .as_mut()
            .filter(|e| e.session.doc().key == doc_key)
        else {
            return;
        };
        edit.session.snapshot_written(result);
        match result {
            Err(e) => {
                let failures = edit.session.snapshot_failures();
                log::warn!("autosave failed ({failures} in a row): {e}");
                // Said once per run of failures; the session backs off
                // between attempts, and the log keeps every one.
                if failures == 1 {
                    let msg = self.msg_args(
                        "writes-recovery-copy-failed",
                        &args!["error" => e.to_string()],
                    );
                    self.say_at(&msg, Verbosity::Low, Priority::Polite);
                }
                self.snapshot_trouble = true;
            }
            Ok(true) if std::mem::take(&mut self.snapshot_trouble) => {
                let msg = self.msg("writes-recovery-copy-resumed");
                self.note(&msg);
            }
            Ok(_) => {}
        }
    }

    fn text_saved(
        &mut self,
        id: u64,
        req: textweaver_editor::SaveRequest,
        result: Result<(String, Option<FileStamp>), SaveFailure>,
    ) -> Vec<Effect> {
        let Some(i) = self.pending_saves.iter().position(|(p, _)| *p == id) else {
            return Vec::new();
        };
        let (_, then) = self.pending_saves.remove(i);
        if self.edit.is_none() {
            // Edit mode ended another way while the file was written.
            return Vec::new();
        }
        match result {
            Ok((text, stamp)) => {
                let Some(edit) = self.edit.as_mut() else {
                    return Vec::new();
                };
                let SaveOutcome::Saved { path, adopted } = edit.session.finish_save(req, text)
                else {
                    return Vec::new();
                };
                self.after_save(&path, adopted);
                if let Some(s) = self.session.as_mut() {
                    s.disk = stamp;
                }
                self.send_snapshot_ops();
                match then {
                    SaveThen::Stay => {
                        let name = path.file_name().map_or_else(
                            || path.display().to_string(),
                            |n| n.to_string_lossy().into(),
                        );
                        let msg = self.msg_args("writes-saved", &args!["name" => name]);
                        self.tell(&msg);
                        // The misspelling count and the preview (Agent P2b).
                        self.on_saved();
                        vec![Effect::Redraw]
                    }
                    // Saved on the way out: leave now (asking again if
                    // more was typed while the file was written).
                    SaveThen::Leave(after) => self.leave_edit(None, None, after),
                }
            }
            Err(SaveFailure::ChangedOnDisk) => {
                let leaving = match then {
                    SaveThen::Stay => None,
                    SaveThen::Leave(after) => Some(after),
                };
                self.ask_overwrite(leaving)
            }
            Err(SaveFailure::Io(e)) => {
                self.speech.earcon(Earcon::Error);
                let msg = self.msg_args("writes-save-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
                vec![Effect::Redraw]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use textweaver_a11y::{Announcer, Priority};

    use super::*;
    use crate::app::AppConfig;

    #[derive(Clone, Default)]
    struct Said(Arc<Mutex<Vec<String>>>);

    impl Announcer for Said {
        fn announce(&mut self, text: &str, _priority: Priority) {
            self.0.lock().unwrap().push(text.to_owned());
        }
    }

    #[test]
    fn quitting_waits_for_a_slow_disk_and_says_so() {
        let said = Said::default();
        let mut app = App::new(AppConfig {
            announcer: Box::new(said.clone()),
            ..AppConfig::for_tests()
        });
        app.writer
            .send(Job::Stall(QUIET_WAIT + Duration::from_millis(400)));
        let t = Instant::now();
        app.shutdown();
        assert!(t.elapsed() >= QUIET_WAIT + Duration::from_millis(400));
        let all = said.0.lock().unwrap().clone();
        assert!(
            all.iter().any(|s| s == "Still saving. Please wait."),
            "{all:?}"
        );
        // A quick quit says nothing about saving.
        let said = Said::default();
        let mut app = App::new(AppConfig {
            announcer: Box::new(said.clone()),
            ..AppConfig::for_tests()
        });
        app.shutdown();
        assert!(said.0.lock().unwrap().is_empty());
    }
}
