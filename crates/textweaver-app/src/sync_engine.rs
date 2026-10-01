//! The sync engine: everything that reads or writes the sync folder
//! (ADR-0049), run on the background writer so a key press never waits on
//! a USB stick or a sync service's folder.
//!
//! The app sends a [`SyncRequest`] (the background writer's `Job::Sync`); the
//! engine answers with a [`SyncResponse`] that the app applies on its next
//! tick (`crate::sync`). The engine keeps, for each document it has seen in
//! this session, this computer's merged view of it ([`DocRecord`]), which
//! is also what the app's state held after the last merge: the **base**.
//!
//! One merge (a *cycle*) of a document:
//!
//! 1. **Local edits.** The app's state (a [`Snapshot`]) is compared with the
//!    base, item by item, ignoring where an item sits (a note found again
//!    after an edit is not a change): a new or changed bookmark, note, or
//!    highlight, or one deleted here (the store's deletion records).
//! 2. **Arrivals.** Every other computer's record is merged into the base.
//! 3. **Both at once.** When the same item changed here and on another
//!    computer, the newer edit by the clock wins (the owner's rule for
//!    notes); the older version of a note is kept in the local backup of
//!    replaced notes when the app applies the change.
//! 4. Local edits are stamped with the hybrid clock, this computer's place
//!    and statistics are set, and the merged view is written, only when it
//!    changed and only when the folder is not read-only.
//!
//! What arrived goes back to the app as [`Arrival`]s, each with the version
//! the app had, so the app applies it only if nothing changed meanwhile.
//! The engine never moves the reader's cursor; places are only reported.
//!
//! A document the app does not have open is merged the same way by
//! [`SyncEngine::sync_all`] (Sync now, `tw sync now`), straight into its
//! state file.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;
use textweaver_store::sync_ids::SyncIds;
use textweaver_store::{
    Bookmark, DocKey, DocState, Highlight, MarkKind, Note, Paths, ReadingStats, StateStore,
    SyncSettings,
};
use textweaver_sync::folder::{DEVICE_FILE, DEVICES_DIR, DOCS_DIR, SYNC_DIR};
use textweaver_sync::merge::{ChangeKind, RegisterMap};
use textweaver_sync::record::{ItemKind, Place};
use textweaver_sync::{
    Clock, DeviceId, DeviceInfo, DocRecord, Found, Identify, Identity, IdentityEvent,
    IdentityIndex, Problem, Stamp, SyncError, SyncFolder, SyncId,
};

/// The folder beside `state/` that keeps a copy of the state files from
/// before the first merge, so turning sync on can be undone (ADR-0049).
pub const STATE_BACKUP_DIR: &str = "state-before-sync";

/// Which groups sync (`[sync]`'s switches). A group turned off neither
/// sends nor takes its items; the others go on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Groups {
    /// Places.
    pub places: bool,
    /// Notes.
    pub notes: bool,
    /// Highlights.
    pub highlights: bool,
    /// Bookmarks.
    pub bookmarks: bool,
    /// Reading statistics.
    pub statistics: bool,
    /// Portable settings.
    pub settings: bool,
    /// Profile definitions.
    pub profiles: bool,
    /// Key overrides.
    pub key_overrides: bool,
    /// The personal word list.
    pub words: bool,
    /// The glossary and pronunciations.
    pub glossary: bool,
    /// Favorite voices.
    pub favorite_voices: bool,
}

impl Groups {
    /// Every group on.
    pub const ALL: Groups = Groups {
        places: true,
        notes: true,
        highlights: true,
        bookmarks: true,
        statistics: true,
        settings: true,
        profiles: true,
        key_overrides: true,
        words: true,
        glossary: true,
        favorite_voices: true,
    };

    /// Every group off.
    pub const NONE: Groups = Groups {
        places: false,
        notes: false,
        highlights: false,
        bookmarks: false,
        statistics: false,
        settings: false,
        profiles: false,
        key_overrides: false,
        words: false,
        glossary: false,
        favorite_voices: false,
    };

    /// Each group's name in `[sync]` (and on the command line), in the
    /// order Set up sync lists them.
    pub const NAMES: [&'static str; 11] = [
        "places",
        "notes",
        "highlights",
        "bookmarks",
        "statistics",
        "settings",
        "profiles",
        "key_overrides",
        "words",
        "glossary",
        "favorite_voices",
    ];

    /// The groups `[sync]` turns on.
    pub fn from_settings(s: &SyncSettings) -> Self {
        Groups {
            places: s.places,
            notes: s.notes,
            highlights: s.highlights,
            bookmarks: s.bookmarks,
            statistics: s.statistics,
            settings: s.settings,
            profiles: s.profiles,
            key_overrides: s.key_overrides,
            words: s.words,
            glossary: s.glossary,
            favorite_voices: s.favorite_voices,
        }
    }

    /// Writes the switches into `[sync]`.
    pub fn write_to(self, s: &mut SyncSettings) {
        s.places = self.places;
        s.notes = self.notes;
        s.highlights = self.highlights;
        s.bookmarks = self.bookmarks;
        s.statistics = self.statistics;
        s.settings = self.settings;
        s.profiles = self.profiles;
        s.key_overrides = self.key_overrides;
        s.words = self.words;
        s.glossary = self.glossary;
        s.favorite_voices = self.favorite_voices;
    }

    /// The switch named `name` (one of [`NAMES`](Self::NAMES)).
    pub fn get_mut(&mut self, name: &str) -> Option<&mut bool> {
        Some(match name {
            "places" => &mut self.places,
            "notes" => &mut self.notes,
            "highlights" => &mut self.highlights,
            "bookmarks" => &mut self.bookmarks,
            "statistics" => &mut self.statistics,
            "settings" => &mut self.settings,
            "profiles" => &mut self.profiles,
            "key_overrides" => &mut self.key_overrides,
            "words" => &mut self.words,
            "glossary" => &mut self.glossary,
            "favorite_voices" => &mut self.favorite_voices,
            _ => return None,
        })
    }

    /// Whether the switch named `name` is on.
    pub fn get(self, name: &str) -> bool {
        let mut g = self;
        g.get_mut(name).is_some_and(|v| *v)
    }

    fn has(self, kind: MarkKind) -> bool {
        match kind {
            MarkKind::Note => self.notes,
            MarkKind::Highlight => self.highlights,
            MarkKind::Bookmark => self.bookmarks,
        }
    }
}

/// What the engine needs to open the folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineConfig {
    /// The sync folder.
    pub folder: PathBuf,
    /// This computer's name; empty takes the first free "Computer N".
    pub device_name: String,
    /// Where this computer's files are.
    pub paths: Paths,
    /// The app's version, for `device.json`.
    pub app_version: String,
    /// The groups that sync.
    pub groups: Groups,
    /// This computer's system, for key overrides.
    pub system: crate::sync_groups::KeySystem,
}

impl EngineConfig {
    /// The configuration `[sync]` describes, or `None` when sync is off or
    /// has no folder.
    pub fn from_settings(s: &SyncSettings, paths: &Paths) -> Option<Self> {
        if !s.enabled {
            return None;
        }
        Some(EngineConfig {
            folder: s.folder.clone()?,
            device_name: s.device_name.trim().to_owned(),
            paths: paths.clone(),
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            groups: Groups::from_settings(s),
            system: crate::sync_groups::KeySystem::current(),
        })
    }
}

/// How sync stands, for the status line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusKind {
    /// Not started yet.
    #[default]
    Starting,
    /// Sync is off, or not set up.
    Off,
    /// The folder is not there (a USB stick not plugged in): everything is
    /// saved on this computer, and synced when the folder is back.
    FolderMissing,
    /// The folder is from a newer textweaver (or its format file is
    /// damaged): nothing is written to it.
    ReadOnly,
    /// Working.
    Ready,
    /// The folder could not be used; the reason is in
    /// [`EngineStatus::failure`].
    Failed,
}

