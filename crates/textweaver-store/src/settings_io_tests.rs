//! Tests for settings import and export. Temporary directories only.

use std::collections::BTreeMap;

use textweaver_core::{
    CapsIndication, HighlightGranularity, Pitch, PunctuationLevel, Rate, Verbosity, Volume,
};

use super::*;
use crate::{AppleBackend, EciDictionaries, FootnoteMode, Paths, TableMode};

fn store() -> (tempfile::TempDir, SettingsStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::new(Paths::under(dir.path()));
    (dir, store)
}

fn config_files(store: &SettingsStore) -> Vec<String> {
    let mut v: Vec<String> = match std::fs::read_dir(&store.paths().config_dir) {
        Ok(d) => d
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect(),
        Err(_) => Vec::new(),
    };
    v.sort();
    v
}

/// Every setting set to a value other than its default, plus unknown keys
/// at every level and a TOML date.
fn everything_changed() -> Settings {
    let mut s = Settings::default();
    let sp = &mut s.speech;
    sp.backend = "sapi".into();
    sp.rate = Rate::Wpm(300);
    sp.volume = Volume::new(80);
    sp.pitch = Pitch::Semitones(-2);
    sp.voice = Some("David".into());
    sp.prefer_voice = None;
    sp.favorite_voices = vec!["David".into(), "Zira".into()];
    sp.punctuation = PunctuationLevel::All;
    sp.split_caps = true;
    sp.caps = CapsIndication::SayCap;
    sp.auto_play = true;
    sp.skip_code = false;
    sp.speed_presets = BTreeMap::from([("fast".to_owned(), 400)]);
    sp.latency_offset_ms = 80;
    sp.pause_heading_ms = 600;
    sp.pause_paragraph_ms = 0;
    sp.pause_list_item_ms = 250;
    sp.markup_pauses = false;
    sp.output_device = Some("wasapi:{test-device}".into());
    sp.verbosity = Verbosity::High;
    sp.eci.dictionaries = EciDictionaries::Path("C:/dicts".into());
    sp.eci.library = Some("C:/eci/eci.dll".into());
    sp.eci.code_factory = true;
    sp.eci
        .extra
        .insert("future_eci".into(), toml::Value::Boolean(true));
    sp.sapi.onecore = false;
    sp.apple.backend = AppleBackend::AvSpeech;
    sp.dectalk.library = Some("C:/dectalk/DECtalk.dll".into());
    sp.piper.voices = Some("D:/voices".into());
    sp.piper.voice = Some("en_US-amy-medium".into());
    sp.piper.phonemizer = crate::PiperPhonemizer::Rust;
    sp.voice_params = BTreeMap::from([(
        "sapi:David".to_owned(),
        crate::RememberedVoice {
            rate: 310,
            pitch: -1,
        },
    )]);
    sp.extra
        .insert("new_engine_option".into(), toml::Value::String("x".into()));
    let h = &mut s.highlight;
    h.enabled = false;
    h.granularity = HighlightGranularity::Both;
    h.lead_words = -2;
    h.speed = 0.7;
    h.color = "#ff8800".into();
    h.sentence_color = Some("yellow".into());
    let n = &mut s.normalization;
    n.math = false;
    n.abbreviations = false;
    n.abbrev_expansions = BTreeMap::from([("approx.".to_owned(), "approximately".to_owned())]);
    n.numbers = false;
    n.use_pronunciations = false;
    n.pronunciations = BTreeMap::from([("GIF".to_owned(), "jif".to_owned())]);
    n.table_mode = TableMode::Flat;
    n.footnote_mode = FootnoteMode::Deferred;
    let r = &mut s.reading;
    r.auto_resume = false;
    r.nav_history_size = 80;
    r.wrap_navigation = true;
    r.cursor_follows_speech = false;
    r.citations = crate::CitationReading::Words;
    r.ocr = false;
    r.ocr_lang = "fra+eng".into();
    r.ocr_engine = crate::OcrEngine::Tesseract;
    r.math_engine = crate::MathEngine::MathCatSimpleSpeak;
    r.math_display = crate::MathDisplay::Unicode;
    r.revisions = crate::RevisionReading::Marked;
    let y = &mut s.sync;
    y.enabled = true;
    y.folder = Some(PathBuf::from("/media/stick/Sync"));
    y.device_name = "lab".into();
    y.places = false;
    y.notes = false;
    y.highlights = false;
    y.bookmarks = false;
    y.statistics = false;
    y.settings = false;
    y.profiles = false;
    y.key_overrides = false;
    y.words = false;
    y.glossary = false;
    y.favorite_voices = false;
    y.position_policy = crate::PositionPolicy::Ask;
    let d = &mut s.display;
    d.theme = "nord".into();
    d.wrap_width = 100;
    d.measure = 72;
    d.tab_width = 2;
    d.show_line_numbers = true;
    d.scroll_margin = 5;
    d.hints = crate::HintsLine::Off;
    let e = &mut s.editing;
    e.autosave_recovery = false;
    e.autosave_interval_secs = 60;
    e.echo_characters = false;
    e.echo_words = false;
    e.echo_deletions = false;
    e.echo_lines_on_move = false;
    e.undo_steps = 200;
    e.undo_memory_mb = 10;
    e.author = "Ada Example".into();
    s.library.recent_limit = 10;
    s.library.folders = vec!["C:/Books".into()];
    s.keyboard.character_keys = false;
    s.keyboard.preset = crate::KeymapPreset::Classic;
    s.keyboard.digit_row = crate::DigitRow::Azerty;
    let acc = &mut s.accessibility;
    acc.mode = crate::AccessMode::Hybrid;
    acc.say_all = crate::SayAll::Voice;
    acc.quiet_screen = crate::QuietScreen::On;
    acc.cursor = crate::CursorPlacement::Status;
    acc.hybrid_offered = true;
    acc.interface_announcements = crate::InterfaceAnnouncements::Full;
    // Agent D3's additions: themes, the community lexicon, audio export,
    // and the reading aids.
    s.display.follow_os_theme = false;
    s.display.theme_explicit = true;
    let lex = &mut s.normalization.community_lexicon;
    lex.enabled = true;
    lex.dir = Some("C:/dicts".into());
    lex.language = "DEU".into();
    s.normalization.medical_lexicon.enabled = true;
    s.normalization.medical_lexicon.overlay = Some("C:/terms/medical.toml".into());
    s.normalization.math_verbosity = textweaver_core::Verbosity::High;
    s.normalization.asciimath_delimiter = Some('`');
    s.export.subtitle_format = crate::SubtitleFormat::Vtt;
    s.export.subtitle_word_level = true;
    s.export.subtitles_with_audio = true;
    s.export.subtitle_karaoke = crate::SubtitleKaraoke::Lines;
    s.export.subtitle_chapters = true;
    s.braille.math_code = crate::MathBrailleCode::Ueb;
    s.braille.table_format = crate::BrailleTableFormat::Listed;
    let a = &mut s.reading_aids;
    a.rsvp = crate::reading_aids::RsvpSettings {
        wpm: 450,
        pacing: crate::reading_aids::Pacing::External,
        clause_pause: 40,
        sentence_pause: 90,
        paragraph_pause: 120,
        long_word_len: 9,
        long_word_step: 12,
        long_word_max: 60,
        show_previous: false,
        show_next: false,
        position: crate::reading_aids::RsvpPosition::Center,
        font_size_pt: 60,
        lead_words: 1,
    };
    a.bionic = true;
    a.bionic_options = crate::reading_aids::BionicOptions {
        ratio: 0.5,
        min_word_len: 3,
        skip_numbers: false,
        skip_urls: false,
        skip_code: false,
    };
    a.spacing = crate::reading_aids::TextSpacing {
        line_height: 2.0,
        paragraph_spacing: 2.0,
        letter_spacing: 0.12,
        word_spacing: 0.16,
    };
    a.font = crate::reading_aids::FontSettings {
        family: "serif".into(),
        size_pt: 18.0,
        weight: 700,
    };
    a.ruler = crate::reading_aids::RulerSettings {
        mode: crate::reading_aids::RulerMode::Ruler,
        scope: crate::reading_aids::RulerScope::Row,
        rows_above: 2,
        rows_below: 2,
        mask_outside: true,
    };
    a.syllables = true;
    a.difficult_words = true;
    a.difficult_definitions = true;
    a.syllable_options = crate::reading_aids::SyllableOptions {
        separator: "-".into(),
        left_min: 1,
        right_min: 3,
        min_word_len: 5,
        skip_urls: false,
        skip_code: false,
    };
    s.preview.auto_reload = true;
    s.preview.live = true;
    s.lexicon.glossary = Some("glossary.txt".into());
    s.lexicon.data_file = Some("lexicon-en.twlex".into());
    s.stats.enabled = false;
    s.summary.sentences = 7;
    s.dictation.speak_while_recording = true;
    s.dictation.model_dir = Some("D:/models/whisper-base.en".into());
    s.dictation.model = "whisper-small.en".into();
    s.components.mirror = "D:/mirror".into();
    s.components.chooser_shown = true;
    s.interface.language = "en-XA".into();
    s.interface.rtl = crate::RtlDisplay::Off;
    s.interface.recent_settings = vec!["speech.rate".into()];
    s.speech
        .voices_by_language
        .insert("es".into(), "espeak:es".into());
    s.gui.announce = crate::GuiAnnounce::Uia;
    s.gui.auto_hide_menu = true;
    s.gui.speak_messages = true;
    s.gui.header = false;
    s.gui.toolbar = false;
    s.gui.sidebar = crate::GuiSidebar::Notes;
    s.gui.window = Some(crate::GuiWindow {
        x: 30,
        y: 40,
        width: 900,
        height: 640,
        maximized: true,
    });
    let c = &mut s.colors;
    c.ruler = "orange".into();
    c.difficult_words = "blue".into();
    c.syllables = "#336699".into();
    c.misspellings = "orange".into();
    c.lint = "navy".into();
    c.find_match = "gold".into();
    c.selection = "skyblue".into();
    c.focus = "orange".into();
    c.links = "blue".into();
    c.headings = "navy".into();
    c.status_bar = "#202020".into();
    c.notes = "pink".into();
    c.bookmarks = "teal".into();
    s.extra.insert("future_key".into(), toml::Value::Integer(1));
    let future: toml::Table = "a = 1\nwhen = 2026-09-25T14:03:07Z\n".parse().unwrap();
    s.extra
        .insert("future_table".into(), toml::Value::Table(future));
    s
}

