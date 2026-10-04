//! Choosers for every prompt for a path (W8a-f): what the GUI's system
//! chooser shows for each prompt, the folder choices it takes over, and
//! the browse key that fills a prompt from the file browser.
//!
//! The browse key is asked of the app (`path_prompt::browse_key`), and the
//! browser's keys are the list's own (`ListKey`), so no chord is written
//! into a test.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::keymap::ActionId;
use textweaver_app::path_prompt::{PathKind, browse_key};
use textweaver_app::store::Paths;
use textweaver_app::testing::recording_service;
use textweaver_app::{App, AppConfig, Command, Effect, ListKey, PromptKey, PromptPurpose};

#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Said {
    fn all(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
    fn any(&self, needle: &str) -> bool {
        self.all().iter().any(|s| s.contains(needle))
    }
}

impl Announcer for Said {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}

/// A reader with its own home (so settings and sync can be used), and a
/// document open in a folder of its own holding a settings file and a
/// reference file.
struct Rig {
    app: App,
    said: Said,
    folder: PathBuf,
    _dir: tempfile::TempDir,
}

fn rig() -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("course");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(
        folder.join("notes.md"),
        "# Notes\n\nCrows remember faces.\n",
    )
    .unwrap();
    std::fs::write(folder.join("mine.toml"), "[speech]\n").unwrap();
    std::fs::write(folder.join("refs.bib"), "@book{ada, title={A}}\n").unwrap();
    let said = Said::default();
    let (speech, _log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        paths: Some(Paths::under(&dir.path().join("home"))),
        announcer: Box::new(said.clone()),
        ..AppConfig::for_tests()
    });
    app.open(&folder.join("notes.md")).unwrap();
    // The folder as the browser shows it (no `\\?\` prefix on Windows).
    let folder = app
        .session()
        .and_then(|s| s.doc.meta.path.clone())
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap();
    Rig {
        app,
        said,
        folder,
        _dir: dir,
    }
}

impl Rig {
    fn key(&mut self, k: ListKey) -> Vec<Effect> {
        self.app.dispatch(Command::ListKey(k))
    }
    fn prompt_key(&mut self, k: PromptKey) -> Vec<Effect> {
        self.app.dispatch(Command::PromptKey(k))
    }
    fn type_str(&mut self, s: &str) {
        for c in s.chars() {
            self.prompt_key(PromptKey::Char(c));
        }
    }
    fn items(&self) -> Vec<String> {
        self.app
            .list_model()
            .map(|l| l.items.clone())
            .unwrap_or_default()
    }
    /// Moves the browser's focus to the row starting with `name`.
    fn focus(&mut self, name: &str) {
        let want = format!("{name},");
        let n = self
            .items()
            .iter()
            .position(|i| i.starts_with(&want))
            .unwrap_or_else(|| panic!("no row {name} in {:?}", self.items()));
        self.app.dispatch(Command::ListFocus(n));
    }
    fn prompt(&self) -> (PromptPurpose, String) {
        let m = self.app.prompt_model().expect("a prompt");
        (m.purpose, m.text())
    }
}

#[test]
fn every_prompt_for_a_path_has_a_chooser_and_no_other() {
    let r = rig();
    let spec = |p| r.app.path_prompt_spec(p).expect("a chooser");

    let image = spec(PromptPurpose::ImagePath);
    assert_eq!(image.title, "Insert an image");
    assert_eq!(image.filter_name, "Images");
    assert!(image.extensions.iter().any(|e| e == "png"));
    assert!(matches!(image.kind, PathKind::Read(_)));
    assert_eq!(image.folder.as_deref(), Some(r.folder.as_path()));

    let refs = spec(PromptPurpose::ImportReferences);
    assert_eq!(refs.title, "Import references");
    assert!(refs.extensions.iter().any(|e| e == "bib"));
    assert_eq!(refs.folder.as_deref(), Some(r.folder.as_path()));

    let import = spec(PromptPurpose::ImportProfiles);
    assert_eq!(import.title, "Import profiles");
    assert_eq!(import.extensions, vec!["toml", "json"]);
    assert_eq!(import.file_name, None);

    let export = spec(PromptPurpose::ExportProfiles);
    assert!(matches!(export.kind, PathKind::Write(_)));
    assert_eq!(
        export.file_name.as_deref(),
        Some("textweaver-profiles.toml")
    );

    let settings = spec(PromptPurpose::ExportSettings);
    assert_eq!(settings.title, "Export settings");
    assert_eq!(
        settings.file_name.as_deref(),
        Some("textweaver-settings.toml")
    );
    assert!(matches!(
        spec(PromptPurpose::ImportSettings).kind,
        PathKind::Read(_)
    ));

    // Open lists the documents textweaver reads (no extensions of its own).
    let open = spec(PromptPurpose::Open);
    assert!(open.extensions.is_empty());
    assert_eq!(open.folder.as_deref(), Some(r.folder.as_path()));

    for p in [
        PromptPurpose::Find,
        PromptPurpose::GoTo,
        PromptPurpose::CommandPalette,
        PromptPurpose::NoteText,
        PromptPurpose::SyncComputerName,
    ] {
        assert_eq!(r.app.path_prompt_spec(p), None, "{p:?}");
    }
}

