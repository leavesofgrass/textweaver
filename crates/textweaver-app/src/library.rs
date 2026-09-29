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
//!
//! The library list **filters as you type** (Wave 5, W5y): each word typed
//! must be in a document's title, path, author, DOI, or ISBN, or in its
//! text when `tw library --search` has indexed it. The author, DOI, and
//! ISBN are recorded on the bookshelf when a document opens
//! ([`DocMetadata`]); a DOI or ISBN in the indexed text of a document never
//! opened counts too.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use textweaver_lexicon::args;
use textweaver_store::library::{self, DocMetadata, LibraryItem, ResumeSource};
use textweaver_store::sync::Resolution;
use textweaver_store::{DocKey, DocState, Library, LibrarySync, Recent, SimpleIndex};
use textweaver_text::Document;

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
        self.record_library_open_with(path, title, format, DocMetadata::default());
    }

    /// [`record_library_open`](Self::record_library_open) for a document
    /// just loaded: its author, DOI, and ISBN go on the bookshelf too, so
    /// the library can be searched by them.
    pub(crate) fn record_library_open_doc(&mut self, path: &Path, title: &str, doc: &Document) {
        let meta = document_metadata(doc);
        self.record_library_open_with(path, title, &doc.meta.format, meta);
    }

    fn record_library_open_with(
        &mut self,
        path: &Path,
        title: &str,
        format: &str,
        meta: DocMetadata,
    ) {
        let Some(paths) = &self.paths else {
            return;
        };
        let job = crate::writer::Job::Opened {
            library_file: paths.library_file(),
            recent_file: paths.recent_file(),
            path: path.to_owned(),
            title: title.to_owned(),
            format: format.to_owned(),
            meta,
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
            fulltext: self
                .paths
                .as_ref()
                .map(textweaver_store::Paths::fulltext_file),
            sync: self.library_sync.clone(),
        }
    }

    /// Shows the library list (`open_library`). The folders are scanned on
    /// a background thread (Phase 2), with the count found shown as it
    /// goes; the list opens when the scan is done ([`library_tick`](Self::library_tick)).
    pub(crate) fn open_library(&mut self) -> Vec<Effect> {
        if let Some(scan) = &self.library_scan {
            let n = scan.found.load(Ordering::Relaxed);
            let msg = self.msg_args("library-still-scanning", &args!["n" => n]);
            self.tell(&msg);
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
                let items = inputs.list(&|n| counter.store(n, Ordering::Relaxed));
                let _ = tx.send(items);
                wake.wake();
            });
        if let Err(e) = spawned {
            let msg = self.msg_args("library-scan-failed", &args!["error" => e.to_string()]);
            self.error(&msg);
            return vec![Effect::Redraw];
        }
        self.library_scan = Some(LibraryScan {
            items: rx,
            found,
            shown_at: Instant::now(),
        });
        let msg = self.msg("library-scanning");
        self.note(&msg);
        vec![Effect::Redraw]
    }

    /// Opens the library list once the scan is done, and shows how many
    /// documents it has found every second meanwhile (from [`App::tick`]).
    pub(crate) fn library_tick(&mut self) -> Vec<Effect> {
        let Some(scan) = &mut self.library_scan else {
            return Vec::new();
        };
        let items = match scan.items.try_recv() {
            Ok(list) => list,
            Err(TryRecvError::Empty) => {
                if scan.shown_at.elapsed() >= PROGRESS_EVERY {
                    scan.shown_at = Instant::now();
                    let n = scan.found.load(Ordering::Relaxed);
                    let msg = self.msg_args("library-scan-progress", &args!["n" => n]);
                    self.show(&msg);
                    return vec![Effect::Redraw];
                }
                return Vec::new();
            }
            Err(TryRecvError::Disconnected) => {
                self.library_scan = None;
                let msg = self.msg("library-scan-stopped");
                self.error(&msg);
                return vec![Effect::Redraw];
            }
        };
        self.library_scan = None;
        self.show_library(items)
    }

    /// The library list's filter typed so far, while it is shown.
    pub(crate) fn library_filter(&self) -> Option<&str> {
        match &self.list {
            Some(ListKind::Library(l)) => Some(&l.filter),
            _ => None,
        }
    }

    /// The library list's filter changed to `query`: the documents holding
    /// every word ([`library::item_matches`]) are shown again.
    pub(crate) fn filter_library(&mut self, query: String) -> Vec<Effect> {
        let Some(ListKind::Library(mut list)) = self.list.take() else {
            return vec![Effect::Redraw];
        };
        list.apply_filter(&query);
        let n = list.shown.len();
        let msg = if query.trim().is_empty() {
            self.msg_args("library-filter-cleared", &args!["n" => n])
        } else if n == 0 {
            self.msg_args("library-filter-none", &args!["query" => query.as_str()])
        } else {
            self.msg_args("library-filter-matched", &args!["n" => n])
        };
        self.tell(&msg);
        self.list_library(list)
    }

    /// Enter on item `n` of the library list: opens the document.
    pub(crate) fn choose_library(&mut self, list: &LibraryList, n: usize) -> Vec<Effect> {
        match list.path_at(n) {
            Some(path) => self.open_command(path.to_owned()),
            None => vec![Effect::Redraw],
        }
    }

    /// Shows a scanned library as a list.
    fn show_library(&mut self, list: LibraryList) -> Vec<Effect> {
        if list.items.is_empty() {
            let open = self.key(textweaver_keymap::ActionId::Open);
            let msg = self.msg_args(
                "library-empty",
                &args!["command" => "tw library --add", "key" => open],
            );
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let msg = self.msg_args("library-intro", &args!["n" => list.items.len()]);
        self.tell(&msg);
        self.list_library(list)
    }

    /// The library list as shown: its title (with the filter, when there is
    /// one) and the documents that pass the filter.
    fn list_library(&mut self, list: LibraryList) -> Vec<Effect> {
        let title = if list.filter.trim().is_empty() {
            self.msg("library-title")
        } else {
            self.msg_args(
                "library-title-filtered",
                &args![
                    "shown" => list.shown.len(),
                    "n" => list.items.len(),
                    "filter" => list.filter.as_str()
                ],
            )
        };
        let items = list
            .shown
            .iter()
            .map(|&i| list.items[i].describe())
            .collect();
        self.list = Some(ListKind::Library(list));
        vec![Effect::ShowList { title, items }]
    }
}

