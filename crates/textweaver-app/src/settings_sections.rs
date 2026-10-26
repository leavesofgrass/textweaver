//! Where each setting is listed, and when it applies (beta 4, the
//! settings rework).
//!
//! # Sections
//!
//! Settings are listed in short sections named for what the reader does
//! ([`SECTIONS`]): Voice and speech, Pauses and verbosity, Morse code, and
//! so on. Every setting on a settings screen is in exactly one section,
//! and the sections' order and contents are the same in the textweaver
//! app's dialog, the terminal reader's settings screen and the generated
//! settings reference. `settings.toml` keeps its own tables: the sections
//! change only how settings are presented.
//!
//! # When a setting applies
//!
//! [`applies`] says whether a setting does anything now, given the speech
//! engine in effect and the operating system ([`SettingsContext`]): the
//! voice and rate do nothing while CW (Morse code) is the engine,
//! Eloquence's options only matter while Eloquence is, and SAPI 5's
//! OneCore voices exist only on Windows. A section lists only the settings
//! that apply; a search of all settings still finds the others, and says
//! why each does not apply now ([`Condition::message_in`]).
//!
//! The sound of Morse code (its speeds, tone and level) applies whatever
//! the engine, because keying and practice use it too. It is listed in
//! Morse code, and moves to the top of Voice and speech while CW is the
//! speech engine ([`section_now`]), so the engine's own settings are
//! always where its voice would be.

use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;

/// One section of the settings screens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Section {
    /// Its id: `section-<id>` is its title's message, and JSON-RPC's
    /// `section_id`.
    pub id: &'static str,
    /// Its title in English, such as "Voice and speech".
    pub title: &'static str,
    /// Its settings, in order, by path.
    pub paths: &'static [&'static str],
}

/// The section of the settings textweaver keeps for itself (a question
/// already asked): in the schema, never on a settings screen.
pub const INTERNAL_SECTION: &str = "internal";

/// The sound of Morse code: listed in Morse code, and at the top of Voice
/// and speech instead while CW is the speech engine.
pub const CW_SOUND: &[&str] = &[
    "speech.cw.wpm",
    "speech.cw.effective_wpm",
    "speech.cw.tone_hz",
    "speech.cw.volume",
];

