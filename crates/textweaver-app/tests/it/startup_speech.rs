//! Messages said before the speech engine is ready (Wave 4, W4h): the
//! engine starts on a helper thread, and what was said meanwhile went to
//! the silent service, so a self-voicing user heard nothing but the last
//! status line. They are kept and said once the engine reports, in order,
//! after "Opened".

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use textweaver_app::a11y::Priority;
use textweaver_app::keymap::ActionId;
use textweaver_app::store::DocKey;
use textweaver_app::testing::{SpeechLog, recording_service};
use textweaver_app::text::Document;
use textweaver_app::{App, AppConfig, Command};

/// A self-voicing app whose engine starts when `go` is sent; the engine's
/// recording log arrives on the returned receiver.
fn starting_app(
    engine_messages: Vec<String>,
) -> (App, mpsc::Sender<()>, mpsc::Receiver<SpeechLog>) {
    let (go_tx, go_rx) = mpsc::channel::<()>();
    let (log_tx, log_rx) = mpsc::channel();
    let mut app = App::new(AppConfig {
        speech: textweaver_app::speech::SpeechService::null(),
        self_voicing: true,
        backend_name: "starting".into(),
        ..AppConfig::for_tests()
    });
    let go = Mutex::new(go_rx);
    let log_tx = Mutex::new(log_tx);
    app.start_speech_in_background(Arc::new(move |_settings| {
        // An engine that takes as long as the test wants.
        if let Ok(go) = go.lock() {
            let _ = go.recv_timeout(Duration::from_secs(60));
        }
        let (service, log) = recording_service().unwrap();
        let _ = log_tx.lock().map(|t| t.send(log));
        (service, "test-recording".into(), engine_messages.clone())
    }));
    (app, go_tx, log_rx)
}

/// Waits (with a deadline) until the log holds `n` utterances.
fn wait_for(log: &SpeechLog, n: usize) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let texts = log.texts();
        if texts.len() >= n || Instant::now() >= deadline {
            return texts;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn messages_before_the_engine_is_ready_are_said_in_order_after_opened() {
    let (mut app, go, logs) = starting_app(vec![
        "Error: Speech engine eci is not available; using test-recording.".into(),
    ]);
    assert!(app.speech_restarting());
    app.open_document(
        Document::from_plain_text("Hello there. A second sentence."),
        DocKey::untitled(7),
        "Essay".into(),
    );
    // What the terminal reader queues after "Opened" at startup, then a
    // key pressed while the engine starts.
    app.announce_queued("Settings file ignored: bad value.", Priority::Assertive);
    let welcome = format!(
        "Welcome to textweaver. {} quits.",
        textweaver_app::named_key(app.keymap(), ActionId::Quit)
    );
    app.announce_queued(&welcome, Priority::Polite);
    app.dispatch(Command::Action(ActionId::SayPosition));
    // Nothing is lost while the engine starts; the status line has it.
    assert!(
        app.status_text().contains("Line 1 of 1"),
        "{}",
        app.status_text()
    );
    go.send(()).unwrap();
    assert!(app.wait_for_speech_start(Duration::from_secs(30)));
    let log = logs.recv_timeout(Duration::from_secs(5)).unwrap();
    let said = wait_for(&log, 5);
    assert_eq!(said.len(), 5, "{said:?}");
    assert!(said[0].starts_with("Opened Essay"), "{said:?}");
    assert_eq!(said[1], "Settings file ignored: bad value.");
    // Keys named in their spoken form.
    assert_eq!(said[2], "Welcome to textweaver. Control Q quits.");
    assert!(said[3].starts_with("Line 1 of 1"), "{said:?}");
    assert_eq!(
        said[4],
        "Error: Speech engine eci is not available; using test-recording."
    );
    // Afterwards messages are spoken at once, as always.
    app.dispatch(Command::Action(ActionId::RepeatMessage));
    let said = wait_for(&log, 6);
    assert_eq!(said.len(), 6, "{said:?}");
}

#[test]
fn a_reading_started_meanwhile_goes_on_instead() {
    let (mut app, go, logs) = starting_app(Vec::new());
    app.open_document(
        Document::from_plain_text("Hello there. A second sentence."),
        DocKey::untitled(8),
        "Essay".into(),
    );
    app.dispatch(Command::Action(ActionId::PlayPause));
    go.send(()).unwrap();
    assert!(app.wait_for_speech_start(Duration::from_secs(30)));
    let log = logs.recv_timeout(Duration::from_secs(5)).unwrap();
    let said = wait_for(&log, 1);
    assert!(
        said.first().is_some_and(|t| t.contains("Hello there")),
        "{said:?}"
    );
    assert!(!said.iter().any(|t| t.starts_with("Opened")), "{said:?}");
}

/// Nothing reads aloud on its own when a document opens (WCAG 1.4.2; the
/// conventions research, QW7): with the default settings opening says
/// only "Opened", and reading starts only with `[speech] auto_play`.
#[test]
fn opening_a_document_says_only_opened_unless_auto_play_is_on() {
    for auto_play in [false, true] {
        let (speech, log) = recording_service().unwrap();
        let mut config = AppConfig::for_tests();
        config.settings.speech.auto_play = auto_play;
        let mut app = App::new(AppConfig {
            speech,
            self_voicing: true,
            ..config
        });
        app.open_document(
            Document::from_plain_text("Hello there. A second sentence."),
            DocKey::untitled(9),
            "Essay".into(),
        );
        let said = if auto_play {
            wait_for(&log, 2)
        } else {
            // Give a reading that should not start time to show itself.
            wait_for(&log, 1);
            std::thread::sleep(Duration::from_millis(300));
            log.texts()
        };
        let read = said.iter().any(|t| t.contains("Hello there"));
        assert_eq!(read, auto_play, "auto_play {auto_play}: {said:?}");
        if !auto_play {
            assert_eq!(said.len(), 1, "{said:?}");
            assert!(said[0].starts_with("Opened Essay"), "{said:?}");
        }
    }
}
