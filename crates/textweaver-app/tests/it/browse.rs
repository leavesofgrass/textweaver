//! The file browser and chooser (Wave 6, W6f; ADR-0045): places first,
//! folders and archives entered and left, rows meaning first, the preview
//! on a helper thread, readable files only, sorting, choosing a folder for
//! another command, and hostile archives refused with a sentence.
//!
//! Keys are the list's own (`ListKey`), and the browser's keys are asked of
//! the app (`BrowseKey::chord` with the keymap's command modifier), never
//! written into a test.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use textweaver_app::browse::{BrowseKey, Location, SortBy, command_modifier};
use textweaver_app::keymap::ActionId;
use textweaver_app::{App, AppConfig, Command, Effect, ListKey};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/f")
        .canonicalize()
        .unwrap()
}

fn w3d() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/w3d")
        .canonicalize()
        .unwrap()
}

/// An app with `doc` open, so its folder is the first place.
fn app_with(doc: &Path) -> App {
    let mut app = App::new(AppConfig::for_tests());
    app.open(doc).unwrap();
    app
}

fn key(app: &mut App, k: ListKey) {
    app.dispatch(Command::ListKey(k));
}

fn items(app: &App) -> Vec<String> {
    app.list_model()
        .map(|l| l.items.clone())
        .unwrap_or_default()
}

/// Moves the focus to the row starting with `name` and a comma.
fn focus(app: &mut App, name: &str) {
    let want = format!("{name},");
    let n = items(app)
        .iter()
        .position(|i| i.starts_with(&want))
        .unwrap_or_else(|| panic!("no row {name} in {:?}", items(app)));
    app.dispatch(Command::ListFocus(n));
}

/// Opens the browser and enters the open document's folder.
fn browse_doc_folder(app: &mut App) {
    app.dispatch(Command::Action(ActionId::BrowseFiles));
    assert_eq!(app.browse_location(), Some(&Location::Places));
    // The document's folder is the first place, and focused.
    assert_eq!(app.list_model().unwrap().selected, 0);
    key(app, ListKey::Enter);
}

#[test]
fn places_come_first_with_the_document_folder_focused() {
    let mut app = app_with(&fixtures().join("readme.md"));
    app.dispatch(Command::Action(ActionId::BrowseFiles));
    let rows = items(&app);
    assert_eq!(rows[0], "f, the document's folder");
    assert!(
        app.status_text().starts_with("Places, "),
        "{}",
        app.status_text()
    );
    // Said position last, so the name leads the Braille line.
    let said = app.list_model().unwrap().spoken_item().unwrap();
    assert!(
        said.starts_with("f, the document's folder, 1 of "),
        "{said}"
    );
    #[cfg(windows)]
    assert!(rows.iter().any(|r| r.contains(", disk")), "{rows:?}");
}

#[test]
fn rows_say_name_and_kind_within_forty_cells() {
    let mut app = app_with(&fixtures().join("readme.md"));
    browse_doc_folder(&mut app);
    assert_eq!(app.browse_location(), Some(&Location::Folder(fixtures())));
    let rows = items(&app);
    for row in &rows {
        let first: String = row.chars().take(40).collect();
        let name = row.split(", ").next().unwrap();
        assert!(first.starts_with(name), "{row}");
        assert!(
            first.matches(", ").count() >= 1,
            "kind within 40 cells: {row}"
        );
    }
    assert!(
        rows.iter()
            .any(|r| r.starts_with("course.zip, zip archive, "))
    );
    assert!(rows.iter().any(|r| r.starts_with("readme.md, Markdown, ")));
    // Readable files only: the Python script is not listed.
    assert!(
        !rows.iter().any(|r| r.starts_with("make_fixtures.py")),
        "{rows:?}"
    );
}

