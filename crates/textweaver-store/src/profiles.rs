//! Settings profiles: named sets of the settings people switch together
//! (voice, rate, theme, font, spacing, highlight, and access mode), kept in
//! `profiles.toml` beside `settings.toml`.
//!
//! A profile stores the values of [`PROFILE_KEYS`] as they were when it was
//! saved, in the same shape as `settings.toml`:
//!
//! ```toml
//! active = "Study"
//!
//! [profiles.Study.speech]
//! rate = 200
//!
//! [profiles.Study.display]
//! theme = "galaxy-light"
//! ```
//!
//! Switching writes those values into the settings and leaves every other
//! setting alone. Profiles work on TOML tables, not on the settings types,
//! so a profile saved by an older or newer textweaver still applies: keys
//! this version does not know are left out when it is switched to or
//! imported, and reported.
//!
//! Star kept its profiles inside `settings.json` (`stats.py`); textweaver
//! keeps them apart, so exporting settings does not drag every profile
//! along, and a profile can be shared on its own ([`Profiles::export`]).

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::settings::Settings;
use crate::{Paths, StoreError, atomic_write};

/// The settings a profile holds, as dotted paths into `settings.toml`. A
/// path to a table (`highlight`) takes the whole table.
pub const PROFILE_KEYS: [&str; 15] = [
    "speech.backend",
    "speech.voice",
    "speech.prefer_voice",
    "speech.rate",
    "speech.pitch",
    "speech.volume",
    "display.theme",
    "highlight",
    "reading_aids.font",
    "reading_aids.spacing",
    "reading_aids.bionic",
    "reading_aids.ruler",
    "accessibility.mode",
    "accessibility.say_all",
    "accessibility.quiet_screen",
];

/// The format marker of an export, `"textweaver_profiles": 1`.
pub const EXPORT_FORMAT: i64 = 1;

/// Profile failures a person can fix.
#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    /// No profile has this name.
    #[error("There is no profile named {0}.")]
    NotFound(String),
    /// A name is empty.
    #[error("A profile needs a name.")]
    EmptyName,
    /// Another profile already has this name.
    #[error("A profile named {0} already exists.")]
    Exists(String),
    /// The file is not a profile export.
    #[error("{0}")]
    NotAnExport(String),
    /// Reading or writing failed.
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// All profiles, and which one was switched to last.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profiles {
    /// The profile switched to or saved last, if it still exists.
    pub active: Option<String>,
    /// Name to its settings, in `settings.toml`'s shape.
    pub profiles: BTreeMap<String, toml::Table>,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

/// What importing did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfileImport {
    /// Profiles added or replaced, by name.
    pub imported: Vec<String>,
    /// Settings left out because this textweaver does not keep them in
    /// profiles, or their values were not valid (`speech.rate in Study`).
    pub dropped: Vec<String>,
}

