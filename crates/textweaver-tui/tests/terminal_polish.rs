//! Wave 4, Agent W4h (terminal polish): what a screen reader user hears
//! and reads in the terminal reader, from the usability pass's list
//! (docs/research/usability-terminal.md).

use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use textweaver_app::keymap::{ActionId, Frontend, Keymap, Layer, Platform};
use textweaver_app::store::DocKey;
use textweaver_app::testing::{SpeechLog, recording_service};
use textweaver_app::text::Document;
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig, Playback};
use textweaver_tui::Tui;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn with(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, mods)
}

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
        "Essay".into(),
    );
    (Tui::with_color_support(app, ColorSupport::NoColor), log)
}

/// Waits (with a deadline) until the voice has said something containing
/// `needle`; returns that utterance.
fn heard(log: &SpeechLog, needle: &str) -> Option<String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Some(t) = log.texts().into_iter().find(|t| t.contains(needle)) {
            return Some(t);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    None
}

fn tui_with(text: &str) -> Tui {
    let mut app = App::new(AppConfig::for_tests());
    app.open_document(
        Document::from_plain_text(text),
        DocKey::untitled(1),
        "Doc".into(),
    );
    Tui::with_color_support(app, ColorSupport::NoColor)
}

/// The title line as drawn, 100 columns wide.
fn title_line(tui: &mut Tui) -> String {
    let mut term = Terminal::new(TestBackend::new(100, 10)).unwrap();
    term.draw(|f| tui.draw(f)).unwrap();
    let buf = term.backend().buffer();
    (0..100).map(|x| buf[(x, 0)].symbol()).collect()
}

/// Deliverable 1: "Ready" until the first reading, then "Stopped". A
/// screen reader reading the title line at startup heard "Stopped" before
/// anything had been read.
#[test]
fn the_title_line_says_ready_until_the_first_reading() {
    let mut tui = tui_with("One sentence here. Another one there.\n");
    let title = title_line(&mut tui);
    assert!(title.contains("Ready"), "{title}");
    assert!(!title.contains("Stopped"), "{title}");
    assert_eq!(tui.app().reading_state(), "Ready");
    // Space reads; Escape stops.
    tui.handle_key(key(KeyCode::Char(' ')));
    assert!(tui.app().has_read());
    tui.handle_key(key(KeyCode::Esc));
    let title = title_line(&mut tui);
    assert!(title.contains("Stopped"), "{title}");
    assert!(!title.contains("Ready"), "{title}");
}

/// Deliverable 5: the two new actions have keys in both frontends that
/// reach them in browse mode, a chord that works with single-key
/// shortcuts off, and palette entries.
#[test]
fn say_status_and_repeat_message_have_keys_everywhere() {
    for frontend in [Frontend::Terminal, Frontend::Gui] {
        let map = Keymap::defaults(Platform::current(), frontend);
        for (action, chord, single) in [
            (ActionId::SayStatus, "Alt+End", "z"),
            (ActionId::RepeatMessage, "Alt+'", "'"),
        ] {
            let chord = chord.parse().unwrap();
            let single = single.parse().unwrap();
            assert_eq!(
                map.lookup(&chord, Layer::Browse),
                Some(action),
                "{frontend:?}"
            );
            assert_eq!(
                map.lookup(&chord, Layer::Edit),
                Some(action),
                "{frontend:?}"
            );
            assert_eq!(
                map.lookup(&single, Layer::Browse),
                Some(action),
                "{frontend:?}"
            );
            let mut off = map.clone();
            off.set_character_keys(false);
            assert_eq!(
                off.lookup(&chord, Layer::Browse),
                Some(action),
                "{frontend:?}"
            );
        }
    }
    assert_eq!(
        textweaver_app::resolve_command("say status"),
        Some(ActionId::SayStatus)
    );
    assert_eq!(
        textweaver_app::resolve_command("repeat message"),
        Some(ActionId::RepeatMessage)
    );
}

