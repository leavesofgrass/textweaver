//! Settings import and export as JSON (or TOML).
//!
//! textweaver keeps its settings in `settings.toml` and key overrides in
//! `keymap.toml`. This module moves both in and out as one versioned
//! document, so they can be backed up, shared, and edited with other tools:
//!
//! ```json
//! {
//!   "exported": "2026-09-25T14:03:07Z",
//!   "keymap": {
//!     "next_sentence": ["Alt+N", "b:."]
//!   },
//!   "settings": {
//!     "speech": { "rate": 300 }
//!   },
//!   "textweaver_settings": 1
//! }
//! ```
//!
//! - **Export** writes every setting (unset ones as `null`) or only the
//!   values that differ from the defaults. JSON uses 2-space indentation
//!   and sorted keys, so two exports diff cleanly. The same document can be
//!   written as TOML.
//! - **Import** reads that document (JSON or TOML), or a bare
//!   `settings.toml`, and validates everything before anything is written:
//!   a wrong type or an out-of-range value is an error naming its path
//!   (`settings.speech.rate`); unknown sections and keys are reported and
//!   kept. A star `settings.json` is pointed to `tw migrate-star`.
//! - **Merge** (the default) changes only what the file names; `null`
//!   returns a setting to its default. **Replace** makes the settings (and,
//!   when the file has a `keymap`, the key overrides) exactly the file's.
//!   Lists and maps such as `speed_presets` or `pronunciations` are one
//!   value each and are replaced whole, as in `settings.toml`.
//! - [`SettingsStore::apply`] backs up the files it is about to change
//!   (`settings.toml.bak-20260925-140307`) and writes atomically.
//!
//! TOML dates in unknown keys have no JSON form; they travel as
//! `{"$__toml_private_datetime": "..."}`, the `toml` crate's own encoding,
//! so they come back as dates.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::settings::STRUCT_TABLES;
use crate::{
    AppleSettings, CommunityLexiconSettings, DisplaySettings, EciSettings, EditingSettings,
    ExportSettings, HighlightSettings, KeyboardSettings, KeymapOverrides, LibrarySettings,
    NormalizationSettings, ReadingSettings, SapiSettings, Settings, SettingsStore, SpeechSettings,
    StoreError,
};

/// The key that marks a settings export and holds its format number.
pub const FORMAT_KEY: &str = "textweaver_settings";

/// The export format this version writes and reads.
pub const FORMAT_VERSION: u64 = 1;

/// How the `toml` crate encodes a date in serde; used for dates in
/// unknown keys, which JSON cannot hold.
const DATETIME_KEY: &str = "$__toml_private_datetime";

/// Settings import and export failures. Every message is a plain sentence
/// that reads well aloud.
#[derive(Debug, thiserror::Error)]
pub enum SettingsIoError {
    /// The file is not valid JSON or TOML.
    #[error("The file is not valid {format}: {message}.")]
    Syntax {
        /// `JSON` or `TOML`.
        format: &'static str,
        /// What is wrong and where.
        message: String,
    },
    /// The file is JSON or TOML but not textweaver settings.
    #[error("This file does not look like textweaver settings: {0}.")]
    NotSettings(String),
    /// The file is a star `settings.json`.
    #[error(
        "This is a star settings file. Import it with: tw migrate-star --from followed by the folder that contains it."
    )]
    StarSettings,
    /// The file was exported by a newer textweaver.
    #[error(
        "This file was exported by a newer textweaver (settings format {0}); this version reads format {FORMAT_VERSION}. Nothing was changed."
    )]
    NewerFormat(u64),
    /// Values of the wrong type or out of range. Nothing was written.
    #[error("{}", invalid_message(.0))]
    Invalid(Vec<String>),
    /// `reset` was given a section that does not exist.
    #[error("There is no settings section named {name}. Sections: {}.", .known.join(", "))]
    UnknownSection {
        /// The name given.
        name: String,
        /// The sections that exist.
        known: Vec<String>,
    },
    /// The current `settings.toml` could not be parsed, so there is nothing
    /// safe to merge into.
    #[error(
        "Your current settings file {} cannot be read ({message}). Fix it or move it away, then try again.",
        .path.display()
    )]
    CurrentUnreadable {
        /// The settings file.
        path: PathBuf,
        /// Parser message.
        message: String,
    },
    /// Reading or writing the settings files failed.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// The settings could not be encoded (should not happen).
    #[error("The settings could not be written as {format}: {message}.")]
    Encode {
        /// `JSON` or `TOML`.
        format: &'static str,
        /// Encoder message.
        message: String,
    },
}

fn invalid_message(errors: &[String]) -> String {
    let n = errors.len();
    let noun = if n == 1 { "problem" } else { "problems" };
    format!(
        "The file has {n} {noun}, so nothing was changed: {}.",
        errors.join("; ")
    )
}

/// Export file format.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExportFormat {
    /// JSON, 2-space indentation, sorted keys.
    #[default]
    Json,
    /// The same document as TOML.
    Toml,
}