/// Every section, in the order the settings screens list them, with every
/// setting a screen shows. A test keeps each setting in exactly one.
pub const SECTIONS: &[Section] = &[
    Section {
        id: "voice",
        title: "Voice and speech",
        paths: &[
            "speech.backend",
            "speech.voice",
            "speech.prefer_voice",
            "speech.favorite_voices",
            "speech.rate",
            "speech.pitch",
            "speech.volume",
            "speech.cw.prosigns",
            "speech.cw.status_line",
            "speech.eci.dictionaries",
            "speech.eci.library",
            "speech.eci.code_factory",
            "speech.sapi.onecore",
            "speech.apple.backend",
            "speech.dectalk.library",
            "speech.espeak.helper",
            "speech.piper.voice",
            "speech.piper.voices",
            "speech.piper.phonemizer",
            "speech.speed_presets",
            "speech.voices_by_language",
            "speech.voice_params",
            "speech.output_device",
        ],
    },
    Section {
        id: "pauses",
        title: "Pauses and verbosity",
        paths: &[
            "speech.verbosity",
            "speech.punctuation",
            "speech.caps",
            "speech.split_caps",
            "speech.pause_heading_ms",
            "speech.pause_paragraph_ms",
            "speech.pause_list_item_ms",
            "speech.markup_pauses",
        ],
    },
    Section {
        id: "speaking-text",
        title: "Speaking text",
        paths: &[
            "normalization.numbers",
            "speech.skip_code",
            "normalization.table_mode",
            "normalization.footnote_mode",
            "normalization.abbreviations",
            "normalization.abbrev_expansions",
            "normalization.use_pronunciations",
            "normalization.pronunciations",
            "normalization.community_lexicon.enabled",
            "normalization.community_lexicon.language",
            "normalization.community_lexicon.dir",
            "normalization.medical_lexicon.enabled",
            "normalization.medical_lexicon.overlay",
        ],
    },
    Section {
        id: "accessibility",
        title: "Accessibility and announcements",
        paths: &[
            "accessibility.mode",
            "accessibility.interface_announcements",
            "accessibility.say_all",
            "accessibility.quiet_screen",
            "accessibility.cursor",
            "gui.speak_messages",
            "gui.announce",
        ],
    },
    Section {
        id: "reading",
        title: "Reading",
        paths: &[
            "speech.auto_play",
            "reading.auto_resume",
            "reading.cursor_follows_speech",
            "reading.wrap_navigation",
            "reading.nav_history_size",
            "reading.stop_at",
            "reading.stop_after_minutes",
            "reading.after_end",
            "reading.recall_prompts",
        ],
    },
    Section {
        id: "documents",
        title: "Documents",
        paths: &[
            "reading.citations",
            "reading.revisions",
            "reading.read_comments",
            "reading.book_audio",
            "reading.ocr",
            "reading.ocr_lang",
            "reading.ocr_engine",
        ],
    },
    Section {
        id: "highlight",
        title: "Highlight",
        paths: &[
            "highlight.enabled",
            "highlight.granularity",
            "highlight.lead_words",
            "highlight.speed",
            "speech.latency_offset_ms",
            "highlight.color",
            "highlight.sentence_color",
            "highlight.palette",
        ],
    },
    Section {
        id: "math",
        title: "Math",
        paths: &[
            "normalization.math",
            "normalization.math_verbosity",
            "reading.math_engine",
            "reading.math_display",
            "normalization.asciimath_delimiter",
        ],
    },
    Section {
        id: "braille",
        title: "Braille",
        paths: &[
            "braille.grade",
            "braille.display_cells",
            "braille.brf_code",
            "braille.table_format",
            "braille.math_code",
            "braille.viewer",
            "braille.viewer_print_line",
            "braille.viewer_cell_outlines",
        ],
    },
    Section {
        id: "morse",
        title: "Morse code",
        paths: &[
            "speech.cw.wpm",
            "speech.cw.effective_wpm",
            "speech.cw.tone_hz",
            "speech.cw.volume",
            "speech.cw.keyer",
            "speech.cw.swap_paddles",
            "speech.cw.sidetone",
            "speech.cw.sidetone_hz",
            "speech.cw.dit_note",
            "speech.cw.dah_note",
            "speech.cw.straight_note",
            "speech.cw.midi_channel",
        ],
    },
    Section {
        id: "morse-practice",
        title: "Morse practice",
        paths: &[
            "speech.cw.practice_threshold",
            "speech.cw.practice_group",
            "speech.cw.practice_groups",
            "speech.cw.practice_delay_ms",
        ],
    },
    Section {
        id: "display",
        title: "Display",
        paths: &[
            "display.theme",
            "display.follow_os_theme",
            "display.show_line_numbers",
            "display.measure",
            "display.wrap_width",
            "display.tab_width",
            "display.scroll_margin",
            "display.hints",
            "interface.rtl",
        ],
    },
    Section {
        id: "colors",
        title: "Colors",
        paths: &[
            "colors.ruler",
            "colors.difficult_words",
            "colors.syllables",
            "colors.misspellings",
            "colors.lint",
            "colors.find_match",
            "colors.selection",
            "colors.focus",
            "colors.links",
            "colors.headings",
            "colors.status_bar",
            "colors.notes",
            "colors.bookmarks",
        ],
    },
    Section {
        id: "spacing",
        title: "Spacing, font and ruler",
        paths: &[
            "reading_aids.spacing.line_height",
            "reading_aids.spacing.paragraph_spacing",
            "reading_aids.spacing.letter_spacing",
            "reading_aids.spacing.word_spacing",
            "reading_aids.font.family",
            "reading_aids.font.size_pt",
            "reading_aids.font.weight",
            "reading_aids.ruler.mode",
            "reading_aids.ruler.scope",
            "reading_aids.ruler.rows_above",
            "reading_aids.ruler.rows_below",
            "reading_aids.ruler.mask_outside",
        ],
    },
    Section {
        id: "word-aids",
        title: "Word aids",
        paths: &[
            "reading_aids.bionic",
            "reading_aids.bionic_options.ratio",
            "reading_aids.bionic_options.min_word_len",
            "reading_aids.bionic_options.skip_numbers",
            "reading_aids.bionic_options.skip_urls",
            "reading_aids.bionic_options.skip_code",
            "reading_aids.syllables",
            "reading_aids.syllable_options.separator",
            "reading_aids.syllable_options.left_min",
            "reading_aids.syllable_options.right_min",
            "reading_aids.syllable_options.min_word_len",
            "reading_aids.syllable_options.skip_urls",
            "reading_aids.syllable_options.skip_code",
            "reading_aids.difficult_words",
            "reading_aids.difficult_definitions",
        ],
    },
    Section {
        id: "rsvp",
        title: "Word by word (RSVP)",
        paths: &[
            "reading_aids.rsvp.wpm",
            "reading_aids.rsvp.pacing",
            "reading_aids.rsvp.clause_pause",
            "reading_aids.rsvp.sentence_pause",
            "reading_aids.rsvp.paragraph_pause",
            "reading_aids.rsvp.long_word_len",
            "reading_aids.rsvp.long_word_step",
            "reading_aids.rsvp.long_word_max",
            "reading_aids.rsvp.show_previous",
            "reading_aids.rsvp.show_next",
            "reading_aids.rsvp.position",
            "reading_aids.rsvp.font_size_pt",
            "reading_aids.rsvp.lead_words",
        ],
    },
    Section {
        id: "editing",
        title: "Editing",
        paths: &[
            "editing.echo_characters",
            "editing.echo_words",
            "editing.echo_deletions",
            "editing.echo_lines_on_move",
            "editing.undo_steps",
            "editing.undo_memory_mb",
            "editing.autosave_recovery",
            "editing.autosave_interval_secs",
            "authoring.author",
            "authoring.track_changes",
            "preview.pane",
            "preview.follow",
            "preview.pane_delay_ms",
        ],
    },
    Section {
        id: "library",
        title: "Library and study tools",
        paths: &[
            "library.folders",
            "library.recent_limit",
            "stats.enabled",
            "summary.sentences",
            "lexicon.glossary",
            "lexicon.data_file",
        ],
    },
    Section {
        id: "dictation",
        title: "Dictation",
        paths: &[
            "dictation.model",
            "dictation.model_dir",
            "dictation.speak_while_recording",
        ],
    },
    Section {
        id: "keyboard",
        title: "Keyboard and language",
        paths: &[
            "interface.language",
            "keyboard.preset",
            "keyboard.character_keys",
            "keyboard.digit_row",
        ],
    },
    Section {
        id: "window",
        title: "Toolbars and panels",
        paths: &[
            "gui.header",
            "gui.header_buttons",
            "gui.toolbar",
            "gui.toolbar_buttons",
            "gui.auto_hide_menu",
            "gui.focus_mode",
            "gui.sidebar",
        ],
    },
    Section {
        id: "export",
        title: "Export",
        paths: &[
            "export.audio_format",
            "export.subtitle_format",
            "export.subtitle_karaoke",
            "export.subtitle_chapters",
            "export.subtitle_word_level",
            "export.subtitles_with_audio",
        ],
    },
    Section {
        id: "live",
        title: "Live documents",
        paths: &[
            "live.sources",
            "live.announce_new_entries",
            "live.speak_new_entries",
            "live.min_interval_seconds",
            "live.quiet_hint_seconds",
            "live.log_folder",
        ],
    },
    Section {
        id: "sync",
        title: "Sync",
        paths: &[
            "sync.enabled",
            "sync.folder",
            "sync.device_name",
            "sync.position_policy",
            "sync.places",
            "sync.notes",
            "sync.highlights",
            "sync.bookmarks",
            "sync.statistics",
            "sync.settings",
            "sync.profiles",
            "sync.key_overrides",
            "sync.words",
            "sync.glossary",
            "sync.favorite_voices",
        ],
    },
    Section {
        id: "updates",
        title: "Updates and components",
        paths: &["updates.check", "components.source", "components.mirror"],
    },
];

