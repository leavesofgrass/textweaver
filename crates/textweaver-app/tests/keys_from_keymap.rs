//! Every key named in a message comes from the keymap (Wave 4, W4h).
//!
//! A message such as "Press Control O to open one" used to be a fixed
//! string, so it lied once `open` was rebound, and its written form
//! ("Alt+.") was heard as "alt" with punctuation off. Two checks:
//!
//! - no string in the app's or the terminal reader's code names a modifier
//!   chord ("Ctrl+O", "Control O", "Alt+C"), apart from a short list of
//!   keys that are not textweaver's (the terminal's paste) or are not
//!   spoken (a log line);
//! - a self-voicing session that walks through messages naming keys never
//!   says a key in its written form, and never shows the marks that carry
//!   both forms.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use textweaver_app::keymap::ActionId;
use textweaver_app::store::DocKey;
use textweaver_app::testing::{SpeechLog, recording_service};
use textweaver_app::text::Document;
use textweaver_app::{App, AppConfig, Command, ListKey, PromptKey};

/// Strings allowed to name a chord, with why: (file name, text).
const ALLOWED: &[(&str, &str)] = &[
    // The terminal's own paste key, which textweaver does not bind.
    ("authoring.rs", "Control Shift V"),
    // The definition of an extra binding, parsed into the keymap.
    ("extra.rs", "Shift+Y"),
    // `textweaver --help`, printed by clap before the keymap exists.
    ("main.rs", "Ctrl+Q"),
    // A log line, never spoken.
    ("signals.rs", "Ctrl+C"),
];

