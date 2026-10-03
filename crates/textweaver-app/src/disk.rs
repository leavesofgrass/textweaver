//! Files changed on disk while open (the September 2026 audit, finding D2).
//!
//! The open file's modification time and size are recorded when it is
//! opened and after every save. Saving in place over a file that changed
//! since then asks first ("... changed on disk since you opened it. Save
//! over those changes? y or n"), so an edit made in Obsidian, VS Code, or
//! by `git pull` is not overwritten without a word. While the document is
//! open and has no unsaved changes, a change on disk is noticed within
//! [`DISK_CHECK_INTERVAL`] and a reload is offered (only when not reading
//! aloud and nothing else is being asked).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;

use crate::app::{App, Mode};
use crate::command::{Confirm, Effect};
use crate::edit::AfterLeave;
use crate::playback::Playback;

/// How often the open file is checked for changes on disk.
pub const DISK_CHECK_INTERVAL: Duration = Duration::from_secs(2);

/// What identifies a version of a file on disk: its modification time and
/// size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileStamp {
    /// Last modification time, where the file system reports one.
    pub modified: Option<SystemTime>,
    /// Size in bytes.
    pub len: u64,
}

impl FileStamp {
    /// The stamp of the file at `path` now, or `None` when it cannot be
    /// read (it was deleted or moved).
    pub fn of(path: &Path) -> Option<FileStamp> {
        let m = std::fs::metadata(path).ok()?;
        Some(FileStamp {
            modified: m.modified().ok(),
            len: m.len(),
        })
    }
}

/// A question about the file on disk, waiting for y or n.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DiskQuestion {
    /// Save over a file changed on disk; `leaving` saves on the way out of
    /// edit mode.
    Overwrite { leaving: Option<AfterLeave> },
    /// Reload a file changed on disk while open and unmodified; `stamp` is
    /// the version found, remembered when the answer is no.
    Reload {
        /// The file.
        path: PathBuf,
        /// Its stamp when the change was noticed.
        stamp: Option<FileStamp>,
    },
    /// Save As onto a file that already exists.
    SaveAsOver {
        /// The path the user typed.
        path: PathBuf,
        /// What the save is for.
        then: crate::edit::SaveThen,
    },
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

impl App {
    /// The path of the file being edited, when saving would write over it.
    fn edited_path(&self) -> Option<PathBuf> {
        self.edit.as_ref()?.session.doc().path.clone()
    }

    /// Asks whether to save over a file that changed on disk since it was
    /// opened or last saved (the writer found it changed and wrote
    /// nothing); `leaving` saves on the way out of edit mode.
    pub(crate) fn ask_overwrite(&mut self, leaving: Option<AfterLeave>) -> Vec<Effect> {
        let Some(path) = self.edited_path() else {
            return vec![Effect::Redraw];
        };
        self.pending_disk = Some(DiskQuestion::Overwrite { leaving });
        self.list = None;
        let question = overwrite_question(self.cat(), &path);
        self.ask(&question);
        vec![Effect::Redraw]
    }

    /// Answers a question about the file on disk.
    pub(crate) fn confirm_disk(&mut self, answer: Confirm) -> Vec<Effect> {
        let Some(question) = self.pending_disk.clone() else {
            return vec![Effect::Redraw];
        };
        match (answer, question) {
            (Confirm::Repeat, DiskQuestion::Overwrite { .. }) => {
                if let Some(p) = self.edited_path() {
                    let question = overwrite_question(self.cat(), &p);
                    self.ask(&question);
                }
                vec![Effect::Redraw]
            }
            (Confirm::Repeat, DiskQuestion::Reload { path: p, .. }) => {
                let question = reload_question(self.cat(), &p);
                self.ask(&question);
                vec![Effect::Redraw]
            }
            (Confirm::Repeat, DiskQuestion::SaveAsOver { path, .. }) => {
                let dest = textweaver_editor::autosave::save_as_path(&path);
                let question =
                    self.msg_args("disk-replace-question", &args!["name" => file_name(&dest)]);
                self.ask(&question);
                vec![Effect::Redraw]
            }
            (Confirm::Yes, DiskQuestion::SaveAsOver { path, then }) => {
                self.pending_disk = None;
                self.save_as_to(path, then, true)
            }
            (Confirm::No, DiskQuestion::SaveAsOver { path, then }) => {
                self.pending_disk = None;
                // A relative name typed next goes in the same folder.
                self.suggested_path = Some(path);
                let label = self.msg("disk-not-replaced");
                self.save_then = Some(then);
                if !self.mode.is_prompt() {
                    self.return_mode = self.mode;
                }
                self.mode = Mode::Prompt;
                self.prompt_purpose = crate::command::PromptPurpose::SaveAs;
                let said = self
                    .path_prompt_said(crate::command::PromptPurpose::SaveAs, &format!("{label}."));
                self.tell(&said);
                vec![Effect::Prompt {
                    label,
                    purpose: crate::command::PromptPurpose::SaveAs,
                }]
            }
            (Confirm::Yes, DiskQuestion::Overwrite { leaving }) => {
                self.pending_disk = None;
                self.overwrite_confirmed = true;
                match leaving {
                    Some(after) => {
                        self.leave_edit(Some(textweaver_editor::session::Choice::Save), None, after)
                    }
                    None => self.save(None),
                }
            }
            (Confirm::No, DiskQuestion::Overwrite { .. }) => {
                self.pending_disk = None;
                let save_as = self.keys(textweaver_keymap::ActionId::SaveAs);
                let msg = self.msg_args("disk-not-saved", &args!["key" => save_as]);
                self.tell(&msg);
                vec![Effect::Redraw]
            }
            (Confirm::Yes, DiskQuestion::Reload { path, .. }) => {
                self.pending_disk = None;
                self.reload(&path)
            }
            (Confirm::No, DiskQuestion::Reload { stamp, .. }) => {
                self.pending_disk = None;
                // Do not ask again about this version.
                if let (Some(s), Some(stamp)) = (self.session.as_mut(), stamp) {
                    s.disk = Some(stamp);
                }
                let msg = self.msg("disk-kept-open-version");
                self.tell(&msg);
                vec![Effect::Redraw]
            }
        }
    }

