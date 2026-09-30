//! Merge types. Each merges so that the order in which computers' files are
//! read, and reading one twice, never changes the result (the merge is
//! commutative, associative, and idempotent; the property tests check it).
//!
//! - [`Register`]: one value with a [`Stamp`]; the newest wins. A deletion
//!   is a register with no value, so a deletion loses to a later edit and a
//!   later deletion wins over an earlier edit.
//! - [`RegisterMap`]: registers by id (notes, highlights, bookmarks,
//!   places). Its merge lists what changed, so the app can say so.
//! - [`AddWinsSet`]: a set with removal records, where adding wins a tie
//!   (tags, word lists, favorites).
//! - [`Counter`]: one count per computer, summed (reading time, sessions).
//!   Each computer only adds to its own count.
//! - [`Maximum`]: the largest value any computer reported (furthest point).
//! - [`Earliest`]: the smallest value any computer reported (when a
//!   document was first added to a library).

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use serde::{Deserialize, Serialize};

use crate::{DeviceId, Stamp};

/// A value that the newest change wins; `None` is a deletion record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Register<T> {
    /// When the value was set or deleted, and by which computer.
    pub stamp: Stamp,
    /// The value, or `None` when it was deleted (a missing value reads as
    /// `None`).
    pub value: Option<T>,
}

impl<T: Serialize> Register<T> {
    /// A register holding `value`.
    pub const fn set(stamp: Stamp, value: T) -> Self {
        Self {
            stamp,
            value: Some(value),
        }
    }

    /// A deletion record.
    pub const fn deleted(stamp: Stamp) -> Self {
        Self { stamp, value: None }
    }

    /// Whether this register wins over `other`. The later stamp wins. Two
    /// equal stamps should never hold different values (one computer never
    /// stamps two changes alike), but a damaged or hand-edited file could;
    /// then an edit wins over a deletion, and otherwise the value whose
    /// JSON sorts last, so every computer still picks the same one.
    pub fn wins_over(&self, other: &Self) -> bool {
        match self.stamp.cmp(&other.stamp) {
            std::cmp::Ordering::Greater => true,
            std::cmp::Ordering::Less => false,
            std::cmp::Ordering::Equal => match (&self.value, &other.value) {
                (Some(_), None) => true,
                (None, _) => false,
                (Some(a), Some(b)) => canonical(a) > canonical(b),
            },
        }
    }

    /// Takes `other` if it wins. Returns whether it did.
    pub fn merge(&mut self, other: &Self) -> bool
    where
        T: Clone,
    {
        if other.wins_over(self) {
            *self = other.clone();
            true
        } else {
            false
        }
    }
}

/// A value's JSON, for breaking exact ties. serde_json's maps are sorted,
/// so this is the same on every computer.
fn canonical<T: Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap_or_default()
}

/// What happened to one item when another computer's changes were merged
/// in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChangeKind {
    /// New here.
    Added,
    /// Replaced by a newer version.
    Replaced,
    /// Removed by a newer deletion.
    Removed,
    /// Brought back: it was deleted here, and another computer edited it
    /// later (a deletion loses to a later edit).
    Restored,
}

/// One change a merge made to a [`RegisterMap`].
#[derive(Clone, Debug, PartialEq)]
pub struct MapChange<T> {
    /// The item's id.
    pub id: String,
    /// What happened.
    pub kind: ChangeKind,
    /// The computer whose change won.
    pub by: DeviceId,
    /// The value it had here before, for [`ChangeKind::Replaced`] and
    /// [`ChangeKind::Removed`], so the app can back it up and say which
    /// one was replaced.
    pub previous: Option<T>,
}

/// Registers by id: every item, including deletion records, in id order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RegisterMap<T>(pub BTreeMap<String, Register<T>>);

impl<T> Default for RegisterMap<T> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl<T: Serialize + Clone + PartialEq> RegisterMap<T> {
    /// An empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets `id` to `value` at `stamp`, if that is newer than what is held.
    pub fn set(&mut self, id: impl Into<String>, stamp: Stamp, value: T) {
        self.put(id.into(), Register::set(stamp, value));
    }

    /// Records the deletion of `id` at `stamp`, if that is newer than what
    /// is held.
    pub fn delete(&mut self, id: impl Into<String>, stamp: Stamp) {
        self.put(id.into(), Register::deleted(stamp));
    }

    fn put(&mut self, id: String, reg: Register<T>) {
        match self.0.entry(id) {
            Entry::Vacant(e) => {
                e.insert(reg);
            }
            Entry::Occupied(mut e) => {
                e.get_mut().merge(&reg);
            }
        }
    }

    /// The live value of `id`, if it has one.
    pub fn get(&self, id: &str) -> Option<&T> {
        self.0.get(id).and_then(|r| r.value.as_ref())
    }

    /// The register of `id`, deletion records included.
    pub fn register(&self, id: &str) -> Option<&Register<T>> {
        self.0.get(id)
    }

