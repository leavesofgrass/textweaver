//! `App::update_settings`: a frontend's own dialog (the GUI's font chooser)
//! changes settings, and they are saved at once and survive the app's own
//! save at quit.

use textweaver_app::store::{Paths, SettingsStore};
use textweaver_app::{App, AppConfig};

#[test]
fn a_frontend_change_is_saved_and_kept() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    let mut app = App::new(AppConfig {
        paths: Some(paths.clone()),
        ..AppConfig::for_tests()
    });
    app.update_settings(|s| {
        s.reading_aids.font.size_pt = 22.0;
        s.reading_aids.font.weight = 700;
    })
    .unwrap();
    assert_eq!(app.settings().reading_aids.font.size_pt, 22.0);
    let (saved, _) = SettingsStore::new(paths.clone()).load();
    assert_eq!(saved.reading_aids.font.size_pt, 22.0);
    assert_eq!(saved.reading_aids.font.weight, 700);
    // Quitting saves the app's copy, which already has the change.
    app.shutdown();
    let (saved, _) = SettingsStore::new(paths).load();
    assert_eq!(saved.reading_aids.font.size_pt, 22.0);
}
