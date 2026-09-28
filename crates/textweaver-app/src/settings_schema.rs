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
    /// A list of texts (favourite voices, library folders), typed as
    /// comma-separated items.
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
        "The voice's id; not set picks one automatically. Choose Voice lists them.",
    ),
    optional(
        "speech.prefer_voice",
        "Preferred voice",
        "When no voice is set, the first voice whose name contains this, such as eloquence.",
    ),
    list(
        "speech.favorite_voices",
        "Favourite voices",
        "Voices listed first in Choose Voice, by id.",
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
        "Also look for Code Factory's Eloquence for Windows. Its licence may not cover other programs.",
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
        "Speed of the timed highlight for engines that report no words.",
        (0.5, 1.5, 0.1),
        "times",
    ),
    text(
        "highlight.color",
        "Word highlight colour",
        "A colour name or #rrggbb over the theme's word highlight; theme keeps the theme's.",
    ),
    optional(
        "highlight.sentence_color",
        "Sentence highlight colour",
        "A colour name or #rrggbb over the theme's sentence highlight; not set keeps the theme's.",
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
        "reading.sync_conflict_policy",
        "Synced positions",
        "Which position wins when another device read further or later.",
        &[
            ("newest", "the newest"),
            ("highest_progress", "the furthest"),
            ("manual", "ask"),
        ],
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
        "The language of scanned text, as Tesseract codes such as fra or deu+eng; empty means the document's own language, else English.",
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
        "Which engine recognizes scanned pages: ocrs for English and Tesseract for other languages, or one of them always.",
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
        "Which engine reads math aloud: textweaver's own, or MathCAT in ClearSpeak or SimpleSpeak, in the document's language. MathCAT needs a build that includes it; otherwise textweaver's own is used.",
        &[
            ("builtin", "textweaver"),
            ("mathcat", "MathCAT ClearSpeak"),
            ("mathcat_simplespeak", "MathCAT SimpleSpeak"),
        ],
    ),
    choice(
        "reading.math_display",
        "Math on screen",
        "How math looks in the reading view: as its source, such as x^2, or as Unicode, such as x with a superscript 2. Speech and edit mode always use the source.",
        &[("source", "source"), ("unicode", "Unicode")],
    ),
    choice(
        "reading.revisions",
        "Tracked changes",
        "How tracked changes in Word, OpenDocument, and RTF files are read: said in place at high verbosity (automatic), always said, or never said, reading the final text. Applies when a document is opened.",
        &[
            ("auto", "automatic"),
            ("marked", "always say them"),
            ("final", "final text only"),
        ],
    ),
    // [display]
    open_choice("display.theme", "Theme", "The colour theme.", &[]),
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
        "Say the line when the caret moves to another line.",
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
        "Folders whose documents the library lists, and whose positions sync between computers.",
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
        "How the terminal recognises the digit keys for heading levels: auto, or a French AZERTY keyboard.",
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
        "Continuous reading in screen-reader mode: a sentence at a time on the status line, or textweaver's voice.",
        &[
            ("screen", "on the status line"),
            ("voice", "with textweaver's voice"),
        ],
    ),
    toggle(
        "accessibility.quiet_screen",
        "Quiet screen while reading",
        "Keep the screen still while textweaver reads aloud.",
    ),
    choice(
        "accessibility.cursor",
        "Cursor",
        "Where the terminal's cursor waits: on what you are working on, or on the status line.",
        &[
            ("follow", "follows focus"),
            ("status", "on the status line"),
        ],
    ),
    internal(
        "accessibility.hybrid_offered",
        "Hybrid mode offered",
        "Set once textweaver has asked whether to use hybrid mode.",
    ),
    // [export]
    choice(
        "export.subtitle_format",
        "Subtitle format",
        "The format of subtitles written without a file name.",
        &[("srt", "SubRip"), ("vtt", "WebVTT")],
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
            ("top-center", "top centre"),
            ("top-right", "top right"),
            ("center-left", "middle left"),
            ("center", "middle"),
            ("center-right", "middle right"),
            ("bottom-left", "bottom left"),
            ("bottom-center", "bottom centre"),
            ("bottom-right", "bottom right"),
        ],
    ),
    number(
        "reading_aids.rsvp.font_size_pt",
        "RSVP size",
        "Size of the RSVP word in the GUI.",
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
        "Leave web and e-mail addresses alone.",
    ),
    toggle(
        "reading_aids.bionic_options.skip_code",
        "Bionic skips code",
        "Leave code alone.",
    ),
    number(
        "reading_aids.spacing.line_height",
        "Line height",
        "Line height in multiples of the font size; WCAG's value is 1.5.",
        (1.0, 3.0, 0.1),
        "times",
    ),
    number(
        "reading_aids.spacing.paragraph_spacing",
        "Paragraph spacing",
        "Space after each paragraph, in multiples of the font size.",
        (0.0, 4.0, 0.25),
        "times",
    ),
    number(
        "reading_aids.spacing.letter_spacing",
        "Letter spacing",
        "Extra space between letters, in multiples of the font size.",
        (0.0, 0.5, 0.02),
        "times",
    ),
    number(
        "reading_aids.spacing.word_spacing",
        "Word spacing",
        "Extra space between words, in multiples of the font size.",
        (0.0, 1.0, 0.04),
        "times",
    ),
    open_choice(
        "reading_aids.font.family",
        "Font",
        "The GUI's reading font; any installed family may be typed.",
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
        "The GUI's font size.",
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
    toggle(
        "reading_aids.font.fetch_missing",
        "Offer missing fonts",
        "Offer to download a reading font that is not installed, after asking.",
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
        "Leave web and e-mail addresses alone.",
    ),
    toggle(
        "reading_aids.syllable_options.skip_code",
        "Syllables skip code",
        "Leave code alone.",
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
        "Your own glossary, looked up before the dictionary: term: definition lines, or Star's JSON. Not set uses glossary.txt in the settings folder.",
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
            ("en-XA", "test: accented"),
            ("ar-XB", "test: right to left"),
        ],
    ),
    choice(
        "interface.rtl",
        "Right-to-left display",
        "Whether the terminal reader reorders right-to-left text for display: automatic leaves it to terminals that do it themselves. Speech and the screen reader always get the text in reading order.",
        &[("auto", "automatic"), ("on", "on"), ("off", "off")],
    ),
    // [gui]
    choice(
        "gui.announce",
        "Announcements",
        "How the window's messages reach the screen reader, from the next start: a live region, or UI Automation notifications (Windows only).",
        &[
            ("live", "live region"),
            ("uia", "UI Automation notifications"),
        ],
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
        "reading_aids" => "Reading aids",
        "preview" => "Preview",
        "lexicon" => "Define word",
        "stats" => "Reading statistics",
        "interface" => "Interface",
        "gui" => "Window",
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
                Setting {
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
/// Eloquence dictionaries) become booleans.
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
                    value: match (*v, default) {
                        ("true", Value::Bool(_)) => Value::Bool(true),
                        ("false", Value::Bool(_)) => Value::Bool(false),
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
fn lookup(c: &Catalog, id: &str, english: &str) -> String {
    if c.has(id) {
        c.tr(id)
    } else {
        english.to_owned()
    }
}

impl Setting {
    /// The label in the catalog's language ("Rate"; `setting-*`).
    pub fn label_in(&self, c: &Catalog) -> String {
        lookup(c, &format!("setting-{}", slug(&self.path)), self.label)
    }

    /// The help in the catalog's language (`setting-*-help`).
    pub fn help_in(&self, c: &Catalog) -> String {
        if self.help.is_empty() {
            return String::new();
        }
        lookup(c, &format!("setting-{}-help", slug(&self.path)), self.help)
    }

    /// The section title in the catalog's language (`section-*`).
    pub fn section_in(&self, c: &Catalog) -> String {
        let top = self.path.split('.').next().unwrap_or_default();
        lookup(c, &format!("section-{}", slug(top)), self.section)
    }

    /// A choice's label in the catalog's language (`choice-*`).
    pub fn choice_label_in(&self, c: &Catalog, choice: &Choice) -> String {
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
                let n = number_text(n.as_f64().unwrap_or_default());
                if unit.is_empty() {
                    n
                } else {
                    let unit = lookup(c, &format!("settings-unit-{}", slug(unit)), unit);
                    c.fmt("settings-number-unit", &args!["n" => n, "unit" => unit])
                }
            }
            (SettingKind::Choice { choices, .. }, v) => choices
                .iter()
                .find(|x| &x.value == v)
                .map(|x| self.choice_label_in(c, x))
                .unwrap_or_else(|| plain(v)),
            (SettingKind::List, Value::Array(items)) if items.is_empty() => c.tr("settings-none"),
            (SettingKind::List, Value::Array(items)) => {
                items.iter().map(plain).collect::<Vec<_>>().join(", ")
            }
            (SettingKind::Table, Value::Object(m)) => {
                c.fmt("settings-entries", &args!["n" => m.len()])
            }
            (_, Value::String(s)) if s.is_empty() => c.tr("settings-empty"),
            (_, v) => plain(v),
        }
    }

    /// The value as text for a prompt: a number, the text, or the list's
    /// items joined with commas.
    pub fn edit_text(&self, value: &Value) -> String {
        match value {
            Value::Array(items) => items.iter().map(plain).collect::<Vec<_>>().join(", "),
            Value::Null => String::new(),
            v => plain(v),
        }
    }

    /// Parses text typed for this setting into a value: a number, on or
    /// off, a choice (by value or label), text, or a comma-separated list.
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
                t.split(',')
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
                let at = choices.iter().position(|c| &c.value == value);
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
}

impl App {
    /// The settings schema, with this session's theme names as the theme's
    /// choices.
    pub fn settings_schema(&self) -> SettingsSchema {
        let mut schema = SettingsSchema::generate();
        let themes: Vec<Choice> = self
            .themes
            .names()
            .into_iter()
            .map(|name| {
                let label = self.themes.resolve(name).0.meta.display_name.clone();
                Choice {
                    value: Value::String(name.to_owned()),
                    label,
                }
            })
            .collect();
        for s in &mut schema.settings {
            if s.path == "display.theme"
                && let SettingKind::Choice { choices, .. } = &mut s.kind
            {
                choices.clone_from(&themes);
            }
        }
        schema
    }

    /// The current value of the setting at `path`, as JSON.
    pub fn setting_value(&self, path: &str) -> Option<Value> {
        get(&json(&self.settings), path).cloned()
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
            self.apply_voice_settings();
            if self.settings.speech.voice != old.speech.voice {
                self.speech.set_voice(self.settings.speech.voice.clone());
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
        if self.settings.display.theme != old.display.theme {
            self.settings.display.theme_explicit = true;
            self.check_theme_name();
        }
        if top == "highlight" || self.settings.display.theme != old.display.theme {
            self.check_reading_colors();
        }
        if self.settings.interface.language != old.interface.language {
            self.apply_interface_language();
        }
        if self.settings.library.folders != old.library.folders
            || self.settings.reading.sync_conflict_policy != old.reading.sync_conflict_policy
        {
            self.library_sync = Self::make_library_sync(&self.settings);
        }
    }

    /// [`Command::SetSetting`](crate::Command::SetSetting).
    pub(crate) fn set_setting_command(&mut self, path: &str, value: Value) -> Vec<Effect> {
        match self.set_setting(path, value) {
            Ok(said) => self.tell(&said),
            Err(why) => self.error(&why),
        }
        self.refresh_settings_screen()
    }

    /// The Settings command: the settings screen.
    pub(crate) fn open_settings_screen(&mut self) -> Vec<Effect> {
        let schema = self.settings_schema();
        let shown: Vec<usize> = (0..schema.settings.len())
            .filter(|&i| !schema.settings[i].internal)
            .collect();
        let n = shown.len();
        self.settings_screen = Some(SettingsScreen {
            schema,
            shown,
            filter: String::new(),
            editing: None,
        });
        let msg = self.msg_args("settings-intro", &args!["n" => n]);
        self.tell(&msg);
        self.show_settings_list()
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
            .map(|&i| {
                let s = &screen.schema.settings[i];
                let v = get(&tree, &s.path).cloned().unwrap_or(Value::Null);
                c.fmt(
                    "settings-item",
                    &args!["label" => s.label_in(c), "value" => s.describe_in(c, &v)],
                )
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
        self.note(&msg);
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
        screen.shown = (0..screen.schema.settings.len())
            .filter(|&i| {
                let s = &screen.schema.settings[i];
                let unit = match &s.kind {
                    SettingKind::Number { unit, .. } => unit,
                    _ => "",
                };
                // English and the interface's language both match.
                !s.internal
                    && crate::lists::matches(
                        &format!(
                            "{} {} {} {} {unit} {} {} {}",
                            s.label,
                            s.section,
                            s.path,
                            s.help,
                            s.label_in(&c),
                            s.section_in(&c),
                            s.help_in(&c)
                        ),
                        &query,
                    )
            })
            .collect();
        screen.filter = query.clone();
        let n = screen.shown.len();
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
            ListKey::Delete => {
                let (_, s) = self.shown_setting(n)?;
                Some(self.set_setting_command(&s.path, Value::Null))
            }
            _ => None,
        }
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
                        "help" => s.help_in(&c)
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
        | "speech.apple.backend" => Some("settings-restart-speech"),
        "keyboard.preset" | "keyboard.digit_row" => Some("settings-next-start"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
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
        assert_eq!(folders.parse("a, b ,"), Ok(serde_json::json!(["a", "b"])));
        let voice = schema.get("speech.voice").unwrap();
        assert_eq!(voice.parse(" "), Ok(Value::Null));
        assert_eq!(voice.describe(&Value::Null), "not set");
        assert!(schema.get("display.theme_explicit").unwrap().internal);
        assert_eq!(fallback_label("display.tab_width"), "Tab width");
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
}
