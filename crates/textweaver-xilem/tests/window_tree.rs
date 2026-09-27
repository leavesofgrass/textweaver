//! The whole window in Masonry's test harness: the controls a screen
//! reader finds, the keys that reach the keymap, the dialogs, and the
//! screenshot tool.

use std::cell::Cell;
use std::rc::Rc;

use masonry::accesskit::Role;
use masonry::core::TextEvent;
use masonry::core::keyboard::{Key, NamedKey};
use masonry_testing::{TestHarness, TestHarnessParams};
use textweaver_app::a11y::LogAnnouncer;
use textweaver_xilem::gui::{self, DOC, LIST, ROOT};
use textweaver_xilem::setup::{self, Options};
use textweaver_xilem::theme::{self, Palette};
use textweaver_xilem::widgets::{KeyAction, Root};

fn app_with_sample(home: &std::path::Path) -> textweaver_app::App {
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

fn harness(app: &textweaver_app::App) -> TestHarness<Root> {
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

fn names_of(h: &TestHarness<Root>, role: Role) -> Vec<String> {
    let mut out = Vec::new();
    let root = h.access_tree().state().root();
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        if n.role() == role {
            out.push(n.label().unwrap_or_default());
        }
        stack.extend(n.children());
    }
    out.sort();
    out
}

#[test]
fn every_control_has_a_role_and_a_name() {
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_sample(dir.path());
    let h = harness(&app);
    let buttons = names_of(&h, Role::Button);
    for b in [
        "Play",
        "Stop",
        "Next sentence",
        "Previous sentence",
        "Open…",
        "Commands…",
    ] {
        assert!(buttons.contains(&b.to_owned()), "{b} in {buttons:?}");
    }
    assert_eq!(names_of(&h, Role::Toolbar), vec!["Reading".to_owned()]);
    assert_eq!(names_of(&h, Role::Document), vec!["Document".to_owned()]);
    let status = names_of(&h, Role::Status);
    assert_eq!(status.len(), 1);
    assert!(status[0].contains("Line 1"), "{status:?}");
}

#[test]
fn keys_the_document_does_not_use_go_to_the_keymap() {
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let doc = h.get_widget(DOC).id();
    h.focus_on(Some(doc));
    h.process_text_event(TextEvent::key_down(Key::Character(" ".into())));
    let (KeyAction(k), _) = h.pop_action::<KeyAction>().expect("Space reaches the root");
    assert_eq!(k.key, Key::Character(" ".into()));
    // Tab moves focus instead.
    h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::Tab)));
    assert!(h.pop_action::<KeyAction>().is_none());
}

#[test]
fn a_list_dialog_is_modal_and_hides_the_window_behind_it() {
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let p = Palette::galaxy();
    let (modal, list_id) =
        gui::list_dialog(&p, "Bookmarks", vec!["One".into(), "Two".into()], 0, false);
    h.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(modal)));
    h.focus_on(Some(list_id));
    let _ = h.redraw();
    let dialogs = names_of(&h, Role::Dialog);
    assert_eq!(dialogs, vec!["Bookmarks".to_owned()]);
    // The document behind it is hidden from screen readers.
    let doc = h.access_node(h.get_widget(DOC).id()).unwrap();
    let mut node = Some(doc);
    let mut hidden = false;
    while let Some(n) = node {
        hidden |= n.is_hidden();
        node = n.parent();
    }
    assert!(hidden, "the window behind a dialog is hidden");
    // Down then Enter chooses the second item.
    h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::ArrowDown)));
    h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::Enter)));
    let (action, _) = h
        .pop_action::<textweaver_xilem::dialog::DialogAction>()
        .expect("chosen");
    assert_eq!(action, textweaver_xilem::dialog::DialogAction::Choose(1));
    let list = h.access_node(h.get_widget(LIST).id()).unwrap();
    assert_eq!(list.role(), Role::ListBox);
    assert_eq!(list.children().count(), 2);
    // Closing it brings the window back.
    h.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, None));
    let _ = h.redraw();
    assert!(names_of(&h, Role::Dialog).is_empty());
    // Every region is back, the status bar too.
    assert_eq!(
        names_of(&h, Role::Status).len(),
        1,
        "the status bar is back"
    );
    assert_eq!(names_of(&h, Role::Toolbar), vec!["Reading".to_owned()]);
}

#[test]
fn screenshots_are_written_at_both_scales() {
    let dir = tempfile::tempdir().unwrap();
    let sample = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sample.md");
    for (scale, name) in [(1.0, "a.png"), (2.0, "b.png")] {
        let o = textweaver_xilem::screenshot::ShotOptions {
            path: dir.path().join(name),
            file: Some(sample.clone()),
            size: (400, 300),
            scale,
            theme: Some("galaxy".into()),
            highlight_at: Some(20),
            list: None,
            settings: false,
            home: Some(dir.path().join("home")),
        };
        textweaver_xilem::screenshot::screenshot(&o).unwrap();
        let bytes = std::fs::read(dir.path().join(name)).unwrap();
        assert!(bytes.starts_with(b"\x89PNG"));
    }
    let a = std::fs::metadata(dir.path().join("a.png")).unwrap().len();
    let b = std::fs::metadata(dir.path().join("b.png")).unwrap().len();
    assert!(b > a, "the 200% screenshot is larger");
}

#[test]
fn themes_switch_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let light = Palette::named("galaxy-light");
    h.set_default_properties(std::sync::Arc::new(theme::default_properties(&light)));
    gui::apply_palette(&mut h, &light);
    let _ = h.redraw();
    // Still the same controls, now drawn light.
    assert_eq!(names_of(&h, Role::Document), vec!["Document".to_owned()]);
    let img = h.render();
    let px = img.get_pixel(4, 4);
    assert!(
        px[0] > 200 && px[1] > 200 && px[2] > 200,
        "a light page: {px:?}"
    );
}
