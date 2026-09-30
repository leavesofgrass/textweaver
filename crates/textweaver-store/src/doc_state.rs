//! Per-document state: position, history, bookmarks, notes, and highlights,
//! one JSON file per document, with debounced position saves.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use textweaver_core::{Bias, CharPos, CharRange, EditOutcome};

use crate::notes::{self, Annotation, Highlight, Note};
use crate::{StoreError, atomic_write};

/// Identifies a document across sessions.
///
/// Star keyed its stores inconsistently (path, path-or-title, a hash). One
/// key for everything here: the file name plus a 64-bit FNV-1a hash of the
/// absolute path, for example `sample.md-9f3c01a2b4d5e6f7`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocKey(pub String);

impl DocKey {
    /// The key for a file path, resolved as library entries are
    /// ([`resolve_path`](crate::library::resolve_path)): absolute, with
    /// links and short names resolved when the file exists, so the same
    /// file reached through a link or a short name keeps one key.
    pub fn for_path(path: &Path) -> Self {
        // The same form as library entries: symbolic links and Windows short
        // names resolved when the file exists, so one file has one key.
        let abs = crate::library::resolve_path(path);
        let s = abs.to_string_lossy();
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in s.as_bytes() {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        let name: String = abs
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || matches!(c, '.' | '-' | '_') {
                    c
                } else {
                    '_'
                }
            })
            .take(64)
            .collect();
        DocKey(format!("{name}-{h:016x}"))
    }

    /// A key for an unsaved document.
    pub fn untitled(n: u32) -> Self {
        DocKey(format!("untitled-{n}"))
    }

    /// The key as a file stem (keys are already file-name safe).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The text at a saved position, kept so the position can be found again
/// after the file was changed outside textweaver (Phase 2 relocation): the
/// characters of [`Anchor::CONTEXT_CHARS`] starting at the position, and a
/// hash of them. Old state files have none and still load.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anchor {
    /// About 40 characters of text starting at the position.
    pub context: String,
    /// 64-bit FNV-1a hash of `context`, as 16 hex digits (a JSON number
    /// would lose bits in tools that read it as a double).
    pub hash: String,
}

impl Anchor {
    /// How many characters of context are kept.
    pub const CONTEXT_CHARS: usize = 40;

    /// An anchor for the text starting at a position: its first
    /// [`CONTEXT_CHARS`](Self::CONTEXT_CHARS) characters are kept.
    pub fn from_text_at(text_from_pos: impl IntoIterator<Item = char>) -> Self {
        let context: String = text_from_pos
            .into_iter()
            .take(Self::CONTEXT_CHARS)
            .collect();
        let hash = format!("{:016x}", fnv1a(context.as_bytes()));
        Anchor { context, hash }
    }

    /// True when `text_from_pos` still starts with this anchor's context
    /// (its hash agrees): the saved position is where it was.
    pub fn matches(&self, text_from_pos: impl IntoIterator<Item = char>) -> bool {
        let here = Anchor::from_text_at(text_from_pos);
        here.hash == self.hash && here.context == self.context
    }
}

/// What a document's text was when its state was saved: its length in
/// characters and a hash of it. On open, a different stamp means the file
/// changed outside textweaver, and saved positions are found again from
/// their anchors (Phase 2). Old state files have none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextStamp {
    /// Length of the canonical text in characters.
    pub chars: usize,
    /// 64-bit FNV-1a hash of the canonical text's UTF-8, as 16 hex digits.
    pub hash: String,
}

impl TextStamp {
    /// The stamp of a text given as consecutive pieces (a rope's chunks).
    pub fn of<'a>(chunks: impl IntoIterator<Item = &'a str>) -> Self {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut chars = 0usize;
        for chunk in chunks {
            chars += chunk.chars().count();
            for b in chunk.bytes() {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
        }
        TextStamp {
            chars,
            hash: format!("{h:016x}"),
        }
    }
}

/// 64-bit FNV-1a, the hash [`DocKey`] also uses.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// A named position.
///
/// Every bookmark has a stable [`id`](Self::id), so bookmarks from two
/// computers are matched by id, not by name (two computers can each make a
/// different `mark1`). A bookmark read from a file written before ids
/// existed (state format 1) is given one derived from its name, place, and
/// time, so reading the same file twice gives the same id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "BookmarkRecord")]
pub struct Bookmark {
    /// Stable id: 16 hex digits for a new bookmark ([`notes::new_id`]),
    /// `bm-` and 16 hex digits for one migrated from an older file
    /// ([`Bookmark::legacy_id`]).
    pub id: String,
    /// User-visible name.
    pub name: String,
    /// Position.
    pub pos: CharPos,
    /// Percentage through the document, floored.
    pub pct: u8,
    /// When it was set (Unix seconds, UTC).
    pub ts: i64,
    /// The text at the bookmark, for finding it again after outside edits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<Anchor>,
    /// The file changed outside textweaver and the bookmark's text could
    /// not be found again: it was placed by percentage, and lists say so
    /// until it is set again.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub not_found: bool,
    /// Unknown fields (from a newer version), preserved.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Bookmark {
    /// A new bookmark with a fresh id ([`notes::new_id`]), no anchor, and
    /// no unknown fields.
    pub fn new(name: impl Into<String>, pos: CharPos, pct: u8, ts: i64) -> Self {
        Bookmark {
            id: notes::new_id(),
            name: name.into(),
            pos,
            pct,
            ts,
            anchor: None,
            not_found: false,
            extra: serde_json::Map::new(),
        }
    }

    /// The id given to a bookmark from an older file that had none: `bm-`
    /// and 16 hex digits derived from its name, place, and time.
    pub fn legacy_id(name: &str, pos: CharPos, ts: i64) -> String {
        notes::stable_id64("bm-", &[name, &pos.0.to_string(), &ts.to_string()])
    }
}

/// A bookmark as stored: the id may be missing (state format 1).
#[derive(Deserialize)]
struct BookmarkRecord {
    #[serde(default)]
    id: String,
    name: String,
    pos: CharPos,
    pct: u8,
    ts: i64,
    #[serde(default)]
    anchor: Option<Anchor>,
    #[serde(default)]
    not_found: bool,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

impl From<BookmarkRecord> for Bookmark {
    fn from(r: BookmarkRecord) -> Self {
        let id = if r.id.trim().is_empty() {
            Bookmark::legacy_id(&r.name, r.pos, r.ts)
        } else {
            r.id
        };
        Bookmark {
            id,
            name: r.name,
            pos: r.pos,
            pct: r.pct,
            ts: r.ts,
            anchor: r.anchor,
            not_found: r.not_found,
            extra: r.extra,
        }
    }
}

/// The state file format this version writes.
///
/// - 1: every file written before the sync wave (it has no `format` key):
///   bookmarks without ids, 8-digit note and highlight ids.
/// - 2: bookmarks carry ids; deletions of notes, highlights, and bookmarks
///   are recorded ([`Deletions`]); replaced note text is kept in
///   [`DocState::note_backups`]; new ids are 16 hex digits.
///
/// Every older file loads unchanged: a missing `format` reads as 1, and a
/// bookmark without an id is given [`Bookmark::legacy_id`]. A file from a
/// newer version loads too (unknown keys are kept), and keeps its newer
/// number when written back.
pub const STATE_FORMAT: u32 = 2;

/// The format of a state file with no `format` key.
pub const LEGACY_STATE_FORMAT: u32 = 1;

fn legacy_format() -> u32 {
    LEGACY_STATE_FORMAT
}

/// Writes at least [`STATE_FORMAT`]: whatever this version writes is in
/// its own format, or a newer one it preserved.
fn write_format<S: serde::Serializer>(v: &u32, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_u32((*v).max(STATE_FORMAT))
}

/// Which kind of mark a deletion record or merge is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MarkKind {
    /// A note.
    Note,
    /// A highlight.
    Highlight,
    /// A bookmark.
    Bookmark,
}

/// A clock stamp in the shape of the sync wave's hybrid logical clock
/// (ADR-0049): wall time in milliseconds, a counter, and the device id for
/// ties. Stamps order by those three fields in that order.
///
/// The store makes plain wall-clock stamps ([`ClockStamp::now_local`]);
/// the sync code passes its own clock's stamps through
/// [`DocState::record_deletion_at`], so this maps onto its type field by
/// field.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(default)]
pub struct ClockStamp {
    /// Wall time, milliseconds since the Unix epoch (UTC).
    pub wall_ms: i64,
    /// Counter for changes within one millisecond.
    pub counter: u32,
    /// The random device id of the computer that made the change; empty
    /// for a change made here before sync was set up.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub device: String,
}

