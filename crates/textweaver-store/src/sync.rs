//! Folder sidecar sync: `<folder>/.textweaver/progress.json`.
//!
//! A library folder synced by Dropbox, OneDrive, Syncthing, or iCloud
//! carries a sidecar keyed by each document's path relative to the folder,
//! so reading progress travels with the files. Two devices can write the
//! same sidecar; this module reconciles them. The merge rules are ported
//! from `star/sync.py` with its 45 tests (docs/star-parity.md Part 3 §3):
//!
//! - per-document progress entries merge by [`ConflictPolicy`];
//! - annotation and highlight lists merge by id (a union, newest per id);
//! - the reserved `_meta` namespace merges document by document, newest wins;
//! - missing, empty, or corrupt input counts as empty and never panics.
//!
//! Changes from Star, all deliberate:
//!
//! - [`SidecarStore::record_progress`] no longer re-asserts the local entry
//!   after the merge, so `highest_progress` and `manual` protect the
//!   document being written too (Star's `record_progress` always kept the
//!   local entry, Part 3 §7 item 23);
//! - conflicts are returned to the caller instead of discarded (item 24);
//! - pending coalesced writes are flushed on drop and on demand (item 22);
//! - timestamps are written as RFC 3339 UTC; a numeric `ts` (Unix seconds)
//!   is compared as the equivalent RFC 3339 string rather than as Python's
//!   `str(number)` (item 19);
//! - numbers compare by value, so `1` equals `1.0` as in Python, but `true`
//!   does not equal `1`.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use textweaver_core::CharPos;

use crate::{StoreError, atomic_write};

/// How conflicting positions from two devices are resolved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictPolicy {
    /// The newest timestamp wins (Star default).
    #[default]
    Newest,
    /// The furthest position wins.
    HighestProgress,
    /// Keep both and ask.
    Manual,
}

impl ConflictPolicy {
    /// Every policy (Star's `POLICIES`).
    pub const ALL: [ConflictPolicy; 3] = [
        ConflictPolicy::Newest,
        ConflictPolicy::HighestProgress,
        ConflictPolicy::Manual,
    ];

    /// Parses a policy name. An unknown name becomes `Newest`, as in Star.
    pub fn parse(name: &str) -> Self {
        match name {
            "highest_progress" => ConflictPolicy::HighestProgress,
            "manual" => ConflictPolicy::Manual,
            _ => ConflictPolicy::Newest,
        }
    }

    /// The policy's name in settings.
    pub fn as_str(self) -> &'static str {
        match self {
            ConflictPolicy::Newest => "newest",
            ConflictPolicy::HighestProgress => "highest_progress",
            ConflictPolicy::Manual => "manual",
        }
    }
}

/// Which side wins an exact timestamp tie.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Prefer {
    /// The local side (the write path: the local payload is the freshest
    /// local edit).
    Local,
    /// The remote side (the read path: the arriving copy is the last write).
    #[default]
    Remote,
}

/// Which side a merge kept for a conflict.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    /// The local value was kept.
    Local,
    /// The remote value was kept.
    Remote,
    /// Nothing was chosen (the `manual` policy keeps local and asks).
    #[default]
    Unresolved,
}

/// One entry that diverged between the local and remote sidecar.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Conflict {
    /// The sidecar key: a document's relative path, a list key such as
    /// `annotations`, or `_meta`.
    pub path: String,
    /// The diverging sub-entry: an annotation id, or the document within
    /// `_meta`. `None` when the whole entry conflicts.
    pub field: Option<String>,
    /// The local value.
    pub local: Value,
    /// The remote value.
    pub remote: Value,
    /// What the merge kept.
    pub resolution: Resolution,
}

impl Conflict {
    /// A conflict with the default resolution, [`Resolution::Unresolved`].
    pub fn new(
        path: impl Into<String>,
        field: Option<String>,
        local: Value,
        remote: Value,
    ) -> Self {
        Conflict {
            path: path.into(),
            field,
            local,
            remote,
            resolution: Resolution::Unresolved,
        }
    }