impl ExportFormat {
    /// `json` or `toml`.
    pub fn name(self) -> &'static str {
        match self {
            ExportFormat::Json => "json",
            ExportFormat::Toml => "toml",
        }
    }

    /// The format a file name suggests: TOML for `.toml`, else JSON.
    pub fn for_path(path: &Path) -> Self {
        match path.extension().and_then(|e| e.to_str()) {
            Some(e) if e.eq_ignore_ascii_case("toml") => ExportFormat::Toml,
            _ => ExportFormat::Json,
        }
    }
}

/// What an export contains.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExportOptions {
    /// Only values that differ from the defaults (plus unknown keys and
    /// every key override).
    pub changed_only: bool,
    /// JSON or TOML.
    pub format: ExportFormat,
}

/// How an import combines the file with the current settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImportMode {
    /// Change only what the file names (the default).
    #[default]
    Merge,
    /// Make the settings exactly the file's; key overrides too when the
    /// file has a `keymap`.
    Replace,
}

/// Which file a [`Change`] is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeArea {
    /// A setting in `settings.toml`.
    Settings,
    /// A key override in `keymap.toml`.
    Keymap,
}

/// One setting or key override that an import or reset changes.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    /// Settings or key overrides.
    pub area: ChangeArea,
    /// Dotted setting path (`speech.rate`), or the action id for keys.
    pub path: String,
    /// The value before; `None` when absent (for keys: the default keys).
    pub before: Option<Value>,
    /// The value after; `None` when absent (for keys: the default keys).
    pub after: Option<Value>,
}

impl Change {
    /// One line in plain words, such as `speech.rate changes from 265 to
    /// 300.` or `Keys for stop change from the default keys to F5.`
    pub fn describe(&self) -> String {
        match self.area {
            ChangeArea::Settings => match (&self.before, &self.after) {
                (Some(b), Some(a)) => {
                    format!("{} changes from {} to {}.", self.path, words(b), words(a))
                }
                (None, Some(a)) => format!("{} is added, set to {}.", self.path, words(a)),
                (Some(b), None) => format!("{} is removed; it was {}.", self.path, words(b)),
                (None, None) => format!("{} changes.", self.path),
            },
            ChangeArea::Keymap => {
                let keys = |v: &Option<Value>| match v {
                    None => "the default keys".to_owned(),
                    Some(Value::Array(a)) if a.is_empty() => "no keys".to_owned(),
                    Some(v) => words(v),
                };
                format!(
                    "Keys for {} change from {} to {}.",
                    self.path,
                    keys(&self.before),
                    keys(&self.after)
                )
            }
        }
    }
}

/// A value in plain words: `on` and `off` for booleans, `not set` for
/// null, lists joined with commas.
fn words(v: &Value) -> String {
    match v {
        Value::Null => "not set".to_owned(),
        Value::Bool(true) => "on".to_owned(),
        Value::Bool(false) => "off".to_owned(),
        Value::Number(n) => n.to_string(),
        Value::String(s) if s.is_empty() => "empty text".to_owned(),
        Value::String(s) => s.clone(),
        Value::Array(a) if a.is_empty() => "an empty list".to_owned(),
        Value::Array(a) => a.iter().map(words).collect::<Vec<_>>().join(", "),
        Value::Object(m) => match m.get(DATETIME_KEY) {
            Some(Value::String(d)) if m.len() == 1 => d.clone(),
            _ if m.is_empty() => "nothing".to_owned(),
            _ => m
                .iter()
                .map(|(k, v)| format!("{k} {}", words(v)))
                .collect::<Vec<_>>()
                .join("; "),
        },
    }
}

/// What an import or reset would do, computed without writing anything.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportPlan {
    /// The settings after the import.
    pub settings: Settings,
    /// The key overrides after the import.
    pub keymap: KeymapOverrides,
    /// Every changed setting and key override, in path order.
    pub changes: Vec<Change>,
    /// Things to tell the user that do not stop the import: unknown keys
    /// kept, invalid values already in the current settings file.
    pub warnings: Vec<String>,
    /// `settings.toml` needs writing.
    pub settings_changed: bool,
    /// `keymap.toml` needs writing.
    pub keymap_changed: bool,
}

impl ImportPlan {
    /// True when nothing would change.
    pub fn is_empty(&self) -> bool {
        !self.settings_changed && !self.keymap_changed
    }

    /// How many settings and key overrides change.
    pub fn change_count(&self) -> usize {
        self.changes.len()
    }

    /// `Nothing changes.`, `1 setting changes.`, or `5 settings change.`
    pub fn summary(&self) -> String {
        match self.change_count() {
            0 if self.is_empty() => "Nothing changes.".to_owned(),
            0 | 1 => "1 setting changes.".to_owned(),
            n => format!("{n} settings change."),
        }
    }

    /// Each change as a line in plain words.
    pub fn lines(&self) -> Vec<String> {
        self.changes.iter().map(Change::describe).collect()
    }
}

/// What [`SettingsStore::apply`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Applied {
    /// Backups made of the files before they were replaced.
    pub backups: Vec<PathBuf>,
    /// Files written.
    pub written: Vec<PathBuf>,
}

// ---------------------------------------------------------------------------
// JSON <-> TOML values

