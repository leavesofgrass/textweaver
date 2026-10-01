//! Lists from the Markdown loader in every writer. A list and its only item
//! share one range, which once sent the tree builder into endless recursion
//! and overflowed the stack for EPUB, DOCX, BRF and PDF.

use crate::common;

use common::options;
use textweaver_formats::{LoadOptions, Loader, MarkdownLoader, Source};
use textweaver_text::Document;
use textweaver_writers::model::{self, Block, Inline, List};
use textweaver_writers::{Format, write_to_vec};

const NESTED: &str = "- one\n  - nested\n- two\n";

fn md(src: &str) -> Document {
    MarkdownLoader
        .load(
            &Source::Bytes {
                data: src.as_bytes().to_vec(),
                hint: "md".into(),
            },
            &LoadOptions::default(),
        )
        .expect("markdown loads")
}

fn only_list(blocks: &[Block]) -> &List {
    match blocks {
        [Block::List(list)] => list,
        other => panic!("expected one list, got {other:?}"),
    }
}

fn text(block: &Block) -> String {
    match block {
        Block::Paragraph(content) => model::collapse_ws(&Inline::plain(content)),
        other => panic!("expected a paragraph, got {other:?}"),
    }
}

fn writes_every_format(src: &str) {
    let doc = md(src);
    for format in Format::ALL {
        let (bytes, _) = write_to_vec(&doc, format, &options())
            .unwrap_or_else(|e| panic!("{} fails: {e}", format.spoken_name()));
        assert!(!bytes.is_empty(), "{} is empty", format.spoken_name());
    }
}

#[test]
fn nested_list_keeps_its_nesting() {
    let blocks = model::blocks(&md(NESTED));
    let list = only_list(&blocks);
    assert_eq!(list.items.len(), 2);
    let first = &list.items[0].blocks;
    assert_eq!(first.len(), 2, "{first:?}");
    assert_eq!(text(&first[0]), "one");
    let inner = only_list(&first[1..]);
    assert_eq!(inner.items.len(), 1);
    assert_eq!(text(&inner.items[0].blocks[0]), "nested");
    assert_eq!(text(&list.items[1].blocks[0]), "two");
}

#[test]
fn one_item_list_is_one_item() {
    let blocks = model::blocks(&md("- one\n"));
    let list = only_list(&blocks);
    assert_eq!(list.items.len(), 1);
    assert_eq!(list.items[0].blocks.len(), 1);
    assert_eq!(text(&list.items[0].blocks[0]), "one");
}

#[test]
fn list_as_the_only_content_of_an_item_nests() {
    let blocks = model::blocks(&md("- - x\n"));
    let outer = only_list(&blocks);
    assert_eq!(outer.items.len(), 1);
    let inner = only_list(&outer.items[0].blocks);
    assert_eq!(inner.items.len(), 1);
    assert_eq!(text(&inner.items[0].blocks[0]), "x");
}

#[test]
fn nested_list_writes_every_format() {
    writes_every_format(NESTED);
}

#[test]
fn one_item_list_writes_every_format() {
    writes_every_format("- one\n");
}

#[test]
fn numbered_list_inside_a_quote_writes_every_format() {
    writes_every_format("> 1. first\n>    - inner\n> 2. second\n");
}

#[test]
fn sample_fixture_writes_every_format() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sample.md");
    let src = std::fs::read_to_string(path).expect("fixtures/sample.md");
    writes_every_format(&src);
}

#[test]
fn very_deep_nesting_does_not_overflow() {
    let src: String = (0..200)
        .map(|d| format!("{}- level {d}\n", "  ".repeat(d)))
        .collect();
    writes_every_format(&src);
}