    /// The same conflict with `resolution`.
    pub fn resolved(mut self, resolution: Resolution) -> Self {
        self.resolution = resolution;
        self
    }
}

/// The reserved key for cross-device metadata (portable reading stats and
/// an annotation count), keyed by the same relative paths.
pub const META_KEY: &str = "_meta";

/// Keys whose values are lists merged by id, whatever their contents.
pub const ANNOTATION_KEYS: [&str; 3] = ["annotations", "highlights", "user_highlights"];

/// The sidecar directory inside a library folder.
pub const SIDECAR_DIR: &str = ".textweaver";

/// The sidecar file name.
pub const SIDECAR_FILE: &str = "progress.json";

/// How long [`SidecarStore`] coalesces writes to one sidecar (Star 0.5 s).
pub const SIDECAR_DEBOUNCE: Duration = Duration::from_millis(500);

/// A JSON object that keeps its key order, used for the top level of a
/// sidecar so merges keep a stable, diff-friendly order: local keys first,
/// then remote-only keys (Star relied on Python's ordered dicts).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SidecarMap(Vec<(String, Value)>);

impl SidecarMap {
    /// An empty map.
    pub fn new() -> Self {
        SidecarMap::default()
    }

    /// The entries of a JSON object, in the object's order; anything else
    /// (null, a list, a string, a number) is empty, as Star's `_as_dict`.
    pub fn from_value(value: &Value) -> Self {
        match value {
            Value::Object(m) => SidecarMap(m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
            _ => SidecarMap::new(),
        }
    }

    /// The map as a JSON object (nested order is not kept).
    pub fn to_value(&self) -> Value {
        Value::Object(self.0.iter().cloned().collect())
    }

    /// The value at `key`.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Sets `key`, in place when present, else at the end.
    pub fn insert(&mut self, key: impl Into<String>, value: Value) {
        let key = key.into();
        match self.0.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = value,
            None => self.0.push((key, value)),
        }
    }

    /// Removes `key`, returning its value.
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        let i = self.0.iter().position(|(k, _)| k == key)?;
        Some(self.0.remove(i).1)
    }

    /// True when `key` is present.
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Keys in order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(k, _)| k.as_str())
    }

    /// Entries in order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// True when there are no entries.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The map without the reserved `_meta` namespace (Star's
    /// `load_sidecar`), so callers iterating documents see only documents.
    pub fn documents(&self) -> SidecarMap {
        SidecarMap(
            self.0
                .iter()
                .filter(|(k, _)| k != META_KEY)
                .cloned()
                .collect(),
        )
    }
}

impl Serialize for SidecarMap {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            m.serialize_entry(k, v)?;
        }
        m.end()
    }
}

impl<'de> Deserialize<'de> for SidecarMap {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = SidecarMap;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a JSON object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<SidecarMap, A::Error> {
                let mut out = SidecarMap::new();
                while let Some((k, v)) = a.next_entry::<String, Value>()? {
                    out.insert(k, v);
                }
                Ok(out)
            }
        }
        d.deserialize_map(V)
    }
}

// ---------------------------------------------------------------------------
// Merge helpers (star/sync.py)
// ---------------------------------------------------------------------------

/// Python's `dict.get`: a missing key and JSON `null` are both `None`.
fn present(v: Option<&Value>) -> Option<&Value> {
    v.filter(|v| !v.is_null())
}

/// Deep equality with Python's numeric rule (`1 == 1.0`).
fn json_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            if let (Some(i), Some(j)) = (x.as_i64(), y.as_i64()) {
                i == j
            } else if let (Some(i), Some(j)) = (x.as_u64(), y.as_u64()) {
                i == j
            } else {
                x.as_f64() == y.as_f64()
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| json_eq(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| json_eq(v, w)))
        }
        _ => a == b,
    }
}

/// Python's `str()` for the scalar values sidecars hold.
fn py_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(true) => "True".to_owned(),
        Value::Bool(false) => "False".to_owned(),
        Value::Null => "None".to_owned(),
        other => other.to_string(),
    }
}

