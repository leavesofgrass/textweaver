//! Edit mode in the window (ADR-0033): the document becomes a multi-line
//! edit, keys type and delete through the app (which keeps the undo
//! history and the typing echo), and a screen reader's or dictation's
//! edits arrive as AccessKit actions.

use std::cell::Cell;
use std::rc::Rc;

use masonry::accesskit::{Action, ActionData, ActionRequest, Role, TreeId};
use masonry::core::keyboard::{Code, Key, KeyState, KeyboardEvent, Modifiers, NamedKey};
use masonry::core::{TextEvent, WidgetId};
use masonry_testing::{TestHarness, TestHarnessParams};
use textweaver_app::a11y::LogAnnouncer;
use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::keymap::ActionId;
use textweaver_app::{App, Command};
use textweaver_xilem::document::{DocAction, DocState, DocumentView};
use textweaver_xilem::gui::{self, DOC};
use textweaver_xilem::setup::{self, Options};
use textweaver_xilem::theme::{self, Palette};
use textweaver_xilem::widgets::{KeyAction, Root};

const NOTES: &str = "# Notes\n\nFirst line.\n";

/// An app with a Markdown file of Ada Example's notes open, and the window
/// in the test harness, kept in step as the window keeps it.
fn setup(home: &std::path::Path) -> (App, TestHarness<Root>, gui::Refresher, WidgetId) {
    setup_with(home, NOTES)
}

/// [`setup`] with other notes.
fn setup_with(
    home: &std::path::Path,
    notes: &str,
) -> (App, TestHarness<Root>, gui::Refresher, WidgetId) {
    let opts = Options {
        no_speech: true,
        home: Some(home.to_path_buf()),
        ..Options::default()
    };
    let (mut app, _) = setup::build_app(&opts, Box::new(LogAnnouncer::default()));
    let file = home.join("Ada Example notes.md");
    std::fs::write(&file, notes).expect("write the notes");
    app.open(&file).expect("the notes open");
    let p = Palette::galaxy();
    let tree = gui::build_tree(
        &p,
        Default::default(),
        Some(&app),
        Rc::new(Cell::new(0)),
        Default::default(),
    );
    let mut params = TestHarnessParams::default();
    params.window_size = (1100, 780).into();
    let mut h = TestHarness::create_with(theme::default_properties(&p), tree.root, params);
    let mut r = gui::Refresher::default();
    let _ = r.refresh(&app, &mut h);
    let _ = h.redraw();
    let doc = h.get_widget(DOC).id();
    h.focus_on(Some(doc));
    (app, h, r, doc)
}

fn key(k: Key, modifiers: Modifiers) -> TextEvent {
    TextEvent::Keyboard(KeyboardEvent {
        state: KeyState::Down,
        key: k,
        code: Code::Unidentified,
        modifiers,
        ..Default::default()
    })
}

fn text_of(app: &App) -> String {
    app.session()
        .map(|s| s.doc.text().to_string())
        .unwrap_or_default()
}

/// What the window does with an edit from the view.
fn apply(app: &mut App, action: DocAction) {
    let cmd = match action {
        DocAction::Typed(text) => Command::Insert(text),
        DocAction::Delete { forward: true } => Command::DeleteForward,
        DocAction::Delete { forward: false } => Command::DeleteBack,
        DocAction::Replace { range, text } => Command::ReplaceRange { range, text },
        DocAction::CaretMoved { caret, .. } => Command::SetCursor(caret),
        DocAction::TableCell { forward: true } => Command::Action(ActionId::NextTableCell),
        DocAction::TableCell { forward: false } => Command::Action(ActionId::PreviousTableCell),
    };
    let _ = app.dispatch(cmd);
}

/// The document's node: its role, whether it is read-only, and its runs'
/// text.
fn doc_node(h: &TestHarness<Root>, doc: WidgetId) -> (Role, bool, String) {
    let node = h.access_node(doc).expect("the document's node");
    let text: String = node.children().filter_map(|c| c.value()).collect();
    (node.role(), node.is_read_only(), text)
}

