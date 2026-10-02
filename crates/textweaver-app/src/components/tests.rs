//! The app's components: the registry's pins, the manager, the first-run
//! list, and the dictation question, through the fake fetcher. Nothing
//! goes to the network, and every test starts with an empty data folder.

// Without dictation, the helpers only the dictation tests use are unused.
#![cfg_attr(not(feature = "dictation"), allow(dead_code))]

use std::io::Read;
use std::sync::Mutex;
use std::time::Duration;

use textweaver_a11y::Announcer;
use textweaver_components::fake::FakeFetcher;
use textweaver_store::Paths;

use super::*;
use crate::command::Command;
use crate::list_model::ListKey;
use crate::{AppConfig, Mode};

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
    fn clear(&self) {
        self.0.lock().unwrap().clear();
    }
}

impl Announcer for Said {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}

const WAIT: Duration = Duration::from_secs(30);

fn app_in(home: &Path, fetcher: Arc<dyn Fetcher>) -> (App, Said) {
    let said = Said::default();
    let mut app = App::new(AppConfig {
        announcer: Box::new(said.clone()),
        paths: Some(Paths::under(home)),
        ..AppConfig::for_tests()
    });
    app.set_component_fetcher(fetcher);
    (app, said)
}

/// The app in edit mode on a short note, ready to dictate.
fn editing(app: &mut App) {
    let mut doc = textweaver_text::Document::from_plain_text("Notes.");
    doc.meta.format = "markdown".into();
    app.open_document(doc, crate::store::DocKey::untitled(3), "notes".into());
    app.enter_edit(Some("Notes.".to_owned()));
    assert!(app.is_editing());
}

/// A made-up dictation model a mirror lists: three small files with
/// Whisper's names, in its own folder.
fn made_up_mirror() -> (FakeFetcher, &'static str, Vec<(String, Vec<u8>)>) {
    let base = "https://mirror.invalid/m";
    let files: Vec<(String, Vec<u8>)> = [
        "encoder_model_int8.onnx",
        "decoder_model_merged_int8.onnx",
        "tokenizer.json",
    ]
    .iter()
    .enumerate()
    .map(|(i, n)| (n.to_string(), vec![b'a' + i as u8; 3000 + i * 10]))
    .collect();
    let mut manifest = String::from(
        "format = 1\n\n[[component]]\nid = \"made-up-dictation\"\ntitle = \"Made-up dictation model\"\nlicense = \"CC0-1.0\"\nfeatures = [\"dictation\"]\nfolder = \"whisper/rten/made-up.en\"\n",
    );
    let mut fake = FakeFetcher::new();
    for (name, bytes) in &files {
        manifest.push_str(&format!(
            "\n[[component.file]]\nname = \"{name}\"\nsize = {}\nsha256 = \"{}\"\n",
            bytes.len(),
            sha256_hex(bytes)
        ));
        fake.insert(format!("{base}/made-up-dictation/{name}"), bytes.clone());
    }
    fake.insert(
        format!("{base}/manifest/components.toml"),
        manifest.into_bytes(),
    );
    (fake, base, files)
}

#[test]
fn the_registry_holds_the_owners_pins() {
    let r = Registry::builtin();
    let ids: Vec<&str> = r.components().iter().map(|c| c.id.as_ref()).collect();
    assert_eq!(
        ids,
        [
            "whisper-base.en",
            "whisper-small.en",
            "ocr-ocrs",
            "ocr-paddle-latin",
            "lexend"
        ]
    );
    for c in r.components() {
        c.check_names().unwrap();
    }
    let base = r.get(WHISPER_BASE_EN).unwrap();
    assert_eq!(base.folder, "whisper/rten/base.en");
    assert_eq!(base.license, "MIT, unconfirmed");
    assert_eq!(base.size(), 23_201_297 + 53_692_803 + 2_405_679);
    assert_eq!(base.size_text(), "79.3 MB");
    assert!(base.files[0].url.ends_with(
        "whisper-base.en/resolve/51eefc0af78b103839eda9e7e4f4186acc6517fe/onnx/encoder_model_int8.onnx"
    ));
    assert_eq!(
        base.files[2].check.expected(),
        "5eb60cec1e77aeeb6869a2bb5a8e01a84c3fe5d072d75369343021fe6f5310d0"
    );
    let small = r.get(WHISPER_SMALL_EN).unwrap();
    assert_eq!(small.size_text(), "251 MB");
    assert_eq!(small.files[2].check, base.files[2].check);
    assert_eq!(small.files[2].size, base.files[2].size);
    assert!(
        small.files[1]
            .url
            .contains("482fb8ba081b6e906f92efe103622316b2a0cc69")
    );
    // With nothing installed, nothing is said to be.
    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(r.installed_count(tmp.path()), (0, 5));
}

