//! The settings schema and the settings screen (Wave 3, Agent W3a).
//!
//! # The schema
//!
//! [`SettingsSchema::generate`] walks the store's settings, as
//! [`settings_to_json`] writes them, so every key in `settings.toml` is in
//! it with its type and default, and joins each key with a label, a help
//! sentence, and its range, step, unit, or choices from [`INFO`]. Tests keep
//! the two in step: a key the store gains without an entry in [`INFO`], or
//! an entry for a key the store lost, fails the build's tests with the key
//! named. The same schema drives:
//!
//! - the terminal reader's settings screen (below);
//! - the GUI's settings dialog (Agent W3b), which builds a labelled
//!   control for each [`SettingKind`];
//! - JSON-RPC's `settings_schema`, `get_setting`, and `set_setting`.
//!
//! Values are JSON ([`serde_json::Value`]), the same shape as
//! `tw settings export`. [`App::set_setting`] checks a value against the
//! store's types and ranges (the store's `validate` clamps it, and says
//! so), puts it into effect at once (the voice, the theme, the keys, the
//! accessibility mode), and saves it through the writer thread.
//!
//! # The settings screen
//!
//! The Settings command (`Shift+F10` in the terminal, `Ctrl+,` in the GUI,
//! or "settings" in the command palette) lists every setting as "Label:
//! value", grouped by section, and filters as you type. Up and Down move;
//! Left and Right change the value (a smaller or larger number, the
//! previous or next choice, on or off); Enter turns a switch on or off,
//! takes the next choice, or asks for a new number or text; Delete puts
//! the default back; Escape closes. Every change is said: "Rate, 300 words
//! per minute."

use serde_json::{Map, Value};
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;
use textweaver_store::Settings;
use textweaver_store::settings_io::settings_to_json;

use crate::app::{App, ListKind};
use crate::command::{Effect, PromptPurpose};
use crate::list_model::ListKey;

/// How a setting is changed, for a dialog or the settings screen.
#[derive(Clone, Debug, PartialEq)]
pub enum SettingKind {
    /// On or off.
    Toggle,
    /// A number from `min` to `max`, changed by `step` (whole numbers when
    /// `step` is whole), with a unit to say after it ("words per minute").
    Number {
        /// Smallest value.
        min: f64,
        /// Largest value.
        max: f64,
        /// How much Left and Right change it.
        step: f64,
        /// The unit, spoken after the value; may be empty.
        unit: &'static str,
    },
    /// One of a list of values. With `open`, other values are allowed too
    /// (a theme of your own, an installed font, a folder).
    Choice {
        /// The values offered, in order.
        choices: Vec<Choice>,
        /// Other values may be typed.
        open: bool,
    },
    /// Free text; with `optional`, an empty answer unsets it.
    Text {
        /// Empty means not set.
        optional: bool,
    },
    /// A list of texts (favorite voices, library folders), typed as items
    /// separated by [`Setting::list_separator`]: commas, or semicolons for
    /// folders, whose names may hold commas.
    List,
    /// A table of names and values (pronunciations, abbreviations, speed
    /// presets): shown with its size, edited in `settings.toml`.
    Table,
}

/// One value offered by a [`SettingKind::Choice`].
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    /// The value as stored.
    pub value: Value,
    /// What is shown and said.
    pub label: String,
}

/// One setting: where it is, what it is called, and how it changes.
#[derive(Clone, Debug, PartialEq)]
pub struct Setting {
    /// The key's path in `settings.toml`, such as `speech.rate`.
    pub path: String,
    /// The section title, such as "Speech".
    pub section: &'static str,
    /// The label, shown and spoken, such as "Rate".
    pub label: &'static str,
    /// One or two sentences of help.
    pub help: &'static str,
    /// How it changes.
    pub kind: SettingKind,
    /// Its default value.
    pub default: Value,
    /// Kept by textweaver itself (a question already asked, a theme picked
    /// by hand): in the schema, but not on the settings screen.
    pub internal: bool,
    /// The frontends it does something in. A setting for one frontend
    /// only is left off the other's settings screen; `settings.toml`,
    /// `tw settings` and JSON-RPC keep every setting.
    pub frontend: Frontend,
}

/// The frontends a setting does something in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Frontend {
    /// The terminal reader and the window.
    #[default]
    Both,
    /// The terminal reader only (wrap width, the cursor's place).
    Terminal,
    /// The window only (its bars, its line length).
    Window,
}

impl Frontend {
    /// True when the setting does something in the window.
    pub fn in_window(self) -> bool {
        self != Frontend::Terminal
    }

    /// True when the setting does something in the terminal reader.
    pub fn in_terminal(self) -> bool {
        self != Frontend::Window
    }

    /// Its name in the JSON schema: "both", "terminal" or "window".
    pub fn name(self) -> &'static str {
        match self {
            Frontend::Both => "both",
            Frontend::Terminal => "terminal",
            Frontend::Window => "window",
        }
    }
}

/// The settings only the terminal reader uses: the window's settings
/// dialog leaves them out (the window's code never reads them).
pub const TERMINAL_ONLY: &[&str] = &[
    "display.wrap_width",
    "display.tab_width",
    "display.show_line_numbers",
    "display.scroll_margin",
    "display.hints",
    "keyboard.digit_row",
    "accessibility.cursor",
    "interface.rtl",
];

/// The settings only the window uses: the terminal's settings screen
/// leaves them out.
pub const WINDOW_ONLY: &[&str] = &[
    "display.measure",
    "gui.announce",
    "gui.header",
    "gui.toolbar",
    "gui.auto_hide_menu",
    "gui.speak_messages",
];

/// Where the reading aids go on a settings screen, first to last: the
/// spacing, the font and the ruler first, RSVP last (W9b-d).
fn aids_rank(path: &str) -> u8 {
    let rest = path.strip_prefix("reading_aids.").unwrap_or(path);
    match rest.split('.').next().unwrap_or_default() {
        "spacing" => 0,
        "font" => 1,
        "ruler" => 2,
        "rsvp" => 9,
        _ => 5,
    }
}

/// Every setting, in the store's order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SettingsSchema {
    /// The settings, section by section.
    pub settings: Vec<Setting>,
}

/// Help, label, and kind for one key, from [`INFO`].
#[derive(Clone, Copy, Debug)]
pub struct Info {
    /// The key's path.
    pub path: &'static str,
    /// The label.
    pub label: &'static str,
    /// The help.
    pub help: &'static str,
    kind: InfoKind,
}

