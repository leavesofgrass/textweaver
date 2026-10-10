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
use textweaver_app::lexicon::i18n::Catalog;
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

/// The document's role on this platform: a Document, or on macOS a
/// read-only text area (VoiceOver reads a Document as an AXGroup).
fn doc_role() -> Role {
    match textweaver_app::keymap::Platform::current() {
        textweaver_app::keymap::Platform::MacOs => Role::MultilineTextInput,
        _ => Role::Document,
    }
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
        "Next paragraph",
        "Previous paragraph",
        "Open",
        "Font",
        "Start editing",
        "Settings",
        "Commands",
        "Slower",
        "Faster",
    ] {
        assert!(buttons.contains(&b.to_owned()), "{b} in {buttons:?}");
    }
    assert_eq!(names_of(&h, Role::Toolbar), vec!["Reading".to_owned()]);
    assert_eq!(
        names_of(&h, doc_role()),
        vec!["Sample Markdown Document, document".to_owned()]
    );
    let status = names_of(&h, Role::Status);
    assert_eq!(status.len(), 1);
    // The terminal's title line parts, from the app (`App::title_parts`).
    assert!(status[0].contains("Line 1 of"), "{status:?}");
    assert!(status[0].contains("wpm"), "{status:?}");
}

/// The window's and the document's names, for the focus announcement
/// (the legal report's item 6, W8a): the focused document's node is named
/// with the document's title first, so NVDA and JAWS say it when the
/// window or the document takes the focus; the window's title (its UI
/// Automation Name) names the document too. With no document, "Document"
/// and "textweaver".
#[test]
fn the_window_and_the_document_are_named_for_the_focus() {
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let doc = h.get_widget(DOC).id();
    h.focus_on(Some(doc));
    let _ = h.redraw();
    assert_eq!(h.focused_widget_id(), Some(doc));
    let focused = h.access_node(doc).expect("the document's node");
    assert_eq!(focused.role(), doc_role());
    let name = focused.label().unwrap_or_default();
    assert_eq!(name, "Sample Markdown Document, document");
    // The meaning first: the title starts the name, within 40 cells.
    assert!(name.starts_with("Sample Markdown Document"));
    assert_eq!(
        gui::window_title(&app),
        "Sample Markdown Document - textweaver"
    );
    assert_eq!(gui::document_label(&app), name);

    let empty = tempfile::tempdir().unwrap();
    let opts = Options {
        no_speech: true,
        home: Some(empty.path().to_path_buf()),
        ..Options::default()
    };
    let (none, _) = setup::build_app(&opts, Box::new(LogAnnouncer::default()));
    let h = harness(&none);
    assert_eq!(names_of(&h, doc_role()), vec!["Document".to_owned()]);
    assert_eq!(gui::window_title(&none), "textweaver");
}

/// Every button's key comes from the keymap: its keyboard shortcut
/// property (UI Automation's AcceleratorKey), which NVDA and JAWS say when
/// their "report shortcut keys" setting is on, and the text on screen,
/// "Open… (Ctrl+O)". The name is the label only: the owner found the key in
/// the name wordy.
#[test]
fn every_button_has_its_key_from_the_keymap() {
    use textweaver_app::keymap::ActionId;
    use textweaver_xilem::widgets::ActionButton;
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_sample(dir.path());
    let h = harness(&app);
    let mut shortcuts = std::collections::HashMap::new();
    let mut stack = vec![h.access_tree().state().root()];
    while let Some(n) = stack.pop() {
        stack.extend(n.children());
        if n.role() == Role::Button {
            shortcuts.insert(
                n.label().unwrap_or_default(),
                n.data().keyboard_shortcut().map(str::to_owned),
            );
        }
    }
    for (name, action) in [
        ("Open", ActionId::Open),
        ("Font", ActionId::ChooseFont),
        ("Start editing", ActionId::ToggleEditMode),
        ("Settings", ActionId::Settings),
        ("Commands", ActionId::CommandPalette),
        ("Play", ActionId::PlayPause),
        ("Stop", ActionId::Stop),
        ("Previous paragraph", ActionId::PreviousParagraph),
        ("Next paragraph", ActionId::NextParagraph),
        ("Slower", ActionId::RateDown),
        ("Faster", ActionId::RateUp),
    ] {
        let written = gui::shortcut_for(&app, action);
        assert!(!written.is_empty(), "{action:?}");
        // Never "the command palette": each has a key of its own.
        assert!(!app.keymap().chords_for(action).is_empty(), "{action:?}");
        assert_eq!(
            shortcuts.get(name),
            Some(&Some(written.clone())),
            "{name}: {shortcuts:?}"
        );
    }
    // The platform's own key for Open, from the keymap (Command on macOS,
    // Control elsewhere): never written into the test.
    let open = app
        .keymap()
        .chords_for(ActionId::Open)
        .into_iter()
        .find(|c| !c.is_text_input())
        .expect("Open has a chord")
        .to_string();
    assert_eq!(shortcuts.get("Open"), Some(&Some(open)));
    // On screen, the written form; the name is the label alone.
    let b = ActionButton::new("Open…").with_shortcut("Ctrl+O");
    assert_eq!(b.shown_text(), "Open… (Ctrl+O)");
    assert_eq!(b.name(), "Open");
    // Commands shows the key people are told (F2), never a lone ":".
    let commands = gui::shortcut_for(&app, ActionId::CommandPalette);
    assert!(
        !commands.chars().all(|c| c.is_ascii_punctuation()),
        "Commands shows {commands:?}"
    );
}

/// The label-in-name rule (WCAG 2.5.3): each header and toolbar button's
/// text on screen starts with its accessible name, and anything after it is
/// a space and its keyboard shortcut in parentheses, so speech input can say
/// what it sees.
#[test]
fn every_button_shows_its_name_then_its_key() {
    use textweaver_app::keymap::ActionId;
    use textweaver_xilem::widgets::ActionButton;
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_sample(dir.path());
    let c = app.catalog();
    for action in [
        ActionId::Open,
        ActionId::ChooseFont,
        ActionId::ToggleEditMode,
        ActionId::Settings,
        ActionId::CommandPalette,
        ActionId::PlayPause,
        ActionId::Stop,
        ActionId::PreviousParagraph,
        ActionId::NextParagraph,
        ActionId::RateDown,
        ActionId::RateUp,
    ] {
        for (reading, editing) in [(false, false), (true, true)] {
            let label = gui::button_label(&c, action, reading, editing);
            let key = gui::shortcut_for(&app, action);
            let b = ActionButton::new(label.as_str()).with_shortcut(key.as_str());
            let shown = b.shown_text();
            let name = b.name();
            let visible = shown.trim_end_matches(&format!(" ({key})")[..]);
            assert!(
                visible.trim_end_matches('…') == name,
                "{action:?}: shows {shown:?}, named {name:?}"
            );
            assert_eq!(shown, format!("{visible} ({key})"), "{action:?}");
        }
    }
}

/// Button descriptions are short and true (conventions research, QW2):
/// NVDA reads a description after the name by default, so it says in a few
/// words what the name does not, and never describes the terminal reader.
#[test]
fn button_descriptions_are_short_and_about_the_window() {
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_sample(dir.path());
    let h = harness(&app);
    let mut stack = vec![h.access_tree().state().root()];
    let mut seen = 0;
    while let Some(n) = stack.pop() {
        stack.extend(n.children());
        if n.role() == Role::Button {
            let name = n.label().unwrap_or_default();
            if let Some(d) = n.description() {
                seen += 1;
                assert!(d.chars().count() <= 60, "{name}: {d}");
                assert!(!d.contains("terminal"), "{name}: {d}");
                assert!(!d.contains("filtered as you type"), "{name}: {d}");
            }
        }
    }
    assert!(seen >= 6, "only {seen} buttons have a description");
}

#[test]
fn the_text_size_and_font_follow_the_settings() {
    use textweaver_xilem::font_chooser::{self, Step};
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let before = h.get_widget(DOC).inner().font().clone();
    // Ctrl+Plus twice, as the window does it, then a family from the list.
    for _ in 0..2 {
        let now = app.settings().reading_aids.font.clone();
        let (size, _) = font_chooser::stepped(now.size_pt, Step::Larger);
        let _ = app.update_settings(|s| s.reading_aids.font = font_chooser::with_size(&now, size));
    }
    let now = app.settings().reading_aids.font.clone();
    let _ = app.update_settings(|s| {
        s.reading_aids.font = font_chooser::with_family(&now, "OpenDyslexic");
    });
    gui::refresh_for_tests(&app, &mut h);
    let after = h.get_widget(DOC).inner().font().clone();
    assert!(after.size > before.size, "{before:?} to {after:?}");
    // 16 points in CSS pixels.
    assert!((after.size - 16.0 * 96.0 / 72.0).abs() < 0.01, "{after:?}");
    assert!(after.family.starts_with("\"OpenDyslexic\""), "{after:?}");
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
    let (modal, list_id) = gui::list_dialog(
        &p,
        &Catalog::english(),
        "Bookmarks",
        vec!["One".into(), "Two".into()],
        0,
        false,
    );
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

/// In an app list, F1 and the Say Status chord reach the driver as chords
/// the keymap knows, so they repeat the list's introduction
/// (`ListKey::Introduce`), as in the terminal reader. F2 still renames.
#[test]
fn help_and_say_status_keys_in_a_list_reach_the_keymap() {
    use textweaver_app::keymap::{ActionId, Layer};
    use textweaver_xilem::dialog::DialogAction;
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let p = Palette::galaxy();
    let (modal, list_id) = gui::list_dialog(
        &p,
        &Catalog::english(),
        "Bookmarks",
        vec!["One".into(), "Two".into()],
        0,
        true,
    );
    h.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(modal)));
    h.focus_on(Some(list_id));
    let _ = h.redraw();
    let lookup = |a: DialogAction| match a {
        DialogAction::Chord(c) => app.keymap().lookup(&c, Layer::Global),
        other => panic!("expected a chord, got {other:?}"),
    };
    h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::F1)));
    let (a, _) = h
        .pop_action::<DialogAction>()
        .expect("F1 reaches the driver");
    assert_eq!(lookup(a), Some(ActionId::Help));
    let say_status = app
        .keymap()
        .chords_for(ActionId::SayStatus)
        .into_iter()
        .find(|c| !c.is_text_input())
        .expect("Say Status has a chord");
    let platform = textweaver_app::keymap::Platform::current();
    h.process_text_event(TextEvent::Keyboard(textweaver_xilem::keys::press(
        &say_status,
        platform,
    )));
    let (a, _) = h
        .pop_action::<DialogAction>()
        .expect("the Say Status key reaches the driver");
    assert_eq!(lookup(a), Some(ActionId::SayStatus));
    // F2 is the list's own key.
    h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::F2)));
    let (a, _) = h.pop_action::<DialogAction>().expect("F2");
    assert_eq!(a, DialogAction::Key(textweaver_app::ListKey::Rename));
}

