//! Print page numbers read back from EPUB and HTML (W9x-i, idea card 1):
//! `epub:type="pagebreak"` and `role="doc-pagebreak"` elements, and the
//! EPUB 3 page list, become `PageBreak` markers labeled with the printed
//! number, so "go to page 112" works as it does in PDF and DAISY.

use std::fmt::Write as _;
use std::io::{Cursor, Write};

use textweaver_core::MarkerKind;
use textweaver_formats::{LoadOptions, Registry, Source};
use textweaver_text::Document;
use zip::write::SimpleFileOptions;

fn epub(chapters: &[(&str, String)], nav: &str) -> Vec<u8> {
    let manifest: String = chapters
        .iter()
        .enumerate()
        .map(|(i, (name, _))| {
            format!(r#"<item id="c{i}" href="{name}" media-type="application/xhtml+xml"/>"#)
        })
        .collect();
    let spine: String = (0..chapters.len())
        .map(|i| format!(r#"<itemref idref="c{i}"/>"#))
        .collect();
    let opf = format!(
        r#"<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="id"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="id">pages</dc:identifier><dc:title>Pages</dc:title><dc:language>en</dc:language></metadata><manifest><item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>{manifest}</manifest><spine>{spine}</spine></package>"#
    );
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let mut add = |name: &str, body: &str| {
        z.start_file(name, SimpleFileOptions::default()).unwrap();
        z.write_all(body.as_bytes()).unwrap();
    };
    add("mimetype", "application/epub+zip");
    add(
        "META-INF/container.xml",
        r#"<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#,
    );
    add("OEBPS/content.opf", &opf);
    add("OEBPS/nav.xhtml", nav);
    for (name, body) in chapters {
        add(&format!("OEBPS/{name}"), body);
    }
    z.finish().unwrap().into_inner()
}

fn load(data: Vec<u8>, hint: &str) -> Document {
    Registry::with_builtins()
        .load(
            &Source::Bytes {
                data,
                hint: hint.into(),
            },
            &LoadOptions::default(),
        )
        .unwrap()
}

fn pages(doc: &Document) -> Vec<(String, String)> {
    doc.marker_index()
        .iter(MarkerKind::PageBreak, None)
        .map(|m| (m.label.clone().unwrap_or_default(), doc.slice(m.range)))
        .collect()
}

const NAV: &str = r#"<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><body><nav epub:type="toc"><ol><li><a href="one.xhtml">One</a></li><li><a href="two.xhtml">Two</a></li></ol></nav></body></html>"#;

#[test]
fn a_400_page_epub_reaches_page_112() {
    let chapter = |from: usize, to: usize| {
        let mut s = String::from(
            r#"<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><body>"#,
        );
        for n in from..=to {
            write!(
                s,
                r#"<span epub:type="pagebreak" role="doc-pagebreak" id="page{n}" title="{n}"/><p>Text of page {n}.</p>"#
            )
            .unwrap();
        }
        s.push_str("</body></html>");
        s
    };
    let doc = load(
        epub(
            &[
                ("one.xhtml", chapter(1, 200)),
                ("two.xhtml", chapter(201, 400)),
            ],
            NAV,
        ),
        "epub",
    );
    let pages = pages(&doc);
    assert_eq!(pages.len(), 400);
    let (_, text) = pages.iter().find(|(l, _)| l == "112").unwrap();
    assert_eq!(text.trim(), "Text of page 112.");
    assert_eq!(pages[399].0, "400");
    // The numbers themselves are not read aloud.
    assert!(doc.text().to_string().starts_with("Text of page 1."));
}

#[test]
fn a_page_runs_on_into_the_next_chapter() {
    let one = r#"<html xmlns:epub="http://www.idpf.org/2007/ops"><body><p>Before.</p><span epub:type="pagebreak" aria-label="Page iv"></span><p>Start.</p></body></html>"#;
    let two = r#"<html><body><p>Carried on.</p><div role="doc-pagebreak">5</div><p>Five.</p></body></html>"#;
    let doc = load(
        epub(
            &[("one.xhtml", one.to_owned()), ("two.xhtml", two.to_owned())],
            NAV,
        ),
        "epub",
    );
    let pages = pages(&doc);
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].0, "iv");
    assert!(pages[0].1.contains("Start.") && pages[0].1.contains("Carried on."));
    assert_eq!(pages[1].0, "5");
    assert_eq!(pages[1].1.trim(), "Five.");
}

#[test]
fn the_epub_3_page_list_names_pages_by_id() {
    let nav = r#"<html xmlns:epub="http://www.idpf.org/2007/ops"><body><nav epub:type="toc"><ol><li><a href="one.xhtml">One</a></li></ol></nav><nav epub:type="page-list" hidden="hidden"><ol><li><a href="one.xhtml#p7">7</a></li><li><a href="one.xhtml#p8">8</a></li></ol></nav></body></html>"#;
    let one = r#"<html><body><p id="p7">Seven.</p><p id="p8">Eight.</p></body></html>"#;
    let doc = load(epub(&[("one.xhtml", one.to_owned())], nav), "epub");
    let pages = pages(&doc);
    let labels: Vec<&str> = pages.iter().map(|(l, _)| l.as_str()).collect();
    assert_eq!(labels, ["7", "8"]);
    assert_eq!(pages[1].1.trim(), "Eight.");
}

#[test]
fn html_page_breaks_are_pages() {
    let html = r#"<p>Front.</p><span class="pagebreak" role="doc-pagebreak" id="page-12" aria-label="Page 12"></span><p>Twelve.</p><span role="doc-pagebreak" aria-label="Page 13"></span><p>Thirteen.</p>"#;
    let doc = load(html.as_bytes().to_vec(), "html");
    let pages = pages(&doc);
    let labels: Vec<&str> = pages.iter().map(|(l, _)| l.as_str()).collect();
    assert_eq!(labels, ["12", "13"]);
    assert_eq!(pages[0].1.trim(), "Twelve.");
    assert_eq!(doc.text().to_string(), "Front.\n\nTwelve.\n\nThirteen.");
}
