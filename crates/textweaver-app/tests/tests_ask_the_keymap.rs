//! Tests press what the keymap binds on the running platform, never a
//! chord written into the test (W6u, the owner's rule after the macOS
//! build broke on Tuesday, September 29, 2026: a test pressed Ctrl+Right
//! for a word, where a Mac uses Option+Right).
//!
//! This reads the frontends' and the app's integration tests and fails on
//! a key built from written modifiers (`KeyModifiers::CONTROL`, `ALT`,
//! `SUPER`; Masonry's `Modifiers::CONTROL`, `ALT`, `META`) or a chord
//! parsed from text with Ctrl, Alt, or Cmd in it. A test asks the keymap
//! instead: `Tui::key_for(action)` in the terminal, `keys::press` with a
//! chord from `Keymap::chords_for` in the GUI, `chords_for` in the app.
//!
//! The keys below are not the keymap's, and are allowed with a reason.

use std::path::Path;

/// File, the text a line holds, and why it may.
const ALLOWED: &[(&str, &str, &str)] = &[
    (
        "altgr.rs",
        "KeyModifiers::",
        "AltGr arrives as Ctrl and Alt: these test typing, not commands",
    ),
    (
        "edit.rs",
        "KeyModifiers::CONTROL | KeyModifiers::SHIFT",
        "the terminal's own caret keys in edit mode, not a command",
    ),
    (
        "edit_mode.rs",
        "Modifiers::CONTROL",
        "Ctrl+Tab moves the focus out of an edit field on every system",
    ),
    (
        "settings_dialog.rs",
        "Modifiers::CONTROL",
        "Ctrl+PageDown moves between a dialog's sections on every system",
    ),
    (
        "tests_ask_the_keymap.rs",
        "",
        "this file names the patterns it looks for",
    ),
];

fn is_written_chord(line: &str) -> bool {
    let t = line.trim_start();
    if t.starts_with("//") {
        return false;
    }
    let modifiers = [
        "KeyModifiers::CONTROL",
        "KeyModifiers::ALT",
        "KeyModifiers::SUPER",
        "Modifiers::CONTROL",
        "Modifiers::ALT",
        "Modifiers::META",
    ];
    if modifiers.iter().any(|m| t.contains(m)) {
        return true;
    }
    // "Ctrl+O".parse() and friends.
    for m in ["\"Ctrl+", "\"Alt+", "\"Cmd+"] {
        if let Some(i) = t.find(m) {
            let rest = &t[i + 1..];
            if let Some(end) = rest.find('"')
                && rest[end + 1..].trim_start().starts_with(".parse")
            {
                return true;
            }
        }
    }
    false
}

#[test]
fn tests_press_the_keymap_s_keys() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut bad = Vec::new();
    for dir in ["textweaver-app", "textweaver-tui", "textweaver-xilem"] {
        let Ok(entries) = std::fs::read_dir(crates.join(dir).join("tests")) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let text = std::fs::read_to_string(&path).unwrap();
            for (n, line) in text.lines().enumerate() {
                if !is_written_chord(line) {
                    continue;
                }
                let allowed = ALLOWED
                    .iter()
                    .any(|(f, needle, _)| *f == name && line.contains(needle));
                if !allowed {
                    bad.push(format!("{dir}/tests/{name}:{}: {}", n + 1, line.trim()));
                }
            }
        }
    }
    assert!(
        bad.is_empty(),
        "tests pressing written chords; ask the keymap for the action's key instead:\n{}",
        bad.join("\n")
    );
}
