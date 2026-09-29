use std::path::PathBuf;

use textweaver_core::{CharPos, CharRange};
use textweaver_keymap::ActionId;
use textweaver_store::DocKey;
use textweaver_text::Document;

use super::*;
use crate::command::Command;
use crate::{App, AppConfig};

fn fixture(name: &str) -> Document {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name);
    textweaver_formats::load_path(&path).unwrap()
}

fn app_with(doc: Document) -> App {
    let mut app = App::new(AppConfig::for_tests());
    app.open_document(doc, DocKey::untitled(1), "Plants".into());
    app
}

fn list(effects: &[Effect]) -> Option<(String, Vec<String>)> {
    effects.iter().find_map(|e| match e {
        Effect::ShowList { title, items } => Some((title.clone(), items.clone())),
        _ => None,
    })
}

fn cursor(app: &App) -> CharPos {
    app.session.as_ref().map_or(CharPos::ZERO, |s| s.cursor)
}

#[test]
fn summarizes_the_chapter_at_the_cursor_and_jumps_to_a_sentence() {
    let doc = fixture("s/chapters.md");
    let two = doc.slice(doc.full_range()).find("Water moves").unwrap_or(0);
    let two = CharPos(doc.slice(doc.full_range())[..two].chars().count());
    let mut app = app_with(doc);
    app.dispatch(Command::SetCursor(two));
    app.settings.summary.sentences = 3;
    let e = app.dispatch(Command::Action(ActionId::Summarize));
    let (title, items) = list(&e).expect("a list");
    assert_eq!(title, "Chapter summary, 3 sentences");
    assert!(
        app.status_text()
            .starts_with("Chapter summary, 3 sentences. Enter goes"),
        "{}",
        app.status_text()
    );
    assert_eq!(items.len(), 3);
    assert!(
        items
            .iter()
            .all(|i| !i.contains("train") && !i.contains("fence")),
        "{items:?}"
    );
    assert!(
        items.iter().all(|i| i.to_lowercase().contains("water")),
        "{items:?}"
    );
    // Enter on the second sentence: the cursor lands on it, and it is said.
    let ranges = match &app.list {
        Some(ListKind::Summary(r)) => r.clone(),
        other => panic!("{other:?}"),
    };
    app.dispatch(Command::Choose(1));
    assert_eq!(cursor(&app), ranges[1].start);
    assert_eq!(app.status_text(), items[1]);
    assert!(app.list.is_none());
}

#[test]
fn summarizes_the_selection_or_the_whole_document() {
    let doc = fixture("s/chapters.md");
    let text = doc.slice(doc.full_range());
    let chars = |needle: &str| text[..text.find(needle).unwrap_or(0)].chars().count();
    let range = CharRange::new(chars("Plants use"), chars("The gardener"));
    let mut app = app_with(doc);
    app.select(range);
    let (title, items) = list(&app.dispatch(Command::Action(ActionId::Summarize))).unwrap();
    assert_eq!(title, "Selection summary, 2 sentences");
    assert_eq!(items.len(), 2);
    // No chapters: the whole document.
    let mut app = app_with(Document::from_plain_text(
        "Readers hear every sentence of the document aloud. \
         The reader speaks each sentence of the document clearly. \
         A cat sat on the mat today.",
    ));
    app.settings.summary.sentences = 1;
    let (title, items) = list(&app.dispatch(Command::Action(ActionId::Summarize))).unwrap();
    assert_eq!(title, "Summary, 1 sentence");
    assert_eq!(items.len(), 1);
    assert!(items[0].contains("document"), "{items:?}");
}

#[test]
fn nothing_to_summarize_is_said() {
    let mut app = app_with(Document::from_plain_text("Too short. Also."));
    let e = app.dispatch(Command::Action(ActionId::Summarize));
    assert!(list(&e).is_none());
    assert_eq!(
        app.status_text(),
        "Nothing to summarize: no sentence of four words or more."
    );
}

#[test]
fn every_new_message_fits_a_braille_line_with_the_key_fact_first() {
    let app = app_with(Document::from_plain_text("x"));
    for (id, key) in [
        ("summary-title", "Summary"),
        ("summary-title-chapter", "Chapter summary"),
        ("summary-title-selection", "Selection summary"),
    ] {
        let t = app.msg_args(id, &args!["n" => 12]);
        assert!(t.starts_with(key) && t.chars().count() <= 40, "{t}");
    }
}
