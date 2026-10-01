//! Wave 3, Agent W3a: the app core for the GUI. The shared list and prompt
//! models, `Command::ReplaceRange`, the settings schema and screen, the
//! waker, opening in the background, the settings writer, and the
//! document window over a session.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::{DocKey, Paths, SettingsStore};
use textweaver_app::testing::recording_service;
use textweaver_app::text::Document;
use textweaver_app::{
    App, AppConfig, Command, DocWindow, Effect, ListKey, PromptKey, PromptPurpose, SettingKind,
    Units, WindowChange,
};

#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Said {
    fn all(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
    fn last(&self) -> String {
        self.all().last().cloned().unwrap_or_default()
    }
    fn any(&self, needle: &str) -> bool {
        self.all().iter().any(|s| s.contains(needle))
    }
}

impl Announcer for Said {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}

fn app_with(paths: Option<Paths>) -> (App, Said) {
    let said = Said::default();
    let (speech, _log) = recording_service().unwrap();
    let app = App::new(AppConfig {
        speech,
        announcer: Box::new(said.clone()),
        paths,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    (app, said)
}

fn app(text: &str) -> (App, Said) {
    let (mut app, said) = app_with(None);
    app.open_document(
        Document::from_plain_text(text),
        DocKey::untitled(1),
        "Test".into(),
    );
    (app, said)
}

fn list_key(app: &mut App, k: ListKey) -> Vec<Effect> {
    app.dispatch(Command::ListKey(k))
}

fn prompt_key(app: &mut App, k: PromptKey) -> Vec<Effect> {
    app.dispatch(Command::PromptKey(k))
}

// The list and prompt models.

#[test]
fn lists_keep_their_focus_and_say_k_of_n() {
    let (mut app, said) = app("One.\n\nTwo.\n\nThree.");
    app.dispatch(Command::Action(ActionId::AddBookmark));
    app.dispatch(Command::Action(ActionId::NextParagraph));
    app.dispatch(Command::Action(ActionId::AddBookmark));
    app.dispatch(Command::Action(ActionId::ListBookmarks));
    let list = app.list_model().expect("the bookmark list").clone();
    assert_eq!(list.items.len(), 2);
    assert_eq!(list.selected, 0);
    // The focused item is said after the list's introduction.
    assert!(said.last().starts_with("1 of 2, "), "{}", said.last());
    list_key(&mut app, ListKey::Down);
    assert!(said.last().starts_with("2 of 2, "), "{}", said.last());
    list_key(&mut app, ListKey::Down);
    assert_eq!(said.last(), "End of list.");
    list_key(&mut app, ListKey::Home);
    assert_eq!(app.list_model().unwrap().selected, 0);
    list_key(&mut app, ListKey::Up);
    assert_eq!(said.last(), "Top of list.");
    // A letter no item starts with.
    list_key(&mut app, ListKey::Char('z'));
    assert_eq!(said.last(), "No item starts with z.");
    // A GUI list reports its own focus quietly.
    let before = said.all().len();
    app.dispatch(Command::ListFocus(1));
    assert_eq!(app.list_model().unwrap().selected, 1);
    assert_eq!(said.all().len(), before);
    // Delete keeps the list open, focused in place.
    list_key(&mut app, ListKey::Delete);
    let list = app.list_model().expect("still open");
    assert_eq!(list.items.len(), 1);
    assert_eq!(list.selected, 0);
    // Enter chooses and closes.
    list_key(&mut app, ListKey::Enter);
    assert!(app.list_model().is_none());
}

/// A GUI whose native list is announced by the screen reader turns the
/// app's own announcement of the first item off.
#[test]
fn the_first_item_can_be_left_to_the_screen_reader() {
    let (mut app, said) = app("Text.");
    app.set_announce_list_focus(false);
    app.dispatch(Command::Action(ActionId::KeyboardHelp));
    assert!(app.list_model().is_some());
    assert!(!said.last().contains(" of "), "{}", said.last());
    list_key(&mut app, ListKey::Down);
    assert!(said.last().starts_with("2 of "), "{}", said.last());
}

#[test]
fn escape_and_backspace_close_a_list() {
    let (mut app, said) = app("Text.");
    app.dispatch(Command::Action(ActionId::KeyboardHelp));
    assert!(app.list_model().is_some());
    list_key(&mut app, ListKey::Escape);
    assert!(app.list_model().is_none());
    assert_eq!(said.last(), "Cancelled.");
    app.dispatch(Command::Action(ActionId::KeyboardHelp));
    list_key(&mut app, ListKey::Backspace);
    assert!(app.list_model().is_none());
}

#[test]
fn filtering_lists_filter_as_you_type() {
    let (mut app, said) = app("");
    let text = "# Methods\n\nText.\n\n# Results\n\nMore.\n\n# Discussion\n";
    app.open_document(
        textweaver_app::formats::Registry::with_builtins()
            .load(
                &textweaver_app::formats::Source::Bytes {
                    data: text.as_bytes().to_vec(),
                    hint: "md".into(),
                },
                &Default::default(),
            )
            .unwrap(),
        DocKey::untitled(2),
        "Outline".into(),
    );
    app.dispatch(Command::Action(ActionId::Outline));
    assert_eq!(app.list_model().unwrap().items.len(), 3);
    list_key(&mut app, ListKey::Char('r'));
    list_key(&mut app, ListKey::Char('e'));
    assert_eq!(app.list_filter(), Some("re"));
    assert_eq!(
        app.list_model().unwrap().items,
        ["Results, level 1".to_owned()]
    );
    assert!(said.any("1 heading match."));
    list_key(&mut app, ListKey::Backspace);
    assert_eq!(app.list_filter(), Some("r"));
    list_key(&mut app, ListKey::Enter);
    assert!(app.list_model().is_none());
}

#[test]
fn prompts_edit_recall_and_answer_in_the_app() {
    let (mut app, said) = app("alpha beta gamma beta");
    app.dispatch(Command::Action(ActionId::Find));
    let p = app.prompt_model().expect("the find prompt");
    assert_eq!(p.purpose, PromptPurpose::Find);
    for c in "betx".chars() {
        prompt_key(&mut app, PromptKey::Char(c));
    }
    prompt_key(&mut app, PromptKey::Backspace);
    prompt_key(&mut app, PromptKey::Char('a'));
    assert_eq!(app.prompt_model().unwrap().text(), "beta");
    prompt_key(&mut app, PromptKey::Enter);
    assert!(app.prompt_model().is_none());
    assert_eq!(app.session().unwrap().cursor, CharPos(6));
    assert_eq!(app.prompt_history(PromptPurpose::Find), ["beta".to_owned()]);
    // Up recalls it next time; Escape cancels.
    app.dispatch(Command::Action(ActionId::Find));
    prompt_key(&mut app, PromptKey::Up);
    assert_eq!(app.prompt_model().unwrap().text(), "beta");
    assert_eq!(said.last(), "beta");
    prompt_key(&mut app, PromptKey::Escape);
    assert!(app.prompt_model().is_none());
    // A GUI text field sets the whole text.
    app.dispatch(Command::Action(ActionId::CommandPalette));
    prompt_key(&mut app, PromptKey::SetText("toggle_line_n".into()));
    prompt_key(&mut app, PromptKey::Tab);
    assert_eq!(app.prompt_model().unwrap().text(), "toggle_line_numbers");
    prompt_key(&mut app, PromptKey::Enter);
    assert!(app.settings().display.show_line_numbers);
}

// Command::ReplaceRange.

#[test]
fn replace_range_edits_like_a_native_control() {
    let (mut app, said) = app("");
    app.dispatch(Command::Action(ActionId::NewDocument));
    assert!(app.is_editing());
    let rev = app.session().unwrap().revision;
    // Typing at the caret, one character at a time: one undo step.
    for (i, c) in "Hello".chars().enumerate() {
        app.dispatch(Command::ReplaceRange {
            range: CharRange::empty(i),
            text: c.to_string(),
        });
    }
    assert_ne!(app.session().unwrap().revision, rev, "the text changed");
    // A correction replacing a word.
    let before = said.all().len();
    app.dispatch(Command::ReplaceRange {
        range: CharRange::new(0, 5),
        text: "Howdy there".into(),
    });
    assert_eq!(
        said.all().len(),
        before,
        "nothing is spoken: the control echoes"
    );
    let s = app.session().unwrap();
    assert_eq!(s.doc.text().to_string(), "Howdy there");
    assert_eq!(s.cursor, CharPos(11), "the caret ends after the new text");
    // A deletion.
    app.dispatch(Command::ReplaceRange {
        range: CharRange::new(5, 11),
        text: String::new(),
    });
    assert_eq!(app.session().unwrap().doc.text().to_string(), "Howdy");
    // Undo: the deletion, the correction, then the typing, in one step.
    app.dispatch(Command::Action(ActionId::Undo));
    app.dispatch(Command::Action(ActionId::Undo));
    assert_eq!(app.session().unwrap().doc.text().to_string(), "Hello");
    app.dispatch(Command::Action(ActionId::Undo));
    assert_eq!(app.session().unwrap().doc.text().to_string(), "");
    // Out of range is refused and said.
    app.dispatch(Command::ReplaceRange {
        range: CharRange::new(3, 9),
        text: "x".into(),
    });
    assert!(said.last().starts_with("Cannot change characters 3 to 9"));
}

#[test]
fn replace_range_outside_edit_mode_says_how_to_edit() {
    let (mut app, said) = app("Read only.");
    app.dispatch(Command::ReplaceRange {
        range: CharRange::new(0, 4),
        text: "Write".into(),
    });
    assert!(
        said.last().starts_with("Turn on edit mode with"),
        "{}",
        said.last()
    );
    assert_eq!(app.session().unwrap().doc.text().to_string(), "Read only.");
}

// The settings schema, the screen, and the writer.

#[test]
fn set_setting_takes_effect_and_is_saved_on_the_writer() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    let (mut app, _said) = app_with(Some(paths.clone()));
    let said = app
        .set_setting("speech.rate", serde_json::json!(300))
        .unwrap();
    assert_eq!(said, "Rate, 300 words per minute.");
    assert_eq!(app.settings().speech.rate.wpm(), 300);
    // Out of range: clamped, and said.
    let said = app
        .set_setting("speech.pitch", serde_json::json!(40))
        .unwrap();
    assert!(said.contains("nearest value"), "{said}");
    assert_eq!(app.settings().speech.pitch.semitones(), 12);
    // The theme, and its choices from this session's themes.
    let said = app
        .set_setting("display.theme", serde_json::json!("light"))
        .unwrap();
    assert!(said.starts_with("Theme, "), "{said}");
    assert!(app.settings().display.theme_explicit);
    let schema = app.settings_schema();
    let SettingKind::Choice { choices, .. } = &schema.get("display.theme").unwrap().kind else {
        panic!("the theme is a choice");
    };
    assert_eq!(choices.len(), app.theme_registry().names().len());
    // Wrong types are refused.
    assert!(
        app.set_setting("speech.rate", serde_json::json!("fast"))
            .is_err()
    );
    assert!(
        app.set_setting("speech.nothing", serde_json::json!(1))
            .is_err()
    );
    // Null puts the default back.
    app.set_setting("speech.rate", serde_json::Value::Null)
        .unwrap();
    assert_eq!(app.settings().speech.rate.wpm(), 265);
    // Keyboard: single keys off, at once.
    app.set_setting("keyboard.character_keys", serde_json::json!(false))
        .unwrap();
    assert!(!app.keymap().character_keys());
    // Saved by the writer thread.
    app.save_settings().unwrap();
    app.wait_for_writes();
    let saved = SettingsStore::new(paths).load().0;
    assert!(!saved.keyboard.character_keys);
    assert_eq!(saved.display.theme, "light");
}

#[test]
fn the_settings_screen_changes_values_with_the_arrows() {
    let (mut app, said) = app("Text.");
    app.dispatch(Command::Action(ActionId::Settings));
    assert!(said.any("Settings, "));
    let n = app.list_model().expect("the settings list").items.len();
    assert!(n > 80, "{n} settings");
    // Filter to the rate.
    for c in "rate".chars() {
        list_key(&mut app, ListKey::Char(c));
    }
    let items = app.list_model().unwrap().items.clone();
    let i = items
        .iter()
        .position(|x| x.starts_with("Rate: "))
        .expect("the rate");
    app.dispatch(Command::ListFocus(i));
    list_key(&mut app, ListKey::Right);
    assert_eq!(app.settings().speech.rate.wpm(), 285);
    assert!(said.any("Rate, 285 words per minute."), "{:?}", said.all());
    // The list stays open, on the same item, with the new value.
    let list = app.list_model().unwrap();
    assert_eq!(list.selected, i);
    assert_eq!(list.items[i], "Rate: 285 words per minute");
    // Delete puts the default back.
    list_key(&mut app, ListKey::Delete);
    assert_eq!(app.settings().speech.rate.wpm(), 265);
    // Enter asks for a number, starting from the current value.
    list_key(&mut app, ListKey::Enter);
    let p = app.prompt_model().expect("a value prompt");
    assert_eq!(p.purpose, PromptPurpose::SettingValue);
    assert_eq!(p.text(), "265");
    prompt_key(&mut app, PromptKey::SetText("410".into()));
    prompt_key(&mut app, PromptKey::Enter);
    assert_eq!(app.settings().speech.rate.wpm(), 410);
    assert!(app.list_model().is_some(), "back in the list");
    // A bad number is refused and the list comes back.
    list_key(&mut app, ListKey::Enter);
    prompt_key(&mut app, PromptKey::SetText("9000".into()));
    prompt_key(&mut app, PromptKey::Enter);
    assert!(said.any("9000 is outside 50 to 900."), "{:?}", said.all());
    assert_eq!(app.settings().speech.rate.wpm(), 410);
    // Escape in the prompt goes back to the list; again closes it.
    list_key(&mut app, ListKey::Enter);
    prompt_key(&mut app, PromptKey::Escape);
    assert!(app.list_model().is_some());
    // A toggle flips with Enter.
    for _ in 0..4 {
        list_key(&mut app, ListKey::Backspace);
    }
    for c in "line numbers".chars() {
        list_key(&mut app, ListKey::Char(c));
    }
    assert_eq!(app.list_model().unwrap().items.len(), 1);
    list_key(&mut app, ListKey::Enter);
    assert!(app.settings().display.show_line_numbers);
    list_key(&mut app, ListKey::Escape);
    assert!(app.list_model().is_none());
    assert_eq!(said.last(), "Settings closed.");
}

/// A GUI or JSON-RPC picks a setting with `Choose`: the list comes back
/// on the same item.
#[test]
fn choosing_a_setting_keeps_the_list_in_place() {
    let (mut app, _said) = app("Text.");
    app.dispatch(Command::Action(ActionId::Settings));
    let items = app.list_model().unwrap().items.clone();
    let i = items
        .iter()
        .position(|x| x.starts_with("Line numbers: "))
        .unwrap();
    app.dispatch(Command::Choose(i));
    assert!(app.settings().display.show_line_numbers);
    let list = app.list_model().expect("shown again");
    assert_eq!(list.selected, i);
    assert_eq!(list.items[i], "Line numbers: on");
    // A table says where to edit it, and the list stays.
    let t = items
        .iter()
        .position(|x| x.starts_with("Pronunciations: "))
        .unwrap();
    app.dispatch(Command::Choose(t));
    assert_eq!(app.list_model().unwrap().selected, t);
}

// The waker.

#[test]
fn the_waker_rings_for_speech_and_the_writer() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _said) = app_with(Some(Paths::under(dir.path())));
    app.open_document(
        Document::from_plain_text("One sentence. Two sentences."),
        DocKey::untitled(3),
        "Waker".into(),
    );
    let rings = Arc::new(AtomicUsize::new(0));
    let r = Arc::clone(&rings);
    app.set_waker(Some(Arc::new(move || {
        r.fetch_add(1, Ordering::SeqCst);
    })));
    let wait_ring = |from: usize| {
        let deadline = Instant::now() + Duration::from_secs(10);
        while rings.load(Ordering::SeqCst) <= from {
            assert!(Instant::now() < deadline, "the waker never rang");
            std::thread::sleep(Duration::from_millis(2));
        }
    };
    // Speech.
    let before = rings.load(Ordering::SeqCst);
    app.dispatch(Command::Action(ActionId::ReadFromCursor));
    wait_ring(before);
    // The writer (a settings save).
    let before = rings.load(Ordering::SeqCst);
    app.set_setting("speech.rate", serde_json::json!(300))
        .unwrap();
    app.save_settings().unwrap();
    wait_ring(before);
    // Nothing to do: the idle tick is long.
    app.dispatch(Command::Action(ActionId::Stop));
    assert!(app.tick_interval(Instant::now()) >= Duration::from_millis(50));
}

