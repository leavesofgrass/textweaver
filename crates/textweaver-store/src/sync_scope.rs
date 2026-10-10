//! Which settings travel between computers (ADR-0049).
//!
//! Every setting is marked **portable** or **machine** in
//! [`SETTING_SCOPES`]:
//!
//! - **Portable** settings are about the reader, not the computer: the
//!   rate, punctuation, verbosity, capitals, the reading aids, the
//!   highlight, the theme, the Braille and math codes, the interface
//!   language, speed presets, the announcement level, and the like. With
//!   `[sync] settings` on they sync, the newest change winning key by key.
//! - **Machine** settings differ from computer to computer and never sync:
//!   the speech engine and voice, the access mode, the NVDA or JAWS key
//!   preset, every path (library folders, the glossary, engine libraries,
//!   the sync folder itself), window and terminal layout, and what
//!   textweaver keeps for itself on one computer.
//!
//! A test fails when a setting has neither mark, or a mark names a setting
//! that no longer exists, so a new setting cannot slip into sync (or out of
//! it) unnoticed.
//!
//! Two portable settings travel in groups of their own rather than with
//! the settings ([`OWN_GROUP_SETTINGS`]): favorite voices (a set, with its
//! own switch) and pronunciations (newest wins per word, with the
//! glossary).
//!
//! A map such as `speech.speed_presets` is one setting: it syncs whole, as
//! it is stored whole in `settings.toml`.
//!
//! Profiles sync too, but a profile holds machine settings as well (the
//! voice, the engine, the access mode it was saved with). Those stay on
//! the computer that saved them: [`profile_portable`] is what a profile
//! publishes, and [`profile_with_machine`] keeps this computer's machine
//! values when another computer's version of a profile arrives.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::Settings;
use crate::settings::STRUCT_TABLES;
use crate::settings_io::settings_to_json;

/// Whether a setting syncs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingScope {
    /// The same on every computer: syncs.
    Portable,
    /// This computer's own: never syncs.
    Machine,
}

use SettingScope::{Machine, Portable};

