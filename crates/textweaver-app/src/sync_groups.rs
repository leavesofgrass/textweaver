//! Sync of what is not about one document (the sync wave, S5; ADR-0049):
//! portable settings, profiles, key overrides, the word list, the glossary
//! and pronunciations, and favorite voices.
//!
//! Each group is one file per computer in the sync folder
//! ([`textweaver_sync::groups`]), holding this computer's merged view. A
//! merge (a *cycle*) of one group, on the background writer:
//!
//! 1. **Local edits.** What this computer has now ([`LocalGroup`]: the
//!    settings as the app holds them, and the profiles, keys, word list,
//!    and glossary files) is compared with the merged view: a key whose
//!    value differs, a key gone, an item added to or removed from a set.
//!    An arrival sent to the app and not applied yet is not an edit. On the
//!    first merge, a setting still at its default is not published, so a
//!    new computer never overwrites the others with defaults.
//! 2. **Arrivals.** Every other computer's file is merged in.
//! 3. **Both at once.** When the same key changed here and elsewhere, the
//!    newer change wins: here, the time the file (or the settings) was last
//!    saved.
//! 4. The merged view is written, only when it changed. What differs
//!    between it and this computer goes back to the app as
//!    [`GroupArrival`]s, each with the value the app had, so the app applies
//!    it only if nothing changed meanwhile.
//!
//! Applying an arrival makes this computer match the merged view, so the
//! next merge finds no edit and writes nothing: no echo. Machine settings
//! are never read or written here ([`textweaver_store::sync_scope`]).
//!
//! Key overrides are labeled with the system that made them. Windows and
//! Linux share key names; a Mac's are its own. An override from the other
//! kind of system is kept in the merged view and passed on, but never
//! applied here.
//!
//! Favorite voices are a set; a favorite not installed on this computer is
//! kept, and listed in Choose voice as "not on this computer".
//!
//! Profiles publish only their portable settings: the voice, engine,
//! access mode, and other machine settings a profile was saved with stay
//! on this computer, and are kept when another computer's version of the
//! profile arrives.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde_json::{Value, json};
use textweaver_store::settings_io::{json_to_toml, toml_to_json};
use textweaver_store::sync_scope::{
    portable_settings, profile_portable, profile_with_machine, sync_group_of, with_portable,
};
use textweaver_store::{Paths, Profiles, Settings, SettingsStore};
use textweaver_sync::{DeviceId, GroupFile, GroupRecord};

use crate::sync_engine::{EngineStatus, Groups, Notice, SyncEngine};
use crate::sync_pending::PendingItem;

/// The settings group's map.
pub const SETTINGS_MAP: &str = "settings";
/// The profiles group's map.
pub const PROFILES_MAP: &str = "profiles";
/// The key overrides group's map.
pub const KEYMAP_MAP: &str = "keymap";
/// The word list's set.
pub const WORDS_SET: &str = "words";
/// The glossary's map.
pub const GLOSSARY_MAP: &str = "glossary";
/// The pronunciations' map, in the glossary group.
pub const PRONUNCIATIONS_MAP: &str = "pronunciations";
/// The favorite voices' set.
pub const VOICES_SET: &str = "voices";

/// The personal word list's file name, in the data folder.
pub const WORDS_FILE: &str = "words.txt";

/// The system a key override was made on. Windows and Linux name keys the
/// same way; a Mac's keys are its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeySystem {
    /// Windows.
    Windows,
    /// macOS.
    MacOs,
    /// Linux and other Unix.
    Linux,
}

impl KeySystem {
    /// The system this textweaver was built for.
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            KeySystem::Windows
        } else if cfg!(target_os = "macos") {
            KeySystem::MacOs
        } else {
            KeySystem::Linux
        }
    }

    /// The name written with an override: `windows`, `macos`, or `linux`.
    pub fn name(self) -> &'static str {
        match self {
            KeySystem::Windows => "windows",
            KeySystem::MacOs => "macos",
            KeySystem::Linux => "linux",
        }
    }

    /// The kind of keys: `mac`, or `pc` for Windows and Linux. An override
    /// applies only on a system of the same kind.
    pub fn family(self) -> &'static str {
        match self {
            KeySystem::MacOs => "mac",
            KeySystem::Windows | KeySystem::Linux => "pc",
        }
    }
}

/// Whether `groups` has the switch for `file` on.
pub fn group_on(groups: Groups, file: GroupFile) -> bool {
    match file {
        GroupFile::Settings => groups.settings,
        GroupFile::Profiles => groups.profiles,
        GroupFile::Keymap => groups.key_overrides,
        GroupFile::Words => groups.words,
        GroupFile::Glossary => groups.glossary,
        GroupFile::Voices => groups.favorite_voices,
    }
}

/// Whether this computer has a say in `key` of map `name`: the settings
/// portable here, the overrides of this kind of system, and everything
/// else.
fn in_scope(name: &str, key: &str, system: KeySystem) -> bool {
    match name {
        SETTINGS_MAP => sync_group_of(key) == Some("settings"),
        KEYMAP_MAP => key
            .split_once(':')
            .is_some_and(|(family, _)| family == system.family()),
        _ => true,
    }
}