impl Profiles {
    /// Loads `profiles.toml`; a missing file gives no profiles.
    pub fn load(paths: &Paths) -> Result<Profiles, StoreError> {
        let path = paths.profiles_file();
        match std::fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).map_err(|e| StoreError::Parse {
                path,
                message: e.message().to_owned(),
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Profiles::default()),
            Err(source) => Err(StoreError::Io { path, source }),
        }
    }

    /// Saves `profiles.toml` atomically.
    pub fn save(&self, paths: &Paths) -> Result<(), StoreError> {
        let path = paths.profiles_file();
        let text = toml::to_string_pretty(self).map_err(|e| StoreError::Parse {
            path: path.clone(),
            message: e.to_string(),
        })?;
        atomic_write(
            &path,
            format!("# textweaver settings profiles.\n\n{text}").as_bytes(),
        )
    }

    /// The names, sorted.
    pub fn names(&self) -> Vec<String> {
        self.profiles.keys().cloned().collect()
    }

    /// Saves the profile keys of `settings` as `name`, replacing a profile
    /// of that name. Returns true when one was replaced.
    pub fn save_current(&mut self, name: &str, settings: &Settings) -> Result<bool, ProfileError> {
        let name = clean_name(name)?;
        let replaced = self
            .profiles
            .insert(name.clone(), capture(settings))
            .is_some();
        self.active = Some(name);
        Ok(replaced)
    }

    /// `settings` with profile `name` applied, and the profile's keys this
    /// version could not use.
    pub fn apply(
        &mut self,
        name: &str,
        settings: &Settings,
    ) -> Result<(Settings, Vec<String>), ProfileError> {
        let profile = self
            .profiles
            .get(name)
            .ok_or_else(|| ProfileError::NotFound(name.to_owned()))?;
        let (s, dropped) = apply(profile, settings);
        self.active = Some(name.to_owned());
        Ok((s, dropped))
    }

    /// Renames a profile.
    pub fn rename(&mut self, from: &str, to: &str) -> Result<(), ProfileError> {
        let to = clean_name(to)?;
        if to != from && self.profiles.contains_key(&to) {
            return Err(ProfileError::Exists(to));
        }
        let p = self
            .profiles
            .remove(from)
            .ok_or_else(|| ProfileError::NotFound(from.to_owned()))?;
        self.profiles.insert(to.clone(), p);
        if self.active.as_deref() == Some(from) {
            self.active = Some(to);
        }
        Ok(())
    }

    /// Deletes a profile.
    pub fn delete(&mut self, name: &str) -> Result<(), ProfileError> {
        self.profiles
            .remove(name)
            .ok_or_else(|| ProfileError::NotFound(name.to_owned()))?;
        if self.active.as_deref() == Some(name) {
            self.active = None;
        }
        Ok(())
    }

    /// The profiles named in `names` (all when `None`) as an export: JSON
    /// (`{"textweaver_profiles": 1, "app_version": ..., "profiles":
    /// {...}}`), or TOML when `toml` is set.
    pub fn export(&self, names: Option<&[String]>, toml: bool) -> Result<String, ProfileError> {
        let mut chosen = BTreeMap::new();
        for (name, p) in &self.profiles {
            if names.is_none_or(|n| n.contains(name)) {
                chosen.insert(name.clone(), p.clone());
            }
        }
        if let Some(n) = names
            && let Some(missing) = n.iter().find(|x| !self.profiles.contains_key(*x))
        {
            return Err(ProfileError::NotFound(missing.clone()));
        }
        let mut t = toml::Table::new();
        t.insert(
            "textweaver_profiles".into(),
            toml::Value::Integer(EXPORT_FORMAT),
        );
        t.insert(
            "app_version".into(),
            toml::Value::String(env!("CARGO_PKG_VERSION").into()),
        );
        t.insert(
            "profiles".into(),
            toml::Value::Table(
                chosen
                    .into_iter()
                    .map(|(k, v)| (k, toml::Value::Table(v)))
                    .collect(),
            ),
        );
        let text = if toml {
            toml::to_string_pretty(&t).map_err(|e| ProfileError::NotAnExport(e.to_string()))?
        } else {
            let mut s = serde_json::to_string_pretty(&t)
                .map_err(|e| ProfileError::NotAnExport(e.to_string()))?;
            s.push('\n');
            s
        };
        Ok(text)
    }

    /// Merges an export (JSON or TOML) into these profiles. A profile of
    /// the same name is replaced: the export is the user's choice. Keys
    /// outside [`PROFILE_KEYS`], and values this version would reject, are
    /// left out and reported.
    pub fn import(&mut self, text: &str) -> Result<ProfileImport, ProfileError> {
        let t: toml::Table = match serde_json::from_str(text) {
            Ok(t) => t,
            Err(json) => toml::from_str(text).map_err(|_| {
                ProfileError::NotAnExport(format!("This is not a profile export ({json})."))
            })?,
        };
        if !t.get("textweaver_profiles").is_some_and(|v| v.is_integer()) {
            return Err(ProfileError::NotAnExport(
                "This is not a textweaver profile export: it has no textweaver_profiles marker."
                    .into(),
            ));
        }
        let Some(toml::Value::Table(profiles)) = t.get("profiles") else {
            return Err(ProfileError::NotAnExport(
                "This profile export has no profiles.".into(),
            ));
        };
        let mut report = ProfileImport::default();
        for (name, v) in profiles {
            let (Ok(name), toml::Value::Table(values)) = (clean_name(name), v) else {
                continue;
            };
            let (clean, dropped) = sanitize(values);
            report
                .dropped
                .extend(dropped.into_iter().map(|k| format!("{k} in {name}")));
            if !clean.is_empty() {
                self.profiles.insert(name.clone(), clean);
                report.imported.push(name);
            }
        }
        Ok(report)
    }

    /// Reads an export file and imports it ([`import`](Self::import)).
    pub fn import_file(&mut self, path: &Path) -> Result<ProfileImport, ProfileError> {
        let text = std::fs::read_to_string(path).map_err(|source| StoreError::Io {
            path: path.to_owned(),
            source,
        })?;
        self.import(text.trim_start_matches('\u{feff}'))
    }
}

