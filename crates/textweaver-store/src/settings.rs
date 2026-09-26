//! `settings.toml` and `keymap.toml`.
//!
//! The settings surface mirrors Star's reading-relevant keys
//! (docs/star-parity.md, "Settings surface") grouped into TOML tables. Every
//! table keeps unknown keys in `extra`, so a newer or older textweaver never
//! loses a user's settings.
//!
//! Loading is lenient: one invalid value falls back to its default and is
//! reported, instead of resetting the whole file. Saving is atomic, happens
//! only when the app asks, and stores only values that differ from the
//! defaults.

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
    /// Defaults to `"eloquence"` (ETI-Eloquence), Star's default
    /// `tts_prefer_voice`; `None` means no preference and is stored as
    /// `prefer_voice = ""` so it survives a reload.
    #[serde(with = "empty_is_none")]
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
    /// `[speech.eci]`: ETI-Eloquence through the ECI host (ADR-0007).
    pub eci: EciSettings,
    /// `[speech.sapi]`: Windows SAPI5 voices (ADR-0009).
    pub sapi: SapiSettings,
    /// `[speech.apple]`: Apple speech on macOS (ADR-0008).
    pub apple: AppleSettings,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

/// Which pronunciation dictionaries the ECI backend loads (ADR-0007): the
/// community IBMTTS dictionaries shipped with textweaver, none, or the
/// user's own directory. Stored as `dictionaries = true`, `false`, or a
/// path string (`"on"` and `"off"` are read too).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum EciDictionaries {
    /// The bundled community dictionaries (the default).
    #[default]
    On,
    /// No dictionaries.
    Off,
    /// Dictionaries from this directory (same layout as the bundled ones).
    Path(PathBuf),
}

impl Serialize for EciDictionaries {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            EciDictionaries::On => s.serialize_bool(true),
            EciDictionaries::Off => s.serialize_bool(false),
            EciDictionaries::Path(p) => s.serialize_str(&p.to_string_lossy()),
        }
    }
}

impl<'de> Deserialize<'de> for EciDictionaries {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Bool(bool),
            Text(String),
        }
        Ok(match Raw::deserialize(d)? {
            Raw::Bool(true) => EciDictionaries::On,
            Raw::Bool(false) => EciDictionaries::Off,
            Raw::Text(t) => match t.trim().to_ascii_lowercase().as_str() {
                "on" | "true" | "yes" | "" => EciDictionaries::On,
                "off" | "false" | "no" | "none" => EciDictionaries::Off,
                _ => EciDictionaries::Path(PathBuf::from(t.trim())),
            },
        })
    }
}

/// `[speech.eci]`: the Eloquence (ECI) backend. The app maps these onto the
/// ECI backend's options at integration; the environment variables
/// (`TEXTWEAVER_ECI_LIBRARY`, `TEXTWEAVER_ECI_DICTIONARIES`,
/// `TEXTWEAVER_ECI_CODE_FACTORY`) still win for one run.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EciSettings {
    /// Pronunciation dictionaries: on (bundled), off, or a directory.
    pub dictionaries: EciDictionaries,
    /// The ECI library to load; unset searches the usual places (an
    /// OpenEVV installation, Voxin on Linux, and Code Factory when
    /// `code_factory` is on).
    pub library: Option<PathBuf>,
    /// Also search Code Factory's "Eloquence for Windows" location. Off by
    /// default: an installed copy is not necessarily licensed for use by
    /// other programs, so textweaver loads it only when the user says so
    /// (ADR-0007).
    pub code_factory: bool,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

/// `[speech.sapi]`: Windows SAPI5 voices.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SapiSettings {
    /// Also list the OneCore voices (Microsoft Mark, David, Zira) through
    /// SAPI5. On by default.
    pub onecore: bool,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

impl Default for SapiSettings {
    fn default() -> Self {
        SapiSettings {
            onecore: true,
            extra: toml::Table::new(),
        }
    }
}

/// Which Apple speech backend to prefer on macOS.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppleBackend {
    /// Let textweaver choose (ADR-0008 picks after measuring on a Mac).
    #[default]
    Auto,
    /// `NSSpeechSynthesizer`: the most responsive; the system plays audio.
    NsSpeech,
    /// `AVSpeechSynthesizer`: textweaver plays audio, so word timing
    /// follows the audio clock.
    AvSpeech,
}

impl AppleBackend {
    /// The backend id the speech registry uses, or `None` for automatic.
    pub fn backend_id(self) -> Option<&'static str> {
        match self {
            AppleBackend::Auto => None,
            AppleBackend::NsSpeech => Some("nsspeech"),
            AppleBackend::AvSpeech => Some("avspeech"),
        }
    }
}

/// `[speech.apple]`: Apple speech on macOS.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppleSettings {
    /// Backend preference.
    pub backend: AppleBackend,
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
            prefer_voice: Some("eloquence".to_owned()),
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
            eci: EciSettings::default(),
            sapi: SapiSettings::default(),
            apple: AppleSettings::default(),
            extra: toml::Table::new(),
        }
    }
}

/// `Option<String>` stored as a string, with `""` for `None`. Used where the
/// default is `Some`, since TOML has no null and an omitted key would bring
/// the default back.
mod empty_is_none {
    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(v: &Option<String>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(v.as_deref().unwrap_or(""))
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
        let s = String::deserialize(d)?;
        Ok((!s.is_empty()).then_some(s))
    }
}

/// Settings that are stored but deliberately not read yet, as
/// `(section.field, reason)`. A test (`tests/settings_used.rs`) fails when a
/// setting is read nowhere and is not listed here, and when a listed one is
/// read after all (Star's lesson: a stored setting must work).
pub const RESERVED_SETTINGS: &[(&str, &str)] = &[
    (
        "reading_aids.syllables",
        "syllable display is built in textweaver-aids, but no frontend draws it yet (Agent D3's follow-up)",
    ),
    (
        "reading_aids.syllable_options",
        "options for the syllable display, which no frontend draws yet",
    ),
];

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
    /// Word highlight color laid over the theme's spoken-word style: a name
    /// (`cyan`, `yellow`, ...) or `#rrggbb`; `"theme"` (the default) keeps
    /// the theme's own.
    pub color: String,
    /// Sentence highlight color laid over the theme's spoken-sentence
    /// style; `None` (or `"theme"`) keeps the theme's own.
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
            color: "theme".into(),
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
    /// How explicit spoken math is: `low` ("a over b"), `normal` (the
    /// default, ClearSpeak style: "the fraction with numerator ... and
    /// denominator ..."), or `high` (adds end markers such as "end
    /// fraction").
    pub math_verbosity: Verbosity,
    /// The character written around ASCIIMath, usually a backtick
    /// (`` asciimath_delimiter = "`" ``). Unset, the default, reads no
    /// ASCIIMath, because in Markdown a backtick marks code.
    pub asciimath_delimiter: Option<char>,
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
    /// `[normalization.community_lexicon]`: the IBMTTS community
    /// pronunciation dictionaries applied as a lexicon, for engines that do
    /// not normalize natively (Eloquence loads them itself).
    pub community_lexicon: CommunityLexiconSettings,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

