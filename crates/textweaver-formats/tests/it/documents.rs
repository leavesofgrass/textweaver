//! The W4c2 fixtures (`fixtures/c2/`): an RTF handout, ODT lecture notes,
//! and a Word draft with comments and tracked changes, loaded through the
//! registry as the reader and `tw` load them.

use std::path::PathBuf;

use textweaver_core::MarkerKind;
use textweaver_formats::{LoadOptions, Registry, RevisionMode, Source, comments, revision_count};
use textweaver_text::Document;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/c2")
        .join(name)
}

fn load_with(name: &str, options: &LoadOptions) -> Document {
    Registry::with_builtins()
        .load(&Source::Path(fixture(name)), options)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn load(name: &str) -> Document {
    load_with(name, &LoadOptions::default())
}

fn marked() -> LoadOptions {
    LoadOptions {
        revisions: RevisionMode::Marked,
        ..LoadOptions::default()
    }
}

fn texts(doc: &Document, kind: MarkerKind) -> Vec<String> {
    doc.markers()
        .iter()
        .filter(|m| m.kind == kind)
        .map(|m| doc.slice(m.range).to_string())
        .collect()
}

#[test]
fn the_rtf_handout_reads_without_pandoc() {
    let doc = load("handout.rtf");
    assert_eq!(doc.meta.format, "rtf");
    assert_eq!(
        doc.meta.title.as_deref(),
        Some("Week 3 Handout: Reading Primary Sources")
    );
    assert_eq!(doc.meta.author.as_deref(), Some("Ada Example"));
    let text = doc.text().to_string();
    assert!(text.contains("A café ledger from 1911 counts as a source\u{2014}so does"));
    assert!(text.contains("in Russian is ворона."));
    assert!(text.contains("Bring three questions."), "{text}");
    assert_eq!(
        texts(&doc, MarkerKind::Heading),
        vec![
            "Week 3 Handout: Reading Primary Sources",
            "Steps",
            "Sources for this week",
            "Footnotes"
        ]
    );
    assert_eq!(texts(&doc, MarkerKind::ListItem).len(), 3);
    assert_eq!(texts(&doc, MarkerKind::TableRow).len(), 3);
    assert_eq!(texts(&doc, MarkerKind::Link), vec!["the course page"]);
    assert!(text.ends_with("[1] Portland Audubon runs a winter crow count every year."));
    assert_eq!(revision_count(&doc.meta), 2);
    let said = load_with("handout.rtf", &marked()).text().to_string();
    assert!(
        said.contains(
            "Bring (deleted by Ada Example: two) (inserted by Ada Example: three) questions."
        ),
        "{said}"
    );
}

#[test]
fn the_odt_notes_read_with_their_comment_and_changes() {
    let doc = load("notes.odt");
    assert_eq!(doc.meta.format, "odt");
    assert_eq!(
        doc.meta.title.as_deref(),
        Some("Lecture Notes: The Water Cycle")
    );
    assert_eq!(doc.meta.language.as_deref(), Some("en-US"));
    let text = doc.text().to_string();
    assert!(text.contains("The quiz is on Thursday."), "{text}");
    let items: Vec<(String, Option<String>)> = doc
        .markers()
        .iter()
        .filter(|m| m.kind == MarkerKind::ListItem)
        .map(|m| (doc.slice(m.range).to_string(), m.label.clone()))
        .collect();
    assert_eq!(items.len(), 5);
    assert_eq!(
        items[1],
        ("From oceans and lakes".into(), Some("a)".into()))
    );
    assert_eq!(items[4], ("Precipitation".into(), Some("3.".into())));
    assert_eq!(texts(&doc, MarkerKind::TableRow).len(), 3);
    assert_eq!(texts(&doc, MarkerKind::Image).len(), 1);
    assert_eq!(texts(&doc, MarkerKind::Bold), vec!["ocean"]);
    let cs = comments(&doc.meta);
    assert_eq!(cs.len(), 1);
    assert_eq!(
        doc.slice(cs[0].range).to_string(),
        "Evaporation is the first step"
    );
    assert_eq!(
        cs[0].spoken(),
        "Comment by Ada Example: This is on the exam."
    );
    assert_eq!(revision_count(&doc.meta), 2);
    let said = load_with("notes.odt", &marked()).text().to_string();
    assert!(
        said.contains(
            "The quiz is on (deleted by Bo Example: Tuesday) (inserted by Ada Example: Thursday)."
        ),
        "{said}"
    );
}

#[test]
fn the_word_draft_carries_comments_and_changes() {
    let doc = load("comments.docx");
    assert_eq!(
        doc.text().to_string(),
        "Lab Report Draft\n\nWe measured the boiling point of salt water at five concentrations.\n\nThe results table is below."
    );
    let cs = comments(&doc.meta);
    assert_eq!(cs.len(), 2);
    assert_eq!(
        doc.slice(cs[0].range).to_string(),
        "boiling point of salt water"
    );
    assert_eq!(
        cs[0].spoken(),
        "Comment by Ada Example: Say how salty, in grams per liter. Reply by Bo Example: Added: 35 grams per liter. Resolved."
    );
    assert!(cs[1].range.is_empty());
    assert_eq!(cs[1].text, "The table is missing.");
    assert_eq!(revision_count(&doc.meta), 2);
    let said = load_with("comments.docx", &marked()).text().to_string();
    assert!(
        said.contains(
            "at (deleted by Bo Example: three) (inserted by Ada Example: five) concentrations."
        ),
        "{said}"
    );
}

fn t1_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/t1")
        .join(name)
}

