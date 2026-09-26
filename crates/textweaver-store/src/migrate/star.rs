//! Reading a Star configuration directory: `settings.json`, the parse
//! cache (for Star's `plain_text`), and the mapping of Star's settings keys
//! onto textweaver's (docs/history/star-parity.md Part 3 §1).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use textweaver_core::{HighlightGranularity, Rate, Volume};

use crate::settings::{FootnoteMode, TableMode};
use crate::sync::ConflictPolicy;
use crate::{Settings, StoreError};

/// Star's configuration directory on this platform: `%APPDATA%\star` on
/// Windows, `~/Library/Application Support/star` on macOS, and
/// `$XDG_CONFIG_HOME/star` (default `~/.config/star`) elsewhere.
pub fn default_star_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|d| d.config_dir().join("star"))
}

/// Star's `settings.json`, parsed.
pub fn read_settings(dir: &Path) -> Result<Map<String, Value>, StoreError> {
    let path = dir.join("settings.json");
    let bytes = std::fs::read(&path).map_err(|source| StoreError::Io {
        path: path.clone(),
        source,
    })?;
    let text = String::from_utf8_lossy(&bytes);
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Object(m)) => Ok(m),
        Ok(_) => Err(StoreError::Parse {
            path,
            message: "not a JSON object".to_owned(),
        }),
        Err(e) => Err(StoreError::Parse {
            path,
            message: e.to_string(),
        }),
    }
}

/// Star's timestamps (`2026-09-25T10:03:07`, local time without a zone)
/// as Unix seconds. The zone is unknown, so they are read as UTC; this
/// shifts them by at most the local offset.
pub fn star_ts(v: Option<&Value>) -> i64 {
    match v {
        Some(Value::String(s)) => crate::time::parse_timestamp(s).unwrap_or(0),
        Some(Value::Number(n)) => n.as_f64().map_or(0, |f| f as i64),
        _ => 0,
    }
}

/// An int-like JSON number (not a bool).
pub fn as_usize(v: Option<&Value>) -> Option<usize> {
    match v {
        Some(Value::Number(n)) => n
            .as_u64()
            .map(|u| usize::try_from(u).unwrap_or(usize::MAX))
            .or_else(|| n.as_f64().filter(|f| *f >= 0.0).map(|f| f as usize)),
        _ => None,
    }
}

/// Star's parse cache, `<star>/cache/*_v1.json`: each file holds one
/// document's `plain_text`, keyed by the path Star opened. Only entries
/// whose recorded modification time still matches the file are used.
#[derive(Clone, Debug, Default)]
pub struct StarCache {
    by_path: HashMap<PathBuf, (PathBuf, f64)>,
}

fn file_mtime(path: &Path) -> Option<f64> {
    let t = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs_f64())
}

impl StarCache {
    /// Indexes the cache directory (reads each entry once, keeps only its
    /// path and modification time).
    pub fn index(star_dir: &Path) -> Self {
        let mut by_path = HashMap::new();
        let Ok(entries) = std::fs::read_dir(star_dir.join("cache")) else {
            return StarCache::default();
        };
        for entry in entries.filter_map(Result::ok) {
            let file = entry.path();
            let name = file.file_name().map(|n| n.to_string_lossy().into_owned());
            if !name.is_some_and(|n| n.ends_with("_v1.json")) {
                continue;
            }
            let Some(payload) = std::fs::read_to_string(&file)
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            else {
                continue;
            };
            let (Some(p), Some(mtime)) = (
                payload.get("path").and_then(Value::as_str),
                payload.get("mtime").and_then(Value::as_f64),
            ) else {
                continue;
            };
            by_path.insert(crate::library::resolve_path(Path::new(p)), (file, mtime));
        }
        StarCache { by_path }
    }

    /// Number of cached documents.
    pub fn len(&self) -> usize {
        self.by_path.len()
    }

    /// True when nothing is cached.
    pub fn is_empty(&self) -> bool {
        self.by_path.is_empty()
    }