fn some_keys() -> KeymapOverrides {
    KeymapOverrides::from([
        (
            "next_sentence".to_owned(),
            vec!["Alt+N".to_owned(), "b:.".to_owned()],
        ),
        ("stop".to_owned(), Vec::new()),
    ])
}

/// Every setting's dotted path, from the defaults.
fn leaf_paths(m: &Map<String, Value>, path: &str, out: &mut Vec<String>) {
    for (k, v) in m {
        let sub = join(path, k);
        match v {
            Value::Object(o) if is_struct(&sub) => leaf_paths(o, &sub, out),
            _ => out.push(sub),
        }
    }
}

fn get<'a>(m: &'a Map<String, Value>, path: &str) -> Option<&'a Value> {
    let mut parts = path.split('.');
    let mut v = m.get(parts.next()?)?;
    for p in parts {
        v = v.as_object()?.get(p)?;
    }
    Some(v)
}

fn saved(store: &SettingsStore, settings: &Settings, keymap: &KeymapOverrides) {
    store.save(settings).unwrap();
    store.save_keymap(keymap).unwrap();
}

fn read(store: &SettingsStore, file: &str) -> String {
    std::fs::read_to_string(store.paths().config_dir.join(file)).unwrap()
}

const JSON_FULL: ExportOptions = ExportOptions {
    changed_only: false,
    format: ExportFormat::Json,
};

