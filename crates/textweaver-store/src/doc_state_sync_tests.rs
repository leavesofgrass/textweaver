//! State format 2 (the sync wave, S3): bookmark ids, deletion records, the
//! backup of replaced notes, and loading every older state file unchanged.
//! Merges here use the store's own functions ([`DocState::merge_marks`]),
//! never the sync crate.

use serde_json::Value;
use textweaver_core::{CharPos, CharRange, Edit};

use super::*;

/// State files as earlier versions wrote them (format 1, no `format` key),
/// in `fixtures/s3`: `(name, text)`.
const OLD_FILES: [(&str, &str); 3] = [
    (
        "state-wave1",
        include_str!("../../../fixtures/s3/state-wave1.json"),
    ),
    (
        "state-wave2-notes",
        include_str!("../../../fixtures/s3/state-wave2-notes.json"),
    ),
    (
        "state-phase2-anchors",
        include_str!("../../../fixtures/s3/state-phase2-anchors.json"),
    ),
];

/// Loads `text` as the state file of a document through a real store.
fn load_through_store(text: &str) -> (tempfile::TempDir, StateStore, DocKey, DocState) {
    let dir = tempfile::tempdir().unwrap();
    let key = DocKey::untitled(1);
    std::fs::write(dir.path().join(format!("{}.json", key.0)), text).unwrap();
    let store = StateStore::new(dir.path().to_owned());
    let state = store.load(&key).expect("an old state file loads");
    (dir, store, key, state)
}

/// The written form of `state` without what format 2 adds to an old file
/// (the `format` key and bookmark ids), to compare with the old file.
fn without_format_2_additions(state: &DocState) -> Value {
    let mut v = serde_json::to_value(state).unwrap();
    let obj = v.as_object_mut().unwrap();
    assert_eq!(obj.remove("format"), Some(Value::from(STATE_FORMAT)));
    if let Some(Value::Array(marks)) = obj.get_mut("bookmarks") {
        for b in marks {
            let id = b.as_object_mut().unwrap().remove("id").unwrap();
            assert!(id.as_str().unwrap().starts_with("bm-"), "{id}");
        }
    }
    v
}

#[test]
fn every_older_state_file_loads_unchanged() {
    for (name, text) in OLD_FILES {
        let original: Value = serde_json::from_str(text).unwrap();
        let (_dir, store, key, state) = load_through_store(text);
        assert_eq!(state.format, LEGACY_STATE_FORMAT, "{name}");
        // Everything the file held is still there, value for value; the
        // only additions are the format number and the bookmark ids.
        assert_eq!(without_format_2_additions(&state), original, "{name}");
        // The old file on disk is not touched by loading it.
        let on_disk = std::fs::read_to_string(store.dir().join(format!("{}.json", key.0))).unwrap();
        assert_eq!(on_disk, text, "{name}");
        // Loading twice gives the same ids; saving and loading again
        // changes nothing but the format number.
        let again = store.load(&key).unwrap();
        assert_eq!(again, state, "{name}");
        store.save(&key, &state).unwrap();
        let saved = store.load(&key).unwrap();
        assert_eq!(saved.format, STATE_FORMAT, "{name}");
        assert_eq!(
            DocState {
                format: LEGACY_STATE_FORMAT,
                ..saved
            },
            state,
            "{name}"
        );
    }
}

#[test]
fn old_note_and_highlight_ids_are_kept_as_they_are() {
    let (_dir, _store, _key, state) = load_through_store(OLD_FILES[1].1);
    let ids: Vec<&str> = state.notes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, vec!["1a2b3c4d", "star-9f8e7d6c"]);
    let ids: Vec<&str> = state.highlights.iter().map(|h| h.id.as_str()).collect();
    assert_eq!(ids, vec!["0badf00d", "star-h-12345678"]);
    assert_eq!(state.notes[0].extra["sr_state"]["interval"], 3);
    // A new note beside them gets a 64-bit id.
    let mut state = state;
    let n = state.add_note(CharRange::new(1, 2), "x", "new", "");
    assert_eq!(n.id.len(), 16);
}

