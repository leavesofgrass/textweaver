use std::time::{Duration, Instant};

use textweaver_core::CharPos;
use textweaver_keymap::ActionId;
use textweaver_lexicon::i18n::bidi_problems;
use textweaver_store::{DocKey, Paths, Profiles, ReadingStats};
use textweaver_text::Document;

use crate::command::{Command, Confirm, Effect, PromptPurpose};
use crate::playback::Playback;
use crate::{App, AppConfig};

fn app_in(dir: Option<&std::path::Path>, text: Option<&str>, lang: &str) -> App {
    let mut config = AppConfig::for_tests();
    config.paths = dir.map(Paths::under);
    config.settings.interface.language = lang.into();
    let mut app = App::new(config);
    if let Some(t) = text {
        app.open_document(
            Document::from_plain_text(t),
            DocKey::untitled(1),
            "Notes".into(),
        );
    }
    app
}

fn list(effects: &[Effect]) -> Option<(String, Vec<String>)> {
    effects.iter().find_map(|e| match e {
        Effect::ShowList { title, items } => Some((title.clone(), items.clone())),
        _ => None,
    })
}

fn at(app: &mut App, pos: usize) {
    app.dispatch(Command::SetCursor(CharPos(pos)));
}

#[test]
fn defines_the_word_at_the_cursor_and_copies_a_sense() {
    let mut app = app_in(None, Some("The geese were running home."), "en");
    at(&mut app, 5);
    let e = app.dispatch(Command::Action(ActionId::DefineWord));
    let (title, items) = list(&e).expect("a list");
    assert!(title.starts_with("Definitions of geese, "), "{title}");
    assert!(title.ends_with("from Open English WordNet"), "{title}");
    assert!(
        app.status_text().starts_with(&title),
        "{}",
        app.status_text()
    );
    assert!(items[0].starts_with("Pronounced "), "{items:?}");
    assert!(items[1].starts_with("goose, noun, 1 of "), "{items:?}");
    app.dispatch(Command::Choose(1));
    assert_eq!(app.status_text(), "Copied.");
    assert_eq!(app.take_clipboard().as_deref(), Some(items[1].as_str()));

    // An inflected verb reaches its base form.
    at(&mut app, 16);
    let (_, items) = list(&app.dispatch(Command::Action(ActionId::DefineWord))).unwrap();
    assert!(
        items.iter().any(|i| i.starts_with("run, verb, 1 of ")),
        "{items:?}"
    );
    assert_eq!(items[0], "Pronounced RUN-ing.");
}

#[test]
fn asks_for_a_word_when_there_is_none() {
    let mut app = app_in(None, None, "en");
    let e = app.dispatch(Command::Action(ActionId::DefineWord));
    assert!(
        e.iter().any(|e| matches!(
            e,
            Effect::Prompt {
                purpose: PromptPurpose::DefineWord,
                ..
            }
        )),
        "{e:?}"
    );
    assert_eq!(app.status_text(), "Define which word?");
    let (title, _) = list(&app.dispatch(Command::Answer("Photosynthesis".into()))).unwrap();
    assert!(
        title.starts_with("Definitions of photosynthesis, 1 sense"),
        "{title}"
    );
    app.dispatch(Command::Action(ActionId::DefineWord));
    app.dispatch(Command::Answer("qwxzv".into()));
    assert_eq!(app.status_text(), "No definition found for qwxzv.");
}

#[test]
fn the_glossary_comes_first() {
    let dir = tempfile::tempdir().unwrap();
    let glossary = dir.path().join("biology.txt");
    std::fs::write(
        &glossary,
        "- **cell**: the smallest unit of life\nno separator\n",
    )
    .unwrap();
    let mut app = app_in(None, Some("Cells divide."), "en");
    app.settings.lexicon.glossary = Some(glossary.clone());
    let (title, items) = list(&app.dispatch(Command::Action(ActionId::DefineWord))).unwrap();
    assert!(title.ends_with("from your glossary"), "{title}");
    assert!(
        items
            .iter()
            .any(|i| i.starts_with("cell, 1 of 1: the smallest unit of life.")),
        "{items:?}"
    );
    // WordNet's senses follow.
    assert!(
        items.iter().any(|i| i.starts_with("cell, noun, 1 of ")),
        "{items:?}"
    );
    // An edited glossary is read again.
    std::fs::write(&glossary, "cell: a room in a prison\n").unwrap();
    let t = std::time::SystemTime::now() + Duration::from_secs(5);
    let f = std::fs::File::options()
        .write(true)
        .open(&glossary)
        .unwrap();
    f.set_modified(t).unwrap();
    let (_, items) = list(&app.dispatch(Command::Action(ActionId::DefineWord))).unwrap();
    assert!(
        items.iter().any(|i| i.contains("a room in a prison")),
        "{items:?}"
    );
}

