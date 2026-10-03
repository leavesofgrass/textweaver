//! The Contents and Notes panels (Wave 8d) in Masonry's test harness: the
//! landmark and its list with each row's place, the focus left alone when
//! the panel is shown from the setting, the panel key going to it and
//! closing it, Enter and Shift+Enter moving the document, and Escape.

use std::cell::Cell;
use std::rc::Rc;

use accesskit_consumer::common_filter;
use masonry::accesskit::Role;
use masonry::core::TextEvent;
use masonry_testing::{TestHarness, TestHarnessParams};
use textweaver_app::a11y::LogAnnouncer;
use textweaver_app::store::GuiSidebar;
use textweaver_app::{App, Command, NoteCommand, Panel};
use textweaver_xilem::dialog::DialogAction;
use textweaver_xilem::gui::{self, DOC};
use textweaver_xilem::keys;
use textweaver_xilem::setup::{self, Options};
use textweaver_xilem::sidebar::{
    self, SIDEBAR, SIDEBAR_LIST, SidebarAction, SidebarChange, SidebarShown, Toggled,
};
use textweaver_xilem::theme::{self, Palette};
use textweaver_xilem::widgets::Root;

fn app_with_sample(home: &std::path::Path) -> App {
    let opts = Options {
        no_speech: true,
        home: Some(home.to_path_buf()),
        ..Options::default()
    };
    let (mut app, _) = setup::build_app(&opts, Box::new(LogAnnouncer::default()));
    let sample = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sample.md");
    app.open(&sample).expect("sample opens");
    app
}

fn harness(app: &App) -> TestHarness<Root> {
    let p = Palette::galaxy();
    let tree = gui::build_tree(
        &p,
        Default::default(),
        Some(app),
        Rc::new(Cell::new(0)),
        Default::default(),
    );
    let mut params = TestHarnessParams::default();
    params.window_size = (1100, 780).into();
    let mut h = TestHarness::create_with(theme::default_properties(&p), tree.root, params);
    gui::refresh_for_tests(app, &mut h);
    let _ = h.redraw();
    h
}

/// The nodes with `role` under the root, with their names.
fn nodes_with(h: &TestHarness<Root>, role: Role) -> Vec<accesskit_consumer::NodeRef<'_>> {
    let mut out = Vec::new();
    let mut stack = vec![h.access_tree().state().root()];
    while let Some(n) = stack.pop() {
        if n.role() == role {
            out.push(n);
        }
        stack.extend(n.children());
    }
    out
}

fn list_id(h: &TestHarness<Root>) -> Option<masonry::core::WidgetId> {
    h.get_widget(SIDEBAR).inner().list_id()
}

/// The char position of `needle` in the open document.
fn char_at(app: &App, needle: &str) -> textweaver_app::core::CharPos {
    let text = app.session().unwrap().doc.text().to_string();
    let byte = text.find(needle).unwrap();
    textweaver_app::core::CharPos(text[..byte].chars().count())
}

fn set_panel(app: &mut App, panel: GuiSidebar) {
    app.update_settings(|s| s.gui.sidebar = panel).unwrap();
}

/// The Contents panel is a Navigation landmark named "Contents" holding a
/// list of the headings, each with its place ("2 of 5"), as the outline
/// names them, and the heading at the caret marked ", current".
#[test]
fn the_contents_panel_is_a_landmark_with_a_list_of_headings() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let p = Palette::galaxy();
    let mut shown = SidebarShown::default();
    // Closed: no landmark, and nothing to do.
    assert_eq!(sidebar::sync(&app, &p, &mut shown, &mut h), None);
    let _ = h.redraw();
    assert!(nodes_with(&h, Role::Navigation).is_empty());

    set_panel(&mut app, GuiSidebar::Contents);
    let change = sidebar::sync(&app, &p, &mut shown, &mut h);
    assert_eq!(change, Some(SidebarChange::Opened(Panel::Contents, 5)));
    let _ = h.redraw();
    let nav = nodes_with(&h, Role::Navigation);
    assert_eq!(nav.len(), 1);
    assert_eq!(nav[0].label().as_deref(), Some("Contents"));
    let lists: Vec<_> = nav[0]
        .children()
        .filter(|n| n.role() == Role::ListBox)
        .collect();
    assert_eq!(lists.len(), 1, "one list in the landmark");
    let list = &lists[0];
    assert_eq!(list.label().as_deref(), Some("Contents"));
    let rows: Vec<_> = list.children().collect();
    assert_eq!(rows.len(), 5);
    let names: Vec<String> = rows.iter().map(|r| r.label().unwrap_or_default()).collect();
    // Meaning first: the heading's words, then its level.
    assert_eq!(names[1], "Lists, level 2", "{names:?}");
    assert_eq!(rows[1].position_in_set(), Some(1));
    assert_eq!(rows[1].size_of_set_from_container(&common_filter), Some(5));
    // The caret is at the start: the first heading is current.
    assert_eq!(names[0], "Sample Markdown Document, level 1, current");
    assert_eq!(shown.current(), Some(0));
    // The document is still there, after the panel.
    let doc = h.get_widget(DOC).id();
    assert!(h.access_node(doc).is_some());
}

