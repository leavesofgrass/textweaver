//! One record per document: each computer's place, the bookmarks, notes,
//! and highlights by id, and the reading statistics, as merge types. A
//! computer writes its full merged view of a document as one record, so
//! merging a record twice, or records in any order, changes nothing.
//!
//! What a record never holds: the document's path or file name, its text
//! (only the short excerpts that anchors and highlights already keep), and
//! any computer, user, or account name.

use serde::{Deserialize, Serialize};
use textweaver_core::CharPos;
use textweaver_store::{Anchor, Bookmark, Highlight, Note};

use crate::merge::{ChangeKind, Counter, Earliest, MapChange, Maximum, RegisterMap};
use crate::{DeviceId, Stamp, SyncError, SyncId};

/// The format this version of textweaver writes and reads. A folder or a
/// file with a larger number was written by a newer textweaver: files are
/// skipped, and a newer `format.json` makes sync read-only. Any change to
/// what a record holds raises it.
pub const FORMAT: u32 = 1;

/// The largest record file read, in bytes. A larger one is reported and
/// skipped.
pub const MAX_RECORD_BYTES: u64 = 32 * 1024 * 1024;

/// The longest id a record may use for a bookmark, note, or highlight.
pub const MAX_ITEM_ID_CHARS: usize = 128;

/// A computer's reading place in a document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Place {
    /// The character position.
    pub pos: CharPos,
    /// Percentage through the document, floored.
    pub pct: u8,
    /// The text at the place, for finding it again after an edit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<Anchor>,
}

/// Reading statistics for one document, over every computer.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DocStatsRecord {
    /// Seconds read aloud, per computer; the total is the sum.
    pub seconds: Counter,
    /// Sessions, per computer; the total is the sum.
    pub sessions: Counter,
    /// The furthest point any computer reached, in percent.
    pub furthest_percent: Maximum,
    /// The furthest point any computer reached, as a character position.
    pub furthest_char: Maximum,
    /// When any computer last read it (milliseconds since 1970, UTC).
    pub last_read: Maximum,
}

impl DocStatsRecord {
    /// Merges `other` in.
    pub fn merge(&mut self, other: &Self) {
        self.seconds.merge(&other.seconds);
        self.sessions.merge(&other.sessions);
        self.furthest_percent.merge(&other.furthest_percent);
        self.furthest_char.merge(&other.furthest_char);
        self.last_read.merge(&other.last_read);
    }
}

/// Everything synced about one document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DocRecord {
    /// The format it was written in ([`FORMAT`]).
    pub format: u32,
    /// The document.
    pub sync_id: SyncId,
    /// Each computer's place, by computer id.
    #[serde(default)]
    pub places: RegisterMap<Place>,
    /// Bookmarks by id.
    #[serde(default)]
    pub bookmarks: RegisterMap<Bookmark>,
    /// Notes by id.
    #[serde(default)]
    pub notes: RegisterMap<Note>,
    /// Highlights by id.
    #[serde(default)]
    pub highlights: RegisterMap<Highlight>,
    /// Reading statistics.
    #[serde(default)]
    pub stats: DocStatsRecord,
    /// What recognizes the document on another computer: its content and
    /// text hashes, its library key, and its title, DOI, and ISBN (S2).
    /// Added without raising [`FORMAT`]: an older reader ignores it.
    #[serde(default, skip_serializing_if = "DocIdentity::is_empty")]
    pub identity: DocIdentity,
}

/// How many hashes of each kind a record keeps: the newest, and the ones
/// before it, so a computer holding an older copy of an edited file still
/// recognizes it.
pub const RECENT_HASHES: usize = 8;

/// The most hashes of one kind a record file may hold.
const MAX_HASHES_READ: usize = 64;

/// SHA-256 hashes (64 lower-case hex digits), each with the stamp it was
/// last published at. Only the [`RECENT_HASHES`] newest are kept. Merging
/// keeps each hash's newest stamp, then the newest hashes, so merge order
/// and repeats never change the result.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecentHashes(pub std::collections::BTreeMap<String, Stamp>);

impl RecentHashes {
    /// No hashes.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Publishes `hash` at `stamp`.
    pub fn publish(&mut self, hash: &str, stamp: Stamp) {
        let e = self.0.entry(hash.to_owned()).or_insert(stamp);
        *e = (*e).max(stamp);
        self.trim();
    }

