//! A live document in the textweaver app (beta 5, N9): its buttons, the
//! status bar and the window title, the values view and the digest in
//! the accessibility tree, and the set-up and open flows through the
//! app's prompt and list.
//!
//! The app is never launched and no audio plays: the window's tree is
//! built in Masonry's test harness, and a scripted source stands in for
//! the broker.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use masonry::accesskit::{Live, Role, TreeUpdate};
use masonry::core::keyboard::{Key, NamedKey};
use masonry::core::{TextEvent, WidgetId};
use masonry::widgets::TextArea;
use masonry_testing::{TestHarness, TestHarnessParams};
use textweaver_app::a11y::{LogAnnouncer, Shared};
use textweaver_app::core::CharPos;
use textweaver_app::keymap::ActionId;
use textweaver_app::live::{BURST, LiveEvent, LiveOptions, ScriptHandle, ScriptedSource};
use textweaver_app::{App, Command, PromptKey};
use textweaver_xilem::dialog::DialogAction;
use textweaver_xilem::gui::{self, ANNOUNCER, Buttons, DOC, POSITION, STATUS_BAR};
use textweaver_xilem::setup::{self, Options};
use textweaver_xilem::theme::{self, Palette};
use textweaver_xilem::widgets::{Announcer, Message, Root};

const TOPIC: &str = "club/log";

struct Rig {
    app: App,
    h: TestHarness<Root>,
    r: gui::Refresher,
    buttons: Buttons,
    doc: WidgetId,
    said: Shared<LogAnnouncer>,
    /// Messages already passed to the live region.
    heard: usize,
    feed: ScriptHandle,
    start: Instant,
}

/// The app with a live source open (the log view, or the values view),
/// in a window of the usual size, the document focused.
fn rig(home: &std::path::Path, values: bool) -> Rig {
    let opts = Options {
        no_speech: true,
        home: Some(home.to_path_buf()),
        ..Options::default()
    };
    let said = Shared::new(LogAnnouncer::default());
    let (mut app, _) = setup::build_app(&opts, Box::new(said.clone()));
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
    let h = TestHarness::create_with(theme::default_properties(&p), tree.root, params);
    let doc = h.get_widget(DOC).id();
    let (source, feed) = ScriptedSource::new();
    let options = LiveOptions {
        id: "club".into(),
        title: "Club log".into(),
        ..LiveOptions::default()
    };
    if values {
        let _ = app.open_live_values(Box::new(source), options);
    } else {
        let _ = app.open_live_source(Box::new(source), options);
    }
    let heard = said.lock().log.len();
    let mut rig = Rig {
        app,
        h,
        r: gui::Refresher::default(),
        buttons: Buttons::default(),
        doc,
        said,
        heard,
        feed,
        start: Instant::now(),
    };
    rig.frame();
    rig.h.focus_on(Some(doc));
    rig.frame();
    rig
}

impl Rig {
    /// One turn of the window, as `Gui::refresh` takes it: the view from
    /// the app, the app's new messages to the live region, the bars built
    /// again when their buttons changed, the live buttons' labels, and a
    /// redraw.
    fn frame(&mut self) -> TreeUpdate {
        let _ = self.r.refresh(&self.app, &mut self.h);
        let new = self.new_messages();
        if !new.is_empty() {
            self.h
                .edit_widget(ANNOUNCER, |mut a| Announcer::say(&mut a, new));
        }
        if self.buttons.differ_from(&self.app) {
            self.buttons = gui::rebuild_bars(&mut self.h, &self.app, false, false);
        }
        self.buttons.relabel_live(&mut self.h, &self.app);
        self.h.redraw().1
    }

    fn new_messages(&mut self) -> Vec<Message> {
        let new: Vec<Message> = self.said.lock().log[self.heard..]
            .iter()
            .map(|(text, priority)| Message {
                text: text.clone(),
                priority: *priority,
            })
            .collect();
        self.heard += new.len();
        new
    }

    /// A message from the source at `secs` seconds, then a tick.
    fn send_at(&mut self, json: &str, ms: u64) {
        self.feed.push_json(TOPIC, json);
        self.tick_at(ms);
    }

    fn tick_at(&mut self, ms: u64) {
        let _ = self.app.tick(self.start + Duration::from_millis(ms));
    }

    fn act(&mut self, action: ActionId) -> TreeUpdate {
        let _ = self.app.dispatch(Command::Action(action));
        self.frame()
    }

