//! The voice manager in the window (W7v), in Masonry's test harness: what
//! a screen reader finds (a dialog named for the list, a list box named
//! Voices with every voice, and buttons with names, keys, and
//! descriptions), and what each button does. Nothing here opens a real
//! window or presses a control through UI Automation.

use std::cell::Cell;
use std::rc::Rc;

use masonry::accesskit::Role;
use masonry_testing::{TestHarness, TestHarnessParams};
use textweaver_app::keymap::ActionId;
use textweaver_app::testing::recording_service;
use textweaver_app::voice_manager::VoiceControl;
use textweaver_app::{App, AppConfig, Command};
use textweaver_xilem::dialog::DialogAction;
use textweaver_xilem::gui::{self, LIST, ROOT};
use textweaver_xilem::theme::{self, Palette};
use textweaver_xilem::voices::{self, VOICE_ENGINE, VOICE_REMOVE, VoiceButton, VoiceDialog};
use textweaver_xilem::widgets::{ActionButton, Pressed, Root};

/// An app on the recording engine ("Test voice", "Second voice") with
/// the voice manager open, as the window opens it: filters as buttons.
fn app() -> App {
    let (speech, _) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    app.wait_for_speech_thread();
    app.set_voice_controls_in_list(false);
    app.dispatch(Command::Action(ActionId::ChooseVoice));
    assert!(app.voice_list_open());
    app
}

/// The voice manager's list and buttons, once it is in the harness.
struct Shown {
    list: masonry::core::WidgetId,
    buttons: Vec<(masonry::core::WidgetId, VoiceButton)>,
}

/// The window with the voice manager open over it, focused on the list.
fn harness(app: &App) -> (TestHarness<Root>, Shown) {
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
    let m = app.list_model().expect("the voice list");
    let (title, items, selected) = (m.title.clone(), m.items.clone(), m.selected);
    let VoiceDialog {
        modal,
        list,
        buttons,
    } = voices::voice_dialog(&p, app, &title, items, selected);
    h.edit_widget(ROOT, |mut r| Root::set_dialog(&mut r, Some(modal)));
    h.focus_on(Some(list));
    let _ = h.redraw();
    (h, Shown { list, buttons })
}

/// Every node of `role` in the tree: its name, keyboard shortcut, and
/// description.
fn nodes_of(h: &TestHarness<Root>, role: Role) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let mut stack = vec![h.access_tree().state().root()];
    while let Some(n) = stack.pop() {
        if n.role() == role {
            out.push((
                n.label().unwrap_or_default(),
                n.data().keyboard_shortcut().unwrap_or_default().to_owned(),
                n.description().unwrap_or_default(),
            ));
        }
        stack.extend(n.children());
    }
    out
}

/// The dialog is named for the list; the list box is named Voices and
/// holds only voices, the focused one selected.
#[test]
fn the_dialog_and_its_list_have_names_and_roles() {
    let app = app();
    let (h, shown) = harness(&app);
    let c = app.catalog();
    let dialogs = nodes_of(&h, Role::Dialog);
    assert_eq!(dialogs.len(), 1, "{dialogs:?}");
    assert_eq!(dialogs[0].0, c.tr("voice-list-title"));
    let list = h.access_node(h.get_widget(LIST).id()).expect("the list");
    assert_eq!(shown.list, h.get_widget(LIST).id());
    assert_eq!(h.focused_widget_id(), Some(shown.list), "focus on the list");
    assert_eq!(list.role(), Role::ListBox);
    assert_eq!(
        list.label().as_deref(),
        Some(c.tr("gui-voices-list").as_str())
    );
    let options: Vec<String> = list
        .children()
        .filter(|n| n.role() == Role::ListBoxOption)
        .map(|n| n.label().unwrap_or_default())
        .collect();
    assert_eq!(options, ["Test voice", "Second voice"], "only voices");
}

/// Every button has a name, its key as its keyboard shortcut (the list's
/// key for the same thing), and a description; the filters say what is
/// shown.
#[test]
fn every_button_has_a_name_a_key_and_a_description() {
    let app = app();
    let (h, d) = harness(&app);
    let c = app.catalog();
    let buttons = nodes_of(&h, Role::Button);
    let find = |name: &str| {
        buttons
            .iter()
            .find(|(n, _, _)| n == name)
            .unwrap_or_else(|| panic!("{name} in {buttons:?}"))
            .clone()
    };
    let preview_key = voices::list_shortcut(&app, ActionId::SayStatus);
    // A chord, never the single key, which jumps by letter in the list.
    assert!(preview_key.contains('+'), "{preview_key}");
    assert!(!preview_key.is_empty());
    for (name, key) in [
        (c.tr("gui-voices-use"), "Enter".to_owned()),
        (c.tr("gui-voices-preview"), preview_key),
        (c.tr("gui-voices-favorite"), "Space".to_owned()),
        (c.tr("gui-voices-remove"), "Delete".to_owned()),
        (c.tr("gui-button-close"), "Escape".to_owned()),
    ] {
        let (_, shortcut, help) = find(&name);
        assert_eq!(shortcut, key, "{name}");
        assert!(!help.is_empty(), "{name} has a description");
    }
    for name in ["Language: all languages", "Engine: all engines"] {
        let (_, _, help) = find(name);
        assert!(!help.is_empty(), "{name} has a description");
    }
    // No data folder: nothing to fetch, so no Fetch button.
    assert!(!d.buttons.iter().any(|(_, b)| *b == VoiceButton::Fetch));
    assert_eq!(d.buttons.len(), 7);
}

