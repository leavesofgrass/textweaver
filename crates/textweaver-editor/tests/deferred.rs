//! Saves and snapshots whose file I/O happens elsewhere (the app's
//! background writer): the session stays correct while the disk catches up.

use std::time::{Duration, Instant};

use textweaver_editor::autosave::{self, AutosavePolicy};
use textweaver_editor::{DocInfo, EditSession, SaveOutcome, SaveStart, Selection, SnapshotOp};

fn session(dir: &std::path::Path, text: &str) -> EditSession {
    let path = dir.join("essay.md");
    std::fs::write(&path, text).unwrap();
    let doc = DocInfo {
        key: "essay".into(),
        path: Some(path),
        loader_id: "markdown".into(),
        title: "Essay".into(),
    };
    let mut s = EditSession::new(doc, text)
        .with_autosave(AutosavePolicy::new(true, 5), dir.join("recovery"));
    s.set_deferred_io(true);
    s.enter_edit();
    let end = textweaver_core::CharPos(text.chars().count());
    s.select(Selection::caret(end)).unwrap();
    s
}

#[test]
fn typing_during_a_save_stays_unsaved() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = session(dir.path(), "one");
    s.editor_mut().unwrap().type_text(" two").unwrap();
    let SaveStart::Write(req) = s.begin_save(None).unwrap() else {
        panic!("an in-place save needs no path");
    };
    assert_eq!(req.text.to_string(), "one two");
    // The user types while the writer works.
    s.editor_mut().unwrap().type_text(" three").unwrap();
    let text = req.text.to_string();
    autosave::save_text(&req.dest, &text).unwrap();
    let out = s.finish_save(req, text);
    assert!(matches!(out, SaveOutcome::Saved { adopted: false, .. }));
    assert_eq!(s.document_text(), "one two");
    assert!(
        s.is_dirty(),
        "the words typed during the save are not saved"
    );
    // Undoing back to the saved text makes it clean.
    while s.editor().unwrap().text() != "one two" {
        assert!(s.undo().unwrap());
    }
    assert!(!s.is_dirty());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("essay.md")).unwrap(),
        "one two"
    );
}

#[test]
fn a_save_that_began_before_a_new_document_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = session(dir.path(), "old");
    s.editor_mut().unwrap().type_text("!").unwrap();
    let SaveStart::Write(req) = s.begin_save(None).unwrap() else {
        panic!("in place");
    };
    s.load(DocInfo::untitled("new"), "fresh");
    s.enter_edit();
    let _ = s.finish_save(req, "old!".into());
    assert_eq!(s.document_text(), "fresh");
    assert!(!s.is_dirty());
}

#[test]
fn snapshots_are_queued_in_order_and_one_write_is_in_flight() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = session(dir.path(), "text");
    s.editor_mut().unwrap().type_text(" more").unwrap();
    let t0 = Instant::now();
    assert!(!s.autosave_tick(t0 + Duration::from_secs(6)).unwrap());
    let ops = s.take_snapshot_ops();
    assert_eq!(ops.len(), 1);
    let SnapshotOp::Write { snapshot, .. } = &ops[0] else {
        panic!("a write");
    };
    assert_eq!(snapshot.to_snapshot().text, "text more");
    // No second write while the first is in flight.
    assert!(!s.autosave_tick(t0 + Duration::from_secs(12)).unwrap());
    assert!(s.take_snapshot_ops().is_empty());
    s.snapshot_written(&Err(std::io::Error::other("disk full")));
    assert_eq!(s.snapshot_failures(), 1);
    // After a failure the wait from the attempt doubles (5 s to 10 s).
    assert!(!s.autosave_tick(t0 + Duration::from_secs(15)).unwrap());
    assert!(s.take_snapshot_ops().is_empty());
    s.autosave_tick(t0 + Duration::from_secs(16)).unwrap();
    assert_eq!(s.take_snapshot_ops().len(), 1);
    s.snapshot_written(&Ok(true));
    assert_eq!(s.snapshot_failures(), 0);
    // Saving clears the snapshot through the writer too, after the write.
    let SaveStart::Write(req) = s.begin_save(None).unwrap() else {
        panic!("in place");
    };
    let _ = s.finish_save(req, "text more".into());
    let ops = s.take_snapshot_ops();
    assert!(
        matches!(&ops[..], [SnapshotOp::Delete { doc_key, .. }] if doc_key == "essay"),
        "{ops:?}"
    );
    assert!(
        !dir.path().join("recovery").exists(),
        "nothing touched the disk"
    );
}
