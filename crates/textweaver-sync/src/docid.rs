//! Document identity (ADR-0049, "Recognizing the same document").
//!
//! Every document gets a random 128-bit [`SyncId`], the same on every
//! computer once the document is recognized there. The local
//! `sync-ids.json` ([`textweaver_store::sync_ids`]) maps this computer's
//! path keys to sync ids. A document with no sync id yet is recognized, in
//! order, by:
//!
//! 1. the file's exact contents (SHA-256 of its bytes);
//! 2. the SHA-256 of its text as textweaver reads it, with runs of white
//!    space made one space, so the same book saved or converted
//!    differently still matches;
//! 3. its library folder's id plus its path inside the folder, hashed
//!    together ([`library_key`]). Each library folder gets a small id file,
//!    `.textweaver/library-id.json`, beside the old progress sidecar, made
//!    the first time a document in the folder is identified;
//! 4. a DOI or an ISBN, which is only ever **suggested**, never matched on
//!    its own, because two chapters of one book share an ISBN. The app
//!    asks: "This may be Cells from the laptop, with 4 notes. Use them? Yes
//!    or no." ([`Suggestion`], [`accept`], [`decline`]).
//!
//! Each step looks first on this computer (the same file at another path:
//! renamed, moved, or copied), then in the other computers' records
//! ([`IdentityIndex`]). Failing all of them, the document gets a new id.
//!
//! Two computers that opened one document before they ever synced each
//! gave it an id of their own. Once their records meet, the ids are
//! **folded**: every time a document is identified, the smallest id among
//! the records that share its content hash, text hash, or library key wins
//! ([`IdentityIndex::smallest_match`]). The computer whose id lost takes the
//! smaller one ([`Resolved::folded_from`]) and publishes its notes, places,
//! bookmarks, and highlights under it; its old record is marked
//! [`DocRecord::folded_into`] so readers count it as part of the winner.
//! The computer holding the smallest id changes nothing, so every computer
//! settles on the same id whatever order they open the document in.
//!
//! After an edit the id stays: the path key still maps to it, the file's
//! new hashes replace the old ones in `sync-ids.json`, and
//! [`Resolved::changed`] says they should be published
//! ([`DocRecord::publish_identity`]). A record keeps the last few hashes,
//! so a computer holding the older copy still recognizes it.
//!
//! Hashing reads the whole file and walks the whole text, so it runs only
//! on the app's background writer, never on the input thread. An unchanged
//! file (same size and modification time) is not hashed again.
//!
//! Nothing here writes a path or a file name to the sync folder: records
//! carry hashes, and the title, DOI, ISBN, author, and format the document
//! states.

use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use textweaver_store::sync_ids::{HashKind, SyncIdEntry, SyncIds};
use textweaver_store::{DocKey, StoreError};

use crate::record::{DocIdentity, detail};
use crate::{DeviceId, DocRecord, LibraryId, Stamp, SyncError, SyncId};

/// The library folder id file's name, in the folder's `.textweaver`
/// folder (where the old progress sidecar lives too).
pub const LIBRARY_ID_FILE: &str = "library-id.json";

/// The buffer a file is read through while hashing: small enough to stay
/// out of the way of a reader's memory, large enough to read fast.
const READ_BUFFER: usize = 256 * 1024;

/// The largest library id file read, in bytes.
const MAX_LIBRARY_ID_BYTES: u64 = 4096;

fn io_err(path: &Path) -> impl FnOnce(std::io::Error) -> SyncError + '_ {
    move |source| SyncError::Io {
        path: path.to_owned(),
        source,
    }
}

fn from_store(e: StoreError) -> SyncError {
    SyncError::from(e)
}

