//! Phase 2 stability (Agent P2a): find over large documents, the background
//! writer, positions that survive outside edits, and restarting speech.

use std::sync::{Arc, Mutex};

use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::core::CharPos;
use textweaver_app::keymap::ActionId;
use textweaver_app::store::DocKey;
use textweaver_app::testing::recording_service;
use textweaver_app::text::Document;
use textweaver_app::{App, AppConfig, Command};

/// Collects announcements for assertions.
#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Said {
    fn all(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
    fn last(&self) -> String {
        self.all().last().cloned().unwrap_or_default()
    }
}

impl Announcer for Said {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}

fn app_with(text: &str) -> (App, Said) {
    let said = Said::default();
    let (speech, _log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        announcer: Box::new(said.clone()),
        ..AppConfig::for_tests()
    });
    app.open_document(
        Document::from_plain_text(text),
        DocKey::untitled(1),
        "Test".into(),
    );
    (app, said)
}

#[test]
fn find_keeps_a_window_of_matches_and_counts_them_all() {
    let n = 25_000;
    let text = "word x ".repeat(n);
    let (mut app, said) = app_with(&text);
    app.dispatch(Command::Find("x".into()));
    let f = app.session().unwrap().find.clone().unwrap();
    assert_eq!(f.total, n);
    assert!(f.hits.len() <= 10_000, "{} kept", f.hits.len());
    assert!(said.last().contains("Match 1 of 25000"), "{}", said.last());
    // Step past the matches kept: the search runs again there and the
    // numbering goes on.
    app.dispatch(Command::GoTo(textweaver_app::text::GoTo::Percent(90)));
    app.dispatch(Command::Action(ActionId::FindNext));
    let at = app.session().unwrap().cursor;
    let number = at.0 / 7 + 1;
    assert!(
        said.last().contains(&format!("Match {number} of 25000")),
        "{} at {at:?}",
        said.last()
    );
    let f = app.session().unwrap().find.clone().unwrap();
    let i = f.current.unwrap();
    assert_eq!(f.first_index + i + 1, number);
    assert_eq!(f.hits[i].start, at);
    // Backward past the start of the window, then wrapping at the top.
    app.dispatch(Command::GoTo(textweaver_app::text::GoTo::Start));
    app.dispatch(Command::Action(ActionId::FindPrevious));
    assert!(
        said.last().contains("Match 25000 of 25000"),
        "{}",
        said.last()
    );
    assert_eq!(app.session().unwrap().cursor, CharPos(7 * (n - 1) + 5));
    app.dispatch(Command::Action(ActionId::FindNext));
    assert!(said.last().contains("Match 1 of 25000"), "{}", said.last());
}

/// An app that saves under a temporary home, with its announcements.
fn persistent_app(home: &std::path::Path) -> (App, Said) {
    let said = Said::default();
    let (speech, _log) = recording_service().unwrap();
    let app = App::new(AppConfig {
        speech,
        paths: Some(textweaver_app::store::Paths::under(home)),
        announcer: Box::new(said.clone()),
        ..AppConfig::for_tests()
    });
    (app, said)
}

