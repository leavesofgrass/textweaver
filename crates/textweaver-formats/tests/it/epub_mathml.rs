//! EPUB 3 MathML read as math (W4c1, ADR-0029): each `<math>` becomes
//! LaTeX with its delimiters under a `Math` marker; `alttext` and image
//! fallbacks are read when there is no math; an `epub:switch` is read once.

use std::io::{Cursor, Write};

use textweaver_core::MarkerKind;
use textweaver_formats::{LoadOptions, Registry, Source};
use textweaver_text::Document;
use zip::write::SimpleFileOptions;

const CHAPTER: &str = include_str!("../../../../fixtures/c1/mathml-chapter.xhtml");

fn epub(chapter: &str) -> Vec<u8> {
    let files = [
        ("mimetype", "application/epub+zip"),
        (
            "META-INF/container.xml",
            r#"<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#,
        ),
        (
            "OEBPS/content.opf",
            r#"<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="id"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="id">c1-mathml</dc:identifier><dc:title>MathML test</dc:title><dc:language>en</dc:language></metadata><manifest><item id="c1" href="chapter.xhtml" media-type="application/xhtml+xml" properties="mathml"/></manifest><spine><itemref idref="c1"/></spine></package>"#,
        ),
        ("OEBPS/chapter.xhtml", chapter),
    ];
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, body) in files {
        z.start_file(name, SimpleFileOptions::default()).unwrap();
        z.write_all(body.as_bytes()).unwrap();
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

fn math_texts(doc: &Document) -> Vec<(String, u8)> {
    doc.marker_index()
        .iter(MarkerKind::Math, None)
        .map(|m| (doc.slice(m.range), m.level))
        .collect()
}

#[test]
fn epub_mathml_is_math() {
    let doc = load(epub(CHAPTER), "epub");
    let text = doc.text().to_string();
    assert_eq!(
        math_texts(&doc),
        [
            ("$x^{2}+1$".to_owned(), 0),
            (r"$$x=\frac{-b\pm\sqrt{b^{2}-4ac}}{2a}$$".to_owned(), 1),
            (r"$\frac{1}{2}$".to_owned(), 0),
            (r"$\sqrt{z}$".to_owned(), 0),
            ("$n!$".to_owned(), 0),
        ],
        "{text}"
    );
    // Text around inline math stays in its sentence.
    assert!(
        text.contains("Presentation MathML: $x^{2}+1$ grows."),
        "{text}"
    );
    // Display math is on a line of its own.
    assert!(
        text.contains("\n$$x=\\frac{-b\\pm\\sqrt{b^{2}-4ac}}{2a}$$\n"),
        "{text}"
    );
    // No math, only alternative text: the text is read.
    assert!(
        text.contains("Only alternative text: the area of the circle ends."),
        "{text}"
    );
    // A switch is read once: its MathML case, or else its default image.
    assert!(text.contains("A switch: $n!$ once."), "{text}");
    assert!(!text.contains("n factorial"), "{text}");
    assert!(text.contains("E equals m c squared"), "{text}");
    assert!(!text.contains("smil"), "{text}");
    // The MathML's own token text is not read as a run of letters.
    assert!(!text.contains("x2+1"), "{text}");
}

/// Web pages read MathML as math too (W5c3, ADR-0035), with the same
/// formulas as the EPUB chapter.
#[test]
fn html_loader_reads_mathml_as_math() {
    let doc = load(CHAPTER.as_bytes().to_vec(), "xhtml");
    let math = math_texts(&doc);
    assert!(!math.is_empty());
    let book = load(epub(CHAPTER), "epub");
    for formula in &math {
        assert!(math_texts(&book).contains(formula), "{formula:?}");
    }
}

/// Hostile MathML in a chapter loads without overflowing the stack.
#[test]
fn deep_mathml_in_an_epub_loads() {
    let deep = format!(
        r#"<html xmlns="http://www.w3.org/1999/xhtml"><body><p><math>{}<mi>x</mi>{}</math></p></body></html>"#,
        "<mrow><mfrac><mn>1</mn>".repeat(3000),
        "</mfrac></mrow>".repeat(3000)
    );
    let doc = load(epub(&deep), "epub");
    assert!(doc.text().to_string().contains('x'));
}
