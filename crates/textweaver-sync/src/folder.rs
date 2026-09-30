//! The sync folder: a folder the user chooses, carried between computers by
//! Syncthing, a cloud folder, or a USB stick. textweaver does no
//! networking; it only reads and writes files here.
//!
//! ```text
//! <folder>/textweaver-sync/
//!   format.json                     the format number
//!   devices/<device-id>/
//!     device.json                   label, app version, install token
//!     docs/<sync-id>.json           this computer's merged view of a document
//! ```
//!
//! The rules (ADR-0049):
//!
//! - Each computer writes only inside its own `devices/<device-id>/`,
//!   always by atomic replace (a temporary file whose name starts with `.`
//!   and ends in `.tmp`, which readers skip, renamed over the old one),
//!   never by appending. The one shared file,
//!   `format.json`, is written only when it is missing, and never changed.
//! - textweaver never deletes or changes another computer's files.
//! - Each computer writes its full merged view of a document, so reading a
//!   record twice, or in any order, changes nothing.
//! - A damaged or partial file (a sync service still copying it, say) is
//!   skipped and reported; the last good copy read in this session is used
//!   in its place.
//! - A `format.json` newer than this textweaver makes sync read-only: it
//!   still reads what it can, but writes nothing.
//! - Only random ids, a label the user chose, and the app's version name
//!   files and fill `device.json`: no computer, user, or account names and
//!   no paths.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::record::{FORMAT, MAX_RECORD_BYTES};
use crate::{
    Clock, ClockAhead, DeviceId, DocRecord, Identity, InstallToken, MergeReport, SyncError, SyncId,
    check_label,
};

/// The folder textweaver keeps inside the chosen folder.
pub const SYNC_DIR: &str = "textweaver-sync";
/// The format file's name.
pub const FORMAT_FILE: &str = "format.json";
/// The folder of computers.
pub const DEVICES_DIR: &str = "devices";
/// A computer's own description.
pub const DEVICE_FILE: &str = "device.json";
/// A computer's folder of document records.
pub const DOCS_DIR: &str = "docs";

/// The largest `device.json` or `format.json` read, in bytes.
const MAX_SMALL_FILE_BYTES: u64 = 64 * 1024;

/// `format.json`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatFile {
    /// The newest format any computer in the folder writes.
    pub format: u32,
}

/// `devices/<device-id>/device.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// The format it was written in.
    pub format: u32,
    /// The computer.
    pub device: DeviceId,
    /// The label the user chose, such as "laptop" or "lab".
    pub label: String,
    /// The textweaver version that wrote it.
    pub app_version: String,
    /// The installation's token, which catches a copied state folder.
    pub install_token: InstallToken,
}

/// Why sync is read-only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadOnly {
    /// `format.json` is from a newer textweaver.
    NewerFormat {
        /// Its format number.
        found: u32,
    },
    /// `format.json` could not be read, so the folder's format is unknown.
    UnreadableFormat,
}

impl std::fmt::Display for ReadOnly {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadOnly::NewerFormat { found } => {
                write!(f, "Read only: sync folder format {found} is newer")
            }
            ReadOnly::UnreadableFormat => write!(f, "Read only: sync format file damaged"),
        }
    }
}

/// Which file a [`Problem`] is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    /// `format.json`.
    Format,
    /// A computer's `device.json`.
    Device,
    /// A computer's record of a document.
    Doc(SyncId),
}

/// Something found while reading the folder, for the app to report. None
/// of these stops sync.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// A file could not be read, or was cut short, and was skipped.
    Damaged {
        /// The computer whose file it is (none for `format.json`).
        device: Option<DeviceId>,
        /// Which file.
        file: FileKind,
        /// Why, for the log.
        reason: String,
        /// The last good copy read in this session was used instead.
        kept_last_good: bool,
    },
    /// A file from a newer textweaver was skipped.
    NewerFormat {
        /// The computer whose file it is.
        device: DeviceId,
        /// Which file.
        file: FileKind,
        /// Its format number.
        found: u32,
    },
    /// A computer's clock is more than a day ahead.
    ClockAhead(ClockAhead),
    /// Sync is read-only.
    ReadOnly(ReadOnly),
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Problem::Damaged {
                device,
                file,
                kept_last_good,
                ..
            } => {
                let what = match file {
                    FileKind::Format => "format file".to_owned(),
                    FileKind::Device => "computer file".to_owned(),
                    FileKind::Doc(id) => format!("document {id}"),
                };
                write!(f, "Damaged file skipped: {what}")?;
                if let Some(d) = device {
                    write!(f, " from computer {d}")?;
                }
                if *kept_last_good {
                    write!(f, "; last good copy used")?;
                }
                Ok(())
            }
            Problem::NewerFormat { device, found, .. } => {
                write!(
                    f,
                    "Newer file skipped: format {found} from computer {device}"
                )
            }
            Problem::ClockAhead(c) => c.fmt(f),
            Problem::ReadOnly(r) => r.fmt(f),
        }
    }
}

