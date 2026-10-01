//! The sync folder with simulated computers: each has its own state folder
//! and, like Syncthing, its own copy of the sync folder, which a test
//! "service" keeps in step by copying each computer's files to the others.
//! Nothing touches a real sync service.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use textweaver_core::{CharPos, CharRange};
use textweaver_store::{Bookmark, Highlight, Note};
use textweaver_sync::folder::{DEVICE_FILE, DEVICES_DIR, DOCS_DIR, FORMAT_FILE, SYNC_DIR};
use textweaver_sync::{
    ChangeKind, Clock, DocRecord, FileKind, Identity, IdentityEvent, ItemKind, Place, Problem,
    ReadOnly, Stamp, SyncError, SyncFolder, SyncId, local_names, wall_ms,
};

const DOC: SyncId = SyncId::from_u128(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef);
const VERSION: &str = "0.1.0-alpha.6";

struct Computer {
    _state: tempfile::TempDir,
    _sync: tempfile::TempDir,
    state_dir: PathBuf,
    sync_dir: PathBuf,
    identity: Identity,
    folder: SyncFolder,
    clock: Clock,
    record: DocRecord,
}

impl Computer {
    fn new(label: &str) -> Self {
        let state = tempfile::tempdir().unwrap();
        let sync = tempfile::tempdir().unwrap();
        Self::with_dirs(label, state, sync)
    }

    fn with_dirs(label: &str, state: tempfile::TempDir, sync: tempfile::TempDir) -> Self {
        let (mut identity, _) = Identity::load_or_create(state.path()).unwrap();
        let opened = SyncFolder::open(sync.path(), &mut identity, label, VERSION).unwrap();
        assert!(opened.problems.is_empty(), "{:?}", opened.problems);
        let clock = Clock::new(identity.device());
        Self {
            state_dir: state.path().to_owned(),
            sync_dir: sync.path().to_owned(),
            _state: state,
            _sync: sync,
            identity,
            folder: opened.folder,
            clock,
            record: DocRecord::new(DOC),
        }
    }

    fn stamp(&mut self, at: u64) -> Stamp {
        self.clock.tick_at(at)
    }

    fn sync(&mut self) -> textweaver_sync::Merged {
        let merged = self.folder.merge_doc(&mut self.record, &mut self.clock);
        self.folder.write_doc(&self.record).unwrap();
        merged
    }

    fn own_dir(&self) -> PathBuf {
        self.sync_dir
            .join(SYNC_DIR)
            .join(DEVICES_DIR)
            .join(self.folder.device().to_string())
    }
}

fn note(id: &str, text: &str) -> Note {
    Note {
        id: id.to_owned(),
        range: CharRange::new(CharPos(10), CharPos(15)),
        anchor: "Cells".to_owned(),
        note: text.to_owned(),
        tags: vec!["biology".to_owned()],
        cite: String::new(),
        color: None,
        relations: Vec::new(),
        created: 1_790_000_000,
        ts: 1_790_000_000,
        extra: serde_json::Map::new(),
    }
}

fn bookmark(name: &str, pos: usize) -> Bookmark {
    Bookmark {
        id: format!("bm-{name}"),
        name: name.to_owned(),
        pos: CharPos(pos),
        pct: 3,
        ts: 1_790_000_000,
        anchor: None,
        not_found: false,
        extra: serde_json::Map::new(),
    }
}

fn place(pos: usize) -> Place {
    Place {
        pos: CharPos(pos),
        pct: 1,
        anchor: None,
    }
}

/// Every file under `dir`, recursively.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_owned()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// The sync service: copies each computer's own folder, and `format.json`,
/// into every other computer's copy of the sync folder.
fn propagate(computers: &[&Computer]) {
    for from in computers {
        let own = from.own_dir();
        let rel_root = from.sync_dir.clone();
        let mut sources = files(&own);
        sources.push(from.sync_dir.join(SYNC_DIR).join(FORMAT_FILE));
        for to in computers {
            if std::ptr::eq(*from, *to) {
                continue;
            }
            for src in &sources {
                let rel = src.strip_prefix(&rel_root).unwrap();
                let dst = to.sync_dir.join(rel);
                std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
                std::fs::copy(src, dst).unwrap();
            }
        }
    }
}

