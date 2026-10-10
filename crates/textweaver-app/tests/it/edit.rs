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
    /// Dispatches, then waits for the background writer and applies its
    /// results (saves, bookmarks, questions about the disk), as the event
    /// loop's next tick would.
    fn send(&mut self, cmd: Command) -> Vec<Effect> {
        let effects = self.app.dispatch(cmd);
        self.app.wait_for_writes();
        effects
    }
    fn act(&mut self, a: ActionId) -> Vec<Effect> {
        self.send(Command::Action(a))
    }
    /// A tick, then the writer's results.
    fn tick(&mut self, now: Instant) -> Vec<Effect> {
        let mut effects = self.app.tick(now);
        effects.extend(self.app.wait_for_writes());
        effects
    }
    /// Quit, answering yes to "Quit textweaver? y or n".
    fn quit(&mut self) -> Vec<Effect> {
        assert_eq!(self.act(ActionId::Quit), vec![Effect::Redraw]);
        assert_eq!(self.app.pending_confirmation(), Some(ActionId::Quit));
        self.send(Command::Confirm(Confirm::Yes))
    }
    fn text(&self) -> String {
        self.app.session().unwrap().doc.text().to_string()
    }
    fn go(&mut self, pos: CharPos) {
        self.send(Command::GoTo(GoTo::Char(pos)));
    }
    fn type_str(&mut self, s: &str) {
        for c in s.chars() {
            self.send(Command::Insert(c.to_string()));
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
    r.send(Command::Select(CharRange::new(there, CharPos(there.0 + 5))));
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
    assert!(r.said.any("unsaved changes"));
    // Cancel keeps editing.
    r.send(Command::Choose(2));
    assert!(r.app.is_editing());
    assert_eq!(r.said.last(), "Still editing.");
    // Escape on the list keeps editing too.
    r.act(ActionId::ToggleEditMode);
    r.send(Command::Cancel);
    assert!(r.app.is_editing());
    // Discard: the text and the bookmark are as they were.
    r.act(ActionId::ToggleEditMode);
    r.send(Command::Choose(1));
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
    r.send(Command::DeleteForward); // "b"
    r.send(Command::DeleteForward); // "e"
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
    r.send(Command::Answer(target.display().to_string()));
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "Draft");
    assert_eq!(r.app.session().unwrap().title, "draft.md");
    assert_eq!(r.app.session().unwrap().key, DocKey::for_path(&target));
    // The next save writes in place, without asking.
    r.type_str("!");
    let effects = r.act(ActionId::Save);
    assert!(!effects.iter().any(|e| matches!(e, Effect::Prompt { .. })));
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "Draft!");
}

/// A name from a system save dialog, which asks before replacing a file
/// itself, is not asked about again; the mark covers that one answer only.
#[test]
fn save_as_from_a_system_dialog_does_not_ask_again() {
    let mut r = rig();
    r.act(ActionId::NewDocument);
    r.type_str("# Field Notes\n\nBody.");
    r.act(ActionId::SaveAs);
    let taken = r.file("taken.md", "keep me");
    r.app.save_as_confirmed_by_system();
    r.send(Command::Answer(taken.display().to_string()));
    assert!(!r.app.confirmation_pending());
    assert_eq!(
        std::fs::read_to_string(&taken).unwrap(),
        "# Field Notes\n\nBody."
    );
    // A typed answer afterwards is asked about as before.
    r.act(ActionId::SaveAs);
    let other = r.file("other.md", "keep me too");
    r.send(Command::Answer(other.display().to_string()));
    assert!(r.app.confirmation_pending());
    assert_eq!(std::fs::read_to_string(&other).unwrap(), "keep me too");
}