fn hex(digest: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// SHA-256 of a file's bytes, as 64 lower-case hex digits. Reads the file
/// through a small buffer, so a 100 MB file does not take 100 MB of memory.
pub fn file_sha256(path: &Path) -> Result<String, SyncError> {
    let mut f = std::fs::File::open(path).map_err(io_err(path))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; READ_BUFFER];
    loop {
        let n = match f.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(io_err(path)(e)),
        };
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

/// SHA-256 of a text as textweaver reads it, given as consecutive pieces
/// (a rope's chunks). Runs of white space count as one space, and white
/// space at either end is left out, so line endings, indentation, and
/// wrapping do not change the hash.
#[derive(Clone, Debug, Default)]
pub struct TextHasher {
    hasher: Sha256,
    started: bool,
    space: bool,
    buf: Vec<u8>,
}

impl TextHasher {
    /// A hasher with nothing in it.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds the next piece of the text.
    pub fn update(&mut self, piece: &str) {
        let mut utf8 = [0u8; 4];
        for c in piece.chars() {
            if c.is_whitespace() {
                self.space = self.started;
                continue;
            }
            if self.space {
                self.buf.push(b' ');
                self.space = false;
            }
            self.started = true;
            self.buf
                .extend_from_slice(c.encode_utf8(&mut utf8).as_bytes());
            if self.buf.len() >= 8192 {
                self.hasher.update(&self.buf);
                self.buf.clear();
            }
        }
    }

    /// The hash, as 64 lower-case hex digits.
    pub fn finish(mut self) -> String {
        self.hasher.update(&self.buf);
        hex(&self.hasher.finalize())
    }
}

/// SHA-256 of a text given as pieces ([`TextHasher`]).
pub fn text_sha256<'a>(pieces: impl IntoIterator<Item = &'a str>) -> String {
    let mut h = TextHasher::new();
    for p in pieces {
        h.update(p);
    }
    h.finish()
}

/// The library key of a document: SHA-256 of its library folder's id and
/// its path inside the folder (with `/` between parts). Only the hash
/// travels; the path never does.
pub fn library_key(library: LibraryId, relative: &str) -> String {
    let mut h = Sha256::new();
    h.update(library.to_string().as_bytes());
    h.update(b"\0");
    h.update(relative.trim_matches('/').as_bytes());
    hex(&h.finalize())
}

/// `library-id.json` in a library folder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct LibraryIdFile {
    format: u32,
    library_id: LibraryId,
}

/// Where a library folder's id file is: `<folder>/.textweaver/library-id.json`.
pub fn library_id_file(folder: &Path) -> PathBuf {
    folder
        .join(textweaver_store::sync::SIDECAR_DIR)
        .join(LIBRARY_ID_FILE)
}

/// A library folder's id: read from its id file, or made and saved there
/// the first time. A damaged id file is left alone (it may be a sync
/// service still copying it) and the error returned; the document is then
/// identified without it.
pub fn library_id(folder: &Path) -> Result<LibraryId, SyncError> {
    let file = library_id_file(folder);
    match std::fs::metadata(&file) {
        Ok(m) if m.len() > MAX_LIBRARY_ID_BYTES => return Err(SyncError::TooLarge),
        Ok(_) => {
            let bytes = std::fs::read(&file).map_err(io_err(&file))?;
            let f: LibraryIdFile = serde_json::from_slice(&bytes)
                .map_err(|e| SyncError::Damaged(format!("library id file: {e}")))?;
            return Ok(f.library_id);
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(io_err(&file)(e)),
    }
    if !folder.is_dir() {
        return Err(io_err(folder)(std::io::Error::from(
            std::io::ErrorKind::NotFound,
        )));
    }
    let id = LibraryId::random();
    let body = serde_json::to_vec_pretty(&LibraryIdFile {
        format: crate::FORMAT,
        library_id: id,
    })
    .map_err(|e| SyncError::Damaged(e.to_string()))?;
    textweaver_store::atomic_write(&file, &body).map_err(from_store)?;
    Ok(id)
}

/// A library folder's id when its id file is there and can be read, without
/// making one: for looking documents up (the library list, "Continue
/// reading") where nothing should be written.
pub fn read_library_id(folder: &Path) -> Option<LibraryId> {
    let file = library_id_file(folder);
    if std::fs::metadata(&file).ok()?.len() > MAX_LIBRARY_ID_BYTES {
        return None;
    }
    let bytes = std::fs::read(&file).ok()?;
    serde_json::from_slice::<LibraryIdFile>(&bytes)
        .ok()
        .map(|f| f.library_id)
}

/// What recognizes one document here: its hashes, and the file's size and
/// modification time when they were taken.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fingerprint {
    /// SHA-256 of the file's bytes.
    pub content_sha256: Option<String>,
    /// SHA-256 of its text ([`text_sha256`]).
    pub text_sha256: Option<String>,
    /// Its [`library_key`], when it is in a library folder.
    pub library_key: Option<String>,
    /// The file's size in bytes.
    pub size: Option<u64>,
    /// The file's modification time, in milliseconds since 1970 (UTC).
    pub modified_ms: Option<u64>,
}