/// A click on a button reports which one it was, and the Engine button's
/// name follows the filter the app applied.
#[test]
fn buttons_report_themselves_and_the_filter_names_follow() {
    let mut app = app();
    let (mut h, d) = harness(&app);
    for (id, which) in d.buttons.clone() {
        h.mouse_click_on(id, None);
        let (action, from) = h.pop_action::<Pressed>().expect("pressed");
        assert_eq!(action, Pressed);
        assert_eq!(from, id);
        assert_eq!(voices::button_for(&d.buttons, from), Some(which));
    }
    app.dispatch(Command::VoiceControl(VoiceControl::NextEngine));
    let engine = app.voice_controls().unwrap().engine;
    assert_eq!(engine, "Engine: test-recording");
    h.edit_widget(VOICE_ENGINE, |mut b| {
        ActionButton::set_label(&mut b, engine.clone());
    });
    let _ = h.redraw();
    assert!(
        nodes_of(&h, Role::Button)
            .iter()
            .any(|(n, _, _)| *n == engine)
    );
}

/// A key pressed and let go, as a keyboard sends it.
fn press_and_release(h: &mut TestHarness<Root>, key: masonry::core::keyboard::Key) {
    use masonry::core::TextEvent;
    use masonry::core::keyboard::{KeyState, KeyboardEvent};
    for state in [KeyState::Down, KeyState::Up] {
        h.process_text_event(TextEvent::Keyboard(KeyboardEvent {
            state,
            key: key.clone(),
            ..KeyboardEvent::default()
        }));
    }
}

/// Enter and Space press every button of the voice manager (W8a): the
/// button reports itself once, on the key's release, and nothing else
/// sees the key.
#[test]
fn enter_and_space_press_every_button() {
    use masonry::core::keyboard::{Key, NamedKey};
    let app = app();
    let (mut h, d) = harness(&app);
    for key in [Key::Named(NamedKey::Enter), Key::Character(" ".into())] {
        for (id, which) in d.buttons.clone() {
            h.focus_on(Some(id));
            press_and_release(&mut h, key.clone());
            let (action, from) = h
                .pop_action::<Pressed>()
                .unwrap_or_else(|| panic!("{key:?} on {which:?}"));
            assert_eq!((action, from), (Pressed, id), "{key:?} on {which:?}");
            assert!(h.pop_action::<Pressed>().is_none(), "once: {which:?}");
            assert!(
                h.pop_action::<DialogAction>().is_none(),
                "{key:?} on {which:?} is the button's alone"
            );
        }
    }
}

/// The app's keys pressed on a button reach the app, as from the list
/// (W8a): before, keys pressed on a button did nothing. The chords are
/// asked of the keymap.
#[test]
fn the_apps_keys_work_on_a_button() {
    use textweaver_app::keymap::{Layer, Platform};
    let app = app();
    let (mut h, d) = harness(&app);
    for action in [ActionId::SayStatus, ActionId::Help] {
        let chord = app
            .keymap()
            .chords_for(action)
            .into_iter()
            .find(|c| !c.is_text_input())
            .unwrap_or_else(|| panic!("a chord for {action:?}"));
        assert_eq!(app.keymap().lookup(&chord, Layer::Global), Some(action));
        for (id, which) in d.buttons.clone() {
            h.focus_on(Some(id));
            let ev = textweaver_xilem::keys::press(&chord, Platform::current());
            h.process_text_event(masonry::core::TextEvent::Keyboard(ev));
            let (got, _) = h
                .pop_action::<DialogAction>()
                .unwrap_or_else(|| panic!("{chord} on {which:?}"));
            assert_eq!(got, DialogAction::Chord(chord), "{chord} on {which:?}");
        }
    }
}

/// Remove is unavailable on a voice that cannot be removed (only
/// downloaded Piper voices can): its node is disabled, its description is
/// the reason in words, and its text on screen says so beside the key. It
/// stays in the Tab order, so a screen reader can land on it and hear why.
#[test]
fn remove_is_unavailable_with_its_reason_for_voices_that_cannot_be_removed() {
    let app = app();
    let (mut h, d) = harness(&app);
    let c = app.catalog();
    assert!(!app.voice_row_removable(0), "the test engine's voices stay");
    let remove = d
        .buttons
        .iter()
        .find(|(_, b)| *b == VoiceButton::Remove)
        .map(|(id, _)| *id)
        .expect("Remove");
    assert_eq!(h.get_widget(VOICE_REMOVE).id(), remove);
    let node = h.access_node(remove).expect("the Remove node");
    assert_eq!(
        node.label().as_deref(),
        Some(c.tr("gui-voices-remove").as_str())
    );
    assert!(node.is_disabled(), "unavailable to a screen reader");
    let reason = c.tr("voice-only-piper-removable");
    assert_eq!(node.description().as_deref(), Some(reason.as_str()));
    let shown = h.get_widget(VOICE_REMOVE).inner().shown_text();
    assert_eq!(shown, "Remove, unavailable (Delete)");
    // Still focusable, and a press still reports itself (the app then
    // says the reason).
    h.focus_on(Some(remove));
    assert_eq!(h.focused_widget_id(), Some(remove));
    press_and_release(
        &mut h,
        masonry::core::keyboard::Key::Named(masonry::core::keyboard::NamedKey::Enter),
    );
    assert!(h.pop_action::<Pressed>().is_some());
    // Available again: the plain text and description come back.
    h.edit_widget(VOICE_REMOVE, |mut b| {
        ActionButton::set_unavailable(&mut b, None)
    });
    let _ = h.redraw();
    let node = h.access_node(remove).expect("the Remove node");
    assert!(!node.is_disabled());
    assert_eq!(
        node.description().as_deref(),
        Some(c.tr("gui-voices-remove-help").as_str())
    );
    assert_eq!(
        h.get_widget(VOICE_REMOVE).inner().shown_text(),
        "Remove (Delete)"
    );
}