/// The test really changes every setting: a setting added later without a
/// value here fails this test until it gets one.
#[test]
fn the_fixture_changes_every_setting() {
    let defaults = settings_to_json(&Settings::default()).unwrap();
    let mut paths = Vec::new();
    leaf_paths(&defaults, "", &mut paths);
    assert!(paths.len() > 50, "{paths:?}");
    let changed = json_minimal(
        settings_to_json(&everything_changed()).unwrap(),
        &defaults,
        "",
    );
    for p in &paths {
        assert!(
            get(&changed, p).is_some(),
            "{p} is not changed by the fixture"
        );
    }
}

#[test]
fn export_then_import_changes_nothing() {
    let (_d, a) = store();
    let (s, k) = (everything_changed(), some_keys());
    saved(&a, &s, &k);
    let settings_text = read(&a, "settings.toml");
    for changed_only in [false, true] {
        for format in [ExportFormat::Json, ExportFormat::Toml] {
            let opts = ExportOptions {
                changed_only,
                format,
            };
            let text = a.export(opts).unwrap();
            for mode in [ImportMode::Merge, ImportMode::Replace] {
                // Over the same settings: nothing to do.
                let plan = a.plan_import(&text, mode).unwrap();
                assert!(plan.is_empty(), "{opts:?} {mode:?}: {:?}", plan.lines());
                assert!(plan.changes.is_empty());
                assert_eq!(a.apply(&plan).unwrap(), Applied::default());
                assert_eq!(read(&a, "settings.toml"), settings_text);

                // Into a fresh store: the same settings, the same file.
                let (_d2, b) = store();
                let plan = b.plan_import(&text, mode).unwrap();
                assert!(
                    plan.warnings.iter().all(|w| w.contains("kept")),
                    "{:?}",
                    plan.warnings
                );
                b.apply(&plan).unwrap();
                assert_eq!(b.load_detailed().settings, s, "{opts:?} {mode:?}");
                assert_eq!(b.load_keymap().unwrap(), k);
                assert_eq!(
                    read(&b, "settings.toml"),
                    settings_text,
                    "{opts:?} {mode:?}"
                );
                // And its export is the same document.
                let strip = |t: &str| {
                    t.lines()
                        .filter(|l| !l.contains("exported"))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                assert_eq!(strip(&b.export(opts).unwrap()), strip(&text));
            }
        }
    }
}

#[test]
fn json_is_sorted_two_space_and_stable() {
    let s = everything_changed();
    let text = export_settings(&s, &some_keys(), JSON_FULL, 1_790_344_987).unwrap();
    assert!(text.ends_with("}\n"));
    assert!(
        text.contains("\n  \"exported\": \"2026-09-25T14:03:07Z\",\n"),
        "{text}"
    );
    assert!(text.contains("\n    \"speech\": {\n"), "{text}");
    let pos = |needle: &str| text.find(needle).unwrap_or_else(|| panic!("{needle}"));
    assert!(pos("\"exported\"") < pos("\"keymap\""));
    assert!(pos("\"keymap\"") < pos("\"settings\""));
    assert!(pos("\"settings\"") < pos("\"textweaver_settings\": 1"));
    assert!(pos("\"apple\"") < pos("\"auto_play\""));
    assert!(pos("\"auto_play\"") < pos("\"backend\": \"sapi\""));
    assert!(pos("\"display\"") < pos("\"editing\""));
    // f32 values in their short form.
    assert!(text.contains("\"speed\": 0.7\n"), "{text}");
    // A TOML date travels as the toml crate's encoding.
    assert!(text.contains("\"$__toml_private_datetime\": \"2026-09-25T14:03:07Z\""));
    // The same input gives the same text.
    assert_eq!(
        text,
        export_settings(&s, &some_keys(), JSON_FULL, 1_790_344_987).unwrap()
    );
}

#[test]
fn full_export_lists_every_setting_and_changed_only_lists_changes() {
    let text =
        export_settings(&Settings::default(), &KeymapOverrides::new(), JSON_FULL, 0).unwrap();
    let doc: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(doc[FORMAT_KEY], 1);
    assert_eq!(doc["settings"]["speech"]["voice"], Value::Null);
    assert_eq!(doc["settings"]["speech"]["rate"], 265);
    assert_eq!(doc["settings"]["highlight"]["sentence_color"], Value::Null);
    assert_eq!(doc["keymap"], serde_json::json!({}));

    let opts = ExportOptions {
        changed_only: true,
        format: ExportFormat::Json,
    };
    let text = export_settings(&Settings::default(), &KeymapOverrides::new(), opts, 0).unwrap();
    let doc: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(doc["settings"], serde_json::json!({}));

    let mut s = Settings::default();
    s.speech.rate = Rate::Wpm(300);
    s.speech.eci.code_factory = true;
    let text = export_settings(&s, &some_keys(), opts, 0).unwrap();
    let doc: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        doc["settings"],
        serde_json::json!({"speech": {"rate": 300, "eci": {"code_factory": true}}})
    );
    assert_eq!(doc["keymap"]["stop"], serde_json::json!([]));
}