#[test]
fn edit_mode_makes_the_document_a_multiline_edit() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, mut h, mut r, doc) = setup(dir.path());
    let (role, read_only, _) = doc_node(&h, doc);
    assert_eq!(role, Role::Document);
    assert!(read_only);
    let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
    assert!(app.is_editing());
    let _ = r.refresh(&app, &mut h);
    let _ = h.redraw();
    let (role, read_only, text) = doc_node(&h, doc);
    assert_eq!(role, Role::MultilineTextInput);
    assert!(!read_only, "an edit takes typing");
    // The Markdown source is what is edited and shown.
    assert!(text.starts_with("# Notes"), "{text:?}");
    assert!(h.get_widget(DOC).inner().editing());
}

#[test]
fn keys_type_and_delete_through_the_app_and_undo_restores() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, mut h, mut r, doc) = setup(dir.path());
    let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
    let _ = r.refresh(&app, &mut h);
    // The caret after "First line.".
    let end = NOTES.find("First line.").unwrap() + "First line.".len();
    let _ = app.dispatch(Command::SetCursor(CharPos(end)));
    let _ = r.refresh(&app, &mut h);
    let _ = h.redraw();

    // A letter: typed at the caret, echoed by the app as the mode says.
    h.process_text_event(key(Key::Character("!".into()), Modifiers::empty()));
    let (action, _) = h.pop_action::<DocAction>().expect("typed");
    assert_eq!(action, DocAction::Typed("!".into()));
    apply(&mut app, action);
    let _ = r.refresh(&app, &mut h);
    let _ = h.redraw();
    assert!(
        text_of(&app).contains("First line.!"),
        "{:?}",
        text_of(&app)
    );
    let (_, _, shown) = doc_node(&h, doc);
    assert!(
        shown.contains("First line.!"),
        "the view shows it: {shown:?}"
    );
    // The caret follows the typing.
    assert_eq!(h.get_widget(DOC).inner().state().caret, CharPos(end + 1));

    // Enter types a new line; Backspace deletes it.
    h.process_text_event(key(Key::Named(NamedKey::Enter), Modifiers::empty()));
    let (action, _) = h.pop_action::<DocAction>().expect("enter");
    assert_eq!(action, DocAction::Typed("\n".into()));
    h.process_text_event(key(Key::Named(NamedKey::Backspace), Modifiers::empty()));
    let (action, _) = h.pop_action::<DocAction>().expect("backspace");
    assert_eq!(action, DocAction::Delete { forward: false });
    apply(&mut app, action);
    assert!(!text_of(&app).contains("line.!"), "{:?}", text_of(&app));

    // Undo is the app's (textweaver-editor): the deleted mark comes back.
    let _ = app.dispatch(Command::Action(ActionId::Undo));
    assert!(
        text_of(&app).contains("First line.!"),
        "{:?}",
        text_of(&app)
    );

    // Command keys are not typed: Ctrl+Z goes on to the keymap.
    h.process_text_event(key(Key::Character("z".into()), Modifiers::CONTROL));
    let (KeyAction(k), _) = h
        .pop_action::<KeyAction>()
        .expect("Ctrl+Z reaches the keymap, untyped");
    assert!(k.modifiers.ctrl());
    // Space types a space in edit mode (it plays in reading).
    h.process_text_event(key(Key::Character(" ".into()), Modifiers::empty()));
    let (action, _) = h.pop_action::<DocAction>().expect("space");
    assert_eq!(action, DocAction::Typed(" ".into()));
}