/// Every setting's mark, by its dotted path in `settings.toml`.
pub const SETTING_SCOPES: &[(&str, SettingScope)] = &[
    // [speech]: the engine, the voice, and what depends on the audio
    // hardware stay; how speech sounds and what it says travel.
    ("speech.backend", Machine),
    ("speech.rate", Portable),
    // Loudness depends on the computer's speakers and mixer.
    ("speech.volume", Machine),
    // Semitones from the voice's own pitch, whatever the voice.
    ("speech.pitch", Portable),
    ("speech.voice", Machine),
    ("speech.prefer_voice", Machine),
    ("speech.favorite_voices", Portable),
    ("speech.punctuation", Portable),
    ("speech.split_caps", Portable),
    ("speech.caps", Portable),
    ("speech.auto_play", Portable),
    ("speech.skip_code", Portable),
    ("speech.speed_presets", Portable),
    // Voice ids, which differ by computer.
    ("speech.voices_by_language", Machine),
    // The audio device's latency.
    ("speech.latency_offset_ms", Machine),
    // How long speech pauses at structure: how the reader listens.
    ("speech.pause_heading_ms", Portable),
    ("speech.pause_paragraph_ms", Portable),
    ("speech.pause_list_item_ms", Portable),
    // Whether written pause markup is read as a pause: how the reader
    // listens.
    ("speech.markup_pauses", Portable),
    // A device id of this computer's audio system.
    ("speech.output_device", Machine),
    ("speech.verbosity", Portable),
    ("speech.eci.dictionaries", Machine),
    ("speech.eci.library", Machine),
    ("speech.eci.code_factory", Machine),
    ("speech.sapi.onecore", Machine),
    ("speech.apple.backend", Machine),
    ("speech.dectalk.library", Machine),
    ("speech.espeak.helper", Machine),
    ("speech.piper.voices", Machine),
    ("speech.piper.voice", Machine),
    ("speech.piper.phonemizer", Machine),
    ("speech.voice_params", Machine),
    // [highlight]
    ("highlight.enabled", Portable),
    ("highlight.granularity", Portable),
    ("highlight.lead_words", Portable),
    ("highlight.speed", Portable),
    ("highlight.color", Portable),
    ("highlight.sentence_color", Portable),
    ("highlight.palette", Portable),
    // [normalization]
    ("normalization.math", Portable),
    ("normalization.math_verbosity", Portable),
    ("normalization.asciimath_delimiter", Portable),
    ("normalization.abbreviations", Portable),
    ("normalization.abbrev_expansions", Portable),
    ("normalization.numbers", Portable),
    ("normalization.use_pronunciations", Portable),
    ("normalization.pronunciations", Portable),
    ("normalization.table_mode", Portable),
    ("normalization.footnote_mode", Portable),
    ("normalization.community_lexicon.enabled", Portable),
    ("normalization.community_lexicon.dir", Machine),
    ("normalization.community_lexicon.language", Portable),
    ("normalization.medical_lexicon.enabled", Portable),
    ("normalization.medical_lexicon.overlay", Machine),
    // [reading]
    ("reading.auto_resume", Portable),
    ("reading.nav_history_size", Portable),
    ("reading.wrap_navigation", Portable),
    ("reading.cursor_follows_speech", Portable),
    ("reading.citations", Portable),
    ("reading.ocr", Portable),
    ("reading.ocr_lang", Portable),
    // Which engines are installed differs by computer.
    ("reading.ocr_engine", Machine),
    ("reading.math_engine", Portable),
    ("reading.math_display", Portable),
    ("reading.revisions", Portable),
    ("reading.stop_at", Portable),
    ("reading.stop_after_minutes", Portable),
    ("reading.recall_prompts", Portable),
    ("reading.book_audio", Portable),
    // [display]
    ("display.theme", Portable),
    ("display.follow_os_theme", Portable),
    ("display.theme_explicit", Portable),
    // The terminal's or window's width.
    ("display.wrap_width", Machine),
    // A reading preference, like the font size.
    ("display.measure", Portable),
    ("display.tab_width", Portable),
    ("display.show_line_numbers", Portable),
    ("display.scroll_margin", Portable),
    ("display.hints", Machine),
    // [editing]
    ("editing.autosave_recovery", Portable),
    ("editing.autosave_interval_secs", Portable),
    ("editing.echo_characters", Portable),
    ("editing.echo_words", Portable),
    ("editing.echo_deletions", Portable),
    ("editing.echo_lines_on_move", Portable),
    // How much memory the computer has.
    ("editing.undo_steps", Machine),
    ("editing.undo_memory_mb", Machine),
    // [authoring]: the name is a person's, never written to the sync
    // folder; tracking changes is a preference that travels.
    ("authoring.author", Machine),
    ("authoring.track_changes", Portable),
    // [library]
    ("library.recent_limit", Portable),
    ("library.folders", Machine),
    // [keyboard]
    ("keyboard.character_keys", Portable),
    ("keyboard.preset", Machine),
    // The keyboard's layout.
    ("keyboard.digit_row", Machine),
    // [accessibility]: the access mode and what goes with the screen
    // reader on this computer stay; the announcement level travels.
    ("accessibility.mode", Machine),
    ("accessibility.say_all", Machine),
    ("accessibility.quiet_screen", Machine),
    ("accessibility.cursor", Machine),
    ("accessibility.hybrid_offered", Machine),
    ("accessibility.interface_announcements", Portable),
    // [export]
    ("export.audio_format", Portable),
    ("export.subtitle_format", Portable),
    ("export.subtitle_word_level", Portable),
    ("export.subtitles_with_audio", Portable),
    ("export.subtitle_karaoke", Portable),
    ("export.subtitle_chapters", Portable),
    // [braille]
    ("braille.math_code", Portable),
    ("braille.table_format", Portable),
    ("braille.brf_code", Portable),
    // [reading_aids]
    ("reading_aids.rsvp.wpm", Portable),
    ("reading_aids.rsvp.pacing", Portable),
    ("reading_aids.rsvp.clause_pause", Portable),
    ("reading_aids.rsvp.sentence_pause", Portable),
    ("reading_aids.rsvp.paragraph_pause", Portable),
    ("reading_aids.rsvp.long_word_len", Portable),
    ("reading_aids.rsvp.long_word_step", Portable),
    ("reading_aids.rsvp.long_word_max", Portable),
    ("reading_aids.rsvp.show_previous", Portable),
    ("reading_aids.rsvp.show_next", Portable),
    ("reading_aids.rsvp.position", Portable),
    ("reading_aids.rsvp.font_size_pt", Portable),
    ("reading_aids.rsvp.lead_words", Portable),
    ("reading_aids.bionic", Portable),
    ("reading_aids.bionic_options.ratio", Portable),
    ("reading_aids.bionic_options.min_word_len", Portable),
    ("reading_aids.bionic_options.skip_numbers", Portable),
    ("reading_aids.bionic_options.skip_urls", Portable),
    ("reading_aids.bionic_options.skip_code", Portable),
    ("reading_aids.spacing.line_height", Portable),
    ("reading_aids.spacing.paragraph_spacing", Portable),
    ("reading_aids.spacing.letter_spacing", Portable),
    ("reading_aids.spacing.word_spacing", Portable),
    // A font missing on a computer is named; Lexend is downloaded there
    // only when the reader chooses it and agrees.
    ("reading_aids.font.family", Portable),
    ("reading_aids.font.size_pt", Portable),
    ("reading_aids.font.weight", Portable),
    ("reading_aids.ruler.mode", Portable),
    ("reading_aids.ruler.scope", Portable),
    ("reading_aids.ruler.rows_above", Portable),
    ("reading_aids.ruler.rows_below", Portable),
    ("reading_aids.ruler.mask_outside", Portable),
    ("reading_aids.syllables", Portable),
    ("reading_aids.difficult_words", Portable),
    ("reading_aids.syllable_options.separator", Portable),
    ("reading_aids.syllable_options.left_min", Portable),
    ("reading_aids.syllable_options.right_min", Portable),
    ("reading_aids.syllable_options.min_word_len", Portable),
    ("reading_aids.syllable_options.skip_urls", Portable),
    ("reading_aids.syllable_options.skip_code", Portable),
    ("reading_aids.difficult_definitions", Portable),
    // [preview]
    ("preview.follow", Portable),
    ("preview.pane", Portable),
    ("preview.pane_delay_ms", Portable),
    // [lexicon]: paths. The glossary's entries sync in the glossary group.
    ("lexicon.glossary", Machine),
    ("lexicon.data_file", Machine),
    // [stats]
    ("stats.enabled", Portable),
    // [summary]
    ("summary.sentences", Portable),
    // [dictation]
    ("dictation.speak_while_recording", Portable),
    ("dictation.model_dir", Machine),
    // What is downloaded on this computer.
    ("dictation.model", Machine),
    // [components] (W8a-d): this computer's mirror and first run.
    ("components.source", Machine),
    ("components.mirror", Machine),
    ("components.chooser_shown", Machine),
    // [interface]
    ("interface.language", Portable),
    // Depends on the terminal.
    ("interface.rtl", Machine),
    // The settings screen's own history on this computer.
    ("interface.recent_settings", Machine),
    // [gui]: how this computer's screen reader hears the window.
    ("gui.announce", Machine),
    // Whether this computer's window hides its menu bar.
    ("gui.auto_hide_menu", Machine),
    // Whether this computer's window voices its messages: it depends on
    // whether a screen reader runs here.
    ("gui.speak_messages", Machine),
    // Whether this computer's window shows its header and toolbar: a
    // small screen hides them, a large one keeps them.
    ("gui.header", Machine),
    ("gui.toolbar", Machine),
    // The buttons this computer's window shows on its bars, like the
    // other window layout settings.
    ("gui.header_buttons", Machine),
    ("gui.toolbar_buttons", Machine),
    // The panel this computer's window shows beside the document.
    ("gui.sidebar", Machine),
    // Where this computer's window was and how big: screens differ.
    ("gui.window", Machine),
    // The version this computer's window last ran, for its "Updated to"
    // notice.
    ("gui.last_version", Machine),
    // [colors]
    ("colors.ruler", Portable),
    ("colors.difficult_words", Portable),
    ("colors.syllables", Portable),
    ("colors.misspellings", Portable),
    ("colors.lint", Portable),
    ("colors.find_match", Portable),
    ("colors.selection", Portable),
    ("colors.focus", Portable),
    ("colors.links", Portable),
    ("colors.headings", Portable),
    ("colors.status_bar", Portable),
    ("colors.notes", Portable),
    ("colors.bookmarks", Portable),
    // [sync]: each computer sets up its own sync.
    ("sync.enabled", Machine),
    ("sync.folder", Machine),
    ("sync.device_name", Machine),
    ("sync.places", Machine),
    ("sync.notes", Machine),
    ("sync.highlights", Machine),
    ("sync.bookmarks", Machine),
    ("sync.statistics", Machine),
    ("sync.settings", Machine),
    ("sync.profiles", Machine),
    ("sync.key_overrides", Machine),
    ("sync.words", Machine),
    ("sync.glossary", Machine),
    ("sync.favorite_voices", Machine),
    ("sync.position_policy", Machine),
];

