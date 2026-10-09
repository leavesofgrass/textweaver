//! Copy, cut, and paste end to end with a fake clipboard (beta 1, B1-cm):
//! formatted text pastes as Markdown, Paste as plain text keeps only the
//! text, every paste is one undo step, large formatted pastes are
//! converted off the input thread, and the context menu lists the
//! commands that fit where the cursor is.

use std::time::{Duration, Instant};

use textweaver_app::core::Direction;
use textweaver_app::keymap::ActionId;
use textweaver_app::menu::MenuItemKind;
use textweaver_app::store::DocKey;
use textweaver_app::text::Document;
use textweaver_app::{
    App, AppConfig, CaretMove, ClipboardContents, Command, Effect, FakeClipboard, ListKey,
};

const SOURCE: &str = "First sentence here. Second one.\n";

/// A silent app with `text` open and a fake clipboard.
fn app_with(text: &str) -> (App, FakeClipboard) {
    let mut app = App::new(AppConfig::for_tests());
    app.open_document(
        Document::from_plain_text(text),
        DocKey::untitled(1),
        "T".into(),
    );
    let clip = FakeClipboard::new();
    app.set_clipboard(Box::new(clip.clone()));
    (app, clip)
}

fn act(app: &mut App, a: ActionId) -> Vec<Effect> {
    app.dispatch(Command::Action(a))
}

fn source(app: &App) -> String {
    app.session().unwrap().doc.text().to_string()
}

fn to_end(app: &mut App) {
    app.dispatch(Command::MoveCaret {
        by: CaretMove::DocumentEdge,
        direction: Direction::Forward,
        extend: false,
    });
}

fn html(html: &str) -> ClipboardContents {
    ClipboardContents {
        text: Some("plain words".into()),
        html: Some(html.into()),
        rtf: None,
    }
}

#[test]
fn copy_and_paste_round_trip_through_the_clipboard() {
    let (mut app, clip) = app_with(SOURCE);
    // Reading: Copy takes the sentence at the cursor and says so.
    act(&mut app, ActionId::Copy);
    let copied = app.take_clipboard().unwrap();
    assert_eq!(copied, "First sentence here.");
    assert_eq!(
        app.status_text(),
        "Copied the sentence: First sentence here."
    );
    // Reading: Paste says how to start editing.
    act(&mut app, ActionId::Paste);
    assert!(
        app.status_text().starts_with("Turn on edit mode"),
        "{}",
        app.status_text()
    );
    // The frontend put the copy on the system clipboard; Paste reads it.
    clip.set(ClipboardContents::text(copied));
    act(&mut app, ActionId::ToggleEditMode);
    to_end(&mut app);
    act(&mut app, ActionId::Paste);
    assert_eq!(source(&app), format!("{SOURCE}First sentence here."));
    assert_eq!(
        app.status_text(),
        "Pasted 20 characters: First sentence here."
    );
    act(&mut app, ActionId::Undo);
    assert_eq!(source(&app), SOURCE);
    // Cut everything, then paste it back.
    act(&mut app, ActionId::SelectAll);
    act(&mut app, ActionId::Cut);
    assert_eq!(app.take_clipboard().as_deref(), Some(SOURCE));
    assert_eq!(source(&app), "");
    clip.set(ClipboardContents::text(SOURCE));
    act(&mut app, ActionId::Paste);
    assert_eq!(source(&app), SOURCE);
}

#[test]
fn lines_are_counted_and_line_ends_are_kept_plain() {
    let (mut app, clip) = app_with(SOURCE);
    act(&mut app, ActionId::ToggleEditMode);
    to_end(&mut app);
    clip.set(ClipboardContents::text("one\r\ntwo\r\nthree"));
    act(&mut app, ActionId::Paste);
    assert_eq!(source(&app), format!("{SOURCE}one\ntwo\nthree"));
    assert_eq!(app.status_text(), "Pasted 3 lines: one two three");
}

#[test]
fn html_pastes_as_markdown_in_one_undo_step() {
    let (mut app, clip) = app_with(SOURCE);
    act(&mut app, ActionId::ToggleEditMode);
    to_end(&mut app);
    clip.set(html(
        "<h2>Results</h2><p>The <em>first</em> finding.</p>\
         <p>See <a href=\"https://example.org/\">the data</a>.</p>\
         <ul><li>one</li><li>two</li></ul>\
         <table><tr><th>Name</th><th>Value</th></tr><tr><td>a</td><td>1</td></tr></table>\
         <pre><code>let x = 1;</code></pre>",
    ));
    act(&mut app, ActionId::Paste);
    let text = source(&app);
    let pasted = text.strip_prefix(SOURCE).unwrap();
    for part in [
        "## Results",
        "The *first* finding.",
        "[the data](https://example.org/)",
        "- one\n- two",
        "| Name | Value |",
        "let x = 1;",
    ] {
        assert!(pasted.contains(part), "{part:?} missing from {pasted:?}");
    }
    let said = app.status_text();
    assert!(said.starts_with("Pasted as Markdown: 1 heading,"), "{said}");
    assert!(said.contains("1 list"), "{said}");
    assert!(said.contains("1 table"), "{said}");
    assert!(said.contains("1 code block"), "{said}");
    act(&mut app, ActionId::Undo);
    assert_eq!(source(&app), SOURCE);
}

