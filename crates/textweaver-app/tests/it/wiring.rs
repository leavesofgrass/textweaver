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
        let mut effects = self.app.dispatch(Command::Action(a));
        // The library is scanned on a background thread; its list opens on
        // a tick, as in the event loop.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while self.app.library_scanning() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(2));
            effects.extend(self.app.tick(std::time::Instant::now()));
        }
        effects
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
    // Whatever the paused reading still reports is ignored: once the speech
    // thread has handled the pause, everything it reported is waiting.
    r.app.wait_for_speech_thread();
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
fn every_action_is_wired() {
    // One app for all: Cancel closes whatever an action opened (a prompt, a
    // list) and answers no to a question, so the next action runs cleanly.
    let mut r = rig(PROSE);
    for &a in ActionId::ALL {
        r.said.0.lock().unwrap().clear();
        r.act(a);
        let unwired = r.said.any("is not available yet");
        r.app.dispatch(Command::Cancel);
        // Choose voice was the last one (Agent D4 wired it).
        assert!(!unwired, "{a:?}: {:?}", r.said.all());
        assert!(r.app.pending_confirmation().is_none(), "{a:?}");
    }
}

#[test]
fn f9_turns_single_keys_off_and_on_and_saves() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::under(home.path());
    let mut r = launch(Settings::default(), Some(paths.clone()));
    let period: KeyChord = ".".parse().unwrap();
    // The sentence chord this platform binds, from the keymap.
    let alt_period: KeyChord = r
        .app
        .keymap()
        .chords_for(ActionId::NextSentence)
        .into_iter()
        .find(|c| !c.is_text_input())
        .expect("next sentence has a chord");
    assert_eq!(
        r.app.keymap().lookup(&period, Layer::Browse),
        Some(ActionId::ReadCurrentSentence)
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
    // Saved at once (by the writer thread), and applied at the next start.
    r.app.wait_for_writes();
    let (saved, _) = SettingsStore::new(paths.clone()).load();
    assert!(!saved.keyboard.character_keys);
    let r2 = launch(saved, Some(paths.clone()));
    assert_eq!(r2.app.keymap().lookup(&period, Layer::Browse), None);
    // And back on.
    r.act(ActionId::ToggleCharacterKeys);
    assert_eq!(r.said.last(), "Single-key shortcuts on.");
    assert_eq!(
        r.app.keymap().lookup(&period, Layer::Browse),
        Some(ActionId::ReadCurrentSentence)
    );
    r.app.wait_for_writes();
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
        s.sync.position_policy =
            textweaver_app::store::PositionPolicy::from_conflict_policy(policy);
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
    assert!(
        r.said
            .any("Library, 3 documents. Type to filter, Enter opens one, F2 edits details.")
    );
    // Then the focused item (the app's list model, Wave 3).
    assert!(r.said.last().starts_with("1 of 3, "), "{}", r.said.last());

    // Enter on the first opens it and puts it on the bookshelf.
    r.app.dispatch(Command::Choose(0));
    assert_eq!(r.app.session().unwrap().title, "alpha.txt");
    // The bookshelf is written by the background writer; opening no
    // longer waits for it on the input thread (W6f).
    assert!(r.app.flush_writes(Duration::from_secs(10)));
    let shelf = textweaver_app::store::Library::load(&lib.paths.library_file()).unwrap();
    assert!(shelf.get(&lib.folder.join("alpha.txt")).is_some());
    assert!(shelf.get(&lib.outside).is_some());
}

