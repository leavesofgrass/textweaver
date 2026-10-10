//! Batch conversion from the menus (Wave 6, W6k): the folder from the file
//! browser, the format, where, the yes-or-no question, the run, and the
//! summary with the failures listed name first.
//!
//! Keys are the list's own (`ListKey`) and commands are dispatched, so no
//! chord is written into a test.

#![cfg(feature = "publish")]

use std::path::Path;
use std::time::{Duration, Instant};

use textweaver_app::keymap::ActionId;
use textweaver_app::{App, AppConfig, Command, Confirm, ListKey};

fn write(dir: &Path, name: &str, text: &[u8]) {
    std::fs::write(dir.join(name), text).unwrap();
}

fn items(app: &App) -> Vec<String> {
    app.list_model()
        .map(|l| l.items.clone())
        .unwrap_or_default()
}

/// Ticks until the batch has ended (a list of failures or the summary is
/// said), for at most a minute.
fn wait_for_end(app: &mut App) {
    let limit = Instant::now() + Duration::from_secs(60);
    while Instant::now() < limit {
        app.tick(Instant::now());
        let s = app.status_text();
        if s.starts_with("Converted ") || s.starts_with("Stopped.") {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("the batch did not end in a minute: {}", app.status_text());
}

#[test]
fn a_folder_converts_to_markdown_and_the_failures_are_listed() {
    let dir = tempfile::tempdir().unwrap();
    let notes = dir.path().join("notes");
    std::fs::create_dir(&notes).unwrap();
    write(&notes, "crows.md", b"# Crows\n\nThey remember faces.\n");
    write(&notes, "river.txt", b"The river is high today.\n");
    write(&notes, "report.docx", b"PK not really a document");
    let mut app = App::new(AppConfig::for_tests());
    app.open(&notes.join("crows.md")).unwrap();

    // The command is in the menus now, and starts with the folder.
    assert!(app.is_available(ActionId::BatchConvert));
    app.dispatch(Command::Action(ActionId::BatchConvert));
    assert!(
        app.status_text()
            .starts_with("Choose the folder to convert. "),
        "{}",
        app.status_text()
    );
    // The document's folder is the place focused: choose it.
    app.dispatch(Command::ListKey(ListKey::ChooseHere));

    // The formats, Markdown first.
    assert!(
        app.status_text()
            .starts_with("Convert notes to: choose a format"),
        "{}",
        app.status_text()
    );
    let formats = items(&app);
    assert_eq!(formats[0], "Markdown");
    assert_eq!(formats[1], "PDF");
    // Seven native formats, then carta's five (AsciiDoc to Org).
    let carta = if cfg!(feature = "carta") { 5 } else { 0 };
    assert_eq!(formats.len(), 7 + carta);
    if carta > 0 {
        assert_eq!(formats[7], "AsciiDoc");
        assert_eq!(formats[11], "Org");
    }
    app.dispatch(Command::Choose(0));

    // Where: the converted folder first.
    let places = items(&app);
    assert!(
        places[0].starts_with("In a converted folder, "),
        "{places:?}"
    );
    app.dispatch(Command::Choose(0));
    let question = app.status_text().to_owned();
    assert!(
        question.starts_with("Convert 3 files to Markdown into "),
        "{question}"
    );
    assert!(question.ends_with("converted? y or n"), "{question}");
    assert!(app.confirmation_pending());

    app.dispatch(Command::Confirm(Confirm::Yes));
    assert!(
        app.status_text()
            .starts_with("Converting 3 files to Markdown."),
        "{}",
        app.status_text()
    );
    wait_for_end(&mut app);
    let said = app.status_text().to_owned();
    assert!(
        said.starts_with("Converted 2 files to Markdown; 0 up to date; 1 failed."),
        "{said}"
    );
    assert!(said.contains("conversion-report.md"), "{said}");
    // The failures, name first, in a list.
    let failed = items(&app);
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert!(failed[0].starts_with("report.docx: "), "{failed:?}");
    let out = notes.join("converted");
    assert!(out.join("river.md").is_file());
    let report = std::fs::read_to_string(out.join("conversion-report.md")).unwrap();
    assert!(report.contains("- report.docx: "), "{report}");
    // No temporary file is left beside the outputs.
    let names: Vec<String> = std::fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(!names.iter().any(|n| n.ends_with(".tmp")), "{names:?}");
}

#[test]
fn no_says_cancelled_and_nothing_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let notes = dir.path().join("notes");
    std::fs::create_dir(&notes).unwrap();
    write(&notes, "crows.md", b"# Crows\n");
    let mut app = App::new(AppConfig::for_tests());
    app.open(&notes.join("crows.md")).unwrap();
    app.dispatch(Command::Action(ActionId::BatchConvert));
    app.dispatch(Command::ListKey(ListKey::ChooseHere));
    app.dispatch(Command::Choose(1)); // PDF
    app.dispatch(Command::Choose(0));
    assert!(
        app.status_text().starts_with("Convert 1 file to PDF into "),
        "{}",
        app.status_text()
    );
    // Escape answers no.
    app.dispatch(Command::Cancel);
    assert_eq!(app.status_text(), "Canceled.");
    assert!(!app.confirmation_pending());
    assert!(!notes.join("converted").exists());
}
