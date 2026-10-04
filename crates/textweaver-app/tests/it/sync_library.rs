//! The library side of sync (the sync wave, S6; ADR-0049): library details
//! by sync id, "Continue reading" from synced places, every computer's
//! reading statistics, and the old library-folder sidecar as a read
//! source. Two app instances, each with its own home, share one temporary
//! sync folder; no real sync service and no audio.
//!
//! After every scenario the sync folder is scanned for anything that names
//! a computer, a user, or a path.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::core::CharPos;
use textweaver_app::keymap::ActionId;
use textweaver_app::lexicon::i18n::Catalog;
use textweaver_app::store::sync::{ProgressEntry, sidecar_file};
use textweaver_app::store::{DocKey, Paths, ReadingStats, Settings, StatsDelta};
use textweaver_app::synced_library::{CombinedStats, SyncedLibrary};
use textweaver_app::testing::recording_service;
use textweaver_app::text::GoTo;
use textweaver_app::{App, AppConfig, Command, Effect};

#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Said {
    fn all(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
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

/// One computer: its own home and app, sharing the sync folder.
struct Computer {
    app: App,
    said: Said,
    home: tempfile::TempDir,
    clock: Instant,
}

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

impl Computer {
    fn paths(&self) -> Paths {
        Paths::under(self.home.path())
    }

    /// Ticks and waits for the writer until sync has nothing left to do.
    fn settle(&mut self) {
        for _ in 0..8 {
            self.clock += Duration::from_secs(5);
            self.app.tick(self.clock);
            self.app.wait_for_writes();
        }
    }

    fn open(&mut self, path: &Path) {
        self.app.open(path).unwrap();
        self.app.wait_for_writes();
        self.settle();
    }

    fn go_percent(&mut self, pct: u8) {
        self.app.dispatch(Command::GoTo(GoTo::Percent(pct)));
        self.app.save_position().unwrap();
        self.app.wait_for_writes();
        self.settle();
    }

    /// Runs an action and waits for a library scan it starts.
    fn act(&mut self, a: ActionId) -> Vec<Effect> {
        let mut effects = self.app.dispatch(Command::Action(a));
        let deadline = Instant::now() + Duration::from_secs(20);
        while self.app.library_scanning() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
            effects.extend(self.app.tick(Instant::now()));
        }
        effects
    }

    fn quit(&mut self) {
        self.app.shutdown();
    }
}

fn list_items(effects: &[Effect]) -> (String, Vec<String>) {
    effects
        .iter()
        .find_map(|e| match e {
            Effect::ShowList { title, items } => Some((title.clone(), items.clone())),
            _ => None,
        })
        .unwrap_or_default()
}

/// Copies a folder and everything in it, as a sync service would.
fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let target = to.join(e.file_name());
        if e.path().is_dir() {
            copy_dir(&e.path(), &target);
        } else {
            std::fs::copy(e.path(), target).unwrap();
        }
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

const TEXT: &str = "Cells divide by mitosis. Each daughter cell gets a full copy.\n\n\
Meiosis makes gametes. It halves the chromosome count.\n\n\
Proteins fold into shapes. Shape decides what they do.\n\n\
Enzymes speed reactions. They are not used up.\n";

const PAPER: &str = "---\ntitle: Cell Energy\nauthor: Ada Example\ndoi: 10.1000/XYZ\n---\n\n\
# Cell Energy\n\nThe mitochondria makes energy for the cell.\n";