#[test]
fn toml_export_reads_back() {
    let opts = ExportOptions {
        changed_only: true,
        format: ExportFormat::Toml,
    };
    let mut s = Settings::default();
    s.highlight.speed = 0.7;
    s.display.theme = "nord".into();
    let text = export_settings(&s, &some_keys(), opts, 1_790_344_987).unwrap();
    assert!(text.starts_with("# textweaver settings export."), "{text}");
    let table: toml::Table = text.parse().unwrap();
    assert_eq!(table[FORMAT_KEY].as_integer(), Some(1));
    assert_eq!(table["settings"]["display"]["theme"].as_str(), Some("nord"));
    assert!(text.contains("speed = 0.7"), "{text}");
    let plan = plan_import(
        &Settings::default(),
        &KeymapOverrides::new(),
        &text,
        ImportMode::Merge,
    )
    .unwrap();
    assert_eq!(plan.settings, s);
    assert_eq!(plan.keymap, some_keys());
}

#[test]
fn merge_changes_only_what_the_file_names() {
    let mut current = Settings::default();
    current.speech.rate = Rate::Wpm(300);
    current.display.theme = "nord".into();
    current.speech.speed_presets.insert("fast".into(), 500);
    let keys = some_keys();
    let text = r#"{"textweaver_settings": 1, "settings": {"speech": {"volume": 50, "speed_presets": {"skim": 400}}}, "keymap": {"quit": ["Ctrl+Q"]}}"#;
    let plan = plan_import(&current, &keys, text, ImportMode::Merge).unwrap();
    assert_eq!(plan.settings.speech.rate, Rate::Wpm(300));
    assert_eq!(plan.settings.display.theme, "nord");
    assert_eq!(plan.settings.speech.volume, Volume::new(50));
    // A map is one value: replaced whole.
    assert_eq!(
        plan.settings.speech.speed_presets,
        BTreeMap::from([("skim".to_owned(), 400)])
    );
    assert_eq!(plan.keymap.len(), 3);
    assert_eq!(plan.keymap["quit"], vec!["Ctrl+Q".to_owned()]);
    let lines = plan.lines();
    assert!(
        lines.contains(&"speech.volume changes from 100 to 50.".to_owned()),
        "{lines:?}"
    );
    assert!(
        lines.contains(&"speech.speed_presets.skim changes from 350 to 400.".to_owned())
            || lines.contains(&"speech.speed_presets.skim is added, set to 400.".to_owned()),
        "{lines:?}"
    );
    assert!(
        lines.contains(&"Keys for quit change from the default keys to Ctrl+Q.".to_owned()),
        "{lines:?}"
    );
    assert_eq!(plan.change_count(), lines.len());
    assert!(plan.settings_changed && plan.keymap_changed);
}

