//! The library side of sync (the sync wave, S6; ADR-0049): the library
//! details other computers published, a "Continue reading" list built from
//! every computer's places, and every computer's reading statistics.
//!
//! Everything here reads the sync folder through
//! [`FolderView`] and writes nothing, so the
//! library scan's helper thread and the command line can use it while the
//! reader's own sync engine keeps the folder open on the background writer.
//!
//! A document on this computer is matched to its record, in order, by:
//!
//! 1. `sync-ids.json` (the document was opened here);
//! 2. its library folder's id and its path inside the folder, when the
//!    folder's id file is there (a library folder synced between computers);
//! 3. the hash of its text, when `tw library --search` has read it.
//!
//! No step reads or hashes a whole file, so a large library stays quick.
//!
//! Library details (title, author, DOI, ISBN, format, first added) travel in
//! the record's identity (S2), newest wins per detail. The reading
//! statistics sum each computer's own counts; "Continue reading" lists only
//! documents found on this computer, newest place first.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;
use textweaver_store::library::{DetailField, LibraryItem};
use textweaver_store::sync_ids::{HashKind, SyncIds};
use textweaver_store::{DocKey, Library, Paths, ReadingStats, Recent, Settings, StateStore};
use textweaver_sync::docid::{library_key, read_library_id, text_sha256};
use textweaver_sync::record::detail;
use textweaver_sync::{DeviceId, DocRecord, FolderView, Identity, SyncId};

/// The most documents "Continue reading" lists.
pub const CONTINUE_MAX: usize = 30;

/// What the sync folder says, read once, for this computer.
#[derive(Clone, Debug)]
pub struct SyncedLibrary {
    view: FolderView,
    me: Option<DeviceId>,
    my_label: Option<String>,
    ids: SyncIds,
    /// Library folders and their ids, when their id files are there.
    library_ids: Vec<(PathBuf, textweaver_sync::LibraryId)>,
    /// The places group is on.
    places: bool,
    /// The statistics group is on.
    statistics: bool,
    /// This computer's files, for finding a document another computer
    /// read on the bookshelf here.
    paths: Paths,
}

/// A document's library details, as the sync folder has them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SyncedDetails {
    /// The title the document states.
    pub title: Option<String>,
    /// The author or authors.
    pub author: Option<String>,
    /// The DOI.
    pub doi: Option<String>,
    /// The ISBN.
    pub isbn: Option<String>,
    /// The kind of file.
    pub format: Option<String>,
    /// When it was first added to a library on any computer (milliseconds
    /// since 1970, UTC).
    pub added_ms: Option<u64>,
}

impl SyncedLibrary {
    /// Reads the sync folder `[sync]` names, when sync is on and the folder
    /// is there; `None` otherwise. Writes nothing.
    pub fn load(paths: &Paths, settings: &Settings) -> Option<Self> {
        let s = &settings.sync;
        if !s.enabled {
            return None;
        }
        let view = match FolderView::read(s.folder.as_deref()?) {
            Ok(v) => v,
            Err(e) => {
                log::debug!("sync folder not read for the library ({e})");
                return None;
            }
        };
        let me = Identity::peek(&paths.data_dir);
        let my_label = me
            .and_then(|d| view.label(d).map(str::to_owned))
            .or_else(|| Some(s.device_name.trim().to_owned()).filter(|n| !n.is_empty()));
        let library_ids = settings
            .library
            .folders
            .iter()
            .filter_map(|f| read_library_id(f).map(|id| (f.clone(), id)))
            .collect();
        Some(SyncedLibrary {
            view,
            me,
            my_label,
            ids: SyncIds::load(&paths.sync_ids_file()),
            library_ids,
            places: s.places,
            statistics: s.statistics,
            paths: paths.clone(),
        })
    }

    /// The folder as read.
    pub fn view(&self) -> &FolderView {
        &self.view
    }

    /// This computer's id, when sync was set up here.
    pub fn me(&self) -> Option<DeviceId> {
        self.me
    }

    /// This computer's name, when known.
    pub fn my_label(&self) -> Option<&str> {
        self.my_label.as_deref()
    }