#[test]
fn into_a_zip_a_folder_inside_and_back_out() {
    let mut app = app_with(&fixtures().join("readme.md"));
    browse_doc_folder(&mut app);
    focus(&mut app, "course.zip");
    key(&mut app, ListKey::Enter);
    let zip = fixtures().join("course.zip");
    assert_eq!(
        app.browse_location(),
        Some(&Location::Archive {
            path: zip.clone(),
            prefix: String::new()
        })
    );
    let rows = items(&app);
    assert_eq!(
        rows,
        [
            "week1, folder, 3 items",
            "week2, folder, 1 item",
            "syllabus.md, Markdown, 43 bytes"
        ],
        "junk (__MACOSX) is left out"
    );
    assert!(app.status_text().starts_with("course.zip, 3 items."));

    focus(&mut app, "week1");
    key(&mut app, ListKey::Enter);
    let rows = items(&app);
    assert!(rows[0].starts_with("slides, folder, 1 item"), "{rows:?}");
    assert!(rows.iter().any(|r| r.starts_with("extra.zip, zip archive")));
    assert!(rows.iter().any(|r| r.starts_with("notes.md, Markdown")));
    assert!(!rows.iter().any(|r| r.starts_with(".DS_Store")));
    let said = app.status_text();
    assert!(
        said.starts_with("week1, 3 items. In course.zip. 1 file hidden."),
        "{said}"
    );

    // Backspace: out of the folder, onto its row.
    key(&mut app, ListKey::Backspace);
    let l = app.list_model().unwrap();
    assert!(l.current().unwrap().starts_with("week1, folder"));
    // Backspace again: out of the archive, onto the archive's row, said
    // once, with the folder's introduction first.
    key(&mut app, ListKey::Backspace);
    assert_eq!(app.browse_location(), Some(&Location::Folder(fixtures())));
    let l = app.list_model().unwrap();
    assert!(l.current().unwrap().starts_with("course.zip, zip archive"));
    let said = app.status_text();
    assert!(said.starts_with("f, "), "{said}");
    assert_eq!(said.matches("course.zip").count(), 1, "{said}");
}

#[test]
fn a_member_opens_by_its_member_path() {
    let mut app = app_with(&fixtures().join("readme.md"));
    browse_doc_folder(&mut app);
    focus(&mut app, "course.zip");
    key(&mut app, ListKey::Enter);
    focus(&mut app, "week1");
    key(&mut app, ListKey::Enter);
    focus(&mut app, "notes.md");
    key(&mut app, ListKey::Enter);
    assert!(app.wait_for_open(Duration::from_secs(20)));
    assert_eq!(app.browse_location(), None, "the browser closed");
    let path = app.session().unwrap().doc.meta.path.clone().unwrap();
    assert!(
        path.to_string_lossy()
            .ends_with("course.zip!week1/notes.md"),
        "{}",
        path.display()
    );
}

#[test]
fn an_archive_in_an_archive_is_entered_and_left() {
    let mut app = app_with(&fixtures().join("readme.md"));
    browse_doc_folder(&mut app);
    focus(&mut app, "course.zip");
    key(&mut app, ListKey::Enter);
    focus(&mut app, "week1");
    key(&mut app, ListKey::Enter);
    focus(&mut app, "extra.zip");
    key(&mut app, ListKey::Enter);
    assert_eq!(items(&app), ["reading.md, Markdown, 48 bytes"]);
    key(&mut app, ListKey::Backspace);
    assert!(
        app.list_model()
            .unwrap()
            .current()
            .unwrap()
            .starts_with("extra.zip, zip archive")
    );
}

#[test]
fn a_seven_zip_archive_backspace_twice() {
    let mut app = app_with(&w3d().join("scan-en.txt"));
    browse_doc_folder(&mut app);
    focus(&mut app, "course.7z");
    key(&mut app, ListKey::Enter);
    assert!(items(&app)[0].starts_with("week1, folder, 1 item"));
    focus(&mut app, "week1");
    key(&mut app, ListKey::Enter);
    key(&mut app, ListKey::Backspace);
    key(&mut app, ListKey::Backspace);
    assert!(
        app.list_model()
            .unwrap()
            .current()
            .unwrap()
            .starts_with("course.7z, 7z archive")
    );
}

#[test]
fn hostile_archives_are_refused_with_a_sentence() {
    let mut app = app_with(&fixtures().join("readme.md"));
    browse_doc_folder(&mut app);
    focus(&mut app, "broken.zip");
    key(&mut app, ListKey::Enter);
    assert_eq!(
        app.status_text(),
        "broken.zip is not an archive textweaver can read; it may be damaged."
    );
    assert_eq!(app.browse_location(), Some(&Location::Folder(fixtures())));
    assert!(app.list_model().is_some(), "the list stays");

    focus(&mut app, "deep.zip");
    for _ in 0..8 {
        key(&mut app, ListKey::Enter);
        if app.status_text().contains("too many archives") {
            break;
        }
    }
    assert!(
        app.status_text()
            .ends_with("is inside too many archives to open."),
        "{}",
        app.status_text()
    );
}

