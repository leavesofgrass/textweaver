//! Editing a document's details by hand (Wave 7, W7m): the form on the
//! shared prompt model (each field a labeled line, Tab and Shift+Tab
//! between, Enter saves, Escape cancels), from the open document and from
//! the library list; the edit survives a reload, is found by the library
//! filter, and reaches a second home through the sync folder, where it
//! wins over the document's own title.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::{Library, Paths, Settings};
use textweaver_app::testing::recording_service;
use textweaver_app::{App, AppConfig, Command, Effect, ListKey, PromptKey, PromptPurpose};

#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Said {
    fn all(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
    fn any(&self, needle: &str) -> bool {
        self.all().iter().any(|s| s.contains(needle))
    }
    fn last(&self) -> String {
        self.all().last().cloned().unwrap_or_default()
    }
}

impl Announcer for Said {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}

struct Home {
    app: App,
    said: Said,
    home: PathBuf,
    clock: Instant,
}

fn home_at(home: &Path, tweak: impl FnOnce(&mut Settings)) -> Home {
    let mut settings = Settings::default();
    tweak(&mut settings);
    let said = Said::default();
    let (speech, _log) = recording_service().unwrap();
    let app = App::new(AppConfig {
        settings,
        speech,
        paths: Some(Paths::under(home)),
        announcer: Box::new(said.clone()),
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    Home {
        app,
        said,
        home: home.to_owned(),
        clock: Instant::now(),
    }
}

impl Home {
    fn paths(&self) -> Paths {
        Paths::under(&self.home)
    }

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

    /// Runs an action and waits for the background work it starts (a
    /// library scan, or reading the details for the form).
    fn act(&mut self, a: ActionId) -> Vec<Effect> {
        let mut effects = self.app.dispatch(Command::Action(a));
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline
            && (self.app.library_scanning()
                || (a == ActionId::EditDocumentDetails && self.app.prompt_model().is_none()))
        {
            std::thread::sleep(Duration::from_millis(2));
            effects.extend(self.app.tick(Instant::now()));
        }
        effects
    }

    fn key(&mut self, k: PromptKey) -> Vec<Effect> {
        self.app.dispatch(Command::PromptKey(k))
    }

    /// Replaces the focused field's text, as a GUI field does.
    fn type_text(&mut self, text: &str) {
        self.key(PromptKey::SetText(text.to_owned()));
    }

    fn field(&self) -> (String, String) {
        let p = self.app.prompt_model().expect("the form is open");
        assert_eq!(p.purpose, PromptPurpose::DocumentDetails);
        (p.label.clone(), p.text())
    }
}

fn list_items(effects: &[Effect]) -> Vec<String> {
    effects
        .iter()
        .find_map(|e| match e {
            Effect::ShowList { items, .. } => Some(items.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

const SCAN: &str = "Cells divide by mitosis. Each daughter cell gets a full copy.\n\n\
Meiosis makes gametes. It halves the chromosome count.\n";

/// From the open document: every field is a labeled line with its value;
/// Tab and Shift+Tab move and keep what was typed; a bad DOI is said and
/// the form opens again on it; Enter saves; the edit survives a reload and
/// the library filter finds it.
#[test]
fn the_form_edits_the_open_document_and_the_edit_survives_a_reload() {
    let home = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    let doc = docs.path().join("scan0042.txt");
    std::fs::write(&doc, SCAN).unwrap();

    let mut h = home_at(home.path(), |_| {});
    h.open(&doc);
    h.act(ActionId::EditDocumentDetails);
    assert!(
        h.said
            .any("Details of scan0042.txt. Tab moves, Enter saves, Escape cancels."),
        "{:?}",
        h.said.all()
    );
    assert_eq!(
        h.field(),
        ("Title, 1 of 4".to_owned(), "scan0042.txt".to_owned())
    );
    assert_eq!(h.said.last(), "Title: scan0042.txt");

    h.type_text("Cell Biology, Chapter 3");
    h.key(PromptKey::Tab);
    assert_eq!(h.field(), ("Author, 2 of 4".to_owned(), String::new()));
    assert_eq!(h.said.last(), "Author: blank");
    // Shift+Tab goes back and keeps what was typed.
    h.key(PromptKey::BackTab);
    assert_eq!(
        h.field(),
        (
            "Title, 1 of 4".to_owned(),
            "Cell Biology, Chapter 3".to_owned()
        )
    );
    assert_eq!(h.said.last(), "Title: Cell Biology, Chapter 3");
    // Shift+Tab from the first field wraps to the last.
    h.key(PromptKey::BackTab);
    assert_eq!(h.field().0, "ISBN, 4 of 4");
    h.key(PromptKey::Tab);
    h.key(PromptKey::Tab);
    h.type_text("Ada Example");
    h.key(PromptKey::Tab);
    h.type_text("not a doi");
    h.key(PromptKey::Enter);
    assert!(
        h.said.any("Not a DOI: not a doi. Fix it or clear it."),
        "{:?}",
        h.said.all()
    );
    assert_eq!(
        h.field(),
        ("DOI, 3 of 4".to_owned(), "not a doi".to_owned()),
        "the form opens again on the DOI"
    );
    h.type_text("https://doi.org/10.1000/CELLS");
    h.key(PromptKey::Enter);
    assert!(h.app.prompt_model().is_none(), "Enter closed the form");
    assert!(
        h.said.any("Details saved: Title, Author, DOI."),
        "{:?}",
        h.said.all()
    );
    assert_eq!(
        h.app.session().unwrap().title,
        "Cell Biology, Chapter 3",
        "the reader's title follows"
    );
    h.app.wait_for_writes();
    h.app.shutdown();

    // A reload: a new app on the same home; the document opens again and
    // records its own details, and the edit still wins.
    let mut h = home_at(home.path(), |_| {});
    h.open(&doc);
    let lib = Library::load(&h.paths().library_file()).unwrap();
    let e = lib.get(&doc).unwrap();
    assert_eq!(e.shown_title(), "Cell Biology, Chapter 3");
    assert_eq!(e.edited.doi.as_deref(), Some("10.1000/cells"));
    let effects = h.act(ActionId::OpenLibrary);
    let items = list_items(&effects);
    assert!(
        items
            .iter()
            .any(|i| i.starts_with("Cell Biology, Chapter 3, by Ada Example")),
        "{items:?}"
    );
    for q in ["chapter 3", "doi:10.1000/cells", "ada example"] {
        let shown = list_items(&h.app.dispatch(Command::FilterList(q.into())));
        assert_eq!(shown.len(), 1, "{q}: {shown:?}");
    }
    h.app.dispatch(Command::Cancel);

    // The form starts with the edited values, and Escape changes nothing,
    // and says so.
    h.act(ActionId::EditDocumentDetails);
    assert_eq!(h.field().1, "Cell Biology, Chapter 3");
    h.type_text("Something else");
    h.key(PromptKey::Escape);
    assert!(h.app.prompt_model().is_none());
    assert_eq!(h.said.last(), "Cancelled. Details not changed.");
    h.app.wait_for_writes();
    let lib = Library::load(&h.paths().library_file()).unwrap();
    assert_eq!(
        lib.get(&doc).unwrap().shown_title(),
        "Cell Biology, Chapter 3"
    );
    // Enter with nothing changed saves nothing.
    h.act(ActionId::EditDocumentDetails);
    h.key(PromptKey::Enter);
    assert_eq!(h.said.last(), "Details not changed.");
    h.app.shutdown();
}

/// A cleared field takes effect at once in the library row (W8a): the
/// row shows the document's own title and no author, without the list
/// being read again, and it matches what a reread shows. Both for an edit
/// made in this list and for one saved in an earlier session.
#[test]
fn a_cleared_field_takes_effect_at_once_in_the_library_row() {
    let home = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    let lib = docs.path().join("Readings");
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::write(lib.join("scan0042.txt"), SCAN).unwrap();
    let folder = lib.clone();
    let with_folder = move |s: &mut Settings| {
        s.library.add_folder(&folder);
    };

    let mut h = home_at(home.path(), with_folder.clone());
    let effects = h.act(ActionId::OpenLibrary);
    assert_eq!(list_items(&effects), ["scan0042, in Readings"]);
    // Set the title and the author.
    h.app.dispatch(Command::ListKey(ListKey::Rename));
    h.type_text("Cell Biology");
    h.key(PromptKey::Tab);
    h.type_text("Ada Example");
    let effects = h.key(PromptKey::Enter);
    assert_eq!(
        list_items(&effects),
        ["Cell Biology, by Ada Example, in Readings"]
    );
    // Clear both: the row shows the document's own title, no author.
    h.app.dispatch(Command::ListKey(ListKey::Rename));
    assert_eq!(h.field().1, "Cell Biology");
    h.type_text("");
    h.key(PromptKey::Tab);
    assert_eq!(h.field().1, "Ada Example");
    h.type_text("");
    let effects = h.key(PromptKey::Enter);
    assert_eq!(
        list_items(&effects),
        ["scan0042, in Readings"],
        "the cleared fields take effect at once"
    );
    // The same as reading the list again.
    h.app.dispatch(Command::Cancel);
    h.app.wait_for_writes();
    let effects = h.act(ActionId::OpenLibrary);
    assert_eq!(list_items(&effects), ["scan0042, in Readings"]);

    // An edit saved in an earlier session, cleared in this one.
    h.app.dispatch(Command::ListKey(ListKey::Rename));
    h.type_text("Cell Biology");
    h.key(PromptKey::Enter);
    h.app.dispatch(Command::Cancel);
    h.app.wait_for_writes();
    h.app.shutdown();
    let mut h = home_at(home.path(), with_folder);
    let effects = h.act(ActionId::OpenLibrary);
    assert_eq!(list_items(&effects), ["Cell Biology, in Readings"]);
    h.app.dispatch(Command::ListKey(ListKey::Rename));
    h.type_text("");
    let effects = h.key(PromptKey::Enter);
    assert_eq!(list_items(&effects), ["scan0042, in Readings"]);
    h.app.shutdown();
}

#[test]
fn with_no_document_open_the_command_says_so() {
    let home = tempfile::tempdir().unwrap();
    let mut h = home_at(home.path(), |_| {});
    h.app
        .dispatch(Command::Action(ActionId::EditDocumentDetails));
    assert!(
        h.said.last().starts_with("No document is open."),
        "{}",
        h.said.last()
    );
    assert!(h.app.prompt_model().is_none());
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

/// From the library list (F2 on a document) on the laptop: the list comes
/// back with the row changed. The edit syncs to the lab, which finds the
/// document by its new title and DOI, and opening it there (publishing its
/// own title) does not undo the edit. Nothing in the sync folder names a
/// path, a file, or a computer.
#[test]
fn an_edit_from_the_library_syncs_to_a_second_home() {
    let folder = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    let lib_a = docs.path().join("laptop").join("Readings");
    std::fs::create_dir_all(&lib_a).unwrap();
    let doc_a = lib_a.join("scan0042.txt");
    std::fs::write(&doc_a, SCAN).unwrap();
    let homes = tempfile::tempdir().unwrap();
    let sync_on = |name: &'static str, lib: PathBuf| {
        let folder = folder.path().to_owned();
        move |s: &mut Settings| {
            s.sync.enabled = true;
            s.sync.folder = Some(folder);
            s.sync.device_name = name.to_owned();
            s.library.add_folder(&lib);
        }
    };

    let mut laptop = home_at(
        &homes.path().join("laptop"),
        sync_on("laptop", lib_a.clone()),
    );
    laptop.settle();
    let effects = laptop.act(ActionId::OpenLibrary);
    assert_eq!(list_items(&effects).len(), 1);
    assert!(
        laptop.said.any("F2 edits details."),
        "the list says how: {:?}",
        laptop.said.all()
    );
    laptop.app.dispatch(Command::ListKey(ListKey::Rename));
    assert_eq!(
        laptop.field(),
        ("Title, 1 of 4".to_owned(), "scan0042".to_owned())
    );
    laptop.type_text("Cell Biology, Chapter 3");
    laptop.key(PromptKey::Tab);
    laptop.key(PromptKey::Tab);
    laptop.type_text("10.1000/cells");
    let effects = laptop.key(PromptKey::Enter);
    let items = list_items(&effects);
    assert_eq!(
        items,
        vec!["Cell Biology, Chapter 3, in Readings".to_owned()],
        "the library list comes back with the row changed"
    );
    assert!(laptop.said.any("Details saved: Title, DOI."));
    laptop.app.dispatch(Command::Cancel);
    laptop.settle();
    laptop.app.shutdown();

    // The library folder arrives on the lab computer, id file and all.
    let lib_b = docs.path().join("lab").join("Course readings");
    copy_dir(&lib_a, &lib_b);
    let mut lab = home_at(&homes.path().join("lab"), sync_on("lab", lib_b.clone()));
    lab.settle();
    let effects = lab.act(ActionId::OpenLibrary);
    assert_eq!(
        list_items(&effects),
        vec!["Cell Biology, Chapter 3, in Course readings".to_owned()]
    );
    for q in ["chapter 3", "10.1000/cells"] {
        let shown = list_items(&lab.app.dispatch(Command::FilterList(q.into())));
        assert_eq!(shown.len(), 1, "{q}: {shown:?}");
    }
    lab.app.dispatch(Command::Cancel);
    // Opening it on the lab publishes its own title; the edit still wins.
    lab.open(&lib_b.join("scan0042.txt"));
    let effects = lab.act(ActionId::OpenLibrary);
    assert_eq!(
        list_items(&effects),
        vec!["Cell Biology, Chapter 3, in Course readings".to_owned()]
    );
    lab.app.dispatch(Command::Cancel);
    // The form on the lab starts with the laptop's edit.
    lab.act(ActionId::EditDocumentDetails);
    assert_eq!(lab.field().1, "Cell Biology, Chapter 3");
    lab.key(PromptKey::Escape);
    lab.app.shutdown();

    // Privacy: no path, file name, or computer name in the sync folder.
    let mut forbidden: Vec<String> = textweaver_app::sync_folder::local_names()
        .into_iter()
        .filter(|n| n.len() >= 3)
        .collect();
    forbidden.push("scan0042".into());
    for p in [&lib_a, &lib_b, &homes.path().to_owned()] {
        forbidden.push(p.display().to_string());
        forbidden.push(p.display().to_string().replace('\\', "\\\\"));
    }
    let mut files = Vec::new();
    walk(folder.path(), &mut files);
    assert!(!files.is_empty());
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
            assert!(!text.contains(pattern), "{} holds a path", f.display());
        }
    }
}