#[test]
fn a_migrated_bookmark_keeps_its_name_and_place() {
    let (_dir, store, key, state) = load_through_store(OLD_FILES[0].1);
    let marks: Vec<(&str, usize, u8, i64)> = state
        .bookmarks
        .iter()
        .map(|b| (b.name.as_str(), b.pos.0, b.pct, b.ts))
        .collect();
    assert_eq!(
        marks,
        vec![
            ("intro", 0, 0, 1_789_990_000),
            ("mark1", 300, 60, 1_789_995_000)
        ]
    );
    let mark1 = state.bookmark("mark1").unwrap();
    assert_eq!(
        mark1.id,
        Bookmark::legacy_id("mark1", CharPos(300), 1_789_995_000)
    );
    assert_ne!(mark1.id, state.bookmark("intro").unwrap().id);
    // The id is written on the next save and kept from then on.
    store.save(&key, &state).unwrap();
    let text = std::fs::read_to_string(store.dir().join(format!("{}.json", key.0))).unwrap();
    assert!(text.contains(&mark1.id), "{text}");
    assert!(text.contains("\"format\": 2"), "{text}");
    let back = store.load(&key).unwrap();
    assert_eq!(back.bookmark("mark1").unwrap().id, mark1.id);
    // Moving it with the same name keeps the id.
    let mut back = back;
    let moved = back.add_bookmark(Some("mark1"), CharPos(10), 500);
    assert_eq!(moved.id, mark1.id);
    assert_eq!(back.bookmarks.len(), 2);
}

#[test]
fn a_new_state_writes_format_2_and_a_newer_format_is_kept() {
    let st = DocState::default();
    assert_eq!(st.format, STATE_FORMAT);
    let v = serde_json::to_value(&st).unwrap();
    assert_eq!(v["format"], 2);
    assert!(v.get("deleted").is_none() && v.get("note_backups").is_none());
    // A file from a newer version loads, keeps its keys and its number.
    let newer = r#"{"format": 7, "position": 3, "sealed_marks": [1, 2]}"#;
    let st: DocState = serde_json::from_str(newer).unwrap();
    assert_eq!(st.format, 7);
    let v = serde_json::to_value(&st).unwrap();
    assert_eq!(v["format"], 7);
    assert_eq!(v["sealed_marks"], serde_json::json!([1, 2]));
}

/// A note made a while ago, so deletions and edits made "now" are later.
fn note_from_before(st: &mut DocState, text: &str) -> Note {
    let mut n = st.add_note(CharRange::new(10, 20), "anchored text", text, "#t");
    n.created = crate::now_ts() - 100;
    n.ts = n.created;
    st.insert_note(n.clone());
    n
}

#[test]
fn a_deleted_note_stays_deleted_after_a_merge() {
    // Two computers with the same note; the laptop deletes it.
    let mut laptop = DocState::default();
    let n = note_from_before(&mut laptop, "Mitochondria");
    let mut lab = laptop.clone();
    assert!(laptop.remove_note(&n.id));
    assert!(laptop.deleted.get(MarkKind::Note, &n.id).is_some());

    // The lab's copy, which still has the note, arrives at the laptop:
    // the note does not come back.
    let report = laptop.merge_marks(&lab, "lab");
    assert!(laptop.note(&n.id).is_none(), "{report:?}");
    assert!(report.added.is_empty());

    // The laptop's copy arrives at the lab: the lab's note goes, and its
    // text is kept in the lab's backup.
    let report = lab.merge_marks(&laptop, "laptop");
    assert!(lab.note(&n.id).is_none());
    assert_eq!(report.deleted, vec![(MarkKind::Note, n.id.clone())]);
    let kept = lab.backups_of_note(&n.id);
    assert_eq!(kept.len(), 1);
    assert!(kept[0].deleted);
    assert_eq!(kept[0].note.note, "Mitochondria");

    // Merging again, either way, changes nothing.
    assert!(lab.merge_marks(&laptop, "laptop").is_empty());
    assert!(laptop.merge_marks(&lab, "lab").is_empty());

    // Survives a save and load, as a deletion record in the file.
    let dir = tempfile::tempdir().unwrap();
    let store = StateStore::new(dir.path().to_owned());
    let key = DocKey::untitled(2);
    store.save(&key, &laptop).unwrap();
    let mut loaded = store.load(&key).unwrap();
    assert!(loaded.merge_marks(&lab, "lab").is_empty());
    assert!(loaded.note(&n.id).is_none());
}

#[test]
fn an_edit_after_a_deletion_brings_the_note_back() {
    let mut laptop = DocState::default();
    let n = note_from_before(&mut laptop, "Old text");
    let mut lab = laptop.clone();
    laptop.remove_note(&n.id);
    // The lab edits the note after the laptop deleted it.
    let mut edited = lab.note(&n.id).unwrap().clone();
    edited.note = "Edited later".into();
    edited.ts = crate::now_ts() + 60;
    lab.insert_note(edited);
    let report = laptop.merge_marks(&lab, "lab");
    assert_eq!(laptop.note(&n.id).unwrap().note, "Edited later");
    assert_eq!(report.added, vec![(MarkKind::Note, n.id.clone())]);
}

