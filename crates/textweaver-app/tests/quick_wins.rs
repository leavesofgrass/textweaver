//! Phase 1 safety and authoring quick wins (Agent P1b), driven through
//! `App::dispatch` without a terminal.

use textweaver_app::keymap::ActionId;
use textweaver_app::store::DocKey;
use textweaver_app::text::Document;
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