/// What a screen reader would learn from one node.
#[derive(Debug)]
struct Seen {
    role: Role,
    value: Option<String>,
    label: Option<String>,
    hidden: bool,
    live: masonry::accesskit::Live,
    focused: bool,
}

/// Every node in the tree, depth first.
fn all_nodes(h: &TestHarness<Root>) -> Vec<Seen> {
    let mut out = Vec::new();
    let mut stack = vec![h.access_tree().state().root()];
    while let Some(n) = stack.pop() {
        stack.extend(n.children());
        out.push(Seen {
            role: n.role(),
            value: n.value(),
            label: n.label(),
            hidden: n.is_hidden(),
            live: n.live(),
            focused: n.is_focused(),
        });
    }
    out
}

/// RSVP's flashing word is a hidden node that is never live and never
/// focused; its status is a quiet node (live off) a screen reader finds by
/// review. Nothing RSVP adds speaks by itself, and the document keeps the
/// focus. The panel is under the document, so it never covers the caret.
#[test]
fn rsvp_is_quiet_for_screen_readers_and_never_covers_the_caret() {
    use masonry::accesskit::Live;
    use textweaver_app::Command;
    use textweaver_app::keymap::ActionId;
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let doc = h.get_widget(DOC).id();
    h.focus_on(Some(doc));
    let mut refresher = gui::Refresher::default();
    let _ = refresher.refresh(&app, &mut h);
    let _ = h.redraw();
    // Off: the panel is hidden and takes no room.
    let panel = h.get_widget(gui::RSVP);
    assert!(panel.inner().shown().is_none());
    assert_eq!(panel.ctx().bounding_box().height(), 0.0);
    let _ = app.dispatch(Command::Action(ActionId::RsvpToggle));
    let _ = app.dispatch(Command::Action(ActionId::CaretNextWord));
    let _ = refresher.refresh(&app, &mut h);
    let _ = h.redraw();
    let shown = h
        .get_widget(gui::RSVP)
        .inner()
        .shown()
        .cloned()
        .expect("RSVP shows");
    let word = shown.word();
    assert!(!word.is_empty());
    assert!(
        shown.status.starts_with("RSVP paused, word 2 of "),
        "{}",
        shown.status
    );
    let nodes = all_nodes(&h);
    // The word: hidden, not live, not focusable.
    let word_nodes: Vec<_> = nodes
        .iter()
        .filter(|n| n.value.as_deref() == Some(word.as_str()) && n.role == Role::Label)
        .collect();
    assert_eq!(word_nodes.len(), 1, "one word node");
    let w = word_nodes[0];
    assert!(w.hidden, "the RSVP word is hidden");
    assert_eq!(w.live, Live::Off, "the RSVP word is never live");
    assert!(!w.focused);
    // The status: a quiet node with its text.
    let status: Vec<_> = nodes
        .iter()
        .filter(|n| n.role == Role::Status && n.label.as_deref() == Some(shown.status.as_str()))
        .collect();
    assert_eq!(status.len(), 1, "one RSVP status node");
    assert_eq!(status[0].live, Live::Off, "the RSVP status is quiet");
    assert!(!status[0].hidden);
    // Nothing RSVP added is live; the document keeps the focus.
    let panel_id = h.get_widget(gui::RSVP).id();
    let panel_node = h.access_node(panel_id).unwrap();
    for c in panel_node.children() {
        assert_eq!(c.live(), Live::Off);
    }
    assert_eq!(h.focused_widget_id(), Some(doc));
    // The panel is its own strip under the document: it never overlaps it.
    let doc_rect = h.get_widget(DOC).ctx().bounding_box();
    let panel_rect = h.get_widget(gui::RSVP).ctx().bounding_box();
    assert!(panel_rect.height() > 40.0, "{panel_rect:?}");
    assert!(
        // (The document's focus ring is drawn a few pixels outside it.)
        panel_rect.y0 >= doc_rect.y1 - 6.0,
        "the panel {panel_rect:?} is below the document {doc_rect:?}"
    );
    // Off again: hidden.
    let _ = app.dispatch(Command::Action(ActionId::RsvpToggle));
    let _ = refresher.refresh(&app, &mut h);
    let _ = h.redraw();
    let panel_node = h.access_node(h.get_widget(gui::RSVP).id()).unwrap();
    assert!(panel_node.is_hidden());
    assert_eq!(panel_node.children().count(), 0);
}

/// The reading aids the view draws: bionic reading and difficult words as
/// spans (the text runs a screen reader gets do not change), text spacing
/// through the layout, and the ruler's marks, which follow the caret and a
/// theme change.
#[test]
fn reading_aids_are_drawn_and_leave_the_text_alone() {
    use textweaver_app::Command;
    use textweaver_app::aids::RowMark;
    use textweaver_app::keymap::ActionId;
    use textweaver_xilem::window::SpanStyle;
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let mut refresher = gui::Refresher::default();
    let _ = refresher.refresh(&app, &mut h);
    let _ = h.redraw();
    let text_before = h
        .access_node(h.get_widget(DOC).id())
        .unwrap()
        .document_range()
        .text();
    let tall_before = h.get_widget(DOC).inner().visible_paragraphs().to_vec();
    // Bionic reading: bold word starts as spans.
    let _ = app.dispatch(Command::Action(ActionId::BionicToggle));
    let _ = refresher.refresh(&app, &mut h);
    let _ = h.redraw();
    let spans = gui::model_for(&app, app.session().unwrap().doc.full_range())
        .unwrap()
        .spans;
    assert!(spans.iter().any(|s| s.style == SpanStyle::Bionic));
    let text_after = h
        .access_node(h.get_widget(DOC).id())
        .unwrap()
        .document_range()
        .text();
    assert_eq!(text_after, text_before, "bionic reading changes no text");
    // Text spacing: WCAG's values spread the paragraphs out.
    app.update_settings(|s| {
        s.reading_aids.spacing.line_height = 2.0;
        s.reading_aids.spacing.paragraph_spacing = 2.0;
        s.reading_aids.spacing.letter_spacing = 0.12;
        s.reading_aids.spacing.word_spacing = 0.16;
    })
    .unwrap();
    let _ = refresher.refresh(&app, &mut h);
    let _ = h.redraw();
    let aids = h.get_widget(DOC).inner().aids();
    assert_eq!(aids.spacing.line_height, 2.0);
    let tall_after = h.get_widget(DOC).inner().visible_paragraphs().to_vec();
    assert!(
        tall_after.len() < tall_before.len()
            || tall_after.last().map(|p| p.1) > tall_before.last().map(|p| p.1),
        "more spacing shows less: {tall_before:?} then {tall_after:?}"
    );
    // The ruler: off, then the current line, then the band.
    assert!(h.get_widget(DOC).inner().ruler_marks().is_empty());
    let _ = app.dispatch(Command::Action(ActionId::RulerCycle));
    let _ = refresher.refresh(&app, &mut h);
    let _ = h.redraw();
    let marks = h.get_widget(DOC).inner().ruler_marks();
    let focus: Vec<_> = marks.iter().filter(|m| m.0 == RowMark::Focus).collect();
    assert!(!focus.is_empty(), "the current line is marked: {marks:?}");
    let first_focus_y = focus[0].1;
    let _ = app.dispatch(Command::Action(ActionId::RulerCycle));
    let _ = app.dispatch(Command::Action(ActionId::NextParagraph));
    let _ = app.dispatch(Command::Action(ActionId::NextParagraph));
    let _ = refresher.refresh(&app, &mut h);
    let _ = h.redraw();
    let marks = h.get_widget(DOC).inner().ruler_marks();
    assert!(marks.iter().any(|m| m.0 == RowMark::Band), "{marks:?}");
    let focus_y = marks.iter().find(|m| m.0 == RowMark::Focus).unwrap().1;
    assert!(focus_y > first_focus_y, "the ruler follows the caret");
    // A theme change keeps the ruler and redraws it in the new colours.
    let light = Palette::named("galaxy-light");
    gui::apply_palette(&mut h, &light);
    let _ = h.redraw();
    assert_eq!(h.get_widget(DOC).inner().ruler_marks(), marks);
    assert_ne!(light.ruler_focus, Palette::galaxy().ruler_focus);
}

/// The ruler's band rows have a bar too, half the reading line's, so the
/// band shows by its shape and not by a tint alone; and a blank line's row
/// is as tall as the blank line, so the band never covers the next
/// paragraph or heading (GUI audit QW3, W8c-w).
#[test]
fn the_rulers_band_rows_have_a_bar_and_blank_rows_fit() {
    use textweaver_app::Command;
    use textweaver_app::aids::RowMark;
    use textweaver_app::keymap::ActionId;
    use textweaver_xilem::document::PaintStep;
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let mut refresher = gui::Refresher::default();
    // The band: the second step of the ruler.
    let _ = app.dispatch(Command::Action(ActionId::RulerCycle));
    let _ = app.dispatch(Command::Action(ActionId::RulerCycle));
    let _ = app.dispatch(Command::Action(ActionId::NextParagraph));
    let _ = app.dispatch(Command::Action(ActionId::NextParagraph));
    let _ = refresher.refresh(&app, &mut h);
    let _ = h.redraw();
    let view = h.get_widget(DOC);
    let marks = view.inner().ruler_marks();
    assert!(marks.iter().any(|m| m.0 == RowMark::Band), "{marks:?}");
    let bars: Vec<_> = view
        .inner()
        .painted()
        .iter()
        .filter_map(|s| match s {
            PaintStep::RulerBar(m, r) => Some((*m, *r)),
            _ => None,
        })
        .collect();
    let marked: Vec<_> = marks
        .iter()
        .filter(|m| matches!(m.0, RowMark::Focus | RowMark::Band))
        .collect();
    assert_eq!(bars.len(), marked.len(), "one bar per marked row");
    for ((mark, bar), (m, y0, y1)) in bars.iter().zip(&marked) {
        assert_eq!(mark, m);
        assert_eq!((bar.y0, bar.y1), (*y0, *y1));
        let want = if *m == RowMark::Focus { 4.0 } else { 2.0 };
        assert!((bar.width() - want).abs() < 1e-6, "{m:?}: {bar:?}");
    }
    // Rows follow one another without overlapping, blank lines included.
    for pair in marks.windows(2) {
        assert!(
            pair[1].1 >= pair[0].2 - 0.5,
            "row at {} overlaps the row ending at {}: {marks:?}",
            pair[1].1,
            pair[0].2
        );
    }
}