/// The bytes and modified time of every file of another computer's.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, (Vec<u8>, SystemTime)> {
    files(dir)
        .into_iter()
        .map(|p| {
            let bytes = std::fs::read(&p).unwrap();
            let mtime = std::fs::metadata(&p).unwrap().modified().unwrap();
            (p, (bytes, mtime))
        })
        .collect()
}

/// Fails if any file in `sync_dir` holds a user name, host name, or full
/// path of this computer.
fn assert_private(sync_dir: &Path, also: &[&Path]) {
    let mut needles: Vec<String> = local_names()
        .into_iter()
        .filter(|n| n.chars().count() >= 3)
        .collect();
    let mut paths: Vec<PathBuf> = vec![sync_dir.to_owned(), std::env::temp_dir()];
    paths.extend(also.iter().map(|p| p.to_path_buf()));
    for var in ["USERPROFILE", "HOME"] {
        if let Ok(v) = std::env::var(var)
            && v.len() > 3
        {
            paths.push(PathBuf::from(v));
        }
    }
    for p in paths {
        let s = p.to_string_lossy().trim_end_matches(['/', '\\']).to_owned();
        needles.push(s.replace('\\', "\\\\"));
        needles.push(s.replace('\\', "/"));
        needles.push(s);
    }
    for f in files(sync_dir) {
        let text = String::from_utf8_lossy(&std::fs::read(&f).unwrap()).to_lowercase();
        for n in &needles {
            assert!(
                !text.contains(&n.to_lowercase()),
                "a local name or path appears in a file in the sync folder"
            );
        }
    }
}

#[test]
fn three_computers_editing_offline_converge() {
    let mut laptop = Computer::new("laptop");
    let mut lab = Computer::new("lab");
    let mut desk = Computer::new("desk");
    let t = wall_ms();

    // The laptop makes a note, and everyone gets it.
    let s = laptop.stamp(t);
    laptop
        .record
        .notes
        .set("n1", s, note("n1", "Mitosis: first draft"));
    laptop.sync();
    propagate(&[&laptop, &lab, &desk]);
    lab.sync();
    desk.sync();
    assert_eq!(
        lab.record.notes.get("n1").unwrap().note,
        "Mitosis: first draft"
    );

    // Offline: the desk deletes the note, the laptop edits it, and the lab
    // edits it last. Each also reads, bookmarks, and highlights.
    let s = desk.stamp(t + 1000);
    desk.record.notes.delete("n1", s);
    let s = laptop.stamp(t + 2000);
    laptop
        .record
        .notes
        .set("n1", s, note("n1", "Mitosis: the laptop's edit"));
    let s = lab.stamp(t + 3000);
    lab.record
        .notes
        .set("n1", s, note("n1", "Mitosis: the lab's edit"));
    for (i, c) in [&mut laptop, &mut lab, &mut desk].into_iter().enumerate() {
        let n = u64::try_from(i).unwrap();
        let s = c.stamp(t + 4000 + n);
        c.record.set_place(s, place(100 * (i + 1)));
        let s = c.stamp(t + 4100 + n);
        c.record
            .bookmarks
            .set(format!("b{i}"), s, bookmark(&format!("mark{i}"), 10 * i));
        let s = c.stamp(t + 4200 + n);
        c.record.highlights.set(
            format!("h{i}"),
            s,
            Highlight {
                id: format!("h{i}"),
                range: CharRange::new(CharPos(i), CharPos(i + 4)),
                color: "yellow".into(),
                text: "Cell".into(),
                ts: 1_790_000_000,
                extra: serde_json::Map::new(),
            },
        );
        let device = c.folder.device();
        c.record.stats.seconds.add(device, 60 * (n + 1));
        c.record.stats.sessions.add(device, 1);
        c.record.stats.furthest_percent.raise(10 * (n + 1));
    }

    // Each writes what it has, the service runs, and each merges and writes
    // again; a second round changes nothing.
    for c in [&mut laptop, &mut lab, &mut desk] {
        c.folder.write_doc(&c.record).unwrap();
    }
    propagate(&[&laptop, &lab, &desk]);
    let laptop_seen = laptop.sync();
    let lab_seen = lab.sync();
    let desk_seen = desk.sync();
    propagate(&[&laptop, &lab, &desk]);
    for c in [&mut laptop, &mut lab, &mut desk] {
        let again = c.sync();
        assert!(again.problems.is_empty(), "{:?}", again.problems);
    }
    assert_eq!(laptop.record, lab.record);
    assert_eq!(lab.record, desk.record);

    // The lab's later edit won everywhere; the laptop hears its note was
    // replaced and keeps the text; the desk hears its deletion was undone.
    assert_eq!(
        laptop.record.notes.get("n1").unwrap().note,
        "Mitosis: the lab's edit"
    );
    let replaced: Vec<_> = laptop_seen.report.replaced_notes().collect();
    assert_eq!(replaced.len(), 1);
    assert_eq!(replaced[0].1.note, "Mitosis: the laptop's edit");
    assert_eq!(replaced[0].2, lab.folder.device());
    assert_eq!(
        desk_seen.report.count(ItemKind::Note, ChangeKind::Restored),
        1
    );
    assert_eq!(
        lab_seen.report.count(ItemKind::Note, ChangeKind::Replaced),
        0
    );
    // Every computer's place, bookmark, and highlight; counts summed.
    assert_eq!(laptop.record.places.live_len(), 3);
    assert_eq!(laptop.record.bookmarks.live_len(), 3);
    assert_eq!(laptop.record.highlights.live_len(), 3);
    assert_eq!(laptop.record.stats.seconds.total(), 60 + 120 + 180);
    assert_eq!(laptop.record.stats.sessions.total(), 3);
    assert_eq!(laptop.record.stats.furthest_percent.0, 30);
    assert_eq!(
        laptop.record.place_of(desk.folder.device()),
        Some(&place(300))
    );

    for c in [&laptop, &lab, &desk] {
        assert_private(&c.sync_dir, &[&c.state_dir]);
    }
}

