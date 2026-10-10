//! The keyboard shortcuts list and the command palette (beta 1): one row
//! format, "Find next, F3", short name first and the key after it; the
//! long explanation on F1 and as the row's description; the keyboard
//! list filters as you type (name, key, menu), says "k of n" commands
//! match, and moves by group with Page Down and Page Up.

use std::sync::{Arc, Mutex};

use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::keymap::{ActionId, Category, Frontend, Keymap, Platform};
use textweaver_app::testing::recording_service;
use textweaver_app::{App, AppConfig, Command, ListKey};

#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Said {
    fn last(&self) -> String {
        self.0.lock().unwrap().last().cloned().unwrap_or_default()
    }
    fn any(&self, needle: &str) -> bool {
        self.0.lock().unwrap().iter().any(|s| s.contains(needle))
    }
}

impl Announcer for Said {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}

fn app_with(keymap: Option<Keymap>) -> (App, Said) {
    let said = Said::default();
    let (speech, _log) = recording_service().unwrap();
    let mut config = AppConfig {
        speech,
        announcer: Box::new(said.clone()),
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    };
    if let Some(k) = keymap {
        config.keymap = k;
    }
    (App::new(config), said)
}

fn keys_list() -> (App, Said) {
    let (mut app, said) = app_with(None);
    app.dispatch(Command::Action(ActionId::KeyboardHelp));
    (app, said)
}

fn typed(app: &mut App, text: &str) {
    for c in text.chars() {
        app.dispatch(Command::ListKey(ListKey::Char(c)));
    }
}

#[test]
fn rows_are_the_short_name_then_the_key() {
    let (app, said) = keys_list();
    assert!(said.any("Type to filter."), "{}", said.last());
    let l = app.list_model().expect("the keyboard list");
    assert_eq!(l.items.len(), ActionId::ALL.len());
    assert_eq!(l.columns.len(), l.items.len());
    assert_eq!(l.descriptions.len(), l.items.len());
    let find = ActionId::FindNext;
    let row = app.command_row(find);
    let key = row.key.clone().expect("find next has a key");
    let i = l
        .items
        .iter()
        .position(|s| s == &format!("{}, {key}", row.name))
        .expect("the find next row");
    assert_eq!(l.columns[i], (row.name.clone(), key));
    // The long explanation is the description, never in the row.
    assert!(
        l.descriptions[i].contains(find.help()),
        "{}",
        l.descriptions[i]
    );
    assert!(l.items.iter().all(|s| !s.contains(find.help())));
}

#[test]
fn f1_on_a_row_says_what_it_does() {
    let (mut app, said) = keys_list();
    typed(&mut app, "find next");
    let row = app
        .list_model()
        .and_then(|l| l.current().map(str::to_owned));
    assert!(row.is_some_and(|r| r.starts_with("Find next")));
    app.dispatch(Command::ListKey(ListKey::Introduce));
    assert!(said.last().starts_with("Find next: "), "{}", said.last());
    assert!(said.last().contains(ActionId::FindNext.help()));
}

#[test]
fn the_keyboard_list_filters_by_name_key_and_menu_and_says_k_of_n() {
    let (mut app, said) = keys_list();
    let total = ActionId::ALL.len();
    typed(&mut app, "bookmark");
    let n = app.list_model().map_or(0, |l| l.items.len());
    assert!(n > 1 && n < total, "{n}");
    assert!(said.any(&format!("{n} of {total} commands match.")));
    let title = app
        .list_model()
        .map(|l| l.title.clone())
        .unwrap_or_default();
    assert_eq!(title, "Keyboard shortcuts matching bookmark");
    // The bookmark commands are there (others match by their menu).
    assert!(
        app.list_model()
            .is_some_and(|l| l.items.iter().any(|i| i.starts_with("Add bookmark")))
    );
    // Backspace takes letters off; an empty filter shows everything.
    for _ in 0.."bookmark".len() {
        app.dispatch(Command::ListKey(ListKey::Backspace));
    }
    assert_eq!(app.list_model().map(|l| l.items.len()), Some(total));
    assert!(said.any(&format!("Filter cleared, {total} commands.")));
    // By key: F3 finds the command it runs.
    let key = app.command_row(ActionId::FindNext).key.unwrap();
    typed(&mut app, &key.to_lowercase());
    assert!(
        app.list_model()
            .is_some_and(|l| l.items.iter().any(|i| i.starts_with("Find next")))
    );
    for _ in 0..key.len() {
        app.dispatch(Command::ListKey(ListKey::Backspace));
    }
    // By menu: a command found by the menu that holds it.
    let path = app.menu_path_of(ActionId::ExportPdf).expect("in a menu");
    let menu = path.split(", ").next().unwrap().to_owned();
    typed(&mut app, &format!("{} pdf", menu.to_lowercase()));
    assert!(
        app.list_model()
            .is_some_and(|l| l.items.iter().any(|i| i.starts_with("Export PDF"))),
        "{menu}"
    );
    // Nothing matches: said in words, and the list stays open.
    typed(&mut app, "zzzq");
    assert_eq!(app.list_model().map(|l| l.items.len()), Some(0));
    assert!(said.any("zzzq. Backspace removes letters."));
    assert!(said.any("No commands match "));
}

