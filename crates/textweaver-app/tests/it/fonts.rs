//! Lexend on first choice (W7l): choosing Lexend when it is neither
//! downloaded nor installed asks first, naming its size and license; a
//! yes downloads the pinned files, checked by hash, into the data folder;
//! a no keeps the choice. Nothing here downloads: a fake fetcher hands
//! out `fixtures/w7l`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::fonts::downloaded::{Fetched, Fetcher, LEXEND};
use textweaver_app::lexicon::i18n::Catalog;
use textweaver_app::store::Paths;
use textweaver_app::testing::recording_service;
use textweaver_app::{App, AppConfig, Command, Confirm};

#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Said {
    fn all(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
    fn any(&self, needle: &str) -> bool {
        self.all().iter().any(|s| s.contains(needle))
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

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/w7l")
            .join(name),
    )
    .unwrap()
}

/// Hands out the fixtures for Lexend's pinned URLs and counts requests.
struct Fake {
    files: HashMap<String, Vec<u8>>,
    asked: AtomicUsize,
}

impl Fake {
    fn lexend() -> Fake {
        Fake {
            files: LEXEND
                .files
                .iter()
                .map(|f| (f.url.to_owned(), fixture(f.file_name)))
                .collect(),
            asked: AtomicUsize::new(0),
        }
    }
}

impl Fetcher for Fake {
    fn open(&self, url: &str, _from: u64) -> Result<Fetched, String> {
        self.asked.fetch_add(1, Ordering::SeqCst);
        let data = self
            .files
            .get(url)
            .cloned()
            .ok_or_else(|| "not found".to_owned())?;
        Ok(Fetched {
            reader: Box::new(std::io::Cursor::new(data)),
            start: 0,
        })
    }
}

const QUESTION: &str = "Download the Lexend font, 206 KB, SIL Open Font License? y or n";

fn app(home: Option<&Path>, fake: Arc<Fake>, installed: bool) -> (App, Said) {
    let said = Said::default();
    let (speech, _log) = recording_service().unwrap();
    let mut app = App::new(AppConfig {
        speech,
        announcer: Box::new(said.clone()),
        paths: home.map(Paths::under),
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    app.set_font_fetcher(fake);
    // Whatever this computer has installed, the tests decide.
    app.set_installed_fonts_check(Arc::new(move |name: &str| {
        installed && name.eq_ignore_ascii_case("Lexend")
    }));
    (app, said)
}

fn choose(app: &mut App, family: &str) {
    app.dispatch(Command::SetSetting {
        path: "reading_aids.font.family".into(),
        value: serde_json::Value::String(family.into()),
    });
}

fn fonts_dir(home: &Path) -> PathBuf {
    textweaver_app::fonts_folder(&Paths::under(home))
}

#[test]
fn choosing_lexend_asks_then_downloads_it_into_the_data_folder() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let fake = Arc::new(Fake::lexend());
    let (mut app, said) = app(Some(&home), Arc::clone(&fake), false);
    choose(&mut app, "lexend");
    // The change is said first, then the question with size and license.
    let all = said.all();
    let font = all.iter().position(|s| s.starts_with("Font, Lexend."));
    let asked = all.iter().position(|s| s == QUESTION);
    assert!(font.is_some() && asked > font, "{all:?}");
    assert!(app.confirmation_pending());
    assert_eq!(fake.asked.load(Ordering::SeqCst), 0, "nothing before a yes");
    // Any other key asks again.
    said.clear();
    app.dispatch(Command::Confirm(Confirm::Repeat));
    assert_eq!(said.all(), [QUESTION]);
    app.dispatch(Command::Confirm(Confirm::Yes));
    assert!(said.any("Downloading Lexend."), "{:?}", said.all());
    assert!(app.wait_for_font_download(Duration::from_secs(20)));
    assert!(said.any("Lexend downloaded and ready."), "{:?}", said.all());
    assert_eq!(app.font_downloads(), 1);
    let fonts = fonts_dir(&home);
    assert_eq!(app.fonts_folder().as_deref(), Some(fonts.as_path()));
    assert!(LEXEND.load_from(&fonts).is_some());
    assert_eq!(
        std::fs::read_to_string(fonts.join("lexend").join("OFL.txt")).unwrap(),
        LEXEND.license_text
    );
    // (The writers' lookup through the data folder is tested in the
    // writers' own tests: the folder is set for the whole process, and
    // other tests here start apps with other data folders.)
    // Chosen again: it is there, so nothing is asked.
    said.clear();
    choose(&mut app, "sans");
    choose(&mut app, "lexend");
    app.tick(Instant::now());
    assert!(!app.confirmation_pending());
    assert!(!said.any("Download the"), "{:?}", said.all());
    assert_eq!(fake.asked.load(Ordering::SeqCst), 2);
}

#[test]
fn no_keeps_the_choice_and_asks_again_next_time() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let fake = Arc::new(Fake::lexend());
    let (mut app, said) = app(Some(&home), Arc::clone(&fake), false);
    choose(&mut app, "lexend");
    app.dispatch(Command::Confirm(Confirm::No));
    assert!(said.any("Not downloaded. Another font is used."));
    assert_eq!(app.settings().reading_aids.font.family, "lexend");
    assert!(!fonts_dir(&home).join("lexend").exists());
    assert_eq!(fake.asked.load(Ordering::SeqCst), 0);
    said.clear();
    choose(&mut app, "lexend");
    assert!(said.any(QUESTION));
    app.dispatch(Command::Confirm(Confirm::No));
}