#[test]
fn typing_over_a_selection_replaces_it() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, mut h, mut r, _doc) = setup(dir.path());
    let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
    let _ = r.refresh(&app, &mut h);
    let start = NOTES.find("First").unwrap();
    // "First" selected backwards, as Shift+Left leaves it.
    h.edit_widget(DOC, |mut d| {
        DocumentView::set_state(
            &mut d,
            DocState {
                caret: CharPos(start),
                anchor: Some(CharPos(start + 5)),
                ..DocState::default()
            },
        );
    });
    h.process_text_event(key(Key::Character("L".into()), Modifiers::SHIFT));
    let (action, _) = h.pop_action::<DocAction>().expect("typed over");
    let range = CharRange::new(start, start + 5);
    assert_eq!(
        action,
        DocAction::Replace {
            range,
            text: "L".into()
        }
    );
    apply(&mut app, action);
    assert!(text_of(&app).contains("L line."), "{:?}", text_of(&app));
    // Backspace over a selection deletes it.
    let _ = r.refresh(&app, &mut h);
    h.edit_widget(DOC, |mut d| {
        DocumentView::set_state(
            &mut d,
            DocState {
                caret: CharPos(start + 1),
                anchor: Some(CharPos(start)),
                ..DocState::default()
            },
        );
    });
    h.process_text_event(key(Key::Named(NamedKey::Backspace), Modifiers::empty()));
    let (action, _) = h.pop_action::<DocAction>().expect("deleted");
    assert_eq!(
        action,
        DocAction::Replace {
            range: CharRange::new(start, start + 1),
            text: String::new()
        }
    );
}

#[test]
fn a_screen_readers_edits_arrive_as_accesskit_actions() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, mut h, mut r, doc) = setup(dir.path());
    // Read-only outside edit mode: the action does nothing.
    let node = h.access_node(doc).unwrap().locate().0;
    let replace = |h: &mut TestHarness<Root>, text: &str| {
        h.process_access_event(ActionRequest {
            action: Action::ReplaceSelectedText,
            target_tree: TreeId::ROOT,
            target_node: node,
            data: Some(ActionData::Value(text.into())),
        });
    };
    replace(&mut h, "dictated");
    assert!(h.pop_action::<DocAction>().is_none());
    let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
    let _ = r.refresh(&app, &mut h);
    let _ = app.dispatch(Command::SetCursor(CharPos(0)));
    let _ = r.refresh(&app, &mut h);
    replace(&mut h, "Dictated. ");
    let (action, _) = h.pop_action::<DocAction>().expect("replaced");
    assert_eq!(
        action,
        DocAction::Replace {
            range: CharRange::new(0, 0),
            text: "Dictated. ".into()
        }
    );
    apply(&mut app, action);
    assert!(
        text_of(&app).starts_with("Dictated. # Notes"),
        "{:?}",
        text_of(&app)
    );
}

/// Spell check while writing: the next misspelling (Alt+M) is selected in
/// the view, so the screen reader says it, and typing replaces it.
#[test]
fn a_misspelling_is_selected_and_typing_corrects_it() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, mut h, mut r, _doc) = setup_with(dir.path(), "This is a tset.\n");
    let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
    let _ = app.dispatch(Command::Action(ActionId::DocumentStart));
    let _ = app.dispatch(Command::Action(ActionId::NextMisspelling));
    let _ = r.refresh(&app, &mut h);
    let _ = h.redraw();
    let start = "This is a ".len();
    let state = h.get_widget(DOC).inner().state();
    let shown = [state.caret.0, state.anchor.map_or(usize::MAX, |a| a.0)];
    assert!(
        shown.contains(&start) && shown.contains(&(start + 4)),
        "the misspelling is selected: {state:?}"
    );
    h.process_text_event(key(Key::Character("t".into()), Modifiers::empty()));
    let (action, _) = h.pop_action::<DocAction>().expect("typed over");
    assert_eq!(
        action,
        DocAction::Replace {
            range: CharRange::new(start, start + 4),
            text: "t".into()
        }
    );
    apply(&mut app, action);
    let _ = app.dispatch(Command::Insert("est".into()));
    assert_eq!(text_of(&app), "This is a test.\n");
}

