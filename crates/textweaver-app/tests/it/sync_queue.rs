//! The reading queue across computers (beta 3): two sync engines, each with
//! its own home, share one temporary sync folder. The queue rides the
//! places switch, as a map from each document's sync id to when it joined
//! the queue; the newest add or removal wins, and the queue is in the order
//! documents were added on every computer. No real sync service and no
//! audio.

use std::path::{Path, PathBuf};
use std::time::Duration;

use textweaver_app::store::{Library, Paths, Settings};
use textweaver_app::sync_engine::{EngineConfig, Groups, SyncEngine};
use textweaver_app::sync_folder::Identify;
use textweaver_app::sync_folder::docid::Details;
use textweaver_app::sync_groups::{AppliedKey, GroupsRequest, KeySystem, apply_groups};

const WEEK3: &str = "Week 3. Cells divide by mitosis.\n";
const WEEK4: &str = "Week 4. Meiosis makes gametes.\n";
const WEEK5: &str = "Week 5. Proteins fold into shapes.\n";

/// One computer: its home, its engine, and its copies of the documents.
struct Computer {
    home: tempfile::TempDir,
    paths: Paths,
    engine: SyncEngine,
    /// The arrivals applied since the last merge, as the app sends them.
    applied: Vec<AppliedKey>,
}

impl Computer {
    fn new(folder: &Path, label: &str, groups: Groups) -> Self {
        let home = tempfile::tempdir().unwrap();
        let paths = Paths::under(home.path());
        std::fs::create_dir_all(&paths.config_dir).unwrap();
        std::fs::create_dir_all(&paths.data_dir).unwrap();
        let mut engine = SyncEngine::new();
        engine.configure(Some(EngineConfig {
            folder: folder.to_owned(),
            device_name: label.into(),
            paths: paths.clone(),
            app_version: "test".into(),
            groups,
            system: KeySystem::Windows,
        }));
        Computer {
            home,
            paths,
            engine,
            applied: Vec::new(),
        }
    }

    /// This computer's copy of a document, under its own name.
    fn doc(&self, name: &str, text: &str) -> PathBuf {
        let p = self.home.path().join("docs").join(name);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, text).unwrap();
        p
    }

    fn shelf(&self) -> Library {
        Library::load(&self.paths.library_file()).unwrap()
    }

    fn edit(&self, f: impl FnOnce(&mut Library)) {
        let mut lib = self.shelf();
        f(&mut lib);
        lib.save(&self.paths.library_file()).unwrap();
        // Each change is a moment after the last one anywhere.
        std::thread::sleep(Duration::from_millis(20));
    }

    /// Opens `path` here: on the bookshelf, and identified, as the reader
    /// does when a document opens.
    fn open(&mut self, path: &Path) {
        self.edit(|l| {
            l.record_open(path, "", "text");
        });
        let identify = Identify {
            ids_file: self.paths.sync_ids_file(),
            path: path.to_owned(),
            library_folders: Vec::new(),
            details: Details::default(),
        };
        self.engine.edit_details(&identify, &[]);
    }

    /// A merge of the groups, with what arrived applied.
    fn sync(&mut self) {
        let mut settings = Settings::default();
        let o = self.engine.groups_cycle(&GroupsRequest {
            settings: Box::new(settings.clone()),
            settings_ms: 0,
            force: true,
            applied: std::mem::take(&mut self.applied),
        });
        let applied = apply_groups(&self.paths, &mut settings, &o.arrivals, KeySystem::Windows);
        self.applied = applied.applied;
    }

    /// The queue's documents by file name, in order; one not found here is
    /// `elsewhere`.
    fn queue(&self) -> Vec<String> {
        self.shelf()
            .queue
            .iter()
            .map(|q| {
                q.path.as_ref().map_or_else(
                    || "elsewhere".to_owned(),
                    |p| p.file_name().unwrap().to_string_lossy().into_owned(),
                )
            })
            .collect()
    }
}

fn two(folder: &Path) -> (Computer, Computer) {
    (
        Computer::new(folder, "laptop", Groups::ALL),
        Computer::new(folder, "lab", Groups::ALL),
    )
}