#[test]
fn rtf_pastes_as_markdown() {
    let (mut app, clip) = app_with(SOURCE);
    act(&mut app, ActionId::ToggleEditMode);
    to_end(&mut app);
    clip.set(ClipboardContents {
        text: None,
        html: None,
        rtf: Some(r"{\rtf1\ansi {\b Bold} words.\par Second paragraph.\par}".into()),
    });
    act(&mut app, ActionId::Paste);
    let text = source(&app);
    assert!(text.contains("**Bold** words."), "{text:?}");
    assert!(text.contains("Second paragraph."), "{text:?}");
    assert!(
        app.status_text().starts_with("Pasted as Markdown:"),
        "{}",
        app.status_text()
    );
}

#[test]
fn paste_as_plain_text_keeps_only_the_text() {
    let (mut app, clip) = app_with(SOURCE);
    act(&mut app, ActionId::ToggleEditMode);
    to_end(&mut app);
    clip.set(html("<h2>Heading</h2>"));
    act(&mut app, ActionId::PastePlainText);
    assert_eq!(source(&app), format!("{SOURCE}plain words"));
    assert_eq!(app.status_text(), "Pasted 11 characters: plain words");
}

#[test]
fn an_empty_clipboard_is_said() {
    let (mut app, _clip) = app_with(SOURCE);
    act(&mut app, ActionId::ToggleEditMode);
    act(&mut app, ActionId::Paste);
    assert_eq!(source(&app), SOURCE);
    assert_eq!(
        app.status_text(),
        "Nothing to paste: the clipboard is empty."
    );
}

#[test]
fn a_large_formatted_paste_is_converted_off_the_input_thread() {
    let (mut app, clip) = app_with(SOURCE);
    act(&mut app, ActionId::ToggleEditMode);
    to_end(&mut app);
    let body = "<p>A paragraph of pasted text for the large paste.</p>".repeat(2000);
    assert!(body.len() > textweaver_app::clipboard::LARGE_PASTE);
    clip.set(html(&body));
    act(&mut app, ActionId::Paste);
    // The key returns at once; the text goes in from a later tick.
    assert!(app.pasting());
    assert_eq!(source(&app), SOURCE);
    assert_eq!(app.status_text(), "Converting the formatted text to paste.");
    let deadline = Instant::now() + Duration::from_secs(60);
    while app.pasting() && Instant::now() < deadline {
        app.tick(Instant::now());
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!app.pasting());
    assert_eq!(app.status_text(), "Pasted as Markdown: 2000 paragraphs");
    assert!(source(&app).ends_with("for the large paste."));
    act(&mut app, ActionId::Undo);
    assert_eq!(source(&app), SOURCE);
}

#[test]
fn the_context_menu_lists_what_fits_here() {
    let (mut app, _clip) = app_with(SOURCE);
    let actions = |app: &App| -> Vec<ActionId> {
        app.context_menu()
            .into_iter()
            .filter_map(|i| match i.kind {
                MenuItemKind::Action(a) => Some(a),
                _ => None,
            })
            .collect()
    };
    let reading = actions(&app);
    assert!(reading.contains(&ActionId::Copy));
    assert!(reading.contains(&ActionId::DefineWord));
    assert!(!reading.contains(&ActionId::Paste), "{reading:?}");
    assert!(!reading.contains(&ActionId::FollowLink), "{reading:?}");
    // Every item carries its key, and the list opens with its name.
    let effects = act(&mut app, ActionId::ContextMenu);
    let Some(Effect::ShowList { title, items }) = effects
        .into_iter()
        .find(|e| matches!(e, Effect::ShowList { .. }))
    else {
        panic!("no list");
    };
    assert_eq!(title, "Context menu");
    assert!(items[0].starts_with("Copy, "), "{items:?}");
    assert_eq!(app.menu_path(), None, "the context menu is not the bar");
    // Escape closes it and says so; the cursor stays.
    app.dispatch(Command::ListKey(ListKey::Escape));
    assert!(app.list_model().is_none());
    assert_eq!(app.status_text(), "Context menu closed.");
    // Edit mode adds Cut and both pastes; Enter runs the focused item.
    act(&mut app, ActionId::ToggleEditMode);
    let editing = actions(&app);
    assert_eq!(
        &editing[..4],
        &[
            ActionId::Cut,
            ActionId::Copy,
            ActionId::Paste,
            ActionId::PastePlainText
        ]
    );
    act(&mut app, ActionId::ContextMenu);
    app.dispatch(Command::ListKey(ListKey::Down));
    app.dispatch(Command::ListKey(ListKey::Enter));
    assert!(app.list_model().is_none());
    assert_eq!(app.status_text(), "Nothing selected to copy.");
}