impl ClockStamp {
    /// A stamp for now from this computer's wall clock, with no device id.
    pub fn now_local() -> Self {
        let wall_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
        ClockStamp {
            wall_ms,
            counter: 0,
            device: String::new(),
        }
    }

    /// A stamp at the start of second `ts` (Unix seconds), for tests and
    /// for converting an item's time.
    pub fn at_secs(ts: i64) -> Self {
        ClockStamp {
            wall_ms: ts.saturating_mul(1000),
            counter: 0,
            device: String::new(),
        }
    }

    /// The stamp's wall time in whole seconds, to compare with an item's
    /// `ts` (Unix seconds).
    pub fn secs(&self) -> i64 {
        self.wall_ms.div_euclid(1000)
    }
}

/// The clock stamp an item carries under the key `clock` (written by the
/// sync code), if any.
fn item_clock(extra: &serde_json::Map<String, serde_json::Value>) -> Option<ClockStamp> {
    extra
        .get("clock")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}

/// A record that a note, highlight, or bookmark was deleted, kept so a
/// later merge with another computer's copy cannot bring it back. The later
/// of a deletion and an edit wins ([`Deletion::covers`]).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Deletion {
    /// The deleted item's id.
    pub id: String,
    /// When it was deleted.
    pub clock: ClockStamp,
    /// Unknown fields, preserved.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Deletion {
    /// True when this deletion wins over a version of the item changed at
    /// `ts` (Unix seconds), or at `clock` when the item carries a stamp:
    /// the deletion is at or after the change. With seconds only, a
    /// deletion in the same second as the change wins.
    pub fn covers(&self, ts: i64, clock: Option<&ClockStamp>) -> bool {
        match clock {
            Some(c) => self.clock >= *c,
            None => self.clock.secs() >= ts,
        }
    }
}

/// Deletion records, one list per kind of mark. Kept on this computer and
/// published with the rest of the document's marks.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Deletions {
    /// Deleted notes.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Deletion>,
    /// Deleted highlights.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub highlights: Vec<Deletion>,
    /// Deleted bookmarks.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bookmarks: Vec<Deletion>,
    /// Unknown kinds (from a newer version), preserved.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Deletions {
    /// True when nothing was ever deleted.
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
            && self.highlights.is_empty()
            && self.bookmarks.is_empty()
            && self.extra.is_empty()
    }

    /// The records of one kind.
    pub fn of(&self, kind: MarkKind) -> &[Deletion] {
        match kind {
            MarkKind::Note => &self.notes,
            MarkKind::Highlight => &self.highlights,
            MarkKind::Bookmark => &self.bookmarks,
        }
    }

    fn of_mut(&mut self, kind: MarkKind) -> &mut Vec<Deletion> {
        match kind {
            MarkKind::Note => &mut self.notes,
            MarkKind::Highlight => &mut self.highlights,
            MarkKind::Bookmark => &mut self.bookmarks,
        }
    }

    /// The deletion record for the item `id` of `kind`, if it was deleted.
    pub fn get(&self, kind: MarkKind, id: &str) -> Option<&Deletion> {
        self.of(kind).iter().find(|d| d.id == id)
    }

    /// Records that `id` was deleted at `clock`. A record for the same id
    /// keeps the later stamp. An empty id records nothing.
    pub fn record(&mut self, kind: MarkKind, id: &str, clock: ClockStamp) {
        self.absorb(
            kind,
            &Deletion {
                id: id.to_owned(),
                clock,
                extra: serde_json::Map::new(),
            },
        );
    }

    /// Adds another computer's record, keeping the later one per id.
    fn absorb(&mut self, kind: MarkKind, d: &Deletion) {
        if d.id.is_empty() {
            return;
        }
        let list = self.of_mut(kind);
        match list.iter_mut().find(|x| x.id == d.id) {
            Some(x) if d.clock > x.clock => *x = d.clone(),
            Some(_) => {}
            None => list.push(d.clone()),
        }
    }

    /// Forgets deletion records made before `before` (Unix seconds).
    /// Returns how many were dropped. A copy of the document on a computer
    /// that has been away longer than that could bring such an item back,
    /// so keep this well beyond the longest time a computer stays offline.
    pub fn prune(&mut self, before: i64) -> usize {
        let mut dropped = 0;
        for kind in [MarkKind::Note, MarkKind::Highlight, MarkKind::Bookmark] {
            let list = self.of_mut(kind);
            let n = list.len();
            list.retain(|d| d.clock.secs() >= before);
            dropped += n - list.len();
        }
        dropped
    }
}

/// How many replaced notes one document keeps ([`DocState::note_backups`]);
/// the oldest are dropped first.
///
/// The last 20 replaced versions per document are kept, with no time limit
/// (the default chosen for ADR-0049's open question).
pub const NOTE_BACKUPS_MAX: usize = 20;

/// An earlier version of a note, kept on this computer when another
/// computer's newer edit replaced it (the owner's newest-wins rule), or a
/// later deletion from another computer removed it. Never published: it is
/// this computer's safety net.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NoteBackup {
    /// This backup's own id, for restoring it.
    pub id: String,
    /// The note as it was, with its own id, text, tags, and time.
    pub note: Note,
    /// When it was replaced (Unix seconds, UTC).
    pub replaced: i64,
    /// The name of the computer whose edit replaced it, when known.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub by: String,
    /// The note was deleted by the other computer, not replaced.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub deleted: bool,
    /// Unknown fields, preserved.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// What [`DocState::merge_marks`] changed, for the announcement ("a note
/// was replaced by the laptop's newer edit").
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MergeReport {
    /// Ids of notes whose text here was replaced by the other copy's newer
    /// edit (the old text is in [`DocState::note_backups`]).
    pub replaced_notes: Vec<String>,
    /// Ids of notes, highlights, and bookmarks the other copy added.
    pub added: Vec<(MarkKind, String)>,
    /// Ids of items removed here because the other copy deleted them later
    /// than they were last changed.
    pub deleted: Vec<(MarkKind, String)>,
    /// Bookmarks from the other copy renamed because a different bookmark
    /// here has the same name: `(old name, new name)`, such as
    /// `("mark1", "mark1, lab")`.
    pub renamed_bookmarks: Vec<(String, String)>,
}

impl MergeReport {
    /// True when the merge changed nothing here.
    pub fn is_empty(&self) -> bool {
        self.replaced_notes.is_empty()
            && self.added.is_empty()
            && self.deleted.is_empty()
            && self.renamed_bookmarks.is_empty()
    }
}

/// When an item was last changed: its clock stamp when the sync code gave
/// it one (the `clock` key), else the start of its `ts` second.
trait Versioned {
    fn version(&self) -> ClockStamp;
}

impl Versioned for Note {
    fn version(&self) -> ClockStamp {
        item_clock(&self.extra).unwrap_or_else(|| ClockStamp::at_secs(self.ts))
    }
}

impl Versioned for Highlight {
    fn version(&self) -> ClockStamp {
        item_clock(&self.extra).unwrap_or_else(|| ClockStamp::at_secs(self.ts))
    }
}

impl Versioned for Bookmark {
    fn version(&self) -> ClockStamp {
        item_clock(&self.extra).unwrap_or_else(|| ClockStamp::at_secs(self.ts))
    }
}

/// True when `a` wins over `b` under newest-wins: the later version
/// ([`Versioned`]), and on a tie the larger serialized form, so both
/// computers pick the same one.
fn wins<T: Serialize + Versioned>(a: &T, b: &T) -> bool {
    match a.version().cmp(&b.version()) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => {
            let a = serde_json::to_string(a).unwrap_or_default();
            let b = serde_json::to_string(b).unwrap_or_default();
            a >= b
        }
    }
}

/// Percentage of `pos` through a document of `len` chars, floored, as Star
/// computed it: `int(100 * offset / max(1, len))`, capped at 100.
pub fn percent(pos: CharPos, len: usize) -> u8 {
    let pct = pos.0.saturating_mul(100) / len.max(1);
    u8::try_from(pct.min(100)).unwrap_or(100)
}