/// Whether two values of map `name` say the same thing. A key override's
/// system label is not part of what it says, so two systems of one kind
/// with the same keys never pass the key back and forth.
fn same(name: &str, a: Option<&Value>, b: Option<&Value>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) if name == KEYMAP_MAP => a.get("chords") == b.get("chords"),
        (a, b) => a == b,
    }
}

/// What this computer has now for one group.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LocalGroup {
    /// Each map's values by key.
    pub maps: BTreeMap<&'static str, BTreeMap<String, Value>>,
    /// Each set's items.
    pub sets: BTreeMap<&'static str, BTreeSet<String>>,
    /// When each map or set was last saved here, in milliseconds.
    pub wall: BTreeMap<&'static str, u64>,
    /// Values not published while the merged view has none: settings still
    /// at their defaults.
    pub defaults: BTreeMap<String, Value>,
}

fn mtime_ms(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// Reads a text file; a missing one is empty, one that cannot be read is
/// `None` (so a group is skipped, never taken for emptied).
fn read_text(path: &Path) -> Option<String> {
    match std::fs::read(path) {
        Ok(b) => Some(
            String::from_utf8_lossy(&b)
                .trim_start_matches('\u{feff}')
                .to_owned(),
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(String::new()),
        Err(e) => {
            log::warn!("sync: cannot read a file to sync ({e})");
            None
        }
    }
}

/// The personal word list's words, as the spell checker reads them.
pub fn parse_words(text: &str) -> BTreeSet<String> {
    text.lines()
        .map(|l| l.trim().to_lowercase())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
}

/// The glossary file in use: `[lexicon] glossary`, else `glossary.txt` or
/// `glossary.json` in the configuration folder, else where a new one goes.
pub fn glossary_path(settings: &Settings, paths: &Paths) -> PathBuf {
    settings
        .lexicon
        .glossary
        .clone()
        .or_else(|| paths.default_glossary())
        .unwrap_or_else(|| paths.config_dir.join("glossary.txt"))
}

fn is_json(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("json"))
}

/// What this computer has now for `file`: the settings as the app holds
/// them (`settings`, last saved at `settings_ms`), and the files. `None`
/// when a file cannot be read, so the group waits for the next merge
/// instead of taking a damaged file for an emptied one.
pub fn read_local(
    file: GroupFile,
    paths: &Paths,
    settings: &Settings,
    settings_ms: u64,
    system: KeySystem,
) -> Option<LocalGroup> {
    let mut g = LocalGroup::default();
    match file {
        GroupFile::Settings => {
            g.maps.insert(SETTINGS_MAP, portable_settings(settings));
            g.wall.insert(SETTINGS_MAP, settings_ms);
            g.defaults = portable_settings(&Settings::default());
        }
        GroupFile::Profiles => {
            let profiles = match Profiles::load(paths) {
                Ok(p) => p,
                Err(e) => {
                    log::warn!("sync: profiles not synced ({e})");
                    return None;
                }
            };
            let map = profiles
                .profiles
                .iter()
                .map(|(name, t)| {
                    let full = toml_to_json(&toml::Value::Table(t.clone()));
                    (name.clone(), profile_portable(&full))
                })
                .collect();
            g.maps.insert(PROFILES_MAP, map);
            g.wall
                .insert(PROFILES_MAP, mtime_ms(&paths.profiles_file()));
        }
        GroupFile::Keymap => {
            let keys = match SettingsStore::new(paths.clone()).load_keymap() {
                Ok(k) => k,
                Err(e) => {
                    log::warn!("sync: key overrides not synced ({e})");
                    return None;
                }
            };
            let map = keys
                .iter()
                .map(|(action, chords)| {
                    (
                        format!("{}:{action}", system.family()),
                        json!({ "system": system.name(), "chords": chords }),
                    )
                })
                .collect();
            g.maps.insert(KEYMAP_MAP, map);
            g.wall.insert(KEYMAP_MAP, mtime_ms(&paths.keymap_file()));
        }
        GroupFile::Words => {
            let path = paths.data_dir.join(WORDS_FILE);
            g.sets.insert(WORDS_SET, parse_words(&read_text(&path)?));
            g.wall.insert(WORDS_SET, mtime_ms(&path));
        }
        GroupFile::Glossary => {
            let path = glossary_path(settings, paths);
            let entries = glossary_entries(&read_text(&path)?, is_json(&path))?;
            g.maps.insert(GLOSSARY_MAP, entries);
            g.wall.insert(GLOSSARY_MAP, mtime_ms(&path));
            let pron = settings
                .normalization
                .pronunciations
                .iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect();
            g.maps.insert(PRONUNCIATIONS_MAP, pron);
            g.wall.insert(PRONUNCIATIONS_MAP, settings_ms);
        }
        GroupFile::Voices => {
            g.sets.insert(
                VOICES_SET,
                settings.speech.favorite_voices.iter().cloned().collect(),
            );
            g.wall.insert(VOICES_SET, settings_ms);
        }
    }
    Some(g)
}

// ----- The glossary file ---------------------------------------------------

/// Splits a text glossary line into its term and definition, by the
/// glossary's own rules (`term: definition`, `term = definition`, `term -
/// definition`), or `None` for a comment or a line that is not an entry.
fn split_entry(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let line = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .unwrap_or(line);
    let (i, len) = [": ", " = ", " - ", " \u{2013} ", " \u{2014} ", ":\t", "\t"]
        .iter()
        .filter_map(|sep| line.find(sep).map(|i| (i, sep.len())))
        .min_by_key(|(i, _)| *i)?;
    let term = line[..i].trim().trim_matches(['*', '_']).trim();
    let definition = line[i + len..].trim();
    (!term.is_empty() && !definition.is_empty()).then(|| (term.to_owned(), definition.to_owned()))
}

/// A glossary file's entries by normalized term: `{"term", "senses"}`,
/// each sense a definition, or Star's JSON object for a JSON glossary.
/// `None` when a JSON glossary is not an object.
pub fn glossary_entries(text: &str, json: bool) -> Option<BTreeMap<String, Value>> {
    let mut out: BTreeMap<String, (String, Vec<Value>)> = BTreeMap::new();
    if json {
        if text.trim().is_empty() {
            return Some(BTreeMap::new());
        }
        let Ok(Value::Object(map)) = serde_json::from_str::<Value>(text) else {
            log::warn!("sync: the glossary is not a JSON object; not synced");
            return None;
        };
        for (term, v) in map {
            let senses = match v {
                Value::Array(a) => a,
                other => vec![other],
            };
            let key = textweaver_lexicon::normalize(&term);
            if !key.is_empty() {
                out.entry(key)
                    .or_insert_with(|| (term.clone(), Vec::new()))
                    .1
                    .extend(senses);
            }
        }
    } else {
        for line in text.lines() {
            let Some((term, def)) = split_entry(line) else {
                continue;
            };
            let key = textweaver_lexicon::normalize(&term);
            if !key.is_empty() {
                out.entry(key)
                    .or_insert_with(|| (term.clone(), Vec::new()))
                    .1
                    .push(Value::String(def));
            }
        }
    }
    Some(
        out.into_iter()
            .map(|(k, (term, senses))| (k, json!({ "term": term, "senses": senses })))
            .collect(),
    )
}

/// A sense as a definition line's text.
fn sense_text(sense: &Value) -> Option<String> {
    match sense {
        Value::String(s) => Some(s.clone()),
        Value::Object(o) => o
            .get("definition")
            .and_then(Value::as_str)
            .map(str::to_owned),
        _ => None,
    }
}

/// The glossary file's text with `changes` made (normalized term to the new
/// entry, or `None` to remove it). A text glossary keeps its comments and
/// the order of its other lines; a changed term's lines are replaced where
/// the term first was, and new terms go at the end.
pub fn write_glossary(text: &str, json: bool, changes: &BTreeMap<String, Option<Value>>) -> String {
    let term_of = |v: &Value, key: &str| {
        v.get("term")
            .and_then(Value::as_str)
            .map_or_else(|| key.to_owned(), str::to_owned)
    };
    let senses_of = |v: &Value| {
        v.get("senses")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    if json {
        let mut map = match serde_json::from_str::<Value>(text) {
            Ok(Value::Object(m)) => m,
            _ => serde_json::Map::new(),
        };
        for (key, entry) in changes {
            map.retain(|term, _| textweaver_lexicon::normalize(term) != *key);
            if let Some(v) = entry {
                let mut senses = senses_of(v);
                let value = if senses.len() == 1 {
                    senses.remove(0)
                } else {
                    Value::Array(senses)
                };
                map.insert(term_of(v, key), value);
            }
        }
        let mut out = serde_json::to_string_pretty(&Value::Object(map)).unwrap_or_default();
        out.push('\n');
        return out;
    }
    let lines_for = |key: &str, v: &Value| -> Vec<String> {
        let term = term_of(v, key);
        senses_of(v)
            .iter()
            .filter_map(sense_text)
            .map(|d| format!("{term}: {d}"))
            .collect()
    };
    let mut out: Vec<String> = Vec::new();
    let mut done: BTreeSet<&str> = BTreeSet::new();
    for line in text.lines() {
        let key = split_entry(line).map(|(t, _)| textweaver_lexicon::normalize(&t));
        match key.as_deref().and_then(|k| changes.get_key_value(k)) {
            Some((k, entry)) => {
                if done.insert(k.as_str())
                    && let Some(v) = entry
                {
                    out.extend(lines_for(k, v));
                }
            }
            None => out.push(line.to_owned()),
        }
    }
    for (k, entry) in changes {
        if !done.contains(k.as_str())
            && let Some(v) = entry
        {
            out.extend(lines_for(k, v));
        }
    }
    let mut text = out.join("\n");
    text.push('\n');
    text
}

// ----- The engine's side --------------------------------------------------

/// One change another computer made to a group, for the app to apply.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupArrival {
    /// The group's file.
    pub file: GroupFile,
    /// The map or set.
    pub name: String,
    /// The key or item.
    pub key: String,
    /// The value now: `None` when removed. For a set, `true` when added.
    pub value: Option<Value>,
    /// What the app had when the merge began (`None` when it had none; for
    /// a set, `true` when it had the item): the app applies the change
    /// only if it still has this.
    pub previous: Option<Value>,
    /// The computer whose change it is, by its name, when known.
    pub label: Option<String>,
}

/// What a merge of the groups did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GroupsOutcome {
    /// What arrived, to apply.
    pub arrivals: Vec<GroupArrival>,
    /// Things to say once.
    pub notices: Vec<Notice>,
    /// How sync stands.
    pub status: EngineStatus,
}

