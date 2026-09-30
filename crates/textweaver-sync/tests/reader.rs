//! The record reader on hostile input: the same checks as the
//! `sync_record` fuzz target, run on the seed fixture, its truncations, and
//! random changes to it, so they hold without a nightly toolchain.

use proptest::prelude::*;
use textweaver_sync::{DocRecord, SyncError};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/s1/record.json");

fn fixture() -> Vec<u8> {
    std::fs::read(FIXTURE).unwrap()
}

/// The fuzz target's checks.
fn check(data: &[u8]) {
    let Ok(record) = DocRecord::from_bytes(data) else {
        return;
    };
    let bytes = record.to_bytes().unwrap();
    assert_eq!(DocRecord::from_bytes(&bytes).unwrap(), record);
    let mut twice = record.clone();
    assert!(twice.merge(&record).unwrap().is_empty());
    assert_eq!(twice, record);
    let empty = DocRecord::new(record.sync_id);
    let mut with_empty = record.clone();
    with_empty.merge(&empty).unwrap();
    assert_eq!(with_empty, record);
    let mut into_empty = empty;
    into_empty.merge(&record).unwrap();
    assert_eq!(into_empty, record);
}

#[test]
fn the_seed_reads_and_holds_every_kind_of_item() {
    let r = DocRecord::from_bytes(&fixture()).unwrap();
    assert_eq!(r.places.live_len(), 2);
    assert_eq!(r.bookmarks.live_len(), 1);
    assert!(r.bookmarks.register("b2").is_some(), "a deletion record");
    assert_eq!(r.notes.get("n1").unwrap().note, "Mitosis happens here.");
    assert_eq!(r.highlights.live_len(), 1);
    assert_eq!(r.stats.seconds.total(), 720);
    check(&fixture());
}

#[test]
fn every_truncation_is_refused_without_a_panic() {
    let f = fixture();
    let end = f.iter().rposition(|b| *b == b'}').unwrap();
    for cut in 0..end {
        assert!(DocRecord::from_bytes(&f[..cut]).is_err(), "cut at {cut}");
    }
}

#[test]
fn an_older_format_reads_as_this_one() {
    let text = String::from_utf8(fixture())
        .unwrap()
        .replacen("\"format\":1", "\"format\":0", 1);
    let r = DocRecord::from_bytes(text.as_bytes()).unwrap();
    assert_eq!(r.format, textweaver_sync::FORMAT);
    check(text.as_bytes());
    let newer =
        String::from_utf8(fixture())
            .unwrap()
            .replacen("\"format\":1", "\"format\":4294967296", 1);
    assert!(matches!(
        DocRecord::from_bytes(newer.as_bytes()),
        Err(SyncError::NewerFormat { found: u32::MAX })
    ));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn random_bytes_never_panic(data in prop::collection::vec(any::<u8>(), 0..512)) {
        check(&data);
    }

    #[test]
    fn changed_seeds_never_panic(
        edits in prop::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..6),
    ) {
        let mut f = fixture();
        for (at, byte) in edits {
            let i = at.index(f.len());
            f[i] = byte;
        }
        check(&f);
    }
}
