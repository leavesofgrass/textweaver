//! Phase 1 safety and authoring quick wins (Agent P1b), driven through
//! `App::dispatch` without a terminal.

use std::time::{Duration, Instant};

use textweaver_app::core::{CharPos, Direction};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::{DocKey, Paths, StateStore};
use textweaver_app::testing::{SpeechLog, recording_service};
use textweaver_app::text::{Document, GoTo};
use textweaver_app::{App, AppConfig, CaretMove, Command, Effect};

/// A self-voicing app recording what it says, with `text` open.
fn voiced_app(text: &str) -> (App, SpeechLog) {
    let (speech, log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    app.open_document(
        Document::from_plain_text(text),
        DocKey::untitled(1),
        "T".into(),
    );
    (app, log)
}

/// Waits (up to a deadline) until the speech log has a text containing
/// `needle`; returns every text.
fn wait_for_speech(log: &SpeechLog, needle: &str) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let texts = log.texts();
        if texts.iter().any(|t| t.contains(needle)) || Instant::now() > deadline {
            return texts;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Selecting or deleting more than about 200 characters says how many and
/// where they start and end, not the whole text.
#[test]
fn big_selections_and_deletions_are_summarized() {
    let long = format!(
        "Opening words here {}closing words here.",
        "and more ".repeat(40)
    );
    let text = format!("{long}\nNext line.\n");
    let mut app = app_with(&text);
    act(&mut app, ActionId::SelectNextLine);
    let n = long.chars().count();
    let expected = format!(
        "{} characters selected, from Opening words here and to more closing words here.",
        textweaver_app::editor::echo::thousands(n)
    );
    assert_eq!(app.status_text(), expected);
    // Short changes are still read as they are.
    act(&mut app, ActionId::SelectNextLine);
    assert_eq!(app.status_text(), "Next line. selected");

    let (mut app, log) = voiced_app(&text);
    act(&mut app, ActionId::ToggleEditMode);
    // Entering edit mode reads the line; only what follows matters here.
    wait_for_speech(&log, "Edit mode on");
    log.clear();
    for _ in 0..n {
        app.dispatch(Command::MoveCaret {
            by: CaretMove::Char,
            direction: Direction::Forward,
            extend: true,
        });
    }
    app.dispatch(Command::DeleteBack);
    let said = wait_for_speech(&log, "characters deleted");
    assert!(
        said.iter()
            .any(|t| t.contains("characters deleted, from Opening words here and to")),
        "{said:?}"
    );
    assert!(
        !said.iter().any(|t| t.contains(&long)),
        "the whole text was read"
    );
}

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
