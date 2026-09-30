//! The groups that are not about one document (ADR-0049): portable
//! settings, profiles, key overrides, the word list, the glossary and
//! pronunciations, and favorite voices.
//!
//! Each group is one file per computer, `devices/<device-id>/<name>.json`
//! ([`GroupFile`]), holding that computer's full merged view as a
//! [`GroupRecord`]: named maps of registers, where the newest change wins
//! key by key and a deletion is a record, and named sets, where adding
//! wins and a removal is recorded. The values are JSON, so a newer
//! textweaver's settings pass through an older one unchanged.
//!
//! The files, and what each holds:
//!
//! - `settings.json`: map `settings`, by dotted setting path.
//! - `profiles.json`: map `profiles`, by profile name.
//! - `keymap.json`: map `keymap`, by `mac:<action>` or `pc:<action>`.
//! - `words.json`: set `words`.
//! - `glossary.json`: maps `glossary` and `pronunciations`, by term.
//! - `voices.json`: set `voices`, by voice id.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::merge::{AddWinsSet, ChangeKind, RegisterMap};
use crate::record::{FORMAT, MAX_RECORD_BYTES};
use crate::{DeviceId, Stamp, SyncError};

/// One group's file in a computer's folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GroupFile {
    /// `settings.json`: portable settings.
    Settings,
    /// `profiles.json`: profile definitions.
    Profiles,
    /// `keymap.json`: key overrides, labeled by system.
    Keymap,
    /// `words.json`: the personal word list.
    Words,
    /// `glossary.json`: glossary entries and pronunciations.
    Glossary,
    /// `voices.json`: favorite voices.
    Voices,
}

impl GroupFile {
    /// Every group file.
    pub const ALL: [GroupFile; 6] = [
        GroupFile::Settings,
        GroupFile::Profiles,
        GroupFile::Keymap,
        GroupFile::Words,
        GroupFile::Glossary,
        GroupFile::Voices,
    ];

    /// The file's name in a computer's folder.
    pub const fn file_name(self) -> &'static str {
        match self {
            GroupFile::Settings => "settings.json",
            GroupFile::Profiles => "profiles.json",
            GroupFile::Keymap => "keymap.json",
            GroupFile::Words => "words.json",
            GroupFile::Glossary => "glossary.json",
            GroupFile::Voices => "voices.json",
        }
    }
}

/// One group's merged view: maps of registers and sets, by name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupRecord {
    /// The format it was written in.
    pub format: u32,
    /// Registers by key, newest wins; `None` values are deletion records.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub maps: BTreeMap<String, RegisterMap<Value>>,
    /// Sets where adding wins, with removal records.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sets: BTreeMap<String, AddWinsSet>,
}

impl Default for GroupRecord {
    fn default() -> Self {
        Self {
            format: FORMAT,
            maps: BTreeMap::new(),
            sets: BTreeMap::new(),
        }
    }
}

/// One change a merge made to a [`GroupRecord`].
#[derive(Clone, Debug, PartialEq)]
pub struct GroupChange {
    /// The map or set.
    pub name: String,
    /// The key or item.
    pub key: String,
    /// Added, replaced, removed, or brought back.
    pub kind: ChangeKind,
    /// The computer whose change won.
    pub by: DeviceId,
}

impl GroupRecord {
    /// An empty record.
    pub fn new() -> Self {
        Self::default()
    }

    /// The map `name`, made when missing.
    pub fn map_mut(&mut self, name: &str) -> &mut RegisterMap<Value> {
        self.maps.entry(name.to_owned()).or_default()
    }

    /// The set `name`, made when missing.
    pub fn set_mut(&mut self, name: &str) -> &mut AddWinsSet {
        self.sets.entry(name.to_owned()).or_default()
    }

    /// The map `name`, if it has one.
    pub fn map(&self, name: &str) -> Option<&RegisterMap<Value>> {
        self.maps.get(name)
    }

    /// The set `name`, if it has one.
    pub fn set(&self, name: &str) -> Option<&AddWinsSet> {
        self.sets.get(name)
    }

    /// Whether it holds nothing.
    pub fn is_empty(&self) -> bool {
        self.maps.values().all(|m| m.0.is_empty()) && self.sets.values().all(|s| s.0.is_empty())
    }

    /// Every stamp held.
    pub fn stamps(&self) -> Vec<Stamp> {
        let mut out: Vec<Stamp> = self.maps.values().flat_map(RegisterMap::stamps).collect();
        out.extend(self.sets.values().flat_map(AddWinsSet::stamps));
        out
    }

    /// Merges `other` in, listing what changed. Commutative, associative,
    /// and idempotent, like every merge in [`crate::merge`].
    pub fn merge(&mut self, other: &GroupRecord) -> Vec<GroupChange> {
        let mut out = Vec::new();
        for (name, theirs) in &other.maps {
            for c in self.map_mut(name).merge(theirs) {
                out.push(GroupChange {
                    name: name.clone(),
                    key: c.id,
                    kind: c.kind,
                    by: c.by,
                });
            }
        }
        for (name, theirs) in &other.sets {
            let ours = self.set_mut(name);
            let before: BTreeMap<String, bool> = ours
                .0
                .keys()
                .chain(theirs.0.keys())
                .map(|k| (k.clone(), ours.contains(k)))
                .collect();
            ours.merge(theirs);
            for (item, was) in before {
                let now = ours.contains(&item);
                if now == was {
                    continue;
                }
                let e = ours.0.get(&item).copied().unwrap_or_default();
                let stamp = if now { e.added } else { e.removed };
                let Some(stamp) = stamp else { continue };
                out.push(GroupChange {
                    name: name.clone(),
                    key: item,
                    kind: if now {
                        ChangeKind::Added
                    } else {
                        ChangeKind::Removed
                    },
                    by: stamp.device,
                });
            }
        }
        out
    }

