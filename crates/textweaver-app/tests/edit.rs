//! Edit mode, notes, highlights, bookmark management, periodic saves, and
//! `highlight.lead_words`, driven through `App::dispatch` without a
//! terminal (Agent D2).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use textweaver_app::a11y::{Announcer, Priority};
use textweaver_app::core::{CharPos, CharRange, Direction};
use textweaver_app::keymap::ActionId;
use textweaver_app::store::{DocKey, Paths, SettingsStore, StateStore};
use textweaver_app::testing::{SpeechLog, recording_service};
use textweaver_app::text::{Document, GoTo};
use textweaver_app::{
    App, AppConfig, CaretMove, Command, Confirm, Effect, Mode, NoteCommand, Playback, PromptPurpose,
};

#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Said {
    fn all(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
    fn last(&self) -> String {
        self.all().last().cloned().unwrap_or_default()
    }
    fn any(&self, needle: &str) -> bool {
        self.all().iter().any(|s| s.contains(needle))
    }
    fn clear(&self) {
        self.0.lock().unwrap().clear();
    }
}

impl Announcer for Said {
    fn announce(&mut self, text: &str, _priority: Priority) {
        self.0.lock().unwrap().push(text.to_owned());
    }
}

struct Rig {
    app: App,
    log: SpeechLog,
    said: Said,
    paths: Paths,
    _dir: tempfile::TempDir,
    dir: PathBuf,
}

fn launch_in(dir: &Path, paths: &Paths) -> (App, SpeechLog, Said) {
    let said = Said::default();
    let (speech, log) = recording_service().unwrap();
    let settings = SettingsStore::new(paths.clone()).load().0;
    let app = App::new(AppConfig {
        settings,
        speech,
        paths: Some(paths.clone()),
        announcer: Box::new(said.clone()),
        self_voicing: true,
        backend_name: "test-recording".into(),
        ..AppConfig::for_tests()
    });
    let _ = dir;
    (app, log, said)
}

fn rig() -> Rig {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_owned();
    let paths = Paths::under(&dir.join("home"));
    let (app, log, said) = launch_in(&dir, &paths);
    Rig {
        app,
        log,
        said,
        paths,
        _dir: tmp,
        dir,
    }
}

