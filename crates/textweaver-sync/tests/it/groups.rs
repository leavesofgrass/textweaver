//! The group file fixtures (`fixtures/w7s`, also the `sync_group` fuzz
//! target's seeds) read, write back the same, and merge in any order to
//! one result.

use textweaver_sync::GroupRecord;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/w7s");

fn fixtures() -> Vec<(String, GroupRecord)> {
    let mut out: Vec<(String, GroupRecord)> = std::fs::read_dir(FIXTURES)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let bytes = std::fs::read(e.path()).unwrap();
            let record = GroupRecord::from_bytes(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
            (name, record)
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[test]
fn the_group_fixtures_read_and_write_back_the_same() {
    let all = fixtures();
    assert!(
        all.len() >= 4,
        "{:?}",
        all.iter().map(|f| &f.0).collect::<Vec<_>>()
    );
    for (name, record) in &all {
        assert!(!record.is_empty(), "{name}");
        let back = GroupRecord::from_bytes(&record.to_bytes().unwrap()).unwrap();
        assert_eq!(&back, record, "{name}");
        let mut twice = record.clone();
        assert!(twice.merge(record).is_empty(), "{name}");
    }
}

/// The nightly's `sync_group` crash (Thursday, October 1, 2026): a file
/// saying format 0, merged into an empty record, came back as format 1.
/// An older format reads as this one, as a document's record does
/// (ADR-0049), so the merge has nothing to change.
#[test]
fn an_older_format_reads_as_this_one_and_merges_unchanged() {
    let crash = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/w7f/sync_group/format-0.json"
    ))
    .unwrap();
    let record = GroupRecord::from_bytes(&crash).unwrap();
    assert_eq!(record.format, textweaver_sync::FORMAT);
    assert_eq!(
        GroupRecord::from_bytes(&record.to_bytes().unwrap()).unwrap(),
        record
    );
    let mut into_empty = GroupRecord::new();
    into_empty.merge(&record);
    assert_eq!(into_empty, record);

    // The same with something in it: a fixture written as format 0.
    let settings = std::fs::read_to_string(format!("{FIXTURES}/group-settings.json")).unwrap();
    let older = settings.replacen("\"format\":1", "\"format\":0", 1);
    assert_ne!(older, settings, "the fixture names its format");
    let record = GroupRecord::from_bytes(older.as_bytes()).unwrap();
    assert_eq!(record.format, textweaver_sync::FORMAT);
    assert!(!record.is_empty());
    let mut into_empty = GroupRecord::new();
    into_empty.merge(&record);
    assert_eq!(into_empty, record);
}

#[test]
fn the_group_fixtures_merge_in_any_order_to_one_result() {
    let all: Vec<GroupRecord> = fixtures().into_iter().map(|(_, r)| r).collect();
    let mut forward = GroupRecord::new();
    for r in &all {
        forward.merge(r);
    }
    let mut backward = GroupRecord::new();
    for r in all.iter().rev() {
        backward.merge(r);
    }
    assert_eq!(forward, backward);
    // The removal recorded in the word list wins over the earlier add.
    let words: Vec<&str> = forward.set("words").unwrap().items().collect();
    assert_eq!(words, ["mitochondrion"]);
}
