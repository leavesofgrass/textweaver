//! Publishing templates (ADR-0041): real Word footnotes, the APA and AMA
//! Word templates, the reading templates' styles, the EPUB cover with its
//! alternative text, and PDF page labels from print page breaks. Each
//! output is read back: Word files with textweaver's own DOCX loader, PDFs
//! with its PDF loader.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use common::{attr, entries, options, words, xml};
use ropey::Rope;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_formats::{DocxLoader, LoadOptions, Loader, MarkdownLoader, PdfLoader, Source};
use textweaver_text::{Document, DocumentMeta, Marker};
use textweaver_writers::{
    DocxOptions, Format, PdfOptions, Template, WriteError, WriteOptions, write_to_vec,
};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// `fixtures/g2/apa-paper.md`, loaded with the Markdown loader.
fn paper() -> Document {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/g2/apa-paper.md");
    MarkdownLoader
        .load(&Source::Path(path), &LoadOptions::default())
        .expect("apa-paper.md loads")
}

fn templated(t: Template) -> WriteOptions {
    options().with_template(t)
}

fn docx(doc: &Document, o: &WriteOptions) -> (Vec<u8>, BTreeMap<String, Vec<u8>>) {
    let bytes = write_to_vec(doc, Format::Docx, o).expect("DOCX written").0;
    let files = entries(&bytes);
    (bytes, files)
}

fn part<'a>(files: &'a BTreeMap<String, Vec<u8>>, name: &str) -> &'a str {
    std::str::from_utf8(&files[name]).unwrap_or_else(|_| panic!("{name} is UTF-8"))
}

/// Every paragraph of a WordprocessingML part: (style, text).
fn paragraphs(text: &str) -> Vec<(Option<String>, String)> {
    let doc = xml(text).expect("the part parses");
    doc.descendants()
        .filter(|n| n.has_tag_name((W, "p")))
        .map(|p| {
            let style = p
                .descendants()
                .find(|d| d.has_tag_name((W, "pStyle")))
                .and_then(|d| d.attribute((W, "val")))
                .map(str::to_owned);
            let text: String = p
                .descendants()
                .filter(|d| d.has_tag_name((W, "t")))
                .filter_map(|d| d.text())
                .collect();
            (style, text)
        })
        .collect()
}