/// What opening the folder found.
#[derive(Debug)]
pub struct Opened {
    /// The open folder.
    pub folder: SyncFolder,
    /// This computer took a fresh id, because another installation's
    /// `device.json` already held its old one (a copied state folder).
    pub fresh_device_id: bool,
    /// Problems found.
    pub problems: Vec<Problem>,
}

/// Every computer's record of one document, as read.
#[derive(Debug, Default)]
pub struct DocRead {
    /// Each computer's record that could be read, or its last good copy.
    pub records: Vec<(DeviceId, DocRecord)>,
    /// Problems found.
    pub problems: Vec<Problem>,
}

/// What [`SyncFolder::merge_doc`] did.
#[derive(Debug, Default)]
pub struct Merged {
    /// What changed in the local record.
    pub report: MergeReport,
    /// Problems found.
    pub problems: Vec<Problem>,
}

/// An open sync folder, for one computer.
#[derive(Debug)]
pub struct SyncFolder {
    root: PathBuf,
    device: DeviceId,
    read_only: Option<ReadOnly>,
    last_good: HashMap<(DeviceId, SyncId), DocRecord>,
    /// Computers seen this session with stamps more than a day ahead, and
    /// by how much at most.
    ahead: BTreeMap<DeviceId, u64>,
}

/// Reads a file of at most `max` bytes.
fn read_limited(path: &Path, max: u64) -> Result<Vec<u8>, SyncError> {
    let io = |source| SyncError::Io {
        path: path.to_owned(),
        source,
    };
    let len = std::fs::metadata(path).map_err(io)?.len();
    if len > max {
        return Err(SyncError::TooLarge);
    }
    std::fs::read(path).map_err(io)
}

fn not_found(e: &SyncError) -> bool {
    matches!(e, SyncError::Io { source, .. } if source.kind() == std::io::ErrorKind::NotFound)
}