/// A DOI known on one computer finds the document in the library of
/// another computer that never opened it: the library folder is synced
/// between them (its id file too), so the document is recognized by the
/// folder's id and its path inside it.
#[test]
fn a_doi_search_works_on_a_second_home() {
    let folder = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    let lib_a = docs.path().join("laptop").join("Papers");
    std::fs::create_dir_all(&lib_a).unwrap();
    let paper_a = lib_a.join("paper.md");
    std::fs::write(&paper_a, PAPER).unwrap();

    let mut laptop = computer(folder.path(), "laptop", |s| {
        s.library.add_folder(&lib_a);
    });
    laptop.open(&paper_a);
    laptop.quit();

    // The library folder arrives on the lab computer, id file and all.
    let lib_b = docs.path().join("lab").join("Course papers");
    copy_dir(&lib_a, &lib_b);
    let mut lab = computer(folder.path(), "lab", |s| {
        s.library.add_folder(&lib_b);
    });
    let effects = lab.act(ActionId::OpenLibrary);
    let (_, items) = list_items(&effects);
    assert_eq!(items.len(), 1, "{items:?}");
    assert!(
        items[0].starts_with("Cell Energy, by Ada Example"),
        "the synced title and author: {items:?}"
    );
    for q in ["10.1000/xyz", "doi:10.1000/XYZ", "ada example"] {
        let shown = lab
            .app
            .dispatch(Command::FilterList(q.into()))
            .into_iter()
            .find_map(|e| match e {
                Effect::ShowList { items, .. } => Some(items),
                _ => None,
            })
            .unwrap_or_default();
        assert_eq!(shown.len(), 1, "{q}: {shown:?}");
    }

    // The details, as the sync folder has them.
    let settings = lab.app.settings().clone();
    let synced = SyncedLibrary::load(&lab.paths(), &settings).unwrap();
    let id = synced
        .sync_id_for(&lib_b.join("paper.md"), None)
        .expect("recognized by the library folder's id");
    let d = synced.details(id).unwrap();
    assert_eq!(d.doi.as_deref(), Some("10.1000/xyz"));
    assert_eq!(d.author.as_deref(), Some("Ada Example"));
    assert_eq!(d.format.as_deref(), Some("markdown"));
    assert!(d.added_ms.is_some(), "first added travels");
    lab.quit();

    privacy_scan(
        folder.path(),
        &[laptop.home.path(), lab.home.path()],
        &[&paper_a, &lib_a, &lib_b],
    );
}

/// Makes `link` another spelling of the folder `target`: a symbolic link,
/// or on Windows a junction when symbolic links need a privilege the test
/// does not have. False when the platform allows neither.
fn dir_link(target: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
    #[cfg(windows)]
    {
        if std::os::windows::fs::symlink_dir(target, link).is_ok() {
            return true;
        }
        std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .is_ok_and(|o| o.status.success())
            && link.is_dir()
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (target, link);
        false
    }
}

/// A library folder reached through another spelling of its path is still
/// matched by its id: the root cause of the Windows runner failure, where
/// the temporary folder had a short name and the stored library folder its
/// long one. Spelled with a parent step through a folder beside it (on
/// every platform), and through a symbolic link or a junction where the
/// platform allows one, both for the document's path and for the library
/// folder as stored in the settings.
#[test]
fn a_library_folder_spelled_another_way_is_still_matched() {
    let folder = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    let lib_a = docs.path().join("laptop").join("papers");
    std::fs::create_dir_all(&lib_a).unwrap();
    let paper_a = lib_a.join("paper.md");
    std::fs::write(&paper_a, PAPER).unwrap();
    let mut laptop = computer(folder.path(), "laptop", |s| {
        s.library.add_folder(&lib_a);
    });
    laptop.open(&paper_a);
    laptop.quit();

    // The library folder arrives on the lab computer, id file and all.
    let lab_dir = docs.path().join("lab");
    let lib_b = lab_dir.join("papers");
    copy_dir(&lib_a, &lib_b);
    let detour = lab_dir.join("detour");
    std::fs::create_dir_all(&detour).unwrap();
    let link = lab_dir.join("linked");
    let linked = dir_link(&lib_b, &link);
    if !linked {
        eprintln!("no symbolic link or junction here; the parent-step spelling only");
    }
    // Through the folder beside the library, and back up.
    let parent_step = detour.join(std::path::Component::ParentDir).join("papers");

    // The library folder stored as it was added (resolved).
    let lab = computer(folder.path(), "lab", |s| {
        s.library.add_folder(&lib_b);
    });
    let synced = SyncedLibrary::load(&lab.paths(), lab.app.settings()).unwrap();
    let expected = synced
        .sync_id_for(&lib_b.join("paper.md"), None)
        .expect("recognized by the library folder's id");
    let mut spellings = vec![parent_step.join("paper.md")];
    if linked {
        spellings.push(link.join("paper.md"));
    }
    for path in &spellings {
        assert_eq!(
            synced.sync_id_for(path, None),
            Some(expected),
            "{} not matched",
            path.display()
        );
    }

    // The library folder stored in another spelling (typed by hand into
    // the settings), and the document reached by its own path.
    let mut stored = vec![parent_step.clone()];
    if linked {
        stored.push(link.clone());
    }
    for library in stored {
        let mut settings = lab.app.settings().clone();
        settings.library.folders = vec![library.clone()];
        let synced = SyncedLibrary::load(&lab.paths(), &settings).unwrap();
        assert_eq!(
            synced.sync_id_for(&lib_b.join("paper.md"), None),
            Some(expected),
            "library folder {} not matched",
            library.display()
        );
    }
}