/// The package is consistent: every part parses, has a content type, and
/// every relationship target exists; every style used is defined; every
/// relationship id a part uses is declared by that part's relationships.
fn check_package(files: &BTreeMap<String, Vec<u8>>) {
    for (name, data) in files {
        if name.ends_with(".xml") || name.ends_with(".rels") {
            xml(std::str::from_utf8(data).unwrap()).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
    }
    let types = part(files, "[Content_Types].xml");
    for name in files.keys() {
        let ext = name.rsplit('.').next().unwrap();
        assert!(
            types.contains(&format!("PartName=\"/{name}\""))
                || types.contains(&format!("Extension=\"{ext}\"")),
            "{name} has no content type"
        );
    }
    for (source, rels) in [
        ("word/document.xml", "word/_rels/document.xml.rels"),
        ("word/footnotes.xml", "word/_rels/footnotes.xml.rels"),
    ] {
        let Some(text) = files.get(source) else {
            continue;
        };
        let text = std::str::from_utf8(text).unwrap();
        let declared = files
            .get(rels)
            .map(|r| std::str::from_utf8(r).unwrap().to_owned())
            .unwrap_or_default();
        if !declared.is_empty() {
            let doc = xml(&declared).unwrap();
            for r in doc
                .descendants()
                .filter(|n| n.tag_name().name() == "Relationship")
            {
                if attr(r, "TargetMode") == Some("External") {
                    continue;
                }
                let target = format!("word/{}", attr(r, "Target").unwrap());
                assert!(files.contains_key(&target), "{rels}: missing {target}");
            }
        }
        for id in text
            .split("r:id=\"")
            .skip(1)
            .chain(text.split("r:embed=\"").skip(1))
        {
            let id = id.split('"').next().unwrap();
            assert!(
                declared.contains(&format!("Id=\"{id}\"")),
                "{source}: {id} is not declared"
            );
        }
    }
    let styles = part(files, "word/styles.xml");
    let defined: BTreeSet<&str> = styles
        .split("w:styleId=\"")
        .skip(1)
        .map(|s| s.split('"').next().unwrap())
        .collect();
    for name in [
        "word/document.xml",
        "word/footnotes.xml",
        "word/header1.xml",
    ] {
        let Some(text) = files.get(name) else {
            continue;
        };
        let text = std::str::from_utf8(text).unwrap();
        for key in [
            "w:pStyle w:val=\"",
            "w:rStyle w:val=\"",
            "w:tblStyle w:val=\"",
        ] {
            for used in text.split(key).skip(1) {
                let used = used.split('"').next().unwrap();
                assert!(
                    defined.contains(used),
                    "{name}: style {used} is not defined"
                );
            }
        }
    }
}

/// Loads a DOCX with textweaver's own loader.
fn load_docx(bytes: Vec<u8>) -> Document {
    DocxLoader
        .load(
            &Source::Bytes {
                data: bytes,
                hint: "docx".to_owned(),
            },
            &LoadOptions::default(),
        )
        .expect("the DOCX loads")
}

#[test]
fn footnotes_are_real_word_footnotes() {
    let doc = common::sample();
    let (bytes, files) = docx(&doc, &options());
    check_package(&files);
    let document = part(&files, "word/document.xml");
    let notes = part(&files, "word/footnotes.xml");
    assert!(document.contains("<w:footnoteReference w:id=\"1\"/>"));
    assert!(document.contains("<w:rStyle w:val=\"FootnoteReference\"/>"));
    // The body moved out of the running text into the footnote.
    assert!(
        !paragraphs(document)
            .iter()
            .any(|(_, t)| t.contains("lists every release"))
    );
    assert!(notes.contains("w:type=\"separator\" w:id=\"-1\""));
    assert!(notes.contains("w:type=\"continuationSeparator\" w:id=\"0\""));
    assert!(notes.contains("<w:footnoteRef/>"));
    let texts = paragraphs(notes);
    assert!(
        texts
            .iter()
            .any(|(style, t)| style.as_deref() == Some("FootnoteText")
                && t.contains("The project page lists every release."))
    );
    assert!(part(&files, "word/_rels/document.xml.rels").contains("Target=\"footnotes.xml\""));
    assert!(part(&files, "word/settings.xml").contains("<w:footnotePr>"));

    // Read back by textweaver: every word, the footnote included.
    let back = load_docx(bytes);
    let text = back.text().to_string();
    assert!(
        text.contains("The project page lists every release."),
        "{text}"
    );
    assert!(
        back.markers()
            .iter()
            .any(|m| m.kind == MarkerKind::Footnote)
    );
    for w in words(&doc.text().to_string()) {
        assert!(
            words(&text).contains(&w),
            "the word {w:?} was lost in the round trip"
        );
    }
}

#[test]
fn footnotes_can_stay_in_place() {
    let o = WriteOptions {
        docx: DocxOptions {
            word_footnotes: false,
        },
        ..options()
    };
    let (_, files) = docx(&common::sample(), &o);
    assert!(!files.contains_key("word/footnotes.xml"));
    let document = part(&files, "word/document.xml");
    assert!(document.contains("<w:hyperlink w:anchor=\"fn_1\""));
    assert!(document.contains("w:name=\"fn_1\""));
}

/// Text with two references to one footnote, a second footnote, and a
/// footnote nothing references.
fn notes_doc() -> Document {
    let text = "One[1] two[1] three[2].\n[1] First note.\n[2] Second note.\n[3] Orphan note.";
    let find = |s: &str, from: usize| text[from..].find(s).unwrap() + from;
    let c = |b: usize| text[..b].chars().count();
    let r1 = find("[1]", 0);
    let r2 = find("[1]", r1 + 1);
    let r3 = find("[2]", 0);
    let n1 = find("[1] First", 0);
    let n2 = find("[2] Second", 0);
    let n3 = find("[3] Orphan", 0);
    let m = |kind, a: usize, b: usize, level: u8, id: &str| {
        Marker::new(kind, CharRange::new(c(a), c(b)))
            .with_level(level)
            .with_reference(id)
    };
    let markers = vec![
        Marker::new(MarkerKind::Paragraph, CharRange::new(0, 23)),
        m(MarkerKind::Footnote, r1, r1 + 3, 0, "1"),
        m(MarkerKind::Footnote, r2, r2 + 3, 0, "1"),
        m(MarkerKind::Footnote, r3, r3 + 3, 0, "2"),
        m(MarkerKind::Footnote, n1, n1 + 15, 1, "1"),
        m(MarkerKind::Footnote, n2, n2 + 16, 1, "2"),
        m(MarkerKind::Footnote, n3, text.len(), 1, "3"),
    ];
    Document::new(DocumentMeta::default(), Rope::from_str(text), markers)
}

#[test]
fn a_second_reference_is_a_cross_reference_and_orphans_stay() {
    let (_, files) = docx(&notes_doc(), &options());
    check_package(&files);
    let document = part(&files, "word/document.xml");
    let notes = part(&files, "word/footnotes.xml");
    // Two Word footnotes, numbered in order of first reference.
    assert_eq!(document.matches("<w:footnoteReference ").count(), 2);
    assert!(notes.contains("<w:footnote w:id=\"1\">") && notes.contains("<w:footnote w:id=\"2\">"));
    assert!(!notes.contains("<w:footnote w:id=\"3\">"));
    // The second reference to note 1 points at the first.
    assert!(document.contains("w:name=\"_RefNote1\""));
    assert!(document.contains(" NOTEREF _RefNote1 \\f \\h "));
    // The orphan note stays in the text.
    assert!(
        paragraphs(document)
            .iter()
            .any(|(_, t)| t.contains("Orphan note"))
    );
    assert!(
        !paragraphs(document)
            .iter()
            .any(|(_, t)| t.contains("First note"))
    );
}

#[test]
fn apa_student_paper_has_its_parts() {
    let doc = paper();
    let o = templated(Template::ApaStudentPaper);
    let (bytes, files) = docx(&doc, &o);
    check_package(&files);
    let document = part(&files, "word/document.xml");
    let paras = paragraphs(document);
    // The title page: title, blank line, author, affiliation, course,
    // instructor, date; then the text starts on a new page.
    let texts: Vec<&str> = paras.iter().map(|(_, t)| t.as_str()).take(7).collect();
    assert_eq!(
        texts,
        [
            "Listening as a Study Skill",
            "",
            "Ada Example",
            "Department of Education, Example State University",
            "EDU 501, Learning and Assistive Technology",
            "Dr. Grace Placeholder",
            "October 5, 2026",
        ]
    );
    assert_eq!(paras[0].0.as_deref(), Some("Title"));
    assert_eq!(document.matches("<w:pageBreakBefore/>").count(), 1);
    // The repeated title and the sections are real headings.
    assert!(
        paras
            .iter()
            .any(|(s, t)| s.as_deref() == Some("Heading1") && t == "Listening as a Study Skill")
    );
    assert!(
        paras
            .iter()
            .any(|(s, t)| s.as_deref() == Some("Heading2") && t == "Method")
    );
    assert!(
        paras
            .iter()
            .any(|(s, t)| s.as_deref() == Some("Heading3") && t == "Materials")
    );
    // Body text indented; references with hanging indents.
    assert!(
        paras
            .iter()
            .any(|(s, t)| s.as_deref() == Some("BodyText") && t.starts_with("Students who read"))
    );
    let refs: Vec<_> = paras
        .iter()
        .filter(|(s, _)| s.as_deref() == Some("Bibliography"))
        .collect();
    assert_eq!(refs.len(), 2, "{refs:?}");
    // Styles: Times New Roman 12, double spacing, APA heading looks
    // (the paper's sections are level 2, so level 2 is centered bold).
    let styles = part(&files, "word/styles.xml");
    assert!(styles.contains("w:ascii=\"Times New Roman\""));
    assert!(styles.contains("w:line=\"480\""));
    let heading2 = styles
        .split("w:styleId=\"Heading2\"")
        .nth(1)
        .unwrap()
        .split("</w:style>")
        .next()
        .unwrap();
    assert!(heading2.contains("<w:jc w:val=\"center\"/>"));
    assert!(heading2.contains("<w:outlineLvl w:val=\"1\"/>"));
    let heading3 = styles
        .split("w:styleId=\"Heading3\"")
        .nth(1)
        .unwrap()
        .split("</w:style>")
        .next()
        .unwrap();
    assert!(!heading3.contains("<w:jc ") && !heading3.contains("<w:i/>"));
    assert!(styles.contains("<w:ind w:left=\"720\" w:hanging=\"720\"/>"));
    // The page number at the top right.
    let header = part(&files, "word/header1.xml");
    assert!(header.contains(" PAGE ") && header.contains("<w:jc w:val=\"right\"/>"));
    assert!(document.contains("<w:headerReference w:type=\"default\" r:id=\"rIdHeader1\"/>"));
    // Two footnotes, one referenced twice.
    let notes = part(&files, "word/footnotes.xml");
    assert_eq!(notes.matches("<w:footnoteRef/>").count(), 2);
    assert!(document.contains(" NOTEREF _RefNote1 "));

    // Read back by textweaver: headings in order, footnotes kept.
    let back = load_docx(bytes);
    let headings: Vec<String> = back
        .markers()
        .iter()
        .filter(|m| m.kind == MarkerKind::Heading)
        .map(|m| back.slice(m.range).trim().to_owned())
        // The loader's own heading over the footnotes it gathers.
        .filter(|h| h != "Footnotes")
        .collect();
    // The title page's title (Word's Title style, which the loader reads
    // as a heading), then the paper's own headings.
    assert_eq!(
        headings,
        [
            "Listening as a Study Skill",
            "Listening as a Study Skill",
            "Method",
            "Materials",
            "Procedure",
            "Results",
            "Discussion",
            "References"
        ]
    );
    let text = back.text().to_string();
    assert!(text.contains("The log form is available from the author."));
    assert!(text.contains("Ada Example"));
}

#[test]
fn ama_manuscript_counts_words_on_its_title_page() {
    let doc = paper();
    let (_, files) = docx(&doc, &templated(Template::AmaManuscript));
    check_package(&files);
    let paras = paragraphs(part(&files, "word/document.xml"));
    let count = paras
        .iter()
        .find_map(|(_, t)| t.strip_prefix("Word count: "))
        .expect("a word count");
    let n: usize = count.parse().unwrap();
    // The running text only: not the headings, table, references, or
    // footnotes: the fixture's paragraphs and list, about 135 words.
    assert!((110..160).contains(&n), "{n}");
    assert!(part(&files, "word/header1.xml").contains(" PAGE "));
}

#[test]
fn every_template_keeps_the_structure() {
    let doc = paper();
    for t in Template::ALL {
        let (bytes, files) = docx(&doc, &templated(t));
        check_package(&files);
        let styles = part(&files, "word/styles.xml");
        for level in 0..6 {
            assert!(
                styles.contains(&format!("<w:outlineLvl w:val=\"{level}\"/>")),
                "{t}: heading level {}",
                level + 1
            );
        }
        let back = load_docx(bytes);
        assert!(
            back.markers()
                .iter()
                .filter(|m| m.kind == MarkerKind::Heading)
                .count()
                >= 7,
            "{t}"
        );
        match t {
            Template::LargePrint => {
                assert!(styles.contains("<w:sz w:val=\"36\"/>"));
                assert!(styles.contains("w:ascii=\"Verdana\""));
            }
            Template::DyslexiaFriendly => {
                assert!(styles.contains("<w:spacing w:val=\"12\"/>"));
                let d = part(&files, "word/document.xml");
                assert!(d.contains("<w:background w:color=\"FBF8EE\"/>"));
                // Emphasis is bold, never italic.
                assert!(!d.contains("<w:i/>"));
            }
            Template::HighContrast => {
                assert!(styles.contains("<w:color w:val=\"000000\"/>"));
            }
            Template::Manuscript => {
                let header = part(&files, "word/header1.xml");
                assert!(header.contains("Example / Listening as a Study / "));
                let paras = paragraphs(part(&files, "word/document.xml"));
                assert!(paras.iter().any(|(_, t)| t == "by Ada Example"));
                assert!(
                    paras
                        .iter()
                        .any(|(_, t)| t.starts_with("About ") && t.ends_with(" words"))
                );
            }
            _ => {}
        }
    }
}

#[test]
fn templated_epub_has_a_cover_and_the_stylesheet() {
    let doc = paper();
    for t in Template::ALL {
        let bytes = write_to_vec(&doc, Format::Epub, &templated(t))
            .expect("EPUB written")
            .0;
        let files = entries(&bytes);
        let css = std::str::from_utf8(&files["OEBPS/style.css"]).unwrap();
        assert!(css.contains(t.stylesheet()), "{t}");
        let cover = std::str::from_utf8(&files["OEBPS/cover.xhtml"]).unwrap();
        let cover_doc = xml(cover).unwrap();
        let img = cover_doc
            .descendants()
            .find(|n| n.tag_name().name() == "img")
            .unwrap();
        assert_eq!(
            img.attribute("alt"),
            Some("Cover: Listening as a Study Skill, by Ada Example.")
        );
        assert_eq!(img.attribute("role"), Some("doc-cover"));
        let svg = std::str::from_utf8(&files["OEBPS/cover.svg"]).unwrap();
        xml(svg).expect("the cover image parses");
        let opf = std::str::from_utf8(&files["OEBPS/content.opf"]).unwrap();
        assert!(opf.contains(
            "href=\"cover.svg\" media-type=\"image/svg+xml\" properties=\"cover-image\""
        ));
        assert!(opf.contains("<itemref idref=\"cover\"/>"));
        assert!(opf.contains(">alternativeText<"));
        let nav = std::str::from_utf8(&files["OEBPS/nav.xhtml"]).unwrap();
        assert!(nav.contains("<a epub:type=\"cover\" href=\"cover.xhtml\">Cover</a>"));
        // The package stays consistent: every manifest item exists, every
        // file is in the manifest, and every spine entry is an item.
        let opf_doc = xml(opf).unwrap();
        let mut ids = BTreeSet::new();
        let mut hrefs = BTreeSet::new();
        for item in opf_doc
            .descendants()
            .filter(|n| n.tag_name().name() == "item")
        {
            let href = format!("OEBPS/{}", attr(item, "href").unwrap());
            assert!(files.contains_key(&href), "{t}: missing {href}");
            ids.insert(attr(item, "id").unwrap().to_owned());
            hrefs.insert(href);
        }
        for name in files.keys() {
            if name != "mimetype" && !name.starts_with("META-INF/") && name != "OEBPS/content.opf" {
                assert!(hrefs.contains(name), "{t}: {name} is not in the manifest");
            }
        }
        for r in opf_doc
            .descendants()
            .filter(|n| n.tag_name().name() == "itemref")
        {
            assert!(ids.contains(attr(r, "idref").unwrap()), "{t}");
        }
        for (name, data) in &files {
            if name.ends_with(".xhtml") || name.ends_with(".svg") {
                xml(std::str::from_utf8(data).unwrap())
                    .unwrap_or_else(|e| panic!("{t}: {name}: {e}"));
            }
        }
    }
    // Without a template, no cover.
    let files = entries(&write_to_vec(&doc, Format::Epub, &options()).unwrap().0);
    assert!(!files.contains_key("OEBPS/cover.xhtml"));
}

/// Six print pages, labelled 41 to 46, each about one and a half PDF
/// pages long.
fn print_pages_doc() -> Document {
    let mut text = String::new();
    let mut markers = Vec::new();
    let para = "Words on a printed page, read aloud and kept in order. ".repeat(12);
    for page in 41..=46 {
        let at = text.chars().count();
        markers.push(
            Marker::new(MarkerKind::PageBreak, CharRange::new(at, at)).with_label(page.to_string()),
        );
        for _ in 0..6 {
            let start = text.chars().count();
            text.push_str(para.trim_end());
            let end = text.chars().count();
            markers.push(Marker::new(
                MarkerKind::Paragraph,
                CharRange::new(start, end),
            ));
            text.push_str("\n\n");
        }
    }
    let meta = DocumentMeta {
        title: Some("Print pages".to_owned()),
        language: Some("en".to_owned()),
        ..DocumentMeta::default()
    };
    Document::new(meta, Rope::from_str(&text), markers)
}

#[test]
fn pdf_pages_are_labelled_with_print_pages() {
    let o = WriteOptions {
        pdf: PdfOptions {
            title_page: true,
            ..PdfOptions::default()
        },
        ..options()
    };
    let bytes = match write_to_vec(&print_pages_doc(), Format::Pdf, &o) {
        Ok((b, _)) => b,
        Err(WriteError::NoFont) => return,
        Err(e) => panic!("{e}"),
    };
    let back = PdfLoader
        .load(
            &Source::Bytes {
                data: bytes,
                hint: "pdf".to_owned(),
            },
            &LoadOptions::default(),
        )
        .expect("the PDF loads");
    let labels: Vec<String> = back
        .markers()
        .iter()
        .filter(|m| m.kind == MarkerKind::PageBreak)
        .filter_map(|m| m.label.clone())
        .collect();
    // The title page is i; each print page (about one PDF page long here)
    // labels the page it is under way at the top of.
    assert_eq!(labels.first().map(String::as_str), Some("i"), "{labels:?}");
    for page in 41..=46 {
        assert!(labels.contains(&page.to_string()), "{page} in {labels:?}");
    }
    // Labels never go backwards.
    let numbers: Vec<u32> = labels.iter().filter_map(|l| l.parse().ok()).collect();
    assert!(numbers.windows(2).all(|w| w[0] <= w[1]), "{numbers:?}");
    assert!(
        labels.len() > 7,
        "more PDF pages than print pages: {labels:?}"
    );
}

#[test]
fn templated_pdf_validates_with_its_title_page() {
    let doc = paper();
    for t in Template::ALL {
        let o = WriteOptions {
            pdf: PdfOptions {
                compress: false,
                ..templated(t).pdf
            },
            ..templated(t)
        };
        let bytes = match write_to_vec(&doc, Format::Pdf, &o) {
            Ok((b, _)) => b,
            Err(WriteError::NoFont) => return,
            Err(e) => panic!("{t}: {e}"),
        };
        let back = PdfLoader
            .load(
                &Source::Bytes {
                    data: bytes,
                    hint: "pdf".to_owned(),
                },
                &LoadOptions::default(),
            )
            .expect("the PDF loads");
        let text = back.text().to_string();
        match t {
            Template::ApaStudentPaper => {
                assert!(text.contains("Dr. Grace Placeholder"), "{text}");
                assert!(text.contains("EDU 501"));
            }
            Template::AmaManuscript => assert!(text.contains("Word count:")),
            Template::Manuscript => assert!(text.contains("by Ada Example")),
            _ => assert!(!text.contains("EDU 501")),
        }
    }
}

/// Opens the APA paper in Microsoft Word through COM and reads back its
/// footnotes, headings, and header. Run with `TEXTWEAVER_WORD=1 cargo test
/// -p textweaver-writers --test templates -- --ignored` on Windows with
/// Word.
#[test]
#[ignore = "needs Microsoft Word (set TEXTWEAVER_WORD=1)"]
fn apa_paper_opens_in_word() {
    if std::env::var("TEXTWEAVER_WORD").as_deref() != Ok("1") {
        eprintln!("TEXTWEAVER_WORD is not 1; skipping");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("apa-paper.docx");
    let (bytes, _) = docx(&paper(), &templated(Template::ApaStudentPaper));
    std::fs::write(&path, bytes).unwrap();
    let script = format!(
        "$ErrorActionPreference = 'Stop'; $w = New-Object -ComObject Word.Application; $w.Visible = $false; try {{ $d = $w.Documents.Open('{}', $false, $true); $out = @(); foreach ($p in $d.Paragraphs) {{ if ($p.OutlineLevel -lt 10) {{ $out += ('H' + $p.OutlineLevel + '|' + $p.Range.Text.Trim()) }} }}; $out += ('NOTES|' + $d.Footnotes.Count); $out += ('NOTE1|' + $d.Footnotes.Item(1).Range.Text.Trim()); $out += ('PAGES|' + $d.ComputeStatistics(2)); $out += ('HEADER|' + $d.Sections.Item(1).Headers.Item(1).Range.Fields.Count); $d.Close($false); $out -join \"`n\" }} finally {{ $w.Quit() }}",
        path.display()
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .expect("powershell runs");
    let text = String::from_utf8_lossy(&out.stdout);
    eprintln!("{text}\n{}", String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success());
    assert!(text.contains("H1|Listening as a Study Skill"));
    assert!(text.contains("H2|Method"));
    assert!(text.contains("H3|Materials"));
    assert!(text.contains("NOTES|2"), "two Word footnotes");
    assert!(text.contains("NOTE1|The same holds"));
    assert!(text.contains("HEADER|1"), "the page number field");
}
