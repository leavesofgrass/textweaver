//! Terminal parts of the Phase 1 quick wins (Agent P1b): copying through
//! OSC 52, path completion in prompts, first-letter jumps and s, d, c in
//! lists, key hints that match the mode and F9, and drawing at narrow
//! widths.

use std::path::Path;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::DocKey;
use textweaver_app::text::Document;
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig, Command, Mode};
use textweaver_tui::Tui;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ch(c: char) -> KeyEvent {
    let mods = if c.is_ascii_uppercase() {
        KeyModifiers::SHIFT
    } else {
        KeyModifiers::NONE
    };
    KeyEvent::new(KeyCode::Char(c), mods)
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

fn tui_opening(file: &Path) -> Tui {
    let mut app = App::new(AppConfig::for_tests());
    app.open(file).unwrap();
    Tui::with_color_support(app, ColorSupport::NoColor)
}

#[test]
fn copy_sends_the_text_to_the_terminal_clipboard() {
    let mut tui = tui_with("Copy me. Not me.\n");
    assert_eq!(tui.take_clipboard_sequence(), None);
    tui.handle_key(tui.key_for(ActionId::Copy));
    assert_eq!(
        tui.take_clipboard_sequence().as_deref(),
        Some("\u{1b}]52;c;Q29weSBtZS4=\u{7}")
    );
    assert_eq!(tui.take_clipboard_sequence(), None);
}

#[test]
fn tab_completes_paths_in_file_prompts() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("chapter-one.md"), "One.\n").unwrap();
    std::fs::write(dir.path().join("notes.md"), "Notes.\n").unwrap();
    let mut tui = tui_with("Text.\n");
    tui.handle_key(tui.key_for(ActionId::Open));
    assert_eq!(tui.app().mode(), Mode::Open);
    let typed = format!("{}/chap", dir.path().display());
    for c in typed.chars() {
        tui.handle_key(ch(c));
    }
    tui.handle_key(key(KeyCode::Tab));
    let text = tui.minibuffer().unwrap().text();
    assert!(text.ends_with("chapter-one.md"), "{text}");
    assert_eq!(tui.app().status_text(), "chapter-one.md, file");
    tui.handle_key(key(KeyCode::Enter));
    assert_eq!(tui.app().session().unwrap().title, "chapter-one.md");
}

#[test]
fn lists_jump_by_first_letter_and_the_save_list_takes_s_d_c() {
    let mut tui = tui_with("Words.\n");
    tui.dispatch(Command::Action(ActionId::KeyboardHelp));
    let all = tui.list().unwrap().items.len();
    // The keyboard list filters as you type, as Settings does (beta 1).
    for c in "voice".chars() {
        tui.handle_key(ch(c));
    }
    let list = tui.list().unwrap();
    assert!(list.items.len() < all, "{}", list.items.len());
    assert!(
        list.items.iter().any(|i| i.starts_with("Voice")),
        "{:?}",
        list.items
    );
    // "1 of 12, ...": the place first (Wave 5, the Braille pass).
    assert!(
        tui.app().status_text().contains(" of "),
        "{}",
        tui.app().status_text()
    );
    // q and j are letters in a list now, not close and down.
    tui.handle_key(ch('q'));
    assert!(tui.list().is_some());
    tui.handle_key(key(KeyCode::Esc));
    assert!(tui.list().is_none());

    // s saves at once from the Save, Discard, Cancel list.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("s.md");
    std::fs::write(&file, "Start.\n").unwrap();
    let mut tui = tui_opening(&file);
    tui.handle_key(tui.key_for(ActionId::ToggleEditMode));
    tui.handle_key(ch('X'));
    tui.handle_key(tui.key_for(ActionId::ToggleEditMode));
    assert!(tui.list().is_some());
    tui.handle_key(ch('s'));
    assert!(tui.list().is_none());
    // The save is written by the background writer; the next tick leaves
    // edit mode.
    tui.app_mut()
        .flush_writes(std::time::Duration::from_secs(10));
    tui.tick();
    assert!(!tui.app().is_editing());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "XStart.\n");
    // d discards, c cancels.
    tui.handle_key(tui.key_for(ActionId::ToggleEditMode));
    tui.handle_key(ch('Y'));
    tui.handle_key(tui.key_for(ActionId::ToggleEditMode));
    tui.handle_key(ch('c'));
    assert!(tui.app().is_editing());
    tui.handle_key(tui.key_for(ActionId::ToggleEditMode));
    tui.handle_key(ch('d'));
    assert!(!tui.app().is_editing());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "XStart.\n");
}