/// The section a setting is listed in by default (its home in the
/// settings reference), and its place in it: `None` for one in no section
/// (the internal ones).
pub fn home_of(path: &str) -> Option<(usize, usize)> {
    SECTIONS
        .iter()
        .enumerate()
        .find_map(|(i, s)| s.paths.iter().position(|p| *p == path).map(|row| (i, row)))
}

/// The section with id `id`.
pub fn section(id: &str) -> Option<&'static Section> {
    SECTIONS.iter().find(|s| s.id == id)
}

/// The operating systems settings can be limited to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    /// Windows.
    Windows,
    /// macOS.
    MacOs,
    /// Linux and the other Unix systems.
    Other,
}

impl Os {
    /// The system textweaver runs on.
    pub fn current() -> Self {
        if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Other
        }
    }

    /// Its name, the same in every language.
    pub fn name(self) -> &'static str {
        match self {
            Os::Windows => "Windows",
            Os::MacOs => "macOS",
            Os::Other => "Linux",
        }
    }
}

/// What a setting's applying depends on: the speech engine in effect and
/// the operating system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsContext {
    /// The speech engine's id (`eci`, `cw`, `sapi`): the one chosen, or,
    /// when the choice is automatic, the one running. `None` when not
    /// known, and every engine's settings then apply.
    pub engine: Option<String>,
    /// The system; `None` when every system's settings apply (the
    /// settings reference).
    pub os: Option<Os>,
}

