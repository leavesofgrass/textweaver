//! Updates in the app, through a fake release server and fake packages.
//! Nothing goes to the network, and nothing is installed outside the
//! temporary folder.

use std::sync::Mutex;

use textweaver_a11y::{Announcer, Priority};
use textweaver_components::fake::FakeFetcher;
use textweaver_store::Paths;

use super::*;
use crate::AppConfig;
use crate::command::Command;

/// What the app announced.
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

const LIST: &str = "https://example.invalid/releases";
const NEWER: &str = "99.0.0";
const WAIT: Duration = Duration::from_secs(30);

/// A fake release server with one newer release whose package is
/// `served`, while its checksum is the checksum of `real`.
fn server(real: &[u8], served: &[u8]) -> FakeFetcher {
    let name = Target::this().unwrap().package_name(NEWER);
    let pkg = format!("https://example.invalid/{NEWER}/{name}");
    let sums = format!("https://example.invalid/{NEWER}/SHA256SUMS.txt");
    let list = format!(
        r#"[{{"tag_name":"v{NEWER}","prerelease":true,"assets":[
          {{"name":"{name}","size":{},"browser_download_url":"{pkg}"}},
          {{"name":"SHA256SUMS.txt","size":100,"browser_download_url":"{sums}"}}]}}]"#,
        real.len()
    );
    FakeFetcher::new()
        .with(LIST, list)
        .with(pkg, served.to_vec())
        .with(
            sums,
            format!("{}  {name}\n", textweaver_components::sha256_hex(real)),
        )
}

fn app_with(home: &std::path::Path, fake: FakeFetcher) -> (App, Said, Arc<FakeFetcher>) {
    let said = Said::default();
    let mut app = App::new(AppConfig {
        announcer: Box::new(said.clone()),
        paths: Some(Paths::under(home)),
        ..AppConfig::for_tests()
    });
    let install = home.join("install");
    std::fs::create_dir_all(&install).unwrap();
    std::fs::write(install.join("NOTICE"), b"old").unwrap();
    let fake = Arc::new(fake);
    app.set_update_source(fake.clone(), LIST, Some(install));
    (app, said, fake)
}

fn tick_until_asked(app: &mut App) {
    assert!(app.wait_for_updates(WAIT));
    app.tick(Instant::now());
}

#[test]
fn help_check_for_updates_asks_with_the_version_and_size() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut app, said, _) = app_with(tmp.path(), server(b"pkg", b"pkg"));
    app.dispatch(Command::Action(ActionId::CheckForUpdates));
    assert!(said.any("Checking for updates."));
    tick_until_asked(&mut app);
    let q = format!("Update available: textweaver {NEWER}, 0 KB. Download it? y or n");
    assert!(said.any(&q), "{:?}", said.all());
    assert!(app.confirmation_pending());
    app.dispatch(Command::Confirm(Confirm::No));
    assert!(said.any("Not downloaded. A newer release will be offered."));
    assert_eq!(app.settings().updates.declined, NEWER);
}

#[test]
fn the_question_waits_until_reading_stops() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut app, said, _) = app_with(tmp.path(), server(b"pkg", b"pkg"));
    app.dispatch(Command::Action(ActionId::CheckForUpdates));
    app.playback = Playback::Reading;
    tick_until_asked(&mut app);
    assert!(!app.confirmation_pending(), "not while reading");
    assert!(!said.any("Update available"));
    app.playback = Playback::Idle;
    app.tick(Instant::now());
    assert!(app.confirmation_pending());
}

#[test]
fn the_automatic_check_is_quiet_without_news() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut app, said, fake) = app_with(tmp.path(), server(b"pkg", b"pkg"));
    // Off: nothing is read.
    app.updates_started(true);
    app.tick(Instant::now());
    assert_eq!(fake.request_count(), 0);
    // On, with this release declined: read once, nothing said.
    let _ = app.update_settings(|s| {
        s.updates.check = true;
        s.updates.asked = true;
        s.updates.declined = NEWER.into();
    });
    let before = said.all().len();
    app.tick(Instant::now());
    tick_until_asked(&mut app);
    assert_eq!(fake.request_count(), 1);
    assert_eq!(said.all().len(), before, "{:?}", said.all());
    assert!(!app.confirmation_pending());
    assert!(app.settings().updates.last_check > 0);
    // Once a day: not again today.
    app.tick(Instant::now());
    assert!(app.wait_for_updates(WAIT));
    assert_eq!(fake.request_count(), 1);
}

#[test]
fn a_tampered_package_is_refused_and_nothing_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut app, said, _) = app_with(tmp.path(), server(b"real package", b"evil package"));
    app.dispatch(Command::Action(ActionId::CheckForUpdates));
    tick_until_asked(&mut app);
    app.dispatch(Command::Confirm(Confirm::Yes));
    assert!(said.any("Downloading the update"), "{:?}", said.all());
    assert!(app.wait_for_updates(WAIT));
    assert!(
        said.any("Update refused: checksum does not match. Nothing changed."),
        "{:?}",
        said.all()
    );
    let install = tmp.path().join("install");
    let names: Vec<_> = std::fs::read_dir(&install).unwrap().flatten().collect();
    assert_eq!(names.len(), 1, "only the old NOTICE: {names:?}");
    assert_eq!(std::fs::read(install.join("NOTICE")).unwrap(), b"old");
}

#[test]
fn a_copy_never_asked_asks_once() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut app, said, fake) = app_with(tmp.path(), server(b"pkg", b"pkg"));
    app.updates_started(false);
    app.tick(Instant::now());
    assert!(said.any("Check for updates automatically, once a day? y or n"));
    app.dispatch(Command::Confirm(Confirm::No));
    assert!(said.any("Updates: not checked."));
    let u = &app.settings().updates;
    assert!(u.asked && !u.check);
    assert_eq!(fake.request_count(), 0, "a no reads nothing");
    // Asked once.
    app.updates_started(false);
    app.tick(Instant::now());
    assert!(!app.confirmation_pending());
}
