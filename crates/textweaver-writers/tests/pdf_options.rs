//! PDF export options (Agent W): the bundled default font, fonts by name,
//! large print, page numbers, the title page and table of contents, working
//! internal links, the document language, and alt-text warnings; and fonts
//! embedded in EPUB. Every PDF here passes krilla's PDF/UA-1 validator
//! (on by default), or the write would fail.

mod common;

use common::{entries, fixture, options, sample};
use textweaver_formats::{LoadOptions, Loader, MarkdownLoader, Source};
use textweaver_text::Document;
use textweaver_writers::{
    EpubOptions, Format, PdfOptions, WriteError, WriteOptions, WriteReport, write_to_vec,
};

fn markdown(text: &str) -> Document {
    MarkdownLoader
        .load(
            &Source::Bytes {
                data: text.as_bytes().to_vec(),
                hint: "md".into(),
            },
            &LoadOptions::default(),
        )
        .expect("markdown loads")
}

fn with_pdf(pdf: PdfOptions) -> WriteOptions {
    WriteOptions {
        pdf: PdfOptions {
            compress: false,
            ..pdf
        },
        ..options()
    }
}

fn pdf(doc: &Document, options: &WriteOptions) -> (Vec<u8>, WriteReport) {
    write_to_vec(doc, Format::Pdf, options).unwrap_or_else(|e| panic!("{e}"))
}

fn count(hay: &[u8], needle: &str) -> usize {
    let needle = needle
        .replace(" /", "/")
        .replace(" (", "(")
        .replace(" [", "[");
    hay.windows(needle.len())
        .filter(|w| *w == needle.as_bytes())
        .count()
}

fn has(hay: &[u8], needle: &str) -> bool {
    count(hay, needle) > 0
}

