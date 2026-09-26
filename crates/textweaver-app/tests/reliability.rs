//! Phase 2 stability (Agent P2a): find over large documents, the background
//! writer, positions that survive outside edits, and restarting speech.

use std::sync::{Arc, Mutex};

use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::core::CharPos;
use textweaver_app::keymap::ActionId;
use textweaver_app::store::DocKey;
use textweaver_app::testing::recording_service;
use textweaver_app::text::Document;
use textweaver_app::{App, AppConfig, Command};

/// Collects announcements for assertions.
#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Said {
    fn all(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
    fn last(&self) -> String {
        self.all().last().cloned().unwrap_or_default()
    }
}

impl Announcer for Said {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}

fn app_with(text: &str) -> (App, Said) {
    let said = Said::default();
    let (speech, _log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        announcer: Box::new(said.clone()),
        ..AppConfig::for_tests()
    });
    app.open_document(
        Document::from_plain_text(text),
        DocKey::untitled(1),
        "Test".into(),
    );
    (app, said)
}

#[test]
fn find_keeps_a_window_of_matches_and_counts_them_all() {
    let n = 25_000;
    let text = "word x ".repeat(n);
    let (mut app, said) = app_with(&text);
    app.dispatch(Command::Find("x".into()));
    let f = app.session().unwrap().find.clone().unwrap();
    assert_eq!(f.total, n);
    assert!(f.hits.len() <= 10_000, "{} kept", f.hits.len());
    assert!(said.last().contains("Match 1 of 25000"), "{}", said.last());
    // Step past the matches kept: the search runs again there and the
    // numbering goes on.
    app.dispatch(Command::GoTo(textweaver_app::text::GoTo::Percent(90)));
    app.dispatch(Command::Action(ActionId::FindNext));
    let at = app.session().unwrap().cursor;
    let number = at.0 / 7 + 1;
    assert!(
        said.last().contains(&format!("Match {number} of 25000")),
        "{} at {at:?}",
        said.last()
    );
    let f = app.session().unwrap().find.clone().unwrap();
    let i = f.current.unwrap();
    assert_eq!(f.first_index + i + 1, number);
    assert_eq!(f.hits[i].start, at);
    // Backward past the start of the window, then wrapping at the top.
    app.dispatch(Command::GoTo(textweaver_app::text::GoTo::Start));
    app.dispatch(Command::Action(ActionId::FindPrevious));
    assert!(
        said.last().contains("Match 25000 of 25000"),
        "{}",
        said.last()
    );
    assert_eq!(app.session().unwrap().cursor, CharPos(7 * (n - 1) + 5));
    app.dispatch(Command::Action(ActionId::FindNext));
    assert!(said.last().contains("Match 1 of 25000"), "{}", said.last());
}