#[test]
fn deletions_and_edits_compare_by_clock_stamp_when_items_carry_one() {
    let mut st = DocState::default();
    let mut n = note_from_before(&mut st, "stamped");
    let deletion = ClockStamp {
        wall_ms: 5_000,
        counter: 1,
        device: "b".into(),
    };
    st.remove_note(&n.id);
    st.record_deletion_at(MarkKind::Note, &n.id, deletion.clone());
    // A later local stamp from before is kept: the later one wins.
    assert!(st.deleted.get(MarkKind::Note, &n.id).unwrap().clock > deletion);

    let mut other = DocState::default();
    let mut deletions = Deletions::default();
    deletions.record(MarkKind::Note, &n.id, deletion.clone());
    other.deleted = deletions;
    // The item's own stamp is one counter later than the deletion: it wins.
    n.ts = 0;
    n.extra.insert(
        "clock".into(),
        serde_json::to_value(ClockStamp {
            wall_ms: 5_000,
            counter: 2,
            device: "a".into(),
        })
        .unwrap(),
    );
    other.insert_note(n.clone());
    let mut here = DocState::default();
    here.merge_marks(&other, "other");
    assert!(
        here.note(&n.id).is_some(),
        "the edit is later than the deletion"
    );
    // One counter earlier: the deletion wins.
    n.extra.insert(
        "clock".into(),
        serde_json::to_value(ClockStamp {
            wall_ms: 5_000,
            counter: 0,
            device: "a".into(),
        })
        .unwrap(),
    );
    other.insert_note(n.clone());
    let mut here = DocState::default();
    here.merge_marks(&other, "other");
    assert!(here.note(&n.id).is_none());
    // Written with the stamp's three fields.
    let v = serde_json::to_value(&other.deleted).unwrap();
    assert_eq!(v["notes"][0]["clock"]["wall_ms"], 5_000);
    assert_eq!(v["notes"][0]["clock"]["counter"], 1);
    assert_eq!(v["notes"][0]["clock"]["device"], "b");
}

#[test]
fn a_replaced_note_can_be_listed_and_restored() {
    let mut laptop = DocState::default();
    let n = note_from_before(&mut laptop, "Laptop words");
    let mut lab = laptop.clone();
    // The lab edits the note later than the laptop's version (but before
    // now, so a restore here is later still).
    let mut edited = lab.note(&n.id).unwrap().clone();
    edited.note = "Lab words".into();
    edited.ts = n.ts + 10;
    lab.insert_note(edited);

    let report = laptop.merge_marks(&lab, "lab");
    assert_eq!(report.replaced_notes, vec![n.id.clone()]);
    assert_eq!(laptop.note(&n.id).unwrap().note, "Lab words");
    // The laptop's own words are listed, newest first, with who replaced
    // them.
    let kept = laptop.backups_of_note(&n.id);
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].note.note, "Laptop words");
    assert_eq!(kept[0].by, "lab");
    assert!(!kept[0].deleted);
    assert_eq!(laptop.note_backups_newest_first().len(), 1);
    // The backup never travels: merging it elsewhere does not copy it.
    let mut third = DocState::default();
    third.merge_marks(&laptop, "laptop");
    assert!(third.note_backups.is_empty());

    // Restoring puts the words back as a new edit and keeps the lab's
    // words in turn.
    let backup_id = kept[0].id.clone();
    let restored = laptop.restore_note_backup(&backup_id).unwrap();
    assert_eq!(restored.note, "Laptop words");
    assert_eq!(laptop.note(&n.id).unwrap().note, "Laptop words");
    assert_eq!(laptop.backups_of_note(&n.id)[0].note.note, "Lab words");
    assert!(laptop.restore_note_backup(&backup_id).is_none());
    // The restore is newer than the lab's edit, so it wins both ways.
    assert!(laptop.merge_marks(&lab, "lab").replaced_notes.is_empty());
    assert_eq!(laptop.note(&n.id).unwrap().note, "Laptop words");
    let report = lab.merge_marks(&laptop, "laptop");
    assert_eq!(report.replaced_notes, vec![n.id.clone()]);
    assert_eq!(lab.note(&n.id).unwrap().note, "Laptop words");
    // Backups survive a save and load.
    let dir = tempfile::tempdir().unwrap();
    let store = StateStore::new(dir.path().to_owned());
    store.save(&DocKey::untitled(3), &laptop).unwrap();
    let back = store.load(&DocKey::untitled(3)).unwrap();
    assert_eq!(back.note_backups, laptop.note_backups);
}

