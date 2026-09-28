//! Nested lists loaded by the Markdown loader, written in every format.
//!
//! Before: a nested bullet list made every writer but HTML and text
//! recurse until the stack overflowed, which aborted `tw convert`. The
//! loader gives a nested list and its only item the same range, and the
//! tree builder took the list as a child of its own item. The writers'
//! other list tests build their documents by hand, so they never saw the
//! markers the Markdown loader makes.

use textweaver_formats::{LoadOptions, Registry, Source};
use textweaver_writers::{Format, WriteOptions, write_to_vec};

fn load(md: &str) -> textweaver_text::Document {
    let source = Source::Bytes {
        data: md.as_bytes().to_vec(),
        hint: "md".into(),
    };
    Registry::with_builtins()
        .load(&source, &LoadOptions::default())
        .expect("the Markdown loads")
}

/// Every format writes `md` without a crash.
fn writes_everywhere(md: &str) {
    let doc = load(md);
    for format in Format::ALL {
        let (bytes, _report) = write_to_vec(&doc, format, &WriteOptions::default())
            .unwrap_or_else(|e| panic!("{format:?}: {e}"));
        assert!(!bytes.is_empty(), "{format:?} wrote nothing");
    }
}

#[test]
fn a_nested_bullet_list_writes() {
    writes_everywhere("- one\n  - nested\n- two\n");
}

#[test]
fn deeper_and_numbered_nesting_writes() {
    writes_everywhere(
        "1. first\n   - a\n     - deep\n       1. deeper\n2. second\n   - b\n   - c\n",
    );
}

#[test]
fn a_nested_list_in_a_quote_writes() {
    writes_everywhere("> - quoted\n>   - nested\n> - again\n");
}

#[test]
fn the_nesting_is_kept_in_the_epub() {
    let doc = load("- one\n  - nested\n- two\n");
    let (bytes, _) = write_to_vec(&doc, Format::Epub, &WriteOptions::default()).unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("an EPUB is a zip");
    let mut xhtml = String::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).unwrap();
        if f.name().ends_with(".xhtml") && !f.name().ends_with("nav.xhtml") {
            std::io::Read::read_to_string(&mut f, &mut xhtml).unwrap();
        }
    }
    let one = xhtml.find("one").expect("one is written");
    let inner = xhtml[one..].find("<ul").expect("a nested list follows one");
    let nested = xhtml[one..].find("nested").expect("nested is written");
    let two = xhtml.find("two").expect("two is written");
    assert!(inner < nested, "nested is inside the inner list: {xhtml}");
    assert!(one + nested < two, "{xhtml}");
    assert_eq!(xhtml.matches("<ul").count(), 2, "{xhtml}");
}
