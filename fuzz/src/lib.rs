//! Shared checks for the fuzz targets: a loader may fail with an error, but
//! it must never panic, and a document it returns must have every marker
//! inside its text, in order, with no range running backwards.

use std::io::{Cursor, Write};

use textweaver_formats::{FootnoteMode, HtmlOptions, LoadOptions, Registry, Source};
use zip::write::SimpleFileOptions;

/// Loads `data` as a file with extension `hint` and checks the result,
/// with the default options and with code skipped and notes inline.
pub fn load_checked(data: &[u8], hint: &str) {
    for options in [
        LoadOptions::default(),
        LoadOptions {
            skip_code: true,
            footnotes: FootnoteMode::Inline,
            ..LoadOptions::default()
        },
    ] {
        let source = Source::Bytes {
            data: data.to_vec(),
            hint: hint.to_owned(),
        };
        let Ok(doc) = Registry::with_builtins().load(&source, &options) else {
            continue;
        };
        let len = doc.len_chars();
        let mut last = 0;
        for m in doc.markers() {
            assert!(m.range.start <= m.range.end, "{m:?} runs backwards");
            assert!(m.range.end.0 <= len, "{m:?} past the end ({len})");
            assert!(m.range.start.0 >= last, "{m:?} out of order");
            last = m.range.start.0;
        }
        // The exports walk every marker; they must not panic either.
        let _ = textweaver_formats::to_markdown(&doc);
        let _ = textweaver_formats::to_html(&doc, &HtmlOptions::default());
    }
}

/// A zip archive of `files`.
pub fn zip(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, body) in files {
        if z.start_file(*name, SimpleFileOptions::default()).is_err() || z.write_all(body).is_err()
        {
            return Vec::new();
        }
    }
    z.finish().map(Cursor::into_inner).unwrap_or_default()
}

/// Fuzz input as a Word document: raw bytes when they are a zip archive,
/// else the bytes as `word/document.xml`, with the second half (after a
/// NUL, when there is one) as `word/numbering.xml`.
pub fn docx(data: &[u8]) -> Vec<u8> {
    if data.starts_with(b"PK") {
        return data.to_vec();
    }
    let (document, numbering) = split(data);
    zip(&[
        ("word/document.xml", document),
        ("word/numbering.xml", numbering),
    ])
}

/// Fuzz input as an EPUB: raw bytes when they are a zip archive, else the
/// bytes as the one chapter, with the second half (after a NUL) as the
/// navigation document.
pub fn epub(data: &[u8]) -> Vec<u8> {
    if data.starts_with(b"PK") {
        return data.to_vec();
    }
    let (chapter, nav) = split(data);
    let opf = br#"<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata/><manifest><item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/><item id="c" href="c.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="c"/></spine></package>"#;
    let container = br#"<?xml version="1.0"?><container xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="content.opf"/></rootfiles></container>"#;
    zip(&[
        ("mimetype", b"application/epub+zip"),
        ("META-INF/container.xml", container),
        ("content.opf", opf),
        ("c.xhtml", chapter),
        ("nav.xhtml", nav),
    ])
}

fn split(data: &[u8]) -> (&[u8], &[u8]) {
    match data.iter().position(|&b| b == 0) {
        Some(i) => (&data[..i], &data[i + 1..]),
        None => (data, b""),
    }
}