#[test]
fn channel_wakers_collapse_into_a_channel() {
    let (waker, rx) = textweaver_app::channel_waker();
    waker();
    waker();
    assert_eq!(rx.try_iter().count(), 2);
}

// Opening in the background.

#[test]
fn large_files_open_in_the_background_and_escape_cancels() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("big.txt");
    std::fs::write(&file, "A sentence here.\n\n".repeat(20)).unwrap();
    let (mut app, said) = app_with(None);
    let rings = Arc::new(AtomicUsize::new(0));
    let r = Arc::clone(&rings);
    app.set_waker(Some(Arc::new(move || {
        r.fetch_add(1, Ordering::SeqCst);
    })));
    app.set_background_open_threshold(0);
    app.dispatch(Command::Open(file.clone()));
    assert!(said.any("Opening big.txt. Escape cancels."));
    assert!(app.opening());
    assert!(app.wait_for_open(Duration::from_secs(30)));
    assert!(rings.load(Ordering::SeqCst) > 0, "the waker rang");
    assert_eq!(app.session().unwrap().title, "big.txt");
    assert!(said.any("Opened big.txt."));
    // Escape before it is loaded: nothing opens.
    let other = dir.path().join("other.txt");
    std::fs::write(&other, "Other text.").unwrap();
    app.dispatch(Command::Open(other));
    app.dispatch(Command::Cancel);
    assert!(!app.opening());
    assert_eq!(said.last(), "Stopped opening other.txt.");
    std::thread::sleep(Duration::from_millis(100));
    app.tick(Instant::now());
    assert_eq!(app.session().unwrap().title, "big.txt");
    // Small files still open at once.
    app.set_background_open_threshold(u64::MAX);
    let small = dir.path().join("small.txt");
    std::fs::write(&small, "Small.").unwrap();
    app.dispatch(Command::Open(small));
    assert!(!app.opening());
    assert_eq!(app.session().unwrap().title, "small.txt");
    // A file that cannot be read says so.
    app.set_background_open_threshold(0);
    let missing = dir.path().join("missing.xyz");
    std::fs::write(&missing, [0u8, 159, 146, 150, 0, 0]).unwrap();
    app.dispatch(Command::Open(missing));
    assert!(app.wait_for_open(Duration::from_secs(30)));
    assert!(said.last().starts_with("Could not open"), "{}", said.last());
}