#[test]
fn the_preview_says_title_and_first_sentence() {
    let mut app = app_with(&fixtures().join("readme.md"));
    browse_doc_folder(&mut app);
    focus(&mut app, "course.zip");
    key(&mut app, ListKey::Enter);
    focus(&mut app, "week1");
    key(&mut app, ListKey::Enter);
    focus(&mut app, "notes.md");
    key(&mut app, ListKey::Details);
    assert!(app.wait_for_preview(Duration::from_secs(20)));
    assert_eq!(
        app.status_text(),
        "Week one notes. Cells are the smallest units of life."
    );
    // An archive: its count and first names.
    key(&mut app, ListKey::Backspace);
    key(&mut app, ListKey::Backspace);
    focus(&mut app, "course.zip");
    key(&mut app, ListKey::Details);
    assert!(app.wait_for_preview(Duration::from_secs(20)));
    let said = app.status_text();
    assert!(
        said.starts_with("course.zip: 6 files, 5 readable."),
        "{said}"
    );
}

#[test]
fn a_preview_for_a_closed_browser_is_not_said() {
    let mut app = app_with(&fixtures().join("readme.md"));
    browse_doc_folder(&mut app);
    focus(&mut app, "readme.md");
    key(&mut app, ListKey::Details);
    key(&mut app, ListKey::Escape);
    assert!(app.wait_for_preview(Duration::from_secs(20)));
    assert!(!app.status_text().contains("File browser fixtures"));
}

#[test]
fn sorting_cycles_and_keeps_the_row() {
    let mut app = app_with(&fixtures().join("readme.md"));
    browse_doc_folder(&mut app);
    assert_eq!(app.browse_sort(), SortBy::Name);
    focus(&mut app, "readme.md");
    key(&mut app, ListKey::Sort);
    assert_eq!(app.browse_sort(), SortBy::Date);
    assert!(
        app.status_text()
            .starts_with("Sorted by date, newest first. readme.md,")
    );
    let l = app.list_model().unwrap();
    assert!(l.current().unwrap().starts_with("readme.md,"));
    key(&mut app, ListKey::Sort);
    assert_eq!(app.browse_sort(), SortBy::Size);
    let rows = items(&app);
    // Largest first: readme.md is larger than broken.zip.
    let readme = rows
        .iter()
        .position(|r| r.starts_with("readme.md"))
        .unwrap();
    let broken = rows
        .iter()
        .position(|r| r.starts_with("broken.zip"))
        .unwrap();
    assert!(readme < broken, "{rows:?}");
    key(&mut app, ListKey::Sort);
    assert_eq!(app.browse_sort(), SortBy::Name);
}

#[test]
fn show_all_lists_every_file() {
    let mut app = app_with(&fixtures().join("readme.md"));
    browse_doc_folder(&mut app);
    assert!(
        !items(&app)
            .iter()
            .any(|r| r.starts_with("make_fixtures.py"))
    );
    key(&mut app, ListKey::ShowAll);
    assert!(app.status_text().starts_with("Showing all files."));
    assert!(
        items(&app)
            .iter()
            .any(|r| r.starts_with("make_fixtures.py, ")),
        "{:?}",
        items(&app)
    );
    key(&mut app, ListKey::ShowAll);
    assert!(
        app.status_text()
            .starts_with("Showing readable files only.")
    );
}

#[test]
fn typing_filters_and_backspace_takes_it_back() {
    let mut app = app_with(&fixtures().join("readme.md"));
    browse_doc_folder(&mut app);
    for c in "cour".chars() {
        key(&mut app, ListKey::Char(c));
    }
    assert_eq!(items(&app).len(), 1);
    assert!(items(&app)[0].starts_with("course.zip"));
    assert!(app.status_text().starts_with("f, 1 item matches cour."));
    for _ in 0..4 {
        key(&mut app, ListKey::Backspace);
    }
    assert!(items(&app).len() > 1);
    assert_eq!(app.browse_location(), Some(&Location::Folder(fixtures())));
}

#[test]
fn the_browser_never_changes_a_file() {
    let mut app = app_with(&fixtures().join("readme.md"));
    browse_doc_folder(&mut app);
    focus(&mut app, "broken.zip");
    key(&mut app, ListKey::Delete);
    assert_eq!(
        app.status_text(),
        "The file browser only opens and chooses files; it never changes them."
    );
    key(&mut app, ListKey::Rename);
    assert!(fixtures().join("broken.zip").is_file());
    assert!(app.list_model().is_some());
}

static CHOSEN: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

fn remember(_: &mut App, path: PathBuf) -> Vec<Effect> {
    CHOSEN.lock().unwrap().push(path);
    vec![Effect::Redraw]
}