/// Deliverable 5: Say Status says the last message, then the title
/// line's parts; Repeat Message says the last message again. Both are
/// heard over the reading, which goes on.
#[test]
fn say_status_and_repeat_message_are_heard() {
    let (mut tui, log) = voiced("One sentence here. Another one there. And a third one.\n");
    // Where am I, to have a message to repeat.
    tui.handle_key(with(KeyCode::Char('W'), KeyModifiers::SHIFT));
    let position = heard(&log, "Line 1 of").expect("the position");
    log.clear();
    tui.handle_key(key(KeyCode::Char('\'')));
    let again = heard(&log, "Line 1 of").expect("the message again");
    assert_eq!(again, position);
    log.clear();
    tui.handle_key(with(KeyCode::End, KeyModifiers::ALT));
    let status = heard(&log, "Browse mode").expect("the status");
    assert!(status.starts_with(&position), "{status}");
    for part in [
        "Essay: Browse mode",
        "Ready",
        "line 1 of 1, zero percent",
        "self-voicing",
        "words per minute",
        "test-recording.",
    ] {
        assert!(status.contains(part), "{part}: {status}");
    }
    // Say Status twice does not say the status twice over, and Repeat
    // Message after it repeats the message, not the status.
    log.clear();
    tui.handle_key(key(KeyCode::Char('z')));
    let twice = heard(&log, "Browse mode").expect("the status again");
    assert_eq!(twice, status);
    log.clear();
    tui.handle_key(with(KeyCode::Char('\''), KeyModifiers::ALT));
    assert_eq!(heard(&log, "Line 1 of").as_deref(), Some(position.as_str()));
    // While reading: heard over the reading, which goes on.
    tui.handle_key(key(KeyCode::Char(' ')));
    assert_eq!(tui.app().playback(), Playback::Reading);
    log.clear();
    tui.handle_key(with(KeyCode::End, KeyModifiers::ALT));
    let reading = heard(&log, "Reading,").expect("the status while reading");
    assert!(reading.contains("Browse mode"), "{reading}");
    assert_eq!(tui.app().playback(), Playback::Reading);
}

/// Without a document the actions still work (no "No document is open").
#[test]
fn say_status_needs_no_document() {
    let (speech, log) = recording_service().unwrap();
    let app = App::new(AppConfig {
        speech,
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    let mut tui = Tui::with_color_support(app, ColorSupport::NoColor);
    tui.handle_key(with(KeyCode::End, KeyModifiers::ALT));
    let status = heard(&log, "No document").expect("the status");
    assert!(
        status.starts_with("No document: Browse mode, Ready"),
        "{status}"
    );
    log.clear();
    tui.handle_key(key(KeyCode::Char('\'')));
    assert!(
        heard(&log, "No message yet.").is_some(),
        "{:?}",
        log.texts()
    );
}

/// Deliverable 6: Escape in edit mode with nothing playing says how to
/// finish, with the key from the keymap: spoken by name, written on the
/// status line.
#[test]
fn escape_in_edit_mode_says_how_to_finish() {
    let (mut tui, log) = voiced(
        "Some text to edit.
",
    );
    tui.handle_key(with(KeyCode::Char('e'), KeyModifiers::CONTROL));
    assert_eq!(tui.app().mode(), textweaver_app::Mode::Edit);
    log.clear();
    tui.handle_key(key(KeyCode::Esc));
    assert_eq!(tui.app().mode(), textweaver_app::Mode::Edit);
    assert_eq!(
        heard(&log, "Still editing").as_deref(),
        Some("Still editing. Control E finishes.")
    );
    assert_eq!(tui.app().status_text(), "Still editing. Ctrl+E finishes.");
    // Out of edit mode, Escape says nothing of the kind.
    tui.handle_key(with(KeyCode::Char('e'), KeyModifiers::CONTROL));
    assert_eq!(tui.app().mode(), textweaver_app::Mode::Browse);
    tui.handle_key(key(KeyCode::Esc));
    assert!(!tui.app().status_text().contains("Still editing"));
}

/// Deliverable 7: the command palette's opening sentence says how to use
/// it; the label drawn on the bottom line stays one word.
#[test]
fn the_command_palette_says_how_to_use_it() {
    let (mut tui, log) = voiced("Text.\n");
    tui.handle_key(key(KeyCode::F(2)));
    let said = heard(&log, "Command.").expect("the palette's sentence");
    assert_eq!(
        said,
        "Command. Type part of a name; Tab completes, Up and Down list matches."
    );
    let prompt = tui.app().prompt_model().expect("the palette is open");
    assert_eq!(prompt.label, "Command");
    let mut term = Terminal::new(TestBackend::new(100, 10)).unwrap();
    term.draw(|f| tui.draw(f)).unwrap();
    let buf = term.backend().buffer();
    let bottom: String = (0..100).map(|x| buf[(x, 9)].symbol()).collect();
    assert!(bottom.trim_start().starts_with("Command"), "{bottom}");
    assert!(!bottom.contains("Tab completes"), "{bottom}");
}
