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