/// Work for the groups: the settings as the app holds them.
#[derive(Clone, Debug)]
pub struct GroupsRequest {
    /// The app's settings.
    pub settings: Box<Settings>,
    /// When they were last saved, in milliseconds since 1970.
    pub settings_ms: u64,
    /// Merge even when nothing seems changed.
    pub force: bool,
}

/// One group's merged view, as this session knows it.
#[derive(Debug, Default)]
pub(crate) struct GroupSlot {
    /// This computer's merged view: the base.
    mine: GroupRecord,
    /// What was last written (or read) for it.
    written: Option<Vec<u8>>,
    /// The other computers' files when last merged.
    others: Vec<(DeviceId, u64, Option<SystemTime>)>,
    /// What this computer had when last merged.
    last_local: Option<LocalGroup>,
    /// Arrivals sent and not seen applied: map or set, key, and what the
    /// app had.
    pending: BTreeMap<(String, String), Option<Value>>,
}

/// A local edit.
#[derive(Debug)]
enum Edit {
    Set(Value),
    Delete,
    Add,
    Remove,
}

impl SyncEngine {
    /// Merges every group that is on: publishes what changed here, takes
    /// what changed elsewhere.
    pub fn groups_cycle(&mut self, request: &GroupsRequest) -> GroupsOutcome {
        let mut notices = Vec::new();
        let mut out = GroupsOutcome::default();
        if !self.ready(&mut notices) {
            out.notices = notices;
            out.status = self.status.clone();
            return out;
        }
        let Some(config) = self.config.clone() else {
            return out;
        };
        self.refresh_labels();
        for file in GroupFile::ALL {
            if !group_on(config.groups, file) {
                continue;
            }
            let Some(local) = read_local(
                file,
                &config.paths,
                &request.settings,
                request.settings_ms,
                config.system,
            ) else {
                continue;
            };
            out.arrivals.extend(self.group_cycle(
                file,
                &local,
                config.system,
                request.force,
                &mut notices,
            ));
        }
        self.count_kept_keys(config.system);
        out.notices = notices;
        out.status = self.status.clone();
        out
    }

