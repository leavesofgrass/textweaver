//! The read-only view of the sync folder and the library details (the sync
//! wave, S6): details merge newest wins per detail, "first added" keeps the
//! earliest, and reading the folder this way writes nothing.

use std::path::Path;

use textweaver_store::Paths;
use textweaver_store::sync_ids::HashKind;
use textweaver_sync::docid::{Details, Fingerprint};
use textweaver_sync::record::detail;
use textweaver_sync::{Clock, DocRecord, FolderView, Identity, SyncError, SyncFolder, SyncId};

struct Home {
    _dir: tempfile::TempDir,
    folder: SyncFolder,
    clock: Clock,
}

fn home(label: &str, sync: &Path) -> Home {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    let (mut identity, _) = Identity::load_or_create(&paths.data_dir).unwrap();
    let folder = SyncFolder::open(sync, &mut identity, label, "test")
        .unwrap()
        .folder;
    let clock = Clock::new(identity.device());
    Home {
        _dir: dir,
        folder,
        clock,
    }
}

impl Home {
    fn publish(&mut self, id: SyncId, details: &Details) {
        let mut record = DocRecord::new(id);
        self.folder.merge_doc(&mut record, &mut self.clock);
        let stamp = self.clock.tick();
        let fp = Fingerprint {
            text_sha256: Some(format!("{:064x}", 7)),
            ..Fingerprint::default()
        };
        record.publish_identity(stamp, &fp, details);
        self.folder.write_doc(&record).unwrap();
    }
}

/// Every file under `dir`, with its bytes, to show nothing was written.
fn snapshot(dir: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_owned()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push((p.clone(), std::fs::read(&p).unwrap()));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn details_merge_newest_wins_and_first_added_keeps_the_earliest() {
    let sync = tempfile::tempdir().unwrap();
    let mut laptop = home("laptop", sync.path());
    let mut lab = home("lab", sync.path());
    let id = SyncId::random();

    laptop.publish(
        id,
        &Details {
            title: Some("Cells".into()),
            author: Some("Ada Example".into()),
            format: Some("markdown".into()),
            added_ms: Some(5_000),
            ..Details::default()
        },
    );
    // Later, the lab learns the DOI and a corrected author; it added the
    // document later than the laptop did.
    lab.publish(
        id,
        &Details {
            author: Some("Ada Example and Bo Example".into()),
            doi: Some("10.1000/xyz".into()),
            added_ms: Some(9_000),
            ..Details::default()
        },
    );

    let before = snapshot(sync.path());
    let view = FolderView::read(sync.path()).unwrap();
    assert_eq!(snapshot(sync.path()), before, "reading writes nothing");
    assert_eq!(view.skipped, 0);
    assert_eq!(view.labels.len(), 2);
    let r = view.doc(id).unwrap();
    let d = |name| r.identity.detail(name);
    assert_eq!(d(detail::TITLE), Some("Cells"), "kept where not changed");
    assert_eq!(
        d(detail::AUTHOR),
        Some("Ada Example and Bo Example"),
        "newest"
    );
    assert_eq!(d(detail::DOI), Some("10.1000/xyz"));
    assert_eq!(d(detail::FORMAT), Some("markdown"));
    assert_eq!(r.identity.added.0, Some(5_000), "the earliest first added");
    assert_eq!(view.find(HashKind::Text, &format!("{:064x}", 7)), Some(id));

    // Unchanged details would change nothing; a new one or an earlier
    // first added would.
    let same = Details {
        title: Some("Cells".into()),
        added_ms: Some(6_000),
        ..Details::default()
    };
    assert!(!same.would_change(&r.identity));
    let earlier = Details {
        added_ms: Some(1_000),
        ..Details::default()
    };
    assert!(earlier.would_change(&r.identity));
    let isbn = Details {
        isbn: Some("9780306406157".into()),
        ..Details::default()
    };
    assert!(isbn.would_change(&r.identity));
}

#[test]
fn a_damaged_record_is_skipped_and_a_missing_folder_is_said() {
    let sync = tempfile::tempdir().unwrap();
    let mut laptop = home("laptop", sync.path());
    let id = SyncId::random();
    laptop.publish(id, &Details::default());
    let docs = sync
        .path()
        .join("textweaver-sync")
        .join("devices")
        .join(laptop.folder.device().to_string())
        .join("docs");
    std::fs::write(
        docs.join(format!("{}.json", SyncId::random())),
        b"{\"format\":1,",
    )
    .unwrap();
    let view = FolderView::read(sync.path()).unwrap();
    assert_eq!(view.docs.len(), 1);
    assert_eq!(view.skipped, 1);

    let empty = tempfile::tempdir().unwrap();
    assert!(FolderView::read(empty.path()).unwrap().docs.is_empty());
    assert!(matches!(
        FolderView::read(&empty.path().join("gone")),
        Err(SyncError::FolderMissing)
    ));
}