#[test]
fn key_hints_show_only_keys_that_work() {
    let mut tui = tui_with("Words here.\n");
    let on = tui.hints(200);
    // The toolbar's buttons come first, by their short names (B1-cb).
    assert!(on.contains("Space Play or pause"), "{on}");
    assert!(on.contains("h heading"), "{on}");
    // F9: single keys off; the hints switch to chords, and actions with
    // only single keys drop out.
    tui.handle_key(key(KeyCode::F(9)));
    let off = tui.hints(200);
    // The keys come from the keymap, as the hints take them.
    let keymap = tui.app().keymap().clone();
    let chord = |a: ActionId| {
        keymap
            .chords_for(a)
            .into_iter()
            .find(|c| !c.is_text_input())
            .map(|c| c.to_string())
            .unwrap_or_else(|| panic!("{a:?} has a chord"))
    };
    assert!(
        off.contains(&format!("{} Play or pause", chord(ActionId::PlayPause))),
        "{off}"
    );
    assert!(
        off.contains(&format!(
            "{} Next paragraph",
            chord(ActionId::NextParagraph)
        )),
        "{off}"
    );
    // Headings have a chord in the terminal since Phase 2.
    assert!(
        off.contains(&format!("{} heading", chord(ActionId::SkipNextHeading))),
        "{off}"
    );
    assert!(!off.contains("h heading"), "{off}");
    assert!(!off.contains("Space"), "{off}");
    // Edit mode shows edit keys, not browse keys.
    tui.handle_key(tui.key_for(ActionId::ToggleEditMode));
    let edit = tui.hints(200);
    let save = chord(ActionId::Save);
    assert!(edit.contains(&format!("{save} save")), "{edit}");
    assert!(edit.contains("F2 commands"), "{edit}");
    assert!(!edit.contains("?"), "{edit}");
}

/// The browse hints follow `[gui] toolbar_buttons` (B1-cb): a toolbar of
/// sentence steps shows them, and the terminal's own keys follow.
#[test]
fn key_hints_follow_the_toolbar_buttons() {
    let mut config = AppConfig::for_tests();
    config.settings.gui.toolbar_buttons = vec!["next_sentence".into(), "from_the_future".into()];
    let mut app = App::new(config);
    app.open_document(
        Document::from_plain_text(
            "Words here.
",
        ),
        DocKey::untitled(1),
        "Doc".into(),
    );
    let tui = Tui::with_color_support(app, ColorSupport::NoColor);
    let hints = tui.hints(200);
    assert!(hints.contains("Next sentence"), "{hints}");
    assert!(!hints.contains("Next paragraph"), "{hints}");
    assert!(!hints.contains("Play or pause"), "{hints}");
    assert!(hints.contains("find"), "{hints}");
}

/// A long heading, a wide table, and a long code line drawn at 20, 40, 60,
/// and 80 columns: nothing panics, no row is wider than the screen, the
/// cursor stays on screen, and the status and hint lines are there.
#[test]
fn narrow_widths_draw_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("wide.md");
    let text = format!(
        "# {}\n\n| {} |\n| {} |\n| {} |\n\n```\n{}\n```\n\nLast paragraph.\n",
        "A heading that goes on and on well past any narrow screen width",
        (1..=12)
            .map(|i| format!("Column {i}"))
            .collect::<Vec<_>>()
            .join(" | "),
        (1..=12).map(|_| "---").collect::<Vec<_>>().join(" | "),
        (1..=12)
            .map(|i| format!("value {i}"))
            .collect::<Vec<_>>()
            .join(" | "),
        "let x = compute(alpha, beta, gamma, delta, epsilon, zeta, eta, theta, iota, kappa);",
    );
    std::fs::write(&file, text).unwrap();
    for width in [20u16, 40, 60, 80] {
        let mut tui = tui_opening(&file);
        let mut term = Terminal::new(TestBackend::new(width, 12)).unwrap();
        for step in 0..6 {
            term.draw(|f| tui.draw(f)).unwrap();
            let buf = term.backend().buffer();
            assert_eq!(buf.area.width, width);
            let cursor = term.backend().cursor_position();
            assert!(
                cursor.x < width && cursor.y < 12,
                "{width}: cursor {cursor:?} at step {step}"
            );
            let rows: Vec<String> = (0..12)
                .map(|y| (0..width).map(|x| buf[(x, y)].symbol()).collect())
                .collect();
            assert!(
                rows[0].contains("textweaver"),
                "{width}: title {:?}",
                rows[0]
            );
            let hints = &rows[11];
            assert!(!hints.trim().is_empty(), "{width}: no hints");
            tui.handle_key(key(KeyCode::Down));
        }
        // Edit mode draws the source at every width too.
        tui.handle_key(tui.key_for(ActionId::ToggleEditMode));
        term.draw(|f| tui.draw(f)).unwrap();
        let c = term.backend().cursor_position();
        assert!(c.x < width, "{width}: {c:?}");
    }
}