impl Rig {
    fn file(&self, name: &str, text: &str) -> PathBuf {
        let p = self.dir.join(name);
        std::fs::write(&p, text).unwrap();
        p
    }
    fn act(&mut self, a: ActionId) -> Vec<Effect> {
        self.app.dispatch(Command::Action(a))
    }
    /// Quit, answering yes to "Quit textweaver? y or n".
    fn quit(&mut self) -> Vec<Effect> {
        assert_eq!(self.act(ActionId::Quit), vec![Effect::Redraw]);
        assert_eq!(self.app.pending_confirmation(), Some(ActionId::Quit));
        self.app.dispatch(Command::Confirm(Confirm::Yes))
    }
    fn text(&self) -> String {
        self.app.session().unwrap().doc.text().to_string()
    }
    fn go(&mut self, pos: CharPos) {
        self.app.dispatch(Command::GoTo(GoTo::Char(pos)));
    }
    fn type_str(&mut self, s: &str) {
        for c in s.chars() {
            self.app.dispatch(Command::Insert(c.to_string()));
        }
    }
    fn relaunch(&mut self) {
        let (app, log, said) = launch_in(&self.dir, &self.paths);
        self.app = app;
        self.log = log;
        self.said = said;
    }
    fn wait_idle(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(30);
        while self.app.playback() != Playback::Idle {
            self.app.poll_speech();
            assert!(Instant::now() < deadline, "speech never finished");
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

fn at(text: &str, needle: &str) -> CharPos {
    let byte = text.find(needle).unwrap();
    CharPos(text[..byte].chars().count())
}

fn bookmark_pos(app: &App) -> CharPos {
    app.session().unwrap().bookmarks[0].pos
}

const NOTE_MD: &str = "# Title\n\nHello world. The end.\n";

/// The acceptance script: type, format, undo, save, reopen, with a
/// bookmark riding along through every step.
#[test]
fn type_format_undo_save_and_reopen() {
    let mut r = rig();
    let file = r.file("note.md", NOTE_MD);
    r.app.open(&file).unwrap();
    let canon = r.text();
    assert!(!canon.contains('#'), "{canon}");
    r.go(at(&canon, "world"));
    r.act(ActionId::AddBookmark);

    // Enter edit mode: the Markdown source is on screen, and the bookmark
    // is on "world" in it.
    r.act(ActionId::ToggleEditMode);
    assert_eq!(r.app.mode(), Mode::Edit);
    assert!(r.app.is_editing());
    assert_eq!(r.text(), NOTE_MD);
    assert_eq!(bookmark_pos(&r.app), at(NOTE_MD, "world"));
    assert!(r.said.any("Edit mode on"), "{:?}", r.said.all());

    // Type after "Hello": echoed, and the bookmark shifts.
    r.go(at(NOTE_MD, " world"));
    r.log.clear();
    r.type_str(" there");
    let edited = "# Title\n\nHello there world. The end.\n";
    assert_eq!(r.text(), edited);
    assert_eq!(bookmark_pos(&r.app), at(edited, "world"));
    assert!(r.app.is_dirty());
    wait_until(|| {
        let t = r.log.texts();
        t.iter().any(|x| x == "Hello") && t.iter().any(|x| x == "t")
    });
    let texts = r.log.texts();
    // The space completed "Hello"; the letters were spoken one by one.
    assert!(texts.iter().any(|t| t == "Hello"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "t"), "{texts:?}");

    // Select "there" and make it bold; the markup counts as an edit.
    let there = at(edited, "there");
    r.app
        .dispatch(Command::Select(CharRange::new(there, CharPos(there.0 + 5))));
    r.act(ActionId::Bold);
    let bold = "# Title\n\nHello **there** world. The end.\n";
    assert_eq!(r.text(), bold);
    assert!(r.said.last().starts_with("Bold."), "{}", r.said.last());
    assert_eq!(bookmark_pos(&r.app), at(bold, "world"));

    // Undo and redo, each announced; undo twice more removes the typing.
    r.act(ActionId::Undo);
    assert_eq!(r.text(), edited);
    assert!(r.said.last().starts_with("Undo."), "{}", r.said.last());
    r.act(ActionId::Redo);
    assert_eq!(r.text(), bold);
    assert!(r.said.last().starts_with("Redo."));

    // Save in place: the file holds the source, and editing continues.
    r.act(ActionId::Save);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), bold);
    assert_eq!(r.said.last(), "Saved note.md. Still editing.");
    assert!(r.app.is_editing());
    assert!(!r.app.is_dirty());

    // Leave edit mode: the reading view is rebuilt from the saved file and
    // the bookmark is back on "world" in the canonical text.
    r.act(ActionId::ToggleEditMode);
    assert_eq!(r.app.mode(), Mode::Browse);
    let canon = r.text();
    assert!(canon.contains("Hello there world."), "{canon}");
    assert!(!canon.contains("**"));
    assert_eq!(bookmark_pos(&r.app), at(&canon, "world"));
    assert!(r.said.any("Edit mode off."));

    // Quit and reopen: the saved text and the bookmark come back.
    assert_eq!(r.quit(), vec![Effect::Quit]);
    r.relaunch();
    r.app.open(&file).unwrap();
    let canon = r.text();
    assert!(canon.contains("Hello there world."));
    assert_eq!(bookmark_pos(&r.app), at(&canon, "world"));
}

#[test]
fn leaving_with_unsaved_changes_asks_and_discard_restores_everything() {
    let mut r = rig();
    let file = r.file("plain.txt", "one two three\nfour five\n");
    r.app.open(&file).unwrap();
    r.go(CharPos(8));
    r.act(ActionId::AddBookmark);
    r.act(ActionId::ToggleEditMode);
    r.go(CharPos(0));
    r.type_str("zero ");
    assert_eq!(r.text(), "zero one two three\nfour five\n");
    assert_eq!(bookmark_pos(&r.app), CharPos(13));

    // Ctrl+E with changes: Save / Discard / Cancel.
    let effects = r.act(ActionId::ToggleEditMode);
    let Some(Effect::ShowList { items, .. }) = effects.first() else {
        panic!("{effects:?}");
    };
    assert_eq!(items.len(), 3);
    assert!(r.said.last().contains("unsaved changes"));
    // Cancel keeps editing.
    r.app.dispatch(Command::Choose(2));
    assert!(r.app.is_editing());
    assert_eq!(r.said.last(), "Still editing.");
    // Escape on the list keeps editing too.
    r.act(ActionId::ToggleEditMode);
    r.app.dispatch(Command::Cancel);
    assert!(r.app.is_editing());
    // Discard: the text and the bookmark are as they were.
    r.act(ActionId::ToggleEditMode);
    r.app.dispatch(Command::Choose(1));
    assert!(!r.app.is_editing());
    assert_eq!(r.text(), "one two three\nfour five\n");
    assert_eq!(bookmark_pos(&r.app), CharPos(8));
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "one two three\nfour five\n"
    );
    assert!(r.said.any("Changes discarded."));
}