    /// A computer's name: this computer's own name for its own id.
    pub fn label(&self, device: DeviceId) -> Option<String> {
        if Some(device) == self.me {
            return self.my_label.clone();
        }
        self.view.label(device).map(str::to_owned)
    }

    /// The sync id of the document at `path`, when a record of it is found
    /// (by `sync-ids.json`, its library folder's id, or `text`'s hash).
    pub fn sync_id_for(&self, path: &Path, text: Option<&str>) -> Option<SyncId> {
        let known = self
            .ids
            .get(&DocKey::for_path(path))
            .and_then(|e| e.sync_id.parse::<SyncId>().ok());
        if let Some(id) = known {
            return Some(id);
        }
        // Library folders are stored resolved (symbolic links followed, and
        // the drive path without `\\?\`), so compare the resolved path:
        // `/var` against `/private/var` on macOS, short names on Windows.
        let resolved = textweaver_store::library::resolve_path(path);
        let by_library = self.library_ids.iter().find_map(|(folder, lib)| {
            let folder = textweaver_store::library::resolve_path(folder);
            let rel = resolved
                .strip_prefix(&folder)
                .or_else(|_| path.strip_prefix(&folder))
                .ok()?;
            let rel = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            self.view.find(HashKind::Library, &library_key(*lib, &rel))
        });
        if by_library.is_some() {
            return by_library;
        }
        let text = text.filter(|_| self.view.has_text_hashes())?;
        self.view.find(HashKind::Text, &text_sha256([text]))
    }

    /// The merged record of `sync_id`.
    pub fn record(&self, sync_id: SyncId) -> Option<&DocRecord> {
        self.view.doc(sync_id)
    }

    /// A document's library details: the owner's hand-edited title,
    /// author, DOI, and ISBN where some computer has one (W7m), else the
    /// document's own.
    pub fn details(&self, sync_id: SyncId) -> Option<SyncedDetails> {
        let id = &self.record(sync_id)?.identity;
        let get = |name: &str| id.shown_detail(name).map(str::to_owned);
        let d = SyncedDetails {
            title: get(detail::TITLE),
            author: get(detail::AUTHOR),
            doi: get(detail::DOI),
            isbn: get(detail::ISBN),
            format: get(detail::FORMAT),
            added_ms: id.added.0,
        };
        (d != SyncedDetails::default()).then_some(d)
    }

    /// Adds what other computers know to the library's items, so the
    /// filter and `tw library --search` find a document by an author, DOI,
    /// or ISBN known elsewhere. The synced details are the newest, from
    /// every computer, this one's included, so they replace the item's;
    /// a title only replaces one made from the file's name. `text` gives a
    /// document's indexed text, when it has been read.
    ///
    /// A hand-edited detail (W7m) wins over the document's own: this
    /// computer's edit unless another computer's edit (or clearing) of the
    /// same detail is newer, and then that one.
    pub fn enrich(&self, items: &mut [LibraryItem], text: &dyn Fn(&Path) -> Option<String>) {
        for item in items {
            let t = text(&item.path);
            let Some(id) = self.sync_id_for(&item.path, t.as_deref()) else {
                continue;
            };
            let Some(record) = self.record(id) else {
                continue;
            };
            let identity = &record.identity;
            for field in DetailField::ALL {
                let name = field.name();
                let local = item.edited.get(field).map(str::to_owned);
                let theirs = identity
                    .edited_register(name)
                    .filter(|r| local.is_none() || r.stamp.time >= item.edited.at_ms);
                let hand = match theirs {
                    Some(r) => {
                        item.edited.set(field, r.value.clone());
                        r.value.clone()
                    }
                    None => local,
                };
                let own = identity.detail(name).map(str::to_owned);
                match field {
                    DetailField::Title => {
                        if let Some(title) = hand {
                            item.title = title;
                        } else if let Some(title) = own
                            && item.title == file_stem(&item.path)
                        {
                            item.title = title;
                        }
                    }
                    DetailField::Author | DetailField::Doi | DetailField::Isbn => {
                        let mine = match field {
                            DetailField::Author => &mut item.meta.author,
                            DetailField::Doi => &mut item.meta.doi,
                            _ => &mut item.meta.isbn,
                        };
                        if let Some(v) = hand.or(own) {
                            *mine = Some(v);
                        }
                    }
                }
            }
        }
    }
}