    /// The toolbar's buttons as the tree has them: id, name, and whether
    /// each is disabled, in order.
    fn toolbar(&self) -> Vec<(WidgetId, String, bool)> {
        self.buttons
            .toolbar
            .iter()
            .map(|id| {
                let n = self.h.access_node(*id).expect("a button's node");
                assert_eq!(n.role(), Role::Button);
                (*id, n.label().unwrap_or_default(), n.data().is_disabled())
            })
            .collect()
    }

    fn names(&self) -> Vec<String> {
        self.toolbar().into_iter().map(|(_, n, _)| n).collect()
    }

    /// The toolbar button named `name`.
    fn button(&self, name: &str) -> WidgetId {
        self.toolbar()
            .into_iter()
            .find(|(_, n, _)| n == name)
            .map(|(id, _, _)| id)
            .unwrap_or_else(|| panic!("{name:?} in {:?}", self.names()))
    }

    fn runs(&self) -> Vec<(String, String)> {
        self.h
            .access_node(self.doc)
            .expect("the document")
            .children()
            .map(|c| (format!("{:?}", c.id()), c.value().unwrap_or_default()))
            .collect()
    }

    fn live_nodes(&self) -> BTreeMap<String, String> {
        let mut live = BTreeMap::new();
        let mut stack = vec![self.h.access_tree().state().root()];
        while let Some(n) = stack.pop() {
            stack.extend(n.children());
            if n.live() != Live::Off {
                live.insert(
                    format!("{:?}", n.id()),
                    n.label().or_else(|| n.value()).unwrap_or_default(),
                );
            }
        }
        live
    }

    fn at(&self, needle: &str) -> CharPos {
        let t = self.app.session().unwrap().doc.text().to_string();
        let b = t
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} in {t:?}"));
        CharPos(t[..b].chars().count())
    }

    fn go(&mut self, needle: &str) {
        let p = self.at(needle);
        let _ = self.app.dispatch(Command::SetCursor(p));
        self.frame();
    }

    fn position(&self) -> String {
        self.h.get_widget(POSITION).inner().text().to_string()
    }

    fn status_bar(&self) -> String {
        let id = self.h.get_widget(STATUS_BAR).id();
        self.h
            .access_node(id)
            .and_then(|n| n.label())
            .unwrap_or_default()
    }
}

/// The live-region messages a tree update sends.
fn live_sent(update: &TreeUpdate) -> Vec<String> {
    update
        .nodes
        .iter()
        .filter(|(_, n)| n.live().is_some_and(|l| l != Live::Off))
        .map(|(_, n)| {
            n.label()
                .or_else(|| n.value())
                .unwrap_or_default()
                .to_owned()
        })
        .collect()
}

fn checkin(call: &str, name: &str) -> String {
    format!(r#"{{"callsign": "{call}", "name": "{name}", "notes": "Mobile."}}"#)
}

/// The log view's buttons: on the toolbar before Slower and Faster, in
/// order, each a button with a name in words and no key in it. Pause
/// becomes Resume and the view switch names the other view in place:
/// the node, and the focus on it, stay. Close source takes Pause and
/// itself away; the focus on it goes to the document.
#[test]
fn the_log_views_buttons_follow_its_state_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = rig(dir.path(), false);
    let names = r.names();
    let at = names
        .iter()
        .position(|n| n == "Pause updates")
        .unwrap_or_else(|| panic!("{names:?}"));
    assert_eq!(
        &names[at..],
        [
            "Pause updates",
            "Jump to newest",
            "Table view",
            "Save log",
            "Close source",
            "Slower",
            "Faster"
        ],
        "{names:?}"
    );
    assert_eq!(names[at - 1], "Edit", "after the reading buttons");
    for (id, name, disabled) in r.toolbar() {
        assert!(!name.is_empty() && !name.contains(", "), "{name:?}");
        assert!(!disabled, "{name}");
        let node = r.h.access_node(id).unwrap();
        assert!(
            node.data()
                .supports_action(masonry::accesskit::Action::Click),
            "{name}"
        );
    }
    // The header has none of them.
    let header: Vec<String> = r
        .buttons
        .header
        .iter()
        .filter_map(|id| r.h.access_node(*id).and_then(|n| n.label()))
        .collect();
    assert!(!header.iter().any(|n| n.contains("updates")), "{header:?}");
    // Pause names its key and describes itself.
    let pause = r.button("Pause updates");
    let node = r.h.access_node(pause).unwrap();
    assert_eq!(node.data().keyboard_shortcut(), Some("Alt+Shift+F1"));
    assert!(node.description().is_some_and(|d| d.contains("wait")));

    // Pressed from the focus: the same node, renamed, still focused.
    r.h.focus_on(Some(pause));
    r.act(ActionId::LiveTogglePause);
    assert!(r.app.live_paused());
    assert_eq!(r.button("Resume updates"), pause);
    assert_eq!(r.h.focused_widget_id(), Some(pause));
    r.act(ActionId::LiveTogglePause);
    assert_eq!(r.button("Pause updates"), pause);
    let view = r.button("Table view");
    r.act(ActionId::LiveSwitchView);
    assert_eq!(r.button("List view"), view);
    r.act(ActionId::LiveSwitchView);
    assert_eq!(r.button("Table view"), view);

    // Close source from its button: Pause and Close go, the focus goes to
    // the document, and the log's own buttons stay.
    let close = r.button("Close source");
    r.h.focus_on(Some(close));
    r.act(ActionId::CloseLiveSource);
    assert!(!r.app.live_receiving());
    let names = r.names();
    assert!(
        !names
            .iter()
            .any(|n| n == "Close source" || n.ends_with("updates")),
        "{names:?}"
    );
    assert!(names.contains(&"Save log".to_owned()), "{names:?}");
    assert_eq!(r.h.focused_widget_id(), Some(r.doc));

    // Another document: no live buttons at all.
    r.app.open_document(
        textweaver_app::text::Document::from_plain_text("Hello."),
        textweaver_app::store::DocKey::untitled(1),
        "Notes".into(),
    );
    r.frame();
    let names = r.names();
    assert!(
        !names
            .iter()
            .any(|n| n == "Jump to newest" || n == "Save log"),
        "{names:?}"
    );
}