#[derive(Clone, Copy, Debug)]
enum InfoKind {
    Toggle,
    Number(f64, f64, f64, &'static str),
    Choice(&'static [(&'static str, &'static str)], bool),
    Text(bool),
    List,
    Table,
    Internal,
}

const fn toggle(path: &'static str, label: &'static str, help: &'static str) -> Info {
    Info {
        path,
        label,
        help,
        kind: InfoKind::Toggle,
    }
}

const fn number(
    path: &'static str,
    label: &'static str,
    help: &'static str,
    range: (f64, f64, f64),
    unit: &'static str,
) -> Info {
    Info {
        path,
        label,
        help,
        kind: InfoKind::Number(range.0, range.1, range.2, unit),
    }
}

const fn choice(
    path: &'static str,
    label: &'static str,
    help: &'static str,
    choices: &'static [(&'static str, &'static str)],
) -> Info {
    Info {
        path,
        label,
        help,
        kind: InfoKind::Choice(choices, false),
    }
}

const fn open_choice(
    path: &'static str,
    label: &'static str,
    help: &'static str,
    choices: &'static [(&'static str, &'static str)],
) -> Info {
    Info {
        path,
        label,
        help,
        kind: InfoKind::Choice(choices, true),
    }
}

const fn text(path: &'static str, label: &'static str, help: &'static str) -> Info {
    Info {
        path,
        label,
        help,
        kind: InfoKind::Text(false),
    }
}

const fn optional(path: &'static str, label: &'static str, help: &'static str) -> Info {
    Info {
        path,
        label,
        help,
        kind: InfoKind::Text(true),
    }
}

const fn list(path: &'static str, label: &'static str, help: &'static str) -> Info {
    Info {
        path,
        label,
        help,
        kind: InfoKind::List,
    }
}

const fn table(path: &'static str, label: &'static str, help: &'static str) -> Info {
    Info {
        path,
        label,
        help,
        kind: InfoKind::Table,
    }
}

const fn internal(path: &'static str, label: &'static str, help: &'static str) -> Info {
    Info {
        path,
        label,
        help,
        kind: InfoKind::Internal,
    }
}

const LEVELS: &[(&str, &str)] = &[("low", "low"), ("normal", "normal"), ("high", "high")];
const PERCENT: (f64, f64, f64) = (0.0, 500.0, 10.0);

/// Labels, help, ranges, and choices for every key in `settings.toml`, in
/// the store's order. Help follows the store's own documentation.
pub const INFO: &[Info] = &[
    // [speech]
    open_choice(
        "speech.backend",
        "Speech engine",
        "The speech engine: auto picks the best one available. A change restarts speech.",
        &[
            ("auto", "automatic"),
            ("eci", "Eloquence"),
            ("sapi", "SAPI 5 voices"),
            ("espeak", "eSpeak NG"),
            ("speechd", "Speech Dispatcher"),
            ("nsspeech", "Apple NSSpeech"),
            ("avspeech", "Apple AVSpeech"),
            ("dectalk", "DECtalk"),
            ("omnivox", "Omnivox"),
            ("null", "silent"),
        ],
    ),
    number(
        "speech.rate",
        "Rate",
        "How fast textweaver speaks.",
        (50.0, 900.0, 20.0),
        "words per minute",
    ),
    number(
        "speech.volume",
        "Volume",
        "How loud textweaver speaks.",
        (0.0, 100.0, 10.0),
        "percent",
    ),
    number(
        "speech.pitch",
        "Pitch",
        "Higher or lower than the voice's own pitch.",
        (-12.0, 12.0, 1.0),
        "semitones",
    ),
    optional(
        "speech.voice",
        "Voice",
        "The voice's id; not set picks one automatically. The Voices command lists them.",
    ),
    optional(
        "speech.prefer_voice",
        "Preferred voice",
        "When no voice is set, the first voice whose name contains this, such as eloquence.",
    ),
    list(
        "speech.favorite_voices",
        "Favorite voices",
        "Voices listed first by the Voices command, by id, separated by commas.",
    ),
    choice(
        "speech.punctuation",
        "Punctuation",
        "How much punctuation is spoken.",
        &[("none", "none"), ("some", "some"), ("all", "all")],
    ),
    toggle(
        "speech.split_caps",
        "Split capitals",
        "Say words joined with capitals, such as TextWeaver, as separate words.",
    ),
    choice(
        "speech.caps",
        "Capitals",
        "How a capital letter is marked when characters are spoken and typed.",
        &[
            ("none", "not marked"),
            ("tone", "a tone"),
            ("pitch", "a higher pitch"),
            ("say_cap", "say cap"),
        ],
    ),
    toggle(
        "speech.auto_play",
        "Read on opening",
        "Start reading when a document opens.",
    ),
    toggle(
        "speech.skip_code",
        "Skip code blocks",
        "Do not speak code blocks.",
    ),
    table(
        "speech.speed_presets",
        "Speed presets",
        "Named rates that F8 cycles through.",
    ),
    table(
        "speech.voices_by_language",
        "Voices by language",
        "The voice for each interface language, by language tag, such as es = the voice's id. A language not listed uses the engine's first voice for it.",
    ),
    number(
        "speech.latency_offset_ms",
        "Highlight delay",
        "How long after an engine reports a word the highlight moves, for engines timed by their audio clock.",
        (0.0, 1000.0, 10.0),
        "milliseconds",
    ),
    number(
        "speech.pause_heading_ms",
        "Pause after headings",
        "Silence after a heading, shorter at faster rates. 0 turns it off.",
        (0.0, 3000.0, 50.0),
        "milliseconds",
    ),
    number(
        "speech.pause_paragraph_ms",
        "Pause after paragraphs",
        "Silence after a paragraph, shorter at faster rates. 0 turns it off.",
        (0.0, 3000.0, 50.0),
        "milliseconds",
    ),
    number(
        "speech.pause_list_item_ms",
        "Pause after list items",
        "Silence after a list item, shorter at faster rates. 0 turns it off.",
        (0.0, 3000.0, 50.0),
        "milliseconds",
    ),
    toggle(
        "speech.markup_pauses",
        "Pauses written as markup",
        "Read pause markup in a document, such as <break time=\"1s\"/>, as a pause. Turn it off for documents that quote such markup.",
    ),
    optional(
        "speech.output_device",
        "Output device",
        "The sound device speech plays on, by its id. Not set, or a device that is not connected, uses the system's default.",
    ),
    choice(
        "speech.verbosity",
        "Verbosity",
        "How much textweaver says about what it does.",
        LEVELS,
    ),
    open_choice(
        "speech.eci.dictionaries",
        "Eloquence dictionaries",
        "The community pronunciation dictionaries for Eloquence: on, off, or a folder of your own.",
        &[("true", "on"), ("false", "off")],
    ),
    optional(
        "speech.eci.library",
        "Eloquence library",
        "The ECI library to load; not set searches the usual places.",
    ),
    toggle(
        "speech.eci.code_factory",
        "Search Code Factory's Eloquence",
        "Also look for Code Factory's Eloquence for Windows. Its license may not cover other programs.",
    ),
    toggle(
        "speech.sapi.onecore",
        "OneCore voices",
        "Also list the Windows OneCore voices through SAPI 5.",
    ),
    choice(
        "speech.apple.backend",
        "Apple speech engine",
        "Which of Apple's speech engines to use on macOS.",
        &[
            ("auto", "automatic"),
            ("nsspeech", "NSSpeechSynthesizer"),
            ("avspeech", "AVSpeechSynthesizer"),
        ],
    ),
    optional(
        "speech.dectalk.library",
        "DECtalk library",
        "The DECtalk library to load; not set searches the usual places.",
    ),
    optional(
        "speech.piper.voices",
        "Piper voices folder",
        "The folder of Piper voices; not set uses the piper folder in textweaver's data folder.",
    ),
    optional(
        "speech.piper.voice",
        "Piper voice",
        "The Piper voice to start with, by id; not set takes the first installed.",
    ),
    choice(
        "speech.piper.phonemizer",
        "Piper phonemizer",
        "How Piper turns text into sounds. The espeak-ng library when installed, that library, or textweaver's own.",
        &[
            ("auto", "automatic"),
            ("library", "espeak-ng library"),
            ("rust", "textweaver's own"),
        ],
    ),
    table(
        "speech.voice_params",
        "Rate and pitch per voice",
        "The rate and pitch each voice was last used at. Choosing a voice again brings them back.",
    ),
    // [highlight]
    toggle(
        "highlight.enabled",
        "Highlight spoken text",
        "Highlight the word or sentence being read.",
    ),
    choice(
        "highlight.granularity",
        "Highlight",
        "What the reading highlight covers.",
        &[
            ("word", "the word"),
            ("sentence", "the sentence"),
            ("both", "the word and the sentence"),
        ],
    ),
    number(
        "highlight.lead_words",
        "Highlight lead",
        "Draw the highlight this many words ahead of the word heard (1 is the word heard).",
        (-5.0, 5.0, 1.0),
        "words",
    ),
    number(
        "highlight.speed",
        "Highlight speed",
        "Speed of the timed highlight for engines that report no words, as a multiple. 1 is normal speed.",
        (0.5, 1.5, 0.1),
        "",
    ),
    open_choice(
        "highlight.color",
        "Word highlight color",
        "The color behind the word being read. Choose a name, or type a hex code. Default: the theme's color.",
        crate::colors::COLOR_CHOICES,
    ),
    open_choice(
        "highlight.sentence_color",
        "Sentence highlight color",
        "The color behind the sentence being read. Choose a name, or type a hex code. Default: the theme's color.",
        crate::colors::COLOR_CHOICES,
    ),
    // [normalization]
    toggle(
        "normalization.math",
        "Speak math",
        "Speak math notation in words.",
    ),
    choice(
        "normalization.math_verbosity",
        "Math verbosity",
        "How explicit spoken math is: low says a over b, normal and high say more.",
        LEVELS,
    ),
    optional(
        "normalization.asciimath_delimiter",
        "ASCIIMath delimiter",
        "The character around ASCIIMath, usually a backtick; not set reads no ASCIIMath.",
    ),
    toggle(
        "normalization.abbreviations",
        "Expand abbreviations",
        "Say abbreviations in full, such as Doctor for Dr.",
    ),
    table(
        "normalization.abbrev_expansions",
        "Your abbreviations",
        "Abbreviations of your own and what they stand for.",
    ),
    toggle(
        "normalization.numbers",
        "Numbers in words",
        "Say numbers, dates, times, and money in words.",
    ),
    toggle(
        "normalization.use_pronunciations",
        "Use pronunciations",
        "Apply your pronunciation list.",
    ),
    table(
        "normalization.pronunciations",
        "Pronunciations",
        "Words and how to say them.",
    ),
    choice(
        "normalization.table_mode",
        "Tables",
        "How tables are read.",
        &[
            ("structured", "with rows and columns"),
            ("flat", "as text"),
            ("skip", "skipped"),
        ],
    ),
    choice(
        "normalization.footnote_mode",
        "Footnotes",
        "Where footnotes are read.",
        &[
            ("inline", "where they are marked"),
            ("deferred", "at the end"),
            ("skip", "skipped"),
        ],
    ),
    toggle(
        "normalization.community_lexicon.enabled",
        "Community lexicon",
        "Apply the community pronunciation dictionaries for engines other than Eloquence.",
    ),
    optional(
        "normalization.community_lexicon.dir",
        "Community lexicon folder",
        "The folder holding the dictionary files; not set looks beside textweaver.",
    ),
    open_choice(
        "normalization.community_lexicon.language",
        "Community lexicon language",
        "The dictionaries' language.",
        &[("ENU", "US English"), ("DEU", "German")],
    ),
    toggle(
        "normalization.medical_lexicon.enabled",
        "Medical lexicon",
        "Read drug names, clinical terms and dosing abbreviations from a medical pronunciation list.",
    ),
    optional(
        "normalization.medical_lexicon.overlay",
        "Medical lexicon file",
        "Your own medical pronunciations, which win over the built-in ones. Not set reads medical-lexicon.toml in the settings folder.",
    ),
    // [reading]
    toggle(
        "reading.auto_resume",
        "Resume where you left off",
        "Go back to the saved position when a document opens.",
    ),
    number(
        "reading.nav_history_size",
        "Back history",
        "How many places Back remembers.",
        (1.0, 1000.0, 10.0),
        "places",
    ),
    toggle(
        "reading.wrap_navigation",
        "Wrap navigation",
        "Moving past the end of the document goes on from the start.",
    ),
    toggle(
        "reading.cursor_follows_speech",
        "Cursor follows speech",
        "The cursor moves with the word being read.",
    ),
    choice(
        "reading.citations",
        "Citations",
        "Citations in continuous reading: skipped, or said in words.",
        &[("off", "skipped"), ("words", "in words")],
    ),
    toggle(
        "reading.ocr",
        "Recognize scanned pages",
        "Read the text of scanned PDFs and pictures by recognizing it (OCR).",
    ),
    open_choice(
        "reading.ocr_lang",
        "Scanned text language",
        "The language of scanned text, as Tesseract codes such as fra or deu+eng. Empty means the document's own language, else English.",
        &[
            ("", "the document's"),
            ("eng", "English"),
            ("fra", "French"),
            ("deu", "German"),
            ("spa", "Spanish"),
        ],
    ),
    choice(
        "reading.ocr_engine",
        "OCR engine",
        "Which engine recognizes scanned pages. ocrs for English and Tesseract for other languages, or one of them always.",
        &[
            ("auto", "automatic"),
            ("ocrs", "ocrs"),
            ("tesseract", "Tesseract"),
            ("paddle", "PaddleOCR (experimental)"),
        ],
    ),
    choice(
        "reading.math_engine",
        "Math speech",
        "Which engine reads math aloud. textweaver's own, or MathCAT in ClearSpeak or SimpleSpeak, in the document's language. MathCAT needs a version that includes it; otherwise textweaver's own is used.",
        &[
            ("builtin", "textweaver"),
            ("mathcat", "MathCAT ClearSpeak"),
            ("mathcat_simplespeak", "MathCAT SimpleSpeak"),
        ],
    ),
    choice(
        "reading.math_display",
        "Math on screen",
        "How math looks in the reading view. As its source, such as x^2, or as Unicode, such as x with a superscript 2. Speech and edit mode always use the source.",
        &[("source", "source"), ("unicode", "Unicode")],
    ),
    choice(
        "reading.revisions",
        "Tracked changes",
        "How tracked changes in Word, OpenDocument, and RTF files are read. Said in place at high verbosity (automatic), always said, or never said, reading the final text. Applies when a document is opened.",
        &[
            ("auto", "automatic"),
            ("marked", "always say them"),
            ("final", "final text only"),
        ],
    ),
    choice(
        "reading.stop_at",
        "Stop at section end",
        "Where continuous reading stops by itself and says End of section. Never, at the next heading of any level, or at the next chapter: a section break, else a level 1 heading. Reading goes on from the heading with the read key.",
        &[
            ("off", "never"),
            ("heading", "next heading"),
            ("chapter", "next chapter"),
        ],
    ),
    number(
        "reading.stop_after_minutes",
        "Reading timer",
        "Continuous reading stops at a sentence end after this many minutes of reading. It says so. Pausing stops the clock; stopping starts it over. 0 turns the timer off.",
        (0.0, 240.0, 5.0),
        "minutes",
    ),
    // [display]
    open_choice("display.theme", "Theme", "The color theme.", &[]),
    toggle(
        "display.follow_os_theme",
        "Follow the system theme",
        "At startup, use a light, dark, or high-contrast theme like the system, unless you picked one.",
    ),
    internal(
        "display.theme_explicit",
        "Theme picked",
        "Set when you pick a theme; it stops following the system.",
    ),
    number(
        "display.wrap_width",
        "Wrap width",
        "Wrap lines at this many columns; 0 uses the whole width.",
        (0.0, 400.0, 10.0),
        "columns",
    ),
    number(
        "display.measure",
        "Line length",
        "How many characters a line holds in the window, from 25 to 90. 0 fills the window. The terminal uses the wrap width.",
        (0.0, 90.0, 1.0),
        "characters",
    ),
    number(
        "display.tab_width",
        "Tab width",
        "Columns a tab takes.",
        (1.0, 16.0, 1.0),
        "columns",
    ),
    toggle(
        "display.show_line_numbers",
        "Line numbers",
        "Show line numbers.",
    ),
    number(
        "display.scroll_margin",
        "Scroll margin",
        "Lines kept in view above and below the cursor.",
        (0.0, 20.0, 1.0),
        "lines",
    ),
    choice(
        "display.hints",
        "Key hints line",
        "Whether the terminal reader shows key hints on its bottom line. Automatic shows them when self-voicing and hides them with a screen reader. F1 and the keyboard shortcuts list always name the keys.",
        &[("auto", "automatic"), ("on", "on"), ("off", "off")],
    ),
    // [editing]
    toggle(
        "editing.autosave_recovery",
        "Recovery snapshots",
        "Keep a copy of unsaved work and offer it after a crash.",
    ),
    number(
        "editing.autosave_interval_secs",
        "Snapshot interval",
        "Seconds between recovery snapshots while there are unsaved changes.",
        (5.0, 3600.0, 5.0),
        "seconds",
    ),
    toggle(
        "editing.echo_characters",
        "Echo characters",
        "Say each character typed.",
    ),
    toggle("editing.echo_words", "Echo words", "Say each word typed."),
    toggle(
        "editing.echo_deletions",
        "Echo deletions",
        "Say what Backspace and Delete remove.",
    ),
    toggle(
        "editing.echo_lines_on_move",
        "Echo lines",
        "Say the line when the cursor moves to another line.",
    ),
    number(
        "editing.undo_steps",
        "Undo steps",
        "Most undo steps kept while editing.",
        (1.0, 100_000.0, 100.0),
        "steps",
    ),
    number(
        "editing.undo_memory_mb",
        "Undo memory",
        "Most memory the undo history may use.",
        (1.0, 4096.0, 16.0),
        "megabytes",
    ),
    text(
        "editing.author",
        "Author",
        "The author written into new documents made from a template; empty leaves it blank.",
    ),
    // [library]
    number(
        "library.recent_limit",
        "Recent files",
        "How many recent files are remembered.",
        (1.0, 500.0, 5.0),
        "files",
    ),
    list(
        "library.folders",
        "Library folders",
        "Folders whose documents the library lists, and whose positions sync between computers. Separate folders with semicolons.",
    ),
    // [keyboard]
    toggle(
        "keyboard.character_keys",
        "Single-key shortcuts",
        "Browse keys such as h and period. Off, dictation and typing never trigger commands.",
    ),
    choice(
        "keyboard.preset",
        "Keys",
        "The default keys: like NVDA's and JAWS's browse mode, or textweaver's earlier keys. Used from the next start.",
        &[("default", "screen reader style"), ("classic", "classic")],
    ),
    choice(
        "keyboard.digit_row",
        "Digit row",
        "How the terminal recognizes the digit keys for heading levels. Auto, or a French AZERTY keyboard.",
        &[("auto", "automatic"), ("azerty", "AZERTY")],
    ),
    // [accessibility]
    choice(
        "accessibility.mode",
        "Accessibility mode",
        "Self-voicing speaks everything; screen reader leaves speech to your screen reader; hybrid voices reading only.",
        &[
            ("self-voicing", "self-voicing"),
            ("screen-reader", "screen reader"),
            ("hybrid", "hybrid"),
        ],
    ),
    choice(
        "accessibility.say_all",
        "Say all with a screen reader",
        "Continuous reading in screen-reader mode. A sentence at a time on the status line, or textweaver's voice.",
        &[
            ("screen", "on the status line"),
            ("voice", "with textweaver's voice"),
        ],
    ),
    choice(
        "accessibility.quiet_screen",
        "Quiet screen while reading",
        "Keep the screen still while textweaver reads aloud. On by default in hybrid mode.",
        &[("auto", "automatic"), ("true", "on"), ("false", "off")],
    ),
    choice(
        "accessibility.cursor",
        "Cursor",
        "Where the terminal's cursor waits. On what you are working on, or on the status line.",
        &[
            ("follow", "follows focus"),
            ("status", "on the status line"),
        ],
    ),
    choice(
        "accessibility.interface_announcements",
        "Interface announcements",
        "How much textweaver says about itself: dialogs, progress, hints, and routine confirmations. Errors and answers to what you asked are always said. Automatic is minimal with a screen reader, normal when self-voicing.",
        &[
            ("auto", "automatic"),
            ("off", "off"),
            ("minimal", "minimal"),
            ("normal", "normal"),
            ("full", "full"),
        ],
    ),
    internal(
        "accessibility.hybrid_offered",
        "Hybrid mode offered",
        "Set once textweaver has asked whether to use hybrid mode.",
    ),
    // [export]
    choice(
        "export.audio_format",
        "Audio export format",
        "The format Export audio lists first. tw export-audio also uses it for a file name with no extension.",
        &[
            ("flac", "FLAC"),
            ("mp3", "MP3"),
            ("opus", "Opus"),
            ("ogg", "Ogg Vorbis"),
            ("wav", "WAV"),
            ("m4b", "M4B audiobook"),
            ("mp4", "MP4 video with captions"),
        ],
    ),
    choice(
        "export.subtitle_format",
        "Subtitle format",
        "The format of subtitles written without a file name.",
        &[("srt", "SubRip"), ("vtt", "WebVTT"), ("ass", "ASS karaoke")],
    ),
    choice(
        "export.subtitle_karaoke",
        "Subtitle karaoke",
        "How subtitle lines show the word being read. Off, underlined as it is spoken (WebVTT tags), or one cue per word in bold and underline.",
        &[
            ("off", "Off"),
            ("tags", "Underline as spoken"),
            ("lines", "One cue per word"),
        ],
    ),
    toggle(
        "export.subtitle_chapters",
        "Chapters file",
        "Also write a WebVTT chapters file beside the subtitles or the audio.",
    ),
    toggle(
        "export.subtitle_word_level",
        "Word subtitles",
        "One subtitle per word instead of caption lines.",
    ),
    toggle(
        "export.subtitles_with_audio",
        "Subtitles with audio",
        "Always write subtitles beside exported audio.",
    ),
    // [braille]
    choice(
        "braille.math_code",
        "Math braille",
        "The braille code for math in BRF files and while exploring a formula with MathCAT. Nemeth, or UEB mathematics. It needs a version that includes MathCAT; otherwise math is written as its spoken words.",
        &[("nemeth", "Nemeth"), ("ueb", "UEB")],
    ),
    choice(
        "braille.table_format",
        "Braille tables",
        "How BRF files lay out tables. Linear, one row per line with semicolons between entries; listed, each row a heading with each entry on its own line after its column heading; or stairstep, each entry two cells right of the one before, for tables of up to four columns.",
        &[
            ("linear", "linear"),
            ("listed", "listed"),
            ("stairstep", "stairstep"),
        ],
    ),
    // [reading_aids]
    number(
        "reading_aids.rsvp.wpm",
        "RSVP rate",
        "Words per minute of rapid serial visual presentation.",
        (60.0, 1500.0, 20.0),
        "words per minute",
    ),
    choice(
        "reading_aids.rsvp.pacing",
        "RSVP pacing",
        "What moves the RSVP word on: its own timer, or speech.",
        &[("timer", "its own timer"), ("external", "speech")],
    ),
    number(
        "reading_aids.rsvp.clause_pause",
        "RSVP clause pause",
        "Extra time after a comma, colon, dash, or bracket, in percent of a word's time.",
        PERCENT,
        "percent",
    ),
    number(
        "reading_aids.rsvp.sentence_pause",
        "RSVP sentence pause",
        "Extra time at the end of a sentence, in percent.",
        PERCENT,
        "percent",
    ),
    number(
        "reading_aids.rsvp.paragraph_pause",
        "RSVP paragraph pause",
        "Extra time at the end of a paragraph, in percent.",
        PERCENT,
        "percent",
    ),
    number(
        "reading_aids.rsvp.long_word_len",
        "RSVP long word",
        "Words longer than this many letters get extra time.",
        (1.0, 40.0, 1.0),
        "letters",
    ),
    number(
        "reading_aids.rsvp.long_word_step",
        "RSVP long word step",
        "Extra time per letter beyond a long word's length, in percent.",
        (0.0, 100.0, 5.0),
        "percent",
    ),
    number(
        "reading_aids.rsvp.long_word_max",
        "RSVP long word most",
        "Most extra time a long word gets, in percent.",
        PERCENT,
        "percent",
    ),
    toggle(
        "reading_aids.rsvp.show_previous",
        "RSVP previous word",
        "Show the previous word too.",
    ),
    toggle(
        "reading_aids.rsvp.show_next",
        "RSVP next word",
        "Show the next word too.",
    ),
    choice(
        "reading_aids.rsvp.position",
        "RSVP position",
        "Where the RSVP word appears.",
        &[
            ("top-left", "top left"),
            ("top-center", "top center"),
            ("top-right", "top right"),
            ("center-left", "middle left"),
            ("center", "center"),
            ("center-right", "middle right"),
            ("bottom-left", "bottom left"),
            ("bottom-center", "bottom center"),
            ("bottom-right", "bottom right"),
        ],
    ),
    number(
        "reading_aids.rsvp.font_size_pt",
        "RSVP size",
        "Size of the RSVP word in the window.",
        (8.0, 200.0, 2.0),
        "points",
    ),
    number(
        "reading_aids.rsvp.lead_words",
        "RSVP lead",
        "With speech pacing, show this many words ahead of the word spoken.",
        (-5.0, 5.0, 1.0),
        "words",
    ),
    toggle(
        "reading_aids.bionic",
        "Bionic reading",
        "Draw the start of each word in bold.",
    ),
    number(
        "reading_aids.bionic_options.ratio",
        "Bionic share",
        "How much of each word is bold.",
        (0.1, 0.9, 0.1),
        "",
    ),
    number(
        "reading_aids.bionic_options.min_word_len",
        "Bionic shortest word",
        "Words shorter than this are left alone.",
        (1.0, 20.0, 1.0),
        "letters",
    ),
    toggle(
        "reading_aids.bionic_options.skip_numbers",
        "Bionic skips numbers",
        "Leave words with digits alone.",
    ),
    toggle(
        "reading_aids.bionic_options.skip_urls",
        "Bionic skips addresses",
        "Leave web and email addresses alone.",
    ),
    toggle(
        "reading_aids.bionic_options.skip_code",
        "Bionic skips code",
        "Leave code alone.",
    ),
    number(
        "reading_aids.spacing.line_height",
        "Line height",
        "Line height as a multiple of the font size. 1.5 is the WCAG value.",
        (1.0, 3.0, 0.1),
        "",
    ),
    number(
        "reading_aids.spacing.paragraph_spacing",
        "Paragraph spacing",
        "Space after each paragraph, in multiples of the font size.",
        (0.0, 4.0, 0.25),
        "",
    ),
    number(
        "reading_aids.spacing.letter_spacing",
        "Letter spacing",
        "Extra space between letters, in multiples of the font size.",
        (0.0, 0.5, 0.02),
        "",
    ),
    number(
        "reading_aids.spacing.word_spacing",
        "Word spacing",
        "Extra space between words, in multiples of the font size.",
        (0.0, 1.0, 0.04),
        "",
    ),
    open_choice(
        "reading_aids.font.family",
        "Font",
        "The window's reading font. You can also type the name of any installed font.",
        &[
            ("system-ui", "the system font"),
            ("sans", "sans serif"),
            ("serif", "serif"),
            ("monospace", "monospace"),
            ("atkinson", "Atkinson Hyperlegible"),
            ("opendyslexic", "OpenDyslexic"),
            ("lexend", "Lexend"),
        ],
    ),
    number(
        "reading_aids.font.size_pt",
        "Font size",
        "The window's font size.",
        (6.0, 72.0, 1.0),
        "points",
    ),
    number(
        "reading_aids.font.weight",
        "Font weight",
        "400 is regular, 700 bold.",
        (100.0, 900.0, 100.0),
        "",
    ),
    choice(
        "reading_aids.ruler.mode",
        "Reading ruler",
        "Mark the current line, or a band of lines.",
        &[
            ("off", "off"),
            ("current_line", "current line"),
            ("ruler", "ruler"),
        ],
    ),
    choice(
        "reading_aids.ruler.scope",
        "Ruler covers",
        "A wrapped row, or the whole line.",
        &[("row", "a row"), ("line", "the whole line")],
    ),
    number(
        "reading_aids.ruler.rows_above",
        "Ruler rows above",
        "Rows of the band above the current one.",
        (0.0, 10.0, 1.0),
        "rows",
    ),
    number(
        "reading_aids.ruler.rows_below",
        "Ruler rows below",
        "Rows of the band below the current one.",
        (0.0, 10.0, 1.0),
        "rows",
    ),
    toggle(
        "reading_aids.ruler.mask_outside",
        "Ruler mask",
        "Dim the rows outside the band.",
    ),
    toggle(
        "reading_aids.syllables",
        "Syllables",
        "Draw words split into syllables with a middle dot; speech is unchanged.",
    ),
    toggle(
        "reading_aids.difficult_words",
        "Difficult words",
        "Underline rare words, and name them on word moves at high verbosity.",
    ),
    text(
        "reading_aids.syllable_options.separator",
        "Syllable separator",
        "What is drawn between syllables.",
    ),
    number(
        "reading_aids.syllable_options.left_min",
        "Syllable first break",
        "Fewest letters before the first break.",
        (1.0, 10.0, 1.0),
        "letters",
    ),
    number(
        "reading_aids.syllable_options.right_min",
        "Syllable last break",
        "Fewest letters after the last break.",
        (1.0, 10.0, 1.0),
        "letters",
    ),
    number(
        "reading_aids.syllable_options.min_word_len",
        "Syllable shortest word",
        "Words shorter than this are never split.",
        (1.0, 20.0, 1.0),
        "letters",
    ),
    toggle(
        "reading_aids.syllable_options.skip_urls",
        "Syllables skip addresses",
        "Leave web and email addresses alone.",
    ),
    toggle(
        "reading_aids.syllable_options.skip_code",
        "Syllables skip code",
        "Leave code alone.",
    ),
    toggle(
        "reading_aids.difficult_definitions",
        "Difficult word definitions",
        "With difficult words marked, at high verbosity also say a difficult word's first definition from the dictionary.",
    ),
    // [preview]
    toggle(
        "preview.auto_reload",
        "Reload the preview",
        "Reload the browser preview after each save, through a small server on this computer only.",
    ),
    toggle(
        "preview.live",
        "Live preview",
        "With reloading on, also reload when typing pauses.",
    ),
    // [lexicon] (Agent W3e)
    optional(
        "lexicon.glossary",
        "Glossary",
        "Your own glossary, looked up before the dictionary: term: definition lines, or star's JSON. Not set uses glossary.txt in the settings folder.",
    ),
    optional(
        "lexicon.data_file",
        "Dictionary file",
        "The define-word dictionary, lexicon-en.twlex. Not set looks beside the program.",
    ),
    // [stats]
    toggle(
        "stats.enabled",
        "Reading statistics",
        "Count the time read aloud, the furthest point, and sessions for each document.",
    ),
    // [summary] (W5s)
    number(
        "summary.sentences",
        "Summary sentences",
        "How many sentences Summarize and tw summarize give, 1 to 50.",
        (1.0, 50.0, 1.0),
        "sentences",
    ),
    // [dictation] (W6d)
    toggle(
        "dictation.speak_while_recording",
        "Speak while dictating",
        "Say dictated words as they come. Off, they are shown on the status line and said at each pause, so the microphone does not hear the voice.",
    ),
    optional(
        "dictation.model_dir",
        "Dictation model folder",
        "A folder holding a Whisper model for dictation. Not set uses the dictation model below, in the data folder.",
    ),
    // W8a-w: the model offered for download.
    open_choice(
        "dictation.model",
        "Dictation model",
        "The Whisper model dictation uses when no folder is set. Download the dictation model, in the Tools menu, gets it.",
        &[
            ("whisper-base.en", "base.en, the default"),
            ("whisper-small.en", "small.en, larger and more accurate"),
        ],
    ),
    // [interface]
    open_choice(
        "interface.language",
        "Interface language",
        "The language of textweaver's own words, changed at once. The voice follows it when the engine has one for it; otherwise the voice stays.",
        &[
            ("en", "English"),
            ("es", "Español"),
            ("fr", "Français"),
            ("de", "Deutsch"),
            ("pt", "Português"),
            ("ar", "العربية"),
        ],
    ),
    choice(
        "interface.rtl",
        "Right-to-left display",
        "Whether the terminal reader reorders right-to-left text for display. Automatic leaves it to terminals that do it themselves. Speech and the screen reader always get the text in reading order.",
        &[("auto", "automatic"), ("on", "on"), ("off", "off")],
    ),
    internal(
        "interface.recent_settings",
        "Recently changed settings",
        "The settings changed last on the settings screen, listed at its top.",
    ),
    // [gui]
    internal(
        "gui.window",
        "Window place and size",
        "Where the window was and how large, kept on this computer and never synced.",
    ),
    choice(
        "gui.announce",
        "Announcements",
        "How the window's messages reach the screen reader, from the next start. A live region, or UI Automation notifications (Windows only).",
        &[
            ("live", "live region"),
            ("uia", "UI Automation notifications"),
        ],
    ),
    toggle(
        "gui.header",
        "Show the header",
        "Show the bar of Open, Font, Edit, Settings and Commands above the document. Off, the commands keep their keys and menu items.",
    ),
    toggle(
        "gui.toolbar",
        "Show the toolbar",
        "Show the bar of Play, Stop and the reading buttons. Off, the commands keep their keys and menu items.",
    ),
    toggle(
        "gui.auto_hide_menu",
        "Hide the menu bar",
        "Windows: hide the window's menu bar until Alt or F10 shows it. It hides again when the menu closes. No effect on Linux, whose menus are the F10 list, or on macOS.",
    ),
    toggle(
        "gui.speak_messages",
        "Speak textweaver's messages",
        "When textweaver reads aloud, also say its messages, typing and cursor moves in its voice, for reading by ear without a screen reader.",
    ),
    choice(
        "gui.sidebar",
        "Panel beside the document",
        "The panel the window shows beside the document. None, the Contents (the headings), or the Notes. The panel keys change it, and the window remembers the last one.",
        &[
            ("off", "none"),
            ("contents", "Contents"),
            ("notes", "Notes"),
        ],
    ),
    // [colors] (W6u): open choices, a named color or #rrggbb.
    open_choice(
        "colors.ruler",
        "Reading ruler color",
        "The band of the reading ruler and the marked current line. The ruler keeps its underline or bold. The terminal reader uses it; the window does not yet. Choose a name, or type a hex code. Default: the theme's color.",
        crate::colors::COLOR_CHOICES,
    ),
    open_choice(
        "colors.difficult_words",
        "Difficult words color",
        "The underline of difficult words; they stay underlined and are named at high verbosity. The terminal reader uses it; the window does not yet. Choose a name, or type a hex code. Default: the theme's color.",
        crate::colors::COLOR_CHOICES,
    ),
    open_choice(
        "colors.syllables",
        "Syllable marks color",
        "The middle dots between syllables. The terminal reader uses it; the window does not yet. Choose a name, or type a hex code. Default: the theme's color.",
        crate::colors::COLOR_CHOICES,
    ),
    open_choice(
        "colors.misspellings",
        "Misspellings color",
        "The underline of misspelled words, which are also said. Not used yet: the window draws the theme's color. Choose a name, or type a hex code. Default: the theme's color.",
        crate::colors::COLOR_CHOICES,
    ),
    open_choice(
        "colors.lint",
        "Lint marks color",
        "The underline of Markdown lint and grammar problems, which are also said. Not used yet: the window draws the theme's color. Choose a name, or type a hex code. Default: the theme's color.",
        crate::colors::COLOR_CHOICES,
    ),
    open_choice(
        "colors.find_match",
        "Search match color",
        "The band behind search matches; they stay underlined. Choose a name, or type a hex code. Default: the theme's color.",
        crate::colors::COLOR_CHOICES,
    ),
    open_choice(
        "colors.selection",
        "Selection color",
        "The band behind selected text. Choose a name, or type a hex code. Default: the theme's color.",
        crate::colors::COLOR_CHOICES,
    ),
    open_choice(
        "colors.focus",
        "Focus color",
        "The focus outline and the focused item of a list; they stay bold. Choose a name, or type a hex code. Default: the theme's color.",
        crate::colors::COLOR_CHOICES,
    ),
    open_choice(
        "colors.links",
        "Link color",
        "The color of links. Links stay underlined. Choose a name, or type a hex code. Default: the theme's color.",
        crate::colors::COLOR_CHOICES,
    ),
    open_choice(
        "colors.headings",
        "Heading color",
        "The color of headings. Headings stay bold. Choose a name, or type a hex code. Default: the theme's color.",
        crate::colors::COLOR_CHOICES,
    ),
    open_choice(
        "colors.status_bar",
        "Status bar color",
        "The band of the status and title bars. Choose a name, or type a hex code. Default: the theme's color.",
        crate::colors::COLOR_CHOICES,
    ),
    open_choice(
        "colors.notes",
        "Note color",
        "The band behind text with a note; it stays italic and underlined. Choose a name, or type a hex code. Default: the theme's color.",
        crate::colors::COLOR_CHOICES,
    ),
    open_choice(
        "colors.bookmarks",
        "Bookmark color",
        "The band behind a bookmarked word; it stays bold and underlined. Choose a name, or type a hex code. Default: the theme's color.",
        crate::colors::COLOR_CHOICES,
    ),
    // [sync] (ADR-0049)
    toggle(
        "sync.enabled",
        "Sync",
        "Sync notes, highlights, bookmarks, and places with your other computers through the sync folder. Tools, Sync, Set up sync turns it on.",
    ),
    optional(
        "sync.folder",
        "Sync folder",
        "The folder your computers share. One kept in step by Syncthing, a cloud folder, or a USB stick.",
    ),
    text(
        "sync.device_name",
        "Computer name",
        "This computer's name in sync messages, such as laptop or lab. Empty uses Computer 1, Computer 2, and so on.",
    ),
    toggle(
        "sync.places",
        "Sync places",
        "Share where you are in each document.",
    ),
    toggle("sync.notes", "Sync notes", "Share notes."),
    toggle("sync.highlights", "Sync highlights", "Share highlights."),
    toggle("sync.bookmarks", "Sync bookmarks", "Share bookmarks."),
    toggle(
        "sync.statistics",
        "Sync statistics",
        "Share each computer's reading time and sessions.",
    ),
    toggle(
        "sync.settings",
        "Sync settings",
        "Share the portable settings: rate, punctuation, theme, reading aids, and the like. The voice, the engine, the access mode, the key preset, and paths stay on each computer.",
    ),
    toggle(
        "sync.profiles",
        "Sync profiles",
        "Share your profiles; which one is in use stays on each computer.",
    ),
    toggle(
        "sync.key_overrides",
        "Sync key overrides",
        "Share keymap.toml. A Mac's keys are kept but not used on Windows or Linux, and the reverse.",
    ),
    toggle(
        "sync.words",
        "Sync word list",
        "Share your spelling word list.",
    ),
    toggle(
        "sync.glossary",
        "Sync glossary",
        "Share your glossary's entries and your pronunciations.",
    ),
    toggle(
        "sync.favorite_voices",
        "Sync favorite voices",
        "Share your favorite voices. One this computer does not have is listed as not on this computer.",
    ),
    choice(
        "sync.position_policy",
        "Place to resume",
        "Which place a document opens at when another computer has one too. The newest, the furthest, or ask.",
        &[
            ("newest", "the newest"),
            ("furthest", "the furthest"),
            ("ask", "ask"),
        ],
    ),
    // [components] (W8a-d)
    text(
        "components.mirror",
        "Components mirror",
        "Where optional components come from first: an https address or a folder on this computer. Empty uses their public sources. Never put a password here.",
    ),
    internal(
        "components.chooser_shown",
        "Components list shown",
        "The first-run list of optional components was shown.",
    ),
];

/// The section title for a top-level key.
fn section_title(key: &str) -> &'static str {
    match key {
        "speech" => "Speech",
        "highlight" => "Highlight",
        "normalization" => "Speaking text",
        "reading" => "Reading",
        "display" => "Display",
        "editing" => "Editing",
        "library" => "Library",
        "keyboard" => "Keyboard",
        "accessibility" => "Accessibility",
        "export" => "Export",
        "braille" => "Braille",
        "reading_aids" => "Reading aids",
        "preview" => "Preview",
        "lexicon" => "Define word",
        "stats" => "Reading statistics",
        "summary" => "Summaries",
        "dictation" => "Dictation",
        "interface" => "Interface",
        "gui" => "Window",
        "colors" => "Colors",
        "sync" => "Sync",
        "components" => "Optional components",
        _ => "Other",
    }
}

/// The leaf paths of a settings JSON tree, with their values, in order.
/// `tables` are paths whose objects are values (maps), not sections.
fn leaves(prefix: &str, map: &Map<String, Value>, tables: &[&str], out: &mut Vec<(String, Value)>) {
    for (k, v) in map {
        let path = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{prefix}.{k}")
        };
        match v {
            Value::Object(m) if !tables.contains(&path.as_str()) => leaves(&path, m, tables, out),
            _ => out.push((path, v.clone())),
        }
    }
}