/// A TOML value as JSON. Dates become `{"$__toml_private_datetime": ...}`;
/// NaN and infinities (which JSON lacks) become strings.
pub fn toml_to_json(v: &toml::Value) -> Value {
    match v {
        toml::Value::String(s) => Value::String(s.clone()),
        toml::Value::Integer(i) => Value::from(*i),
        toml::Value::Float(f) => serde_json::Number::from_f64(*f)
            .map_or_else(|| Value::String(f.to_string()), Value::Number),
        toml::Value::Boolean(b) => Value::Bool(*b),
        toml::Value::Datetime(d) => {
            let mut m = Map::new();
            m.insert(DATETIME_KEY.to_owned(), Value::String(d.to_string()));
            Value::Object(m)
        }
        toml::Value::Array(a) => Value::Array(a.iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => Value::Object(
            t.iter()
                .map(|(k, v)| (k.clone(), toml_to_json(v)))
                .collect(),
        ),
    }
}

/// A JSON value as TOML. `null` is `None` at the top and drops a key inside
/// an object; a list may not contain `null`. `path` names the value in
/// error messages.
pub fn json_to_toml(v: &Value, path: &str) -> Result<Option<toml::Value>, String> {
    Ok(Some(match v {
        Value::Null => return Ok(None),
        Value::Bool(b) => toml::Value::Boolean(*b),
        Value::String(s) => toml::Value::String(s.clone()),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                toml::Value::Integer(i)
            } else if n.is_u64() {
                return Err(format!("{path}: the number {n} is too large"));
            } else {
                toml::Value::Float(n.as_f64().unwrap_or_default())
            }
        }
        Value::Array(a) => {
            let mut out = Vec::with_capacity(a.len());
            for (i, item) in a.iter().enumerate() {
                match json_to_toml(item, &format!("{path}[{i}]"))? {
                    Some(t) => out.push(t),
                    None => return Err(format!("{path}[{i}]: a list cannot contain null")),
                }
            }
            toml::Value::Array(out)
        }
        Value::Object(m) => {
            if let (1, Some(Value::String(d))) = (m.len(), m.get(DATETIME_KEY)) {
                let date = d
                    .parse::<toml::value::Datetime>()
                    .map_err(|e| format!("{path}: {d} is not a date ({e})"))?;
                return Ok(Some(toml::Value::Datetime(date)));
            }
            toml::Value::Table(json_object_to_toml(m, path)?)
        }
    }))
}

fn json_object_to_toml(m: &Map<String, Value>, path: &str) -> Result<toml::Table, String> {
    let mut t = toml::Table::new();
    for (k, v) in m {
        let sub = join(path, k);
        if let Some(value) = json_to_toml(v, &sub)? {
            t.insert(k.clone(), value);
        }
    }
    Ok(t)
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_owned()
    } else {
        format!("{path}.{key}")
    }
}

/// Objects rebuilt with sorted keys, and floats that came from an `f32`
/// (such as `highlight.speed`) written in their shortest form (`0.7`, not
/// `0.699999988079071`).
fn tidy(v: Value) -> Value {
    match v {
        Value::Object(m) => {
            let mut entries: Vec<(String, Value)> = m.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(entries.into_iter().map(|(k, v)| (k, tidy(v))).collect())
        }
        Value::Array(a) => Value::Array(a.into_iter().map(tidy).collect()),
        Value::Number(n) if n.is_f64() => {
            let f = n.as_f64().unwrap_or_default();
            #[allow(clippy::cast_possible_truncation)]
            let single = f as f32;
            let short = if f64::from(single) == f {
                single.to_string().parse::<f64>().unwrap_or(f)
            } else {
                f
            };
            serde_json::Number::from_f64(short).map_or(Value::Number(n), Value::Number)
        }
        other => other,
    }
}

/// Every setting as a JSON object with sorted keys; unset values are
/// `null`, unknown keys are included.
pub fn settings_to_json(settings: &Settings) -> Result<Map<String, Value>, SettingsIoError> {
    let v = serde_json::to_value(settings).map_err(|e| SettingsIoError::Encode {
        format: "JSON",
        message: e.to_string(),
    })?;
    match tidy(v) {
        Value::Object(m) => Ok(m),
        _ => Err(SettingsIoError::Encode {
            format: "JSON",
            message: "settings are not an object".into(),
        }),
    }
}

/// `full` without the values equal to `defaults`, descending into the
/// struct tables (the JSON twin of `settings::minimal_table`).
fn json_minimal(
    full: Map<String, Value>,
    defaults: &Map<String, Value>,
    path: &str,
) -> Map<String, Value> {
    let mut out = Map::new();
    for (key, value) in full {
        let sub = join(path, &key);
        match (value, defaults.get(&key)) {
            (Value::Object(t), Some(Value::Object(d))) if is_struct(&sub) => {
                let kept = json_minimal(t, d, &sub);
                if !kept.is_empty() {
                    out.insert(key, Value::Object(kept));
                }
            }
            (value, Some(d)) if *d == value => {}
            (value, _) => {
                out.insert(key, value);
            }
        }
    }
    out
}

fn is_struct(path: &str) -> bool {
    STRUCT_TABLES.contains(&path)
}

/// The top-level settings sections (`speech`, `highlight`, ...).
fn top_sections() -> impl Iterator<Item = &'static str> {
    STRUCT_TABLES.iter().copied().filter(|s| !s.contains('.'))
}

