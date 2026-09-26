//! Fuzz target: `keymap.toml`, the keyboard overrides. The text is read as
//! the overrides table and applied to every platform's and frontend's
//! defaults, and each line is parsed as a key chord. Nothing may panic, a
//! chord that parses must print and parse back to itself, and its spoken
//! name must not be empty.

#![no_main]

use std::collections::BTreeMap;

use libfuzzer_sys::fuzz_target;
use textweaver_keymap::{Frontend, KeyChord, Keymap, Platform};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(overrides) = toml::from_str::<BTreeMap<String, Vec<String>>>(text) {
        for platform in Platform::ALL {
            for frontend in Frontend::ALL {
                let (map, _warnings) = Keymap::with_overrides(platform, frontend, &overrides);
                let _ = map.conflicts();
            }
        }
    }
    for line in text.lines().take(64) {
        if let Ok(chord) = line.parse::<KeyChord>() {
            let printed = chord.to_string();
            let again: KeyChord = printed
                .parse()
                .unwrap_or_else(|e| panic!("{printed:?} (from {line:?}) does not parse: {e}"));
            assert_eq!(again, chord, "{line:?} printed as {printed:?}");
            assert!(!chord.spoken().is_empty());
        }
    }
});