#[test]
fn save_as_offers_the_document_name_in_its_folder() {
    let mut r = rig();
    r.app.dispatch(Command::Action(ActionId::ToggleEditMode));
    let effects = r.app.dispatch(Command::Action(ActionId::SaveAs));
    assert!(
        matches!(
            effects.first(),
            Some(Effect::Prompt {
                purpose: PromptPurpose::SaveAs,
                ..
            })
        ),
        "{effects:?}"
    );
    let spec = r.app.path_prompt_spec(PromptPurpose::SaveAs).unwrap();
    assert!(matches!(spec.kind, PathKind::Write(_)));
    assert_eq!(spec.title, "Save as");
    assert_eq!(spec.file_name.as_deref(), Some("notes.md"));
    assert_eq!(spec.extensions, vec!["md"]);
    assert_eq!(spec.filter_name, "MD files");
    assert_eq!(spec.folder.as_deref(), Some(r.folder.as_path()));
}

#[test]
fn the_browse_key_fills_the_prompt_with_the_file_chosen() {
    let mut r = rig();
    r.app.dispatch(Command::Action(ActionId::ImportSettings));
    assert_eq!(r.prompt().0, PromptPurpose::ImportSettings);
    r.type_str("x");
    r.prompt_key(PromptKey::Browse);
    // The browser, on the places, says what it chooses for.
    assert!(r.app.prompt_model().is_none());
    assert!(
        r.said.any("Choose the file: Import settings from file"),
        "{:?}",
        r.said.all()
    );
    // The document's folder is focused: enter it, and choose the file.
    r.key(ListKey::Enter);
    r.focus("mine.toml");
    let effects = r.key(ListKey::Enter);
    assert!(
        matches!(
            effects.first(),
            Some(Effect::Prompt {
                purpose: PromptPurpose::ImportSettings,
                ..
            })
        ),
        "{effects:?}"
    );
    let (purpose, text) = r.prompt();
    assert_eq!(purpose, PromptPurpose::ImportSettings);
    assert_eq!(PathBuf::from(text), r.folder.join("mine.toml"));
    assert!(r.app.prompt_model().unwrap().from_browser());
    assert!(
        r.said.any("mine.toml chosen. Enter confirms."),
        "{:?}",
        r.said.all()
    );
    // Only the extensions the prompt reads are listed: no Markdown.
    assert!(r.app.list_model().is_none());
}

#[test]
fn escape_in_the_browser_goes_back_to_the_prompt_as_it_was() {
    let mut r = rig();
    r.app.dispatch(Command::Action(ActionId::ImportSettings));
    r.type_str("abc");
    r.prompt_key(PromptKey::Browse);
    assert!(r.app.list_model().is_some());
    // The terminal's Escape (the list's own key).
    r.key(ListKey::Escape);
    assert_eq!(
        r.prompt(),
        (PromptPurpose::ImportSettings, "abc".to_owned())
    );
    assert!(r.app.list_model().is_none());
    // A GUI list's Escape (Command::Cancel) does the same.
    r.prompt_key(PromptKey::Browse);
    r.key(ListKey::Enter);
    r.app.dispatch(Command::Cancel);
    assert_eq!(
        r.prompt(),
        (PromptPurpose::ImportSettings, "abc".to_owned())
    );
    // And Escape in the prompt still cancels it.
    r.prompt_key(PromptKey::Escape);
    assert!(r.app.prompt_model().is_none());
    assert!(r.app.list_model().is_none());
}

#[test]
fn a_file_to_write_takes_a_folder_and_keeps_the_name() {
    let mut r = rig();
    // Nothing typed: the name offered goes in the folder chosen.
    r.app.dispatch(Command::Action(ActionId::ExportSettings));
    r.prompt_key(PromptKey::Browse);
    assert!(
        r.said.any("Choose the folder: Export settings"),
        "{:?}",
        r.said.all()
    );
    // The places, with the document's folder focused: choose it.
    r.key(ListKey::ChooseHere);
    let (purpose, text) = r.prompt();
    assert_eq!(purpose, PromptPurpose::ExportSettings);
    assert_eq!(
        PathBuf::from(text),
        r.folder.join("textweaver-settings.toml")
    );
    // A name typed first is kept.
    r.prompt_key(PromptKey::Escape);
    r.app.dispatch(Command::Action(ActionId::ExportSettings));
    r.type_str("week1.json");
    r.prompt_key(PromptKey::Browse);
    r.key(ListKey::ChooseHere);
    assert_eq!(PathBuf::from(r.prompt().1), r.folder.join("week1.json"));
    // Enter answers the prompt as typed: the settings are written there.
    r.prompt_key(PromptKey::Enter);
    assert!(r.folder.join("week1.json").is_file());
}

