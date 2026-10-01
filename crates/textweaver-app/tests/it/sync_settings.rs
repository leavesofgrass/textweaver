//! Settings and word lists across computers (the sync wave, S5;
//! ADR-0049): portable settings, profiles, key overrides, the word list,
//! the glossary and pronunciations, and favorite voices. Two app instances,
//! each with its own home, share one temporary sync folder; no real sync
//! service and no audio.
//!
//! After every scenario the sync folder is scanned for anything that names
//! a computer, a user, or a path.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::json;
use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::core::{PunctuationLevel, Rate};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::{Paths, Profiles, Settings, SettingsStore};
use textweaver_app::sync_engine::{EngineConfig, Groups, SyncEngine};
use textweaver_app::sync_groups::{GroupsRequest, KeySystem, apply_groups};
use textweaver_app::testing::recording_service;
use textweaver_app::{App, AppConfig, Command, Playback};

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

/// One computer: its own home folder and app, sharing the sync folder.
struct Computer {
    app: App,
    said: Said,
    home: tempfile::TempDir,
    clock: Instant,
}

/// A name that must never reach the sync folder (`editing.author`, a
/// machine setting).
const AUTHOR: &str = "Ada Example";

const TEXT: &str = "Cells divide by mitosis. Each daughter cell gets a full copy.\n\n\
Meiosis makes gametes. It halves the chromosome count.\n\n\
Proteins fold into shapes. Shape decides what they do.\n";