/// How sync stands, with the names to say.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EngineStatus {
    /// The state.
    pub kind: StatusKind,
    /// This computer's name.
    pub label: String,
    /// The other computers' names, in id order.
    pub others: Vec<String>,
    /// The computers whose clocks were seen more than a day ahead.
    pub clocks_ahead: Vec<String>,
    /// Damaged files skipped in this session.
    pub damaged: usize,
    /// Writing to the folder failed, and has not worked since.
    pub write_error: Option<String>,
    /// Why the folder could not be used.
    pub failure: Option<String>,
    /// Key overrides from the other kind of system (a Mac's on Windows or
    /// Linux, and the reverse), kept in the sync folder but not used here.
    pub kept_key_overrides: usize,
}

/// Something the owner should hear once in a session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    /// A damaged or cut-short file from another computer was skipped.
    Damaged {
        /// The computer, when known.
        label: Option<String>,
    },
    /// A file from a newer textweaver was skipped.
    NewerFile {
        /// The computer.
        label: Option<String>,
    },
    /// The folder is read-only on this computer.
    ReadOnly,
    /// A computer's clock is more than a day ahead.
    ClockAhead {
        /// The computer.
        label: Option<String>,
        /// By how many hours.
        hours: u64,
    },
    /// This state folder was a copy of another computer's: it took a new
    /// id, and keeps its data.
    FreshDeviceId,
    /// Writing to the folder failed.
    WriteFailed(String),
    /// The computer name cannot be used.
    NameRefused(String),
}

/// A bookmark, a note, or a highlight.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// A bookmark.
    Bookmark(Bookmark),
    /// A note.
    Note(Box<Note>),
    /// A highlight.
    Highlight(Highlight),
}

impl Item {
    /// Its kind.
    pub fn kind(&self) -> MarkKind {
        match self {
            Item::Bookmark(_) => MarkKind::Bookmark,
            Item::Note(_) => MarkKind::Note,
            Item::Highlight(_) => MarkKind::Highlight,
        }
    }

    fn json(&self) -> Value {
        match self {
            Item::Bookmark(b) => serde_json::to_value(b),
            Item::Note(n) => serde_json::to_value(n),
            Item::Highlight(h) => serde_json::to_value(h),
        }
        .unwrap_or(Value::Null)
    }
}

/// An item's content: everything but where it sits, which each computer
/// finds again in its own copy of the text.
fn content<T: Serialize>(kind: MarkKind, item: &T) -> Value {
    let mut v = serde_json::to_value(item).unwrap_or(Value::Null);
    if let Some(o) = v.as_object_mut() {
        let place: &[&str] = match kind {
            MarkKind::Bookmark => &["pos", "pct", "anchor", "not_found"],
            MarkKind::Note | MarkKind::Highlight => &["range", "not_found"],
        };
        for k in place {
            o.remove(*k);
        }
    }
    v
}

/// True when two versions of an item say the same thing, wherever they sit.
pub fn same_content(a: &Item, b: &Item) -> bool {
    a.kind() == b.kind() && strip(a.kind(), a.json()) == strip(b.kind(), b.json())
}

fn strip(kind: MarkKind, v: Value) -> Value {
    content(kind, &v)
}

/// One change another computer made, for the app to apply.
#[derive(Clone, Debug, PartialEq)]
pub struct Arrival {
    /// What kind of item.
    pub kind: MarkKind,
    /// Its id.
    pub id: String,
    /// Added, replaced, removed, or brought back.
    pub change: ChangeKind,
    /// The computer whose change it is, by its name, when known.
    pub label: Option<String>,
    /// The item now (none when removed).
    pub value: Option<Item>,
    /// The item as the app had it when the merge began (none when it had
    /// none): the app applies the change only if it still has this.
    pub previous: Option<Item>,
}

/// Another computer's place in a document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OtherPlace {
    /// The computer.
    pub device: DeviceId,
    /// Its name, when known.
    pub label: Option<String>,
    /// When the place was set.
    pub stamp: Stamp,
    /// The place.
    pub place: Place,
}

/// A document on another computer that may be this one (the same DOI or
/// ISBN), to ask about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuggestionInfo {
    /// Its sync id.
    pub sync_id: SyncId,
    /// Its title, when that computer knows one.
    pub title: Option<String>,
    /// A computer that has it, by name.
    pub label: Option<String>,
    /// How many notes it has.
    pub notes: usize,
}

/// What opening a document found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenInfo {
    /// How its sync id was found.
    pub found: Found,
    /// Documents to ask about.
    pub suggestions: Vec<SuggestionInfo>,
}

/// What a merge of the open document did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CycleOutcome {
    /// The document's path key.
    pub key: Option<DocKey>,
    /// Its sync id.
    pub sync_id: Option<SyncId>,
    /// What arrived, to apply.
    pub arrivals: Vec<Arrival>,
    /// The other computers' places, newest first (the places group on).
    pub places: Vec<OtherPlace>,
    /// This computer's place as the folder knows it.
    pub my_place: Option<(Stamp, Place)>,
    /// A place from another computer is new or moved.
    pub places_changed: bool,
    /// Things to say once.
    pub notices: Vec<Notice>,
    /// How sync stands.
    pub status: EngineStatus,
    /// Set when the request opened the document.
    pub opened: Option<OpenInfo>,
}

/// What Sync now did for the documents not open.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AllOutcome {
    /// Documents looked at.
    pub documents: usize,
    /// Documents that took changes from another computer.
    pub changed: usize,
    /// Notes replaced by a newer edit from another computer.
    pub replaced_notes: usize,
    /// Things to say once.
    pub notices: Vec<Notice>,
    /// How sync stands.
    pub status: EngineStatus,
}

/// This computer's side of a document, as the app has it.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    /// The document's path key.
    pub key: DocKey,
    /// Its state: the place, bookmarks, notes, highlights, and deletions.
    pub state: DocState,
    /// Publish the place too (a position save, a document switch, quit),
    /// not only edits: a scan while reading does not write the place every
    /// few seconds.
    pub publish_place: bool,
}

/// Work for the engine.
#[derive(Debug)]
pub enum SyncRequest {
    /// Open the folder with this configuration (or close it, with `None`).
    Configure(Option<Box<EngineConfig>>),
    /// A document opened: find its sync id, then merge it.
    Open {
        /// The identification job.
        identify: Box<Identify>,
        /// The text as read, for its hash.
        text: Option<ropey::Rope>,
        /// The app's side.
        snapshot: Box<Snapshot>,
    },
    /// A document was saved: publish its new hashes (its id stays).
    Reidentify {
        /// The identification job.
        identify: Box<Identify>,
    },
    /// Merge the open document: publish what changed here and take what
    /// changed elsewhere. `force` merges even when nothing seems changed.
    Cycle {
        /// The document.
        sync_id: SyncId,
        /// The app's side.
        snapshot: Box<Snapshot>,
        /// Merge even when no file changed.
        force: bool,
    },
    /// The owner said yes to a suggestion: the document takes `sync_id`.
    Adopt {
        /// The document's new sync id.
        sync_id: SyncId,
        /// The app's side.
        snapshot: Box<Snapshot>,
    },
    /// The owner said no to a suggestion.
    Decline {
        /// The document.
        key: DocKey,
        /// The suggestion's sync id.
        sync_id: SyncId,
    },
    /// Merge the groups that are not about one document (settings,
    /// profiles, keys, the word list, the glossary, favorite voices).
    Groups(Box<crate::sync_groups::GroupsRequest>),
    /// The owner edited a document's details by hand (W7m): publish the
    /// fields changed.
    EditDetails {
        /// The identification job, to find the document's sync id.
        identify: Box<Identify>,
        /// The fields changed.
        edits: Vec<textweaver_sync::DetailEdit>,
    },
    /// Merge every document this computer knows, except `skip` (the open
    /// one, which the app merges itself).
    All {
        /// The open document.
        skip: Option<DocKey>,
    },
}