    /// The overrides of the other kind of system in the merged view, for
    /// the status.
    fn count_kept_keys(&mut self, system: KeySystem) {
        self.status.kept_key_overrides = self
            .groups
            .get(&GroupFile::Keymap)
            .and_then(|s| s.mine.map(KEYMAP_MAP))
            .map_or(0, |m| {
                m.live()
                    .filter(|(k, _)| !in_scope(KEYMAP_MAP, k, system))
                    .count()
            });
    }

    /// The slot for `file`, read from this computer's own file the first
    /// time.
    fn group_slot(&mut self, file: GroupFile) -> Option<&mut GroupSlot> {
        if !self.groups.contains_key(&file) {
            let folder = self.folder.as_mut()?;
            let me = folder.device();
            let (records, _) = folder.read_group(file);
            let mine = records
                .into_iter()
                .find(|(d, _)| *d == me)
                .map(|(_, r)| r)
                .unwrap_or_default();
            if let Some(clock) = self.clock.as_mut() {
                for s in mine.stamps() {
                    clock.observe(s);
                }
            }
            let written = mine.to_bytes().ok();
            // Arrivals a session before this one sent and the app may not
            // have applied (it stopped first): saved before publishing.
            let pending = self.journal.as_ref().map_or_else(BTreeMap::new, |j| {
                j.group(file)
                    .into_iter()
                    .map(|p| ((p.kind.clone(), p.id.clone()), p.before()))
                    .collect()
            });
            self.groups.insert(
                file,
                GroupSlot {
                    mine,
                    written,
                    pending,
                    ..GroupSlot::default()
                },
            );
        }
        self.groups.get_mut(&file)
    }

