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

/// A recognizer that records nothing and never opens a model or a
/// microphone, for tests that only need dictation to start and stop.
#[cfg(feature = "dictation")]
#[derive(Default)]
struct Quiet(Option<textweaver_dictation::DictationState>);

#[cfg(feature = "dictation")]
impl textweaver_dictation::Dictation for Quiet {
    fn name(&self) -> &str {
        "quiet"
    }
    fn start(
        &mut self,
        _input: textweaver_dictation::DictationInput,
    ) -> Result<(), textweaver_dictation::DictationError> {
        self.0 = Some(textweaver_dictation::DictationState::Recording);
        Ok(())
    }
    fn stop(&mut self) -> Result<(), textweaver_dictation::DictationError> {
        self.0 = Some(textweaver_dictation::DictationState::Transcribing);
        Ok(())
    }
    fn cancel(&mut self) {
        self.0 = None;
    }
    fn poll(&mut self) -> Vec<textweaver_dictation::DictationEvent> {
        Vec::new()
    }
    fn state(&self) -> textweaver_dictation::DictationState {
        self.0.unwrap_or(textweaver_dictation::DictationState::Idle)
    }
    fn shutdown(&mut self, _wait: Duration) -> bool {
        self.0 = None;
        true
    }
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
    // The helper programs with a build for this computer come last.
    let helpers: Vec<&str> = Helper::ALL
        .iter()
        .filter(|h| h.component().is_some())
        .map(|h| h.id())
        .collect();
    let mut expected = vec![
        "whisper-base.en",
        "whisper-small.en",
        "ocr-ocrs",
        "ocr-paddle-latin",
        "lexend",
    ];
    expected.extend(&helpers);
    assert_eq!(ids, expected);
    if cfg!(windows) {
        assert_eq!(helpers, ["ffmpeg", "liblouis", "pandoc"]);
    }
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
    assert_eq!(r.installed_count(tmp.path()), (0, 5 + helpers.len()));
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
    let n = Registry::builtin().components().len();
    assert!(
        said.any(&format!("{n} optional components.")),
        "{:?}",
        said.all()
    );
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
    // The downloaded files are made up, so no real recognizer or microphone
    // is opened (on Windows runners that crashed the test process).
    let made_from: Arc<Mutex<Vec<std::path::PathBuf>>> = Arc::default();
    let made = made_from.clone();
    app.dictation.factory = Some(Box::new(move |dir: &Path| {
        made.lock().unwrap().push(dir.to_path_buf());
        Box::new(Quiet::default()) as Box<dyn textweaver_dictation::Dictation>
    }));
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
    // The recognizer was made from the folder just downloaded.
    assert!(
        made_from.lock().unwrap().iter().any(|d| d == &dir),
        "made from {:?}, downloaded to {}",
        made_from.lock().unwrap(),
        dir.display()
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
    // The built-in components, the helper programs with a build here last.
    let n = Registry::builtin().components().len();
    assert!(
        model.items[..n]
            .iter()
            .all(|i| i.starts_with("Not chosen: "))
    );
    assert!(
        model.items[0]
            .starts_with("Not chosen: Whisper base.en, English dictation, for dictation, 79.3 MB"),
        "{:?}",
        model.items
    );
    assert_eq!(model.items[n], "Download the chosen ones");
    assert_eq!(model.items[n + 1], "Skip for now");
    assert!(app.settings().components.chooser_shown);
    // Nothing chosen: Download downloads nothing.
    app.dispatch(Command::Choose(n));
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

/// A components source that is a folder (no sign-in, no network): its
/// `components.toml` adds the components for this platform, a file comes
/// from `<release>/`, and a file whose checksum does not match is refused
/// with the reason and a next step, leaving nothing installed.
#[test]
fn a_source_folder_lists_installs_and_refuses_a_bad_checksum() {
    use textweaver_components::fake::FakeSource;
    let tmp = tempfile::tempdir().unwrap();
    let mut source = FakeSource::new(&tmp.path().join("my-components"));
    source
        .add(
            "made-up-font",
            "2.1",
            Platform::Any,
            Action::Place,
            &[("font.ttf", b"a made-up font")],
        )
        .unwrap();
    source
        .add(
            "made-up-tool",
            "",
            Platform::current(),
            Action::Place,
            &[("tool.bin", b"a made-up tool")],
        )
        .unwrap();
    let elsewhere = if Platform::current() == Platform::Linux {
        Platform::Windows
    } else {
        Platform::Linux
    };
    source
        .add(
            "other-platform",
            "",
            elsewhere,
            Action::Unpack,
            &[("x.zip", b"x")],
        )
        .unwrap();
    // Built only from its folder: the standard fetcher reads it.
    let (mut app, said) = app_in(tmp.path(), Arc::new(StandardFetcher));
    let root = source.root().to_string_lossy().into_owned();
    let _ = app.update_settings(|s| s.components.source = root.clone());
    app.dispatch(Command::Action(ActionId::ManageComponents));
    let model = app.list_model().unwrap().clone();
    let row = |title: &str| {
        model
            .items
            .iter()
            .position(|i| i.starts_with(title))
            .unwrap_or_else(|| panic!("{title} in {:?}", model.items))
    };
    let font = row("The made-up-font component: not installed");
    let _ = row("The made-up-tool component: not installed");
    assert!(
        !model.items.iter().any(|i| i.contains("other-platform")),
        "{:?}",
        model.items
    );
    let (c, _) = app.component_and_dir("made-up-font").unwrap();
    assert_eq!(c.listing.as_ref().unwrap().version, "2.1");
    assert_eq!(c.release(), "made-up-font-2.1");

    // A wrong checksum: refused, nothing installed.
    source
        .tamper("made-up-font-2.1", "font.ttf", b"a made-up fonT")
        .unwrap();
    app.dispatch(Command::Choose(font));
    app.dispatch(Command::Choose(0));
    app.dispatch(Command::Confirm(Confirm::Yes));
    assert!(app.wait_for_components(WAIT));
    assert!(
        said.any("Not installed: a file did not match. Get it again, or check the source."),
        "{:?}",
        said.all()
    );
    let (c, dir) = app.component_and_dir("made-up-font").unwrap();
    assert_eq!(c.status_in(&dir), Status::NotInstalled);

    // The right file: installed from the source folder.
    source
        .tamper("made-up-font-2.1", "font.ttf", b"a made-up font")
        .unwrap();
    app.dispatch(Command::Action(ActionId::ManageComponents));
    app.dispatch(Command::Choose(font));
    app.dispatch(Command::Choose(0));
    app.dispatch(Command::Confirm(Confirm::Yes));
    assert!(app.wait_for_components(WAIT));
    assert_eq!(c.status_in(&dir), Status::Installed);
    assert!(dir.starts_with(tmp.path().join("data").join("components")));
}

/// The log of this test program, written at trace level by the app's own
/// file logger (so its filter applies), beside the test program in the
/// target folder. Installed once.
#[cfg(feature = "publish")]
fn log_at_trace() -> PathBuf {
    static LOG: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    LOG.get_or_init(|| {
        let exe = std::env::current_exe().unwrap();
        let path = exe
            .parent()
            .unwrap()
            .join("token-privacy")
            .join("textweaver.log");
        let _ = std::fs::write(&path, "");
        let logger = crate::logfile::FileLogger::new(
            path.clone(),
            log::LevelFilter::Trace,
            512 * 1024 * 1024,
        );
        let logger: &'static crate::logfile::FileLogger = Box::leak(Box::new(logger));
        log::set_logger(logger).expect("no other logger in this test program");
        log::set_max_level(log::LevelFilter::Trace);
        path
    })
    .clone()
}

/// Every 10-character piece of `token` that is found in `text`.
#[cfg(feature = "publish")]
fn pieces_in(token: &str, text: &str) -> Vec<String> {
    (0..=token.len() - 10)
        .map(|i| &token[i..i + 10])
        .filter(|p| text.contains(p))
        .map(str::to_owned)
        .collect()
}

/// A private components source on a fake GitHub (no network, no real
/// token, a memory credential store): Manage optional components asks
/// once for a token, masked and never echoed, keeps it, lists the
/// source's component, and downloads it signed in. Then the token is in
/// none of settings.toml, the log (at trace level), anything said, or any
/// file the test made.
#[cfg(feature = "publish")]
#[test]
fn a_private_source_signs_in_once_and_the_token_is_never_written() {
    use crate::list_model::PromptKey;
    use textweaver_components::fake::{FakeGitHub, FakeSource};
    const TOKEN: &str = "ghp_EXAMPLEonlyNotARealToken7777";
    fake::memory_credentials();
    let log = log_at_trace();
    let tmp = tempfile::tempdir().unwrap();

    // The private repository: its list in the release `manifest`, its
    // file in the release `private-sample-1.0`.
    let mut source = FakeSource::new(&tmp.path().join("made"));
    source
        .add(
            "private-sample",
            "1.0",
            Platform::Any,
            Action::Place,
            &[("p.bin", b"a private file")],
        )
        .unwrap();
    let list = std::fs::read(source.root().join("components.toml")).unwrap();
    let gh = FakeGitHub::start(
        TOKEN,
        &[
            ("manifest", "components.toml", list.as_slice()),
            ("private-sample-1.0", "p.bin", &b"a private file"[..]),
        ],
    )
    .unwrap();

    let said = Said::default();
    let mut app = App::new(AppConfig {
        announcer: Box::new(said.clone()),
        paths: Some(Paths::under(tmp.path())),
        ..AppConfig::for_tests()
    });
    app.components.github_api = Some(gh.api().to_owned());
    let _ = app.update_settings(|s| s.components.source = "example-org/parts".to_owned());

    // Asked once, masked, nothing echoed, no history.
    app.dispatch(Command::Action(ActionId::ManageComponents));
    let purpose = app.prompt_model().map(|p| p.purpose);
    assert_eq!(purpose, Some(PromptPurpose::GitHubToken));
    said.clear();
    for c in TOKEN.chars() {
        app.dispatch(Command::PromptKey(PromptKey::Char(c)));
    }
    assert_eq!(
        app.prompt_model().unwrap().shown_text(),
        "*".repeat(TOKEN.len())
    );
    assert!(said.all().is_empty(), "{:?}", said.all());
    app.dispatch(Command::PromptKey(PromptKey::Enter));
    assert!(app.prompt_history(PromptPurpose::GitHubToken).is_empty());
    assert!(said.any("Token kept in the system credential store."));

    // The source's list was read signed in, and its file downloads.
    let model = app.list_model().unwrap().clone();
    let row = model
        .items
        .iter()
        .position(|i| i.starts_with("The private-sample component: not installed"))
        .unwrap_or_else(|| panic!("{:?}", model.items));
    app.dispatch(Command::Choose(row));
    app.dispatch(Command::Choose(0));
    app.dispatch(Command::Confirm(Confirm::Yes));
    assert!(app.wait_for_components(WAIT));
    let (c, dir) = app.component_and_dir("private-sample").unwrap();
    assert_eq!(c.status_in(&dir), Status::Installed, "{:?}", said.all());

    // Not asked again this session.
    app.dispatch(Command::Action(ActionId::ManageComponents));
    assert!(app.prompt_model().is_none());
    app.save_settings().unwrap();

    // The token went to the API, and nowhere else.
    let signed = gh
        .requests()
        .iter()
        .filter(|(_, h)| h.iter().any(|(k, _)| k == "authorization"))
        .count();
    assert!(signed >= 3, "{:?}", gh.requests());

    // Nowhere written: the log, what was said, and every file made.
    log::info!("token privacy test: fetched");
    log::logger().flush();
    let logged = std::fs::read_to_string(&log).unwrap();
    assert!(logged.contains("token privacy test: fetched"));
    assert_eq!(pieces_in(TOKEN, &logged), Vec::<String>::new(), "the log");
    let spoken = said.all().join("\n");
    assert_eq!(pieces_in(TOKEN, &spoken), Vec::<String>::new(), "messages");
    let settings = std::fs::read_to_string(Paths::under(tmp.path()).settings_file()).unwrap();
    assert!(settings.contains("example-org/parts"));
    let mut files = vec![tmp.path().to_owned()];
    let mut checked = 0;
    while let Some(p) = files.pop() {
        if p.is_dir() {
            files.extend(std::fs::read_dir(&p).unwrap().map(|e| e.unwrap().path()));
        } else {
            let text = String::from_utf8_lossy(&std::fs::read(&p).unwrap()).into_owned();
            assert_eq!(
                pieces_in(TOKEN, &text),
                Vec::<String>::new(),
                "{}",
                p.display()
            );
            checked += 1;
        }
    }
    assert!(checked >= 3, "settings, the list, and the file");

    // Forget the token: said in words, and asked again next time.
    app.dispatch(Command::Action(ActionId::ForgetGitHubToken));
    assert!(said.any("Token forgotten."));
    assert_eq!(credentials::stored_token(), None);
    app.dispatch(Command::Action(ActionId::ManageComponents));
    let purpose = app.prompt_model().map(|p| p.purpose);
    assert_eq!(purpose, Some(PromptPurpose::GitHubToken));
}

/// A launcher that records each installer it is asked to start and runs
/// nothing: the fake installer.
fn recording_launcher() -> (Launcher, Arc<Mutex<Vec<PathBuf>>>) {
    let ran = Arc::new(Mutex::new(Vec::new()));
    let r = Arc::clone(&ran);
    let launcher: Launcher = Arc::new(move |p: &Path| {
        r.lock().unwrap().push(p.to_owned());
        Ok(())
    });
    (launcher, ran)
}

/// The rows of the manager, and the row whose text starts with `title`.
fn manager_row(app: &mut App, title: &str) -> usize {
    app.dispatch(Command::Action(ActionId::ManageComponents));
    let model = app.list_model().unwrap().clone();
    model
        .items
        .iter()
        .position(|i| i.starts_with(title))
        .unwrap_or_else(|| panic!("{title} in {:?}", model.items))
}

/// A source with one component of each action: textweaver lists all
/// three, installs the first two (the archive unpacked), and for the
/// installer says its name, version, and license note and stops at the
/// question. No launches nothing and is remembered; yes launches it once.
#[test]
fn place_unpack_and_installer_from_a_source_folder() {
    use textweaver_components::fake::{FakeSource, zip_bytes};
    let tmp = tempfile::tempdir().unwrap();
    let here = Platform::current();
    let mut source = FakeSource::new(&tmp.path().join("my-components"));
    source
        .add(
            "made-up-voice",
            "1.0",
            Platform::Any,
            Action::Place,
            &[("voice.onnx", b"a voice")],
        )
        .unwrap();
    let archive = zip_bytes(&[("made-up-tool-2.0/bin/tool.exe", b"a made-up program")]);
    source
        .add(
            "made-up-tool",
            "2.0",
            here,
            Action::Unpack,
            &[("made-up-tool-2.0.zip", &archive)],
        )
        .unwrap();
    source
        .add(
            "made-up-setup",
            "0.3.0",
            here,
            Action::Installer,
            &[("setup.exe", b"a made-up installer")],
        )
        .unwrap();
    let (mut app, said) = app_in(tmp.path(), Arc::new(StandardFetcher));
    let (launcher, ran) = recording_launcher();
    app.set_installer_launcher(launcher);
    let root = source.root().to_string_lossy().into_owned();
    let _ = app.update_settings(|s| s.components.source = root.clone());

    // Place.
    let row = manager_row(&mut app, "The made-up-voice component: not installed");
    app.dispatch(Command::Choose(row));
    app.dispatch(Command::Choose(0));
    app.dispatch(Command::Confirm(Confirm::Yes));
    assert!(app.wait_for_components(WAIT));
    let (c, dir) = app.component_and_dir("made-up-voice").unwrap();
    assert_eq!(c.status_in(&dir), Status::Installed);
    assert!(dir.join("voice.onnx").is_file());

    // Unpack: the program is in the folder, the archive is not.
    let row = manager_row(&mut app, "The made-up-tool component: not installed");
    app.dispatch(Command::Choose(row));
    app.dispatch(Command::Choose(0));
    app.dispatch(Command::Confirm(Confirm::Yes));
    assert!(app.wait_for_components(WAIT));
    let (c, dir) = app.component_and_dir("made-up-tool").unwrap();
    assert_eq!(c.status_in(&dir), Status::Installed);
    assert!(
        dir.join("made-up-tool-2.0")
            .join("bin")
            .join("tool.exe")
            .is_file()
    );
    assert!(!dir.join("made-up-tool-2.0.zip").exists());
    // The engine discovery's search finds what was unpacked.
    let comp = Paths::under(tmp.path()).components_dir();
    assert_eq!(
        textweaver_store::find_in_components(&comp, &["tool.exe"]),
        Some(dir.join("made-up-tool-2.0").join("bin").join("tool.exe"))
    );

    // Installer: downloaded and checked, then the question, said first.
    said.clear();
    let row = manager_row(&mut app, "The made-up-setup component: not installed");
    app.dispatch(Command::Choose(row));
    app.dispatch(Command::Choose(0));
    app.dispatch(Command::Confirm(Confirm::Yes));
    assert!(app.wait_for_components(WAIT));
    assert!(
        said.any(
            "Launch installer: The made-up-setup component, version 0.3.0, license CC0-1.0, made up for tests? The system asks next. y or n"
        ),
        "{:?}",
        said.all()
    );
    app.dispatch(Command::Confirm(Confirm::No));
    assert!(
        said.any("Not launched; the installer is kept."),
        "{:?}",
        said.all()
    );
    assert!(ran.lock().unwrap().is_empty(), "no launches nothing");
    assert!(app.component_declined("made-up-setup"));
    let _ = manager_row(
        &mut app,
        "The made-up-setup component: installer downloaded",
    );

    // Download again on a downloaded installer: the launch is offered.
    let row = manager_row(&mut app, "The made-up-setup component");
    app.dispatch(Command::Choose(row));
    app.dispatch(Command::Choose(0));
    app.dispatch(Command::Confirm(Confirm::Yes));
    let ran = ran.lock().unwrap().clone();
    assert_eq!(ran.len(), 1, "{ran:?}");
    assert!(ran[0].ends_with("setup.exe"), "{ran:?}");
    assert!(
        said.any("Installer started: The made-up-setup component."),
        "{:?}",
        said.all()
    );
}

/// A braille file opened without liblouis: one question instead of "not
/// found", with the source supplying liblouis; a no is remembered for the
/// session, and the file is not asked about again.
#[test]
fn liblouis_is_offered_once_when_a_braille_file_needs_it() {
    use textweaver_components::fake::{FakeSource, zip_bytes};
    let tmp = tempfile::tempdir().unwrap();
    let mut source = FakeSource::new(&tmp.path().join("my-components"));
    let archive = zip_bytes(&[("liblouis/bin/lou_translate.exe", b"a made-up program")]);
    source
        .add(
            "liblouis",
            "3.39.0",
            Platform::current(),
            Action::Unpack,
            &[("liblouis-made-up.zip", &archive)],
        )
        .unwrap();
    let (mut app, said) = app_in(tmp.path(), Arc::new(StandardFetcher));
    let root = source.root().to_string_lossy().into_owned();
    let _ = app.update_settings(|s| s.components.source = root.clone());
    let brf = tmp.path().join("book.brf");
    app.say_braille_untranslated(&brf);
    assert!(
        said.any("Reading braille as print needs The liblouis component, "),
        "{:?}",
        said.all()
    );
    app.dispatch(Command::Confirm(Confirm::No));
    assert!(said.any("Not downloaded."), "{:?}", said.all());
    said.clear();
    app.say_braille_untranslated(&brf);
    assert!(
        said.any("Braille shown as braille: liblouis is missing."),
        "{:?}",
        said.all()
    );
    assert!(!said.any("Download it now?"), "{:?}", said.all());
    // The source's liblouis took the built-in one's place.
    let (c, _) = app.component_and_dir("liblouis").unwrap();
    assert_eq!(c.title, "The liblouis component");
}

/// Yes: the source's liblouis is fetched and unpacked, and the braille
/// file is opened again.
#[test]
fn after_a_yes_liblouis_is_unpacked_and_the_file_opens_again() {
    use textweaver_components::fake::{FakeSource, zip_bytes};
    let tmp = tempfile::tempdir().unwrap();
    let mut source = FakeSource::new(&tmp.path().join("my-components"));
    let archive = zip_bytes(&[("liblouis/bin/lou_translate.exe", b"a made-up program")]);
    source
        .add(
            "liblouis",
            "3.39.0",
            Platform::current(),
            Action::Unpack,
            &[("liblouis-made-up.zip", &archive)],
        )
        .unwrap();
    let (mut app, said) = app_in(tmp.path(), Arc::new(StandardFetcher));
    let root = source.root().to_string_lossy().into_owned();
    let _ = app.update_settings(|s| s.components.source = root.clone());
    let brf = tmp.path().join("book.brf");
    std::fs::write(&brf, "  ,HELLO\r\n").unwrap();
    app.say_braille_untranslated(&brf);
    app.dispatch(Command::Confirm(Confirm::Yes));
    assert!(app.wait_for_components(WAIT));
    let (c, dir) = app.component_and_dir("liblouis").unwrap();
    assert_eq!(c.status_in(&dir), Status::Installed);
    assert!(
        dir.join("liblouis")
            .join("bin")
            .join("lou_translate.exe")
            .is_file()
    );
    assert!(
        said.any("Ready: The liblouis component."),
        "{:?}",
        said.all()
    );
    let open = app.session.as_ref().and_then(|s| s.doc.meta.path.clone());
    assert_eq!(
        open.as_deref(),
        Some(brf.as_path()),
        "the file opened again"
    );
}