    /// Whether `hash` is held.
    pub fn contains(&self, hash: &str) -> bool {
        self.0.contains_key(hash)
    }

    /// The newest hash.
    pub fn newest(&self) -> Option<&str> {
        self.0
            .iter()
            .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
            .map(|(h, _)| h.as_str())
    }

    /// Merges `other` in.
    pub fn merge(&mut self, other: &Self) {
        for (h, s) in &other.0 {
            let e = self.0.entry(h.clone()).or_insert(*s);
            *e = (*e).max(*s);
        }
        self.trim();
    }

    /// Keeps the [`RECENT_HASHES`] newest (ties: the smaller hash).
    fn trim(&mut self) {
        if self.0.len() <= RECENT_HASHES {
            return;
        }
        let mut all: Vec<(Stamp, String)> = std::mem::take(&mut self.0)
            .into_iter()
            .map(|(h, s)| (s, h))
            .collect();
        all.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        all.truncate(RECENT_HASHES);
        self.0 = all.into_iter().map(|(s, h)| (h, s)).collect();
    }

    fn validate(&self) -> Result<(), SyncError> {
        if self.0.len() > MAX_HASHES_READ {
            return Err(SyncError::Damaged("too many hashes".into()));
        }
        if self.0.keys().all(|h| is_sha256(h)) {
            Ok(())
        } else {
            Err(SyncError::Damaged("a hash is not 64 hex digits".into()))
        }
    }
}

/// Whether `s` is 64 lower-case hex digits.
pub(crate) fn is_sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The detail names [`DocIdentity::details`] uses: the library details
/// (ADR-0049), newest wins per detail.
pub mod detail {
    /// The document's title.
    pub const TITLE: &str = "title";
    /// Its DOI, lowercase (`10.1000/xyz`).
    pub const DOI: &str = "doi";
    /// Its ISBN, digits only (and a final `X` for an ISBN-10).
    pub const ISBN: &str = "isbn";
    /// Its author or authors, as the document gives them.
    pub const AUTHOR: &str = "author";
    /// The kind of file, by the loader that reads it (`markdown`, `pdf`).
    pub const FORMAT: &str = "format";
}

/// The longest detail value read or written, in characters.
pub const MAX_DETAIL_CHARS: usize = 400;

/// What recognizes a document on another computer (ADR-0049,
/// "Recognizing the same document"). No path or file name is ever part of
/// it: the library key is a hash of the library folder's id and the path
/// inside the folder.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DocIdentity {
    /// SHA-256 of the file's bytes.
    #[serde(skip_serializing_if = "RecentHashes::is_empty")]
    pub content: RecentHashes,
    /// SHA-256 of the text as textweaver reads it.
    #[serde(skip_serializing_if = "RecentHashes::is_empty")]
    pub text: RecentHashes,
    /// SHA-256 of the library folder's id and the path inside the folder.
    #[serde(skip_serializing_if = "RecentHashes::is_empty")]
    pub library: RecentHashes,
    /// Title, DOI, ISBN, author, and format ([`detail`]), newest wins per
    /// detail. A DOI or an ISBN is only ever a suggestion, never a match on
    /// its own.
    #[serde(skip_serializing_if = "details_empty")]
    pub details: RegisterMap<String>,
    /// When the document was first added to a library on any computer
    /// (milliseconds since 1970, UTC): the earliest wins (S6).
    #[serde(skip_serializing_if = "Earliest::is_none")]
    pub added: Earliest,
}

fn details_empty(d: &RegisterMap<String>) -> bool {
    d.0.is_empty()
}

impl DocIdentity {
    /// Nothing recorded.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
            && self.text.is_empty()
            && self.library.is_empty()
            && self.details.0.is_empty()
            && self.added.is_none()
    }

    /// A detail's live value.
    pub fn detail(&self, name: &str) -> Option<&str> {
        self.details.get(name).map(String::as_str)
    }

    /// Merges `other` in.
    pub fn merge(&mut self, other: &Self) {
        self.content.merge(&other.content);
        self.text.merge(&other.text);
        self.library.merge(&other.library);
        self.details.merge(&other.details);
        self.added.merge(&other.added);
    }

    /// Every stamp held.
    pub fn stamps(&self) -> impl Iterator<Item = Stamp> + '_ {
        self.content
            .0
            .values()
            .chain(self.text.0.values())
            .chain(self.library.0.values())
            .copied()
            .chain(self.details.stamps())
    }

    fn validate(&self) -> Result<(), SyncError> {
        self.content.validate()?;
        self.text.validate()?;
        self.library.validate()?;
        if self.details.0.len() > 16 {
            return Err(SyncError::Damaged("too many details".into()));
        }
        let long = self
            .details
            .live()
            .any(|(k, v)| k.chars().count() > 32 || v.chars().count() > MAX_DETAIL_CHARS);
        if long {
            return Err(SyncError::Damaged("a detail is too long".into()));
        }
        Ok(())
    }
}