/// What a document says about itself: its title, DOI, ISBN, author, and
/// format (from the document, as the bookshelf keeps them), and when it was
/// first added to the library here. These are the library details the
/// record carries (ADR-0049): newest wins per detail, and the earliest
/// "first added" wins.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Details {
    /// The title.
    pub title: Option<String>,
    /// The DOI, lowercase (`10.1000/xyz`).
    pub doi: Option<String>,
    /// The ISBN, digits only (and a final `X` for an ISBN-10).
    pub isbn: Option<String>,
    /// The author or authors.
    pub author: Option<String>,
    /// The kind of file, by the loader that reads it (`markdown`, `pdf`).
    pub format: Option<String>,
    /// When it was first added to the library on this computer
    /// (milliseconds since 1970, UTC).
    pub added_ms: Option<u64>,
}

impl Details {
    fn pairs(&self) -> [(&'static str, Option<&str>); 5] {
        [
            (detail::TITLE, self.title.as_deref()),
            (detail::DOI, self.doi.as_deref()),
            (detail::ISBN, self.isbn.as_deref()),
            (detail::AUTHOR, self.author.as_deref()),
            (detail::FORMAT, self.format.as_deref()),
        ]
    }

    /// Whether publishing these details would change `identity`: a detail
    /// it lacks or holds differently, or an earlier "first added".
    pub fn would_change(&self, identity: &DocIdentity) -> bool {
        let detail_differs = self.pairs().into_iter().any(|(name, value)| {
            clean_detail(value).is_some_and(|v| identity.detail(name) != Some(v.as_str()))
        });
        let earlier = self
            .added_ms
            .is_some_and(|a| identity.added.0.is_none_or(|b| a < b));
        detail_differs || earlier
    }
}

/// One library detail the owner edited by hand (Wave 7, W7m): `value` is
/// the new value, or `None` when the owner cleared the field, so the
/// document's own value shows again. Only the fields the owner changed are
/// published, so editing the title never touches another computer's
/// hand-edited author.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailEdit {
    /// The detail: [`detail::TITLE`], [`detail::AUTHOR`], [`detail::DOI`],
    /// or [`detail::ISBN`].
    pub name: &'static str,
    /// The value typed, or `None` for a cleared field.
    pub value: Option<String>,
}

/// A detail as published: trimmed, not empty, at most
/// [`crate::record::MAX_DETAIL_CHARS`] characters.
fn clean_detail(value: Option<&str>) -> Option<String> {
    let v = value.map(str::trim).filter(|v| !v.is_empty())?;
    Some(v.chars().take(crate::record::MAX_DETAIL_CHARS).collect())
}

/// How a document's sync id was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Found {
    /// `sync-ids.json` already had it for this path.
    Known,
    /// By the file's SHA-256.
    Content,
    /// By the SHA-256 of its text.
    Text,
    /// By its library folder's id and its path inside the folder.
    Library,
    /// Not found: a new id was made.
    New,
}

/// Which identifier a [`Suggestion`] shares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedIdentifier {
    /// The same DOI.
    Doi,
    /// The same ISBN.
    Isbn,
}

/// A document on another computer that may be this one, because it has the
/// same DOI or ISBN. Never used without asking: "This may be Cells from the
/// laptop, with 4 notes. Use them? Yes or no."
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    /// Its sync id.
    pub sync_id: SyncId,
    /// Its title, as that computer knows it.
    pub title: Option<String>,
    /// The computers with a record of it, for naming one by its label.
    pub devices: Vec<DeviceId>,
    /// How many notes it has.
    pub notes: usize,
    /// What it shares with this document.
    pub shared: SharedIdentifier,
}

/// What identifying a document found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    /// The document's path key here.
    pub key: DocKey,
    /// Its sync id.
    pub sync_id: SyncId,
    /// How it was found.
    pub found: Found,
    /// Its hashes now.
    pub fingerprint: Fingerprint,
    /// Its hashes are new or changed since they were last published (a
    /// new document, or one edited since): the record should publish them.
    pub changed: bool,
    /// Documents on other computers with the same DOI or ISBN, to ask
    /// about. Empty once another computer has this document's id.
    pub suggestions: Vec<Suggestion>,
    /// The id this document had here before it was folded into
    /// [`sync_id`](Self::sync_id), a smaller id another computer gave the
    /// same document; `None` when it kept its id.
    pub folded_from: Option<SyncId>,
}

