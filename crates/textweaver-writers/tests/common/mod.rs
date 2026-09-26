//! Helpers shared by the writer integration tests.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use textweaver_core::{CharRange, MarkerKind};
use textweaver_formats::{LoadOptions, Loader, MarkdownLoader, Source};
use textweaver_text::Document;
use textweaver_writers::WriteOptions;

/// `fixtures/m/<name>`.
pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/m")
        .join(name)
}

/// The Markdown sample, loaded with the Markdown loader.
pub fn sample() -> Document {
    MarkdownLoader
        .load(&Source::Path(fixture("sample.md")), &LoadOptions::default())
        .expect("sample.md loads")
}

/// Options with a fixed timestamp, for reproducible output.
pub fn options() -> WriteOptions {
    WriteOptions {
        timestamp: Some(1_790_339_696),
        ..WriteOptions::default()
    }
}

/// Every entry of a zip archive, in archive order.
pub fn unzip(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("a zip archive");
    let mut out = Vec::new();
    for i in 0..archive.len() {
        let mut f = archive.by_index(i).expect("an entry");
        let mut data = Vec::new();
        f.read_to_end(&mut data).expect("entry data");
        out.push((f.name().to_owned(), data));
    }
    out
}

/// A zip archive's entries by name.
pub fn entries(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    unzip(bytes).into_iter().collect()
}

/// Lowercase words (alphanumeric runs) of a text.
pub fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// The document's text without the given kinds of block markers' ranges.
pub fn text_without(doc: &Document, skip: impl Fn(&textweaver_text::Marker) -> bool) -> String {
    let mut cut: Vec<CharRange> = doc
        .markers()
        .iter()
        .filter(|m| skip(m))
        .map(|m| m.range)
        .collect();
    cut.sort_by_key(|r| r.start);
    let mut out = String::new();
    let mut pos = 0;
    for r in cut {
        if r.start.0 > pos {
            out.push_str(&doc.slice(CharRange::new(pos, r.start.0)));
        }
        pos = pos.max(r.end.0);
        out.push(' ');
    }
    out.push_str(&doc.slice(CharRange::new(pos, doc.len_chars())));
    out
}

/// True for a footnote body marker.
pub fn is_footnote_body(m: &textweaver_text::Marker) -> bool {
    m.kind == MarkerKind::Footnote && m.level == 1
}

/// Local name and attribute lookup ignoring namespaces.
pub fn attr<'a>(node: roxmltree::Node<'a, '_>, name: &str) -> Option<&'a str> {
    node.attributes()
        .find(|a| a.name() == name)
        .map(|a| a.value())
}

/// True when `program` runs (with `arg`).
pub fn runs(program: &str, arg: &str) -> bool {
    std::process::Command::new(program)
        .arg(arg)
        .output()
        .is_ok()
}

/// Parses XML, allowing the `<!DOCTYPE html>` of XHTML content documents.
pub fn xml(text: &str) -> Result<roxmltree::Document<'_>, roxmltree::Error> {
    roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        },
    )
}