/// String literals in Rust source, outside comments, up to the file's
/// `#[cfg(test)]` module.
fn string_literals(src: &str) -> Vec<String> {
    let src = match src.find("#[cfg(test)]\nmod tests") {
        Some(i) => &src[..i],
        None => src,
    };
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if c == 'r' && (next == Some('"') || next == Some('#')) && is_raw_start(&chars, i) {
            let mut j = i + 1;
            let mut hashes = 0;
            while chars.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            j += 1; // the opening quote
            let start = j;
            loop {
                if j >= chars.len() {
                    break;
                }
                if chars[j] == '"' && (0..hashes).all(|h| chars.get(j + 1 + h) == Some(&'#')) {
                    break;
                }
                j += 1;
            }
            out.push(chars[start..j.min(chars.len())].iter().collect());
            i = j + 1 + hashes;
        } else if c == '"' {
            let mut j = i + 1;
            let mut s = String::new();
            while j < chars.len() && chars[j] != '"' {
                if chars[j] == '\\' {
                    j += 1;
                    // A line continuation swallows the break and the
                    // indentation after it.
                    if chars.get(j) == Some(&'\n') {
                        while chars.get(j + 1).is_some_and(|c| c.is_whitespace()) {
                            j += 1;
                        }
                    } else if let Some(&e) = chars.get(j) {
                        s.push(e);
                    }
                } else {
                    s.push(chars[j]);
                }
                j += 1;
            }
            out.push(s);
            i = j + 1;
        } else if c == '\'' {
            // A char literal ('x', '\n', '"'), else a lifetime.
            if next == Some('\\') {
                let close = (i + 2..(i + 12).min(chars.len())).find(|&k| chars[k] == '\'');
                i = close.map_or(i + 1, |k| k + 1);
            } else if chars.get(i + 2) == Some(&'\'') {
                i += 3;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    out
}

/// `r"` or `r#"` starting a raw string, not the end of an identifier.
fn is_raw_start(chars: &[char], i: usize) -> bool {
    let before = i.checked_sub(1).map(|k| chars[k]);
    if before.is_some_and(|b| b.is_alphanumeric() || b == '_') {
        return false;
    }
    let mut j = i + 1;
    while chars.get(j) == Some(&'#') {
        j += 1;
    }
    chars.get(j) == Some(&'"')
}

const MODIFIERS: [&str; 6] = ["Ctrl", "Control", "Alt", "Cmd", "Command", "Shift"];

const KEY_NAMES: [&str; 16] = [
    "Space",
    "Tab",
    "Enter",
    "Escape",
    "Up",
    "Down",
    "Left",
    "Right",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Page",
    "Backspace",
    "Delete",
    "Insert",
];

/// The chords named in `s`, such as "Ctrl+O", "Control O", "Alt+.",
/// "Shift+F7": a modifier, then `+` or a space, then a key.
fn chords_in(s: &str) -> Vec<String> {
    let mut found = Vec::new();
    for m in MODIFIERS {
        let mut from = 0;
        while let Some(at) = s[from..].find(m) {
            let start = from + at;
            from = start + m.len();
            let word_start = s[..start]
                .chars()
                .next_back()
                .is_none_or(|b| !b.is_alphanumeric());
            if !word_start {
                continue;
            }
            let rest = &s[from..];
            let Some(sep) = rest.chars().next() else {
                continue;
            };
            if sep != '+' && sep != ' ' {
                continue;
            }
            let key: String = rest[1..]
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != ',')
                .collect();
            // Sentence punctuation after the key is not part of it, but
            // a punctuation key itself ("Alt+.") is.
            let trimmed = key.trim_end_matches(['.', ')', ';', ':']);
            let key = if trimmed.is_empty() {
                key.chars().next().map(String::from).unwrap_or_default()
            } else {
                trimmed.to_owned()
            };
            let key = key.as_str();
            let is_key = match key.chars().count() {
                0 => rest[1..].starts_with(',') && sep == '+',
                1 => {
                    let k = key.chars().next().unwrap_or(' ');
                    k.is_ascii_uppercase()
                        || k.is_ascii_digit()
                        || (sep == '+' && !k.is_alphanumeric())
                }
                _ => {
                    KEY_NAMES.contains(&key)
                        || MODIFIERS.contains(&key)
                        || (key.starts_with('F') && key[1..].parse::<u8>().is_ok())
                        || [
                            "period",
                            "comma",
                            "plus",
                            "minus",
                            "equals",
                            "semicolon",
                            "slash",
                        ]
                        .contains(&key)
                }
            };
            if is_key {
                found.push(format!("{m}{sep}{key}"));
            }
        }
    }
    found
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn the_chord_finder_finds_chords_and_nothing_else() {
    assert_eq!(chords_in("Press Control O to open one."), ["Control O"]);
    assert_eq!(
        chords_in("Save: Ctrl+S. Finish: Ctrl+E."),
        ["Ctrl+S", "Ctrl+E"]
    );
    assert_eq!(chords_in("Insert one with Alt+C."), ["Alt+C"]);
    assert_eq!(
        chords_in("next sentence: Alt+. or Alt+Down"),
        ["Alt+.", "Alt+Down"]
    );
    assert_eq!(chords_in("Louder: Shift+F7"), ["Shift+F7"]);
    assert!(chords_in("with Shift for the previous one").is_empty());
    assert!(chords_in("Command. Type part of a name").is_empty());
    assert!(chords_in("the command palette").is_empty());
    assert!(chords_in("Alternative text").is_empty());
    assert_eq!(
        string_literals("let a = \"one\"; // \"two\"\nlet b = r#\"three \"x\"\"#; 'c'; '\\n';"),
        ["one", "three \"x\""]
    );
}

#[test]
fn no_message_names_a_key_in_a_fixed_string() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    rust_files(&crates.join("textweaver-app").join("src"), &mut files);
    rust_files(&crates.join("textweaver-tui").join("src"), &mut files);
    assert!(files.len() > 50, "{}", files.len());
    let mut fixed = Vec::new();
    for file in &files {
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        let src = std::fs::read_to_string(file).unwrap().replace("\r\n", "\n");
        for lit in string_literals(&src) {
            for chord in chords_in(&lit) {
                let allowed = ALLOWED.iter().any(|(f, text)| {
                    *f == name && lit.contains(text) && text.contains(chord.as_str())
                });
                if !allowed {
                    fixed.push(format!("{name}: {chord} in {lit:?}"));
                }
            }
        }
    }
    assert!(
        fixed.is_empty(),
        "keys named in fixed strings; take them from the keymap (App::key, App::keys, named_key):\n{}",
        fixed.join("\n")
    );
}

/// A self-voicing app with a document open, and what its voice says.
fn voiced(text: &str) -> (App, SpeechLog) {
    let (speech, log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    app.open_document(
        Document::from_plain_text(text),
        DocKey::untitled(1),
        "Doc".into(),
    );
    (app, log)
}

/// Waits (with a deadline) until the voice has said something containing
/// `needle`, and returns everything said.
fn heard(log: &SpeechLog, needle: &str) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let texts = log.texts();
        if texts.iter().any(|t| t.contains(needle)) || Instant::now() >= deadline {
            return texts;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn is_mark(c: char) -> bool {
    ('\u{E000}'..='\u{E002}').contains(&c)
}

#[test]
fn keys_are_spoken_by_name_and_written_on_the_status_line() {
    let (mut app, log) = voiced("A sentence to read. Another one.\n");
    // Edit mode names Save and Finish.
    app.dispatch(Command::Action(ActionId::ToggleEditMode));
    let said = heard(&log, "Edit mode on");
    let msg = said
        .iter()
        .find(|t| t.starts_with("Edit mode on"))
        .unwrap_or_else(|| panic!("{said:?}"));
    assert!(msg.contains("Save: Control S."), "{msg}");
    assert!(msg.contains("Finish: Control E."), "{msg}");
    assert!(
        app.status_text().contains("Save: Ctrl+S. Finish: Ctrl+E."),
        "{}",
        app.status_text()
    );
    app.dispatch(Command::Action(ActionId::ToggleEditMode));
    // The help list: items drawn with "Ctrl+O", the focused one spoken
    // with "Control O".
    log.clear();
    app.dispatch(Command::Action(ActionId::Help));
    let list = app.list_model().expect("the help list").clone();
    assert!(
        list.items
            .iter()
            .any(|i| i.contains("Open a document: Ctrl+O.")),
        "{:?}",
        list.items
    );
    app.dispatch(Command::ListKey(ListKey::Down));
    let said = heard(&log, "Open a document");
    assert!(
        said.iter()
            .any(|t| t.contains("Open a document: Control O.")),
        "{said:?}"
    );
    app.dispatch(Command::ListKey(ListKey::Escape));
    // The command palette: Tab completes and says the keys by name.
    log.clear();
    app.dispatch(Command::Action(ActionId::CommandPalette));
    for c in "next_sent".chars() {
        app.dispatch(Command::PromptKey(PromptKey::Char(c)));
    }
    app.dispatch(Command::PromptKey(PromptKey::Tab));
    let said = heard(&log, "next_sentence:");
    assert!(
        said.iter()
            .any(|t| t.contains("Alt period or Alt Down Arrow")),
        "{said:?}"
    );
    app.dispatch(Command::PromptKey(PromptKey::Escape));
    // Nothing spoken names a key in its written form; nothing shown
    // carries the marks.
    let written = ["Ctrl+", "Alt+", "Shift+", "Cmd+"];
    for t in log.texts() {
        assert!(!written.iter().any(|w| t.contains(w)), "spoken: {t}");
        assert!(!t.chars().any(is_mark), "spoken: {t:?}");
    }
    assert!(!app.status_text().chars().any(is_mark));
}

#[test]
fn lists_and_effects_never_carry_the_marks() {
    let (mut app, _log) = voiced("Text.\n");
    for a in [ActionId::Help, ActionId::KeyboardHelp] {
        let effects = app.dispatch(Command::Action(a));
        for e in &effects {
            if let textweaver_app::Effect::ShowList { items, .. } = e {
                assert!(
                    items.iter().all(|i| !i.chars().any(is_mark)),
                    "{a:?}: {items:?}"
                );
            }
        }
        let list = app.list_model().expect("a list");
        assert!(list.items.iter().all(|i| !i.chars().any(is_mark)));
        app.dispatch(Command::ListKey(ListKey::Escape));
    }
    let cands = app.palette_candidates("open");
    assert!(cands.iter().all(|(_, d)| !d.chars().any(is_mark)));
}