/// Save As suggests a name from the first heading and asks y or n before
/// replacing another file; n asks for another name, y replaces it.
#[test]
fn save_as_suggests_from_the_heading_and_asks_before_overwriting() {
    let mut r = rig();
    r.act(ActionId::NewDocument);
    r.type_str("# Field Notes\n\nBody.");
    let effects = r.act(ActionId::SaveAs);
    let Some(Effect::Prompt { label, .. }) = effects.first() else {
        panic!("{effects:?}");
    };
    assert!(label.ends_with("field-notes.md"), "{label}");
    // Another file is in the way.
    let taken = r.file("taken.md", "keep me");
    r.send(Command::Answer(taken.display().to_string()));
    assert!(r.app.confirmation_pending());
    assert!(
        r.said
            .last()
            .contains("taken.md already exists. Replace it? y or n")
    );
    // No: nothing written, and the name is asked again.
    let effects = r.send(Command::Confirm(Confirm::No));
    assert!(matches!(
        effects.first(),
        Some(Effect::Prompt {
            purpose: PromptPurpose::SaveAs,
            ..
        })
    ));
    assert_eq!(std::fs::read_to_string(&taken).unwrap(), "keep me");
    // A relative answer goes in the same folder; it exists too, and y
    // replaces it.
    r.send(Command::Answer("taken.md".into()));
    assert!(r.app.confirmation_pending());
    r.send(Command::Confirm(Confirm::Yes));
    assert!(!r.app.confirmation_pending());
    assert_eq!(
        std::fs::read_to_string(&taken).unwrap(),
        "# Field Notes\n\nBody."
    );
    // Saving onto the file already open does not ask.
    r.type_str("!");
    let effects = r.act(ActionId::SaveAs);
    assert!(matches!(effects.first(), Some(Effect::Prompt { .. })));
    r.send(Command::Answer(String::new()));
    assert!(!r.app.confirmation_pending());
    assert!(std::fs::read_to_string(&taken).unwrap().ends_with('!'));
}

/// A recovery snapshot that cannot be written is announced once, the
/// attempts back off, and a later success is said.
#[test]
fn a_failing_recovery_copy_is_announced_once() {
    let mut r = rig();
    let file = r.file("s.md", "Text.\n");
    r.app.open(&file).unwrap();
    // A file where the recovery folder should be.
    let rec = r.paths.recovery_dir();
    std::fs::create_dir_all(rec.parent().unwrap()).unwrap();
    std::fs::write(&rec, "blocked").unwrap();
    r.act(ActionId::ToggleEditMode);
    r.type_str("More ");
    r.said.clear();
    let t0 = Instant::now();
    for s in 0..200u64 {
        r.tick(t0 + Duration::from_millis(s * 500));
    }
    let failures: Vec<String> = r
        .said
        .all()
        .into_iter()
        .filter(|m| m.contains("Could not write the recovery copy"))
        .collect();
    assert_eq!(failures.len(), 1, "{:?}", r.said.all());
    std::fs::remove_file(&rec).unwrap();
    r.tick(t0 + Duration::from_secs(1000));
    assert!(r.said.any("The recovery copy is being written again."));
}

/// A crash or a closing terminal writes the snapshot at once and saves the
/// position, without asking.
#[test]
fn emergency_save_writes_the_snapshot_at_once() {
    let mut r = rig();
    let file = r.file("e.md", "Original.\n");
    r.app.open(&file).unwrap();
    r.act(ActionId::ToggleEditMode);
    r.type_str("Unsaved ");
    r.app.emergency_save();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "Original.\n");
    r.relaunch();
    let effects = r.app.offer_recovery();
    assert!(
        matches!(effects.first(), Some(Effect::ShowList { .. })),
        "{effects:?}"
    );
}

/// "Bookmark set" is said only when the bookmark reached the disk.
#[test]
fn a_bookmark_that_cannot_be_saved_is_not_announced_as_set() {
    let mut r = rig();
    let file = r.file(
        "b.md",
        "One two three.
",
    );
    r.app.open(&file).unwrap();
    // A file where the state folder should be: saving fails.
    let state = r.paths.state_dir();
    let _ = std::fs::remove_dir_all(&state);
    std::fs::create_dir_all(state.parent().unwrap()).unwrap();
    std::fs::write(&state, "blocked").unwrap();
    r.said.clear();
    r.act(ActionId::AddBookmark);
    assert!(!r.said.any("set at"), "{:?}", r.said.all());
    assert!(r.said.any("could not be saved"), "{:?}", r.said.all());
    // It still works for this session.
    assert!(r.app.bookmark_position(0).is_some());
    // Once saving works again, so does the announcement.
    std::fs::remove_file(&state).unwrap();
    r.act(ActionId::CaretNextWord);
    r.said.clear();
    r.act(ActionId::AddBookmark);
    assert!(r.said.any("Bookmark mark2 set at"), "{:?}", r.said.all());
}