/// The engine's answer.
#[derive(Debug)]
pub enum SyncResponse {
    /// The folder was opened, or closed.
    Configured {
        /// How sync stands.
        status: EngineStatus,
        /// Things to say once.
        notices: Vec<Notice>,
    },
    /// A document was merged (or opened).
    Cycle(Box<CycleOutcome>),
    /// Nothing changed.
    Idle {
        /// The document.
        key: DocKey,
        /// How sync stands.
        status: EngineStatus,
    },
    /// Sync now's pass over the other documents.
    All(Box<AllOutcome>),
    /// The groups were merged: settings, profiles, keys, the word list,
    /// the glossary, and favorite voices.
    Groups(Box<crate::sync_groups::GroupsOutcome>),
    /// Nothing to report (a declined suggestion, a new hash published).
    Done {
        /// How sync stands.
        status: EngineStatus,
    },
}

/// One document's record, as this session knows it.
#[derive(Debug)]
struct Slot {
    /// This computer's merged view: the base.
    mine: DocRecord,
    /// What was last written (or read) for it, to write only changes.
    written: Option<Vec<u8>>,
    /// The other computers' files when last merged.
    others: Vec<FileSig>,
    /// What the last merge sent the app to apply: each item and the
    /// content the app had then. Until the app's state shows it applied, a
    /// difference from the base is not an edit made here, and the arrival
    /// is sent again.
    pending: Vec<Pending>,
}

/// An arrival sent to the app and not yet seen applied.
#[derive(Clone, Debug, PartialEq)]
struct Pending {
    kind: MarkKind,
    id: String,
    /// The content the app had (none when it had no such item).
    before: Option<Value>,
}

/// A file's size and modification time, to notice a change without
/// reading it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct FileSig {
    device: DeviceId,
    len: u64,
    modified: Option<std::time::SystemTime>,
}

/// The engine. One per app; it lives on the writer thread's side.
#[derive(Debug, Default)]
pub struct SyncEngine {
    pub(crate) config: Option<EngineConfig>,
    pub(crate) folder: Option<SyncFolder>,
    identity: Option<Identity>,
    pub(crate) clock: Option<Clock>,
    label: String,
    pub(crate) labels: BTreeMap<DeviceId, String>,
    docs: HashMap<SyncId, Slot>,
    /// Each group's merged view (`crate::sync_groups`).
    pub(crate) groups: BTreeMap<textweaver_sync::GroupFile, crate::sync_groups::GroupSlot>,
    reported: HashSet<String>,
    index: Option<(Vec<(String, u64)>, IdentityIndex)>,
    pub(crate) status: EngineStatus,
}

/// A local edit found by comparing the app's state with the base.
#[derive(Debug)]
struct LocalEdit {
    kind: MarkKind,
    id: String,
    /// The new version, or `None` for a deletion.
    value: Option<Item>,
    /// When it was made, by this computer's wall clock (milliseconds).
    wall_ms: u64,
}

fn secs_to_ms(ts: i64) -> u64 {
    u64::try_from(ts.max(0)).unwrap_or(0).saturating_mul(1000)
}

/// A document comment (Word, OpenDocument) that became a note and was
/// never edited: it comes from the document itself, on every computer, so
/// it is not published.
fn unedited_comment(n: &Note) -> bool {
    n.id.starts_with("comment-") && n.ts == n.created
}

impl SyncEngine {
    /// A new engine, off.
    pub fn new() -> Self {
        Self::default()
    }

    /// How sync stands.
    pub fn status(&self) -> &EngineStatus {
        &self.status
    }

    /// This computer's id, once the folder is open.
    pub fn device(&self) -> Option<DeviceId> {
        self.folder.as_ref().map(SyncFolder::device)
    }

    /// Does one request.
    pub fn handle(&mut self, request: SyncRequest) -> SyncResponse {
        match request {
            SyncRequest::Configure(config) => {
                let notices = self.configure(config.map(|c| *c));
                SyncResponse::Configured {
                    status: self.status.clone(),
                    notices,
                }
            }
            SyncRequest::Open {
                identify,
                text,
                snapshot,
            } => self.open(&identify, text.as_ref(), &snapshot),
            SyncRequest::Reidentify { identify } => {
                self.reidentify(&identify);
                SyncResponse::Done {
                    status: self.status.clone(),
                }
            }
            SyncRequest::Cycle {
                sync_id,
                snapshot,
                force,
            } => match self.cycle(sync_id, &snapshot, force) {
                Some(o) => SyncResponse::Cycle(Box::new(o)),
                None => SyncResponse::Idle {
                    key: snapshot.key.clone(),
                    status: self.status.clone(),
                },
            },
            SyncRequest::Adopt { sync_id, snapshot } => {
                if let Some(c) = &self.config
                    && let Err(e) = textweaver_sync::docid::accept(
                        &c.paths.sync_ids_file(),
                        &snapshot.key,
                        sync_id,
                    )
                {
                    log::warn!("sync: cannot keep the chosen document ({e})");
                }
                let mut o = self
                    .cycle(sync_id, &snapshot, true)
                    .unwrap_or_else(|| self.outcome(&snapshot.key, sync_id));
                o.opened = Some(OpenInfo {
                    found: Found::Known,
                    suggestions: Vec::new(),
                });
                SyncResponse::Cycle(Box::new(o))
            }
            SyncRequest::Decline { key, sync_id } => {
                if let Some(c) = &self.config
                    && let Err(e) =
                        textweaver_sync::docid::decline(&c.paths.sync_ids_file(), &key, sync_id)
                {
                    log::warn!("sync: cannot remember the answer ({e})");
                }
                SyncResponse::Done {
                    status: self.status.clone(),
                }
            }
            SyncRequest::EditDetails { identify, edits } => {
                self.edit_details(&identify, &edits);
                SyncResponse::Done {
                    status: self.status.clone(),
                }
            }
            SyncRequest::All { skip } => SyncResponse::All(Box::new(self.sync_all(skip.as_ref()))),
            SyncRequest::Groups(request) => {
                SyncResponse::Groups(Box::new(self.groups_cycle(&request)))
            }
        }
    }

    /// Opens the folder (or closes it with `None`). Returns what to say.
    pub fn configure(&mut self, config: Option<EngineConfig>) -> Vec<Notice> {
        self.folder = None;
        self.clock = None;
        self.docs.clear();
        self.groups.clear();
        self.index = None;
        self.labels.clear();
        self.config = config;
        let mut notices = Vec::new();
        if self.config.is_none() {
            self.status = EngineStatus {
                kind: StatusKind::Off,
                ..EngineStatus::default()
            };
            return notices;
        }
        self.reopen(&mut notices);
        notices
    }

