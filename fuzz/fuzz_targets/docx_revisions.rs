//! Fuzz target: Word comments and tracked changes. A zip archive is loaded
//! as it is; other input is split at NUL bytes into `word/document.xml`,
//! `word/comments.xml`, and `word/commentsExtended.xml`. Each is loaded
//! with the default options and with tracked changes said in place; every
//! comment must lie inside the text, and replies must not be lost into a
//! loop.

#![no_main]

use libfuzzer_sys::fuzz_target;
use textweaver_formats::{Registry, RevisionMode, Source, comments};

fuzz_target!(|data: &[u8]| {
    let package = if data.starts_with(b"PK") {
        data.to_vec()
    } else {
        let mut parts = data.splitn(3, |&b| b == 0);
        let document = parts.next().unwrap_or_default();
        let notes = parts.next().unwrap_or_default();
        let extended = parts.next().unwrap_or_default();
        textweaver_fuzz::zip(&[
            ("word/document.xml", document),
            ("word/comments.xml", notes),
            ("word/commentsExtended.xml", extended),
        ])
    };
    for revisions in [RevisionMode::Final, RevisionMode::Marked] {
        let options = textweaver_formats::LoadOptions {
            revisions,
            ..textweaver_fuzz::options()
        };
        let source = Source::Bytes {
            data: package.clone(),
            hint: "docx".into(),
        };
        let Ok(doc) = Registry::with_builtins().load(&source, &options) else {
            continue;
        };
        textweaver_fuzz::check(&doc);
        let len = doc.len_chars();
        for c in comments(&doc.meta) {
            assert!(c.range.start <= c.range.end, "{c:?} runs backwards");
            assert!(c.range.end.0 <= len, "{c:?} past the end ({len})");
        }
    }
});
