//! The background writer (Phase 2): every file the app writes while it
//! runs goes through one thread, so a key press never waits on the disk.
//!
//! - **Saves** (Ctrl+S, Save As, saving on the way out of edit mode): the
//!   app takes the text (a rope clone, free) and the writer writes it
//!   atomically with [`textweaver_editor::autosave::save_text`], keeping
//!   the file's BOM and line endings. When the app knows the file's
//!   version, the writer first checks the file did not change on disk; if
//!   it did, nothing is written and the app asks before overwriting.
//! - **Autosave snapshots** and their deletion, in order
//!   ([`SnapshotOp`]); the writer holds each snapshot's lock.
//! - **Reading positions, bookmarks, notes, and highlights**
//!   (`state/<key>.json`) and the library folder's sidecar. Queued saves of
//!   the same document collapse into the newest.
//! - **The check for a change on disk** every two seconds, and the
//!   bookshelf and recent-files updates when a document opens.
//! - **Reading statistics** (`stats.json`) and **settings profiles**
//!   (`profiles.toml`), Agent W3e.
//! - **Document identity** (`sync-ids.json`, ADR-0049): hashing an opened
//!   or saved file to find its sync id.
//!
//! Each job's result comes back as a [`Report`], which the app applies on
//! its next [`App::tick`](crate::App::tick): "Saved", an error, or a
//! question. Quitting waits for the writer, with a bounded wait
//! ([`Writer::flush`]).

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

use textweaver_editor::autosave::{self, SnapshotLock};
use textweaver_editor::{SaveRequest, SnapshotOp};
use textweaver_store::{
    DocKey, DocState, Library, LibrarySync, Paths, Profiles, ReadingStats, Recent, Settings,
    SettingsStore, StateStore, StatsDelta,
};

use crate::disk::FileStamp;
use crate::wake::WakeSlot;

/// What a state save was for, so its result can be announced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StateNote {
    /// A periodic or incidental save: failures are logged.
    Quiet,
    /// A bookmark was added: say it is set, or that it could not be saved.
    Bookmark {
        /// The bookmark's name.
        name: String,
        /// Its percentage.
        pct: u8,
    },
}

/// Work for the writer.
pub(crate) enum Job {
    /// Save a document's state, then mirror it into the library sidecar.
    State {
        store: StateStore,
        key: DocKey,
        state: Box<DocState>,
        sync: Option<(LibrarySync, PathBuf)>,
        note: StateNote,
    },
    /// Write or delete a recovery snapshot.
    Snapshot(SnapshotOp),
    /// Save the text being edited.
    Save {
        id: u64,
        req: SaveRequest,
        /// The file's version when opened or last saved: the save stops
        /// with [`SaveFailure::ChangedOnDisk`] when the file differs.
        expect: Option<FileStamp>,
    },
    /// Read the stamp of the open file.
    DiskCheck { path: PathBuf },
    /// Record an opened document on the bookshelf and the recent list.
    Opened {
        library_file: PathBuf,
        recent_file: PathBuf,
        path: PathBuf,
        title: String,
        format: String,
        /// Author, DOI, and ISBN from the document.
        meta: textweaver_store::library::DocMetadata,
        recent_limit: usize,
    },
    /// Find or make a document's sync id and keep its hashes in
    /// `sync-ids.json` (ADR-0049). Hashing a file reads all of it, so it is
    /// done here, never on the input thread.
    Identify {
        job: Box<textweaver_sync::Identify>,
        /// The text as read, for its hash.
        text: Option<ropey::Rope>,
    },
    /// Write the library sidecars' pending entries.
    SyncFlush(LibrarySync),
    /// Add reading to `stats.json`.
    Stats {
        paths: Paths,
        deltas: Vec<StatsDelta>,
    },
    /// Save `profiles.toml`.
    Profiles {
        paths: Paths,
        profiles: Box<Profiles>,
    },
    /// Save the settings (`settings.toml`); queued saves collapse into the
    /// newest.
    Settings {
        store: SettingsStore,
        settings: Box<Settings>,
    },
    /// Answer when everything sent before has been done.
    Barrier(Sender<()>),
    /// A disk that takes this long (tests of waiting).
    #[cfg(test)]
    Stall(Duration),
}