    /// Opens the configured folder, when it is there.
    fn reopen(&mut self, notices: &mut Vec<Notice>) {
        let Some(config) = self.config.clone() else {
            return;
        };
        self.status.failure = None;
        if self.identity.is_none() {
            match Identity::load_or_create(&config.paths.data_dir) {
                Ok((id, event)) => {
                    if event == IdentityEvent::CopiedStateFolder {
                        self.notice(notices, "fresh", Notice::FreshDeviceId);
                    }
                    self.identity = Some(id);
                }
                Err(e) => {
                    self.fail(format!("{e}"));
                    return;
                }
            }
        }
        if !config.folder.is_dir() {
            self.status.kind = StatusKind::FolderMissing;
            return;
        }
        backup_state_once(&config.paths);
        let label = if config.device_name.is_empty() {
            let taken = labels_in(&config.folder, self.identity.as_ref().map(Identity::device));
            textweaver_sync::default_label(taken.iter().map(String::as_str))
        } else {
            config.device_name.clone()
        };
        let Some(identity) = self.identity.as_mut() else {
            return;
        };
        match SyncFolder::open(&config.folder, identity, &label, &config.app_version) {
            Ok(opened) => {
                if opened.fresh_device_id {
                    self.notice(notices, "fresh", Notice::FreshDeviceId);
                }
                self.clock = Some(Clock::new(opened.folder.device()));
                self.status.kind = match opened.folder.read_only() {
                    Some(_) => StatusKind::ReadOnly,
                    None => StatusKind::Ready,
                };
                self.folder = Some(opened.folder);
                self.label = label.clone();
                self.status.label = label;
                self.problems(opened.problems, notices);
                self.refresh_labels();
            }
            Err(SyncError::FolderMissing) => self.status.kind = StatusKind::FolderMissing,
            Err(SyncError::Label(why)) => {
                self.notice(
                    notices,
                    &format!("name:{why}"),
                    Notice::NameRefused(why.to_owned()),
                );
                self.fail(format!("label {why}"));
            }
            Err(e) => self.fail(e.to_string()),
        }
    }

    fn fail(&mut self, why: String) {
        log::warn!("sync: {why}");
        self.status.kind = StatusKind::Failed;
        self.status.failure = Some(why);
    }

    /// Adds `notice` unless it was said in this session.
    pub(crate) fn notice(&mut self, notices: &mut Vec<Notice>, key: &str, notice: Notice) {
        if self.reported.insert(key.to_owned()) {
            notices.push(notice);
        }
    }

    /// Turns the folder's problems into notices, each once per session.
    pub(crate) fn problems(&mut self, problems: Vec<Problem>, notices: &mut Vec<Notice>) {
        for p in problems {
            match p {
                Problem::Damaged { device, file, .. } => {
                    let key = format!("damaged:{device:?}:{file:?}");
                    if self.reported.insert(key) {
                        self.status.damaged += 1;
                        let label = device.and_then(|d| self.labels.get(&d).cloned());
                        notices.push(Notice::Damaged { label });
                    }
                }
                Problem::NewerFormat { device, .. } => {
                    let label = self.labels.get(&device).cloned();
                    self.notice(
                        notices,
                        &format!("newer:{device}"),
                        Notice::NewerFile { label },
                    );
                }
                Problem::ClockAhead(c) => {
                    let label = self.labels.get(&c.device).cloned();
                    let name = label.clone().unwrap_or_else(|| c.device.to_string());
                    if !self.status.clocks_ahead.contains(&name) {
                        self.status.clocks_ahead.push(name);
                    }
                    self.notice(
                        notices,
                        &format!("ahead:{}", c.device),
                        Notice::ClockAhead {
                            label,
                            hours: c.ahead_ms / 3_600_000,
                        },
                    );
                }
                Problem::ReadOnly(_) => {
                    self.status.kind = StatusKind::ReadOnly;
                    self.notice(notices, "readonly", Notice::ReadOnly);
                }
            }
        }
    }

    /// Reads every computer's name.
    pub(crate) fn refresh_labels(&mut self) {
        let Some(folder) = &self.folder else {
            return;
        };
        let me = folder.device();
        let (infos, _) = folder.devices();
        self.labels = infos.into_iter().map(|i| (i.device, i.label)).collect();
        self.labels.insert(me, self.label.clone());
        self.status.others = self
            .labels
            .iter()
            .filter(|(d, _)| **d != me)
            .map(|(_, l)| l.clone())
            .collect();
    }

    /// True when the folder is open (reopening it when it came back).
    pub(crate) fn ready(&mut self, notices: &mut Vec<Notice>) -> bool {
        let Some(config) = &self.config else {
            return false;
        };
        if !config.folder.is_dir() {
            if self.folder.is_some() {
                log::info!("sync: the folder is gone; saving here until it is back");
            }
            self.folder = None;
            self.status.kind = StatusKind::FolderMissing;
            return false;
        }
        if self.folder.is_none() {
            self.reopen(notices);
        }
        self.folder.is_some()
    }

    fn outcome(&self, key: &DocKey, sync_id: SyncId) -> CycleOutcome {
        CycleOutcome {
            key: Some(key.clone()),
            sync_id: Some(sync_id),
            status: self.status.clone(),
            ..CycleOutcome::default()
        }
    }

    /// The other computers' records, as an index for recognizing
    /// documents; read again only when a file changed.
    fn identity_index(&mut self, notices: &mut Vec<Notice>) -> Option<IdentityIndex> {
        let sig = all_docs_signature(folder_root(self.config.as_ref()?).as_path());
        let stale = self.index.as_ref().is_none_or(|(s, _)| *s != sig);
        if stale {
            let (index, problems) = self.folder.as_mut()?.identity_index();
            self.problems(problems, notices);
            self.index = Some((sig, index));
        }
        self.index.as_ref().map(|(_, i)| i.clone())
    }

    fn open(
        &mut self,
        identify: &Identify,
        text: Option<&ropey::Rope>,
        snapshot: &Snapshot,
    ) -> SyncResponse {
        let mut notices = Vec::new();
        let ready = self.ready(&mut notices);
        let index = if ready {
            self.identity_index(&mut notices)
        } else {
            None
        };
        let resolved = match identify.run(text.map(ropey::Rope::chunks), index.as_ref()) {
            Ok(r) => r,
            Err(e) => {
                log::warn!("sync: cannot identify a document ({e})");
                return SyncResponse::Idle {
                    key: snapshot.key.clone(),
                    status: self.status.clone(),
                };
            }
        };
        if !ready {
            let mut o = self.outcome(&snapshot.key, resolved.sync_id);
            o.notices = notices;
            o.opened = Some(OpenInfo {
                found: resolved.found,
                suggestions: Vec::new(),
            });
            return SyncResponse::Cycle(Box::new(o));
        }
        // The library details (S6): what the document states, and when it
        // was first added to the bookshelf here (written before this job,
        // on the same writer).
        let mut details = identify.details.clone();
        if details.added_ms.is_none()
            && let Some(c) = &self.config
        {
            details.added_ms = first_added_ms(&c.paths, &identify.path);
        }
        let publish = self.slot(resolved.sync_id).is_some_and(|slot| {
            resolved.changed
                || slot.mine.identity.is_empty()
                || details.would_change(&slot.mine.identity)
        });
        if publish && let Some(clock) = self.clock.as_mut() {
            let stamp = clock.tick();
            if let Some(slot) = self.docs.get_mut(&resolved.sync_id) {
                slot.mine
                    .publish_identity(stamp, &resolved.fingerprint, &details);
            }
        }
        let mut o = self
            .cycle(resolved.sync_id, snapshot, true)
            .unwrap_or_else(|| self.outcome(&snapshot.key, resolved.sync_id));
        o.notices.splice(0..0, notices);
        let suggestions = resolved
            .suggestions
            .iter()
            .map(|s| SuggestionInfo {
                sync_id: s.sync_id,
                title: s.title.clone(),
                label: s.devices.iter().find_map(|d| self.labels.get(d).cloned()),
                notes: s.notes,
            })
            .collect();
        o.opened = Some(OpenInfo {
            found: resolved.found,
            suggestions,
        });
        SyncResponse::Cycle(Box::new(o))
    }

