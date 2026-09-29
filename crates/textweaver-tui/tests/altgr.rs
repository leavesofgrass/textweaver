//! AltGr characters on non-US Windows layouts arrive from crossterm as
//! Control plus Alt; they type text in edit mode and prompts when no
//! binding claims them (the September 2026 audit, finding X1; Agent D4).

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::DocKey;
use textweaver_app::text::Document;
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig, Command, Mode};
use textweaver_tui::Tui;
use textweaver_tui::ui::typed_char;

fn altgr(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL | KeyModifiers::ALT)
}

#[test]
fn which_keys_type() {
    assert_eq!(typed_char(&altgr('@')), Some('@'));
    assert_eq!(typed_char(&altgr('€')), Some('€'));
    assert_eq!(typed_char(&altgr('ą')), Some('ą'));
    assert_eq!(
        typed_char(&KeyEvent::new(
            KeyCode::Char('{'),
            KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT
        )),
        Some('{')
    );
    // Real shortcuts do not type.
    assert_eq!(typed_char(&altgr('e')), None);
    assert_eq!(typed_char(&altgr('7')), None);
    assert_eq!(
        typed_char(&KeyEvent::new(KeyCode::Char('['), KeyModifiers::CONTROL)),
        None
    );
    assert_eq!(
        typed_char(&KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT)),
        None
    );
    assert_eq!(
        typed_char(&KeyEvent::new(KeyCode::Char('x'), KeyModifiers::SHIFT)),
        Some('x')
    );
}

#[test]
fn altgr_characters_type_in_edit_mode_and_prompts() {
    let mut app = App::new(AppConfig::for_tests());
    app.open_document(
        Document::from_plain_text("text\n"),
        DocKey::untitled(1),
        "T".into(),
    );
    let mut tui = Tui::with_color_support(app, ColorSupport::TrueColor);
    tui.dispatch(Command::Action(ActionId::ToggleEditMode));
    assert_eq!(tui.app().mode(), Mode::Edit);
    for c in "[a](b) {x} \\ | @ ~ €".chars() {
        let k = if c.is_ascii_alphanumeric() || c == ' ' || c == '(' || c == ')' {
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
        } else {
            altgr(c)
        };
        tui.handle_key(k);
    }
    let text = tui.app().session().unwrap().doc.text().to_string();
    assert!(text.starts_with("[a](b) {x} \\ | @ ~ €"), "{text:?}");
    // And in a prompt (find).
    tui.dispatch(Command::Action(ActionId::ToggleEditMode));
    if tui.app().mode() == Mode::Edit {
        // Unsaved changes: discard.
        tui.dispatch(Command::Choose(1));
    }
    tui.dispatch(Command::Action(ActionId::Find));
    tui.handle_key(altgr('@'));
    tui.handle_key(altgr('{'));
    assert_eq!(tui.minibuffer().map(|m| m.text()).as_deref(), Some("@{"));
}