/// Everything remembered about one document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DocState {
    /// The format of the file this state was read from ([`STATE_FORMAT`]
    /// for a new state, [`LEGACY_STATE_FORMAT`] for a file with no
    /// `format` key). Written as at least [`STATE_FORMAT`].
    #[serde(default = "legacy_format", serialize_with = "write_format")]
    pub format: u32,
    /// Char offset of the word being read when the position was saved.
    pub position: CharPos,
    /// Percentage through the document, floored.
    pub pct: u8,
    /// When the position was saved (Unix seconds, UTC).
    pub ts: i64,
    /// The text at [`position`](Self::position), for finding it again after
    /// outside edits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<Anchor>,
    /// The text these positions belong to (see [`TextStamp`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextStamp>,
    /// Navigation history, oldest first.
    pub history: Vec<CharPos>,
    /// Bookmarks.
    pub bookmarks: Vec<Bookmark>,
    /// Notes, in document order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
    /// Highlights, in document order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub highlights: Vec<Highlight>,
    /// Deleted notes, highlights, and bookmarks ([`Deletions`]), so a
    /// merge cannot bring them back.
    #[serde(skip_serializing_if = "Deletions::is_empty")]
    pub deleted: Deletions,
    /// Earlier versions of notes replaced by another computer's newer edit
    /// (at most [`NOTE_BACKUPS_MAX`], oldest dropped first). Local only:
    /// never published to other computers.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub note_backups: Vec<NoteBackup>,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for DocState {
    fn default() -> Self {
        DocState {
            format: STATE_FORMAT,
            position: CharPos::ZERO,
            pct: 0,
            ts: 0,
            anchor: None,
            text: None,
            history: Vec::new(),
            bookmarks: Vec::new(),
            notes: Vec::new(),
            highlights: Vec::new(),
            deleted: Deletions::default(),
            note_backups: Vec::new(),
            extra: serde_json::Map::new(),
        }
    }
}

impl DocState {
    /// Records the reading position: the char offset of the current word in
    /// a document of `doc_len` chars, the floored percentage, and now.
    pub fn set_position(&mut self, pos: CharPos, doc_len: usize) {
        let pos = pos.clamp_to(doc_len);
        self.position = pos;
        self.pct = percent(pos, doc_len);
        self.ts = crate::now_ts();
    }

    /// True when a position has been saved.
    pub fn has_position(&self) -> bool {
        self.ts != 0
    }

    /// Forgets the saved position, keeping bookmarks. Returns whether there
    /// was one ("Reading position cleared" / "No saved position for this
    /// document").
    pub fn clear_position(&mut self) -> bool {
        let had = self.has_position();
        self.position = CharPos::ZERO;
        self.pct = 0;
        self.ts = 0;
        had
    }

    /// Appends a history entry, skipping an entry equal to the last one and
    /// keeping at most `cap` entries (Star's `nav_history_size`).
    pub fn push_history(&mut self, pos: CharPos, cap: usize) {
        if self.history.last() != Some(&pos) {
            self.history.push(pos);
        }
        let cap = cap.max(1);
        if self.history.len() > cap {
            let excess = self.history.len() - cap;
            self.history.drain(..excess);
        }
    }

    /// The first free automatic bookmark name: `mark1`, `mark2`, ...
    pub fn next_bookmark_name(&self) -> String {
        (1..)
            .map(|n| format!("mark{n}"))
            .find(|name| self.bookmark(name).is_none())
            .unwrap_or_else(|| "mark".to_owned())
    }

    /// Adds a bookmark at `pos`. An empty or missing name gets the first free
    /// `markN` (Star's rule). A bookmark with the same name is moved there:
    /// it keeps its id and gets a new time. Bookmarks stay sorted by
    /// position. Returns the bookmark added.
    pub fn add_bookmark(&mut self, name: Option<&str>, pos: CharPos, doc_len: usize) -> Bookmark {
        let name = match name.map(str::trim) {
            Some(n) if !n.is_empty() => n.to_owned(),
            _ => self.next_bookmark_name(),
        };
        let pos = pos.clamp_to(doc_len);
        let mut mark = Bookmark::new(name, pos, percent(pos, doc_len), crate::now_ts());
        if let Some(i) = self.bookmarks.iter().position(|b| b.name == mark.name) {
            let old = self.bookmarks.remove(i);
            mark.id = old.id;
            mark.extra = old.extra;
        }
        let at = self.bookmarks.partition_point(|b| b.pos <= pos);
        self.bookmarks.insert(at, mark.clone());
        mark
    }

    /// The bookmark called `name`.
    pub fn bookmark(&self, name: &str) -> Option<&Bookmark> {
        self.bookmarks.iter().find(|b| b.name == name)
    }

    /// The bookmark with `id`.
    pub fn bookmark_by_id(&self, id: &str) -> Option<&Bookmark> {
        self.bookmarks.iter().find(|b| b.id == id)
    }

    /// Removes the bookmark called `name`, recording the deletion. Returns
    /// whether it existed.
    pub fn remove_bookmark(&mut self, name: &str) -> bool {
        let Some(i) = self.bookmarks.iter().position(|b| b.name == name) else {
            return false;
        };
        let b = self.bookmarks.remove(i);
        self.record_deletion(MarkKind::Bookmark, &b.id);
        true
    }

    /// Records, now, that the note, highlight, or bookmark `id` was
    /// deleted, so a later merge with another computer's copy cannot bring
    /// it back. The store's own remove functions call this; a frontend that
    /// removes items from its own lists calls it too. The stamp is this
    /// computer's wall clock ([`ClockStamp::now_local`]).
    pub fn record_deletion(&mut self, kind: MarkKind, id: &str) {
        self.deleted.record(kind, id, ClockStamp::now_local());
    }

    /// Records that `id` was deleted at `clock` (the sync clock's stamp).
    pub fn record_deletion_at(&mut self, kind: MarkKind, id: &str, clock: ClockStamp) {
        self.deleted.record(kind, id, clock);
    }

    /// Bookmarks in document order.
    pub fn sorted_bookmarks(&self) -> Vec<&Bookmark> {
        let mut v: Vec<&Bookmark> = self.bookmarks.iter().collect();
        v.sort_by(|a, b| a.pos.cmp(&b.pos).then_with(|| a.name.cmp(&b.name)));
        v
    }

    /// The first bookmark after `pos`, wrapping to the first one when `wrap`
    /// is set.
    pub fn next_bookmark(&self, pos: CharPos, wrap: bool) -> Option<&Bookmark> {
        let sorted = self.sorted_bookmarks();
        sorted
            .iter()
            .find(|b| b.pos > pos)
            .or_else(|| if wrap { sorted.first() } else { None })
            .copied()
    }

    /// The last bookmark before `pos`, wrapping to the last one when `wrap`
    /// is set.
    pub fn previous_bookmark(&self, pos: CharPos, wrap: bool) -> Option<&Bookmark> {
        let sorted = self.sorted_bookmarks();
        sorted
            .iter()
            .rev()
            .find(|b| b.pos < pos)
            .or_else(|| if wrap { sorted.last() } else { None })
            .copied()
    }

    /// Adds a note on `range`. `anchor_text` is the text of that range (or
    /// of the word or paragraph at a point); it is kept, collapsed and
    /// shortened, so the note still reads well if the text changes. Tags are
    /// parsed from `tags` as Star did (`#a, b c`). Notes stay in document
    /// order. Returns the new note.
    pub fn add_note(
        &mut self,
        range: CharRange,
        anchor_text: &str,
        note: &str,
        tags: &str,
    ) -> Note {
        let now = crate::now_ts();
        let n = Note {
            id: self.fresh_id(),
            range,
            anchor: notes::collapse(anchor_text, notes::ANCHOR_MAX_CHARS),
            note: note.trim().to_owned(),
            tags: notes::parse_tags(tags),
            created: now,
            ts: now,
            ..Note::default()
        };
        self.insert_note(n.clone());
        n
    }

    /// An id no note or highlight of this document uses.
    fn fresh_id(&self) -> String {
        let mut id = notes::new_id();
        while self.note(&id).is_some() || self.highlight(&id).is_some() {
            id = notes::new_id();
        }
        id
    }

    /// Inserts a prepared note in document order, replacing a note with the
    /// same id (imports and merges).
    pub fn insert_note(&mut self, note: Note) {
        self.notes.retain(|n| n.id != note.id);
        let key = (note.range.start, note.range.end);
        let at = self
            .notes
            .partition_point(|n| (n.range.start, n.range.end) <= key);
        self.notes.insert(at, note);
    }

    /// The note with `id`.
    pub fn note(&self, id: &str) -> Option<&Note> {
        self.notes.iter().find(|n| n.id == id)
    }