// ---------------------------------------------------------------------------
// Export

/// The export document for `settings` and `keymap`, stamped with `now`
/// (Unix seconds, UTC), as JSON or TOML text ending in a newline.
pub fn export_settings(
    settings: &Settings,
    keymap: &KeymapOverrides,
    options: ExportOptions,
    now: i64,
) -> Result<String, SettingsIoError> {
    let mut body = settings_to_json(settings)?;
    if options.changed_only {
        let defaults = settings_to_json(&Settings::default())?;
        body = json_minimal(body, &defaults, "");
    }
    let keys: Map<String, Value> = keymap
        .iter()
        .map(|(action, chords)| {
            (
                action.clone(),
                Value::Array(chords.iter().cloned().map(Value::String).collect()),
            )
        })
        .collect();
    let mut doc = Map::new();
    doc.insert(FORMAT_KEY.to_owned(), Value::from(FORMAT_VERSION));
    doc.insert(
        "exported".to_owned(),
        Value::String(crate::time::rfc3339(now)),
    );
    doc.insert("settings".to_owned(), Value::Object(body));
    doc.insert("keymap".to_owned(), Value::Object(keys));
    let doc = tidy(Value::Object(doc));
    match options.format {
        ExportFormat::Json => {
            let mut text =
                serde_json::to_string_pretty(&doc).map_err(|e| SettingsIoError::Encode {
                    format: "JSON",
                    message: e.to_string(),
                })?;
            text.push('\n');
            Ok(text)
        }
        ExportFormat::Toml => {
            let encode = |message: String| SettingsIoError::Encode {
                format: "TOML",
                message,
            };
            let table = match json_to_toml(&doc, "").map_err(encode)? {
                Some(toml::Value::Table(t)) => t,
                _ => return Err(encode("the export is not a table".into())),
            };
            let body = toml::to_string_pretty(&table).map_err(|e| encode(e.to_string()))?;
            Ok(format!("{TOML_EXPORT_HEADER}{body}"))
        }
    }
}

const TOML_EXPORT_HEADER: &str = "\
# textweaver settings export. Import it with: tw settings import FILE
# Settings are under [settings], key overrides under [keymap].

";

// ---------------------------------------------------------------------------
// Import

/// The parts of an import file.
struct Incoming {
    settings: Option<Map<String, Value>>,
    keymap: Option<Map<String, Value>>,
    /// `settings.` for an export document, empty for a bare settings file,
    /// so error paths match the file.
    prefix: &'static str,
    warnings: Vec<String>,
}

/// Parses JSON (text starting with `{`) or TOML into a JSON object.
fn parse_input(text: &str) -> Result<Map<String, Value>, SettingsIoError> {
    let text = text.trim_start_matches('\u{feff}');
    if text.trim_start().starts_with('{') {
        return match serde_json::from_str::<Value>(text) {
            Ok(Value::Object(m)) => Ok(m),
            Ok(_) => Err(SettingsIoError::NotSettings(
                "it is not a JSON object".into(),
            )),
            Err(e) => Err(SettingsIoError::Syntax {
                format: "JSON",
                message: e.to_string(),
            }),
        };
    }
    match text.parse::<toml::Table>() {
        Ok(t) => Ok(t
            .iter()
            .map(|(k, v)| (k.clone(), toml_to_json(v)))
            .collect()),
        Err(e) => {
            let line = e
                .span()
                .map(|s| text[..s.start.min(text.len())].matches('\n').count() + 1);
            let message = e.message().trim().trim_end_matches('.').replace('`', "");
            Err(SettingsIoError::Syntax {
                format: "TOML",
                message: match line {
                    Some(l) => format!("{message}, at line {l}"),
                    None => message,
                },
            })
        }
    }
}

/// True for a star `settings.json`: `tts_` keys, reading positions, or
/// keybindings, and none of textweaver's sections.
fn looks_like_star(doc: &Map<String, Value>) -> bool {
    doc.keys().any(|k| {
        k.starts_with("tts_")
            || matches!(
                k.as_str(),
                "reading_positions" | "keybindings" | "recent_files" | "bookmarks"
            )
    })
}