#[test]
fn plain_text_keeps_crlf_and_edits_move_marks_exactly() {
    let mut r = rig();
    let file = r.file("crlf.txt", "alpha beta\r\ngamma delta\r\n");
    r.app.open(&file).unwrap();
    r.go(CharPos(11)); // gamma
    r.act(ActionId::AddBookmark);
    r.act(ActionId::ToggleEditMode);
    // Plain text is edited as is: the bookmark does not move on entry.
    assert_eq!(bookmark_pos(&r.app), CharPos(11));
    r.go(CharPos(6));
    r.app.dispatch(Command::DeleteForward); // "b"
    r.app.dispatch(Command::DeleteForward); // "e"
    assert_eq!(r.text(), "alpha ta\ngamma delta\n");
    assert_eq!(bookmark_pos(&r.app), CharPos(9));
    r.act(ActionId::Save);
    assert_eq!(
        std::fs::read(&file).unwrap(),
        b"alpha ta\r\ngamma delta\r\n".to_vec()
    );
    r.act(ActionId::ToggleEditMode);
    assert_eq!(bookmark_pos(&r.app), CharPos(9));
    assert_eq!(
        r.app.session().unwrap().doc.slice(CharRange::new(9, 14)),
        "gamma"
    );
}

#[test]
fn new_document_saves_as_and_adopts_the_path() {
    let mut r = rig();
    r.act(ActionId::NewDocument);
    assert!(r.app.is_editing());
    assert_eq!(r.text(), "");
    assert!(r.said.any("New document ready for editing."));
    r.type_str("Draft");
    // Save needs a name: the prompt suggests one; a relative answer is
    // taken relative to the suggestion's folder.
    let effects = r.act(ActionId::Save);
    let Some(Effect::Prompt { label, purpose }) = effects.first() else {
        panic!("{effects:?}");
    };
    assert_eq!(*purpose, PromptPurpose::SaveAs);
    assert!(label.starts_with("Save as"), "{label}");
    let target = r.dir.join("draft.md");
    r.app
        .dispatch(Command::Answer(target.display().to_string()));
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "Draft");
    assert_eq!(r.app.session().unwrap().title, "draft.md");
    assert_eq!(r.app.session().unwrap().key, DocKey::for_path(&target));
    // The next save writes in place, without asking.
    r.type_str("!");
    let effects = r.act(ActionId::Save);
    assert!(!effects.iter().any(|e| matches!(e, Effect::Prompt { .. })));
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "Draft!");
}

#[test]
fn quitting_with_changes_saves_on_request() {
    let mut r = rig();
    let file = r.file("q.md", "Some text.\n");
    r.app.open(&file).unwrap();
    r.act(ActionId::ToggleEditMode);
    r.app.dispatch(Command::MoveCaret {
        by: CaretMove::DocumentEdge,
        direction: Direction::Forward,
        extend: false,
    });
    r.type_str("More.");
    let effects = r.quit();
    assert!(matches!(effects.first(), Some(Effect::ShowList { .. })));
    let effects = r.app.dispatch(Command::Choose(0));
    assert_eq!(effects, vec![Effect::Quit]);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "Some text.\nMore.");
}