/// Nothing in the sync folder names either home (no path leaves the
/// computer).
fn no_paths_in(folder: &Path, homes: &[&Computer]) {
    let mut stack = vec![folder.to_owned()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let text = std::fs::read_to_string(&p).unwrap_or_default();
            for c in homes {
                let home = c.home.path().display().to_string();
                assert!(!text.contains(&home), "{} names a home", p.display());
                assert!(
                    !text.contains(&home.replace('\\', "\\\\")),
                    "{} names a home",
                    p.display()
                );
                assert!(!text.contains("week"), "{} names a file", p.display());
            }
        }
    }
}

#[test]
fn a_document_added_on_one_computer_joins_the_other_in_order() {
    let folder = tempfile::tempdir().unwrap();
    let (mut laptop, mut lab) = two(folder.path());
    let l3 = laptop.doc("week3.md", WEEK3);
    let l4 = laptop.doc("week4.md", WEEK4);
    let b3 = lab.doc("lab-week3.md", WEEK3);
    let b4 = lab.doc("lab-week4.md", WEEK4);
    // The lab opened week 3 before; week 4 is not on its bookshelf.
    lab.open(&b3);
    laptop.edit(|l| {
        l.queue_add(&l3);
    });
    laptop.edit(|l| {
        l.queue_add(&l4);
    });
    laptop.sync();
    lab.sync();
    // Week 3 is found by its contents; week 4 keeps its place, not found.
    assert_eq!(lab.queue(), ["lab-week3.md", "elsewhere"]);
    let shelf = lab.shelf();
    assert!(shelf.queue[1].sync_id.is_some());
    assert_eq!(
        shelf.queue_next(None),
        Some(textweaver_app::store::library::resolve_path(&b3).as_path())
    );
    // Opened here, week 4 is found, and keeps its place.
    lab.open(&b4);
    lab.sync();
    assert_eq!(lab.queue(), ["lab-week3.md", "lab-week4.md"]);
    // Nothing changed on the laptop.
    laptop.sync();
    assert_eq!(laptop.queue(), ["week3.md", "week4.md"]);
    no_paths_in(folder.path(), &[&laptop, &lab]);
}

#[test]
fn a_removed_document_is_removed_everywhere_and_never_comes_back() {
    let folder = tempfile::tempdir().unwrap();
    let (mut laptop, mut lab) = two(folder.path());
    let l3 = laptop.doc("week3.md", WEEK3);
    let l4 = laptop.doc("week4.md", WEEK4);
    let b3 = lab.doc("lab-week3.md", WEEK3);
    let b4 = lab.doc("lab-week4.md", WEEK4);
    lab.open(&b3);
    lab.open(&b4);
    laptop.edit(|l| {
        l.queue_add(&l3);
        l.queue_add(&l4);
    });
    laptop.sync();
    lab.sync();
    assert_eq!(lab.queue(), ["lab-week3.md", "lab-week4.md"]);

    // Read to the end on the lab: week 3 leaves its queue.
    lab.edit(|l| {
        l.queue_remove(&b3);
    });
    lab.sync();
    laptop.sync();
    assert_eq!(laptop.queue(), ["week4.md"]);
    // The laptop, which still had it, does not bring it back.
    lab.sync();
    laptop.sync();
    assert_eq!(lab.queue(), ["lab-week4.md"]);
    assert_eq!(laptop.queue(), ["week4.md"]);

    // Adding it again later wins over the removal, on both.
    laptop.edit(|l| {
        l.queue_add(&l3);
    });
    laptop.sync();
    lab.sync();
    assert_eq!(laptop.queue(), ["week4.md", "week3.md"]);
    assert_eq!(lab.queue(), ["lab-week4.md", "lab-week3.md"]);
}

#[test]
fn a_removal_and_an_add_made_apart_both_survive() {
    let folder = tempfile::tempdir().unwrap();
    let (mut laptop, mut lab) = two(folder.path());
    let l3 = laptop.doc("week3.md", WEEK3);
    let l5 = laptop.doc("week5.md", WEEK5);
    let b3 = lab.doc("lab-week3.md", WEEK3);
    let b5 = lab.doc("lab-week5.md", WEEK5);
    lab.open(&b3);
    lab.open(&b5);
    laptop.edit(|l| {
        l.queue_add(&l3);
    });
    laptop.sync();
    lab.sync();
    // Apart: the lab takes week 3 out, the laptop adds week 5.
    lab.edit(|l| {
        l.queue_remove(&b3);
    });
    laptop.edit(|l| {
        l.queue_add(&l5);
    });
    lab.sync();
    laptop.sync();
    lab.sync();
    assert_eq!(laptop.queue(), ["week5.md"]);
    assert_eq!(lab.queue(), ["lab-week5.md"]);
}

