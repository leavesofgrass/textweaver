//! Malformed and hostile input: deep nesting, huge list counters, and
//! binary files. Every case must load (or fail with a clear error) without
//! overflowing the stack, overflowing a counter, or reading more than it
//! needs to.

use std::io::{Cursor, Write};

use textweaver_core::MarkerKind;
use textweaver_formats::{
    LoadError, LoadOptions, MAX_NESTING, NESTING_WARNING, Registry, Source, warnings,
};
use textweaver_text::Document;
use zip::write::SimpleFileOptions;

/// Far deeper than any real document, and deep enough to overflow a
/// recursive walker's stack on a test thread.
const DEEP: usize = 20_000;

fn zip(files: &[(&str, &str)]) -> Vec<u8> {
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, body) in files {
        z.start_file(*name, SimpleFileOptions::default()).unwrap();
        z.write_all(body.as_bytes()).unwrap();
    }
    z.finish().unwrap().into_inner()
}

fn load(data: Vec<u8>, hint: &str) -> Result<Document, LoadError> {
    Registry::with_builtins().load(
        &Source::Bytes {
            data,
            hint: hint.into(),
        },
        &LoadOptions::default(),
    )
}

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

fn docx(body: &str, numbering: Option<&str>) -> Vec<u8> {
    let document = format!(
        r#"<?xml version="1.0"?><w:document xmlns:w="{W}"><w:body>{body}</w:body></w:document>"#
    );
    let mut files = vec![("word/document.xml", document.as_str())];
    let numbering = numbering
        .map(|n| format!(r#"<?xml version="1.0"?><w:numbering xmlns:w="{W}">{n}</w:numbering>"#));
    if let Some(n) = &numbering {
        files.push(("word/numbering.xml", n.as_str()));
    }
    zip(&files)
}

fn para(text: &str) -> String {
    format!("<w:p><w:r><w:t>{text}</w:t></w:r></w:p>")
}

fn item(num: &str, ilvl: u8, text: &str) -> String {
    format!(
        r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="{ilvl}"/><w:numId w:val="{num}"/></w:numPr></w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>"#
    )
}

fn item_labels(doc: &Document) -> Vec<String> {
    doc.marker_index()
        .iter(MarkerKind::ListItem, None)
        .map(|m| m.label.clone().unwrap_or_default())
        .collect()
}

#[test]
fn docx_hostile_list_counters_are_clamped() {
    let numbering = format!(
        r#"<w:abstractNum w:abstractNumId="1">
            <w:lvl w:ilvl="0"><w:start w:val="4294967295"/><w:numFmt w:val="upperRoman"/><w:lvlText w:val="%1."/></w:lvl>
            <w:lvl w:ilvl="1"><w:start w:val="-99999999999999999999999"/><w:numFmt w:val="lowerLetter"/><w:lvlText w:val="{}"/></w:lvl>
            <w:lvl w:ilvl="2"><w:start w:val="99999999999999999999999"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1.%2.%3"/></w:lvl>
          </w:abstractNum>
          <w:num w:numId="7"><w:abstractNumId w:val="1"/>
            <w:lvlOverride w:ilvl="0"><w:startOverride w:val="18446744073709551615"/></w:lvlOverride>
          </w:num>"#,
        "%1%2".repeat(5_000)
    );
    let body = [
        item("7", 0, "one"),
        item("7", 0, "two"),
        item("7", 0, "three"),
        item("7", 1, "deep"),
        item("7", 2, "deeper"),
        item("7", 250, "past the last level"),
    ]
    .concat();
    let doc = load(docx(&body, Some(&numbering)), "docx").unwrap();
    let labels = item_labels(&doc);
    assert_eq!(labels.len(), 6, "{labels:?}");
    // The counter starts at the cap and stays there instead of overflowing
    // (roman numerals past 3,999 read as decimal).
    assert_eq!(&labels[..3], ["999999.", "999999.", "999999."]);
    for l in &labels {
        assert!(l.chars().count() <= 64, "{l}");
    }
    assert!(labels[3].starts_with("999999a"), "{}", labels[3]);
    assert_eq!(labels[4], "999999.a.999999");
}

#[test]
fn docx_deep_nesting_is_flattened() {
    let mut body = String::new();
    body.push_str(&para("Before."));
    for _ in 0..DEEP {
        body.push_str("<w:sdt><w:sdtContent>");
    }
    body.push_str(&para("Deep block."));
    for _ in 0..DEEP {
        body.push_str("</w:sdtContent></w:sdt>");
    }
    // Deep inline nesting inside one paragraph.
    body.push_str("<w:p>");
    for _ in 0..DEEP {
        body.push_str("<w:smartTag>");
    }
    body.push_str("<w:r><w:t>Deep run.</w:t></w:r>");
    for _ in 0..DEEP {
        body.push_str("</w:smartTag>");
    }
    body.push_str("</w:p>");
    // Tables nested in cells.
    for _ in 0..DEEP / 4 {
        body.push_str("<w:tbl><w:tr><w:tc>");
    }
    body.push_str(&para("Deep cell."));
    for _ in 0..DEEP / 4 {
        body.push_str("</w:tc></w:tr></w:tbl>");
    }
    body.push_str(&para("After."));
    let doc = load(docx(&body, None), "docx").unwrap();
    let text = doc.text().to_string();
    for want in [
        "Before.",
        "Deep block.",
        "Deep run.",
        "Deep cell.",
        "After.",
    ] {
        assert!(text.contains(want), "{want} missing from {text:?}");
    }
    assert_eq!(warnings(&doc.meta), vec![NESTING_WARNING.to_owned()]);
}

#[test]
fn docx_shallow_nesting_keeps_structure_and_has_no_warning() {
    // Each level is two elements, under `w:document` and `w:body`, around
    // a paragraph, a run, and its text: just inside the limit.
    let levels = (MAX_NESTING - 5) / 2;
    let mut body = String::new();
    for _ in 0..levels {
        body.push_str("<w:sdt><w:sdtContent>");
    }
    body.push_str(&para("Kept."));
    for _ in 0..levels {
        body.push_str("</w:sdtContent></w:sdt>");
    }
    let doc = load(docx(&body, None), "docx").unwrap();
    assert_eq!(doc.text().to_string(), "Kept.");
    assert_eq!(doc.marker_index().count(MarkerKind::Paragraph, None), 1);
    assert!(warnings(&doc.meta).is_empty());
}

/// HTML nested `depth` deep in `open`/`close` tags around `inner`.
fn nested_html(depth: usize, open: &str, close: &str, inner: &str) -> String {
    let mut s = String::from("<!doctype html><html><body><p>Before.</p>");
    for _ in 0..depth {
        s.push_str(open);
    }
    s.push_str(inner);
    for _ in 0..depth {
        s.push_str(close);
    }
    s.push_str("<p>After.</p></body></html>");
    s
}

#[test]
fn html_deep_nesting_is_flattened() {
    let html = nested_html(
        DEEP,
        "<span>",
        "</span>",
        "<b>Deep</b> <i>text</i><script>not read</script><em hidden>hidden</em>",
    );
    let doc = load(html.into_bytes(), "html").unwrap();
    let text = doc.text().to_string();
    assert!(text.contains("Before."), "{text}");
    assert!(text.contains("Deep text"), "{text}");
    assert!(text.contains("After."), "{text}");
    assert!(!text.contains("not read"), "{text}");
    assert!(!text.contains("hidden"), "{text}");
    assert_eq!(warnings(&doc.meta), vec![NESTING_WARNING.to_owned()]);

    // html5ever's tree builder is quadratic in the depth of list items, so
    // this depth keeps the test quick while still far past the limit.
    let lists = nested_html(4_000, "<ul><li>", "</li></ul>", "Deep item");
    let doc = load(lists.into_bytes(), "html").unwrap();
    assert!(doc.text().to_string().contains("Deep item"));
}

#[test]
fn epub_deep_chapter_and_toc_load() {
    let depth = 3_000;
    let chapter = nested_html(depth, "<div>", "</div>", "Deep chapter text.").replace(
        "<!doctype html><html>",
        "<html xmlns=\"http://www.w3.org/1999/xhtml\">",
    );
    let mut toc = String::from("<ol>");
    for i in 0..depth {
        toc.push_str(&format!("<li><a href=\"c1.xhtml\">Level {i}</a><ol>"));
    }
    for _ in 0..depth {
        toc.push_str("</ol></li>");
    }
    toc.push_str("</ol>");
    let nav = format!(
        r#"<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><body><nav epub:type="toc">{toc}</nav></body></html>"#
    );
    let mut ncx_map = String::new();
    for i in 0..depth {
        ncx_map.push_str(&format!(
            "<navPoint id=\"n{i}\"><navLabel><text>Point {i}</text></navLabel><content src=\"c1.xhtml\"/>"
        ));
    }
    for _ in 0..depth {
        ncx_map.push_str("</navPoint>");
    }
    let ncx = format!(
        r#"<?xml version="1.0"?><ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1"><navMap>{ncx_map}</navMap></ncx>"#
    );
    let opf = r#"<?xml version="1.0"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="id">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Deep</dc:title><dc:identifier id="id">x</dc:identifier></metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
    <item id="c1" href="c1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine toc="ncx"><itemref idref="c1"/></spine>
</package>"#;
    let container = r#"<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#;
    for with_nav in [true, false] {
        let opf = if with_nav {
            opf.to_owned()
        } else {
            opf.replace(" properties=\"nav\"", "")
        };
        let data = zip(&[
            ("mimetype", "application/epub+zip"),
            ("META-INF/container.xml", container),
            ("OEBPS/content.opf", &opf),
            ("OEBPS/nav.xhtml", &nav),
            ("OEBPS/toc.ncx", &ncx),
            ("OEBPS/c1.xhtml", &chapter),
        ]);
        let doc = load(data, "epub").unwrap();
        let text = doc.text().to_string();
        assert!(text.contains("Deep chapter text."), "{text}");
        assert!(text.contains("After."), "{text}");
        assert_eq!(warnings(&doc.meta), vec![NESTING_WARNING.to_owned()]);
        let sections = doc.marker_index().count(MarkerKind::SectionBreak, None);
        assert!(sections <= MAX_NESTING + 1, "{sections} sections");
    }
}

#[test]
fn binary_files_are_refused_from_their_first_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("movie.txt");
    let mut data = b"\x00\x01\x02 binary".to_vec();
    data.resize(4 << 20, 0);
    std::fs::write(&path, &data).unwrap();
    let head = Source::Path(path.clone())
        .read_head(textweaver_formats::encoding::SNIFF_BYTES)
        .unwrap();
    assert_eq!(head.len(), textweaver_formats::encoding::SNIFF_BYTES);
    let err = Registry::with_builtins()
        .load(&Source::Path(path), &LoadOptions::default())
        .unwrap_err();
    assert!(matches!(err, LoadError::Binary(..)), "{err}");
    // A short text file reads whole.
    let short = dir.path().join("short.txt");
    std::fs::write(&short, "hello").unwrap();
    assert_eq!(
        Source::Path(short).read_head(8192).unwrap(),
        b"hello".to_vec()
    );
}