// Speech starting in the background.

#[test]
fn speech_starts_in_the_background_and_says_what_was_said_meanwhile() {
    let (speech_log_tx, speech_log_rx) = std::sync::mpsc::channel();
    let said = Said::default();
    let mut app = App::new(AppConfig {
        speech: textweaver_app::speech::SpeechService::null(),
        announcer: Box::new(said.clone()),
        self_voicing: true,
        backend_name: "starting".into(),
        ..AppConfig::for_tests()
    });
    let tx = Mutex::new(speech_log_tx);
    app.start_speech_in_background(Arc::new(move |_settings| {
        // An engine that takes a moment to start.
        std::thread::sleep(Duration::from_millis(50));
        let (service, log) = recording_service().unwrap();
        let _ = tx.lock().map(|t| t.send(log));
        (service, "test-recording".into(), Vec::new())
    }));
    assert!(app.speech_restarting());
    app.open_document(
        Document::from_plain_text("Hello there."),
        DocKey::untitled(4),
        "Starting".into(),
    );
    assert!(app.wait_for_speech_start(Duration::from_secs(30)));
    assert_eq!(app.backend_name(), "test-recording");
    let log = speech_log_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !log.texts().iter().any(|t| t.contains("Opened Starting.")) {
        assert!(Instant::now() < deadline, "{:?}", log.texts());
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        !said.any("Speech restarted"),
        "a first start is not a restart"
    );
}