#[test]
fn the_backup_keeps_the_last_twenty_versions() {
    let mut st = DocState::default();
    let n = note_from_before(&mut st, "v");
    for i in 0..25 {
        let old = Note {
            note: format!("version {i}"),
            ts: i,
            ..n.clone()
        };
        st.backup_note(&old, "lab", false);
        // Each backup a second apart, so the order is known.
        if let Some(b) = st.note_backups.last_mut() {
            b.replaced = i;
        }
    }
    assert_eq!(st.note_backups.len(), NOTE_BACKUPS_MAX);
    assert_eq!(NOTE_BACKUPS_MAX, 20);
    let newest: Vec<String> = st
        .note_backups_newest_first()
        .iter()
        .map(|b| b.note.note.clone())
        .collect();
    assert_eq!(newest.first().map(String::as_str), Some("version 24"));
    assert_eq!(newest.last().map(String::as_str), Some("version 5"));
    // The same version is kept once.
    let again = Note {
        note: "version 24".into(),
        ts: 24,
        ..n.clone()
    };
    st.backup_note(&again, "lab", false);
    assert_eq!(st.note_backups.len(), NOTE_BACKUPS_MAX);
}

#[test]
fn a_duplicate_bookmark_name_from_another_computer_is_renamed() {
    let mut laptop = DocState::default();
    let mut lab = DocState::default();
    let mine = laptop.add_bookmark(Some("mark1"), CharPos(10), 100);
    let theirs = lab.add_bookmark(Some("mark1"), CharPos(50), 100);
    assert_ne!(mine.id, theirs.id);

    let report = laptop.merge_marks(&lab, "lab");
    assert_eq!(
        report.renamed_bookmarks,
        vec![("mark1".to_owned(), "mark1, lab".to_owned())]
    );
    assert_eq!(laptop.bookmark("mark1").unwrap().id, mine.id);
    let renamed = laptop.bookmark("mark1, lab").unwrap();
    assert_eq!(
        (renamed.id.as_str(), renamed.pos),
        (theirs.id.as_str(), CharPos(50))
    );
    // Merging the same copy again changes nothing.
    assert!(laptop.merge_marks(&lab, "lab").is_empty());
    assert_eq!(laptop.bookmarks.len(), 2);
    // A third mark1 from the same computer gets a number.
    let mut lab2 = DocState::default();
    lab2.add_bookmark(Some("mark1"), CharPos(70), 100);
    laptop.merge_marks(&lab2, "lab");
    assert!(laptop.bookmark("mark1, lab 2").is_some());
    // With no name for the other computer, it says so in words.
    let mut other = DocState::default();
    other.add_bookmark(Some("mark1"), CharPos(90), 100);
    laptop.merge_marks(&other, " ");
    assert!(laptop.bookmark("mark1, other computer").is_some());
}

#[test]
fn a_deleted_bookmark_and_highlight_stay_deleted() {
    let mut laptop = DocState::default();
    let b = laptop.add_bookmark(Some("intro"), CharPos(5), 100);
    let h = laptop
        .add_highlight(CharRange::new(20, 30), "yellow", "words")
        .unwrap();
    for m in &mut laptop.bookmarks {
        m.ts -= 100;
    }
    for x in &mut laptop.highlights {
        x.ts -= 100;
    }
    let lab = laptop.clone();
    assert!(laptop.remove_bookmark("intro"));
    assert!(laptop.remove_highlight(&h.id).is_some());
    laptop.merge_marks(&lab, "lab");
    assert!(laptop.bookmark_by_id(&b.id).is_none());
    assert!(laptop.highlight(&h.id).is_none());
    let mut lab = lab;
    let report = lab.merge_marks(&laptop, "laptop");
    assert!(lab.bookmarks.is_empty() && lab.highlights.is_empty());
    assert_eq!(report.deleted.len(), 2);
}