/// The author, DOI, and ISBN of a loaded document: its own metadata, then
/// the start of its text ([`DocMetadata::from_document`]).
pub(crate) fn document_metadata(doc: &Document) -> DocMetadata {
    let rope = doc.text();
    let end = rope.len_chars().min(library::METADATA_SCAN_CHARS);
    let head = rope.slice(..end).to_string();
    DocMetadata::from_document(doc.meta.author.as_deref(), &doc.meta.properties, &head)
}

/// The library list: every document, the filter typed, and which
/// documents pass it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LibraryList {
    /// Every document, in the library's order.
    items: Vec<LibraryItem>,
    /// Indexed text by document, lowercase (from `tw library --search`'s
    /// cache), for the filter.
    texts: Arc<std::collections::BTreeMap<PathBuf, String>>,
    /// The filter typed so far.
    pub(crate) filter: String,
    /// Items shown, as indexes into `items`.
    shown: Vec<usize>,
}

impl LibraryList {
    /// Every item shown, no filter.
    pub(crate) fn new(
        items: Vec<LibraryItem>,
        texts: std::collections::BTreeMap<PathBuf, String>,
    ) -> Self {
        LibraryList {
            shown: (0..items.len()).collect(),
            items,
            texts: Arc::new(texts),
            filter: String::new(),
        }
    }

    /// Shows the items matching `query`.
    pub(crate) fn apply_filter(&mut self, query: &str) {
        self.filter = query.to_owned();
        self.shown = (0..self.items.len())
            .filter(|&i| {
                let item = &self.items[i];
                let text = self.texts.get(&item.path).map(String::as_str);
                library::item_matches(item, query, text)
            })
            .collect();
    }

    /// The document shown as item `n`.
    pub(crate) fn path_at(&self, n: usize) -> Option<&Path> {
        self.shown
            .get(n)
            .and_then(|&i| self.items.get(i))
            .map(|item| item.path.as_path())
    }
}

/// How often a running library scan shows its count.
const PROGRESS_EVERY: Duration = Duration::from_secs(1);

/// A library scan on a background thread.
pub(crate) struct LibraryScan {
    items: Receiver<LibraryList>,
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
    /// `tw library --search`'s text cache.
    fulltext: Option<PathBuf>,
    sync: LibrarySync,
}

impl LibraryInputs {
    /// Scans the folders and builds the list, calling `found` with the
    /// count of documents found so far.
    fn items(&self, found: &dyn Fn(usize)) -> Vec<LibraryItem> {
        self.list(found).items
    }

    /// [`items`](Self::items) with the indexed text of each document, for
    /// the filter. A DOI or ISBN in the text fills what the bookshelf
    /// lacks.
    fn list(&self, found: &dyn Fn(usize)) -> LibraryList {
        let mut items = self.view(found);
        let index = self
            .fulltext
            .as_deref()
            .map(SimpleIndex::load)
            .unwrap_or_default();
        let mut texts = std::collections::BTreeMap::new();
        for item in &mut items {
            if let Some(e) = index.entries.get(&item.path) {
                let head: String = e.text.chars().take(library::METADATA_SCAN_CHARS).collect();
                item.meta.fill_from(&DocMetadata::from_document(
                    None,
                    &Default::default(),
                    &head,
                ));
                texts.insert(item.path.clone(), e.text.to_lowercase());
            }
        }
        LibraryList::new(items, texts)
    }

    /// The library view: the folders scanned, the bookshelf, the recent
    /// list, and progress.
    fn view(&self, found: &dyn Fn(usize)) -> Vec<LibraryItem> {
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
