//! `settings.toml` and `keymap.toml`.
//!
//! The settings surface mirrors Star's reading-relevant keys
//! (docs/star-parity.md, "Settings surface") grouped into TOML tables. Every
//! table keeps unknown keys in `extra`, so a newer or older textweaver never
//! loses a user's settings.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use textweaver_core::{
    CapsIndication, HighlightGranularity, Pitch, PunctuationLevel, Rate, Verbosity, Volume,
};

use crate::sync::ConflictPolicy;
use crate::{Paths, StoreError, atomic_write};

/// Speech settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpeechSettings {
    /// Backend id, or `"auto"`.
    pub backend: String,
    /// Speaking rate (Star default 265 wpm).
    pub rate: Rate,
    /// Volume.
    pub volume: Volume,
    /// Pitch (Star had none).
    pub pitch: Pitch,
    /// Voice id; `None` resolves automatically.
    pub voice: Option<String>,
    /// Substring used to pick a default voice when `voice` is unset.
    pub prefer_voice: Option<String>,
    /// Starred voices.
    pub favorite_voices: Vec<String>,
    /// Punctuation verbosity.
    pub punctuation: PunctuationLevel,
    /// Speak capitals within words separately.
    pub split_caps: bool,
    /// How capitals are indicated for characters and typing echo.
    pub caps: CapsIndication,
    /// Start reading when a document opens.
    pub auto_play: bool,
    /// Do not speak code blocks.
    pub skip_code: bool,
    /// Named rate presets.
    pub speed_presets: BTreeMap<String, u16>,
    /// Latency offset for audio-clock word events, in ms (Star 120).
    pub latency_offset_ms: u32,
    /// Announcement verbosity.
    pub verbosity: Verbosity,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

impl Default for SpeechSettings {
    fn default() -> Self {
        SpeechSettings {
            backend: "auto".into(),
            rate: Rate::default(),
            volume: Volume::default(),
            pitch: Pitch::default(),
            voice: None,
            prefer_voice: None,
            favorite_voices: Vec::new(),
            punctuation: PunctuationLevel::default(),
            split_caps: false,
            caps: CapsIndication::default(),
            auto_play: false,
            skip_code: true,
            speed_presets: [
                ("skim", 350),
                ("normal", 265),
                ("study", 200),
                ("slow", 150),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
            latency_offset_ms: 120,
            verbosity: Verbosity::default(),
            extra: toml::Table::new(),
        }
    }
}

/// Reading highlight settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HighlightSettings {
    /// Highlight the spoken text at all.
    pub enabled: bool,
    /// Word, sentence, or both.
    pub granularity: HighlightGranularity,
    /// Visual lead (positive) or lag (negative) in words, -5..=5.
    pub lead_words: i8,
    /// Timer pacing speed multiplier, 0.5..=1.5.
    pub speed: f32,
    /// Word highlight color (name or `#rrggbb`).
    pub color: String,
    /// Sentence highlight color; `None` uses the theme's selection color.
    pub sentence_color: Option<String>,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

impl Default for HighlightSettings {
    fn default() -> Self {
        HighlightSettings {
            enabled: true,
            granularity: HighlightGranularity::default(),
            lead_words: 1,
            speed: 1.0,
            color: "cyan".into(),
            sentence_color: None,
            extra: toml::Table::new(),
        }
    }
}

/// How tables are read aloud.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableMode {
    /// With row and column context.
    #[default]
    Structured,
    /// Cell text only.
    Flat,
    /// Not at all.
    Skip,
}

/// Where footnotes are read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FootnoteMode {
    /// Where referenced (a no-op in Star; implemented here).
    #[default]
    Inline,
    /// At the end of the section.
    Deferred,
    /// Not at all.
    Skip,
}