/// Sorts a parsed file into settings and key overrides.
fn classify(mut doc: Map<String, Value>) -> Result<Incoming, SettingsIoError> {
    let has_section = doc.keys().any(|k| top_sections().any(|s| s == k));
    let wrapped = doc.contains_key(FORMAT_KEY)
        || (!has_section && (doc.contains_key("settings") || doc.contains_key("keymap")));
    if !wrapped {
        if !has_section && looks_like_star(&doc) {
            return Err(SettingsIoError::StarSettings);
        }
        if !has_section && !doc.is_empty() {
            let sections: Vec<&str> = top_sections().collect();
            return Err(SettingsIoError::NotSettings(format!(
                "it has none of textweaver's sections ({})",
                sections.join(", ")
            )));
        }
        return Ok(Incoming {
            settings: Some(doc),
            keymap: None,
            prefix: "",
            warnings: Vec::new(),
        });
    }
    let mut errors = Vec::new();
    match doc.remove(FORMAT_KEY) {
        None => {}
        Some(v) => match v.as_u64() {
            Some(n) if n > FORMAT_VERSION => return Err(SettingsIoError::NewerFormat(n)),
            Some(n) if n >= 1 => {}
            _ => errors.push(format!(
                "{FORMAT_KEY}: expected the format number {FORMAT_VERSION}, found {}",
                words(&v)
            )),
        },
    }
    doc.remove("exported");
    let mut part = |name: &str| match doc.remove(name) {
        None | Some(Value::Null) => None,
        Some(Value::Object(m)) => Some(m),
        Some(other) => {
            errors.push(format!(
                "{name}: expected a JSON object, found {}",
                words(&other)
            ));
            None
        }
    };
    let settings = part("settings");
    let keymap = part("keymap");
    if !errors.is_empty() {
        return Err(SettingsIoError::Invalid(errors));
    }
    let warnings = doc
        .keys()
        .map(|k| format!("{k} is not part of a settings export and is ignored."))
        .collect();
    Ok(Incoming {
        settings,
        keymap,
        prefix: "settings.",
        warnings,
    })
}

/// Checks one value against the settings struct for `section` by
/// deserializing a table holding only that key.
fn check_leaf(section: &str, key: &str, value: toml::Value) -> Result<(), String> {
    fn fits<T: serde::de::DeserializeOwned>(key: &str, value: toml::Value) -> Result<(), String> {
        let mut t = toml::Table::new();
        t.insert(key.to_owned(), value);
        toml::Value::Table(t)
            .try_into::<T>()
            .map(drop)
            .map_err(|e| plain_serde(e.message()))
    }
    match section {
        "speech" => fits::<SpeechSettings>(key, value),
        "speech.eci" => fits::<EciSettings>(key, value),
        "speech.sapi" => fits::<SapiSettings>(key, value),
        "speech.apple" => fits::<AppleSettings>(key, value),
        "speech.dectalk" => fits::<crate::DectalkSettings>(key, value),
        "speech.piper" => fits::<crate::PiperSettings>(key, value),
        "highlight" => fits::<HighlightSettings>(key, value),
        "normalization" => fits::<NormalizationSettings>(key, value),
        "normalization.community_lexicon" => fits::<CommunityLexiconSettings>(key, value),
        "normalization.medical_lexicon" => fits::<crate::MedicalLexiconSettings>(key, value),
        "reading" => fits::<ReadingSettings>(key, value),
        "display" => fits::<DisplaySettings>(key, value),
        "editing" => fits::<EditingSettings>(key, value),
        "authoring" => fits::<crate::AuthoringSettings>(key, value),
        "library" => fits::<LibrarySettings>(key, value),
        "keyboard" => fits::<KeyboardSettings>(key, value),
        "accessibility" => fits::<crate::AccessibilitySettings>(key, value),
        "export" => fits::<ExportSettings>(key, value),
        "colors" => fits::<crate::ColorSettings>(key, value),
        "sync" => fits::<crate::SyncSettings>(key, value),
        _ => Ok(()),
    }
}

/// A serde message in plain words: `invalid type: string "fast", expected
/// u16` becomes `expected a whole number, found the text "fast"`.
fn plain_serde(msg: &str) -> String {
    let msg = msg.trim().replace('`', "");
    let split = |rest: &str| {
        rest.split_once(", expected ")
            .map(|(f, e)| (f.trim().to_owned(), e.trim().to_owned()))
    };
    if let Some((found, expected)) = msg.strip_prefix("invalid type: ").and_then(split) {
        return format!(
            "expected {}, found {}",
            plain_expected(&expected),
            plain_found(&found)
        );
    }
    if let Some((found, _)) = msg.strip_prefix("invalid value: ").and_then(split) {
        return format!("{} is out of range", plain_found(&found));
    }
    if let Some(rest) = msg.strip_prefix("unknown variant ")
        && let Some((found, choices)) = rest.split_once(", expected one of ")
    {
        return format!("{found} is not one of the choices: {choices}");
    }
    msg
}

fn plain_expected(e: &str) -> String {
    match e {
        "f32" | "f64" => "a number",
        "u8" | "u16" | "u32" | "u64" | "usize" => "a whole number, 0 or more",
        "i8" | "i16" | "i32" | "i64" | "isize" => "a whole number",
        "a sequence" => "a list",
        "a map" => "a section",
        "struct EciSettings" | "struct SapiSettings" | "struct AppleSettings" => "a section",
        other => other,
    }
    .to_owned()
}

fn plain_found(f: &str) -> String {
    if let Some(s) = f.strip_prefix("string ") {
        return format!("the text {s}");
    }
    for p in ["integer ", "floating point "] {
        if let Some(n) = f.strip_prefix(p) {
            return format!("the number {n}");
        }
    }
    if let Some(b) = f.strip_prefix("boolean ") {
        return b.to_owned();
    }
    match f {
        "sequence" => "a list",
        "map" => "a section",
        other => other,
    }
    .to_owned()
}

