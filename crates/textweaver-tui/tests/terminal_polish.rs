//! Wave 4, Agent W4h (terminal polish): what a screen reader user hears
//! and reads in the terminal reader, from the usability pass's list
//! (docs/research/usability-terminal.md).

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use textweaver_app::store::DocKey;
use textweaver_app::text::Document;
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig};
use textweaver_tui::Tui;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
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