#[test]
fn null_returns_a_setting_or_key_to_its_default() {
    let mut current = Settings::default();
    current.speech.voice = Some("David".into());
    current.speech.rate = Rate::Wpm(300);
    let text = r#"{"textweaver_settings": 1, "settings": {"speech": {"voice": null, "rate": null}}, "keymap": {"stop": null}}"#;
    let plan = plan_import(&current, &some_keys(), text, ImportMode::Merge).unwrap();
    assert_eq!(plan.settings, Settings::default());
    assert!(!plan.keymap.contains_key("stop"));
    let lines = plan.lines();
    assert!(
        lines.contains(&"speech.voice changes from David to not set.".to_owned()),
        "{lines:?}"
    );
    assert!(
        lines.contains(&"Keys for stop change from no keys to the default keys.".to_owned()),
        "{lines:?}"
    );
}

#[test]
fn replace_makes_the_settings_exactly_the_files() {
    let mut current = Settings::default();
    current.speech.rate = Rate::Wpm(300);
    current.display.theme = "nord".into();
    let only_volume = r#"{"textweaver_settings": 1, "settings": {"speech": {"volume": 50}}}"#;
    let plan = plan_import(&current, &some_keys(), only_volume, ImportMode::Replace).unwrap();
    let mut want = Settings::default();
    want.speech.volume = Volume::new(50);
    assert_eq!(plan.settings, want);
    // No keymap in the file: the overrides stay.
    assert_eq!(plan.keymap, some_keys());
    let with_keys = r#"{"textweaver_settings": 1, "settings": {}, "keymap": {}}"#;
    let plan = plan_import(&current, &some_keys(), with_keys, ImportMode::Replace).unwrap();
    assert_eq!(plan.settings, Settings::default());
    assert!(plan.keymap.is_empty());
}

