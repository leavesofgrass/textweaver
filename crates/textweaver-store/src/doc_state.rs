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

/// A named position.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bookmark {
    /// User-visible name.
    pub name: String,
    /// Position.
    pub pos: CharPos,
    /// Percentage through the document, floored.
    pub pct: u8,
    /// When it was set (Unix seconds, UTC).
    pub ts: i64,
}

/// Percentage of `pos` through a document of `len` chars, floored, as Star
/// computed it: `int(100 * offset / max(1, len))`, capped at 100.
pub fn percent(pos: CharPos, len: usize) -> u8 {
    let pct = pos.0.saturating_mul(100) / len.max(1);
    u8::try_from(pct.min(100)).unwrap_or(100)
}

/// Everything remembered about one document.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DocState {
    /// Char offset of the word being read when the position was saved.
    pub position: CharPos,
    /// Percentage through the document, floored.
    pub pct: u8,
    /// When the position was saved (Unix seconds, UTC).
    pub ts: i64,
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
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
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
    /// `markN` (Star's rule). A bookmark with the same name is replaced.
    /// Bookmarks stay sorted by position. Returns the bookmark added.
    pub fn add_bookmark(&mut self, name: Option<&str>, pos: CharPos, doc_len: usize) -> Bookmark {
        let name = match name.map(str::trim) {
            Some(n) if !n.is_empty() => n.to_owned(),
            _ => self.next_bookmark_name(),
        };
        self.bookmarks.retain(|b| b.name != name);
        let pos = pos.clamp_to(doc_len);
        let mark = Bookmark {
            name,
            pos,
            pct: percent(pos, doc_len),
            ts: crate::now_ts(),
        };
        let at = self.bookmarks.partition_point(|b| b.pos <= pos);
        self.bookmarks.insert(at, mark.clone());
        mark
    }

    /// The bookmark called `name`.
    pub fn bookmark(&self, name: &str) -> Option<&Bookmark> {
        self.bookmarks.iter().find(|b| b.name == name)
    }

    /// Removes the bookmark called `name`. Returns whether it existed.
    pub fn remove_bookmark(&mut self, name: &str) -> bool {
        let before = self.bookmarks.len();
        self.bookmarks.retain(|b| b.name != name);
        self.bookmarks.len() != before
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

    /// Removes the note with `id`. Returns whether it existed.
    pub fn remove_note(&mut self, id: &str) -> bool {
        let before = self.notes.len();
        self.notes.retain(|n| n.id != id);
        self.notes.len() != before
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
    /// is replaced, so highlighting again changes the color. An empty range
    /// highlights nothing and returns `None`.
    pub fn add_highlight(
        &mut self,
        range: CharRange,
        color: &str,
        text: &str,
    ) -> Option<Highlight> {
        if range.is_empty() {
            return None;
        }
        let h = Highlight {
            id: self.fresh_id(),
            range,
            color: notes::highlight_color(color),
            text: notes::collapse(text, notes::HIGHLIGHT_TEXT_MAX_CHARS),
            ts: crate::now_ts(),
            extra: serde_json::Map::new(),
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

    /// Removes the highlight with `id`. Returns it if it existed.
    pub fn remove_highlight(&mut self, id: &str) -> Option<Highlight> {
        let i = self.highlights.iter().position(|h| h.id == id)?;
        Some(self.highlights.remove(i))
    }

    /// Removes every highlight (Star's Clear All Highlights). Returns how
    /// many there were.
    pub fn clear_highlights(&mut self) -> usize {
        std::mem::take(&mut self.highlights).len()
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
    pub fn shift(&mut self, outcome: &EditOutcome) {
        notes::shift(&mut self.notes, &mut self.highlights, outcome);

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
    pub fn load(&self, key: &DocKey) -> Option<DocState> {
        if let Some((state, _)) = self.inner.lock().pending.get(key) {
            return Some(state.clone());
        }
        let text = std::fs::read_to_string(self.inner.file(key)).ok()?;
        serde_json::from_str(&text).ok()
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
            .map(|p| {
                let ts = std::fs::read_to_string(&p)
                    .ok()
                    .and_then(|t| serde_json::from_str::<DocState>(&t).ok())
                    .map_or(0, |s| s.ts);
                (ts, p)
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
        assert!(a.created > 0 && a.id.len() == 8 && a.id != b.id);
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
}