fn clean_name(name: &str) -> Result<String, ProfileError> {
    let n: String = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if n.is_empty() {
        Err(ProfileError::EmptyName)
    } else {
        Ok(n.chars().take(80).collect())
    }
}

/// The value at a dotted path.
pub(crate) fn get<'a>(t: &'a toml::Table, path: &str) -> Option<&'a toml::Value> {
    let mut parts = path.split('.');
    let first = parts.next()?;
    let mut v = t.get(first)?;
    for p in parts {
        v = v.as_table()?.get(p)?;
    }
    Some(v)
}

/// Sets the value at a dotted path, making tables on the way.
pub(crate) fn set(t: &mut toml::Table, path: &str, value: toml::Value) {
    let mut parts: Vec<&str> = path.split('.').collect();
    let Some(last) = parts.pop() else { return };
    let mut cur = t;
    for p in parts {
        let entry = cur
            .entry(p.to_owned())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        if !entry.is_table() {
            *entry = toml::Value::Table(toml::Table::new());
        }
        let Some(next) = entry.as_table_mut() else {
            return;
        };
        cur = next;
    }
    cur.insert(last.to_owned(), value);
}

/// The profile keys of `settings`, as a profile.
pub fn capture(settings: &Settings) -> toml::Table {
    let full = toml::Table::try_from(settings).unwrap_or_default();
    let mut out = toml::Table::new();
    for key in PROFILE_KEYS {
        if let Some(v) = get(&full, key) {
            set(&mut out, key, v.clone());
        }
    }
    out
}

/// `settings` with `profile`'s values written in, and the profile keys that
/// were left out ([`sanitize`]). A theme in a profile counts as chosen,
/// so it stops the theme from following the system's.
pub fn apply(profile: &toml::Table, settings: &Settings) -> (Settings, Vec<String>) {
    let (clean, dropped) = sanitize(profile);
    let mut full = toml::Table::try_from(settings).unwrap_or_default();
    for key in PROFILE_KEYS {
        if let Some(v) = get(&clean, key) {
            set(&mut full, key, v.clone());
        }
    }
    if get(&clean, "display.theme").is_some() {
        set(
            &mut full,
            "display.theme_explicit",
            toml::Value::Boolean(true),
        );
    }
    let (s, _) = Settings::from_table(full);
    (s, dropped)
}

/// `profile` without the keys a profile does not hold and the values this
/// version rejects, and those keys.
pub fn sanitize(profile: &toml::Table) -> (toml::Table, Vec<String>) {
    let mut clean = toml::Table::new();
    let mut dropped = Vec::new();
    let defaults = toml::Table::try_from(Settings::default()).unwrap_or_default();
    for key in PROFILE_KEYS {
        let Some(v) = get(profile, key) else { continue };
        let mut probe = defaults.clone();
        set(&mut probe, key, v.clone());
        let (_, warnings) = Settings::from_table_unclamped(probe);
        if warnings.is_empty() {
            set(&mut clean, key, v.clone());
        } else {
            dropped.push(key.to_owned());
        }
    }
    let mut leaves = Vec::new();
    collect_leaves(profile, "", &mut leaves);
    for leaf in leaves {
        let covered = PROFILE_KEYS
            .iter()
            .any(|k| leaf == *k || leaf.starts_with(&format!("{k}.")));
        if !covered {
            dropped.push(leaf);
        }
    }
    (clean, dropped)
}

fn collect_leaves(t: &toml::Table, prefix: &str, out: &mut Vec<String>) {
    for (k, v) in t {
        let path = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{prefix}.{k}")
        };
        let whole = PROFILE_KEYS.contains(&path.as_str());
        match v {
            toml::Value::Table(sub) if !whole => collect_leaves(sub, &path, out),
            _ => out.push(path),
        }
    }
}

