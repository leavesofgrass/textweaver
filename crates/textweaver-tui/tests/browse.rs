//! The file browser in the terminal (Wave 6, W6f): the Say Status key
//! previews the focused row, and the browser's own keys arrive as its list
//! keys. Keys come from the keymap and from the app's browser keys, never
//! written here.

use std::path::{Path, PathBuf};
use std::time::Duration;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use textweaver_app::browse::{BrowseKey, SortBy, command_modifier};
use textweaver_app::keymap::ActionId;
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig};
use textweaver_tui::{Tui, key_event};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/f")
        .canonicalize()
        .unwrap()
}

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn browsing() -> Tui {
    let mut app = App::new(AppConfig::for_tests());
    app.open(&fixtures().join("readme.md")).unwrap();
    let mut tui = Tui::with_color_support(app, ColorSupport::NoColor);
    tui.app_mut()
        .dispatch(textweaver_app::Command::Action(ActionId::BrowseFiles));
    // Enter the document's folder, the first place.
    tui.handle_key(plain(KeyCode::Enter));
    tui
}

#[test]
fn the_sort_key_sorts_in_the_browser() {
    let mut tui = browsing();
    let command = command_modifier(tui.app().keymap());
    tui.handle_key(key_event(&BrowseKey::Sort.chord(command)));
    assert_eq!(tui.app().browse_sort(), SortBy::Date);
    assert!(tui.app().status_text().starts_with("Sorted by date"));
}

#[test]
fn say_status_previews_the_focused_row() {
    let mut tui = browsing();
    let n = tui
        .app()
        .list_model()
        .unwrap()
        .items
        .iter()
        .position(|i| i.starts_with("readme.md,"))
        .unwrap();
    tui.app_mut()
        .dispatch(textweaver_app::Command::ListFocus(n));
    tui.handle_key(tui.key_for(ActionId::SayStatus));
    assert!(tui.app_mut().wait_for_preview(Duration::from_secs(20)));
    assert_eq!(
        tui.app().status_text(),
        "File browser fixtures. These files let you try the file browser without your own documents."
    );
}
