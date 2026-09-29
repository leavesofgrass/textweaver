//! Star's lesson: a stored setting must work. This test fails when a field
//! of a settings struct in `src/settings.rs` is read nowhere in the
//! workspace's sources and is not listed in
//! [`RESERVED_SETTINGS`](textweaver_store::RESERVED_SETTINGS) with a reason.
//!
//! "Read" is found by text search, so it is a heuristic: a field counts as
//! read when some source outside the settings files themselves says
//! `section.field` (`settings.highlight.color`, `sp.eci.library`), or uses
//! `alias.field` after binding the section (`let e = &settings.editing;`)
//! or taking it as a parameter (`h: &HighlightSettings`). The settings
//! files, import and export, the Star migration, and `tw settings` touch
//! every field and do not count.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use regex::Regex;
use textweaver_store::RESERVED_SETTINGS;

/// Settings structs and the section each one is.
const SECTIONS: &[(&str, &str)] = &[
    ("SpeechSettings", "speech"),
    ("EciSettings", "eci"),
    ("SapiSettings", "sapi"),
    ("AppleSettings", "apple"),
    ("HighlightSettings", "highlight"),
    ("NormalizationSettings", "normalization"),
    ("CommunityLexiconSettings", "community_lexicon"),
    ("ExportSettings", "export"),
    ("BrailleSettings", "braille"),
    ("ReadingSettings", "reading"),
    ("DisplaySettings", "display"),
    ("EditingSettings", "editing"),
    ("LibrarySettings", "library"),
    ("KeyboardSettings", "keyboard"),
    ("AccessibilitySettings", "accessibility"),
    ("ReadingAidsSettings", "reading_aids"),
    ("PreviewSettings", "preview"),
    ("LexiconSettings", "lexicon"),
    ("StatsSettings", "stats"),
    ("InterfaceSettings", "interface"),
    ("GuiSettings", "gui"),
];

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// `(struct, field)` for every settings field, from the source.
fn fields(settings_rs: &str) -> Vec<(String, String)> {
    let start = Regex::new(r"^pub struct (\w+) \{").unwrap();
    let field = Regex::new(r"^    pub (\w+):").unwrap();
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    for line in settings_rs.lines() {
        if let Some(c) = start.captures(line) {
            current = Some(c[1].to_owned());
            continue;
        }
        if line.starts_with('}') {
            current = None;
            continue;
        }
        if let (Some(s), Some(c)) = (&current, field.captures(line)) {
            out.push((s.clone(), c[1].to_owned()));
        }
    }
    out
}

fn sources(root: &Path, out: &mut Vec<(PathBuf, String)>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for e in entries.filter_map(Result::ok) {
        let p = e.path();
        if p.is_dir() {
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            if name == "target" || name.starts_with('.') || name == "migrate" {
                continue;
            }
            sources(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            let s = p.to_string_lossy().replace('\\', "/");
            let excluded = s.ends_with("textweaver-store/src/settings.rs")
                || s.contains("textweaver-store/src/settings_io")
                || s.ends_with("textweaver-cli/src/cmd/settings.rs")
                || s.contains("/tests/");
            if !excluded && let Ok(text) = std::fs::read_to_string(&p) {
                out.push((p, text));
            }
        }
    }
}

fn is_read(sources: &[(PathBuf, String)], strukt: &str, section: &str, field: &str) -> bool {
    let direct = Regex::new(&format!(r"\b{section}\s*\.\s*{field}\b")).unwrap();
    let bind = Regex::new(&format!(
        r"let\s+(?:mut\s+)?(\w+)\s*=\s*&?(?:mut\s+)?[\w.]*\b{section}\s*;"
    ))
    .unwrap();
    let param = Regex::new(&format!(r"(\w+)\s*:\s*&(?:mut\s+)?(?:[\w:]*::)?{strukt}\b")).unwrap();
    sources.iter().any(|(_, text)| {
        if direct.is_match(text) {
            return true;
        }
        let aliases: Vec<String> = bind
            .captures_iter(text)
            .chain(param.captures_iter(text))
            .map(|c| c[1].to_owned())
            .collect();
        aliases.iter().any(|a| {
            Regex::new(&format!(r"\b{a}\s*\.\s*{field}\b"))
                .unwrap()
                .is_match(text)
        })
    })
}

#[test]
fn every_setting_is_read_or_reserved() {
    let root = workspace();
    let settings_rs =
        std::fs::read_to_string(root.join("crates/textweaver-store/src/settings.rs")).unwrap();
    let sections: BTreeMap<&str, &str> = SECTIONS.iter().copied().collect();
    let mut srcs = Vec::new();
    sources(&root.join("crates"), &mut srcs);
    assert!(srcs.len() > 100, "found only {} sources", srcs.len());

    let mut unread = Vec::new();
    let mut unmapped = Vec::new();
    for (strukt, field) in fields(&settings_rs) {
        if field == "extra" || strukt == "Settings" || strukt == "SettingsLoad" {
            continue;
        }
        if !strukt.ends_with("Settings") {
            continue;
        }
        let Some(section) = sections.get(strukt.as_str()) else {
            unmapped.push(strukt.clone());
            continue;
        };
        // A field that is itself a section is checked field by field.
        if sections.values().any(|s| *s == field) {
            continue;
        }
        let key = format!("{section}.{field}");
        let reserved = RESERVED_SETTINGS.iter().any(|(k, _)| *k == key);
        let read = is_read(&srcs, &strukt, section, &field);
        if !read && !reserved {
            unread.push(key);
        }
    }
    unmapped.sort();
    unmapped.dedup();
    assert!(
        unmapped.is_empty(),
        "add these settings structs to SECTIONS in this test: {unmapped:?}"
    );
    assert!(
        unread.is_empty(),
        "these settings are stored but never read; use them, or list them in \
         RESERVED_SETTINGS (crates/textweaver-store/src/settings.rs) with a reason: {unread:?}"
    );
}

/// Every reserved entry names a real setting and gives a reason, and a
/// setting that is read is not also reserved (a stale entry).
#[test]
fn reserved_settings_are_real_and_still_unread() {
    let root = workspace();
    let settings_rs =
        std::fs::read_to_string(root.join("crates/textweaver-store/src/settings.rs")).unwrap();
    let sections: BTreeMap<&str, &str> = SECTIONS.iter().copied().collect();
    let known: Vec<(String, String, String)> = fields(&settings_rs)
        .into_iter()
        .filter_map(|(s, f)| {
            let section = sections.get(s.as_str())?;
            Some((format!("{section}.{f}"), s, f))
        })
        .collect();
    let mut srcs = Vec::new();
    sources(&root.join("crates"), &mut srcs);
    for (key, reason) in RESERVED_SETTINGS {
        assert!(!reason.trim().is_empty(), "{key} needs a reason");
        let Some((_, strukt, field)) = known.iter().find(|(k, _, _)| k == key) else {
            panic!("RESERVED_SETTINGS lists {key}, which is not a setting");
        };
        let section = sections[strukt.as_str()];
        assert!(
            !is_read(&srcs, strukt, section, field),
            "{key} is read now; take it off RESERVED_SETTINGS"
        );
    }
}
