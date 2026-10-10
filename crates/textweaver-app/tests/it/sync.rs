//! Sync in the reader (the sync wave, S4; ADR-0049): two or three app
//! instances, each with its own home, sharing one temporary sync folder.
//! No real sync service and no audio: the folder is a temporary folder,
//! and speech is the recording backend.
//!
//! After every scenario the sync folder is scanned for anything that names
//! a computer, a user, or a path.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::core::CharPos;
use textweaver_app::keymap::{ActionId, Layer};
use textweaver_app::lexicon::i18n::Catalog;
use textweaver_app::store::{InterfaceAnnouncements, Paths, PositionPolicy, Settings};
use textweaver_app::testing::recording_service;
use textweaver_app::text::GoTo;
use textweaver_app::{App, AppConfig, Command, Confirm, ListKey, Playback};

#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Said {
    fn all(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
    fn any(&self, needle: &str) -> bool {
        self.all().iter().any(|s| s.contains(needle))
    }
    fn count(&self, needle: &str) -> usize {
        self.all().iter().filter(|s| s.contains(needle)).count()
    }
    fn clear(&self) {
        self.0.lock().unwrap().clear();
    }
}

impl Announcer for Said {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}

/// One computer: its own home folder and app, sharing `folder`.
struct Computer {
    app: App,
    said: Said,
    home: tempfile::TempDir,
    /// A fake clock for ticks, so publishing and scanning are due at once.
    clock: Instant,
}

const TEXT: &str = "Cells divide by mitosis. Each daughter cell gets a full copy.\n\n\
Meiosis makes gametes. It halves the chromosome count.\n\n\
Proteins fold into shapes. Shape decides what they do.\n\n\
Enzymes speed reactions. They are not used up.\n";

