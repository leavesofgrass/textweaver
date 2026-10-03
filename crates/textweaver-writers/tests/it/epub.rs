//! EPUB 3 writer: package structure, semantics, accessibility metadata,
//! a round trip through the HTML loader, and epubcheck when installed.

use crate::common;

use std::collections::{BTreeMap, BTreeSet};

use common::{
    attr, entries, fixture, is_footnote_body, options, sample, text_without, unzip, words,
};
use ropey::Rope;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_formats::{HtmlLoader, LoadOptions, Loader, Source};
use textweaver_text::{Document, DocumentMeta, Marker};
use textweaver_writers::{EpubOptions, Format, WriteOptions, write_to_vec};

const OPF: &str = "http://www.idpf.org/2007/opf";
const XHTML: &str = "http://www.w3.org/1999/xhtml";
const OPS: &str = "http://www.idpf.org/2007/ops";

fn epub(doc: &Document, options: &WriteOptions) -> Vec<u8> {
    write_to_vec(doc, Format::Epub, options)
        .expect("EPUB written")
        .0
}

/// Parses every XML part; panics with the part name when one is malformed.
fn parse_all(files: &BTreeMap<String, Vec<u8>>) {
    for (name, data) in files {
        if name.ends_with(".xhtml")
            || name.ends_with(".opf")
            || name.ends_with(".ncx")
            || name.ends_with(".xml")
        {
            let text = std::str::from_utf8(data).expect("UTF-8");
            common::xml(text).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
    }
}

#[test]
fn package_is_well_formed() {
    let bytes = epub(&sample(), &options());
    // The mimetype comes first, stored, with no extra field (OCF 3.3).
    assert_eq!(&bytes[..4], b"PK\x03\x04");
    let method = u16::from_le_bytes([bytes[8], bytes[9]]);
    let name_len = u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
    let extra_len = u16::from_le_bytes([bytes[28], bytes[29]]);
    assert_eq!(method, 0, "mimetype must be stored");
    assert_eq!(extra_len, 0, "mimetype must have no extra field");
    assert_eq!(&bytes[30..30 + name_len], b"mimetype");
    assert_eq!(
        &bytes[30 + name_len..30 + name_len + 20],
        b"application/epub+zip"
    );

    let listed = unzip(&bytes);
    assert_eq!(listed[0].0, "mimetype");
    let files = entries(&bytes);
    parse_all(&files);

    let container = std::str::from_utf8(&files["META-INF/container.xml"]).unwrap();
    assert!(container.contains("full-path=\"OEBPS/content.opf\""));

    let opf_text = std::str::from_utf8(&files["OEBPS/content.opf"]).unwrap();
    let opf = common::xml(opf_text).unwrap();
    let items: Vec<roxmltree::Node<'_, '_>> = opf
        .descendants()
        .filter(|n| n.has_tag_name((OPF, "item")))
        .collect();
    let mut ids = BTreeSet::new();
    let mut hrefs = BTreeSet::new();
    for item in &items {
        let href = attr(*item, "href").unwrap();
        assert!(ids.insert(attr(*item, "id").unwrap()), "duplicate id");
        assert!(
            files.contains_key(&format!("OEBPS/{href}")),
            "missing {href}"
        );
        hrefs.insert(format!("OEBPS/{href}"));
    }
    // Every file in the package is in the manifest.
    for name in files.keys() {
        if name != "mimetype" && !name.starts_with("META-INF/") && name != "OEBPS/content.opf" {
            assert!(hrefs.contains(name), "{name} not in the manifest");
        }
    }
    let nav = items
        .iter()
        .filter(|i| attr(**i, "properties") == Some("nav"))
        .count();
    assert_eq!(nav, 1);
    for itemref in opf
        .descendants()
        .filter(|n| n.has_tag_name((OPF, "itemref")))
    {
        assert!(ids.contains(attr(itemref, "idref").unwrap()));
    }
    let png = items
        .iter()
        .find(|i| attr(**i, "media-type") == Some("image/png"))
        .expect("the image is in the package");
    let png_href = format!("OEBPS/{}", attr(*png, "href").unwrap());
    assert_eq!(
        files[&png_href],
        std::fs::read(fixture("pixel.png")).unwrap()
    );
}

#[test]
fn metadata_declares_accessibility() {
    let files = entries(&epub(&sample(), &options()));
    let opf = std::str::from_utf8(&files["OEBPS/content.opf"]).unwrap();
    for needle in [
        "<dc:title>Reading Guide</dc:title>",
        "<dc:language>en-US</dc:language>",
        "<dc:creator>Ada Example</dc:creator>",
        "<meta property=\"dcterms:modified\">2026-09-25T12:34:56Z</meta>",
        "<meta property=\"schema:accessMode\">textual</meta>",
        "<meta property=\"schema:accessMode\">visual</meta>",
        "<meta property=\"schema:accessModeSufficient\">textual</meta>",
        "<meta property=\"schema:accessibilityFeature\">structuralNavigation</meta>",
        "<meta property=\"schema:accessibilityFeature\">tableOfContents</meta>",
        "<meta property=\"schema:accessibilityFeature\">alternativeText</meta>",
        "<meta property=\"schema:accessibilityHazard\">none</meta>",
        "<meta property=\"schema:accessibilitySummary\">",
    ] {
        assert!(opf.contains(needle), "missing {needle}\n{opf}");
    }
    // Same document, same identifier.
    let again = entries(&epub(&sample(), &options()));
    assert_eq!(files["OEBPS/content.opf"], again["OEBPS/content.opf"]);
    let id = opf.split("<dc:identifier id=\"book-id\">").nth(1).unwrap();
    assert!(id.starts_with("urn:uuid:"));
    assert_eq!(id.split('<').next().unwrap().len(), "urn:uuid:".len() + 36);
}

#[test]
fn content_is_semantic_and_links_resolve() {
    let files = entries(&epub(&sample(), &options()));
    let mut ids: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut links: Vec<(String, String)> = Vec::new();
    let mut tags: BTreeSet<String> = BTreeSet::new();
    for (name, data) in &files {
        if !name.ends_with(".xhtml") {
            continue;
        }
        let text = std::str::from_utf8(data).unwrap();
        let doc = common::xml(text).unwrap();
        let root = doc.root_element();
        assert_eq!(attr(root, "lang"), Some("en-US"), "{name}");
        for n in doc.descendants().filter(|n| n.is_element()) {
            assert_eq!(n.tag_name().namespace(), Some(XHTML));
            tags.insert(n.tag_name().name().to_owned());
            if let Some(id) = attr(n, "id") {
                assert!(
                    ids.entry(name.clone()).or_default().insert(id.to_owned()),
                    "duplicate id {id}"
                );
            }
            if n.has_tag_name((XHTML, "a")) {
                let href = attr(n, "href").unwrap();
                if !href.contains("://") && !href.starts_with("mailto:") {
                    links.push((name.clone(), href.to_owned()));
                }
            }
            if n.has_tag_name((XHTML, "img")) {
                assert_eq!(attr(n, "alt"), Some("Two coloured squares"));
                assert!(files.contains_key(&format!("OEBPS/{}", attr(n, "src").unwrap())));
            }
            if n.has_tag_name((XHTML, "th")) {
                assert_eq!(attr(n, "scope"), Some("col"));
            }
        }
    }
    for t in [
        "h1",
        "h2",
        "h3",
        "ol",
        "ul",
        "li",
        "table",
        "thead",
        "th",
        "td",
        "figure",
        "img",
        "pre",
        "code",
        "blockquote",
        "strong",
        "em",
        "aside",
        "nav",
    ] {
        assert!(tags.contains(t), "no <{t}>");
    }
    // Internal links resolve to a file and an id in it.
    assert!(!links.is_empty());
    for (from, href) in links {
        let (file, frag) = href.split_once('#').unwrap_or((&href, ""));
        let file = if file.is_empty() {
            from.clone()
        } else {
            format!("OEBPS/{file}")
        };
        assert!(files.contains_key(&file), "{from} links to missing {href}");
        if !frag.is_empty() {
            assert!(
                ids.get(&file).is_some_and(|s| s.contains(frag)),
                "{from}: {href}"
            );
        }
    }
    // Footnote reference and body.
    let all: String = files
        .iter()
        .filter(|(n, _)| n.ends_with(".xhtml"))
        .map(|(_, d)| String::from_utf8_lossy(d).into_owned())
        .collect();
    assert!(all.contains("epub:type=\"noteref\" role=\"doc-noteref\" href=\"#fn-1\">[1]</a>"));
    assert!(all.contains("<aside epub:type=\"footnote\" role=\"doc-footnote\" id=\"fn-1\">"));
    assert!(all.contains("<ol start=\"1\">") || all.contains("<ol>"));
    assert!(all.contains("&lt;, &gt;, &amp; and \"quotes\""));
    assert!(all.contains("<a href=\"https://example.org/textweaver\">project page</a>"));
    assert!(all.contains("<pre><code class=\"language-rust\">"));
}

#[test]
fn nav_follows_the_heading_outline() {
    let files = entries(&epub(&sample(), &options()));
    let nav_text = std::str::from_utf8(&files["OEBPS/nav.xhtml"]).unwrap();
    let nav = common::xml(nav_text).unwrap();
    let toc = nav
        .descendants()
        .find(|n| n.has_tag_name((XHTML, "nav")) && n.attribute((OPS, "type")) == Some("toc"))
        .expect("a toc nav");
    // Reading Guide > (Getting started > Shortcuts), Notes, Footnotes.
    let top = toc
        .children()
        .find(|n| n.has_tag_name((XHTML, "ol")))
        .unwrap();
    let top_items: Vec<_> = top
        .children()
        .filter(|n| n.has_tag_name((XHTML, "li")))
        .collect();
    assert_eq!(top_items.len(), 1);
    let label = |li: roxmltree::Node<'_, '_>| {
        li.children()
            .find(|n| n.has_tag_name((XHTML, "a")))
            .and_then(|a| a.text())
            .unwrap_or_default()
            .to_owned()
    };
    assert_eq!(label(top_items[0]), "Reading Guide");
    let second: Vec<_> = top_items[0]
        .children()
        .find(|n| n.has_tag_name((XHTML, "ol")))
        .unwrap()
        .children()
        .filter(|n| n.has_tag_name((XHTML, "li")))
        .collect();
    let names: Vec<String> = second.iter().map(|li| label(*li)).collect();
    assert_eq!(names, ["Getting started", "Notes", "Footnotes"]);
    let third = second[0]
        .children()
        .find(|n| n.has_tag_name((XHTML, "ol")))
        .unwrap();
    assert_eq!(label(third.first_element_child().unwrap()), "Shortcuts");
    let ncx = std::str::from_utf8(&files["OEBPS/toc.ncx"]).unwrap();
    assert!(ncx.contains("<text>Shortcuts</text>"));
    assert!(ncx.contains("<meta name=\"dtb:depth\" content=\"3\"/>"));
}

#[test]
fn round_trips_through_the_html_loader() {
    let doc = sample();
    let files = entries(&epub(&doc, &options()));
    let opf = std::str::from_utf8(&files["OEBPS/content.opf"]).unwrap();
    let mut got = Vec::new();
    for part in opf.split("<itemref idref=\"").skip(1) {
        // The navigation document is in the spine, outside the reading order.
        if part.split('>').next().unwrap().contains("linear=\"no\"") {
            continue;
        }
        let id = part.split('"').next().unwrap();
        let href = opf
            .split(&format!("<item id=\"{id}\" href=\""))
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        let loaded = HtmlLoader
            .load(
                &Source::Bytes {
                    data: files[&format!("OEBPS/{href}")].clone(),
                    hint: "xhtml".into(),
                },
                &LoadOptions::default(),
            )
            .unwrap();
        got.extend(words(&loaded.text().to_string()));
    }
    // The HTML loader skips `aside` (Star's skip list), so footnote bodies
    // are compared separately.
    let expected = words(&text_without(&doc, is_footnote_body));
    assert_eq!(got, expected);
}

#[test]
fn splits_chapters_at_section_breaks_with_page_list() {
    let text = "One\n\nFirst chapter text.\n\nTwo\n\nSecond chapter text.";
    let markers = vec![
        Marker::new(MarkerKind::SectionBreak, CharRange::new(0, 24)).with_label("Part one"),
        Marker::new(MarkerKind::Heading, CharRange::new(0, 3)).with_level(1),
        Marker::new(MarkerKind::Paragraph, CharRange::new(5, 24)),
        Marker::new(MarkerKind::SectionBreak, CharRange::new(26, 52)),
        Marker::new(MarkerKind::PageBreak, CharRange::empty(26)).with_label("7"),
        Marker::new(MarkerKind::Heading, CharRange::new(26, 29)).with_level(1),
        Marker::new(MarkerKind::Paragraph, CharRange::new(31, 52)),
    ];
    let doc = Document::new(DocumentMeta::default(), Rope::from_str(text), markers);
    let files = entries(&epub(&doc, &options()));
    parse_all(&files);
    let ch1 = std::str::from_utf8(&files["OEBPS/chapter-1.xhtml"]).unwrap();
    let ch2 = std::str::from_utf8(&files["OEBPS/chapter-2.xhtml"]).unwrap();
    assert!(ch1.contains("<title>One</title>"));
    assert!(
        ch1.contains(
            "<section epub:type=\"chapter\" role=\"doc-chapter\" aria-labelledby=\"h-1\">"
        )
    );
    assert!(ch2.contains("id=\"page-1\" aria-label=\"7\""));
    assert!(ch2.contains("<h1 id=\"h-2\">Two</h1>"));
    let nav = std::str::from_utf8(&files["OEBPS/nav.xhtml"]).unwrap();
    assert!(nav.contains("epub:type=\"page-list\""));
    assert!(nav.contains("<a href=\"chapter-2.xhtml#page-1\">7</a>"));
    let opf = std::str::from_utf8(&files["OEBPS/content.opf"]).unwrap();
    assert!(opf.contains("printPageNumbers"));
    // Untitled documents are named from their first heading.
    assert!(opf.contains("<dc:title>One</dc:title>"));

    // One content document when splitting is off.
    let single = entries(&epub(
        &doc,
        &WriteOptions {
            epub: EpubOptions {
                split_chapters: false,
                ..EpubOptions::default()
            },
            ..options()
        },
    ));
    assert!(single.contains_key("OEBPS/chapter-1.xhtml"));
    assert!(!single.contains_key("OEBPS/chapter-2.xhtml"));
}

#[test]
fn plain_text_and_empty_documents_are_valid() {
    for text in [
        "",
        "Just a line.\nAnd another.\n\nA <second> & last paragraph.",
    ] {
        let doc = Document::from_plain_text(text);
        let files = entries(&epub(&doc, &options()));
        parse_all(&files);
        let opf = std::str::from_utf8(&files["OEBPS/content.opf"]).unwrap();
        assert!(opf.contains("<dc:title>Untitled document</dc:title>"));
        assert!(opf.contains("<dc:language>en</dc:language>"));
        let nav = std::str::from_utf8(&files["OEBPS/nav.xhtml"]).unwrap();
        assert!(nav.contains("<li><a href=\"chapter-1.xhtml\">Untitled document</a></li>"));
    }
}

#[test]
fn missing_images_are_described_and_reported() {
    let doc = Document::new(
        DocumentMeta::default(),
        Rope::from_str("A chart of sales"),
        vec![
            Marker::new(MarkerKind::Paragraph, CharRange::new(0, 16)),
            Marker::new(MarkerKind::Image, CharRange::new(0, 16))
                .with_reference("nowhere/chart.png"),
        ],
    );
    let (bytes, report) = write_to_vec(&doc, Format::Epub, &options()).unwrap();
    let files = entries(&bytes);
    let ch = std::str::from_utf8(&files["OEBPS/chapter-1.xhtml"]).unwrap();
    assert!(
        ch.contains("<span role=\"img\" aria-label=\"A chart of sales\">A chart of sales</span>")
    );
    assert_eq!(report.warnings.len(), 1);
    assert!(report.warnings[0].contains("chart.png was not found"));
}

/// epubcheck, when it is installed (`epubcheck` on the PATH, or Java with
/// `EPUBCHECK_JAR`); skipped otherwise.
#[test]
fn passes_epubcheck_when_available() {
    let jar = std::env::var_os("EPUBCHECK_JAR");
    let mut cmd = if common::runs("epubcheck", "--version") {
        std::process::Command::new("epubcheck")
    } else if let Some(jar) = jar.filter(|_| common::runs("java", "-version")) {
        let mut c = std::process::Command::new("java");
        c.arg("-jar").arg(jar);
        c
    } else {
        common::skip_or_fail(
            "epubcheck",
            "epubcheck is not installed (set EPUBCHECK_JAR or put epubcheck on the PATH)",
        );
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sample.epub");
    std::fs::write(&path, epub(&sample(), &options())).unwrap();
    let out = cmd.arg(&path).output().unwrap();
    assert!(
        out.status.success(),
        "epubcheck failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// epubcheck: a landmark link must point into the spine (RSC-011), and the
/// navigation document's nav takes no naming attribute (RSC-005).
#[test]
fn the_navigation_document_passes_epubcheck_rules() {
    let doc = sample();
    let files = entries(&epub(&doc, &options()));
    let opf = std::str::from_utf8(&files["OEBPS/content.opf"]).unwrap();
    assert!(
        opf.contains("<itemref idref=\"nav\" linear=\"no\"/>"),
        "the nav is in the spine, outside the reading order: {opf}"
    );
    let nav = std::str::from_utf8(&files["OEBPS/nav.xhtml"]).unwrap();
    let toc = nav.split("<nav").nth(1).unwrap().split('>').next().unwrap();
    assert!(!toc.contains("aria-label"), "{toc}");
    assert!(nav.contains("<h1 id=\"toc-title\">Contents</h1>"), "{nav}");
}