// The document window over a session.

#[test]
fn the_window_follows_reading_and_reloads_after_edits() {
    let text: Vec<String> = (0..3000)
        .map(|i| format!("Paragraph {i} has a few words in it."))
        .collect();
    let (mut app, _said) = app(&text.join("\n\n"));
    let s = app.session().unwrap();
    let mut w = DocWindow::with_budget(&s.doc, s.cursor, 20_000).for_revision(s.revision);
    assert_eq!(w.to_ctrl(&s.doc, s.cursor, Units::Utf16), Some(0));
    // Highlights in window offsets: select the first word.
    app.dispatch(Command::Select(CharRange::new(0, 9)));
    let marks = app.window_highlights(&w, Units::Utf8);
    assert!(
        marks.contains(&(0..9, textweaver_app::HighlightKind::Selection)),
        "{marks:?}"
    );
    // A jump far away recentres.
    app.dispatch(Command::Action(ActionId::DocumentEnd));
    let s = app.session().unwrap();
    assert_eq!(w.follow_session(s, s.cursor), WindowChange::Recentred);
    assert!(w.contains(&s.doc, s.cursor));
    // Editing changes the revision: the window reloads.
    app.dispatch(Command::Action(ActionId::ToggleEditMode));
    let s = app.session().unwrap();
    assert_eq!(w.follow_session(s, s.cursor), WindowChange::Recentred);
    assert_eq!(w.revision(), s.revision);
    assert_eq!(w.follow_session(s, s.cursor), WindowChange::Unchanged);
}