/// A computer, with files written into its home before the app starts.
fn computer_with(
    folder: &Path,
    name: &str,
    files: &[(&str, &str)],
    tweak: impl FnOnce(&mut Settings),
) -> Computer {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::under(home.path());
    std::fs::create_dir_all(&paths.config_dir).unwrap();
    std::fs::create_dir_all(&paths.data_dir).unwrap();
    for (rel, text) in files {
        let p = home.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
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
        paths: Some(paths),
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

fn computer(folder: &Path, name: &str, tweak: impl FnOnce(&mut Settings)) -> Computer {
    computer_with(folder, name, &[], tweak)
}

impl Computer {
    fn paths(&self) -> Paths {
        Paths::under(self.home.path())
    }

    /// Ticks and waits for the writer until sync has nothing left to do.
    fn settle(&mut self) {
        for _ in 0..6 {
            self.clock += Duration::from_secs(5);
            self.app.tick(self.clock);
            self.app.wait_for_writes();
        }
    }

    fn set(&mut self, path: &str, value: serde_json::Value) {
        self.app.dispatch(Command::SetSetting {
            path: path.into(),
            value,
        });
        self.app.wait_for_writes();
    }

    fn words(&self) -> String {
        std::fs::read_to_string(self.paths().data_dir.join("words.txt")).unwrap_or_default()
    }

    fn write_words(&self, text: &str) {
        std::fs::write(self.paths().data_dir.join("words.txt"), text).unwrap();
    }
}

/// Both computers settle, twice around, so each sees the other's writes.
fn settle(a: &mut Computer, b: &mut Computer) {
    for _ in 0..2 {
        a.settle();
        b.settle();
    }
}

/// Every file in the sync folder and its bytes.
fn folder_files(folder: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = Vec::new();
    walk(folder, &mut files);
    files
        .into_iter()
        .map(|f| {
            let b = std::fs::read(&f).unwrap();
            (f, b)
        })
        .collect()
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

/// Fails when a file in the sync folder names a path, a user, a computer,
/// or the author's name.
fn privacy_scan(folder: &Path, homes: &[&Path]) {
    let mut forbidden: Vec<String> = textweaver_app::sync_folder::local_names()
        .into_iter()
        .filter(|n| n.len() >= 3)
        .collect();
    forbidden.push(AUTHOR.to_owned());
    for h in homes {
        forbidden.push(h.display().to_string());
        forbidden.push(h.display().to_string().replace('\\', "\\\\"));
    }
    let files = folder_files(folder);
    assert!(!files.is_empty(), "the sync folder has files");
    for (f, bytes) in files {
        let text = String::from_utf8_lossy(&bytes);
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

/// The group file of one computer, by its name in `device.json`.
fn group_file_of(folder: &Path, label: &str, file: &str) -> Option<serde_json::Value> {
    let devices = folder.join("textweaver-sync").join("devices");
    for d in std::fs::read_dir(devices).ok()?.flatten() {
        let info: serde_json::Value =
            serde_json::from_slice(&std::fs::read(d.path().join("device.json")).ok()?).ok()?;
        if info["label"] == label {
            let bytes = std::fs::read(d.path().join(file)).ok()?;
            return serde_json::from_slice(&bytes).ok();
        }
    }
    None
}

#[test]
fn two_computers_converge_on_portable_settings_and_keep_their_machine_settings() {
    let folder = tempfile::tempdir().unwrap();
    let mut laptop = computer(folder.path(), "laptop", |s| {
        s.display.wrap_width = 80;
        s.editing.author = AUTHOR.into();
        s.library.folders = vec![PathBuf::from("D:/Books")];
    });
    let mut lab = computer(folder.path(), "lab", |s| {
        s.display.wrap_width = 100;
    });
    settle(&mut laptop, &mut lab);

    laptop.set("speech.rate", json!(320));
    laptop.set("speech.punctuation", json!("all"));
    laptop.set("reading_aids.bionic", json!(true));
    lab.said.clear();
    settle(&mut laptop, &mut lab);

    let s = lab.app.settings();
    assert_eq!(s.speech.rate, Rate::Wpm(320));
    assert_eq!(s.speech.punctuation, PunctuationLevel::All);
    assert!(s.reading_aids.bionic);
    // Machine settings stay each computer's own.
    assert_eq!(s.display.wrap_width, 100);
    assert_eq!(laptop.app.settings().display.wrap_width, 80);
    assert!(s.editing.author.is_empty());
    assert!(s.library.folders.is_empty());
    // One short summary, meaning first.
    assert!(
        lab.said.any("Settings: 3 changes from laptop."),
        "{:?}",
        lab.said.all()
    );
    // The lab saved what arrived.
    let saved = SettingsStore::new(lab.paths()).load().0;
    assert_eq!(saved.speech.rate, Rate::Wpm(320));

    // A later change on the lab goes back to the laptop.
    lab.set("speech.rate", json!(200));
    settle(&mut laptop, &mut lab);
    assert_eq!(laptop.app.settings().speech.rate, Rate::Wpm(200));
    assert_eq!(lab.app.settings().speech.rate, Rate::Wpm(200));
    assert_eq!(
        laptop.app.settings().speech.punctuation,
        PunctuationLevel::All
    );

    privacy_scan(folder.path(), &[laptop.home.path(), lab.home.path()]);
}

#[test]
fn no_echo_between_applying_saving_and_publishing() {
    let folder = tempfile::tempdir().unwrap();
    let mut laptop = computer(folder.path(), "laptop", |_| {});
    let mut lab = computer(folder.path(), "lab", |_| {});
    settle(&mut laptop, &mut lab);
    laptop.set("speech.rate", json!(300));
    laptop.set("display.tab_width", json!(8));
    laptop.write_words("mitochondrion\n");
    settle(&mut laptop, &mut lab);
    assert_eq!(lab.app.settings().speech.rate, Rate::Wpm(300));
    assert!(lab.words().contains("mitochondrion"));

    // Converged: more rounds write nothing and say nothing.
    let before = folder_files(folder.path());
    laptop.said.clear();
    lab.said.clear();
    for _ in 0..3 {
        settle(&mut laptop, &mut lab);
    }
    let after = folder_files(folder.path());
    assert_eq!(
        before.keys().collect::<Vec<_>>(),
        after.keys().collect::<Vec<_>>()
    );
    for (f, bytes) in &before {
        assert!(after.get(f) == Some(bytes), "{} changed", f.display());
    }
    assert!(!laptop.said.any("Settings:"), "{:?}", laptop.said.all());
    assert!(!lab.said.any("Settings:"), "{:?}", lab.said.all());
    // Saving the settings again (a machine setting changed) publishes
    // nothing new either.
    lab.set("display.wrap_width", json!(72));
    settle(&mut laptop, &mut lab);
    let settings_files: Vec<_> = folder_files(folder.path())
        .into_iter()
        .filter(|(f, _)| f.ends_with("settings.json"))
        .collect();
    for (f, bytes) in settings_files {
        assert!(before.get(&f) == Some(&bytes), "{} changed", f.display());
    }
    assert_eq!(laptop.app.settings().display.wrap_width, 0);
    privacy_scan(folder.path(), &[laptop.home.path(), lab.home.path()]);
}

#[test]
fn each_groups_switch_stops_that_group_only() {
    let folder = tempfile::tempdir().unwrap();
    let mut laptop = computer(folder.path(), "laptop", |_| {});
    let mut lab = computer_with(
        folder.path(),
        "lab",
        &[("data/words.txt", "labword\n")],
        |s| {
            s.sync.words = false;
        },
    );
    settle(&mut laptop, &mut lab);
    laptop.write_words("mitochondrion\n");
    laptop.set("speech.rate", json!(310));
    settle(&mut laptop, &mut lab);

    // The settings group still syncs...
    assert_eq!(lab.app.settings().speech.rate, Rate::Wpm(310));
    // ...the word list does not, either way.
    assert!(!lab.words().contains("mitochondrion"), "{}", lab.words());
    assert!(!laptop.words().contains("labword"), "{}", laptop.words());
    assert!(group_file_of(folder.path(), "lab", "words.json").is_none());
    assert!(group_file_of(folder.path(), "laptop", "words.json").is_some());

    // And the other way round: settings off, words on.
    let mut desk = computer_with(
        folder.path(),
        "desk",
        &[("data/words.txt", "deskword\n")],
        |s| s.sync.settings = false,
    );
    settle(&mut laptop, &mut desk);
    assert_eq!(desk.app.settings().speech.rate, Rate::default());
    assert!(desk.words().contains("mitochondrion"), "{}", desk.words());
    assert!(laptop.words().contains("deskword"), "{}", laptop.words());
    assert!(group_file_of(folder.path(), "desk", "settings.json").is_none());
    privacy_scan(
        folder.path(),
        &[laptop.home.path(), lab.home.path(), desk.home.path()],
    );
}

#[test]
fn profile_definitions_sync_and_the_active_profile_does_not() {
    let folder = tempfile::tempdir().unwrap();
    let mut laptop = computer(folder.path(), "laptop", |_| {});
    let mut lab = computer(folder.path(), "lab", |_| {});
    settle(&mut laptop, &mut lab);

    let mut profiles = Profiles::default();
    let mut study = Settings::default();
    study.speech.rate = Rate::Wpm(200);
    profiles.save_current("Study", &study).unwrap();
    assert_eq!(profiles.active.as_deref(), Some("Study"));
    profiles.save(&laptop.paths()).unwrap();
    settle(&mut laptop, &mut lab);

    let theirs = Profiles::load(&lab.paths()).unwrap();
    assert_eq!(theirs.names(), ["Study"]);
    assert_eq!(theirs.profiles["Study"], profiles.profiles["Study"]);
    assert_eq!(
        theirs.active, None,
        "which one is active stays on the laptop"
    );
    assert!(lab.said.any("Settings: 1 change from laptop."));

    // Deleted on the laptop, deleted on the lab.
    profiles.delete("Study").unwrap();
    profiles.save(&laptop.paths()).unwrap();
    settle(&mut laptop, &mut lab);
    assert!(Profiles::load(&lab.paths()).unwrap().names().is_empty());
    privacy_scan(folder.path(), &[laptop.home.path(), lab.home.path()]);
}

#[test]
fn the_word_list_glossary_pronunciations_and_favorite_voices_merge() {
    let folder = tempfile::tempdir().unwrap();
    let mut laptop = computer_with(
        folder.path(),
        "laptop",
        &[
            ("data/words.txt", "mitochondrion\n"),
            ("config/glossary.txt", "# Biology\ncell: the unit of life\n"),
        ],
        |s| {
            s.normalization
                .pronunciations
                .insert("GIF".into(), "jif".into());
            s.speech.favorite_voices = vec!["eci:not-on-the-lab".into()];
        },
    );
    let mut lab = computer_with(
        folder.path(),
        "lab",
        &[("data/words.txt", "ribosome\n")],
        |s| {
            s.normalization
                .pronunciations
                .insert("SQL".into(), "sequel".into());
        },
    );
    settle(&mut laptop, &mut lab);

    // Sets: adding wins, both words everywhere.
    for c in [&laptop, &lab] {
        assert!(c.words().contains("mitochondrion"), "{}", c.words());
        assert!(c.words().contains("ribosome"), "{}", c.words());
    }
    // Pronunciations, newest wins per word: both.
    for c in [&laptop, &lab] {
        let p = &c.app.settings().normalization.pronunciations;
        assert_eq!(p.get("GIF").map(String::as_str), Some("jif"));
        assert_eq!(p.get("SQL").map(String::as_str), Some("sequel"));
    }
    // The glossary entry reached the lab's glossary file.
    let glossary = std::fs::read_to_string(lab.paths().config_dir.join("glossary.txt")).unwrap();
    assert_eq!(glossary, "cell: the unit of life\n");
    // A favorite voice the lab does not have is kept in its list.
    assert_eq!(
        lab.app.settings().speech.favorite_voices,
        ["eci:not-on-the-lab"]
    );

    // A removal is recorded: the word goes on the other computer too.
    laptop.write_words("mitochondrion\n");
    // A glossary entry changed on the lab wins there and here.
    std::fs::write(
        lab.paths().config_dir.join("glossary.txt"),
        "cell: the smallest living unit\n",
    )
    .unwrap();
    settle(&mut laptop, &mut lab);
    assert!(!lab.words().contains("ribosome"), "{}", lab.words());
    let glossary = std::fs::read_to_string(laptop.paths().config_dir.join("glossary.txt")).unwrap();
    assert_eq!(glossary, "# Biology\ncell: the smallest living unit\n");
    privacy_scan(folder.path(), &[laptop.home.path(), lab.home.path()]);
}

#[test]
fn settings_wait_for_the_pause_in_reading() {
    let folder = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let doc = dir.path().join("cells.txt");
    std::fs::write(&doc, TEXT).unwrap();
    let mut laptop = computer(folder.path(), "laptop", |_| {});
    let mut lab = computer(folder.path(), "lab", |_| {});
    lab.app.open(&doc).unwrap();
    settle(&mut laptop, &mut lab);

    lab.app.dispatch(Command::Action(ActionId::ReadFromCursor));
    assert_eq!(lab.app.playback(), Playback::Reading);
    laptop.set("speech.rate", json!(333));
    laptop.settle();
    lab.said.clear();
    for _ in 0..4 {
        lab.clock += Duration::from_secs(5);
        lab.app.tick(lab.clock);
        lab.app.wait_for_writes();
    }
    assert_eq!(lab.app.playback(), Playback::Reading, "reading went on");
    assert_eq!(lab.app.settings().speech.rate, Rate::default());
    assert!(!lab.said.any("Settings:"), "{:?}", lab.said.all());

    lab.app.dispatch(Command::Action(ActionId::Stop));
    lab.settle();
    assert_eq!(lab.app.settings().speech.rate, Rate::Wpm(333));
    assert_eq!(lab.said.count("Settings: 1 change from laptop."), 1);
    privacy_scan(folder.path(), &[laptop.home.path(), lab.home.path()]);
}

/// A computer's engine and paths, driven directly, as a given system.
fn engine(
    folder: &Path,
    home: &Path,
    label: &str,
    system: KeySystem,
    keys: &str,
) -> (SyncEngine, Paths) {
    let paths = Paths::under(home);
    std::fs::create_dir_all(&paths.config_dir).unwrap();
    std::fs::write(paths.keymap_file(), keys).unwrap();
    let mut e = SyncEngine::new();
    e.configure(Some(EngineConfig {
        folder: folder.to_owned(),
        device_name: label.into(),
        paths: paths.clone(),
        app_version: "test".into(),
        groups: Groups::ALL,
        system,
    }));
    (e, paths)
}

fn cycle(e: &mut SyncEngine, paths: &Paths, system: KeySystem) -> usize {
    let mut settings = Settings::default();
    let o = e.groups_cycle(&GroupsRequest {
        settings: Box::new(settings.clone()),
        settings_ms: 0,
        force: true,
    });
    apply_groups(paths, &mut settings, &o.arrivals, system);
    o.status.kept_key_overrides
}

#[test]
fn a_mac_override_is_kept_but_not_applied_on_windows_or_linux_and_the_reverse() {
    let folder = tempfile::tempdir().unwrap();
    let homes: Vec<tempfile::TempDir> = (0..3).map(|_| tempfile::tempdir().unwrap()).collect();
    let (mut mac, mac_paths) = engine(
        folder.path(),
        homes[0].path(),
        "mac",
        KeySystem::MacOs,
        "stop = [\"Cmd+.\"]\n",
    );
    let (mut win, win_paths) = engine(
        folder.path(),
        homes[1].path(),
        "desk",
        KeySystem::Windows,
        "next_sentence = [\"Alt+N\"]\n",
    );
    let (mut linux, linux_paths) =
        engine(folder.path(), homes[2].path(), "lab", KeySystem::Linux, "");
    for _ in 0..2 {
        cycle(&mut mac, &mac_paths, KeySystem::MacOs);
        cycle(&mut win, &win_paths, KeySystem::Windows);
        cycle(&mut linux, &linux_paths, KeySystem::Linux);
    }
    let keys = |p: &Paths| SettingsStore::new(p.clone()).load_keymap().unwrap();

    // The Mac's override: kept by Windows and Linux, not applied.
    assert!(!keys(&win_paths).contains_key("stop"));
    assert!(!keys(&linux_paths).contains_key("stop"));
    assert_eq!(cycle(&mut win, &win_paths, KeySystem::Windows), 1);
    let kept = group_file_of(folder.path(), "desk", "keymap.json").unwrap();
    assert_eq!(
        kept["maps"]["keymap"]["mac:stop"]["value"]["system"], "macos",
        "{kept}"
    );
    // Windows's override: applied on Linux (the same keys), not on the Mac.
    assert_eq!(keys(&linux_paths)["next_sentence"], ["Alt+N"]);
    assert!(!keys(&mac_paths).contains_key("next_sentence"));
    assert_eq!(keys(&mac_paths)["stop"], ["Cmd+."]);
    assert_eq!(cycle(&mut mac, &mac_paths, KeySystem::MacOs), 1);
    privacy_scan(
        folder.path(),
        &[homes[0].path(), homes[1].path(), homes[2].path()],
    );
}