/// `[normalization.community_lexicon]`. The app maps it onto the speech
/// service's `CommunityLexiconConfig`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CommunityLexiconSettings {
    /// Apply the lexicon (off by default).
    pub enabled: bool,
    /// Directory holding the `.dic` files; unset searches beside the
    /// program and `TEXTWEAVER_ECI_DICTIONARIES`.
    pub dir: Option<PathBuf>,
    /// ECI's three-letter language code in the file names: `ENU` (US
    /// English, the default) or `DEU` (German).
    pub language: String,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

impl Default for CommunityLexiconSettings {
    fn default() -> Self {
        CommunityLexiconSettings {
            enabled: false,
            dir: None,
            language: "ENU".to_owned(),
            extra: toml::Table::new(),
        }
    }
}

/// Subtitle file format for audio export.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubtitleFormat {
    /// SubRip (`.srt`), Star's default.
    #[default]
    Srt,
    /// WebVTT (`.vtt`).
    Vtt,
}

impl SubtitleFormat {
    /// The file extension, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            SubtitleFormat::Srt => "srt",
            SubtitleFormat::Vtt => "vtt",
        }
    }
}

/// `[export]`: audio export (`tw export-audio`), Star's `subtitle_format`,
/// `subtitle_word_level`, and `export_subtitles_with_audio`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportSettings {
    /// Subtitle format when subtitles are written without a file name.
    pub subtitle_format: SubtitleFormat,
    /// One subtitle cue per word instead of caption lines.
    pub subtitle_word_level: bool,
    /// Always write subtitles beside exported audio (Star's
    /// `export_subtitles_with_audio`), named like the audio file with the
    /// subtitle format's extension.
    pub subtitles_with_audio: bool,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

impl Default for NormalizationSettings {
    fn default() -> Self {
        NormalizationSettings {
            math: true,
            math_verbosity: Verbosity::Normal,
            asciimath_delimiter: None,
            abbreviations: true,
            abbrev_expansions: BTreeMap::new(),
            numbers: true,
            use_pronunciations: true,
            pronunciations: BTreeMap::new(),
            table_mode: TableMode::default(),
            footnote_mode: FootnoteMode::default(),
            community_lexicon: CommunityLexiconSettings::default(),
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
    /// Theme name (Galaxy by default; see `docs/themes.md`).
    pub theme: String,
    /// Follow the system's light, dark, or high-contrast setting at startup
    /// (Star's `follow_os_theme`), unless a theme was chosen explicitly.
    pub follow_os_theme: bool,
    /// Set when the user picked a theme; stops following the system.
    pub theme_explicit: bool,
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
            follow_os_theme: true,
            theme_explicit: false,
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

/// Keyboard settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyboardSettings {
    /// Single-key shortcuts (browse keys such as `.` and `a`). Off means
    /// printable keys and Space never trigger commands, so dictation,
    /// speech recognition, and switch keyboards cannot set them off by
    /// accident (WCAG 2.1.4). The app passes this to
    /// `Keymap::set_character_keys`.
    pub character_keys: bool,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

impl Default for KeyboardSettings {
    fn default() -> Self {
        KeyboardSettings {
            character_keys: true,
            extra: toml::Table::new(),
        }
    }
}

/// `[reading_aids]`: RSVP, bionic reading, text spacing, fonts, the
/// reading ruler, and syllable splitting (`textweaver-aids`, ADR-0022).
/// The option types are the aids crate's own, so every frontend reads the
/// same values.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReadingAidsSettings {
    /// `[reading_aids.rsvp]`: rate, pauses, context words, position.
    pub rsvp: textweaver_aids::RsvpSettings,
    /// Bionic reading on (off by default).
    pub bionic: bool,
    /// `[reading_aids.bionic_options]`: ratio and what to skip.
    pub bionic_options: textweaver_aids::BionicOptions,
    /// `[reading_aids.spacing]`: line height, paragraph, letter, and word
    /// spacing in multiples of the font size (the terminal shows the
    /// nearest whole rows and spaces).
    pub spacing: textweaver_aids::TextSpacing,
    /// `[reading_aids.font]`: family, size, and weight (the GUI's).
    pub font: textweaver_aids::FontSettings,
    /// `[reading_aids.ruler]`: off, current line, or ruler.
    pub ruler: textweaver_aids::RulerSettings,
    /// Syllable splitting on (off by default).
    pub syllables: bool,
    /// `[reading_aids.syllable_options]`.
    pub syllable_options: textweaver_aids::SyllableOptions,
    /// Unknown keys, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
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
    /// `[keyboard]`
    pub keyboard: KeyboardSettings,
    /// `[export]`
    pub export: ExportSettings,
    /// `[reading_aids]`
    pub reading_aids: ReadingAidsSettings,
    /// Unknown top-level keys and tables, preserved.
    #[serde(flatten)]
    pub extra: toml::Table,
}

/// Keymap overrides as stored: action id to chord strings, for example
/// `next_sentence = ["Alt+.", "."]`. An empty list unbinds the action.
pub type KeymapOverrides = BTreeMap<String, Vec<String>>;

/// The result of loading `settings.toml`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SettingsLoad {
    /// The settings to use.
    pub settings: Settings,
    /// Set when the file could not be used at all: it was unreadable or not
    /// TOML, so defaults are in effect. The message says where the backup
    /// is. Show and announce it once.
    pub error: Option<String>,
    /// Individual values that were invalid or out of range and were replaced
    /// by defaults or clamped. The rest of the file was used.
    pub warnings: Vec<String>,
}

impl SettingsLoad {
    /// The error and warnings as one message for the user, if there is
    /// anything to say.
    pub fn message(&self) -> Option<String> {
        let mut parts: Vec<String> = self.error.iter().cloned().collect();
        if !self.warnings.is_empty() {
            parts.push(format!(
                "Some settings were invalid and use their defaults: {}",
                self.warnings.join("; ")
            ));
        }
        (!parts.is_empty()).then(|| parts.join(". "))
    }
}

/// Deserializes one settings table leniently: the whole table when it is
/// valid, otherwise key by key, dropping (and reporting) the keys whose
/// values do not fit. Unknown keys always survive in the table's `extra`.
fn lenient_section<T>(name: &str, value: Option<toml::Value>, warnings: &mut Vec<String>) -> T
where
    T: serde::de::DeserializeOwned + Default,
{
    let Some(value) = value else {
        return T::default();
    };
    let toml::Value::Table(table) = value else {
        warnings.push(format!("[{name}] is not a table"));
        return T::default();
    };
    if let Ok(v) = toml::Value::Table(table.clone()).try_into::<T>() {
        return v;
    }
    let mut good = toml::Table::new();
    for (key, v) in table {
        let mut candidate = good.clone();
        candidate.insert(key.clone(), v);
        if toml::Value::Table(candidate.clone())
            .try_into::<T>()
            .is_ok()
        {
            good = candidate;
        } else {
            warnings.push(format!("{name}.{key} has an invalid value"));
        }
    }
    toml::Value::Table(good).try_into().unwrap_or_default()
}

impl Settings {
    /// Builds settings from a parsed TOML table, keeping every valid value,
    /// replacing invalid ones with defaults, and preserving unknown keys.
    /// Returns the warnings for the replaced values, then applies
    /// [`validate`](Self::validate).
    pub fn from_table(table: toml::Table) -> (Settings, Vec<String>) {
        let (mut s, mut w) = Settings::from_table_unclamped(table);
        w.extend(s.validate());
        (s, w)
    }

