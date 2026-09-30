//! Document identity in the reader (ADR-0049): opening a document finds or
//! makes its sync id on the background writer, and saving it keeps the id
//! and records the file's new hash.

use textweaver_app::keymap::ActionId;
use textweaver_app::store::sync_ids::SyncIds;
use textweaver_app::store::{DocKey, Paths};
use textweaver_app::{App, AppConfig, Command};

fn entry(paths: &Paths, file: &std::path::Path) -> textweaver_app::store::SyncIdEntry {
    SyncIds::load(&paths.sync_ids_file())
        .get(&DocKey::for_path(file))
        .cloned()
        .expect("an entry for the document")
}

#[test]
fn opening_and_saving_keep_one_sync_id() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = Paths::under(&tmp.path().join("home"));
    let file = tmp.path().join("cells.md");
    std::fs::write(&file, "# Cells\n\nMitosis divides a cell.\n").unwrap();
    let mut app = App::new(AppConfig {
        paths: Some(paths.clone()),
        ..AppConfig::for_tests()
    });
    app.open(&file).unwrap();
    app.wait_for_writes();
    let opened = entry(&paths, &file);
    assert_eq!(opened.sync_id.len(), 32);
    assert!(opened.content_sha256.is_some());
    assert!(opened.text_sha256.is_some(), "the text as read is hashed");

    app.dispatch(Command::Action(ActionId::ToggleEditMode));
    app.dispatch(Command::Insert("New ".into()));
    app.dispatch(Command::Action(ActionId::Save));
    app.wait_for_writes();
    let now = std::fs::read_to_string(&file).unwrap();
    assert!(now.contains("New "), "saved: {now:?}");
    let saved = entry(&paths, &file);
    assert_eq!(saved.sync_id, opened.sync_id, "an edit keeps the id");
    assert_ne!(saved.content_sha256, opened.content_sha256);

    // Opened again after a restart: the same id.
    app.shutdown();
    let mut again = App::new(AppConfig {
        paths: Some(paths.clone()),
        ..AppConfig::for_tests()
    });
    again.open(&file).unwrap();
    again.wait_for_writes();
    assert_eq!(entry(&paths, &file).sync_id, opened.sync_id);
    again.shutdown();
}
