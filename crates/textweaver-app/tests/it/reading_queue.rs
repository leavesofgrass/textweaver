//! The reading queue (`[reading] after_end`) and the review list, beta 3
//! (B1-s3). Driven through `App::dispatch` with the recording speech
//! double and a data folder in a temporary directory; nothing is played.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use textweaver_app::keymap::ActionId;
use textweaver_app::store::{AfterEnd, Library, Paths, ReadingStats, Settings, StatsDelta};
use textweaver_app::testing::{SpeechLog, recording_service};
use textweaver_app::{App, AppConfig, Command, Effect, ListKey, Playback};

struct Lab {
    app: App,
    log: SpeechLog,
    paths: Paths,
    week3: PathBuf,
    week4: PathBuf,
    _tmp: tempfile::TempDir,
}

/// Two short documents in a library folder, and a self-voicing app with
/// `after_end`.
fn lab(after_end: AfterEnd) -> Lab {
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path().join("readings");
    std::fs::create_dir_all(&folder).unwrap();
    let week3 = folder.join("week3.md");
    let week4 = folder.join("week4.md");
    std::fs::write(&week3, "# Week 3\n\nOne fish.\n").unwrap();
    std::fs::write(&week4, "# Week 4\n\nTwo fish.\n").unwrap();
    let paths = Paths::under(&tmp.path().join("home"));
    let mut settings = Settings::default();
    settings.reading.after_end = after_end;
    settings.library.add_folder(&folder);
    let (speech, log) = recording_service().unwrap();
    let app = App::new(AppConfig {
        speech,
        self_voicing: true,
        backend_name: "test-recording".into(),
        settings,
        paths: Some(paths.clone()),
        ..AppConfig::for_tests()
    });
    Lab {
        app,
        log,
        paths,
        week3,
        week4,
        _tmp: tmp,
    }
}

impl Lab {
    fn act(&mut self, a: ActionId) -> Vec<Effect> {
        let mut effects = self.app.dispatch(Command::Action(a));
        let deadline = Instant::now() + Duration::from_secs(20);
        while self.app.library_scanning() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
            effects.extend(self.app.tick(Instant::now()));
        }
        effects
    }

    fn open(&mut self, p: &Path) {
        self.app.open(p).unwrap();
    }

    /// Reads continuously until speech is done.
    fn read_to_end(&mut self) {
        self.act(ActionId::ReadFromCursor);
        self.wait_idle();
    }

    fn wait_idle(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            self.app.poll_speech();
            if self.app.playback() == Playback::Idle {
                return;
            }
            assert!(Instant::now() < deadline, "still reading");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn status(&self) -> String {
        self.app.status_text().to_owned()
    }

    fn said(&self, needle: &str) -> bool {
        self.log.texts().iter().any(|t| t.contains(needle))
    }

    fn open_path(&self) -> Option<PathBuf> {
        self.app.session()?.doc.meta.path.clone()
    }

    fn shelf(&mut self) -> Library {
        assert!(self.app.flush_writes(Duration::from_secs(10)));
        Library::load(&self.paths.library_file()).unwrap()
    }

    /// Queues week 4, then week 3, with the queue command on each, and
    /// leaves week 3 open.
    fn queue_both(&mut self) {
        let (w3, w4) = (self.week3.clone(), self.week4.clone());
        self.open(&w4);
        self.act(ActionId::QueueDocument);
        assert!(
            self.status().starts_with("Added to the reading queue: "),
            "{}",
            self.status()
        );
        self.open(&w3);
        self.act(ActionId::QueueDocument);
        assert!(
            self.status().ends_with("2 documents in the queue."),
            "{}",
            self.status()
        );
    }
}