/// The comparable timestamp of an entry (Star's `_ts_key`): `ts`, else
/// `last_ts`, else `""`, which sorts earliest. A numeric timestamp (Unix
/// seconds) compares as its RFC 3339 form.
pub fn ts_key(entry: &Value) -> String {
    let Value::Object(m) = entry else {
        return String::new();
    };
    let ts = present(m.get("ts")).or_else(|| present(m.get("last_ts")));
    match ts {
        None => String::new(),
        Some(Value::Number(n)) => match n.as_i64().or_else(|| n.as_f64().map(|f| f.floor() as i64))
        {
            Some(secs) => crate::time::rfc3339(secs),
            None => n.to_string(),
        },
        Some(v) => py_str(v),
    }
}

/// The reading position of an entry for `highest_progress` (Star's
/// `_progress_value`): the first of `offset` and `pct` that is a number
/// (not a bool), else -1.
pub fn progress_value(entry: &Value) -> f64 {
    let Value::Object(m) = entry else {
        return -1.0;
    };
    for field in ["offset", "pct"] {
        if let Some(Value::Number(n)) = m.get(field) {
            if let Some(f) = n.as_f64() {
                return f;
            }
        }
    }
    -1.0
}

/// The newer of two entries by timestamp; `prefer` breaks exact ties
/// (Star's `_newest`).
fn newest<'a>(local: &'a Value, remote: &'a Value, prefer: Prefer) -> (&'a Value, Resolution) {
    let (lt, rt) = (ts_key(local), ts_key(remote));
    let remote_wins = match rt.cmp(&lt) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => prefer == Prefer::Remote,
    };
    if remote_wins {
        (remote, Resolution::Remote)
    } else {
        (local, Resolution::Local)
    }
}

/// Reconciles one document's entry (Star's `_merge_entry`). Returns the
/// winner and the conflict, if the two sides both exist and differ. Also
/// usable to choose between a local saved position and a sidecar one when
/// resuming, so resume honors the policy (Star ignored it, Part 3 §7
/// item 25).
pub fn resolve_entry(
    key: &str,
    local: Option<&Value>,
    remote: Option<&Value>,
    policy: ConflictPolicy,
    prefer: Prefer,
) -> (Value, Option<Conflict>) {
    let (local, remote) = match (present(local), present(remote)) {
        (l, None) => return (l.cloned().unwrap_or(Value::Null), None),
        (None, Some(r)) => return (r.clone(), None),
        (Some(l), Some(r)) => (l, r),
    };
    if json_eq(local, remote) {
        return (local.clone(), None);
    }
    let conflict = Conflict::new(key, None, local.clone(), remote.clone());
    match policy {
        ConflictPolicy::HighestProgress => {
            let (lp, rp) = (progress_value(local), progress_value(remote));
            let (winner, res) = if rp > lp {
                (remote, Resolution::Remote)
            } else if lp > rp {
                (local, Resolution::Local)
            } else {
                newest(local, remote, prefer)
            };
            (winner.clone(), Some(conflict.resolved(res)))
        }
        ConflictPolicy::Manual => (local.clone(), Some(conflict)),
        ConflictPolicy::Newest => {
            let (winner, res) = newest(local, remote, prefer);
            (winner.clone(), Some(conflict.resolved(res)))
        }
    }
}

/// The id of an annotation: `str(a["id"])` when it is truthy (Star's
/// `_ann_id`). `""`, `0`, `false`, and `null` count as no id.
fn ann_id(ann: &Value) -> Option<String> {
    let id = ann.as_object()?.get("id")?;
    let truthy = match id {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    };
    truthy.then(|| py_str(id))
}