/// In High Contrast the ruler's bands are the page's own color, so lines
/// in the focus color mark the reading line's and the band's edges; other
/// themes draw none (alpha.8 screenshots).
#[test]
fn the_ruler_shows_its_edges_in_high_contrast() {
    use textweaver_app::Command;
    use textweaver_app::keymap::ActionId;
    use textweaver_xilem::document::PaintStep;
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let mut refresher = gui::Refresher::default();
    let _ = app.dispatch(Command::Action(ActionId::RulerCycle));
    let _ = app.dispatch(Command::Action(ActionId::RulerCycle));
    let _ = app.dispatch(Command::Action(ActionId::NextParagraph));
    let _ = app.dispatch(Command::Action(ActionId::NextParagraph));
    let _ = refresher.refresh(&app, &mut h);
    let _ = h.redraw();
    let edges = |h: &TestHarness<Root>| {
        h.get_widget(DOC)
            .inner()
            .painted()
            .iter()
            .filter(|s| matches!(s, PaintStep::RulerEdge(_)))
            .count()
    };
    assert_eq!(edges(&h), 0, "Galaxy's bands show by their tint");
    gui::apply_palette(&mut h, &Palette::named("high-contrast"));
    let _ = h.redraw();
    // The reading line's top and bottom, and the band's.
    assert!(edges(&h) >= 3, "{}", edges(&h));
}

/// Syllables (Alt+Shift+Z) are drawn between the chars of long words, as
/// the terminal draws them, and the text runs a screen reader gets stay the
/// words. Turning them off takes the marks away again: a reading aid turned
/// on or off lays the paragraphs out again while keeping their run nodes.
#[test]
fn syllables_are_drawn_and_the_text_stays_the_words() {
    use textweaver_app::Command;
    use textweaver_app::keymap::ActionId;
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let mut refresher = gui::Refresher::default();
    let _ = refresher.refresh(&app, &mut h);
    let _ = h.redraw();
    let doc_id = h.get_widget(DOC).id();
    let text_before = h.access_node(doc_id).unwrap().document_range().text();
    let runs_before: Vec<_> = h
        .access_node(doc_id)
        .unwrap()
        .children()
        .map(|c| c.id())
        .collect();
    assert_eq!(h.get_widget(DOC).inner().syllable_marks_on_screen(), 0);

    // The GUI's keymap has a chord for it on this platform.
    let chord = app
        .keymap()
        .chords_in_mode(ActionId::SyllablesToggle, app.mode().layer())
        .into_iter()
        .find(|c| !c.is_text_input())
        .expect("syllables have a chord");
    assert_eq!(
        app.keymap().lookup(&chord, app.mode().layer()),
        Some(ActionId::SyllablesToggle)
    );
    let _ = app.dispatch(Command::Action(ActionId::SyllablesToggle));
    let _ = refresher.refresh(&app, &mut h);
    let _ = h.redraw();
    let model = gui::model_for(&app, app.session().unwrap().doc.full_range()).unwrap();
    assert!(!model.breaks.is_empty(), "the sample has long words");
    assert_eq!(model.separator, "\u{b7}");
    let marks = h.get_widget(DOC).inner().syllable_marks_on_screen();
    assert!(marks > 0, "separators are drawn");
    let text_after = h.access_node(doc_id).unwrap().document_range().text();
    assert_eq!(text_after, text_before, "syllables change no text");
    assert!(!text_after.contains('\u{b7}'));
    // The run nodes that stay keep their ids, so a screen reader keeps its
    // place (the paragraphs are laid out again underneath).
    let runs_after: Vec<_> = h
        .access_node(doc_id)
        .unwrap()
        .children()
        .map(|c| c.id())
        .collect();
    assert!(
        runs_after
            .iter()
            .filter(|id| runs_before.contains(id))
            .count()
            > runs_before.len() / 2
    );

    let _ = app.dispatch(Command::Action(ActionId::SyllablesToggle));
    let _ = refresher.refresh(&app, &mut h);
    let _ = h.redraw();
    assert_eq!(h.get_widget(DOC).inner().syllable_marks_on_screen(), 0);
}

/// The app's yes-or-no questions (a voice download after its size and
/// licence are said, a removal, a file changed on disk) are a dialog in the
/// window: named by the question, with Yes and No buttons whose keys are Y
/// and N, and typed answers as in the terminal.
#[test]
fn a_question_is_a_dialog_with_yes_and_no() {
    use textweaver_app::Confirm;
    use textweaver_xilem::dialog::DialogAction;
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let question = "Download Ada Example's voice, 63 megabytes, license CC BY 4.0? y or n";
    let q = gui::question_dialog(&Palette::galaxy(), &app.catalog(), question, None);
    let (yes, no) = (q.yes, q.no);
    assert_eq!(
        q.focus, yes,
        "a question that destroys nothing starts on Yes"
    );
    h.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(q.modal)));
    h.focus_on(Some(yes));
    let _ = h.redraw();
    // The dialog is named by the question alone: the Yes and No buttons
    // show the keys, so "y or n" is not said twice.
    let dialogs = names_of(&h, Role::Dialog);
    assert_eq!(
        dialogs,
        ["Download Ada Example's voice, 63 megabytes, license CC BY 4.0?".to_owned()]
    );
    let buttons = names_of(&h, Role::Button);
    assert!(buttons.contains(&"Yes".to_owned()) && buttons.contains(&"No".to_owned()));
    for (id, key) in [(yes, "Y"), (no, "N")] {
        let node = h.access_node(id).unwrap();
        assert_eq!(node.data().keyboard_shortcut(), Some(key));
    }
    // Typed answers: y is yes, n is no, anything else asks again; Escape is no.
    let answer = |h: &mut TestHarness<Root>, key: Key| {
        h.process_text_event(TextEvent::key_down(key));
        h.pop_action::<DialogAction>().map(|(a, _)| a)
    };
    assert_eq!(
        answer(&mut h, Key::Character("y".into())),
        Some(DialogAction::Answer(Confirm::Yes))
    );
    assert_eq!(
        answer(&mut h, Key::Character("n".into())),
        Some(DialogAction::Answer(Confirm::No))
    );
    assert_eq!(
        answer(&mut h, Key::Character("q".into())),
        Some(DialogAction::Answer(Confirm::Repeat))
    );
    assert_eq!(
        answer(&mut h, Key::Named(NamedKey::Escape)),
        Some(DialogAction::Cancel)
    );
}

/// The window's command palette lists only commands the window runs: the
/// terminal reader's scrolling and line numbers are left out, as the
/// menus leave them out; a key for one says where it works (W8c-w).
#[test]
fn the_palette_leaves_out_terminal_only_commands() {
    use textweaver_app::keymap::ActionId;
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_sample(dir.path());
    let all = app.palette_candidates("");
    let shown = gui::window_palette(&app, "");
    for a in [
        ActionId::ScrollDown,
        ActionId::ScrollUp,
        ActionId::ToggleLineNumbers,
    ] {
        assert!(!shown.iter().any(|(b, _)| *b == a), "{a:?} is listed");
    }
    // Everything else the app lists is kept, in the same order.
    let kept: Vec<_> = all
        .iter()
        .filter(|(a, _)| textweaver_xilem::menus::in_window(*a))
        .cloned()
        .collect();
    assert_eq!(shown, kept);
    assert!(shown.iter().any(|(a, _)| *a == ActionId::PlayPause));
    assert!(
        app.palette_candidates("scroll")
            .iter()
            .any(|(a, _)| *a == ActionId::ScrollDown)
    );
    assert!(
        !gui::window_palette(&app, "scroll")
            .iter()
            .any(|(a, _)| *a == ActionId::ScrollDown)
    );
    assert_eq!(
        app.catalog().tr("app-terminal-only"),
        "This command works in the terminal reader."
    );
}

/// A question that deletes, removes or replaces something starts on No,
/// so Enter keeps things, and its confirming button says the verb; the
/// keys Y and N are unchanged (W8c-w).
#[test]
fn a_destructive_question_starts_on_no_with_a_verb() {
    use textweaver_app::DestructiveVerb;
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_sample(dir.path());
    for (verb, word) in [
        (DestructiveVerb::Delete, "Delete"),
        (DestructiveVerb::Remove, "Remove"),
        (DestructiveVerb::Replace, "Replace"),
    ] {
        let mut h = harness(&app);
        let question = "Ada Example notes.md already exists. Replace it? y or n.";
        let q = gui::question_dialog(&Palette::galaxy(), &app.catalog(), question, Some(verb));
        let (yes, no) = (q.yes, q.no);
        assert_eq!(q.focus, no, "{verb:?} starts on No");
        h.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(q.modal)));
        h.focus_on(Some(q.focus));
        let _ = h.redraw();
        let buttons = names_of(&h, Role::Button);
        assert!(buttons.contains(&word.to_owned()), "{buttons:?}");
        assert!(!buttons.contains(&"Yes".to_owned()), "{buttons:?}");
        assert_eq!(
            h.access_node(yes).unwrap().data().keyboard_shortcut(),
            Some("Y")
        );
        assert_eq!(
            h.access_node(no).unwrap().data().keyboard_shortcut(),
            Some("N")
        );
        assert_eq!(h.focused_widget_id(), Some(no));
    }
}

/// The voice manager (Ctrl+Shift+V in the window) is the app's list, with
/// only voices in it: the language and engine filters are buttons beside
/// it (W7v). In its dialog, Enter, Space (a favorite), and Delete (a
/// downloaded voice) go to the app's list model, as in the terminal.
#[test]
fn the_voice_manager_is_the_apps_list_with_its_keys() {
    use textweaver_app::keymap::{ActionId, KeyChord};
    use textweaver_app::{Command, Effect, ListKey};
    use textweaver_xilem::dialog::DialogAction;
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    // The keymap's key for the platform (Cmd+Shift+V on macOS,
    // Ctrl+Shift+V elsewhere), from the keymap itself.
    let chord: KeyChord = app
        .keymap()
        .chords_for(ActionId::ChooseVoice)
        .into_iter()
        .find(|c| !c.is_text_input())
        .expect("Choose Voice has a chord");
    assert_eq!(
        app.keymap().lookup(&chord, app.mode().layer()),
        Some(ActionId::ChooseVoice)
    );
    let effects = app.dispatch(Command::Action(ActionId::ChooseVoice));
    let (title, items) = effects
        .into_iter()
        .find_map(|e| match e {
            Effect::ShowList { title, items } => Some((title, items)),
            _ => None,
        })
        .expect("the voice list");
    assert!(
        !items
            .iter()
            .any(|i| i.starts_with("Language: ") || i.starts_with("Engine: ")),
        "{items:?}"
    );
    let controls = app.voice_controls().expect("the filters, as buttons");
    assert!(controls.language.starts_with("Language: "));
    assert!(controls.engine.starts_with("Engine: "));
    let mut h = harness(&app);
    let selected = app.list_model().map_or(0, |m| m.selected);
    let d =
        textweaver_xilem::voices::voice_dialog(&Palette::galaxy(), &app, &title, items, selected);
    let list_id = d.list;
    h.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(d.modal)));
    h.focus_on(Some(list_id));
    let _ = h.redraw();
    let mut key = |k: Key| {
        h.process_text_event(TextEvent::key_down(k));
        h.pop_action::<DialogAction>().map(|(a, _)| a)
    };
    assert_eq!(
        key(Key::Named(NamedKey::Enter)),
        Some(DialogAction::Key(ListKey::Enter))
    );
    assert_eq!(
        key(Key::Character(" ".into())),
        Some(DialogAction::Key(ListKey::Char(' ')))
    );
    assert_eq!(
        key(Key::Named(NamedKey::Delete)),
        Some(DialogAction::Key(ListKey::Delete))
    );
}