#[test]
fn autosave_snapshot_is_offered_after_a_crash() {
    let mut r = rig();
    let file = r.file("crash.md", "Before.\n");
    r.app.open(&file).unwrap();
    r.act(ActionId::ToggleEditMode);
    r.type_str("Lost? ");
    // The first tick while dirty writes a snapshot.
    r.app.tick(Instant::now());
    // One snapshot (the lock file beside it guards it; see autosave).
    let snaps: Vec<_> = std::fs::read_dir(r.paths.recovery_dir())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_none_or(|x| x != "lock"))
        .collect();
    assert_eq!(snaps.len(), 1);
    // "Crash": no shutdown, no leaving edit mode.
    r.relaunch();
    let effects = r.app.offer_recovery();
    let Some(Effect::ShowList { title, items }) = effects.first() else {
        panic!("{effects:?}");
    };
    assert!(title.contains("crash.md"), "{title}");
    assert_eq!(items.len(), 2);
    assert!(r.said.last().contains("unsaved changes"));
    r.app.dispatch(Command::Choose(0));
    assert!(r.app.is_editing());
    assert!(r.app.is_dirty());
    assert_eq!(r.text(), "Lost? Before.\n");
    assert!(r.said.any("Remember to save"));
    assert_eq!(
        std::fs::read_dir(r.paths.recovery_dir()).unwrap().count(),
        0,
        "the snapshot is deleted once recovered"
    );
    // The file itself is untouched until saved.
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "Before.\n");
    r.act(ActionId::Save);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "Lost? Before.\n");
}

#[test]
fn declining_recovery_deletes_the_snapshot() {
    let mut r = rig();
    let file = r.file("d.md", "Text.\n");
    r.app.open(&file).unwrap();
    r.act(ActionId::ToggleEditMode);
    r.type_str("x");
    r.app.tick(Instant::now());
    r.relaunch();
    assert!(!r.app.offer_recovery().is_empty());
    r.app.dispatch(Command::Choose(1));
    assert!(!r.app.is_editing());
    assert!(r.said.last().starts_with("Discarded the unsaved changes"));
    assert_eq!(
        std::fs::read_dir(r.paths.recovery_dir()).unwrap().count(),
        0
    );
    // Nothing more to offer.
    assert!(r.app.offer_recovery().is_empty());
}

#[test]
fn formatting_outside_edit_mode_is_refused_and_announced() {
    let mut r = rig();
    let file = r.file("r.md", "Read only.\n");
    r.app.open(&file).unwrap();
    r.act(ActionId::Bold);
    assert!(
        r.said.last().starts_with("Turn on edit mode"),
        "{}",
        r.said.last()
    );
    r.app.dispatch(Command::Insert("x".into()));
    assert!(r.said.last().starts_with("Turn on edit mode"));
    assert!(r.text().starts_with("Read only."));
}

#[test]
fn replace_all_heading_cycle_and_caret_echo() {
    let mut r = rig();
    let file = r.file("cats.md", "cat cat dog\n");
    r.app.open(&file).unwrap();
    r.act(ActionId::ToggleEditMode);
    let effects = r.act(ActionId::Replace);
    assert!(matches!(
        effects.first(),
        Some(Effect::Prompt {
            purpose: PromptPurpose::ReplaceFind,
            ..
        })
    ));
    r.app.dispatch(Command::Answer("cat".into()));
    assert!(r.said.last().contains("2 matches"), "{}", r.said.last());
    r.app.dispatch(Command::Answer("bird".into()));
    assert_eq!(r.text(), "bird bird dog\n");
    assert_eq!(r.said.last(), "Replaced 2 matches.");
    r.act(ActionId::Undo);
    assert_eq!(r.text(), "cat cat dog\n");

    // Heading cycles 1, 2, ... on the caret line.
    r.act(ActionId::Heading);
    assert_eq!(r.text(), "# cat cat dog\n");
    r.act(ActionId::Heading);
    assert_eq!(r.text(), "## cat cat dog\n");
    assert_eq!(r.said.last(), "Heading level 2.");

    // Caret moves speak what they reach.
    r.app.dispatch(Command::MoveCaret {
        by: CaretMove::LineEdge,
        direction: Direction::Backward,
        extend: false,
    });
    r.log.clear();
    r.app.dispatch(Command::MoveCaret {
        by: CaretMove::Word,
        direction: Direction::Forward,
        extend: false,
    });
    wait_until(|| r.log.texts().iter().any(|t| t == "cat"));
    assert!(
        r.log.texts().iter().any(|t| t == "cat"),
        "{:?}",
        r.log.texts()
    );
    r.app.dispatch(Command::MoveCaret {
        by: CaretMove::Word,
        direction: Direction::Forward,
        extend: true,
    });
    assert_eq!(r.said.last(), "cat selected");
    // Backspace deletes the selection.
    r.app.dispatch(Command::DeleteBack);
    assert_eq!(r.text(), "## cat dog\n");
}

