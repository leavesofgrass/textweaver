//! What arrived and was not applied yet, kept on disk (the crash window,
//! W7s).
//!
//! A merge writes this computer's merged view to the sync folder, and sends
//! what arrived from other computers to the app, which applies it and saves
//! the document's state a moment later. Until it does, the engine remembers
//! each arrival and the version the app had (its *pending* list), so a
//! difference between the app and the merged view is not taken for an
//! edit made here.
//!
//! If textweaver stopped between the two (a crash, a power cut), that list
//! was lost: on the next start the app's older version looked like a new
//! edit, was stamped with a newer clock, and won over the arrival. So the
//! pending lists are saved here, in `sync-pending.json` in the data folder,
//! **before** the merged view is published, and read back when a document
//! or group is first merged in a session. The file never syncs.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use textweaver_store::{MarkKind, Paths, atomic_write};
use textweaver_sync::{DeviceId, GroupFile, SyncId};

/// The file's name, in the data folder.
pub const PENDING_FILE: &str = "sync-pending.json";

/// The format this version writes.
const FORMAT: u32 = 1;

/// The largest file read; a larger one is ignored (and replaced on the
/// next save).
const MAX_BYTES: u64 = 16 * 1024 * 1024;

/// One arrival not applied yet: the item and what the app had.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingItem {
    /// For a document: `bookmark`, `note`, or `highlight`. For a group:
    /// the map or set's name.
    pub kind: String,
    /// The item's id, or the group's key.
    pub id: String,
    /// Whether the app had a version.
    pub had: bool,
    /// The version the app had (its content), when it had one.
    #[serde(default)]
    pub before: Value,
}

impl PendingItem {
    /// What the app had, as the engine keeps it.
    pub fn before(&self) -> Option<Value> {
        self.had.then(|| self.before.clone())
    }

    /// An item from what the app had.
    pub fn new(kind: &str, id: &str, before: Option<&Value>) -> Self {
        Self {
            kind: kind.to_owned(),
            id: id.to_owned(),
            had: before.is_some(),
            before: before.cloned().unwrap_or(Value::Null),
        }
    }
}

/// The saved pending lists.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct File {
    format: u32,
    /// The computer the lists belong to; lists from another id (a copied
    /// state folder, sync set up again) are dropped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    device: Option<DeviceId>,
    /// By document sync id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    docs: BTreeMap<String, Vec<PendingItem>>,
    /// By group file name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    groups: BTreeMap<String, Vec<PendingItem>>,
}

/// The pending lists of one computer, read once and saved on change.
#[derive(Clone, Debug, Default)]
pub struct PendingJournal {
    path: PathBuf,
    file: File,
}

/// A mark kind's name in the file.
pub fn kind_name(kind: MarkKind) -> &'static str {
    match kind {
        MarkKind::Bookmark => "bookmark",
        MarkKind::Note => "note",
        MarkKind::Highlight => "highlight",
    }
}

/// A mark kind from its name in the file.
pub fn kind_from_name(name: &str) -> Option<MarkKind> {
    match name {
        "bookmark" => Some(MarkKind::Bookmark),
        "note" => Some(MarkKind::Note),
        "highlight" => Some(MarkKind::Highlight),
        _ => None,
    }
}

fn read(path: &Path) -> Option<File> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > MAX_BYTES {
        log::warn!("sync: the pending arrivals file is too large; ignored");
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    match serde_json::from_slice::<File>(&bytes) {
        Ok(f) if f.format <= FORMAT => Some(f),
        Ok(_) => None,
        Err(e) => {
            log::warn!("sync: the pending arrivals file is damaged ({e}); ignored");
            None
        }
    }
}

impl PendingJournal {
    /// Reads `sync-pending.json` for `device`. A missing, damaged, or
    /// other computer's file gives empty lists.
    pub fn load(paths: &Paths, device: DeviceId) -> Self {
        let path = paths.data_dir.join(PENDING_FILE);
        let file = read(&path)
            .filter(|f| f.device == Some(device))
            .unwrap_or_else(|| File {
                format: FORMAT,
                device: Some(device),
                ..File::default()
            });
        Self { path, file }
    }

