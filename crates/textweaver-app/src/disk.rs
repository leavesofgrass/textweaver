//! Files changed on disk while open (docs/audit-2026-09.md, finding D2).
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
    /// Reload a file changed on disk while open and unmodified.
    Reload(PathBuf),
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

    /// Records the open file's stamp as the version the app knows.
    pub(crate) fn remember_disk_state(&mut self) {
        let path = self
            .edited_path()
            .or_else(|| self.session.as_ref()?.doc.meta.path.clone());
        let stamp = path.as_deref().and_then(FileStamp::of);
        if let Some(s) = self.session.as_mut() {
            s.disk = stamp;
        }
    }

    /// True when the file about to be saved over changed on disk since it
    /// was opened or last saved.
    fn changed_on_disk(&self) -> Option<PathBuf> {
        let path = self.edited_path()?;
        let known = self.session.as_ref()?.disk?;
        let now = FileStamp::of(&path)?;
        (now != known).then_some(path)
    }

    /// Before saving in place: when the file changed on disk, asks whether
    /// to save over it and returns the effects of asking. `None` means go
    /// ahead (or the user already said yes).
    pub(crate) fn check_overwrite(&mut self, leaving: Option<AfterLeave>) -> Option<Vec<Effect>> {
        if std::mem::take(&mut self.overwrite_confirmed) {
            return None;
        }
        let path = self.changed_on_disk()?;
        self.pending_disk = Some(DiskQuestion::Overwrite { leaving });
        self.list = None;
        self.tell(&overwrite_question(&path));
        Some(vec![Effect::Redraw])
    }

    /// Answers a question about the file on disk.
    pub(crate) fn confirm_disk(&mut self, answer: Confirm) -> Vec<Effect> {
        let Some(question) = self.pending_disk.clone() else {
            return vec![Effect::Redraw];
        };
        match (answer, question) {
            (Confirm::Repeat, DiskQuestion::Overwrite { .. }) => {
                if let Some(p) = self.edited_path() {
                    self.tell(&overwrite_question(&p));
                }
                vec![Effect::Redraw]
            }
            (Confirm::Repeat, DiskQuestion::Reload(p)) => {
                self.tell(&reload_question(&p));
                vec![Effect::Redraw]
            }
            (Confirm::Repeat, DiskQuestion::SaveAsOver { path, .. }) => {
                let dest = textweaver_editor::autosave::save_as_path(&path);
                self.tell(&format!(
                    "{} already exists. Replace it? y or n.",
                    file_name(&dest)
                ));
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
                let label = "Not replaced. Type another name";
                self.save_then = Some(then);
                if !self.mode.is_prompt() {
                    self.return_mode = self.mode;
                }
                self.mode = Mode::Prompt;
                self.prompt_purpose = crate::command::PromptPurpose::SaveAs;
                self.tell(&format!("{label}."));
                vec![Effect::Prompt {
                    label: label.to_owned(),
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
                let save_as =
                    crate::help::chords_text(&self.keymap, textweaver_keymap::ActionId::SaveAs);
                self.tell(&format!(
                    "Not saved. Still editing. Save As, {save_as}, keeps both versions."
                ));
                vec![Effect::Redraw]
            }
            (Confirm::Yes, DiskQuestion::Reload(path)) => {
                self.pending_disk = None;
                self.reload(&path)
            }
            (Confirm::No, DiskQuestion::Reload(_)) => {
                self.pending_disk = None;
                // Do not ask again about this version.
                self.remember_disk_state();
                self.tell("Kept the open version.");
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

    /// Periodic check for a change on disk; offers a reload when the open
    /// document has no unsaved changes, nothing is being read aloud, and
    /// nothing else is being asked.
    pub(crate) fn disk_tick(&mut self, now: Instant) -> Vec<Effect> {
        if self
            .last_disk_check
            .is_some_and(|t| now.saturating_duration_since(t) < DISK_CHECK_INTERVAL)
        {
            return Vec::new();
        }
        self.last_disk_check = Some(now);
        let busy = self.confirmation_pending()
            || self.list.is_some()
            || !self.may_offer_reload()
            || self.playback == Playback::Reading
            || self.is_dirty();
        if busy {
            return Vec::new();
        }
        let path = self
            .edited_path()
            .or_else(|| self.session.as_ref()?.doc.meta.path.clone());
        let (Some(path), Some(known)) = (path, self.session.as_ref().and_then(|s| s.disk)) else {
            return Vec::new();
        };
        match FileStamp::of(&path) {
            Some(now) if now != known => {
                self.pending_disk = Some(DiskQuestion::Reload(path.clone()));
                self.error(&reload_question(&path));
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

fn overwrite_question(path: &Path) -> String {
    format!(
        "{} changed on disk since you opened it. Save over those changes? y or n.",
        file_name(path)
    )
}

fn reload_question(path: &Path) -> String {
    format!("{} changed on disk. Reload it? y or n.", file_name(path))
}