/// Why a save did not write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SaveFailure {
    /// The file changed on disk since the app last knew it; nothing was
    /// written.
    ChangedOnDisk,
    /// Writing failed.
    Io(String),
}

/// A result, for the app's next tick.
pub(crate) enum Report {
    /// A state save finished.
    State {
        note: StateNote,
        result: Result<(), String>,
    },
    /// A snapshot write finished (deletes are only logged).
    Snapshot {
        doc_key: String,
        result: std::io::Result<bool>,
    },
    /// A save finished: the text written and the file's new stamp, or why
    /// not.
    Saved {
        id: u64,
        req: SaveRequest,
        result: Result<(String, Option<FileStamp>), SaveFailure>,
    },
    /// The open file's stamp now (`None`: it cannot be read).
    Disk {
        path: PathBuf,
        stamp: Option<FileStamp>,
    },
    /// Saving the settings profiles failed.
    ProfilesFailed(String),
    /// A settings save finished.
    Settings { result: Result<(), String> },
}

impl Job {
    /// True for jobs whose result comes back as a [`Report`].
    fn reports(&self) -> bool {
        matches!(
            self,
            Job::State { .. }
                | Job::Snapshot(SnapshotOp::Write { .. })
                | Job::Save { .. }
                | Job::DiskCheck { .. }
                | Job::Settings { .. }
        )
    }
}

/// The handle to the writer thread.
pub(crate) struct Writer {
    tx: Option<Sender<Job>>,
    rx: Receiver<Report>,
    thread: Option<JoinHandle<()>>,
    next_id: u64,
    /// Reports of jobs done on the caller's thread (no writer thread).
    inline_reports: Vec<Report>,
    /// Jobs sent so far (barriers aside).
    sent: u64,
    /// Jobs the writer has done so far (barriers aside), counted by the
    /// writer thread.
    done: Arc<AtomicU64>,
    /// The newest state queued for each document, with the number of the
    /// job that writes it: until that job is done, this is what the state
    /// file will hold, so opening a document reads it from here instead of
    /// waiting for the disk ([`queued_state`](Self::queued_state)).
    queued_states: HashMap<(PathBuf, String), (u64, DocState)>,
}

impl std::fmt::Debug for Writer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Writer").finish_non_exhaustive()
    }
}

impl Writer {
    /// How long quitting waits for the writer before giving up.
    pub(crate) const QUIT_WAIT: Duration = Duration::from_secs(10);

    /// Starts the writer thread. It rings `wake` after each batch of jobs
    /// that produced reports (crate::wake).
    pub(crate) fn spawn(wake: WakeSlot) -> Writer {
        let (tx, jobs) = mpsc::channel::<Job>();
        let (report_tx, rx) = mpsc::channel::<Report>();
        let done = Arc::new(AtomicU64::new(0));
        let counted = Arc::clone(&done);
        let thread = std::thread::Builder::new()
            .name("textweaver-writer".into())
            .spawn(move || run(&jobs, &report_tx, &wake, &counted))
            .map_err(|e| log::error!("cannot start the writer thread: {e}"))
            .ok();
        Writer {
            tx: thread.is_some().then_some(tx),
            rx,
            thread,
            next_id: 1,
            inline_reports: Vec::new(),
            sent: 0,
            done,
            queued_states: HashMap::new(),
        }
    }

    /// The newest state queued for `key` in `store` and not yet written,
    /// if any. Opening a document reads its saved state from here first:
    /// the file on disk may still be older, and waiting for the writer on
    /// the input thread could freeze the keyboard on a slow disk (up to two
    /// seconds before Wave 6).
    pub(crate) fn queued_state(&mut self, store: &StateStore, key: &DocKey) -> Option<DocState> {
        self.forget_written_states();
        self.queued_states
            .get(&(store.dir().to_owned(), key.0.clone()))
            .map(|(_, state)| state.clone())
    }