#[test]
fn a_save_is_written_by_the_writer_and_typing_goes_on_meanwhile() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("big.md");
    let body = "Some words to read aloud. ".repeat(40_000);
    std::fs::write(&file, &body).unwrap();
    let (mut app, said) = persistent_app(&dir.path().join("home"));
    app.open(&file).unwrap();
    app.dispatch(Command::Action(ActionId::ToggleEditMode));
    app.dispatch(Command::Insert("X".into()));
    app.dispatch(Command::Action(ActionId::Save));
    // Nothing is announced or marked saved until the writer reports.
    assert!(!said.all().iter().any(|s| s.starts_with("Saved")));
    assert!(app.is_dirty());
    // Typing goes on while the file is written.
    app.dispatch(Command::Insert("Y".into()));
    app.wait_for_writes();
    assert_eq!(said.last(), "Saved big.md. Still editing.");
    let saved = std::fs::read_to_string(&file).unwrap();
    assert_eq!(saved.len(), body.len() + 1);
    assert!(saved.starts_with('X'), "the text as it was at Save");
    assert!(app.is_dirty(), "Y was typed after the save began");
    // Two saves in a row, the second queued before the first is heard of:
    // the first one's change on disk is not taken for another program's.
    app.dispatch(Command::Action(ActionId::Save));
    app.dispatch(Command::Insert("Z".into()));
    app.dispatch(Command::Action(ActionId::Save));
    app.wait_for_writes();
    assert!(!app.confirmation_pending(), "{:?}", said.all());
    assert_eq!(said.last(), "Saved big.md. Still editing.");
    assert!(std::fs::read_to_string(&file).unwrap().starts_with("XYZ"));
    assert!(!app.is_dirty());
}

#[test]
fn quitting_writes_the_position_and_bookmarks_before_it_returns() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "Alpha beta gamma. Delta epsilon zeta.\n").unwrap();
    let home = dir.path().join("home");
    let (mut app, _said) = persistent_app(&home);
    app.open(&file).unwrap();
    app.dispatch(Command::GoTo(textweaver_app::text::GoTo::Char(CharPos(18))));
    app.dispatch(Command::Action(ActionId::AddBookmark));
    app.dispatch(Command::GoTo(textweaver_app::text::GoTo::Char(CharPos(6))));
    // No waiting here: shutdown waits for the writer itself.
    app.shutdown();
    let state = textweaver_app::store::StateStore::new(
        textweaver_app::store::Paths::under(&home).state_dir(),
    )
    .load(&DocKey::for_path(&file))
    .unwrap();
    assert_eq!(state.position, CharPos(6));
    assert_eq!(state.bookmarks.len(), 1);
    assert_eq!(state.bookmarks[0].pos, CharPos(18));
    assert!(state.text.is_some(), "the text stamp is saved too");
}

const P1: &str = "Alpha paragraph talks about apples and orchards.\n\n";
const P2: &str = "Beta paragraph describes bridges over rivers.\n\n";
const P3: &str = "Gamma paragraph covers gardens in spring.\n\n";
const P4: &str = "Delta paragraph ends with deserts at dusk.\n";

/// The char offset of `needle` in the open document.
fn offset_of(app: &App, needle: &str) -> CharPos {
    let text = app.session().unwrap().doc.text().to_string();
    let byte = text
        .find(needle)
        .unwrap_or_else(|| panic!("{needle:?} in {text:?}"));
    CharPos(text[..byte].chars().count())
}

/// The document text from `pos`, `n` chars.
fn text_at(app: &App, pos: CharPos, n: usize) -> String {
    let rope = app.session().unwrap().doc.text();
    rope.slice(pos.0..(pos.0 + n).min(rope.len_chars()))
        .to_string()
}

/// Reads a file, places a reading position, two bookmarks, a note, and a
/// highlight, and quits; then another program rewrites the file with
/// `change`, and textweaver opens it again.
fn marks_then_outside_edit(change: impl FnOnce(&str) -> String) -> (App, Said) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("essay.txt");
    let original = format!("{P1}{P2}{P3}{P4}");
    std::fs::write(&file, &original).unwrap();
    let home = dir.path().join("home");
    let (mut app, _said) = persistent_app(&home);
    app.open(&file).unwrap();
    let go = |app: &mut App, needle: &str| {
        let pos = offset_of(app, needle);
        app.dispatch(Command::GoTo(textweaver_app::text::GoTo::Char(pos)));
    };
    go(&mut app, "bridges");
    app.dispatch(Command::Action(ActionId::AddBookmark));
    go(&mut app, "gardens");
    app.dispatch(Command::Action(ActionId::AddBookmark));
    go(&mut app, "deserts");
    app.dispatch(Command::Notes(textweaver_app::NoteCommand::Add));
    app.dispatch(Command::Answer("Check the dusk scene".into()));
    app.dispatch(Command::Notes(textweaver_app::NoteCommand::ToggleHighlight));
    go(&mut app, "covers");
    app.shutdown();
    drop(app);
    // Another program (an editor, git) rewrites the file.
    std::fs::write(&file, change(&original)).unwrap();
    let (mut app, said) = persistent_app(&home);
    app.open(&file).unwrap();
    // Keep the directory alive with the app.
    std::mem::forget(dir);
    (app, said)
}