/// Measures the window on a ten-million-character document: making one,
/// following the focus, and mapping positions. Run with `--nocapture` to
/// see the times (debug builds; release is several times faster).
#[test]
fn window_timings_on_ten_million_characters() {
    let para = "A sentence of about sixty characters, for the window test. ".repeat(8);
    let n = 10_000_000 / para.len() + 1;
    let text = vec![para.trim_end(); n].join("\n\n");
    let doc = Document::from_plain_text(&text);
    assert!(doc.len_chars() >= 10_000_000);
    let t = Instant::now();
    let _ = doc.display().len_utf16();
    let index = t.elapsed();
    let t = Instant::now();
    let mut w = DocWindow::around(&doc, CharPos(5_000_000));
    let make = t.elapsed();
    let t = Instant::now();
    let body = w.text(&doc);
    let slice = t.elapsed();
    assert!(body.len() >= 400_000, "{}", body.len());
    // Follow the focus word by word through 1,000 words.
    let mut pos = w.range().end.0 - 70_000;
    let t = Instant::now();
    let mut slides = 0;
    for _ in 0..1000 {
        pos += 64;
        if w.follow(&doc, CharPos(pos)) != WindowChange::Unchanged {
            slides += 1;
        }
    }
    let follow = t.elapsed() / 1000;
    assert!(slides >= 1, "reading on slid the window");
    let t = Instant::now();
    for i in 0..1000 {
        let p = CharPos(w.range().start.0 + i * 97);
        let c = w.to_ctrl(&doc, p, Units::Utf16).unwrap();
        assert_eq!(w.to_doc(&doc, c, Units::Utf16), p);
    }
    let map = t.elapsed() / 1000;
    println!(
        "10M chars: index {index:?}, window {make:?}, slice {slice:?}, follow {follow:?} per word, map {map:?} per round trip"
    );
    // Generous bounds for a loaded machine and a debug build.
    assert!(make < Duration::from_millis(500), "{make:?}");
    assert!(follow < Duration::from_millis(30), "{follow:?}");
}