/// "Continue reading" lists only documents found on this computer, newest
/// place first, each row meaning first and naming the computer.
#[test]
fn continue_reading_lists_documents_here_newest_first() {
    let folder = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    let lib_a = docs.path().join("laptop-books");
    std::fs::create_dir_all(&lib_a).unwrap();
    let cells_a = lib_a.join("cells.txt");
    std::fs::write(&cells_a, TEXT).unwrap();
    let only_laptop = docs.path().join("elsewhere.txt");
    std::fs::write(&only_laptop, format!("Only here. {TEXT}")).unwrap();
    let notes_lab = docs.path().join("lab-notes.txt");
    std::fs::write(&notes_lab, format!("Lab notes. {TEXT}")).unwrap();

    // The lab reads its own notes first.
    let lib_b = docs.path().join("lab-books");
    let mut lab = computer(folder.path(), "lab", |s| {
        s.library.add_folder(&lib_b);
    });
    lab.open(&notes_lab);
    lab.go_percent(30);
    // Later, on the laptop: a book the lab has too, then one only the
    // laptop has.
    std::thread::sleep(Duration::from_millis(1100));
    let mut laptop = computer(folder.path(), "laptop", |s| {
        s.library.add_folder(&lib_a);
    });
    laptop.open(&cells_a);
    laptop.go_percent(42);
    laptop.open(&only_laptop);
    laptop.go_percent(10);
    laptop.quit();

    copy_dir(&lib_a, &lib_b);
    let effects = lab.act(ActionId::ContinueReading);
    let (title, items) = list_items(&effects);
    assert_eq!(title, "Continue reading");
    assert_eq!(items.len(), 2, "only documents found here: {items:?}");
    assert!(items[0].starts_with("cells, "), "{items:?}");
    assert!(items[0].contains(" percent, laptop, just now"), "{items:?}");
    assert!(items[1].starts_with("lab-notes.txt, "), "{items:?}");
    assert!(items[1].contains(" percent, lab, "), "{items:?}");
    assert!(
        lab.said.any("Continue reading: 2 documents, newest first."),
        "{:?}",
        lab.said.all()
    );
    // Enter opens the document, at the laptop's place (the newest).
    lab.app.dispatch(Command::Choose(0));
    lab.app.wait_for_writes();
    lab.settle();
    let s = lab.app.session().unwrap();
    assert!(s.doc.meta.path.as_deref() == Some(lib_b.join("cells.txt").as_path()));
    assert!(s.cursor > CharPos::ZERO, "resumed at the laptop's place");
    lab.quit();

    privacy_scan(
        folder.path(),
        &[laptop.home.path(), lab.home.path()],
        &[&cells_a, &only_laptop, &notes_lab],
    );
}

/// Reading on two computers sums per document, in the statistics list and
/// in `CombinedStats`, with each computer's share on request.
#[test]
fn statistics_from_two_computers_sum() {
    let folder = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    let a = docs.path().join("biology.txt");
    let b = docs.path().join("Biology copy.txt");
    std::fs::write(&a, TEXT).unwrap();
    std::fs::write(&b, TEXT).unwrap();

    let delta = |path: &Path, seconds: f64, sessions: u32| {
        (0..sessions)
            .map(|i| StatsDelta {
                key: DocKey::for_path(path).0,
                title: "Biology".into(),
                path: Some(path.to_owned()),
                seconds: seconds / f64::from(sessions),
                new_session: true,
                furthest_percent: 40,
                furthest_char: 10,
                at: 1_000 + i64::from(i),
            })
            .collect::<Vec<_>>()
    };
    let mut laptop = computer(folder.path(), "laptop", |_| {});
    ReadingStats::add_to_file(&laptop.paths(), &delta(&a, 120.0, 2)).unwrap();
    laptop.open(&a);
    laptop.quit();

    let mut lab = computer(folder.path(), "lab", |_| {});
    ReadingStats::add_to_file(&lab.paths(), &delta(&b, 60.0, 1)).unwrap();
    lab.open(&b);

    let settings = lab.app.settings().clone();
    let synced = SyncedLibrary::load(&lab.paths(), &settings).unwrap();
    let local = ReadingStats::load(&lab.paths()).unwrap();
    let combined = CombinedStats::build(&local, Some(&synced));
    assert_eq!(combined.documents.len(), 1, "{combined:?}");
    let d = &combined.documents[0];
    assert_eq!(d.seconds, 180.0);
    assert_eq!(d.sessions, 3);
    assert_eq!(combined.total_seconds(), 180.0);
    assert!(d.computers[0].this_computer);
    assert_eq!(d.computers[1].device.as_deref(), Some("laptop"));
    assert_eq!(d.computers[1].sessions, 2);

    // The statistics list says the sums, and each computer on request.
    let (_, items) = list_items(&lab.act(ActionId::ReadingStatistics));
    assert!(
        items[0].starts_with("3 minutes and 0 seconds read in all, in 3 sessions"),
        "{items:?}"
    );
    let row = items
        .iter()
        .position(|i| i.starts_with("Each computer: hidden."))
        .expect("a row to show each computer");
    let shown = lab.app.dispatch(Command::Choose(row));
    let (_, items) = list_items(&shown);
    assert!(
        items.iter().any(|i| i.starts_with("lab: 1 minute")),
        "{items:?}"
    );
    assert!(
        items
            .iter()
            .any(|i| i.starts_with("laptop: 2 minutes") && i.ends_with("2 sessions")),
        "{items:?}"
    );
    assert!(items.iter().any(|i| i.starts_with("Each computer: shown.")));
    lab.quit();

    privacy_scan(
        folder.path(),
        &[laptop.home.path(), lab.home.path()],
        &[&a, &b],
    );
}