/// Text normalization settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NormalizationSettings {
    /// Speak math notation.
    pub math: bool,
    /// Expand abbreviations.
    pub abbreviations: bool,
    /// User abbreviation expansions, `"abbrev." = "expansion"`.
    pub abbrev_expansions: BTreeMap<String, String>,
    /// Numbers, dates, times, and currency to words.
    pub numbers: bool,
    /// Apply the pronunciation lexicon.
    pub use_pronunciations: bool,
    /// Pronunciation lexicon, `term = "spoken form"`.
    pub pronunciations: BTreeMap<String, String>,
    /// Table narration.
    pub table_mode: TableMode,
    /// Footnote narration.
    pub footnote_mode: FootnoteMode,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

impl Default for NormalizationSettings {
    fn default() -> Self {
        NormalizationSettings {
            math: true,
            abbreviations: true,
            abbrev_expansions: BTreeMap::new(),
            numbers: true,
            use_pronunciations: true,
            pronunciations: BTreeMap::new(),
            table_mode: TableMode::default(),
            footnote_mode: FootnoteMode::default(),
            extra: toml::Table::new(),
        }
    }
}

/// Reading and navigation settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReadingSettings {
    /// Restore the saved position when a document opens.
    pub auto_resume: bool,
    /// Back/forward history capacity.
    pub nav_history_size: usize,
    /// Navigation wraps around the document ends.
    pub wrap_navigation: bool,
    /// Cursor follows the spoken word.
    pub cursor_follows_speech: bool,
    /// Sidecar merge policy.
    pub sync_conflict_policy: ConflictPolicy,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

impl Default for ReadingSettings {
    fn default() -> Self {
        ReadingSettings {
            auto_resume: true,
            nav_history_size: 50,
            wrap_navigation: false,
            cursor_follows_speech: true,
            sync_conflict_policy: ConflictPolicy::default(),
            extra: toml::Table::new(),
        }
    }
}

/// Display settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplaySettings {
    /// Theme name.
    pub theme: String,
    /// Wrap width in columns; 0 is the terminal width.
    pub wrap_width: u16,
    /// Tab width.
    pub tab_width: u8,
    /// Show line numbers.
    pub show_line_numbers: bool,
    /// Context lines kept above and below the cursor.
    pub scroll_margin: u16,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        DisplaySettings {
            theme: "galaxy".into(),
            wrap_width: 0,
            tab_width: 4,
            show_line_numbers: false,
            scroll_margin: 3,
            extra: toml::Table::new(),
        }
    }
}

/// Editing settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EditingSettings {
    /// Write recovery snapshots and offer them at startup.
    pub autosave_recovery: bool,
    /// Seconds between snapshots while dirty (Star 20).
    pub autosave_interval_secs: u32,
    /// Speak each typed character.
    pub echo_characters: bool,
    /// Speak each completed word.
    pub echo_words: bool,
    /// Speak deleted text.
    pub echo_deletions: bool,
    /// Speak the line when the cursor moves between lines.
    pub echo_lines_on_move: bool,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

impl Default for EditingSettings {
    fn default() -> Self {
        EditingSettings {
            autosave_recovery: true,
            autosave_interval_secs: 20,
            echo_characters: true,
            echo_words: true,
            echo_deletions: true,
            echo_lines_on_move: true,
            extra: toml::Table::new(),
        }
    }
}

/// Library settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LibrarySettings {
    /// Maximum recent files.
    pub recent_limit: usize,
    /// Library folders.
    pub folders: Vec<PathBuf>,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

impl Default for LibrarySettings {
    fn default() -> Self {
        LibrarySettings {
            recent_limit: 20,
            folders: Vec::new(),
            extra: toml::Table::new(),
        }
    }
}

/// All settings, one TOML table per group.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// `[speech]`
    pub speech: SpeechSettings,
    /// `[highlight]`
    pub highlight: HighlightSettings,
    /// `[normalization]`
    pub normalization: NormalizationSettings,
    /// `[reading]`
    pub reading: ReadingSettings,
    /// `[display]`
    pub display: DisplaySettings,
    /// `[editing]`
    pub editing: EditingSettings,
    /// `[library]`
    pub library: LibrarySettings,
    /// Unknown top-level keys and tables, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