    /// Live items, in id order.
    pub fn live(&self) -> impl Iterator<Item = (&str, &T)> {
        self.0
            .iter()
            .filter_map(|(k, r)| r.value.as_ref().map(|v| (k.as_str(), v)))
    }

    /// How many live items there are.
    pub fn live_len(&self) -> usize {
        self.live().count()
    }

    /// Every stamp held, deletion records included.
    pub fn stamps(&self) -> impl Iterator<Item = Stamp> + '_ {
        self.0.values().map(|r| r.stamp)
    }

    /// Merges `other` in and lists the changes that matter to a reader:
    /// items added, replaced with a different value, removed, or brought
    /// back. A newer stamp on the same value, or a deletion record for an
    /// item this computer never had, is taken silently.
    pub fn merge(&mut self, other: &Self) -> Vec<MapChange<T>> {
        let mut changes = Vec::new();
        for (id, theirs) in &other.0 {
            match self.0.entry(id.clone()) {
                Entry::Vacant(e) => {
                    if theirs.value.is_some() {
                        changes.push(MapChange {
                            id: id.clone(),
                            kind: ChangeKind::Added,
                            by: theirs.stamp.device,
                            previous: None,
                        });
                    }
                    e.insert(theirs.clone());
                }
                Entry::Occupied(mut e) => {
                    let ours = e.get_mut();
                    if !theirs.wins_over(ours) {
                        continue;
                    }
                    let kind = match (&ours.value, &theirs.value) {
                        (Some(a), Some(b)) if a == b => None,
                        (Some(_), Some(_)) => Some(ChangeKind::Replaced),
                        (Some(_), None) => Some(ChangeKind::Removed),
                        (None, Some(_)) => Some(ChangeKind::Restored),
                        (None, None) => None,
                    };
                    let previous = std::mem::replace(ours, theirs.clone()).value;
                    if let Some(kind) = kind {
                        changes.push(MapChange {
                            id: id.clone(),
                            kind,
                            by: theirs.stamp.device,
                            previous: match kind {
                                ChangeKind::Replaced | ChangeKind::Removed => previous,
                                ChangeKind::Added | ChangeKind::Restored => None,
                            },
                        });
                    }
                }
            }
        }
        changes
    }
}

/// When an item was last added and last removed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetEntry {
    /// The latest add.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub added: Option<Stamp>,
    /// The latest removal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub removed: Option<Stamp>,
}

impl SetEntry {
    /// In the set: added, and not removed later. Adding wins a tie.
    pub fn present(&self) -> bool {
        match (self.added, self.removed) {
            (Some(a), Some(r)) => a >= r,
            (Some(_), None) => true,
            (None, _) => false,
        }
    }
}

/// A set of strings with removal records, where adding wins.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AddWinsSet(pub BTreeMap<String, SetEntry>);

impl AddWinsSet {
    /// An empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `item` at `stamp`.
    pub fn insert(&mut self, item: impl Into<String>, stamp: Stamp) {
        let e = self.0.entry(item.into()).or_default();
        e.added = e.added.max(Some(stamp));
    }

    /// Removes `item` at `stamp`, keeping a removal record.
    pub fn remove(&mut self, item: impl Into<String>, stamp: Stamp) {
        let e = self.0.entry(item.into()).or_default();
        e.removed = e.removed.max(Some(stamp));
    }

    /// Whether `item` is in the set.
    pub fn contains(&self, item: &str) -> bool {
        self.0.get(item).is_some_and(SetEntry::present)
    }

    /// The items in the set, in order.
    pub fn items(&self) -> impl Iterator<Item = &str> {
        self.0
            .iter()
            .filter(|(_, e)| e.present())
            .map(|(k, _)| k.as_str())
    }

    /// Merges `other` in: the latest add and the latest removal of each item.
    pub fn merge(&mut self, other: &Self) {
        for (item, theirs) in &other.0 {
            let e = self.0.entry(item.clone()).or_default();
            e.added = e.added.max(theirs.added);
            e.removed = e.removed.max(theirs.removed);
        }
    }

    /// Every stamp held.
    pub fn stamps(&self) -> impl Iterator<Item = Stamp> + '_ {
        self.0
            .values()
            .flat_map(|e| e.added.into_iter().chain(e.removed))
    }
}

/// One count per computer; the total is their sum. A computer only ever
/// raises its own count, so merging takes each computer's largest.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Counter(pub BTreeMap<DeviceId, u64>);

impl Counter {
    /// A counter at zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `n` to `device`'s count.
    pub fn add(&mut self, device: DeviceId, n: u64) {
        let c = self.0.entry(device).or_default();
        *c = c.saturating_add(n);
    }

    /// `device`'s own count.
    pub fn of(&self, device: DeviceId) -> u64 {
        self.0.get(&device).copied().unwrap_or(0)
    }

    /// The sum over every computer.
    pub fn total(&self) -> u64 {
        self.0.values().fold(0u64, |a, b| a.saturating_add(*b))
    }

