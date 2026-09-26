//! Wave 3 (Agent W3a): the settings screen in the terminal reader, on the
//! app's settings schema and list model. Shift+F10 opens it; typing
//! filters; Left and Right change a value; Enter asks for one; Escape
//! closes. It draws at a narrow width with the focused item in view.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use textweaver_app::store::DocKey;
use textweaver_app::text::Document;
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig, PromptPurpose};
use textweaver_tui::Tui;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn tui() -> Tui {
    let mut app = App::new(AppConfig::for_tests());
    app.open_document(
        Document::from_plain_text("Some text.\n"),
        DocKey::untitled(1),
        "Doc".into(),
    );
    Tui::with_color_support(app, ColorSupport::NoColor)
}

fn type_str(tui: &mut Tui, s: &str) {
    for c in s.chars() {
        tui.handle_key(key(KeyCode::Char(c)));
    }
}

fn screen(tui: &mut Tui, w: u16, h: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
    terminal.draw(|f| tui.draw(f)).unwrap();
    let buf = terminal.backend().buffer().clone();
    (0..h)
        .map(|y| {
            (0..w)
                .map(|x| buf[(x, y)].symbol().to_owned())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn shift_f10_opens_the_settings_and_the_arrows_change_them() {
    let mut tui = tui();
    tui.handle_key(KeyEvent::new(KeyCode::F(10), KeyModifiers::SHIFT));
    let list = tui.list().expect("the settings list");
    assert_eq!(list.title, "Settings");
    assert!(
        list.items[0].starts_with("Speech engine: "),
        "{:?}",
        list.items[0]
    );
    // Filter to "rate", then pick "Rate".
    type_str(&mut tui, "words per");
    let items = tui.list().unwrap().items.clone();
    let i = items.iter().position(|x| x.starts_with("Rate: ")).unwrap();
    for _ in 0..i {
        tui.handle_key(key(KeyCode::Down));
    }
    tui.handle_key(key(KeyCode::Right));
    assert_eq!(tui.app().settings().speech.rate.wpm(), 285);
    assert_eq!(
        tui.list().unwrap().items[i],
        "Rate: 285 words per minute",
        "the list shows the new value, on the same item"
    );
    assert_eq!(tui.list().unwrap().selected, i);
    tui.handle_key(key(KeyCode::Left));
    tui.handle_key(key(KeyCode::Left));
    assert_eq!(tui.app().settings().speech.rate.wpm(), 245);
    // Enter types a value.
    tui.handle_key(key(KeyCode::Enter));
    let mb = tui.minibuffer().expect("a value prompt");
    assert_eq!(mb.purpose, PromptPurpose::SettingValue);
    assert_eq!(mb.text(), "245");
    for _ in 0..3 {
        tui.handle_key(key(KeyCode::Backspace));
    }
    type_str(&mut tui, "350");
    tui.handle_key(key(KeyCode::Enter));
    assert_eq!(tui.app().settings().speech.rate.wpm(), 350);
    assert_eq!(tui.list().unwrap().selected, i, "back on the same item");
    // It draws at 40 columns, the focused item in view.
    let shown = screen(&mut tui, 40, 12);
    assert!(shown.contains("Rate: 350"), "{shown}");
    // Escape closes it.
    tui.handle_key(key(KeyCode::Esc));
    assert!(tui.list().is_none());
    assert_eq!(tui.app().status_text(), "Settings closed.");
}