/// The text of pages `first..=last` through `pdftotext`, when installed.
fn page_text(bytes: &[u8], first: usize, last: usize) -> Option<String> {
    if !common::runs("pdftotext", "-v") {
        eprintln!("pdftotext is not installed; skipping the text check");
        return None;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.pdf");
    std::fs::write(&path, bytes).unwrap();
    let out = std::process::Command::new("pdftotext")
        .args(["-raw", "-enc", "UTF-8", "-f"])
        .arg(first.to_string())
        .arg("-l")
        .arg(last.to_string())
        .arg(&path)
        .arg("-")
        .output()
        .unwrap();
    assert!(out.status.success());
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn pages(bytes: &[u8]) -> usize {
    count(bytes, "/Type /Page") - count(bytes, "/Type /Pages")
}

#[cfg(feature = "bundled-fonts")]
#[test]
fn the_bundled_fonts_are_the_default() {
    let (bytes, report) = pdf(&sample(), &with_pdf(PdfOptions::default()));
    assert!(report.warnings.is_empty(), "{report:?}");
    // Subset fonts are named ABCDEF+Family-Style.
    assert!(has(&bytes, "+AtkinsonHyperlegibleNext-Regular"));
    assert!(has(&bytes, "+AtkinsonHyperlegibleNext-Bold"));
    // The sample has code.
    assert!(has(&bytes, "+AtkinsonHyperlegibleMono-Regular"));
}

#[cfg(feature = "bundled-fonts")]
#[test]
fn fonts_are_chosen_by_name() {
    let (bytes, _) = pdf(
        &sample(),
        &with_pdf(PdfOptions {
            font_family: Some("OpenDyslexic".into()),
            ..PdfOptions::default()
        }),
    );
    assert!(has(&bytes, "+OpenDyslexic-Regular"));
    assert!(!has(&bytes, "+AtkinsonHyperlegibleNext-Regular"));
    let err = write_to_vec(
        &sample(),
        Format::Pdf,
        &with_pdf(PdfOptions {
            font_family: Some("Not A Real Font Family".into()),
            ..PdfOptions::default()
        }),
    )
    .unwrap_err();
    assert!(matches!(err, WriteError::Font(..)), "{err}");
    assert!(err.to_string().contains("OpenDyslexic"), "{err}");
}

#[test]
fn large_print_is_18_points_with_gentler_headings() {
    let doc = markdown("# Title\n\nBody text in large print.\n\n    code stays full size\n");
    let (bytes, _) = pdf(&doc, &with_pdf(PdfOptions::large_print()));
    // Body and code at 18 points, the level 1 heading at 1.5 times that.
    assert!(has(&bytes, " 18 Tf"), "no 18 pt text");
    assert!(has(&bytes, " 27 Tf"), "no 27 pt heading");
    assert!(!has(&bytes, " 16.2 Tf"), "code was shrunk");
    // A size below 18 is raised in large print.
    let small = PdfOptions {
        font_size: 11.0,
        large_print: true,
        ..PdfOptions::default()
    };
    let (bytes, _) = pdf(&doc, &with_pdf(small));
    assert!(has(&bytes, " 18 Tf"));
}

#[test]
fn page_numbers_can_be_turned_off() {
    let doc = markdown("Hello.\n");
    let (with, _) = pdf(&doc, &with_pdf(PdfOptions::default()));
    let (without, _) = pdf(
        &doc,
        &with_pdf(PdfOptions {
            page_numbers: false,
            ..PdfOptions::default()
        }),
    );
    assert!(has(&with, "/Footer"));
    assert!(!has(&without, "/Footer"));
    if let Some(t) = page_text(&with, 1, 1) {
        assert!(t.contains("Page 1 of 1"), "{t}");
    }
    if let Some(t) = page_text(&without, 1, 1) {
        assert!(!t.contains("Page 1"), "{t}");
    }
}

#[test]
fn title_page_and_table_of_contents() {
    let doc = sample();
    let options = WriteOptions {
        pdf: PdfOptions {
            title_page: true,
            date: Some("September 25, 2026".into()),
            toc: true,
            compress: false,
            ..PdfOptions::default()
        },
        ..options()
    };
    let (bytes, report) = pdf(&doc, &options);
    assert!(report.warnings.is_empty(), "{report:?}");
    assert!(has(&bytes, "/S /TOC"));
    // The sample's five headings are all at levels 1 to 3.
    assert_eq!(count(&bytes, "/S /TOCI"), 5);
    // "Contents" is a heading too, and first in the bookmarks.
    assert!(has(&bytes, "/T (Contents)"));
    assert!(has(&bytes, "/Title (Contents)"));
    // Title page, contents page, then the body.
    assert!(pages(&bytes) >= 3);
    if let Some(t) = page_text(&bytes, 1, 1) {
        assert!(t.contains("Reading Guide"), "{t}");
        assert!(t.contains("Jon Pielaet"), "{t}");
        assert!(t.contains("September 25, 2026"), "{t}");
        assert!(
            !t.contains("Page 1 of"),
            "the title page has no number: {t}"
        );
    }
    if let Some(t) = page_text(&bytes, 2, 2) {
        assert!(t.contains("Contents"), "{t}");
        // Each entry ends with the page its heading is on (page 3 or later).
        let first = t
            .lines()
            .find(|l| l.starts_with("Reading Guide"))
            .expect("an entry for the first heading");
        let n: usize = first.rsplit(' ').next().unwrap().trim().parse().unwrap();
        assert!(n >= 3, "{first}");
    }
    // No date given and none in the document: the title page has none
    // (textweaver never guesses today's date).
    let undated = WriteOptions {
        pdf: PdfOptions {
            title_page: true,
            ..PdfOptions::default()
        },
        ..common::options()
    };
    let (bytes, _) = pdf(&doc, &undated);
    if let Some(t) = page_text(&bytes, 1, 1) {
        assert!(!t.contains("2026"), "{t}");
    }
}

#[test]
fn internal_links_jump_and_dangling_ones_are_reported() {
    let doc = markdown(
        "# Start\n\nSee [the second part](#second-part) and a note.[^n]\n\n\
         ## Second part\n\nMore text. [Back](#start).\n\n[^n]: The note.\n",
    );
    let (bytes, report) = pdf(&doc, &with_pdf(PdfOptions::default()));
    assert!(report.warnings.is_empty(), "{report:?}");
    // Three internal links (two headings, one footnote), each an XYZ
    // destination, tagged as links.
    assert_eq!(count(&bytes, "/Subtype /Link"), 3);
    assert!(count(&bytes, "/XYZ") >= 3, "internal destinations");
    assert_eq!(count(&bytes, "/URI"), 0);
    assert_eq!(count(&bytes, "/S /Link"), 3);

    let doc = markdown("# Only\n\nA [broken link](#nowhere).\n");
    let (bytes, report) = pdf(&doc, &with_pdf(PdfOptions::default()));
    assert_eq!(count(&bytes, "/Subtype /Link"), 0);
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("not in this document")),
        "{report:?}"
    );
}

#[test]
fn the_language_is_set_on_the_catalog_and_structure() {
    let doc = markdown("Bonjour.\n");
    let options = WriteOptions {
        language: Some("fr-CA".into()),
        ..with_pdf(PdfOptions::default())
    };
    let (bytes, _) = pdf(&doc, &options);
    assert!(count(&bytes, "/Lang (fr-CA)") >= 2);
}

