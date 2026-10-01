//! The event loop draws a highlight step as soon as it arrives
//! (the September 2026 audit, finding R3; Agent D4).

use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use textweaver_app::keymap::ActionId;
use textweaver_app::store::DocKey;
use textweaver_app::testing::recording_service;
use textweaver_app::text::Document;
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig, Command, Playback};
use textweaver_tui::{IDLE_POLL, READING_POLL, Tui, frame, input_wait};

fn rendered(term: &Terminal<TestBackend>) -> String {
    let buf = term.backend().buffer();
    let area = buf.area;
    let mut out = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            let c = &buf[(x, y)];
            out.push_str(c.symbol());
            out.push_str(&format!("{:?}", c.bg));
        }
        out.push('\n');
    }
    out
}

#[test]
fn a_frame_applies_speech_status_before_drawing() {
    let (speech, log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    let text = "One two three four. Five six seven eight. Nine ten eleven.\n".repeat(20);
    app.open_document(
        Document::from_plain_text(&text),
        DocKey::untitled(1),
        "T".into(),
    );
    let mut tui = Tui::with_color_support(app, ColorSupport::TrueColor);
    let mut term = Terminal::new(TestBackend::new(80, 24)).unwrap();
    term.draw(|f| tui.draw(f)).unwrap();
    tui.dispatch(Command::Action(ActionId::ReadFromCursor));
    assert!(tui.app().session().unwrap().spoken.is_none());
    // Run frames (with a deadline) until one applies word positions.
    let deadline = Instant::now() + Duration::from_secs(10);
    while tui.app().spoken_log().is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
        frame(&mut term, &mut tui).unwrap();
    }
    assert!(!log.text_utterances().is_empty(), "nothing was spoken");
    assert!(
        !tui.app().spoken_log().is_empty(),
        "no frame applied word positions"
    );
    // What the frame left on screen is the state after the statuses: drawing
    // again without applying anything changes nothing. (Drawing before
    // applying status left the previous state on screen until the next
    // pass, up to 40 ms later.)
    let after_frame = rendered(&term);
    let cursor = term.backend().cursor_position();
    term.draw(|f| tui.draw(f)).unwrap();
    assert_eq!(after_frame, rendered(&term));
    assert_eq!(cursor, term.backend().cursor_position());
}

#[test]
fn the_loop_waits_briefly_while_reading() {
    let (speech, _log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        ..AppConfig::for_tests()
    });
    let now = Instant::now();
    assert_eq!(input_wait(&app, now), IDLE_POLL);
    app.open_document(
        Document::from_plain_text(&"Some words to read aloud. ".repeat(2000)),
        DocKey::untitled(1),
        "T".into(),
    );
    app.dispatch(Command::Action(ActionId::ReadFromCursor));
    assert_eq!(app.playback(), Playback::Reading);
    assert_eq!(input_wait(&app, now), READING_POLL);
    assert!(READING_POLL <= Duration::from_millis(10));
}

/// The loop draws only when something could have changed (W6u): after a
/// command, a key, a message from the app, or a resize; while idle it
/// draws at most once a second.
#[test]
fn the_loop_draws_only_when_something_changed() {
    use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use textweaver_app::a11y::Priority;
    use textweaver_tui::REDRAW_AT_LEAST;
    let (speech, _log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    app.open_document(
        Document::from_plain_text(&"One two three. Four five six.\n".repeat(50)),
        DocKey::untitled(1),
        "T".into(),
    );
    let mut tui = Tui::with_color_support(app, ColorSupport::TrueColor);
    let mut term = Terminal::new(TestBackend::new(80, 24)).unwrap();
    frame(&mut term, &mut tui).unwrap();
    let first = tui.draws();
    assert_eq!(first, 1, "the first frame draws");
    // Idle: nothing changes, nothing is drawn.
    for _ in 0..20 {
        frame(&mut term, &mut tui).unwrap();
    }
    assert_eq!(tui.draws(), first, "idle frames drew");
    // A command draws once.
    tui.dispatch(Command::Action(ActionId::NextSentence));
    frame(&mut term, &mut tui).unwrap();
    frame(&mut term, &mut tui).unwrap();
    assert_eq!(tui.draws(), first + 1);
    // A key event draws.
    tui.handle_event(&Event::Key(KeyEvent::new(
        KeyCode::Down,
        KeyModifiers::NONE,
    )));
    frame(&mut term, &mut tui).unwrap();
    assert_eq!(tui.draws(), first + 2);
    // A message the app says by itself (not through the terminal reader)
    // draws: the status line changed.
    tui.app_mut().announce("Saved at last.", Priority::Polite);
    frame(&mut term, &mut tui).unwrap();
    assert_eq!(tui.draws(), first + 3);
    let buf = term.backend().buffer();
    let screen: String = (0..buf.area.height)
        .flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
        .map(|(x, y)| buf[(x, y)].symbol().to_owned())
        .collect();
    assert!(
        screen.contains("Saved at last."),
        "the message is on screen"
    );
    // A resize draws.
    tui.handle_event(&Event::Resize(100, 30));
    frame(&mut term, &mut tui).unwrap();
    assert_eq!(tui.draws(), first + 4);
    // At most a second passes without a draw.
    let now = Instant::now();
    assert!(tui.draw_due_in(now) <= REDRAW_AT_LEAST);
    assert!(tui.wants_draw(false, now + REDRAW_AT_LEAST));
}