    fn group_cycle(
        &mut self,
        file: GroupFile,
        local: &LocalGroup,
        system: KeySystem,
        force: bool,
        notices: &mut Vec<Notice>,
    ) -> Vec<GroupArrival> {
        let Some(folder) = self.folder.as_ref() else {
            return Vec::new();
        };
        let me = folder.device();
        let others = folder.group_signature(file);
        let Some(slot) = self.group_slot(file) else {
            return Vec::new();
        };
        if !force && slot.others == others && slot.last_local.as_ref() == Some(local) {
            return Vec::new();
        }
        let edits = local_edits(local, &slot.mine, &slot.pending, system);

        let (Some(folder), Some(clock), Some(slot)) = (
            self.folder.as_mut(),
            self.clock.as_mut(),
            self.groups.get_mut(&file),
        ) else {
            return Vec::new();
        };
        let (changes, problems) = folder.merge_group(file, &mut slot.mine, clock);
        for (name, key, edit, wall) in edits {
            let remote = changes
                .iter()
                .any(|c| c.name == name && c.key == key && c.by != me);
            if remote && wall <= time_of(&slot.mine, &name, &key) {
                continue;
            }
            let stamp = clock.tick();
            match edit {
                Edit::Set(v) => slot.mine.map_mut(&name).set(key, stamp, v),
                Edit::Delete => slot.mine.map_mut(&name).delete(key, stamp),
                Edit::Add => slot.mine.set_mut(&name).insert(key, stamp),
                Edit::Remove => slot.mine.set_mut(&name).remove(key, stamp),
            }
        }
        let arrivals = arrivals(file, local, &slot.mine, system, &self.labels);
        slot.pending = arrivals
            .iter()
            .map(|a| ((a.name.clone(), a.key.clone()), a.previous.clone()))
            .collect();
        slot.others = others;
        slot.last_local = Some(local.clone());
        let saved = slot
            .pending
            .iter()
            .map(|((name, key), before)| PendingItem::new(name, key, before.as_ref()))
            .collect();
        self.problems(problems, notices);
        // What arrived is saved here before the merged view is published.
        if self
            .journal
            .as_mut()
            .is_none_or(|j| j.set_group(file, saved))
        {
            self.write_group(file, notices);
        }
        arrivals
    }

    /// Writes this computer's view of `file` when it changed.
    fn write_group(&mut self, file: GroupFile, notices: &mut Vec<Notice>) {
        let (Some(folder), Some(slot)) = (&self.folder, self.groups.get_mut(&file)) else {
            return;
        };
        if folder.read_only().is_some() {
            return;
        }
        let Ok(bytes) = slot.mine.to_bytes() else {
            return;
        };
        if slot.written.as_ref() == Some(&bytes) {
            return;
        }
        match folder.write_group(file, &slot.mine) {
            Ok(()) => slot.written = Some(bytes),
            Err(e) => {
                let why = e.to_string();
                log::warn!("sync: cannot write ({why})");
                self.status.write_error = Some(why.clone());
                self.notice(notices, &format!("write:{why}"), Notice::WriteFailed(why));
            }
        }
    }
}

/// When `key` of map or set `name` last changed in `record` (0 when never).
fn time_of(record: &GroupRecord, name: &str, key: &str) -> u64 {
    if let Some(r) = record.map(name).and_then(|m| m.register(key)) {
        return r.stamp.time;
    }
    record
        .set(name)
        .and_then(|s| s.0.get(key))
        .map_or(0, |e| e.added.max(e.removed).map_or(0, |s| s.time))
}

/// The edits made here since the base: map name, key, what, and when.
fn local_edits(
    local: &LocalGroup,
    base: &GroupRecord,
    pending: &BTreeMap<(String, String), Option<Value>>,
    system: KeySystem,
) -> Vec<(String, String, Edit, u64)> {
    let mut out = Vec::new();
    let unapplied = |name: &str, key: &str, now: Option<&Value>| {
        pending
            .get(&(name.to_owned(), key.to_owned()))
            .is_some_and(|before| same(name, before.as_ref(), now))
    };
    for (name, values) in &local.maps {
        let wall = local.wall.get(name).copied().unwrap_or(0);
        let base_map = base.map(name);
        for (key, value) in values {
            if !in_scope(name, key, system) {
                continue;
            }
            let register = base_map.and_then(|m| m.register(key));
            let changed = match register {
                None => local.defaults.get(key) != Some(value),
                Some(r) => match &r.value {
                    Some(v) => !same(name, Some(v), Some(value)),
                    // Deleted in the base: only a change made after the
                    // deletion brings it back.
                    None => wall > r.stamp.time,
                },
            };
            if changed && !unapplied(name, key, Some(value)) {
                out.push((
                    (*name).to_owned(),
                    key.clone(),
                    Edit::Set(value.clone()),
                    wall,
                ));
            }
        }
        for (key, _) in base_map.into_iter().flat_map(|m| m.live()) {
            if !in_scope(name, key, system) || values.contains_key(key) {
                continue;
            }
            if !unapplied(name, key, None) {
                out.push(((*name).to_owned(), key.to_owned(), Edit::Delete, wall));
            }
        }
    }
    for (name, items) in &local.sets {
        let wall = local.wall.get(name).copied().unwrap_or(0);
        let base_set = base.set(name);
        let had = |present: bool| Some(Value::Bool(present));
        for item in items {
            let entry = base_set.and_then(|s| s.0.get(item));
            let present = entry.is_some_and(|e| e.present());
            if present {
                continue;
            }
            // Removed in the base: only an add made after the removal
            // brings it back.
            let after = entry.and_then(|e| e.removed).is_none_or(|r| wall > r.time);
            if after && !unapplied(name, item, had(true).as_ref()) {
                out.push(((*name).to_owned(), item.clone(), Edit::Add, wall));
            }
        }
        for item in base_set.into_iter().flat_map(|s| s.items()) {
            if items.contains(item) {
                continue;
            }
            if !unapplied(name, item, None) {
                out.push(((*name).to_owned(), item.to_owned(), Edit::Remove, wall));
            }
        }
    }
    out
}