#[test]
fn page_down_and_page_up_move_by_group() {
    let (mut app, said) = keys_list();
    let first = Category::Reading.title();
    let start = app.list_model().map_or(0, |l| l.selected);
    assert_eq!(start, 0);
    app.dispatch(Command::ListKey(ListKey::PageDown));
    let at = app.list_model().map_or(0, |l| l.selected);
    assert!(at > 0);
    let next = Category::Navigation.title();
    assert!(
        said.last().starts_with(&format!("{next}, ")),
        "{}",
        said.last()
    );
    assert!(said.last().contains(&format!("{} of ", at + 1)));
    // Up from the first row of a group says the group it enters.
    app.dispatch(Command::ListKey(ListKey::Up));
    assert!(
        said.last().starts_with(&format!("{first}, ")),
        "{}",
        said.last()
    );
    // Page Up goes to the start of the group, then to the one before.
    app.dispatch(Command::ListKey(ListKey::PageUp));
    assert_eq!(app.list_model().map(|l| l.selected), Some(0));
    app.dispatch(Command::ListKey(ListKey::PageDown));
    app.dispatch(Command::ListKey(ListKey::PageDown));
    app.dispatch(Command::ListKey(ListKey::PageUp));
    assert_eq!(app.list_model().map(|l| l.selected), Some(at));
}

/// Every row is short enough for a 40-cell Braille line: the short name
/// and one key, in both frontends and on every platform.
#[test]
fn rows_fit_forty_cells() {
    for platform in [Platform::Linux, Platform::Windows, Platform::MacOs] {
        for frontend in [Frontend::Terminal, Frontend::Gui] {
            let (mut app, _) = app_with(Some(Keymap::defaults(platform, frontend)));
            app.dispatch(Command::Action(ActionId::KeyboardHelp));
            let l = app.list_model().expect("the keyboard list");
            let long: Vec<&String> = l.items.iter().filter(|i| i.chars().count() > 40).collect();
            assert!(long.is_empty(), "{platform:?} {frontend:?}: {long:#?}");
        }
    }
}

#[test]
fn the_palette_uses_the_same_rows_and_f1() {
    let (mut app, said) = app_with(None);
    let cands = app.palette_candidates("find next");
    let (a, line) = cands.first().cloned().expect("a match");
    assert_eq!(a, ActionId::FindNext);
    assert_eq!(line, app.command_row(a).text(app.catalog().as_ref()));
    // Ctrl+L: the matches as a list, with the same columns and F1.
    app.dispatch(Command::Action(ActionId::CommandPalette));
    for c in "find next".chars() {
        app.dispatch(Command::PromptKey(textweaver_app::PromptKey::Char(c)));
    }
    app.dispatch(Command::PromptKey(textweaver_app::PromptKey::ShowMatches));
    let l = app.list_model().expect("the matches");
    assert_eq!(l.columns.len(), l.items.len());
    app.dispatch(Command::ListKey(ListKey::Introduce));
    assert!(said.last().starts_with("Find next: "), "{}", said.last());
}