#[test]
fn every_tracked_change_is_recorded_with_its_author_and_date() {
    use textweaver_formats::{ChangeKind, changes};
    let registry = Registry::with_builtins();
    let source = Source::Path(t1_fixture("changes.docx"));
    let doc = registry.load(&source, &LoadOptions::default()).unwrap();
    let text = doc.text().to_string();
    assert_eq!(
        text,
        "Case Notes\n\nThe patient has acute renal failure.\n\nFluids were given every hour.\n\nCall the family.\n\nThen check the labs first. Repeat tomorrow."
    );
    let cs = changes(&doc.meta);
    let kinds: Vec<ChangeKind> = cs.iter().map(|c| c.kind).collect();
    assert_eq!(
        kinds,
        vec![
            ChangeKind::Inserted,
            ChangeKind::Deleted,
            ChangeKind::MovedAway,
            ChangeKind::MovedHere,
            ChangeKind::Inserted,
        ]
    );
    assert_eq!(revision_count(&doc.meta), 5);
    let renal = &cs[0];
    assert_eq!(doc.slice(renal.range).to_string(), "renal");
    assert_eq!(renal.author, "Ada Example");
    assert_eq!(renal.date, "2026-03-03T09:15:00Z");
    assert_eq!(renal.id, "1");
    assert_eq!(cs[1].text, "rarely");
    assert!(cs[1].range.is_empty());
    assert_eq!(cs[1].author, "Bo Example");
    assert_eq!(cs[2].text, "Check the labs first.");
    assert_eq!(
        doc.slice(textweaver_core::CharRange::new(
            cs[2].range.start.0,
            cs[2].range.start.0 + 4
        ))
        .to_string(),
        "Call",
        "a deletion at a paragraph start sits after the break"
    );
    assert_eq!(doc.slice(cs[3].range).to_string(), "check the labs first");
    assert_eq!(doc.slice(cs[4].range).to_string(), "Repeat tomorrow.");
    assert!(cs[4].date.is_empty(), "no date is invented");
    // The halves of a move outside any move range pair in reading order.
    assert_eq!((cs[2].pair.as_str(), cs[3].pair.as_str()), ("#1", "#1"));
    assert!(cs[0].pair.is_empty());
    let threads = comments(&doc.meta);
    assert_eq!(threads.len(), 2);
    assert_eq!(doc.slice(threads[0].range).to_string(), "Call the family.");
    assert_eq!(threads[0].replies.len(), 1);
    assert!(threads[0].resolved);
    assert!(threads[1].date.is_empty());

    let said = registry.load(&source, &marked()).unwrap();
    let said_text = said.text().to_string();
    assert!(
        said_text.contains("acute (inserted by Ada Example: renal) failure."),
        "{said_text}"
    );
    let cs = changes(&said.meta);
    assert_eq!(cs.len(), 5);
    assert_eq!(
        said.slice(cs[0].range).to_string(),
        "(inserted by Ada Example: renal)"
    );
    assert_eq!(cs[0].text, "renal");
    assert_eq!(
        said.slice(cs[1].range).to_string(),
        "(deleted by Bo Example: rarely)"
    );
}

#[test]
fn rtf_revision_times_are_read_as_dates() {
    use textweaver_formats::changes;
    // 2026-03-03 09:15, packed as Word's DTTM.
    let packed = 15 | (9 << 6) | (3 << 11) | (3 << 16) | (126 << 20);
    let rtf = format!(
        "{{\\rtf1\\ansi{{\\*\\revtbl {{Unknown;}}{{Ada Example;}}}}\\pard The {{\\revised\\revauth1\\revdttm{packed} new }}plan.\\par}}"
    );
    let doc = Registry::with_builtins()
        .load(
            &Source::Bytes {
                data: rtf.into_bytes(),
                hint: "rtf".into(),
            },
            &LoadOptions::default(),
        )
        .unwrap();
    let cs = changes(&doc.meta);
    assert_eq!(cs.len(), 1);
    assert_eq!(cs[0].author, "Ada Example");
    assert_eq!(cs[0].date, "2026-03-03T09:15:00");
    assert_eq!(doc.slice(cs[0].range).to_string(), "new");
}

/// The halves of a move are paired by the name of Word's move ranges, so
/// they are decided together (task B1-t2).
#[test]
fn the_halves_of_a_named_move_share_their_pair() {
    use textweaver_formats::{ChangeKind, changes};
    let source = Source::Path(t1_fixture("word-review.docx"));
    let doc = Registry::with_builtins()
        .load(&source, &LoadOptions::default())
        .unwrap();
    let moves: Vec<(ChangeKind, String)> = changes(&doc.meta)
        .into_iter()
        .filter(|c| matches!(c.kind, ChangeKind::MovedAway | ChangeKind::MovedHere))
        .map(|c| (c.kind, c.pair))
        .collect();
    assert_eq!(
        moves,
        vec![
            (ChangeKind::MovedAway, "move1".to_owned()),
            (ChangeKind::MovedHere, "move1".to_owned()),
        ]
    );
}
