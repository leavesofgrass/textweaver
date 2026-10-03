//! Settings golden files: a `settings.toml` as each alpha from alpha.1 to
//! alpha.8 saved it (`fixtures/w9a-d/settings/`), loaded by today's code
//! with every value asserted after migration (W9a-d). A rename keeps its
//! value under the new key, a removed setting is dropped, list settings
//! keep items with commas, and nothing is reported as invalid. Each file
//! also survives a save and a second load unchanged.

use std::path::{Path, PathBuf};

use textweaver_store::{Paths, SettingsStore};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/w9a-d/settings")
        .join(name)
}

/// The value at a dotted path in `table`, or `None`.
fn at<'a>(table: &'a toml::Table, path: &str) -> Option<&'a toml::Value> {
    let mut parts = path.split('.');
    let mut cur = table.get(parts.next()?)?;
    for p in parts {
        cur = cur.as_table()?.get(p)?;
    }
    Some(cur)
}

/// Loads `name` through a [`SettingsStore`], as the app does, and checks
/// each `(path, expected)`; returns the loaded settings as a table.
fn load(name: &str, expected: &[(&str, toml::Value)]) -> toml::Table {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    std::fs::create_dir_all(paths.settings_file().parent().unwrap()).unwrap();
    std::fs::copy(fixture(name), paths.settings_file()).unwrap();
    let store = SettingsStore::new(paths);
    let loaded = store.load_detailed();
    assert!(loaded.error.is_none(), "{name}: {:?}", loaded.error);
    assert!(
        loaded.warnings.is_empty(),
        "{name} warned: {:?}",
        loaded.warnings
    );
    let table = toml::Table::try_from(&loaded.settings).unwrap();
    for (path, want) in expected {
        assert_eq!(at(&table, path), Some(want), "{name}: {path}");
    }
    // A save and a second load give the same settings.
    store.save(&loaded.settings).unwrap();
    let (again, message) = store.load();
    assert!(message.is_none(), "{name}: {message:?}");
    assert_eq!(again, loaded.settings, "{name}: changed by a save");
    table
}

fn s(v: &str) -> toml::Value {
    toml::Value::String(v.to_owned())
}

fn i(v: i64) -> toml::Value {
    toml::Value::Integer(v)
}

fn b(v: bool) -> toml::Value {
    toml::Value::Boolean(v)
}

fn list(items: &[&str]) -> toml::Value {
    toml::Value::Array(items.iter().map(|x| s(x)).collect())
}

#[test]
fn alpha_1_settings_load() {
    let t = load(
        "alpha-1.toml",
        &[
            ("speech.rate", i(300)),
            ("speech.punctuation", s("all")),
            ("speech.auto_play", b(true)),
            (
                "speech.favorite_voices",
                list(&["espeak:en-us", "espeak:en-gb"]),
            ),
            ("highlight.granularity", s("sentence")),
            ("highlight.lead_words", i(2)),
            ("normalization.table_mode", s("flat")),
            ("normalization.footnote_mode", s("deferred")),
            ("reading.auto_resume", b(false)),
            // Renamed in the sync wave; the old value reads as the new one.
            ("sync.position_policy", s("furthest")),
            ("display.theme", s("contrast")),
            ("display.wrap_width", i(72)),
            ("editing.echo_words", b(false)),
            ("library.recent_limit", i(50)),
            (
                "library.folders",
                list(&["D:/Readings, Fall 2026", "D:/Lab notes"]),
            ),
        ],
    );
    assert_eq!(at(&t, "reading.sync_conflict_policy"), None);
}

#[test]
fn alpha_2_settings_load() {
    load(
        "alpha-2.toml",
        &[
            ("speech.rate", i(200)),
            ("speech.punctuation", s("none")),
            ("speech.latency_offset_ms", i(80)),
            ("highlight.granularity", s("both")),
            ("highlight.sentence_color", s("blue")),
            ("sync.position_policy", s("ask")),
            ("reading.wrap_navigation", b(false)),
            ("display.show_line_numbers", b(true)),
            ("library.folders", list(&["D:/Readings, Fall 2026"])),
        ],
    );
}

#[test]
fn alpha_3_settings_load() {
    load(
        "alpha-3.toml",
        &[
            ("speech.rate", i(250)),
            ("speech.backend", s("espeak")),
            ("speech.eci.code_factory", b(true)),
            ("sync.position_policy", s("newest")),
            ("keyboard.character_keys", b(false)),
            ("library.recent_limit", i(30)),
        ],
    );
}

#[test]
fn alpha_4_settings_load() {
    load(
        "alpha-4.toml",
        &[
            ("speech.rate", i(320)),
            // The default preset's old name.
            ("keyboard.preset", s("default")),
            ("accessibility.mode", s("hybrid")),
            ("reading_aids.bionic", b(true)),
            ("reading_aids.syllables", b(true)),
            ("stats.enabled", b(false)),
            ("interface.language", s("es")),
        ],
    );
}

#[test]
fn alpha_5_settings_load() {
    let t = load(
        "alpha-5.toml",
        &[
            ("summary.sentences", i(7)),
            ("reading_aids.bionic", b(true)),
            ("accessibility.mode", s("screen-reader")),
            ("sync.position_policy", s("furthest")),
        ],
    );
    // Removed since: dropped, not kept as an unknown key.
    assert_eq!(at(&t, "reading_aids.font.fetch_missing"), None);
}

#[test]
fn alpha_6_settings_load() {
    load(
        "alpha-6.toml",
        &[
            ("speech.rate", i(280)),
            (
                "speech.favorite_voices",
                list(&["sapi:Zira, English (United States)"]),
            ),
            ("keyboard.preset", s("classic")),
            ("dictation.speak_while_recording", b(true)),
            (
                "library.folders",
                list(&["D:/Readings, Fall 2026", "D:/Readings; Spring 2027"]),
            ),
        ],
    );
}

#[test]
fn alpha_7_settings_load() {
    let t = load(
        "alpha-7.toml",
        &[
            ("sync.enabled", b(true)),
            ("sync.device_name", s("Lab computer")),
            // The new key wins over the old one in the same file.
            ("sync.position_policy", s("furthest")),
            ("sync.statistics", b(false)),
        ],
    );
    assert_eq!(at(&t, "reading.sync_conflict_policy"), None);
}

#[test]
fn alpha_8_settings_load() {
    load(
        "alpha-8.toml",
        &[
            ("speech.rate", i(265)),
            ("speech.auto_play", b(true)),
            ("components.chooser_shown", b(true)),
            ("sync.enabled", b(true)),
            ("sync.device_name", s("Home laptop")),
            ("library.recent_limit", i(40)),
            ("library.folders", list(&["D:/Readings, Fall 2026"])),
        ],
    );
}

/// Every golden file is tested: one per alpha, none forgotten.
#[test]
fn a_golden_file_for_every_alpha() {
    for n in 1..=8 {
        assert!(fixture(&format!("alpha-{n}.toml")).is_file(), "alpha-{n}");
    }
}