#[test]
fn notes_add_list_jump_edit_delete_and_persist() {
    let mut r = rig();
    let text = "First sentence here. Second sentence there. Third one.";
    let file = r.file("n.txt", text);
    r.app.open(&file).unwrap();
    r.go(at(text, "Second"));
    let effects = r.app.dispatch(Command::Notes(NoteCommand::Add));
    assert!(matches!(
        effects.first(),
        Some(Effect::Prompt {
            purpose: PromptPurpose::NoteText,
            ..
        })
    ));
    r.app.dispatch(Command::Answer("Check this #exam".into()));
    assert!(
        r.said.last().starts_with("Note added with tags exam"),
        "{}",
        r.said.last()
    );
    let s = r.app.session().unwrap();
    assert_eq!(s.notes.len(), 1);
    assert_eq!(s.notes[0].tags, vec!["exam"]);
    assert_eq!(s.notes[0].anchor, "Second sentence there.");

    // Jump to it from elsewhere, by next note and from the list.
    r.go(CharPos(0));
    r.app.dispatch(Command::Notes(NoteCommand::Next));
    assert_eq!(r.app.session().unwrap().cursor, at(text, "Second"));
    assert!(r.said.last().contains("Check this #exam"));
    r.go(CharPos(0));
    let effects = r.app.dispatch(Command::Notes(NoteCommand::List));
    let Some(Effect::ShowList { items, .. }) = effects.first() else {
        panic!("{effects:?}");
    };
    assert_eq!(items.len(), 1);
    r.app.dispatch(Command::Choose(0));
    assert_eq!(r.app.session().unwrap().cursor, at(text, "Second"));

    // Edit it through F2 on the list.
    r.app.dispatch(Command::Notes(NoteCommand::List));
    let effects = r.app.dispatch(Command::RenameItem(0));
    assert!(matches!(
        effects.first(),
        Some(Effect::Prompt {
            purpose: PromptPurpose::EditNote,
            ..
        })
    ));
    r.app.dispatch(Command::Answer("Revised #review".into()));
    assert_eq!(r.said.last(), "Note updated.");

    // A highlight on the first sentence.
    r.go(CharPos(0));
    r.app.dispatch(Command::Notes(NoteCommand::ToggleHighlight));
    assert!(
        r.said.last().starts_with("Highlighted"),
        "{}",
        r.said.last()
    );
    let hl = r.app.highlights(CharRange::new(0, text.len()));
    assert!(
        hl.iter()
            .any(|h| h.kind == textweaver_app::HighlightKind::UserHighlight)
    );
    assert!(
        hl.iter()
            .any(|h| h.kind == textweaver_app::HighlightKind::Note)
    );

    // Both survive a relaunch.
    r.quit();
    r.relaunch();
    r.app.open(&file).unwrap();
    let s = r.app.session().unwrap();
    assert_eq!(s.notes.len(), 1);
    assert_eq!(s.notes[0].note, "Revised #review");
    assert_eq!(s.notes[0].tags, vec!["review"]);
    assert_eq!(s.highlights.len(), 1);
    // Stored in the typed fields every other tool reads.
    let on_disk = StateStore::new(r.paths.state_dir())
        .load(&DocKey::for_path(&file))
        .unwrap();
    assert_eq!(on_disk.notes, s.notes);
    assert_eq!(on_disk.highlights[0].color, "#ffff00");
    assert_eq!(on_disk.highlights[0].text, "First sentence here.");

    // Toggling again on the highlight removes it; Delete removes the note.
    r.go(CharPos(2));
    r.app.dispatch(Command::Notes(NoteCommand::ToggleHighlight));
    assert!(r.said.last().starts_with("Highlight removed"));
    r.app.dispatch(Command::Notes(NoteCommand::List));
    let effects = r.app.dispatch(Command::DeleteItem(0));
    assert!(!effects.iter().any(|e| matches!(e, Effect::ShowList { .. })));
    assert!(r.said.last().starts_with("Note deleted"));
    let store = StateStore::new(r.paths.state_dir());
    let state = store.load(&DocKey::for_path(&file)).unwrap();
    assert!(!state.extra.contains_key("app_notes"));
    assert!(state.notes.is_empty() && state.highlights.is_empty());
}