/// `action`'s key typed in the document, as the window's driver sees it:
/// the key is the app's keymap's for this platform and the app's mode
/// (never a key the platform does not use), pressed as the platform sends
/// it (`keys::press`); the view leaves it for the keymap (a `KeyAction` from
/// the root), and the keymap names the action it reaches.
fn press(
    h: &mut TestHarness<Root>,
    app: &textweaver_app::App,
    action: textweaver_app::keymap::ActionId,
    modified: bool,
) -> Option<textweaver_app::keymap::ActionId> {
    use textweaver_xilem::keys;
    let platform = textweaver_app::keymap::Platform::current();
    let layer = app.mode().layer();
    let chord = app
        .keymap()
        .chords_in_mode(action, layer)
        .into_iter()
        // The view keeps its caret keys; `modified` asks for a chord with a
        // modifier (one that works with single-key shortcuts off).
        .find(|c| !keys::is_native(c, platform) && c.is_text_input() != modified)
        .unwrap_or_else(|| panic!("{action:?} has a key in {layer:?}"));
    h.process_text_event(TextEvent::Keyboard(keys::press(&chord, platform)));
    let (KeyAction(k), _) = h.pop_action::<KeyAction>()?;
    let chord = keys::chord(&k, platform)?;
    app.keymap().lookup(&chord, layer)
}

/// Parity with the terminal reader: the outline, the notes list, the
/// access modes, and tables and links by key reach the app from the
/// document, and do what they do in the terminal (the lists open as the
/// GUI's list dialogs, through `Effect::ShowList`).
#[test]
fn outline_notes_access_modes_tables_and_links_work_from_the_document() {
    use textweaver_app::keymap::ActionId;
    use textweaver_app::{Command, Effect, ListKey};
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let doc = h.get_widget(DOC).id();
    h.focus_on(Some(doc));
    let text = app.session().unwrap().doc.text().to_string();
    let at_cursor = |app: &textweaver_app::App, n: usize| -> String {
        let s = app.session().unwrap();
        text.chars().skip(s.cursor.0).take(n).collect()
    };
    let list_of = |effects: &[Effect]| {
        effects.iter().find_map(|e| match e {
            Effect::ShowList { title, items } => Some((title.clone(), items.clone())),
            _ => None,
        })
    };

    // Alt+O: the outline, as a list; Enter jumps to a heading.
    let a = press(&mut h, &app, ActionId::Outline, true);
    assert_eq!(a, Some(ActionId::Outline));
    let effects = app.dispatch(Command::Action(ActionId::Outline));
    let (_, items) = list_of(&effects).expect("the outline is a list");
    let table = items
        .iter()
        .position(|i| i.contains("A Table"))
        .expect("the outline lists the headings");
    let _ = app.dispatch(Command::ListFocus(table));
    let _ = app.dispatch(Command::ListKey(ListKey::Enter));
    assert!(app.list_model().is_none(), "Enter closes the outline");
    assert!(
        at_cursor(&app, 7).starts_with("A Table"),
        "{}",
        at_cursor(&app, 20)
    );

    // t and Shift+T: tables (browse keys); Ctrl+T (Cmd+T) in any layer.
    let _ = app.dispatch(Command::SetCursor(textweaver_app::core::CharPos::ZERO));
    let a = press(&mut h, &app, ActionId::NextTable, true);
    assert_eq!(a, Some(ActionId::NextTable));
    let a = press(&mut h, &app, ActionId::NextTable, false);
    assert_eq!(a, Some(ActionId::NextTable));
    let _ = app.dispatch(Command::Action(ActionId::NextTable));
    assert!(at_cursor(&app, 4) == "Name", "{:?}", at_cursor(&app, 20));
    // In a table, Ctrl+Alt+Down moves down a row in the same column.
    let a = press(&mut h, &app, ActionId::TableNextRow, true);
    assert_eq!(a, Some(ActionId::TableNextRow));
    let _ = app.dispatch(Command::Action(ActionId::TableNextRow));
    assert!(at_cursor(&app, 3) == "Ada", "{:?}", at_cursor(&app, 20));

    // k: the next link (a browse key, from the start).
    let _ = app.dispatch(Command::SetCursor(textweaver_app::core::CharPos::ZERO));
    let a = press(&mut h, &app, ActionId::NextLink, false);
    assert_eq!(a, Some(ActionId::NextLink));
    let _ = app.dispatch(Command::Action(ActionId::NextLink));
    assert!(at_cursor(&app, 4) == "link", "{:?}", at_cursor(&app, 20));

    // Alt+Shift+A: the access modes, as in the terminal.
    let a = press(&mut h, &app, ActionId::CycleAccessMode, true);
    assert_eq!(a, Some(ActionId::CycleAccessMode));
    let before = app.access_mode();
    let _ = app.dispatch(Command::Action(ActionId::CycleAccessMode));
    assert_ne!(app.access_mode(), before);

    // Ctrl+Shift+N: the notes list, after adding a note.
    let effects = app.dispatch(Command::Action(ActionId::AddNote));
    assert!(
        effects.iter().any(|e| matches!(e, Effect::Prompt { .. })),
        "{effects:?}"
    );
    let _ = app.dispatch(Command::Answer("Check this link".into()));
    let a = press(&mut h, &app, ActionId::ListNotes, true);
    assert_eq!(a, Some(ActionId::ListNotes));
    let effects = app.dispatch(Command::Action(ActionId::ListNotes));
    let (_, items) = list_of(&effects).expect("the notes list");
    assert!(
        items.iter().any(|i| i.contains("Check this link")),
        "{items:?}"
    );
}

/// Options scrolled out of a list's box stay in the tree a screen reader
/// gets, with their scrolled bounds: AccessKit's own filter (the one the
/// UI Automation and AT-SPI adapters use) keeps every one, at the top and
/// after End. Before W4s, the list said it clipped its children, and the
/// filter left out every option past the first one beyond each edge.
#[test]
fn every_option_of_a_long_list_stays_in_the_tree() {
    use accesskit_consumer::common_filter;
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let p = Palette::galaxy();
    let items: Vec<String> = (1..=40).map(|i| format!("Bookmark {i}")).collect();
    let (modal, list_id) = gui::list_dialog(&p, &Catalog::english(), "Bookmarks", items, 0, false);
    h.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(modal)));
    h.focus_on(Some(list_id));
    let _ = h.redraw();
    let check = |h: &TestHarness<Root>, when: &str| {
        let list = h.access_node(h.get_widget(LIST).id()).unwrap();
        let all: Vec<String> = list
            .children()
            .map(|o| o.label().unwrap_or_default())
            .collect();
        assert_eq!(all.len(), 40, "{when}");
        let kept: Vec<String> = list
            .filtered_children(common_filter)
            .map(|o| o.label().unwrap_or_default())
            .collect();
        assert_eq!(kept, all, "{when}: AccessKit's filter keeps every option");
        assert!(!list.clips_children(), "{when}: the list claims to clip");
        let list_box = list.bounding_box().expect("the list has bounds");
        let outside = list
            .children()
            .filter(|o| {
                o.bounding_box()
                    .is_some_and(|b| b.intersect(list_box).is_empty())
            })
            .count();
        assert!(
            outside > 20,
            "{when}: most options are scrolled out of view"
        );
    };
    check(&h, "at the top");
    h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::End)));
    let _ = h.redraw();
    let list = h.access_node(h.get_widget(LIST).id()).unwrap();
    let last = list.children().last().unwrap();
    assert!(
        last.is_selected() == Some(true),
        "End selects the last option"
    );
    assert_eq!(list.active_descendant().map(|n| n.id()), Some(last.id()));
    // "40 of 40": AccessKit's position is zero-based (the adapters add
    // one), and the adapters read the size from the list.
    assert_eq!(last.position_in_set(), Some(39));
    assert_eq!(last.size_of_set_from_container(&common_filter), Some(40));
    check(&h, "after End");
    // The rows scrolled into view are drawn with their text, not blank:
    // the option before the last has more than its background's colors.
    let list = h.access_node(h.get_widget(LIST).id()).unwrap();
    let row = list
        .children()
        .nth(38)
        .and_then(|o| o.bounding_box())
        .expect("option 39 has bounds");
    let img = h.render();
    let mut colors = std::collections::HashSet::new();
    for y in row.y0.ceil() as u32..row.y1.floor() as u32 {
        for x in row.x0.ceil() as u32..(row.x0 + 120.0) as u32 {
            if x < img.width() && y < img.height() {
                colors.insert(img.get_pixel(x, y).0);
            }
        }
    }
    assert!(
        colors.len() > 4,
        "option 39 is drawn with its text after End ({} colors)",
        colors.len()
    );
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
            aids: false,
            colors: false,
            voices: false,
            edit: false,
            panel: None,
            ruler: false,
            reading: false,
            settings_filter: None,
        };
        textweaver_xilem::screenshot::screenshot(&o).unwrap();
        let bytes = std::fs::read(dir.path().join(name)).unwrap();
        assert!(bytes.starts_with(b"\x89PNG"));
    }
    let a = std::fs::metadata(dir.path().join("a.png")).unwrap().len();
    let b = std::fs::metadata(dir.path().join("b.png")).unwrap().len();
    assert!(b > a, "the 200% screenshot is larger");
}