#[test]
fn another_computers_files_are_never_changed() {
    let mut laptop = Computer::new("laptop");
    let mut lab = Computer::new("lab");
    let s = lab.stamp(wall_ms());
    lab.record.notes.set("n1", s, note("n1", "From the lab"));
    lab.sync();
    propagate(&[&laptop, &lab]);
    let theirs = laptop
        .sync_dir
        .join(SYNC_DIR)
        .join(DEVICES_DIR)
        .join(lab.folder.device().to_string());
    let before = snapshot(&theirs);
    assert!(!before.is_empty());
    for _ in 0..3 {
        let s = laptop.stamp(wall_ms());
        laptop.record.set_place(s, place(5));
        laptop.sync();
    }
    // Reopening writes device.json only in the laptop's own folder.
    let _ = SyncFolder::open(&laptop.sync_dir, &mut laptop.identity, "laptop", "0.2.0").unwrap();
    assert_eq!(snapshot(&theirs), before);
    assert_eq!(laptop.record.notes.get("n1").unwrap().note, "From the lab");
    assert_private(&laptop.sync_dir, &[&laptop.state_dir]);
}

#[test]
fn damaged_and_partial_files_are_skipped_and_the_last_good_copy_kept() {
    let mut laptop = Computer::new("laptop");
    let mut lab = Computer::new("lab");
    let s = lab.stamp(wall_ms());
    lab.record.notes.set("n1", s, note("n1", "Kept"));
    lab.sync();
    propagate(&[&laptop, &lab]);
    assert!(laptop.sync().problems.is_empty());

    let theirs = laptop
        .sync_dir
        .join(SYNC_DIR)
        .join(DEVICES_DIR)
        .join(lab.folder.device().to_string())
        .join(DOCS_DIR)
        .join(format!("{DOC}.json"));
    let good = std::fs::read(&theirs).unwrap();
    let other_doc = DocRecord::new(SyncId::from_u128(7)).to_bytes().unwrap();
    let bad: [&[u8]; 5] = [
        &good[..good.len() / 2],
        b"",
        b"\x00\x01\x02 not json",
        b"{\"format\":1,\"sync_id\":\"zz\"}",
        &other_doc,
    ];
    for (i, bytes) in bad.iter().enumerate() {
        // The service delivers a cut-short or damaged copy.
        std::fs::write(&theirs, bytes).unwrap();
        let merged = laptop.sync();
        assert_eq!(merged.problems.len(), 1, "case {i}: {:?}", merged.problems);
        assert!(
            matches!(
                &merged.problems[0],
                Problem::Damaged { device: Some(d), file: FileKind::Doc(id), kept_last_good: true, .. }
                    if *d == lab.folder.device() && *id == DOC
            ),
            "case {i}: {:?}",
            merged.problems
        );
        assert!(
            merged.problems[0]
                .to_string()
                .starts_with("Damaged file skipped")
        );
        assert_eq!(laptop.record.notes.get("n1").unwrap().note, "Kept");
    }

    // A fresh session has no last good copy: the file is still only
    // skipped and reported.
    let (mut identity, e) = Identity::load_or_create(&laptop.state_dir).unwrap();
    assert_eq!(e, IdentityEvent::Loaded);
    let mut fresh = SyncFolder::open(&laptop.sync_dir, &mut identity, "laptop", VERSION)
        .unwrap()
        .folder;
    std::fs::write(&theirs, &good[..10]).unwrap();
    let mut record = DocRecord::new(DOC);
    let merged = fresh.merge_doc(&mut record, &mut laptop.clock);
    assert!(matches!(
        &merged.problems[0],
        Problem::Damaged {
            kept_last_good: false,
            ..
        }
    ));

    // Temporary files and stray folders are ignored.
    let docs = laptop.own_dir().join(DOCS_DIR);
    std::fs::write(docs.join(format!(".{DOC}.json.99.0.tmp")), b"{\"part").unwrap();
    std::fs::create_dir_all(
        laptop
            .sync_dir
            .join(SYNC_DIR)
            .join(DEVICES_DIR)
            .join("notes"),
    )
    .unwrap();
    assert_eq!(fresh.doc_ids().into_iter().collect::<Vec<_>>(), [DOC]);
    assert_eq!(fresh.device_ids().len(), 2);
}