    /// Star's `plain_text` for `doc`, when cached and still current.
    pub fn plain_text(&self, doc: &Path) -> Option<String> {
        let (file, mtime) = self.by_path.get(&crate::library::resolve_path(doc))?;
        let current = file_mtime(doc)?;
        if (current - mtime).abs() > 1e-3 {
            return None;
        }
        let payload: Value = serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()?;
        payload
            .get("data")?
            .get("plain_text")?
            .as_str()
            .map(str::to_owned)
    }
}

/// What happened to one Star setting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingOutcome {
    /// Imported, with a description of the textweaver setting.
    Imported(String),
    /// Same as Star's default: nothing to import.
    Default,
    /// Already the value textweaver has.
    Unchanged,
    /// Not imported, and why.
    Skipped(String),
}

/// Theme renames Star applied on load (`star/themes.py`).
fn theme_alias(name: &str) -> &str {
    match name {
        "obsidian" => "galaxy",
        "obsidian-light" => "galaxy-light",
        "zed-one-dark" => "one-dark",
        "zed-one-light" => "one-light",
        other => other,
    }
}

/// Star's `tts_backend` as a textweaver backend id.
fn backend_for(star: &str) -> Result<&'static str, String> {
    match star {
        "auto" => Ok("auto"),
        "espeak" => Ok("espeak"),
        "none" | "silent" => Ok("null"),
        "pyttsx3" if cfg!(windows) => Ok("sapi"),
        "pyttsx3" | "applesay" if cfg!(target_os = "macos") => Ok("nsspeech"),
        "pyttsx3" => Ok("espeak"),
        other => Err(format!(
            "textweaver has no {other} backend; automatic selection is used"
        )),
    }
}

fn want_bool(v: &Value) -> Result<bool, String> {
    v.as_bool().ok_or_else(|| "not true or false".to_owned())
}

fn want_str(v: &Value) -> Result<&str, String> {
    v.as_str().ok_or_else(|| "not text".to_owned())
}

fn want_num(v: &Value) -> Result<f64, String> {
    match v {
        Value::Number(n) => n.as_f64().ok_or_else(|| "not a number".to_owned()),
        _ => Err("not a number".to_owned()),
    }
}

fn want_str_map(v: &Value) -> Result<std::collections::BTreeMap<String, String>, String> {
    let m = v.as_object().ok_or_else(|| "not a table".to_owned())?;
    Ok(m.iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_owned())))
        .collect())
}

type Apply = fn(&Value, &mut Settings) -> Result<String, String>;

/// One Star setting textweaver imports: its key, Star's default, and how
/// to apply a changed value.
struct Rule {
    key: &'static str,
    default: fn() -> Value,
    apply: Apply,
}

macro_rules! rule {
    ($key:literal, $default:tt, |$v:ident, $s:ident| $body:expr) => {
        Rule {
            key: $key,
            default: || serde_json::json!($default),
            apply: |$v: &Value, $s: &mut Settings| -> Result<String, String> { $body },
        }
    };
}