/// The window at `size` (logical pixels) and `scale`, with a long status
/// message, laid out until the bars have folded or unfolded.
fn harness_at(app: &textweaver_app::App, size: (u32, u32), scale: f64) -> TestHarness<Root> {
    let p = Palette::galaxy();
    let tree = gui::build_tree(
        &p,
        Default::default(),
        Some(app),
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
    gui::refresh_for_tests(app, &mut h);
    let _ = h.redraw();
    let _ = h.redraw();
    h
}

/// The buttons inside the first region with `role` (the header's Banner,
/// the Toolbar), in tree order, which is the Tab order: each name and
/// its bounds.
fn bar_buttons_of(h: &TestHarness<Root>, role: Role) -> Vec<(String, masonry::accesskit::Rect)> {
    let mut stack = vec![h.access_tree().state().root()];
    let mut region = None;
    while let Some(n) = stack.pop() {
        if n.role() == role {
            region = Some(n);
            break;
        }
        stack.extend(n.children());
    }
    let mut out = Vec::new();
    let Some(region) = region else {
        return out;
    };
    let mut stack = vec![region];
    while let Some(n) = stack.pop() {
        if n.role() == Role::Button
            && let Some(b) = n.bounding_box()
        {
            out.push((n.label().unwrap_or_default(), b));
        }
        let mut children: Vec<_> = n.children().collect();
        children.reverse();
        stack.extend(children);
    }
    out
}

/// Customize buttons (B1-cb, B1-g2c): with custom sets (a long header, a
/// toolbar without Play, and an empty toolbar) at every review size,
/// every button is inside the window and at least 24 by 24, and the Tab
/// order follows what is drawn: row by row, left to right in each bar.
#[test]
fn custom_button_sets_fit_every_review_size_in_drawn_order() {
    use textweaver_app::buttons::Bar;
    use textweaver_app::keymap::ActionId;
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    for a in [
        ActionId::ListBookmarks,
        ActionId::AddNote,
        ActionId::ListNotes,
        ActionId::StudyCards,
        ActionId::SelfTest,
    ] {
        let _ = app.add_button(Bar::Header, a);
    }
    let _ = app.remove_button(Bar::Toolbar, 0);
    assert_eq!(app.bar_buttons(Bar::Header).len(), 10);
    assert!(!app.bar_buttons(Bar::Toolbar).contains(&ActionId::PlayPause));
    let long = app.bar_buttons(Bar::Toolbar).len();
    for set in ["long header, no Play", "empty toolbar"] {
        if set == "empty toolbar" {
            for _ in 0..long {
                let _ = app.remove_button(Bar::Toolbar, 0);
            }
            assert!(app.bar_buttons(Bar::Toolbar).is_empty());
        }
        for (size, scale) in [
            ((1100, 780), 1.0),
            ((960, 540), 1.0),
            ((683, 384), 2.0),
            ((420, 320), 1.0),
        ] {
            let h = harness_at(&app, size, scale);
            let at = format!("{set}, {}x{} at {}%", size.0, size.1, scale * 100.0);
            let window = masonry::kurbo::Rect::new(
                0.0,
                0.0,
                f64::from(size.0) * scale,
                f64::from(size.1) * scale,
            );
            for role in [Role::Banner, Role::Toolbar] {
                let buttons = bar_buttons_of(&h, role);
                for (name, b) in &buttons {
                    assert!(
                        b.x0 >= window.x0 - 0.5
                            && b.y0 >= window.y0 - 0.5
                            && b.x1 <= window.x1 + 0.5
                            && b.y1 <= window.y1 + 0.5,
                        "{at}: {name:?} at {b:?} leaves the window {window:?}"
                    );
                    assert!(
                        b.width() >= 24.0 * scale - 0.5 && b.height() >= 24.0 * scale - 0.5,
                        "{at}: {name:?} is smaller than 24 by 24: {b:?}"
                    );
                }
                for pair in buttons.windows(2) {
                    let ((a, ra), (b, rb)) = (&pair[0], &pair[1]);
                    let same_row = (ra.y0 - rb.y0).abs() < 2.0;
                    assert!(
                        (same_row && ra.x0 < rb.x0) || rb.y0 > ra.y0 + 2.0,
                        "{at}: Tab goes from {a:?} at {ra:?} to {b:?} at {rb:?}"
                    );
                }
            }
            let header = bar_buttons_of(&h, Role::Banner);
            assert!(
                header.iter().any(|(n, _)| n == "Commands"),
                "{at}: Commands stays: {header:?}"
            );
        }
    }
}

/// A change in Customize buttons shows at once (B1-g2c): the window
/// builds its bars again from the app's lists, in the order given.
#[test]
fn the_bars_are_built_again_when_their_buttons_change() {
    use textweaver_app::buttons::Bar;
    use textweaver_app::keymap::ActionId;
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let mut h = harness_at(&app, (1100, 780), 1.0);
    let before = bar_buttons_of(&h, Role::Toolbar).len();
    assert_eq!(before, app.bar_buttons(Bar::Toolbar).len());
    let _ = app.add_button(Bar::Toolbar, ActionId::ListNotes);
    let _ = app.move_bar_button(Bar::Toolbar, before, true);
    let _ = app.remove_button(Bar::Header, 0);
    let built = gui::rebuild_bars(&mut h, &app, false, false);
    let _ = h.redraw();
    assert!(!built.differ_from(&app), "the drawn bars are the app's");
    assert_eq!(built.toolbar.len(), before + 1);
    let toolbar = bar_buttons_of(&h, Role::Toolbar);
    assert_eq!(toolbar.len(), before + 1);
    let header = bar_buttons_of(&h, Role::Banner);
    assert_eq!(header.len(), app.bar_buttons(Bar::Header).len());
    assert!(!header.iter().any(|(n, _)| n == "Open"), "{header:?}");
}

/// The bounds test (W9b-n): at the review sizes, from the largest to a
/// 1366 by 768 laptop at 200 percent and the smallest window, every
/// control is inside the window and at least 24 by 24 (WCAG 2.5.8), the
/// status texts never overlap, and below 800 px the bars fold into one,
/// above the document, with their keys hidden on screen but still the
/// buttons' key property.
#[test]
fn no_control_leaves_the_window_at_any_review_size() {
    use textweaver_xilem::bars::FRAME;
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    // The longest status the window shows, with the position beside it.
    app.announce(
        "Speed reading is on: one word at a time under the document. Press the same key again to turn it off.",
        textweaver_app::a11y::Priority::Polite,
    );
    for (size, scale) in [
        ((1100, 780), 1.0),
        ((960, 540), 1.0),
        ((683, 384), 2.0),
        ((420, 320), 1.0),
    ] {
        let h = harness_at(&app, size, scale);
        let at = format!("{}x{} at {}%", size.0, size.1, scale * 100.0);
        let window = masonry::kurbo::Rect::new(
            0.0,
            0.0,
            f64::from(size.0) * scale,
            f64::from(size.1) * scale,
        );
        // Every control a screen reader finds, by its bounds on screen.
        let root = h.access_tree().state().root();
        let mut stack = vec![root];
        let mut buttons = 0;
        let mut names = Vec::new();
        let mut row_tops: Vec<f64> = Vec::new();
        while let Some(n) = stack.pop() {
            stack.extend(n.children());
            if !matches!(n.role(), Role::Button) {
                continue;
            }
            buttons += 1;
            let name = n.label().unwrap_or_default();
            let b = n.bounding_box().expect("a button has bounds");
            names.push(name.clone());
            if !row_tops.iter().any(|y| (y - b.y0).abs() < 2.0) {
                row_tops.push(b.y0);
            }
            assert!(
                b.x0 >= window.x0 - 0.5
                    && b.y0 >= window.y0 - 0.5
                    && b.x1 <= window.x1 + 0.5
                    && b.y1 <= window.y1 + 0.5,
                "{at}: {name:?} at {b:?} leaves the window {window:?}"
            );
            assert!(
                b.width() >= 24.0 * scale - 0.5 && b.height() >= 24.0 * scale - 0.5,
                "{at}: {name:?} is smaller than 24 by 24: {b:?}"
            );
        }
        // Below 800 by 480 the folded bar keeps to two rows: the buttons
        // that do not fit leave the screen and the tree, and Commands,
        // which lists them all, stays.
        let short = size.0 < 800 && size.1 < 480;
        assert_eq!(h.get_widget(FRAME).inner().is_short(), short, "{at}");
        if short {
            assert!(row_tops.len() <= 2, "{at}: rows at {row_tops:?}");
            assert!(names.iter().any(|n| n == "Commands"), "{at}: {names:?}");
            assert!(names.iter().any(|n| n == "Play"), "{at}: {names:?}");
            assert!(buttons <= 11, "{at}: {names:?}");
            // The document keeps about five lines.
            let doc = h.get_widget(DOC).ctx().bounding_box();
            assert!(doc.height() >= 130.0, "{at}: the document {doc:?}");
        } else {
            assert_eq!(buttons, 11, "{at}: the header's five and the toolbar's six");
        }
        // The status texts, in logical pixels.
        let logical = masonry::kurbo::Rect::new(0.0, 0.0, f64::from(size.0), f64::from(size.1));
        let msg = h.get_widget(gui::STATUS).ctx().bounding_box();
        let pos = h.get_widget(gui::POSITION).ctx().bounding_box();
        assert!(
            msg.intersect(pos).area() <= 0.0,
            "{at}: the message {msg:?} overlaps the position {pos:?}"
        );
        for (what, r) in [("message", msg), ("position", pos)] {
            assert!(
                r.x0 >= -0.5
                    && r.y0 >= -0.5
                    && r.x1 <= logical.x1 + 0.5
                    && r.y1 <= logical.y1 + 0.5,
                "{at}: the {what} {r:?} leaves the window"
            );
        }
        // Folded below 800 px, with the keys hidden on screen only.
        let narrow = size.0 < 800;
        assert_eq!(h.get_widget(FRAME).inner().is_folded(), narrow, "{at}");
        let play = h.get_widget(gui::PLAY);
        assert_eq!(
            play.inner().key_drawn(),
            !narrow,
            "{at}: Play's key on screen"
        );
        assert!(
            !play.inner().shortcut().is_empty(),
            "{at}: Play keeps its key"
        );
        let play_box = play.ctx().bounding_box();
        let doc_box = h.get_widget(DOC).ctx().bounding_box();
        assert_eq!(
            play_box.y1 <= doc_box.y0 + 0.5,
            narrow,
            "{at}: Play {play_box:?} is above the document {doc_box:?} only when folded"
        );
    }
}

/// In the smallest window the Contents panel, which would leave the
/// document under five lines, is hidden (not a region F6 goes to), and
/// the document keeps the row; at the laptop size it shows.
#[test]
fn a_short_window_hides_the_panel_for_the_document() {
    use textweaver_app::store::GuiSidebar;
    use textweaver_xilem::sidebar::{self, SIDEBAR, SidebarShown};
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    app.update_settings(|s| s.gui.sidebar = GuiSidebar::Contents)
        .unwrap();
    for (size, hidden) in [((420, 320), true), ((1100, 780), false)] {
        let mut h = harness_at(&app, size, 1.0);
        let p = Palette::galaxy();
        let _ = sidebar::sync(&app, &p, &mut SidebarShown::default(), &mut h);
        let _ = h.redraw();
        let _ = h.redraw();
        let s = h.get_widget(SIDEBAR);
        assert!(s.inner().is_open(), "{size:?}");
        assert_eq!(s.inner().shown_list_id().is_none(), hidden, "{size:?}");
        let doc = h.get_widget(DOC).ctx().bounding_box();
        assert!(doc.height() >= 130.0, "{size:?}: the document {doc:?}");
        let lists = names_of(&h, Role::ListBox);
        assert_eq!(lists.is_empty(), hidden, "{size:?}: {lists:?}");
    }
}

/// The View menu's Header and Toolbar commands hide and show their bars:
/// a hidden bar's buttons leave the window and the accessibility tree,
/// and the setting is remembered.
#[test]
fn the_header_and_toolbar_can_be_hidden() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    app.update_settings(|s| s.gui.toolbar = false).unwrap();
    let h = harness(&app);
    let names = names_of(&h, Role::Button);
    assert!(names.iter().any(|n| n == "Open"), "{names:?}");
    assert!(!names.iter().any(|n| n == "Play"), "{names:?}");
    assert_eq!(names.len(), 5, "{names:?}");
}