/// The value at `path` in a settings JSON tree.
fn get<'a>(map: &'a Map<String, Value>, path: &str) -> Option<&'a Value> {
    let mut parts = path.split('.');
    let mut v = map.get(parts.next()?)?;
    for p in parts {
        v = v.as_object()?.get(p)?;
    }
    Some(v)
}

/// Sets the value at `path`, creating sections on the way.
fn set(map: &mut Map<String, Value>, path: &str, value: Value) {
    let parts: Vec<&str> = path.split('.').collect();
    let mut m = map;
    for p in &parts[..parts.len() - 1] {
        let entry = m
            .entry((*p).to_owned())
            .or_insert_with(|| Value::Object(Map::new()));
        if !entry.is_object() {
            *entry = Value::Object(Map::new());
        }
        let Value::Object(next) = entry else {
            return;
        };
        m = next;
    }
    if let Some(last) = parts.last() {
        m.insert((*last).to_owned(), value);
    }
}

/// Every setting's JSON, or an empty map when it cannot be written (it
/// always can: every value of `Settings` is JSON-safe).
fn json(settings: &Settings) -> Map<String, Value> {
    settings_to_json(settings).unwrap_or_default()
}

/// The schema [`SettingsSchema::generate`] makes, built once: it depends
/// on no state, and the menus read it for every toggle they show and
/// every change asks it first. [`App::settings_schema`] adds this session's
/// themes to a copy.
pub(crate) fn base_schema() -> &'static SettingsSchema {
    static SCHEMA: std::sync::LazyLock<SettingsSchema> =
        std::sync::LazyLock::new(SettingsSchema::generate);
    &SCHEMA
}

