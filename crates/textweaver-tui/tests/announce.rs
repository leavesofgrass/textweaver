//! What the terminal says when a list opens and when a message repeats
//! (docs/audit-2026-09.md, findings A4 and A5; Agent D4).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::core::CharPos;
use textweaver_app::keymap::ActionId;
use textweaver_app::store::DocKey;
use textweaver_app::testing::{SpeechLog, recording_service};
use textweaver_app::text::{Document, GoTo};
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig, Command};
use textweaver_tui::Tui;
use textweaver_tui::ui::REPEAT_BLANK;

#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Said {
    fn all(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

impl Announcer for Said {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}

fn tui(text: &str) -> (Tui, Said, SpeechLog, Terminal<TestBackend>) {
    let said = Said::default();
    let (speech, log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        announcer: Box::new(said.clone()),
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    app.open_document(
        Document::from_plain_text(text),
        DocKey::untitled(1),
        "T".into(),
    );
    let tui = Tui::with_color_support(app, ColorSupport::TrueColor);
    let term = Terminal::new(TestBackend::new(80, 24)).unwrap();
    (tui, said, log, term)
}

fn status_rows(tui: &Tui, term: &Terminal<TestBackend>) -> String {
    let buf = term.backend().buffer();
    let a = tui.areas(buf.area).status;
    (a.y..a.y + a.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_owned()
}

/// Waits (with a deadline) until the speech thread has spoken `text`.
fn wait_spoken(log: &SpeechLog, text: &str) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if log.texts().iter().any(|t| t.contains(text)) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    false
}

#[test]
fn a_list_says_its_first_item_after_the_introduction() {
    let (mut tui, said, log, _term) =
        tui("First line here.\nSecond line here.\nThird line here.\n");
    for pos in [0, 17, 35] {
        tui.dispatch(Command::GoTo(GoTo::Char(CharPos(pos))));
        tui.dispatch(Command::Action(ActionId::AddBookmark));
    }
    log.clear();
    tui.dispatch(Command::Action(ActionId::ListBookmarks));
    let list = tui.list().expect("the bookmark list is open").clone();
    let first = format!("{}, 1 of 3", list.items[0]);
    let all = said.all();
    let intro = all
        .iter()
        .rposition(|m| m.starts_with("Bookmarks"))
        .expect("an introduction");
    assert_eq!(all.last(), Some(&first), "{all:?}");
    assert!(intro < all.len() - 1);
    // Spoken after the introduction, queued, not interrupting it.
    assert!(wait_spoken(&log, &first), "{:?}", log.texts());
    let texts = log.texts();
    let i = texts
        .iter()
        .position(|t| t.starts_with("Bookmarks"))
        .unwrap();
    let j = texts.iter().position(|t| *t == first).unwrap();
    assert!(i < j, "{texts:?}");
    // The status line shows the introduction and the item.
    let status = tui.status_line();
    assert!(
        status.starts_with("Bookmarks") && status.ends_with(&first),
        "{status}"
    );
    // Moving says the item and where it is.
    tui.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(
        said.all().last().cloned(),
        Some(format!("{}, 2 of 3", list.items[1]))
    );
}

#[test]
fn a_repeated_message_blanks_the_status_line_once() {
    let (mut tui, _said, _log, mut term) = tui("Only one line, no headings.\n");
    tui.dispatch(Command::Action(ActionId::NextHeading));
    term.draw(|f| tui.draw(f)).unwrap();
    let first = status_rows(&tui, &term);
    assert!(first.contains("No next heading"), "{first}");
    // The same message again: blank for a moment, then back.
    tui.dispatch(Command::Action(ActionId::NextHeading));
    term.draw(|f| tui.draw(f)).unwrap();
    assert_eq!(status_rows(&tui, &term), "");
    assert!(tui.status_blanked());
    let deadline = Instant::now() + REPEAT_BLANK + Duration::from_secs(5);
    loop {
        std::thread::sleep(Duration::from_millis(20));
        term.draw(|f| tui.draw(f)).unwrap();
        if status_rows(&tui, &term) == first {
            break;
        }
        assert!(Instant::now() < deadline, "the message never came back");
    }
    assert!(!tui.status_blanked());
    // Drawing again without a new announcement does not blank it.
    term.draw(|f| tui.draw(f)).unwrap();
    assert_eq!(status_rows(&tui, &term), first);
}