    /// Changes a note's text and tags. Empty text deletes the note (Star's
    /// rule). Returns false when there is no such note.
    pub fn edit_note(&mut self, id: &str, note: &str, tags: &str) -> bool {
        if note.trim().is_empty() {
            return self.remove_note(id);
        }
        let Some(n) = self.notes.iter_mut().find(|n| n.id == id) else {
            return false;
        };
        n.note = note.trim().to_owned();
        n.tags = notes::parse_tags(tags);
        n.ts = crate::now_ts();
        true
    }

    /// Removes the note with `id`, recording the deletion. Returns whether
    /// it existed.
    pub fn remove_note(&mut self, id: &str) -> bool {
        let before = self.notes.len();
        self.notes.retain(|n| n.id != id);
        let removed = self.notes.len() != before;
        if removed {
            self.record_deletion(MarkKind::Note, id);
        }
        removed
    }

    /// Notes at `pos`: those whose range contains it, and point notes at it.
    pub fn notes_at(&self, pos: CharPos) -> Vec<&Note> {
        self.notes
            .iter()
            .filter(|n| n.range.contains(pos) || n.range.start == pos)
            .collect()
    }

    /// Notes matching a search query (Star's rules, [`Note::matches`]).
    pub fn search_notes(&self, query: &str) -> Vec<&Note> {
        self.notes.iter().filter(|n| n.matches(query)).collect()
    }

