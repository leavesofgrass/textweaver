//! The reading queue and the review list (beta 3, B1-s3).
//!
//! **The reading queue** is a list of documents to read next, kept in
//! `library.json` beside the bookshelf ([`Library::queue`]). A document
//! joins it from the library list (Ctrl+Enter, which takes it out again) or
//! with the `queue_document` command on the open document; it leaves when
//! it is read to the end, when Delete takes it out of the queue list, or
//! when it is removed from the bookshelf. Changes go through the writer, as
//! the bookshelf's do.
//!
//! When continuous reading reaches the end of a document, `[reading]
//! after_end` decides what follows: nothing (`stop`, the default), a
//! question that names the next document (`ask`: "Next: Week 4. Enter to
//! read.", where the read key, at the end of the document, opens and reads
//! it), or the next document opens and is read (`next`). No document is
//! ever started unless the setting is `next` or the reader presses a read
//! key on the offer.
//!
//! **The review list** sits beside Continue reading: the documents read
//! before, the one read longest ago first, "Pharmacology 2, 21 days, 80
//! percent, 3 notes", from the reading statistics
//! ([`ReadingStats::review`]). It is a convenience for coming back to old
//! readings, not a study method.

use std::path::{Path, PathBuf};
use std::time::Duration;

use textweaver_core::CharPos;
use textweaver_lexicon::args;
use textweaver_store::library::resolve_path;
use textweaver_store::{AfterEnd, DocKey, Library, ReadingStats};

use crate::app::{App, ListKind};
use crate::browse::BrowseKey;
use crate::command::Effect;
use crate::list_model::ListKey;

/// How long reading the queue waits for the writer's queued bookshelf
/// changes.
const WRITER_WAIT: Duration = Duration::from_secs(1);

/// The reading queue's state in the app.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct QueueState {
    /// "Next: Week 4. Enter to read.": the document offered, and the place
    /// in the finished document where the offer was made. A read key there
    /// opens and reads it; anywhere else it reads as usual.
    pub(crate) offer: Option<(PathBuf, CharPos)>,
    /// The document the queue is opening, read once it is open.
    pub(crate) read_on_open: Option<PathBuf>,
}