/// Showing the panel from the setting (the window opening with it, or the
/// settings) leaves the focus where it was; closing it the same way does
/// too, and a caret move only moves the current mark.
#[test]
fn the_panel_never_takes_the_focus_unasked() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let p = Palette::galaxy();
    let mut shown = SidebarShown::default();
    let doc = h.get_widget(DOC).id();
    h.focus_on(Some(doc));
    set_panel(&mut app, GuiSidebar::Contents);
    let _ = sidebar::sync(&app, &p, &mut shown, &mut h);
    let _ = h.redraw();
    assert!(list_id(&h).is_some());
    assert_eq!(h.focused_widget_id(), Some(doc), "opened from the setting");
    // The caret moves to the third heading: the mark follows, the focus
    // stays.
    let pos = char_at(&app, "A Table");
    app.dispatch(Command::SetCursor(pos));
    let _ = sidebar::sync(&app, &p, &mut shown, &mut h);
    assert_eq!(shown.current(), Some(2));
    assert_eq!(h.get_widget(SIDEBAR_LIST).inner().current(), Some(2));
    assert_eq!(
        h.get_widget(SIDEBAR_LIST).inner().selected(),
        2,
        "the selection follows the caret while the list is not focused"
    );
    assert_eq!(h.focused_widget_id(), Some(doc));
    // Switching to the Notes and closing, from the setting: still there.
    set_panel(&mut app, GuiSidebar::Notes);
    let _ = sidebar::sync(&app, &p, &mut shown, &mut h);
    assert_eq!(shown.panel(), Some(Panel::Notes));
    assert_eq!(h.focused_widget_id(), Some(doc));
    set_panel(&mut app, GuiSidebar::Off);
    let change = sidebar::sync(&app, &p, &mut shown, &mut h);
    assert_eq!(change, Some(SidebarChange::Closed(Panel::Notes)));
    let _ = h.redraw();
    assert!(list_id(&h).is_none());
    assert!(nodes_with(&h, Role::Navigation).is_empty());
    assert_eq!(h.focused_widget_id(), Some(doc));
}

/// The panel key shows the panel and goes to it; pressed in the panel, it
/// closes it and returns to the document; the choice is saved.
#[test]
fn the_panel_key_goes_to_the_panel_and_closes_it_from_inside() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let p = Palette::galaxy();
    let mut shown = SidebarShown::default();
    let doc = h.get_widget(DOC).id();
    h.focus_on(Some(doc));
    let t = sidebar::toggle(&mut app, Panel::Contents, &p, &mut shown, &mut h);
    assert_eq!(t, Toggled::Opened(Panel::Contents, 5));
    assert_eq!(app.settings().gui.sidebar, GuiSidebar::Contents);
    assert_eq!(h.focused_widget_id(), list_id(&h));
    assert_eq!(
        sidebar::toggled_message(&app, t).as_deref(),
        Some("Contents open, 5 items.")
    );
    // From the document, the key goes back to the panel shown.
    h.focus_on(Some(doc));
    let t = sidebar::toggle(&mut app, Panel::Contents, &p, &mut shown, &mut h);
    assert_eq!(t, Toggled::Focused(Panel::Contents));
    assert_eq!(h.focused_widget_id(), list_id(&h));
    assert_eq!(sidebar::toggled_message(&app, t), None);
    // The Notes key in the Contents switches panels, focus in the new one.
    let t = sidebar::toggle(&mut app, Panel::Notes, &p, &mut shown, &mut h);
    assert_eq!(t, Toggled::Opened(Panel::Notes, 0));
    assert_eq!(h.focused_widget_id(), list_id(&h));
    // No notes: one row saying so, which goes nowhere.
    let _ = h.redraw();
    let rows = h.get_widget(SIDEBAR_LIST).inner().items().to_vec();
    assert_eq!(rows, vec!["No notes.".to_owned()]);
    let before = app.session().unwrap().cursor;
    let _ = sidebar::go(&mut app, &shown, 0, false, &mut h);
    assert_eq!(app.session().unwrap().cursor, before);
    // In the panel, the key closes it and returns to the document.
    let t = sidebar::toggle(&mut app, Panel::Notes, &p, &mut shown, &mut h);
    assert_eq!(t, Toggled::Closed(Panel::Notes));
    assert_eq!(app.settings().gui.sidebar, GuiSidebar::Off);
    assert_eq!(h.focused_widget_id(), Some(doc));
    assert!(list_id(&h).is_none());
    assert_eq!(
        sidebar::toggled_message(&app, t).as_deref(),
        Some("Notes closed.")
    );
}