#[test]
fn profiles_save_switch_rename_export_import_delete() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let mut app = app_in(Some(&home), None, "en");
    let (title, items) = list(&app.dispatch(Command::Action(ActionId::SettingsProfiles))).unwrap();
    assert_eq!(title, "Settings profiles, none saved yet");
    assert_eq!(
        items,
        [
            "Save the current settings as a new profile",
            "Import profiles from a file"
        ]
    );

    app.settings.speech.rate = textweaver_core::Rate::Wpm(200);
    app.dispatch(Command::Choose(0));
    assert_eq!(app.status_text(), "Name for the new profile");
    app.dispatch(Command::Answer("Study".into()));
    assert_eq!(app.status_text(), "Saved the current settings as Study.");

    app.settings.speech.rate = textweaver_core::Rate::Wpm(400);
    let (title, items) = list(&app.dispatch(Command::Action(ActionId::SettingsProfiles))).unwrap();
    assert_eq!(title, "Settings profiles, 1 profile");
    assert_eq!(
        items[0],
        "Study, in use: rate 200, theme galaxy, self voicing mode"
    );
    assert_eq!(items[2], "Save the current settings into Study");
    app.dispatch(Command::Choose(0));
    assert_eq!(app.status_text(), "Switched to Study.");
    assert_eq!(app.settings().speech.rate.wpm(), 200);
    app.wait_for_writes();
    // Saved at once, with the profiles file.
    let saved = textweaver_store::SettingsStore::new(Paths::under(&home))
        .load()
        .0;
    assert_eq!(saved.speech.rate.wpm(), 200);
    assert_eq!(
        Profiles::load(&Paths::under(&home)).unwrap().names(),
        ["Study"]
    );

    app.dispatch(Command::Action(ActionId::SettingsProfiles));
    app.dispatch(Command::RenameItem(0));
    assert_eq!(
        app.status_text(),
        "New name for the profile, Enter keeps it"
    );
    app.dispatch(Command::Answer("Exam".into()));
    assert_eq!(app.status_text(), "Renamed Study to Exam.");

    let file = dir.path().join("mine.json");
    let (_, items) = list(&app.dispatch(Command::Action(ActionId::SettingsProfiles))).unwrap();
    let export = items
        .iter()
        .position(|i| i == "Export all profiles to a file")
        .unwrap();
    app.dispatch(Command::Choose(export));
    app.dispatch(Command::Answer(file.display().to_string()));
    assert!(
        app.status_text().starts_with("Exported 1 profile to "),
        "{}",
        app.status_text()
    );

    app.dispatch(Command::Action(ActionId::SettingsProfiles));
    app.dispatch(Command::DeleteItem(0));
    assert_eq!(app.status_text(), "Delete the profile Exam? y or n");
    assert!(app.confirmation_pending());
    app.dispatch(Command::Confirm(Confirm::Repeat));
    assert_eq!(app.status_text(), "Delete the profile Exam? y or n");
    let e = app.dispatch(Command::Confirm(Confirm::Yes));
    assert!(!app.confirmation_pending());
    assert_eq!(list(&e).unwrap().0, "Settings profiles, none saved yet");

    let (_, items) = list(&app.dispatch(Command::Action(ActionId::SettingsProfiles))).unwrap();
    let import = items
        .iter()
        .position(|i| i == "Import profiles from a file")
        .unwrap();
    app.dispatch(Command::Choose(import));
    app.dispatch(Command::Answer(file.display().to_string()));
    assert_eq!(
        app.status_text(),
        "Imported 1 profile from mine.json: Exam."
    );
    app.wait_for_writes();
    assert_eq!(
        Profiles::load(&Paths::under(&home)).unwrap().names(),
        ["Exam"]
    );
}

