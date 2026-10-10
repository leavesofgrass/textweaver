//! The library (`open_library`, Alt+L in the terminal): documents in the
//! library folders and recently opened files, as one list; Enter opens one.
//!
//! Opening any document records it on the bookshelf (`library.json`,
//! [`Library::record_open`]) and the recent list. Documents inside a library
//! folder also sync their reading position through the folder's sidecar
//! (`<folder>/.textweaver/progress.json`, [`LibrarySync`]): the position is
//! mirrored there whenever it is saved, the sidecars are flushed on
//! document switch and quit, and on open the local and synced positions are
//! chosen between by `[sync] position_policy` (newest, furthest, or ask,
//! which asks, naming the other place's percentage; formerly
//! `reading.sync_conflict_policy`).
//!
//! The library list **filters as you type** (Wave 5, W5y): each word typed
//! must be in a document's title, path, author, DOI, or ISBN, or in its
//! text when `tw library search` has indexed it. The author, DOI, and
//! ISBN are recorded on the bookshelf when a document opens
//! ([`DocMetadata`]); a DOI or ISBN in the indexed text of a document never
//! opened counts too.
//!
//! With sync on (the sync wave, S6), the library details other computers
//! published fill these in too ([`SyncedLibrary::enrich`]), and **Continue
//! reading** (`continue_reading`) lists the documents found here with a
//! place saved on any computer, newest first. Places then go to the sync
//! folder, and a library folder's old sidecar is only read, so folders
//! written by an older textweaver or converted from star still resume.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use textweaver_lexicon::args;
use textweaver_store::library::{self, DetailField, DocMetadata, LibraryItem, ResumeSource};
use textweaver_store::sync::Resolution;
use textweaver_store::{DocKey, DocState, Library, LibrarySync, Recent, SimpleIndex};
use textweaver_text::Document;

use crate::app::{App, ListKind};
use crate::command::Effect;
use crate::synced_library::{ContinueItem, SyncedLibrary};

/// Where a document resumes, and how to say so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResumePoint {
    /// Char offset to resume at.
    pub(crate) pos: textweaver_core::CharPos,
    /// The position came from another device (the folder's sidecar).
    pub(crate) synced: bool,
    /// Another device's position differs and nothing was chosen (the
    /// `ask` policy): this device's is used, and the other one is asked
    /// about.
    pub(crate) unresolved: bool,
    /// The other device's position, when it differs and nothing was
    /// chosen.
    pub(crate) other: Option<textweaver_core::CharPos>,
}

