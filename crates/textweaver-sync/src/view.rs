//! A read-only view of the sync folder: every computer's records read and
//! merged, and every computer's name, without writing anything and without
//! this computer's identity. The library (its details, for searching),
//! "Continue reading", and the statistics read the folder this way, on a
//! helper thread or from the command line, while the reader's own sync
//! engine keeps the folder open on the background writer.
//!
//! Damaged, cut-short, and newer files are skipped and counted, as the
//! engine skips them; nothing here reports them aloud (the engine does).

use std::collections::BTreeMap;
use std::path::Path;

use textweaver_store::sync_ids::HashKind;

use crate::folder::{DEVICES_DIR, DOCS_DIR, SYNC_DIR, SyncFolder, not_found, read_limited};
use crate::record::MAX_RECORD_BYTES;
use crate::{DeviceId, DocRecord, SyncError, SyncId};

/// Every computer's records, merged by document, and every computer's name.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FolderView {
    /// Each computer's name, from its `device.json`.
    pub labels: BTreeMap<DeviceId, String>,
    /// Each document's records from every computer, merged. A record
    /// folded into a smaller id is merged into that id's
    /// ([`DocRecord::folded_into`]).
    pub docs: BTreeMap<SyncId, DocRecord>,
    /// Files skipped because they were damaged, cut short, or from a newer
    /// textweaver.
    pub skipped: usize,
    /// Each folded id and the id it was folded into, for
    /// [`doc`](Self::doc) to follow.
    pub folded: BTreeMap<SyncId, SyncId>,
}

impl FolderView {
    /// Reads `folder` (the folder the owner chose). A folder that is not
    /// there is [`SyncError::FolderMissing`]; a folder with no
    /// `textweaver-sync` in it yet is an empty view. Writes nothing.
    pub fn read(folder: &Path) -> Result<Self, SyncError> {
        if !folder.is_dir() {
            return Err(SyncError::FolderMissing);
        }
        let root = folder.join(SYNC_DIR);
        let mut view = FolderView::default();
        let devices = std::fs::read_dir(root.join(DEVICES_DIR));
        let mut ids: Vec<DeviceId> = devices
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .filter_map(|e| e.file_name().to_str().and_then(|n| n.parse().ok()))
            .collect();
        ids.sort();
        for device in ids {
            let dir = root.join(DEVICES_DIR).join(device.to_string());
            match SyncFolder::read_device_at(&root, device) {
                Ok(info) => {
                    view.labels.insert(device, info.label);
                }
                Err(e) if not_found(&e) => {}
                Err(_) => view.skipped += 1,
            }
            view.read_docs(&dir.join(DOCS_DIR));
        }
        view.fold();
        Ok(view)
    }

    /// Merges each record folded into a smaller id into that id's record,
    /// largest id first, so a chain of folds ends in the smallest.
    fn fold(&mut self) {
        let folded: Vec<SyncId> = self
            .docs
            .iter()
            .rev()
            .filter(|(_, r)| r.folded_into.is_some())
            .map(|(id, _)| *id)
            .collect();
        for id in folded {
            let Some(mut record) = self.docs.remove(&id) else {
                continue;
            };
            let Some(target) = record.folded_into.take() else {
                continue;
            };
            self.folded.insert(id, target);
            record.sync_id = target;
            match self.docs.get_mut(&target) {
                Some(merged) => {
                    let _ = merged.merge(&record);
                }
                None => {
                    self.docs.insert(target, record);
                }
            }
        }
    }

    /// Reads and merges one computer's records.
    fn read_docs(&mut self, dir: &Path) {
        let mut files: Vec<(SyncId, std::path::PathBuf)> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let name = e.file_name();
                let id = name.to_str()?.strip_suffix(".json")?.parse().ok()?;
                Some((id, e.path()))
            })
            .collect();
        files.sort();
        for (id, path) in files {
            let record = read_limited(&path, MAX_RECORD_BYTES)
                .and_then(|b| DocRecord::from_bytes(&b))
                .ok()
                .filter(|r| r.sync_id == id);
            let Some(record) = record else {
                self.skipped += 1;
                continue;
            };
            match self.docs.get_mut(&id) {
                Some(merged) => {
                    let _ = merged.merge(&record);
                }
                None => {
                    self.docs.insert(id, record);
                }
            }
        }
    }

    /// A computer's name, when its `device.json` could be read.
    pub fn label(&self, device: DeviceId) -> Option<&str> {
        self.labels.get(&device).map(String::as_str)
    }

    /// The merged record of `sync_id`, or of the id it was folded into.
    pub fn doc(&self, sync_id: SyncId) -> Option<&DocRecord> {
        let mut id = sync_id;
        // Each fold goes to a smaller id, so this ends.
        while let Some(next) = self.folded.get(&id) {
            id = *next;
        }
        self.docs.get(&id)
    }

    /// The document whose hashes of `kind` include `hash`; when several do,
    /// the one that published it last (ties: the smallest sync id), as the
    /// engine's recognition picks.
    pub fn find(&self, kind: HashKind, hash: &str) -> Option<SyncId> {
        self.docs
            .iter()
            .filter_map(|(id, r)| {
                let set = match kind {
                    HashKind::Content => &r.identity.content,
                    HashKind::Text => &r.identity.text,
                    HashKind::Library => &r.identity.library,
                };
                set.0.get(hash).map(|stamp| (*stamp, *id))
            })
            .max_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)))
            .map(|(_, id)| id)
    }

    /// Whether any record holds a text hash (so hashing a text to look it
    /// up can find anything).
    pub fn has_text_hashes(&self) -> bool {
        self.docs.values().any(|r| !r.identity.text.is_empty())
    }
}