#[test]
fn a_file_that_does_not_match_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let mut fake = Fake::lexend();
    let url = LEXEND.files[0].url.to_owned();
    fake.files.get_mut(&url).unwrap()[500] ^= 0xff;
    let (mut app, said) = app(Some(&home), Arc::new(fake), false);
    choose(&mut app, "lexend");
    app.dispatch(Command::Confirm(Confirm::Yes));
    assert!(app.wait_for_font_download(Duration::from_secs(20)));
    assert!(
        said.any("Lexend not downloaded: Lexend-Regular.ttf does not match its published hash"),
        "{:?}",
        said.all()
    );
    assert_eq!(app.font_downloads(), 0);
    assert!(!fonts_dir(&home).join("lexend").exists());
}

#[test]
fn nothing_is_asked_when_lexend_is_installed_or_another_font_is_chosen() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let fake = Arc::new(Fake::lexend());
    let (mut app, said) = app(Some(&home), Arc::clone(&fake), true);
    choose(&mut app, "lexend");
    app.tick(Instant::now());
    assert!(!app.confirmation_pending(), "{:?}", said.all());
    let (mut app, said) = self::app(Some(&home), fake, false);
    choose(&mut app, "opendyslexic");
    choose(&mut app, "Verdana");
    app.tick(Instant::now());
    assert!(!app.confirmation_pending(), "{:?}", said.all());
}

#[test]
fn without_a_data_folder_it_says_so() {
    let (mut app, said) = app(None, Arc::new(Fake::lexend()), false);
    choose(&mut app, "lexend");
    assert!(
        said.any("No data folder to keep Lexend in."),
        "{:?}",
        said.all()
    );
    assert!(!app.confirmation_pending());
}

#[test]
fn a_setting_changed_elsewhere_asks_on_the_next_tick() {
    // The GUI's settings dialog changes settings with `set_setting` and
    // says the change itself; the question follows on the next tick.
    let dir = tempfile::tempdir().unwrap();
    let (mut app, said) = app(
        Some(&dir.path().join("home")),
        Arc::new(Fake::lexend()),
        false,
    );
    let changed = app
        .set_setting(
            "reading_aids.font.family",
            serde_json::Value::String("lexend".into()),
        )
        .unwrap();
    assert_eq!(changed, "Font, Lexend.");
    assert!(!said.any(QUESTION));
    app.tick(Instant::now());
    assert!(said.any(QUESTION), "{:?}", said.all());
    assert!(app.confirmation_pending());
    app.dispatch(Command::Confirm(Confirm::No));
    // Declined: remembered for the session, so a change made elsewhere
    // does not ask again (W9a-d) ...
    said.clear();
    for family in ["sans", "lexend"] {
        app.set_setting(
            "reading_aids.font.family",
            serde_json::Value::String(family.into()),
        )
        .unwrap();
        app.tick(Instant::now());
    }
    assert!(!said.any(QUESTION), "{:?}", said.all());
    assert!(!app.confirmation_pending());
    // ... while choosing it in Settings is the way back.
    choose(&mut app, "lexend");
    assert!(said.any(QUESTION), "{:?}", said.all());
    app.dispatch(Command::Confirm(Confirm::No));
}

#[test]
fn the_messages_are_in_six_languages_and_short() {
    let ids = [
        "font-download-question",
        "font-downloading",
        "font-downloaded",
        "font-download-failed",
        "font-download-declined",
        "font-download-busy",
        "font-download-no-folder",
        "font-download-not-in-build",
        "gui-font-to-download",
        "gui-font-downloaded",
    ];
    let english = Catalog::english();
    for tag in ["en", "es", "fr", "de", "pt", "ar"] {
        let (c, _) = Catalog::for_language(tag, None);
        for id in ids {
            let a = textweaver_app::lexicon::args![
                "font" => "Lexend",
                "family" => "Lexend",
                "kb" => 206u64,
                "licence" => "SIL Open Font License",
                "error" => "x"
            ];
            let text = c.fmt(id, &a);
            assert_ne!(text, id, "{tag} lacks {id}");
            if tag != "en" {
                assert_ne!(text, english.fmt(id, &a), "{tag} {id} is English");
            }
            // The question names the size and license, so it is longer
            // than a line of 40 cells; its meaning comes first.
            if id != "font-download-question" {
                assert!(
                    text.chars().count() <= 40,
                    "{tag} {id}: {text} ({} cells)",
                    text.chars().count()
                );
            } else {
                assert!(text.contains("206") && text.contains("SIL Open Font License"));
            }
        }
    }
}