    /// [`from_table`](Self::from_table) without clamping out-of-range
    /// values, so an import can report them as errors instead.
    pub(crate) fn from_table_unclamped(mut table: toml::Table) -> (Settings, Vec<String>) {
        let mut w = Vec::new();
        // Engine sub-tables are read on their own, so one bad value in
        // `[speech.eci]` does not throw away the rest of `[speech]`.
        let mut speech_table = table.remove("speech");
        let mut sub = |name: &str| match &mut speech_table {
            Some(toml::Value::Table(t)) => t.remove(name),
            _ => None,
        };
        let (eci, sapi, apple) = (sub("eci"), sub("sapi"), sub("apple"));
        let mut speech: SpeechSettings = lenient_section("speech", speech_table, &mut w);
        speech.eci = lenient_section("speech.eci", eci, &mut w);
        speech.sapi = lenient_section("speech.sapi", sapi, &mut w);
        speech.apple = lenient_section("speech.apple", apple, &mut w);
        let mut normalization_table = table.remove("normalization");
        let lexicon = match &mut normalization_table {
            Some(toml::Value::Table(t)) => t.remove("community_lexicon"),
            _ => None,
        };
        let mut normalization: NormalizationSettings =
            lenient_section("normalization", normalization_table, &mut w);
        normalization.community_lexicon =
            lenient_section("normalization.community_lexicon", lexicon, &mut w);
        let s = Settings {
            speech,
            highlight: lenient_section("highlight", table.remove("highlight"), &mut w),
            normalization,
            reading: lenient_section("reading", table.remove("reading"), &mut w),
            display: lenient_section("display", table.remove("display"), &mut w),
            editing: lenient_section("editing", table.remove("editing"), &mut w),
            library: lenient_section("library", table.remove("library"), &mut w),
            keyboard: lenient_section("keyboard", table.remove("keyboard"), &mut w),
            export: lenient_section("export", table.remove("export"), &mut w),
            reading_aids: lenient_section("reading_aids", table.remove("reading_aids"), &mut w),
            extra: table,
        };
        (s, w)
    }

    /// Clamps values to their supported ranges. Returns a message for each
    /// value changed. Star validated nothing, so a bad value failed later
    /// (docs/star-parity.md Part 3 §7 items 4 and 7).
    pub fn validate(&mut self) -> Vec<String> {
        self.fix_ranges()
            .into_iter()
            .map(|f| format!("{} {}; using {}", f.path, f.problem, f.using))
            .collect()
    }

    /// Clamps values to their supported ranges, describing each change.
    pub(crate) fn fix_ranges(&mut self) -> Vec<RangeFix> {
        let mut w = Vec::new();
        let mut fix = |path: String, problem: String, using: String| {
            w.push(RangeFix {
                path,
                problem,
                using,
            });
        };
        let rate = self.speech.rate.clamped();
        if rate != self.speech.rate {
            fix(
                "speech.rate".into(),
                format!(
                    "{} is outside {} to {} words per minute",
                    self.speech.rate.wpm(),
                    Rate::MIN_WPM,
                    Rate::MAX_WPM
                ),
                rate.wpm().to_string(),
            );
            self.speech.rate = rate;
        }
        let volume = Volume::new(self.speech.volume.percent());
        if volume != self.speech.volume {
            fix(
                "speech.volume".into(),
                format!(
                    "{} is outside 0 to 100 percent",
                    self.speech.volume.percent()
                ),
                volume.percent().to_string(),
            );
            self.speech.volume = volume;
        }
        let pitch = self.speech.pitch.clamped();
        if pitch != self.speech.pitch {
            fix(
                "speech.pitch".into(),
                format!(
                    "{} is outside {} to {} semitones",
                    self.speech.pitch.semitones(),
                    Pitch::MIN_SEMITONES,
                    Pitch::MAX_SEMITONES
                ),
                pitch.semitones().to_string(),
            );
            self.speech.pitch = pitch;
        }
        for (name, wpm) in &mut self.speech.speed_presets {
            let c = (*wpm).clamp(Rate::MIN_WPM, Rate::MAX_WPM);
            if c != *wpm {
                fix(
                    format!("speech.speed_presets.{name}"),
                    format!(
                        "{wpm} is outside {} to {} words per minute",
                        Rate::MIN_WPM,
                        Rate::MAX_WPM
                    ),
                    c.to_string(),
                );
                *wpm = c;
            }
        }
        let lead = self.highlight.lead_words.clamp(-5, 5);
        if lead != self.highlight.lead_words {
            fix(
                "highlight.lead_words".into(),
                format!("{} is outside -5 to 5", self.highlight.lead_words),
                lead.to_string(),
            );
            self.highlight.lead_words = lead;
        }
        let speed = self.highlight.speed;
        let fixed = if speed.is_finite() {
            speed.clamp(0.5, 1.5)
        } else {
            1.0
        };
        if !speed.is_finite() || (fixed - speed).abs() > f32::EPSILON {
            fix(
                "highlight.speed".into(),
                format!("{speed} is outside 0.5 to 1.5"),
                fixed.to_string(),
            );
            self.highlight.speed = fixed;
        }
        let mut at_least = |value: &mut usize, min: usize, name: &str| {
            if *value < min {
                fix(
                    name.to_owned(),
                    format!("{value} is below {min}"),
                    min.to_string(),
                );
                *value = min;
            }
        };
        at_least(
            &mut self.reading.nav_history_size,
            1,
            "reading.nav_history_size",
        );
        at_least(&mut self.library.recent_limit, 1, "library.recent_limit");
        if self.display.tab_width == 0 {
            fix(
                "display.tab_width".into(),
                "0 is below 1".into(),
                "4".into(),
            );
            self.display.tab_width = 4;
        }
        if let Err(e) = self.reading_aids.font.validate() {
            let fixed = self.reading_aids.font.clamped();
            fix(
                "reading_aids.font".into(),
                e.to_string(),
                fixed.summary().trim_end_matches('.').to_owned(),
            );
            self.reading_aids.font = fixed;
        }
        if self.editing.autosave_interval_secs < 5 {
            fix(
                "editing.autosave_interval_secs".into(),
                format!("{} is below 5", self.editing.autosave_interval_secs),
                "5".into(),
            );
            self.editing.autosave_interval_secs = 5;
        }
        w
    }