fn rules() -> Vec<Rule> {
    vec![
        rule!("tts_backend", "auto", |v, s| {
            let id = backend_for(want_str(v)?)?;
            s.speech.backend = id.to_owned();
            Ok(format!("speech.backend = {id}"))
        }),
        rule!("tts_rate", 265, |v, s| {
            let wpm = want_num(v)?.round().clamp(0.0, f64::from(u16::MAX)) as u16;
            s.speech.rate = Rate::Wpm(wpm.clamp(Rate::MIN_WPM, Rate::MAX_WPM));
            Ok(format!("speech.rate = {}", s.speech.rate.wpm()))
        }),
        rule!("tts_volume", 1.0, |v, s| {
            let pct = (want_num(v)?.clamp(0.0, 1.0) * 100.0).round() as u8;
            s.speech.volume = Volume::new(pct);
            Ok(format!("speech.volume = {pct}"))
        }),
        rule!("tts_prefer_voice", "eloquence", |v, s| {
            let p = want_str(v)?.trim();
            s.speech.prefer_voice = (!p.is_empty()).then(|| p.to_owned());
            Ok(format!("speech.prefer_voice = \"{p}\""))
        }),
        rule!("tts_voice", "", |v, _s| Err(format!(
            "voice {} is one of Star's engine ids, which textweaver's engines do not share; choose the voice again",
            v
        ))),
        rule!("tts_favorite_voices", [], |_v, _s| Err(
            "favorite voices are Star engine ids; star them again".to_owned()
        )),
        rule!("tts_auto_play", false, |v, s| {
            s.speech.auto_play = want_bool(v)?;
            Ok(format!("speech.auto_play = {}", s.speech.auto_play))
        }),
        rule!("tts_skip_code", true, |v, s| {
            s.speech.skip_code = want_bool(v)?;
            Ok(format!("speech.skip_code = {}", s.speech.skip_code))
        }),
        rule!(
            "speed_presets",
            {"skim": 350, "normal": 265, "study": 200, "slow": 150},
            |v, s| {
                let m = v.as_object().ok_or_else(|| "not a table".to_owned())?;
                let presets: std::collections::BTreeMap<String, u16> = m
                    .iter()
                    .filter_map(|(k, v)| {
                        let w = v.as_f64()?.round().clamp(0.0, f64::from(u16::MAX)) as u16;
                        Some((k.clone(), w.clamp(Rate::MIN_WPM, Rate::MAX_WPM)))
                    })
                    .collect();
                if presets.is_empty() {
                    return Err("no usable presets".to_owned());
                }
                s.speech.speed_presets = presets;
                Ok(format!("speech.speed_presets ({} presets)", m.len()))
            }
        ),
        rule!("espeak_highlight_offset_ms", 120, |v, s| {
            s.speech.latency_offset_ms = want_num(v)?.clamp(0.0, 5000.0) as u32;
            Ok(format!(
                "speech.latency_offset_ms = {}",
                s.speech.latency_offset_ms
            ))
        }),
        rule!("pronunciations", {}, |v, s| {
            let m = want_str_map(v)?;
            let n = m.len();
            s.normalization.pronunciations.extend(m);
            Ok(format!("normalization.pronunciations ({n} entries)"))
        }),
        rule!("use_pronunciations", true, |v, s| {
            s.normalization.use_pronunciations = want_bool(v)?;
            Ok(format!(
                "normalization.use_pronunciations = {}",
                s.normalization.use_pronunciations
            ))
        }),
        rule!("highlight_current_word", true, |v, s| {
            s.highlight.enabled = want_bool(v)?;
            Ok(format!("highlight.enabled = {}", s.highlight.enabled))
        }),
        rule!("highlight_color", "cyan", |v, s| {
            s.highlight.color = want_str(v)?.to_owned();
            Ok(format!("highlight.color = \"{}\"", s.highlight.color))
        }),
        rule!("sentence_highlight_color", "", |v, s| {
            let c = want_str(v)?.trim();
            s.highlight.sentence_color = (!c.is_empty()).then(|| c.to_owned());
            Ok(format!("highlight.sentence_color = \"{c}\""))
        }),
        rule!("highlight_lead_words", 1, |v, s| {
            s.highlight.lead_words = want_num(v)?.round().clamp(-5.0, 5.0) as i8;
            Ok(format!("highlight.lead_words = {}", s.highlight.lead_words))
        }),
        rule!("highlight_granularity", "word", |v, s| {
            s.highlight.granularity = match want_str(v)? {
                "word" => HighlightGranularity::Word,
                "sentence" => HighlightGranularity::Sentence,
                "both" => HighlightGranularity::Both,
                other => return Err(format!("unknown granularity {other}")),
            };
            Ok(format!("highlight.granularity = {}", want_str(v)?))
        }),
        rule!("highlight_speed", 1.0, |v, s| {
            s.highlight.speed = want_num(v)?.clamp(0.5, 1.5) as f32;
            Ok(format!("highlight.speed = {}", s.highlight.speed))
        }),
        rule!("normalize_math", true, |v, s| {
            s.normalization.math = want_bool(v)?;
            Ok(format!("normalization.math = {}", s.normalization.math))
        }),
        rule!("expand_abbreviations", true, |v, s| {
            s.normalization.abbreviations = want_bool(v)?;
            Ok(format!(
                "normalization.abbreviations = {}",
                s.normalization.abbreviations
            ))
        }),
        rule!("abbrev_expansions", {}, |v, s| {
            let m = want_str_map(v)?;
            let n = m.len();
            s.normalization.abbrev_expansions.extend(m);
            Ok(format!("normalization.abbrev_expansions ({n} entries)"))
        }),
        rule!("normalize_numbers", true, |v, s| {
            s.normalization.numbers = want_bool(v)?;
            Ok(format!(
                "normalization.numbers = {}",
                s.normalization.numbers
            ))
        }),
        rule!("table_reading_mode", "structured", |v, s| {
            s.normalization.table_mode = match want_str(v)? {
                "structured" => TableMode::Structured,
                "flat" => TableMode::Flat,
                "skip" => TableMode::Skip,
                other => return Err(format!("unknown table mode {other}")),
            };
            Ok(format!("normalization.table_mode = {}", want_str(v)?))
        }),
        rule!("footnote_mode", "inline", |v, s| {
            s.normalization.footnote_mode = match want_str(v)? {
                "inline" => FootnoteMode::Inline,
                "deferred" => FootnoteMode::Deferred,
                "skip" => FootnoteMode::Skip,
                other => return Err(format!("unknown footnote mode {other}")),
            };
            Ok(format!("normalization.footnote_mode = {}", want_str(v)?))
        }),
        rule!("tts_auto_resume", true, |v, s| {
            s.reading.auto_resume = want_bool(v)?;
            Ok(format!("reading.auto_resume = {}", s.reading.auto_resume))
        }),
        rule!("nav_history_size", 50, |v, s| {
            s.reading.nav_history_size = want_num(v)?.clamp(1.0, 10_000.0) as usize;
            Ok(format!(
                "reading.nav_history_size = {}",
                s.reading.nav_history_size
            ))
        }),
        rule!("sync_conflict_policy", "newest", |v, s| {
            s.reading.sync_conflict_policy = ConflictPolicy::parse(want_str(v)?);
            Ok(format!(
                "reading.sync_conflict_policy = {}",
                s.reading.sync_conflict_policy.as_str()
            ))
        }),
        rule!("tui_caret_follow_speech", true, |v, s| {
            s.reading.cursor_follows_speech = want_bool(v)?;
            Ok(format!(
                "reading.cursor_follows_speech = {}",
                s.reading.cursor_follows_speech
            ))
        }),
        rule!("theme", "galaxy", |v, s| {
            s.display.theme = theme_alias(want_str(v)?).to_owned();
            Ok(format!("display.theme = \"{}\"", s.display.theme))
        }),
        rule!("wrap_width", 0, |v, s| {
            s.display.wrap_width = want_num(v)?.clamp(0.0, 10_000.0) as u16;
            Ok(format!("display.wrap_width = {}", s.display.wrap_width))
        }),
        rule!("tab_width", 4, |v, s| {
            s.display.tab_width = want_num(v)?.clamp(1.0, 16.0) as u8;
            Ok(format!("display.tab_width = {}", s.display.tab_width))
        }),
        rule!("show_line_numbers", false, |v, s| {
            s.display.show_line_numbers = want_bool(v)?;
            Ok(format!(
                "display.show_line_numbers = {}",
                s.display.show_line_numbers
            ))
        }),
        rule!("scroll_margin", 3, |v, s| {
            s.display.scroll_margin = want_num(v)?.clamp(0.0, 100.0) as u16;
            Ok(format!(
                "display.scroll_margin = {}",
                s.display.scroll_margin
            ))
        }),
        rule!("autosave_recovery", true, |v, s| {
            s.editing.autosave_recovery = want_bool(v)?;
            Ok(format!(
                "editing.autosave_recovery = {}",
                s.editing.autosave_recovery
            ))
        }),
        rule!("recent_files_limit", 20, |v, s| {
            s.library.recent_limit = want_num(v)?.clamp(1.0, 1000.0) as usize;
            Ok(format!("library.recent_limit = {}", s.library.recent_limit))
        }),
    ]
}

