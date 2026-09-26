//! Shared checks for the fuzz targets: a loader may fail with an error, but
//! it must never panic, and a document it returns must have every marker
//! inside its text, in order, with no range running backwards.

use std::io::{Cursor, Write};

use textweaver_formats::{
    FootnoteMode, HtmlOptions, LoadOptions, OcrOptions, Registry, Source,
};
use zip::write::SimpleFileOptions;

/// Loads `data` as a file with extension `hint` and checks the result,
/// with the default options and with code skipped and notes inline. OCR is
/// off: the fuzzer tests the loaders, not the recognition engines.
pub fn load_checked(data: &[u8], hint: &str) {
    let no_ocr = OcrOptions {
        enabled: false,
        ..OcrOptions::default()
    };
    for options in [
        LoadOptions {
            ocr: no_ocr.clone(),
            ..LoadOptions::default()
        },
        LoadOptions {
            skip_code: true,
            footnotes: FootnoteMode::Inline,
            ocr: no_ocr.clone(),
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

/// Fuzz input as a PowerPoint file: raw bytes when they are a zip
/// archive, else the bytes as the one slide, with the second half (after a
/// NUL) as its speaker notes.
pub fn pptx(data: &[u8]) -> Vec<u8> {
    if data.starts_with(b"PK") {
        return data.to_vec();
    }
    let (slide, notes) = split(data);
    let rels = |target: &str, ty: &str| {
        format!(
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/{ty}" Target="{target}"/></Relationships>"#
        )
    };
    let pres = br#"<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst></p:presentation>"#;
    let pres_rels = rels("slides/slide1.xml", "slide");
    let slide_rels = rels("../notesSlides/notesSlide1.xml", "notesSlide");
    zip(&[
        ("ppt/presentation.xml", pres),
        ("ppt/_rels/presentation.xml.rels", pres_rels.as_bytes()),
        ("ppt/slides/slide1.xml", slide),
        ("ppt/slides/_rels/slide1.xml.rels", slide_rels.as_bytes()),
        ("ppt/notesSlides/notesSlide1.xml", notes),
    ])
}

/// Fuzz input as a DAISY book in a zip: raw bytes when they are a zip
/// archive, else the bytes as the DTBook, with the second half (after a
/// NUL) as the NCX.
pub fn daisy(data: &[u8]) -> Vec<u8> {
    if data.starts_with(b"PK") {
        return data.to_vec();
    }
    let (book, ncx) = split(data);
    let opf = br#"<package><manifest><item id="n" href="nav.ncx" media-type="application/x-dtbncx+xml"/><item id="s" href="a.smil" media-type="application/smil"/><item id="d" href="book.xml" media-type="application/x-dtbook+xml"/></manifest><spine><itemref idref="s"/></spine></package>"#;
    let smil = br#"<smil><body><seq><par id="p1"><text src="book.xml#x1"/></par></seq></body></smil>"#;
    zip(&[
        ("b/package.opf", opf),
        ("b/a.smil", smil),
        ("b/book.xml", book),
        ("b/nav.ncx", ncx),
    ])
}

/// Fuzz input as a spreadsheet: the first byte picks CSV, TSV, OpenDocument,
/// or Excel; the rest is the file, or (for the zip-based two, when it is not
/// a zip) the sheet's XML.
pub fn sheet(data: &[u8]) -> (Vec<u8>, &'static str) {
    let Some((&pick, rest)) = data.split_first() else {
        return (Vec::new(), "csv");
    };
    match pick % 4 {
        0 => (rest.to_vec(), "csv"),
        1 => (rest.to_vec(), "tsv"),
        2 if rest.starts_with(b"PK") => (rest.to_vec(), "ods"),
        2 => (zip(&[("content.xml", rest)]), "ods"),
        _ if rest.starts_with(b"PK") => (rest.to_vec(), "xlsx"),
        _ => {
            let root = br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#;
            let wb = br#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="S" sheetId="1" r:id="rId1"/></sheets></workbook>"#;
            let wb_rels = br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#;
            (
                zip(&[
                    ("_rels/.rels", root),
                    ("xl/workbook.xml", wb),
                    ("xl/_rels/workbook.xml.rels", wb_rels),
                    ("xl/worksheets/sheet1.xml", rest),
                ]),
                "xlsx",
            )
        }
    }
}

fn split(data: &[u8]) -> (&[u8], &[u8]) {
    match data.iter().position(|&b| b == 0) {
        Some(i) => (&data[..i], &data[i + 1..]),
        None => (data, b""),
    }
}