/// Portable settings that travel in a group of their own, not with the
/// settings: the setting, then the `[sync]` switch that carries it.
pub const OWN_GROUP_SETTINGS: &[(&str, &str)] = &[
    ("speech.favorite_voices", "favorite_voices"),
    ("normalization.pronunciations", "glossary"),
];

/// The mark of the setting at `path`, if it has one. Unknown keys (from a
/// newer textweaver, or typed by hand) have none and never sync.
pub fn setting_scope(path: &str) -> Option<SettingScope> {
    SETTING_SCOPES
        .iter()
        .find(|(p, _)| *p == path)
        .map(|(_, s)| *s)
}

/// The `[sync]` switch that carries the setting at `path`: `settings` for
/// a portable setting, another group's for [`OWN_GROUP_SETTINGS`], and
/// `None` for a machine setting.
pub fn sync_group_of(path: &str) -> Option<&'static str> {
    if let Some((_, g)) = OWN_GROUP_SETTINGS.iter().find(|(p, _)| *p == path) {
        return Some(g);
    }
    (setting_scope(path) == Some(Portable)).then_some("settings")
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_owned()
    } else {
        format!("{path}.{key}")
    }
}

/// Every setting of `settings` as a dotted path and its JSON value (unset
/// ones as `null`), sorted section by section, unknown keys included. A table of settings is walked into; a map (`speed_presets`)
/// is one setting.
pub fn setting_values(settings: &Settings) -> Vec<(String, Value)> {
    fn walk(m: Map<String, Value>, path: &str, out: &mut Vec<(String, Value)>) {
        for (k, v) in m {
            let sub = join(path, &k);
            match v {
                Value::Object(inner)
                    if path.is_empty() || STRUCT_TABLES.contains(&sub.as_str()) =>
                {
                    walk(inner, &sub, out);
                }
                other => out.push((sub, other)),
            }
        }
    }
    let mut out = Vec::new();
    if let Ok(m) = settings_to_json(settings) {
        walk(m, "", &mut out);
    }
    out
}

