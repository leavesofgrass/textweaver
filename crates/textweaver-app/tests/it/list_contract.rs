//! The list contract (conventions checklist row C2, W9e-c): every list
//! the app shows, opened through its action on a fixture document, says
//! its introduction once, then "1 of n" first; Home and End say the place
//! first; a letter jumps (or filters, in a list that filters as you
//! type); F1 (Introduce) says the introduction again; the Say Status key
//! (Details) says something; Escape closes and says so.
//!
//! The lists are found by dispatching every action in `ActionId::ALL`, so
//! a new list is checked the day it lands. The window's lists are drawn
//! from the same model (`textweaver-xilem` tests the dialog half, C1).

use std::path::Path;
use std::sync::{Arc, Mutex};

use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::keymap::ActionId;
use textweaver_app::testing::recording_service;
use textweaver_app::{App, AppConfig, Command, ListKey, Mode};

#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Said {
    fn all(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
    fn since(&self, n: usize) -> Vec<String> {
        self.all().split_off(n.min(self.all().len()))
    }
    fn len(&self) -> usize {
        self.0.lock().unwrap().len()
    }
}

impl Announcer for Said {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}

/// The app on the sample document, with two bookmarks and a note, so the
/// lists that need marks are not empty.
/// The sample is copied into a temporary folder first: some actions write
/// beside the document (a study sheet), never into the repository.
fn launch(announce_first_item: bool) -> (App, Said, tempfile::TempDir) {
    let said = Said::default();
    let (speech, _log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        announcer: Box::new(said.clone()),
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    app.set_announce_list_focus(announce_first_item);
    let dir = tempfile::tempdir().unwrap();
    let sample = dir.path().join("sample.md");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sample.md"),
        &sample,
    )
    .unwrap();
    app.dispatch(Command::Open(sample));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while app.session().is_none() && std::time::Instant::now() < deadline {
        app.tick(std::time::Instant::now());
        std::thread::yield_now();
    }
    assert!(app.session().is_some(), "the sample opens");
    app.dispatch(Command::Action(ActionId::AddBookmark));
    app.dispatch(Command::Action(ActionId::NextParagraph));
    app.dispatch(Command::Action(ActionId::NextParagraph));
    app.dispatch(Command::Action(ActionId::AddBookmark));
    app.dispatch(Command::Cancel);
    (app, said, dir)
}

/// Puts the app back in reading: closes what an action opened, leaves
/// edit mode, stops speech.
fn settle(app: &mut App) {
    for _ in 0..3 {
        app.dispatch(Command::Cancel);
    }
    if app.mode() == Mode::Edit {
        app.dispatch(Command::Action(ActionId::ToggleEditMode));
    }
    app.dispatch(Command::Action(ActionId::Stop));
}

/// Actions left out of the sweep, with the reason.
fn skipped(a: ActionId) -> bool {
    // Quitting ends the app; the rest of the sweep would run on nothing.
    matches!(a, ActionId::Quit)
}

/// "k of n" as the list says it, first or (in the file browser) last.
fn says_place(line: &str, k: usize, n: usize) -> bool {
    let place = format!("{k} of {n}");
    line.starts_with(&format!("{place},")) || line.trim_end_matches('.').ends_with(&place)
}

/// Defects the sweep found (W9e-c), as the action and the start of the
/// failure's text. [`every_list_keeps_the_list_contract`] leaves them
/// out; the ignored [`the_known_list_defects_are_fixed`] holds them until
/// they are fixed. Take a line off when its fix lands.
const KNOWN: &[(&str, &str)] = &[
    // The file browser's Say Status key says nothing on a Places row.
    ("AddLibraryFolder", "the Say Status key says nothing"),
    ("BrowseFiles", "the Say Status key says nothing"),
    ("BatchConvert", "the Say Status key says nothing"),
    // Escape closes these without a word.
    ("SettingsProfiles", "Escape closes it without a word"),
    ("ColorSettings", "Escape closes it without a word"),
];