    /// A document's saved list.
    pub fn doc(&self, sync_id: SyncId) -> Vec<PendingItem> {
        self.file
            .docs
            .get(&sync_id.to_string())
            .cloned()
            .unwrap_or_default()
    }

    /// A group's saved list.
    pub fn group(&self, file: GroupFile) -> Vec<PendingItem> {
        self.file
            .groups
            .get(file.file_name())
            .cloned()
            .unwrap_or_default()
    }

    /// Sets a document's list and saves the file when it changed. Returns
    /// false when saving failed: the caller then holds back publishing.
    pub fn set_doc(&mut self, sync_id: SyncId, items: Vec<PendingItem>) -> bool {
        let key = sync_id.to_string();
        self.set(|f| &mut f.docs, key, items)
    }

    /// Sets a group's list and saves the file when it changed. Returns
    /// false when saving failed.
    pub fn set_group(&mut self, file: GroupFile, items: Vec<PendingItem>) -> bool {
        let key = file.file_name().to_owned();
        self.set(|f| &mut f.groups, key, items)
    }

    fn set(
        &mut self,
        map: impl Fn(&mut File) -> &mut BTreeMap<String, Vec<PendingItem>>,
        key: String,
        items: Vec<PendingItem>,
    ) -> bool {
        let m = map(&mut self.file);
        let changed = if items.is_empty() {
            m.remove(&key).is_some()
        } else if m.get(&key) == Some(&items) {
            false
        } else {
            m.insert(key, items);
            true
        };
        !changed || self.save()
    }

    fn save(&self) -> bool {
        let saved = serde_json::to_vec_pretty(&self.file)
            .map_err(|e| e.to_string())
            .and_then(|mut bytes| {
                bytes.push(b'\n');
                atomic_write(&self.path, &bytes).map_err(|e| e.to_string())
            });
        match saved {
            Ok(()) => true,
            Err(e) => {
                log::warn!("sync: cannot save the pending arrivals ({e})");
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: DeviceId = DeviceId::from_u128(0xa);
    const B: DeviceId = DeviceId::from_u128(0xb);
    const DOC: SyncId = SyncId::from_u128(0x1);

    #[test]
    fn lists_survive_a_restart_and_belong_to_one_computer() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::under(dir.path());
        std::fs::create_dir_all(&paths.data_dir).unwrap();
        let mut j = PendingJournal::load(&paths, A);
        let item = PendingItem::new("note", "n1", Some(&Value::from("old")));
        assert!(j.set_doc(DOC, vec![item.clone()]));
        let gone = PendingItem::new("words", "ribosome", None);
        assert!(j.set_group(GroupFile::Words, vec![gone.clone()]));

        let again = PendingJournal::load(&paths, A);
        assert_eq!(again.doc(DOC), vec![item]);
        assert_eq!(again.group(GroupFile::Words), vec![gone]);
        assert_eq!(again.doc(DOC)[0].before(), Some(Value::from("old")));
        assert_eq!(again.group(GroupFile::Words)[0].before(), None);

        // Another computer id (a copied state folder) starts empty.
        assert!(PendingJournal::load(&paths, B).doc(DOC).is_empty());

        // An emptied list is removed from the file.
        let mut j = PendingJournal::load(&paths, A);
        assert!(j.set_doc(DOC, Vec::new()));
        assert!(PendingJournal::load(&paths, A).doc(DOC).is_empty());
    }

    #[test]
    fn a_damaged_file_gives_empty_lists() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::under(dir.path());
        std::fs::create_dir_all(&paths.data_dir).unwrap();
        std::fs::write(
            paths.data_dir.join(PENDING_FILE),
            b"{\"format\": 1, \"docs\": ",
        )
        .unwrap();
        assert!(PendingJournal::load(&paths, A).doc(DOC).is_empty());
    }
}