#[test]
fn a_newer_format_stays_read_only() {
    let mut lab = Computer::new("lab");
    let s = lab.stamp(wall_ms());
    lab.record.notes.set("n1", s, note("n1", "Readable"));
    lab.sync();

    let format = lab.sync_dir.join(SYNC_DIR).join(FORMAT_FILE);
    std::fs::write(&format, b"{\"format\": 2, \"new_field\": true}").unwrap();
    let before = snapshot(&lab.sync_dir);

    let state = tempfile::tempdir().unwrap();
    let (mut identity, _) = Identity::load_or_create(state.path()).unwrap();
    let opened = SyncFolder::open(&lab.sync_dir, &mut identity, "laptop", VERSION).unwrap();
    let mut folder = opened.folder;
    assert_eq!(folder.read_only(), Some(ReadOnly::NewerFormat { found: 2 }));
    assert!(
        opened
            .problems
            .contains(&Problem::ReadOnly(ReadOnly::NewerFormat { found: 2 }))
    );
    // It still reads what it can.
    let mut clock = Clock::new(identity.device());
    let mut record = DocRecord::new(DOC);
    let merged = folder.merge_doc(&mut record, &mut clock);
    assert!(merged.problems.is_empty());
    assert_eq!(record.notes.get("n1").unwrap().note, "Readable");
    // And writes nothing: no record, no device.json, format.json untouched.
    assert!(matches!(
        folder.write_doc(&record),
        Err(SyncError::ReadOnly(ReadOnly::NewerFormat { found: 2 }))
    ));
    assert_eq!(snapshot(&lab.sync_dir), before);

    // A damaged format.json is read-only too.
    std::fs::write(&format, b"{\"form").unwrap();
    let opened = SyncFolder::open(&lab.sync_dir, &mut identity, "laptop", VERSION).unwrap();
    assert_eq!(opened.folder.read_only(), Some(ReadOnly::UnreadableFormat));
}

