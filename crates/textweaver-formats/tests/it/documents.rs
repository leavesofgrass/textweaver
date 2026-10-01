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