/// The settings group's values: every portable setting of `settings`
/// except [`OWN_GROUP_SETTINGS`], by dotted path.
pub fn portable_settings(settings: &Settings) -> BTreeMap<String, Value> {
    setting_values(settings)
        .into_iter()
        .filter(|(p, _)| sync_group_of(p) == Some("settings"))
        .collect()
}

/// `settings` with the values in `values` (dotted paths from
/// [`portable_settings`]) put in, clamped to their ranges. Paths that are
/// not portable here (a machine setting, or one this version does not
/// know) are left alone, so another computer can never change a machine
/// setting. Returns the new settings and the paths whose value changed.
pub fn with_portable(
    settings: &Settings,
    values: &BTreeMap<String, Value>,
) -> (Settings, Vec<String>) {
    let Ok(mut tree) = settings_to_json(settings) else {
        return (settings.clone(), Vec::new());
    };
    let mut touched = Vec::new();
    for (path, value) in values {
        if sync_group_of(path).is_none() {
            continue;
        }
        if set_path(&mut tree, path, value.clone()) {
            touched.push(path.clone());
        }
    }
    if touched.is_empty() {
        return (settings.clone(), Vec::new());
    }
    let Ok(mut new) = serde_json::from_value::<Settings>(Value::Object(tree)) else {
        // One value did not fit: put them in one at a time, keeping those
        // that do.
        let mut s = settings.clone();
        for p in &touched {
            let (Some(v), Ok(mut t)) = (values.get(p), settings_to_json(&s)) else {
                continue;
            };
            set_path(&mut t, p, v.clone());
            if let Ok(n) = serde_json::from_value::<Settings>(Value::Object(t)) {
                s = n;
            }
        }
        s.validate();
        let changed = changed_paths(settings, &s);
        return (s, changed);
    };
    new.validate();
    let changed = changed_paths(settings, &new);
    (new, changed)
}