#[test]
fn the_browse_key_is_named_when_a_frontend_has_one() {
    let mut r = rig();
    r.app.dispatch(Command::Action(ActionId::ImportSettings));
    assert!(!r.said.any("to browse"), "{:?}", r.said.all());
    r.prompt_key(PromptKey::Escape);
    r.app.set_prompt_browse_key(Some(browse_key()));
    r.app.dispatch(Command::Action(ActionId::ImportSettings));
    let hint = format!("{} to browse.", browse_key());
    assert!(
        r.said.any(&format!("Import settings from file. {hint}")),
        "{:?}",
        r.said.all()
    );
    // The hint fits a 40-cell Braille line.
    assert!(hint.chars().count() <= 40);
    // A prompt that is not for a path does not name it.
    r.prompt_key(PromptKey::Escape);
    r.app.dispatch(Command::Action(ActionId::Find));
    assert_eq!(r.prompt().0, PromptPurpose::Find);
    assert!(!r.said.all().last().unwrap().contains("to browse"));
    // There the key does nothing: the prompt stays.
    r.type_str("crow");
    r.prompt_key(PromptKey::Browse);
    assert_eq!(r.prompt(), (PromptPurpose::Find, "crow".to_owned()));
    assert!(r.app.list_model().is_none());
}

#[test]
fn a_folder_choice_waits_for_the_system_chooser() {
    let mut r = rig();
    r.app.dispatch(Command::Action(ActionId::SyncSetup));
    let choice = r.app.folder_choice().expect("a folder choice");
    assert_eq!(choice.title, "Choose the sync folder");
    assert_eq!(choice.folder.as_deref(), Some(r.folder.as_path()));
    // Taken once: the browser behind it is the fallback, not taken over.
    assert!(r.app.take_folder_choice().is_some());
    assert_eq!(r.app.folder_choice(), None);
    // The folder chosen goes to the command: the computer's name is next.
    let sync = r.folder.join("sync");
    std::fs::create_dir(&sync).unwrap();
    r.app.dispatch(Command::PathChosen(Some(sync)));
    assert_eq!(r.prompt().0, PromptPurpose::SyncComputerName);
    assert!(r.app.list_model().is_none());
}

#[test]
fn a_folder_chooser_closed_cancels_the_choice() {
    let mut r = rig();
    r.app.dispatch(Command::Action(ActionId::SyncSetup));
    assert!(r.app.take_folder_choice().is_some());
    r.app.dispatch(Command::PathChosen(None));
    assert!(r.app.list_model().is_none());
    assert!(r.app.prompt_model().is_none());
    assert!(r.said.any("Canceled."), "{:?}", r.said.all());
    // With nothing waiting, a path chosen does nothing.
    r.app.dispatch(Command::PathChosen(Some(r.folder.clone())));
    assert!(r.app.prompt_model().is_none());
}

#[test]
fn the_browser_from_a_prompt_is_not_a_folder_choice() {
    let mut r = rig();
    r.app.dispatch(Command::Action(ActionId::ExportSettings));
    r.prompt_key(PromptKey::Browse);
    // The file browser was asked for: the GUI shows it, not its chooser.
    assert_eq!(r.app.folder_choice(), None);
    // Browse files on its own is not a choice either.
    r.key(ListKey::Escape);
    r.prompt_key(PromptKey::Escape);
    r.app.dispatch(Command::Action(ActionId::BrowseFiles));
    assert_eq!(r.app.folder_choice(), None);
}

/// "Add a folder to the library" (W9a-c), a command in both frontends:
/// the browser opens on the places, the folder chosen joins the library
/// folders once, and a second time says it is already there.
#[test]
fn add_a_folder_to_the_library_through_the_browser() {
    let mut r = rig();
    r.app.dispatch(Command::Action(ActionId::AddLibraryFolder));
    assert!(
        r.said.any("Choose the folder to add to the library"),
        "{:?}",
        r.said.all()
    );
    // The places, with the document's folder focused: choose it.
    r.key(ListKey::ChooseHere);
    assert!(
        r.said.any("Added course to the library."),
        "{:?}",
        r.said.all()
    );
    assert_eq!(r.app.settings().library.folders, vec![r.folder.clone()]);
    r.app.dispatch(Command::Action(ActionId::AddLibraryFolder));
    r.key(ListKey::ChooseHere);
    assert!(r.said.any("course is already in the library."));
    assert_eq!(r.app.settings().library.folders.len(), 1);
}