/// What differs between the merged view and this computer, for the app to
/// apply: every key this computer has a say in whose merged value is not
/// what it has. A key the merged view has never held (a default not
/// published) is left alone.
fn arrivals(
    file: GroupFile,
    local: &LocalGroup,
    base: &GroupRecord,
    system: KeySystem,
    labels: &BTreeMap<DeviceId, String>,
) -> Vec<GroupArrival> {
    let mut out = Vec::new();
    let empty = BTreeMap::new();
    for (name, map) in &base.maps {
        let here = local.maps.get(name.as_str()).unwrap_or(&empty);
        for (key, r) in &map.0 {
            if !in_scope(name, key, system) {
                continue;
            }
            // A profile published with machine settings (by an earlier
            // build) arrives without them.
            let value = match (name.as_str(), &r.value) {
                (PROFILES_MAP, Some(v)) => Some(profile_portable(v)),
                (_, v) => v.clone(),
            };
            let now = here.get(key);
            if same(name, value.as_ref(), now) {
                continue;
            }
            out.push(GroupArrival {
                file,
                name: name.clone(),
                key: key.clone(),
                value,
                previous: now.cloned(),
                label: labels.get(&r.stamp.device).cloned(),
            });
        }
    }
    let empty_set = BTreeSet::new();
    for (name, set) in &base.sets {
        let here = local.sets.get(name.as_str()).unwrap_or(&empty_set);
        for (item, e) in &set.0 {
            let present = e.present();
            let had = here.contains(item);
            if present == had {
                continue;
            }
            let stamp = if present { e.added } else { e.removed };
            out.push(GroupArrival {
                file,
                name: name.clone(),
                key: item.clone(),
                value: present.then_some(Value::Bool(true)),
                previous: had.then_some(Value::Bool(true)),
                label: stamp.and_then(|s| labels.get(&s.device).cloned()),
            });
        }
    }
    out
}

// ----- Applying arrivals --------------------------------------------------

/// What applying group arrivals did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GroupsApplied {
    /// Changes made, all groups together.
    pub changes: usize,
    /// The computers they came from, by name, in the order first seen.
    pub from: Vec<String>,
    /// The settings paths whose value changed (pronunciations and favorite
    /// voices included), to put into effect.
    pub settings_paths: Vec<String>,
    /// The key overrides changed: the keys must be built again.
    pub keys_changed: bool,
    /// The word list changed: read it again.
    pub words_changed: bool,
    /// Arrivals not applied because the value changed here meanwhile.
    pub skipped: usize,
}

impl GroupsApplied {
    fn count(&mut self, label: Option<&String>) {
        self.changes += 1;
        if let Some(l) = label
            && !self.from.contains(l)
        {
            self.from.push(l.clone());
        }
    }
}

/// Applies group arrivals: settings in `settings` (the caller saves them
/// and puts them into effect), the profiles, keys, word list, and glossary
/// in their files. An arrival whose value changed here since the merge
/// began is skipped; the next merge settles it.
pub fn apply_groups(
    paths: &Paths,
    settings: &mut Settings,
    arrivals: &[GroupArrival],
    system: KeySystem,
) -> GroupsApplied {
    let mut out = GroupsApplied::default();
    let by = |file: GroupFile| arrivals.iter().filter(move |a| a.file == file);

    // Portable settings.
    let current = portable_settings(settings);
    let mut values = BTreeMap::new();
    for a in by(GroupFile::Settings) {
        if a.name != SETTINGS_MAP || current.get(&a.key) != a.previous.as_ref() {
            out.skipped += 1;
            continue;
        }
        if let Some(v) = &a.value {
            values.insert(a.key.clone(), v.clone());
            out.count(a.label.as_ref());
        }
    }
    if !values.is_empty() {
        let (new, changed) = with_portable(settings, &values);
        *settings = new;
        out.settings_paths.extend(changed);
    }

    // Pronunciations and favorite voices, in the settings too.
    for a in by(GroupFile::Glossary).filter(|a| a.name == PRONUNCIATIONS_MAP) {
        let prons = &mut settings.normalization.pronunciations;
        let now = prons.get(&a.key).map(|v| Value::String(v.clone()));
        if now != a.previous {
            out.skipped += 1;
            continue;
        }
        match a.value.as_ref().and_then(Value::as_str) {
            Some(v) => {
                prons.insert(a.key.clone(), v.to_owned());
            }
            None => {
                prons.remove(&a.key);
            }
        }
        out.count(a.label.as_ref());
        push_once(&mut out.settings_paths, "normalization.pronunciations");
    }
    for a in by(GroupFile::Voices) {
        let favs = &mut settings.speech.favorite_voices;
        let had = favs.contains(&a.key);
        if had != a.previous.is_some() {
            out.skipped += 1;
            continue;
        }
        if a.value.is_some() {
            favs.push(a.key.clone());
        } else {
            favs.retain(|f| *f != a.key);
        }
        out.count(a.label.as_ref());
        push_once(&mut out.settings_paths, "speech.favorite_voices");
    }

    apply_profiles(paths, by(GroupFile::Profiles), &mut out);
    apply_keys(paths, by(GroupFile::Keymap), system, &mut out);
    apply_words(paths, by(GroupFile::Words), &mut out);
    let glossary: Vec<&GroupArrival> = by(GroupFile::Glossary)
        .filter(|a| a.name == GLOSSARY_MAP)
        .collect();
    apply_glossary(&glossary_path(settings, paths), &glossary, &mut out);
    out
}