/// The review harness's newer views (W8c-x) draw at the smallest review
/// size: edit mode, the window with no document, the ruler alone, and
/// each panel (the Notes panel with its sample notes).
#[test]
fn screenshots_draw_edit_mode_no_document_the_ruler_and_the_panels() {
    use textweaver_app::store::GuiSidebar;
    let dir = tempfile::tempdir().unwrap();
    let sample = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sample.md");
    let base = textweaver_xilem::screenshot::ShotOptions {
        path: dir.path().join("unused.png"),
        file: Some(sample),
        size: (420, 320),
        scale: 1.0,
        theme: Some("galaxy".into()),
        highlight_at: None,
        list: None,
        settings: false,
        home: Some(dir.path().join("home")),
        aids: false,
        colors: false,
        voices: false,
        edit: false,
        panel: None,
        ruler: false,
        reading: false,
        settings_filter: None,
    };
    let mut shots = Vec::new();
    let mut o = base.clone();
    o.edit = true;
    shots.push(("edit.png", o));
    let mut o = base.clone();
    o.file = None;
    shots.push(("empty.png", o));
    let mut o = base.clone();
    o.ruler = true;
    o.home = Some(dir.path().join("home-ruler"));
    shots.push(("ruler.png", o));
    for (name, panel) in [
        ("contents.png", GuiSidebar::Contents),
        ("notes.png", GuiSidebar::Notes),
    ] {
        let mut o = base.clone();
        o.panel = Some(panel);
        o.home = Some(dir.path().join(name.replace(".png", "")));
        shots.push((name, o));
    }
    for (name, mut o) in shots {
        o.path = dir.path().join(name);
        textweaver_xilem::screenshot::screenshot(&o).unwrap();
        let bytes = std::fs::read(&o.path).unwrap();
        assert!(bytes.starts_with(b"\x89PNG"), "{name}");
    }
    // The panel needs a home folder, so the reader's own is never changed.
    let mut o = base;
    o.home = None;
    o.panel = Some(GuiSidebar::Contents);
    o.path = dir.path().join("refused.png");
    let err = textweaver_xilem::screenshot::screenshot(&o).unwrap_err();
    assert!(err.contains("home folder"), "{err}");
}

/// star's rule since 0.1.31: new themes go after the existing ones, so the
/// F5 cycle a reader knows never changes. The first nine, from Galaxy, in
/// the app and in the window's palettes.
#[test]
fn the_first_nine_themes_keep_their_f5_order() {
    use textweaver_app::Command;
    use textweaver_app::keymap::ActionId;
    // F5 cycles the themes that meet AA first, partners side by side.
    const FIRST_NINE: [&str; 9] = [
        "galaxy",
        "galaxy-light",
        "high-contrast",
        "contrast",
        "lamplight",
        "sepia",
        "gruvbox-dark",
        "gruvbox-light",
        "tokyo-night",
    ];
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let mut seen = vec![app.current_theme().name().to_owned()];
    for _ in 1..FIRST_NINE.len() {
        let _ = app.dispatch(Command::Action(ActionId::NextTheme));
        seen.push(app.current_theme().name().to_owned());
    }
    assert_eq!(seen, FIRST_NINE);
    // The window draws each one: its palette carries the theme's name.
    for name in FIRST_NINE {
        assert_eq!(Palette::named(name).name, name);
    }
    // F5 is the key in the window, as in the terminal.
    let f5 = app.keymap().chords_for(ActionId::NextTheme);
    assert!(f5.iter().any(|c| c.to_string() == "F5"), "{f5:?}");
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
    assert_eq!(
        names_of(&h, doc_role()),
        vec!["Sample Markdown Document, document".to_owned()]
    );
    let img = h.render();
    let px = img.get_pixel(4, 4);
    assert!(
        px[0] > 200 && px[1] > 200 && px[2] > 200,
        "a light page: {px:?}"
    );
}

/// Speech Cursor mode's own keys (Up and Down read the previous and next
/// line) reach the keymap from the document, as in the terminal; in browse
/// mode the same keys move the caret in the view.
#[test]
fn speech_cursor_line_keys_reach_the_keymap() {
    use textweaver_app::Command;
    use textweaver_app::keymap::{ActionId, Layer, Platform};
    use textweaver_xilem::keys;
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let doc = h.get_widget(DOC).id();
    h.focus_on(Some(doc));
    let platform = Platform::current();
    let down = app
        .keymap()
        .chords_in_mode(ActionId::SpeechCursorNextLine, Layer::SpeechCursor)
        .into_iter()
        .find(|c| keys::is_native(c, platform))
        .expect("a caret key reads the next line");
    // Browse mode: the view moves its caret.
    h.process_text_event(TextEvent::Keyboard(keys::press(&down, platform)));
    let moved = h.pop_action::<textweaver_xilem::document::DocAction>();
    assert!(
        matches!(
            moved,
            Some((textweaver_xilem::document::DocAction::CaretMoved { .. }, _))
        ),
        "the view kept {down}: {moved:?}"
    );
    // Speech Cursor mode: the key goes on to the keymap.
    let _ = app.dispatch(Command::Action(ActionId::SpeechCursorToggle));
    assert_eq!(app.mode().layer(), Layer::SpeechCursor);
    let mut r = gui::Refresher::default();
    let _ = r.refresh(&app, &mut h);
    h.process_text_event(TextEvent::Keyboard(keys::press(&down, platform)));
    let (KeyAction(k), _) = h
        .pop_action::<KeyAction>()
        .expect("the key reaches the keymap");
    let chord = keys::chord(&k, platform).expect("a chord");
    assert_eq!(
        app.keymap().lookup(&chord, app.mode().layer()),
        Some(ActionId::SpeechCursorNextLine)
    );
}

/// Parity with the terminal reader, the commands W6a5's brief names: reading
/// statistics, settings profiles, summaries, and define word open the app's
/// lists, which the window shows as list dialogs; Markdown lint works in
/// edit mode (the window builds with the `lint` feature, as the terminal
/// does).
#[test]
fn study_lists_summaries_profiles_and_lint_work_in_the_window() {
    use textweaver_app::keymap::ActionId;
    use textweaver_app::{Command, Effect};
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let list_of = |effects: Vec<Effect>| {
        effects.into_iter().find_map(|e| match e {
            Effect::ShowList { title, items } => Some((title, items)),
            _ => None,
        })
    };
    for action in [
        ActionId::ReadingStatistics,
        ActionId::SettingsProfiles,
        ActionId::Summarize,
        ActionId::Outline,
    ] {
        let effects = app.dispatch(Command::Action(action));
        let (title, items) = list_of(effects).unwrap_or_else(|| panic!("{action:?}: a list"));
        assert!(!items.is_empty(), "{action:?}");
        // The window shows it as its list dialog, the app's keys in it.
        let (modal, list_id) =
            gui::list_dialog(&Palette::galaxy(), &app.catalog(), &title, items, 0, true);
        h.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(modal)));
        h.focus_on(Some(list_id));
        let _ = h.redraw();
        assert!(h.get_widget(ROOT).inner().has_dialog(), "{action:?}");
        let _ = app.dispatch(Command::Cancel);
        h.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, None));
        let _ = h.redraw();
    }
    // Define word: the dictionary loads on a helper thread the first time
    // ("Dictionary still loading"), then the list comes on a later tick.
    let at = app
        .session()
        .unwrap()
        .doc
        .text()
        .to_string()
        .find("reader")
        .map_or(0, |b| b);
    let _ = app.dispatch(Command::SetCursor(textweaver_app::core::CharPos(at)));
    let started = std::time::Instant::now();
    let mut effects = app.dispatch(Command::Action(ActionId::DefineWord));
    while list_of(effects.clone()).is_none() && started.elapsed().as_secs() < 30 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        effects = app.tick(std::time::Instant::now());
        if app.list_model().is_some() {
            break;
        }
        if !app.status_text().to_lowercase().contains("loading") {
            effects = app.dispatch(Command::Action(ActionId::DefineWord));
        }
    }
    assert!(
        app.list_model().is_some() || list_of(effects).is_some(),
        "define word opened a list: {}",
        app.status_text()
    );
    let _ = app.dispatch(Command::Cancel);

    // Lint in edit mode: a problem is found, not "not in this build".
    let _ = app.dispatch(Command::Action(ActionId::ToggleEditMode));
    assert!(app.is_editing());
    let _ = app.dispatch(Command::Action(ActionId::NextLintProblem));
    let said = app.status_text().to_lowercase();
    assert!(
        said.contains("lint") && !said.contains("not in this build"),
        "{said}"
    );
}

/// Notes, the reader's highlights, and search matches are drawn in the
/// window, as in the terminal, each with a shape as well as a color.
#[test]
fn notes_highlights_and_matches_are_drawn() {
    use textweaver_app::keymap::ActionId;
    use textweaver_app::{Command, core::CharRange};
    use textweaver_xilem::document::DocMark;
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let mut r = gui::Refresher::default();
    let _ = r.refresh(&app, &mut h);
    assert!(h.get_widget(DOC).inner().marks().is_empty());
    // A highlight on a selection, and a note on it.
    let _ = app.dispatch(Command::Select(CharRange::new(2, 8)));
    let _ = app.dispatch(Command::Action(ActionId::HighlightSelection));
    let _ = app.dispatch(Command::Action(ActionId::AddNote));
    let _ = app.dispatch(Command::Answer("A note".into()));
    let _ = r.refresh(&app, &mut h);
    let marks: Vec<DocMark> = h
        .get_widget(DOC)
        .inner()
        .marks()
        .iter()
        .map(|(_, m)| *m)
        .collect();
    assert!(marks.contains(&DocMark::Highlight), "{marks:?}");
    assert!(marks.contains(&DocMark::Note), "{marks:?}");
    // They are drawn without a panic.
    let _ = h.render();
}