    fn reidentify(&mut self, identify: &Identify) {
        let mut notices = Vec::new();
        if !self.ready(&mut notices) {
            identify.run_logged(None::<std::iter::Empty<&str>>);
            return;
        }
        let index = self.identity_index(&mut notices);
        let resolved = match identify.run(None::<std::iter::Empty<&str>>, index.as_ref()) {
            Ok(r) => r,
            Err(e) => {
                log::warn!("sync: cannot identify a document ({e})");
                return;
            }
        };
        if !resolved.changed || self.slot(resolved.sync_id).is_none() {
            return;
        }
        let Some(clock) = self.clock.as_mut() else {
            return;
        };
        let stamp = clock.tick();
        if let Some(slot) = self.docs.get_mut(&resolved.sync_id) {
            slot.mine
                .publish_identity(stamp, &resolved.fingerprint, &identify.details);
        }
        self.write(resolved.sync_id, &mut notices);
    }

    /// Publishes the owner's hand edits of a document's details (W7m): the
    /// document's sync id is found (or made) as when it opens, and only
    /// the fields in `edits` are stamped, after every other computer's
    /// stamps for it, so the edit is the newest. Returns true when the
    /// edit was written to the sync folder.
    ///
    /// Reads and hashes the file when it is new here, so call it off the
    /// input thread (the reader sends [`SyncRequest::EditDetails`]).
    pub fn edit_details(
        &mut self,
        identify: &Identify,
        edits: &[textweaver_sync::DetailEdit],
    ) -> bool {
        let mut notices = Vec::new();
        if !self.ready(&mut notices) {
            return false;
        }
        let index = self.identity_index(&mut notices);
        let resolved = match identify.run(None::<std::iter::Empty<&str>>, index.as_ref()) {
            Ok(r) => r,
            Err(e) => {
                log::warn!("sync: cannot identify a document to edit ({e})");
                return false;
            }
        };
        let sync_id = resolved.sync_id;
        if self.slot(sync_id).is_none() {
            return false;
        }
        // Only observed, never merged here: the open document's merge
        // weighs the app's state against this computer's view.
        let others = self
            .folder
            .as_mut()
            .map(|f| f.read_doc(sync_id).records)
            .unwrap_or_default();
        let Some(clock) = self.clock.as_mut() else {
            return false;
        };
        for (_, r) in &others {
            for s in r.identity.stamps() {
                clock.observe(s);
            }
        }
        let stamp = clock.tick();
        let Some(slot) = self.docs.get_mut(&sync_id) else {
            return false;
        };
        let mut changed = slot.mine.publish_edits(stamp, edits);
        if resolved.changed {
            slot.mine
                .publish_identity(stamp, &resolved.fingerprint, &identify.details);
            changed = true;
        }
        if !changed {
            return false;
        }
        self.write(sync_id, &mut notices);
        self.status.write_error.is_none()
            && self
                .folder
                .as_ref()
                .is_some_and(|f| f.read_only().is_none())
    }

    /// The slot for `sync_id`, read from this computer's own file the first
    /// time.
    fn slot(&mut self, sync_id: SyncId) -> Option<&mut Slot> {
        if !self.docs.contains_key(&sync_id) {
            let folder = self.folder.as_mut()?;
            let me = folder.device();
            let read = folder.read_doc(sync_id);
            let mine = read
                .records
                .into_iter()
                .find(|(d, _)| *d == me)
                .map_or_else(|| DocRecord::new(sync_id), |(_, r)| r);
            if let Some(clock) = self.clock.as_mut() {
                for s in mine.stamps() {
                    clock.observe(s);
                }
            }
            let written = mine.to_bytes().ok();
            self.docs.insert(
                sync_id,
                Slot {
                    mine,
                    written,
                    others: Vec::new(),
                    pending: Vec::new(),
                },
            );
        }
        self.docs.get_mut(&sync_id)
    }

    /// The other computers' files for `sync_id`, as sizes and times.
    fn others_signature(&self, sync_id: SyncId) -> Vec<FileSig> {
        let (Some(folder), Some(config)) = (&self.folder, &self.config) else {
            return Vec::new();
        };
        let me = folder.device();
        let root = folder_root(config);
        folder
            .device_ids()
            .into_iter()
            .filter(|d| *d != me)
            .map(|d| {
                let path = root
                    .join(DEVICES_DIR)
                    .join(d.to_string())
                    .join(DOCS_DIR)
                    .join(format!("{sync_id}.json"));
                let meta = std::fs::metadata(&path).ok();
                FileSig {
                    device: d,
                    len: meta.as_ref().map_or(0, std::fs::Metadata::len),
                    modified: meta.and_then(|m| m.modified().ok()),
                }
            })
            .collect()
    }

    /// Merges one document: publishes what changed here, takes what changed
    /// elsewhere. `None` when nothing changed anywhere (and `force` is off).
    pub fn cycle(
        &mut self,
        sync_id: SyncId,
        snapshot: &Snapshot,
        force: bool,
    ) -> Option<CycleOutcome> {
        let mut notices = Vec::new();
        if !self.ready(&mut notices) {
            if force || !notices.is_empty() {
                let mut o = self.outcome(&snapshot.key, sync_id);
                o.notices = notices;
                return Some(o);
            }
            return None;
        }
        let groups = self.config.as_ref().map_or(Groups::ALL, |c| c.groups);
        let others = self.others_signature(sync_id);
        let me = self.folder.as_ref()?.device();
        let place_now = groups.places.then(|| place_of(&snapshot.state));
        let slot = self.slot(sync_id)?;
        let edits = local_edits(&snapshot.state, &slot.mine, groups, me, &slot.pending);
        let place_changed = snapshot.publish_place
            && place_now
                .as_ref()
                .is_some_and(|p| slot.mine.place_of(me).is_none_or(|q| q.pos != p.pos));
        if !force && edits.is_empty() && !place_changed && others == slot.others {
            return None;
        }

        // The computers' names, for saying who changed what.
        self.refresh_labels();
        // Every other computer's record, into the base.
        let (folder, clock) = (self.folder.as_mut()?, self.clock.as_mut()?);
        let slot = self.docs.get_mut(&sync_id)?;
        let merged = folder.merge_doc(&mut slot.mine, clock);
        slot.others = others;
        let mut report = merged.report;

        // Local edits, unless another computer's newer edit to the same
        // item just arrived.
        for e in edits {
            let item = mark_to_item(e.kind);
            let remote = report
                .changes
                .iter()
                .position(|c| c.item == item && c.id == e.id && c.by != me);
            if let Some(i) = remote {
                let theirs = register_time(&slot.mine, e.kind, &e.id);
                if e.wall_ms <= theirs {
                    continue;
                }
                report.changes.remove(i);
            }
            let stamp = clock.tick();
            match e.value {
                Some(Item::Bookmark(b)) => slot.mine.bookmarks.set(e.id, stamp, b),
                Some(Item::Note(n)) => slot.mine.notes.set(e.id, stamp, *n),
                Some(Item::Highlight(h)) => slot.mine.highlights.set(e.id, stamp, h),
                None => match e.kind {
                    MarkKind::Bookmark => slot.mine.bookmarks.delete(e.id, stamp),
                    MarkKind::Note => slot.mine.notes.delete(e.id, stamp),
                    MarkKind::Highlight => slot.mine.highlights.delete(e.id, stamp),
                },
            }
        }
        if place_changed && let Some(p) = place_now {
            slot.mine.set_place(clock.tick(), p);
        }
        if groups.statistics
            && let Some(c) = &self.config
        {
            publish_stats(&mut slot.mine, me, &c.paths, &snapshot.key);
        }

        let places_changed = report
            .changes
            .iter()
            .any(|c| c.item == ItemKind::Place && c.by != me);
        let arrivals = arrivals(
            &report,
            &slot.mine,
            &snapshot.state,
            groups,
            &self.labels,
            &slot.pending,
        );
        slot.pending = arrivals
            .iter()
            .map(|a| Pending {
                kind: a.kind,
                id: a.id.clone(),
                before: a.previous.as_ref().map(|p| strip(a.kind, p.json())),
            })
            .collect();
        let my_place = slot
            .mine
            .places
            .register(&me.to_string())
            .and_then(|r| r.value.clone().map(|v| (r.stamp, v)));
        self.problems(merged.problems, &mut notices);
        self.write(sync_id, &mut notices);
        let places = self.places(sync_id, groups);
        let mut o = self.outcome(&snapshot.key, sync_id);
        o.arrivals = arrivals;
        o.places = places;
        o.my_place = my_place;
        o.places_changed = places_changed;
        o.notices = notices;
        Some(o)
    }