impl SettingsSchema {
    /// The schema of the store's settings: every key of a default
    /// `settings.toml`, with its [`INFO`]. Keys with no entry in [`INFO`]
    /// get a label made from the key and no help (the tests list them).
    pub fn generate() -> Self {
        let defaults = json(&Settings::default());
        let tables: Vec<&str> = INFO
            .iter()
            .filter(|i| matches!(i.kind, InfoKind::Table))
            .map(|i| i.path)
            .collect();
        let mut found = Vec::new();
        leaves("", &defaults, &tables, &mut found);
        // The store's order (speech first), not the JSON's sorted keys.
        found.sort_by_key(|(path, _)| {
            INFO.iter()
                .position(|i| i.path == path)
                .unwrap_or(usize::MAX)
        });
        // The reading aids: spacing, font and ruler first, RSVP last.
        if let (Some(first), Some(last)) = (
            found
                .iter()
                .position(|(p, _)| p.starts_with("reading_aids.")),
            found
                .iter()
                .rposition(|(p, _)| p.starts_with("reading_aids.")),
        ) {
            found[first..=last].sort_by_key(|(p, _)| aids_rank(p));
        }
        let settings = found
            .into_iter()
            .map(|(path, default)| {
                let top = path.split('.').next().unwrap_or_default().to_owned();
                let info = INFO.iter().find(|i| i.path == path);
                let (label, help, kind, internal) = match info {
                    Some(i) => (
                        i.label,
                        i.help,
                        i.kind,
                        matches!(i.kind, InfoKind::Internal),
                    ),
                    None => ("", "", infer(&default), false),
                };
                let frontend = if TERMINAL_ONLY.contains(&path.as_str()) {
                    Frontend::Terminal
                } else if WINDOW_ONLY.contains(&path.as_str()) {
                    Frontend::Window
                } else {
                    Frontend::Both
                };
                Setting {
                    frontend,
                    kind: kind_of(kind, &default),
                    label: if label.is_empty() {
                        // Leaked once per unknown key; the tests keep this
                        // list empty.
                        Box::leak(fallback_label(&path).into_boxed_str())
                    } else {
                        label
                    },
                    help,
                    section: section_title(&top),
                    internal,
                    default,
                    path,
                }
            })
            .collect();
        SettingsSchema { settings }
    }

