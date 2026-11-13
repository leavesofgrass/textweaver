//! Find and replace keeps its matches while stepping (beta 6): a
//! replacement moves the matches after it, and every "Match N of M" is
//! what a search of the whole text says.

use textweaver_app::keymap::ActionId;
use textweaver_app::{App, AppConfig, Command, Effect};

fn list_title(effects: &[Effect]) -> String {
    effects
        .iter()
        .find_map(|e| match e {
            Effect::ShowList { title, .. } => Some(title.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// Edit mode on `text`, with the caret at the start.
fn editing(text: &str) -> (App, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("replace.md");
    std::fs::write(&path, text).unwrap();
    let mut app = App::new(AppConfig::for_tests());
    app.open(&path).unwrap();
    app.dispatch(Command::Action(ActionId::ToggleEditMode));
    (app, tmp)
}

fn text(app: &App) -> String {
    app.session().unwrap().doc.text().to_string()
}

/// A replacement longer than the match, holding the text searched for:
/// the matches after it move, the one it adds is counted, and every
/// "Match N of M" is what a search of the whole text says.
#[test]
fn replacing_with_a_longer_text_keeps_the_numbers_exact() {
    let (mut app, _tmp) = editing("a teh b teh c teh\n");
    app.dispatch(Command::Action(ActionId::Replace));
    app.dispatch(Command::Answer("teh".into()));
    let effects = app.dispatch(Command::Answer("the teh".into()));
    assert_eq!(
        list_title(&effects),
        "Match 1 of 3, line 1: teh becomes the teh"
    );
    let effects = app.dispatch(Command::Choose(0));
    assert_eq!(text(&app), "a the teh b teh c teh\n");
    // The new "teh" counts (three in all), and the next is the second
    // old one.
    assert_eq!(
        list_title(&effects),
        "Match 2 of 3, line 1: teh becomes the teh"
    );
    let effects = app.dispatch(Command::Choose(0));
    assert_eq!(text(&app), "a the teh b the teh c teh\n");
    assert_eq!(
        list_title(&effects),
        "Match 3 of 3, line 1: teh becomes the teh"
    );
    app.dispatch(Command::Choose(0));
    assert_eq!(text(&app), "a the teh b the teh c the teh\n");
    assert_eq!(app.status_text(), "Replaced 3 matches.");
}

/// A shorter replacement over several lines: the numbers count down as
/// matches go, and a skip keeps its place.
#[test]
fn replacing_with_a_shorter_text_moves_the_rest() {
    let (mut app, _tmp) = editing("catalog cat\ncat and cat\nlast cat\n");
    app.dispatch(Command::Action(ActionId::Replace));
    app.dispatch(Command::Answer("cat".into()));
    let effects = app.dispatch(Command::Answer("c".into()));
    assert!(
        list_title(&effects).starts_with("Match 1 of 5, line 1"),
        "{}",
        list_title(&effects)
    );
    let effects = app.dispatch(Command::Choose(1));
    assert!(
        list_title(&effects).starts_with("Match 2 of 5, line 1"),
        "{}",
        list_title(&effects)
    );
    let effects = app.dispatch(Command::Choose(0));
    assert_eq!(text(&app), "catalog c\ncat and cat\nlast cat\n");
    assert!(
        list_title(&effects).starts_with("Match 2 of 4, line 2"),
        "{}",
        list_title(&effects)
    );
    let effects = app.dispatch(Command::Choose(0));
    assert_eq!(text(&app), "catalog c\nc and cat\nlast cat\n");
    assert!(
        list_title(&effects).starts_with("Match 2 of 3, line 2: cat becomes c"),
        "{}",
        list_title(&effects)
    );
}