fn bookmark_text(app: &App, i: usize) -> String {
    let b = &app.session().unwrap().bookmarks[i];
    text_at(app, b.pos, 7)
}

#[test]
fn positions_follow_text_inserted_before_them() {
    let (app, said) =
        marks_then_outside_edit(|t| format!("A preface another program added.\n\n{t}"));
    let s = app.session().unwrap();
    assert_eq!(text_at(&app, s.cursor, 6), "covers");
    assert_eq!(bookmark_text(&app, 0), "bridges");
    assert_eq!(bookmark_text(&app, 1), "gardens");
    assert!(s.bookmarks.iter().all(|b| !b.not_found));
    let note = &s.notes[0];
    assert_eq!(
        s.doc.slice(note.range),
        "Delta paragraph ends with deserts at dusk."
    );
    assert_eq!(s.doc.slice(s.highlights[0].range), s.doc.slice(note.range));
    assert!(
        said.all().iter().any(|m| m.contains(
            "The file changed; your reading position, 2 bookmarks, 1 note, and 1 highlight were moved to match."
        )),
        "{:?}",
        said.all()
    );
}

#[test]
fn positions_follow_paragraphs_that_were_reordered() {
    let (app, said) = marks_then_outside_edit(|_| format!("{P3}{P1}{P4}\n\n{P2}"));
    let s = app.session().unwrap();
    assert_eq!(text_at(&app, s.cursor, 6), "covers");
    // Bookmarks are kept in document order: gardens now comes first.
    assert_eq!(bookmark_text(&app, 0), "gardens");
    assert_eq!(bookmark_text(&app, 1), "bridges");
    assert_eq!(
        s.doc.slice(s.notes[0].range),
        "Delta paragraph ends with deserts at dusk."
    );
    assert!(
        said.all().iter().any(|m| m.contains("were moved to match")),
        "{:?}",
        said.all()
    );
}

#[test]
fn text_deleted_elsewhere_is_marked_not_found() {
    // The third paragraph is deleted (the reading position and the second
    // bookmark were in it), and a word inside the note's sentence changes.
    let (mut app, said) = marks_then_outside_edit(|t| {
        t.replace(P3, "")
            .replace("ends with deserts", "ends among deserts")
    });
    let s = app.session().unwrap();
    assert_eq!(bookmark_text(&app, 0), "bridges");
    assert!(!s.bookmarks[0].not_found);
    assert!(s.bookmarks[1].not_found, "{:?}", s.bookmarks[1]);
    // The note's sentence was found by similarity.
    assert!(
        s.doc
            .slice(s.notes[0].range)
            .starts_with("Delta paragraph ends among deserts"),
        "{:?}",
        s.doc.slice(s.notes[0].range)
    );
    let msg = said
        .all()
        .into_iter()
        .find(|m| m.contains("The file changed"))
        .unwrap();
    assert!(
        msg.contains("your reading position and 1 bookmark could not be found and are marked"),
        "{msg}"
    );
    // The bookmark list says which one was not found.
    let effects = app.dispatch(Command::Action(ActionId::ListBookmarks));
    let items = effects
        .iter()
        .find_map(|e| match e {
            textweaver_app::Effect::ShowList { items, .. } => Some(items.clone()),
            _ => None,
        })
        .unwrap();
    assert!(
        items[1].contains("(not found after the file changed)"),
        "{items:?}"
    );
    assert!(!items[0].contains("not found"), "{items:?}");
}

