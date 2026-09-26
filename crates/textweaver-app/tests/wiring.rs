//! Wave 2 wiring (Agent D3): reading generations and capability changes,
//! the new actions (read paragraph, select by word and line, single-key
//! switch), the library list, and reading positions synced through library
//! folders' sidecars.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::core::{CharPos, CharRange, Utterance};
use textweaver_app::keymap::{ActionId, KeyChord, Layer};
use textweaver_app::speech::{
    Caps, EventSink, RawEvent, ServiceConfig, SpeechBackend, SpeechError, SpeechService, Voice,
    VoiceParams,
};
use textweaver_app::store::sync::{ConflictPolicy, ProgressEntry, sidecar_file};
use textweaver_app::store::{DocKey, Paths, Settings, SettingsStore, StateStore};
use textweaver_app::testing::{SpeechLog, recording_service};
use textweaver_app::text::{Document, GoTo};
use textweaver_app::{App, AppConfig, Command, Confirm, Effect, Playback};

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

struct Rig {
    app: App,
    log: SpeechLog,
    said: Said,
}

fn launch(settings: Settings, paths: Option<Paths>) -> Rig {
    let said = Said::default();
    let (speech, log) = recording_service().unwrap();
    let app = App::new(AppConfig {
        settings,
        speech,
        paths,
        announcer: Box::new(said.clone()),
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    Rig { app, log, said }
}

fn rig(text: &str) -> Rig {
    let mut r = launch(Settings::default(), None);
    r.app.open_document(
        Document::from_plain_text(text),
        DocKey::untitled(1),
        "Test".into(),
    );
    r
}

impl Rig {
    fn act(&mut self, a: ActionId) -> Vec<Effect> {
        self.app.dispatch(Command::Action(a))
    }
    fn go(&mut self, pos: CharPos) {
        self.app.dispatch(Command::GoTo(GoTo::Char(pos)));
    }
    fn cursor(&self) -> CharPos {
        self.app.session().unwrap().cursor
    }
    fn wait_idle(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            self.app.poll_speech();
            if self.app.playback() == Playback::Idle {
                return;
            }
            assert!(Instant::now() < deadline, "speech never finished");
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    fn quit(&mut self) {
        self.act(ActionId::Quit);
        assert_eq!(
            self.app.dispatch(Command::Confirm(Confirm::Yes)),
            vec![Effect::Quit]
        );
    }
}

fn at(text: &str, needle: &str) -> CharPos {
    CharPos(text.find(needle).expect("needle in text"))
}

const PROSE: &str = "Alpha beta gamma delta epsilon zeta. Eta theta iota.\n\nKappa lambda mu. Nu xi.\n\nOmicron pi.";

// ---- Reading generations and capabilities ----

#[test]
fn a_repeated_reading_is_followed_once() {
    let mut r = rig(PROSE);
    // Two readings of the same sentence, back to back: the first one's
    // words (same text, same place) belong to an older generation and must
    // not move the highlight; its "finished" must not end the second.
    r.act(ActionId::ReadCurrentSentence);
    r.act(ActionId::ReadCurrentSentence);
    assert_eq!(r.app.playback(), Playback::Reading);
    r.wait_idle();
    let words = r.app.spoken_log().len();
    assert_eq!(words, 6, "{:?}", r.app.spoken_log());
    let sentence = CharRange::new(CharPos(0), at(PROSE, " Eta"));
    assert!(
        r.app
            .spoken_log()
            .iter()
            .all(|h| sentence.contains_range(*h))
    );
}

#[test]
fn pausing_drops_the_paused_readings_late_words() {
    let mut r = rig(PROSE);
    r.act(ActionId::ReadFromCursor);
    r.act(ActionId::PlayPause);
    let Playback::Paused { resume_at } = r.app.playback() else {
        panic!("not paused: {:?}", r.app.playback());
    };
    let before = r.app.spoken_log().len();
    // Whatever the paused reading still reports is ignored.
    std::thread::sleep(Duration::from_millis(50));
    r.app.poll_speech();
    assert_eq!(r.app.spoken_log().len(), before);
    assert!(matches!(r.app.playback(), Playback::Paused { .. }));
    // Resume reads again from the resume point.
    r.act(ActionId::PlayPause);
    r.wait_idle();
    let first_after = r.app.spoken_log()[before];
    assert_eq!(Some(first_after.start), resume_at.or(Some(CharPos(0))));
}

/// A backend whose "plain" voice gives no word events and cannot change
/// pitch, like some SAPI voices.
struct TwoVoices {
    plain: bool,
}

impl SpeechBackend for TwoVoices {
    fn id(&self) -> &'static str {
        "two-voices"
    }
    fn capabilities(&self) -> Caps {
        if self.plain {
            Caps::PAUSE | Caps::VOLUME
        } else {
            Caps::WORD_EVENTS | Caps::PAUSE | Caps::PITCH | Caps::VOLUME
        }
    }
    fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        Ok(["words", "plain"]
            .into_iter()
            .map(|id| Voice {
                id: id.into(),
                name: id.into(),
                ..Voice::default()
            })
            .collect())
    }
    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
        self.plain = params.voice.as_deref() == Some("plain");
        Ok(())
    }
    fn effective_wpm(&self) -> u16 {
        265
    }
    fn speak(&mut self, u: &Utterance, sink: &mut dyn EventSink) -> Result<(), SpeechError> {
        sink.emit(u.id, RawEvent::Started);
        sink.emit(u.id, RawEvent::Finished);
        Ok(())
    }
    fn stop(&mut self) {}
}