/// A command that needs a folder, as batch conversion registers one.
fn convert(app: &mut App) -> Vec<Effect> {
    app.choose_folder("Choose the folder to convert", remember)
}

#[test]
fn choosing_a_folder_hands_it_to_the_caller() {
    let mut app = app_with(&fixtures().join("readme.md"));
    app.register_handler(ActionId::BatchConvert, convert);
    app.dispatch(Command::Action(ActionId::BatchConvert));
    assert_eq!(app.browse_location(), Some(&Location::Places));
    assert!(
        app.status_text()
            .starts_with("Choose the folder to convert. Places, "),
        "{}",
        app.status_text()
    );
    // The Choose Folder key on a place takes it.
    key(&mut app, ListKey::ChooseHere);
    assert_eq!(app.browse_location(), None);
    assert!(app.list_model().is_none());
    assert!(CHOSEN.lock().unwrap().contains(&fixtures()));

    // In a folder, the first row chooses it (for terminals without
    // Ctrl+Enter); Enter on a file says how to choose.
    app.dispatch(Command::Action(ActionId::BatchConvert));
    key(&mut app, ListKey::Enter);
    assert_eq!(items(&app)[0], "Choose this folder, f");
    focus(&mut app, "readme.md");
    key(&mut app, ListKey::Enter);
    assert!(
        app.status_text()
            .starts_with("Choose a folder: Enter opens one, ")
    );
    // Archives are not entered while choosing a folder.
    focus(&mut app, "course.zip");
    key(&mut app, ListKey::Enter);
    assert_eq!(app.browse_location(), Some(&Location::Folder(fixtures())));
    assert!(app.status_text().starts_with("Choose a folder:"));
    // Enter on the first row takes the folder shown.
    CHOSEN.lock().unwrap().clear();
    app.dispatch(Command::ListFocus(0));
    key(&mut app, ListKey::Enter);
    assert_eq!(app.browse_location(), None);
    assert_eq!(*CHOSEN.lock().unwrap(), [fixtures()]);
}

#[test]
fn the_choose_key_inside_an_archive_takes_nothing() {
    let mut app = app_with(&fixtures().join("readme.md"));
    browse_doc_folder(&mut app);
    focus(&mut app, "course.zip");
    key(&mut app, ListKey::Enter);
    key(&mut app, ListKey::ChooseHere);
    assert_eq!(
        app.status_text(),
        "No command is waiting for a folder; Enter opens it."
    );
}

#[test]
fn choose_folder_without_a_caller_says_so() {
    let mut app = app_with(&fixtures().join("readme.md"));
    browse_doc_folder(&mut app);
    key(&mut app, ListKey::ChooseHere);
    assert_eq!(
        app.status_text(),
        "No command is waiting for a folder; Enter opens it."
    );
    assert!(app.list_model().is_some());
}

#[test]
fn the_browser_keys_come_from_the_app_while_it_is_shown() {
    let mut app = app_with(&fixtures().join("readme.md"));
    let command = command_modifier(app.keymap());
    let sort = BrowseKey::Sort.chord(command);
    assert_eq!(app.browse_list_key_for(&sort), None, "not browsing");
    browse_doc_folder(&mut app);
    for k in BrowseKey::ALL {
        assert_eq!(
            app.browse_list_key_for(&k.chord(command)),
            Some(k.list_key())
        );
    }
    // None of them shadows a list key or a command the list uses.
    let say_status = app.keymap().chords_for(ActionId::SayStatus);
    for k in BrowseKey::ALL {
        assert!(!say_status.contains(&k.chord(command)));
    }
}

#[test]
fn escape_closes_and_says_so() {
    let mut app = app_with(&fixtures().join("readme.md"));
    browse_doc_folder(&mut app);
    key(&mut app, ListKey::Escape);
    assert_eq!(app.browse_location(), None);
    assert!(app.list_model().is_none());
    // Backspace on the places closes too.
    app.dispatch(Command::Action(ActionId::BrowseFiles));
    key(&mut app, ListKey::Backspace);
    assert_eq!(app.browse_location(), None);
}

#[test]
fn browse_files_is_in_the_file_menu_and_the_palette() {
    let app = App::new(AppConfig::for_tests());
    assert!(app.is_available(ActionId::BrowseFiles));
    assert!(
        app.palette_candidates("browse")
            .iter()
            .any(|(a, _)| *a == ActionId::BrowseFiles)
    );
}
