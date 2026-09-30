//! `sync-ids.json`: which sync id each document on this computer has
//! (ADR-0049, "Recognizing the same document").
//!
//! Local state keys documents by their full path ([`DocKey`]), so one
//! document has a different key on every computer. Sync names documents by
//! a random 128-bit sync id instead, the same everywhere once the document
//! is recognized. This file maps each path key to its sync id and keeps
//! what recognized it: the file's SHA-256, the SHA-256 of its text as
//! textweaver reads it, and the hash of its library folder's id and its
//! path inside that folder. It also keeps the file's size and modification
//! time, so an unchanged file is not hashed again.
//!
//! The file lives in the data folder, beside `library.json` and
//! `stats.json`, never in `state/` (whose pruning reads every JSON file
//! there as a document's state). It never goes to the sync folder, and no
//! existing state file is renamed: `state/<key>.json` stays where it is.
//!
//! Sync ids are kept here as the 32 hex digits `textweaver-sync` writes;
//! this crate does not depend on it and only stores them. Fields it does
//! not know are kept, so a newer textweaver's additions survive a save by
//! an older one.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{DocKey, StoreError};

/// The file's name, in the data folder ([`crate::Paths::sync_ids_file`]).
pub const SYNC_IDS_FILE: &str = "sync-ids.json";

/// The format this version writes.
pub const SYNC_IDS_FORMAT: u32 = 1;

/// The most sync ids one document remembers declining, so a question
/// ("This may be Cells from the laptop. Use them?") answered no is not
/// asked again.
pub const MAX_DECLINED: usize = 16;

/// One document's entry.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncIdEntry {
    /// The document's sync id: 32 lower-case hex digits.
    pub sync_id: String,
    /// The file's size in bytes when it was last hashed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// The file's modification time when it was last hashed, in
    /// milliseconds since 1970 (UTC).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_ms: Option<u64>,
    /// SHA-256 of the file's bytes, 64 lower-case hex digits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_sha256: Option<String>,
    /// SHA-256 of the document's text as textweaver reads it, with runs of
    /// white space made one space.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_sha256: Option<String>,
    /// SHA-256 of the library folder's id and the path inside the folder.
    /// The path itself is never stored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library_key: Option<String>,
    /// Sync ids the owner said no to for this document.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub declined: Vec<String>,
    /// When the entry last changed (Unix seconds, UTC).
    #[serde(default)]
    pub updated: i64,
    /// Fields a newer textweaver wrote, kept as they are.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl SyncIdEntry {
    /// Remembers that the owner declined `sync_id` for this document
    /// (newest last, at most [`MAX_DECLINED`]).
    pub fn decline(&mut self, sync_id: &str) {
        self.declined.retain(|d| d != sync_id);
        self.declined.push(sync_id.to_owned());
        if self.declined.len() > MAX_DECLINED {
            let cut = self.declined.len() - MAX_DECLINED;
            self.declined.drain(..cut);
        }
    }

    /// Whether the owner declined `sync_id` for this document.
    pub fn declined(&self, sync_id: &str) -> bool {
        self.declined.iter().any(|d| d == sync_id)
    }
}

/// Which hash an entry is looked up by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HashKind {
    /// [`SyncIdEntry::content_sha256`].
    Content,
    /// [`SyncIdEntry::text_sha256`].
    Text,
    /// [`SyncIdEntry::library_key`].
    Library,
}

/// `sync-ids.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncIds {
    /// The format it was written in ([`SYNC_IDS_FORMAT`]).
    #[serde(default = "format_one")]
    pub format: u32,
    /// Entries by path key ([`DocKey`]).
    #[serde(default)]
    pub docs: BTreeMap<String, SyncIdEntry>,
    /// Fields a newer textweaver wrote, kept as they are.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

fn format_one() -> u32 {
    SYNC_IDS_FORMAT
}

impl Default for SyncIds {
    fn default() -> Self {
        Self {
            format: SYNC_IDS_FORMAT,
            docs: BTreeMap::new(),
            extra: Map::new(),
        }
    }
}

