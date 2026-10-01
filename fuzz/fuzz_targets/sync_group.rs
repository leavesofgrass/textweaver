//! Fuzz target: the sync group file reader (ADR-0049). The bytes are read
//! as one of a computer's group files (`settings.json`, `profiles.json`,
//! `keymap.json`, `words.json`, `glossary.json`, `voices.json`). Nothing
//! may panic. A file that reads must write back and read again the same;
//! merged with itself or with an empty record it must not change, merging
//! it into an empty record must give the record back, and two halves of
//! it merged in either order must agree.

#![no_main]

use libfuzzer_sys::fuzz_target;
use textweaver_sync::GroupRecord;

fuzz_target!(|data: &[u8]| {
    let Ok(record) = GroupRecord::from_bytes(data) else {
        return;
    };
    let bytes = record.to_bytes().expect("a record that reads writes");
    let again = GroupRecord::from_bytes(&bytes).expect("a written record reads back");
    assert_eq!(again, record);

    let mut twice = record.clone();
    let changes = twice.merge(&record);
    assert!(changes.is_empty(), "merging a record with itself changed it");
    assert_eq!(twice, record);

    let empty = GroupRecord::new();
    let mut with_empty = record.clone();
    assert!(with_empty.merge(&empty).is_empty());
    assert_eq!(with_empty, record);
    let mut into_empty = empty.clone();
    into_empty.merge(&record);
    assert_eq!(into_empty, record);

    // The maps and the sets as two records, merged both ways.
    let maps = GroupRecord {
        sets: Default::default(),
        ..record.clone()
    };
    let sets = GroupRecord {
        maps: Default::default(),
        ..record.clone()
    };
    let mut ab = maps.clone();
    ab.merge(&sets);
    let mut ba = sets.clone();
    ba.merge(&maps);
    assert_eq!(ab, ba);
    assert_eq!(ab, record);

    let _ = record.stamps();
    let _ = record.is_empty();
});