/// Validates an incoming settings object against the defaults' shape:
/// type errors go to `errors`, unknown keys to `warnings`.
fn check_table(
    incoming: &Map<String, Value>,
    defaults: &Map<String, Value>,
    path: &str,
    prefix: &str,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    for (key, value) in incoming {
        let sub = join(path, key);
        let shown = format!("{prefix}{sub}");
        match defaults.get(key) {
            None => {
                warnings.push(if path.is_empty() && value.is_object() {
                    format!("{shown} is not a textweaver settings section; it is kept.")
                } else {
                    format!("{shown} is not a textweaver setting; it is kept.")
                });
                if let Err(e) = json_to_toml(value, &shown) {
                    errors.push(e);
                }
            }
            Some(d) if is_struct(&sub) => match (value, d) {
                (Value::Null, _) => {}
                (Value::Object(m), Value::Object(dm)) => {
                    check_table(m, dm, &sub, prefix, errors, warnings);
                }
                _ => errors.push(format!(
                    "{shown}: expected a section (a JSON object), found {}",
                    words(value)
                )),
            },
            Some(_) => match json_to_toml(value, &shown) {
                Ok(Some(t)) => {
                    if let Err(e) = check_leaf(path, key, t) {
                        errors.push(format!("{shown}: {e}"));
                    }
                }
                Ok(None) => {}
                Err(e) => errors.push(e),
            },
        }
    }
}

/// Merges `incoming` into `target`: `null` removes a key (back to its
/// default), struct tables and unknown tables merge key by key, anything
/// else replaces the old value.
fn merge(
    target: &mut Map<String, Value>,
    incoming: &Map<String, Value>,
    defaults: Option<&Map<String, Value>>,
    path: &str,
) {
    for (key, value) in incoming {
        let sub = join(path, key);
        if value.is_null() {
            target.remove(key);
            continue;
        }
        let default = defaults.and_then(|d| d.get(key));
        let known_leaf = default.is_some() && !is_struct(&sub);
        match (target.get_mut(key), value) {
            (Some(Value::Object(t)), Value::Object(m)) if !known_leaf => {
                merge(t, m, default.and_then(Value::as_object), &sub);
            }
            _ => {
                target.insert(key.clone(), value.clone());
            }
        }
    }
}

/// Key overrides from an incoming `keymap` object.
fn keymap_from_json(
    incoming: &Map<String, Value>,
    base: KeymapOverrides,
    errors: &mut Vec<String>,
) -> KeymapOverrides {
    let mut out = base;
    for (action, value) in incoming {
        let bad = || {
            format!(
                "keymap.{action}: expected a list of keys such as [\"Alt+N\"], or null for the default keys; found {}",
                words(value)
            )
        };
        match value {
            Value::Null => {
                out.remove(action);
            }
            Value::String(s) => {
                out.insert(action.clone(), vec![s.clone()]);
            }
            Value::Array(items) => {
                let keys: Option<Vec<String>> = items
                    .iter()
                    .map(|i| i.as_str().map(str::to_owned))
                    .collect();
                match keys {
                    Some(k) => {
                        out.insert(action.clone(), k);
                    }
                    None => errors.push(bad()),
                }
            }
            _ => errors.push(bad()),
        }
    }
    out
}

/// Plans importing `text` (an export document in JSON or TOML, or a bare
/// `settings.toml`) over the current settings and key overrides. Nothing
/// is written; see [`SettingsStore::apply`].
pub fn plan_import(
    current: &Settings,
    current_keymap: &KeymapOverrides,
    text: &str,
    mode: ImportMode,
) -> Result<ImportPlan, SettingsIoError> {
    let mut incoming = classify(parse_input(text)?)?;
    let prefix = incoming.prefix;
    let mut warnings = incoming.warnings;
    // Renamed keys move to their new names first, so their values are
    // imported, not warned about (crate::settings::RENAMED_SETTINGS).
    if let Some(map) = incoming.settings.take() {
        let renamed = json_object_to_toml(&map, "").ok().and_then(|mut t| {
            (!crate::settings::rename_legacy_settings(&mut t).is_empty()).then_some(t)
        });
        incoming.settings = Some(match renamed {
            Some(t) => match toml_to_json(&toml::Value::Table(t)) {
                Value::Object(m) => m,
                _ => map,
            },
            None => map,
        });
    }
    let mut errors = Vec::new();
    let settings = match &incoming.settings {
        None => current.clone(),
        Some(map) => {
            let defaults = settings_to_json(&Settings::default())?;
            check_table(map, &defaults, "", prefix, &mut errors, &mut warnings);
            let merged = match mode {
                ImportMode::Merge => {
                    let mut m = settings_to_json(current)?;
                    merge(&mut m, map, Some(&defaults), "");
                    m
                }
                ImportMode::Replace => map.clone(),
            };
            match json_object_to_toml(&merged, prefix.trim_end_matches('.')) {
                Ok(table) => {
                    let (mut s, w) = Settings::from_table_unclamped(table);
                    for f in s.fix_ranges() {
                        errors.push(format!("{prefix}{}: {}", f.path, f.problem));
                    }
                    if errors.is_empty() {
                        warnings.extend(w);
                    }
                    s
                }
                Err(e) => {
                    if errors.is_empty() {
                        errors.push(e);
                    }
                    current.clone()
                }
            }
        }
    };
    let keymap = match &incoming.keymap {
        None => current_keymap.clone(),
        Some(map) => {
            let base = match mode {
                ImportMode::Merge => current_keymap.clone(),
                ImportMode::Replace => KeymapOverrides::new(),
            };
            keymap_from_json(map, base, &mut errors)
        }
    };
    if !errors.is_empty() {
        return Err(SettingsIoError::Invalid(errors));
    }
    build_plan(current, current_keymap, settings, keymap, warnings)
}