/// Star keys handled elsewhere in the migration (documents, library,
/// keys), not by [`apply_settings`].
pub const STATE_KEYS: [&str; 11] = [
    "reading_positions",
    "bookmarks",
    "annotations",
    "user_highlights",
    "reading_stats",
    "library",
    "library_folders",
    "recent_files",
    "last_path",
    "keybindings",
    "annotation_filter_presets",
];

/// Applies the Star settings textweaver understands to `settings`. Values
/// equal to Star's defaults are left alone, so textweaver's own defaults
/// keep applying. Returns each handled key with what happened, and the
/// keys textweaver has no equivalent for.
pub fn apply_settings(
    star: &Map<String, Value>,
    settings: &mut Settings,
) -> (Vec<(String, SettingOutcome)>, Vec<String>) {
    let mut out = Vec::new();
    let rules = rules();
    for rule in &rules {
        let Some(v) = star.get(rule.key) else {
            continue;
        };
        let outcome = if crate::sync::json_eq(v, &(rule.default)()) {
            SettingOutcome::Default
        } else {
            let before = settings.clone();
            match (rule.apply)(v, settings) {
                Ok(_) if *settings == before => SettingOutcome::Unchanged,
                Ok(desc) => SettingOutcome::Imported(desc),
                Err(why) => SettingOutcome::Skipped(why),
            }
        };
        out.push((rule.key.to_owned(), outcome));
    }
    let mut unknown: Vec<String> = star
        .keys()
        .filter(|k| !rules.iter().any(|r| r.key == k.as_str()) && !STATE_KEYS.contains(&k.as_str()))
        .cloned()
        .collect();
    unknown.sort();
    (out, unknown)
}