    /// Reads a record from a file's bytes. A truncated or damaged file is
    /// an error, never a panic; a newer format is
    /// [`SyncError::NewerFormat`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SyncError> {
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_RECORD_BYTES {
            return Err(SyncError::TooLarge);
        }
        let value: Value =
            serde_json::from_slice(bytes).map_err(|e| SyncError::Damaged(e.to_string()))?;
        let format = value
            .get("format")
            .and_then(Value::as_u64)
            .ok_or_else(|| SyncError::Damaged("no format number".into()))?;
        if format > u64::from(FORMAT) {
            return Err(SyncError::NewerFormat {
                found: u32::try_from(format).unwrap_or(u32::MAX),
            });
        }
        serde_json::from_value(value).map_err(|e| SyncError::Damaged(e.to_string()))
    }

    /// The record as file bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, SyncError> {
        let mut out = serde_json::to_vec(self).map_err(|e| SyncError::Damaged(e.to_string()))?;
        out.push(b'\n');
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const A: DeviceId = DeviceId::from_u128(0xa);
    const B: DeviceId = DeviceId::from_u128(0xb);

    fn s(t: u64, d: DeviceId) -> Stamp {
        Stamp::new(t, d)
    }

    #[test]
    fn maps_take_the_newest_value_and_sets_let_adding_win() {
        let mut laptop = GroupRecord::new();
        laptop
            .map_mut("settings")
            .set("speech.rate", s(10, A), Value::from(300));
        laptop.set_mut("words").insert("mitochondrion", s(10, A));
        let mut lab = GroupRecord::new();
        lab.map_mut("settings")
            .set("speech.rate", s(20, B), Value::from(250));
        lab.set_mut("words").insert("ribosome", s(5, B));
        lab.set_mut("words").remove("mitochondrion", s(9, B));

        let changes = laptop.merge(&lab);
        assert_eq!(
            laptop.map("settings").and_then(|m| m.get("speech.rate")),
            Some(&Value::from(250))
        );
        let words: Vec<&str> = laptop.set("words").map(|w| w.items().collect()).unwrap();
        assert_eq!(words, ["mitochondrion", "ribosome"]);
        assert!(
            changes
                .iter()
                .any(|c| c.key == "speech.rate" && c.kind == ChangeKind::Replaced && c.by == B)
        );
        assert!(
            changes
                .iter()
                .any(|c| c.key == "ribosome" && c.kind == ChangeKind::Added)
        );
        assert!(
            laptop.merge(&lab).is_empty(),
            "a second merge changes nothing"
        );
    }

    #[test]
    fn a_removal_is_listed_with_its_computer() {
        let mut here = GroupRecord::new();
        here.set_mut("voices").insert("eci:reed", s(1, A));
        let mut there = here.clone();
        there.set_mut("voices").remove("eci:reed", s(2, B));
        let changes = here.merge(&there);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].kind, ChangeKind::Removed);
        assert_eq!(changes[0].by, B);
    }

    #[test]
    fn bytes_round_trip_and_a_newer_format_is_refused() {
        let mut r = GroupRecord::new();
        r.map_mut("profiles").set(
            "Study",
            s(3, A),
            serde_json::json!({"speech": {"rate": 200}}),
        );
        let back = GroupRecord::from_bytes(&r.to_bytes().unwrap()).unwrap();
        assert_eq!(back, r);
        assert!(matches!(
            GroupRecord::from_bytes(b"{\"format\": 99}"),
            Err(SyncError::NewerFormat { found: 99 })
        ));
        assert!(GroupRecord::from_bytes(b"{\"format\": 1, \"maps\": ").is_err());
    }

    fn record() -> impl Strategy<Value = GroupRecord> {
        let entry = (
            0u8..4,
            0u64..5,
            0u8..2,
            prop::option::of(0i64..3),
            any::<bool>(),
        );
        prop::collection::vec(entry, 0..12).prop_map(|entries| {
            let mut r = GroupRecord::new();
            for (key, t, d, v, set) in entries {
                let dev = if d == 0 { A } else { B };
                let key = format!("k{key}");
                if set {
                    match v {
                        Some(_) => r.set_mut("s").insert(key, s(t, dev)),
                        None => r.set_mut("s").remove(key, s(t, dev)),
                    }
                } else {
                    match v {
                        Some(v) => r.map_mut("m").set(key, s(t, dev), Value::from(v)),
                        None => r.map_mut("m").delete(key, s(t, dev)),
                    }
                }
            }
            r
        })
    }

    proptest! {
        #[test]
        fn merge_order_and_repeats_never_change_the_result(
            a in record(), b in record(), c in record()
        ) {
            let mut abc = a.clone();
            abc.merge(&b);
            abc.merge(&c);
            let mut cba = c.clone();
            cba.merge(&b);
            cba.merge(&a);
            cba.merge(&a);
            let live = |r: &GroupRecord| {
                let m: Vec<(String, Option<Value>)> = r
                    .map("m")
                    .map(|m| m.0.iter().map(|(k, v)| (k.clone(), v.value.clone())).collect())
                    .unwrap_or_default();
                let s: Vec<String> = r
                    .set("s")
                    .map(|s| s.items().map(str::to_owned).collect())
                    .unwrap_or_default();
                (m, s)
            };
            prop_assert_eq!(live(&abc), live(&cba));
        }
    }
}
