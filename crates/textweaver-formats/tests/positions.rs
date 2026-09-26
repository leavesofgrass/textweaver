//! Property tests: on arbitrary input, every loader either fails with an
//! error or returns a document whose markers all lie inside its text, in
//! order, with ranges that do not run backwards. No input may panic.

use std::io::{Cursor, Write};

use proptest::prelude::*;
use textweaver_formats::{LoadOptions, Registry, Source};
use textweaver_text::Document;
use zip::write::SimpleFileOptions;

fn load(data: Vec<u8>, hint: &str) -> Option<Document> {
    Registry::with_builtins()
        .load(
            &Source::Bytes {
                data,
                hint: hint.into(),
            },
            &LoadOptions::default(),
        )
        .ok()
}

/// Every marker in bounds and ordered by start.
fn check(doc: &Document) -> Result<(), TestCaseError> {
    let len = doc.len_chars();
    let mut last = 0;
    for m in doc.markers() {
        prop_assert!(m.range.start <= m.range.end, "{:?} runs backwards", m);
        prop_assert!(m.range.end.0 <= len, "{:?} past the end ({len})", m);
        prop_assert!(m.range.start.0 >= last, "{:?} out of order", m);
        last = m.range.start.0;
    }
    Ok(())
}

/// Text made of Markdown's structural characters, words, and newlines.
fn markdown() -> impl Strategy<Value = String> {
    let pieces = prop::sample::select(vec![
        "# ",
        "## ",
        "- ",
        "1. ",
        "> ",
        "> [!NOTE]\n",
        "```\n",
        "`",
        "*",
        "**",
        "_",
        "~~",
        "$",
        "$$",
        "\\frac{a}{b}",
        "[",
        "]",
        "(",
        ")",
        "[[",
        "]]",
        "|",
        "---\n",
        "***\n",
        "[^1]",
        "[^1]: note\n",
        "<b>",
        "</b>",
        "<!--",
        "-->",
        "\n",
        "\n\n",
        "  ",
        "word",
        "é",
        "😀",
        "{#id}",
        "| a | b |\n|---|---|\n",
        "- [ ] ",
        "---\ntitle: x\n---\n",
        "\t",
    ]);
    prop::collection::vec(pieces, 0..80).prop_map(|v| v.concat())
}

/// Tag soup: HTML's elements, open and closed in any order.
fn html() -> impl Strategy<Value = String> {
    let pieces = prop::sample::select(vec![
        "<p>",
        "</p>",
        "<h1>",
        "</h1>",
        "<ul>",
        "<li>",
        "</li>",
        "</ul>",
        "<ol start=\"-5\">",
        "<table>",
        "<tr>",
        "<td>",
        "<th>",
        "</table>",
        "<pre>",
        "</pre>",
        "<code>",
        "<a href=x>",
        "</a>",
        "<img alt=\"pic\">",
        "<br>",
        "<script>",
        "</script>",
        "<div hidden>",
        "</div>",
        "<blockquote>",
        "text",
        " ",
        "&amp;",
        "&#0;",
        "<",
        ">",
        "<meta charset=utf-8>",
    ]);
    prop::collection::vec(pieces, 0..80).prop_map(|v| v.concat())
}

/// A WordprocessingML body made of paragraphs, runs, lists, tables, and
/// odd nesting.
fn docx_body() -> impl Strategy<Value = String> {
    let pieces = prop::sample::select(vec![
        "<w:p>",
        "</w:p>",
        "<w:r><w:t>text</w:t></w:r>",
        "<w:pPr><w:numPr><w:ilvl w:val=\"9\"/><w:numId w:val=\"1\"/></w:numPr></w:pPr>",
        "<w:pPr><w:pStyle w:val=\"Heading1\"/></w:pPr>",
        "<w:tbl><w:tr><w:tc>",
        "</w:tc></w:tr></w:tbl>",
        "<w:sdt><w:sdtContent>",
        "</w:sdtContent></w:sdt>",
        "<w:hyperlink w:anchor=\"x\">",
        "</w:hyperlink>",
        "<w:r><w:br/><w:tab/></w:r>",
        "<w:r><w:footnoteReference w:id=\"1\"/></w:r>",
    ]);
    prop::collection::vec(pieces, 0..60).prop_map(|v| v.concat())
}

fn zip(files: &[(&str, &str)]) -> Vec<u8> {
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, body) in files {
        z.start_file(*name, SimpleFileOptions::default()).unwrap();
        z.write_all(body.as_bytes()).unwrap();
    }
    z.finish().unwrap().into_inner()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn markdown_positions_stay_in_bounds(src in markdown()) {
        let doc = load(src.into_bytes(), "md").expect("Markdown always loads");
        check(&doc)?;
    }

    #[test]
    fn html_positions_stay_in_bounds(src in html()) {
        let doc = load(src.into_bytes(), "html").expect("HTML always loads");
        check(&doc)?;
    }

    #[test]
    fn text_positions_stay_in_bounds(bytes in prop::collection::vec(any::<u8>(), 0..400)) {
        if let Some(doc) = load(bytes, "txt") {
            check(&doc)?;
        }
    }

    #[test]
    fn docx_positions_stay_in_bounds(body in docx_body(), start in any::<i64>()) {
        let document = format!(
            r#"<?xml version="1.0"?><w:document xmlns:w="w"><w:body>{body}</w:body></w:document>"#
        );
        let numbering = format!(
            r#"<?xml version="1.0"?><w:numbering xmlns:w="w"><w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:start w:val="{start}"/><w:numFmt w:val="upperRoman"/><w:lvlText w:val="%1.%9"/></w:lvl></w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num></w:numbering>"#
        );
        let data = zip(&[
            ("word/document.xml", &document),
            ("word/numbering.xml", &numbering),
        ]);
        // Unbalanced tags are an XML error, not a panic.
        if let Some(doc) = load(data, "docx") {
            check(&doc)?;
        }
    }

    #[test]
    fn epub_positions_stay_in_bounds(chapter in html()) {
        let opf = r#"<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata/><manifest><item id="c" href="c.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="c"/></spine></package>"#;
        let container = r#"<?xml version="1.0"?><container xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="content.opf"/></rootfiles></container>"#;
        let data = zip(&[
            ("mimetype", "application/epub+zip"),
            ("META-INF/container.xml", container),
            ("content.opf", opf),
            ("c.xhtml", &chapter),
        ]);
        if let Some(doc) = load(data, "epub") {
            check(&doc)?;
        }
    }

    #[test]
    fn pdf_bytes_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..600)) {
        let mut data = b"%PDF-1.4\n".to_vec();
        data.extend(bytes);
        if let Some(doc) = load(data, "pdf") {
            check(&doc)?;
        }
    }
}