/// Measurements for the report (Wave 3, W3a): what the input thread waits
/// for now, against the work moved off it. Run with
/// `cargo test -p textweaver-app --test it -- app_core:: --ignored --nocapture`.
#[test]
#[ignore = "a measurement: run it with --ignored --nocapture"]
fn measure_work_off_the_input_thread() {
    let dir = tempfile::tempdir().unwrap();
    let para = "Some **bold** text with a few wrods to chek, and more prose after them. ";
    let mut text = String::new();
    let mut n = 0;
    while text.len() < 10_000_000 {
        n += 1;
        text.push_str(&format!("## Section {n}\n\n{}\n\n", para.repeat(6)));
    }
    let file = dir.path().join("big.md");
    std::fs::write(&file, &text).unwrap();
    let doc = Document::from_plain_text(&text);

    // The UTF-16 index: the old Vec of every char's offset, and the rope.
    let t = Instant::now();
    let mut old: Vec<u32> = Vec::with_capacity(doc.len_chars() + 1);
    let mut at = 0u32;
    for c in doc.text().chars() {
        old.push(at);
        at += c.len_utf16() as u32;
    }
    old.push(at);
    let old_index = t.elapsed();
    let t = Instant::now();
    let new_len = doc.display().len_utf16();
    let new_index = t.elapsed();
    assert_eq!(new_len as u32, at);
    println!(
        "UTF-16 index on {} chars: old Vec {old_index:?} and {} MB, now {new_index:?} and no copy",
        doc.len_chars(),
        old.len() * 4 / 1_000_000
    );
    drop(old);

    // Opening 10 MB of Markdown: at once, and in the background.
    let (mut app, _said) = app_with(None);
    app.set_background_open_threshold(u64::MAX);
    let t = Instant::now();
    app.dispatch(Command::Open(file.clone()));
    let at_once = t.elapsed();
    let (mut app, _said) = app_with(None);
    app.set_background_open_threshold(0);
    let t = Instant::now();
    app.dispatch(Command::Open(file.clone()));
    let returned = t.elapsed();
    assert!(app.wait_for_open(Duration::from_secs(120)));
    let loaded = t.elapsed();
    println!(
        "open 10 MB: at once {at_once:?}; in the background the key returns in {returned:?}, the document is ready after {loaded:?}"
    );

    // Saving in edit mode: the misspelling count now follows on a thread.
    let dir2 = tempfile::tempdir().unwrap();
    let (mut app, said) = app_with(Some(Paths::under(dir2.path())));
    app.open(&file).unwrap();
    app.dispatch(Command::Action(ActionId::ToggleEditMode));
    app.dispatch(Command::Insert("x".into()));
    let t = Instant::now();
    app.dispatch(Command::Action(ActionId::Save));
    let save_key = t.elapsed();
    let t = Instant::now();
    app.wait_for_writes();
    let written = t.elapsed();
    let t = Instant::now();
    assert!(app.wait_for_spell_count(Duration::from_secs(120)));
    let counted = t.elapsed();
    assert!(said.any("possible misspellings"), "{:?}", said.all());
    println!(
        "save 10 MB: the key returns in {save_key:?}; written and reported in {written:?}; the misspelling count arrives {counted:?} later, off the input thread"
    );

    // Settings: the key no longer waits for settings.toml.
    let t = Instant::now();
    app.dispatch(Command::Action(ActionId::RateUp));
    let settings_key = t.elapsed();
    let t = Instant::now();
    SettingsStore::new(Paths::under(dir2.path()))
        .save(app.settings())
        .unwrap();
    let direct = t.elapsed();
    println!(
        "settings: a key that changes one returns in {settings_key:?}; writing settings.toml takes {direct:?} on the writer"
    );
}