/// With sync on, a library folder's old progress file (written by an older
/// textweaver, or converted from star's) is still read, and places go to
/// the sync folder instead of being written there.
#[test]
fn the_old_sidecar_is_still_honored() {
    let folder = tempfile::tempdir().unwrap();
    let lib = tempfile::tempdir().unwrap();
    let doc = lib.path().join("cells.txt");
    std::fs::write(&doc, TEXT).unwrap();
    let meiosis = CharPos(TEXT.find("Meiosis").unwrap());
    let later = textweaver_app::store::now_ts() + 60;
    let theirs = ProgressEntry::new(meiosis, 25, later).to_value();
    let written = serde_json::json!({ "cells.txt": theirs }).to_string();
    let file = sidecar_file(lib.path());
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, &written).unwrap();

    let mut lab = computer(folder.path(), "lab", |s| {
        s.library.add_folder(lib.path());
    });
    lab.open(&doc);
    assert_eq!(lab.app.session().unwrap().cursor, meiosis);
    assert!(
        lab.said.any("percent, from another device."),
        "{:?}",
        lab.said.all()
    );
    lab.go_percent(80);
    lab.quit();
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        written,
        "the old file is read, not written, while sync is on"
    );
}

/// Every message S6 added: in all six catalogs, meaning first, and the
/// short ones within 40 Braille cells.
#[test]
fn every_library_sync_message_is_translated_and_fits_a_braille_line() {
    let en = Catalog::english();
    let ids: Vec<&str> = en
        .ids()
        .into_iter()
        .filter(|id| {
            id.starts_with("continue-")
                || *id == "name-continue-reading"
                || *id == "action-continue-reading"
                || [
                    "stats-others-slow",
                    "stats-untitled",
                    "stats-computer",
                    "stats-by-computer-on",
                    "stats-by-computer-off",
                ]
                .contains(id)
        })
        .collect();
    assert_eq!(ids.len(), 16, "{ids:?}");
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
            "n" | "pct" | "sessions" => Arg::Num(2),
            "title" => Arg::Str("Cells".into()),
            "device" => Arg::Str("laptop".into()),
            "time" => Arg::Str("2 minutes".into()),
            "when" => Arg::Str("2 hours ago".into()),
            _ => Arg::Str("x".into()),
        }
    };
    for tag in ["en", "ar", "de", "es", "fr", "pt"] {
        let c = if tag == "en" {
            Catalog::english()
        } else {
            Catalog::builtin(tag).unwrap()
        };
        for id in &ids {
            let vars = c.variables(id);
            let args: Vec<(&str, Arg)> = vars.iter().map(|v| (v.as_str(), sample(v))).collect();
            let s = c.fmt(id, &args);
            assert!(!s.is_empty() && !s.contains('{'), "{tag} {id}: {s}");
            // The short messages fit one line of a 40-cell display.
            let long = id.starts_with("action-") || *id == "continue-intro";
            if !long {
                // Bidirectional isolation marks around arguments take no
                // cell on the display.
                let cells = s
                    .chars()
                    .filter(|c| !matches!(c, '\u{2068}' | '\u{2069}'))
                    .count();
                assert!(cells <= 40, "{tag} {id} is over 40 cells: {s}");
            }
        }
    }
    // A row starts with the title: the meaning is in the first cells.
    let row = en.fmt(
        "continue-item",
        &[
            ("title", Arg::Str("Cells".into())),
            ("pct", Arg::Num(42)),
            ("device", Arg::Str("laptop".into())),
            ("when", Arg::Str("2 hours ago".into())),
        ],
    );
    assert_eq!(row, "Cells, 42 percent, laptop, 2 hours ago");
}