fn file_stem(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// One row of "Continue reading".
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContinueItem {
    /// The document, on this computer.
    pub path: PathBuf,
    /// Its title.
    pub title: String,
    /// How far into it the place is, in percent.
    pub pct: u8,
    /// The computer the place is from, by its name; `None` for this
    /// computer when it has no name.
    pub device: Option<String>,
    /// The place is this computer's own.
    pub this_computer: bool,
    /// When the place was saved (milliseconds since 1970, UTC).
    pub when_ms: u64,
}

impl ContinueItem {
    /// The row, meaning first: "Cells, 42 percent, laptop, 2 hours ago".
    pub fn describe(&self, c: &Catalog, now_ms: u64) -> String {
        let device = match (&self.device, self.this_computer) {
            (Some(d), _) => d.clone(),
            (None, true) => c.tr("continue-this-computer"),
            (None, false) => c.tr("sync-another-computer"),
        };
        c.fmt(
            "continue-item",
            &args![
                "title" => self.title.as_str(),
                "pct" => self.pct,
                "device" => device,
                "when" => ago(c, now_ms, self.when_ms)
            ],
        )
    }
}

/// How long ago `then_ms` was, in words: "just now", "5 minutes ago",
/// "2 hours ago", "3 days ago".
pub fn ago(c: &Catalog, now_ms: u64, then_ms: u64) -> String {
    let secs = now_ms.saturating_sub(then_ms) / 1000;
    let (id, n) = match secs {
        0..60 => return c.tr("continue-ago-now"),
        60..3_600 => ("continue-ago-minutes", secs / 60),
        3_600..86_400 => ("continue-ago-hours", secs / 3_600),
        _ => ("continue-ago-days", secs / 86_400),
    };
    c.fmt(id, &args!["n" => n])
}

/// "Continue reading": for each document in `items` (the library's
/// documents and recent files) that is found on this computer, its newest
/// place, from this computer's saved state or any computer's synced place,
/// newest first, at most [`CONTINUE_MAX`]. Documents with no place are left
/// out. `text` gives a document's indexed text, for finding its record.
pub fn continue_reading(
    items: &[LibraryItem],
    state_dir: Option<&Path>,
    synced: Option<&SyncedLibrary>,
    text: &dyn Fn(&Path) -> Option<String>,
) -> Vec<ContinueItem> {
    let states = state_dir.map(|d| StateStore::new(d.to_owned()));
    let my_label = synced.and_then(SyncedLibrary::my_label).map(str::to_owned);
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for item in items {
        if !item.path.is_file() || !seen.insert(item.path.clone()) {
            continue;
        }
        // (when, pct, device name, this computer)
        let mut best: Option<(u64, u8, Option<String>, bool)> = None;
        let mut consider = |cand: (u64, u8, Option<String>, bool)| {
            if best.as_ref().is_none_or(|b| cand.0 > b.0) {
                best = Some(cand);
            }
        };
        if let Some(st) = states
            .as_ref()
            .and_then(|s| s.load(&DocKey::for_path(&item.path)))
            .filter(textweaver_store::DocState::has_position)
        {
            let when = u64::try_from(st.ts.max(0))
                .unwrap_or(0)
                .saturating_mul(1000);
            consider((when, st.pct, my_label.clone(), true));
        }
        let mut title = item.title.clone();
        if let Some(s) = synced {
            let t = text(&item.path);
            if let Some(id) = s.sync_id_for(&item.path, t.as_deref()) {
                if let Some(r) = s.record(id).filter(|_| s.places) {
                    for (stamp, place) in r.places_newest_first() {
                        let mine = Some(stamp.device) == s.me;
                        consider((stamp.time, place.pct, s.label(stamp.device), mine));
                    }
                }
                if title == file_stem(&item.path)
                    && let Some(t) = s.details(id).and_then(|d| d.title)
                {
                    title = t;
                }
            }
        }
        if let Some((when_ms, pct, device, this_computer)) = best {
            out.push(ContinueItem {
                path: item.path.clone(),
                title,
                pct,
                device,
                this_computer,
                when_ms,
            });
        }
    }
    out.sort_by(|a, b| b.when_ms.cmp(&a.when_ms).then_with(|| a.path.cmp(&b.path)));
    out.truncate(CONTINUE_MAX);
    out
}

/// The library's documents and recent files, for "Continue reading" from
/// the command line (the reader scans on a helper thread instead).
pub fn library_items(paths: &Paths, settings: &Settings) -> Vec<LibraryItem> {
    let exts = textweaver_formats::Registry::with_builtins().extensions();
    let supported = |ext: &str| exts.contains(&ext);
    let scanned = textweaver_store::library::scan_library(&settings.library.folders, &supported);
    let lib = Library::load(&paths.library_file()).unwrap_or_default();
    let recent = Recent::load(&paths.recent_file());
    let sidecars =
        textweaver_store::sync::SidecarStore::new(settings.sync.position_policy.conflict_policy());
    textweaver_store::library::library_view(&scanned, &lib, &recent, &sidecars, &|_| None)
}

/// One computer's share of a document's reading.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ComputerShare {
    /// The computer's name, when known.
    pub device: Option<String>,
    /// This computer.
    pub this_computer: bool,
    /// Seconds read aloud there.
    pub seconds: f64,
    /// Sessions there.
    pub sessions: u64,
}