    /// Every distinct tag, sorted case-insensitively.
    pub fn tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self.notes.iter().flat_map(|n| n.tags.clone()).collect();
        tags.sort_by_key(|t| t.to_lowercase());
        tags.dedup_by(|a, b| a.to_lowercase() == b.to_lowercase());
        tags
    }

    /// Highlights `range` in `color` (a name such as "yellow" or `#rrggbb`).
    /// `text` is the highlighted text. A highlight on exactly the same range
    /// is replaced, keeping its id with a new time, so highlighting again
    /// changes the color. An empty range highlights nothing and returns
    /// `None`.
    pub fn add_highlight(
        &mut self,
        range: CharRange,
        color: &str,
        text: &str,
    ) -> Option<Highlight> {
        if range.is_empty() {
            return None;
        }
        let (id, extra) = match self.highlights.iter().find(|x| x.range == range) {
            Some(old) => (old.id.clone(), old.extra.clone()),
            None => (self.fresh_id(), serde_json::Map::new()),
        };
        let h = Highlight {
            id,
            range,
            color: notes::highlight_color(color),
            text: notes::collapse(text, notes::HIGHLIGHT_TEXT_MAX_CHARS),
            ts: crate::now_ts(),
            extra,
        };
        self.insert_highlight(h.clone());
        Some(h)
    }

    /// Inserts a prepared highlight in document order, replacing one with
    /// the same id or the same range.
    pub fn insert_highlight(&mut self, h: Highlight) {
        self.highlights
            .retain(|x| x.id != h.id && x.range != h.range);
        let key = (h.range.start, h.range.end);
        let at = self
            .highlights
            .partition_point(|x| (x.range.start, x.range.end) <= key);
        self.highlights.insert(at, h);
    }

    /// The highlight with `id`.
    pub fn highlight(&self, id: &str) -> Option<&Highlight> {
        self.highlights.iter().find(|h| h.id == id)
    }

    /// Highlights covering `pos`.
    pub fn highlights_at(&self, pos: CharPos) -> Vec<&Highlight> {
        self.highlights
            .iter()
            .filter(|h| h.range.contains(pos))
            .collect()
    }

    /// Removes the highlight with `id`, recording the deletion. Returns it
    /// if it existed.
    pub fn remove_highlight(&mut self, id: &str) -> Option<Highlight> {
        let i = self.highlights.iter().position(|h| h.id == id)?;
        let h = self.highlights.remove(i);
        self.record_deletion(MarkKind::Highlight, &h.id);
        Some(h)
    }

    /// Removes every highlight (Star's Clear All Highlights), recording
    /// each deletion. Returns how many there were.
    pub fn clear_highlights(&mut self) -> usize {
        let gone = std::mem::take(&mut self.highlights);
        for h in &gone {
            self.record_deletion(MarkKind::Highlight, &h.id);
        }
        gone.len()
    }

    /// Removes the note or highlight at `pos`: the shortest note at `pos`
    /// first, else the shortest highlight covering it. Returns what was
    /// removed as it would be spoken, for the announcement.
    pub fn remove_annotation_at(&mut self, pos: CharPos) -> Option<String> {
        let note = self
            .notes_at(pos)
            .into_iter()
            .min_by_key(|n| n.range.len())
            .map(|n| (n.id.clone(), Annotation::Note(n).spoken()));
        if let Some((id, spoken)) = note {
            self.remove_note(&id);
            return Some(spoken);
        }
        let (id, spoken) = self
            .highlights_at(pos)
            .into_iter()
            .min_by_key(|h| h.range.len())
            .map(|h| (h.id.clone(), Annotation::Highlight(h).spoken()))?;
        self.remove_highlight(&id);
        Some(spoken)
    }

    /// Notes and highlights together, in document order (a note before a
    /// highlight at the same position).
    pub fn annotations(&self) -> Vec<Annotation<'_>> {
        let mut v: Vec<Annotation<'_>> = self
            .notes
            .iter()
            .map(Annotation::Note)
            .chain(self.highlights.iter().map(Annotation::Highlight))
            .collect();
        v.sort_by_key(|a| {
            let r = a.range();
            (r.start, matches!(a, Annotation::Highlight(_)), r.end)
        });
        v
    }

    /// The first note or highlight starting after `pos`, wrapping to the
    /// first one when `wrap` is set.
    pub fn next_annotation(&self, pos: CharPos, wrap: bool) -> Option<Annotation<'_>> {
        notes::step(&self.annotations(), pos, true, wrap)
    }

    /// The last note or highlight starting before `pos`, wrapping to the
    /// last one when `wrap` is set.
    pub fn previous_annotation(&self, pos: CharPos, wrap: bool) -> Option<Annotation<'_>> {
        notes::step(&self.annotations(), pos, false, wrap)
    }

    /// The notes (and, if asked, highlights) as Markdown
    /// ([`notes::notes_markdown`]).
    pub fn notes_markdown(&self, opts: &notes::NotesExport<'_>) -> String {
        notes::notes_markdown(&self.notes, &self.highlights, opts)
    }

    /// Moves every stored position across an edit, so the reading position,
    /// history, bookmarks, notes, and highlights keep pointing at the same
    /// text. Positions inside deleted text move to the start of the
    /// replacement; a highlight whose text is deleted is dropped, and a note
    /// whose text is deleted stays as a point note (its anchor keeps the old
    /// text).
    /// Highlights dropped this way are recorded as deleted.
    pub fn shift(&mut self, outcome: &EditOutcome) {
        let before: Vec<String> = self.highlights.iter().map(|h| h.id.clone()).collect();
        notes::shift(&mut self.notes, &mut self.highlights, outcome);
        for id in before {
            if self.highlight(&id).is_none() {
                self.record_deletion(MarkKind::Highlight, &id);
            }
        }

        self.position = outcome.map_pos(self.position, Bias::Before);
        for h in &mut self.history {
            *h = outcome.map_pos(*h, Bias::Before);
        }
        self.history.dedup();
        for b in &mut self.bookmarks {
            b.pos = outcome.map_pos(b.pos, Bias::Before);
        }
        self.bookmarks.sort_by_key(|b| b.pos);
    }

    /// Keeps `old`, a version of a note that is being replaced or removed
    /// by another computer's change, in [`note_backups`](Self::note_backups).
    /// `by` names that computer (empty when unknown). The same version is
    /// kept once; past [`NOTE_BACKUPS_MAX`], the oldest backups go. Returns
    /// the backup's id.
    pub fn backup_note(&mut self, old: &Note, by: &str, deleted: bool) -> String {
        if let Some(b) = self
            .note_backups
            .iter()
            .find(|b| b.note == *old && b.deleted == deleted)
        {
            return b.id.clone();
        }
        let id = notes::new_id();
        self.note_backups.push(NoteBackup {
            id: id.clone(),
            note: old.clone(),
            replaced: crate::now_ts(),
            by: by.to_owned(),
            deleted,
            extra: serde_json::Map::new(),
        });
        if self.note_backups.len() > NOTE_BACKUPS_MAX {
            // Oldest first out; the list is in the order they were made.
            self.note_backups.sort_by_key(|b| b.replaced);
            let excess = self.note_backups.len() - NOTE_BACKUPS_MAX;
            self.note_backups.drain(..excess);
        }
        id
    }

    /// The kept versions of replaced notes, newest first.
    pub fn note_backups_newest_first(&self) -> Vec<&NoteBackup> {
        let mut v: Vec<&NoteBackup> = self.note_backups.iter().collect();
        v.sort_by(|a, b| {
            b.replaced
                .cmp(&a.replaced)
                .then_with(|| b.note.ts.cmp(&a.note.ts))
        });
        v
    }

    /// The kept versions of the note `note_id`, newest first.
    pub fn backups_of_note(&self, note_id: &str) -> Vec<&NoteBackup> {
        self.note_backups_newest_first()
            .into_iter()
            .filter(|b| b.note.id == note_id)
            .collect()
    }

    /// Puts a kept version back as the note's current text, as a new edit
    /// (its time is now, so it wins over the other computers' copies at
    /// the next merge, and over a deletion). The text it replaces is kept
    /// in turn, so restoring can be undone. Returns the restored note, or
    /// `None` when there is no backup `backup_id`.
    pub fn restore_note_backup(&mut self, backup_id: &str) -> Option<Note> {
        let i = self.note_backups.iter().position(|b| b.id == backup_id)?;
        let backup = self.note_backups.remove(i);
        let mut note = backup.note;
        if let Some(current) = self.note(&note.id).cloned() {
            self.backup_note(&current, "", false);
            // Keep the place the note has now; only the words come back.
            note.range = current.range;
        }
        note.ts = crate::now_ts();
        self.insert_note(note.clone());
        Some(note)
    }

    /// Merges another copy of this document's notes, highlights, and
    /// bookmarks (another computer's) into this one, item by item, by id:
    ///
    /// - the newer version of an item wins (`ts`; on a tie both computers
    ///   pick the same one);
    /// - a deletion wins over any version not changed after it, on either
    ///   side, so a deleted note stays deleted; an edit made after the
    ///   deletion wins and brings the item back;
    /// - a note here whose text loses to the other copy's newer edit, or is
    ///   removed by its later deletion, is kept in
    ///   [`note_backups`](Self::note_backups) first;
    /// - an arriving bookmark whose name is taken here by a different
    ///   bookmark is renamed "name, other_name" ("mark1, lab");
    /// - the deletion records of both copies are kept (the later per id).
    ///
    /// Merging the same copy again changes nothing. Positions, history,
    /// and backups of the other copy are not touched: places have their own
    /// rule, and backups stay on the computer that made them.
    pub fn merge_marks(&mut self, other: &DocState, other_name: &str) -> MergeReport {
        let mut report = MergeReport::default();
        for kind in [MarkKind::Note, MarkKind::Highlight, MarkKind::Bookmark] {
            for d in other.deleted.of(kind) {
                self.deleted.absorb(kind, d);
            }
        }
        self.merge_notes(other, other_name, &mut report);
        self.merge_highlights(other, &mut report);
        self.merge_bookmarks(other, other_name, &mut report);
        report
    }

    /// True when a deletion record of `kind` for `id` wins over a version
    /// changed at `ts` carrying `extra` (which may hold a clock stamp).
    fn deleted_since(
        &self,
        kind: MarkKind,
        id: &str,
        ts: i64,
        extra: &serde_json::Map<String, serde_json::Value>,
    ) -> bool {
        self.deleted
            .get(kind, id)
            .is_some_and(|d| d.covers(ts, item_clock(extra).as_ref()))
    }

    fn merge_notes(&mut self, other: &DocState, other_name: &str, report: &mut MergeReport) {
        for theirs in &other.notes {
            match self.note(&theirs.id).cloned() {
                Some(mine) if mine == *theirs => {}
                Some(mine) => {
                    if wins(theirs, &mine) {
                        if mine.note != theirs.note || mine.tags != theirs.tags {
                            self.backup_note(&mine, other_name, false);
                            report.replaced_notes.push(theirs.id.clone());
                        }
                        self.insert_note(theirs.clone());
                    }
                }
                None => {
                    if !self.deleted_since(MarkKind::Note, &theirs.id, theirs.ts, &theirs.extra) {
                        self.insert_note(theirs.clone());
                        report.added.push((MarkKind::Note, theirs.id.clone()));
                    }
                }
            }
        }
        // Deletions (from either copy) that are later than the note here.
        let gone: Vec<Note> = self
            .notes
            .iter()
            .filter(|n| self.deleted_since(MarkKind::Note, &n.id, n.ts, &n.extra))
            .cloned()
            .collect();
        for n in gone {
            if other.deleted.get(MarkKind::Note, &n.id).is_some() {
                self.backup_note(&n, other_name, true);
            }
            self.notes.retain(|x| x.id != n.id);
            report.deleted.push((MarkKind::Note, n.id));
        }
    }

    fn merge_highlights(&mut self, other: &DocState, report: &mut MergeReport) {
        for theirs in &other.highlights {
            match self.highlight(&theirs.id).cloned() {
                Some(mine) if mine == *theirs => {}
                Some(mine) => {
                    if wins(theirs, &mine) {
                        self.put_highlight(theirs.clone());
                    }
                }
                None => {
                    if !self.deleted_since(
                        MarkKind::Highlight,
                        &theirs.id,
                        theirs.ts,
                        &theirs.extra,
                    ) {
                        self.put_highlight(theirs.clone());
                        report.added.push((MarkKind::Highlight, theirs.id.clone()));
                    }
                }
            }
        }
        let gone: Vec<String> = self
            .highlights
            .iter()
            .filter(|h| self.deleted_since(MarkKind::Highlight, &h.id, h.ts, &h.extra))
            .map(|h| h.id.clone())
            .collect();
        for id in gone {
            self.highlights.retain(|h| h.id != id);
            report.deleted.push((MarkKind::Highlight, id));
        }
    }

    /// Inserts a merged highlight. Two highlights on the same range keep
    /// only the newer (the same choice on every computer).
    fn put_highlight(&mut self, h: Highlight) {
        if let Some(same) = self
            .highlights
            .iter()
            .find(|x| x.range == h.range && x.id != h.id)
            && wins(same, &h)
        {
            return;
        }
        self.insert_highlight(h);
    }

    fn merge_bookmarks(&mut self, other: &DocState, other_name: &str, report: &mut MergeReport) {
        for theirs in &other.bookmarks {
            let mut theirs = theirs.clone();
            let arriving = match self.bookmark_by_id(&theirs.id).cloned() {
                Some(mine) if mine == theirs => continue,
                // Renamed here by an earlier merge of this same version.
                Some(mine)
                    if mine.name != theirs.name
                        && Bookmark {
                            name: mine.name.clone(),
                            ..theirs.clone()
                        } == mine
                        && self
                            .bookmarks
                            .iter()
                            .any(|b| b.name == theirs.name && b.id != theirs.id) =>
                {
                    continue;
                }
                Some(mine) => {
                    if !wins(&theirs, &mine) {
                        continue;
                    }
                    self.bookmarks.retain(|b| b.id != mine.id);
                    false
                }
                None => {
                    if self.deleted_since(MarkKind::Bookmark, &theirs.id, theirs.ts, &theirs.extra)
                    {
                        continue;
                    }
                    true
                }
            };
            if self.bookmark(&theirs.name).is_some() {
                let new = self.free_bookmark_name(&theirs.name, other_name);
                report
                    .renamed_bookmarks
                    .push((theirs.name.clone(), new.clone()));
                theirs.name = new;
            }
            if arriving {
                report.added.push((MarkKind::Bookmark, theirs.id.clone()));
            }
            let at = self.bookmarks.partition_point(|b| b.pos <= theirs.pos);
            self.bookmarks.insert(at, theirs);
        }
        let gone: Vec<String> = self
            .bookmarks
            .iter()
            .filter(|b| self.deleted_since(MarkKind::Bookmark, &b.id, b.ts, &b.extra))
            .map(|b| b.id.clone())
            .collect();
        for id in gone {
            self.bookmarks.retain(|b| b.id != id);
            report.deleted.push((MarkKind::Bookmark, id));
        }
    }

    /// A bookmark name not in use here, for a bookmark called `name` from
    /// the computer `other_name`: "mark1, lab", then "mark1, lab 2", and so
    /// on ("mark1, other computer" when the name is unknown).
    fn free_bookmark_name(&self, name: &str, other_name: &str) -> String {
        let other = other_name.trim();
        let other = if other.is_empty() {
            "other computer"
        } else {
            other
        };
        let base = format!("{name}, {other}");
        if self.bookmark(&base).is_none() {
            return base;
        }
        (2..)
            .map(|n| format!("{base} {n}"))
            .find(|n| self.bookmark(n).is_none())
            .unwrap_or(base)
    }
}