/// Unions two annotation or highlight lists by id, newest per id (Star's
/// `merge_annotations`). A non-list side counts as empty. Local order comes
/// first, then remote-only ids in remote order, then id-less remote entries.
pub fn merge_annotations(
    key: &str,
    local: Option<&Value>,
    remote: Option<&Value>,
    mut conflicts: Option<&mut Vec<Conflict>>,
    prefer: Prefer,
) -> Vec<Value> {
    let empty = Vec::new();
    let llist = local.and_then(Value::as_array).unwrap_or(&empty);
    let rlist = remote.and_then(Value::as_array).unwrap_or(&empty);

    let mut remote_by_id: HashMap<String, &Value> = HashMap::new();
    let mut remote_idless: Vec<&Value> = Vec::new();
    for ann in rlist {
        match ann_id(ann) {
            // The last duplicate id on the remote side wins the comparison.
            Some(id) => {
                remote_by_id.insert(id, ann);
            }
            None => remote_idless.push(ann),
        }
    }

    let mut merged = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for ann in llist {
        let Some(id) = ann_id(ann) else {
            merged.push(ann.clone());
            continue;
        };
        seen.insert(id.clone());
        match remote_by_id.get(&id) {
            Some(r) if !json_eq(r, ann) => {
                let (winner, res) = newest(ann, r, prefer);
                if let Some(c) = conflicts.as_deref_mut() {
                    c.push(Conflict::new(key, Some(id), ann.clone(), (*r).clone()).resolved(res));
                }
                merged.push(winner.clone());
            }
            _ => merged.push(ann.clone()),
        }
    }
    for ann in rlist {
        if let Some(id) = ann_id(ann) {
            if seen.insert(id) {
                merged.push(ann.clone());
            }
        }
    }
    merged.extend(remote_idless.into_iter().cloned());
    merged
}

/// Merges the `_meta` namespace document by document, newest wins; the
/// policy does not apply (Star's `_merge_meta`).
fn merge_meta(
    local: Option<&Value>,
    remote: Option<&Value>,
    conflicts: &mut Vec<Conflict>,
    prefer: Prefer,
) -> Value {
    let lmeta = SidecarMap::from_value(local.unwrap_or(&Value::Null));
    let rmeta = SidecarMap::from_value(remote.unwrap_or(&Value::Null));
    let mut out = lmeta.clone();
    for (rel, r_entry) in rmeta.iter() {
        match present(lmeta.get(rel)) {
            None => out.insert(rel, r_entry.clone()),
            Some(l_entry) if json_eq(l_entry, r_entry) => {}
            Some(l_entry) => {
                let (winner, res) = newest(l_entry, r_entry, prefer);
                conflicts.push(
                    Conflict::new(
                        META_KEY,
                        Some(rel.to_owned()),
                        l_entry.clone(),
                        r_entry.clone(),
                    )
                    .resolved(res),
                );
                out.insert(rel, winner.clone());
            }
        }
    }
    out.to_value()
}

/// True when a key should merge by id: either side is a list and neither
/// is an object (Star's `_is_annotation_list`). A list against an object is
/// corruption, not an annotation collection, and goes to the entry merge,
/// which keeps the valid object.
pub fn is_annotation_list(local: Option<&Value>, remote: Option<&Value>) -> bool {
    let is = |v: Option<&Value>, f: fn(&Value) -> bool| v.is_some_and(f);
    if is(local, Value::is_object) || is(remote, Value::is_object) {
        return false;
    }
    is(local, Value::is_array) || is(remote, Value::is_array)
}

/// Reconciles two ordered sidecar maps (Star's `merge_progress`). Returns
/// the merged map, local keys first then remote-only keys, and every
/// conflict with its resolution.
pub fn merge_maps(
    local: &SidecarMap,
    remote: &SidecarMap,
    policy: ConflictPolicy,
    prefer: Prefer,
) -> (SidecarMap, Vec<Conflict>) {
    let mut conflicts = Vec::new();
    let mut merged = SidecarMap::new();
    let keys: Vec<&str> = local
        .keys()
        .chain(remote.keys().filter(|k| !local.contains_key(k)))
        .collect();
    for key in keys {
        let (l, r) = (local.get(key), remote.get(key));
        let value = if key == META_KEY {
            merge_meta(present(l), present(r), &mut conflicts, prefer)
        } else if ANNOTATION_KEYS.contains(&key) || is_annotation_list(present(l), present(r)) {
            Value::Array(merge_annotations(
                key,
                present(l),
                present(r),
                Some(&mut conflicts),
                prefer,
            ))
        } else {
            let (v, c) = resolve_entry(key, l, r, policy, prefer);
            conflicts.extend(c);
            v
        };
        merged.insert(key, value);
    }
    (merged, conflicts)
}