/// One document as the other computers' records know it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Known {
    identity: DocIdentity,
    devices: Vec<DeviceId>,
    notes: usize,
}

/// What every computer's records say about every document, for
/// recognizing documents opened here ([`crate::SyncFolder::identity_index`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IdentityIndex {
    docs: std::collections::BTreeMap<SyncId, Known>,
}

impl IdentityIndex {
    /// An empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `device`'s record. A record folded into a smaller id counts
    /// as that id's.
    pub fn add(&mut self, device: DeviceId, record: &DocRecord) {
        let id = record.folded_into.unwrap_or(record.sync_id);
        let k = self.docs.entry(id).or_default();
        k.identity.merge(&record.identity);
        if !k.devices.contains(&device) {
            k.devices.push(device);
            k.devices.sort();
        }
        k.notes = k.notes.max(record.notes.live_len());
    }

    /// Whether any computer has a record of `sync_id`.
    pub fn contains(&self, sync_id: SyncId) -> bool {
        self.docs.contains_key(&sync_id)
    }

    /// How many documents it knows.
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    /// Whether it knows none.
    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    /// The document whose hashes of `kind` include `hash`; when several do,
    /// the one that published it last (ties: the smallest sync id), so
    /// every computer picks the same one.
    pub fn find(&self, kind: HashKind, hash: &str) -> Option<SyncId> {
        self.docs
            .iter()
            .filter_map(|(id, k)| {
                let set = match kind {
                    HashKind::Content => &k.identity.content,
                    HashKind::Text => &k.identity.text,
                    HashKind::Library => &k.identity.library,
                };
                set.0.get(hash).map(|stamp| (*stamp, *id))
            })
            .max_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)))
            .map(|(_, id)| id)
    }

    /// The smallest sync id among the documents whose content hashes, text
    /// hashes, or library keys include one of `fp`'s: the id two computers'
    /// ids for one document fold into. A DOI or an ISBN never counts.
    pub fn smallest_match(&self, fp: &Fingerprint) -> Option<SyncId> {
        self.docs
            .iter()
            .filter(|(_, k)| {
                let has = |set: &crate::RecentHashes, h: &Option<String>| {
                    h.as_deref().is_some_and(|h| set.contains(h))
                };
                has(&k.identity.content, &fp.content_sha256)
                    || has(&k.identity.text, &fp.text_sha256)
                    || has(&k.identity.library, &fp.library_key)
            })
            .map(|(id, _)| *id)
            .min()
    }

    /// Documents with the same DOI or ISBN as `details`, other than
    /// `except` and those in `declined`, in sync id order.
    pub fn suggestions(
        &self,
        details: &Details,
        except: SyncId,
        declined: &[String],
    ) -> Vec<Suggestion> {
        let same = |k: &Known, name: &str, ours: Option<&str>| {
            ours.is_some_and(|o| !o.is_empty() && k.identity.shown_detail(name) == Some(o))
        };
        self.docs
            .iter()
            .filter(|(id, _)| **id != except && !declined.iter().any(|d| *d == id.to_string()))
            .filter_map(|(id, k)| {
                let shared = if same(k, detail::DOI, details.doi.as_deref()) {
                    SharedIdentifier::Doi
                } else if same(k, detail::ISBN, details.isbn.as_deref()) {
                    SharedIdentifier::Isbn
                } else {
                    return None;
                };
                Some(Suggestion {
                    sync_id: *id,
                    title: k.identity.shown_detail(detail::TITLE).map(str::to_owned),
                    devices: k.devices.clone(),
                    notes: k.notes,
                    shared,
                })
            })
            .collect()
    }
}