/// The kind of item a [`Change`] is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ItemKind {
    /// A computer's place; the change's id is the computer's id.
    Place,
    /// A bookmark.
    Bookmark,
    /// A note.
    Note,
    /// A highlight.
    Highlight,
}

/// What an item held before a merge replaced or removed it.
#[derive(Clone, Debug, PartialEq)]
pub enum Previous {
    /// A place.
    Place(Place),
    /// A bookmark.
    Bookmark(Bookmark),
    /// A note: the text that was replaced, for the local backup of
    /// replaced notes.
    Note(Box<Note>),
    /// A highlight.
    Highlight(Highlight),
}

/// One change a merge made, for the app to announce.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    /// What kind of item.
    pub item: ItemKind,
    /// The item's id.
    pub id: String,
    /// What happened to it.
    pub kind: ChangeKind,
    /// The computer whose change won.
    pub by: DeviceId,
    /// What it held here before, when it was replaced or removed.
    pub previous: Option<Previous>,
}

/// What merging another computer's record changed here.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MergeReport {
    /// The changes, places first, then bookmarks, notes, and highlights,
    /// each in id order.
    pub changes: Vec<Change>,
}

impl MergeReport {
    /// Whether nothing a reader would notice changed.
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Adds `other`'s changes after these.
    pub fn extend(&mut self, other: MergeReport) {
        self.changes.extend(other.changes);
    }

    /// Notes replaced by a newer edit from another computer: the note's
    /// id, the text it had here, and the computer whose edit won. The
    /// owner's rule: the newest edit wins, the reader hears which note was
    /// replaced, and the older text goes to a local backup.
    pub fn replaced_notes(&self) -> impl Iterator<Item = (&str, &Note, DeviceId)> {
        self.changes
            .iter()
            .filter_map(|c| match (&c.kind, &c.previous) {
                (ChangeKind::Replaced, Some(Previous::Note(n))) => {
                    Some((c.id.as_str(), &**n, c.by))
                }
                _ => None,
            })
    }

    /// Changes of `kind` to items of `item`.
    pub fn count(&self, item: ItemKind, kind: ChangeKind) -> usize {
        self.changes
            .iter()
            .filter(|c| c.item == item && c.kind == kind)
            .count()
    }
}

fn changes<T>(
    item: ItemKind,
    list: Vec<MapChange<T>>,
    wrap: fn(T) -> Previous,
) -> impl Iterator<Item = Change> {
    list.into_iter().map(move |c| Change {
        item,
        id: c.id,
        kind: c.kind,
        by: c.by,
        previous: c.previous.map(wrap),
    })
}

impl DocRecord {
    /// An empty record for `sync_id`.
    pub fn new(sync_id: SyncId) -> Self {
        Self {
            format: FORMAT,
            sync_id,
            places: RegisterMap::new(),
            bookmarks: RegisterMap::new(),
            notes: RegisterMap::new(),
            highlights: RegisterMap::new(),
            stats: DocStatsRecord::default(),
            identity: DocIdentity::default(),
        }
    }

    /// Sets the place of the computer that made `stamp`.
    pub fn set_place(&mut self, stamp: Stamp, place: Place) {
        self.places.set(stamp.device.to_string(), stamp, place);
    }

    /// `device`'s place.
    pub fn place_of(&self, device: DeviceId) -> Option<&Place> {
        self.places.get(&device.to_string())
    }

    /// Every computer's place, with the stamp it was set at, newest first.
    pub fn places_newest_first(&self) -> Vec<(Stamp, &Place)> {
        let mut v: Vec<(Stamp, &Place)> = self
            .places
            .0
            .values()
            .filter_map(|r| r.value.as_ref().map(|p| (r.stamp, p)))
            .collect();
        v.sort_by_key(|(stamp, _)| std::cmp::Reverse(*stamp));
        v
    }