#[test]
fn reading_time_is_counted_and_listed() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let mut app = app_in(Some(&home), Some("One two three four five six."), "en");
    let (_, items) = list(&app.dispatch(Command::Action(ActionId::ReadingStatistics))).unwrap();
    assert_eq!(
        items[0],
        "No reading recorded yet. Time is counted while textweaver reads aloud."
    );
    assert_eq!(items[1], "This document has not been read aloud yet.");
    assert_eq!(
        items.last().unwrap(),
        "Statistics are on. Enter turns them off."
    );
    app.dispatch(Command::Cancel);

    // Four seconds of reading, then a pause that does not count.
    let t0 = Instant::now();
    app.playback = Playback::Reading;
    for s in [0, 2, 4] {
        app.stats_tick(t0 + Duration::from_secs(s));
    }
    app.playback = Playback::Idle;
    app.stats_tick(t0 + Duration::from_secs(60));
    let (_, items) = list(&app.dispatch(Command::Action(ActionId::ReadingStatistics))).unwrap();
    assert_eq!(
        items[0],
        "4 seconds read in all, in 1 session, over 1 document."
    );
    assert_eq!(
        items[1],
        "This document: 4 seconds read, furthest point 0 percent, 1 session."
    );
    let stats = ReadingStats::load(&Paths::under(&home)).unwrap();
    assert_eq!(stats.documents.len(), 1);

    // Turning statistics off stops the counting.
    let toggle = items.len() - 1;
    app.dispatch(Command::Choose(toggle));
    assert!(app.status_text().starts_with("Reading statistics are off."));
    assert!(!app.settings().stats.enabled);
    app.playback = Playback::Reading;
    app.stats_tick(t0 + Duration::from_secs(61));
    app.stats_tick(t0 + Duration::from_secs(63));
    app.playback = Playback::Idle;
    app.stats_flush();
    app.wait_for_writes();
    let stats = ReadingStats::load(&Paths::under(&home)).unwrap();
    assert_eq!(stats.total_seconds(), 4.0);
}

#[test]
fn every_study_message_comes_from_the_catalog() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(Some(dir.path()), Some("The geese flew."), "en-XA");
    at(&mut app, 5);
    let (title, items) = list(&app.dispatch(Command::Action(ActionId::DefineWord))).unwrap();
    for s in std::iter::once(&title).chain(&items) {
        assert!(s.starts_with('⟦') && s.ends_with('⟧'), "{s}");
    }
    assert!(app.status_text().starts_with('⟦'), "{}", app.status_text());
    for a in [ActionId::SettingsProfiles, ActionId::ReadingStatistics] {
        let (title, items) = list(&app.dispatch(Command::Action(a))).unwrap();
        for s in std::iter::once(&title).chain(&items) {
            assert!(s.starts_with('⟦') && s.ends_with('⟧'), "{s}");
        }
        app.dispatch(Command::Cancel);
    }
    app.dispatch(Command::Action(ActionId::DefineWord));
    app.dispatch(Command::Cancel);
    let mut none = app_in(None, None, "en-XA");
    none.dispatch(Command::Action(ActionId::DefineWord));
    assert!(
        none.status_text().starts_with('⟦'),
        "{}",
        none.status_text()
    );
}

#[test]
fn right_to_left_messages_keep_their_direction_marks_balanced() {
    let mut app = app_in(None, Some("The geese flew."), "ar-XB");
    assert_eq!(
        app.catalog().direction(),
        textweaver_lexicon::i18n::Direction::RightToLeft
    );
    at(&mut app, 5);
    let (title, items) = list(&app.dispatch(Command::Action(ActionId::DefineWord))).unwrap();
    for s in std::iter::once(&title).chain(&items) {
        assert!(bidi_problems(s).is_empty(), "{s:?}");
        assert!(s.contains('\u{2068}'), "values are isolated: {s:?}");
    }
    assert!(bidi_problems(app.status_text()).is_empty());
}

#[test]
fn an_unknown_language_falls_back_to_english() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    std::fs::create_dir_all(paths.locales_dir()).unwrap();
    std::fs::write(
        paths.locales_dir().join("it.ftl"),
        "define-copied = Copiato.\n",
    )
    .unwrap();
    let app = app_in(Some(dir.path()), None, "it");
    assert_eq!(app.catalog().tr("define-copied"), "Copiato.");
    assert_eq!(
        app.catalog().tr("define-nothing-here"),
        "There is no word at the cursor."
    );
    let app = app_in(None, None, "xx");
    assert_eq!(app.catalog().lang(), "en");
}