/// The values view's buttons: Show changes and Speak changes say on or
/// off in words for the field on the cursor's line and follow it in
/// place; on a line with no field they are unavailable, with the reason,
/// and still in the Tab order. The values view has no table and keeps no
/// log, so neither button is there.
#[test]
fn the_values_views_field_buttons_say_on_or_off() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = rig(dir.path(), true);
    r.send_at(r#"{"temp_c": 21.4, "humidity": 55}"#, 0);
    r.frame();
    let names = r.names();
    for absent in ["Table view", "List view", "Save log"] {
        assert!(!names.iter().any(|n| n == absent), "{names:?}");
    }
    r.go("Humidity");
    let field = r.app.live_field_at_cursor().expect("a field");
    let word = |on: bool| if on { "on" } else { "off" };
    let on = field.show == textweaver_app::store::LiveNotice::OnChange;
    let show = r.button(&format!("Show changes: {}", word(on)));
    let speak = r.button("Speak changes: off");
    r.h.focus_on(Some(speak));
    r.act(ActionId::LiveToggleFieldSpeak);
    assert_eq!(r.button("Speak changes: on"), speak);
    assert_eq!(r.h.focused_widget_id(), Some(speak));
    r.act(ActionId::LiveToggleFieldShow);
    assert_eq!(r.button(&format!("Show changes: {}", word(!on))), show);

    // The heading line holds no field: unavailable, said why.
    r.go("Club log");
    assert!(r.app.live_field_at_cursor().is_none());
    let (_, name, disabled) = r
        .toolbar()
        .into_iter()
        .find(|(id, _, _)| *id == show)
        .unwrap();
    assert_eq!(name, "Show changes");
    assert!(disabled);
    let node = r.h.access_node(show).unwrap();
    assert_eq!(
        node.description().as_deref(),
        Some("There is no field on this line.")
    );
}