impl DocRecord {
    /// Publishes a document's hashes and details at `stamp`: the newest
    /// hashes win, and older ones are kept a while
    /// ([`crate::record::RECENT_HASHES`]); a detail is set only when it
    /// differs, and "first added" only ever moves earlier.
    pub fn publish_identity(&mut self, stamp: Stamp, fp: &Fingerprint, details: &Details) {
        let id = &mut self.identity;
        if let Some(h) = &fp.content_sha256 {
            id.content.publish(h, stamp);
        }
        if let Some(h) = &fp.text_sha256 {
            id.text.publish(h, stamp);
        }
        if let Some(h) = &fp.library_key {
            id.library.publish(h, stamp);
        }
        for (name, value) in details.pairs() {
            let Some(v) = clean_detail(value) else {
                continue;
            };
            if id.detail(name) != Some(v.as_str()) {
                id.details.set(name, stamp, v);
            }
        }
        if let Some(a) = details.added_ms {
            id.added.lower(a);
        }
    }

    /// Publishes the owner's hand edits at `stamp` (Wave 7, W7m): a value
    /// sets the detail's hand-edited register, `None` clears it with a
    /// deletion record, so the clearing travels too. Newest wins per
    /// detail, as for every library detail. Returns whether anything
    /// changed; edits of details that cannot be edited are ignored.
    pub fn publish_edits(&mut self, stamp: Stamp, edits: &[DetailEdit]) -> bool {
        let mut changed = false;
        for e in edits {
            let Some(name) = detail::edited(e.name) else {
                continue;
            };
            let details = &mut self.identity.details;
            match clean_detail(e.value.as_deref()) {
                Some(v) if details.get(name) != Some(&v) => {
                    details.set(name, stamp, v);
                    changed = true;
                }
                Some(_) => {}
                None if details.register(name).is_some_and(|r| r.value.is_some()) => {
                    details.delete(name, stamp);
                    changed = true;
                }
                None => {}
            }
        }
        changed
    }
}

fn modified_ms(m: &std::fs::Metadata) -> Option<u64> {
    let t = m.modified().ok()?;
    let d = t.duration_since(std::time::UNIX_EPOCH).ok()?;
    u64::try_from(d.as_millis()).ok()
}

/// Identifies one document: the job the app's background writer runs when
/// a document opens or is saved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identify {
    /// `sync-ids.json` ([`textweaver_store::Paths::sync_ids_file`]).
    pub ids_file: PathBuf,
    /// The document's file.
    pub path: PathBuf,
    /// The library folders (`[library] folders`).
    pub library_folders: Vec<PathBuf>,
    /// What the document says about itself.
    pub details: Details,
}