#[test]
fn wrong_types_and_ranges_are_errors_naming_the_path() {
    let (_d, store) = store();
    let mut s = Settings::default();
    s.display.theme = "nord".into();
    store.save(&s).unwrap();
    let before = config_files(&store);
    let text = r#"{
      "textweaver_settings": 1,
      "settings": {
        "speech": {"rate": "fast", "volume": 150, "punctuation": "loud", "eci": 5,
                   "speed_presets": {"skim": 2000}},
        "highlight": {"speed": 3.0, "lead_words": -9},
        "display": "dark"
      },
      "keymap": {"stop": 5, "quit": ["Ctrl+Q", 7]}
    }"#;
    let err = store.plan_import(text, ImportMode::Merge).unwrap_err();
    let SettingsIoError::Invalid(errors) = &err else {
        panic!("{err}");
    };
    let has = |p: &str| errors.iter().any(|e| e.starts_with(p));
    for p in [
        "settings.speech.rate:",
        "settings.speech.punctuation:",
        "settings.speech.eci:",
        "settings.display:",
        "keymap.stop:",
        "keymap.quit:",
    ] {
        assert!(has(p), "{p} missing from {errors:?}");
    }
    for e in [
        "settings.speech.rate: expected a whole number, 0 or more, found the text \"fast\"",
        "settings.speech.punctuation: loud is not one of the choices: none, some, all",
        "settings.display: expected a section (a JSON object), found dark",
    ] {
        assert!(errors.iter().any(|x| x == e), "{e} missing from {errors:?}");
    }
    let SettingsIoError::Invalid(big) = store
        .plan_import(r#"{"speech": {"rate": 70000}}"#, ImportMode::Merge)
        .unwrap_err()
    else {
        panic!()
    };
    assert_eq!(big, vec!["speech.rate: the number 70000 is out of range"]);
    assert!(
        errors.iter().all(|e| !e.contains('`')),
        "no backticks for screen readers: {errors:?}"
    );
    let message = err.to_string();
    assert!(message.contains("nothing was changed"), "{message}");
    assert_eq!(config_files(&store), before, "nothing written");

    // Ranges are checked once the types are right.
    let text = r#"{"textweaver_settings": 1, "settings": {"speech": {"volume": 150, "rate": 5000, "speed_presets": {"skim": 2000}}, "highlight": {"speed": 3.0, "lead_words": -9}}}"#;
    let SettingsIoError::Invalid(errors) = store.plan_import(text, ImportMode::Merge).unwrap_err()
    else {
        panic!()
    };
    for p in [
        "settings.speech.volume: 150 is outside 0 to 100 percent",
        "settings.speech.rate: 5000 is outside 50 to 900 words per minute",
        "settings.speech.speed_presets.skim: 2000",
        "settings.highlight.speed: 3 is outside 0.5 to 1.5",
        "settings.highlight.lead_words: -9 is outside -5 to 5",
    ] {
        assert!(
            errors.iter().any(|e| e.starts_with(p)),
            "{p} missing from {errors:?}"
        );
    }
}

#[test]
fn unknown_sections_and_keys_are_reported_and_kept() {
    let (_d, store) = store();
    let text = r#"{"textweaver_settings": 1, "settings": {"future": {"a": 1}, "speech": {"new_option": "x", "rate": 280}}, "extra_part": 1}"#;
    let plan = store.plan_import(text, ImportMode::Merge).unwrap();
    let w = plan.warnings.join("\n");
    assert!(
        w.contains("settings.future is not a textweaver settings section; it is kept."),
        "{w}"
    );
    assert!(
        w.contains("settings.speech.new_option is not a textweaver setting; it is kept."),
        "{w}"
    );
    assert!(
        w.contains("extra_part is not part of a settings export and is ignored."),
        "{w}"
    );
    store.apply(&plan).unwrap();
    let text = read(&store, "settings.toml");
    assert!(
        text.contains("[future]") && text.contains("new_option = \"x\""),
        "{text}"
    );
    let back = store.export(JSON_FULL).unwrap();
    assert!(back.contains("\"new_option\": \"x\""));
}

#[test]
fn star_settings_point_to_migrate_star() {
    let star = r#"{"tts_rate": 300, "theme": "galaxy", "reading_positions": {}}"#;
    let err = plan_import(
        &Settings::default(),
        &KeymapOverrides::new(),
        star,
        ImportMode::Merge,
    )
    .unwrap_err();
    assert!(matches!(err, SettingsIoError::StarSettings));
    assert!(err.to_string().contains("tw migrate-star"), "{err}");
}

#[test]
fn a_bare_settings_toml_imports() {
    let text = "[speech]\nrate = 310\n[display]\ntheme = \"nord\"\n";
    let plan = plan_import(&Settings::default(), &some_keys(), text, ImportMode::Merge).unwrap();
    assert_eq!(plan.settings.speech.rate, Rate::Wpm(310));
    assert_eq!(
        plan.keymap,
        some_keys(),
        "a settings file leaves the keys alone"
    );
    // Paths in errors match the file: no `settings.` prefix.
    let SettingsIoError::Invalid(e) = plan_import(
        &Settings::default(),
        &KeymapOverrides::new(),
        "[speech]\nrate = \"x\"\n",
        ImportMode::Merge,
    )
    .unwrap_err() else {
        panic!()
    };
    assert!(e[0].starts_with("speech.rate:"), "{e:?}");
    // A bare JSON settings object works too.
    let plan = plan_import(
        &Settings::default(),
        &KeymapOverrides::new(),
        r#"{"speech": {"rate": 320}}"#,
        ImportMode::Merge,
    )
    .unwrap();
    assert_eq!(plan.settings.speech.rate, Rate::Wpm(320));
}

