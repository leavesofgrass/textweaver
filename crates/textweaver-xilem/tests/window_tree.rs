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
        "Open",
        "Font",
        "Edit",
        "Settings",
        "Commands",
        "Slower",
        "Faster",
    ] {
        assert!(buttons.contains(&b.to_owned()), "{b} in {buttons:?}");
    }
    assert_eq!(names_of(&h, Role::Toolbar), vec!["Reading".to_owned()]);
    assert_eq!(names_of(&h, Role::Document), vec!["Document".to_owned()]);
    let status = names_of(&h, Role::Status);
    assert_eq!(status.len(), 1);
    // The terminal's title line parts, from the app (`App::title_parts`).
    assert!(status[0].contains("line 1 of"), "{status:?}");
    assert!(status[0].contains("wpm"), "{status:?}");
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
        ("Edit", ActionId::ToggleEditMode),
        ("Settings", ActionId::Settings),
        ("Commands", ActionId::CommandPalette),
        ("Play", ActionId::PlayPause),
        ("Stop", ActionId::Stop),
        ("Previous sentence", ActionId::PreviousSentence),
        ("Next sentence", ActionId::NextSentence),
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
    assert_eq!(shortcuts.get("Open"), Some(&Some("Ctrl+O".to_owned())));
    // On screen, the written form; the name is the label alone.
    let b = ActionButton::new("Open…").with_shortcut("Ctrl+O");
    assert_eq!(b.shown_text(), "Open… (Ctrl+O)");
    assert_eq!(b.name(), "Open");
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

/// In an app list, F1 and the Say Status chord reach the driver as chords
/// the keymap knows, so they repeat the list's introduction
/// (`ListKey::Introduce`), as in the terminal reader. F2 still renames.
#[test]
fn help_and_say_status_keys_in_a_list_reach_the_keymap() {
    use masonry::core::keyboard::Modifiers;
    use textweaver_app::keymap::{ActionId, Layer};
    use textweaver_xilem::dialog::DialogAction;
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_sample(dir.path());
    let mut h = harness(&app);
    let p = Palette::galaxy();
    let (modal, list_id) =
        gui::list_dialog(&p, "Bookmarks", vec!["One".into(), "Two".into()], 0, true);
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
    assert_eq!(say_status.to_string(), "Alt+End");
    let mut alt_end = masonry::core::keyboard::KeyboardEvent {
        key: Key::Named(NamedKey::End),
        ..Default::default()
    };
    alt_end.modifiers = Modifiers::ALT;
    h.process_text_event(TextEvent::Keyboard(alt_end));
    let (a, _) = h
        .pop_action::<DialogAction>()
        .expect("Alt+End reaches the driver");
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

    // The key is the same as the terminal's, in the GUI's keymap.
    let chord: textweaver_app::keymap::KeyChord = "Alt+Shift+Z".parse().expect("chord");
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

/// A key typed in the document, as the window's driver sees it: the view
/// leaves it for the keymap (a `KeyAction` from the root), and the keymap's
/// layer for the app's mode names the action.
fn press(
    h: &mut TestHarness<Root>,
    app: &textweaver_app::App,
    key: Key,
    mods: masonry::core::keyboard::Modifiers,
) -> Option<textweaver_app::keymap::ActionId> {
    let mut e = masonry::core::keyboard::KeyboardEvent {
        key,
        ..Default::default()
    };
    e.modifiers = mods;
    h.process_text_event(TextEvent::Keyboard(e));
    let (KeyAction(k), _) = h.pop_action::<KeyAction>()?;
    let chord = textweaver_xilem::keys::chord(&k, textweaver_app::keymap::Platform::current())?;
    app.keymap().lookup(&chord, app.mode().layer())
}

/// Parity with the terminal reader: the outline, the notes list, the
/// access modes, and tables and links by key reach the app from the
/// document, and do what they do in the terminal (the lists open as the
/// GUI's list dialogs, through `Effect::ShowList`).
#[test]
fn outline_notes_access_modes_tables_and_links_work_from_the_document() {
    use masonry::core::keyboard::Modifiers;
    use textweaver_app::keymap::ActionId;
    use textweaver_app::{Command, Effect, ListKey};
    // The GUI keymap on macOS turns every Ctrl chord into Cmd.
    let ctrl = if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    };
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
    let a = press(&mut h, &app, Key::Character("o".into()), Modifiers::ALT);
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

    // t and Shift+T: tables (browse keys); Ctrl+T in any layer.
    let _ = app.dispatch(Command::SetCursor(textweaver_app::core::CharPos::ZERO));
    let a = press(&mut h, &app, Key::Character("t".into()), ctrl);
    assert_eq!(a, Some(ActionId::NextTable));
    let _ = app.dispatch(Command::Action(ActionId::NextTable));
    assert!(at_cursor(&app, 4) == "Name", "{:?}", at_cursor(&app, 20));
    // In a table, Ctrl+Alt+Down moves down a row in the same column.
    let a = press(
        &mut h,
        &app,
        Key::Named(NamedKey::ArrowDown),
        ctrl | Modifiers::ALT,
    );
    assert_eq!(a, Some(ActionId::TableNextRow));
    let _ = app.dispatch(Command::Action(ActionId::TableNextRow));
    assert!(at_cursor(&app, 3) == "Ada", "{:?}", at_cursor(&app, 20));

    // k: the next link (a browse key, from the start).
    let _ = app.dispatch(Command::SetCursor(textweaver_app::core::CharPos::ZERO));
    let a = press(&mut h, &app, Key::Character("k".into()), Modifiers::empty());
    assert_eq!(a, Some(ActionId::NextLink));
    let _ = app.dispatch(Command::Action(ActionId::NextLink));
    assert!(at_cursor(&app, 4) == "link", "{:?}", at_cursor(&app, 20));

    // Alt+Shift+A: the access modes, as in the terminal.
    let a = press(
        &mut h,
        &app,
        Key::Character("A".into()),
        Modifiers::ALT | Modifiers::SHIFT,
    );
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
    let a = press(
        &mut h,
        &app,
        Key::Character("N".into()),
        ctrl | Modifiers::SHIFT,
    );
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
    let (modal, list_id) = gui::list_dialog(&p, "Bookmarks", items, 0, false);
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
        };
        textweaver_xilem::screenshot::screenshot(&o).unwrap();
        let bytes = std::fs::read(dir.path().join(name)).unwrap();
        assert!(bytes.starts_with(b"\x89PNG"));
    }
    let a = std::fs::metadata(dir.path().join("a.png")).unwrap().len();
    let b = std::fs::metadata(dir.path().join("b.png")).unwrap().len();
    assert!(b > a, "the 200% screenshot is larger");
}

/// Star's rule since 0.1.31: new themes go after the existing ones, so the
/// F5 cycle a reader knows never changes. The first nine, from Galaxy, in
/// the app and in the window's palettes.
#[test]
fn the_first_nine_themes_keep_their_f5_order() {
    use textweaver_app::Command;
    use textweaver_app::keymap::ActionId;
    const FIRST_NINE: [&str; 9] = [
        "galaxy",
        "galaxy-light",
        "one-dark",
        "one-light",
        "dark",
        "light",
        "contrast",
        "high-contrast",
        "phosphor",
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
    assert_eq!(names_of(&h, Role::Document), vec!["Document".to_owned()]);
    let img = h.render();
    let px = img.get_pixel(4, 4);
    assert!(
        px[0] > 200 && px[1] > 200 && px[2] > 200,
        "a light page: {px:?}"
    );
}