/// [`merge_maps`] over JSON values: a non-object side counts as empty.
pub fn merge_progress(
    local: &Value,
    remote: &Value,
    policy: ConflictPolicy,
    prefer: Prefer,
) -> (SidecarMap, Vec<Conflict>) {
    merge_maps(
        &SidecarMap::from_value(local),
        &SidecarMap::from_value(remote),
        policy,
        prefer,
    )
}

// ---------------------------------------------------------------------------
// Progress entries and library folders
// ---------------------------------------------------------------------------

/// One document's synced position: `{"offset": 1200, "pct": 42, "ts": "..."}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressEntry {
    /// Char offset of the word being read.
    pub offset: CharPos,
    /// Percentage through the document, floored.
    pub pct: u8,
    /// When it was saved, RFC 3339 UTC (Star: zone-less local time).
    pub ts: String,
}

impl ProgressEntry {
    /// An entry for a position saved at `ts` (Unix seconds).
    pub fn new(offset: CharPos, pct: u8, ts: i64) -> Self {
        ProgressEntry {
            offset,
            pct,
            ts: crate::time::rfc3339(ts),
        }
    }

    /// The entry as JSON.
    pub fn to_value(&self) -> Value {
        serde_json::json!({ "offset": self.offset.0, "pct": self.pct, "ts": self.ts })
    }

    /// Reads an entry, tolerating Star's shapes: a missing offset is 0, a
    /// float pct is floored, a missing ts is empty.
    pub fn from_value(v: &Value) -> Option<Self> {
        let m = v.as_object()?;
        let num = |k: &str| m.get(k).and_then(Value::as_f64).filter(|f| *f >= 0.0);
        let offset = num("offset");
        let pct = num("pct");
        if offset.is_none() && pct.is_none() {
            return None;
        }
        Some(ProgressEntry {
            offset: CharPos(offset.map_or(0, |f| f as usize)),
            pct: pct.map_or(0, |f| f.min(100.0) as u8),
            ts: ts_key(v),
        })
    }

    /// The timestamp as Unix seconds, when it parses.
    pub fn ts_secs(&self) -> Option<i64> {
        crate::time::parse_timestamp(&self.ts)
    }
}

/// `<folder>/.textweaver/progress.json`.
pub fn sidecar_file(folder: &Path) -> PathBuf {
    folder.join(SIDECAR_DIR).join(SIDECAR_FILE)
}

/// The canonical path when it exists, else the absolute path.
fn resolved(p: &Path) -> PathBuf {
    std::fs::canonicalize(p)
        .or_else(|_| std::path::absolute(p))
        .unwrap_or_else(|_| p.to_owned())
}

