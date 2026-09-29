//! Math, strikethrough, and horizontal rules from Markdown in every writer:
//! math is typeset (never printed with its `$` delimiters), struck text is
//! struck, and a rule is drawn.

mod common;

use common::{entries, options};
use textweaver_formats::{LoadOptions, Loader, MarkdownLoader, Source};
use textweaver_text::Document;
use textweaver_writers::{Format, PdfOptions, WriteError, WriteOptions, write_to_vec};

const SOURCE: &str =
    "# Circles\n\nThe area is $\\pi r^2$ and ~~was~~ is known.\n\n---\n\n$$\\frac{a+b}{2}$$\n";

fn md(src: &str) -> Document {
    MarkdownLoader
        .load(
            &Source::Bytes {
                data: src.as_bytes().to_vec(),
                hint: "md".into(),
            },
            &LoadOptions::default(),
        )
        .expect("markdown loads")
}

#[test]
fn docx_writes_office_math_strike_and_a_rule() {
    let (bytes, _) = write_to_vec(&md(SOURCE), Format::Docx, &options()).unwrap();
    let files = entries(&bytes);
    let xml = String::from_utf8(files["word/document.xml"].clone()).unwrap();
    roxmltree::Document::parse(&xml).expect("well-formed document.xml");
    assert!(!xml.contains("$\\pi"), "raw math in {xml}");
    assert!(xml.contains("<m:oMath><m:r>"), "{xml}");
    assert!(xml.contains("<m:sSup><m:e>"), "{xml}");
    assert!(
        xml.contains("<m:oMathPara><m:oMath><m:f><m:num>"),
        "display math: {xml}"
    );
    assert!(xml.contains("<w:strike/>"), "{xml}");
    assert!(xml.contains("<w:pBdr><w:bottom"), "{xml}");
    // Read back, the formula is LaTeX math again.
    let back = textweaver_formats::Registry::with_builtins()
        .load(
            &Source::Bytes {
                data: bytes,
                hint: "docx".into(),
            },
            &LoadOptions::default(),
        )
        .unwrap();
    let text = back.text().to_string();
    assert!(
        text.contains("The area is $\\pi r^{2}$ and was is known."),
        "{text}"
    );
}

#[test]
fn epub_writes_mathml_with_the_manifest_property() {
    let (bytes, _) = write_to_vec(&md(SOURCE), Format::Epub, &options()).unwrap();
    let files = entries(&bytes);
    let (chapter, xhtml) = files
        .iter()
        .find(|(n, d)| n.ends_with(".xhtml") && String::from_utf8_lossy(d).contains("<math"))
        .expect("a chapter with MathML");
    let xhtml = String::from_utf8_lossy(xhtml);
    assert!(!xhtml.contains("$\\pi"), "{xhtml}");
    assert!(xhtml.contains("<del>was</del>"), "{xhtml}");
    assert!(xhtml.contains("<hr/>"), "{xhtml}");
    assert!(xhtml.contains("display=\"block\""), "{xhtml}");
    let opf = files
        .iter()
        .find(|(n, _)| n.ends_with(".opf"))
        .map(|(_, d)| String::from_utf8_lossy(d).into_owned())
        .unwrap();
    let name = chapter.rsplit('/').next().unwrap();
    let item = opf
        .lines()
        .find(|l| l.contains(&format!("href=\"{name}\"")))
        .unwrap();
    assert!(item.contains("properties=\"mathml\""), "{item}");
}

/// Without MathCAT, braille reads math as it is spoken (with it, math is
/// Nemeth or UEB: tests/math_braille.rs).
#[cfg(not(feature = "mathcat"))]
#[test]
fn braille_reads_math_as_it_is_spoken() {
    let with_math = md("The area is $\\pi r^2$.\n");
    let spoken = md("The area is pi r squared.\n");
    let a = write_to_vec(&with_math, Format::Brf, &options()).unwrap().0;
    let b = write_to_vec(&spoken, Format::Brf, &options()).unwrap().0;
    assert_eq!(a, b);
}

#[test]
fn pdf_typesets_math_in_a_formula_tag() {
    let opts = WriteOptions {
        pdf: PdfOptions {
            compress: false,
            ..PdfOptions::default()
        },
        ..options()
    };
    let bytes = match write_to_vec(&md(SOURCE), Format::Pdf, &opts) {
        Ok((b, _)) => b,
        Err(WriteError::NoFont) => {
            eprintln!("no font for PDF output on this system; skipping");
            return;
        }
        Err(e) => panic!("{e}"),
    };
    let has = |needle: &str| bytes.windows(needle.len()).any(|w| w == needle.as_bytes());
    // PDF/UA: math in a Formula tag with its spoken form as alt text (the
    // validator in krilla runs by default and would have failed the write).
    assert!(has("/S /Formula") || has("/S/Formula"), "no Formula tag");
    assert!(has("(pi r squared)"), "no alt text");
    // The page shows the linear form, not the LaTeX source.
    let doc = textweaver_formats::Registry::with_builtins()
        .load(
            &Source::Bytes {
                data: bytes.clone(),
                hint: "pdf".into(),
            },
            &LoadOptions::default(),
        )
        .expect("the PDF reads back");
    let text = doc.text().to_string();
    assert!(text.contains("\u{3C0}r\u{B2}"), "{text}");
    assert!(text.contains("(a + b)/2"), "{text}");
    assert!(!text.contains('$'), "{text}");
    assert!(!text.contains("\\pi"), "{text}");
}