#[test]
fn a_record_from_a_newer_textweaver_is_skipped() {
    let mut laptop = Computer::new("laptop");
    let lab = Computer::new("lab");
    propagate(&[&laptop, &lab]);
    let theirs = laptop
        .sync_dir
        .join(SYNC_DIR)
        .join(DEVICES_DIR)
        .join(lab.folder.device().to_string())
        .join(DOCS_DIR)
        .join(format!("{DOC}.json"));
    std::fs::create_dir_all(theirs.parent().unwrap()).unwrap();
    std::fs::write(&theirs, format!("{{\"format\":9,\"sync_id\":\"{DOC}\"}}")).unwrap();
    let merged = laptop.sync();
    assert_eq!(
        merged.problems,
        [Problem::NewerFormat {
            device: lab.folder.device(),
            file: FileKind::Doc(DOC),
            found: 9
        }]
    );
}

#[test]
fn a_copied_state_folder_takes_a_fresh_device_id() {
    let mut laptop = Computer::new("laptop");
    let s = laptop.stamp(wall_ms());
    laptop.record.set_place(s, place(1));
    laptop.sync();

    // The whole state folder is copied to another computer, which uses the
    // same sync folder.
    let copy = tempfile::tempdir().unwrap();
    for f in files(&laptop.state_dir) {
        let rel = f.strip_prefix(&laptop.state_dir).unwrap();
        std::fs::copy(&f, copy.path().join(rel)).unwrap();
    }
    let (mut other, e) = Identity::load_or_create(copy.path()).unwrap();
    assert_eq!(e, IdentityEvent::CopiedStateFolder);
    assert_ne!(other.device(), laptop.folder.device());
    let opened = SyncFolder::open(&laptop.sync_dir, &mut other, "lab", VERSION).unwrap();
    assert!(!opened.fresh_device_id);
    assert_eq!(opened.folder.device_ids().len(), 2);

    // A copy the fingerprint cannot catch (the same marker in the same
    // place) is caught by device.json: another installation's token.
    let before = snapshot(&laptop.own_dir());
    let (mut twin, e) = Identity::load_or_create(&laptop.state_dir).unwrap();
    assert_eq!(e, IdentityEvent::Loaded);
    let device_json = laptop.own_dir().join(DEVICE_FILE);
    let text = std::fs::read_to_string(&device_json).unwrap();
    let token = laptop.identity.token().to_string();
    let forged = text.replace(&token, &"f".repeat(32));
    std::fs::write(&device_json, &forged).unwrap();
    let opened = SyncFolder::open(&laptop.sync_dir, &mut twin, "laptop", VERSION).unwrap();
    assert!(opened.fresh_device_id);
    assert_ne!(twin.device(), laptop.folder.device());
    assert_eq!(opened.folder.device_ids().len(), 3);
    // The other installation's folder is untouched, except for the forged
    // device.json this test wrote.
    let mut after = snapshot(&laptop.own_dir());
    let mut expected = before;
    after.remove(&device_json);
    expected.remove(&device_json);
    assert_eq!(after, expected);
    assert_eq!(std::fs::read_to_string(&device_json).unwrap(), forged);
    assert_private(&laptop.sync_dir, &[&laptop.state_dir, copy.path()]);
}

#[test]
fn a_clock_far_ahead_is_named_once() {
    let mut laptop = Computer::new("laptop");
    let mut lab = Computer::new("lab");
    let far = wall_ms() + 3 * 24 * 3_600_000;
    let s = lab.stamp(far);
    lab.record.notes.set("n1", s, note("n1", "From the future"));
    lab.sync();
    propagate(&[&laptop, &lab]);
    let first = laptop.sync();
    assert!(matches!(
        first.problems.as_slice(),
        [Problem::ClockAhead(w)] if w.device == lab.folder.device()
    ));
    assert!(laptop.sync().problems.is_empty(), "reported once");
    let ahead = laptop.folder.clocks_ahead();
    assert_eq!(ahead.len(), 1);
    assert_eq!(ahead[0].device, lab.folder.device());
    // The laptop's next edit still comes after the future stamp.
    assert!(laptop.clock.tick() > s);
}