fn known(failure: &str) -> bool {
    KNOWN.iter().any(|(action, what)| {
        failure.starts_with(&format!("{action} (")) && failure.contains(&format!("): {what}"))
    })
}

#[test]
fn every_list_keeps_the_list_contract() {
    let failures: Vec<String> = sweep().into_iter().filter(|f| !known(f)).collect();
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
#[ignore = "defects: the file browser's Say Status key is silent on Places; Escape is silent in the profiles and colors lists (W9e-c)"]
fn the_known_list_defects_are_fixed() {
    let failures = sweep();
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Opens every list and checks it; returns the failures.
fn sweep() -> Vec<String> {
    let (mut app, said, _dir) = launch(true);
    let mut lists = Vec::new();
    let mut failures = Vec::new();
    for &a in ActionId::ALL {
        if skipped(a) {
            continue;
        }
        settle(&mut app);
        let before = said.len();
        app.dispatch(Command::Action(a));
        let Some(list) = app.list_model().cloned() else {
            continue;
        };
        let n = list.items.len();
        let title = list.title.clone();
        lists.push(format!("{a:?}: {title} ({n})"));
        let mut fail = |what: String| failures.push(format!("{a:?} ({title}): {what}"));
        let opened = said.since(before);
        // The introduction once, then the first place.
        if opened.is_empty() {
            fail("says nothing when it opens".into());
            continue;
        }
        let intro = opened[0].trim_end_matches('.').to_owned();
        if opened.iter().filter(|l| l.contains(&intro)).count() != 1 {
            fail(format!(
                "the introduction is said more than once: {opened:?}"
            ));
        }
        // The place of the focused item (the first, or the current voice).
        let k = list.selected + 1;
        if n > 0 && !opened.iter().any(|l| says_place(l, k, n)) {
            fail(format!("no \"{k} of {n}\" when it opens: {opened:?}"));
        }
        // F1 says the introduction again, or, in a settings list, the
        // focused setting's help; then the Say Status key says the
        // introduction (or the file browser's preview, or a voice
        // sample). Between them the list's name is heard again.
        let at = said.len();
        app.dispatch(Command::ListKey(ListKey::Introduce));
        let f1 = said.since(at);
        if f1.iter().all(|l| l.trim().is_empty()) {
            fail("F1 says nothing".into());
        }
        let at = said.len();
        app.dispatch(Command::ListKey(ListKey::Details));
        let status = said.since(at);
        if app.list_model().is_some() && status.iter().all(|l| l.trim().is_empty()) {
            fail("the Say Status key says nothing".into());
        }
        // The list's name as its introduction says it ("Voice manager"),
        // or, after a purpose ("Choose the folder to convert."), the
        // name in a later clause ("Places").
        let names: Vec<&str> = intro
            .split([',', '.', ':'])
            .map(str::trim)
            .filter(|s| s.chars().any(char::is_alphabetic))
            .take(2)
            .collect();
        let name = names.join(" or ");
        if !f1
            .iter()
            .chain(&status)
            .any(|l| names.iter().any(|n| l.contains(n)))
        {
            fail(format!(
                "neither F1 nor the Say Status key names it ({name:?}): {f1:?} {status:?}"
            ));
        }
        if app.list_model().is_none() {
            continue;
        }
        // End and Home: the place said, the focus moved.
        if n > 1 {
            app.dispatch(Command::ListKey(ListKey::End));
            let last = said.all().last().cloned().unwrap_or_default();
            let l = app.list_model().map(|l| (l.selected, l.items.len()));
            if l != Some((n - 1, n)) || !says_place(&last, n, n) {
                fail(format!("End: {l:?}, said {last:?}"));
            }
            app.dispatch(Command::ListKey(ListKey::Home));
            let first = said.all().last().cloned().unwrap_or_default();
            let l = app.list_model().map(|l| l.selected);
            if l != Some(0) || !says_place(&first, 1, n) {
                fail(format!("Home: {l:?}, said {first:?}"));
            }
        }
        // A letter: the first letter of a later item that differs from
        // the first item's jumps there, or filters a filtering list.
        let initial = |s: &str| {
            s.chars()
                .find(|c| c.is_alphanumeric())
                .map(|c| c.to_lowercase().next().unwrap_or(c))
        };
        let first = list.items.first().and_then(|s| initial(s));
        if let Some(c) = list
            .items
            .iter()
            .skip(1)
            .filter_map(|s| initial(s))
            .find(|c| Some(*c) != first && c.is_ascii_alphabetic())
        {
            let at = said.len();
            app.dispatch(Command::ListKey(ListKey::Char(c)));
            let heard = said.since(at);
            match app.list_model() {
                // An accelerator chose an item and closed the list.
                None => {}
                Some(l) if l.items.len() == n && l.title == title => {
                    let now = l.current().and_then(initial);
                    if now != Some(c) {
                        fail(format!("letter {c} lands on {:?}", l.current()));
                    }
                }
                // A list that filters as you type.
                Some(_) => {}
            }
            if heard.iter().all(|l| l.trim().is_empty()) {
                fail(format!("letter {c} says nothing"));
            }
        }
        // Escape closes it, and says so.
        if app.list_model().is_some() {
            let at = said.len();
            app.dispatch(Command::ListKey(ListKey::Escape));
            if app.list_model().is_some() {
                fail("Escape leaves it open".into());
            }
            if said.since(at).is_empty() {
                fail("Escape closes it without a word".into());
            }
        }
    }
    assert!(
        lists.len() >= 8,
        "the sweep found too few lists: {lists:#?}"
    );
    failures
}

/// Enter chooses: on every list, Enter on the first item closes the list
/// or moves on to the next step (another list, a prompt, a question), and
/// never leaves the same list silent.
#[test]
fn enter_chooses_in_every_list() {
    let (mut app, said, _dir) = launch(true);
    let mut failures = Vec::new();
    for &a in ActionId::ALL {
        if skipped(a) {
            continue;
        }
        settle(&mut app);
        app.dispatch(Command::Action(a));
        let Some(list) = app.list_model().cloned() else {
            continue;
        };
        if list.items.is_empty() {
            continue;
        }
        let at = said.len();
        app.dispatch(Command::ListKey(ListKey::Enter));
        let same = app
            .list_model()
            .is_some_and(|l| l.title == list.title && l.items == list.items);
        if same && said.since(at).is_empty() {
            failures.push(format!("{a:?} ({}): Enter does nothing", list.title));
        }
    }
    settle(&mut app);
    assert!(failures.is_empty(), "{failures:#?}");
}

/// With a screen reader, the window turns the app's own first-item line
/// off, since the reader says the focused item: opening a list then says
/// only its introduction, never "1 of n" over the reader (nothing speaks
/// twice on open).
#[test]
fn opening_a_list_under_a_screen_reader_says_only_its_introduction() {
    let (mut app, said, _dir) = launch(false);
    let mut failures = Vec::new();
    let mut seen = 0;
    for &a in ActionId::ALL {
        if skipped(a) {
            continue;
        }
        settle(&mut app);
        let before = said.len();
        app.dispatch(Command::Action(a));
        let Some(list) = app.list_model().cloned() else {
            continue;
        };
        seen += 1;
        let opened = said.since(before);
        let n = list.items.len();
        if opened.len() > 1 {
            failures.push(format!("{a:?}: says more than one line: {opened:?}"));
        }
        if n > 0 && opened.iter().any(|l| l.starts_with(&format!("1 of {n},"))) {
            failures.push(format!("{a:?}: says the first item: {opened:?}"));
        }
    }
    settle(&mut app);
    assert!(seen >= 8, "{seen} lists");
    assert!(failures.is_empty(), "{failures:#?}");
}