    /// The settings as TOML, keeping only values that differ from the
    /// defaults, plus every unknown key. A default that changes in a later
    /// release therefore reaches users who never changed it (Star wrote every
    /// default and needed migrations to fix old ones, Part 3 §7 item 2).
    pub fn to_minimal_toml(&self) -> Result<String, toml::ser::Error> {
        let full = toml::Table::try_from(self)?;
        let defaults = toml::Table::try_from(Settings::default())?;
        let out = minimal_table(full, &defaults, "");
        let body = toml::to_string_pretty(&out)?;
        Ok(format!("{SETTINGS_HEADER}{body}"))
    }
}

/// A value outside its supported range, clamped by
/// [`Settings::fix_ranges`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RangeFix {
    /// Dotted path, such as `speech.rate`.
    pub(crate) path: String,
    /// What is wrong, such as `5000 is outside 50 to 900 words per minute`.
    pub(crate) problem: String,
    /// The value used instead.
    pub(crate) using: String,
}

/// Tables whose own keys are compared one by one with the defaults. Every
/// other table-valued setting (`speed_presets`, `pronunciations`, ...) is a
/// map that replaces its default as a whole, so it is stored whole.
pub(crate) const STRUCT_TABLES: [&str; 20] = [
    "keyboard",
    "reading_aids",
    "reading_aids.rsvp",
    "reading_aids.bionic_options",
    "reading_aids.spacing",
    "reading_aids.font",
    "reading_aids.ruler",
    "reading_aids.syllable_options",
    "export",
    "normalization.community_lexicon",
    "speech",
    "speech.eci",
    "speech.sapi",
    "speech.apple",
    "highlight",
    "normalization",
    "reading",
    "display",
    "editing",
    "library",
];