#[test]
fn the_privacy_scan_catches_a_path() {
    let laptop = Computer::new("laptop");
    assert_private(&laptop.sync_dir, &[&laptop.state_dir]);
    let leak = laptop.own_dir().join("leak.json");
    let path = serde_json::to_string(&laptop.state_dir.join("x")).unwrap();
    std::fs::write(&leak, path).unwrap();
    let caught =
        std::panic::catch_unwind(|| assert_private(&laptop.sync_dir, &[&laptop.state_dir]));
    assert!(
        caught.is_err(),
        "a full path in the sync folder was not caught"
    );
}

#[test]
fn labels_are_checked_when_opening() {
    let state = tempfile::tempdir().unwrap();
    let sync = tempfile::tempdir().unwrap();
    let (mut identity, _) = Identity::load_or_create(state.path()).unwrap();
    assert!(matches!(
        SyncFolder::open(sync.path(), &mut identity, "   ", VERSION),
        Err(SyncError::Label(_))
    ));
    for name in local_names() {
        assert!(SyncFolder::open(sync.path(), &mut identity, &name, VERSION).is_err());
    }
    let missing = sync.path().join("unplugged");
    assert!(matches!(
        SyncFolder::open(&missing, &mut identity, "lab", VERSION),
        Err(SyncError::FolderMissing)
    ));
}

/// Group files (settings, word lists, and the rest; sync wave S5): each
/// computer writes only its own, merges everyone's, and a damaged or
/// newer one is skipped and reported, the last good copy used.
#[test]
fn group_files_merge_and_damaged_ones_are_skipped() {
    use textweaver_sync::{GroupFile, GroupRecord};
    let mut laptop = Computer::new("laptop");
    let mut lab = Computer::new("lab");
    let mut mine = GroupRecord::new();
    let s = laptop.stamp(wall_ms());
    mine.set_mut("words").insert("mitochondrion", s);
    laptop.folder.write_group(GroupFile::Words, &mine).unwrap();
    let mut theirs = GroupRecord::new();
    let s = lab.stamp(wall_ms());
    theirs.set_mut("words").insert("ribosome", s);
    lab.folder.write_group(GroupFile::Words, &theirs).unwrap();
    propagate(&[&laptop, &lab]);

    let (changes, problems) =
        laptop
            .folder
            .merge_group(GroupFile::Words, &mut mine, &mut laptop.clock);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].by, lab.folder.device());
    let words: Vec<&str> = mine.set("words").unwrap().items().collect();
    assert_eq!(words, ["mitochondrion", "ribosome"]);

    // The lab's file arrives cut short, then from a newer textweaver.
    let path = laptop
        .sync_dir
        .join(SYNC_DIR)
        .join(DEVICES_DIR)
        .join(lab.folder.device().to_string())
        .join(GroupFile::Words.file_name());
    let good = std::fs::read(&path).unwrap();
    std::fs::write(&path, &good[..good.len() / 2]).unwrap();
    let (records, problems) = laptop.folder.read_group(GroupFile::Words);
    assert!(
        matches!(
            &problems[..],
            [Problem::Damaged {
                file: FileKind::Group(GroupFile::Words),
                kept_last_good: true,
                ..
            }]
        ),
        "{problems:?}"
    );
    assert_eq!(records.len(), 2, "the last good copy is used");
    std::fs::write(&path, b"{\"format\": 99}").unwrap();
    let (_, problems) = laptop.folder.read_group(GroupFile::Words);
    assert!(
        matches!(
            &problems[..],
            [Problem::NewerFormat {
                found: 99,
                file: FileKind::Group(GroupFile::Words),
                ..
            }]
        ),
        "{problems:?}"
    );
}