/// How long position saves are coalesced by default.
pub const DEFAULT_DEBOUNCE: Duration = Duration::from_secs(2);

#[derive(Debug, Default)]
struct Pending {
    /// State recorded but not yet written.
    pending: HashMap<DocKey, (DocState, Instant)>,
    /// When each document was last written.
    last_write: HashMap<DocKey, Instant>,
    /// Number of files written (diagnostics and tests).
    writes: u64,
}

#[derive(Debug)]
struct Inner {
    dir: PathBuf,
    debounce: Duration,
    state: Mutex<Pending>,
}

impl Inner {
    fn lock(&self) -> MutexGuard<'_, Pending> {
        // A panic while holding the lock leaves plain data behind; keep going.
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn file(&self, key: &DocKey) -> PathBuf {
        self.dir.join(format!("{}.json", key.0))
    }

    fn write(&self, key: &DocKey, state: &DocState) -> Result<(), StoreError> {
        let path = self.file(key);
        let text = serde_json::to_string_pretty(state).map_err(|e| StoreError::Parse {
            path: path.clone(),
            message: e.to_string(),
        })?;
        atomic_write(&path, text.as_bytes())
    }

    fn flush_where(&self, due: impl Fn(Instant) -> bool) -> Result<usize, StoreError> {
        let mut st = self.lock();
        let keys: Vec<DocKey> = st
            .pending
            .iter()
            .filter(|(_, (_, since))| due(*since))
            .map(|(k, _)| k.clone())
            .collect();
        let mut first_err = None;
        let mut written = 0;
        for key in keys {
            let Some((state, _)) = st.pending.remove(&key) else {
                continue;
            };
            match self.write(&key, &state) {
                Ok(()) => {
                    st.last_write.insert(key, Instant::now());
                    st.writes += 1;
                    written += 1;
                }
                Err(e) => {
                    // Keep it pending so a later flush can retry.
                    st.pending.insert(key, (state, Instant::now()));
                    first_err.get_or_insert(e);
                }
            }
        }
        first_err.map_or(Ok(written), Err)
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        // Star's `flush_pending` was never called, so a position recorded
        // just before quitting was lost (Part 3 §7 item 22). Flush on drop.
        let _ = self.flush_where(|_| true);
    }
}

/// Reads and writes `state/<doc-key>.json`.
///
/// Position saves go through [`record`](Self::record), which coalesces rapid
/// saves: the first save for a document is written at once, later saves
/// within the debounce window are held in memory (and returned by
/// [`load`](Self::load)) until [`flush`](Self::flush),
/// [`flush_due`](Self::flush_due), the next save after the window, or drop.
/// Clones share the pending state; the last clone dropped flushes it.
#[derive(Clone, Debug)]
pub struct StateStore {
    inner: Arc<Inner>,
}

impl StateStore {
    /// A store writing under `dir`, coalescing position saves for
    /// [`DEFAULT_DEBOUNCE`].
    pub fn new(dir: PathBuf) -> Self {
        StateStore::with_debounce(dir, DEFAULT_DEBOUNCE)
    }

    /// A store with a custom debounce window.
    pub fn with_debounce(dir: PathBuf, debounce: Duration) -> Self {
        StateStore {
            inner: Arc::new(Inner {
                dir,
                debounce,
                state: Mutex::new(Pending::default()),
            }),
        }
    }

    /// The directory holding the state files.
    pub fn dir(&self) -> &Path {
        &self.inner.dir
    }

    /// The saved state, if any: the pending (not yet written) state when
    /// there is one, else the file. Unreadable files count as no state.
    ///
    /// A file that exists but does not parse (a hand edit, a sync conflict,
    /// a newer version's format) is renamed to
    /// `<key>.corrupt-<unix time>.bak` first, so the next save cannot
    /// overwrite the notes, bookmarks, and highlights it holds.
    pub fn load(&self, key: &DocKey) -> Option<DocState> {
        if let Some((state, _)) = self.inner.lock().pending.get(key) {
            return Some(state.clone());
        }
        let path = self.inner.file(key);
        let text = std::fs::read_to_string(&path).ok()?;
        match serde_json::from_str(&text) {
            Ok(state) => Some(state),
            Err(e) => {
                crate::atomic::set_aside(&path, &e);
                None
            }
        }
    }

    /// Saves state atomically now, replacing anything pending for `key`.
    /// Use for explicit changes (bookmarks); use [`record`](Self::record)
    /// for frequent position saves.
    pub fn save(&self, key: &DocKey, state: &DocState) -> Result<(), StoreError> {
        let mut st = self.inner.lock();
        st.pending.remove(key);
        self.inner.write(key, state)?;
        st.last_write.insert(key.clone(), Instant::now());
        st.writes += 1;
        Ok(())
    }

    /// Records state, coalescing rapid saves. Writes at once when nothing
    /// was written for `key` within the debounce window; otherwise keeps the
    /// state pending. Returns whether a file was written.
    pub fn record(&self, key: &DocKey, state: &DocState) -> Result<bool, StoreError> {
        let mut st = self.inner.lock();
        let now = Instant::now();
        let recent = st
            .last_write
            .get(key)
            .is_some_and(|t| now.duration_since(*t) < self.inner.debounce);
        if recent {
            let since = st.pending.get(key).map_or(now, |(_, s)| *s);
            st.pending.insert(key.clone(), (state.clone(), since));
            return Ok(false);
        }
        st.pending.remove(key);
        self.inner.write(key, state)?;
        st.last_write.insert(key.clone(), now);
        st.writes += 1;
        Ok(true)
    }

    /// Writes everything pending. Call on quit and on document switch.
    /// Returns the number of files written.
    pub fn flush(&self) -> Result<usize, StoreError> {
        self.inner.flush_where(|_| true)
    }

    /// Writes pending state older than the debounce window. Call from the
    /// event loop when idle so a coalesced position lands soon after the
    /// user stops moving.
    pub fn flush_due(&self) -> Result<usize, StoreError> {
        let window = self.inner.debounce;
        self.inner.flush_where(|since| since.elapsed() >= window)
    }

    /// True when some state has not been written yet.
    pub fn has_pending(&self) -> bool {
        !self.inner.lock().pending.is_empty()
    }

    /// Number of state files written by this store so far.
    pub fn writes(&self) -> u64 {
        self.inner.lock().writes
    }

