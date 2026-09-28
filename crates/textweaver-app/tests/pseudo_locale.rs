//! The pseudo-locale check (Wave 4, W4d; ADR-0030): in `en-XA` every word
//! textweaver says comes out accented and in `⟦ ⟧` brackets, so a
//! message that never went through the catalog shows as plain English.
//! Every action runs once over a document written in Greek (so the
//! document's own words never look like English), and every message,
//! list title and item, and prompt label is checked. In `ar-XB`, the
//! right-to-left pseudo-locale, every message must close the direction
//! marks it opens.
//!
//! `scripts/dev-check.sh` and `.ps1` run this test.

use std::sync::{Arc, Mutex};

use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::keymap::ActionId;
use textweaver_app::lexicon::i18n::bidi_problems;
use textweaver_app::testing::recording_service;
use textweaver_app::{App, AppConfig, Command, Effect};

#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Announcer for Said {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}

/// Actions left out: they quit, or reach outside the test (a browser, the
/// clipboard, files written beside the document).
const SKIPPED: &[ActionId] = &[
    ActionId::Quit,
    ActionId::PreviewInBrowser,
    ActionId::ExportHtml,
    ActionId::ExportPdf,
    ActionId::ExportDocx,
    ActionId::ExportEpub,
    ActionId::ExportBrf,
    ActionId::ExportStudySheet,
    ActionId::ListenRendered,
    ActionId::Copy,
    ActionId::Cut,
    ActionId::Paste,
    ActionId::FollowLink,
];

/// Whole messages that are data: the recording backend's voice names,
/// focused in the voice list.
const DATA: &[&str] = &["Test voice", "Second voice"];

/// Words allowed outside brackets: written key names, and the test
/// backend's name.
const ALLOWED: &[&str] = &[
    "ctrl",
    "alt",
    "shift",
    "cmd",
    "space",
    "enter",
    "escape",
    "tab",
    "backspace",
    "delete",
    "insert",
    "home",
    "end",
    "pageup",
    "pagedown",
    "up",
    "down",
    "left",
    "right",
    "test",
    "recording",
];

const DOC: &str = "# Αρχή\n\nΑλφα βήτα γάμμα. Δέλτα έψιλον ζήτα.\n\n## Μέση\n\n- ήτα\n- θήτα\n\nΙώτα κάππα λάμδα.\n";

fn run(lang: &str) -> Vec<String> {
    let said = Said::default();
    let (speech, _log) = recording_service().unwrap();
    let mut config = AppConfig {
        speech,
        announcer: Box::new(said.clone()),
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    };
    config.settings.interface.language = lang.into();
    let mut app = App::new(config);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ωμέγα.md");
    std::fs::write(&path, DOC).unwrap();
    app.open(&path).unwrap();
    let mut shown: Vec<String> = Vec::new();
    for &a in ActionId::ALL {
        if SKIPPED.contains(&a) {
            continue;
        }
        let effects = app.dispatch(Command::Action(a));
        collect(&effects, &mut shown);
        shown.push(app.status_text().to_owned());
        // Close whatever opened: a question, a list, a prompt, a mode.
        if app.pending_confirmation().is_some() {
            app.dispatch(Command::Confirm(textweaver_app::Confirm::No));
        }
        let effects = app.dispatch(Command::Cancel);
        collect(&effects, &mut shown);
        if a == ActionId::ToggleEditMode || a == ActionId::SpeechCursorToggle {
            let effects = app.dispatch(Command::Action(a));
            collect(&effects, &mut shown);
        }
        app.wait_for_speech_thread();
        app.poll_speech();
    }
    let mut all = said.0.lock().unwrap().clone();
    all.extend(shown);
    all.retain(|s| !s.trim().is_empty());
    all
}

fn collect(effects: &[Effect], out: &mut Vec<String>) {
    for e in effects {
        match e {
            Effect::ShowList { title, items } => {
                out.push(title.clone());
                out.extend(items.iter().cloned());
            }
            Effect::Prompt { label, .. } => out.push(label.clone()),
            _ => {}
        }
    }
}

/// English words left outside the brackets in `s`.
fn plain_english(s: &str) -> Vec<String> {
    let mut outside = String::new();
    let mut depth = 0usize;
    for c in s.chars() {
        match c {
            '⟦' => depth += 1,
            '⟧' => depth = depth.saturating_sub(1),
            c if depth == 0 => outside.push(c),
            _ => {}
        }
    }
    outside
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|w| w.len() >= 3)
        .map(str::to_lowercase)
        .filter(|w| !ALLOWED.contains(&w.as_str()))
        .collect()
}

#[test]
fn every_message_is_in_the_accented_pseudo_locale() {
    let mut problems: Vec<String> = Vec::new();
    for s in run("en-XA") {
        if DATA.contains(&s.as_str()) {
            continue;
        }
        let words = plain_english(&s);
        if !words.is_empty() {
            problems.push(format!("{s:?}: {}", words.join(" ")));
        }
    }
    problems.sort();
    problems.dedup();
    assert!(
        problems.is_empty(),
        "plain English outside the catalog in {} messages:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

#[test]
fn every_message_closes_its_direction_marks() {
    let mut problems: Vec<String> = Vec::new();
    for s in run("ar-XB") {
        let p = bidi_problems(&s);
        if !p.is_empty() {
            problems.push(format!("{s:?}: {p:?}"));
        }
    }
    problems.sort();
    problems.dedup();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