/// A document named on the command line that cannot be opened is said as
/// Ctrl+O says it, with a next step, not as a raw system error
/// (walkthroughs QW3).
#[test]
fn a_missing_startup_file_gets_the_friendly_message() {
    let dir = tempfile::tempdir().unwrap();
    let opts = Options {
        no_speech: true,
        home: Some(dir.path().to_path_buf()),
        ..Options::default()
    };
    let (mut app, _) = setup::build_app(&opts, Box::new(LogAnnouncer::default()));
    let missing = dir.path().join("x.md");
    let err = app.open(&missing).expect_err("there is no x.md");
    let said = gui::startup_open_message(&Catalog::english(), &missing, &err);
    assert!(said.starts_with("Could not open x.md: "), "{said}");
    assert!(said.contains("there is no file named x.md"), "{said}");
    assert!(said.ends_with("Check the name."), "{said}");
}

/// Shift+F1, then a command the window runs itself (the font list): the
/// app describes it, so the window must not run it (walkthroughs QW4).
#[test]
fn a_described_window_command_is_described_not_run() {
    use textweaver_app::Command;
    use textweaver_app::keymap::ActionId;
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let _ = app.dispatch(Command::Action(ActionId::WhatDoesThisKeyDo));
    assert!(app.describing_next_key());
    let key = Command::Action(ActionId::ChooseFont);
    assert_eq!(
        gui::window_command_of(&key, app.describing_next_key()),
        None
    );
    let _ = app.dispatch(key);
    assert!(!app.describing_next_key());
    let said = app.status_text().to_owned();
    assert!(said.starts_with("Font: "), "{said}");
    assert!(said.contains("Keys: "), "{said}");
}

/// The command palette and the keyboard shortcuts list (beta 1): each
/// option is named "Find next, F3" (the short name, then the key), has
/// the long explanation as its description, and the list keeps the key
/// for drawing at the right edge.
#[test]
fn command_rows_are_named_name_then_key_with_the_help_as_description() {
    use textweaver_app::keymap::ActionId;
    use textweaver_xilem::dialog::{ChoiceList, Rows};
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let p = Palette::galaxy();
    let ids = [ActionId::FindNext, ActionId::PlayPause, ActionId::ExportPdf];
    let names: Vec<String> = ids
        .iter()
        .map(|&a| app.command_row(a).text(&Catalog::english()))
        .collect();
    let rows = Rows::commands(&app, &ids, names.clone());
    let (modal, list_id) =
        gui::palette_dialog(&p, &Catalog::english(), "Commands", rows.items.clone());
    h.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(modal)));
    h.edit_widget(LIST, |mut l| ChoiceList::set_rows(&mut l, rows.clone()));
    h.focus_on(Some(list_id));
    let _ = h.redraw();
    let list = h.access_node(h.get_widget(LIST).id()).unwrap();
    let options: Vec<_> = list.children().collect();
    assert_eq!(options.len(), 3);
    for (i, o) in options.iter().enumerate() {
        let row = app.command_row(ids[i]);
        assert_eq!(o.label().unwrap_or_default(), names[i]);
        assert!(o.label().unwrap_or_default().starts_with(&row.name));
        assert_eq!(o.description().unwrap_or_default(), row.help);
    }
    let w = h.get_widget(LIST);
    let list = w.inner();
    assert_eq!(
        list.key(0),
        app.command_row(ActionId::FindNext).key.as_deref()
    );
    // A command without a key draws none.
    assert_eq!(list.key(2), None);
    assert_eq!(
        names[0],
        format!("{}, {}", list.items()[0], list.key(0).unwrap())
    );
}

/// The find and replace panel in the bounds test's review sizes, from the
/// largest window to a 1366 by 768 laptop at 200 percent and the smallest
/// window (compact below 480 px high): every field, check box and button
/// is inside the window, and every button at least 24 by 24.
#[test]
fn the_find_panel_fits_every_review_size() {
    use textweaver_xilem::find_panel::{self, FindTexts};
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_sample(dir.path());
    for (size, scale) in [
        ((1100, 780), 1.0),
        ((960, 540), 1.0),
        ((683, 384), 2.0),
        ((420, 320), 1.0),
    ] {
        let mut h = harness_at(&app, size, scale);
        let short = f64::from(size.1) < find_panel::SHORT_HEIGHT;
        let d = find_panel::find_dialog(&Palette::galaxy(), &app, &FindTexts::default(), short);
        let mut back = None;
        gui::open_dialog_in(&mut h, d.modal, d.find, &mut back);
        let _ = h.redraw();
        let _ = h.redraw();
        let at = format!("{}x{} at {}%", size.0, size.1, scale * 100.0);
        let window = masonry::kurbo::Rect::new(
            0.0,
            0.0,
            f64::from(size.0) * scale,
            f64::from(size.1) * scale,
        );
        // The panel's controls (the window's own bars are behind it).
        let mut stack = vec![h.access_tree().state().root()];
        let mut dialog = None;
        while let Some(n) = stack.pop() {
            if n.role() == Role::Dialog {
                dialog = Some(n);
                break;
            }
            stack.extend(n.children());
        }
        let mut seen = Vec::new();
        let mut stack = vec![dialog.expect("the panel is open")];
        while let Some(n) = stack.pop() {
            stack.extend(n.children());
            if !matches!(n.role(), Role::Button | Role::CheckBox | Role::TextInput) {
                continue;
            }
            let name = n.label().unwrap_or_default();
            let Some(b) = n.bounding_box() else {
                continue;
            };
            seen.push(name.clone());
            assert!(
                b.x0 >= window.x0 - 0.5
                    && b.y0 >= window.y0 - 0.5
                    && b.x1 <= window.x1 + 0.5
                    && b.y1 <= window.y1 + 0.5,
                "{at}: {name:?} at {b:?} leaves the window {window:?}"
            );
            if n.role() == Role::Button {
                assert!(
                    b.width() >= 24.0 * scale - 0.5 && b.height() >= 24.0 * scale - 0.5,
                    "{at}: {name:?} is smaller than 24 by 24: {b:?}"
                );
            }
        }
        for name in [
            "Find what",
            "Replace with",
            "Across lines",
            "Replace all",
            "Close",
        ] {
            assert!(
                seen.iter().any(|s| s == name),
                "{at}: no {name:?} in {seen:?}"
            );
        }
    }
}

/// The key typed in the open list dialog, as the dialog hands it on.
fn list_dialog_key(
    h: &mut TestHarness<Root>,
    event: masonry::core::keyboard::KeyboardEvent,
) -> Option<textweaver_app::ListKey> {
    use textweaver_xilem::dialog::DialogAction;
    h.process_text_event(TextEvent::Keyboard(event));
    match h.pop_action::<DialogAction>() {
        Some((DialogAction::Key(k), _)) => Some(k),
        _ => None,
    }
}

/// `chord` ("Shift+A", "F2") as the platform sends it.
fn chord_event(chord: &str) -> masonry::core::keyboard::KeyboardEvent {
    let platform = textweaver_app::keymap::Platform::current();
    textweaver_xilem::keys::press(&chord.parse().unwrap(), platform)
}

/// A character typed with no modifier (with Caps Lock, a capital).
fn typed(c: &str) -> masonry::core::keyboard::KeyboardEvent {
    masonry::core::keyboard::KeyboardEvent {
        key: Key::Character(c.into()),
        ..Default::default()
    }
}

/// The list the app showed: its title and rows.
fn shown_list(effects: &[textweaver_app::Effect]) -> Option<(String, Vec<String>)> {
    effects.iter().find_map(|e| match e {
        textweaver_app::Effect::ShowList { title, items } => Some((title.clone(), items.clone())),
        _ => None,
    })
}

/// Shows `items` as the window shows an app list, with the focus on it.
fn show_app_list(
    h: &mut TestHarness<Root>,
    app: &textweaver_app::App,
    title: &str,
    items: Vec<String>,
) {
    let (modal, list) = gui::list_dialog(&Palette::galaxy(), &app.catalog(), title, items, 0, true);
    h.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(modal)));
    h.focus_on(Some(list));
    let _ = h.redraw();
}

/// Shows the app's open question as the window does, and returns the
/// dialog's name; asserts it starts on No with `verb` on its button.
fn show_question(h: &mut TestHarness<Root>, app: &textweaver_app::App, verb: &str) -> String {
    let q = gui::question_dialog(
        &Palette::galaxy(),
        &app.catalog(),
        app.status_text(),
        app.destructive_question(),
    );
    assert_eq!(q.focus, q.no, "Enter keeps things");
    h.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(q.modal)));
    let _ = h.redraw();
    assert!(names_of(h, Role::Button).contains(&verb.to_owned()));
    names_of(h, Role::Dialog).join("|")
}

/// The changes and comments list in the window (B1-t1): Ctrl+Shift+J
/// (Cmd+Shift+J on macOS) reaches it from the document; in its dialog, a
/// and r, Shift+A and Shift+R as capitals, and n go to the app as typed
/// letters (Caps Lock alone never makes a capital), and F2, Space and
/// Delete as the list's keys; Delete on a comment asks through the usual
/// question, which starts on No with its verb.
#[test]
fn the_changes_list_takes_its_keys_in_the_window() {
    use textweaver_app::keymap::ActionId;
    use textweaver_app::{Command, Confirm, DestructiveVerb, ListKey};
    let dir = tempfile::tempdir().unwrap();
    let opts = Options {
        no_speech: true,
        home: Some(dir.path().to_path_buf()),
        ..Options::default()
    };
    let (mut app, _) = setup::build_app(&opts, Box::new(LogAnnouncer::default()));
    let docx =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/t1/changes.docx");
    app.open(&docx).expect("the tracked changes fixture opens");
    let mut h = harness(&app);
    let doc = h.get_widget(DOC).id();
    h.focus_on(Some(doc));
    assert_eq!(
        press(&mut h, &app, ActionId::ListChanges, true),
        Some(ActionId::ListChanges)
    );
    let (title, items) =
        shown_list(&app.dispatch(Command::Action(ActionId::ListChanges))).expect("the list");
    assert_eq!(title, "Changes and comments");
    assert!(items[0].starts_with("Inserted: "), "{items:?}");

    show_app_list(&mut h, &app, &title, items.clone());
    for (event, want) in [
        (typed("a"), ListKey::Char('a')),
        (typed("r"), ListKey::Char('r')),
        (typed("n"), ListKey::Char('n')),
        (chord_event("Shift+A"), ListKey::Char('A')),
        (chord_event("Shift+R"), ListKey::Char('R')),
        (typed("A"), ListKey::Char('a')),
        (chord_event("F2"), ListKey::Rename),
        (chord_event("Space"), ListKey::Char(' ')),
        (chord_event("Delete"), ListKey::Delete),
    ] {
        assert_eq!(list_dialog_key(&mut h, event), Some(want));
    }

    // Delete on a comment: the question, as a dialog starting on No.
    let comment = items
        .iter()
        .position(|i| i.starts_with("Comment by"))
        .expect("a comment row");
    let _ = app.dispatch(Command::ListFocus(comment));
    let _ = app.dispatch(Command::ListKey(ListKey::Delete));
    assert!(app.confirmation_pending());
    assert_eq!(app.destructive_question(), Some(DestructiveVerb::Delete));
    assert_eq!(
        show_question(&mut h, &app, "Delete"),
        "Delete this comment and its replies?"
    );
    let (_, kept) = shown_list(&app.dispatch(Command::Confirm(Confirm::No))).expect("shown again");
    assert_eq!(kept, items, "No keeps the comment");

    // a on the first change accepts it: the list is shown without it.
    let _ = app.dispatch(Command::ListFocus(0));
    let (_, after) =
        shown_list(&app.dispatch(Command::ListKey(ListKey::Char('a')))).expect("shown again");
    assert_eq!(after.len(), items.len() - 1, "{after:?}");
    assert!(
        app.status_text().starts_with("Accepted. "),
        "{}",
        app.status_text()
    );
}