/// The dotted paths whose values differ between `a` and `b`.
pub fn changed_paths(a: &Settings, b: &Settings) -> Vec<String> {
    let before: BTreeMap<String, Value> = setting_values(a).into_iter().collect();
    setting_values(b)
        .into_iter()
        .filter(|(p, v)| before.get(p) != Some(v))
        .map(|(p, _)| p)
        .collect()
}

/// Splits a settings-shaped JSON table into its portable and machine
/// parts, by each value's dotted path. A table that is not itself a
/// setting is walked into; a value with no mark (from a newer textweaver,
/// or typed by hand) counts as portable, so it passes through.
fn split_table(tree: &Map<String, Value>, path: &str) -> (Map<String, Value>, Map<String, Value>) {
    let (mut portable, mut machine) = (Map::new(), Map::new());
    for (k, v) in tree {
        let sub = join(path, k);
        match (setting_scope(&sub), v) {
            (Some(Machine), _) => {
                machine.insert(k.clone(), v.clone());
            }
            (None, Value::Object(inner)) => {
                let (p, m) = split_table(inner, &sub);
                if !p.is_empty() {
                    portable.insert(k.clone(), Value::Object(p));
                }
                if !m.is_empty() {
                    machine.insert(k.clone(), Value::Object(m));
                }
            }
            _ => {
                portable.insert(k.clone(), v.clone());
            }
        }
    }
    (portable, machine)
}

/// Merges `from` into `into`, table by table; `from` wins a clash.
fn merge_tables(into: &mut Map<String, Value>, from: Map<String, Value>) {
    for (k, v) in from {
        match (into.get_mut(&k), v) {
            (Some(Value::Object(a)), Value::Object(b)) => merge_tables(a, b),
            (_, v) => {
                into.insert(k, v);
            }
        }
    }
}

/// A profile (a table in `settings.toml`'s shape, as JSON) as it syncs:
/// without its machine settings (the voice, the engine, the access mode,
/// and every other setting marked machine). A value that is not a table
/// is returned as it is.
pub fn profile_portable(profile: &Value) -> Value {
    match profile {
        Value::Object(t) => Value::Object(split_table(t, "").0),
        other => other.clone(),
    }
}

/// A profile arriving from another computer (`incoming`), with the machine
/// settings of this computer's version of it (`local`) kept: another
/// computer never sets the voice, engine, or access mode a profile uses
/// here. Machine settings in `incoming` are dropped.
pub fn profile_with_machine(incoming: &Value, local: Option<&Value>) -> Value {
    let Value::Object(t) = incoming else {
        return incoming.clone();
    };
    let mut out = split_table(t, "").0;
    if let Some(Value::Object(l)) = local {
        merge_tables(&mut out, split_table(l, "").1);
    }
    Value::Object(out)
}

