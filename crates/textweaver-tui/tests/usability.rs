//! The terminal usability pass (the terminal usability research): a
//! yes-or-no question is spoken even while textweaver reads aloud, open
//! failures are plain sentences, the help names the keys a new user needs,
//! and the "no document" message comes from the keymap.

use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::DocKey;
use textweaver_app::testing::{SpeechLog, recording_service};
use textweaver_app::text::Document;
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig, Command, Playback};
use textweaver_tui::Tui;

/// A self-voicing reader with `text` open, and what its voice says.
fn voiced(text: &str) -> (Tui, SpeechLog) {
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
        "Doc".into(),
    );
    (Tui::with_color_support(app, ColorSupport::NoColor), log)
}

/// Waits (with a deadline) until the voice has said something containing
/// `needle`; false when it never does.
fn spoken(log: &SpeechLog, needle: &str) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if log.texts().iter().any(|t| t.contains(needle)) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    false
}

fn plain(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

/// Ctrl+Q while reading aloud: the question is spoken, because the next
/// key answers it. Before the pass it went to the status line only, so a
/// self-voicing user heard the reading go on and the next key vanished.
#[test]
fn a_question_asked_while_reading_is_spoken() {
    let (mut tui, log) = voiced(
        "A first sentence that goes on for a while. A second one follows it. And a third.\n",
    );
    tui.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(tui.app().playback(), Playback::Reading);
    log.clear();
    tui.handle_key(tui.key_for(ActionId::Quit));
    assert!(tui.app().confirmation_pending());
    assert!(
        spoken(&log, "Quit textweaver? y or n"),
        "the question was not spoken: {:?}",
        log.texts()
    );
    // Any other key repeats it aloud too.
    log.clear();
    tui.handle_key(plain('x'));
    assert!(spoken(&log, "Quit textweaver? y or n"), "{:?}", log.texts());
    tui.handle_key(plain('n'));
    assert!(!tui.should_quit());
    assert!(!tui.app().confirmation_pending());

    // Deleting a note at the cursor asks the same way.
    tui.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    tui.dispatch(Command::Notes(textweaver_app::NoteCommand::Add));
    tui.dispatch(Command::Answer("Remember this.".into()));
    tui.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(tui.app().playback(), Playback::Reading);
    log.clear();
    tui.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
    assert!(tui.app().confirmation_pending());
    assert!(
        spoken(&log, "Delete this note or highlight? y or n"),
        "{:?}",
        log.texts()
    );
    tui.handle_key(plain('a'));
    assert_eq!(tui.app().session().unwrap().notes.len(), 1);
}

/// "Could not open" names the file and says why in plain words, with no
/// operating system error code, for a missing file and for a folder.
#[test]
fn open_failures_are_plain_sentences() {
    let (mut tui, _log) = voiced("Text.\n");
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nothere.md");
    tui.dispatch(Command::Open(missing.clone()));
    let status = tui.app().status_text().to_owned();
    assert!(
        status.starts_with("Could not open nothere.md: there is no file named nothere.md in "),
        "{status}"
    );
    assert!(status.ends_with(". Check the name."), "{status}");
    assert!(!status.contains("os error"), "{status}");

    tui.dispatch(Command::Open(dir.path().to_owned()));
    let status = tui.app().status_text().to_owned();
    assert!(
        status.contains("is a folder, not a document. Give the name of a file in it."),
        "{status}"
    );
    // The document that was open stays open.
    assert_eq!(tui.app().session().unwrap().title, "Doc");
}

/// F1 names how to open a document, the quick navigation keys, and the
/// keys a screen reader user needs first.
#[test]
fn the_help_names_the_keys_a_new_user_needs() {
    let (mut tui, _log) = voiced("Text.\n");
    tui.dispatch(Command::Action(ActionId::Help));
    let items = tui.list().expect("the help list").items.clone();
    let has = |needle: &str| items.iter().any(|i| i.contains(needle));
    assert!(
        has("Open a document: Ctrl+O. Library and recent files: Alt+L."),
        "{items:?}"
    );
    assert!(
        has("Heading at a level: 1 to 6, with Shift for the previous one."),
        "{items:?}"
    );
    assert!(
        has(
            "Quick keys, as in NVDA and JAWS: list l, list item i, table t, link k, block quote q, separator s, graphic g, section d."
        ),
        "{items:?}"
    );
    let mode_key = textweaver_app::key_text(tui.app().keymap(), ActionId::CycleAccessMode);
    assert!(
        has(&format!(
            "{mode_key} cycles self-voicing, hybrid, and screen reader mode"
        )),
        "{items:?}"
    );
    assert!(
        has("Single-key shortcuts on or off, for dictation: F9. Settings: Shift+F10."),
        "{items:?}"
    );
    assert!(
        has("Choose a voice: Alt+V. Restart speech if it stops: Shift+F8."),
        "{items:?}"
    );
}

/// Without a document, a reading key names the open key from the keymap:
/// written on the status line, spoken by name by textweaver's voice
/// (Wave 4, W4h).
#[test]
fn the_no_document_message_names_the_open_key() {
    let (speech, log) = recording_service().unwrap();
    let app = App::new(AppConfig {
        speech,
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    let mut tui = Tui::with_color_support(app, ColorSupport::NoColor);
    tui.handle_key(plain('m'));
    assert_eq!(
        tui.app().status_text(),
        "No document is open. Press Ctrl+O to open one."
    );
    assert!(
        spoken(&log, "No document is open. Press Control O to open one."),
        "{:?}",
        log.texts()
    );
}