#[test]
fn the_fixture_manifest_adds_made_up_components_only() {
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/w8a-d/mirror/manifest/components.toml"),
    )
    .unwrap();
    let mut r = Registry::builtin();
    let lexend = r.get("lexend").unwrap().clone();
    let refused = r.add_manifest(&text).unwrap();
    assert_eq!(refused.len(), 1);
    assert_eq!(refused[0].0, "lexend");
    assert_eq!(r.get("lexend"), Some(&lexend), "built-in pins kept");
    assert!(r.get("example-voice-pack").is_some());
    assert_eq!(
        r.get("example-ocr-language").unwrap().folder,
        "models/ocr/example-language"
    );
    assert!(!r.is_builtin("example-voice-pack"));
}

/// No network request is made before a yes: the manager, Verify, the
/// first-run list, and a declined dictation question fetch nothing.
#[cfg(feature = "dictation")]
#[test]
fn nothing_is_fetched_before_a_yes() {
    let tmp = tempfile::tempdir().unwrap();
    let fake = Arc::new(FakeFetcher::new());
    let (mut app, said) = app_in(tmp.path(), fake.clone());
    app.dispatch(Command::Action(ActionId::ManageComponents));
    assert!(said.any("5 optional components."), "{:?}", said.all());
    let model = app.list_model().unwrap().clone();
    assert!(
        model.items[0].starts_with("Whisper base.en, English dictation: not installed, 79.3 MB"),
        "{:?}",
        model.items
    );
    app.dispatch(Command::Choose(0));
    app.dispatch(Command::Choose(1));
    assert!(app.wait_for_components(WAIT));
    assert!(said.any("do not check out"), "{:?}", said.all());
    app.offer_components_on_first_run();
    app.tick(std::time::Instant::now());
    assert!(
        said.any("Optional extras, none chosen."),
        "{:?}",
        said.all()
    );
    app.dispatch(Command::Cancel);
    editing(&mut app);
    said.clear();
    app.dispatch(Command::Action(ActionId::Dictate));
    assert!(
        said.any(
            "Dictation needs the Whisper model, 79.3 MB, license MIT, unconfirmed. Download it now? y or n"
        ),
        "{:?}",
        said.all()
    );
    app.dispatch(Command::Confirm(Confirm::No));
    assert!(said.any("No model, so no dictation for now."));
    // Not asked again this session.
    said.clear();
    app.dispatch(Command::Action(ActionId::Dictate));
    assert!(!app.confirmation_pending());
    assert!(
        said.any("No model, so no dictation for now."),
        "{:?}",
        said.all()
    );
    assert_eq!(fake.request_count(), 0, "{:?}", fake.requests());
}

/// A yes downloads; a file that does not match its pin is refused, in
/// words, and nothing is installed.
#[cfg(feature = "dictation")]
#[test]
fn a_bad_file_is_said_and_nothing_is_installed() {
    let tmp = tempfile::tempdir().unwrap();
    let mut fake = FakeFetcher::new();
    for f in whisper_base_en().files.iter() {
        fake.insert(f.url.to_string(), vec![0u8; 64]);
    }
    let fake = Arc::new(fake);
    let (mut app, said) = app_in(tmp.path(), fake.clone());
    editing(&mut app);
    app.dispatch(Command::Action(ActionId::Dictate));
    app.dispatch(Command::Confirm(Confirm::Yes));
    assert!(
        said.any("Downloading. Escape stops it."),
        "{:?}",
        said.all()
    );
    assert!(app.wait_for_components(WAIT));
    assert!(
        said.any("Not installed: wrong file size."),
        "{:?}",
        said.all()
    );
    assert!(fake.request_count() >= 1);
    let (c, dir) = app.component_and_dir(WHISPER_BASE_EN).unwrap();
    assert_eq!(c.status_in(&dir), Status::NotInstalled);
}