    /// Merges `other` in: each computer's largest count.
    pub fn merge(&mut self, other: &Self) {
        for (d, n) in &other.0 {
            let c = self.0.entry(*d).or_default();
            *c = (*c).max(*n);
        }
    }
}

/// The largest value any computer reported.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Maximum(pub u64);

impl Maximum {
    /// Raises it to `v` if `v` is larger.
    pub fn raise(&mut self, v: u64) {
        self.0 = self.0.max(v);
    }

    /// Merges `other` in.
    pub fn merge(&mut self, other: &Self) {
        self.raise(other.0);
    }
}

/// The earliest value any computer reported (when a document was first
/// added to a library), or none yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Earliest(pub Option<u64>);

impl Earliest {
    /// Nothing reported yet.
    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }

    /// Lowers it to `v` if `v` is earlier (or nothing was reported).
    pub fn lower(&mut self, v: u64) {
        self.0 = Some(self.0.map_or(v, |w| w.min(v)));
    }

    /// Merges `other` in.
    pub fn merge(&mut self, other: &Self) {
        if let Some(v) = other.0 {
            self.lower(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: DeviceId = DeviceId::from_u128(1);
    const B: DeviceId = DeviceId::from_u128(2);

    fn s(t: u64, d: DeviceId) -> Stamp {
        Stamp::new(t, d)
    }

    #[test]
    fn the_newest_edit_wins_and_the_change_is_listed() {
        let mut here = RegisterMap::new();
        here.set("n1", s(10, A), "old text".to_owned());
        let mut there = RegisterMap::new();
        there.set("n1", s(20, B), "new text".to_owned());
        let changes = here.merge(&there);
        assert_eq!(here.get("n1").map(String::as_str), Some("new text"));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].kind, ChangeKind::Replaced);
        assert_eq!(changes[0].by, B);
        assert_eq!(changes[0].previous.as_deref(), Some("old text"));
        // Merging again changes nothing and lists nothing.
        assert!(here.merge(&there).is_empty());
    }

    #[test]
    fn an_older_edit_loses() {
        let mut here = RegisterMap::new();
        here.set("n1", s(20, A), "mine".to_owned());
        let mut there = RegisterMap::new();
        there.set("n1", s(10, B), "theirs".to_owned());
        assert!(here.merge(&there).is_empty());
        assert_eq!(here.get("n1").map(String::as_str), Some("mine"));
    }

    #[test]
    fn a_deletion_loses_to_a_later_edit_and_wins_over_an_earlier_one() {
        let mut here: RegisterMap<String> = RegisterMap::new();
        here.delete("n1", s(10, A));
        let mut there = RegisterMap::new();
        there.set("n1", s(20, B), "edited later".to_owned());
        let changes = here.merge(&there);
        assert_eq!(changes[0].kind, ChangeKind::Restored);
        assert_eq!(here.get("n1").map(String::as_str), Some("edited later"));

        let mut later: RegisterMap<String> = RegisterMap::new();
        later.delete("n1", s(30, A));
        let changes = here.merge(&later);
        assert_eq!(changes[0].kind, ChangeKind::Removed);
        assert_eq!(changes[0].previous.as_deref(), Some("edited later"));
        assert!(here.get("n1").is_none());
        assert!(here.register("n1").is_some(), "the deletion record is kept");
    }

    #[test]
    fn equal_stamps_pick_the_same_winner_either_way() {
        let a = Register::set(s(5, A), "x".to_owned());
        let b = Register::set(s(5, A), "y".to_owned());
        let d: Register<String> = Register::deleted(s(5, A));
        assert!(b.wins_over(&a) && !a.wins_over(&b));
        assert!(a.wins_over(&d) && !d.wins_over(&a));
        assert!(!d.wins_over(&d.clone()));
    }

    #[test]
    fn adding_wins_in_the_set() {
        let mut here = AddWinsSet::new();
        here.insert("physics", s(10, A));
        let mut there = AddWinsSet::new();
        there.remove("physics", s(10, A));
        here.merge(&there);
        assert!(here.contains("physics"), "a tie goes to the add");
        there.remove("physics", s(11, B));
        here.merge(&there);
        assert!(!here.contains("physics"));
        here.insert("physics", s(12, A));
        assert_eq!(here.items().collect::<Vec<_>>(), ["physics"]);
    }

    #[test]
    fn counters_sum_each_computers_own_count() {
        let mut here = Counter::new();
        here.add(A, 30);
        let mut there = Counter::new();
        there.add(B, 12);
        there.add(A, 5); // an old copy of A's count
        here.merge(&there);
        assert_eq!(here.of(A), 30);
        assert_eq!(here.total(), 42);
        let mut m = Maximum::default();
        m.merge(&Maximum(40));
        m.raise(12);
        assert_eq!(m.0, 40);
        let mut e = Earliest::default();
        e.merge(&Earliest(Some(30)));
        e.merge(&Earliest(None));
        e.lower(40);
        assert_eq!(e.0, Some(30));
        let mut f = Earliest(Some(20));
        f.merge(&e);
        assert_eq!(f.0, Some(20), "either order gives the earliest");
    }
}
