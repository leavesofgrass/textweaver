//! The library (`open_library`, Alt+L in the terminal): documents in the
//! library folders and recently opened files, as one list; Enter opens one.
//!
//! Opening any document records it on the bookshelf (`library.json`,
//! [`Library::record_open`]) and the recent list. Documents inside a library
//! folder also sync their reading position through the folder's sidecar
//! (`<folder>/.textweaver/progress.json`, [`LibrarySync`]): the position is
//! mirrored there whenever it is saved, the sidecars are flushed on
//! document switch and quit, and on open the local and synced positions are
//! chosen between by `reading.sync_conflict_policy` (C2's rule: newest,
//! highest progress, or manual, which keeps this device's position and
//! says that another device differs).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use textweaver_store::library::{self, LibraryItem, ResumeSource};
use textweaver_store::sync::Resolution;
use textweaver_store::{DocKey, DocState, Library, LibrarySync, Recent};

use crate::app::{App, ListKind};
use crate::command::Effect;

/// Where a document resumes, and how to say so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResumePoint {
    /// Char offset to resume at.
    pub(crate) pos: textweaver_core::CharPos,
    /// The position came from another device (the folder's sidecar).
    pub(crate) synced: bool,
    /// Another device's position differs and nothing was chosen (the
    /// `manual` policy): this device's is used.
    pub(crate) unresolved: bool,
}

impl App {
    /// A library sync for the configured folders and conflict policy.
    pub(crate) fn make_library_sync(settings: &textweaver_store::Settings) -> LibrarySync {
        LibrarySync::new(
            &settings.library.folders,
            settings.reading.sync_conflict_policy,
        )
    }

    /// Where `path` should resume: the local saved position or the one
    /// synced through its library folder, by the conflict policy.
    pub(crate) fn resume_point(
        &self,
        path: &Path,
        local: Option<&DocState>,
    ) -> Option<ResumePoint> {
        let local_pos = local
            .map(|s| s.position)
            .filter(|p| *p > textweaver_core::CharPos::ZERO);
        match self.library_sync.resume(path, local) {
            Some(r) => Some(ResumePoint {
                pos: match r.source {
                    ResumeSource::Sidecar => r.pos,
                    // A local state saved without a timestamp still counts.
                    ResumeSource::Local => local_pos.unwrap_or(r.pos),
                },
                synced: r.source == ResumeSource::Sidecar,
                unresolved: r
                    .conflict
                    .is_some_and(|c| c.resolution == Resolution::Unresolved),
            }),
            None => local_pos.map(|pos| ResumePoint {
                pos,
                synced: false,
                unresolved: false,
            }),
        }
    }

    /// Records an opened document on the bookshelf (`library.json`) and the
    /// recent list, on the background writer.
    pub(crate) fn record_library_open(&mut self, path: &Path, title: &str, format: &str) {
        let Some(paths) = &self.paths else {
            return;
        };
        let job = crate::writer::Job::Opened {
            library_file: paths.library_file(),
            recent_file: paths.recent_file(),
            path: path.to_owned(),
            title: title.to_owned(),
            format: format.to_owned(),
            recent_limit: self.settings.library.recent_limit,
        };
        self.writer.send(job);
    }

    /// Writes pending sidecars (document switch and quit), on the writer.
    pub(crate) fn flush_library_sync(&mut self) {
        let sync = self.library_sync.clone();
        self.writer.send(crate::writer::Job::SyncFlush(sync));
    }

    /// The library's documents: every document in the library folders,
    /// then recently opened ones, newest first. Scans the folders here,
    /// which can take a while (up to 20,000 files); the Library command
    /// scans on a background thread instead.
    pub fn library_items(&self) -> Vec<LibraryItem> {
        self.library_inputs().items(&|_| {})
    }

    /// True while the library is being scanned for the Library command.
    pub fn library_scanning(&self) -> bool {
        self.library_scan.is_some()
    }

    /// What a library scan needs, to run on another thread.
    fn library_inputs(&self) -> LibraryInputs {
        LibraryInputs {
            folders: self.settings.library.folders.clone(),
            extensions: self.registry.extensions(),
            files: self
                .paths
                .as_ref()
                .map(|p| (p.library_file(), p.recent_file(), p.state_dir())),
            sync: self.library_sync.clone(),
        }
    }

