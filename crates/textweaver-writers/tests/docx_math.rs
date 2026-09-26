//! Math through DOCX and back: the DOCX writer turns LaTeX into Office Math
//! (OMML), and the DOCX loader turns that back into LaTeX under `Math`
//! markers.

mod common;

use common::options;
use textweaver_core::MarkerKind;
use textweaver_formats::{DocxLoader, LoadOptions, Loader, MarkdownLoader, Source};
use textweaver_text::Document;
use textweaver_writers::{Format, write_to_vec};

fn load(loader: &dyn Loader, data: Vec<u8>, hint: &str) -> Document {
    loader
        .load(
            &Source::Bytes {
                data,
                hint: hint.into(),
            },
            &LoadOptions::default(),
        )
        .expect("the document loads")
}

/// (covered text without spaces, level) of every `Math` marker.
fn math(doc: &Document) -> Vec<(String, u8)> {
    doc.markers()
        .iter()
        .filter(|m| m.kind == MarkerKind::Math)
        .map(|m| {
            let text: String = doc
                .slice(m.range)
                .to_string()
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            (text, m.level)
        })
        .collect()
}

#[test]
fn latex_survives_a_docx_round_trip() {
    let md = "Inline $\\frac{a}{b}$ and $x^{2}$ and $\\sqrt{x}$ and $\\left( a \\right)$.\n\n\
              $$\\sum_{i=1}^{n} i$$\n";
    let source = load(&MarkdownLoader, md.as_bytes().to_vec(), "md");
    let (docx, _) = write_to_vec(&source, Format::Docx, &options()).expect("DOCX writes");
    let back = load(&DocxLoader, docx, "docx");
    // Spacing may differ; the LaTeX, its delimiters, and the levels match.
    assert_eq!(
        math(&back),
        vec![
            ("$\\frac{a}{b}$".to_owned(), 0),
            ("$x^{2}$".to_owned(), 0),
            ("$\\sqrt{x}$".to_owned(), 0),
            ("$\\left(a\\right)$".to_owned(), 0),
            ("$$\\sum_{i=1}^{n}i$$".to_owned(), 1),
        ],
        "{}",
        back.text()
    );
    // The formulas are where they were in the text.
    let text = back.text().to_string();
    assert!(
        text.starts_with("Inline $\\frac{a}{b}$ and $x^{2}$"),
        "{text}"
    );
}