/// Star's GUI default shortcuts (the keys of its `keybindings` remaps)
/// and the textweaver action each belongs to (docs/history/star-parity.md Part 1
/// §6.1). Star's other shortcuts have no textweaver action.
pub const STAR_SHORTCUTS: [(&str, &str); 43] = [
    ("Ctrl+N", "new_document"),
    ("Ctrl+O", "open"),
    ("Ctrl+Shift+B", "open_library"),
    ("Ctrl+Q", "quit"),
    ("Ctrl+Shift+1", "highlight_selection"),
    ("Ctrl+Shift+A", "add_note"),
    ("Ctrl+Shift+D", "delete_note"),
    ("Ctrl+Shift+N", "list_notes"),
    ("Ctrl+M", "add_bookmark"),
    ("Space", "play_pause"),
    ("Escape", "stop"),
    ("Esc", "stop"),
    ("Ctrl+Space", "read_from_cursor"),
    ("Ctrl+=", "rate_up"),
    ("Ctrl+-", "rate_down"),
    ("Ctrl+Shift+V", "choose_voice"),
    ("Tab", "speech_cursor_toggle"),
    ("Alt+.", "next_sentence"),
    ("Alt+,", "previous_sentence"),
    ("Alt+;", "replay_sentence"),
    ("Ctrl+P", "next_paragraph"),
    ("Ctrl+Shift+P", "previous_paragraph"),
    ("Ctrl+R", "replay_paragraph"),
    ("Ctrl+H", "next_heading"),
    ("Ctrl+Shift+H", "previous_heading"),
    ("Ctrl+T", "next_table"),
    ("Ctrl+Shift+T", "previous_table"),
    ("Alt+Left", "history_back"),
    ("Alt+Right", "history_forward"),
    ("Ctrl+B", "bold"),
    ("Ctrl+I", "italic"),
    ("Ctrl+U", "underline"),
    ("Ctrl+K", "insert_link"),
    ("Ctrl+Z", "undo"),
    ("Ctrl+Y", "redo"),
    ("Ctrl+F", "find"),
    ("Ctrl+E", "toggle_edit_mode"),
    ("Ctrl+S", "save"),
    ("F5", "next_theme"),
    ("F2", "command_palette"),
    ("F3", "keyboard_help"),
    ("F1", "help"),
    ("F7", "caret_browsing"),
];