    /// Drops queued states whose jobs the writer has done: the files hold
    /// them now.
    fn forget_written_states(&mut self) {
        let done = self.done.load(Ordering::Acquire);
        self.queued_states.retain(|_, (seq, _)| *seq > done);
    }

    /// A fresh id for a save.
    pub(crate) fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Queues a job. Without a writer thread (it could not start, or it
    /// died), the job runs here, so nothing is lost.
    pub(crate) fn send(&mut self, job: Job) {
        if !matches!(job, Job::Barrier(_)) {
            self.sent += 1;
        }
        if let Job::State {
            store, key, state, ..
        } = &job
        {
            self.forget_written_states();
            self.queued_states.insert(
                (store.dir().to_owned(), key.0.clone()),
                (self.sent, (**state).clone()),
            );
        }
        let job = match &self.tx {
            Some(tx) => match tx.send(job) {
                Ok(()) => return,
                Err(mpsc::SendError(job)) => {
                    log::error!("the writer thread stopped; writing on this thread");
                    self.tx = None;
                    job
                }
            },
            None => job,
        };
        let counts = !matches!(job, Job::Barrier(_));
        let (report_tx, report_rx) = mpsc::channel();
        let mut state = WriterState::default();
        do_job(job, &report_tx, &mut state, false);
        if counts {
            self.done.fetch_add(1, Ordering::AcqRel);
        }
        drop(report_tx);
        self.inline_reports.extend(report_rx.try_iter());
    }

    /// Reports ready now, in the order the jobs were done.
    pub(crate) fn reports(&mut self) -> Vec<Report> {
        let mut out = std::mem::take(&mut self.inline_reports);
        while let Ok(r) = self.rx.try_recv() {
            out.push(r);
        }
        out
    }

    /// A receiver that answers once every job sent so far is done, for a
    /// helper thread to wait on before it reads what they write (the
    /// library's scan reads the recent list); `None` when there is no
    /// writer thread (jobs are then done at once).
    pub(crate) fn barrier(&self) -> Option<Receiver<()>> {
        let tx = self.tx.as_ref()?;
        let (done_tx, done_rx) = mpsc::channel();
        tx.send(Job::Barrier(done_tx)).ok()?;
        Some(done_rx)
    }

    /// Waits until every job sent so far is done, for at most `timeout`.
    /// True when it finished in time.
    pub(crate) fn flush(&self, timeout: Duration) -> bool {
        let Some(tx) = &self.tx else {
            return true;
        };
        let (done_tx, done_rx) = mpsc::channel();
        if tx.send(Job::Barrier(done_tx)).is_err() {
            return true;
        }
        match done_rx.recv_timeout(timeout) {
            Ok(()) | Err(RecvTimeoutError::Disconnected) => true,
            Err(RecvTimeoutError::Timeout) => false,
        }
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        // Finish what was queued (bounded), then let the thread end.
        let finished = self.flush(Self::QUIT_WAIT);
        if !finished {
            log::error!("the writer did not finish within {:?}", Self::QUIT_WAIT);
        }
        // With the queue empty, closing the channel ends the thread at
        // once; wait for it (briefly), so the snapshot locks it holds are
        // released before this returns.
        self.tx = None;
        if let Some(t) = self.thread.take() {
            let deadline = std::time::Instant::now() + Duration::from_secs(2);
            while finished && !t.is_finished() && std::time::Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(1));
            }
            if t.is_finished() {
                let _ = t.join();
            }
        }
    }
}