#[test]
fn removing_in_every_way_records_the_deletion() {
    let mut st = DocState::default();
    let a = st
        .add_highlight(CharRange::new(0, 5), "yellow", "a")
        .unwrap();
    // Highlighting the same range again keeps the id.
    let again = st.add_highlight(CharRange::new(0, 5), "pink", "a").unwrap();
    assert_eq!(again.id, a.id);
    assert!(st.deleted.is_empty());
    let b = st
        .add_highlight(CharRange::new(10, 15), "yellow", "b")
        .unwrap();
    let c = st
        .add_highlight(CharRange::new(30, 35), "yellow", "c")
        .unwrap();
    // An edit that deletes the highlighted text.
    st.shift(&Edit::delete(9..16).outcome());
    assert!(st.deleted.get(MarkKind::Highlight, &b.id).is_some());
    assert_eq!(st.clear_highlights(), 2);
    assert!(st.deleted.get(MarkKind::Highlight, &a.id).is_some());
    assert!(st.deleted.get(MarkKind::Highlight, &c.id).is_some());
    // A note deleted by emptying its text, and one at the cursor.
    let n1 = st.add_note(CharRange::new(1, 3), "x", "one", "");
    let n2 = st.add_note(CharRange::new(40, 44), "y", "two", "");
    assert!(st.edit_note(&n1.id, " ", ""));
    assert!(st.remove_annotation_at(CharPos(41)).is_some());
    assert!(st.deleted.get(MarkKind::Note, &n1.id).is_some());
    assert!(st.deleted.get(MarkKind::Note, &n2.id).is_some());
    // Records survive a round trip and old ones can be pruned.
    let back: DocState = serde_json::from_str(&serde_json::to_string(&st).unwrap()).unwrap();
    assert_eq!(back.deleted, st.deleted);
    let mut pruned = back.deleted.clone();
    assert_eq!(pruned.prune(0), 0);
    assert_eq!(pruned.prune(i64::MAX), 5);
    assert!(pruned.is_empty());
}

/// Only what travels: the marks and their deletion records.
fn synced(st: &DocState) -> (Vec<Note>, Vec<Highlight>, Vec<Bookmark>, Deletions) {
    let mut d = st.deleted.clone();
    for list in [&mut d.notes, &mut d.highlights, &mut d.bookmarks] {
        list.sort_by(|a, b| a.id.cmp(&b.id));
    }
    let mut b = st.bookmarks.clone();
    b.sort_by(|x, y| x.id.cmp(&y.id));
    let mut n = st.notes.clone();
    n.sort_by(|x, y| x.id.cmp(&y.id));
    let mut h = st.highlights.clone();
    h.sort_by(|x, y| x.id.cmp(&y.id));
    (n, h, b, d)
}

#[test]
fn three_computers_converge_in_any_order() {
    let now = crate::now_ts();
    let mut base = DocState::default();
    let shared = note_from_before(&mut base, "shared");
    let doomed = note_from_before(&mut base, "doomed");
    let mut a = base.clone();
    let mut b = base.clone();
    let mut c = base.clone();
    // a edits the shared note; b edits it later; c deletes the doomed one
    // and adds a bookmark; a adds a highlight.
    let mut e = shared.clone();
    e.note = "from a".into();
    e.ts = now - 50;
    a.insert_note(e.clone());
    e.note = "from b".into();
    e.ts = now - 40;
    b.insert_note(e);
    c.remove_note(&doomed.id);
    c.add_bookmark(Some("from c"), CharPos(7), 100);
    a.add_highlight(CharRange::new(3, 9), "green", "hl");

    let orders: [[&DocState; 3]; 3] = [[&a, &b, &c], [&c, &b, &a], [&b, &a, &c]];
    let mut results = Vec::new();
    for order in orders {
        let mut here = DocState::default();
        for other in order {
            here.merge_marks(other, "x");
        }
        // Merging everything a second time changes nothing.
        for other in order {
            assert!(here.merge_marks(other, "x").is_empty());
        }
        results.push(synced(&here));
    }
    assert_eq!(results[0], results[1]);
    assert_eq!(results[0], results[2]);
    let (notes, highlights, bookmarks, _) = &results[0];
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].note, "from b");
    assert_eq!(highlights.len(), 1);
    assert_eq!(bookmarks.len(), 1);
}

#[test]
fn versions_compare_by_clock_stamp_when_they_carry_one() {
    let mut a = DocState::default();
    let n = note_from_before(&mut a, "first");
    let stamp = |counter| {
        serde_json::to_value(ClockStamp {
            wall_ms: 9_000,
            counter,
            device: "d".into(),
        })
        .unwrap()
    };
    let mut older = n.clone();
    older.note = "older by the clock".into();
    older.extra.insert("clock".into(), stamp(1));
    let mut newer = n.clone();
    newer.note = "newer by the clock".into();
    newer.extra.insert("clock".into(), stamp(2));
    let mut x = DocState::default();
    x.insert_note(older);
    let mut y = DocState::default();
    y.insert_note(newer);
    // The same `ts`; the clock decides, whichever side merges.
    let mut xy = x.clone();
    xy.merge_marks(&y, "y");
    let mut yx = y.clone();
    yx.merge_marks(&x, "x");
    assert_eq!(xy.note(&n.id).unwrap().note, "newer by the clock");
    assert_eq!(yx.note(&n.id).unwrap().note, "newer by the clock");
}