    /// The other computers' places in `sync_id`, newest first.
    fn places(&self, sync_id: SyncId, groups: Groups) -> Vec<OtherPlace> {
        let (Some(slot), Some(folder)) = (self.docs.get(&sync_id), &self.folder) else {
            return Vec::new();
        };
        if !groups.places {
            return Vec::new();
        }
        let me = folder.device();
        slot.mine
            .places_newest_first()
            .into_iter()
            .filter(|(s, _)| s.device != me)
            .map(|(stamp, place)| OtherPlace {
                device: stamp.device,
                label: self.labels.get(&stamp.device).cloned(),
                stamp,
                place: place.clone(),
            })
            .collect()
    }

    /// Writes this computer's view of `sync_id` when it changed.
    fn write(&mut self, sync_id: SyncId, notices: &mut Vec<Notice>) {
        let (Some(folder), Some(slot)) = (&self.folder, self.docs.get_mut(&sync_id)) else {
            return;
        };
        if folder.read_only().is_some() {
            return;
        }
        let Ok(bytes) = slot.mine.to_bytes() else {
            return;
        };
        if slot.written.as_ref() == Some(&bytes) {
            return;
        }
        match folder.write_doc(&slot.mine) {
            Ok(()) => {
                slot.written = Some(bytes);
                if self.status.write_error.take().is_some() {
                    self.reported.retain(|k| !k.starts_with("write:"));
                }
            }
            Err(e) => {
                let why = e.to_string();
                log::warn!("sync: cannot write ({why})");
                self.status.write_error = Some(why.clone());
                self.notice(notices, &format!("write:{why}"), Notice::WriteFailed(why));
            }
        }
    }

    /// Merges every document this computer has a sync id for (in
    /// `sync-ids.json`), except `skip`, straight into its state file. For
    /// Sync now and `tw sync now`. Documents are not open, so arriving
    /// items keep the place they had on the other computer; the reader
    /// finds them again when the document opens and its text differs.
    pub fn sync_all(&mut self, skip: Option<&DocKey>) -> AllOutcome {
        let mut out = AllOutcome::default();
        let mut notices = Vec::new();
        if !self.ready(&mut notices) {
            out.notices = notices;
            out.status = self.status.clone();
            return out;
        }
        let Some(config) = self.config.clone() else {
            return out;
        };
        let ids = SyncIds::load(&config.paths.sync_ids_file());
        let store = StateStore::new(config.paths.state_dir());
        for (key, entry) in &ids.docs {
            let key = DocKey(key.clone());
            if skip == Some(&key) {
                continue;
            }
            let Ok(sync_id) = entry.sync_id.parse::<SyncId>() else {
                continue;
            };
            let mut state = store.load(&key).unwrap_or_default();
            let snapshot = Snapshot {
                key: key.clone(),
                state: state.clone(),
                publish_place: true,
            };
            out.documents += 1;
            let Some(o) = self.cycle(sync_id, &snapshot, false) else {
                continue;
            };
            notices.extend(o.notices);
            if o.arrivals.is_empty() {
                continue;
            }
            let applied = apply_arrivals(&mut state, &o.arrivals, &mut |_| true);
            if applied.changed() {
                out.changed += 1;
                out.replaced_notes += applied.replaced_notes.len();
                if let Err(e) = store.save(&key, &state) {
                    log::warn!("sync: cannot save a merged document ({e})");
                }
            }
        }
        out.notices = notices;
        out.status = self.status.clone();
        out
    }
}

/// The sync status line, meaning first: "Sync: up to date", "Sync: folder
/// missing, saving here", "Sync: newer format, read only", "Sync: lab's
/// clock is ahead". `on` is whether sync is set up and on here.
pub fn status_line(
    c: &textweaver_lexicon::i18n::Catalog,
    settings: &SyncSettings,
    on: bool,
    st: &EngineStatus,
) -> String {
    use textweaver_lexicon::args;
    if !on {
        return c.tr(if settings.folder.is_some() {
            "sync-status-off"
        } else {
            "sync-status-not-set-up"
        });
    }
    match st.kind {
        StatusKind::Off | StatusKind::Starting => c.tr("sync-status-starting"),
        StatusKind::FolderMissing => c.tr("sync-status-folder-missing"),
        StatusKind::ReadOnly => c.tr("sync-status-read-only"),
        StatusKind::Failed => c.tr("sync-status-failed"),
        StatusKind::Ready => {
            if st.write_error.is_some() {
                c.tr("sync-status-cannot-write")
            } else if let Some(who) = st.clocks_ahead.first() {
                c.fmt("sync-status-clock-ahead", &args!["device" => who.as_str()])
            } else if st.damaged > 0 {
                c.fmt("sync-status-damaged", &args!["n" => st.damaged])
            } else {
                c.tr("sync-status-up-to-date")
            }
        }
    }
}

/// What to say for a notice, and at what level (ADR-0043): write failures
/// are errors, the rest results.
pub fn notice_text(
    c: &textweaver_lexicon::i18n::Catalog,
    n: &Notice,
) -> (String, textweaver_a11y::Importance) {
    use textweaver_a11y::Importance;
    use textweaver_lexicon::args;
    let name = |label: &Option<String>| {
        label
            .clone()
            .unwrap_or_else(|| c.tr("sync-another-computer"))
    };
    match n {
        Notice::Damaged { label } => (
            c.fmt("sync-damaged", &args!["device" => name(label)]),
            Importance::Result,
        ),
        Notice::NewerFile { label } => (
            c.fmt("sync-newer-file", &args!["device" => name(label)]),
            Importance::Result,
        ),
        Notice::ReadOnly => (c.tr("sync-read-only"), Importance::Result),
        Notice::ClockAhead { label, hours } => (
            c.fmt(
                "sync-clock-ahead",
                &args!["device" => name(label), "hours" => *hours],
            ),
            Importance::Result,
        ),
        Notice::FreshDeviceId => (c.tr("sync-fresh-id"), Importance::Result),
        Notice::WriteFailed(error) => (
            c.fmt("sync-write-failed", &args!["error" => error.as_str()]),
            Importance::Error,
        ),
        Notice::NameRefused(_) => (c.tr("sync-name-refused"), Importance::Error),
    }
}

/// The sync folder's own folder inside the chosen one.
fn folder_root(config: &EngineConfig) -> PathBuf {
    config.folder.join(SYNC_DIR)
}

/// The names in the folder's `device.json` files, other than `me`'s, for
/// choosing a free "Computer N".
fn labels_in(folder: &Path, me: Option<DeviceId>) -> Vec<String> {
    let devices = folder.join(SYNC_DIR).join(DEVICES_DIR);
    std::fs::read_dir(devices)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_str()
                .and_then(|n| n.parse::<DeviceId>().ok())
                .is_some_and(|d| Some(d) != me)
        })
        .filter_map(|e| {
            let path = e.path().join(DEVICE_FILE);
            let len = std::fs::metadata(&path).ok()?.len();
            if len > 64 * 1024 {
                return None;
            }
            let bytes = std::fs::read(path).ok()?;
            serde_json::from_slice::<DeviceInfo>(&bytes)
                .ok()
                .map(|i| i.label)
        })
        .collect()
}