#[test]
fn an_unchanged_file_says_nothing_and_moves_nothing() {
    let (app, said) = marks_then_outside_edit(str::to_owned);
    assert!(
        !said.all().iter().any(|m| m.contains("The file changed")),
        "{:?}",
        said.all()
    );
    assert_eq!(bookmark_text(&app, 0), "bridges");
}

/// An engine with a bug: speaking "boom" panics on the speech thread.
struct Boom;

impl textweaver_app::speech::SpeechBackend for Boom {
    fn id(&self) -> textweaver_app::speech::BackendId {
        "boom"
    }
    fn capabilities(&self) -> textweaver_app::speech::Caps {
        textweaver_app::speech::Caps::empty()
    }
    fn voices(
        &self,
    ) -> Result<Vec<textweaver_app::speech::Voice>, textweaver_app::speech::SpeechError> {
        Ok(Vec::new())
    }
    fn set_params(
        &mut self,
        _: &textweaver_app::speech::VoiceParams,
    ) -> Result<(), textweaver_app::speech::SpeechError> {
        Ok(())
    }
    fn effective_wpm(&self) -> u16 {
        200
    }
    fn speak(
        &mut self,
        u: &textweaver_app::core::Utterance,
        sink: &mut dyn textweaver_app::speech::EventSink,
    ) -> Result<(), textweaver_app::speech::SpeechError> {
        assert!(!u.text.contains("boom"), "the engine blew up");
        sink.emit(u.id, textweaver_app::speech::RawEvent::Started);
        sink.emit(u.id, textweaver_app::speech::RawEvent::Finished);
        Ok(())
    }
    fn stop(&mut self) {}
}

fn boom_service() -> textweaver_app::speech::SpeechService {
    textweaver_app::speech::SpeechService::spawn(
        Box::new(|| Ok(Box::new(Boom) as _)),
        textweaver_app::speech::ServiceConfig::default(),
    )
    .unwrap()
}