#[test]
fn quitting_with_changes_saves_on_request() {
    let mut r = rig();
    let file = r.file("q.md", "Some text.\n");
    r.app.open(&file).unwrap();
    r.act(ActionId::ToggleEditMode);
    r.send(Command::MoveCaret {
        by: CaretMove::DocumentEdge,
        direction: Direction::Forward,
        extend: false,
    });
    r.type_str("More.");
    let effects = r.quit();
    assert!(matches!(effects.first(), Some(Effect::ShowList { .. })));
    let effects = r.send(Command::Choose(0));
    assert_eq!(effects, vec![Effect::Quit]);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "Some text.\nMore.");
}

/// The window's close button and Alt+F4 (W8c-w): no "Quit textweaver?"
/// question, but unsaved edits ask Save, Discard or Cancel as Quit does,
/// and Cancel keeps the window open with the edits.
#[test]
fn closing_the_window_asks_about_unsaved_edits_only() {
    let mut r = rig();
    let file = r.file("close.md", "Some text.\n");
    r.app.open(&file).unwrap();
    // Nothing unsaved: the window closes at once, with no question.
    assert_eq!(r.app.close_requested(), vec![Effect::Quit]);
    assert!(!r.app.confirmation_pending());

    let mut r = rig();
    let file = r.file("close.md", "Some text.\n");
    r.app.open(&file).unwrap();
    r.act(ActionId::ToggleEditMode);
    r.type_str("New. ");
    let effects = r.app.close_requested();
    let Some(Effect::ShowList { items, .. }) = effects.first() else {
        panic!("{effects:?}");
    };
    assert_eq!(items.len(), 3, "Save, Discard, Cancel: {items:?}");
    assert!(!effects.contains(&Effect::Quit));
    assert!(r.said.any("unsaved changes"));
    // Cancel: still editing, nothing lost, the window stays.
    let effects = r.send(Command::Choose(2));
    assert!(!effects.contains(&Effect::Quit), "{effects:?}");
    assert!(r.app.is_dirty());
    assert_eq!(r.text(), "New. Some text.\n");
    // Again, then Discard: the window closes and the file is untouched.
    r.app.close_requested();
    let effects = r.send(Command::Choose(1));
    assert_eq!(effects, vec![Effect::Quit]);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "Some text.\n");
}

/// A close while Quit's own question is open closes rather than leaving
/// that question behind.
#[test]
fn closing_the_window_drops_the_quit_question() {
    let mut r = rig();
    let file = r.file("q.md", "Some text.\n");
    r.app.open(&file).unwrap();
    r.act(ActionId::Quit);
    assert_eq!(r.app.pending_confirmation(), Some(ActionId::Quit));
    assert_eq!(r.app.close_requested(), vec![Effect::Quit]);
    assert_eq!(r.app.pending_confirmation(), None);
}

/// Questions that delete, remove or replace name the verb for the
/// window's confirming button; the others have none (W8c-w).
#[test]
fn destructive_questions_name_their_verb() {
    use textweaver_app::DestructiveVerb;
    let mut r = rig();
    let file = r.file("v.md", "Some text.\n");
    r.app.open(&file).unwrap();
    assert_eq!(r.app.destructive_question(), None);
    r.act(ActionId::Quit);
    assert!(r.app.confirmation_pending());
    assert_eq!(
        r.app.destructive_question(),
        None,
        "quitting deletes nothing"
    );
    r.send(Command::Confirm(Confirm::No));
    r.act(ActionId::DeleteNote);
    assert!(r.app.confirmation_pending());
    assert_eq!(r.app.destructive_question(), Some(DestructiveVerb::Delete));
    r.send(Command::Confirm(Confirm::No));
    assert_eq!(r.app.destructive_question(), None);
}

