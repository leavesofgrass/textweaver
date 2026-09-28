//! Fuzz target: the OpenDocument text loader. A zip archive is loaded as
//! it is; other input becomes `content.xml`, and, after a NUL byte,
//! `meta.xml`, in a minimal package, and is also loaded as flat `.fodt`
//! XML. Each is loaded with the default options, with notes inline, and
//! with tracked changes said in place; every comment must lie inside the
//! text.

#![no_main]

use libfuzzer_sys::fuzz_target;
use textweaver_formats::{Registry, RevisionMode, Source, comments};

fn split(data: &[u8]) -> (&[u8], &[u8]) {
    match data.iter().position(|&b| b == 0) {
        Some(i) => (&data[..i], &data[i + 1..]),
        None => (data, &[]),
    }
}

fn marked(bytes: &[u8], hint: &str) {
    let options = textweaver_formats::LoadOptions {
        revisions: RevisionMode::Marked,
        ..textweaver_fuzz::options()
    };
    let source = Source::Bytes {
        data: bytes.to_vec(),
        hint: hint.to_owned(),
    };
    if let Ok(doc) = Registry::with_builtins().load(&source, &options) {
        textweaver_fuzz::check(&doc);
        let len = doc.len_chars();
        for c in comments(&doc.meta) {
            assert!(c.range.start <= c.range.end, "{c:?} runs backwards");
            assert!(c.range.end.0 <= len, "{c:?} past the end ({len})");
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let package = if data.starts_with(b"PK") {
        data.to_vec()
    } else {
        let (content, meta) = split(data);
        textweaver_fuzz::zip(&[
            ("mimetype", b"application/vnd.oasis.opendocument.text"),
            ("content.xml", content),
            ("meta.xml", meta),
        ])
    };
    textweaver_fuzz::load_checked(&package, "odt");
    marked(&package, "odt");
    if !data.starts_with(b"PK") {
        textweaver_fuzz::load_checked(data, "fodt");
        marked(data, "fodt");
    }
});