    /// Shows the library list (`open_library`). The folders are scanned on
    /// a background thread (Phase 2), with the count found shown as it
    /// goes; the list opens when the scan is done ([`library_tick`](Self::library_tick)).
    pub(crate) fn open_library(&mut self) -> Vec<Effect> {
        if let Some(scan) = &self.library_scan {
            let n = scan.found.load(Ordering::Relaxed);
            self.tell(&format!("Still scanning the library: {n} found so far."));
            return vec![Effect::Redraw];
        }
        let inputs = self.library_inputs();
        let found = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&found);
        let (tx, rx) = mpsc::channel();
        let wake = self.waker_slot();
        let spawned = std::thread::Builder::new()
            .name("textweaver-library-scan".into())
            .spawn(move || {
                let items = inputs.items(&|n| counter.store(n, Ordering::Relaxed));
                let _ = tx.send(items);
                wake.wake();
            });
        if let Err(e) = spawned {
            self.error(&format!("Could not scan the library: {e}."));
            return vec![Effect::Redraw];
        }
        self.library_scan = Some(LibraryScan {
            items: rx,
            found,
            shown_at: Instant::now(),
        });
        self.note("Scanning the library.");
        vec![Effect::Redraw]
    }

    /// Opens the library list once the scan is done, and shows how many
    /// documents it has found every second meanwhile (from [`App::tick`]).
    pub(crate) fn library_tick(&mut self) -> Vec<Effect> {
        let Some(scan) = &mut self.library_scan else {
            return Vec::new();
        };
        let items = match scan.items.try_recv() {
            Ok(items) => items,
            Err(TryRecvError::Empty) => {
                if scan.shown_at.elapsed() >= PROGRESS_EVERY {
                    scan.shown_at = Instant::now();
                    let n = scan.found.load(Ordering::Relaxed);
                    self.show(&format!("Scanning the library: {n} found so far."));
                    return vec![Effect::Redraw];
                }
                return Vec::new();
            }
            Err(TryRecvError::Disconnected) => {
                self.library_scan = None;
                self.error("The library scan stopped with an internal error.");
                return vec![Effect::Redraw];
            }
        };
        self.library_scan = None;
        self.show_library(items)
    }

    /// Shows a scanned library as a list.
    fn show_library(&mut self, items: Vec<LibraryItem>) -> Vec<Effect> {
        if items.is_empty() {
            self.tell(
                "The library is empty. Add a folder with tw library --add, or open a file with Control O.",
            );
            return vec![Effect::Redraw];
        }
        let n = items.len();
        let paths: Vec<PathBuf> = items.iter().map(|i| i.path.clone()).collect();
        let lines: Vec<String> = items.iter().map(LibraryItem::describe).collect();
        self.list = Some(ListKind::Library(paths));
        self.tell(&format!(
            "Library, {n} {}. Enter opens one.",
            if n == 1 { "document" } else { "documents" }
        ));
        vec![Effect::ShowList {
            title: "Library".into(),
            items: lines,
        }]
    }
}

/// How often a running library scan shows its count.
const PROGRESS_EVERY: Duration = Duration::from_secs(1);

/// A library scan on a background thread.
pub(crate) struct LibraryScan {
    items: Receiver<Vec<LibraryItem>>,
    found: Arc<AtomicUsize>,
    shown_at: Instant,
}

impl std::fmt::Debug for LibraryScan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LibraryScan")
            .field("found", &self.found.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

/// What a library scan reads: the folders, the extensions the loaders
/// open, the bookshelf, recent list, and state files, and the sidecars.
struct LibraryInputs {
    folders: Vec<PathBuf>,
    extensions: Vec<&'static str>,
    files: Option<(PathBuf, PathBuf, PathBuf)>,
    sync: LibrarySync,
}

impl LibraryInputs {
    /// Scans the folders and builds the list, calling `found` with the
    /// count of documents found so far.
    fn items(&self, found: &dyn Fn(usize)) -> Vec<LibraryItem> {
        let supported = |ext: &str| self.extensions.contains(&ext);
        let scanned = library::scan_library_with(&self.folders, &supported, found);
        let (lib, recent) = match &self.files {
            Some((library_file, recent_file, _)) => (
                Library::load(library_file).unwrap_or_default(),
                Recent::load(recent_file),
            ),
            None => (Library::default(), Recent::default()),
        };
        let states = self
            .files
            .as_ref()
            .map(|(_, _, dir)| textweaver_store::StateStore::new(dir.clone()));
        let local = |p: &Path| {
            states
                .as_ref()?
                .load(&DocKey::for_path(p))
                .filter(DocState::has_position)
                .map(|s| s.pct)
        };
        library::library_view(&scanned, &lib, &recent, self.sync.sidecars(), &local)
    }
}