#[test]
fn legacy_app_notes_are_migrated_once_on_open() {
    let mut r = rig();
    let text = "Alpha beta. Gamma delta.";
    let file = r.file("legacy.txt", text);
    // A state file as the first wave 2 build wrote it.
    let store = StateStore::new(r.paths.state_dir());
    let key = DocKey::for_path(&file);
    let mut old = textweaver_app::store::DocState::default();
    old.extra.insert(
        "app_notes".into(),
        serde_json::json!([{
            "id": "0000abcd", "range": {"start": 12, "end": 24},
            "anchor": "Gamma delta.", "text": "Old note #exam", "tags": ["exam"], "ts": 5
        }]),
    );
    old.extra.insert(
        "app_highlights".into(),
        serde_json::json!([{ "range": {"start": 0, "end": 11}, "color": "yellow", "ts": 6 }]),
    );
    store.save(&key, &old).unwrap();
    r.app.open(&file).unwrap();
    let s = r.app.session().unwrap();
    assert_eq!(s.notes.len(), 1);
    assert_eq!(s.notes[0].note, "Old note #exam");
    assert_eq!(s.highlights.len(), 1);
    assert_eq!(s.doc.slice(s.highlights[0].range), "Alpha beta.");
    // The file was rewritten at once: typed fields, no legacy keys.
    let saved = StateStore::new(r.paths.state_dir()).load(&key).unwrap();
    assert!(!saved.extra.contains_key("app_notes"));
    assert!(!saved.extra.contains_key("app_highlights"));
    assert_eq!(saved.notes.len(), 1);
    assert_eq!(saved.highlights.len(), 1);
    // Stepping and deleting work on the migrated note.
    r.go(CharPos(0));
    r.app.dispatch(Command::Notes(NoteCommand::Next));
    assert_eq!(r.app.session().unwrap().cursor, at(text, "Gamma"));
}

#[test]
fn notes_move_with_edits() {
    let mut r = rig();
    let text = "Alpha beta. Gamma delta.";
    let file = r.file("m.txt", text);
    r.app.open(&file).unwrap();
    r.go(at(text, "Gamma"));
    r.app.dispatch(Command::Notes(NoteCommand::Add));
    r.app.dispatch(Command::Answer("note".into()));
    r.act(ActionId::ToggleEditMode);
    r.go(CharPos(0));
    r.type_str("New. ");
    let s = r.app.session().unwrap();
    assert_eq!(s.doc.slice(s.notes[0].range), "Gamma delta.");
    r.act(ActionId::Save);
    r.act(ActionId::ToggleEditMode);
    let s = r.app.session().unwrap();
    assert_eq!(s.doc.slice(s.notes[0].range), "Gamma delta.");
}

#[test]
fn bookmarks_rename_and_delete() {
    let mut r = rig();
    let text = "One two three four five six.";
    let file = r.file("b.txt", text);
    r.app.open(&file).unwrap();
    r.go(at(text, "two"));
    r.act(ActionId::AddBookmark);
    r.go(at(text, "five"));
    r.act(ActionId::AddBookmark);
    // Rename the bookmark at the cursor directly.
    let effects = r.app.dispatch(Command::Notes(NoteCommand::RenameBookmark));
    assert!(matches!(
        effects.first(),
        Some(Effect::Prompt {
            purpose: PromptPurpose::RenameBookmark,
            ..
        })
    ));
    r.app.dispatch(Command::Answer("chapter".into()));
    assert_eq!(r.said.last(), "Bookmark mark2 renamed to chapter.");
    // A clashing name is refused.
    r.act(ActionId::ListBookmarks);
    r.app.dispatch(Command::RenameItem(0));
    r.app.dispatch(Command::Answer("chapter".into()));
    assert!(r.said.last().contains("already a bookmark called chapter"));
    // Delete from the list; the list stays open with one item.
    r.act(ActionId::ListBookmarks);
    let effects = r.app.dispatch(Command::DeleteItem(0));
    let Some(Effect::ShowList { items, .. }) = effects.first() else {
        panic!("{effects:?}");
    };
    assert_eq!(items.len(), 1);
    assert!(items[0].starts_with("chapter"));
    assert_eq!(r.said.last(), "Bookmark mark1 deleted.");
    // Persisted at once.
    let state = StateStore::new(r.paths.state_dir())
        .load(&DocKey::for_path(&file))
        .unwrap();
    assert_eq!(state.bookmarks.len(), 1);
    assert_eq!(state.bookmarks[0].name, "chapter");
    // Delete the one at the cursor through the command.
    r.go(at(text, "five"));
    r.app.dispatch(Command::Notes(NoteCommand::DeleteBookmark));
    assert!(r.app.session().unwrap().bookmarks.is_empty());
}