impl SyncFolder {
    /// Opens `folder` (the folder the user chose) for this computer.
    ///
    /// Makes `textweaver-sync/` and `format.json` when they are missing,
    /// goes read-only when the format is newer or unreadable, takes a fresh
    /// id when another installation's `device.json` holds this computer's
    /// id with another install token (saving it through `identity`), and
    /// writes this computer's `device.json` with `label` (checked by
    /// [`check_label`]) and `app_version`.
    pub fn open(
        folder: &Path,
        identity: &mut Identity,
        label: &str,
        app_version: &str,
    ) -> Result<Opened, SyncError> {
        if !folder.is_dir() {
            return Err(SyncError::FolderMissing);
        }
        let label = check_label(label)?;
        let root = folder.join(SYNC_DIR);
        let io = |source| SyncError::Io {
            path: root.clone(),
            source,
        };
        std::fs::create_dir_all(root.join(DEVICES_DIR)).map_err(io)?;
        let mut problems = Vec::new();

        let format_path = root.join(FORMAT_FILE);
        let read_only = match read_limited(&format_path, MAX_SMALL_FILE_BYTES) {
            Err(e) if not_found(&e) => {
                let body = serde_json::to_vec_pretty(&FormatFile { format: FORMAT })
                    .map_err(|e| SyncError::Damaged(e.to_string()))?;
                textweaver_store::atomic_write(&format_path, &body)?;
                None
            }
            Err(e) => {
                problems.push(Problem::Damaged {
                    device: None,
                    file: FileKind::Format,
                    reason: e.to_string(),
                    kept_last_good: false,
                });
                Some(ReadOnly::UnreadableFormat)
            }
            Ok(bytes) => match serde_json::from_slice::<FormatFile>(&bytes) {
                Ok(f) if f.format > FORMAT => Some(ReadOnly::NewerFormat { found: f.format }),
                Ok(_) => None,
                Err(e) => {
                    problems.push(Problem::Damaged {
                        device: None,
                        file: FileKind::Format,
                        reason: e.to_string(),
                        kept_last_good: false,
                    });
                    Some(ReadOnly::UnreadableFormat)
                }
            },
        };
        if let Some(r) = read_only {
            problems.push(Problem::ReadOnly(r));
        }

        // Another installation already writes under this id: this one is
        // the copy, and takes a fresh id before writing anything.
        let mut fresh_device_id = false;
        let theirs = Self::read_device_at(&root, identity.device());
        if let Ok(info) = &theirs
            && info.install_token != identity.token()
        {
            identity.renew()?;
            fresh_device_id = true;
        }

        let folder = SyncFolder {
            root,
            device: identity.device(),
            read_only,
            last_good: HashMap::new(),
            ahead: BTreeMap::new(),
        };
        if folder.read_only.is_none() {
            let info = DeviceInfo {
                format: FORMAT,
                device: identity.device(),
                label,
                app_version: app_version.to_owned(),
                install_token: identity.token(),
            };
            let unchanged = !fresh_device_id && theirs.as_ref().ok() == Some(&info);
            if !unchanged {
                let body = serde_json::to_vec_pretty(&info)
                    .map_err(|e| SyncError::Damaged(e.to_string()))?;
                let path = folder.device_dir(folder.device).join(DEVICE_FILE);
                textweaver_store::atomic_write(&path, &body)?;
            }
        }
        Ok(Opened {
            folder,
            fresh_device_id,
            problems,
        })
    }

    /// This computer's id.
    pub fn device(&self) -> DeviceId {
        self.device
    }

    /// Why sync is read-only, if it is.
    pub fn read_only(&self) -> Option<ReadOnly> {
        self.read_only
    }

    /// The computers whose stamps, seen in this session's merges, were more
    /// than a day ahead of this computer's clock, with the most each was
    /// ahead: for the sync status to name them, so the owner can fix their
    /// clocks (ADR-0049).
    pub fn clocks_ahead(&self) -> Vec<ClockAhead> {
        self.ahead
            .iter()
            .map(|(device, ahead_ms)| ClockAhead {
                device: *device,
                ahead_ms: *ahead_ms,
            })
            .collect()
    }

    fn device_dir(&self, device: DeviceId) -> PathBuf {
        self.root.join(DEVICES_DIR).join(device.to_string())
    }

    fn doc_path(&self, device: DeviceId, sync_id: SyncId) -> PathBuf {
        self.device_dir(device)
            .join(DOCS_DIR)
            .join(format!("{sync_id}.json"))
    }

    fn read_device_at(root: &Path, device: DeviceId) -> Result<DeviceInfo, SyncError> {
        let path = root
            .join(DEVICES_DIR)
            .join(device.to_string())
            .join(DEVICE_FILE);
        let bytes = read_limited(&path, MAX_SMALL_FILE_BYTES)?;
        let info: DeviceInfo =
            serde_json::from_slice(&bytes).map_err(|e| SyncError::Damaged(e.to_string()))?;
        if info.device != device {
            return Err(SyncError::Damaged(
                "device.json names another computer".into(),
            ));
        }
        Ok(info)
    }

    /// The computers with a folder here, in id order. Folders whose names
    /// are not computer ids are ignored.
    pub fn device_ids(&self) -> Vec<DeviceId> {
        let mut ids: Vec<DeviceId> = std::fs::read_dir(self.root.join(DEVICES_DIR))
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .filter_map(|e| e.file_name().to_str().and_then(|n| n.parse().ok()))
            .collect();
        ids.sort();
        ids
    }