    /// The setting at `path`.
    pub fn get(&self, path: &str) -> Option<&Setting> {
        self.settings.iter().find(|s| s.path == path)
    }

    /// The settings shown on a settings screen or dialog (not the ones
    /// textweaver keeps for itself).
    pub fn visible(&self) -> impl Iterator<Item = &Setting> {
        self.settings.iter().filter(|s| !s.internal)
    }

    /// The settings shown in the window's settings dialog: the visible
    /// ones that do something in the window.
    pub fn in_window(&self) -> impl Iterator<Item = &Setting> {
        self.visible().filter(|s| s.frontend.in_window())
    }

    /// The schema as JSON, for JSON-RPC's `settings_schema`: each setting
    /// with `path`, `section`, `label`, `help`, `kind`, `default`, and, as
    /// the kind needs, `min`, `max`, `step`, `unit`, `choices`, `open`,
    /// and `optional`.
    pub fn to_json(&self) -> Value {
        Value::Array(self.settings.iter().map(Setting::to_json).collect())
    }
}

/// A kind for a key with no [`INFO`] entry, from its default value.
fn infer(default: &Value) -> InfoKind {
    match default {
        Value::Bool(_) => InfoKind::Toggle,
        Value::Number(_) => InfoKind::Number(f64::MIN, f64::MAX, 1.0, ""),
        Value::Array(_) => InfoKind::List,
        Value::Object(_) => InfoKind::Table,
        Value::Null | Value::String(_) => InfoKind::Text(true),
    }
}

/// "Tab width" from `display.tab_width`.
fn fallback_label(path: &str) -> String {
    let key = path.rsplit('.').next().unwrap_or(path).replace('_', " ");
    let mut c = key.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// The public kind for an [`InfoKind`]; choices stored as booleans (the
/// Eloquence dictionaries, quiet screen) become booleans.
fn kind_of(kind: InfoKind, default: &Value) -> SettingKind {
    match kind {
        InfoKind::Toggle => SettingKind::Toggle,
        InfoKind::Number(min, max, step, unit) => SettingKind::Number {
            min,
            max,
            step,
            unit,
        },
        InfoKind::Choice(choices, open) => SettingKind::Choice {
            choices: choices
                .iter()
                .map(|(v, label)| Choice {
                    value: match *v {
                        "true" => Value::Bool(true),
                        "false" => Value::Bool(false),
                        _ => Value::String((*v).to_owned()),
                    },
                    label: (*label).to_owned(),
                })
                .collect(),
            open,
        },
        InfoKind::Text(optional) => SettingKind::Text { optional },
        InfoKind::List => SettingKind::List,
        InfoKind::Table => SettingKind::Table,
        InfoKind::Internal => match default {
            Value::Bool(_) => SettingKind::Toggle,
            _ => SettingKind::Text { optional: true },
        },
    }
}

/// A number as it reads aloud: no trailing zeros ("1.5", "300").
fn number_text(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        let s = format!("{n:.2}");
        s.trim_end_matches('0').trim_end_matches('.').to_owned()
    }
}

/// A path or value as part of a message id: `speech-rate` for
/// `speech.rate`, `say-cap` for `say_cap`.
fn slug(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_owned()
}

/// A choice's value as part of its message id: `true` for a boolean
/// choice (the Eloquence dictionaries), the text otherwise.
fn value_key(v: &Value) -> String {
    match v {
        Value::Bool(b) => b.to_string(),
        other => plain(other),
    }
}

/// Message `id` in `c`, or `english` when no catalog has it (a theme or
/// font name, a setting added without a message).
/// A setting's unit after `value`, in the catalog's language: "1 word" and
/// "2 words" (`settings-unit-*`, a plural select on `$n`). A whole number
/// chooses the plural form; a fraction reads with the general one ("1.5
/// seconds").
fn unit_in(c: &Catalog, unit: &str, value: f64) -> String {
    let id = format!("settings-unit-{}", slug(unit));
    if !c.has(&id) {
        return unit.to_owned();
    }
    if value.fract() == 0.0 && value.abs() < 1e15 {
        c.fmt(&id, &args!["n" => value as i64])
    } else {
        c.fmt(&id, &args!["n" => number_text(value)])
    }
}

fn lookup(c: &Catalog, id: &str, english: &str) -> String {
    if c.has(id) {
        c.tr(id)
    } else {
        english.to_owned()
    }
}

/// The accessibility mode as the window offers it: two modes, "textweaver
/// reads aloud" (stored as hybrid; self-voicing shows as it too) and "my
/// screen reader reads" (stored as screen-reader). `settings.toml` keeps
/// its three values; the window's dialog and View menu show these two.
pub(crate) fn window_access_mode(base: &Setting) -> Setting {
    let mut s = base.clone();
    s.frontend = Frontend::Window;
    s.kind = SettingKind::Choice {
        choices: vec![
            Choice {
                value: Value::String("hybrid".into()),
                label: "textweaver reads aloud".into(),
            },
            Choice {
                value: Value::String("screen-reader".into()),
                label: "my screen reader reads".into(),
            },
        ],
        open: false,
    };
    s
}

/// The first sentence of `text`: up to the first period followed by a
/// space, unless it ends an abbreviation, a single character or a word
/// with a period inside ("z. B.", "e.g.").
fn first_sentence(text: &str) -> &str {
    let mut from = 0;
    while let Some(i) = text[from..].find(". ").map(|i| i + from) {
        let word = text[..i].rsplit(' ').next().unwrap_or_default();
        if !word.contains('.') && word.chars().count() > 1 {
            return &text[..=i];
        }
        from = i + 2;
    }
    text
}

impl Setting {
    /// True when `query` matches the setting, as the settings screens
    /// filter as you type: its label, section, path, help and unit, in
    /// English and in `c`'s language. An empty query matches every one.
    pub fn matches(&self, c: &Catalog, query: &str) -> bool {
        let unit = match &self.kind {
            SettingKind::Number { unit, .. } => unit,
            _ => "",
        };
        crate::lists::matches(
            &format!(
                "{} {} {} {} {unit} {} {} {}",
                self.label,
                self.section,
                self.path,
                self.help,
                self.label_in(c),
                self.section_in(c),
                self.help_in(c)
            ),
            query,
        )
    }

    /// The label in the catalog's language ("Rate"; `setting-*`).
    pub fn label_in(&self, c: &Catalog) -> String {
        lookup(c, &format!("setting-{}", slug(&self.path)), self.label)
    }

    /// The help in the catalog's language (`setting-*-help`).
    pub fn help_in(&self, c: &Catalog) -> String {
        if self.is_window_access_mode() {
            return c.tr("access-window-mode-help");
        }
        if self.help.is_empty() {
            return String::new();
        }
        lookup(c, &format!("setting-{}-help", slug(&self.path)), self.help)
    }

    /// True for the accessibility mode as the window offers it
    /// ([`window_access_mode`]).
    fn is_window_access_mode(&self) -> bool {
        self.path == "accessibility.mode" && self.frontend == Frontend::Window
    }

    /// True when the stored `value` is shown as `choice`: the same value,
    /// or, in the window's two modes, self-voicing shown as "textweaver
    /// reads aloud", whose stored value is hybrid.
    fn shows_as(&self, choice: &Value, value: &Value) -> bool {
        choice == value
            || (self.is_window_access_mode()
                && choice.as_str() == Some("hybrid")
                && value.as_str() == Some("self-voicing"))
    }

    /// The help's first sentence in the catalog's language: what is shown
    /// under the row and said as its description. F1 says the whole help.
    pub fn short_help_in(&self, c: &Catalog) -> String {
        first_sentence(&self.help_in(c)).to_owned()
    }

    /// The section title in the catalog's language (`section-*`).
    pub fn section_in(&self, c: &Catalog) -> String {
        let top = self.path.split('.').next().unwrap_or_default();
        lookup(c, &format!("section-{}", slug(top)), self.section)
    }

    /// A choice's label in the catalog's language (`choice-*`; a color
    /// setting's named colors are `color-name-*`, shared by all of them).
    pub fn choice_label_in(&self, c: &Catalog, choice: &Choice) -> String {
        if self.is_window_access_mode() {
            return c.tr(if choice.value.as_str() == Some("screen-reader") {
                "access-window-choice-screen-reader"
            } else {
                "access-window-choice-reads-aloud"
            });
        }
        if crate::colors::is_color_setting(&self.path) {
            return lookup(
                c,
                &format!("color-name-{}", slug(&value_key(&choice.value))),
                &choice.label,
            );
        }
        lookup(
            c,
            &format!(
                "choice-{}-{}",
                slug(&self.path),
                slug(&value_key(&choice.value))
            ),
            &choice.label,
        )
    }

    /// `value` as said and shown: "on", "300 words per minute", "the
    /// sentence", "not set", "3 entries".
    pub fn describe(&self, value: &Value) -> String {
        self.describe_in(&Catalog::english(), value)
    }

    /// [`describe`](Self::describe) in the catalog's language.
    pub fn describe_in(&self, c: &Catalog, value: &Value) -> String {
        match (&self.kind, value) {
            (_, Value::Null) => c.tr("settings-not-set"),
            (SettingKind::Toggle, Value::Bool(b)) => crate::words::on_off(c, *b),
            (SettingKind::Number { unit, .. }, Value::Number(n)) => {
                let value = n.as_f64().unwrap_or_default();
                let n = number_text(value);
                if unit.is_empty() {
                    n
                } else {
                    let unit = unit_in(c, unit, value);
                    c.fmt("settings-number-unit", &args!["n" => n, "unit" => unit])
                }
            }
            (SettingKind::Choice { choices, .. }, v) => choices
                .iter()
                .find(|x| self.shows_as(&x.value, v))
                .map(|x| self.choice_label_in(c, x))
                .unwrap_or_else(|| plain(v)),
            (SettingKind::List, Value::Array(items)) if items.is_empty() => c.tr("settings-none"),
            (SettingKind::List, Value::Array(items)) => items
                .iter()
                .map(plain)
                .collect::<Vec<_>>()
                .join(self.list_separator()),
            (SettingKind::Table, Value::Object(m)) => {
                c.fmt("settings-entries", &args!["n" => m.len()])
            }
            (_, Value::String(s)) if s.is_empty() => c.tr("settings-empty"),
            (_, v) => plain(v),
        }
    }

