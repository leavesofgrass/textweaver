//! The preview pane beside the editor (B1-p1): off by default, shown in
//! edit mode when turned on, the document as the reading view draws it,
//! following the caret with its block marked, quiet, and laid out at every
//! review size.

use std::cell::Cell;
use std::rc::Rc;

use masonry::accesskit::Role;
use masonry::core::WidgetId;
use masonry_testing::{TestHarness, TestHarnessParams};
use textweaver_app::a11y::LogAnnouncer;
use textweaver_app::core::CharPos;
use textweaver_app::keymap::ActionId;
use textweaver_app::{App, Command};
use textweaver_xilem::gui::{self, DOC};
use textweaver_xilem::preview::{self, Toggled};
use textweaver_xilem::setup::{self, Options};
use textweaver_xilem::sidebar::SIDEBAR;
use textweaver_xilem::theme::{self, Palette};
use textweaver_xilem::widgets::Root;

const NOTES: &str = "# Notes\n\nFirst line with *some* words.\n\n- Apples\n- Pears\n\nLast line.\n";

/// Ada Example's notes open, the window at `size`, kept in step as the
/// window keeps it.
fn setup(
    home: &std::path::Path,
    size: (u32, u32),
    scale: f64,
) -> (App, TestHarness<Root>, gui::Refresher) {
    let opts = Options {
        no_speech: true,
        home: Some(home.to_path_buf()),
        ..Options::default()
    };
    let (mut app, _) = setup::build_app(&opts, Box::new(LogAnnouncer::default()));
    let file = home.join("notes.md");
    std::fs::write(&file, NOTES).expect("write the notes");
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
    params.window_size = (
        (f64::from(size.0) * scale) as u32,
        (f64::from(size.1) * scale) as u32,
    )
        .into();
    params.scale_factor = scale;
    let mut h = TestHarness::create_with(theme::default_properties(&p), tree.root, params);
    let mut r = gui::Refresher::default();
    let _ = r.refresh(&app, &mut h);
    let _ = h.redraw();
    let doc = h.get_widget(DOC).id();
    h.focus_on(Some(doc));
    (app, h, r)
}

fn settle(app: &App, h: &mut TestHarness<Root>, r: &mut gui::Refresher) {
    let _ = r.refresh(app, h);
    let _ = h.redraw();
    let _ = h.redraw();
}

fn preview_id(h: &TestHarness<Root>) -> Option<WidgetId> {
    h.get_widget(SIDEBAR).inner().preview_doc_id()
}

/// The preview's node: its role, whether it is read-only, its name, and
/// its runs' text.
fn preview_node(h: &TestHarness<Root>) -> (Role, bool, String, String) {
    let id = preview_id(h).expect("the preview is shown");
    let node = h.access_node(id).expect("the preview's node");
    let text: String = node.children().filter_map(|c| c.value()).collect();
    (
        node.role(),
        node.is_read_only(),
        node.label().unwrap_or_default(),
        text,
    )
}

fn doc_role() -> Role {
    match textweaver_app::keymap::Platform::current() {
        textweaver_app::keymap::Platform::MacOs => Role::MultilineTextInput,
        _ => Role::Document,
    }
}

#[test]
fn the_preview_is_off_by_default_and_waits_for_edit_mode() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, mut h, mut r) = setup(dir.path(), (1100, 780), 1.0);
    assert!(!app.settings().preview.pane);
    let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
    settle(&app, &mut h, &mut r);
    assert!(preview_id(&h).is_none(), "off by default");
    // Turned on outside edit mode, it waits for edit mode.
    let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
    settle(&app, &mut h, &mut r);
    let t = r.preview_key(&mut app, &mut h);
    assert_eq!(t, Toggled::Later);
    assert_eq!(
        preview::toggled_message(&app, t).as_deref(),
        Some("Preview on. It shows beside the editor in edit mode.")
    );
    assert!(app.settings().preview.pane, "remembered");
    settle(&app, &mut h, &mut r);
    assert!(preview_id(&h).is_none());
    let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
    settle(&app, &mut h, &mut r);
    assert!(preview_id(&h).is_some(), "shown in edit mode");
    // Leaving edit mode takes it away, silently.
    let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
    settle(&app, &mut h, &mut r);
    assert!(preview_id(&h).is_none());
}

fn regions(h: &TestHarness<Root>) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![h.access_tree().state().root()];
    while let Some(n) = stack.pop() {
        if n.role() == Role::Region {
            out.push(n.label().unwrap_or_default());
        }
        stack.extend(n.children());
    }
    out
}