/// The names [`plan_reset`] accepts: the settings sections, `keymap`, and
/// any unknown sections present in `current`.
pub fn reset_sections(current: &Settings) -> Vec<String> {
    // STRUCT_TABLES, in the order of settings.toml's documentation.
    let mut v: Vec<String> = [
        "speech",
        "speech.eci",
        "speech.sapi",
        "speech.apple",
        "speech.dectalk",
        "speech.piper",
        "highlight",
        "normalization",
        "normalization.community_lexicon",
        "normalization.medical_lexicon",
        "reading",
        "display",
        "editing",
        "authoring",
        "library",
        "keyboard",
        "accessibility",
        "export",
        "braille",
        "reading_aids",
        "reading_aids.rsvp",
        "reading_aids.bionic_options",
        "reading_aids.spacing",
        "reading_aids.font",
        "reading_aids.ruler",
        "reading_aids.syllable_options",
        "preview",
        "lexicon",
        "stats",
        "summary",
        "dictation",
        "interface",
        "gui",
        "colors",
        "sync",
        "components",
        "keymap",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    v.extend(
        current
            .extra
            .iter()
            .filter(|(_, v)| v.is_table())
            .map(|(k, _)| k.clone()),
    );
    v
}

/// Plans a reset: everything (settings and key overrides) when `section`
/// is `None`, else one section (`speech`, `speech.eci`, ..., or `keymap`
/// for the key overrides).
pub fn plan_reset(
    current: &Settings,
    current_keymap: &KeymapOverrides,
    section: Option<&str>,
) -> Result<ImportPlan, SettingsIoError> {
    let Some(name) = section.map(str::trim) else {
        return build_plan(
            current,
            current_keymap,
            Settings::default(),
            KeymapOverrides::new(),
            Vec::new(),
        );
    };
    if matches!(name, "keymap" | "keys") {
        return build_plan(
            current,
            current_keymap,
            current.clone(),
            KeymapOverrides::new(),
            Vec::new(),
        );
    }
    if !reset_sections(current).iter().any(|s| s == name) {
        return Err(SettingsIoError::UnknownSection {
            name: name.to_owned(),
            known: reset_sections(current),
        });
    }
    fn remove_path(m: &mut Map<String, Value>, path: &str) {
        match path.split_once('.') {
            Some((head, rest)) => {
                if let Some(Value::Object(sub)) = m.get_mut(head) {
                    remove_path(sub, rest);
                }
            }
            None => {
                m.remove(path);
            }
        }
    }
    let mut json = settings_to_json(current)?;
    remove_path(&mut json, name);
    let toml = json_object_to_toml(&json, "").map_err(|message| SettingsIoError::Encode {
        format: "TOML",
        message,
    })?;
    let (settings, _) = Settings::from_table(toml);
    build_plan(
        current,
        current_keymap,
        settings,
        current_keymap.clone(),
        Vec::new(),
    )
}

/// Lists the differences between the current and the new settings and key
/// overrides.
fn build_plan(
    current: &Settings,
    current_keymap: &KeymapOverrides,
    settings: Settings,
    keymap: KeymapOverrides,
    warnings: Vec<String>,
) -> Result<ImportPlan, SettingsIoError> {
    let before = Value::Object(settings_to_json(current)?);
    let after = Value::Object(settings_to_json(&settings)?);
    let mut changes = Vec::new();
    diff(Some(&before), Some(&after), "", &mut changes);
    let actions: BTreeSet<&String> = current_keymap.keys().chain(keymap.keys()).collect();
    for action in actions {
        let (b, a) = (current_keymap.get(action), keymap.get(action));
        if b != a {
            let list = |k: Option<&Vec<String>>| {
                k.map(|k| Value::Array(k.iter().cloned().map(Value::String).collect()))
            };
            changes.push(Change {
                area: ChangeArea::Keymap,
                path: action.clone(),
                before: list(b),
                after: list(a),
            });
        }
    }
    Ok(ImportPlan {
        settings_changed: settings != *current,
        keymap_changed: keymap != *current_keymap,
        settings,
        keymap,
        changes,
        warnings,
    })
}

/// Appends a [`Change`] for every leaf that differs, descending into
/// objects (so `speech.speed_presets.skim` is its own line).
fn diff(before: Option<&Value>, after: Option<&Value>, path: &str, out: &mut Vec<Change>) {
    if before == after {
        return;
    }
    let is_obj = |v: Option<&Value>| v.is_none_or(Value::is_object);
    let is_date = |v: Option<&Value>| {
        v.and_then(Value::as_object)
            .is_some_and(|m| m.contains_key(DATETIME_KEY))
    };
    if !path.is_empty()
        && (before.is_some() || after.is_some())
        && is_obj(before)
        && is_obj(after)
        && !is_date(before)
        && !is_date(after)
    {
        let empty = Map::new();
        let b = before.and_then(Value::as_object).unwrap_or(&empty);
        let a = after.and_then(Value::as_object).unwrap_or(&empty);
        let n = out.len();
        let keys: BTreeSet<&String> = b.keys().chain(a.keys()).collect();
        for k in keys {
            diff(b.get(k), a.get(k), &join(path, k), out);
        }
        if out.len() == n {
            out.push(Change {
                area: ChangeArea::Settings,
                path: path.to_owned(),
                before: before.cloned(),
                after: after.cloned(),
            });
        }
        return;
    }
    if path.is_empty() {
        let empty = Map::new();
        let b = before.and_then(Value::as_object).unwrap_or(&empty);
        let a = after.and_then(Value::as_object).unwrap_or(&empty);
        let keys: BTreeSet<&String> = b.keys().chain(a.keys()).collect();
        for k in keys {
            diff(b.get(k), a.get(k), k, out);
        }
        return;
    }
    out.push(Change {
        area: ChangeArea::Settings,
        path: path.to_owned(),
        before: before.cloned(),
        after: after.cloned(),
    });
}

// ---------------------------------------------------------------------------
// The store side

impl SettingsStore {
    /// The current settings and key overrides for import and export, with
    /// warnings for invalid values in `settings.toml` (which use their
    /// defaults). Unlike [`load`](Self::load), an unparsable settings file
    /// is an error here and is not moved aside.
    pub fn load_for_io(&self) -> Result<(Settings, KeymapOverrides, Vec<String>), SettingsIoError> {
        let path = self.paths().settings_file();
        let (settings, warnings) = match std::fs::read(&path) {
            Ok(bytes) => {
                let table = std::str::from_utf8(&bytes)
                    .map_err(|e| e.to_string())
                    .and_then(|t| t.parse::<toml::Table>().map_err(|e| e.message().to_owned()))
                    .map_err(|message| SettingsIoError::CurrentUnreadable {
                        path: path.clone(),
                        message: message.trim().trim_end_matches('.').to_owned(),
                    })?;
                let (s, w) = Settings::from_table(table);
                let w = w
                    .into_iter()
                    .map(|w| format!("In your current settings, {w}."))
                    .collect();
                (s, w)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Settings::default(), Vec::new()),
            Err(source) => return Err(StoreError::Io { path, source }.into()),
        };
        let keymap = self.load_keymap()?;
        Ok((settings, keymap, warnings))
    }

    /// Exports the current settings and key overrides.
    pub fn export(&self, options: ExportOptions) -> Result<String, SettingsIoError> {
        let (settings, keymap, _) = self.load_for_io()?;
        export_settings(&settings, &keymap, options, crate::now_ts())
    }

    /// Plans importing `text` over the current files; see [`plan_import`].
    pub fn plan_import(&self, text: &str, mode: ImportMode) -> Result<ImportPlan, SettingsIoError> {
        let (settings, keymap, load_warnings) = self.load_for_io()?;
        let mut plan = plan_import(&settings, &keymap, text, mode)?;
        plan.warnings.splice(0..0, load_warnings);
        Ok(plan)
    }

    /// Plans a reset of everything or of one section; see [`plan_reset`].
    pub fn plan_reset(&self, section: Option<&str>) -> Result<ImportPlan, SettingsIoError> {
        let (settings, keymap, load_warnings) = self.load_for_io()?;
        let mut plan = plan_reset(&settings, &keymap, section)?;
        plan.warnings.splice(0..0, load_warnings);
        Ok(plan)
    }

    /// Writes a plan: each file that changes is first copied to
    /// `<name>.bak-<UTC stamp>` (when it exists), then written atomically.
    /// A plan with no changes writes nothing.
    pub fn apply(&self, plan: &ImportPlan) -> Result<Applied, StoreError> {
        let stamp = crate::time::file_stamp(crate::now_ts());
        let mut applied = Applied::default();
        if plan.settings_changed {
            let path = self.paths().settings_file();
            applied.backups.extend(backup(&path, &stamp)?);
            self.save(&plan.settings)?;
            applied.written.push(path);
        }
        if plan.keymap_changed {
            let path = self.paths().keymap_file();
            applied.backups.extend(backup(&path, &stamp)?);
            self.save_keymap(&plan.keymap)?;
            applied.written.push(path);
        }
        Ok(applied)
    }
}

/// Copies `path` to `<name>.bak-<stamp>` (with `-2`, `-3`, ... when taken).
/// Returns `None` when `path` does not exist.
fn backup(path: &Path, stamp: &str) -> Result<Option<PathBuf>, StoreError> {
    if !path.exists() {
        return Ok(None);
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut target = path.with_file_name(format!("{name}.bak-{stamp}"));
    let mut n = 2;
    while target.exists() {
        target = path.with_file_name(format!("{name}.bak-{stamp}-{n}"));
        n += 1;
    }
    std::fs::copy(path, &target).map_err(|source| StoreError::Io {
        path: target.clone(),
        source,
    })?;
    Ok(Some(target))
}

#[cfg(test)]
#[path = "settings_io_tests.rs"]
mod tests;