fn computer(folder: &Path, name: &str, tweak: impl FnOnce(&mut Settings)) -> Computer {
    let home = tempfile::tempdir().unwrap();
    let mut settings = Settings::default();
    settings.sync.enabled = true;
    settings.sync.folder = Some(folder.to_owned());
    settings.sync.device_name = name.to_owned();
    tweak(&mut settings);
    let said = Said::default();
    let (speech, _log) = recording_service().unwrap();
    let app = App::new(AppConfig {
        settings,
        speech,
        paths: Some(Paths::under(home.path())),
        announcer: Box::new(said.clone()),
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    Computer {
        app,
        said,
        home,
        clock: Instant::now(),
    }
}

/// A copy of the shared document in its own folder: the same book at a
/// different path on each computer.
fn copy_of(text: &str, name: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    std::fs::write(&path, text).unwrap();
    (dir, path)
}

impl Computer {
    /// Ticks and waits for the writer until sync has nothing left to do.
    fn settle(&mut self) {
        for _ in 0..8 {
            self.clock += Duration::from_secs(5);
            self.app.tick(self.clock);
            self.app.wait_for_writes();
            if self.app.sync_idle() && !self.app.confirmation_pending() {
                self.clock += Duration::from_secs(5);
                self.app.tick(self.clock);
                self.app.wait_for_writes();
            }
        }
    }

    fn open(&mut self, path: &Path) {
        self.app.open(path).unwrap();
        self.app.wait_for_writes();
        self.settle();
    }

    fn cursor(&self) -> CharPos {
        self.app.session().unwrap().cursor
    }

    fn notes(&self) -> Vec<String> {
        self.app
            .session()
            .unwrap()
            .notes
            .iter()
            .map(|n| n.note.clone())
            .collect()
    }

    fn act(&mut self, a: ActionId) {
        self.app.dispatch(Command::Action(a));
    }

    fn add_note(&mut self, text: &str) {
        self.act(ActionId::AddNote);
        self.app.dispatch(Command::Answer(text.to_owned()));
        self.app.wait_for_writes();
    }

    fn edit_first_note(&mut self, text: &str) {
        self.act(ActionId::ListNotes);
        self.app.dispatch(Command::RenameItem(0));
        self.app.dispatch(Command::Answer(text.to_owned()));
        self.app.dispatch(Command::Cancel);
        self.app.wait_for_writes();
    }

    fn go_percent(&mut self, pct: u8) {
        self.app.dispatch(Command::GoTo(GoTo::Percent(pct)));
        self.app.save_position().unwrap();
        self.app.wait_for_writes();
    }

    fn quit(&mut self) {
        self.app.shutdown();
    }
}

/// Fails when a file in the sync folder names a path, a file, a user, or a
/// computer (ADR-0049, privacy).
fn privacy_scan(folder: &Path, homes: &[&Path], docs: &[&Path]) {
    let mut forbidden: Vec<String> = textweaver_app::sync_folder::local_names()
        .into_iter()
        .filter(|n| n.len() >= 3)
        .collect();
    for h in homes.iter().chain(docs) {
        forbidden.push(h.display().to_string());
        forbidden.push(h.display().to_string().replace('\\', "\\\\"));
    }
    for d in docs {
        if let Some(name) = d.file_name() {
            forbidden.push(name.to_string_lossy().into_owned());
        }
    }
    let mut files = Vec::new();
    walk(folder, &mut files);
    assert!(!files.is_empty(), "the sync folder has files");
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap_or_default();
        for bad in &forbidden {
            assert!(
                !text.contains(bad.as_str()),
                "{} names {bad:?}",
                f.display()
            );
        }
        for pattern in [":\\\\", ":/", "/home/", "/Users/", "/tmp/"] {
            assert!(
                !text.contains(pattern),
                "{} holds a path ({pattern})",
                f.display()
            );
        }
    }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

#[test]
fn a_note_made_on_one_computer_appears_on_the_other() {
    let folder = tempfile::tempdir().unwrap();
    let (da, pa) = copy_of(TEXT, "biology.txt");
    let (db, pb) = copy_of(TEXT, "Biology notes.txt");
    let mut laptop = computer(folder.path(), "laptop", |_| {});
    let mut lab = computer(folder.path(), "lab", |_| {});

    laptop.open(&pa);
    laptop.add_note("Mitosis keeps the count");
    laptop.settle();

    lab.open(&pb);
    assert_eq!(lab.notes(), vec!["Mitosis keeps the count".to_owned()]);
    assert!(lab.said.any("1 change from laptop"), "{:?}", lab.said.all());
    // Found in the lab's text, on the sentence it was made on.
    let n = &lab.app.session().unwrap().notes[0];
    assert_eq!(n.range.start, CharPos(0));

    // A deletion travels too, and does not come back.
    lab.act(ActionId::ListNotes);
    lab.app.dispatch(Command::DeleteItem(0));
    lab.app.dispatch(Command::Confirm(Confirm::Yes));
    lab.app.dispatch(Command::Cancel);
    lab.app.wait_for_writes();
    lab.settle();
    laptop.settle();
    assert!(laptop.notes().is_empty(), "{:?}", laptop.notes());
    lab.settle();
    assert!(lab.notes().is_empty());

    laptop.quit();
    lab.quit();
    privacy_scan(
        folder.path(),
        &[laptop.home.path(), lab.home.path()],
        &[&pa, &pb, da.path(), db.path()],
    );
}

/// The study cards in `path`'s card file on computer `c`: each card's
/// grades, by name.
fn card_grades(c: &Computer, path: &Path) -> Vec<Vec<String>> {
    let paths = Paths::under(c.home.path());
    textweaver_app::store::CardStore::new(paths.cards_dir())
        .load(&textweaver_app::store::DocKey::for_path(path))
        .cards
        .iter()
        .map(|card| {
            card.reviews
                .iter()
                .map(|r| r.grade.as_str().to_owned())
                .collect()
        })
        .collect()
}

/// Cards sync with the notes (B1-f2): a card made and graded on one
/// computer arrives on the other with its grade, and a grade given on
/// each computer survives on both.
#[test]
fn a_card_and_its_grades_arrive_on_the_other_computer() {
    let folder = tempfile::tempdir().unwrap();
    let (da, pa) = copy_of(TEXT, "biology.txt");
    let (db, pb) = copy_of(TEXT, "Biology notes.txt");
    let mut laptop = computer(folder.path(), "laptop", |_| {});
    let mut lab = computer(folder.path(), "lab", |_| {});

    laptop.open(&pa);
    laptop.add_note("What keeps the count?");
    laptop.act(ActionId::MakeCards);
    laptop.act(ActionId::StudyCards);
    laptop.act(ActionId::GradeGood);
    laptop.app.wait_for_writes();
    assert_eq!(card_grades(&laptop, &pa), vec![vec!["good".to_owned()]]);
    laptop.settle();

    lab.open(&pb);
    lab.settle();
    assert_eq!(card_grades(&lab, &pb), vec![vec!["good".to_owned()]]);

    // A grade on each computer: both keep both.
    lab.act(ActionId::StudyCards);
    lab.act(ActionId::GradeAgain);
    lab.app.wait_for_writes();
    laptop.act(ActionId::StudyCards);
    laptop.act(ActionId::GradeEasy);
    laptop.app.wait_for_writes();
    for _ in 0..2 {
        lab.settle();
        laptop.settle();
    }
    let mut want = card_grades(&laptop, &pa);
    want[0].sort();
    assert_eq!(want, vec![vec!["again", "easy", "good"]]);
    let mut there = card_grades(&lab, &pb);
    there[0].sort();
    assert_eq!(there, want);

    laptop.quit();
    lab.quit();
    privacy_scan(
        folder.path(),
        &[laptop.home.path(), lab.home.path()],
        &[&pa, &pb, da.path(), db.path()],
    );
}

#[test]
fn the_newer_edit_of_a_note_wins_and_is_said() {
    let folder = tempfile::tempdir().unwrap();
    let (da, pa) = copy_of(TEXT, "cells.txt");
    let (db, pb) = copy_of(TEXT, "cells.txt");
    let mut laptop = computer(folder.path(), "laptop", |_| {});
    let mut lab = computer(folder.path(), "lab", |_| {});
    laptop.open(&pa);
    laptop.add_note("First words");
    laptop.settle();
    lab.open(&pb);
    assert_eq!(lab.notes(), vec!["First words".to_owned()]);

    // Both edit the note while apart; the lab's edit is the newer one.
    laptop.edit_first_note("Laptop version");
    laptop.settle();
    std::thread::sleep(Duration::from_millis(1100));
    lab.edit_first_note("Lab version");
    lab.settle();
    laptop.said.clear();
    laptop.settle();

    assert_eq!(laptop.notes(), vec!["Lab version".to_owned()]);
    assert_eq!(lab.notes(), vec!["Lab version".to_owned()]);
    let title = laptop.app.session().unwrap().title.clone();
    let expected = format!("{title}: a note was replaced by lab's newer edit.");
    assert!(laptop.said.any(&expected), "{:?}", laptop.said.all());

    // The replaced text is in the laptop's backup, and comes back.
    laptop.act(ActionId::SyncReplacedNotes);
    assert!(
        laptop.said.any("1 replaced note"),
        "{:?}",
        laptop.said.all()
    );
    laptop.app.dispatch(Command::Choose(0));
    assert_eq!(laptop.notes(), vec!["Laptop version".to_owned()]);
    assert!(laptop.said.any("Note put back: Laptop version"));
    // Restoring is a new edit: it wins on the lab too.
    laptop.settle();
    lab.settle();
    assert_eq!(lab.notes(), vec!["Laptop version".to_owned()]);

    laptop.quit();
    lab.quit();
    privacy_scan(
        folder.path(),
        &[laptop.home.path(), lab.home.path()],
        &[&pa, &pb, da.path(), db.path()],
    );
}

#[test]
fn a_place_arrives_and_resumes_with_the_other_computer_named() {
    let folder = tempfile::tempdir().unwrap();
    let (da, pa) = copy_of(TEXT, "a.txt");
    let (db, pb) = copy_of(TEXT, "b.txt");
    let mut laptop = computer(folder.path(), "laptop", |_| {});
    laptop.open(&pa);
    laptop.go_percent(50);
    let far = laptop.app.reading_position().unwrap();
    let far = textweaver_app::text_util::word_start(&laptop.app.session().unwrap().doc, far);
    laptop.quit();

    let mut lab = computer(folder.path(), "lab", |_| {});
    lab.open(&pb);
    assert_eq!(lab.cursor(), far, "{:?}", lab.said.all());
    let pct = textweaver_app::text_util::percent(&lab.app.session().unwrap().doc, far);
    assert!(
        lab.said
            .any(&format!("resumed at {pct} percent, from laptop")),
        "{:?}",
        lab.said.all()
    );
    lab.quit();
    privacy_scan(
        folder.path(),
        &[laptop.home.path(), lab.home.path()],
        &[&pa, &pb, da.path(), db.path()],
    );
}

#[test]
fn ask_really_asks_and_names_the_computer() {
    let folder = tempfile::tempdir().unwrap();
    let (_da, pa) = copy_of(TEXT, "a.txt");
    let (_db, pb) = copy_of(TEXT, "b.txt");
    let mut laptop = computer(folder.path(), "laptop", |_| {});
    laptop.open(&pa);
    laptop.go_percent(75);
    let far = laptop.app.reading_position().unwrap();
    let far = textweaver_app::text_util::word_start(&laptop.app.session().unwrap().doc, far);
    laptop.quit();

    let mut lab = computer(folder.path(), "Lab computer", |s| {
        s.sync.position_policy = PositionPolicy::Ask;
    });
    lab.app.open(&pb).unwrap();
    lab.app.wait_for_writes();
    lab.clock += Duration::from_secs(5);
    lab.app.tick(lab.clock);
    lab.app.wait_for_writes();
    assert_eq!(lab.cursor(), CharPos(0), "asking moves nothing");
    assert!(lab.app.confirmation_pending());
    let q = lab.app.pending_question().unwrap();
    assert!(
        q.starts_with("laptop at ") && q.ends_with("Go there? y or n"),
        "{q}"
    );
    lab.app.dispatch(Command::Confirm(Confirm::Yes));
    assert_eq!(lab.cursor(), far);
    assert!(lab.said.any("laptop's place,"), "{:?}", lab.said.all());
}

#[test]
fn the_cursor_never_moves_while_reading_and_messages_wait_for_the_pause() {
    let folder = tempfile::tempdir().unwrap();
    let (_da, pa) = copy_of(TEXT, "a.txt");
    let (_db, pb) = copy_of(TEXT, "b.txt");
    let mut laptop = computer(folder.path(), "laptop", |_| {});
    let mut lab = computer(folder.path(), "lab", |_| {});
    laptop.open(&pa);
    lab.open(&pb);

    // The lab reads from the start; while it reads, the laptop adds a note
    // and moves far on.
    lab.act(ActionId::ReadFromCursor);
    assert_eq!(lab.app.playback(), Playback::Reading);
    let reading_from = lab.app.reading_position().unwrap();
    let cursor = lab.cursor();
    laptop.add_note("Proteins note");
    laptop.go_percent(90);
    laptop.settle();
    lab.said.clear();
    for _ in 0..4 {
        lab.clock += Duration::from_secs(5);
        lab.app.tick(lab.clock);
        lab.app.wait_for_writes();
    }
    assert_eq!(lab.app.playback(), Playback::Reading);
    assert_eq!(lab.cursor(), cursor, "the cursor did not move");
    assert!(lab.app.reading_position().unwrap() < laptop.cursor());
    let _ = reading_from;
    assert_eq!(
        lab.notes(),
        vec!["Proteins note".to_owned()],
        "the note arrived"
    );
    assert!(
        !lab.said.any("from laptop") && !lab.said.any("laptop's place"),
        "nothing said while reading: {:?}",
        lab.said.all()
    );

    // At the pause, what arrived is said, and the place is only offered.
    lab.act(ActionId::Stop);
    lab.clock += Duration::from_secs(5);
    lab.app.tick(lab.clock);
    assert!(lab.said.any("1 change from laptop"), "{:?}", lab.said.all());
    assert!(lab.said.any("laptop's place:"), "{:?}", lab.said.all());
    assert_ne!(lab.cursor(), laptop.cursor());

    // Go to another computer's place goes there on request.
    lab.act(ActionId::SyncGoToPlace);
    lab.app.dispatch(Command::Choose(0));
    let theirs =
        textweaver_app::text_util::word_start(&laptop.app.session().unwrap().doc, laptop.cursor());
    let first =
        textweaver_app::text_util::first_word_at_or_after(&lab.app.session().unwrap().doc, theirs);
    assert_eq!(lab.cursor(), first);
}

#[test]
fn announcements_follow_the_owner_s_level() {
    let folder = tempfile::tempdir().unwrap();
    let (_da, pa) = copy_of(TEXT, "a.txt");
    let (_db, pb) = copy_of(TEXT, "b.txt");
    let mut laptop = computer(folder.path(), "laptop", |_| {});
    let mut lab = computer(folder.path(), "lab", |s| {
        s.accessibility.interface_announcements = InterfaceAnnouncements::Minimal;
    });
    laptop.open(&pa);
    laptop.add_note("One");
    laptop.settle();
    lab.open(&pb);
    assert_eq!(lab.notes(), vec!["One".to_owned()]);
    // The lab edits while apart; the laptop's later edit wins, and the
    // laptop adds a note on another sentence.
    lab.edit_first_note("Lab edit");
    std::thread::sleep(Duration::from_millis(1100));
    laptop.edit_first_note("Laptop edit");
    laptop.app.dispatch(Command::GoTo(GoTo::Percent(60)));
    laptop.add_note("Another");
    laptop.settle();
    lab.said.clear();
    lab.settle();
    assert!(
        lab.notes().contains(&"Laptop edit".to_owned()),
        "{:?}",
        lab.notes()
    );
    // Minimal: results (a replaced note) are said, routine arrivals not.
    assert!(
        lab.said.any("a note was replaced by laptop"),
        "{:?}",
        lab.said.all()
    );
    assert!(!lab.said.any("changes from laptop"), "{:?}", lab.said.all());
}

#[test]
fn a_damaged_file_is_said_once_a_session() {
    let folder = tempfile::tempdir().unwrap();
    let (_da, pa) = copy_of(TEXT, "a.txt");
    let (_db, pb) = copy_of(TEXT, "b.txt");
    let mut laptop = computer(folder.path(), "laptop", |_| {});
    laptop.open(&pa);
    laptop.add_note("Note");
    laptop.settle();
    // A third computer's record of the same document, cut short.
    let devices = folder.path().join("textweaver-sync").join("devices");
    let laptop_dir = std::fs::read_dir(&devices)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let record = std::fs::read_dir(laptop_dir.join("docs"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
    let other = devices
        .join("0123456789abcdef0123456789abcdef")
        .join("docs");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(
        other.join(record.file_name()),
        b"{\"format\":1,\"sync_id\":\"ab",
    )
    .unwrap();

    let mut lab = computer(folder.path(), "lab", |_| {});
    lab.open(&pb);
    lab.settle();
    lab.act(ActionId::SyncNow);
    lab.settle();
    lab.act(ActionId::SyncNow);
    lab.settle();
    assert_eq!(lab.said.count("damaged file"), 1, "{:?}", lab.said.all());
    assert_eq!(
        lab.notes(),
        vec!["Note".to_owned()],
        "the good files still merge"
    );
    assert!(
        lab.app
            .sync_status_line()
            .starts_with("Sync: 1 damaged file")
    );
}

#[test]
fn a_shared_isbn_is_asked_about_before_notes_are_shared() {
    let folder = tempfile::tempdir().unwrap();
    let a = format!("ISBN 978-0-306-40615-7\n\nChapter one.\n\n{TEXT}");
    let b = "ISBN 978-0-306-40615-7\n\nChapter two. A different text.\n".to_owned();
    let (_da, pa) = copy_of(&a, "one.txt");
    let (_db, pb) = copy_of(&b, "two.txt");
    let mut laptop = computer(folder.path(), "laptop", |_| {});
    laptop.open(&pa);
    laptop.add_note("Chapter one note");
    laptop.settle();

    // No: the two stay separate, and the question is not asked again.
    let mut lab = computer(folder.path(), "lab", |_| {});
    lab.app.open(&pb).unwrap();
    lab.app.wait_for_writes();
    lab.settle();
    let q = lab.app.pending_question().unwrap_or_default();
    assert!(
        q.starts_with("This may be ") && q.contains("from laptop, with 1 note"),
        "{q}"
    );
    lab.app.dispatch(Command::Confirm(Confirm::No));
    lab.settle();
    assert!(lab.notes().is_empty());
    lab.app.open(&pb).unwrap();
    lab.app.wait_for_writes();
    lab.settle();
    assert!(!lab.app.confirmation_pending());

    // Yes, on another pair of computers: the laptop's notes come over.
    let folder = tempfile::tempdir().unwrap();
    let mut laptop = computer(folder.path(), "laptop", |_| {});
    laptop.open(&pa);
    laptop.add_note("Chapter one note");
    laptop.settle();
    let (_dc, pc) = copy_of(&b, "three.txt");
    let mut desk = computer(folder.path(), "desk", |_| {});
    desk.app.open(&pc).unwrap();
    desk.app.wait_for_writes();
    desk.settle();
    assert!(desk.app.confirmation_pending());
    desk.app.dispatch(Command::Confirm(Confirm::Yes));
    desk.settle();
    assert_eq!(desk.notes(), vec!["Chapter one note".to_owned()]);
}

#[test]
fn sync_status_has_its_key_and_says_the_line() {
    let folder = tempfile::tempdir().unwrap();
    let (_da, pa) = copy_of(TEXT, "a.txt");
    let mut laptop = computer(folder.path(), "laptop", |_| {});
    laptop.open(&pa);
    // The one sync command with a default key, asked of the keymap.
    let chords = laptop.app.keymap().chords_for(ActionId::SyncStatus);
    assert!(!chords.is_empty());
    assert_eq!(
        laptop.app.keymap().lookup(&chords[0], Layer::Browse),
        Some(ActionId::SyncStatus)
    );
    laptop.said.clear();
    laptop.act(ActionId::SyncStatus);
    assert!(
        laptop.said.any("Sync: up to date"),
        "{:?}",
        laptop.said.all()
    );
    assert!(laptop.said.any("This computer: laptop."));

    // The folder gone (a USB stick pulled out): saving goes on here.
    let moved = folder.path().with_extension("away");
    std::fs::rename(folder.path(), &moved).unwrap();
    laptop.add_note("Offline note");
    laptop.settle();
    assert_eq!(
        laptop.app.sync_status_line(),
        "Sync: folder missing, saving here"
    );
    std::fs::rename(&moved, folder.path()).unwrap();
    laptop.settle();
    assert_eq!(laptop.app.sync_status_line(), "Sync: up to date");

    // Stop syncing: the setting is off, the folder is left alone.
    laptop.act(ActionId::SyncStop);
    laptop.settle();
    assert!(laptop.said.any("Sync off here."));
    assert!(!laptop.app.settings().sync.enabled);
    assert!(folder.path().join("textweaver-sync").is_dir());
}

#[test]
fn set_up_sync_chooses_the_folder_names_the_computer_and_the_groups() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("Sync");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("reading.txt"), TEXT).unwrap();
    let home = tempfile::tempdir().unwrap();
    let said = Said::default();
    let (speech, _log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        settings: Settings::default(),
        speech,
        paths: Some(Paths::under(home.path())),
        announcer: Box::new(said.clone()),
        ..AppConfig::for_tests()
    });
    assert_eq!(app.sync_status_line(), "Sync: not set up");
    app.open(&folder.join("reading.txt")).unwrap();
    app.dispatch(Command::Action(ActionId::SyncSetup));
    assert!(said.any("Choose the sync folder"), "{:?}", said.all());
    // The file browser opens on the document's folder: choose it. The
    // computer's name is next, "Computer 1".
    app.dispatch(Command::ListKey(ListKey::ChooseHere));
    let prompt = app.prompt_model().expect("the name prompt");
    assert_eq!(prompt.text(), "Computer 1");
    // The computer's own name is refused.
    if let Some(own) = textweaver_app::sync_folder::local_names().first() {
        app.dispatch(Command::Answer(own.clone()));
        assert!(said.any("Name not allowed"), "{:?}", said.all());
    }
    app.dispatch(Command::Answer("laptop".into()));
    // Groups: turn statistics off, then start.
    assert!(said.any("What syncs, as laptop"), "{:?}", said.all());
    app.dispatch(Command::Choose(4));
    assert!(said.any("Statistics: off"), "{:?}", said.all());
    // Start syncing comes after the eleven groups.
    app.dispatch(Command::Choose(11));
    assert!(said.any("Sync on, as laptop."), "{:?}", said.all());
    let s = &app.settings().sync;
    assert!(s.enabled && s.notes && !s.statistics);
    assert_eq!(s.device_name, "laptop");
    assert_eq!(
        s.folder.as_deref().map(|p| p.canonicalize().unwrap()),
        Some(folder.canonicalize().unwrap())
    );
}

/// Every message S4 added: in all six catalogs, and its meaning in the
/// first 40 cells of a Braille line in English (all of it, for the status
/// lines).
#[test]
fn every_sync_message_is_translated_and_fits_a_braille_line() {
    let en = Catalog::english();
    let ids: Vec<&str> = en
        .ids()
        .into_iter()
        .filter(|id| id.starts_with("sync-") || id.starts_with("name-sync-"))
        .collect();
    assert!(ids.len() > 60, "{}", ids.len());
    for tag in ["ar", "de", "es", "fr", "pt"] {
        let c = Catalog::builtin(tag).unwrap();
        let own = c.ids();
        for id in &ids {
            assert!(own.contains(id), "{tag}.ftl lacks {id}");
        }
    }
    use textweaver_app::lexicon::i18n::Arg;
    let sample = |v: &str| -> Arg {
        match v {
            "n" | "pct" | "hours" => Arg::Num(2),
            "title" => Arg::Str("Cells".into()),
            "device" | "name" => Arg::Str("laptop".into()),
            "names" => Arg::Str("lab".into()),
            "key" => Arg::Str("Shift+F5".into()),
            "state" => Arg::Str("on".into()),
            _ => Arg::Str("x".into()),
        }
    };
    for id in &ids {
        let vars = en.variables(id);
        let args: Vec<(&str, Arg)> = vars.iter().map(|v| (v.as_str(), sample(v))).collect();
        let s = en.fmt(id, &args);
        if id.starts_with("sync-status-") || id.starts_with("name-sync-") {
            assert!(s.chars().count() <= 40, "{id} is over 40 cells: {s}");
        }
        // Meaning first: the first 40 cells say what happened, not only
        // decoration; the longest messages are sentences that start with
        // their subject.
        let first: String = s.chars().take(40).collect();
        assert!(
            first.split_whitespace().count() >= 2 || s.chars().count() <= 40,
            "{id}: {s}"
        );
    }
    // The status lines all start with "Sync".
    for id in ids.iter().filter(|i| i.starts_with("sync-status-")) {
        if [
            "sync-status-this-computer",
            "sync-status-no-others",
            "sync-status-others",
            "sync-status-error",
        ]
        .contains(id)
        {
            continue;
        }
        let vars = en.variables(id);
        let args: Vec<(&str, Arg)> = vars.iter().map(|v| (v.as_str(), sample(v))).collect();
        assert!(en.fmt(id, &args).starts_with("Sync: "), "{id}");
    }
}

/// Copies a folder and everything in it, as a sync service would: a file
/// is copied only where the other copy is missing or older, so a stale
/// copy never overwrites the computer's own newer file.
fn copy_dir(from: &Path, to: &Path) {
    let modified = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let target = to.join(e.file_name());
        if e.path().is_dir() {
            copy_dir(&e.path(), &target);
        } else if modified(&target).is_none_or(|t| modified(&e.path()).is_some_and(|s| s > t)) {
            std::fs::copy(e.path(), &target).unwrap();
            // Keep the source's time, as a sync service does, so the copy
            // is never taken for the newer one.
            if let Some(t) = modified(&e.path()) {
                let f = std::fs::File::options().write(true).open(&target).unwrap();
                f.set_modified(t).unwrap();
            }
        }
    }
}