/// The values view in place: a changed value replaces only its own line's
/// run; every other node keeps its id; the caret and the focus stay.
#[test]
fn a_changed_value_keeps_every_other_node() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = rig(dir.path(), true);
    r.send_at(r#"{"temp_c": 21.4, "wind": 12, "humidity": 55}"#, 0);
    r.frame();
    r.go("Humidity");
    let caret = r.h.get_widget(DOC).inner().state().caret;
    let before = r.runs();
    let live_before = r.live_nodes();
    r.send_at(r#"{"temp_c": 21.6, "wind": 12, "humidity": 55}"#, 30_000);
    let update = r.frame();
    let after = r.runs();
    assert_eq!(after.len(), before.len());
    let changed: Vec<usize> = (0..after.len())
        .filter(|&i| after[i] != before[i])
        .collect();
    assert_eq!(changed.len(), 1, "{before:?} {after:?}");
    assert!(after[changed[0]].1.contains("21.6"), "{after:?}");
    assert_eq!(r.h.get_widget(DOC).inner().state().caret, caret);
    assert_eq!(r.h.focused_widget_id(), Some(r.doc));
    // At most the one change is said, once.
    let new_live: Vec<String> = r
        .live_nodes()
        .into_iter()
        .filter(|(k, _)| !live_before.contains_key(k))
        .map(|(_, v)| v)
        .collect();
    assert!(new_live.len() <= 1, "{new_live:?}");
    assert!(
        new_live.iter().all(|m| m.contains("21.6")),
        "{new_live:?} {:?}",
        live_sent(&update)
    );
}

/// The digest: the first entry after a quiet spell is one message; the
/// entries of a burst are added silently (no live node at all, the nodes
/// already there kept), then said together as one message.
#[test]
fn a_burst_is_said_once_as_a_digest() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = rig(dir.path(), false);
    r.send_at(&checkin("N0CALL", "Ada Example"), 0);
    let update = r.frame();
    let sent = live_sent(&update);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert!(sent[0].starts_with("N0CALL"), "{sent:?}");
    r.go("N0CALL");

    let calls = ["N1TEST", "N2TEST", "N3TEST"];
    for (i, call) in calls.iter().enumerate() {
        let before = r.runs();
        r.send_at(&checkin(call, "Grace Sample"), 300 * (i as u64 + 1));
        let update = r.frame();
        assert!(live_sent(&update).is_empty(), "{:?}", live_sent(&update));
        let after = r.runs();
        assert_eq!(&after[..before.len() - 1], &before[..before.len() - 1]);
    }
    let before = r.runs();
    let live_before = r.live_nodes();
    let heard = r.heard;
    let ms = u64::try_from(BURST.as_millis()).unwrap() + 300;
    r.tick_at(ms);
    let update = r.frame();
    assert_eq!(r.heard - heard, 1, "the app asks for one message");
    // One new live node, the digest; an earlier message's node may be
    // sent again as the region changes, but keeps its id.
    let new: Vec<String> = r
        .live_nodes()
        .into_iter()
        .filter(|(k, _)| !live_before.contains_key(k))
        .map(|(_, v)| v)
        .collect();
    assert_eq!(new.len(), 1, "one digest: {new:?} {:?}", live_sent(&update));
    assert!(new[0].starts_with("N3TEST"), "newest first: {new:?}");
    assert!(new[0].contains("3 new entries"), "{new:?}");
    assert_eq!(r.runs(), before, "the digest changes no text");
    // Nothing more is said on a quiet tick.
    let live_before = r.live_nodes();
    r.tick_at(ms + 5_000);
    let update = r.frame();
    assert!(live_sent(&update).is_empty(), "{:?}", live_sent(&update));
    assert_eq!(r.live_nodes(), live_before);
}

/// The status bar and the window title: the unread count where the
/// position goes, frozen while entries arrive; the connection in words.
#[test]
fn the_status_bar_says_unread_and_the_connection_in_words() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = rig(dir.path(), false);
    r.feed.push(LiveEvent::Connecting { attempt: 1 });
    r.tick_at(0);
    r.frame();
    assert!(
        r.position().starts_with("None unread, Connecting"),
        "{}",
        r.position()
    );
    r.feed.push(LiveEvent::Connected {
        encrypted: true,
        tailscale: false,
    });
    r.tick_at(10);
    r.frame();
    assert!(
        r.position()
            .starts_with("None unread, Connected, encrypted"),
        "{}",
        r.position()
    );
    r.send_at(&checkin("N0CALL", "Ada Example"), 20);
    r.go("N0CALL");
    for (i, call) in ["N1TEST", "N2TEST", "N3TEST"].iter().enumerate() {
        r.send_at(&checkin(call, "Grace Sample"), 5_000 * (i as u64 + 1));
    }
    r.frame();
    let position = r.position();
    assert!(position.starts_with("3 unread, Connected"), "{position}");
    assert!(!position.contains("Line"), "frozen: {position}");
    assert!(r.status_bar().contains("3 unread"), "{}", r.status_bar());
    assert_eq!(gui::window_title(&r.app), "Club log, 3 unread - textweaver");

    r.feed.push(LiveEvent::Disconnected {
        reason: "Connection lost to logger.example:1883.".into(),
    });
    r.tick_at(30_000);
    r.frame();
    assert!(
        r.position().starts_with("3 unread, Connection lost"),
        "{}",
        r.position()
    );
    r.feed.push(LiveEvent::Connecting { attempt: 2 });
    r.tick_at(31_000);
    r.frame();
    assert!(r.position().contains("Reconnecting"), "{}", r.position());
    r.feed.push(LiveEvent::Connected {
        encrypted: false,
        tailscale: false,
    });
    r.tick_at(32_000);
    r.frame();
    assert!(
        r.position().contains("Connected, not encrypted"),
        "{}",
        r.position()
    );

    // Closed: the entries in place of the unread count, no connection.
    r.act(ActionId::CloseLiveSource);
    assert!(r.position().starts_with("4 entries"), "{}", r.position());
    assert!(!r.position().contains("onnect"), "{}", r.position());
}