    /// Deletes a document's state. Returns whether a file existed.
    pub fn remove(&self, key: &DocKey) -> Result<bool, StoreError> {
        let mut st = self.inner.lock();
        st.pending.remove(key);
        let path = self.inner.file(key);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(source) => Err(StoreError::Io { path, source }),
        }
    }

    /// Keeps the `keep` most recently saved documents and deletes the rest
    /// (Star capped positions at 200 entries). Flushes first. Returns the
    /// number of files removed.
    pub fn prune(&self, keep: usize) -> Result<usize, StoreError> {
        self.flush()?;
        let dir = &self.inner.dir;
        let io = |source| StoreError::Io {
            path: dir.clone(),
            source,
        };
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(e) => return Err(io(e)),
        };
        let mut files: Vec<(i64, PathBuf)> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            // A file that does not parse is kept, not pruned first: it may
            // hold notes (`load` sets it aside when its document opens).
            .filter_map(|p| {
                let ts = std::fs::read_to_string(&p)
                    .ok()
                    .and_then(|t| serde_json::from_str::<DocState>(&t).ok())?
                    .ts;
                Some((ts, p))
            })
            .collect();
        if files.len() <= keep {
            return Ok(0);
        }
        files.sort_by_key(|f| std::cmp::Reverse(f.0));
        let mut removed = 0;
        for (_, p) in files.into_iter().skip(keep) {
            if std::fs::remove_file(&p).is_ok() {
                removed += 1;
            }
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use textweaver_core::Edit;

    use super::*;

    #[test]
    fn keys_are_stable_and_readable() {
        let a = DocKey::for_path(Path::new("/tmp/some doc.md"));
        let b = DocKey::for_path(Path::new("/tmp/some doc.md"));
        assert_eq!(a, b);
        assert!(a.0.starts_with("some_doc.md-"));
        assert_ne!(a, DocKey::for_path(Path::new("/tmp/other/some doc.md")));
        assert_ne!(DocKey::untitled(1), DocKey::untitled(2));
    }

    /// Anchors keep about 40 characters and a hash; files written before
    /// anchors existed still load, and files without anchors do not gain
    /// empty ones.
    #[test]
    fn anchors_round_trip_and_old_files_still_load() {
        let text = "The mitochondria is the powerhouse of the cell, they say.";
        let a = Anchor::from_text_at(text.chars());
        assert_eq!(a.context.chars().count(), Anchor::CONTEXT_CHARS);
        assert!(text.starts_with(&a.context));
        assert_eq!(a.hash.len(), 16);
        assert!(a.matches(text.chars()));
        assert!(!a.matches("The mitochondria was the powerhouse".chars()));
        assert_eq!(Anchor::from_text_at("short".chars()).context, "short");

        let old = r#"{"position":12,"pct":20,"ts":5,"history":[],
            "bookmarks":[{"name":"mark1","pos":3,"pct":5,"ts":6}]}"#;
        let st: DocState = serde_json::from_str(old).unwrap();
        assert_eq!(st.position, CharPos(12));
        assert_eq!(st.anchor, None);
        assert_eq!(st.bookmarks[0].anchor, None);
        let json = serde_json::to_string(&st).unwrap();
        assert!(!json.contains("anchor"), "{json}");

        let mut st = st;
        st.anchor = Some(a.clone());
        st.bookmarks[0].anchor = Some(a.clone());
        let back: DocState = serde_json::from_str(&serde_json::to_string(&st).unwrap()).unwrap();
        assert_eq!(back.anchor.as_ref(), Some(&a));
        assert_eq!(back.bookmarks[0].anchor.as_ref(), Some(&a));
    }

    #[test]
    fn state_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::new(dir.path().to_owned());
        let key = DocKey::untitled(1);
        assert!(store.load(&key).is_none());
        let st = DocState {
            position: CharPos(42),
            pct: 7,
            ts: 1,
            ..DocState::default()
        };
        store.save(&key, &st).unwrap();
        assert_eq!(store.load(&key), Some(st));
    }

    #[test]
    fn unknown_keys_survive() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::new(dir.path().to_owned());
        let key = DocKey::untitled(3);
        std::fs::write(
            dir.path().join("untitled-3.json"),
            r#"{"position": 5, "notes": [{"id": "n1"}]}"#,
        )
        .unwrap();
        let st = store.load(&key).unwrap();
        assert_eq!(st.position, CharPos(5));
        store.save(&key, &st).unwrap();
        let text = std::fs::read_to_string(dir.path().join("untitled-3.json")).unwrap();
        assert!(text.contains("\"notes\""));
    }

    #[test]
    fn position_and_percent_follow_star() {
        let mut st = DocState::default();
        assert!(!st.has_position());
        st.set_position(CharPos(333), 1000);
        assert_eq!((st.position, st.pct), (CharPos(333), 33));
        assert!(st.has_position());
        st.set_position(CharPos(5000), 1000);
        assert_eq!((st.position, st.pct), (CharPos(1000), 100));
        assert_eq!(percent(CharPos(0), 0), 0);
        assert!(st.clear_position());
        assert!(!st.clear_position());
    }

    #[test]
    fn bookmarks_auto_name_sort_and_step() {
        let mut st = DocState::default();
        let a = st.add_bookmark(None, CharPos(50), 100);
        let b = st.add_bookmark(Some(""), CharPos(10), 100);
        let c = st.add_bookmark(Some("intro"), CharPos(30), 100);
        assert_eq!((a.name.as_str(), b.name.as_str()), ("mark1", "mark2"));
        assert_eq!(c.pct, 30);
        let order: Vec<&str> = st.bookmarks.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(order, vec!["mark2", "intro", "mark1"]);
        assert_eq!(st.next_bookmark(CharPos(10), false).unwrap().name, "intro");
        assert_eq!(st.next_bookmark(CharPos(50), false), None);
        assert_eq!(st.next_bookmark(CharPos(50), true).unwrap().name, "mark2");
        assert_eq!(
            st.previous_bookmark(CharPos(30), false).unwrap().name,
            "mark2"
        );
        assert_eq!(
            st.previous_bookmark(CharPos(10), true).unwrap().name,
            "mark1"
        );
        // Same name replaces; removing frees the name.
        st.add_bookmark(Some("intro"), CharPos(90), 100);
        assert_eq!(st.bookmarks.len(), 3);
        assert_eq!(st.bookmark("intro").unwrap().pos, CharPos(90));
        assert!(st.remove_bookmark("mark1"));
        assert!(!st.remove_bookmark("mark1"));
        assert_eq!(st.next_bookmark_name(), "mark1");
    }

    #[test]
    fn history_dedups_and_caps() {
        let mut st = DocState::default();
        for p in [1, 1, 2, 3, 3, 4] {
            st.push_history(CharPos(p), 3);
        }
        assert_eq!(st.history, vec![CharPos(2), CharPos(3), CharPos(4)]);
    }

    #[test]
    fn shift_moves_positions_across_edits() {
        let mut st = DocState::default();
        st.set_position(CharPos(20), 100);
        st.add_bookmark(Some("a"), CharPos(5), 100);
        st.add_bookmark(Some("b"), CharPos(30), 100);
        st.history = vec![CharPos(2), CharPos(25)];
        st.shift(&Edit::insert(10, "hello").outcome());
        assert_eq!(st.position, CharPos(25));
        assert_eq!(st.bookmark("a").unwrap().pos, CharPos(5));
        assert_eq!(st.bookmark("b").unwrap().pos, CharPos(35));
        assert_eq!(st.history, vec![CharPos(2), CharPos(30)]);
        st.shift(&Edit::delete(20..40).outcome());
        assert_eq!(st.position, CharPos(20));
        assert_eq!(st.bookmark("b").unwrap().pos, CharPos(20));
    }

    #[test]
    fn notes_add_edit_search_and_remove() {
        let mut st = DocState::default();
        let a = st.add_note(CharRange::new(40, 50), "second  part\n", " later ", "#b");
        let b = st.add_note(
            CharRange::new(10, 20),
            "first part",
            "Check this",
            "#exam, bio",
        );
        assert_eq!(a.anchor, "second part");
        assert_eq!(a.note, "later");
        assert_eq!(b.tags, vec!["exam", "bio"]);
        assert!(a.created > 0 && a.id.len() == 16 && a.id != b.id);
        let order: Vec<&str> = st.notes.iter().map(|n| n.note.as_str()).collect();
        assert_eq!(order, vec!["Check this", "later"], "document order");
        assert_eq!(st.search_notes("#exam").len(), 1);
        assert_eq!(st.search_notes("").len(), 2);
        assert_eq!(st.tags(), vec!["b", "bio", "exam"]);
        assert_eq!(st.notes_at(CharPos(15)).len(), 1);
        assert!(st.edit_note(&b.id, "Changed", "x"));
        assert_eq!(st.note(&b.id).unwrap().tags, vec!["x"]);
        assert!(st.edit_note(&b.id, "  ", ""), "empty text deletes");
        assert!(st.note(&b.id).is_none());
        assert!(!st.edit_note("nope", "x", ""));
        assert!(st.remove_note(&a.id));
        assert!(st.notes.is_empty());
    }

    #[test]
    fn highlights_replace_by_range_and_step_with_notes() {
        let mut st = DocState::default();
        assert!(
            st.add_highlight(CharRange::empty(5), "yellow", "")
                .is_none()
        );
        let h = st
            .add_highlight(CharRange::new(30, 40), "green", "key idea")
            .unwrap();
        assert_eq!(h.color, "#90ee90");
        st.add_highlight(CharRange::new(30, 40), "pink", "key idea");
        assert_eq!(st.highlights.len(), 1, "same range replaces");
        assert_eq!(st.highlights[0].color, "#ffc0cb");
        let n = st.add_note(CharRange::new(10, 12), "is", "why?", "");
        st.add_highlight(CharRange::new(60, 70), "yellow", "later");
        let next = st.next_annotation(CharPos(0), false).unwrap();
        assert_eq!(next.id(), n.id);
        let next = st.next_annotation(CharPos(10), false).unwrap();
        assert_eq!(next.spoken(), "Pink highlight: key idea");
        assert!(st.next_annotation(CharPos(60), false).is_none());
        assert_eq!(st.next_annotation(CharPos(60), true).unwrap().id(), n.id);
        assert_eq!(
            st.previous_annotation(CharPos(10), true).unwrap().range(),
            CharRange::new(60, 70)
        );
        assert_eq!(
            st.remove_annotation_at(CharPos(11)).as_deref(),
            Some("Note: why?, on \u{201c}is\u{201d}")
        );
        assert_eq!(
            st.remove_annotation_at(CharPos(35)).as_deref(),
            Some("Pink highlight: key idea")
        );
        assert!(st.remove_annotation_at(CharPos(35)).is_none());
        assert_eq!(st.clear_highlights(), 1);
    }

    #[test]
    fn notes_and_highlights_follow_edits() {
        let mut st = DocState::default();
        let n = st.add_note(CharRange::new(20, 30), "anchored", "n", "");
        st.add_highlight(CharRange::new(40, 50), "yellow", "hl");
        st.add_bookmark(Some("m"), CharPos(45), 100);
        st.shift(&Edit::insert(0, "abc").outcome());
        assert_eq!(st.note(&n.id).unwrap().range, CharRange::new(23, 33));
        assert_eq!(st.highlights[0].range, CharRange::new(43, 53));
        assert_eq!(st.bookmark("m").unwrap().pos, CharPos(48));
        // Deleting the highlighted text drops the highlight; deleting the
        // noted text leaves a point note that keeps its anchor.
        st.shift(&Edit::delete(20..60).outcome());
        assert!(st.highlights.is_empty());
        let note = st.note(&n.id).unwrap();
        assert_eq!(note.range, CharRange::empty(20));
        assert_eq!(note.anchor, "anchored");
    }

    #[test]
    fn notes_survive_a_save_and_old_files_still_load() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::new(dir.path().to_owned());
        let key = DocKey::untitled(9);
        let mut st = DocState::default();
        let n = st.add_note(CharRange::new(1, 4), "abc", "note", "t");
        st.notes[0]
            .extra
            .insert("sr_state".into(), serde_json::json!({"interval": 3}));
        st.add_highlight(CharRange::new(5, 9), "cyan", "text");
        store.save(&key, &st).unwrap();
        let back = store.load(&key).unwrap();
        assert_eq!(back, st);
        assert_eq!(back.note(&n.id).unwrap().extra["sr_state"]["interval"], 3);
        // A wave 1 file without notes or highlights loads with none, and a
        // state without them writes neither key.
        std::fs::write(dir.path().join("untitled-8.json"), r#"{"position": 3}"#).unwrap();
        let old = store.load(&DocKey::untitled(8)).unwrap();
        assert!(old.notes.is_empty() && old.highlights.is_empty());
        let text = serde_json::to_string(&old).unwrap();
        assert!(!text.contains("notes") && !text.contains("highlights"));
    }

    #[test]
    fn notes_export_as_markdown() {
        let mut st = DocState::default();
        st.add_note(
            CharRange::new(50, 60),
            "line one\nline two",
            "Important.",
            "#exam",
        );
        st.notes[0].ts = 1_790_344_987;
        st.add_note(CharRange::empty(0), "", "Point note", "");
        st.notes[0].ts = 0;
        st.add_highlight(CharRange::new(80, 90), "yellow", "bright words");
        let md = st.notes_markdown(&notes::NotesExport {
            title: "Essay",
            source: Some("essay.md"),
            exported: Some(1_790_344_987),
            doc_len: Some(100),
            highlights: true,
            ..notes::NotesExport::default()
        });
        let expected = "# Notes: Essay\n\n- Source: `essay.md`\n- Exported: 2026-09-25\n- Notes: 2\n- Highlights: 1\n\n## Note 1\n\nPoint note\n\n*At 0 percent*\n\n## Note 2\n\n> line one line two\n\nImportant.\n\n*At 50 percent, tags #exam, saved 2026-09-25 14:03 UTC*\n\n## Highlights\n\n- yellow, at 80 percent: bright words\n";
        assert_eq!(md, expected);
    }

    /// Star's sidecar debounce test (`test_library.py:249`), applied to the
    /// state store: four rapid saves give one disk write; the pending value
    /// is readable; flushing writes it.
    #[test]
    fn rapid_saves_coalesce_and_flush() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::with_debounce(dir.path().to_owned(), Duration::from_secs(60));
        let key = DocKey::untitled(1);
        let mut st = DocState::default();
        for pos in [10, 20, 30, 40] {
            st.position = CharPos(pos);
            store.record(&key, &st).unwrap();
        }
        assert_eq!(store.writes(), 1);
        assert!(store.has_pending());
        assert_eq!(store.load(&key).unwrap().position, CharPos(40));
        let on_disk = || {
            let text = std::fs::read_to_string(dir.path().join("untitled-1.json")).unwrap();
            serde_json::from_str::<DocState>(&text).unwrap().position
        };
        assert_eq!(on_disk(), CharPos(10));
        assert_eq!(store.flush_due().unwrap(), 0, "not due yet");
        assert_eq!(store.flush().unwrap(), 1);
        assert_eq!(on_disk(), CharPos(40));
        assert!(!store.has_pending());
    }

    #[test]
    fn zero_debounce_writes_every_time_and_flush_due_writes_old_entries() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::with_debounce(dir.path().to_owned(), Duration::ZERO);
        let key = DocKey::untitled(2);
        for pos in [1, 2, 3] {
            let st = DocState {
                position: CharPos(pos),
                ..DocState::default()
            };
            assert!(store.record(&key, &st).unwrap());
        }
        assert_eq!(store.writes(), 3);
        assert_eq!(store.flush_due().unwrap(), 0);
    }

    #[test]
    fn dropping_the_last_clone_flushes() {
        let dir = tempfile::tempdir().unwrap();
        let key = DocKey::untitled(4);
        {
            let store = StateStore::with_debounce(dir.path().to_owned(), Duration::from_secs(60));
            let clone = store.clone();
            let mut st = DocState {
                position: CharPos(1),
                ..DocState::default()
            };
            store.record(&key, &st).unwrap();
            st.position = CharPos(99);
            clone.record(&key, &st).unwrap();
            drop(store);
            assert!(clone.has_pending(), "clones share pending state");
        }
        let fresh = StateStore::new(dir.path().to_owned());
        assert_eq!(fresh.load(&key).unwrap().position, CharPos(99));
    }

    #[test]
    fn save_replaces_pending_and_remove_deletes() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::with_debounce(dir.path().to_owned(), Duration::from_secs(60));
        let key = DocKey::untitled(5);
        let mut st = DocState::default();
        store.record(&key, &st).unwrap();
        st.position = CharPos(7);
        store.record(&key, &st).unwrap();
        st.position = CharPos(8);
        store.save(&key, &st).unwrap();
        assert!(!store.has_pending());
        assert!(store.remove(&key).unwrap());
        assert!(!store.remove(&key).unwrap());
        assert!(store.load(&key).is_none());
    }

    #[test]
    fn prune_keeps_the_newest() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::new(dir.path().to_owned());
        for n in 1..=5 {
            let st = DocState {
                ts: i64::from(n),
                ..DocState::default()
            };
            store.save(&DocKey::untitled(n), &st).unwrap();
        }
        assert_eq!(store.prune(2).unwrap(), 3);
        assert!(store.load(&DocKey::untitled(5)).is_some());
        assert!(store.load(&DocKey::untitled(4)).is_some());
        assert!(store.load(&DocKey::untitled(1)).is_none());
    }

    #[test]
    fn a_corrupt_state_file_is_set_aside_not_overwritten() {
        // Before: load returned None, and the next position save replaced
        // the file, losing its notes and bookmarks.
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::new(dir.path().to_owned());
        let key = DocKey::untitled(1);
        let file = dir.path().join(format!("{}.json", key.0));
        let damaged = r#"{"position": 12, "bookmarks": [ {"pos": 3, "name": "kept"#;
        std::fs::write(&file, damaged).unwrap();
        // Unparseable files are not pruned first either.
        for n in 2..=4 {
            store
                .save(&DocKey::untitled(n), &DocState::default())
                .unwrap();
        }
        assert_eq!(store.prune(1).unwrap(), 2);
        assert!(file.exists());
        assert!(store.load(&key).is_none());
        store.save(&key, &DocState::default()).unwrap();
        let backups: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.to_string_lossy().contains(".corrupt-"))
            .collect();
        assert_eq!(backups.len(), 1, "{backups:?}");
        assert_eq!(std::fs::read_to_string(&backups[0]).unwrap(), damaged);
        assert!(backups[0].extension().is_some_and(|x| x == "bak"));
        assert!(store.load(&key).is_some());
    }
}

#[cfg(test)]
#[path = "doc_state_sync_tests.rs"]
mod sync_tests;