#[test]
fn a_voice_without_word_events_is_announced() {
    let speech = SpeechService::spawn(
        Box::new(|| Ok(Box::new(TwoVoices { plain: false }) as _)),
        ServiceConfig::default(),
    )
    .unwrap();
    assert!(speech.capabilities().contains(Caps::WORD_EVENTS));
    let said = Said::default();
    let mut settings = Settings::default();
    settings.speech.voice = Some("plain".into());
    let mut app = App::new(AppConfig {
        settings,
        speech,
        announcer: Box::new(said.clone()),
        ..AppConfig::for_tests()
    });
    let deadline = Instant::now() + Duration::from_secs(30);
    while !said.any("does not report words") {
        app.poll_speech();
        assert!(
            Instant::now() < deadline,
            "no announcement: {:?}",
            said.all()
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(
        said.last(),
        "This voice does not report words, so the word highlight is estimated. Pitch cannot be changed with this voice."
    );
    assert!(!app.speech_capabilities().contains(Caps::WORD_EVENTS));
}

// ---- New actions ----

#[test]
fn read_paragraph_reads_it_in_place() {
    let mut r = rig(PROSE);
    r.go(at(PROSE, "theta"));
    r.act(ActionId::ReadParagraph);
    r.wait_idle();
    let spoken = r.log.spoken_ranges();
    assert_eq!(spoken.first().unwrap().start, CharPos(0));
    assert_eq!(spoken.last().unwrap().end, at(PROSE, "\n\nKappa"));
    assert_eq!(r.cursor(), at(PROSE, "theta"), "reading in place");
}

#[test]
fn select_actions_extend_the_selection() {
    let mut r = rig(PROSE);
    r.act(ActionId::SelectNextWord);
    r.act(ActionId::SelectNextWord);
    let sel = r.app.session().unwrap().selection.unwrap();
    assert_eq!(r.app.session().unwrap().doc.slice(sel), "Alpha beta");
    assert_eq!(r.said.last(), "beta selected");
    r.act(ActionId::SelectPreviousWord);
    assert_eq!(r.said.last(), "beta unselected");
    r.act(ActionId::SelectNextLine);
    let sel = r.app.session().unwrap().selection.unwrap();
    assert_eq!(sel.end, at(PROSE, "\n\nKappa"));
    r.act(ActionId::SelectPreviousLine);
    assert_eq!(r.app.session().unwrap().selection, None);
    // Reading the selection reads exactly it.
    r.act(ActionId::SelectNextWord);
    r.act(ActionId::ReadSelection);
    r.wait_idle();
    assert_eq!(
        r.log.spoken_ranges().last().copied(),
        Some(CharRange::new(0, 5))
    );
}

#[test]
fn every_action_is_wired_except_choose_voice() {
    // One app for all: Cancel closes whatever an action opened (a prompt, a
    // list) and answers no to a question, so the next action runs cleanly.
    let mut r = rig(PROSE);
    for &a in ActionId::ALL {
        r.said.0.lock().unwrap().clear();
        r.act(a);
        let unwired = r.said.any("is not available yet");
        r.app.dispatch(Command::Cancel);
        assert_eq!(
            unwired,
            a == ActionId::ChooseVoice,
            "{a:?}: {:?}",
            r.said.all()
        );
        assert!(r.app.pending_confirmation().is_none(), "{a:?}");
    }
}

#[test]
fn f9_turns_single_keys_off_and_on_and_saves() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::under(home.path());
    let mut r = launch(Settings::default(), Some(paths.clone()));
    let period: KeyChord = ".".parse().unwrap();
    let alt_period: KeyChord = "Alt+.".parse().unwrap();
    assert_eq!(
        r.app.keymap().lookup(&period, Layer::Browse),
        Some(ActionId::NextSentence)
    );
    r.act(ActionId::ToggleCharacterKeys);
    assert_eq!(r.said.last(), "Single-key shortcuts off.");
    assert!(!r.app.settings().keyboard.character_keys);
    assert_eq!(r.app.keymap().lookup(&period, Layer::Browse), None);
    assert_eq!(
        r.app.keymap().lookup(&alt_period, Layer::Browse),
        Some(ActionId::NextSentence),
        "modifier chords keep working"
    );
    // Saved at once, and applied at the next start.
    let (saved, _) = SettingsStore::new(paths.clone()).load();
    assert!(!saved.keyboard.character_keys);
    let r2 = launch(saved, Some(paths.clone()));
    assert_eq!(r2.app.keymap().lookup(&period, Layer::Browse), None);
    // And back on.
    r.act(ActionId::ToggleCharacterKeys);
    assert_eq!(r.said.last(), "Single-key shortcuts on.");
    assert_eq!(
        r.app.keymap().lookup(&period, Layer::Browse),
        Some(ActionId::NextSentence)
    );
    assert!(SettingsStore::new(paths).load().0.keyboard.character_keys);
}

// ---- Library ----

struct Library {
    _tmp: tempfile::TempDir,
    folder: PathBuf,
    paths: Paths,
    outside: PathBuf,
}

fn library() -> Library {
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path().join("Readings");
    std::fs::create_dir_all(folder.join("week1")).unwrap();
    std::fs::write(folder.join("alpha.txt"), PROSE).unwrap();
    std::fs::write(
        folder.join("week1").join("beta.md"),
        "# Beta\n\nBeta text here.",
    )
    .unwrap();
    std::fs::write(folder.join("image.png"), "not a document").unwrap();
    let outside = tmp.path().join("loose.txt");
    std::fs::write(&outside, "A loose file.").unwrap();
    let paths = Paths::under(&tmp.path().join("home"));
    Library {
        folder,
        paths,
        outside,
        _tmp: tmp,
    }
}

impl Library {
    fn settings(&self, policy: ConflictPolicy) -> Settings {
        let mut s = Settings::default();
        s.library.add_folder(&self.folder);
        s.reading.sync_conflict_policy = policy;
        s
    }
}

fn items_of(effects: &[Effect]) -> Vec<String> {
    effects
        .iter()
        .find_map(|e| match e {
            Effect::ShowList { title, items } if title == "Library" => Some(items.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

#[test]
fn the_library_lists_folder_documents_and_recent_files_and_opens_them() {
    let lib = library();
    let mut r = launch(Settings::default(), Some(lib.paths.clone()));
    // Nothing yet.
    r.act(ActionId::OpenLibrary);
    assert!(
        r.said.last().starts_with("The library is empty."),
        "{}",
        r.said.last()
    );

    // A loose file opened once becomes a recent entry; the folder's
    // documents are listed first.
    let mut r = launch(
        lib.settings(ConflictPolicy::Newest),
        Some(lib.paths.clone()),
    );
    r.app.open(&lib.outside).unwrap();
    let effects = r.act(ActionId::OpenLibrary);
    let items = items_of(&effects);
    assert_eq!(items.len(), 3, "{items:?}");
    assert!(items[0].starts_with("alpha, in Readings"), "{items:?}");
    assert!(items[1].starts_with("beta, in Readings"), "{items:?}");
    assert!(items[2].starts_with("loose.txt, recent"), "{items:?}");
    assert_eq!(r.said.last(), "Library, 3 documents. Enter opens one.");

    // Enter on the first opens it and puts it on the bookshelf.
    r.app.dispatch(Command::Choose(0));
    assert_eq!(r.app.session().unwrap().title, "alpha.txt");
    let shelf = textweaver_app::store::Library::load(&lib.paths.library_file()).unwrap();
    assert!(shelf.get(&lib.folder.join("alpha.txt")).is_some());
    assert!(shelf.get(&lib.outside).is_some());
}

#[test]
fn positions_sync_through_the_library_folder() {
    let lib = library();
    let doc = lib.folder.join("alpha.txt");
    // Read somewhere and quit: the sidecar gets the position.
    let mut r = launch(
        lib.settings(ConflictPolicy::Newest),
        Some(lib.paths.clone()),
    );
    r.app.open(&doc).unwrap();
    r.go(at(PROSE, "Kappa"));
    r.quit();
    let sidecar: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(sidecar_file(&lib.folder)).unwrap()).unwrap();
    assert_eq!(sidecar["alpha.txt"]["offset"], at(PROSE, "Kappa").0);

    // Another device read further, later: the next open resumes there.
    let later = textweaver_app::store::now_ts() + 60;
    let theirs = ProgressEntry::new(at(PROSE, "Omicron"), 90, later).to_value();
    std::fs::write(
        sidecar_file(&lib.folder),
        serde_json::json!({ "alpha.txt": theirs }).to_string(),
    )
    .unwrap();
    let mut r = launch(
        lib.settings(ConflictPolicy::Newest),
        Some(lib.paths.clone()),
    );
    r.app.open(&doc).unwrap();
    assert_eq!(r.cursor(), at(PROSE, "Omicron"));
    assert!(
        r.said.any("percent, from another device."),
        "{:?}",
        r.said.all()
    );
    drop(r);

    // Under the manual policy this device's position is kept, and the user
    // is told the other device differs.
    let local = StateStore::new(lib.paths.state_dir())
        .load(&DocKey::for_path(&doc))
        .unwrap();
    assert!(local.has_position());
    assert_eq!(local.position, at(PROSE, "Kappa"));
    let mut r = launch(
        lib.settings(ConflictPolicy::Manual),
        Some(lib.paths.clone()),
    );
    r.app.open(&doc).unwrap();
    assert_eq!(r.cursor(), at(PROSE, "Kappa"));
    assert!(
        r.said
            .any("Another device is at a different place; kept this device's."),
        "{:?}",
        r.said.all()
    );
}

#[test]
fn opening_from_the_library_asks_about_unsaved_edits_first() {
    let lib = library();
    let mut r = launch(
        lib.settings(ConflictPolicy::Newest),
        Some(lib.paths.clone()),
    );
    r.app.open(&lib.outside).unwrap();
    r.act(ActionId::ToggleEditMode);
    r.app.dispatch(Command::Insert("x".into()));
    r.act(ActionId::OpenLibrary);
    let effects = r.app.dispatch(Command::Choose(0));
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, Effect::ShowList { items, .. } if items.len() == 3)),
        "Save / Discard / Cancel: {effects:?}"
    );
}