impl App {
    /// The bookshelf as it will be once the writer's queued changes are
    /// written; `None` without a data folder, or when it cannot be read
    /// (said as an error).
    fn shelf(&mut self) -> Option<Library> {
        let file = self.paths.as_ref()?.library_file();
        self.writer.flush(WRITER_WAIT);
        match Library::load(&file) {
            Ok(lib) => Some(lib),
            Err(e) => {
                let msg = self.msg_args("queue-read-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
                None
            }
        }
    }

    /// Adds `path` to the reading queue or takes it out, on the writer.
    fn queue_job(&mut self, path: &Path, add: bool) {
        let Some(paths) = &self.paths else {
            return;
        };
        let job = crate::writer::Job::Queue {
            library_file: paths.library_file(),
            path: path.to_owned(),
            add,
        };
        self.writer.send(job);
    }

    /// Puts `path` in the reading queue, or takes it out when it is there,
    /// and says which: "Added to the reading queue: Week 4, 3 of 3."
    fn toggle_queued(&mut self, path: &Path) {
        let Some(mut lib) = self.shelf() else {
            return;
        };
        let title = lib.title_of(path);
        let msg = if lib.queue_remove(path) {
            self.queue_job(path, false);
            self.msg_args("queue-removed", &args!["title" => title])
        } else {
            lib.queue_add(path);
            self.queue_job(path, true);
            let n = lib.queue.len();
            self.msg_args("queue-added", &args!["title" => title, "n" => n])
        };
        self.tell(&msg);
    }

    /// `queue_document`: the open document joins the reading queue, or
    /// leaves it.
    pub(crate) fn queue_document(&mut self) -> Vec<Effect> {
        let Some(s) = self.session.as_ref() else {
            let key = self.key(textweaver_keymap::ActionId::Open);
            let msg = self.msg_args("app-no-document-open", &args!["key" => key]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        let Some(path) = s.doc.meta.path.clone().filter(|p| p.is_file()) else {
            let msg = self.msg("queue-no-file");
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        self.toggle_queued(&path);
        vec![Effect::Redraw]
    }

    /// The reading queue's keys in lists: in the library list, Ctrl+Enter
    /// puts the focused document in the queue or takes it out, and the
    /// list stays open.
    pub(crate) fn queue_list_key(&mut self, key: ListKey) -> Option<Vec<Effect>> {
        if key != ListKey::ChooseHere {
            return None;
        }
        let Some(ListKind::Library(list)) = &self.list else {
            return None;
        };
        let n = self.list_model.as_ref().map_or(0, |l| l.selected);
        let path = list.path_at(n)?.to_owned();
        self.toggle_queued(&path);
        Some(vec![Effect::Redraw])
    }

    /// `reading_queue`: the queue as a list, in order. Enter opens a
    /// document; Delete takes it out of the queue.
    pub(crate) fn open_reading_queue(&mut self) -> Vec<Effect> {
        self.show_queue(0)
    }

    fn show_queue(&mut self, focus: usize) -> Vec<Effect> {
        let Some(lib) = self.shelf() else {
            return vec![Effect::Redraw];
        };
        if lib.queue.is_empty() {
            let key = self.browse_key_named(BrowseKey::ChooseFolder);
            let msg = self.msg_args("queue-empty", &args!["key" => key]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let rows = lib
            .queue
            .iter()
            .map(|p| {
                let title = lib.title_of(p);
                if p.is_file() {
                    title
                } else {
                    self.msg_args("queue-item-missing", &args!["title" => title])
                }
            })
            .collect();
        let msg = self.msg_args("queue-intro", &args!["n" => lib.queue.len()]);
        self.tell(&msg);
        let title = self.msg("queue-title");
        self.pending_list_focus = Some(focus.min(lib.queue.len() - 1));
        self.list = Some(ListKind::Queue(lib.queue));
        vec![Effect::ShowList { title, items: rows }]
    }

    /// Delete on a row of the reading queue: takes the document out, and
    /// shows what is left.
    pub(crate) fn unqueue_row(&mut self, paths: &[PathBuf], n: usize) -> Vec<Effect> {
        let Some(path) = paths.get(n) else {
            return vec![Effect::Redraw];
        };
        let title = self
            .shelf()
            .map_or_else(|| path.display().to_string(), |l| l.title_of(path));
        self.queue_job(path, false);
        let msg = self.msg_args("queue-removed", &args!["title" => title]);
        self.tell(&msg);
        if paths.len() <= 1 {
            self.list = None;
            return vec![Effect::Redraw];
        }
        self.show_queue(n)
    }

    /// `review_list`: the documents read before, the one read longest ago
    /// first: "Pharmacology 2, 21 days, 80 percent, 3 notes". Enter opens
    /// one.
    pub(crate) fn open_review_list(&mut self) -> Vec<Effect> {
        let Some(paths) = self.paths.clone() else {
            let msg = self.msg("review-empty");
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        // The reading added since the last write is in the writer's queue.
        self.writer.flush(WRITER_WAIT);
        let stats = ReadingStats::load(&paths).unwrap_or_default();
        let states = textweaver_store::StateStore::new(paths.state_dir());
        let rows = stats.review(textweaver_store::now_ts(), &Path::is_file, &|key| {
            states
                .load(&DocKey(key.to_owned()))
                .map_or(0, |s| s.notes.len())
        });
        if rows.is_empty() {
            let id = if self.settings.stats.enabled {
                "review-empty"
            } else {
                "review-stats-off"
            };
            let msg = self.msg(id);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let items = rows
            .iter()
            .map(|r| {
                let days = self.msg_args("review-days", &args!["n" => r.days]);
                let notes = self.msg_args("review-notes", &args!["n" => r.notes]);
                self.msg_args(
                    "review-item",
                    &args![
                        "title" => r.title.as_str(),
                        "days" => days,
                        "pct" => r.percent,
                        "notes" => notes
                    ],
                )
            })
            .collect();
        let msg = self.msg_args("review-intro", &args!["n" => rows.len()]);
        self.tell(&msg);
        let title = self.msg("review-title");
        self.list = Some(ListKind::Review(rows.into_iter().map(|r| r.path).collect()));
        vec![Effect::ShowList { title, items }]
    }

    /// Continuous reading reached the end of the open document: it leaves
    /// the reading queue, and `[reading] after_end` decides what follows.
    /// Says `done` ("Done reading.") with the next document's name and
    /// returns true when the queue has one to offer or read; returns false
    /// otherwise, and the caller says `done` as usual.
    pub(crate) fn queue_after_end(&mut self, done: &str) -> bool {
        let current = self.session.as_ref().and_then(|s| s.doc.meta.path.clone());
        let Some(file) = self
            .paths
            .as_ref()
            .map(textweaver_store::Paths::library_file)
        else {
            return false;
        };
        self.writer.flush(WRITER_WAIT);
        // Reading on is no time to report a damaged bookshelf: the library
        // list does that.
        let Ok(lib) = Library::load(&file) else {
            return false;
        };
        if let Some(p) = &current
            && lib.queued(p)
        {
            self.queue_job(p, false);
        }
        let after = self.settings.reading.after_end;
        let next = match lib.queue_next(current.as_deref()) {
            Some(n) if after != AfterEnd::Stop => n.to_owned(),
            _ => return false,
        };
        let title = lib.title_of(&next);
        match after {
            AfterEnd::Ask => {
                let key = self.key(textweaver_keymap::ActionId::ReadFromCursor);
                let next_msg =
                    self.msg_args("queue-next-ask", &args!["title" => title, "key" => key]);
                let at = self.session.as_ref().map_or(CharPos::ZERO, |s| s.cursor);
                self.queue.offer = Some((next, at));
                self.tell(&format!("{done} {next_msg}"));
            }
            AfterEnd::Next | AfterEnd::Stop => {
                let next_msg = self.msg_args("queue-next-reading", &args!["title" => title]);
                self.tell(&format!("{done} {next_msg}"));
                self.queue.read_on_open = Some(resolve_path(&next));
                let _ = self.open_command(next);
            }
        }
        true
    }

    /// True when a read key now takes up the reading queue's offer: one
    /// is open, nothing is being read, and the cursor is where the
    /// finished document's reading ended.
    pub(crate) fn queue_offer_here(&self) -> bool {
        let Some((_, at)) = &self.queue.offer else {
            return false;
        };
        self.playback == crate::Playback::Idle
            && self.session.as_ref().is_some_and(|s| s.cursor == *at)
    }

    /// The read key on the offer: opens the next document and reads it.
    pub(crate) fn accept_queue_offer(&mut self) -> Vec<Effect> {
        let Some((next, _)) = self.queue.offer.take() else {
            return vec![Effect::Redraw];
        };
        self.queue.read_on_open = Some(resolve_path(&next));
        self.open_command(next)
    }

    /// A document was opened: drops the offer, and returns true when the
    /// queue opened `opened` to read it.
    pub(crate) fn queue_opened(&mut self, opened: Option<&Path>) -> bool {
        self.queue.offer = None;
        match (self.queue.read_on_open.take(), opened) {
            (Some(want), Some(p)) => want == resolve_path(p),
            _ => false,
        }
    }
}