    /// What separates a list setting's items when typed or shown: "; "
    /// for folders, whose names may hold commas (a folder named
    /// "Readings, Fall 2026"), and ", " for everything else.
    pub fn list_separator(&self) -> &'static str {
        if self.path.ends_with("folders") {
            "; "
        } else {
            ", "
        }
    }

    /// The value as text for a prompt: a number, the text, or the list's
    /// items joined by [`list_separator`](Self::list_separator).
    pub fn edit_text(&self, value: &Value) -> String {
        match value {
            Value::Array(items) => items
                .iter()
                .map(plain)
                .collect::<Vec<_>>()
                .join(self.list_separator()),
            Value::Null => String::new(),
            v => plain(v),
        }
    }

    /// Parses text typed for this setting into a value: a number, on or
    /// off, a choice (by value or label), text, or a list separated by
    /// [`list_separator`](Self::list_separator).
    pub fn parse(&self, text: &str) -> Result<Value, String> {
        self.parse_in(&Catalog::english(), text)
    }

    /// [`parse`](Self::parse), also taking on, off, and the choices in
    /// the catalog's language, and saying what is wrong in it.
    pub fn parse_in(&self, c: &Catalog, text: &str) -> Result<Value, String> {
        let t = text.trim();
        match &self.kind {
            SettingKind::Toggle => {
                let lower = t.to_lowercase();
                if matches!(lower.as_str(), "on" | "yes" | "true" | "1")
                    || lower == c.tr("common-on").to_lowercase()
                {
                    Ok(Value::Bool(true))
                } else if matches!(lower.as_str(), "off" | "no" | "false" | "0")
                    || lower == c.tr("common-off").to_lowercase()
                {
                    Ok(Value::Bool(false))
                } else {
                    Err(c.tr("settings-type-on-or-off"))
                }
            }
            SettingKind::Number { min, max, step, .. } => {
                let n: f64 = t.replace(',', ".").parse().map_err(|_| {
                    c.fmt(
                        "settings-type-a-number",
                        &args!["min" => number_text(*min), "max" => number_text(*max)],
                    )
                })?;
                if n < *min || n > *max {
                    return Err(c.fmt(
                        "settings-outside",
                        &args![
                            "n" => number_text(n),
                            "min" => number_text(*min),
                            "max" => number_text(*max)
                        ],
                    ));
                }
                Ok(number_value(n, *step, &self.default))
            }
            SettingKind::Choice { choices, open } => {
                let lower = t.to_lowercase();
                if let Some(x) = choices.iter().find(|x| {
                    plain(&x.value).to_lowercase() == lower
                        || x.label.to_lowercase() == lower
                        || self.choice_label_in(c, x).to_lowercase() == lower
                }) {
                    return Ok(x.value.clone());
                }
                if *open && !t.is_empty() {
                    return Ok(Value::String(t.to_owned()));
                }
                let names: Vec<String> =
                    choices.iter().map(|x| self.choice_label_in(c, x)).collect();
                Err(c.fmt(
                    "settings-choose-one-of",
                    &args!["names" => names.join(", ")],
                ))
            }
            SettingKind::Text { optional } => {
                if t.is_empty() && *optional {
                    Ok(Value::Null)
                } else {
                    Ok(Value::String(text.to_owned()))
                }
            }
            SettingKind::List => Ok(Value::Array(
                t.split(self.list_separator().trim())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| Value::String(s.to_owned()))
                    .collect(),
            )),
            SettingKind::Table => {
                Err(c.fmt("settings-edit-table", &args!["label" => self.label_in(c)]))
            }
        }
    }

    /// The value one step from `value`: `forward` is larger, the next
    /// choice, or on. `None` for text, lists, and tables.
    pub fn stepped(&self, value: &Value, forward: bool) -> Option<Value> {
        match &self.kind {
            SettingKind::Toggle => Some(Value::Bool(!value.as_bool().unwrap_or(false))),
            SettingKind::Number { min, max, step, .. } => {
                let n = value.as_f64().unwrap_or(*min);
                let next = if forward { n + step } else { n - step };
                // Rounded, so 0.30000000000000004 reads 0.3.
                let rounded = (next * 1000.0).round() / 1000.0;
                Some(number_value(
                    rounded.clamp(*min, *max),
                    *step,
                    &self.default,
                ))
            }
            SettingKind::Choice { choices, .. } if !choices.is_empty() => {
                let n = choices.len();
                let at = choices.iter().position(|c| self.shows_as(&c.value, value));
                let i = match (at, forward) {
                    (Some(i), true) => (i + 1) % n,
                    (Some(i), false) => (i + n - 1) % n,
                    (None, _) => 0,
                };
                Some(choices[i].value.clone())
            }
            _ => None,
        }
    }

    fn to_json(&self) -> Value {
        let mut m = Map::new();
        m.insert("path".into(), Value::String(self.path.clone()));
        m.insert("section".into(), Value::String(self.section.into()));
        m.insert("label".into(), Value::String(self.label.into()));
        m.insert("help".into(), Value::String(self.help.into()));
        m.insert("default".into(), self.default.clone());
        m.insert("internal".into(), Value::Bool(self.internal));
        m.insert(
            "frontend".into(),
            Value::String(self.frontend.name().into()),
        );
        let kind = match &self.kind {
            SettingKind::Toggle => "toggle",
            SettingKind::Number {
                min,
                max,
                step,
                unit,
            } => {
                m.insert("min".into(), serde_json::json!(min));
                m.insert("max".into(), serde_json::json!(max));
                m.insert("step".into(), serde_json::json!(step));
                m.insert("unit".into(), Value::String((*unit).into()));
                "number"
            }
            SettingKind::Choice { choices, open } => {
                let list = choices
                    .iter()
                    .map(|c| serde_json::json!({ "value": c.value, "label": c.label }))
                    .collect();
                m.insert("choices".into(), Value::Array(list));
                m.insert("open".into(), Value::Bool(*open));
                "choice"
            }
            SettingKind::Text { optional } => {
                m.insert("optional".into(), Value::Bool(*optional));
                "text"
            }
            SettingKind::List => "list",
            SettingKind::Table => "table",
        };
        m.insert("kind".into(), Value::String(kind.into()));
        Value::Object(m)
    }
}

/// A number stored as the key's own type: whole for whole steps and whole
/// defaults.
fn number_value(n: f64, step: f64, default: &Value) -> Value {
    let whole = step.fract() == 0.0 && (default.is_i64() || default.is_u64());
    if whole {
        serde_json::json!(n.round() as i64)
    } else {
        serde_json::json!((n * 1000.0).round() / 1000.0)
    }
}

/// A JSON value as plain text.
fn plain(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Bool(b) => if *b { "on" } else { "off" }.to_owned(),
        other => other.to_string(),
    }
}

/// The settings screen's state while it is open.
#[derive(Clone, Debug)]
pub(crate) struct SettingsScreen {
    schema: SettingsSchema,
    /// Indexes into `schema.settings` of the items shown, in order.
    shown: Vec<usize>,
    /// The filter typed so far.
    pub(crate) filter: String,
    /// The setting whose new value the prompt is asking for, and the list
    /// item it was on (focused again when the list comes back).
    editing: Option<(usize, usize)>,
    /// A view of some settings only (View, Colors): the paths it shows.
    scope: Option<fn(&str) -> bool>,
    /// How many of the first items are the recently changed settings,
    /// listed again at the top (W6u).
    recent: usize,
}

/// Most recently changed settings listed at the top of the screen.
pub const RECENT_SETTINGS: usize = 5;

impl App {
    /// The settings schema, with this session's theme names as the theme's
    /// choices.
    pub fn settings_schema(&self) -> SettingsSchema {
        let mut schema = base_schema().clone();
        let themes: Vec<Choice> = self
            .themes
            .names()
            .into_iter()
            .map(|name| {
                // Grouped in words (W9b-d): the themes that meet AA come
                // first in the cycle and say so; the rest say "below AA".
                let theme = self.themes.resolve(name).0;
                let shown = self.theme_name_in_words(name, &theme.meta.display_name);
                let id = if textweaver_theme::check(theme).failures().count() == 0 {
                    "themes-choice-aa"
                } else {
                    "themes-choice-below-aa"
                };
                let label = self.msg_args(id, &args!["theme" => shown.as_str()]);
                Choice {
                    value: Value::String(name.to_owned()),
                    label,
                }
            })
            .collect();
        let window = self.uses_window_modes();
        for s in &mut schema.settings {
            if window && s.path == "accessibility.mode" {
                *s = window_access_mode(s);
            }
            if s.path == "display.theme"
                && let SettingKind::Choice { choices, .. } = &mut s.kind
            {
                choices.clone_from(&themes);
            }
            if s.path == "speech.backend"
                && let SettingKind::Choice { choices, .. } = &mut s.kind
            {
                // Only the engines that can exist on this system: no Apple
                // speech on Windows, no SAPI 5 on Linux. (The settings
                // reference, generated, lists them all.)
                choices.retain(|c| {
                    c.value
                        .as_str()
                        .is_none_or(textweaver_speech::backends::exists_on_this_os)
                });
            }
        }
        schema
    }

    /// The current value of the setting at `path`, as JSON.
    pub fn setting_value(&self, path: &str) -> Option<Value> {
        get(&json(&self.settings), path).cloned()
    }

    /// The values at each of `paths`, as [`setting_value`](Self::setting_value)
    /// gives one, from a single copy of the settings as JSON.
    pub fn setting_values(&self, paths: &[&str]) -> Vec<Option<Value>> {
        let tree = json(&self.settings);
        paths.iter().map(|p| get(&tree, p).cloned()).collect()
    }

    /// Changes the setting at `path` to `value` (JSON; `null` puts the
    /// default back), checked against the store's types and ranges, puts
    /// it into effect, and saves it. Returns what is said ("Rate, 300
    /// words per minute."), or why it was refused.
    pub fn set_setting(&mut self, path: &str, value: Value) -> Result<String, String> {
        let schema = self.settings_schema();
        let setting = schema
            .get(path)
            .ok_or_else(|| self.msg_args("settings-no-such-setting", &args!["path" => path]))?;
        let value =
            if value.is_null() && !matches!(setting.kind, SettingKind::Text { optional: true }) {
                setting.default.clone()
            } else {
                value
            };
        let mut tree = json(&self.settings);
        set(&mut tree, path, value);
        let mut new: Settings = serde_json::from_value(Value::Object(tree)).map_err(|e| {
            self.msg_args(
                "settings-cannot-be",
                &args!["label" => setting.label_in(self.cat()), "error" => e.to_string()],
            )
        })?;
        let clamped = new.validate();
        let old = std::mem::replace(&mut self.settings, new);
        self.settings_dirty = true;
        self.settings_changed(&old, path);
        let now = self.setting_value(path).unwrap_or(Value::Null);
        // Said in the language now in effect (a language change is heard
        // in the new language).
        let c = self.catalog();
        let mut said = c.fmt(
            "settings-changed",
            &args!["label" => setting.label_in(&c), "value" => setting.describe_in(&c, &now)],
        );
        if !clamped.is_empty() {
            said.push(' ');
            said.push_str(&c.tr("settings-clamped"));
        }
        if let Some(extra) = restart_note(path) {
            said.push(' ');
            said.push_str(&c.tr(extra));
        }
        if crate::colors::is_color_setting(path) {
            let note = self.color_change_note(path);
            if !note.is_empty() {
                said.push(' ');
                said.push_str(&note);
            }
        }
        if !setting.internal {
            self.remember_setting(path);
        }
        if path == "reading_aids.font.family" {
            // A reading font that is downloaded on first choice: asked after
            // this change is said (crate::font_download).
            self.fonts.offer_pending = true;
        }
        if path == "interface.language" {
            // In the new language: what happened to the voice, then the
            // title line, so the change is heard to have worked.
            if let Some(note) = self.language_note.take() {
                said.push(' ');
                said.push_str(&note);
            }
            said.push(' ');
            said.push_str(&self.status_sentence());
        }
        Ok(said)
    }