fn relative_posix(root: &Path, p: &Path) -> Option<String> {
    let rel = p.strip_prefix(root).ok()?;
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// The deepest library folder containing `path`, and the path relative to
/// it with `/` separators (Star's `folder_for`). `None` when the document is
/// in no library folder.
pub fn folder_for(folders: &[PathBuf], path: &Path) -> Option<(PathBuf, String)> {
    let p = resolved(path);
    let p_abs = std::path::absolute(path).unwrap_or_else(|_| path.to_owned());
    let mut best: Option<(PathBuf, String)> = None;
    for folder in folders {
        let root = resolved(folder);
        let root_abs = std::path::absolute(folder).unwrap_or_else(|_| folder.clone());
        let Some(rel) = relative_posix(&root, &p).or_else(|| relative_posix(&root_abs, &p_abs))
        else {
            continue;
        };
        let deeper = best
            .as_ref()
            .is_none_or(|(b, _)| root.as_os_str().len() > b.as_os_str().len());
        if deeper {
            best = Some((root, rel));
        }
    }
    best
}

/// Reads a sidecar; empty on any failure (missing, unreadable, not an
/// object), as Star's `_read_sidecar_raw`.
pub fn read_sidecar(folder: &Path) -> SidecarMap {
    std::fs::read_to_string(sidecar_file(folder))
        .ok()
        .and_then(|t| serde_json::from_str::<SidecarMap>(&t).ok())
        .unwrap_or_default()
}

/// Merges a local payload with the sidecar currently on disk, which a sync
/// may have rewritten since it was read (Star's `_reconcile_before_write`).
/// The disk copy is the remote side; exact ties go to the local payload.
/// With no disk copy the payload is returned unchanged.
pub fn reconcile_before_write(
    folder: &Path,
    local: &SidecarMap,
    policy: ConflictPolicy,
) -> (SidecarMap, Vec<Conflict>) {
    let remote = read_sidecar(folder);
    if remote.is_empty() {
        return (local.clone(), Vec::new());
    }
    merge_maps(local, &remote, policy, Prefer::Local)
}

fn write_sidecar(folder: &Path, data: &SidecarMap) -> Result<(), StoreError> {
    let path = sidecar_file(folder);
    let text = serde_json::to_string_pretty(data).map_err(|e| StoreError::Parse {
        path: path.clone(),
        message: e.to_string(),
    })?;
    atomic_write(&path, text.as_bytes())
}

/// What [`SidecarStore::record_progress`] did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Recorded {
    /// True when the sidecar was written now; false when the write was
    /// coalesced and is pending.
    pub written: bool,
    /// Conflicts found while reconciling with the file on disk.
    pub conflicts: Vec<Conflict>,
}

#[derive(Debug, Default)]
struct SidecarState {
    pending: HashMap<PathBuf, SidecarMap>,
    last_write: HashMap<PathBuf, Instant>,
    writes: u64,
}

#[derive(Debug)]
struct SidecarInner {
    policy: Mutex<ConflictPolicy>,
    debounce: Duration,
    state: Mutex<SidecarState>,
}

impl SidecarInner {
    fn lock(&self) -> MutexGuard<'_, SidecarState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn policy(&self) -> ConflictPolicy {
        *self.policy.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn flush_folder(
        &self,
        st: &mut SidecarState,
        folder: &Path,
    ) -> Result<Vec<Conflict>, StoreError> {
        let Some(data) = st.pending.get(folder).cloned() else {
            return Ok(Vec::new());
        };
        let (to_write, conflicts) = reconcile_before_write(folder, &data, self.policy());
        write_sidecar(folder, &to_write)?;
        st.pending.remove(folder);
        st.last_write.insert(folder.to_owned(), Instant::now());
        st.writes += 1;
        Ok(conflicts)
    }
}

impl Drop for SidecarInner {
    fn drop(&mut self) {
        let mut st = std::mem::take(&mut *self.lock());
        let folders: Vec<PathBuf> = st.pending.keys().cloned().collect();
        for folder in folders {
            let _ = self.flush_folder(&mut st, &folder);
        }
    }
}

/// Reads and writes library-folder sidecars with Star's coalescing, under a
/// lock so concurrent writers cannot interleave a read-modify-write.
/// Clones share state; the last clone dropped flushes pending writes.
#[derive(Clone, Debug)]
pub struct SidecarStore {
    inner: Arc<SidecarInner>,
}

impl Default for SidecarStore {
    fn default() -> Self {
        SidecarStore::new(ConflictPolicy::default())
    }
}

impl SidecarStore {
    /// A store merging with `policy` and coalescing writes for
    /// [`SIDECAR_DEBOUNCE`].
    pub fn new(policy: ConflictPolicy) -> Self {
        SidecarStore::with_debounce(policy, SIDECAR_DEBOUNCE)
    }

    /// A store with a custom debounce window.
    pub fn with_debounce(policy: ConflictPolicy, debounce: Duration) -> Self {
        SidecarStore {
            inner: Arc::new(SidecarInner {
                policy: Mutex::new(policy),
                debounce,
                state: Mutex::new(SidecarState::default()),
            }),
        }
    }