impl SettingsContext {
    /// Everything applies: no engine or system is known.
    pub fn any() -> Self {
        SettingsContext {
            engine: None,
            os: None,
        }
    }

    /// The engine `engine` on this system.
    pub fn with_engine(engine: &str) -> Self {
        SettingsContext {
            engine: Some(engine.to_owned()),
            os: Some(Os::current()),
        }
    }

    /// The engine `engine` on system `os`.
    pub fn on(engine: &str, os: Os) -> Self {
        SettingsContext {
            engine: Some(engine.to_owned()),
            os: Some(os),
        }
    }

    /// True when CW is the speech engine.
    pub fn cw(&self) -> bool {
        self.engine.as_deref() == Some(CW)
    }
}

/// CW's engine id.
const CW: &str = "cw";

/// A condition a setting needs to do anything; unmet, it is why the
/// setting does not apply now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    /// One of these engines is the speech engine.
    Engines(&'static [&'static str]),
    /// This engine is not the speech engine.
    NotEngine(&'static str),
    /// textweaver runs on this system.
    Os(Os),
}

impl Condition {
    /// True when the condition is met in `cx`. An engine or system not
    /// known meets it.
    pub fn holds(&self, cx: &SettingsContext) -> bool {
        match self {
            Condition::Engines(ids) => cx.engine.as_deref().is_none_or(|e| ids.contains(&e)),
            Condition::NotEngine(id) => cx.engine.as_deref() != Some(id),
            Condition::Os(os) => cx.os.is_none_or(|o| o == *os),
        }
    }

    /// The condition as a sentence, in the catalog's language: "Applies
    /// when Eloquence is the speech engine."
    pub fn message_in(&self, c: &Catalog) -> String {
        match self {
            Condition::Engines([one]) => c.fmt(
                "settings-applies-engine",
                &args!["engine" => engine_name(c, one)],
            ),
            Condition::Engines([first, second, ..]) => c.fmt(
                "settings-applies-engines",
                &args![
                    "first" => engine_name(c, first),
                    "second" => engine_name(c, second)
                ],
            ),
            Condition::Engines([]) => String::new(),
            Condition::NotEngine(id) => c.fmt(
                "settings-applies-not-engine",
                &args!["engine" => engine_name(c, id)],
            ),
            Condition::Os(os) => c.fmt("settings-applies-os", &args!["os" => os.name()]),
        }
    }
}

/// An engine's name as the Speech engine setting offers it ("Eloquence").
fn engine_name(c: &Catalog, id: &str) -> String {
    crate::settings_schema::base_schema()
        .get("speech.backend")
        .and_then(|s| match &s.kind {
            crate::SettingKind::Choice { choices, .. } => choices
                .iter()
                .find(|x| x.value.as_str() == Some(id))
                .map(|x| s.choice_label_in(c, x)),
            _ => None,
        })
        .unwrap_or_else(|| id.to_owned())
}

/// The settings for a speaking voice: nothing while CW is the engine.
const VOICE_ONLY: &[&str] = &[
    "speech.voice",
    "speech.prefer_voice",
    "speech.favorite_voices",
    "speech.rate",
    "speech.pitch",
    "speech.speed_presets",
    "speech.voices_by_language",
    "speech.voice_params",
];