fn push_once(v: &mut Vec<String>, s: &str) {
    if !v.iter().any(|x| x == s) {
        v.push(s.to_owned());
    }
}

fn apply_profiles<'a>(
    paths: &Paths,
    arrivals: impl Iterator<Item = &'a GroupArrival>,
    out: &mut GroupsApplied,
) {
    let arrivals: Vec<&GroupArrival> = arrivals.collect();
    if arrivals.is_empty() {
        return;
    }
    let Ok(mut profiles) = Profiles::load(paths) else {
        out.skipped += arrivals.len();
        return;
    };
    let mut changed = false;
    for a in arrivals {
        let full = profiles
            .profiles
            .get(&a.key)
            .map(|t| toml_to_json(&toml::Value::Table(t.clone())));
        if full.as_ref().map(profile_portable) != a.previous {
            out.skipped += 1;
            continue;
        }
        match &a.value {
            // This computer's voice, engine, and access mode for the
            // profile stay.
            Some(v) => match json_to_toml(&profile_with_machine(v, full.as_ref()), &a.key) {
                Ok(Some(toml::Value::Table(t))) => {
                    profiles.profiles.insert(a.key.clone(), t);
                }
                _ => {
                    out.skipped += 1;
                    continue;
                }
            },
            None => {
                profiles.profiles.remove(&a.key);
                // Which profile is active is this computer's own; a
                // profile that no longer exists is not active.
                if profiles.active.as_deref() == Some(a.key.as_str()) {
                    profiles.active = None;
                }
            }
        }
        changed = true;
        out.count(a.label.as_ref());
    }
    if changed && let Err(e) = profiles.save(paths) {
        log::warn!("sync: cannot save profiles ({e})");
    }
}

fn apply_keys<'a>(
    paths: &Paths,
    arrivals: impl Iterator<Item = &'a GroupArrival>,
    system: KeySystem,
    out: &mut GroupsApplied,
) {
    let arrivals: Vec<&GroupArrival> = arrivals
        .filter(|a| in_scope(KEYMAP_MAP, &a.key, system))
        .collect();
    if arrivals.is_empty() {
        return;
    }
    let store = SettingsStore::new(paths.clone());
    let Ok(mut keys) = store.load_keymap() else {
        out.skipped += arrivals.len();
        return;
    };
    let mut changed = false;
    for a in arrivals {
        let Some((_, action)) = a.key.split_once(':') else {
            continue;
        };
        let now = keys
            .get(action)
            .map(|c| json!({ "system": system.name(), "chords": c }));
        if !same(KEYMAP_MAP, now.as_ref(), a.previous.as_ref()) {
            out.skipped += 1;
            continue;
        }
        match a
            .value
            .as_ref()
            .and_then(|v| v.get("chords"))
            .and_then(|c| serde_json::from_value::<Vec<String>>(c.clone()).ok())
        {
            Some(chords) => {
                keys.insert(action.to_owned(), chords);
            }
            None if a.value.is_none() => {
                keys.remove(action);
            }
            None => {
                out.skipped += 1;
                continue;
            }
        }
        changed = true;
        out.count(a.label.as_ref());
    }
    if changed {
        match store.save_keymap(&keys) {
            Ok(()) => out.keys_changed = true,
            Err(e) => log::warn!("sync: cannot save key overrides ({e})"),
        }
    }
}