/// Enter on a row moves the document there and keeps the focus in the
/// list; Shift+Enter moves it and returns to the document; Escape returns
/// without moving. The keys reach the driver as the list's choice and the
/// panel's actions.
#[test]
fn enter_moves_the_document_and_shift_enter_returns() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let p = Palette::galaxy();
    let mut shown = SidebarShown::default();
    let doc = h.get_widget(DOC).id();
    h.focus_on(Some(doc));
    let _ = sidebar::toggle(&mut app, Panel::Contents, &p, &mut shown, &mut h);
    let _ = h.redraw();
    let list = list_id(&h).unwrap();
    let (table, quotes) = (char_at(&app, "A Table"), char_at(&app, "Quotes and Code"));

    // Down twice, then Enter: the list chooses row 2.
    let platform = textweaver_app::keymap::Platform::current();
    let key = |s: &str| TextEvent::Keyboard(keys::press(&s.parse().unwrap(), platform));
    h.process_text_event(key("Down"));
    h.process_text_event(key("Down"));
    h.process_text_event(key("Enter"));
    let (action, from) = h.pop_action::<DialogAction>().expect("Enter chooses");
    assert_eq!(action, DialogAction::Choose(2));
    assert_eq!(from, list);
    let _ = sidebar::go(&mut app, &shown, 2, false, &mut h);
    assert!(
        app.session().unwrap().cursor >= table,
        "at the table's heading"
    );
    assert!(app.session().unwrap().cursor < quotes);
    assert_eq!(h.focused_widget_id(), Some(list), "Enter stays in the list");

    // Shift+Enter on the next row: the panel asks to go and leave.
    h.process_text_event(key("Down"));
    h.process_text_event(key("Shift+Enter"));
    let (action, _) = h.pop_action::<SidebarAction>().expect("Shift+Enter");
    assert_eq!(action, SidebarAction::Leave { go: true });
    let row = h.get_widget(SIDEBAR_LIST).inner().selected();
    assert_eq!(row, 3);
    let _ = sidebar::go(&mut app, &shown, row, true, &mut h);
    assert!(app.session().unwrap().cursor >= quotes);
    assert_eq!(h.focused_widget_id(), Some(doc), "Shift+Enter returns");

    // Escape in the list: leave without going.
    h.focus_on(Some(list));
    h.process_text_event(key("Escape"));
    let (action, _) = h.pop_action::<SidebarAction>().expect("Escape");
    assert_eq!(action, SidebarAction::Leave { go: false });
}

/// The Notes panel lists the notes with the notes list's labels, and Enter
/// goes to a note; a new note shows without a caret move rebuilding the
/// rows otherwise.
#[test]
fn the_notes_panel_lists_the_notes() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let p = Palette::galaxy();
    let mut shown = SidebarShown::default();
    set_panel(&mut app, GuiSidebar::Notes);
    let _ = sidebar::sync(&app, &p, &mut shown, &mut h);
    assert_eq!(shown.rows(), 0);
    // A note on the table's heading.
    let at = char_at(&app, "A Table");
    app.dispatch(Command::SetCursor(at));
    app.dispatch(Command::Notes(NoteCommand::Add));
    app.dispatch(Command::Answer("Check the totals".into()));
    let key = app.panel_key(Panel::Notes);
    let _ = sidebar::sync(&app, &p, &mut shown, &mut h);
    assert_eq!(shown.rows(), 1);
    let rows = h.get_widget(SIDEBAR_LIST).inner().items().to_vec();
    assert!(rows[0].starts_with("Check the totals"), "{rows:?}");
    // A caret move changes no key, so nothing is rebuilt.
    app.dispatch(Command::SetCursor(textweaver_app::core::CharPos(0)));
    assert_eq!(app.panel_key(Panel::Notes), key);
    let _ = sidebar::go(&mut app, &shown, 0, false, &mut h);
    let s = app.session().unwrap();
    assert_eq!(s.cursor, s.notes[0].range.start);
}
