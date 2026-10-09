use std::path::PathBuf;

use textweaver_formats::{LoadOptions, Registry, RevisionMode, Source};
use textweaver_keymap::ActionId;
use textweaver_store::DocKey;
use textweaver_text::Document;

use super::*;
use crate::command::Command;
use crate::list_model::ListKey;
use crate::{App, AppConfig};

fn fixture(revisions: RevisionMode) -> Document {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/t1/changes.docx");
    Registry::with_builtins()
        .load(
            &Source::Path(path),
            &LoadOptions {
                revisions,
                ..LoadOptions::default()
            },
        )
        .unwrap()
}

fn app_with(doc: Document) -> App {
    let mut app = App::new(AppConfig::for_tests());
    app.open_document(doc, DocKey::untitled(1), "Case notes".into());
    app
}

fn list(effects: &[Effect]) -> Option<Vec<String>> {
    effects.iter().find_map(|e| match e {
        Effect::ShowList { items, .. } => Some(items.clone()),
        _ => None,
    })
}

fn text(app: &App) -> String {
    app.session.as_ref().unwrap().doc.text().to_string()
}

#[test]
fn every_row_says_kind_text_author_and_date_in_words() {
    let mut app = app_with(fixture(RevisionMode::Final));
    let items = list(&app.dispatch(Command::Action(ActionId::ListChanges))).expect("a list");
    assert_eq!(
        items,
        vec![
            "Inserted: 'renal', by Ada Example, Tuesday, March 3, 2026",
            "Deleted: 'rarely', by Bo Example, Wednesday, March 4, 2026",
            "Moved away: 'Check the labs first.', by Ada Example, Thursday, March 5, 2026",
            "Comment by Bo Example: check this date, 1 reply, resolved",
            "Moved here: 'check the labs first', by Ada Example, Thursday, March 5, 2026",
            "Inserted: 'Repeat tomorrow.', by Bo Example, date not recorded",
            "Comment by Ada Example: Who repeats it?",
        ]
    );
    assert!(
        app.status_text()
            .starts_with("Changes and comments, 7 items. Enter goes to one."),
        "{}",
        app.status_text()
    );
}

#[test]
fn dates_are_said_in_full_and_never_guessed() {
    let c = Catalog::english();
    assert_eq!(
        spoken_date(&c, "2026-10-09T08:00:00Z").as_deref(),
        Some("Friday, October 9, 2026")
    );
    assert_eq!(spoken_date(&c, "2026-02-30"), None);
    assert_eq!(spoken_date(&c, "soon"), None);
    assert_eq!(spoken_date(&c, ""), None);
}

#[test]
fn accepting_and_rejecting_edit_the_text_read_as_final() {
    let mut doc = fixture(RevisionMode::Final);
    // Reject the insertion "renal", accept the deletion "rarely" (no edit),
    // reject the move away (the text comes back).
    let (c, o) = resolve_change(&mut doc, 0, false).unwrap();
    assert_eq!(c.text, "renal");
    assert!(o.is_some());
    let (_, o) = resolve_change(&mut doc, 0, true).unwrap();
    assert!(
        o.is_none(),
        "accepting a deletion read as final changes nothing"
    );
    resolve_change(&mut doc, 0, false).unwrap();
    assert_eq!(
        doc.text().to_string(),
        "Case Notes\n\nThe patient has acute failure.\n\nFluids were given every hour.\n\nCheck the labs first. Call the family.\n\nThen check the labs first. Repeat tomorrow."
    );
    // The comment moved with its text.
    let cs = textweaver_formats::comments(&doc.meta);
    assert_eq!(doc.slice(cs[0].range).to_string(), "Call the family.");
    assert_eq!(textweaver_formats::changes(&doc.meta).len(), 2);
}

#[test]
fn reject_all_restores_the_original_and_accept_all_the_final_text() {
    let mut said = fixture(RevisionMode::Marked);
    let done = resolve_all(&mut said, true, None);
    assert_eq!(done.len(), 5);
    assert_eq!(
        said.text().to_string(),
        fixture(RevisionMode::Final).text().to_string(),
        "accepting every change said in place gives the final text"
    );
    assert!(textweaver_formats::changes(&said.meta).is_empty());
    assert!(
        said.markers()
            .iter()
            .all(|m| !m.range.is_empty() || m.label.is_none())
    );

    let mut doc = fixture(RevisionMode::Final);
    resolve_all(&mut doc, false, None);
    assert_eq!(
        doc.text().to_string(),
        "Case Notes\n\nThe patient has acute failure.\n\nFluids were rarely given every hour.\n\nCheck the labs first. Call the family.\n\nThen."
    );
}

