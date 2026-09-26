//! How the terminal reader ends after a panic or a signal: unsaved edits
//! go to the recovery snapshot and the position is saved, without a
//! question (Agent P1b, roadmap Phase 1 "Panic hook and signals").

use textweaver_app::keymap::ActionId;
use textweaver_app::store::{Paths, StateStore};
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig, Command};
use textweaver_tui::{Tui, finish, signals};

fn editing_tui(home: &std::path::Path, file: &std::path::Path) -> Tui {
    let app = App::new(AppConfig {
        paths: Some(Paths::under(home)),
        ..AppConfig::for_tests()
    });
    let mut tui = Tui::with_color_support(app, ColorSupport::NoColor);
    tui.app_mut().open(file).unwrap();
    tui.dispatch(Command::Action(ActionId::ToggleEditMode));
    tui.dispatch(Command::Insert("X".into()));
    assert!(tui.app().is_dirty());
    tui
}

fn snapshots(home: &std::path::Path) -> usize {
    std::fs::read_dir(Paths::under(home).recovery_dir())
        .map(|d| {
            d.filter_map(Result::ok)
                .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
                .count()
        })
        .unwrap_or(0)
}

/// One test, because the signal flag is global to the process.
#[test]
fn a_panic_or_a_signal_keeps_unsaved_work_and_the_place() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.md");
    std::fs::write(&file, "Words to keep.\n").unwrap();

    // A panic in the loop.
    let home = dir.path().join("home1");
    let mut tui = editing_tui(&home, &file);
    let outcome = std::panic::catch_unwind(|| -> anyhow::Result<()> { panic!("boom") });
    let err = finish(&mut tui, outcome).unwrap_err().to_string();
    assert!(err.contains("boom"), "{err}");
    assert!(err.contains("offered for recovery"), "{err}");
    assert_eq!(snapshots(&home), 1);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "Words to keep.\n");

    // A signal: the loop stops without the user quitting.
    signals::reset();
    let home = dir.path().join("home2");
    let mut tui = editing_tui(&home, &file);
    signals::request();
    assert!(signals::requested());
    finish(&mut tui, Ok(Ok(()))).unwrap();
    assert_eq!(snapshots(&home), 1);
    signals::reset();

    // A signal while reading: the position is saved.
    let home = dir.path().join("home3");
    let app = App::new(AppConfig {
        paths: Some(Paths::under(&home)),
        ..AppConfig::for_tests()
    });
    let mut tui = Tui::with_color_support(app, ColorSupport::NoColor);
    tui.app_mut().open(&file).unwrap();
    tui.dispatch(Command::Action(ActionId::CaretNextWord));
    signals::request();
    finish(&mut tui, Ok(Ok(()))).unwrap();
    signals::reset();
    let key = tui.app().session().unwrap().key.clone();
    let state = StateStore::new(Paths::under(&home).state_dir())
        .load(&key)
        .expect("position saved");
    assert!(state.position.0 > 0);
}