/// `table` without the values equal to `defaults`, descending into the
/// [`STRUCT_TABLES`]; empty sub-tables are dropped.
fn minimal_table(table: toml::Table, defaults: &toml::Table, path: &str) -> toml::Table {
    let mut out = toml::Table::new();
    for (key, value) in table {
        let sub_path = if path.is_empty() {
            key.clone()
        } else {
            format!("{path}.{key}")
        };
        match (value, defaults.get(&key)) {
            (toml::Value::Table(t), Some(toml::Value::Table(d)))
                if STRUCT_TABLES.contains(&sub_path.as_str()) =>
            {
                let kept = minimal_table(t, d, &sub_path);
                if !kept.is_empty() {
                    out.insert(key, toml::Value::Table(kept));
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

const SETTINGS_HEADER: &str = "\
# textweaver settings. Only values that differ from the defaults are stored;
# remove a line to return to the default. Unknown keys are kept.

";

/// Loads and saves `settings.toml` and `keymap.toml`.
///
/// Nothing is written unless the caller saves: textweaver writes settings
/// only on an explicit change (Star rewrote the whole file on every `set`,
/// Part 3 §7 item 1).
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
    /// (Star's behavior, kept). Invalid individual values are replaced by
    /// their defaults and reported in the message too. Never panics.
    pub fn load(&self) -> (Settings, Option<String>) {
        let loaded = self.load_detailed();
        let message = loaded.message();
        (loaded.settings, message)
    }

    /// Loads settings, reporting the file-level error and per-value warnings
    /// separately.
    pub fn load_detailed(&self) -> SettingsLoad {
        let path = self.paths.settings_file();
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return SettingsLoad::default(),
            Err(e) => {
                return SettingsLoad {
                    error: Some(format!(
                        "Settings file could not be read ({e}); using defaults for this session"
                    )),
                    ..SettingsLoad::default()
                };
            }
        };
        let parsed = std::str::from_utf8(&bytes)
            .map_err(|e| e.to_string())
            .and_then(|t| t.parse::<toml::Table>().map_err(|e| e.message().to_owned()));
        match parsed {
            Ok(table) => {
                let (settings, warnings) = Settings::from_table(table);
                SettingsLoad {
                    settings,
                    error: None,
                    warnings,
                }
            }
            Err(reason) => {
                let backup = backup_path(&path);
                let saved = std::fs::copy(&path, &backup).is_ok();
                let mut msg = format!(
                    "Settings file was unreadable and has been reset to defaults ({})",
                    reason.trim()
                );
                if saved {
                    msg.push_str(&format!("; backup saved to {}", backup.display()));
                }
                SettingsLoad {
                    settings: Settings::default(),
                    error: Some(msg),
                    warnings: Vec::new(),
                }
            }
        }
    }

    /// Saves settings atomically, storing only non-default values. Call only
    /// on an explicit change.
    pub fn save(&self, settings: &Settings) -> Result<(), StoreError> {
        let path = self.paths.settings_file();
        let text = settings.to_minimal_toml().map_err(|e| StoreError::Parse {
            path: path.clone(),
            message: e.to_string(),
        })?;
        atomic_write(&path, text.as_bytes())
    }

    /// Saves `settings` only when they differ from `previous` (the settings
    /// as last loaded or saved). Returns whether a write happened.
    pub fn save_if_changed(
        &self,
        settings: &Settings,
        previous: &Settings,
    ) -> Result<bool, StoreError> {
        if settings == previous {
            return Ok(false);
        }
        self.save(settings).map(|()| true)
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

/// `settings.toml.corrupt-YYYYmmdd-HHMMSS.bak` (UTC), with a counter when
/// that name is taken.
fn backup_path(path: &std::path::Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "settings.toml".to_owned());
    let stamp = crate::time::file_stamp(crate::now_ts());
    let mut candidate = path.with_file_name(format!("{name}.corrupt-{stamp}.bak"));
    let mut n = 2;
    while candidate.exists() {
        candidate = path.with_file_name(format!("{name}.corrupt-{stamp}-{n}.bak"));
        n += 1;
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, SettingsStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(Paths::under(dir.path()));
        std::fs::create_dir_all(&store.paths().config_dir).unwrap();
        (dir, store)
    }

    fn write(store: &SettingsStore, text: &str) {
        std::fs::write(store.paths().settings_file(), text).unwrap();
    }

    fn config_files(store: &SettingsStore) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(&store.paths().config_dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    #[test]
    fn round_trip_preserves_unknown_keys() {
        let (_d, store) = store();
        write(
            &store,
            "future_key = 1\n[speech]\nrate = 300\nnew_engine_option = \"x\"\n[future_table]\na = 1\n",
        );
        let (s, err) = store.load();
        assert!(err.is_none());
        assert_eq!(s.speech.rate, Rate::Wpm(300));
        assert_eq!(s.speech.volume, Volume::default());
        store.save(&s).unwrap();
        let text = std::fs::read_to_string(store.paths().settings_file()).unwrap();
        assert!(text.contains("future_key"));
        assert!(text.contains("new_engine_option"));
        assert!(text.contains("[future_table]"));
        assert_eq!(store.load().0, s);
    }

    /// Wave 2 additions for the app (Agent D3): the community lexicon and
    /// audio export options, with Star's defaults, read leniently, and
    /// stored only when changed.
    #[test]
    fn lexicon_and_export_settings_default_round_trip_and_stay_minimal() {
        let s = Settings::default();
        assert!(!s.normalization.community_lexicon.enabled);
        assert_eq!(s.normalization.community_lexicon.dir, None);
        assert_eq!(s.normalization.community_lexicon.language, "ENU");
        assert_eq!(s.export.subtitle_format, SubtitleFormat::Srt);
        assert!(!s.export.subtitle_word_level);
        assert!(!s.export.subtitles_with_audio);
        let text = s.to_minimal_toml().unwrap();
        assert!(!text.contains("community_lexicon") && !text.contains("[export]"));

        let (_d, store) = store();
        write(
            &store,
            "[normalization]\nnumbers = false\n[normalization.community_lexicon]\nenabled = true\nlanguage = \"DEU\"\nfuture = 1\n[export]\nsubtitle_format = \"vtt\"\nsubtitle_word_level = true\nsubtitles_with_audio = true\n",
        );
        let (s, err) = store.load();
        assert!(err.is_none(), "{err:?}");
        assert!(!s.normalization.numbers);
        let lex = &s.normalization.community_lexicon;
        assert!(lex.enabled);
        assert_eq!(lex.language, "DEU");
        assert_eq!(lex.extra["future"].as_integer(), Some(1));
        assert_eq!(s.export.subtitle_format, SubtitleFormat::Vtt);
        assert_eq!(s.export.subtitle_format.extension(), "vtt");
        assert!(s.export.subtitle_word_level && s.export.subtitles_with_audio);
        store.save(&s).unwrap();
        assert_eq!(store.load().0, s);
        let text = std::fs::read_to_string(store.paths().settings_file()).unwrap();
        assert!(
            !text.contains("dir"),
            "unchanged values are not written: {text}"
        );

        // A bad value costs only itself.
        write(
            &store,
            "[normalization.community_lexicon]\nenabled = \"yes\"\nlanguage = \"DEU\"\n[export]\nsubtitle_format = \"ass\"\nsubtitle_word_level = true\n",
        );
        let (s, _) = store.load();
        assert!(!s.normalization.community_lexicon.enabled);
        assert_eq!(s.normalization.community_lexicon.language, "DEU");
        assert_eq!(s.export.subtitle_format, SubtitleFormat::Srt);
        assert!(s.export.subtitle_word_level);
    }

    /// Spoken math settings (Agent V): normal verbosity and no ASCIIMath by
    /// default, stored only when changed, and a bad value costs only itself.
    #[test]
    fn math_settings_default_round_trip_and_stay_minimal() {
        let s = Settings::default();
        assert!(s.normalization.math);
        assert_eq!(s.normalization.math_verbosity, Verbosity::Normal);
        assert_eq!(s.normalization.asciimath_delimiter, None);
        let text = s.to_minimal_toml().unwrap();
        assert!(!text.contains("math_verbosity") && !text.contains("asciimath"));

        let (_d, store) = store();
        write(
            &store,
            "[normalization]\nmath_verbosity = \"high\"\nasciimath_delimiter = \"`\"\n",
        );
        let (s, err) = store.load();
        assert!(err.is_none(), "{err:?}");
        assert_eq!(s.normalization.math_verbosity, Verbosity::High);
        assert_eq!(s.normalization.asciimath_delimiter, Some('`'));
        store.save(&s).unwrap();
        assert_eq!(store.load().0, s);
        let text = std::fs::read_to_string(store.paths().settings_file()).unwrap();
        assert!(text.contains("math_verbosity = \"high\""), "{text}");
        assert!(text.contains("asciimath_delimiter = \"`\""), "{text}");
        assert!(
            !text.contains("numbers"),
            "unchanged values stay out: {text}"
        );

        // Invalid values fall back to the defaults, one by one.
        write(
            &store,
            "[normalization]\nmath_verbosity = \"loud\"\nasciimath_delimiter = \"``\"\nnumbers = false\n",
        );
        let (s, err) = store.load();
        assert_eq!(s.normalization.math_verbosity, Verbosity::Normal);
        assert_eq!(s.normalization.asciimath_delimiter, None);
        assert!(!s.normalization.numbers);
        let err = err.unwrap_or_default();
        assert!(err.contains("normalization.math_verbosity"), "{err}");
        assert!(err.contains("normalization.asciimath_delimiter"), "{err}");
    }

    #[test]
    fn theme_following_defaults_and_round_trip() {
        let s = Settings::default();
        assert_eq!(s.display.theme, "galaxy");
        assert!(s.display.follow_os_theme);
        assert!(!s.display.theme_explicit);
        let (_d, store) = store();
        let mut s = Settings::default();
        s.display.theme = "nord".into();
        s.display.theme_explicit = true;
        s.display.follow_os_theme = false;
        store.save(&s).unwrap();
        assert_eq!(store.load().0, s);
        assert_eq!(
            store.paths().themes_dir(),
            store.paths().config_dir.join("themes")
        );
    }

    #[test]
    fn reading_aids_default_round_trip_and_stay_minimal() {
        let s = Settings::default();
        assert!(!s.reading_aids.bionic && !s.reading_aids.syllables);
        assert_eq!(s.reading_aids.rsvp.wpm, 300);
        assert_eq!(s.reading_aids.ruler.mode, textweaver_aids::RulerMode::Off);
        let text = s.to_minimal_toml().unwrap();
        assert!(!text.contains("reading_aids"), "{text}");
        let (_d, store) = store();
        write(
            &store,
            "[reading_aids]\nbionic = true\n[reading_aids.rsvp]\nwpm = 450\nposition = \"center\"\n[reading_aids.ruler]\nmode = \"ruler\"\n[reading_aids.spacing]\nline_height = 2.0\n",
        );
        let (s, err) = store.load();
        assert!(err.is_none(), "{err:?}");
        let a = &s.reading_aids;
        assert!(a.bionic);
        assert_eq!(a.rsvp.wpm, 450);
        assert_eq!(a.rsvp.position, textweaver_aids::RsvpPosition::Center);
        assert_eq!(a.ruler.mode, textweaver_aids::RulerMode::Ruler);
        assert!((a.spacing.line_height - 2.0).abs() < 1e-6);
        store.save(&s).unwrap();
        assert_eq!(store.load().0, s);
        let text = std::fs::read_to_string(store.paths().settings_file()).unwrap();
        assert!(
            text.contains("wpm = 450") && !text.contains("clause_pause"),
            "{text}"
        );
    }

    #[test]
    fn corrupt_file_resets_with_message() {
        let (_d, store) = store();
        write(&store, "{ not toml");
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
        assert_eq!(s.speech.speed_presets["normal"], 265);
        assert_eq!(s.speech.speed_presets["study"], 200);
        assert_eq!(s.speech.speed_presets["slow"], 150);
        assert_eq!(s.display.theme, "galaxy");
        assert!(s.reading.auto_resume);
        assert!(s.speech.skip_code);
        assert_eq!(s.library.recent_limit, 20);
        assert_eq!(s.editing.autosave_interval_secs, 20);
    }

    /// Star's `tts_prefer_voice` default is `"eloquence"` (ETI-Eloquence).
    #[test]
    fn prefer_voice_defaults_to_eloquence_like_star() {
        let s = Settings::default();
        assert_eq!(s.speech.prefer_voice.as_deref(), Some("eloquence"));
        assert_eq!(s.speech.voice, None);
    }

    #[test]
    fn no_voice_preference_survives_a_reload() {
        let (_d, store) = store();
        let mut s = Settings::default();
        s.speech.prefer_voice = None;
        store.save(&s).unwrap();
        let text = std::fs::read_to_string(store.paths().settings_file()).unwrap();
        assert!(text.contains("prefer_voice = \"\""), "{text}");
        assert_eq!(store.load().0.speech.prefer_voice, None);
        s.speech.prefer_voice = Some("david".into());
        store.save(&s).unwrap();
        assert_eq!(store.load().0.speech.prefer_voice.as_deref(), Some("david"));
    }

    // ---- tests/test_settings.py, ported (docs/star-parity.md Part 3 §1.3) ----

    /// Star test 1, `test_save_writes_valid_json`
    #[test]
    fn star_01_save_writes_valid_toml() {
        let (_d, store) = store();
        let mut s = Settings::default();
        s.display.theme = "contrast".into();
        store.save(&s).unwrap();
        let text = std::fs::read_to_string(store.paths().settings_file()).unwrap();
        let table: toml::Table = text.parse().unwrap();
        assert_eq!(table["display"]["theme"].as_str(), Some("contrast"));
    }

    /// Star test 2, `test_save_leaves_no_temp_file_behind`
    #[test]
    fn star_02_save_leaves_no_temp_file_behind() {
        let (_d, store) = store();
        store.save(&Settings::default()).unwrap();
        assert_eq!(config_files(&store), vec!["settings.toml".to_owned()]);
    }

    /// Star test 3, `test_save_is_atomic_never_truncates_existing`
    #[test]
    fn star_03_save_is_atomic_never_truncates_existing() {
        let (_d, store) = store();
        let mut s = Settings::default();
        for width in [111, 222] {
            s.display.wrap_width = width;
            store.save(&s).unwrap();
            let text = std::fs::read_to_string(store.paths().settings_file()).unwrap();
            assert!(text.parse::<toml::Table>().is_ok());
        }
        assert_eq!(store.load().0.display.wrap_width, 222);
        assert_eq!(config_files(&store), vec!["settings.toml".to_owned()]);
    }

    /// Star test 4, `test_save_never_raises_on_unwritable_target`: the parent path is a
    /// regular file. textweaver returns the error instead of swallowing it,
    /// but never panics, and the target does not exist.
    #[test]
    fn star_04_save_on_unwritable_target_errors_without_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("config");
        std::fs::write(&blocker, "a file, not a directory").unwrap();
        let store = SettingsStore::new(Paths::under(dir.path()));
        assert!(store.save(&Settings::default()).is_err());
        assert!(!store.paths().settings_file().exists());
    }

    /// Star test 5, `test_load_reads_preexisting_file`
    #[test]
    fn star_05_load_reads_preexisting_file() {
        let (_d, store) = store();
        write(
            &store,
            "[display]\ntheme = \"solarized\"\nwrap_width = 1234\n[speech]\nrate = 300\n",
        );
        let s = store.load().0;
        assert_eq!(s.display.theme, "solarized");
        assert_eq!(s.speech.rate, Rate::Wpm(300));
        assert_eq!(s.display.wrap_width, 1234);
        assert_eq!(s.speech.volume, Volume::default());
    }

    /// Star test 6, `test_load_merges_nested_dict_with_defaults`. Deliberate change:
    /// Star merged `speed_presets` one level deep with the defaults, so a
    /// default preset could never be removed (Part 3 §7 item 6). A stored
    /// table replaces the default table; other settings keep their defaults.
    #[test]
    fn star_06_nested_table_replaces_default_table() {
        let (_d, store) = store();
        write(&store, "[speech.speed_presets]\nskim = 999\n");
        let s = store.load().0;
        assert_eq!(s.speech.speed_presets.get("skim"), Some(&900), "clamped");
        assert_eq!(s.speech.speed_presets.get("normal"), None);
        assert_eq!(s.speech.rate.wpm(), 265);
    }

    /// Star test 7, `test_load_missing_file_uses_defaults`
    #[test]
    fn star_07_load_missing_file_uses_defaults() {
        let (_d, store) = store();
        let (s, err) = store.load();
        assert_eq!(s.display.theme, "galaxy");
        assert!(err.is_none());
    }

    /// Star test 8, `test_load_corrupt_file_falls_back_to_defaults`
    #[test]
    fn star_08_load_corrupt_file_falls_back_to_defaults() {
        let (_d, store) = store();
        write(&store, "{ this is not valid json ");
        assert_eq!(store.load().0.display.theme, "galaxy");
    }

    /// Star test 9, `test_save_load_round_trip`
    #[test]
    fn star_09_save_load_round_trip() {
        let (_d, store) = store();
        let mut s = Settings::default();
        s.speech.rate = Rate::Wpm(275);
        s.library.folders = vec![PathBuf::from("/a"), PathBuf::from("/b")];
        store.save(&s).unwrap();
        let fresh = SettingsStore::new(store.paths().clone()).load().0;
        assert_eq!(fresh.speech.rate, Rate::Wpm(275));
        assert_eq!(fresh.library.folders, s.library.folders);
    }

    /// Star test 10, `test_corrupt_settings_backed_up_and_reported`
    #[test]
    fn star_10_corrupt_settings_backed_up_and_reported() {
        let (_d, store) = store();
        let garbage = "{ this is not toml";
        write(&store, garbage);
        let loaded = store.load_detailed();
        assert_eq!(loaded.settings, Settings::default());
        let err = loaded.error.unwrap();
        assert!(err.to_lowercase().contains("reset"));
        let backups: Vec<String> = config_files(&store)
            .into_iter()
            .filter(|n| n.starts_with("settings.toml.corrupt-") && n.ends_with(".bak"))
            .collect();
        assert_eq!(backups.len(), 1);
        let bak = store.paths().config_dir.join(&backups[0]);
        assert_eq!(std::fs::read_to_string(bak).unwrap(), garbage);
        assert!(err.contains(&backups[0]));
    }

    /// Star test 11, `test_clean_settings_have_no_load_error`
    #[test]
    fn star_11_clean_settings_have_no_load_error() {
        let (_d, store) = store();
        let loaded = store.load_detailed();
        assert!(loaded.error.is_none());
        assert!(loaded.warnings.is_empty());
        assert!(loaded.message().is_none());
    }

    // 12. `test_every_default_key_documented` checks Star's
    // docs/configuration.md. Here every settings field carries rustdoc
    // (`missing_docs` is denied in CI), the equivalent guarantee.

    // ---- Fixes for Star's settings bugs (Part 3 §7) ----

    #[test]
    fn one_bad_value_does_not_reset_the_rest() {
        let (_d, store) = store();
        write(
            &store,
            "[speech]\nrate = \"fast\"\nvolume = 40\n[display]\ntheme = \"nord\"\n",
        );
        let loaded = store.load_detailed();
        assert!(loaded.error.is_none());
        assert_eq!(loaded.settings.speech.rate, Rate::default());
        assert_eq!(loaded.settings.speech.volume, Volume::new(40));
        assert_eq!(loaded.settings.display.theme, "nord");
        assert_eq!(loaded.warnings, vec!["speech.rate has an invalid value"]);
        assert!(loaded.message().unwrap().contains("speech.rate"));
    }

    #[test]
    fn out_of_range_values_are_clamped_and_reported() {
        let (_d, store) = store();
        write(
            &store,
            "[speech]\nrate = 5000\npitch = 40\n[highlight]\nspeed = 9.0\nlead_words = -9\n",
        );
        let loaded = store.load_detailed();
        let s = &loaded.settings;
        assert_eq!(s.speech.rate.wpm(), Rate::MAX_WPM);
        assert_eq!(s.speech.pitch.semitones(), Pitch::MAX_SEMITONES);
        assert!((s.highlight.speed - 1.5).abs() < f32::EPSILON);
        assert_eq!(s.highlight.lead_words, -5);
        assert_eq!(loaded.warnings.len(), 4, "{:?}", loaded.warnings);
    }

    /// `Volume` deserializes as a bare number, so 150 got through before.
    #[test]
    fn volume_above_100_is_clamped_and_reported() {
        let (_d, store) = store();
        write(&store, "[speech]\nvolume = 150\n");
        let loaded = store.load_detailed();
        assert_eq!(loaded.settings.speech.volume, Volume::new(100));
        assert_eq!(
            loaded.warnings,
            vec!["speech.volume 150 is outside 0 to 100 percent; using 100"]
        );
    }

    #[test]
    fn non_table_section_and_non_utf8_file() {
        let (_d, store) = store();
        write(&store, "speech = 5\n");
        let loaded = store.load_detailed();
        assert!(loaded.error.is_none());
        assert_eq!(loaded.warnings, vec!["[speech] is not a table"]);

        std::fs::write(store.paths().settings_file(), [0xff, 0xfe, 0x00, 0x41]).unwrap();
        let loaded = store.load_detailed();
        assert!(loaded.error.unwrap().contains("reset"));
    }

    #[test]
    fn only_changed_values_are_written() {
        let (_d, store) = store();
        store.save(&Settings::default()).unwrap();
        let text = std::fs::read_to_string(store.paths().settings_file()).unwrap();
        let table: toml::Table = text.parse().unwrap();
        assert!(table.is_empty(), "defaults are not persisted: {text}");

        let mut s = Settings::default();
        s.speech.pitch = Pitch::Semitones(-2);
        store.save(&s).unwrap();
        let text = std::fs::read_to_string(store.paths().settings_file()).unwrap();
        let table: toml::Table = text.parse().unwrap();
        assert_eq!(table.len(), 1);
        assert_eq!(table["speech"].as_table().unwrap().len(), 1);
        assert_eq!(table["speech"]["pitch"].as_integer(), Some(-2));
        assert_eq!(store.load().0, s);
    }

    #[test]
    fn save_if_changed_skips_identical_settings() {
        let (_d, store) = store();
        let before = Settings::default();
        assert!(!store.save_if_changed(&before, &before).unwrap());
        assert!(!store.paths().settings_file().exists());
        let mut after = before.clone();
        after.speech.rate = Rate::Wpm(300);
        assert!(store.save_if_changed(&after, &before).unwrap());
        assert!(store.paths().settings_file().exists());
    }

    #[test]
    fn loading_never_writes() {
        let (_d, store) = store();
        let _ = store.load();
        assert!(config_files(&store).is_empty());
    }

    #[test]
    fn engine_tables_default_round_trip_and_stay_minimal() {
        let s = Settings::default();
        assert_eq!(s.speech.eci.dictionaries, EciDictionaries::On);
        assert!(!s.speech.eci.code_factory, "Code Factory is opt-in");
        assert!(s.speech.sapi.onecore);
        assert_eq!(s.speech.apple.backend, AppleBackend::Auto);

        let (_d, store) = store();
        let mut s = Settings::default();
        s.speech.eci.code_factory = true;
        s.speech.eci.dictionaries = EciDictionaries::Path(PathBuf::from("dicts"));
        s.speech.sapi.onecore = false;
        s.speech.apple.backend = AppleBackend::NsSpeech;
        store.save(&s).unwrap();
        let text = std::fs::read_to_string(store.paths().settings_file()).unwrap();
        let table: toml::Table = text.parse().unwrap();
        let eci = table["speech"]["eci"].as_table().unwrap();
        assert_eq!(eci.len(), 2, "only changed keys: {text}");
        assert_eq!(eci["code_factory"].as_bool(), Some(true));
        assert_eq!(eci["dictionaries"].as_str(), Some("dicts"));
        assert_eq!(table["speech"]["sapi"]["onecore"].as_bool(), Some(false));
        assert_eq!(
            table["speech"]["apple"]["backend"].as_str(),
            Some("nsspeech")
        );
        assert_eq!(table["speech"].as_table().unwrap().len(), 3, "{text}");
        assert_eq!(store.load().0, s);

        // Only one engine key changed: the others are not written.
        let mut s = Settings::default();
        s.speech.eci.dictionaries = EciDictionaries::Off;
        store.save(&s).unwrap();
        let text = std::fs::read_to_string(store.paths().settings_file()).unwrap();
        assert!(text.contains("dictionaries = false"), "{text}");
        assert!(
            !text.contains("sapi") && !text.contains("code_factory"),
            "{text}"
        );
        assert_eq!(store.load().0, s);
    }

    #[test]
    fn engine_tables_read_leniently() {
        let (_d, store) = store();
        write(
            &store,
            "[speech]\nrate = 300\n[speech.eci]\ndictionaries = \"off\"\ncode_factory = \"yes please\"\nfuture = 1\n[speech.apple]\nbackend = \"avspeech\"\n",
        );
        let loaded = store.load_detailed();
        let s = &loaded.settings;
        assert_eq!(
            s.speech.rate,
            Rate::Wpm(300),
            "the rest of [speech] survives"
        );
        assert_eq!(s.speech.eci.dictionaries, EciDictionaries::Off);
        assert!(!s.speech.eci.code_factory);
        assert!(s.speech.eci.extra.contains_key("future"));
        assert_eq!(s.speech.apple.backend.backend_id(), Some("avspeech"));
        assert_eq!(
            loaded.warnings,
            vec!["speech.eci.code_factory has an invalid value"]
        );
        write(&store, "[speech.eci]\ndictionaries = true\n");
        assert_eq!(store.load().0.speech.eci.dictionaries, EciDictionaries::On);
    }

    #[test]
    fn character_keys_default_on_and_round_trip() {
        let (_d, store) = store();
        assert!(Settings::default().keyboard.character_keys);
        let mut s = Settings::default();
        s.keyboard.character_keys = false;
        store.save(&s).unwrap();
        let text = std::fs::read_to_string(store.paths().settings_file()).unwrap();
        assert!(
            text.contains("[keyboard]\ncharacter_keys = false"),
            "{text}"
        );
        assert!(!store.load().0.keyboard.character_keys);
    }

    #[test]
    fn map_tables_are_stored_whole() {
        let (_d, store) = store();
        let mut s = Settings::default();
        s.speech.speed_presets.insert("skim".into(), 400);
        store.save(&s).unwrap();
        assert_eq!(store.load().0.speech.speed_presets.len(), 4);
    }

    #[test]
    fn display_font_round_trips_and_is_clamped() {
        let (_d, store) = store();
        let mut s = Settings::default();
        s.reading_aids.font.family = textweaver_aids::FontFamily::Named("OpenDyslexic".into());
        s.reading_aids.font.size_pt = 20.0;
        s.reading_aids.font.weight = 700;
        store.save(&s).unwrap();
        let text = std::fs::read_to_string(store.paths().settings_file()).unwrap();
        assert!(
            text.contains(
                "[reading_aids.font]\nfamily = \"OpenDyslexic\"\nsize_pt = 20.0\nweight = 700"
            ),
            "{text}"
        );
        assert_eq!(store.load().0.reading_aids.font, s.reading_aids.font);
        // Out of range: clamped, with a warning naming the setting.
        std::fs::write(
            store.paths().settings_file(),
            "[reading_aids.font]\nfamily = \"Verdana\"\nsize_pt = 500.0\n",
        )
        .unwrap();
        let load = store.load_detailed();
        assert_eq!(load.settings.reading_aids.font.size_pt, 144.0);
        assert!(
            load.warnings
                .iter()
                .any(|w| w.starts_with("reading_aids.font")),
            "{:?}",
            load.warnings
        );
    }

    #[test]
    fn keymap_overrides_round_trip() {
        let (_d, store) = store();
        assert!(store.load_keymap().unwrap().is_empty());
        let mut o = KeymapOverrides::new();
        o.insert("next_sentence".into(), vec!["Alt+.".into(), ".".into()]);
        o.insert("stop".into(), vec![]);
        store.save_keymap(&o).unwrap();
        assert_eq!(store.load_keymap().unwrap(), o);
    }
}