/// Set up a live source and Open live source run through the app's own
/// prompt and list, which the window shows as a labelled field and a
/// list; the password prompt shows stars, never the password.
#[cfg(feature = "mqtt")]
#[test]
fn set_up_and_open_go_through_the_prompt_and_the_list() {
    use textweaver_app::store::LiveSourceSettings;
    let dir = tempfile::tempdir().unwrap();
    let mut r = rig(dir.path(), false);
    for a in [
        ActionId::SetUpLiveSource,
        ActionId::OpenLiveSource,
        ActionId::CloseLiveSource,
    ] {
        assert!(r.app.is_available(a), "{a:?} in the menus and palette");
    }
    r.act(ActionId::SetUpLiveSource);
    let label = r
        .app
        .prompt_model()
        .expect("the form's first field")
        .label
        .clone();
    assert_eq!(label, "Name, 1 of 7");
    let p = Palette::galaxy();
    let c = r.app.catalog();
    let (modal, field) = gui::prompt_dialog(&p, &c, &label, "", "", false, true);
    let mut back = None;
    gui::open_dialog_in(&mut r.h, modal, field, &mut back);
    let _ = r.h.redraw();
    assert_eq!(
        r.h.focused_widget_id(),
        Some(field),
        "the field has the focus"
    );
    let node = r.h.access_node(field).unwrap();
    assert_eq!(node.label().as_deref(), Some(label.as_str()));
    gui::close_dialog_in(&mut r.h, &mut back);
    let _ = r.app.dispatch(Command::PromptKey(PromptKey::Escape));
    assert!(r.app.prompt_model().is_none(), "Escape cancels the form");

    // Open live source: the sources by name, in a list.
    let _ = r.app.update_settings(|s| {
        s.live.sources.insert(
            "practice".into(),
            LiveSourceSettings {
                name: "Practice log".into(),
                host: "logger.example".into(),
                topics: vec!["log/checkins".into()],
                username: "reader".into(),
                ..LiveSourceSettings::default()
            },
        );
    });
    r.act(ActionId::OpenLiveSource);
    let list = r.app.list_model().expect("the sources").clone();
    assert_eq!(
        list.items,
        ["Practice log, logger.example, log/checkins"],
        "{list:?}"
    );
    let (modal, focus) = gui::list_dialog(&p, &c, &list.title, list.items.clone(), 0, true);
    gui::open_dialog_in(&mut r.h, modal, focus, &mut back);
    let _ = r.h.redraw();
    assert_eq!(
        r.h.focused_widget_id(),
        Some(focus),
        "the list has the focus"
    );
    gui::close_dialog_in(&mut r.h, &mut back);

    // The password prompt shows one star per character.
    let _ = r.app.ask_live_password("practice");
    let model = r.app.prompt_model().expect("the password prompt");
    assert!(model.purpose.is_secret());
    let (secret, _) = gui::secret_edit("", "s3cret!", 0..0, 7);
    let _ = r
        .app
        .dispatch(Command::PromptKey(textweaver_app::PromptKey::SetText(
            secret,
        )));
    let shown = r.app.prompt_model().unwrap().shown_text();
    assert_eq!(shown, "*******");
}

/// The keys the open dialog sent, as dialog actions.
fn dialog_actions(h: &mut TestHarness<Root>) -> Vec<DialogAction> {
    let mut out = Vec::new();
    while let Some((a, _)) = h.pop_action_erased() {
        if let Ok(a) = a.downcast::<DialogAction>() {
            out.push(*a);
        }
    }
    out
}

