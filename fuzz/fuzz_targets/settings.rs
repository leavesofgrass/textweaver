//! Fuzz target: the settings files. `settings.toml` is read with
//! `Settings::from_table` (invalid values replaced, unknown keys kept), and
//! the same text is planned as a settings import (JSON or TOML). Neither
//! may panic, and settings written back as TOML must read again.

#![no_main]

use libfuzzer_sys::fuzz_target;
use textweaver_store::{ImportMode, KeymapOverrides, Settings, settings_io};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(table) = text.parse::<toml::Table>() {
        let (settings, _warnings) = Settings::from_table(table);
        if let Ok(again) = settings.to_minimal_toml() {
            let table: toml::Table = again.parse().expect("settings write valid TOML");
            let _ = Settings::from_table(table);
        }
    }
    for mode in [ImportMode::Merge, ImportMode::Replace] {
        let _ = settings_io::plan_import(&Settings::default(), &KeymapOverrides::new(), text, mode);
    }
});