#[test]
fn images_without_alt_text_are_decorative_and_reported() {
    // The Markdown loader drops images with no alt text, so build the
    // document by hand: a blank image paragraph, then a described one.
    use textweaver_core::{CharRange, MarkerKind};
    use textweaver_text::{DocumentMeta, Marker};
    let text = " \n\nA single pixel";
    let doc = Document::new(
        DocumentMeta::default(),
        ropey::Rope::from_str(text),
        vec![
            Marker::new(MarkerKind::Paragraph, CharRange::new(0, 1)),
            Marker::new(MarkerKind::Image, CharRange::new(0, 1)).with_reference("pixel.png"),
            Marker::new(MarkerKind::Paragraph, CharRange::new(3, 17)),
            Marker::new(MarkerKind::Image, CharRange::new(3, 17)).with_reference("pixel.png"),
        ],
    );
    let options = WriteOptions {
        resource_dir: Some(fixture("")),
        ..with_pdf(PdfOptions::default())
    };
    let (bytes, report) = pdf(&doc, &options);
    assert_eq!(count(&bytes, "/S /Figure"), 1);
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.starts_with("An image has no description")),
        "{report:?}"
    );
}

#[test]
fn page_size_margins_and_spacing() {
    let doc = markdown(&"Words to fill a page. ".repeat(400));
    let base = pdf(&doc, &with_pdf(PdfOptions::default())).0;
    let roomy = pdf(
        &doc,
        &with_pdf(PdfOptions {
            page_size: textweaver_writers::PageSize::parse("a5").unwrap(),
            margin: textweaver_writers::parse_length("1.5in").unwrap(),
            line_spacing: 2.0,
            ..PdfOptions::default()
        }),
    )
    .0;
    assert!(has(&roomy, "/MediaBox [0 0 419.53 595.28]"));
    assert!(pages(&roomy) > pages(&base));
}

#[cfg(feature = "bundled-fonts")]
#[test]
fn epub_embeds_bundled_fonts_with_their_licence() {
    let options = WriteOptions {
        epub: EpubOptions {
            font: Some("opendyslexic".into()),
            code_font: Some("Atkinson Hyperlegible Mono".into()),
            ..EpubOptions::default()
        },
        ..options()
    };
    let (bytes, report) = write_to_vec(&sample(), Format::Epub, &options).unwrap();
    assert!(report.warnings.is_empty(), "{report:?}");
    let files = entries(&bytes);
    for f in [
        "OEBPS/fonts/opendyslexic/OpenDyslexic-Regular.otf",
        "OEBPS/fonts/opendyslexic/OpenDyslexic-Bold-Italic.otf",
        "OEBPS/fonts/opendyslexic/OFL.txt",
        "OEBPS/fonts/atkinson-hyperlegible-mono/AtkinsonHyperlegibleMono-Regular.ttf",
        "OEBPS/fonts/atkinson-hyperlegible-mono/OFL.txt",
    ] {
        assert!(files.contains_key(f), "missing {f}");
    }
    let licence = std::str::from_utf8(&files["OEBPS/fonts/opendyslexic/OFL.txt"]).unwrap();
    assert!(licence.contains("Reserved Font Name OpenDyslexic"));
    let css = std::str::from_utf8(&files["OEBPS/style.css"]).unwrap();
    assert!(css.contains("@font-face { font-family: \"OpenDyslexic\"; font-weight: bold; font-style: italic; src: url(\"fonts/opendyslexic/OpenDyslexic-Bold-Italic.otf\"); }"), "{css}");
    assert!(css.contains("body { font-family: \"OpenDyslexic\", sans-serif; }"));
    assert!(css.contains("pre, code { font-family: \"Atkinson Hyperlegible Mono\", monospace; }"));
    let opf = std::str::from_utf8(&files["OEBPS/content.opf"]).unwrap();
    assert!(
        opf.contains(
            "href=\"fonts/opendyslexic/OpenDyslexic-Regular.otf\" media-type=\"font/otf\""
        )
    );
    assert!(opf.contains(
        "href=\"fonts/atkinson-hyperlegible-mono/AtkinsonHyperlegibleMono-Bold.ttf\" media-type=\"font/ttf\""
    ));
    assert!(opf.contains("href=\"fonts/opendyslexic/OFL.txt\" media-type=\"text/plain\""));
    // Every manifest href is in the package.
    let doc = common::xml(opf).unwrap();
    for item in doc.descendants().filter(|n| n.has_tag_name("item")) {
        let href = common::attr(item, "href").unwrap();
        assert!(files.contains_key(&format!("OEBPS/{href}")), "{href}");
    }

    // A font that is not bundled is left out, with a warning.
    let options = WriteOptions {
        epub: EpubOptions {
            font: Some("Comic Sans MS".into()),
            ..EpubOptions::default()
        },
        ..common::options()
    };
    let (bytes, report) = write_to_vec(&sample(), Format::Epub, &options).unwrap();
    assert!(!entries(&bytes).keys().any(|k| k.contains("fonts/")));
    assert!(
        report.warnings[0].contains("only bundled fonts"),
        "{report:?}"
    );
}