/// The live source form in the textweaver app: each field is the app's
/// prompt in a dialog, a text field named by its label ("Port, 4 of 7"),
/// in the form's order; Tab and Shift+Tab ask the app for the next or
/// previous field, and Up and Down on a choice reach the app, which
/// changes it in place. The password asked after saving is a password
/// field to the screen reader.
#[cfg(feature = "mqtt")]
#[test]
fn the_live_source_form_names_each_field_and_tab_moves_between_them() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = rig(dir.path(), false);
    r.act(ActionId::SetUpLiveSource);
    let p = Palette::galaxy();
    let c = r.app.catalog();
    let mut back = None;
    let mut labels = Vec::new();
    for _ in 0..7 {
        let model = r.app.prompt_model().expect("a field").clone();
        assert!(model.purpose.is_form());
        let (modal, field) = gui::prompt_dialog(
            &p,
            &c,
            &model.label,
            "",
            &model.shown_text(),
            false,
            model.purpose.is_form(),
        );
        gui::open_dialog_in(&mut r.h, modal, field, &mut back);
        let _ = r.h.redraw();
        assert_eq!(r.h.focused_widget_id(), Some(field), "{}", model.label);
        let node = r.h.access_node(field).unwrap();
        assert_eq!(node.role(), Role::TextInput, "{}", model.label);
        assert_eq!(node.label().as_deref(), Some(model.label.as_str()));
        labels.push(model.label.clone());
        if model.label.starts_with("Encryption") {
            // Up and Down are the app's: they change the choice.
            r.h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::ArrowDown)));
            assert_eq!(dialog_actions(&mut r.h), [DialogAction::Recall(false)]);
            let _ = r.app.dispatch(Command::PromptKey(PromptKey::Down));
            assert_eq!(r.app.prompt_model().unwrap().text(), "Bundled roots");
        }
        r.h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::Tab)));
        assert_eq!(
            dialog_actions(&mut r.h),
            [DialogAction::Field(true)],
            "{}: Tab moves to the next field",
            model.label
        );
        gui::close_dialog_in(&mut r.h, &mut back);
        let _ = r.app.dispatch(Command::PromptKey(PromptKey::Tab));
    }
    assert_eq!(
        labels,
        [
            "Name, 1 of 7",
            "Broker host, 2 of 7",
            "Encryption, 3 of 7, Up or Down Arrow changes",
            "Port, 4 of 7",
            "Topic, 5 of 7",
            "User name, 6 of 7",
            "View, 7 of 7, Up or Down Arrow changes",
        ]
    );
    // Shift+Tab goes back.
    let model = r.app.prompt_model().unwrap().clone();
    let (modal, field) = gui::prompt_dialog(&p, &c, &model.label, "", "", false, true);
    gui::open_dialog_in(&mut r.h, modal, field, &mut back);
    let mut k = masonry::core::KeyboardEvent {
        state: masonry::core::keyboard::KeyState::Down,
        key: Key::Named(NamedKey::Tab),
        ..Default::default()
    };
    k.modifiers = masonry::core::Modifiers::SHIFT;
    r.h.process_text_event(TextEvent::Keyboard(k));
    assert_eq!(dialog_actions(&mut r.h), [DialogAction::Field(false)]);
    gui::close_dialog_in(&mut r.h, &mut back);
    let _ = r.app.dispatch(Command::PromptKey(PromptKey::Escape));

    // A secret prompt's field is a password field.
    let (modal, field) =
        gui::prompt_dialog(&p, &c, "Password for Practice log", "", "", false, false);
    gui::open_dialog_in(&mut r.h, modal, field, &mut back);
    r.h.edit_widget(gui::PROMPT_FIELD, |mut f| {
        TextArea::set_password(&mut f, true)
    });
    let _ = r.h.redraw();
    let node = r.h.access_node(field).unwrap();
    assert_eq!(node.role(), Role::PasswordInput);
    assert_eq!(node.label().as_deref(), Some("Password for Practice log"));
    gui::close_dialog_in(&mut r.h, &mut back);
}