#[test]
fn both_computers_adding_one_document_keep_it_once_in_one_order() {
    let folder = tempfile::tempdir().unwrap();
    let (mut laptop, mut lab) = two(folder.path());
    let l3 = laptop.doc("week3.md", WEEK3);
    let l4 = laptop.doc("week4.md", WEEK4);
    let b3 = lab.doc("lab-week3.md", WEEK3);
    let b4 = lab.doc("lab-week4.md", WEEK4);
    laptop.open(&l3);
    laptop.open(&l4);
    lab.open(&b3);
    lab.open(&b4);
    // Apart, both queue week 3; the lab queues week 4 first.
    lab.edit(|l| {
        l.queue_add(&b4);
    });
    laptop.edit(|l| {
        l.queue_add(&l3);
    });
    lab.edit(|l| {
        l.queue_add(&b3);
    });
    laptop.sync();
    lab.sync();
    laptop.sync();
    // Week 3 once, at the newer add's time: after week 4.
    assert_eq!(laptop.queue(), ["week4.md", "week3.md"]);
    assert_eq!(lab.queue(), ["lab-week4.md", "lab-week3.md"]);
}

#[test]
fn taking_a_document_out_and_adding_it_again_moves_it_to_the_end_everywhere() {
    let folder = tempfile::tempdir().unwrap();
    let (mut laptop, mut lab) = two(folder.path());
    let docs: Vec<(PathBuf, PathBuf)> = [
        ("week3.md", WEEK3),
        ("week4.md", WEEK4),
        ("week5.md", WEEK5),
    ]
    .iter()
    .map(|(n, t)| (laptop.doc(n, t), lab.doc(&format!("lab-{n}"), t)))
    .collect();
    for (_, b) in &docs {
        lab.open(b);
    }
    for (l, _) in &docs {
        laptop.edit(|lib| {
            lib.queue_add(l);
        });
    }
    laptop.sync();
    lab.sync();
    assert_eq!(
        lab.queue(),
        ["lab-week3.md", "lab-week4.md", "lab-week5.md"]
    );
    // On the lab, week 3 moves to the end.
    lab.edit(|l| {
        l.queue_remove(&docs[0].1);
    });
    lab.edit(|l| {
        l.queue_add(&docs[0].1);
    });
    lab.sync();
    laptop.sync();
    assert_eq!(laptop.queue(), ["week4.md", "week5.md", "week3.md"]);
    assert_eq!(
        lab.queue(),
        ["lab-week4.md", "lab-week5.md", "lab-week3.md"]
    );
}

#[test]
fn with_places_off_the_queue_stays_on_this_computer() {
    let folder = tempfile::tempdir().unwrap();
    let mut laptop = Computer::new(
        folder.path(),
        "laptop",
        Groups {
            places: false,
            ..Groups::ALL
        },
    );
    let mut lab = Computer::new(folder.path(), "lab", Groups::ALL);
    let l3 = laptop.doc("week3.md", WEEK3);
    let b3 = lab.doc("lab-week3.md", WEEK3);
    lab.open(&b3);
    laptop.edit(|l| {
        l.queue_add(&l3);
    });
    laptop.sync();
    lab.sync();
    assert!(lab.queue().is_empty());
}

/// Taking a document not found here out of the queue removes it on the
/// computer that has it too.
#[test]
fn a_document_not_found_here_can_be_taken_out() {
    let folder = tempfile::tempdir().unwrap();
    let (mut laptop, mut lab) = two(folder.path());
    let l4 = laptop.doc("week4.md", WEEK4);
    laptop.edit(|l| {
        l.queue_add(&l4);
    });
    laptop.sync();
    lab.sync();
    assert_eq!(lab.queue(), ["elsewhere"]);
    let id = lab.shelf().queue[0].sync_id.clone().unwrap();
    lab.edit(|l| {
        assert!(l.queue_remove_elsewhere(&id));
    });
    lab.sync();
    laptop.sync();
    assert!(laptop.queue().is_empty());
}