/// Hands out each file a few bytes at a time, slowly, so Escape lands
/// part way.
struct Slow(FakeFetcher, Arc<AtomicU64>);

struct SlowRead(Box<dyn Read + Send>, Arc<AtomicU64>);

impl Read for SlowRead {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        std::thread::sleep(Duration::from_millis(2));
        let n = buf.len().min(16);
        let got = self.0.read(&mut buf[..n])?;
        self.1.fetch_add(got as u64, Ordering::SeqCst);
        Ok(got)
    }
}

impl Fetcher for Slow {
    fn open(&self, address: &str, from: u64) -> Result<Fetched, String> {
        let f = self.0.open(address, from)?;
        Ok(Fetched {
            reader: Box::new(SlowRead(f.reader, self.1.clone())),
            start: f.start,
        })
    }
}

/// Escape stops a download; the next one goes on from where it stopped,
/// and the feature finds the model in the same session (no restart).
#[cfg(feature = "dictation")]
#[test]
fn escape_cancels_and_the_next_download_resumes_and_is_seen_at_once() {
    let tmp = tempfile::tempdir().unwrap();
    let (fake, base, files) = made_up_mirror();
    let read = Arc::new(AtomicU64::new(0));
    let (mut app, said) = app_in(tmp.path(), Arc::new(Slow(fake, read.clone())));
    let _ = app.update_settings(|s| {
        s.components.mirror = base.to_owned();
        s.dictation.model = "made-up-dictation".into();
    });
    editing(&mut app);
    app.dispatch(Command::Action(ActionId::Dictate));
    assert!(
        said.any("Dictation needs the Whisper model, 9 KB"),
        "{:?}",
        said.all()
    );
    read.store(0, Ordering::SeqCst);
    app.dispatch(Command::Confirm(Confirm::Yes));
    // Escape once some of the model's bytes arrived (the mirror's list,
    // read when the question was asked, does not count).
    let start = std::time::Instant::now();
    while read.load(Ordering::SeqCst) < 64 && start.elapsed() < WAIT {
        std::thread::sleep(Duration::from_millis(5));
    }
    app.components_tick();
    app.dispatch(Command::Cancel);
    assert!(app.wait_for_components(WAIT));
    assert!(
        said.any("Download stopped; it resumes later."),
        "{:?}",
        said.all()
    );
    let (c, dir) = app.component_and_dir("made-up-dictation").unwrap();
    assert_ne!(c.status_in(&dir), Status::Installed);
    let part = dir
        .with_file_name("made-up.en.partial")
        .join("encoder_model_int8.onnx.part");
    let kept = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    let listing: Vec<(String, u64)> = std::fs::read_dir(part.parent().unwrap())
        .map(|d| {
            d.flatten()
                .map(|e| {
                    (
                        e.file_name().to_string_lossy().into_owned(),
                        e.metadata().map(|m| m.len()).unwrap_or(0),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    assert!(
        kept > 0,
        "{} holds {kept} bytes; read {}; {listing:?}; {:?}",
        part.display(),
        read.load(Ordering::SeqCst),
        said.all()
    );
    // The same session, a fast source: it goes on from the part file.
    let (fake, _, _) = made_up_mirror();
    let fake = Arc::new(fake);
    app.set_component_fetcher(fake.clone());
    said.clear();
    app.dispatch(Command::Action(ActionId::Dictate));
    app.dispatch(Command::Confirm(Confirm::Yes));
    assert!(app.wait_for_components(WAIT));
    assert!(
        said.any("Ready: Made-up dictation model."),
        "{:?}",
        said.all()
    );
    assert!(
        fake.requests().iter().any(|(_, from)| *from > 0),
        "{:?}",
        fake.requests()
    );
    assert_eq!(c.status_in(&dir), Status::Installed);
    for (name, bytes) in &files {
        assert_eq!(&std::fs::read(dir.join(name)).unwrap(), bytes);
    }
    // Seen at once: dictation started with the model as the download
    // finished, with no restart; Dictate now stops it, and the model is
    // never asked for again.
    said.clear();
    app.dispatch(Command::Action(ActionId::Dictate));
    assert!(!app.confirmation_pending(), "{:?}", said.all());
    assert!(!said.any("needs the Whisper model"), "{:?}", said.all());
    assert!(
        said.any("Finishing dictation.") || said.any("Dictation failed: "),
        "{:?}",
        said.all()
    );
}

#[test]
fn the_first_run_list_chooses_nothing_and_shows_once() {
    let tmp = tempfile::tempdir().unwrap();
    let fake = Arc::new(FakeFetcher::new());
    let (mut app, said) = app_in(tmp.path(), fake.clone());
    app.offer_components_on_first_run();
    app.tick(std::time::Instant::now());
    let model = app.list_model().unwrap().clone();
    assert_eq!(model.title, "Optional components");
    assert!(
        model.items[..5]
            .iter()
            .all(|i| i.starts_with("Not chosen: "))
    );
    assert!(
        model.items[0]
            .starts_with("Not chosen: Whisper base.en, English dictation, for dictation, 79.3 MB"),
        "{:?}",
        model.items
    );
    assert_eq!(model.items[5], "Download the chosen ones");
    assert_eq!(model.items[6], "Skip for now");
    assert!(app.settings().components.chooser_shown);
    // Nothing chosen: Download downloads nothing.
    app.dispatch(Command::Choose(5));
    assert!(said.any("Nothing chosen, nothing downloaded."));
    assert_eq!(fake.request_count(), 0);
    // Shown once.
    app.offer_components_on_first_run();
    app.tick(std::time::Instant::now());
    assert!(app.list_model().is_none(), "shown once");
    assert_eq!(app.mode(), Mode::Browse);
}

#[test]
fn space_marks_rows_in_the_first_run_list_in_words() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut app, said) = app_in(tmp.path(), Arc::new(FakeFetcher::new()));
    app.offer_components_on_first_run();
    app.tick(std::time::Instant::now());
    app.dispatch(Command::ListKey(ListKey::Char(' ')));
    assert!(said.any("Chosen"), "{:?}", said.all());
    let model = app.list_model().unwrap().clone();
    assert!(
        model.items[0].starts_with("Chosen: Whisper base.en"),
        "{:?}",
        model.items
    );
    app.dispatch(Command::ListKey(ListKey::Char(' ')));
    let model = app.list_model().unwrap().clone();
    assert!(model.items[0].starts_with("Not chosen: "));
}

#[test]
fn remove_asks_and_takes_only_the_components_files() {
    let tmp = tempfile::tempdir().unwrap();
    let (fake, base, _) = made_up_mirror();
    let (mut app, said) = app_in(tmp.path(), Arc::new(fake));
    let _ = app.update_settings(|s| s.components.mirror = base.to_owned());
    app.dispatch(Command::Action(ActionId::ManageComponents));
    let model = app.list_model().unwrap().clone();
    let row = model
        .items
        .iter()
        .position(|i| i.starts_with("Made-up dictation model: not installed"))
        .unwrap();
    app.dispatch(Command::Choose(row));
    app.dispatch(Command::Choose(0));
    assert!(
        said.any("Download Made-up dictation model, 9 KB, license CC0-1.0? y or n"),
        "{:?}",
        said.all()
    );
    app.dispatch(Command::Confirm(Confirm::Yes));
    assert!(app.wait_for_components(WAIT));
    let before = app.component_changes();
    let (c, dir) = app.component_and_dir("made-up-dictation").unwrap();
    assert_eq!(c.status_in(&dir), Status::Installed);
    std::fs::write(dir.join("mine.txt"), b"keep").unwrap();
    app.dispatch(Command::Action(ActionId::ManageComponents));
    app.dispatch(Command::ListKey(ListKey::Delete));
    // Row 0 is base.en, not installed: says so.
    assert!(
        said.any("Not installed: Whisper base.en"),
        "{:?}",
        said.all()
    );
    app.dispatch(Command::Action(ActionId::ManageComponents));
    app.dispatch(Command::Choose(row));
    app.dispatch(Command::Choose(2));
    assert!(said.any("Remove Made-up dictation model? y or n"));
    app.dispatch(Command::Confirm(Confirm::Yes));
    assert!(said.any("Removed: Made-up dictation model."));
    assert_eq!(c.status_in(&dir), Status::NotInstalled);
    assert!(dir.join("mine.txt").is_file());
    assert!(app.component_changes() > before);
}