/// The conditions the setting at `path` needs, all of them, to do
/// anything; none for most settings. The one place this is decided, for
/// both frontends and the settings reference.
///
/// - The voice, preferred and favorite voices, rate, pitch, speed
///   presets, voices by language and rate and pitch per voice: CW is not
///   the engine.
/// - CW prosigns and CW characters on the status line: CW is the engine.
/// - Eloquence's dictionaries and library: Eloquence is the engine; its
///   Code Factory search also needs Windows.
/// - OneCore voices: Windows, and SAPI 5 is the engine.
/// - Apple speech engine: macOS, and an Apple engine is the engine.
/// - DECtalk library, eSpeak NG helper, and Piper's voice, folder and
///   phonemizer: that engine is the engine.
pub fn conditions(path: &str) -> &'static [Condition] {
    use Condition::{Engines, NotEngine, Os as On};
    if VOICE_ONLY.contains(&path) {
        return &[NotEngine(CW)];
    }
    match path {
        "speech.cw.prosigns" | "speech.cw.status_line" => &[Engines(&[CW])],
        "speech.eci.dictionaries" | "speech.eci.library" => &[Engines(&["eci"])],
        "speech.eci.code_factory" => &[On(Os::Windows), Engines(&["eci"])],
        "speech.sapi.onecore" => &[On(Os::Windows), Engines(&["sapi"])],
        "speech.apple.backend" => &[On(Os::MacOs), Engines(&["nsspeech", "avspeech"])],
        "speech.dectalk.library" => &[Engines(&["dectalk"])],
        "speech.espeak.helper" => &[Engines(&["espeak"])],
        "speech.piper.voice" | "speech.piper.voices" | "speech.piper.phonemizer" => {
            &[Engines(&["piper"])]
        }
        _ => &[],
    }
}

/// Whether the setting at `path` applies in `cx`: `Ok` when it does, or
/// the first of its [`conditions`] not met.
pub fn applies(path: &str, cx: &SettingsContext) -> Result<(), Condition> {
    match conditions(path).iter().find(|c| !c.holds(cx)) {
        Some(c) => Err(*c),
        None => Ok(()),
    }
}

/// The id of the section the setting at `path` is listed in now: its home
/// section, except the sound of Morse code, at the top of Voice and speech
/// while CW is the engine. [`INTERNAL_SECTION`] for one in no section.
pub fn section_now(path: &str, cx: &SettingsContext) -> &'static str {
    if cx.cw() && CW_SOUND.contains(&path) {
        return "voice";
    }
    home_of(path).map_or(INTERNAL_SECTION, |(i, _)| SECTIONS[i].id)
}