impl SyncIds {
    /// Reads `file`. A missing file is empty. A damaged one is set aside
    /// (`sync-ids.corrupt-<time>.bak`) and read as empty: every document is
    /// then recognized again by its hashes, so nothing is lost.
    pub fn load(file: &Path) -> Self {
        let text = match std::fs::read_to_string(file) {
            Ok(t) => t,
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    log::warn!("{}: {e}", file.display());
                }
                return Self::default();
            }
        };
        match serde_json::from_str::<SyncIds>(&text) {
            Ok(ids) => ids,
            Err(e) => {
                crate::atomic::set_aside(file, &e);
                Self::default()
            }
        }
    }

    /// Writes `file` atomically. A newer format number read from the file
    /// is kept.
    pub fn save(&self, file: &Path) -> Result<(), StoreError> {
        let mut out = self.clone();
        out.format = out.format.max(SYNC_IDS_FORMAT);
        let json = serde_json::to_vec_pretty(&out).map_err(|e| StoreError::Parse {
            path: file.to_owned(),
            message: e.to_string(),
        })?;
        crate::atomic_write(file, &json)
    }

    /// The entry of `key`.
    pub fn get(&self, key: &DocKey) -> Option<&SyncIdEntry> {
        self.docs.get(&key.0)
    }

    /// The entry of `key`, for changing.
    pub fn get_mut(&mut self, key: &DocKey) -> Option<&mut SyncIdEntry> {
        self.docs.get_mut(&key.0)
    }

    /// Sets the entry of `key`, stamping it with the time now.
    pub fn set(&mut self, key: &DocKey, mut entry: SyncIdEntry) {
        entry.updated = crate::now_ts();
        self.docs.insert(key.0.clone(), entry);
    }

    /// The sync id of the most recently updated entry, other than `except`,
    /// whose hash of `kind` is `hash`: the same document at another path
    /// (renamed, moved, or copied). Ties go to the smallest sync id, so the
    /// answer does not depend on the file's order.
    pub fn find(&self, kind: HashKind, hash: &str, except: Option<&DocKey>) -> Option<&str> {
        self.docs
            .iter()
            .filter(|(k, _)| except.is_none_or(|e| e.0 != **k))
            .filter(|(_, e)| {
                let h = match kind {
                    HashKind::Content => &e.content_sha256,
                    HashKind::Text => &e.text_sha256,
                    HashKind::Library => &e.library_key,
                };
                h.as_deref() == Some(hash)
            })
            .map(|(_, e)| e)
            .max_by(|a, b| {
                a.updated
                    .cmp(&b.updated)
                    .then_with(|| b.sync_id.cmp(&a.sync_id))
            })
            .map(|e| e.sync_id.as_str())
    }

    /// The path keys that map to `sync_id`.
    pub fn keys_of<'a>(&'a self, sync_id: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.docs
            .iter()
            .filter(move |(_, e)| e.sync_id == sync_id)
            .map(|(k, _)| k.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, content: &str, updated: i64) -> SyncIdEntry {
        SyncIdEntry {
            sync_id: id.to_owned(),
            content_sha256: Some(content.to_owned()),
            updated,
            ..SyncIdEntry::default()
        }
    }

    #[test]
    fn a_missing_file_is_empty_and_a_saved_one_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(SYNC_IDS_FILE);
        let mut ids = SyncIds::load(&file);
        assert!(ids.docs.is_empty());
        ids.set(&DocKey("a.md-1".into()), entry("11", "c1", 0));
        ids.save(&file).unwrap();
        let back = SyncIds::load(&file);
        assert_eq!(back, ids);
        assert!(back.get(&DocKey("a.md-1".into())).unwrap().updated > 0);
    }

    #[test]
    fn unknown_fields_are_kept() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(SYNC_IDS_FILE);
        std::fs::write(
            &file,
            r#"{"format":3,"later":true,"docs":{"k":{"sync_id":"ab","newer":[1,2]}}}"#,
        )
        .unwrap();
        let ids = SyncIds::load(&file);
        ids.save(&file).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(v["format"], 3);
        assert_eq!(v["later"], true);
        assert_eq!(v["docs"]["k"]["newer"], serde_json::json!([1, 2]));
    }

    #[test]
    fn a_damaged_file_is_set_aside_and_read_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(SYNC_IDS_FILE);
        std::fs::write(&file, "{\"docs\": {").unwrap();
        assert!(SyncIds::load(&file).docs.is_empty());
        let kept = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains(".corrupt-"))
            .count();
        assert_eq!(kept, 1);
    }

    #[test]
    fn find_picks_the_newest_other_entry() {
        let mut ids = SyncIds::default();
        ids.docs.insert("old".into(), entry("aa", "c", 10));
        ids.docs.insert("new".into(), entry("bb", "c", 20));
        ids.docs.insert("other".into(), entry("cc", "d", 30));
        assert_eq!(ids.find(HashKind::Content, "c", None), Some("bb"));
        assert_eq!(
            ids.find(HashKind::Content, "c", Some(&DocKey("new".into()))),
            Some("aa")
        );
        assert_eq!(ids.find(HashKind::Text, "c", None), None);
        assert_eq!(ids.keys_of("bb").collect::<Vec<_>>(), ["new"]);
    }

    #[test]
    fn declines_are_capped_and_not_repeated() {
        let mut e = SyncIdEntry::default();
        for n in 0..20 {
            e.decline(&format!("{n}"));
        }
        e.decline("19");
        assert_eq!(e.declined.len(), MAX_DECLINED);
        assert!(e.declined("19") && e.declined("4") && !e.declined("3"));
    }
}