/// Keymap overrides as stored: action id to chord strings, for example
/// `read_next_sentence = ["Alt+.", "."]`. An empty list unbinds the action.
pub type KeymapOverrides = BTreeMap<String, Vec<String>>;

/// Loads and saves `settings.toml` and `keymap.toml`.
#[derive(Clone, Debug)]
pub struct SettingsStore {
    paths: Paths,
}

impl SettingsStore {
    /// A store over `paths`.
    pub fn new(paths: Paths) -> Self {
        SettingsStore { paths }
    }

    /// The paths in use.
    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    /// Loads settings. A missing file gives defaults. A corrupt file is
    /// copied aside and defaults are returned with a message for the user
    /// (Star's behavior, kept).
    pub fn load(&self) -> (Settings, Option<String>) {
        let path = self.paths.settings_file();
        let Ok(text) = std::fs::read_to_string(&path) else {
            return (Settings::default(), None);
        };
        match toml::from_str(&text) {
            Ok(s) => (s, None),
            Err(e) => {
                let backup = path.with_extension(format!("toml.corrupt-{}.bak", crate::now_ts()));
                let _ = std::fs::copy(&path, &backup);
                let msg = format!(
                    "Settings file was unreadable and has been reset to defaults ({}); backup saved to {}",
                    e.message(),
                    backup.display()
                );
                (Settings::default(), Some(msg))
            }
        }
    }

    /// Saves settings atomically. Call only on an explicit change.
    pub fn save(&self, settings: &Settings) -> Result<(), StoreError> {
        let path = self.paths.settings_file();
        let text = toml::to_string_pretty(settings).map_err(|e| StoreError::Parse {
            path: path.clone(),
            message: e.to_string(),
        })?;
        atomic_write(&path, text.as_bytes())
    }

    /// Loads keymap overrides (empty when the file is missing).
    pub fn load_keymap(&self) -> Result<KeymapOverrides, StoreError> {
        let path = self.paths.keymap_file();
        match std::fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).map_err(|e| StoreError::Parse {
                path,
                message: e.message().to_owned(),
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(KeymapOverrides::new()),
            Err(source) => Err(StoreError::Io { path, source }),
        }
    }

    /// Saves keymap overrides atomically.
    pub fn save_keymap(&self, overrides: &KeymapOverrides) -> Result<(), StoreError> {
        let path = self.paths.keymap_file();
        let text = toml::to_string_pretty(overrides).map_err(|e| StoreError::Parse {
            path: path.clone(),
            message: e.to_string(),
        })?;
        atomic_write(&path, text.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_unknown_keys() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(Paths::under(dir.path()));
        std::fs::create_dir_all(&store.paths().config_dir).unwrap();
        std::fs::write(
            store.paths().settings_file(),
            "future_key = 1\n[speech]\nrate = 300\nnew_engine_option = \"x\"\n",
        )
        .unwrap();
        let (s, err) = store.load();
        assert!(err.is_none());
        assert_eq!(s.speech.rate, Rate::Wpm(300));
        assert_eq!(s.speech.volume, Volume::default());
        store.save(&s).unwrap();
        let text = std::fs::read_to_string(store.paths().settings_file()).unwrap();
        assert!(text.contains("future_key"));
        assert!(text.contains("new_engine_option"));
    }

    #[test]
    fn corrupt_file_resets_with_message() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(Paths::under(dir.path()));
        std::fs::create_dir_all(&store.paths().config_dir).unwrap();
        std::fs::write(store.paths().settings_file(), "{ not toml").unwrap();
        let (s, err) = store.load();
        assert_eq!(s, Settings::default());
        assert!(err.unwrap().contains("reset"));
    }

    #[test]
    fn defaults_match_star() {
        let s = Settings::default();
        assert_eq!(s.speech.rate.wpm(), 265);
        assert_eq!(s.reading.nav_history_size, 50);
        assert_eq!(s.display.tab_width, 4);
        assert_eq!(s.speech.speed_presets["skim"], 350);
    }
}