fn apply_words<'a>(
    paths: &Paths,
    arrivals: impl Iterator<Item = &'a GroupArrival>,
    out: &mut GroupsApplied,
) {
    let arrivals: Vec<&GroupArrival> = arrivals.collect();
    if arrivals.is_empty() {
        return;
    }
    let path = paths.data_dir.join(WORDS_FILE);
    let Some(text) = read_text(&path) else {
        out.skipped += arrivals.len();
        return;
    };
    let mut words = parse_words(&text);
    let mut changed = false;
    for a in arrivals {
        if words.contains(&a.key) != a.previous.is_some() {
            out.skipped += 1;
            continue;
        }
        if a.value.is_some() {
            words.insert(a.key.clone());
        } else {
            words.remove(&a.key);
        }
        changed = true;
        out.count(a.label.as_ref());
    }
    if changed {
        let body: String = words.iter().map(|w| format!("{w}\n")).collect();
        match textweaver_store::atomic_write(&path, body.as_bytes()) {
            Ok(()) => out.words_changed = true,
            Err(e) => log::warn!("sync: cannot save the word list ({e})"),
        }
    }
}

fn apply_glossary(path: &Path, arrivals: &[&GroupArrival], out: &mut GroupsApplied) {
    if arrivals.is_empty() {
        return;
    }
    let json = is_json(path);
    let Some(text) = read_text(path) else {
        out.skipped += arrivals.len();
        return;
    };
    let Some(now) = glossary_entries(&text, json) else {
        out.skipped += arrivals.len();
        return;
    };
    let mut changes = BTreeMap::new();
    for a in arrivals {
        if now.get(&a.key) != a.previous.as_ref() {
            out.skipped += 1;
            continue;
        }
        changes.insert(a.key.clone(), a.value.clone());
        out.count(a.label.as_ref());
    }
    if changes.is_empty() {
        return;
    }
    let body = write_glossary(&text, json, &changes);
    if let Some(dir) = path.parent()
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        log::warn!("sync: cannot make the glossary's folder ({e})");
        return;
    }
    if let Err(e) = textweaver_store::atomic_write(path, body.as_bytes()) {
        log::warn!("sync: cannot save the glossary ({e})");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_glossary_keeps_its_comments_and_order() {
        let text = "# Biology\ncell: the unit of life\nmitosis: division\ncell: a room\n\n# End\n";
        let entries = glossary_entries(text, false).unwrap();
        assert_eq!(
            entries["cell"]["senses"],
            json!(["the unit of life", "a room"])
        );
        let mut changes = BTreeMap::new();
        changes.insert(
            "cell".to_owned(),
            Some(json!({"term": "cell", "senses": ["the smallest living unit"]})),
        );
        changes.insert("mitosis".to_owned(), None);
        changes.insert(
            "enzyme".to_owned(),
            Some(json!({"term": "enzyme", "senses": ["a catalyst"]})),
        );
        let out = write_glossary(text, false, &changes);
        assert_eq!(
            out,
            "# Biology\ncell: the smallest living unit\n\n# End\nenzyme: a catalyst\n"
        );
        let back = glossary_entries(&out, false).unwrap();
        assert_eq!(back["enzyme"], changes["enzyme"].clone().unwrap());
    }

    #[test]
    fn a_json_glossary_is_written_as_json() {
        let text = r#"{"Cell": {"definition": "the unit of life", "pos": "noun"}}"#;
        let entries = glossary_entries(text, true).unwrap();
        assert_eq!(entries["cell"]["term"], "Cell");
        let mut changes = BTreeMap::new();
        changes.insert(
            "enzyme".to_owned(),
            Some(json!({"term": "enzyme", "senses": ["a catalyst"]})),
        );
        let out = write_glossary(text, true, &changes);
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["enzyme"], "a catalyst");
        assert_eq!(v["Cell"]["pos"], "noun");
        assert!(glossary_entries("[1, 2]", true).is_none());
    }

    #[test]
    fn overrides_apply_only_on_the_same_kind_of_system() {
        assert!(in_scope(KEYMAP_MAP, "pc:stop", KeySystem::Windows));
        assert!(in_scope(KEYMAP_MAP, "pc:stop", KeySystem::Linux));
        assert!(!in_scope(KEYMAP_MAP, "mac:stop", KeySystem::Windows));
        assert!(!in_scope(KEYMAP_MAP, "mac:stop", KeySystem::Linux));
        assert!(in_scope(KEYMAP_MAP, "mac:stop", KeySystem::MacOs));
        assert!(!in_scope(KEYMAP_MAP, "pc:stop", KeySystem::MacOs));
        // The label is not what an override says.
        let w = json!({"system": "windows", "chords": ["F5"]});
        let l = json!({"system": "linux", "chords": ["F5"]});
        assert!(same(KEYMAP_MAP, Some(&w), Some(&l)));
        assert!(!same(SETTINGS_MAP, Some(&w), Some(&l)));
    }

    #[test]
    fn only_portable_settings_are_in_scope() {
        assert!(in_scope(SETTINGS_MAP, "speech.rate", KeySystem::Windows));
        assert!(!in_scope(
            SETTINGS_MAP,
            "speech.backend",
            KeySystem::Windows
        ));
        assert!(!in_scope(
            SETTINGS_MAP,
            "speech.favorite_voices",
            KeySystem::Windows
        ));
        assert!(!in_scope(
            SETTINGS_MAP,
            "future.setting",
            KeySystem::Windows
        ));
    }
}