/// The writer thread: takes every job waiting, collapses state saves of the
/// same document into the newest, and does the rest in order.
fn run(jobs: &Receiver<Job>, reports: &Sender<Report>, wake: &WakeSlot, done: &AtomicU64) {
    let mut state = WriterState::default();
    while let Ok(first) = jobs.recv() {
        let mut batch = vec![first];
        batch.extend(jobs.try_iter());
        let reported = batch.iter().any(Job::reports);
        // Settings saves queued together: only the newest is written.
        let last_settings = batch
            .iter()
            .rposition(|j| matches!(j, Job::Settings { .. }));
        // A state save followed by a newer one for the same file is
        // skipped (its report still goes out, with the newer result's
        // file on disk).
        let mut later: HashMap<(PathBuf, String), usize> = HashMap::new();
        for (i, job) in batch.iter().enumerate() {
            if let Job::State { store, key, .. } = job {
                later.insert((store.dir().to_owned(), key.0.clone()), i);
            }
        }
        for (i, job) in batch.into_iter().enumerate() {
            let superseded = match &job {
                Job::State { store, key, .. } => later
                    .get(&(store.dir().to_owned(), key.0.clone()))
                    .is_some_and(|&last| last != i),
                Job::Settings { .. } => last_settings != Some(i),
                _ => false,
            };
            let counts = !matches!(job, Job::Barrier(_));
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                do_job(job, reports, &mut state, superseded);
            }));
            if outcome.is_err() {
                log::error!("a write failed with an internal error; the writer goes on");
            }
            // Jobs are done in the order sent, so once the count reaches a
            // state job's number its state is on disk. A superseded job is
            // counted too; the app only keeps the newest state per
            // document, whose number is later.
            if counts {
                done.fetch_add(1, Ordering::AcqRel);
            }
        }
        if reported {
            wake.wake();
        }
    }
}

/// What the writer thread keeps between jobs.
#[derive(Default)]
struct WriterState {
    /// Snapshot locks held while their documents are being edited.
    locks: HashMap<PathBuf, SnapshotLock>,
    /// The stamp each file had after this writer last saved it: a second
    /// save queued before the app heard of the first is not a change made
    /// by another program.
    wrote: HashMap<PathBuf, FileStamp>,
}

/// Does one job and sends its report.
fn do_job(job: Job, reports: &Sender<Report>, state: &mut WriterState, superseded: bool) {
    let report = match job {
        Job::State {
            store,
            key,
            state,
            sync,
            note,
        } => {
            let result = if superseded {
                Ok(())
            } else {
                store.save(&key, &state).map_err(|e| e.to_string())
            };
            if result.is_ok()
                && !superseded
                && let Some((sync, path)) = sync
                && let Err(e) = sync.record(&path, &state, None)
            {
                log::warn!("cannot sync the reading position: {e}");
            }
            Some(Report::State { note, result })
        }
        Job::Snapshot(SnapshotOp::Write { dir, snapshot }) => {
            let file = autosave::snapshot_file(&dir, &snapshot.doc_key);
            let result = write_snapshot(&dir, &file, &snapshot, &mut state.locks);
            Some(Report::Snapshot {
                doc_key: snapshot.doc_key,
                result,
            })
        }
        Job::Snapshot(SnapshotOp::Delete { dir, doc_key }) => {
            let file = autosave::snapshot_file(&dir, &doc_key);
            if let Err(e) = autosave::delete_snapshot(&file) {
                log::warn!("cannot delete the recovery snapshot: {e}");
            }
            state.locks.remove(&file);
            None
        }
        Job::Save { id, req, expect } => {
            let result = save(&req, expect, &mut state.wrote);
            Some(Report::Saved { id, req, result })
        }
        Job::DiskCheck { path } => Some(Report::Disk {
            stamp: FileStamp::of(&path),
            path,
        }),
        Job::Opened {
            library_file,
            recent_file,
            path,
            title,
            format,
            meta,
            recent_limit,
        } => {
            record_open(
                &library_file,
                &recent_file,
                &path,
                (&title, &format, &meta),
                recent_limit,
            );
            None
        }
        Job::Identify { job, text } => {
            job.run_logged(text.as_ref().map(ropey::Rope::chunks));
            None
        }
        Job::SyncFlush(sync) => {
            match sync.flush() {
                Ok(conflicts) if !conflicts.is_empty() => {
                    log::info!("{} sidecar entries differed on write", conflicts.len());
                }
                Ok(_) => {}
                Err(e) => log::warn!("cannot write the library sidecar: {e}"),
            }
            None
        }
        Job::Stats { paths, deltas } => {
            if let Err(e) = ReadingStats::add_to_file(&paths, &deltas) {
                log::warn!("cannot save reading statistics: {e}");
            }
            None
        }
        Job::Profiles { paths, profiles } => profiles
            .save(&paths)
            .err()
            .map(|e| Report::ProfilesFailed(e.to_string())),
        Job::Settings { store, settings } => {
            let result = if superseded {
                Ok(())
            } else {
                store.save(&settings).map_err(|e| e.to_string())
            };
            Some(Report::Settings { result })
        }
        Job::Barrier(done) => {
            let _ = done.send(());
            None
        }
        #[cfg(test)]
        Job::Stall(d) => {
            std::thread::sleep(d);
            None
        }
    };
    if let Some(r) = report {
        let _ = reports.send(r);
    }
}