/// The join code field in the textweaver app: a password field of
/// stars, with an unchecked Show box right after it, named in words. Tab
/// goes from the field to the box and from the box to the next field;
/// F3 asks the app to show the code. Shown, the field is an ordinary
/// text field holding the code, the box checked and the caret kept;
/// hidden again, it is a password field of stars.
#[cfg(feature = "mqtt")]
#[test]
fn the_join_code_is_a_password_field_with_a_show_box() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = rig(dir.path(), false);
    r.act(ActionId::SetUpLiveSource);
    let _ = r
        .app
        .dispatch(Command::PromptKey(PromptKey::SetText("Club".into())));
    let _ = r.app.dispatch(Command::PromptKey(PromptKey::Tab));
    let _ = r.app.dispatch(Command::PromptKey(PromptKey::Tab));
    for _ in 0..4 {
        let _ = r.app.dispatch(Command::PromptKey(PromptKey::Down));
    }
    let _ = r.app.dispatch(Command::PromptKey(PromptKey::Tab));
    let _ = r
        .app
        .dispatch(Command::PromptKey(PromptKey::SetText("abcd".into())));
    let model = r.app.prompt_model().expect("the join code field").clone();
    assert!(
        model.label.starts_with("Join code, 4 of 7"),
        "{}",
        model.label
    );
    assert!(model.is_masked());
    let p = Palette::galaxy();
    let c = r.app.catalog();
    let show_name = c.tr(model.show_message_ids().0);
    assert_eq!(show_name, "Show the join code");
    let mut back = None;
    let (modal, field) = gui::secret_prompt_dialog(
        &p,
        &c,
        &model.label,
        "",
        &model.shown_text(),
        true,
        &show_name,
    );
    gui::open_dialog_in(&mut r.h, modal, field, &mut back);
    r.h.edit_widget(gui::PROMPT_FIELD, |mut f| {
        TextArea::set_password(&mut f, true)
    });
    let _ = r.h.redraw();
    let node = r.h.access_node(field).unwrap();
    assert_eq!(node.role(), Role::PasswordInput);
    assert_eq!(node.value().as_deref(), Some("****"));
    let show = r.h.get_widget(gui::PROMPT_SHOW).id();
    let node = r.h.access_node(show).unwrap();
    assert_eq!(node.role(), Role::CheckBox);
    assert_eq!(node.label().as_deref(), Some("Show the join code"));
    assert_eq!(node.toggled(), Some(masonry::accesskit::Toggled::False));
    // Tab from the field reaches the box, and from the box the next field.
    r.h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::Tab)));
    assert_eq!(dialog_actions(&mut r.h), []);
    assert_eq!(r.h.focused_widget_id(), Some(show));
    r.h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::Tab)));
    assert_eq!(dialog_actions(&mut r.h), [DialogAction::Field(true)]);
    // F3 anywhere in the dialog asks for the code shown.
    r.h.focus_on(Some(field));
    r.h.edit_widget(gui::PROMPT_FIELD, |mut f| {
        TextArea::select_byte_range(&mut f, 2, 2)
    });
    r.h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::F3)));
    assert_eq!(dialog_actions(&mut r.h), [DialogAction::ToggleShown]);
    let _ = r.app.dispatch(Command::PromptKey(PromptKey::ToggleShown));
    let model = r.app.prompt_model().unwrap().clone();
    gui::show_secret_in(&mut r.h, &model);
    let _ = r.h.redraw();
    let node = r.h.access_node(field).unwrap();
    assert_eq!(node.role(), Role::TextInput);
    assert_eq!(node.value().as_deref(), Some("abcd"));
    let caret =
        r.h.get_widget(gui::PROMPT_FIELD)
            .inner()
            .selection_byte_range();
    assert_eq!(caret, 2..2, "the caret stays");
    let node = r.h.access_node(show).unwrap();
    assert_eq!(node.toggled(), Some(masonry::accesskit::Toggled::True));
    // Hidden again.
    let _ = r.app.dispatch(Command::PromptKey(PromptKey::ToggleShown));
    let model = r.app.prompt_model().unwrap().clone();
    gui::show_secret_in(&mut r.h, &model);
    let _ = r.h.redraw();
    let node = r.h.access_node(field).unwrap();
    assert_eq!(node.role(), Role::PasswordInput);
    assert_eq!(node.value().as_deref(), Some("****"));
    let node = r.h.access_node(show).unwrap();
    assert_eq!(node.toggled(), Some(masonry::accesskit::Toggled::False));
    gui::close_dialog_in(&mut r.h, &mut back);
    let _ = r.app.dispatch(Command::PromptKey(PromptKey::Escape));
}

/// A live document message as `tw share` or Share this document live
/// sends it.
fn shared(rev: u64, text: &str) -> String {
    serde_json::json!({
        "type": "textweaver.document", "rev": rev, "title": "Agenda",
        "format": "markdown", "text": text
    })
    .to_string()
}

