//! Per-document state: position, history, bookmarks (notes and highlights in
//! wave 2), one JSON file per document.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use textweaver_core::CharPos;

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
    /// The key for a file path (made absolute; not canonicalized, so the
    /// same file reached through different links gets different keys).
    pub fn for_path(path: &Path) -> Self {
        let abs = std::path::absolute(path).unwrap_or_else(|_| path.to_owned());
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
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Reads and writes `state/<doc-key>.json`.
#[derive(Clone, Debug)]
pub struct StateStore {
    dir: PathBuf,
}

impl StateStore {
    /// A store writing under `dir`.
    pub fn new(dir: PathBuf) -> Self {
        StateStore { dir }
    }

    fn file(&self, key: &DocKey) -> PathBuf {
        self.dir.join(format!("{}.json", key.0))
    }

    /// The saved state, if any. Unreadable files count as no state.
    pub fn load(&self, key: &DocKey) -> Option<DocState> {
        let text = std::fs::read_to_string(self.file(key)).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Saves state atomically. Callers debounce (Agent C adds the debouncer).
    pub fn save(&self, key: &DocKey, state: &DocState) -> Result<(), StoreError> {
        let path = self.file(key);
        let text = serde_json::to_string_pretty(state).map_err(|e| StoreError::Parse {
            path: path.clone(),
            message: e.to_string(),
        })?;
        atomic_write(&path, text.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_stable_and_readable() {
        let a = DocKey::for_path(Path::new("/tmp/some doc.md"));
        let b = DocKey::for_path(Path::new("/tmp/some doc.md"));
        assert_eq!(a, b);
        assert!(a.0.starts_with("some_doc.md-"));
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
}