#[test]
fn files_that_are_not_settings_are_refused() {
    let k = KeymapOverrides::new();
    let d = Settings::default();
    let err = plan_import(&d, &k, r#"{"name": "x"}"#, ImportMode::Merge).unwrap_err();
    assert!(matches!(err, SettingsIoError::NotSettings(_)), "{err}");
    let err = plan_import(&d, &k, "[1, 2]", ImportMode::Merge).unwrap_err();
    assert!(matches!(err, SettingsIoError::Syntax { .. }), "{err}");
    let err = plan_import(&d, &k, "{\n  \"settings\": {,\n}", ImportMode::Merge).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.starts_with("The file is not valid JSON") && msg.contains("line 2"),
        "{msg}"
    );
    let err = plan_import(&d, &k, "[speech]\nrate = = 3\n", ImportMode::Merge).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.starts_with("The file is not valid TOML") && msg.contains("line 2"),
        "{msg}"
    );
    let err = plan_import(
        &d,
        &k,
        r#"{"textweaver_settings": 2, "settings": {}}"#,
        ImportMode::Merge,
    )
    .unwrap_err();
    assert!(matches!(err, SettingsIoError::NewerFormat(2)), "{err}");
    let err = plan_import(
        &d,
        &k,
        r#"{"textweaver_settings": "one"}"#,
        ImportMode::Merge,
    )
    .unwrap_err();
    assert!(matches!(err, SettingsIoError::Invalid(_)), "{err}");
}

#[test]
fn apply_backs_up_and_writes_atomically() {
    let (_d, store) = store();
    let mut s = Settings::default();
    s.display.theme = "nord".into();
    saved(&store, &s, &some_keys());
    let old_settings = read(&store, "settings.toml");
    let old_keys = read(&store, "keymap.toml");
    let text = r#"{"textweaver_settings": 1, "settings": {"display": {"theme": "sepia"}}, "keymap": {"quit": ["Ctrl+Q"]}}"#;
    let plan = store.plan_import(text, ImportMode::Merge).unwrap();
    let applied = store.apply(&plan).unwrap();
    assert_eq!(applied.backups.len(), 2);
    assert_eq!(applied.written.len(), 2);
    let names: Vec<String> = applied
        .backups
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert!(names[0].starts_with("settings.toml.bak-2"), "{names:?}");
    assert!(names[1].starts_with("keymap.toml.bak-2"), "{names:?}");
    assert_eq!(
        std::fs::read_to_string(&applied.backups[0]).unwrap(),
        old_settings
    );
    assert_eq!(
        std::fs::read_to_string(&applied.backups[1]).unwrap(),
        old_keys
    );
    assert_eq!(store.load().0.display.theme, "sepia");
    assert_eq!(
        config_files(&store).len(),
        4,
        "no temp files: {:?}",
        config_files(&store)
    );

    // A second import in the same second gets its own backup name.
    let text = r#"{"textweaver_settings": 1, "settings": {"display": {"theme": "nord"}}}"#;
    let plan = store.plan_import(text, ImportMode::Merge).unwrap();
    let again = store.apply(&plan).unwrap();
    assert_eq!(again.backups.len(), 1);
    assert_ne!(again.backups[0], applied.backups[0]);
}

#[test]
fn first_import_needs_no_backup_and_planning_never_writes() {
    let (_d, store) = store();
    let text = r#"{"textweaver_settings": 1, "settings": {"speech": {"rate": 300}}}"#;
    let plan = store.plan_import(text, ImportMode::Merge).unwrap();
    assert_eq!(plan.summary(), "1 setting changes.");
    assert!(config_files(&store).is_empty(), "a dry run writes nothing");
    let applied = store.apply(&plan).unwrap();
    assert!(applied.backups.is_empty());
    assert_eq!(config_files(&store), vec!["settings.toml".to_owned()]);
}

