//! The event loop draws a highlight step as soon as it arrives
//! (docs/history/audit-2026-09.md, finding R3; Agent D4).

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