/// Sets the value at dotted `path` in `tree`. Returns false when a table
/// on the way is missing.
fn set_path(tree: &mut Map<String, Value>, path: &str, value: Value) -> bool {
    let mut parts: Vec<&str> = path.split('.').collect();
    let Some(last) = parts.pop() else {
        return false;
    };
    let mut cur = tree;
    for p in parts {
        match cur.get_mut(p) {
            Some(Value::Object(m)) => cur = m,
            _ => return false,
        }
    }
    cur.insert(last.to_owned(), value);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use textweaver_core::Rate;

    /// Every setting is marked portable or machine. A setting added
    /// without a mark fails here, named.
    #[test]
    fn every_setting_is_marked_portable_or_machine() {
        let unmarked: Vec<String> = setting_values(&Settings::default())
            .into_iter()
            .map(|(p, _)| p)
            .filter(|p| setting_scope(p).is_none())
            .collect();
        assert!(
            unmarked.is_empty(),
            "these settings are neither portable nor machine; add them to SETTING_SCOPES: {unmarked:?}"
        );
    }

    /// The window's place and size stay on this computer, and survive a
    /// round trip through the settings file.
    #[test]
    fn the_window_place_is_a_machine_setting() {
        assert_eq!(setting_scope("gui.window"), Some(Machine));
        let mut s = Settings::default();
        s.gui.window = Some(crate::GuiWindow {
            x: -1200,
            y: 40,
            width: 900,
            height: 640,
            maximized: true,
        });
        let text = toml::to_string(&s).expect("serialize");
        let back: Settings = toml::from_str(&text).expect("parse");
        assert_eq!(back.gui.window, s.gui.window);
        assert!(!portable_settings(&s).contains_key("gui.window"));
    }

    /// Every mark names a setting that exists, once.
    #[test]
    fn every_mark_names_a_setting() {
        let paths: Vec<String> = setting_values(&Settings::default())
            .into_iter()
            .map(|(p, _)| p)
            .collect();
        let mut seen = std::collections::HashSet::new();
        for (p, _) in SETTING_SCOPES {
            assert!(
                paths.iter().any(|x| x == p),
                "{p} is marked but is not a setting"
            );
            assert!(seen.insert(*p), "{p} is marked twice");
        }
        for (p, _) in OWN_GROUP_SETTINGS {
            assert_eq!(setting_scope(p), Some(Portable), "{p}");
        }
    }

    /// The owner's list: these never sync.
    #[test]
    fn the_machine_list_holds_the_engine_voice_access_mode_key_preset_and_paths() {
        for p in [
            "speech.backend",
            "speech.voice",
            "accessibility.mode",
            "keyboard.preset",
            "library.folders",
            "lexicon.glossary",
            "lexicon.data_file",
            "speech.eci.library",
            "speech.dectalk.library",
            "speech.piper.voices",
            "dictation.model_dir",
            "normalization.community_lexicon.dir",
            "normalization.medical_lexicon.overlay",
            "sync.folder",
            "sync.device_name",
            "display.wrap_width",
            "authoring.author",
        ] {
            assert_eq!(setting_scope(p), Some(Machine), "{p}");
            assert_eq!(sync_group_of(p), None, "{p}");
        }
        // Every path-valued setting is a machine setting: none of the
        // portable defaults looks like a path.
        let mut s = Settings::default();
        s.library.folders = vec!["D:/Books".into()];
        s.lexicon.glossary = Some("D:/glossary.txt".into());
        for (p, v) in portable_settings(&s) {
            assert!(!v.to_string().contains("D:/"), "{p} carries a path");
        }
    }

    /// And these do.
    #[test]
    fn the_portable_list_holds_the_reading_preferences() {
        for p in [
            "speech.rate",
            "speech.punctuation",
            "speech.verbosity",
            "speech.caps",
            "reading_aids.bionic",
            "highlight.granularity",
            "display.theme",
            "braille.math_code",
            "interface.language",
            "speech.speed_presets",
            "accessibility.interface_announcements",
        ] {
            assert_eq!(sync_group_of(p), Some("settings"), "{p}");
        }
        assert_eq!(
            sync_group_of("speech.favorite_voices"),
            Some("favorite_voices")
        );
        assert_eq!(
            sync_group_of("normalization.pronunciations"),
            Some("glossary")
        );
        let portable = portable_settings(&Settings::default());
        assert!(!portable.contains_key("speech.favorite_voices"));
        assert!(!portable.contains_key("speech.backend"));
        assert!(portable.contains_key("speech.rate"));
    }

    /// A profile publishes only its portable settings, and an arriving
    /// version keeps this computer's voice, engine, and access mode.
    #[test]
    fn profiles_keep_their_machine_settings_on_their_own_computer() {
        let laptop = serde_json::json!({
            "speech": {"backend": "sapi", "voice": "David", "rate": 320},
            "accessibility": {"mode": "screen-reader", "interface_announcements": "brief"},
            "highlight": {"granularity": "word"},
            "future": {"knob": 1},
        });
        let published = profile_portable(&laptop);
        assert_eq!(
            published,
            serde_json::json!({
                "speech": {"rate": 320},
                "accessibility": {"interface_announcements": "brief"},
                "highlight": {"granularity": "word"},
                "future": {"knob": 1},
            })
        );
        assert_eq!(profile_portable(&published), published, "idempotent");

        let lab = serde_json::json!({
            "speech": {"backend": "espeak", "voice": "en-us", "rate": 200},
            "accessibility": {"mode": "self-voicing"},
        });
        // The laptop's version arrives, with a machine key smuggled in.
        let mut incoming = published.clone();
        incoming["speech"]["voice"] = Value::from("David");
        let merged = profile_with_machine(&incoming, Some(&lab));
        assert_eq!(merged["speech"]["rate"], 320);
        assert_eq!(merged["speech"]["backend"], "espeak");
        assert_eq!(merged["speech"]["voice"], "en-us");
        assert_eq!(merged["accessibility"]["mode"], "self-voicing");
        assert_eq!(merged["accessibility"]["interface_announcements"], "brief");
        assert_eq!(profile_portable(&merged), published);
        // A profile new here takes no machine settings at all.
        let fresh = profile_with_machine(&incoming, None);
        assert_eq!(fresh, published);
    }

    /// Portable values put into other settings change only those values,
    /// and never a machine setting.
    #[test]
    fn portable_values_apply_and_leave_machine_settings_alone() {
        let mut laptop = Settings::default();
        laptop.speech.rate = Rate::Wpm(320);
        laptop.display.theme = "galaxy-light".into();
        laptop.speech.backend = "sapi".into();
        let mut lab = Settings::default();
        lab.speech.backend = "espeak".into();
        lab.library.folders = vec!["E:/Lab".into()];
        let mut values = portable_settings(&laptop);
        // A machine setting smuggled in is ignored.
        values.insert("speech.backend".into(), Value::String("dectalk".into()));
        let (new, changed) = with_portable(&lab, &values);
        assert_eq!(new.speech.rate, Rate::Wpm(320));
        assert_eq!(new.display.theme, "galaxy-light");
        assert_eq!(new.speech.backend, "espeak");
        assert_eq!(new.library.folders, lab.library.folders);
        assert_eq!(changed, ["display.theme", "speech.rate"]);
        // Applying the same values again changes nothing.
        let (again, changed) = with_portable(&new, &values);
        assert_eq!(again, new);
        assert!(changed.is_empty());
    }

    /// A value out of range is clamped; one of the wrong type is left out
    /// and the rest still apply.
    #[test]
    fn bad_values_are_clamped_or_left_out() {
        let values: BTreeMap<String, Value> = [
            ("speech.rate".to_owned(), Value::from(5000)),
            ("speech.punctuation".to_owned(), Value::from(12)),
            ("display.tab_width".to_owned(), Value::from(8)),
        ]
        .into();
        let (new, changed) = with_portable(&Settings::default(), &values);
        assert_eq!(new.speech.rate, Rate::Wpm(Rate::MAX_WPM));
        assert_eq!(new.display.tab_width, 8);
        assert!(changed.contains(&"display.tab_width".to_owned()));
        assert!(!changed.contains(&"speech.punctuation".to_owned()));
    }
}