/// The textweaver action for one of Star's default shortcuts. Matching
/// ignores case and spaces (`ctrl+shift+p`).
pub fn action_for_star_shortcut(shortcut: &str) -> Option<&'static str> {
    let norm = |s: &str| s.replace(' ', "").to_lowercase();
    let wanted = norm(shortcut);
    STAR_SHORTCUTS
        .iter()
        .find(|(k, _)| norm(k) == wanted)
        .map(|(_, a)| *a)
        .filter(|a| *a != "caret_browsing")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_changed_settings_are_applied() {
        let star: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "tts_rate": 300,
            "tts_volume": 1.0,
            "theme": "obsidian",
            "tts_backend": "festival",
            "tts_voice": "HKEY_LOCAL_MACHINE\\voice",
            "highlight_granularity": "both",
            "speed_presets": {"skim": 999, "normal": 265, "study": 200, "slow": 150},
            "sync_conflict_policy": "manual",
            "qt_font_size": 18,
            "reading_positions": {},
            "tts_skip_code": "yes"
        }))
        .unwrap();
        let mut s = Settings::default();
        let (outcomes, unknown) = apply_settings(&star, &mut s);
        let get = |k: &str| {
            outcomes
                .iter()
                .find(|(key, _)| key == k)
                .map(|(_, o)| o.clone())
        };
        assert_eq!(s.speech.rate, Rate::Wpm(300));
        assert!(matches!(get("tts_rate"), Some(SettingOutcome::Imported(_))));
        assert_eq!(get("tts_volume"), Some(SettingOutcome::Default));
        assert_eq!(s.display.theme, "galaxy", "theme alias applied");
        assert!(matches!(
            get("tts_backend"),
            Some(SettingOutcome::Skipped(_))
        ));
        assert_eq!(s.speech.backend, "auto");
        assert!(matches!(get("tts_voice"), Some(SettingOutcome::Skipped(_))));
        assert_eq!(s.highlight.granularity, HighlightGranularity::Both);
        assert_eq!(s.speech.speed_presets["skim"], 900, "clamped");
        assert_eq!(s.reading.sync_conflict_policy, ConflictPolicy::Manual);
        assert!(matches!(
            get("tts_skip_code"),
            Some(SettingOutcome::Skipped(_))
        ));
        assert_eq!(unknown, vec!["qt_font_size"]);
    }

    #[test]
    fn star_shortcuts_map_to_actions() {
        assert_eq!(
            action_for_star_shortcut("ctrl+shift+p"),
            Some("previous_paragraph")
        );
        assert_eq!(action_for_star_shortcut("Alt+."), Some("next_sentence"));
        assert_eq!(action_for_star_shortcut("F7"), None, "no textweaver action");
        assert_eq!(action_for_star_shortcut("Ctrl+Alt+E"), None);
    }

    #[test]
    fn timestamps_and_numbers() {
        assert_eq!(
            star_ts(Some(&Value::from("2026-09-25T14:03:07"))),
            1_790_344_987
        );
        assert_eq!(star_ts(Some(&Value::from("legacy"))), 0);
        assert_eq!(star_ts(None), 0);
        assert_eq!(as_usize(Some(&Value::from(5))), Some(5));
        assert_eq!(as_usize(Some(&Value::from(true))), None);
        assert_eq!(as_usize(Some(&Value::from(-1))), None);
    }

    #[test]
    fn cache_entries_are_used_only_while_current() {
        let dir = tempfile::tempdir().unwrap();
        let doc = dir.path().join("book.md");
        std::fs::write(&doc, "# Book\n\ntext").unwrap();
        let mtime = file_mtime(&doc).unwrap();
        let cache = dir.path().join("star").join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        let payload = serde_json::json!({
            "version": 1, "path": doc.to_string_lossy(), "mtime": mtime,
            "settings_fingerprint": "abcd1234",
            "data": {"plain_text": "Book text", "markdown": "# Book\n\ntext"}
        });
        std::fs::write(
            cache.join("0123456789abcdef_abcd1234_v1.json"),
            payload.to_string(),
        )
        .unwrap();
        std::fs::write(cache.join("junk_v1.json"), "not json").unwrap();
        let idx = StarCache::index(&dir.path().join("star"));
        assert_eq!(idx.len(), 1);
        assert_eq!(idx.plain_text(&doc).as_deref(), Some("Book text"));
        let stale = serde_json::json!({
            "version": 1, "path": doc.to_string_lossy(), "mtime": mtime - 100.0,
            "data": {"plain_text": "old"}
        });
        std::fs::write(
            cache.join("0123456789abcdef_abcd1234_v1.json"),
            stale.to_string(),
        )
        .unwrap();
        assert_eq!(
            StarCache::index(&dir.path().join("star")).plain_text(&doc),
            None
        );
        assert!(StarCache::index(dir.path()).is_empty());
    }
}
