//! EPUB accessibility metadata (EPUB Accessibility 1.0 and 1.1): read from
//! the package document into the document's properties, with nothing for
//! a book that gives none, and hostile values cut and cleaned.

use std::io::{Cursor, Write};

use textweaver_formats::epub::{Accessibility, accessibility, term_words};
use textweaver_formats::{LoadOptions, Registry, Source};
use textweaver_text::Document;
use zip::write::SimpleFileOptions;

/// A one-chapter EPUB whose package metadata ends with `extra`.
fn epub(extra: &str) -> Vec<u8> {
    let opf = format!(
        r#"<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="id"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="id">access-test</dc:identifier><dc:title>Cells</dc:title><dc:creator>Ada Example</dc:creator><dc:language>en</dc:language>{extra}</metadata><manifest><item id="c1" href="chapter.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="c1"/></spine></package>"#
    );
    let files = [
        ("mimetype", "application/epub+zip".to_owned()),
        (
            "META-INF/container.xml",
            r#"<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#.to_owned(),
        ),
        ("OEBPS/content.opf", opf),
        (
            "OEBPS/chapter.xhtml",
            r#"<html xmlns="http://www.w3.org/1999/xhtml"><body><h1>Cells</h1><p>A cell is small.</p></body></html>"#.to_owned(),
        ),
    ];
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, body) in files {
        z.start_file(name, SimpleFileOptions::default()).unwrap();
        z.write_all(body.as_bytes()).unwrap();
    }
    z.finish().unwrap().into_inner()
}

fn load(data: Vec<u8>) -> Document {
    Registry::with_builtins()
        .load(
            &Source::Bytes {
                data,
                hint: "epub".into(),
            },
            &LoadOptions::default(),
        )
        .unwrap()
}

const EPUB3: &str = r##"
<meta property="schema:accessMode">textual</meta>
<meta property="schema:accessMode">visual</meta>
<meta property="schema:accessModeSufficient">textual</meta>
<meta property="schema:accessModeSufficient">textual,visual</meta>
<meta property="schema:accessibilityFeature">tableOfContents</meta>
<meta property="schema:accessibilityFeature">alternativeText</meta>
<meta property="schema:accessibilityFeature">tableOfContents</meta>
<meta property="schema:accessibilityHazard">none</meta>
<meta property="schema:accessibilitySummary">Every  picture has
  a description.</meta>
<meta property="dcterms:conformsTo">EPUB Accessibility 1.1 - WCAG 2.1 Level AA</meta>
<meta property="a11y:certifiedBy">Example Access Lab</meta>
<meta property="a11y:certifierCredential" refines="#c">Credential</meta>
"##;

#[test]
fn epub3_metadata_is_read() {
    let doc = load(epub(EPUB3));
    let a = accessibility(&doc.meta).expect("an EPUB has the property");
    assert_eq!(
        a,
        Accessibility {
            summary: Some("Every picture has a description.".into()),
            access_modes: vec!["textual".into(), "visual".into()],
            sufficient: vec![
                vec!["textual".into()],
                vec!["textual".into(), "visual".into()]
            ],
            features: vec!["tableOfContents".into(), "alternativeText".into()],
            hazards: vec!["none".into()],
            conformance: vec!["EPUB Accessibility 1.1, WCAG 2.1 Level AA".into()],
            certifier: Some("Example Access Lab".into()),
        }
    );
    assert_eq!(a.reads_as_text(), Some(true));
    // The title and author are still read as before.
    assert_eq!(doc.meta.title.as_deref(), Some("Cells"));
    assert_eq!(doc.meta.author.as_deref(), Some("Ada Example"));
}

#[test]
fn epub_without_metadata_says_so() {
    let doc = load(epub(""));
    let a = accessibility(&doc.meta).expect("an EPUB has the property");
    assert!(a.is_empty());
    assert_eq!(a.reads_as_text(), None);
    // Not an EPUB: no accessibility information at all.
    let md = Registry::with_builtins()
        .load(
            &Source::Bytes {
                data: b"# Hi".to_vec(),
                hint: "md".into(),
            },
            &LoadOptions::default(),
        )
        .unwrap();
    assert_eq!(accessibility(&md.meta), None);
}

#[test]
fn epub2_and_accessibility_1_0_forms() {
    let extra = r##"
<meta name="schema:accessModeSufficient" content="textual, visual"/>
<meta name="schema:accessibilityFeature" content="printPageNumbers"/>
<meta name="schema:accessibilityHazard" content="noFlashingHazard"/>
<link rel="dcterms:conformsTo" href="http://www.idpf.org/epub/a11y/accessibility-20170105.html#wcag-aa"/>
"##;
    let a = accessibility(&load(epub(extra)).meta).unwrap();
    assert_eq!(a.sufficient, vec![vec!["textual", "visual"]]);
    assert_eq!(a.reads_as_text(), Some(false));
    assert_eq!(a.features, ["printPageNumbers"]);
    assert_eq!(a.hazards, ["noFlashingHazard"]);
    assert_eq!(a.conformance, ["EPUB Accessibility 1.0, WCAG 2.0 Level AA"]);
}

#[test]
fn hostile_values_are_cut_and_cleaned() {
    let long = "word ".repeat(2_000);
    let many: String = (0..200)
        .map(|i| format!("<meta property=\"schema:accessibilityFeature\">feature{i}</meta>"))
        .collect();
    let extra = format!(
        "<meta property=\"schema:accessibilitySummary\">{long}</meta>{many}<meta property=\"a11y:certifiedBy\">Bad&#x9F;Name</meta>"
    );
    let a = accessibility(&load(epub(&extra)).meta).unwrap();
    let summary = a.summary.unwrap();
    assert!(summary.chars().count() <= textweaver_formats::epub::MAX_ACCESS_CHARS + 3);
    assert!(summary.ends_with("..."));
    assert_eq!(a.features.len(), textweaver_formats::epub::MAX_ACCESS_ITEMS);
    assert_eq!(a.certifier.as_deref(), Some("BadName"));
}

#[test]
fn terms_in_words() {
    for (term, words) in [
        ("tableOfContents", "table of contents"),
        ("structuralNavigation", "structural navigation"),
        ("textual", "text"),
        ("visual", "images"),
        ("MathML", "MathML"),
        ("noFlashingHazard", "no flashing hazard"),
        ("none", "none"),
    ] {
        assert_eq!(term_words(term), words);
    }
}