    /// Every stamp in the record, for the clock to take in.
    pub fn stamps(&self) -> impl Iterator<Item = Stamp> + '_ {
        self.places
            .stamps()
            .chain(self.bookmarks.stamps())
            .chain(self.notes.stamps())
            .chain(self.highlights.stamps())
            .chain(self.identity.stamps())
    }

    /// Merges another computer's record for the same document in, and
    /// lists what changed here.
    pub fn merge(&mut self, other: &DocRecord) -> Result<MergeReport, SyncError> {
        if other.sync_id != self.sync_id {
            return Err(SyncError::DifferentDocument);
        }
        let mut report = MergeReport::default();
        report.changes.extend(changes(
            ItemKind::Place,
            self.places.merge(&other.places),
            Previous::Place,
        ));
        report.changes.extend(changes(
            ItemKind::Bookmark,
            self.bookmarks.merge(&other.bookmarks),
            Previous::Bookmark,
        ));
        report.changes.extend(changes(
            ItemKind::Note,
            self.notes.merge(&other.notes),
            |n| Previous::Note(Box::new(n)),
        ));
        report.changes.extend(changes(
            ItemKind::Highlight,
            self.highlights.merge(&other.highlights),
            Previous::Highlight,
        ));
        self.stats.merge(&other.stats);
        self.identity.merge(&other.identity);
        Ok(report)
    }

    /// Checks what serde cannot: places are keyed by computer ids, and item
    /// ids are not empty or overlong.
    fn validate(&self) -> Result<(), SyncError> {
        for id in self.places.0.keys() {
            id.parse::<DeviceId>()
                .map_err(|_| SyncError::Damaged("a place is not keyed by a computer id".into()))?;
        }
        let ids = self
            .bookmarks
            .0
            .keys()
            .chain(self.notes.0.keys())
            .chain(self.highlights.0.keys());
        for id in ids {
            if id.is_empty() || id.chars().count() > MAX_ITEM_ID_CHARS {
                return Err(SyncError::Damaged("an item id is empty or too long".into()));
            }
        }
        self.identity.validate()
    }

    /// Reads a record from a file's bytes: JSON in [`FORMAT`] or older, at
    /// most [`MAX_RECORD_BYTES`] long. A truncated or damaged file is an
    /// error, never a panic; a newer format is [`SyncError::NewerFormat`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SyncError> {
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_RECORD_BYTES {
            return Err(SyncError::TooLarge);
        }
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|e| SyncError::Damaged(e.to_string()))?;
        let format = value
            .get("format")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| SyncError::Damaged("no format number".into()))?;
        if format > u64::from(FORMAT) {
            return Err(SyncError::NewerFormat {
                found: u32::try_from(format).unwrap_or(u32::MAX),
            });
        }
        let mut record: DocRecord =
            serde_json::from_value(value).map_err(|e| SyncError::Damaged(e.to_string()))?;
        record.validate()?;
        // An older record is read as this format; it is written back as one.
        record.format = FORMAT;
        Ok(record)
    }

    /// The record as a file's bytes (compact JSON, one line).
    pub fn to_bytes(&self) -> Result<Vec<u8>, SyncError> {
        let mut out = serde_json::to_vec(self).map_err(|e| SyncError::Damaged(e.to_string()))?;
        out.push(b'\n');
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: DeviceId = DeviceId::from_u128(0xa);
    const B: DeviceId = DeviceId::from_u128(0xb);
    const DOC: SyncId = SyncId::from_u128(0xd0c);

    fn note(id: &str, text: &str, ts: i64) -> Note {
        Note {
            id: id.to_owned(),
            range: textweaver_core::CharRange::new(CharPos(0), CharPos(5)),
            anchor: "Cells".to_owned(),
            note: text.to_owned(),
            tags: Vec::new(),
            cite: String::new(),
            color: None,
            relations: Vec::new(),
            created: ts,
            ts,
            extra: serde_json::Map::new(),
        }
    }

    #[test]
    fn a_record_round_trips() {
        let mut r = DocRecord::new(DOC);
        r.set_place(
            Stamp::new(100, A),
            Place {
                pos: CharPos(42),
                pct: 7,
                anchor: None,
            },
        );
        r.notes
            .set("n1", Stamp::new(101, A), note("n1", "Mitosis", 1));
        r.stats.seconds.add(A, 60);
        let bytes = r.to_bytes().unwrap();
        assert_eq!(DocRecord::from_bytes(&bytes).unwrap(), r);
    }

    #[test]
    fn a_replaced_note_is_listed_with_its_old_text() {
        let mut here = DocRecord::new(DOC);
        here.notes
            .set("n1", Stamp::new(10, A), note("n1", "old", 1));
        let mut there = DocRecord::new(DOC);
        there
            .notes
            .set("n1", Stamp::new(20, B), note("n1", "new", 2));
        let report = here.merge(&there).unwrap();
        let replaced: Vec<_> = report.replaced_notes().collect();
        assert_eq!(replaced.len(), 1);
        assert_eq!(replaced[0].0, "n1");
        assert_eq!(replaced[0].1.note, "old");
        assert_eq!(replaced[0].2, B);
        assert_eq!(here.notes.get("n1").unwrap().note, "new");
    }

    #[test]
    fn places_are_kept_per_computer() {
        let mut here = DocRecord::new(DOC);
        let p = |n| Place {
            pos: CharPos(n),
            pct: 0,
            anchor: None,
        };
        here.set_place(Stamp::new(10, A), p(1));
        let mut there = DocRecord::new(DOC);
        there.set_place(Stamp::new(20, B), p(2));
        let report = here.merge(&there).unwrap();
        assert_eq!(report.count(ItemKind::Place, ChangeKind::Added), 1);
        assert_eq!(here.place_of(A), Some(&p(1)));
        assert_eq!(here.place_of(B), Some(&p(2)));
        assert_eq!(here.places_newest_first()[0].0.device, B);
    }

    #[test]
    fn identity_round_trips_and_keeps_the_newest_hashes() {
        let h = |n: u64| format!("{n:064x}");
        let mut r = DocRecord::new(DOC);
        for n in 0..12u64 {
            r.identity.content.publish(&h(n), Stamp::new(n, A));
        }
        assert_eq!(r.identity.content.0.len(), RECENT_HASHES);
        assert_eq!(r.identity.content.newest(), Some(h(11).as_str()));
        assert!(!r.identity.content.contains(&h(3)));
        r.identity
            .details
            .set(detail::ISBN, Stamp::new(20, A), "9780306406157".to_owned());
        assert_eq!(r.identity.detail(detail::ISBN), Some("9780306406157"));
        assert!(r.stamps().any(|s| s.time == 20));
        let bytes = r.to_bytes().unwrap();
        assert_eq!(DocRecord::from_bytes(&bytes).unwrap(), r);

        // A record with no identity writes none (older readers see what
        // they always saw).
        let plain = String::from_utf8(DocRecord::new(DOC).to_bytes().unwrap()).unwrap();
        assert!(!plain.contains("identity"));

        // Only hex digests are read as hashes; a path never is.
        let bad = format!(
            "{{\"format\":1,\"sync_id\":\"{DOC}\",\"identity\":{{\"content\":{{\"C:/Users/x.md\":{{\"wall_ms\":1,\"device\":\"{A}\"}}}}}}}}"
        );
        assert!(DocRecord::from_bytes(bad.as_bytes()).is_err());
    }

    #[test]
    fn another_document_is_refused() {
        let mut here = DocRecord::new(DOC);
        let there = DocRecord::new(SyncId::from_u128(1));
        assert!(matches!(
            here.merge(&there),
            Err(SyncError::DifferentDocument)
        ));
    }

    #[test]
    fn damaged_truncated_and_newer_files_are_errors() {
        let mut r = DocRecord::new(DOC);
        r.notes.set("n1", Stamp::new(1, A), note("n1", "x", 1));
        let bytes = r.to_bytes().unwrap();
        for cut in 0..bytes.len() - 2 {
            assert!(
                DocRecord::from_bytes(&bytes[..cut]).is_err(),
                "cut at {cut}"
            );
        }
        assert!(DocRecord::from_bytes(b"\xff\xfe{").is_err());
        let newer = String::from_utf8(bytes)
            .unwrap()
            .replacen("\"format\":1", "\"format\":2", 1);
        assert!(matches!(
            DocRecord::from_bytes(newer.as_bytes()),
            Err(SyncError::NewerFormat { found: 2 })
        ));
        let bad_place = format!(
            "{{\"format\":1,\"sync_id\":\"{DOC}\",\"places\":{{\"laptop\":{{\"stamp\":{{\"wall_ms\":1,\"device\":\"{A}\"}}}}}}}}"
        );
        assert!(DocRecord::from_bytes(bad_place.as_bytes()).is_err());
    }
}
