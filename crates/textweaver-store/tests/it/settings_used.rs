//! Star's lesson: a stored setting must work. This test fails when a field
//! of a settings struct in `src/settings.rs` or `src/reading_aids.rs` is
//! read nowhere in the workspace's sources and is not listed in
//! [`RESERVED_SETTINGS`](textweaver_store::RESERVED_SETTINGS) with a reason.
//! Since Wave 5 (W5y) the reading aids' own tables (`[reading_aids.font]`
//! and the rest) are checked field by field too: `fetch_missing` was stored,
//! shown on the settings screen, and read by nothing that acted on it.
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
    ("DectalkSettings", "dectalk"),
    ("PiperSettings", "piper"),
    ("HighlightSettings", "highlight"),
    ("NormalizationSettings", "normalization"),
    ("CommunityLexiconSettings", "community_lexicon"),
    ("MedicalLexiconSettings", "medical_lexicon"),
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
    ("SummarySettings", "summary"),
    ("DictationSettings", "dictation"),
    ("InterfaceSettings", "interface"),
    ("GuiSettings", "gui"),
    ("ColorSettings", "colors"),
    ("SyncSettings", "sync"),
    ("ComponentsSettings", "components"),
    // `src/reading_aids.rs`.
    ("RsvpSettings", "rsvp"),
    ("BionicOptions", "bionic_options"),
    ("TextSpacing", "spacing"),
    ("FontSettings", "font"),
    ("RulerSettings", "ruler"),
    ("SyllableOptions", "syllable_options"),
];

/// The files that define settings structs; they touch every field, so they
/// do not count as readers.
const SETTINGS_FILES: [&str; 2] = [
    "crates/textweaver-store/src/settings.rs",
    "crates/textweaver-store/src/reading_aids.rs",
];

/// `(struct, field)` for every settings field in [`SETTINGS_FILES`].
fn all_fields(root: &Path) -> Vec<(String, String)> {
    SETTINGS_FILES
        .iter()
        .flat_map(|f| fields(&std::fs::read_to_string(root.join(f)).unwrap()))
        .collect()
}

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
            let excluded = SETTINGS_FILES.iter().any(|f| s.ends_with(f))
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
    let sections: BTreeMap<&str, &str> = SECTIONS.iter().copied().collect();
    let mut srcs = Vec::new();
    sources(&root.join("crates"), &mut srcs);
    assert!(srcs.len() > 100, "found only {} sources", srcs.len());

    let mut unread = Vec::new();
    let mut unmapped = Vec::new();
    for (strukt, field) in all_fields(&root) {
        if field == "extra" || strukt == "Settings" || strukt == "SettingsLoad" {
            continue;
        }
        if !strukt.ends_with("Settings") && !sections.contains_key(strukt.as_str()) {
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
    let sections: BTreeMap<&str, &str> = SECTIONS.iter().copied().collect();
    let known: Vec<(String, String, String)> = all_fields(&root)
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

/// The check itself: a field only its own settings file mentions is
/// caught, and one read through the section or an alias is not.
#[test]
fn the_check_catches_a_setting_no_reader_touches() {
    let settings =
        "pub struct FontSettings {\n    pub family: String,\n    pub fetch_missing: bool,\n}\n";
    let fields = fields(settings);
    assert_eq!(
        fields,
        [
            ("FontSettings".to_owned(), "family".to_owned()),
            ("FontSettings".to_owned(), "fetch_missing".to_owned()),
        ]
    );
    let reader = vec![(
        PathBuf::from("reader.rs"),
        "fn f(s: &Settings) { let font = &s.reading_aids.font; use_it(&font.family); }".to_owned(),
    )];
    assert!(is_read(&reader, "FontSettings", "font", "family"));
    assert!(!is_read(&reader, "FontSettings", "font", "fetch_missing"));
    let param = vec![(
        PathBuf::from("reader.rs"),
        "fn g(f: &FontSettings) -> bool { f.fetch_missing }".to_owned(),
    )];
    assert!(is_read(&param, "FontSettings", "font", "fetch_missing"));
}