/// One document's reading, summed over every computer.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DocTotals {
    /// Its title, when known.
    pub title: String,
    /// Its file on this computer, when known.
    pub path: Option<PathBuf>,
    /// Its key in this computer's `stats.json`, when read here.
    #[serde(skip)]
    pub key: Option<String>,
    /// Seconds read aloud, over every computer.
    pub seconds: f64,
    /// Sessions, over every computer.
    pub sessions: u64,
    /// The furthest point any computer reached, in percent.
    pub furthest_percent: u8,
    /// When any computer last read it (Unix seconds, UTC).
    pub last_read: i64,
    /// Each computer's share, this computer first.
    pub computers: Vec<ComputerShare>,
}

impl DocTotals {
    /// Another computer read it too.
    pub fn from_others(&self) -> bool {
        self.computers.iter().any(|c| !c.this_computer)
    }
}

/// Reading statistics over every computer.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct CombinedStats {
    /// Every document, most read first.
    pub documents: Vec<DocTotals>,
}

impl CombinedStats {
    /// This computer's `stats.json`, with the other computers' statistics
    /// from `synced` added, document by document (the statistics group
    /// on). Each computer adds only its own counts, so the totals are sums.
    pub fn build(local: &ReadingStats, synced: Option<&SyncedLibrary>) -> Self {
        let synced = synced.filter(|s| s.statistics);
        let me = synced.and_then(|s| s.me);
        let my_label = synced.and_then(SyncedLibrary::my_label).map(str::to_owned);
        let mut covered = std::collections::HashSet::new();
        let mut docs = Vec::new();
        for (key, d) in &local.documents {
            let record = synced.and_then(|s| {
                let id = s.ids.docs.get(key)?.sync_id.parse::<SyncId>().ok()?;
                covered.insert(id);
                s.record(id).map(|r| (s, r))
            });
            let mut t = DocTotals {
                title: d.title.clone(),
                path: d.path.clone(),
                key: Some(key.clone()),
                seconds: d.seconds.max(0.0),
                sessions: u64::from(d.sessions),
                furthest_percent: d.furthest_percent,
                last_read: d.last_read,
                computers: vec![ComputerShare {
                    device: my_label.clone(),
                    this_computer: true,
                    seconds: d.seconds.max(0.0),
                    sessions: u64::from(d.sessions),
                }],
            };
            if let Some((s, r)) = record {
                add_others(&mut t, r, s, me);
            }
            docs.push(t);
        }
        if let Some(s) = synced {
            let paths_by_id = local_paths_by_id(s, &s.paths);

            for (id, r) in &s.view.docs {
                if covered.contains(id) {
                    continue;
                }
                let mut t = DocTotals {
                    title: s.details(*id).and_then(|d| d.title).unwrap_or_default(),
                    path: paths_by_id.get(id).cloned(),
                    key: None,
                    seconds: 0.0,
                    sessions: 0,
                    furthest_percent: 0,
                    last_read: 0,
                    computers: Vec::new(),
                };
                add_others(&mut t, r, s, me);
                if !t.computers.is_empty() {
                    docs.push(t);
                }
            }
        }
        docs.sort_by(|a, b| {
            b.seconds
                .total_cmp(&a.seconds)
                .then(b.last_read.cmp(&a.last_read))
        });
        CombinedStats { documents: docs }
    }