#[test]
fn autosave_snapshot_is_offered_after_a_crash() {
    let mut r = rig();
    let file = r.file("crash.md", "Before.\n");
    r.app.open(&file).unwrap();
    r.act(ActionId::ToggleEditMode);
    r.type_str("Lost? ");
    // The first tick while dirty writes a snapshot.
    r.tick(Instant::now());
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
    assert!(r.said.any("unsaved changes"));
    r.send(Command::Choose(0));
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
    r.tick(Instant::now());
    r.relaunch();
    assert!(!r.app.offer_recovery().is_empty());
    r.send(Command::Choose(1));
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
    r.send(Command::Insert("x".into()));
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
    r.send(Command::Answer("cat".into()));
    assert!(r.said.last().contains("2 matches"), "{}", r.said.last());
    // One match at a time: the list asks; "a" replaces all the rest.
    let effects = r.send(Command::Answer("bird".into()));
    assert!(
        effects.iter().any(|e| matches!(e, Effect::ShowList { .. })),
        "{effects:?}"
    );
    assert_eq!(r.app.list_accelerator('a'), Some(2));
    assert_eq!(r.app.list_accelerator('x'), Some(5));
    assert_eq!(r.app.list_accelerator('l'), Some(6));
    r.send(Command::Choose(2));
    assert_eq!(r.said.last(), "Replace all 2 remaining matches? y or n");
    r.send(Command::Confirm(Confirm::Yes));
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
    r.send(Command::MoveCaret {
        by: CaretMove::LineEdge,
        direction: Direction::Backward,
        extend: false,
    });
    r.log.clear();
    r.send(Command::MoveCaret {
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
    r.send(Command::MoveCaret {
        by: CaretMove::Word,
        direction: Direction::Forward,
        extend: true,
    });
    assert_eq!(r.said.last(), "cat selected");
    // Backspace deletes the selection.
    r.send(Command::DeleteBack);
    assert_eq!(r.text(), "## cat dog\n");
}

#[test]
fn notes_add_list_jump_edit_delete_and_persist() {
    let mut r = rig();
    let text = "First sentence here. Second sentence there. Third one.";
    let file = r.file("n.txt", text);
    r.app.open(&file).unwrap();
    r.go(at(text, "Second"));
    let effects = r.send(Command::Notes(NoteCommand::Add));
    assert!(matches!(
        effects.first(),
        Some(Effect::Prompt {
            purpose: PromptPurpose::NoteText,
            ..
        })
    ));
    r.send(Command::Answer("Check this #exam".into()));
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
    r.send(Command::Notes(NoteCommand::Next));
    assert_eq!(r.app.session().unwrap().cursor, at(text, "Second"));
    assert!(r.said.last().contains("Check this #exam"));
    r.go(CharPos(0));
    let effects = r.send(Command::Notes(NoteCommand::List));
    let Some(Effect::ShowList { items, .. }) = effects.first() else {
        panic!("{effects:?}");
    };
    assert_eq!(items.len(), 1);
    r.send(Command::Choose(0));
    assert_eq!(r.app.session().unwrap().cursor, at(text, "Second"));

    // Edit it through F2 on the list.
    r.send(Command::Notes(NoteCommand::List));
    let effects = r.send(Command::RenameItem(0));
    assert!(matches!(
        effects.first(),
        Some(Effect::Prompt {
            purpose: PromptPurpose::EditNote,
            ..
        })
    ));
    r.send(Command::Answer("Revised #review".into()));
    assert_eq!(r.said.last(), "Note updated.");

    // A highlight on the first sentence.
    r.go(CharPos(0));
    r.send(Command::Notes(NoteCommand::ToggleHighlight));
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
    r.send(Command::Notes(NoteCommand::ToggleHighlight));
    assert!(r.said.last().starts_with("Highlight removed"));
    r.send(Command::Notes(NoteCommand::List));
    // Delete in the list asks first; n keeps the note and shows the list
    // again, y deletes it.
    r.send(Command::DeleteItem(0));
    assert!(r.app.confirmation_pending());
    assert_eq!(r.said.last(), "Delete this note? y or n");
    let effects = r.send(Command::Confirm(Confirm::No));
    assert!(effects.iter().any(|e| matches!(e, Effect::ShowList { .. })));
    assert_eq!(r.app.session().unwrap().notes.len(), 1);
    let effects = r.send(Command::DeleteItem(0));
    assert!(!effects.iter().any(|e| matches!(e, Effect::ShowList { .. })));
    let effects = r.send(Command::Confirm(Confirm::Yes));
    assert!(!effects.iter().any(|e| matches!(e, Effect::ShowList { .. })));
    assert!(r.said.last().starts_with("Note deleted"));
    let store = StateStore::new(r.paths.state_dir());
    let state = store.load(&DocKey::for_path(&file)).unwrap();
    assert!(state.notes.is_empty() && state.highlights.is_empty());
    // Both deletions are recorded, so a later sync merge cannot bring
    // them back (state format 2).
    assert_eq!(state.deleted.notes.len(), 1);
    assert_eq!(state.deleted.highlights.len(), 1);
}

#[test]
fn notes_move_with_edits() {
    let mut r = rig();
    let text = "Alpha beta. Gamma delta.";
    let file = r.file("m.txt", text);
    r.app.open(&file).unwrap();
    r.go(at(text, "Gamma"));
    r.send(Command::Notes(NoteCommand::Add));
    r.send(Command::Answer("note".into()));
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
    let effects = r.send(Command::Notes(NoteCommand::RenameBookmark));
    assert!(matches!(
        effects.first(),
        Some(Effect::Prompt {
            purpose: PromptPurpose::RenameBookmark,
            ..
        })
    ));
    r.send(Command::Answer("chapter".into()));
    assert_eq!(r.said.last(), "Bookmark mark2 renamed to chapter.");
    // A clashing name is refused.
    r.act(ActionId::ListBookmarks);
    r.send(Command::RenameItem(0));
    r.send(Command::Answer("chapter".into()));
    assert!(r.said.last().contains("already a bookmark called chapter"));
    // Delete from the list; the list stays open with one item.
    r.act(ActionId::ListBookmarks);
    let effects = r.send(Command::DeleteItem(0));
    let Some(Effect::ShowList { items, .. }) = effects.first() else {
        panic!("{effects:?}");
    };
    assert_eq!(items.len(), 1);
    assert!(items[0].starts_with("chapter"));
    assert!(r.said.any("Bookmark mark1 deleted."));
    // Persisted at once.
    let state = StateStore::new(r.paths.state_dir())
        .load(&DocKey::for_path(&file))
        .unwrap();
    assert_eq!(state.bookmarks.len(), 1);
    assert_eq!(state.bookmarks[0].name, "chapter");
    // Bookmarks carry ids, and the deletion is recorded (state format 2).
    assert_eq!(state.bookmarks[0].id.len(), 16);
    assert_eq!(state.deleted.bookmarks.len(), 1);
    // Delete the one at the cursor through the command.
    r.go(at(text, "five"));
    r.send(Command::Notes(NoteCommand::DeleteBookmark));
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
    r.tick(t0);
    r.go(at(text, "Three"));
    let store = StateStore::new(r.paths.state_dir());
    let key = DocKey::for_path(&file);
    // Not yet: less than the interval has passed.
    r.tick(t0 + Duration::from_secs(5));
    assert_ne!(
        store.load(&key).map(|s| s.position),
        Some(at(text, "Three"))
    );
    r.tick(t0 + App::POSITION_SAVE_INTERVAL + Duration::from_secs(1));
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

/// Changes a file on disk the way another editor would: new contents,
/// another size.
fn change_on_disk(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap();
}

#[test]
fn saving_over_a_file_changed_on_disk_asks_first() {
    let mut r = rig();
    let file = r.file("shared.md", NOTE_MD);
    r.app.open(&file).unwrap();
    r.act(ActionId::ToggleEditMode);
    r.type_str("Mine. ");
    let theirs = "# Title\n\nSomeone else wrote this.\n";
    change_on_disk(&file, theirs);
    r.act(ActionId::Save);
    assert!(r.app.confirmation_pending());
    assert_eq!(
        r.said.last(),
        "shared.md changed on disk since you opened it. Save over those changes? y or n"
    );
    // No keeps their version on disk and stays in edit mode.
    r.send(Command::Confirm(Confirm::No));
    assert!(
        r.said.last().starts_with("Not saved. Still editing."),
        "{}",
        r.said.last()
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), theirs);
    assert!(r.app.is_dirty());
    // Yes saves over them; the next save does not ask again.
    r.act(ActionId::Save);
    assert!(r.app.confirmation_pending());
    r.send(Command::Confirm(Confirm::Yes));
    assert!(!r.app.is_dirty());
    assert!(std::fs::read_to_string(&file).unwrap().contains("Mine. "));
    r.type_str("More. ");
    r.act(ActionId::Save);
    assert!(!r.app.confirmation_pending());
    assert!(
        r.said.last().starts_with("Saved shared.md"),
        "{}",
        r.said.last()
    );
}

#[test]
fn leaving_with_save_over_a_changed_file_asks_first() {
    let mut r = rig();
    let file = r.file("leave.md", NOTE_MD);
    r.app.open(&file).unwrap();
    r.act(ActionId::ToggleEditMode);
    r.type_str("Mine. ");
    change_on_disk(&file, "# Title\n\nChanged elsewhere, longer.\n");
    r.act(ActionId::ToggleEditMode);
    // Save, discard, or cancel: save.
    r.send(Command::Choose(0));
    assert!(r.app.confirmation_pending());
    assert!(r.app.is_editing());
    r.send(Command::Confirm(Confirm::Yes));
    assert!(!r.app.is_editing());
    assert!(std::fs::read_to_string(&file).unwrap().contains("Mine. "));
}

#[test]
fn a_file_changed_on_disk_while_open_offers_a_reload() {
    let mut r = rig();
    let file = r.file("watched.md", NOTE_MD);
    r.app.open(&file).unwrap();
    let t0 = Instant::now();
    r.tick(t0);
    assert!(!r.app.confirmation_pending());
    change_on_disk(&file, "# Title\n\nFresh text from elsewhere.\n");
    r.tick(t0 + Duration::from_secs(5));
    assert!(r.app.confirmation_pending());
    assert_eq!(
        r.said.last(),
        "watched.md changed on disk. Reload it? y or n"
    );
    r.send(Command::Confirm(Confirm::Yes));
    assert!(
        r.text().contains("Fresh text from elsewhere."),
        "{}",
        r.text()
    );
    // No: keep the open version, and do not ask again about it.
    change_on_disk(&file, "# Title\n\nA third version, longer still.\n");
    r.tick(t0 + Duration::from_secs(10));
    assert!(r.app.confirmation_pending());
    r.send(Command::Confirm(Confirm::No));
    assert_eq!(r.said.last(), "Kept the open version.");
    r.tick(t0 + Duration::from_secs(15));
    assert!(!r.app.confirmation_pending());
    assert!(r.text().contains("Fresh text"));
}

#[test]
fn a_reload_is_not_offered_over_unsaved_changes_but_is_in_clean_edit_mode() {
    let mut r = rig();
    let file = r.file("busy.md", NOTE_MD);
    r.app.open(&file).unwrap();
    r.act(ActionId::ToggleEditMode);
    r.type_str("Unsaved. ");
    let t0 = Instant::now();
    change_on_disk(&file, "# Title\n\nOther words here.\n");
    r.tick(t0 + Duration::from_secs(5));
    assert!(!r.app.confirmation_pending(), "{}", r.said.last());
    // Undo back to clean: the reload is offered and keeps edit mode on.
    for _ in 0..20 {
        if !r.app.is_dirty() {
            break;
        }
        r.act(ActionId::Undo);
    }
    assert!(!r.app.is_dirty());
    r.tick(t0 + Duration::from_secs(10));
    assert!(r.app.confirmation_pending());
    r.send(Command::Confirm(Confirm::Yes));
    assert!(r.app.is_editing());
    assert!(r.text().contains("Other words here."), "{}", r.text());
}

/// The items of the list a dispatch showed.
fn shown(effects: &[Effect]) -> Vec<String> {
    effects
        .iter()
        .find_map(|e| match e {
            Effect::ShowList { items, .. } => Some(items.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no list: {effects:?}"))
}

/// Relations between notes through the list keys every frontend sends
/// (B1-g1): Space on a note opens its links; adding one by type (typed to
/// filter) and target, in this document and in another library document;
/// the counts in the notes list; "What links here"; removing after the
/// question; every row's meaning in its first 40 characters.
#[test]
fn notes_link_to_notes_through_the_lists() {
    use textweaver_app::list_model::ListKey;
    let mut r = rig();
    let text = "Alpha one here. Beta two here. Gamma three here.";
    let a = r.file("a.txt", text);
    r.app.open(&a).unwrap();
    for (word, note) in [("Alpha", "Energy note"), ("Gamma", "Chapter note")] {
        r.go(at(text, word));
        r.send(Command::Notes(NoteCommand::Add));
        r.send(Command::Answer(note.into()));
    }
    r.send(Command::Notes(NoteCommand::List));
    let items = shown(&r.send(Command::ListKey(ListKey::Char(' '))));
    assert_eq!(items, vec!["What links here: nothing yet", "Add a link"]);
    assert!(
        r.said.any("Links of Energy note: 0 out, 0 in."),
        "{:?}",
        r.said.all()
    );
    r.send(Command::ListKey(ListKey::End));
    let types = shown(&r.send(Command::ListKey(ListKey::Enter)));
    assert_eq!(types.len(), 10);
    r.send(Command::ListKey(ListKey::Char('s')));
    r.send(Command::ListKey(ListKey::Char('u')));
    let filtered = shown(&r.send(Command::ListKey(ListKey::Char('p'))));
    assert_eq!(filtered, vec!["supports"]);
    let targets = shown(&r.send(Command::ListKey(ListKey::Enter)));
    assert_eq!(targets, vec!["Chapter note", "A note in another document"]);
    let links = shown(&r.send(Command::ListKey(ListKey::Enter)));
    assert!(
        r.said.any("Linked: supports Chapter note."),
        "{:?}",
        r.said.all()
    );
    assert_eq!(links[0], "supports: Chapter note");
    // The notes list counts links both ways.
    r.send(Command::ListKey(ListKey::Escape));
    let notes = shown(&r.send(Command::Notes(NoteCommand::List)));
    assert!(notes[0].ends_with("Links: 1 out, 0 in."), "{notes:?}");
    assert!(notes[1].ends_with("Links: 0 out, 1 in."), "{notes:?}");
    // What links to the second note.
    r.send(Command::ListKey(ListKey::Down));
    r.send(Command::ListKey(ListKey::Char(' ')));
    let back = shown(&r.send(Command::ListKey(ListKey::Enter)));
    assert_eq!(back, vec!["supports this, from: Energy note"]);
    r.send(Command::ListKey(ListKey::Enter));
    assert_eq!(r.app.session().unwrap().cursor, at(text, "Alpha"));

    // From another document: the target is a library document's note.
    let b = r.file("b.txt", "Week four starts here.");
    r.app.open(&b).unwrap();
    r.app.wait_for_writes();
    r.send(Command::Notes(NoteCommand::Add));
    r.send(Command::Answer("Week four note".into()));
    r.go(CharPos(0));
    r.send(Command::Notes(NoteCommand::Links));
    r.send(Command::ListKey(ListKey::End));
    r.send(Command::ListKey(ListKey::Enter));
    for c in "cit".chars() {
        r.send(Command::ListKey(ListKey::Char(c)));
    }
    let only_other = shown(&r.send(Command::ListKey(ListKey::Enter)));
    assert_eq!(only_other, vec!["A note in another document"]);
    let docs = shown(&r.send(Command::ListKey(ListKey::Enter)));
    assert_eq!(docs.len(), 1, "{docs:?}");
    assert!(docs[0].ends_with(", 2 notes"), "{docs:?}");
    r.send(Command::ListKey(ListKey::Enter));
    r.send(Command::ListKey(ListKey::Down));
    let links = shown(&r.send(Command::ListKey(ListKey::Enter)));
    assert!(
        links[0].starts_with("cites: Chapter note, in "),
        "{links:?}"
    );
    // Enter follows it into the other document, to the note.
    r.send(Command::ListKey(ListKey::Home));
    r.send(Command::ListKey(ListKey::Enter));
    r.app.wait_for_writes();
    let s = r.app.session().unwrap();
    // Canonical paths: temp folders differ by short name (Windows) or /private (macOS).
    let opened = s
        .doc
        .meta
        .path
        .as_deref()
        .map(|p| std::fs::canonicalize(p).unwrap());
    assert_eq!(opened, Some(std::fs::canonicalize(&a).unwrap()));
    assert_eq!(s.cursor, at(text, "Gamma"));
    let notes = shown(&r.send(Command::Notes(NoteCommand::List)));
    assert!(notes[1].ends_with("Links: 0 out, 2 in."), "{notes:?}");

    // Delete asks, then removes the link, and the file has it no more.
    r.send(Command::ListKey(ListKey::Home));
    r.send(Command::ListKey(ListKey::Char(' ')));
    r.send(Command::ListKey(ListKey::Delete));
    assert!(r.said.last().starts_with("Remove this link? y or n"));
    let after = shown(&r.send(Command::Confirm(Confirm::Yes)));
    assert!(
        r.said.any("Link removed: supports Chapter note."),
        "{:?}",
        r.said.all()
    );
    assert_eq!(after, vec!["What links here: nothing yet", "Add a link"]);
    let on_disk = StateStore::new(r.paths.state_dir())
        .load(&DocKey::for_path(&a))
        .unwrap();
    assert!(on_disk.notes[0].relations.is_empty());
    for line in r.said.all() {
        if line.starts_with("supports") || line.starts_with("cites") || line.starts_with("Link") {
            let head: String = line.chars().take(40).collect();
            assert!(head.contains(':'), "meaning first: {line}");
        }
    }
}

/// Export the knowledge graph (B1-g2): nothing to export says so; with a
/// link, the formats list starts with the Markdown list, and Enter writes
/// the file beside the document and offers to open it.
#[test]
fn the_knowledge_graph_exports_in_a_format_chosen_from_a_list() {
    use textweaver_app::list_model::ListKey;
    let mut r = rig();
    let text = "Alpha one here. Gamma three here.";
    let a = r.file("a.txt", text);
    r.app.open(&a).unwrap();
    r.send(Command::Action(ActionId::ExportKnowledgeGraph));
    assert!(
        r.said.any("No links between notes to export."),
        "{:?}",
        r.said.all()
    );
    for (word, note) in [("Alpha", "Energy note"), ("Gamma", "Chapter note")] {
        r.go(at(text, word));
        r.send(Command::Notes(NoteCommand::Add));
        r.send(Command::Answer(note.into()));
    }
    r.go(at(text, "Alpha"));
    r.send(Command::Notes(NoteCommand::Links));
    r.send(Command::ListKey(ListKey::End));
    r.send(Command::ListKey(ListKey::Enter));
    for c in "sup".chars() {
        r.send(Command::ListKey(ListKey::Char(c)));
    }
    r.send(Command::ListKey(ListKey::Enter));
    r.send(Command::ListKey(ListKey::Enter));
    r.send(Command::ListKey(ListKey::Escape));
    let formats = shown(&r.send(Command::Action(ActionId::ExportKnowledgeGraph)));
    assert_eq!(formats.len(), 7, "{formats:?}");
    assert_eq!(formats[0], "Markdown list, the text to read");
    assert!(
        r.said.any(
            "Knowledge graph, links: 1. Choose a format; the Markdown list is the text to read."
        ),
        "{:?}",
        r.said.all()
    );
    r.send(Command::Choose(0));
    let out = a.with_file_name("knowledge-graph.md");
    let written = std::fs::read_to_string(&out).unwrap();
    assert!(written.starts_with("# Knowledge graph\n"), "{written}");
    assert!(
        written.contains("\n- supports: Chapter note, in "),
        "{written}"
    );
    assert!(
        r.said
            .last()
            .starts_with("Knowledge graph saved as knowledge-graph.md."),
        "{}",
        r.said.last()
    );
}