/// Shown, the preview is a region named "Preview" around a read-only
/// document drawn from the parsed Markdown; the focus stays in the editor;
/// typing shows up after the parse; the block at the caret is marked.
#[test]
fn the_preview_shows_the_parsed_document_and_follows_the_caret() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, mut h, mut r) = setup(dir.path(), (1100, 780), 1.0);
    let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
    settle(&app, &mut h, &mut r);
    let t = r.preview_key(&mut app, &mut h);
    assert_eq!(t, Toggled::Shown);
    assert_eq!(
        preview::toggled_message(&app, t).as_deref(),
        Some("Preview shown beside the editor. F6 moves to it.")
    );
    settle(&app, &mut h, &mut r);
    let editor = h.get_widget(DOC).id();
    assert_eq!(h.focused_widget_id(), Some(editor), "the focus stays");
    let (role, read_only, name, text) = preview_node(&h);
    assert_eq!(role, doc_role());
    assert!(read_only);
    assert_eq!(name, "Preview");
    assert!(text.starts_with("Notes"), "{text:?}");
    assert!(!text.contains('#') && !text.contains('*'), "{text:?}");
    assert!(text.contains("Apples"), "{text:?}");
    assert_eq!(regions(&h), ["Preview"]);
    assert!(h.get_widget(SIDEBAR).inner().preview_beside());

    // Typing a heading: the preview shows it, as a heading, without "##".
    let _ = app.dispatch(Command::SetCursor(CharPos(NOTES.chars().count())));
    let _ = app.dispatch(Command::Insert("\n## Added part\n".into()));
    settle(&app, &mut h, &mut r);
    let (_, _, _, text) = preview_node(&h);
    assert!(text.contains("Added part"), "{text:?}");
    assert!(!text.contains("##"), "{text:?}");
    assert_eq!(h.focused_widget_id(), Some(editor));

    // The caret in "Pears": that block is marked in the preview.
    let at = NOTES.find("Pears").unwrap() + 2;
    let _ = app.dispatch(Command::SetCursor(CharPos(at)));
    settle(&app, &mut h, &mut r);
    let block = r.preview().block().expect("a block is marked");
    let shown = r.preview().text().unwrap();
    let marked: String = shown
        .chars()
        .skip(block.start.0)
        .take(block.end.0 - block.start.0)
        .collect();
    assert_eq!(marked, "Pears");
    let id = preview_id(&h).unwrap();
    let view = h.get_widget_with_id(id);
    let view = view
        .downcast::<textweaver_xilem::document::DocumentView>()
        .expect("the preview's view");
    assert_eq!(view.inner().block(), Some(block));
    // Drawn as the sentence is: a band and the line under it.
    assert!(!view.inner().sentence_underlines().is_empty());

    // Off again: the pane goes, and the setting is saved.
    let t = r.preview_key(&mut app, &mut h);
    assert_eq!(t, Toggled::Hidden);
    settle(&app, &mut h, &mut r);
    assert!(preview_id(&h).is_none());
    assert!(!app.settings().preview.pane);
}

/// At every review size the pane stacks or hides as the room allows, the
/// buttons stay inside the window, and a short window keeps the editor's
/// five lines; the preview key then shows the pane and goes to it.
#[test]
fn the_pane_fits_every_review_size() {
    for (size, scale, beside, hidden) in [
        ((1100, 780), 1.0, true, false),
        ((960, 540), 1.0, true, false),
        // A narrow, tall window: under the editor.
        ((700, 780), 1.0, false, false),
        // The laptop at 200 percent is short too (under 480 px): hidden.
        ((683, 384), 2.0, false, true),
        ((420, 320), 1.0, false, true),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, mut h, mut r) = setup(dir.path(), size, scale);
        app.update_settings(|s| s.preview.pane = true).unwrap();
        let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
        settle(&app, &mut h, &mut r);
        let at = format!("{}x{} at {}%", size.0, size.1, scale * 100.0);
        let s = h.get_widget(SIDEBAR);
        assert!(s.inner().preview_doc_id().is_some(), "{at}");
        assert_eq!(s.inner().preview_beside(), beside, "{at}");
        assert_eq!(s.inner().shown_preview_id().is_none(), hidden, "{at}");
        let (w, hgt) = (f64::from(size.0) * scale, f64::from(size.1) * scale);
        let mut stack = vec![h.access_tree().state().root()];
        while let Some(n) = stack.pop() {
            stack.extend(n.children());
            if n.role() != Role::Button {
                continue;
            }
            let b = n.bounding_box().expect("a button has bounds");
            assert!(
                b.x0 >= -0.5 && b.y0 >= -0.5 && b.x1 <= w + 0.5 && b.y1 <= hgt + 0.5,
                "{at}: {:?} at {b:?}",
                n.label()
            );
        }
        let doc = h.get_widget(DOC).ctx().bounding_box();
        assert!(doc.height() >= 130.0, "{at}: the editor {doc:?}");
        if hidden {
            let t = r.preview_key(&mut app, &mut h);
            assert_eq!(t, Toggled::Focused, "{at}");
            settle(&app, &mut h, &mut r);
            let id = h.get_widget(SIDEBAR).inner().shown_preview_id();
            assert!(id.is_some(), "{at}: revealed");
            assert_eq!(h.focused_widget_id(), id, "{at}");
            assert!(app.settings().preview.pane, "{at}: still on");
        }
    }
}