/// Study cards in the window (B1-f1, B1-g2c): in the study session's list
/// the digits 1 to 4, R and C reach the app as plain list key presses
/// (Caps Lock alone never makes a capital); 3 grades the card Good and
/// the list moves to the next card, said with its place. The Study cards
/// submenu is in the window's menus, from the shared menu model, with
/// every cards command; the self-test still shows as a list.
#[test]
fn the_study_list_takes_its_keys_in_the_window() {
    use textweaver_app::keymap::ActionId;
    use textweaver_app::{Command, ListKey, NoteCommand};
    use textweaver_xilem::menus::{self, Entry};
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let text = app.session().unwrap().doc.text().to_string();
    let at = |needle: &str| {
        let byte = text.find(needle).unwrap();
        textweaver_app::core::CharPos(text[..byte].chars().count())
    };
    for (word, note) in [("Lists", "Energy note"), ("A Table", "Chapter note")] {
        let _ = app.dispatch(Command::SetCursor(at(word)));
        let _ = app.dispatch(Command::Notes(NoteCommand::Add));
        let _ = app.dispatch(Command::Answer(note.into()));
    }
    let _ = app.dispatch(Command::Action(ActionId::MakeCards));
    let (title, cards) =
        shown_list(&app.dispatch(Command::Action(ActionId::StudyCards))).expect("the session");
    assert!(title.starts_with("Study cards"), "{title}");
    assert!(cards.len() >= 2, "{cards:?}");
    let mut h = harness(&app);
    show_app_list(&mut h, &app, &title, cards.clone());
    for (event, want) in [
        (typed("1"), ListKey::Char('1')),
        (typed("2"), ListKey::Char('2')),
        (typed("3"), ListKey::Char('3')),
        (typed("4"), ListKey::Char('4')),
        (typed("r"), ListKey::Char('r')),
        (typed("R"), ListKey::Char('r')),
        (typed("c"), ListKey::Char('c')),
        (chord_event("Shift+R"), ListKey::Char('R')),
    ] {
        assert_eq!(list_dialog_key(&mut h, event), Some(want));
    }
    // The window hands 3 to the app: Good, and the next card.
    let _ = app.dispatch(Command::ListKey(ListKey::Char('3')));
    assert!(
        app.status_text().starts_with("Good, next") && app.status_text().contains("Card 2 of"),
        "{}",
        app.status_text()
    );
    assert_eq!(app.list_model().map(|m| m.selected), Some(1));
    // R on the next card is the session's, not a first-letter jump.
    let _ = app.dispatch(Command::ListKey(ListKey::Char('r')));
    assert!(
        !app.status_text().starts_with("No study session"),
        "{}",
        app.status_text()
    );

    // The Study cards submenu, in the window's menus.
    fn find<'a>(entries: &'a [Entry], name: &str) -> Option<&'a [Entry]> {
        entries.iter().find_map(|e| match e {
            Entry::Submenu { label, entries } if label.replace('&', "") == name => {
                Some(entries.as_slice())
            }
            Entry::Submenu { entries, .. } => find(entries, name),
            _ => None,
        })
    }
    let tree = menus::tree(&app);
    let study = tree
        .iter()
        .find_map(|m| find(&m.entries, "Study cards"))
        .expect("a Study cards submenu");
    let mut wanted = vec![
        ActionId::MakeCards,
        ActionId::StudyCards,
        ActionId::ListCards,
    ];
    wanted.extend([
        ActionId::GradeAgain,
        ActionId::GradeHard,
        ActionId::GradeGood,
    ]);
    wanted.push(ActionId::GradeEasy);
    let top = [menus::TopMenu {
        id: tree[0].id,
        title: String::new(),
        entries: study.to_vec(),
    }];
    let offered = menus::commands(&top);
    for a in wanted {
        assert!(offered.contains(&a), "{a:?} in {offered:?}");
    }
    assert!(menus::commands(&tree).contains(&ActionId::SelfTest));
}

/// F1's help in the window (B1-hp, B1-g2c): it is an app list that
/// filters as you type, so a letter typed in it reaches the app as a list
/// key and turns it into Search help, whose title the window shows in
/// place (the dialog stays open). F1 on a search row comes from the
/// dialog as a chord the keymap names Help, which the window sends to the
/// app as the list's Introduce key: the row's help is said.
#[test]
fn typing_in_f1_help_searches_and_f1_says_a_rows_help() {
    use textweaver_app::keymap::{ActionId, Layer};
    use textweaver_app::{Command, ListKey};
    use textweaver_xilem::dialog::DialogAction;
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let (title, items) =
        shown_list(&app.dispatch(Command::Action(ActionId::Help))).expect("F1's help");
    assert_eq!(app.list_filter(), Some(""), "it filters as you type");
    let mut h = harness(&app);
    show_app_list(&mut h, &app, &title, items);
    let r = list_dialog_key(&mut h, typed("r")).expect("a typed letter");
    assert_eq!(r, ListKey::Char('r'));
    let _ = app.dispatch(Command::ListKey(r));
    assert_eq!(app.list_filter(), Some("r"));
    let search = app
        .list_model()
        .map(|m| m.title.clone())
        .unwrap_or_default();
    assert_ne!(search, title, "the title follows the search");
    assert!(app.list_model().is_some_and(|m| !m.items.is_empty()));
    // F1 on a row: a chord, which the window maps to Introduce.
    h.process_text_event(TextEvent::Keyboard(chord_event("F1")));
    let chord = match h.pop_action::<DialogAction>() {
        Some((DialogAction::Chord(c), _)) => c,
        other => panic!("F1 is a chord: {other:?}"),
    };
    assert_eq!(
        app.keymap().lookup(&chord, Layer::Global),
        Some(ActionId::Help)
    );
    let before = app.status_text().to_owned();
    let _ = app.dispatch(Command::ListKey(ListKey::Introduce));
    assert_ne!(app.status_text(), before, "the row's help is said");
}

/// A note's links in the window (B1-g1): Space on a note in the notes
/// list reaches the app as the list's Space and opens its links; letters
/// typed in the types list filter it; F2 and Delete on a link are the
/// list's keys, and Delete asks through the usual question, which starts
/// on No and says Remove.
#[test]
fn a_notes_links_take_their_keys_in_the_window() {
    use textweaver_app::{Command, Confirm, DestructiveVerb, ListKey, NoteCommand};
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_sample(dir.path());
    let text = app.session().unwrap().doc.text().to_string();
    let at = |needle: &str| {
        let byte = text.find(needle).unwrap();
        textweaver_app::core::CharPos(text[..byte].chars().count())
    };
    for (word, note) in [("Lists", "Energy note"), ("A Table", "Chapter note")] {
        let _ = app.dispatch(Command::SetCursor(at(word)));
        let _ = app.dispatch(Command::Notes(NoteCommand::Add));
        let _ = app.dispatch(Command::Answer(note.into()));
    }
    let mut h = harness(&app);
    let (title, notes) =
        shown_list(&app.dispatch(Command::Notes(NoteCommand::List))).expect("the notes");
    show_app_list(&mut h, &app, &title, notes);
    let space = list_dialog_key(&mut h, chord_event("Space")).expect("Space is the list's");
    assert_eq!(space, ListKey::Char(' '));
    let (_, links) = shown_list(&app.dispatch(Command::ListKey(space))).expect("the links");
    assert_eq!(links, ["What links here: nothing yet", "Add a link"]);

    // Add a link: the types, filtered by typing, then the target.
    let _ = app.dispatch(Command::ListKey(ListKey::End));
    let (types_title, types) =
        shown_list(&app.dispatch(Command::ListKey(ListKey::Enter))).expect("the types");
    assert_eq!(types.len(), 10);
    show_app_list(&mut h, &app, &types_title, types);
    for c in ["s", "u", "p"] {
        let k = list_dialog_key(&mut h, typed(c)).expect("a letter is the list's");
        let _ = app.dispatch(Command::ListKey(k));
    }
    assert_eq!(app.list_filter(), Some("sup"));
    let (_, targets) =
        shown_list(&app.dispatch(Command::ListKey(ListKey::Enter))).expect("the targets");
    assert_eq!(targets[0], "Chapter note");
    let (links_title, links) =
        shown_list(&app.dispatch(Command::ListKey(ListKey::Enter))).expect("the links");
    assert_eq!(links[0], "supports: Chapter note");

    // In the links dialog, F2 and Delete are the list's keys.
    show_app_list(&mut h, &app, &links_title, links);
    assert_eq!(
        list_dialog_key(&mut h, chord_event("F2")),
        Some(ListKey::Rename)
    );
    let delete = list_dialog_key(&mut h, chord_event("Delete")).expect("Delete is the list's");
    let _ = app.dispatch(Command::ListKey(delete));
    assert!(app.confirmation_pending());
    assert_eq!(app.destructive_question(), Some(DestructiveVerb::Remove));
    assert_eq!(show_question(&mut h, &app, "Remove"), "Remove this link?");
    let (_, after) = shown_list(&app.dispatch(Command::Confirm(Confirm::Yes))).expect("shown");
    assert_eq!(after, ["What links here: nothing yet", "Add a link"]);
}