/// The paths section `id` lists in `cx`, in order, before the applying
/// test: its own, with the sound of Morse code moved to the top of Voice
/// and speech (after the engine) while CW is the engine.
pub fn paths_now(id: &str, cx: &SettingsContext) -> Vec<&'static str> {
    let Some(s) = section(id) else {
        return Vec::new();
    };
    let mut paths: Vec<&'static str> = s
        .paths
        .iter()
        .copied()
        .filter(|p| section_now(p, cx) == id)
        .collect();
    if id == "voice" && cx.cw() {
        let at = paths.len().min(1);
        paths.splice(at..at, CW_SOUND.iter().copied());
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_setting_is_in_two_sections() {
        let mut seen = std::collections::HashSet::new();
        for s in SECTIONS {
            for p in s.paths {
                assert!(seen.insert(*p), "{p} is in two sections");
            }
        }
        let mut ids = std::collections::HashSet::new();
        for s in SECTIONS {
            assert!(ids.insert(s.id), "two sections are {}", s.id);
            assert_ne!(s.id, INTERNAL_SECTION);
        }
    }

    #[test]
    fn cw_moves_the_sound_of_morse_to_voice_and_speech() {
        let cw = SettingsContext::on("cw", Os::Windows);
        let eci = SettingsContext::on("eci", Os::Windows);
        let voice = paths_now("voice", &cw);
        assert_eq!(voice[0], "speech.backend");
        assert_eq!(&voice[1..5], CW_SOUND);
        assert!(!paths_now("morse", &cw).contains(&"speech.cw.wpm"));
        assert!(paths_now("morse", &eci).contains(&"speech.cw.wpm"));
        assert!(!paths_now("voice", &eci).contains(&"speech.cw.wpm"));
        assert_eq!(section_now("speech.cw.tone_hz", &cw), "voice");
        assert_eq!(section_now("speech.cw.tone_hz", &eci), "morse");
        assert_eq!(section_now("gui.window", &eci), INTERNAL_SECTION);
    }

    #[test]
    fn engine_settings_apply_only_with_their_engine() {
        let cw = SettingsContext::on("cw", Os::Windows);
        let eci = SettingsContext::on("eci", Os::Windows);
        let sapi = SettingsContext::on("sapi", Os::Windows);
        for p in VOICE_ONLY {
            assert_eq!(applies(p, &cw), Err(Condition::NotEngine("cw")), "{p}");
            assert_eq!(applies(p, &eci), Ok(()), "{p}");
        }
        assert_eq!(applies("speech.cw.prosigns", &cw), Ok(()));
        assert!(applies("speech.cw.prosigns", &eci).is_err());
        assert!(applies("speech.cw.status_line", &sapi).is_err());
        for p in CW_SOUND {
            assert_eq!(applies(p, &eci), Ok(()), "{p} keys and practice");
        }
        assert_eq!(applies("speech.eci.library", &eci), Ok(()));
        assert_eq!(
            applies("speech.eci.library", &cw),
            Err(Condition::Engines(&["eci"]))
        );
        assert_eq!(applies("speech.sapi.onecore", &sapi), Ok(()));
        assert!(applies("speech.sapi.onecore", &eci).is_err());
        for (path, engine) in [
            ("speech.dectalk.library", "dectalk"),
            ("speech.espeak.helper", "espeak"),
            ("speech.piper.voice", "piper"),
            ("speech.piper.voices", "piper"),
            ("speech.piper.phonemizer", "piper"),
        ] {
            assert_eq!(
                applies(path, &SettingsContext::on(engine, Os::Other)),
                Ok(())
            );
            assert!(applies(path, &eci).is_err(), "{path}");
        }
        assert_eq!(applies("speech.rate", &SettingsContext::any()), Ok(()));
        assert_eq!(
            applies("speech.eci.library", &SettingsContext::any()),
            Ok(())
        );
        assert_eq!(applies("speech.volume", &cw), Ok(()));
        assert_eq!(applies("display.theme", &cw), Ok(()));
    }

    #[test]
    fn platform_settings_apply_only_on_their_system() {
        let sapi_mac = SettingsContext::on("sapi", Os::MacOs);
        assert_eq!(
            applies("speech.sapi.onecore", &sapi_mac),
            Err(Condition::Os(Os::Windows))
        );
        let eci_linux = SettingsContext::on("eci", Os::Other);
        assert_eq!(
            applies("speech.eci.code_factory", &eci_linux),
            Err(Condition::Os(Os::Windows))
        );
        assert_eq!(applies("speech.eci.library", &eci_linux), Ok(()));
        let apple = SettingsContext::on("avspeech", Os::MacOs);
        assert_eq!(applies("speech.apple.backend", &apple), Ok(()));
        let apple_win = SettingsContext::on("avspeech", Os::Windows);
        assert_eq!(
            applies("speech.apple.backend", &apple_win),
            Err(Condition::Os(Os::MacOs))
        );
        assert!(
            applies(
                "speech.apple.backend",
                &SettingsContext::on("eci", Os::MacOs)
            )
            .is_err()
        );
    }

    #[test]
    fn the_reason_reads_as_a_sentence() {
        let c = Catalog::english();
        assert_eq!(
            Condition::Engines(&["eci"]).message_in(&c),
            "Applies when Eloquence is the speech engine."
        );
        assert_eq!(
            Condition::NotEngine("cw").message_in(&c),
            "Applies when CW (Morse code) is not the speech engine."
        );
        assert_eq!(
            Condition::Os(Os::Windows).message_in(&c),
            "Applies on Windows only."
        );
        assert_eq!(
            Condition::Engines(&["nsspeech", "avspeech"]).message_in(&c),
            "Applies when Apple NSSpeech or Apple AVSpeech is the speech engine."
        );
    }
}
