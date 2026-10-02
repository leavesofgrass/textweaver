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

/// W8a-f: the browse key in a prompt for a path opens the file browser,
/// and the file chosen fills the prompt for Enter to confirm; Escape in
/// the browser goes back to the prompt as it was.
#[test]
fn the_browse_key_fills_a_prompt_for_a_path() {
    use textweaver_app::path_prompt::browse_key;
    use textweaver_app::{Command, PromptPurpose};
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("notes.md"), "# Notes\n\nCrows.\n").unwrap();
    std::fs::write(dir.path().join("mine.toml"), "[speech]\n").unwrap();
    let mut app = App::new(AppConfig::for_tests());
    app.open(&dir.path().join("notes.md")).unwrap();
    let folder = app
        .session()
        .and_then(|s| s.doc.meta.path.clone())
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap();
    let mut tui = Tui::with_color_support(app, ColorSupport::NoColor);
    tui.app_mut()
        .dispatch(Command::Action(ActionId::ImportSettings));
    // The prompt names the key, and the hint fits 40 Braille cells.
    let hint = format!("{} to browse.", browse_key());
    assert!(hint.chars().count() <= 40);
    assert!(
        tui.app().status_text().ends_with(&hint),
        "{}",
        tui.app().status_text()
    );
    tui.handle_key(plain(KeyCode::Char('x')));
    tui.handle_key(key_event(&browse_key()));
    assert!(tui.app().list_model().is_some());
    assert!(tui.app().prompt_model().is_none());
    // Escape: back to the prompt, with what was typed.
    tui.handle_key(plain(KeyCode::Esc));
    let m = tui.app().prompt_model().expect("the prompt again");
    assert_eq!(
        (m.purpose, m.text()),
        (PromptPurpose::ImportSettings, "x".to_owned())
    );
    // Again, into the document's folder, and the file chosen.
    tui.handle_key(key_event(&browse_key()));
    tui.handle_key(plain(KeyCode::Enter));
    let n = tui
        .app()
        .list_model()
        .unwrap()
        .items
        .iter()
        .position(|i| i.starts_with("mine.toml,"))
        .unwrap();
    tui.app_mut().dispatch(Command::ListFocus(n));
    tui.handle_key(plain(KeyCode::Enter));
    let m = tui.app().prompt_model().expect("the prompt, filled");
    assert_eq!(m.purpose, PromptPurpose::ImportSettings);
    assert_eq!(PathBuf::from(m.text()), folder.join("mine.toml"));
    assert_eq!(tui.app().status_text(), "mine.toml chosen. Enter confirms.");
}