#[test]
fn settings_are_saved_when_changed() {
    let mut r = rig();
    assert!(!r.paths.settings_file().exists());
    r.act(ActionId::RateUp);
    assert!(r.paths.settings_file().exists());
    let s = SettingsStore::new(r.paths.clone()).load().0;
    assert_eq!(s.speech.rate.wpm(), r.app.settings().speech.rate.wpm());
}

#[test]
fn position_is_saved_periodically() {
    let mut r = rig();
    let text = "One. Two. Three. Four.";
    let file = r.file("p.txt", text);
    r.app.open(&file).unwrap();
    let t0 = Instant::now();
    r.app.tick(t0);
    r.go(at(text, "Three"));
    let store = StateStore::new(r.paths.state_dir());
    let key = DocKey::for_path(&file);
    // Not yet: less than the interval has passed.
    r.app.tick(t0 + Duration::from_secs(5));
    assert_ne!(
        store.load(&key).map(|s| s.position),
        Some(at(text, "Three"))
    );
    r.app
        .tick(t0 + App::POSITION_SAVE_INTERVAL + Duration::from_secs(1));
    assert_eq!(store.load(&key).unwrap().position, at(text, "Three"));
}

#[test]
fn highlight_lead_shifts_only_the_drawn_word() {
    let said = Said::default();
    let (speech, _log) = recording_service().unwrap();
    let mut config = AppConfig {
        speech,
        announcer: Box::new(said.clone()),
        ..AppConfig::for_tests()
    };
    config.settings.highlight.lead_words = 2;
    let mut app = App::new(config);
    app.open_document(
        Document::from_plain_text("aa bb cc dd"),
        DocKey::untitled(9),
        "Lead".into(),
    );
    assert_eq!(app.highlight_lead(), 1);
    app.dispatch(Command::Action(ActionId::ReadFromCursor));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(true) = app.poll_speech_step()
            && app.session().unwrap().spoken.is_some()
        {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    let s = app.session().unwrap();
    let spoken = s.spoken.unwrap();
    let (shown, _) = app.shown_spoken();
    let shown = shown.unwrap();
    assert!(shown.start > spoken.start, "{shown:?} {spoken:?}");
    // The cursor stays on the confirmed word.
    assert_eq!(s.cursor, spoken.start);
    said.clear();
}

#[test]
fn reading_the_whole_document_and_the_source_while_editing() {
    let mut r = rig();
    let file = r.file("w.md", "# Head\n\nBody text.\n");
    r.app.open(&file).unwrap();
    r.go(CharPos(6));
    r.log.clear();
    r.act(ActionId::ReadDocument);
    r.wait_idle();
    let ranges = r.log.spoken_ranges();
    assert_eq!(ranges.first().map(|x| x.start), Some(CharPos(0)));
}

/// Ported from the audit's patch S5 (Agent D4).
#[test]
fn quitting_in_edit_mode_after_a_save_keeps_positions_on_the_saved_text() {
    // Quitting while still in edit mode (clean after a save) restored the
    // reading text from before the edits and saved positions against it.
    let mut r = rig();
    let file = r.file("note.md", NOTE_MD);
    r.app.open(&file).unwrap();
    let canon = r.text();
    r.go(at(&canon, "The end"));
    r.act(ActionId::AddBookmark);
    r.act(ActionId::ToggleEditMode);
    r.go(CharPos(at(NOTE_MD, "Hello").0));
    r.type_str("A much longer opening sentence goes here. ");
    r.act(ActionId::Save);
    assert!(!r.app.is_dirty());
    assert_eq!(r.quit(), vec![Effect::Quit]);
    r.relaunch();
    r.app.open(&file).unwrap();
    let canon = r.text();
    assert!(canon.contains("A much longer opening"), "{canon}");
    assert_eq!(bookmark_pos(&r.app), at(&canon, "The end"));
}

/// Waits (up to five seconds) for speech sent on the speech thread to reach
/// the recording backend; fixed sleeps race on a busy machine.
fn wait_until(done: impl Fn() -> bool) {
    let end = std::time::Instant::now() + Duration::from_secs(5);
    while !done() && std::time::Instant::now() < end {
        std::thread::sleep(Duration::from_millis(5));
    }
}