/// Two sync folders joined, as a sync service would once both computers
/// use it: each computer's folder is copied into the other sync folder.
fn exchange(a: &Path, b: &Path) {
    let devices = |p: &Path| p.join("textweaver-sync").join("devices");
    for (from, to) in [(a, b), (b, a)] {
        for e in std::fs::read_dir(devices(from)).unwrap().flatten() {
            copy_dir(&e.path(), &devices(to).join(e.file_name()));
        }
    }
}

/// The sync id this computer's `sync-ids.json` gives `path`.
fn sync_id_of(c: &Computer, path: &Path) -> textweaver_app::sync_folder::SyncId {
    let paths = Paths::under(c.home.path());
    let ids = textweaver_app::store::sync_ids::SyncIds::load(&paths.sync_ids_file());
    ids.get(&textweaver_app::store::DocKey::for_path(path))
        .expect("identified")
        .sync_id
        .parse()
        .unwrap()
}

/// Two computers that opened one document before they ever synced gave it
/// two ids. Once their records meet, the ids fold together: the smallest
/// wins on both, and the notes and places of both are merged under it.
#[test]
fn two_ids_for_one_document_fold_into_the_smallest() {
    use textweaver_app::sync_folder::{DocRecord, FolderView};

    // Each computer syncs to a folder of its own at first.
    let fa = tempfile::tempdir().unwrap();
    let fb = tempfile::tempdir().unwrap();
    let (da, pa) = copy_of(TEXT, "cells.txt");
    let (db, pb) = copy_of(TEXT, "cells-copy.txt");
    let mut laptop = computer(fa.path(), "laptop", |_| {});
    let mut lab = computer(fb.path(), "lab", |_| {});
    laptop.open(&pa);
    laptop.add_note("Laptop note");
    laptop.go_percent(40);
    laptop.settle();
    lab.open(&pb);
    lab.add_note("Lab note");
    lab.go_percent(70);
    lab.settle();
    let (id_laptop, id_lab) = (sync_id_of(&laptop, &pa), sync_id_of(&lab, &pb));
    assert_ne!(id_laptop, id_lab, "two ids before syncing");
    let winner = id_laptop.min(id_lab);

    // The two folders are joined; each computer opens the document again.
    exchange(fa.path(), fb.path());
    laptop.open(&pa);
    lab.open(&pb);
    for _ in 0..3 {
        exchange(fa.path(), fb.path());
        laptop.settle();
        lab.settle();
    }

    assert_eq!(sync_id_of(&laptop, &pa), winner);
    assert_eq!(sync_id_of(&lab, &pb), winner);
    let sorted = |mut v: Vec<String>| {
        v.sort();
        v
    };
    let both = vec!["Lab note".to_owned(), "Laptop note".to_owned()];
    assert_eq!(sorted(laptop.notes()), both);
    assert_eq!(sorted(lab.notes()), both);

    // The folder holds one document, with both computers' places; the
    // record that lost is marked folded into the winner.
    let view = FolderView::read(fa.path()).unwrap();
    assert_eq!(view.docs.len(), 1, "{:?}", view.docs.keys());
    let doc = view.doc(winner).unwrap();
    assert_eq!(doc.places_newest_first().len(), 2);
    let loser = id_laptop.max(id_lab);
    let devices = fa.path().join("textweaver-sync").join("devices");
    let folded: Vec<DocRecord> = std::fs::read_dir(&devices)
        .unwrap()
        .flatten()
        .filter_map(|d| std::fs::read(d.path().join("docs").join(format!("{loser}.json"))).ok())
        .map(|b| DocRecord::from_bytes(&b).unwrap())
        .collect();
    assert_eq!(
        folded.len(),
        1,
        "the computer that lost kept its old record"
    );
    assert_eq!(folded[0].folded_into, Some(winner));

    laptop.quit();
    lab.quit();
    privacy_scan(
        fa.path(),
        &[laptop.home.path(), lab.home.path()],
        &[&pa, &pb, da.path(), db.path()],
    );
}