/// The names in the folder's `device.json` files, for suggesting a free
/// "Computer N" when the folder is chosen.
pub fn labels_elsewhere(folder: &Path) -> Vec<String> {
    labels_in(folder, None)
}

/// Every computer's document files, by name and size, to notice when the
/// identity index must be read again.
fn all_docs_signature(root: &Path) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    for dev in std::fs::read_dir(root.join(DEVICES_DIR))
        .into_iter()
        .flatten()
        .flatten()
    {
        for f in std::fs::read_dir(dev.path().join(DOCS_DIR))
            .into_iter()
            .flatten()
            .flatten()
        {
            let meta = f.metadata().ok();
            let modified = meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_millis());
            out.push((
                format!(
                    "{}/{}/{modified}",
                    dev.file_name().to_string_lossy(),
                    f.file_name().to_string_lossy()
                ),
                meta.map_or(0, |m| m.len()),
            ));
        }
    }
    out.sort();
    out
}

/// Copies the state files once, before the first merge, so turning sync on
/// can be undone. Only copies; never deletes.
fn backup_state_once(paths: &Paths) {
    let backup = paths.data_dir.join(STATE_BACKUP_DIR);
    if backup.exists() {
        return;
    }
    if let Err(e) = std::fs::create_dir_all(&backup) {
        log::warn!("sync: cannot back up the state folder ({e})");
        return;
    }
    for e in std::fs::read_dir(paths.state_dir())
        .into_iter()
        .flatten()
        .flatten()
    {
        if e.file_type().is_ok_and(|t| t.is_file())
            && let Err(err) = std::fs::copy(e.path(), backup.join(e.file_name()))
        {
            log::warn!("sync: cannot back up a state file ({err})");
        }
    }
}

/// This computer's place, from its state.
fn place_of(state: &DocState) -> Place {
    Place {
        pos: state.position,
        pct: state.pct,
        anchor: state.anchor.clone(),
    }
}

fn mark_to_item(kind: MarkKind) -> ItemKind {
    match kind {
        MarkKind::Bookmark => ItemKind::Bookmark,
        MarkKind::Note => ItemKind::Note,
        MarkKind::Highlight => ItemKind::Highlight,
    }
}

fn item_to_mark(kind: ItemKind) -> Option<MarkKind> {
    match kind {
        ItemKind::Bookmark => Some(MarkKind::Bookmark),
        ItemKind::Note => Some(MarkKind::Note),
        ItemKind::Highlight => Some(MarkKind::Highlight),
        ItemKind::Place => None,
    }
}

/// The time of the register of item `id` in `record`, 0 when none.
fn register_time(record: &DocRecord, kind: MarkKind, id: &str) -> u64 {
    match kind {
        MarkKind::Bookmark => record.bookmarks.register(id).map(|r| r.stamp.time),
        MarkKind::Note => record.notes.register(id).map(|r| r.stamp.time),
        MarkKind::Highlight => record.highlights.register(id).map(|r| r.stamp.time),
    }
    .unwrap_or(0)
}

/// The record's live item `id`.
fn record_item(record: &DocRecord, kind: MarkKind, id: &str) -> Option<Item> {
    match kind {
        MarkKind::Bookmark => record.bookmarks.get(id).cloned().map(Item::Bookmark),
        MarkKind::Note => record
            .notes
            .get(id)
            .cloned()
            .map(|n| Item::Note(Box::new(n))),
        MarkKind::Highlight => record.highlights.get(id).cloned().map(Item::Highlight),
    }
}

/// The state's item `id`.
pub fn state_item(state: &DocState, kind: MarkKind, id: &str) -> Option<Item> {
    match kind {
        MarkKind::Bookmark => state
            .bookmarks
            .iter()
            .find(|b| b.id == id)
            .cloned()
            .map(Item::Bookmark),
        MarkKind::Note => state
            .notes
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .map(|n| Item::Note(Box::new(n))),
        MarkKind::Highlight => state
            .highlights
            .iter()
            .find(|h| h.id == id)
            .cloned()
            .map(Item::Highlight),
    }
}

/// The local edits: items new or changed here since the base, and items
/// deleted here that the base still has.
fn local_edits(
    state: &DocState,
    base: &DocRecord,
    groups: Groups,
    me: DeviceId,
    pending: &[Pending],
) -> Vec<LocalEdit> {
    let mut out = Vec::new();
    if groups.bookmarks {
        diff(
            MarkKind::Bookmark,
            state.bookmarks.iter().map(|b| (b.id.as_str(), b.ts, b)),
            &base.bookmarks,
            (state, me, pending),
            |b| Item::Bookmark(b.clone()),
            &mut out,
        );
    }
    if groups.notes {
        diff(
            MarkKind::Note,
            state
                .notes
                .iter()
                .filter(|n| !unedited_comment(n))
                .map(|n| (n.id.as_str(), n.ts, n)),
            &base.notes,
            (state, me, pending),
            |n| Item::Note(Box::new(n.clone())),
            &mut out,
        );
    }
    if groups.highlights {
        diff(
            MarkKind::Highlight,
            state.highlights.iter().map(|h| (h.id.as_str(), h.ts, h)),
            &base.highlights,
            (state, me, pending),
            |h| Item::Highlight(h.clone()),
            &mut out,
        );
    }
    out
}

fn diff<'a, T: Serialize + Clone + PartialEq + 'a>(
    kind: MarkKind,
    local: impl Iterator<Item = (&'a str, i64, &'a T)>,
    base: &RegisterMap<T>,
    (state, me, pending): (&DocState, DeviceId, &[Pending]),
    wrap: impl Fn(&T) -> Item,
    out: &mut Vec<LocalEdit>,
) {
    let mut here = HashSet::new();
    for (id, ts, item) in local {
        if id.is_empty() {
            continue;
        }
        here.insert(id.to_owned());
        // An arrival the app has not applied yet is not an edit here.
        let unapplied = pending.iter().any(|p| {
            p.kind == kind && p.id == id && p.before.as_ref() == Some(&content(kind, item))
        });
        if unapplied {
            continue;
        }
        let changed = match base.register(id) {
            None => true,
            Some(r) => match &r.value {
                Some(v) => content(kind, v) != content(kind, item),
                // Deleted in the base: only an edit made after the deletion
                // brings it back.
                None => secs_to_ms(ts) > r.stamp.time,
            },
        };
        if changed {
            out.push(LocalEdit {
                kind,
                id: id.to_owned(),
                value: Some(wrap(item)),
                wall_ms: secs_to_ms(ts),
            });
        }
    }
    for (id, _) in base.live() {
        if here.contains(id) {
            continue;
        }
        let Some(d) = state.deleted.get(kind, id) else {
            continue;
        };
        let wall = u64::try_from(d.clock.wall_ms.max(0)).unwrap_or(0);
        let deleted_after = base
            .register(id)
            .is_some_and(|r| r.stamp.device == me || wall >= r.stamp.time);
        if deleted_after {
            out.push(LocalEdit {
                kind,
                id: id.to_owned(),
                value: None,
                wall_ms: wall,
            });
        }
    }
}