/// The value of a profile key in `profile`, as text for a list or a
/// message (`200` for the rate, `galaxy` for the theme), if it has one.
pub fn value_text(profile: &toml::Table, key: &str) -> Option<String> {
    get(profile, key).map(|v| match v {
        toml::Value::String(s) => s.clone(),
        other => other.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use textweaver_core::Rate;

    fn study() -> Settings {
        let mut s = Settings::default();
        s.speech.rate = Rate::Wpm(200);
        s.display.theme = "galaxy-light".into();
        s.accessibility.mode = crate::AccessMode::Hybrid;
        s.editing.undo_steps = 7; // not in a profile
        s
    }

    #[test]
    fn save_switch_rename_delete() {
        let mut p = Profiles::default();
        assert!(!p.save_current("  Study  time ", &study()).unwrap());
        assert_eq!(p.names(), ["Study time"]);
        assert_eq!(p.active.as_deref(), Some("Study time"));
        let prof = &p.profiles["Study time"];
        assert_eq!(value_text(prof, "speech.rate").as_deref(), Some("200"));
        assert!(get(prof, "editing.undo_steps").is_none());

        let (s, dropped) = p.apply("Study time", &Settings::default()).unwrap();
        assert!(dropped.is_empty(), "{dropped:?}");
        assert_eq!(s.speech.rate.wpm(), 200);
        assert_eq!(s.display.theme, "galaxy-light");
        assert!(s.display.theme_explicit);
        assert_eq!(s.accessibility.mode, crate::AccessMode::Hybrid);
        assert_eq!(s.editing.undo_steps, Settings::default().editing.undo_steps);

        p.save_current("Skim", &Settings::default()).unwrap();
        assert!(matches!(
            p.rename("Skim", "Study time"),
            Err(ProfileError::Exists(_))
        ));
        p.rename("Study time", "Study").unwrap();
        assert_eq!(p.active.as_deref(), Some("Skim"));
        assert!(matches!(
            p.rename("Nope", "X"),
            Err(ProfileError::NotFound(_))
        ));
        assert!(matches!(
            p.save_current(" ", &study()),
            Err(ProfileError::EmptyName)
        ));
        p.delete("Skim").unwrap();
        assert_eq!(p.active, None);
        assert!(p.delete("Skim").is_err());
        assert_eq!(p.names(), ["Study"]);
    }

    #[test]
    fn file_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::under(dir.path());
        assert_eq!(Profiles::load(&paths).unwrap(), Profiles::default());
        let mut p = Profiles::default();
        p.save_current("Study", &study()).unwrap();
        p.save(&paths).unwrap();
        let text = std::fs::read_to_string(paths.profiles_file()).unwrap();
        assert!(text.contains("active = \"Study\""), "{text}");
        assert_eq!(Profiles::load(&paths).unwrap(), p);
        std::fs::write(paths.profiles_file(), "not = [toml").unwrap();
        assert!(Profiles::load(&paths).is_err());
    }

    #[test]
    fn export_and_import_across_versions() {
        let mut p = Profiles::default();
        p.save_current("Study", &study()).unwrap();
        p.save_current("Night", &Settings::default()).unwrap();
        let json = p.export(Some(&["Study".to_owned()]), false).unwrap();
        assert!(json.contains("\"textweaver_profiles\": 1"));
        assert!(!json.contains("Night"));
        assert!(p.export(Some(&["Nope".to_owned()]), false).is_err());

        let mut q = Profiles::default();
        let r = q.import(&json).unwrap();
        assert_eq!(r.imported, ["Study"]);
        assert!(r.dropped.is_empty());
        assert_eq!(q.profiles["Study"], p.profiles["Study"]);

        let toml_text = p.export(None, true).unwrap();
        let mut q = Profiles::default();
        assert_eq!(q.import(&toml_text).unwrap().imported, ["Night", "Study"]);

        // A newer version's keys and a bad value are left out, and said.
        let r = q
            .import(
                r#"{"textweaver_profiles": 2, "profiles": {"Future": {
                    "speech": {"rate": 300, "timbre": 3, "pitch": "high"},
                    "display": {"theme": "nord"}, "gadget": {"x": 1}}}}"#,
            )
            .unwrap();
        assert_eq!(r.imported, ["Future"]);
        assert_eq!(
            r.dropped,
            [
                "speech.pitch in Future",
                "gadget.x in Future",
                "speech.timbre in Future"
            ]
        );
        assert_eq!(
            value_text(&q.profiles["Future"], "display.theme").as_deref(),
            Some("nord")
        );

        assert!(matches!(q.import("{}"), Err(ProfileError::NotAnExport(_))));
        assert!(matches!(
            q.import("garbage ["),
            Err(ProfileError::NotAnExport(_))
        ));
        assert!(matches!(
            q.import(r#"{"textweaver_profiles": 1}"#),
            Err(ProfileError::NotAnExport(_))
        ));
    }
}