    /// Puts a changed setting into effect: the voice, the speech pipeline,
    /// the keys, the accessibility mode, the theme, the library.
    pub(crate) fn settings_changed(&mut self, old: &Settings, path: &str) {
        let top = path.split('.').next().unwrap_or_default();
        if matches!(top, "speech" | "normalization" | "highlight") || path == "reading.math_engine"
        {
            let config = textweaver_engines::service_config(&self.settings);
            self.speech.set_normalization(self.speech_normalization());
            self.speech.set_pacing(config.pacing);
            self.speech.set_pauses(config.pauses);
            self.apply_voice_settings();
            if self.settings.speech.voice != old.speech.voice {
                self.speech.set_voice(self.settings.speech.voice.clone());
            }
            if self.settings.speech.output_device != old.speech.output_device {
                // Open outputs move to the new device on their next look.
                textweaver_engines::apply_output_device(&self.settings);
            }
        }
        if self.settings.keyboard.character_keys != old.keyboard.character_keys {
            self.keymap
                .set_character_keys(self.settings.keyboard.character_keys);
        }
        if self.settings.accessibility.mode != old.accessibility.mode {
            self.access_mode =
                crate::access::access_mode_from_setting(self.settings.accessibility.mode);
        }
        if self.settings.accessibility.mode != old.accessibility.mode
            || self.settings.gui.speak_messages != old.gui.speak_messages
        {
            // The window's two modes and its switch (W9b-f).
            self.apply_window_mode();
        }
        if self.settings.display.theme != old.display.theme {
            self.settings.display.theme_explicit = true;
            self.check_theme_name();
        }
        // Turning "Follow the system theme" on again means it: a theme
        // picked earlier (one F5) no longer stops it for good.
        if self.settings.display.follow_os_theme && !old.display.follow_os_theme {
            self.settings.display.theme_explicit = false;
        }
        if top == "highlight" || self.settings.display.theme != old.display.theme {
            self.check_reading_colors();
        }
        if self.settings.interface.language != old.interface.language {
            self.apply_interface_language();
        }
        if self.settings.library.folders != old.library.folders
            || self.settings.sync.position_policy != old.sync.position_policy
        {
            self.library_sync = Self::make_library_sync(&self.settings);
        }
    }

    /// [`Command::SetSetting`](crate::Command::SetSetting).
    pub(crate) fn set_setting_command(&mut self, path: &str, value: Value) -> Vec<Effect> {
        let mut asked = Vec::new();
        match self.set_setting(path, value) {
            Ok(said) => {
                self.tell(&said);
                if self.fonts.offer_pending && !self.confirmation_pending() {
                    asked = self.offer_font_download();
                }
            }
            Err(why) => self.error(&why),
        }
        let mut effects = self.refresh_settings_screen();
        effects.extend(asked);
        effects
    }

    /// The Settings command: the settings screen, with the settings changed
    /// last at the top.
    pub(crate) fn open_settings_screen(&mut self) -> Vec<Effect> {
        self.open_settings_screen_with(None, "settings-intro")
    }

    /// The settings screen showing the settings `scope` takes (every one
    /// for `None`), introduced by message `intro` (with `$n`).
    pub(crate) fn open_settings_screen_with(
        &mut self,
        scope: Option<fn(&str) -> bool>,
        intro: &str,
    ) -> Vec<Effect> {
        let schema = self.settings_schema();
        self.settings_screen = Some(SettingsScreen {
            schema,
            shown: Vec::new(),
            filter: String::new(),
            editing: None,
            scope,
            recent: 0,
        });
        self.fill_settings_rows();
        let n = self
            .settings_screen
            .as_ref()
            .map_or(0, |s| s.shown.len() - s.recent);
        let msg = self.msg_args(intro, &args!["n" => n]);
        self.say_result(&msg);
        self.show_settings_list()
    }

    /// Works out the rows of the settings screen: the recently changed
    /// settings first (without a filter, on the whole screen), then every
    /// setting the view shows that matches the filter.
    fn fill_settings_rows(&mut self) {
        let c = self.catalog();
        let recent_paths = self.settings.interface.recent_settings.clone();
        let Some(screen) = self.settings_screen.as_mut() else {
            return;
        };
        let query = screen.filter.clone();
        let scope = screen.scope;
        let visible = |s: &Setting| {
            !s.internal && s.frontend.in_terminal() && scope.is_none_or(|f| f(&s.path))
        };
        let mut shown: Vec<usize> = Vec::new();
        if query.trim().is_empty() && scope.is_none() {
            for path in recent_paths.iter().take(RECENT_SETTINGS) {
                if let Some(i) = screen
                    .schema
                    .settings
                    .iter()
                    .position(|s| &s.path == path && visible(s))
                {
                    shown.push(i);
                }
            }
        }
        screen.recent = shown.len();
        shown.extend((0..screen.schema.settings.len()).filter(|&i| {
            let s = &screen.schema.settings[i];
            visible(s) && s.matches(&c, &query)
        }));
        screen.shown = shown;
    }

    /// Records `path` as the setting changed last (the top of the settings
    /// screen), saved with the settings.
    pub(crate) fn remember_setting(&mut self, path: &str) {
        let recent = &mut self.settings.interface.recent_settings;
        recent.retain(|p| p != path);
        recent.insert(0, path.to_owned());
        recent.truncate(RECENT_SETTINGS);
        self.settings_dirty = true;
    }

    /// The settings list's title and items, for the frontend.
    fn show_settings_list(&mut self) -> Vec<Effect> {
        let Some(screen) = self.settings_screen.as_ref() else {
            return vec![Effect::Redraw];
        };
        let tree = json(&self.settings);
        let c = self.cat();
        let items: Vec<String> = screen
            .shown
            .iter()
            .enumerate()
            .map(|(row, &i)| {
                let s = &screen.schema.settings[i];
                let v = get(&tree, &s.path).cloned().unwrap_or(Value::Null);
                let (label, value) = (s.label_in(c), s.describe_in(c, &v));
                let line = if crate::colors::is_color_setting(&s.path) {
                    self.color_row(c, &label, &value, &s.path)
                } else {
                    c.fmt("settings-item", &args!["label" => label, "value" => value])
                };
                if row < screen.recent {
                    c.fmt("settings-item-recent", &args!["item" => line])
                } else {
                    line
                }
            })
            .collect();
        let title = if screen.filter.is_empty() {
            c.tr("settings-title")
        } else {
            c.fmt(
                "settings-title-matching",
                &args!["filter" => screen.filter.as_str()],
            )
        };
        self.list = Some(ListKind::Settings);
        vec![Effect::ShowList { title, items }]
    }

    /// The list again after a change, when the screen is open.
    fn refresh_settings_screen(&mut self) -> Vec<Effect> {
        if self.settings_screen.is_some() && self.list == Some(ListKind::Settings) {
            self.show_settings_list()
        } else {
            vec![Effect::Redraw]
        }
    }

    /// Escape on the settings screen.
    pub(crate) fn close_settings_screen(&mut self) {
        self.settings_screen = None;
        let msg = self.msg("settings-closed");
        self.say_dialog(&msg);
    }

    /// The filter of the settings screen, when it is showing.
    pub(crate) fn settings_filter(&self) -> Option<&str> {
        match (&self.list, &self.settings_screen) {
            (Some(ListKind::Settings), Some(s)) => Some(&s.filter),
            _ => None,
        }
    }

    /// The settings screen's filter changed to `query`.
    pub(crate) fn filter_settings(&mut self, query: String) -> Vec<Effect> {
        let c = self.catalog();
        let Some(screen) = self.settings_screen.as_mut() else {
            return vec![Effect::Redraw];
        };
        screen.filter = query.clone();
        self.fill_settings_rows();
        let n = self
            .settings_screen
            .as_ref()
            .map_or(0, |s| s.shown.len() - s.recent);
        let msg = if query.trim().is_empty() {
            c.fmt("settings-filter-cleared", &args!["n" => n])
        } else if n == 0 {
            c.fmt("settings-filter-none", &args!["query" => query.as_str()])
        } else {
            c.fmt("settings-filter-match", &args!["n" => n])
        };
        self.tell(&msg);
        self.show_settings_list()
    }

    /// The setting shown as item `n`.
    fn shown_setting(&self, n: usize) -> Option<(usize, Setting)> {
        let screen = self.settings_screen.as_ref()?;
        let i = *screen.shown.get(n)?;
        Some((i, screen.schema.settings[i].clone()))
    }

    /// Keys the settings screen handles itself: Left, Right, Enter,
    /// Delete. `None` leaves the key to the list.
    pub(crate) fn settings_list_key(&mut self, key: ListKey) -> Option<Vec<Effect>> {
        if self.list != Some(ListKind::Settings) {
            return None;
        }
        let n = self.list_model.as_ref().map_or(0, |l| l.selected);
        match key {
            ListKey::Left | ListKey::Right => Some(self.step_setting(n, key == ListKey::Right)),
            ListKey::Enter => Some(self.choose_setting(n)),
            ListKey::Delete => Some(self.reset_setting(n)),
            ListKey::Introduce => Some(self.say_setting_help(n)),
            _ => None,
        }
    }

    /// Delete on item `n`: the setting's default comes back, named
    /// ("Rate back to its default, 200 words per minute.").
    fn reset_setting(&mut self, n: usize) -> Vec<Effect> {
        let Some((_, s)) = self.shown_setting(n) else {
            return vec![Effect::Redraw];
        };
        match self.set_setting(&s.path, Value::Null) {
            Ok(_) => {
                let c = self.catalog();
                let now = self.setting_value(&s.path).unwrap_or(Value::Null);
                let msg = c.fmt(
                    "settings-reset",
                    &args!["label" => s.label_in(&c), "value" => s.describe_in(&c, &now)],
                );
                self.tell(&msg);
            }
            Err(why) => self.error(&why),
        }
        self.pending_list_focus = Some(n);
        self.refresh_settings_screen()
    }