/// What arrived: the merge's changes to bookmarks, notes, and highlights,
/// with the version the app has, plus items the base has that the app
/// lacks (a state saved over a merge, a group turned back on).
fn arrivals(
    report: &textweaver_sync::MergeReport,
    mine: &DocRecord,
    state: &DocState,
    groups: Groups,
    labels: &BTreeMap<DeviceId, String>,
    pending: &[Pending],
) -> Vec<Arrival> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for c in &report.changes {
        let Some(kind) = item_to_mark(c.item) else {
            continue;
        };
        if !groups.has(kind) {
            continue;
        }
        let previous = state_item(state, kind, &c.id);
        let value = record_item(mine, kind, &c.id);
        seen.insert((kind_key(kind), c.id.clone()));
        let change = match (&previous, &value) {
            (None, None) => continue,
            (Some(p), Some(v)) if same_content(p, v) => continue,
            (None, Some(_)) if c.kind == ChangeKind::Restored => ChangeKind::Restored,
            (None, Some(_)) => ChangeKind::Added,
            (Some(_), None) => ChangeKind::Removed,
            (Some(_), Some(_)) => ChangeKind::Replaced,
        };
        out.push(Arrival {
            kind,
            id: c.id.clone(),
            change,
            label: labels.get(&c.by).cloned(),
            value,
            previous,
        });
    }
    // Items the base has and the app lacks, or has though the base deleted
    // them and nothing here changed them since.
    for kind in [MarkKind::Bookmark, MarkKind::Note, MarkKind::Highlight] {
        if !groups.has(kind) {
            continue;
        }
        let registers: Vec<(String, Stamp, Option<Item>)> = match kind {
            MarkKind::Bookmark => regs(&mine.bookmarks, |b| Item::Bookmark(b.clone())),
            MarkKind::Note => regs(&mine.notes, |n| Item::Note(Box::new(n.clone()))),
            MarkKind::Highlight => regs(&mine.highlights, |h| Item::Highlight(h.clone())),
        };
        for (id, stamp, value) in registers {
            if seen.contains(&(kind_key(kind), id.clone())) {
                continue;
            }
            let previous = state_item(state, kind, &id);
            let unapplied = pending.iter().any(|p| {
                p.kind == kind
                    && p.id == id
                    && p.before == previous.as_ref().map(|x| strip(kind, x.json()))
            });
            let change = match (&previous, &value) {
                (None, Some(_)) if state.deleted.get(kind, &id).is_none() => ChangeKind::Added,
                (None, Some(_)) if unapplied => ChangeKind::Restored,
                (Some(_), None) => ChangeKind::Removed,
                // Sent before and not applied yet: sent again.
                (Some(p), Some(v)) if unapplied && !same_content(p, v) => ChangeKind::Replaced,
                _ => continue,
            };
            out.push(Arrival {
                kind,
                id,
                change,
                label: labels.get(&stamp.device).cloned(),
                value,
                previous,
            });
        }
    }
    out
}

fn kind_key(kind: MarkKind) -> u8 {
    match kind {
        MarkKind::Bookmark => 0,
        MarkKind::Note => 1,
        MarkKind::Highlight => 2,
    }
}

fn regs<T: Serialize + Clone + PartialEq>(
    map: &RegisterMap<T>,
    wrap: impl Fn(&T) -> Item,
) -> Vec<(String, Stamp, Option<Item>)> {
    map.0
        .iter()
        .map(|(id, r)| (id.clone(), r.stamp, r.value.as_ref().map(&wrap)))
        .collect()
}

/// When the bookshelf here first recorded `path` (milliseconds since 1970),
/// for the library details' "first added".
fn first_added_ms(paths: &Paths, path: &Path) -> Option<u64> {
    let lib = textweaver_store::Library::load(&paths.library_file()).ok()?;
    let added = lib.get(path)?.added;
    (added > 0).then(|| secs_to_ms(added))
}

/// Sets this computer's reading statistics for the document in its record.
fn publish_stats(record: &mut DocRecord, me: DeviceId, paths: &Paths, key: &DocKey) {
    let Ok(stats) = ReadingStats::load(paths) else {
        return;
    };
    let Some(d) = stats.documents.get(key.as_str()) else {
        return;
    };
    let seconds = d.seconds.max(0.0).round() as u64;
    let s = &mut record.stats;
    let c = s.seconds.0.entry(me).or_default();
    *c = (*c).max(seconds);
    let c = s.sessions.0.entry(me).or_default();
    *c = (*c).max(u64::from(d.sessions));
    s.furthest_percent.raise(u64::from(d.furthest_percent));
    s.furthest_char.raise(d.furthest_char);
    s.last_read.raise(secs_to_ms(d.last_read));
}

/// What applying arrivals did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Applied {
    /// Items added, by kind.
    pub added: Vec<(MarkKind, Option<String>)>,
    /// Notes replaced by a newer edit: the old text and the computer.
    pub replaced_notes: Vec<(String, Option<String>)>,
    /// Bookmarks and highlights replaced by a newer edit.
    pub replaced_other: usize,
    /// Items removed.
    pub removed: Vec<(MarkKind, Option<String>)>,
    /// Items a newer edit brought back after they were deleted here.
    pub restored: Vec<(MarkKind, Option<String>)>,
    /// Arrivals not applied because the item changed here meanwhile.
    pub skipped: usize,
}

impl Applied {
    /// Whether anything changed.
    pub fn changed(&self) -> bool {
        !(self.added.is_empty()
            && self.replaced_notes.is_empty()
            && self.replaced_other == 0
            && self.removed.is_empty()
            && self.restored.is_empty())
    }
}

/// Applies arrivals to `state` (bookmarks, notes, highlights, and the
/// local backup of replaced notes). `place` finds an arriving item again in
/// this computer's text, and says whether to keep it (a highlight whose
/// text is gone is dropped). An arrival whose item changed here since the
/// merge began is skipped: the next merge settles it.
pub fn apply_arrivals(
    state: &mut DocState,
    arrivals: &[Arrival],
    place: &mut dyn FnMut(&mut Item) -> bool,
) -> Applied {
    let mut out = Applied::default();
    for a in arrivals {
        let now = state_item(state, a.kind, &a.id);
        let unchanged = match (&now, &a.previous) {
            (None, None) => true,
            (Some(x), Some(y)) => same_content(x, y),
            _ => false,
        };
        if !unchanged {
            out.skipped += 1;
            continue;
        }
        match (&a.value, a.change) {
            (None, _) | (_, ChangeKind::Removed) => {
                if let Some(Item::Note(n)) = &now {
                    state.backup_note(n, a.label.as_deref().unwrap_or(""), true);
                }
                remove_item(state, a.kind, &a.id);
                out.removed.push((a.kind, a.label.clone()));
            }
            (Some(v), change) => {
                let mut v = v.clone();
                if !place(&mut v) {
                    continue;
                }
                if let (Some(Item::Note(old)), ChangeKind::Replaced) = (&now, change) {
                    state.backup_note(old, a.label.as_deref().unwrap_or(""), false);
                    out.replaced_notes.push((old.note.clone(), a.label.clone()));
                } else if change == ChangeKind::Replaced {
                    out.replaced_other += 1;
                } else if change == ChangeKind::Restored {
                    out.restored.push((a.kind, a.label.clone()));
                } else {
                    out.added.push((a.kind, a.label.clone()));
                }
                put_item(state, v, a.label.as_deref());
            }
        }
    }
    out
}

fn remove_item(state: &mut DocState, kind: MarkKind, id: &str) {
    match kind {
        MarkKind::Bookmark => state.bookmarks.retain(|b| b.id != id),
        MarkKind::Note => state.notes.retain(|n| n.id != id),
        MarkKind::Highlight => state.highlights.retain(|h| h.id != id),
    }
}

/// Puts an arriving item in place of the one with its id. An arriving
/// bookmark whose name another bookmark here already has is named with the
/// other computer's name ("mark1, lab").
fn put_item(state: &mut DocState, item: Item, label: Option<&str>) {
    match item {
        Item::Bookmark(mut b) => {
            state.bookmarks.retain(|x| x.id != b.id);
            if let Some(l) = label
                && state.bookmarks.iter().any(|x| x.name == b.name)
            {
                b.name = format!("{}, {l}", b.name);
            }
            state.bookmarks.push(b);
            state.bookmarks.sort_by_key(|x| x.pos);
        }
        Item::Note(n) => state.insert_note(*n),
        Item::Highlight(h) => state.insert_highlight(h),
    }
}