    /// Every computer's `device.json` that can be read, and a problem for
    /// each that cannot.
    pub fn devices(&self) -> (Vec<DeviceInfo>, Vec<Problem>) {
        let mut infos = Vec::new();
        let mut problems = Vec::new();
        for d in self.device_ids() {
            match Self::read_device_at(&self.root, d) {
                Ok(info) if info.format > FORMAT => problems.push(Problem::NewerFormat {
                    device: d,
                    file: FileKind::Device,
                    found: info.format,
                }),
                Ok(info) => infos.push(info),
                Err(e) if not_found(&e) => {}
                Err(e) => problems.push(Problem::Damaged {
                    device: Some(d),
                    file: FileKind::Device,
                    reason: e.to_string(),
                    kept_last_good: false,
                }),
            }
        }
        (infos, problems)
    }

    /// Every document any computer has a record of.
    pub fn doc_ids(&self) -> BTreeSet<SyncId> {
        let mut ids = BTreeSet::new();
        for d in self.device_ids() {
            let docs = std::fs::read_dir(self.device_dir(d).join(DOCS_DIR));
            for e in docs.into_iter().flatten().flatten() {
                let name = e.file_name();
                if let Some(id) = name
                    .to_str()
                    .and_then(|n| n.strip_suffix(".json"))
                    .and_then(|n| n.parse().ok())
                {
                    ids.insert(id);
                }
            }
        }
        ids
    }

    /// Reads every computer's record of `sync_id`, this computer's included.
    /// A damaged file is skipped and reported, and the last good copy read
    /// in this session is used instead.
    pub fn read_doc(&mut self, sync_id: SyncId) -> DocRead {
        let mut out = DocRead::default();
        for d in self.device_ids() {
            let path = self.doc_path(d, sync_id);
            let result = read_limited(&path, MAX_RECORD_BYTES)
                .and_then(|b| DocRecord::from_bytes(&b))
                .and_then(|r| {
                    if r.sync_id == sync_id {
                        Ok(r)
                    } else {
                        Err(SyncError::Damaged(
                            "the record is for another document".into(),
                        ))
                    }
                });
            match result {
                Ok(record) => {
                    self.last_good.insert((d, sync_id), record.clone());
                    out.records.push((d, record));
                }
                Err(e) if not_found(&e) => {}
                Err(SyncError::NewerFormat { found }) => out.problems.push(Problem::NewerFormat {
                    device: d,
                    file: FileKind::Doc(sync_id),
                    found,
                }),
                Err(e) => {
                    let last = self.last_good.get(&(d, sync_id)).cloned();
                    log::warn!("sync: skipped a damaged record ({e})");
                    out.problems.push(Problem::Damaged {
                        device: Some(d),
                        file: FileKind::Doc(sync_id),
                        reason: e.to_string(),
                        kept_last_good: last.is_some(),
                    });
                    if let Some(record) = last {
                        out.records.push((d, record));
                    }
                }
            }
        }
        out
    }

    /// Reads every computer's record of `local`'s document and merges them
    /// into `local`, in order of computer id (any order gives the same
    /// result). `clock` takes in every stamp, so this computer's next change
    /// comes after all of them. A computer whose clock is more than a day
    /// ahead is reported the first time it is seen in this session, and
    /// kept in [`SyncFolder::clocks_ahead`].
    pub fn merge_doc(&mut self, local: &mut DocRecord, clock: &mut Clock) -> Merged {
        let read = self.read_doc(local.sync_id);
        let mut merged = Merged {
            report: MergeReport::default(),
            problems: read.problems,
        };
        for (_, record) in &read.records {
            for stamp in record.stamps() {
                if let Some(w) = clock.observe(stamp) {
                    let seen = self.ahead.entry(w.device).or_insert(0);
                    if *seen == 0 {
                        merged.problems.push(Problem::ClockAhead(w));
                    }
                    *seen = (*seen).max(w.ahead_ms);
                }
            }
            match local.merge(record) {
                Ok(report) => merged.report.extend(report),
                // read_doc only returns records for this document.
                Err(e) => log::warn!("sync: {e}"),
            }
        }
        merged
    }

    /// Writes this computer's merged view of a document, by atomic replace.
    /// Refused when sync is read-only.
    pub fn write_doc(&self, record: &DocRecord) -> Result<(), SyncError> {
        if let Some(r) = self.read_only {
            return Err(SyncError::ReadOnly(r));
        }
        let bytes = record.to_bytes()?;
        textweaver_store::atomic_write(&self.doc_path(self.device, record.sync_id), &bytes)?;
        Ok(())
    }
}
