//! Phase 1 safety and authoring quick wins (Agent P1b), driven through
//! `App::dispatch` without a terminal.

use textweaver_app::core::CharPos;
use textweaver_app::keymap::ActionId;
use textweaver_app::store::{DocKey, Paths, StateStore};
use textweaver_app::text::{Document, GoTo};
use textweaver_app::{App, AppConfig, Command, Effect};

/// A silent app (no self-voicing, as with `--no-speech`) with `text` open.
fn app_with(text: &str) -> App {
    let mut app = App::new(AppConfig::for_tests());
    app.open_document(
        Document::from_plain_text(text),
        DocKey::untitled(1),
        "T".into(),
    );
    app
}

fn act(app: &mut App, a: ActionId) -> Vec<Effect> {
    app.dispatch(Command::Action(a))
}

/// The saved position and every bookmark carry the text found there, for
/// finding them again after the file changes outside textweaver.
#[test]
fn positions_and_bookmarks_are_saved_with_anchors() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(&dir.path().join("home"));
    let file = dir.path().join("a.txt");
    let text = "Alpha beta gamma. Delta epsilon zeta eta theta iota kappa lambda.\n";
    std::fs::write(&file, text).unwrap();
    let mut app = App::new(AppConfig {
        paths: Some(paths.clone()),
        ..AppConfig::for_tests()
    });
    app.open(&file).unwrap();
    app.dispatch(Command::GoTo(GoTo::Char(CharPos(18))));
    act(&mut app, ActionId::AddBookmark);
    app.dispatch(Command::GoTo(GoTo::Char(CharPos(6))));
    app.save_position().unwrap();
    let state = StateStore::new(paths.state_dir())
        .load(&DocKey::for_path(&file))
        .unwrap();
    let anchor = state.anchor.expect("position anchor");
    assert!(anchor.context.starts_with("beta gamma."), "{anchor:?}");
    assert_eq!(anchor.context.chars().count(), 40);
    let b = state.bookmarks[0].anchor.as_ref().expect("bookmark anchor");
    assert!(b.context.starts_with("Delta epsilon"), "{b:?}");
}

/// With `--no-speech` a screen reader reads the status line, so every
/// line the Speech Cursor reads is put there.
#[test]
fn speech_cursor_lines_are_on_the_status_line() {
    let mut app = app_with("First line.\n\nThird line here.\n");
    act(&mut app, ActionId::SpeechCursorToggle);
    assert!(
        app.status_text().contains("First line."),
        "{}",
        app.status_text()
    );
    act(&mut app, ActionId::SpeechCursorNextLine);
    assert_eq!(app.status_text(), "blank");
    act(&mut app, ActionId::SpeechCursorNextLine);
    assert_eq!(app.status_text(), "Third line here.");
    act(&mut app, ActionId::SpeechCursorRereadLine);
    assert_eq!(app.status_text(), "Third line here.");
}