impl App {
    /// A library sync for the configured folders and conflict policy.
    pub(crate) fn make_library_sync(settings: &textweaver_store::Settings) -> LibrarySync {
        LibrarySync::new(
            &settings.library.folders,
            settings.sync.position_policy.conflict_policy(),
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
                    .as_ref()
                    .is_some_and(|c| c.resolution == Resolution::Unresolved),
                other: r
                    .conflict
                    .as_ref()
                    .filter(|c| c.resolution == Resolution::Unresolved)
                    .and_then(|c| textweaver_store::sync::ProgressEntry::from_value(&c.remote))
                    .map(|e| e.offset),
            }),
            None => local_pos.map(|pos| ResumePoint {
                pos,
                synced: false,
                unresolved: false,
                other: None,
            }),
        }
    }

    /// Records an opened document on the bookshelf (`library.json`) and the
    /// recent list, on the background writer.
    pub(crate) fn record_library_open(&mut self, path: &Path, title: &str, format: &str) {
        self.record_library_open_with(path, title, format, DocMetadata::default());
        let details = textweaver_sync::docid::Details::default();
        self.sync_document_opened(path, None, details);
    }

    /// [`record_library_open`](Self::record_library_open) for a document
    /// just loaded: its author, DOI, and ISBN go on the bookshelf too, so
    /// the library can be searched by them. Returns what the document says
    /// about itself, for finding its sync id once it is open
    /// ([`App::sync_document_opened`]).
    pub(crate) fn record_library_open_doc(
        &mut self,
        path: &Path,
        title: &str,
        doc: &Document,
    ) -> textweaver_sync::docid::Details {
        let meta = document_metadata(doc);
        let details = sync_details_from(doc, &meta);
        self.record_library_open_with(path, title, &doc.meta.format, meta);
        details
    }

    /// Finds or makes the sync id of the document at `path` and keeps its
    /// hashes in `sync-ids.json` (ADR-0049), on the writer: hashing a large
    /// file must never hold up a key press. `text` is the document's text
    /// as read (a rope clone costs nothing), when there is one.
    pub(crate) fn identify_on_writer(
        &mut self,
        path: &Path,
        text: Option<ropey::Rope>,
        details: textweaver_sync::docid::Details,
    ) {
        let Some(paths) = &self.paths else {
            return;
        };
        let job = textweaver_sync::Identify {
            ids_file: paths.sync_ids_file(),
            path: path.to_owned(),
            library_folders: self.settings.library.folders.clone(),
            details,
        };
        self.writer.send(crate::writer::Job::Identify {
            job: Box::new(job),
            text,
        });
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
        // A guide packaged with textweaver is help, not a document of the
        // reader's: it stays out of the recent list and the library.
        if self.is_bundled_guide(path) {
            return;
        }
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
            synced: self.paths.clone().map(|p| (p, self.settings.clone())),
        }
    }

    /// Shows the library list (`open_library`). The folders are scanned on
    /// a background thread (Phase 2), with the count found shown as it
    /// goes; the list opens when the scan is done ([`library_tick`](Self::library_tick)).
    pub(crate) fn open_library(&mut self) -> Vec<Effect> {
        self.start_library_scan(ScanMode::Library)
    }

    /// Shows "Continue reading" (`continue_reading`, S6): the documents on
    /// this computer with a place saved here or on another computer, newest
    /// first. The library is scanned on a background thread first, as for
    /// the library list.
    pub(crate) fn open_continue_reading(&mut self) -> Vec<Effect> {
        self.start_library_scan(ScanMode::Continue)
    }

    /// "Add a folder to the library", in both frontends (W9a-c): the file
    /// browser chooses the folder, and it joins `[library] folders`, as
    /// `tw library add` does.
    pub(crate) fn add_library_folder(&mut self) -> Vec<Effect> {
        let purpose = self.msg("library-add-folder-choose");
        self.choose_folder(&purpose, |app, folder| app.library_folder_chosen(&folder))
    }

    /// Adds `folder` to the library folders, once, and says so.
    pub(crate) fn library_folder_chosen(&mut self, folder: &Path) -> Vec<Effect> {
        let (stored, added) = self.settings.library.add_folder(folder);
        let name = stored.file_name().map_or_else(
            || stored.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let id = if added {
            self.settings_dirty = true;
            self.library_sync = Self::make_library_sync(&self.settings);
            "library-folder-added"
        } else {
            "library-folder-already"
        };
        let msg = self.msg_args(id, &args!["name" => name]);
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    fn start_library_scan(&mut self, mode: ScanMode) -> Vec<Effect> {
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
        // The recent list and bookshelf updates queued when a document
        // opened are written first; the scan thread waits, not the keys.
        let written = self.writer.barrier();
        let spawned = std::thread::Builder::new()
            .name("textweaver-library-scan".into())
            .spawn(move || {
                if let Some(w) = written {
                    let _ = w.recv_timeout(std::time::Duration::from_secs(5));
                }
                let found = |n| counter.store(n, Ordering::Relaxed);
                let result = match mode {
                    ScanMode::Library => ScanResult::Library(inputs.list(&found)),
                    ScanMode::Continue => ScanResult::Continue(inputs.continue_reading(&found)),
                };
                let _ = tx.send(result);
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
        let result = match scan.items.try_recv() {
            Ok(result) => result,
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
        match result {
            ScanResult::Library(items) => self.show_library(items),
            ScanResult::Continue(items) => self.show_continue_reading(items),
        }
    }

    /// Shows "Continue reading", each row meaning first: "Cells, 42
    /// percent, laptop, 2 hours ago".
    fn show_continue_reading(&mut self, items: Vec<ContinueItem>) -> Vec<Effect> {
        if items.is_empty() {
            let msg = self.msg("continue-empty");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let msg = self.msg_args("continue-intro", &args!["n" => items.len()]);
        self.tell(&msg);
        let now = textweaver_sync::wall_ms();
        let c = self.cat().clone();
        let rows = items.iter().map(|i| i.describe(&c, now)).collect();
        let title = self.msg("continue-title");
        self.list = Some(ListKind::Continue(
            items.into_iter().map(|i| i.path).collect(),
        ));
        vec![Effect::ShowList { title, items: rows }]
    }

    /// Enter on a row of "Continue reading": opens the document, which
    /// resumes by `[sync] position_policy`.
    pub(crate) fn choose_continue(&mut self, paths: &[PathBuf], n: usize) -> Vec<Effect> {
        match paths.get(n) {
            Some(path) => self.open_command(path.clone()),
            None => vec![Effect::Redraw],
        }
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
            let msg = self.msg_args("library-empty", &args!["key" => open]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let msg = self.msg_args("library-intro", &args!["n" => list.items.len()]);
        self.tell(&msg);
        self.list_library(list)
    }

    /// Shows the library list again (after editing a document's details,
    /// W7m).
    pub(crate) fn reshow_library(&mut self, list: LibraryList) -> Vec<Effect> {
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

/// What a loaded document says about itself, for the sync folder's library
/// details (ADR-0049): the title it states, its DOI, ISBN, author, and
/// format. Only a title the document states (the reader's title falls back
/// to the file name, which must never reach the sync folder), and never an
/// author that is this computer's user or computer name (a Word file's
/// author is often the account's name).
pub(crate) fn sync_details(doc: &Document) -> textweaver_sync::docid::Details {
    sync_details_from(doc, &document_metadata(doc))
}

fn sync_details_from(doc: &Document, meta: &DocMetadata) -> textweaver_sync::docid::Details {
    let names = textweaver_sync::local_names();
    let author = meta.author.clone().filter(|a| {
        let a = a.to_lowercase();
        !names
            .iter()
            .any(|n| n.chars().count() >= 3 && a.contains(&n.to_lowercase()))
    });
    textweaver_sync::docid::Details {
        title: stated_title(doc),
        doi: meta.doi.clone(),
        isbn: meta.isbn.clone(),
        author,
        format: Some(doc.meta.format.clone()).filter(|f| !f.trim().is_empty()),
        added_ms: None,
    }
}

/// The title a document states itself, for the sync folder: not one a
/// loader made from the file's name (ADR-0049: no file name ever reaches
/// the sync folder).
pub(crate) fn stated_title(doc: &Document) -> Option<String> {
    let title = doc.meta.title.clone()?;
    let from_path = doc.meta.path.as_deref().is_some_and(|p| {
        let t = title.trim().to_lowercase();
        let name = p.file_name().map(|n| n.to_string_lossy().to_lowercase());
        let stem = p.file_stem().map(|n| n.to_string_lossy().to_lowercase());
        name.as_deref() == Some(t.as_str()) || stem.as_deref() == Some(t.as_str())
    });
    (!from_path).then_some(title)
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
    /// Indexed text by document, lowercase (from `tw library search`'s
    /// cache), for the filter.
    texts: Arc<std::collections::BTreeMap<PathBuf, String>>,
    /// The filter typed so far.
    pub(crate) filter: String,
    /// Items shown, as indexes into `items`.
    shown: Vec<usize>,
    /// The document's own title and details, for each document with hand
    /// edits: what its row shows when an edit is cleared (W8a).
    own: std::collections::BTreeMap<PathBuf, OwnDetails>,
}

/// A document's own title, author, DOI, and ISBN, without the owner's
/// hand edits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OwnDetails {
    pub(crate) title: String,
    pub(crate) meta: DocMetadata,
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
            own: std::collections::BTreeMap::new(),
        }
    }

    /// With the documents' own details, for the ones with hand edits.
    pub(crate) fn with_own(mut self, own: std::collections::BTreeMap<PathBuf, OwnDetails>) -> Self {
        self.own = own;
        self
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

    /// The document shown as item `n`, with its details.
    pub(crate) fn item_at(&self, n: usize) -> Option<&LibraryItem> {
        self.shown.get(n).and_then(|&i| self.items.get(i))
    }

    /// The owner edited item `n`'s details (W7m): its row shows the new
    /// values at once. A cleared field shows the document's own value at
    /// once too (W8a; before, the old value stayed until the library was
    /// read again).
    pub(crate) fn apply_edits(&mut self, n: usize, edits: &[(DetailField, Option<String>)]) {
        let Some(item) = self.shown.get(n).and_then(|&i| self.items.get_mut(i)) else {
            return;
        };
        // A document edited for the first time in this list shows its own
        // details now: keep them for a later clear. (One whose edits came
        // only from another computer has no own values here: a cleared
        // title falls back to the file name, a cleared detail to none.)
        let own = self
            .own
            .entry(item.path.clone())
            .or_insert_with(|| {
                let e = &item.edited;
                let keep = |field: DetailField, v: &Option<String>| {
                    if e.get(field).is_some() {
                        None
                    } else {
                        v.clone()
                    }
                };
                OwnDetails {
                    title: if e.title.is_some() {
                        String::new()
                    } else {
                        item.title.clone()
                    },
                    meta: DocMetadata {
                        author: keep(DetailField::Author, &item.meta.author),
                        doi: keep(DetailField::Doi, &item.meta.doi),
                        isbn: keep(DetailField::Isbn, &item.meta.isbn),
                    },
                }
            })
            .clone();
        for (field, value) in edits {
            item.edited.set(*field, value.clone());
            match (field, value.clone()) {
                (DetailField::Title, Some(v)) => item.title = v,
                (DetailField::Title, None) => {
                    item.title = if own.title.is_empty() {
                        file_stem_of(&item.path)
                    } else {
                        own.title.clone()
                    };
                }
                (DetailField::Author, v) => item.meta.author = v.or(own.meta.author.clone()),
                (DetailField::Doi, v) => item.meta.doi = v.or(own.meta.doi.clone()),
                (DetailField::Isbn, v) => item.meta.isbn = v.or(own.meta.isbn.clone()),
            }
        }
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

/// What a library scan is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScanMode {
    /// The library list.
    Library,
    /// "Continue reading".
    Continue,
}

/// What a library scan found.
enum ScanResult {
    /// The library list.
    Library(LibraryList),
    /// "Continue reading", newest first.
    Continue(Vec<ContinueItem>),
}

/// A library scan on a background thread.
pub(crate) struct LibraryScan {
    items: Receiver<ScanResult>,
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
    /// `tw library search`'s text cache.
    fulltext: Option<PathBuf>,
    sync: LibrarySync,
    /// Where this computer's files are and the settings, for reading the
    /// sync folder (S6: other computers' library details and places).
    synced: Option<(textweaver_store::Paths, textweaver_store::Settings)>,
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
        let (mut items, mut own) = self.view(found);
        let index = self
            .fulltext
            .as_deref()
            .map(SimpleIndex::load)
            .unwrap_or_default();
        // What other computers know: an author, DOI, or ISBN found only
        // there makes the filter find the document here too.
        if let Some(synced) = self.synced_library() {
            synced.enrich(&mut items, &|p| {
                index.entries.get(p).map(|e| e.text.clone())
            });
        }
        let mut texts = std::collections::BTreeMap::new();
        for item in &mut items {
            if let Some(e) = index.entries.get(&item.path) {
                let head: String = e.text.chars().take(library::METADATA_SCAN_CHARS).collect();
                let found = DocMetadata::from_document(None, &Default::default(), &head);
                item.meta.fill_from(&found);
                if let Some(o) = own.get_mut(&item.path) {
                    o.meta.fill_from(&found);
                }
                texts.insert(item.path.clone(), e.text.to_lowercase());
            }
        }
        LibraryList::new(items, texts).with_own(own)
    }

    /// The sync folder as read now, when sync is on and the folder is there.
    fn synced_library(&self) -> Option<SyncedLibrary> {
        let (paths, settings) = self.synced.as_ref()?;
        SyncedLibrary::load(paths, settings)
    }

    /// "Continue reading": the library's documents found here, each at its
    /// newest place from any computer, newest first.
    fn continue_reading(&self, found: &dyn Fn(usize)) -> Vec<ContinueItem> {
        let (items, _) = self.view(found);
        let index = self
            .fulltext
            .as_deref()
            .map(SimpleIndex::load)
            .unwrap_or_default();
        let synced = self.synced_library();
        let state_dir = self.files.as_ref().map(|(_, _, d)| d.as_path());
        crate::synced_library::continue_reading(&items, state_dir, synced.as_ref(), &|p| {
            index.entries.get(p).map(|e| e.text.clone())
        })
    }

    /// The library view: the folders scanned, the bookshelf, the recent
    /// list, and progress; with the own details of each document that has
    /// hand edits.
    fn view(
        &self,
        found: &dyn Fn(usize),
    ) -> (
        Vec<LibraryItem>,
        std::collections::BTreeMap<PathBuf, OwnDetails>,
    ) {
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
        let items = library::library_view(&scanned, &lib, &recent, self.sync.sidecars(), &local);
        // The document's own details, for each one with hand edits: the
        // bookshelf's title (else the file name) and details (W8a).
        let own = items
            .iter()
            .filter(|i| !i.edited.is_empty())
            .map(|i| {
                let entry = lib.get(&i.path);
                let title = entry
                    .map(|e| e.title.clone())
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| file_stem_of(&i.path));
                let meta = entry.map(|e| e.meta.clone()).unwrap_or_default();
                (i.path.clone(), OwnDetails { title, meta })
            })
            .collect();
        (items, own)
    }
}

/// A file's name without its extension, as the library shows a document
/// with no title.
fn file_stem_of(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}