    /// The merge policy in use.
    pub fn policy(&self) -> ConflictPolicy {
        self.inner.policy()
    }

    /// Changes the merge policy (the user changed the setting).
    pub fn set_policy(&self, policy: ConflictPolicy) {
        *self.inner.policy.lock().unwrap_or_else(|e| e.into_inner()) = policy;
    }

    /// The freshest full sidecar: pending data when there is some, else the
    /// file.
    pub fn state(&self, folder: &Path) -> SidecarMap {
        let st = self.inner.lock();
        st.pending
            .get(folder)
            .cloned()
            .unwrap_or_else(|| read_sidecar(folder))
    }

    /// Document entries, without `_meta` (Star's `load_sidecar`).
    pub fn load(&self, folder: &Path) -> SidecarMap {
        self.state(folder).documents()
    }

    /// The progress entry for `rel` in `folder`'s sidecar.
    pub fn progress_for(&self, folder: &Path, rel: &str) -> Option<Value> {
        present(self.state(folder).get(rel)).cloned()
    }

    /// The `_meta` entry for `rel` in `folder`'s sidecar.
    pub fn metadata_for(&self, folder: &Path, rel: &str) -> Option<Value> {
        let state = self.state(folder);
        state
            .get(META_KEY)
            .and_then(Value::as_object)
            .and_then(|m| present(m.get(rel)))
            .cloned()
    }

    /// Records `entry` for `rel` (and its `_meta` entry, when given) in
    /// `folder`'s sidecar. Within the debounce window of the last write the
    /// data stays pending (readable through [`state`](Self::state));
    /// otherwise it is reconciled with the file on disk under the policy and
    /// written. Unlike Star, the recorded entry is not forced over the merge
    /// result, so `highest_progress` and `manual` hold for it too.
    pub fn record_progress(
        &self,
        folder: &Path,
        rel: &str,
        entry: Value,
        meta: Option<Value>,
    ) -> Result<Recorded, StoreError> {
        let folder = folder.to_owned();
        let mut st = self.inner.lock();
        let mut data = st
            .pending
            .get(&folder)
            .cloned()
            .unwrap_or_else(|| read_sidecar(&folder));
        data.insert(rel, entry);
        if let Some(meta_entry) = meta {
            let mut m = SidecarMap::from_value(data.get(META_KEY).unwrap_or(&Value::Null));
            m.insert(rel, meta_entry);
            data.insert(META_KEY, m.to_value());
        }
        st.pending.insert(folder.clone(), data);
        let recent = st
            .last_write
            .get(&folder)
            .is_some_and(|t| t.elapsed() < self.inner.debounce);
        if recent {
            return Ok(Recorded::default());
        }
        let conflicts = self.inner.flush_folder(&mut st, &folder)?;
        Ok(Recorded {
            written: true,
            conflicts,
        })
    }

    /// Writes `folder`'s pending data now, reconciled with the disk copy.
    /// Returns the conflicts found (empty when nothing was pending).
    pub fn flush(&self, folder: &Path) -> Result<Vec<Conflict>, StoreError> {
        let mut st = self.inner.lock();
        self.inner.flush_folder(&mut st, folder)
    }

    /// Writes every folder's pending data. Call on quit and on document
    /// switch. Stops at the first write error.
    pub fn flush_all(&self) -> Result<Vec<Conflict>, StoreError> {
        let mut st = self.inner.lock();
        let folders: Vec<PathBuf> = st.pending.keys().cloned().collect();
        let mut all = Vec::new();
        for folder in folders {
            all.extend(self.inner.flush_folder(&mut st, &folder)?);
        }
        Ok(all)
    }

    /// True when some sidecar data has not been written yet.
    pub fn has_pending(&self) -> bool {
        !self.inner.lock().pending.is_empty()
    }

    /// Number of sidecar files written by this store so far.
    pub fn writes(&self) -> u64 {
        self.inner.lock().writes
    }
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