fn write_snapshot(
    dir: &Path,
    file: &Path,
    snapshot: &textweaver_editor::PendingSnapshot,
    locks: &mut HashMap<PathBuf, SnapshotLock>,
) -> std::io::Result<bool> {
    if !locks.contains_key(file) {
        match SnapshotLock::acquire(dir, &snapshot.doc_key)? {
            Some(lock) => {
                locks.insert(file.to_owned(), lock);
            }
            // Another instance is editing this document and owns its
            // snapshot; do not overwrite it.
            None => return Ok(false),
        }
    }
    autosave::write_snapshot(dir, &snapshot.to_snapshot())?;
    Ok(true)
}

fn save(
    req: &SaveRequest,
    expect: Option<FileStamp>,
    wrote: &mut HashMap<PathBuf, FileStamp>,
) -> Result<(String, Option<FileStamp>), SaveFailure> {
    if let Some(known) = expect
        && let Some(now) = FileStamp::of(&req.dest)
        && now != known
        && wrote.get(&req.dest) != Some(&now)
    {
        return Err(SaveFailure::ChangedOnDisk);
    }
    let text = req.text.to_string();
    autosave::save_text(&req.dest, &text)
        .map_err(|e| SaveFailure::Io(format!("{}: {e}", req.dest.display())))?;
    let stamp = FileStamp::of(&req.dest);
    if let Some(st) = stamp {
        wrote.insert(req.dest.clone(), st);
    }
    Ok((text, stamp))
}

fn record_open(
    library_file: &Path,
    recent_file: &Path,
    path: &Path,
    (title, format, meta): (&str, &str, &textweaver_store::library::DocMetadata),
    recent_limit: usize,
) {
    let mut recent = Recent::load(recent_file);
    recent.touch(path, Some(title.to_owned()), recent_limit);
    if let Err(e) = recent.save(recent_file) {
        log::warn!("cannot save recent files: {e}");
    }
    match Library::load(library_file) {
        Ok(mut lib) => {
            lib.record_open(path, title, format);
            lib.record_metadata(path, meta);
            if let Err(e) = lib.save(library_file) {
                log::warn!("cannot save the library: {e}");
            }
        }
        // An unreadable library is left alone rather than overwritten.
        Err(e) => log::warn!("cannot read the library: {e}"),
    }
}