/// The crash window: a merge publishes what arrived before the app has
/// saved it. If textweaver stops in between, the next session must not
/// take the app's older version for a new edit: the arrival is sent again,
/// and the newer edit from the other computer still wins.
#[test]
fn what_arrived_survives_a_crash_before_it_was_saved() {
    use textweaver_app::store::{DocKey, StateStore};
    use textweaver_app::sync_engine::{
        CycleOutcome, EngineConfig, Groups, Item, Snapshot, SyncEngine, apply_arrivals,
    };
    use textweaver_app::sync_folder::{DocRecord, Identity};
    use textweaver_app::sync_groups::KeySystem;

    let folder = tempfile::tempdir().unwrap();
    let (da, pa) = copy_of(TEXT, "cells.txt");
    let (db, pb) = copy_of(TEXT, "cells.txt");
    let mut laptop = computer(folder.path(), "laptop", |_| {});
    let mut lab = computer(folder.path(), "lab", |_| {});
    laptop.open(&pa);
    laptop.add_note("First words");
    laptop.settle();
    lab.open(&pb);
    assert_eq!(lab.notes(), vec!["First words".to_owned()]);
    laptop.quit();

    // The lab's newer edit, while the laptop is off.
    std::thread::sleep(Duration::from_millis(1100));
    lab.edit_first_note("Lab version");
    lab.settle();

    // The laptop's next session: its engine merges, and textweaver stops
    // before the app applies and saves what arrived.
    let paths = Paths::under(laptop.home.path());
    let config = EngineConfig {
        folder: folder.path().to_owned(),
        device_name: "laptop".into(),
        paths: paths.clone(),
        app_version: "test".into(),
        groups: Groups::ALL,
        system: KeySystem::current(),
    };
    let key = DocKey::for_path(&pa);
    let store = StateStore::new(paths.state_dir());
    let mut state = store.load(&key).unwrap();
    assert_eq!(state.notes[0].note, "First words");
    let snapshot = Snapshot {
        key: key.clone(),
        state: state.clone(),
        publish_place: false,
    };
    let sync_id = sync_id_of(&laptop, &pa);
    let me = Identity::peek(&paths.data_dir).unwrap();
    let mine = || {
        let file = folder
            .path()
            .join("textweaver-sync")
            .join("devices")
            .join(me.to_string())
            .join("docs")
            .join(format!("{sync_id}.json"));
        let r = DocRecord::from_bytes(&std::fs::read(file).unwrap()).unwrap();
        r.notes
            .live()
            .map(|(_, n)| n.note.clone())
            .collect::<Vec<_>>()
    };
    let note_arrival = |o: &CycleOutcome| {
        o.arrivals.iter().find_map(|a| match &a.value {
            Some(Item::Note(n)) => Some(n.note.clone()),
            _ => None,
        })
    };

    let mut first = SyncEngine::new();
    first.configure(Some(config.clone()));
    let o = first.cycle(sync_id, &snapshot, true).unwrap();
    assert_eq!(note_arrival(&o).as_deref(), Some("Lab version"));
    assert_eq!(mine(), ["Lab version"], "published with the arrival");
    drop(first);

    // After the crash: the arrival is sent again, and the laptop's older
    // text is not published as a new edit.
    let mut second = SyncEngine::new();
    second.configure(Some(config.clone()));
    let o = second.cycle(sync_id, &snapshot, true).unwrap();
    assert_eq!(note_arrival(&o).as_deref(), Some("Lab version"));
    assert_eq!(mine(), ["Lab version"]);

    // Applied and saved, it is done: nothing arrives again, and nothing
    // is left waiting in the local file.
    let applied = apply_arrivals(&mut state, &o.arrivals, &mut |_| true);
    assert_eq!(applied.replaced_notes.len(), 1);
    store.save(&key, &state).unwrap();
    let snapshot = Snapshot {
        key,
        state,
        publish_place: false,
    };
    let o = second.cycle(sync_id, &snapshot, true).unwrap();
    assert!(o.arrivals.is_empty(), "{:?}", o.arrivals);
    drop(second);
    let pending = std::fs::read_to_string(
        paths
            .data_dir
            .join(textweaver_app::sync_pending::PENDING_FILE),
    )
    .unwrap();
    assert!(!pending.contains(&sync_id.to_string()), "{pending}");
    lab.settle();
    assert_eq!(lab.notes(), vec!["Lab version".to_owned()]);

    lab.quit();
    privacy_scan(
        folder.path(),
        &[laptop.home.path(), lab.home.path()],
        &[&pa, &pb, da.path(), db.path()],
    );
}