#[test]
fn keys_in_the_list_accept_reject_and_handle_comments() {
    let mut app = app_with(fixture(RevisionMode::Final));
    app.dispatch(Command::Action(ActionId::ListChanges));
    // Row 0: "renal". r rejects it; the list stays, one row shorter.
    let e = app.dispatch(Command::ListKey(ListKey::Char('r')));
    let items = list(&e).expect("the list again");
    assert_eq!(items.len(), 6);
    assert!(text(&app).contains("acute failure."));
    assert!(
        app.status_text()
            .starts_with("Rejected. Inserted: 'renal'.")
    );
    let review = app.review().unwrap();
    assert_eq!(review.decisions.len(), 1);
    assert!(!review.decisions[0].accepted);
    // Shift+A on the move away: every change by Ada Example.
    app.dispatch(Command::ListKey(ListKey::Down));
    app.dispatch(Command::ListKey(ListKey::Char('A')));
    assert!(
        app.status_text()
            .starts_with("Accepted 2 changes by Ada Example.")
    );
    // Space on the resolved comment opens it again; F2 replies.
    let rows = match &app.list {
        Some(ListKind::Changes(rows)) => rows.clone(),
        other => panic!("{other:?}"),
    };
    let n = rows
        .iter()
        .position(|r| matches!(r, Row::Comment(0)))
        .unwrap();
    app.dispatch(Command::ListFocus(n));
    app.dispatch(Command::ListKey(ListKey::Char(' ')));
    assert!(app.status_text().starts_with("Comment open again."));
    app.dispatch(Command::ListKey(ListKey::Rename));
    let e = app.dispatch(Command::Answer("Yes, March 4.".into()));
    let items = list(&e).expect("the list after the reply");
    assert!(
        items.contains(&"Comment by Bo Example: check this date, 2 replies".to_owned()),
        "{items:?}"
    );
    let s = app.session.as_ref().unwrap();
    let note = s.notes.iter().find(|n| n.id == "comment-0").unwrap();
    assert!(note.note.contains("Yes, March 4."), "{}", note.note);
    assert!(app.review().unwrap().comments_changed);
    // Accept all: the rest.
    app.dispatch(Command::Cancel);
    app.dispatch(Command::Action(ActionId::AcceptAllChanges));
    assert!(
        app.status_text().starts_with("Accepted 2 changes."),
        "{}",
        app.status_text()
    );
    assert!(textweaver_formats::changes(&app.session.as_ref().unwrap().doc.meta).is_empty());
}

#[test]
fn a_comment_is_deleted_after_its_question_and_a_new_one_added() {
    let mut app = app_with(fixture(RevisionMode::Final));
    app.dispatch(Command::Action(ActionId::ListChanges));
    app.dispatch(Command::ListKey(ListKey::End));
    app.dispatch(Command::ListKey(ListKey::Delete));
    assert!(
        app.status_text()
            .starts_with("Delete this comment and its replies? y or n")
    );
    app.dispatch(Command::Confirm(crate::command::Confirm::Yes));
    let s = app.session.as_ref().unwrap();
    assert_eq!(textweaver_formats::comments(&s.doc.meta).len(), 1);
    assert!(!s.notes.iter().any(|n| n.id == "comment-2"));
    assert_eq!(app.review().unwrap().deleted_comments.len(), 1);

    app.dispatch(Command::Cancel);
    app.dispatch(Command::Action(ActionId::AddComment));
    app.dispatch(Command::Answer("See the chart.".into()));
    let s = app.session.as_ref().unwrap();
    let cs = textweaver_formats::comments(&s.doc.meta);
    assert_eq!(cs.len(), 2);
    assert!(
        cs.iter()
            .any(|c| c.text == "See the chart." && !c.date.is_empty())
    );
    assert!(s.notes.iter().any(|n| n.note.contains("See the chart.")));
}