/// Measures how long `f` took (for the writer's own tests).
#[cfg(test)]
fn timed(f: impl FnOnce()) -> Duration {
    let t = std::time::Instant::now();
    f();
    t.elapsed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_saves_collapse_and_barriers_wait() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::new(dir.path().to_owned());
        let key = DocKey("doc".into());
        let mut w = Writer::spawn(WakeSlot::default());
        for pos in 0..50 {
            let state = DocState {
                position: textweaver_core::CharPos(pos),
                ..DocState::default()
            };
            w.send(Job::State {
                store: store.clone(),
                key: key.clone(),
                state: Box::new(state),
                sync: None,
                note: StateNote::Quiet,
            });
        }
        assert!(w.flush(Duration::from_secs(10)));
        let reports = w.reports();
        assert_eq!(reports.len(), 50, "every save reports");
        assert!(
            reports
                .iter()
                .all(|r| matches!(r, Report::State { result: Ok(()), .. }))
        );
        assert_eq!(
            store.load(&key).unwrap().position,
            textweaver_core::CharPos(49)
        );
        assert!(store.writes() <= 50);
    }

    /// A state queued behind a slow disk is read from the writer, not
    /// waited for; once written, the file is the source again.
    #[test]
    fn queued_states_are_read_without_waiting() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::new(dir.path().to_owned());
        let key = DocKey("doc".into());
        let mut w = Writer::spawn(WakeSlot::default());
        w.send(Job::Stall(Duration::from_millis(300)));
        let state = DocState {
            position: textweaver_core::CharPos(42),
            ..DocState::default()
        };
        w.send(Job::State {
            store: store.clone(),
            key: key.clone(),
            state: Box::new(state),
            sync: None,
            note: StateNote::Quiet,
        });
        let took = timed(|| {
            let queued = w.queued_state(&store, &key).expect("queued");
            assert_eq!(queued.position, textweaver_core::CharPos(42));
        });
        assert!(took < Duration::from_millis(250), "{took:?}");
        assert!(store.load(&key).is_none(), "not on disk yet");
        assert!(w.flush(Duration::from_secs(10)));
        assert!(w.queued_state(&store, &key).is_none());
        assert_eq!(
            store.load(&key).unwrap().position,
            textweaver_core::CharPos(42)
        );
        let other = DocKey("other".into());
        assert!(w.queued_state(&store, &other).is_none());
    }

    #[test]
    fn a_save_refuses_a_file_changed_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.md");
        std::fs::write(&path, "one").unwrap();
        let known = FileStamp::of(&path);
        std::fs::write(&path, "changed elsewhere, longer").unwrap();
        let mut s = textweaver_editor::EditSession::new(
            textweaver_editor::DocInfo {
                key: "a".into(),
                path: Some(path.clone()),
                loader_id: "markdown".into(),
                title: "a".into(),
            },
            "one",
        );
        s.enter_edit();
        s.editor_mut().unwrap().type_text("x").unwrap();
        let textweaver_editor::SaveStart::Write(req) = s.begin_save(None).unwrap() else {
            panic!("in place");
        };
        let mut wrote = HashMap::new();
        assert_eq!(
            save(&req, known, &mut wrote),
            Err(SaveFailure::ChangedOnDisk)
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "changed elsewhere, longer"
        );
        let (text, stamp) = save(&req, None, &mut wrote).unwrap();
        assert_eq!(text, "xone");
        assert_eq!(stamp, FileStamp::of(&path));
        // A second save queued before the app heard of the first still
        // expects the old version: its own change is not a conflict.
        std::fs::write(&path, "x").unwrap();
        let theirs = FileStamp::of(&path);
        let mut wrote = HashMap::new();
        save(&req, theirs, &mut wrote).unwrap();
        assert!(save(&req, theirs, &mut wrote).is_ok());
    }

    #[test]
    fn sending_is_quick_whatever_the_size() {
        let dir = tempfile::tempdir().unwrap();
        let mut w = Writer::spawn(WakeSlot::default());
        let text = ropey::Rope::from_str(&"word ".repeat(2_000_000));
        let mut s = textweaver_editor::EditSession::new(
            textweaver_editor::DocInfo {
                key: "big".into(),
                path: Some(dir.path().join("big.md")),
                loader_id: "markdown".into(),
                title: "big".into(),
            },
            text.to_string(),
        );
        s.enter_edit();
        let textweaver_editor::SaveStart::Write(req) = s.begin_save(None).unwrap() else {
            panic!("in place");
        };
        let took = timed(|| {
            w.send(Job::Save {
                id: 1,
                req,
                expect: None,
            });
        });
        // Handing over is a rope clone and a channel send: no copy, no
        // disk. (Generous: a loaded machine can pause any thread.)
        assert!(took < Duration::from_millis(500), "{took:?}");
        assert!(w.flush(Duration::from_secs(30)));
        assert!(matches!(
            w.reports().as_slice(),
            [Report::Saved {
                id: 1,
                result: Ok(_),
                ..
            }]
        ));
        assert_eq!(
            std::fs::metadata(dir.path().join("big.md")).unwrap().len(),
            10_000_000
        );
    }
}