impl Identify {
    /// Finds or makes the document's sync id and saves it with its hashes
    /// in `sync-ids.json`. `text` is the document's text as read (a rope's
    /// chunks), when there is one; `index` is what the other computers'
    /// records say, when sync is on.
    ///
    /// Reads and hashes the whole file when it changed or is new here, so
    /// call it only off the input thread.
    pub fn run<'a, I>(
        &self,
        text: Option<I>,
        index: Option<&IdentityIndex>,
    ) -> Result<Resolved, SyncError>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let key = DocKey::for_path(&self.path);
        let mut ids = SyncIds::load(&self.ids_file);
        let meta = std::fs::metadata(&self.path).map_err(io_err(&self.path))?;
        let (size, modified) = (Some(meta.len()), modified_ms(&meta));
        let entry = ids.get(&key).cloned();
        let unchanged = entry.as_ref().is_some_and(|e| {
            e.size == size && e.modified_ms == modified && e.content_sha256.is_some()
        });

        let content = match &entry {
            Some(e) if unchanged => e.content_sha256.clone(),
            _ => Some(file_sha256(&self.path)?),
        };
        let text_hash = match (&entry, text) {
            (Some(e), _) if unchanged && e.text_sha256.is_some() => e.text_sha256.clone(),
            (_, Some(t)) => Some(text_sha256(t)),
            (Some(e), None) if unchanged => e.text_sha256.clone(),
            (_, None) => None,
        };
        let library = match textweaver_store::sync::folder_for(&self.library_folders, &self.path) {
            Some((folder, rel)) => match library_id(&folder) {
                Ok(lib) => Some(library_key(lib, &rel)),
                Err(e) => {
                    log::warn!("sync: no library folder id ({e})");
                    entry.as_ref().and_then(|e| e.library_key.clone())
                }
            },
            None => None,
        };
        let fingerprint = Fingerprint {
            content_sha256: content,
            text_sha256: text_hash,
            library_key: library,
            size,
            modified_ms: modified,
        };

        let (mut sync_id, found) = match entry.as_ref().and_then(|e| e.sync_id.parse().ok()) {
            Some(id) => (id, Found::Known),
            None => find(&ids, &key, &fingerprint, index),
        };
        // Two ids for one document, made before the computers ever synced:
        // the smallest wins.
        let mut folded_from = None;
        if let Some(smaller) = index
            .and_then(|ix| ix.smallest_match(&fingerprint))
            .filter(|s| *s < sync_id)
        {
            // Only an id this path already had here is folded; one just
            // found for it simply becomes the smallest.
            if found == Found::Known {
                folded_from = Some(sync_id);
            }
            sync_id = smaller;
        }
        let changed = entry.as_ref().is_none_or(|e| {
            e.sync_id != sync_id.to_string()
                || e.content_sha256 != fingerprint.content_sha256
                || e.text_sha256 != fingerprint.text_sha256
                || e.library_key != fingerprint.library_key
        });
        let declined = entry
            .as_ref()
            .map(|e| e.declined.clone())
            .unwrap_or_default();
        let suggestions = match index {
            Some(ix) if !ix.contains(sync_id) => ix.suggestions(&self.details, sync_id, &declined),
            _ => Vec::new(),
        };

        let differs = entry.as_ref().is_none_or(|e| {
            changed || e.size != fingerprint.size || e.modified_ms != fingerprint.modified_ms
        });
        if differs {
            let mut e = entry.unwrap_or_default();
            e.sync_id = sync_id.to_string();
            e.size = fingerprint.size;
            e.modified_ms = fingerprint.modified_ms;
            e.content_sha256.clone_from(&fingerprint.content_sha256);
            e.text_sha256.clone_from(&fingerprint.text_sha256);
            e.library_key.clone_from(&fingerprint.library_key);
            ids.set(&key, e);
            ids.save(&self.ids_file).map_err(from_store)?;
        }
        Ok(Resolved {
            key,
            sync_id,
            found,
            fingerprint,
            changed,
            suggestions,
            folded_from,
        })
    }

    /// [`run`](Self::run) with failures logged, for the writer.
    pub fn run_logged<'a, I>(&self, text: Option<I>)
    where
        I: IntoIterator<Item = &'a str>,
    {
        match self.run(text, None) {
            Ok(r) => log::debug!("sync id {} ({:?})", r.sync_id, r.found),
            Err(e) => log::warn!("sync: cannot identify a document ({e})"),
        }
    }
}

/// Steps 1 to 3: this computer's other paths first, then the other
/// computers' records; else a new id.
fn find(
    ids: &SyncIds,
    key: &DocKey,
    fp: &Fingerprint,
    index: Option<&IdentityIndex>,
) -> (SyncId, Found) {
    let steps = [
        (HashKind::Content, &fp.content_sha256, Found::Content),
        (HashKind::Text, &fp.text_sha256, Found::Text),
        (HashKind::Library, &fp.library_key, Found::Library),
    ];
    for (kind, hash, found) in steps {
        let Some(hash) = hash else { continue };
        if let Some(id) = ids
            .find(kind, hash, Some(key))
            .and_then(|s| s.parse::<SyncId>().ok())
        {
            return (id, found);
        }
        if let Some(id) = index.and_then(|ix| ix.find(kind, hash)) {
            return (id, found);
        }
    }
    (SyncId::random(), Found::New)
}

fn edit_entry(
    ids_file: &Path,
    key: &DocKey,
    f: impl FnOnce(&mut SyncIdEntry),
) -> Result<(), SyncError> {
    let mut ids = SyncIds::load(ids_file);
    let mut e = ids.get(key).cloned().unwrap_or_default();
    f(&mut e);
    ids.set(key, e);
    ids.save(ids_file).map_err(from_store)
}

/// The owner said yes to a [`Suggestion`]: the document at `key` takes
/// `sync_id`, and with it the other computer's notes and places.
pub fn accept(ids_file: &Path, key: &DocKey, sync_id: SyncId) -> Result<(), SyncError> {
    edit_entry(ids_file, key, |e| {
        e.sync_id = sync_id.to_string();
        e.declined.retain(|d| *d != e.sync_id);
    })
}

/// The owner said no to a [`Suggestion`]: it is not offered again for the
/// document at `key`.
pub fn decline(ids_file: &Path, key: &DocKey, sync_id: SyncId) -> Result<(), SyncError> {
    edit_entry(ids_file, key, |e| e.decline(&sync_id.to_string()))
}