fn list_of(effects: &[Effect], want: &str) -> Vec<String> {
    effects
        .iter()
        .find_map(|e| match e {
            Effect::ShowList { title, items } if title == want => Some(items.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

#[test]
fn ask_names_the_next_document_and_never_starts_it_unasked() {
    let mut l = lab(AfterEnd::Ask);
    l.queue_both();
    l.read_to_end();
    let msg = l.status();
    assert!(msg.starts_with("Done reading. Next: "), "{msg}");
    assert!(msg.ends_with(" to read."), "{msg}");
    // Nothing was opened or read: week 3 is still the document.
    assert!(l.open_path().unwrap().ends_with("week3.md"));
    assert!(!l.said("Two fish."));
    // Week 3 is finished: it left the queue; week 4 waits.
    let shelf = l.shelf();
    assert!(!shelf.queued(&l.week3));
    assert!(shelf.queued(&l.week4));
    // The read key on the offer opens week 4 and reads it.
    l.log.clear();
    l.act(ActionId::ReadFromCursor);
    assert!(l.open_path().unwrap().ends_with("week4.md"));
    l.wait_idle();
    assert!(l.said("Two fish."), "{:?}", l.log.texts());
    // Read to the end, it leaves the queue too, and nothing is offered.
    assert!(!l.status().contains("Next:"), "{}", l.status());
    assert!(l.shelf().queue.is_empty());
}

#[test]
fn ask_is_dropped_when_the_reader_moves_away() {
    let mut l = lab(AfterEnd::Ask);
    l.queue_both();
    l.read_to_end();
    assert!(l.status().starts_with("Done reading. Next: "));
    // Moving off the end and reading reads this document again.
    l.act(ActionId::DocumentStart);
    l.log.clear();
    l.read_to_end();
    assert!(l.open_path().unwrap().ends_with("week3.md"));
    assert!(l.said("One fish."), "{:?}", l.log.texts());
}

#[test]
fn stop_is_the_default_and_names_nothing() {
    assert_eq!(Settings::default().reading.after_end, AfterEnd::Stop);
    let mut l = lab(AfterEnd::Stop);
    l.queue_both();
    l.read_to_end();
    // "Done reading." alone, said at high verbosity as before.
    assert!(!l.status().contains("Next:"), "{}", l.status());
    assert!(l.open_path().unwrap().ends_with("week3.md"));
    l.log.clear();
    l.act(ActionId::ReadFromCursor);
    l.wait_idle();
    assert!(l.open_path().unwrap().ends_with("week3.md"));
    assert!(!l.said("Two fish."));
    // Finished documents leave the queue whatever the setting.
    assert!(!l.shelf().queued(&l.week3));
}

#[test]
fn next_opens_and_reads_the_next_document() {
    let mut l = lab(AfterEnd::Next);
    l.queue_both();
    l.act(ActionId::ReadFromCursor);
    // Week 3, then week 4, read through.
    let deadline = Instant::now() + Duration::from_secs(10);
    while !l.said("Two fish.") {
        l.app.poll_speech();
        assert!(Instant::now() < deadline, "{:?}", l.log.texts());
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(l.said("One fish."));
    assert!(l.open_path().unwrap().ends_with("week4.md"));
    l.wait_idle();
    assert!(l.shelf().queue.is_empty());
}

#[test]
fn the_library_list_queues_with_ctrl_enter_and_the_queue_list_removes() {
    let mut l = lab(AfterEnd::Stop);
    let effects = l.act(ActionId::OpenLibrary);
    let rows = list_of(&effects, "Library");
    assert_eq!(rows.len(), 2, "{rows:?}");
    l.app.dispatch(Command::ListKey(ListKey::ChooseHere));
    assert!(
        l.status()
            .starts_with("Added to the reading queue: week3. 1 document"),
        "{}",
        l.status()
    );
    assert!(l.app.list_model().is_some(), "the library stays open");
    l.app.dispatch(Command::ListKey(ListKey::Down));
    l.app.dispatch(Command::ListKey(ListKey::ChooseHere));
    assert!(l.status().ends_with("2 documents in the queue."));
    // Again takes it out.
    l.app.dispatch(Command::ListKey(ListKey::ChooseHere));
    assert_eq!(l.status(), "Removed from the reading queue: week4.");
    l.app.dispatch(Command::ListKey(ListKey::ChooseHere));
    l.app.dispatch(Command::ListKey(ListKey::Escape));
    assert_eq!(l.shelf().queue.len(), 2);

    let effects = l.act(ActionId::ReadingQueue);
    assert_eq!(list_of(&effects, "Reading queue"), ["week3", "week4"]);
    l.app.dispatch(Command::ListKey(ListKey::Delete));
    assert!(
        l.said("Removed from the reading queue: week3."),
        "{:?}",
        l.log.texts()
    );
    let shelf = l.shelf();
    assert_eq!(shelf.queue.len(), 1);
    assert!(shelf.queued(&l.week4));
    // Enter opens the document; reading waits for the read key.
    l.act(ActionId::ReadingQueue);
    l.app.dispatch(Command::ListKey(ListKey::Enter));
    assert!(l.open_path().unwrap().ends_with("week4.md"));
    assert_eq!(l.app.playback(), Playback::Idle);
}

#[test]
fn the_review_list_is_oldest_first_with_days_percent_and_notes() {
    let mut l = lab(AfterEnd::Stop);
    let empty = l.act(ActionId::ReviewList);
    assert!(list_of(&empty, "Review list").is_empty());
    assert!(
        l.status().starts_with("Nothing to review yet."),
        "{}",
        l.status()
    );
    let now = textweaver_app::store::now_ts();
    let delta = |p: &Path, title: &str, days: i64, pct: u8| StatsDelta {
        key: textweaver_app::store::DocKey::for_path(p).0,
        title: title.into(),
        path: Some(p.to_owned()),
        seconds: 60.0,
        new_session: true,
        furthest_percent: pct,
        furthest_char: 10,
        at: now - days * 86_400 - 30,
    };
    ReadingStats::add_to_file(
        &l.paths,
        &[
            delta(&l.week4, "Week 4", 2, 30),
            delta(&l.week3, "Week 3", 21, 80),
        ],
    )
    .unwrap();
    let effects = l.act(ActionId::ReviewList);
    let rows = list_of(&effects, "Review list");
    assert_eq!(
        rows,
        [
            "Week 3, 21 days, 80 percent, no notes",
            "Week 4, 2 days, 30 percent, no notes"
        ]
    );
    l.app.dispatch(Command::ListKey(ListKey::Enter));
    assert!(l.open_path().unwrap().ends_with("week3.md"));
}