/// Wave 5 (W5y): the library list filters as you type, by title, author,
/// DOI, and ISBN, recorded on the bookshelf when a document opens.
#[test]
fn the_library_filters_by_author_doi_and_isbn() {
    let lib = library();
    let paper = lib.folder.join("paper.md");
    std::fs::write(
        &paper,
        "---\ntitle: Cell Energy\nauthor: Ada Example\ndoi: 10.1000/XYZ\n---\n\n# Cell Energy\n\nISBN 978-0-306-40615-7\n",
    )
    .unwrap();
    let mut r = launch(
        lib.settings(ConflictPolicy::Newest),
        Some(lib.paths.clone()),
    );
    r.app.open(&paper).unwrap();
    assert!(r.app.flush_writes(Duration::from_secs(20)));
    let shelf = textweaver_app::store::Library::load(&lib.paths.library_file()).unwrap();
    let entry = shelf.get(&paper).unwrap();
    assert_eq!(entry.meta.author.as_deref(), Some("Ada Example"));
    assert_eq!(entry.meta.doi.as_deref(), Some("10.1000/xyz"));
    assert_eq!(entry.meta.isbn.as_deref(), Some("9780306406157"));

    let effects = r.act(ActionId::OpenLibrary);
    assert_eq!(items_of(&effects).len(), 3, "{effects:?}");
    assert_eq!(r.app.list_filter(), Some(""), "the library filters");
    let filter = |r: &mut Rig, q: &str| -> Vec<String> {
        r.said.0.lock().unwrap().clear();
        r.app
            .dispatch(Command::FilterList(q.into()))
            .into_iter()
            .find_map(|e| match e {
                Effect::ShowList { items, .. } => Some(items),
                _ => None,
            })
            .unwrap_or_default()
    };
    for q in [
        "10.1000/xyz",
        "doi:10.1000/XYZ",
        "0306406152",
        "ada example",
    ] {
        let shown = filter(&mut r, q);
        assert_eq!(shown.len(), 1, "{q}: {shown:?}");
        assert!(
            shown[0].starts_with("Cell Energy, by Ada Example"),
            "{shown:?}"
        );
        assert!(
            r.said.all().iter().any(|s| s == "1 document matches."),
            "{:?}",
            r.said.all()
        );
    }
    assert!(filter(&mut r, "grace").is_empty());
    assert!(
        r.said.any("No documents match grace."),
        "{:?}",
        r.said.all()
    );
    assert_eq!(filter(&mut r, "").len(), 3);
    assert!(
        r.said.any("Filter cleared, 3 documents."),
        "{:?}",
        r.said.all()
    );
    // Enter opens the item shown, not the one at that place unfiltered.
    filter(&mut r, "beta");
    r.app.dispatch(Command::Choose(0));
    let title = r.app.session().unwrap().title.to_lowercase();
    assert!(title.contains("beta"), "{title}");
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

    // Under the "ask" policy (formerly manual) this device's position is
    // kept, and the other device's is asked about (ADR-0049, problem 1:
    // "ask" used to keep this one without asking).
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
        r.said.any("another computer at") && r.said.any("Go there? Y or N"),
        "{:?}",
        r.said.all()
    );
    assert!(r.app.confirmation_pending());
    r.app
        .dispatch(Command::Confirm(textweaver_app::Confirm::Yes));
    assert_ne!(
        r.cursor(),
        at(PROSE, "Kappa"),
        "yes goes to the other place"
    );
    assert!(!r.app.confirmation_pending());
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

// ---- GUI requests (Agent K) ----

#[test]
fn set_cursor_moves_quietly_and_sets_the_resume_point() {
    let mut r = rig(PROSE);
    let before = r.said.all().len();
    r.app.dispatch(Command::SetCursor(at(PROSE, "Kappa")));
    assert_eq!(r.cursor(), at(PROSE, "Kappa"));
    assert_eq!(r.said.all().len(), before, "no announcement");
    assert_eq!(r.app.playback(), Playback::Idle, "no reading");
    // No history entry: Back has nowhere to go.
    r.act(ActionId::HistoryBack);
    assert_eq!(r.cursor(), at(PROSE, "Kappa"));
    // Out of range is clamped.
    r.app.dispatch(Command::SetCursor(CharPos(10_000)));
    assert_eq!(r.cursor(), CharPos(PROSE.chars().count()));

    // While paused, reading resumes from the new place.
    r.app.dispatch(Command::SetCursor(CharPos(0)));
    r.act(ActionId::ReadFromCursor);
    r.act(ActionId::PlayPause);
    assert!(matches!(r.app.playback(), Playback::Paused { .. }));
    r.app.dispatch(Command::SetCursor(at(PROSE, "Omicron")));
    assert_eq!(
        r.app.playback(),
        Playback::Paused {
            resume_at: Some(at(PROSE, "Omicron"))
        }
    );
    r.act(ActionId::PlayPause);
    r.wait_idle();
    // The resumed reading is the last one and starts at Omicron.
    assert_eq!(
        r.log.spoken_ranges().last().unwrap().start,
        at(PROSE, "Omicron")
    );
    assert_eq!(r.app.spoken_log().last().unwrap().start, at(PROSE, "pi"));
}

#[test]
fn play_after_saying_one_word_reads_on_instead_of_pausing() {
    let mut r = rig(PROSE);
    r.go(at(PROSE, "Kappa"));
    r.act(ActionId::ReadCurrentWord);
    assert_eq!(r.app.playback(), Playback::Reading);
    r.act(ActionId::PlayPause);
    assert_eq!(r.app.playback(), Playback::Reading, "reading, not paused");
    r.wait_idle();
    // The last reading ran from Kappa to the end of the document.
    let last = r.log.spoken_ranges();
    assert_eq!(last.last().unwrap().end.0, PROSE.len());
    assert!(
        last.iter()
            .any(|s| s.start == at(PROSE, "Kappa") && s.len() > 5),
        "{last:?}"
    );
}