    /// Opens `path` again, keeping edit mode on when it was on.
    fn reload(&mut self, path: &Path) -> Vec<Effect> {
        let editing = self.edit.is_some();
        if editing {
            // Clean (a reload is offered only then): nothing to lose.
            if let Some(e) = self.edit.as_mut() {
                let _ = e
                    .session
                    .finish_editing(Some(textweaver_editor::session::Choice::Discard), None);
            }
            self.finish_leave(false, false);
        }
        let effects = self.dispatch_open(path);
        if editing && self.session.is_some() {
            self.enter_edit(None);
        }
        effects
    }

    /// Periodic check for a change on disk: every
    /// [`DISK_CHECK_INTERVAL`] the writer reads the open file's stamp
    /// ([`disk_checked`](Self::disk_checked) acts on it), so a slow disk or
    /// network drive never holds up a key press.
    pub(crate) fn disk_tick(&mut self, now: Instant) -> Vec<Effect> {
        if self
            .last_disk_check
            .is_some_and(|t| now.saturating_duration_since(t) < DISK_CHECK_INTERVAL)
        {
            return Vec::new();
        }
        self.last_disk_check = Some(now);
        if self.reload_busy() {
            return Vec::new();
        }
        let path = self
            .edited_path()
            .or_else(|| self.session.as_ref()?.doc.meta.path.clone());
        if let (Some(path), Some(_)) = (path, self.session.as_ref().and_then(|s| s.disk)) {
            self.request_disk_check(path);
        }
        Vec::new()
    }

    /// True when a reload may not be offered now: something is being asked
    /// or listed, a prompt is open, reading is on, or there are unsaved
    /// changes.
    fn reload_busy(&self) -> bool {
        self.confirmation_pending()
            || self.list.is_some()
            || !self.may_offer_reload()
            || self.playback == Playback::Reading
            || self.is_dirty()
    }

    /// The writer read the open file's stamp: offers a reload when it
    /// changed, the open document has no unsaved changes, nothing is being
    /// read aloud, and nothing else is being asked.
    pub(crate) fn disk_checked(&mut self, path: PathBuf, stamp: Option<FileStamp>) -> Vec<Effect> {
        if self.reload_busy() {
            return Vec::new();
        }
        let open = self
            .edited_path()
            .or_else(|| self.session.as_ref()?.doc.meta.path.clone());
        let known = self.session.as_ref().and_then(|s| s.disk);
        match (open, known, stamp) {
            (Some(open), Some(known), Some(now)) if open == path && now != known => {
                self.pending_disk = Some(DiskQuestion::Reload {
                    path: path.clone(),
                    stamp: Some(now),
                });
                let question = reload_question(self.cat(), &path);
                self.ask(&question);
                vec![Effect::Redraw]
            }
            _ => Vec::new(),
        }
    }

    /// True in Browse, Speech Cursor, and Edit mode (not while a prompt is
    /// open): where a reload may be offered.
    fn may_offer_reload(&self) -> bool {
        matches!(self.mode, Mode::Browse | Mode::Edit | Mode::SpeechCursor)
    }
}

fn overwrite_question(c: &Catalog, path: &Path) -> String {
    c.fmt("disk-overwrite-question", &args!["name" => file_name(path)])
}

fn reload_question(c: &Catalog, path: &Path) -> String {
    c.fmt("disk-reload-question", &args!["name" => file_name(path)])
}