/// A citation while writing: the app's citation picker (Alt+C) is a list
/// the window shows as its list dialog, filtered as you type.
#[cfg(feature = "publish")]
#[test]
fn a_citation_while_writing_opens_the_picker() {
    use textweaver_app::Effect;
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _h, _r, _doc) = setup_with(dir.path(), "# Paper\n\nAs shown.\n");
    let bib = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/p/sample.bib");
    let _ = app.dispatch(Command::Action(ActionId::ImportReferences));
    let _ = app.dispatch(Command::Answer(bib.display().to_string()));
    let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
    let _ = app.dispatch(Command::SetCursor(CharPos("# Paper\n\nAs shown".len())));
    let effects = app.dispatch(Command::Action(ActionId::InsertCitation));
    let title = effects
        .iter()
        .find_map(|e| match e {
            Effect::ShowList { title, .. } => Some(title.clone()),
            _ => None,
        })
        .expect("the citation picker");
    assert!(title.starts_with("Insert citation, "), "{title}");
    assert_eq!(app.list_filter(), Some(""), "it filters as you type");
}

/// Tab in edit mode types a tab (or moves to the next table cell), as in
/// the terminal, through the app's `next_table_cell`; Shift+Tab goes back
/// a cell. Ctrl+Tab still moves the focus out of the edit, so the keyboard
/// is never trapped; outside edit mode Tab moves the focus as before.
#[test]
fn tab_types_a_tab_in_edit_mode_and_ctrl_tab_leaves() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, mut h, mut r, _doc) = setup_with(dir.path(), "Ada Example\n");
    // Reading: Tab is left to Masonry, which moves the focus.
    h.process_text_event(key(Key::Named(NamedKey::Tab), Modifiers::empty()));
    assert!(h.pop_action::<DocAction>().is_none());
    assert!(h.pop_action::<KeyAction>().is_none());
    let doc = h.get_widget(DOC).id();
    h.focus_on(Some(doc));
    let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
    let _ = r.refresh(&app, &mut h);
    let _ = h.redraw();
    let _ = app.dispatch(Command::SetCursor(CharPos(3)));
    h.process_text_event(key(Key::Named(NamedKey::Tab), Modifiers::empty()));
    let (action, _) = h.pop_action::<DocAction>().expect("Tab is taken");
    assert_eq!(action, DocAction::TableCell { forward: true });
    apply(&mut app, action);
    assert_eq!(text_of(&app), "Ada\t Example\n");
    h.process_text_event(key(Key::Named(NamedKey::Tab), Modifiers::SHIFT));
    let (action, _) = h.pop_action::<DocAction>().expect("Shift+Tab is taken");
    assert_eq!(action, DocAction::TableCell { forward: false });
    // Ctrl+Tab is not typed and not a command: it moves the focus.
    h.process_text_event(key(Key::Named(NamedKey::Tab), Modifiers::CONTROL));
    assert!(h.pop_action::<DocAction>().is_none());
    assert!(h.pop_action::<KeyAction>().is_none());
    assert_ne!(h.focused_widget_id(), Some(doc), "Ctrl+Tab left the edit");
}

/// Misspelled words are marked in edit mode (W4a3 left them unmarked):
/// the app gives their places, the view draws a dotted underline under
/// them, and the text and its runs stay as they are.
#[test]
fn misspelled_words_are_marked_in_edit_mode() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, mut h, mut r, doc) = setup_with(dir.path(), "I recieve the notes.\n");
    let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
    let _ = r.refresh(&app, &mut h);
    let _ = h.redraw();
    let before = doc_node(&h, doc).2;
    let ranges = app.misspelled_ranges();
    assert_eq!(ranges, vec![CharRange::new(2, 9)], "recieve");
    h.edit_widget(DOC, |mut d| {
        DocumentView::set_misspelled(&mut d, ranges.clone())
    });
    let _ = h.redraw();
    assert_eq!(h.get_widget(DOC).inner().misspelled(), ranges.as_slice());
    assert_eq!(doc_node(&h, doc).2, before, "the marks are drawn only");
}