#[test]
fn an_unreadable_current_file_stops_the_import() {
    let (_d, store) = store();
    std::fs::create_dir_all(&store.paths().config_dir).unwrap();
    std::fs::write(store.paths().settings_file(), "{ not toml").unwrap();
    let err = store
        .plan_import(r#"{"speech": {"rate": 300}}"#, ImportMode::Merge)
        .unwrap_err();
    assert!(
        matches!(err, SettingsIoError::CurrentUnreadable { .. }),
        "{err}"
    );
    assert!(err.to_string().contains("cannot be read"));
    assert_eq!(
        config_files(&store),
        vec!["settings.toml".to_owned()],
        "not moved aside"
    );
}

#[test]
fn reset_everything_one_section_or_the_keys() {
    let s = everything_changed();
    let k = some_keys();
    let all = plan_reset(&s, &k, None).unwrap();
    assert_eq!(all.settings, Settings::default());
    assert!(all.keymap.is_empty());
    assert!(all.change_count() > 50);

    let speech = plan_reset(&s, &k, Some("speech")).unwrap();
    assert_eq!(speech.settings.speech, Settings::default().speech);
    assert_eq!(speech.settings.display, s.display);
    assert_eq!(speech.keymap, k);

    let eci = plan_reset(&s, &k, Some("speech.eci")).unwrap();
    assert_eq!(eci.settings.speech.eci, EciSettings::default());
    assert_eq!(eci.settings.speech.rate, s.speech.rate);

    let keys = plan_reset(&s, &k, Some("keymap")).unwrap();
    assert_eq!(keys.settings, s);
    assert!(keys.keymap.is_empty());
    assert_eq!(keys.change_count(), 2);

    let future = plan_reset(&s, &k, Some("future_table")).unwrap();
    assert!(!future.settings.extra.contains_key("future_table"));

    let sections = reset_sections(&s);
    for t in STRUCT_TABLES {
        assert!(sections.iter().any(|x| x == t), "{t} cannot be reset");
    }
    assert_eq!(sections[0], "speech");

    let err = plan_reset(&s, &k, Some("sound")).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("no settings section named sound") && msg.contains("speech.eci"),
        "{msg}"
    );

    let nothing = plan_reset(&Settings::default(), &KeymapOverrides::new(), None).unwrap();
    assert!(nothing.is_empty());
    assert_eq!(nothing.summary(), "Nothing changes.");
}

#[test]
fn change_lines_read_as_plain_words() {
    let c = |path: &str, before: Option<Value>, after: Option<Value>| Change {
        area: ChangeArea::Settings,
        path: path.into(),
        before,
        after,
    };
    use serde_json::json;
    assert_eq!(
        c("reading.auto_resume", Some(json!(true)), Some(json!(false))).describe(),
        "reading.auto_resume changes from on to off."
    );
    assert_eq!(
        c(
            "speech.prefer_voice",
            Some(json!("eloquence")),
            Some(json!(""))
        )
        .describe(),
        "speech.prefer_voice changes from eloquence to empty text."
    );
    assert_eq!(
        c(
            "speech.favorite_voices",
            Some(json!([])),
            Some(json!(["David", "Zira"]))
        )
        .describe(),
        "speech.favorite_voices changes from an empty list to David, Zira."
    );
    assert_eq!(
        c("normalization.pronunciations.GIF", None, Some(json!("jif"))).describe(),
        "normalization.pronunciations.GIF is added, set to jif."
    );
    assert_eq!(
        c("future_key", Some(json!(1)), None).describe(),
        "future_key is removed; it was 1."
    );
    let keys = Change {
        area: ChangeArea::Keymap,
        path: "next_sentence".into(),
        before: None,
        after: Some(json!(["Alt+N", "b:."])),
    };
    assert_eq!(
        keys.describe(),
        "Keys for next_sentence change from the default keys to Alt+N, b:.."
    );
}

#[test]
fn toml_dates_in_unknown_keys_survive_json() {
    let mut s = Settings::default();
    let t: toml::Table = "when = 1979-05-27T07:32:00Z\n".parse().unwrap();
    s.extra.insert("future".into(), toml::Value::Table(t));
    let text = export_settings(&s, &KeymapOverrides::new(), JSON_FULL, 0).unwrap();
    let plan = plan_import(
        &Settings::default(),
        &KeymapOverrides::new(),
        &text,
        ImportMode::Replace,
    )
    .unwrap();
    assert_eq!(plan.settings, s);
    assert!(plan.settings.extra["future"]["when"].is_datetime());
}

#[test]
fn export_format_follows_the_file_name() {
    assert_eq!(
        ExportFormat::for_path(Path::new("a.TOML")),
        ExportFormat::Toml
    );
    assert_eq!(
        ExportFormat::for_path(Path::new("a.json")),
        ExportFormat::Json
    );
    assert_eq!(ExportFormat::for_path(Path::new("a")), ExportFormat::Json);
}

/// Every section of `Settings` is either compared key by key (and can be
/// reset alone) or is a map stored whole. A new section missing from
/// `STRUCT_TABLES` would be saved whole and could not be reset by name.
#[test]
fn every_settings_section_is_known() {
    // Tables that replace their default as a whole (maps, not sections).
    const WHOLE: [&str; 0] = [];
    let value = toml::Value::try_from(Settings::default()).unwrap();
    let table = value.as_table().unwrap();
    for (key, v) in table {
        if !v.is_table() || WHOLE.contains(&key.as_str()) {
            continue;
        }
        assert!(
            STRUCT_TABLES.contains(&key.as_str()),
            "[{key}] is a settings section missing from STRUCT_TABLES"
        );
    }
}