    /// Seconds read over every document and computer.
    pub fn total_seconds(&self) -> f64 {
        self.documents.iter().map(|d| d.seconds).sum()
    }

    /// Sessions over every document and computer.
    pub fn total_sessions(&self) -> u64 {
        self.documents.iter().map(|d| d.sessions).sum()
    }

    /// Another computer's reading is included.
    pub fn has_others(&self) -> bool {
        self.documents.iter().any(DocTotals::from_others)
    }

    /// The document with this computer's `stats.json` key.
    pub fn by_key(&self, key: &str) -> Option<&DocTotals> {
        self.documents
            .iter()
            .find(|d| d.key.as_deref() == Some(key))
    }
}

/// Adds the other computers' counts in `r` to `t`.
fn add_others(t: &mut DocTotals, r: &DocRecord, s: &SyncedLibrary, me: Option<DeviceId>) {
    let st = &r.stats;
    let mut devices: Vec<DeviceId> = st
        .seconds
        .0
        .keys()
        .chain(st.sessions.0.keys())
        .copied()
        .filter(|d| Some(*d) != me)
        .collect();
    devices.sort();
    devices.dedup();
    for d in devices {
        let seconds = st.seconds.of(d) as f64;
        let sessions = st.sessions.of(d);
        if seconds <= 0.0 && sessions == 0 {
            continue;
        }
        t.seconds += seconds;
        t.sessions += sessions;
        t.computers.push(ComputerShare {
            device: s.label(d),
            this_computer: false,
            seconds,
            sessions,
        });
    }
    let furthest = u8::try_from(st.furthest_percent.0.min(100)).unwrap_or(100);
    t.furthest_percent = t.furthest_percent.max(furthest);
    let last = i64::try_from(st.last_read.0 / 1000).unwrap_or(i64::MAX);
    t.last_read = t.last_read.max(last);
}

/// Documents on this computer by sync id, from the bookshelf and the recent
/// list (for opening a document another computer read).
fn local_paths_by_id(s: &SyncedLibrary, paths: &Paths) -> HashMap<SyncId, PathBuf> {
    let lib = Library::load(&paths.library_file()).unwrap_or_default();
    let recent = Recent::load(&paths.recent_file());
    let candidates = lib
        .entries
        .iter()
        .map(|e| e.path.clone())
        .chain(recent.entries.iter().map(|r| r.path.clone()));
    let mut out = HashMap::new();
    for p in candidates {
        if let Some(id) = s.sync_id_for(&p, None) {
            out.entry(id).or_insert(p);
        }
    }
    out
}

/// A document's name for the statistics: its title, else its file's name,
/// else "Untitled document".
pub fn stats_title(c: &Catalog, t: &DocTotals) -> String {
    if !t.title.trim().is_empty() {
        return t.title.clone();
    }
    t.path
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| c.tr("stats-untitled"))
}

/// One computer's line: "laptop: 20 minutes, 2 sessions".
pub fn computer_line(c: &Catalog, share: &ComputerShare) -> String {
    let device = match (&share.device, share.this_computer) {
        (Some(d), _) => d.clone(),
        (None, true) => c.tr("continue-this-computer"),
        (None, false) => c.tr("sync-another-computer"),
    };
    c.fmt(
        "stats-computer",
        &args![
            "device" => device,
            "time" => textweaver_lexicon::i18n::duration(c, share.seconds),
            "sessions" => share.sessions
        ],
    )
}