/// Polls and ticks until `said` has a line containing `needle`.
fn until_said(app: &mut App, said: &Said, needle: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while !said.all().iter().any(|m| m.contains(needle)) {
        assert!(
            std::time::Instant::now() < deadline,
            "never said {needle:?}: {:?}",
            said.all()
        );
        app.poll_speech();
        app.tick(std::time::Instant::now());
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[test]
fn speech_restarts_in_place_once_by_itself_then_on_request() {
    let starts = Arc::new(Mutex::new(0u32));
    let counted = Arc::clone(&starts);
    let said = Said::default();
    let mut app = App::new(AppConfig {
        speech: boom_service(),
        announcer: Box::new(said.clone()),
        self_voicing: true,
        backend_name: "boom".into(),
        ..AppConfig::for_tests()
    });
    app.set_speech_starter(Arc::new(move |_settings| {
        *counted.lock().unwrap() += 1;
        (boom_service(), "boom".to_owned(), Vec::new())
    }));
    // The engine dies: textweaver restarts speech by itself, once.
    app.announce("boom", Priority::Polite);
    until_said(&mut app, &said, "Speech stopped working");
    assert!(
        said.all().iter().any(|m| m.contains("Restarting speech.")),
        "{:?}",
        said.all()
    );
    until_said(&mut app, &said, "Speech restarted.");
    assert_eq!(*starts.lock().unwrap(), 1);
    assert_eq!(app.backend_name(), "boom");
    // Speech works again (a word that does not break it is spoken).
    app.announce("fine", Priority::Polite);
    // It dies again: no second automatic restart; the message says how.
    said.0.lock().unwrap().clear();
    app.announce("boom again", Priority::Polite);
    until_said(&mut app, &said, "Speech stopped working");
    let msg = said.last();
    assert!(msg.contains("Restart speech with Shift+F8."), "{msg}");
    std::thread::sleep(std::time::Duration::from_millis(200));
    app.tick(std::time::Instant::now());
    assert_eq!(*starts.lock().unwrap(), 1);
    // Restart Speech (Shift+F8) restarts it.
    app.dispatch(Command::Action(ActionId::RestartSpeech));
    until_said(&mut app, &said, "Speech restarted.");
    assert_eq!(*starts.lock().unwrap(), 2);
}

#[test]
fn without_a_starter_a_dead_speech_thread_leaves_textweaver_silent() {
    let said = Said::default();
    let mut app = App::new(AppConfig {
        speech: boom_service(),
        announcer: Box::new(said.clone()),
        self_voicing: true,
        ..AppConfig::for_tests()
    });
    app.announce("boom", Priority::Polite);
    until_said(&mut app, &said, "Speech stopped working");
    assert!(
        said.last().contains("textweaver is silent now"),
        "{}",
        said.last()
    );
    app.dispatch(Command::Action(ActionId::RestartSpeech));
    assert_eq!(said.last(), "Speech cannot be restarted here.");
}

/// A null engine whose voices are listed in the background, as SAPI's are.
struct SlowVoices(textweaver_app::speech::VoiceCache);

impl textweaver_app::speech::SpeechBackend for SlowVoices {
    fn id(&self) -> textweaver_app::speech::BackendId {
        "slow-voices"
    }
    fn capabilities(&self) -> textweaver_app::speech::Caps {
        textweaver_app::speech::Caps::empty()
    }
    fn voices(
        &self,
    ) -> Result<Vec<textweaver_app::speech::Voice>, textweaver_app::speech::SpeechError> {
        panic!("never asked on the speech thread");
    }
    fn set_params(
        &mut self,
        _: &textweaver_app::speech::VoiceParams,
    ) -> Result<(), textweaver_app::speech::SpeechError> {
        Ok(())
    }
    fn effective_wpm(&self) -> u16 {
        200
    }
    fn speak(
        &mut self,
        u: &textweaver_app::core::Utterance,
        sink: &mut dyn textweaver_app::speech::EventSink,
    ) -> Result<(), textweaver_app::speech::SpeechError> {
        sink.emit(u.id, textweaver_app::speech::RawEvent::Started);
        sink.emit(u.id, textweaver_app::speech::RawEvent::Finished);
        Ok(())
    }
    fn stop(&mut self) {}
    fn voice_cache(&self) -> Option<textweaver_app::speech::VoiceCache> {
        Some(self.0.clone())
    }
}

#[test]
fn edit_mode_deletes_graphemes_and_keeps_the_undo_steps_set() {
    let mut settings = textweaver_app::store::Settings::default();
    settings.editing.undo_steps = 3;
    let said = Said::default();
    let mut app = App::new(AppConfig {
        settings,
        announcer: Box::new(said.clone()),
        ..AppConfig::for_tests()
    });
    let flag = "\u{1F1E8}\u{1F1E6}";
    app.open_document(
        Document::from_plain_text(&format!("a{flag}b")),
        DocKey::untitled(1),
        "Test".into(),
    );
    app.dispatch(Command::Action(ActionId::ToggleEditMode));
    app.dispatch(Command::SetCursor(CharPos(3)));
    // Backspace removes the whole flag (two code points), and so does
    // Delete from before it.
    app.dispatch(Command::DeleteBack);
    assert_eq!(app.session().unwrap().doc.text().to_string(), "ab");
    app.dispatch(Command::Action(ActionId::Undo));
    app.dispatch(Command::SetCursor(CharPos(1)));
    app.dispatch(Command::DeleteForward);
    assert_eq!(app.session().unwrap().doc.text().to_string(), "ab");
    // Only three undo steps are kept.
    app.dispatch(Command::SetCursor(CharPos(2)));
    for w in [" one", " two", " three", " four", " five"] {
        app.dispatch(Command::Insert(w.into()));
        app.dispatch(Command::SetCursor(CharPos(
            app.session().unwrap().doc.len_chars(),
        )));
    }
    for _ in 0..10 {
        app.dispatch(Command::Action(ActionId::Undo));
    }
    let text = app.session().unwrap().doc.text().to_string();
    assert!(text.starts_with("ab one two"), "{text}");
}

#[test]
fn the_library_is_scanned_off_the_input_thread() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("Readings");
    for sub in 0..10 {
        let d = folder.join(format!("part{sub}"));
        std::fs::create_dir_all(&d).unwrap();
        for i in 0..300 {
            std::fs::write(d.join(format!("doc{i}.txt")), "Text.").unwrap();
        }
    }
    let mut settings = textweaver_app::store::Settings::default();
    settings.library.add_folder(&folder);
    let said = Said::default();
    let mut app = App::new(AppConfig {
        settings,
        paths: Some(textweaver_app::store::Paths::under(
            &dir.path().join("home"),
        )),
        announcer: Box::new(said.clone()),
        ..AppConfig::for_tests()
    });
    let effects = app.dispatch(Command::Action(ActionId::OpenLibrary));
    // No list yet: the scan runs on its own thread.
    assert!(
        !effects
            .iter()
            .any(|e| matches!(e, textweaver_app::Effect::ShowList { .. })),
        "{effects:?}"
    );
    assert!(app.library_scanning());
    assert_eq!(said.last(), "Scanning the library.");
    // Asking again while it scans says how far it got.
    app.dispatch(Command::Action(ActionId::OpenLibrary));
    assert!(
        said.last().starts_with("Still scanning the library: "),
        "{}",
        said.last()
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let items = loop {
        assert!(std::time::Instant::now() < deadline, "the scan never ended");
        let effects = app.tick(std::time::Instant::now());
        if let Some(items) = effects.iter().find_map(|e| match e {
            textweaver_app::Effect::ShowList { items, .. } => Some(items.clone()),
            _ => None,
        }) {
            break items;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    };
    assert_eq!(items.len(), 3000);
    assert!(
        said.all()
            .iter()
            .any(|s| s.starts_with("Library, 3000 documents.")),
        "{:?}",
        said.all()
    );
    assert!(!app.library_scanning());
}

#[test]
fn choose_voice_never_waits_and_opens_when_the_voices_arrive() {
    use textweaver_app::speech::{ServiceConfig, SpeechService, Voice, VoiceCache};
    let cache = VoiceCache::loading();
    let engine = cache.clone();
    let speech = SpeechService::spawn(
        Box::new(move || Ok(Box::new(SlowVoices(engine)) as _)),
        ServiceConfig::default(),
    )
    .unwrap();
    let said = Said::default();
    let mut app = App::new(AppConfig {
        speech,
        announcer: Box::new(said.clone()),
        ..AppConfig::for_tests()
    });
    let t = std::time::Instant::now();
    app.dispatch(Command::Action(ActionId::ChooseVoice));
    assert!(t.elapsed() < std::time::Duration::from_millis(200));
    assert_eq!(
        said.last(),
        "The voices are still loading. The list opens when they are ready."
    );
    assert!(app.tick(std::time::Instant::now()).is_empty());
    cache.set(Ok(vec![Voice {
        id: "v1".into(),
        name: "Vera".into(),
        ..Voice::default()
    }]));
    let effects = app.tick(std::time::Instant::now());
    let Some(textweaver_app::Effect::ShowList { items, .. }) = effects.first() else {
        panic!("{effects:?}");
    };
    assert!(items[2].starts_with("Vera"), "{items:?}");
    assert!(
        said.all()
            .iter()
            .any(|s| s.starts_with("Voice manager. 1 voice")),
        "{:?}",
        said.all()
    );
}