/// The document view in place: a new revision replaces only the lines
/// that changed; every other node keeps its id and text, the caret and
/// the focus stay, and the only live-region message is the short
/// revision line, never the document read again.
#[test]
fn a_new_revision_keeps_every_other_node_and_says_one_short_line() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = rig(dir.path(), false);
    let agenda = "# Agenda\n\n- Check-ins\n- Traffic\n- Announcements\n- Close\n";
    r.send_at(&shared(1, agenda), 0);
    r.frame();
    assert!(r.app.live_shows_document());
    r.go("Traffic");
    let caret = r.h.get_widget(DOC).inner().state().caret;
    let before = r.runs();
    let live_before = r.live_nodes();
    let heard = r.heard;
    let changed = agenda.replace("- Announcements", "- Announcements from Ada Example");
    r.send_at(&shared(2, &changed), 1_000);
    let update = r.frame();
    let after = r.runs();
    assert_eq!(after.len(), before.len(), "{before:?} {after:?}");
    let differ: Vec<usize> = (0..after.len())
        .filter(|&i| after[i] != before[i])
        .collect();
    assert_eq!(differ.len(), 1, "{before:?} {after:?}");
    assert!(after[differ[0]].1.contains("Ada Example"), "{after:?}");
    assert_eq!(r.h.get_widget(DOC).inner().state().caret, caret);
    assert_eq!(r.h.focused_widget_id(), Some(r.doc));
    assert_eq!(r.heard - heard, 1, "the app asks for one message");
    let new_live: Vec<String> = r
        .live_nodes()
        .into_iter()
        .filter(|(k, _)| !live_before.contains_key(k))
        .map(|(_, v)| v)
        .collect();
    assert!(
        new_live.iter().all(|m| m == "Agenda updated, revision 2."),
        "{new_live:?} {:?}",
        live_sent(&update)
    );
    assert!(
        !live_sent(&update).iter().any(|m| m.contains("Check-ins")),
        "nothing read again: {:?}",
        live_sent(&update)
    );
    // The status bar says the revision, meaning first; no log buttons.
    assert!(r.position().starts_with("Revision 2"), "{}", r.position());
    let names = r.names();
    assert!(
        !names.iter().any(|n| n.contains("Table") || n == "Save log"),
        "{names:?}"
    );
}

/// Send a check-in: each field is a text input named by its label and
/// place, Tab moves to the next; the broker's state is in the status bar
/// in words.
#[cfg(feature = "mqtt")]
#[test]
fn the_check_in_form_names_each_field_and_the_broker_is_in_the_status_bar() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = rig(dir.path(), false);
    let _ = r
        .app
        .update_settings(|s| s.live.broker.listen = vec!["127.0.0.1:0".into()]);
    r.act(ActionId::StartBroker);
    assert!(r.app.live_broker_running());
    assert!(
        r.status_bar().contains("Broker on, 0 clients"),
        "{}",
        r.status_bar()
    );
    r.act(ActionId::SendCheckIn);
    let p = Palette::galaxy();
    let c = r.app.catalog();
    let mut back = None;
    let mut labels = Vec::new();
    for _ in 0..3 {
        let model = r.app.prompt_model().expect("a field").clone();
        assert!(model.purpose.is_form());
        assert!(!model.purpose.is_secret());
        let (modal, field) = gui::prompt_dialog(
            &p,
            &c,
            &model.label,
            "",
            &model.shown_text(),
            false,
            model.purpose.is_form(),
        );
        gui::open_dialog_in(&mut r.h, modal, field, &mut back);
        let _ = r.h.redraw();
        assert_eq!(r.h.focused_widget_id(), Some(field), "{}", model.label);
        let node = r.h.access_node(field).unwrap();
        assert_eq!(node.role(), Role::TextInput, "{}", model.label);
        assert_eq!(node.label().as_deref(), Some(model.label.as_str()));
        labels.push(model.label.clone());
        r.h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::Tab)));
        assert_eq!(dialog_actions(&mut r.h), [DialogAction::Field(true)]);
        gui::close_dialog_in(&mut r.h, &mut back);
        let _ = r.app.dispatch(Command::PromptKey(PromptKey::Tab));
    }
    assert_eq!(
        labels,
        ["Callsign, 1 of 3", "Name, 2 of 3", "Notes, 3 of 3"]
    );
    let _ = r.app.dispatch(Command::PromptKey(PromptKey::Escape));
    assert!(r.app.prompt_model().is_none());
    r.act(ActionId::StopBroker);
    assert!(!r.status_bar().contains("Broker on"), "{}", r.status_bar());
    r.app.shutdown();
}