    /// F1 on item `n`: the setting's value, its default, and its help.
    fn say_setting_help(&mut self, n: usize) -> Vec<Effect> {
        let Some((_, s)) = self.shown_setting(n) else {
            return self.repeat_list_introduction();
        };
        let c = self.catalog();
        let now = self.setting_value(&s.path).unwrap_or(Value::Null);
        let msg = c.fmt(
            "settings-row-help",
            &args![
                "label" => s.label_in(&c),
                "value" => s.describe_in(&c, &now),
                "default" => s.describe_in(&c, &s.default),
                "help" => s.help_in(&c)
            ],
        );
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    /// Left or Right on item `n`.
    fn step_setting(&mut self, n: usize, forward: bool) -> Vec<Effect> {
        let Some((_, s)) = self.shown_setting(n) else {
            return vec![Effect::Redraw];
        };
        let now = self.setting_value(&s.path).unwrap_or(Value::Null);
        match s.stepped(&now, forward) {
            Some(v) if v == now => {
                self.speech.earcon(textweaver_speech::Earcon::Boundary);
                let id = if forward {
                    "settings-largest"
                } else {
                    "settings-smallest"
                };
                let msg = self.msg_args(id, &args!["value" => s.describe_in(self.cat(), &now)]);
                self.tell(&msg);
                vec![Effect::Redraw]
            }
            Some(v) => self.set_setting_command(&s.path, v),
            None => {
                let msg = self.msg_args(
                    "settings-press-enter",
                    &args!["label" => s.label_in(self.cat())],
                );
                self.tell(&msg);
                vec![Effect::Redraw]
            }
        }
    }

    /// Enter on item `n` (or [`Command::Choose`](crate::Command::Choose)
    /// from a GUI or JSON-RPC): turns a switch on or off, takes the next
    /// choice, or asks for a new value.
    pub(crate) fn choose_setting(&mut self, n: usize) -> Vec<Effect> {
        // `choose` took the list kind; the screen stays.
        self.list = Some(ListKind::Settings);
        let Some((i, s)) = self.shown_setting(n) else {
            return vec![Effect::Redraw];
        };
        let now = self.setting_value(&s.path).unwrap_or(Value::Null);
        // The list is shown again on the same item (a `Choose` from a GUI
        // or JSON-RPC closed it).
        let keep_focus = matches!(
            s.kind,
            SettingKind::Toggle | SettingKind::Table | SettingKind::Choice { open: false, .. }
        );
        if keep_focus {
            self.pending_list_focus = Some(n);
        }
        match &s.kind {
            SettingKind::Toggle => {
                self.set_setting_command(&s.path, Value::Bool(!now.as_bool().unwrap_or(false)))
            }
            SettingKind::Choice { open: false, .. } => match s.stepped(&now, true) {
                Some(v) => self.set_setting_command(&s.path, v),
                None => self.show_settings_list(),
            },
            SettingKind::Table => {
                let msg = self.msg_args(
                    "settings-table-item",
                    &args!["label" => s.label_in(self.cat()), "value" => s.describe_in(self.cat(), &now)],
                );
                self.tell(&msg);
                self.show_settings_list()
            }
            _ => {
                if let Some(screen) = self.settings_screen.as_mut() {
                    screen.editing = Some((i, n));
                }
                self.list = None;
                let mut effects = self.prompt(PromptPurpose::SettingValue);
                let current = s.edit_text(&now);
                let c = self.catalog();
                let msg = c.fmt(
                    "settings-editing",
                    &args![
                        "label" => s.label_in(&c),
                        "value" => s.describe_in(&c, &now),
                        "help" => s.short_help_in(&c)
                    ],
                );
                self.tell(&msg);
                if let Some(Effect::Prompt { label, .. }) = effects.first_mut() {
                    *label = format!("{} ({})", label, s.label_in(&c));
                }
                self.pending_prompt_text = Some(current);
                effects
            }
        }
    }

    /// The answer to the new-value prompt.
    pub(crate) fn answer_setting_value(&mut self, text: &str) -> Vec<Effect> {
        let Some((i, item)) = self.settings_screen.as_mut().and_then(|s| s.editing.take()) else {
            return vec![Effect::Redraw];
        };
        self.pending_list_focus = Some(item);
        let Some(s) = self
            .settings_screen
            .as_ref()
            .map(|screen| screen.schema.settings[i].clone())
        else {
            return vec![Effect::Redraw];
        };
        let now = self.setting_value(&s.path).unwrap_or(Value::Null);
        let may_unset = matches!(s.kind, SettingKind::Text { optional: true });
        if text.trim() == s.edit_text(&now).trim() || (text.trim().is_empty() && !may_unset) {
            let msg = self.msg("common-kept");
            self.note(&msg);
            return self.show_settings_list();
        }
        match s.parse_in(self.cat(), text) {
            Ok(v) => {
                let said = self.set_setting(&s.path, v);
                match said {
                    Ok(m) => self.tell(&m),
                    Err(why) => self.error(&why),
                }
            }
            Err(why) => self.error(&why),
        }
        self.show_settings_list()
    }

    /// Cancelling the new-value prompt goes back to the list.
    pub(crate) fn cancel_setting_value(&mut self) -> Vec<Effect> {
        if let Some((_, item)) = self.settings_screen.as_mut().and_then(|s| s.editing.take()) {
            self.pending_list_focus = Some(item);
        }
        let msg = self.msg("common-kept");
        self.note(&msg);
        self.show_settings_list()
    }
}

/// What to add when a setting takes effect only later: a message id.
fn restart_note(path: &str) -> Option<&'static str> {
    match path {
        "speech.backend"
        | "speech.eci.dictionaries"
        | "speech.eci.library"
        | "speech.eci.code_factory"
        | "speech.sapi.onecore"
        | "speech.apple.backend"
        | "speech.dectalk.library"
        | "speech.piper.voices"
        | "speech.piper.voice"
        | "speech.piper.phonemizer" => Some("settings-restart-speech"),
        "keyboard.preset" | "keyboard.digit_row" => Some("settings-next-start"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_short_help_is_the_first_sentence() {
        assert_eq!(first_sentence("One. Two."), "One.");
        assert_eq!(
            first_sentence("From 25 to 90. 0 fills it."),
            "From 25 to 90."
        );
        assert_eq!(
            first_sentence("Codes, z. B. deu. Leer."),
            "Codes, z. B. deu."
        );
        assert_eq!(
            first_sentence("Such as e.g. this. More."),
            "Such as e.g. this."
        );
        assert_eq!(first_sentence("Which one. ocrs or not."), "Which one.");
        assert_eq!(first_sentence("Only one."), "Only one.");
    }

    use super::*;

    /// Every key of `settings.toml` has an [`INFO`] entry, and every entry
    /// names a key that exists.
    /// Every label, help, section, choice, and unit shown on the settings
    /// screen is in en.ftl, as INFO words it, so the catalog cannot drift
    /// from the schema (W4d).
    #[test]
    fn english_catalog_matches_the_schema() {
        let c = Catalog::english();
        for s in SettingsSchema::generate().visible() {
            let id = format!("setting-{}", slug(&s.path));
            assert!(c.has(&id), "en.ftl has no {id}");
            assert_eq!(s.label_in(&c), s.label, "{id}");
            assert_eq!(s.help_in(&c), s.help, "{id}-help");
            assert_eq!(s.section_in(&c), s.section, "{}", s.path);
            match &s.kind {
                SettingKind::Choice { choices, .. } => {
                    for x in choices {
                        assert_eq!(
                            s.choice_label_in(&c, x),
                            x.label,
                            "{} {:?}",
                            s.path,
                            x.value
                        );
                    }
                }
                SettingKind::Number { unit, .. } if !unit.is_empty() => {
                    assert!(c.has(&format!("settings-unit-{}", slug(unit))), "{unit}");
                }
                _ => {}
            }
        }
        // Spanish on the settings screen, and a Spanish answer is taken.
        let es = Catalog::builtin("es").unwrap();
        let schema = SettingsSchema::generate();
        let toggle = schema.get("speech.auto_play").unwrap();
        assert_eq!(
            toggle.parse_in(&es, &es.tr("common-on")),
            Ok(Value::Bool(true))
        );
        assert_eq!(slug("reading_aids.rsvp.wpm"), "reading-aids-rsvp-wpm");
        assert_eq!(value_key(&Value::Bool(true)), "true");
    }

    #[test]
    fn the_schema_covers_every_setting() {
        let defaults = json(&Settings::default());
        let tables: Vec<&str> = INFO
            .iter()
            .filter(|i| matches!(i.kind, InfoKind::Table))
            .map(|i| i.path)
            .collect();
        let mut found = Vec::new();
        leaves("", &defaults, &tables, &mut found);
        let missing: Vec<&str> = found
            .iter()
            .map(|(p, _)| p.as_str())
            .filter(|p| !INFO.iter().any(|i| i.path == *p))
            .collect();
        assert!(
            missing.is_empty(),
            "settings with no label or help; add them to INFO in crates/textweaver-app/src/settings_schema.rs: {missing:?}"
        );
        let stale: Vec<&str> = INFO
            .iter()
            .map(|i| i.path)
            .filter(|p| !found.iter().any(|(f, _)| f == p))
            .collect();
        assert!(
            stale.is_empty(),
            "INFO entries for keys the store no longer has: {stale:?}"
        );
        for i in INFO {
            assert!(!i.label.is_empty() && !i.help.is_empty(), "{}", i.path);
            assert!(
                i.help.ends_with('.'),
                "help reads as a sentence: {}",
                i.path
            );
        }
    }

    /// Every default is inside its range and is one of its choices.
    #[test]
    fn defaults_fit_their_kinds() {
        let schema = SettingsSchema::generate();
        for s in &schema.settings {
            match (&s.kind, &s.default) {
                (SettingKind::Number { min, max, .. }, Value::Number(n)) => {
                    let n = n.as_f64().unwrap_or_default();
                    assert!(*min <= n && n <= *max, "{} default {n}", s.path);
                }
                (
                    SettingKind::Choice {
                        choices,
                        open: false,
                    },
                    v,
                ) => {
                    assert!(
                        choices.iter().any(|c| &c.value == v),
                        "{} default {v} is not a choice",
                        s.path
                    );
                }
                (SettingKind::Toggle, v) => assert!(v.is_boolean(), "{}", s.path),
                _ => {}
            }
        }
    }

    /// Every choice is a value the store accepts.
    #[test]
    fn every_choice_deserializes() {
        let schema = SettingsSchema::generate();
        for s in &schema.settings {
            let SettingKind::Choice { choices, .. } = &s.kind else {
                continue;
            };
            for c in choices {
                let mut tree = json(&Settings::default());
                set(&mut tree, &s.path, c.value.clone());
                let back: Settings = serde_json::from_value(Value::Object(tree))
                    .unwrap_or_else(|e| panic!("{} = {}: {e}", s.path, c.value));
                let again = json(&back);
                assert_eq!(get(&again, &s.path), Some(&c.value), "{}", s.path);
            }
        }
    }

    #[test]
    fn values_read_aloud_parse_and_step() {
        let schema = SettingsSchema::generate();
        let rate = schema.get("speech.rate").unwrap();
        assert_eq!(
            rate.describe(&serde_json::json!(300)),
            "300 words per minute"
        );
        assert_eq!(rate.parse("320"), Ok(serde_json::json!(320)));
        assert!(rate.parse("5").unwrap_err().contains("outside 50 to 900"));
        assert_eq!(
            rate.stepped(&serde_json::json!(890), true),
            Some(serde_json::json!(900))
        );
        let speed = schema.get("highlight.speed").unwrap();
        assert_eq!(
            speed.stepped(&serde_json::json!(1.0), true),
            Some(serde_json::json!(1.1))
        );
        let g = schema.get("highlight.granularity").unwrap();
        assert_eq!(
            g.describe(&serde_json::json!("both")),
            "the word and the sentence"
        );
        assert_eq!(g.parse("The sentence"), Ok(serde_json::json!("sentence")));
        let dict = schema.get("speech.eci.dictionaries").unwrap();
        assert_eq!(dict.describe(&Value::Bool(true)), "on");
        assert_eq!(
            dict.parse("C:\\dicts"),
            Ok(Value::String("C:\\dicts".into()))
        );
        let folders = schema.get("library.folders").unwrap();
        assert_eq!(folders.parse("a; b ;"), Ok(serde_json::json!(["a", "b"])));
        let voices = schema.get("speech.favorite_voices").unwrap();
        assert_eq!(voices.parse("a, b ,"), Ok(serde_json::json!(["a", "b"])));
        let voice = schema.get("speech.voice").unwrap();
        assert_eq!(voice.parse(" "), Ok(Value::Null));
        assert_eq!(voice.describe(&Value::Null), "not set");
        assert!(schema.get("display.theme_explicit").unwrap().internal);
        assert_eq!(fallback_label("display.tab_width"), "Tab width");
    }

    #[test]
    fn a_folder_with_a_comma_survives_a_round_trip() {
        let schema = SettingsSchema::generate();
        let folders = schema.get("library.folders").unwrap();
        let typed = "Readings, Fall 2026; Notes";
        let value = folders.parse(typed).unwrap();
        assert_eq!(value, serde_json::json!(["Readings, Fall 2026", "Notes"]));
        assert_eq!(folders.edit_text(&value), typed);
        assert_eq!(folders.parse(&folders.edit_text(&value)), Ok(value.clone()));
        // Through the settings file and back.
        let dir = tempfile::tempdir().unwrap();
        let store =
            textweaver_store::SettingsStore::new(textweaver_store::Paths::under(dir.path()));
        let mut s = textweaver_store::Settings::default();
        s.library.folders = serde_json::from_value(value.clone()).unwrap();
        store.save(&s).unwrap();
        let loaded = store.load().0;
        let back = serde_json::to_value(&loaded.library.folders).unwrap();
        assert_eq!(folders.edit_text(&back), typed);
    }

    #[test]
    fn the_schema_as_json_has_what_a_dialog_needs() {
        let j = SettingsSchema::generate().to_json();
        let rate = j
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["path"] == "speech.rate")
            .unwrap();
        assert_eq!(rate["kind"], "number");
        assert_eq!(rate["unit"], "words per minute");
        assert_eq!(rate["label"], "Rate");
    }

    /// Units read grammatically: "1 word", "2 words", "1.5 seconds", never
    /// "1 words" (the words report, QW4), in English and the others.
    #[test]
    fn units_agree_with_the_number() {
        let schema = SettingsSchema::generate();
        let en = Catalog::english();
        for s in &schema.settings {
            let SettingKind::Number { unit, .. } = &s.kind else {
                continue;
            };
            if unit.is_empty() {
                continue;
            }
            let one = s.describe_in(&en, &serde_json::json!(1));
            let unit_said = one.trim_start_matches("1 ");
            let plural = [
                "words", "rows", "times", "lines", "seconds", "steps", "points",
            ];
            assert!(
                !plural.contains(&unit_said) && !unit_said.ends_with(" words per minute"),
                "{}: {one}",
                s.path
            );
        }
        let lead = schema.get("highlight.lead_words").unwrap();
        assert_eq!(lead.describe(&serde_json::json!(1)), "1 word");
        assert_eq!(lead.describe(&serde_json::json!(3)), "3 words");
        let rows = schema.get("reading_aids.ruler.rows_above").unwrap();
        assert_eq!(rows.describe(&serde_json::json!(1)), "1 row");
        assert_eq!(rows.describe(&serde_json::json!(0)), "0 rows");
        // A multiple of the font size has no unit: its help says what it is.
        let spacing = schema
            .get("reading_aids.spacing.paragraph_spacing")
            .unwrap();
        assert_eq!(spacing.describe(&serde_json::json!(1)), "1");
        let de = Catalog::builtin("de").unwrap();
        assert_eq!(lead.describe_in(&de, &serde_json::json!(1)), "1 Wort");
        assert_eq!(lead.describe_in(&de, &serde_json::json!(2)), "2 Wörter");
    }
}
