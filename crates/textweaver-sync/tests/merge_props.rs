//! Property tests: the order in which records are merged, and merging one
//! twice, never change the result.

use proptest::prelude::*;
use textweaver_core::{CharPos, CharRange};
use textweaver_store::{Bookmark, Note};
use textweaver_sync::{AddWinsSet, DeviceId, DocRecord, Place, Stamp, SyncId};

const DOC: SyncId = SyncId::from_u128(0x5eed);

fn device(n: u8) -> DeviceId {
    DeviceId::from_u128(u128::from(n) + 1)
}

/// One change a computer makes.
#[derive(Clone, Debug)]
enum Op {
    Note { id: u8, text: u8 },
    DeleteNote { id: u8 },
    Bookmark { id: u8, pos: u16 },
    DeleteBookmark { id: u8 },
    Place { pos: u16 },
    Read { seconds: u16 },
    /// Publishes content (0), text (1), or library (2) hash number `hash`;
    /// there are more hashes than a record keeps, so trimming is tested.
    Hash { kind: u8, hash: u8 },
    /// Sets the title (0), DOI (1), or ISBN (2).
    Detail { which: u8, value: u8 },
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0..4u8, 0..4u8).prop_map(|(id, text)| Op::Note { id, text }),
        (0..4u8).prop_map(|id| Op::DeleteNote { id }),
        (0..3u8, any::<u16>()).prop_map(|(id, pos)| Op::Bookmark { id, pos }),
        (0..3u8).prop_map(|id| Op::DeleteBookmark { id }),
        any::<u16>().prop_map(|pos| Op::Place { pos }),
        any::<u16>().prop_map(|seconds| Op::Read { seconds }),
        (0..3u8, 0..12u8).prop_map(|(kind, hash)| Op::Hash { kind, hash }),
        (0..3u8, 0..3u8).prop_map(|(which, value)| Op::Detail { which, value }),
    ]
}

fn note(id: u8, text: u8) -> Note {
    Note {
        id: format!("n{id}"),
        range: CharRange::new(CharPos(0), CharPos(4)),
        anchor: "Cell".into(),
        note: format!("text {text}"),
        tags: Vec::new(),
        cite: String::new(),
        color: None,
        relations: Vec::new(),
        created: 1,
        ts: 1,
        extra: serde_json::Map::new(),
    }
}

/// A computer's record after `ops`, each at a small time so stamps from
/// different computers often tie on time (and even the same computer's,
/// which a damaged file could hold).
fn record(dev: u8, ops: &[(u8, Op)]) -> DocRecord {
    let d = device(dev);
    let mut r = DocRecord::new(DOC);
    for (t, op) in ops {
        let s = Stamp::new(u64::from(*t), d);
        match op {
            Op::Note { id, text } => r.notes.set(format!("n{id}"), s, note(*id, *text)),
            Op::DeleteNote { id } => r.notes.delete(format!("n{id}"), s),
            Op::Bookmark { id, pos } => r.bookmarks.set(
                format!("b{id}"),
                s,
                Bookmark {
                    id: format!("b{id}"),
                    name: format!("mark{id}"),
                    pos: CharPos(usize::from(*pos)),
                    pct: 0,
                    ts: 0,
                    anchor: None,
                    not_found: false,
                    extra: serde_json::Map::new(),
                },
            ),
            Op::DeleteBookmark { id } => r.bookmarks.delete(format!("b{id}"), s),
            Op::Place { pos } => r.set_place(
                s,
                Place {
                    pos: CharPos(usize::from(*pos)),
                    pct: 0,
                    anchor: None,
                },
            ),
            Op::Read { seconds } => {
                r.stats.seconds.add(d, u64::from(*seconds));
                r.stats.sessions.add(d, 1);
                r.stats.furthest_char.raise(u64::from(*seconds));
            }
            Op::Hash { kind, hash } => {
                let h = format!("{hash:064x}");
                let set = match kind {
                    0 => &mut r.identity.content,
                    1 => &mut r.identity.text,
                    _ => &mut r.identity.library,
                };
                set.publish(&h, s);
            }
            Op::Detail { which, value } => {
                let name = ["title", "doi", "isbn"][usize::from(*which) % 3];
                r.identity.details.set(name, s, format!("v{value}"));
            }
        }
    }
    r
}

fn records() -> impl Strategy<Value = Vec<DocRecord>> {
    prop::collection::vec(prop::collection::vec((0..8u8, op()), 0..12), 1..5).prop_map(|per| {
        per.iter()
            .enumerate()
            .map(|(i, ops)| record(u8::try_from(i % 3).unwrap_or(0), ops))
            .collect()
    })
}

fn merged(order: &[&DocRecord]) -> DocRecord {
    let mut out = DocRecord::new(DOC);
    for r in order {
        out.merge(r).unwrap();
    }
    out
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn merge_is_commutative(rs in records()) {
        let a = &rs[0];
        let b = rs.last().unwrap();
        prop_assert_eq!(merged(&[a, b]), merged(&[b, a]));
    }

    #[test]
    fn merge_is_idempotent(rs in records()) {
        let once = merged(&rs.iter().collect::<Vec<_>>());
        let mut twice = once.clone();
        for r in &rs {
            let report = twice.merge(r).unwrap();
            prop_assert!(report.is_empty(), "a repeat listed changes: {:?}", report);
        }
        prop_assert_eq!(&twice, &once);
        let mut self_merge = once.clone();
        self_merge.merge(&once).unwrap();
        prop_assert_eq!(self_merge, once);
    }

    #[test]
    fn any_order_and_grouping_gives_the_same_result(rs in records(), seed in any::<u64>()) {
        let forward = merged(&rs.iter().collect::<Vec<_>>());
        let backward = merged(&rs.iter().rev().collect::<Vec<_>>());
        prop_assert_eq!(&forward, &backward);
        // A shuffled order, with some records merged twice.
        let mut order: Vec<&DocRecord> = rs.iter().chain(rs.iter().step_by(2)).collect();
        let mut x = seed | 1;
        for i in (1..order.len()).rev() {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let j = usize::try_from(x % (i as u64 + 1)).unwrap_or(0);
            order.swap(i, j);
        }
        prop_assert_eq!(&merged(&order), &forward);
        // Grouped: (a + b) + (c + ...) equals the flat merge.
        let (left, right) = rs.split_at(rs.len() / 2);
        let l = merged(&left.iter().collect::<Vec<_>>());
        let r = merged(&right.iter().collect::<Vec<_>>());
        prop_assert_eq!(&merged(&[&l, &r]), &forward);
    }

    #[test]
    fn sets_merge_in_any_order(
        ops in prop::collection::vec((0..3u8, 0..6u8, 0..4u8, any::<bool>()), 0..24),
    ) {
        let mut sets = [AddWinsSet::new(), AddWinsSet::new(), AddWinsSet::new()];
        for (dev, t, item, add) in &ops {
            let s = Stamp::new(u64::from(*t), device(*dev));
            let set = &mut sets[usize::from(*dev)];
            if *add { set.insert(format!("w{item}"), s) } else { set.remove(format!("w{item}"), s) }
        }
        let mut ab = sets[0].clone();
        ab.merge(&sets[1]);
        ab.merge(&sets[2]);
        let mut ba = sets[2].clone();
        ba.merge(&sets[1]);
        ba.merge(&sets[0]);
        ba.merge(&sets[0]);
        prop_assert_eq!(ab, ba);
    }
}
