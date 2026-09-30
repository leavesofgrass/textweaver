//! Fuzz target: the sync record reader (ADR-0049). The bytes are read as a
//! computer's `docs/<sync-id>.json`. Nothing may panic. A record that reads
//! must write back and read again the same; merged with itself or with an
//! empty record it must not change, and merging it into an empty record
//! must give the record back, in either order.

#![no_main]

use libfuzzer_sys::fuzz_target;
use textweaver_sync::DocRecord;

fuzz_target!(|data: &[u8]| {
    let Ok(record) = DocRecord::from_bytes(data) else {
        return;
    };
    let bytes = record.to_bytes().expect("a record that reads writes");
    let again = DocRecord::from_bytes(&bytes).expect("a written record reads back");
    assert_eq!(again, record);

    let mut twice = record.clone();
    let report = twice.merge(&record).expect("same document");
    assert!(report.is_empty(), "merging a record with itself changed it");
    assert_eq!(twice, record);

    let empty = DocRecord::new(record.sync_id);
    let mut with_empty = record.clone();
    with_empty.merge(&empty).expect("same document");
    assert_eq!(with_empty, record);
    let mut into_empty = empty.clone();
    into_empty.merge(&record).expect("same document");
    assert_eq!(into_empty, record);

    let _ = record.places_newest_first();
    let _ = record.stamps().count();
});
