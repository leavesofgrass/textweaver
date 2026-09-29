//! DOCX writer: package structure, real heading styles, list numbering,
//! table header rows, image alt text, and a structural round trip read
//! back with a small WordprocessingML reader (Agent A2's DOCX loader
//! replaces it at integration). Opening in Word is an ignored test.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{attr, entries, fixture, options, sample, words, xml};
use ropey::Rope;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, DocumentMeta, Marker};
use textweaver_writers::{Format, write_to_vec};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

fn docx(doc: &Document) -> BTreeMap<String, Vec<u8>> {
    entries(
        &write_to_vec(doc, Format::Docx, &options())
            .expect("DOCX written")
            .0,
    )
}

fn part<'a>(files: &'a BTreeMap<String, Vec<u8>>, name: &str) -> &'a str {
    std::str::from_utf8(&files[name]).expect("UTF-8")
}

/// A paragraph or table as read back.
#[derive(Debug)]
enum Read {
    Para {
        style: Option<String>,
        num: Option<(String, String)>,
        text: String,
    },
    Table {
        header_rows: usize,
        rows: Vec<Vec<String>>,
    },
}

fn para_text(p: roxmltree::Node<'_, '_>) -> String {
    let mut s = String::new();
    for n in p.descendants() {
        if n.has_tag_name((W, "t")) {
            s.push_str(n.text().unwrap_or(""));
        } else if n.has_tag_name((W, "tab")) {
            s.push('\t');
        } else if n.has_tag_name((W, "br")) {
            s.push('\n');
        } else if n.has_tag_name((W, "footnoteReference")) {
            // Word shows the footnote's number.
            s.push_str(n.attribute((W, "id")).unwrap_or(""));
        } else if n.has_tag_name((W, "drawing")) {
            let descr = n
                .descendants()
                .find(|d| d.tag_name().name() == "docPr")
                .and_then(|d| d.attribute("descr"))
                .unwrap_or("");
            s.push_str(descr);
        }
    }
    s
}

fn wval<'a>(n: roxmltree::Node<'a, '_>, tag: &str) -> Option<&'a str> {
    n.descendants()
        .find(|d| d.has_tag_name((W, tag)))
        .and_then(|d| d.attribute((W, "val")))
}

/// Reads the body of `word/document.xml`.
fn read_body(document: &str) -> Vec<Read> {
    let doc = xml(document).expect("document.xml parses");
    let body = doc
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .unwrap();
    let mut out = Vec::new();
    for n in body.children().filter(|n| n.is_element()) {
        if n.has_tag_name((W, "p")) {
            let ppr = n.children().find(|c| c.has_tag_name((W, "pPr")));
            let style = ppr.and_then(|p| wval(p, "pStyle")).map(str::to_owned);
            let num = ppr
                .and_then(|p| p.children().find(|c| c.has_tag_name((W, "numPr"))))
                .map(|np| {
                    (
                        wval(np, "numId").unwrap_or("").to_owned(),
                        wval(np, "ilvl").unwrap_or("").to_owned(),
                    )
                });
            out.push(Read::Para {
                style,
                num,
                text: para_text(n),
            });
        } else if n.has_tag_name((W, "tbl")) {
            let mut header_rows = 0;
            let mut rows = Vec::new();
            for tr in n.children().filter(|c| c.has_tag_name((W, "tr"))) {
                if tr.descendants().any(|d| d.has_tag_name((W, "tblHeader"))) {
                    header_rows += 1;
                }
                rows.push(
                    tr.children()
                        .filter(|c| c.has_tag_name((W, "tc")))
                        .map(para_text)
                        .collect(),
                );
            }
            out.push(Read::Table { header_rows, rows });
        }
    }
    out
}

#[test]
fn package_parts_are_well_formed_and_related() {
    let files = docx(&sample());
    for (name, data) in &files {
        if name.ends_with(".xml") || name.ends_with(".rels") {
            xml(std::str::from_utf8(data).unwrap()).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
    }
    let types = part(&files, "[Content_Types].xml");
    for name in files.keys() {
        let ext = name.rsplit('.').next().unwrap();
        let covered = types.contains(&format!("PartName=\"/{name}\""))
            || types.contains(&format!("Extension=\"{ext}\""));
        assert!(covered, "{name} has no content type");
    }
    // Every internal relationship target exists.
    for (rels, base) in [
        ("_rels/.rels", ""),
        ("word/_rels/document.xml.rels", "word/"),
    ] {
        let doc = xml(part(&files, rels)).unwrap();
        for r in doc
            .descendants()
            .filter(|n| n.tag_name().name() == "Relationship")
        {
            if attr(r, "TargetMode") == Some("External") {
                continue;
            }
            let target = format!("{base}{}", attr(r, "Target").unwrap());
            assert!(files.contains_key(&target), "{rels}: missing {target}");
        }
    }
    // Every relationship the document uses is declared.
    let rels = part(&files, "word/_rels/document.xml.rels");
    let document = part(&files, "word/document.xml");
    for id in document
        .split("r:id=\"")
        .chain(document.split("r:embed=\""))
        .skip(1)
    {
        let id = id.split('"').next().unwrap();
        if id.starts_with("rId") {
            assert!(rels.contains(&format!("Id=\"{id}\"")), "{id} undeclared");
        }
    }
    let core = part(&files, "docProps/core.xml");
    assert!(core.contains("<dc:title>Reading Guide</dc:title>"));
    assert!(core.contains("<dc:creator>Ada Example</dc:creator>"));
    assert!(core.contains("<dc:language>en-US</dc:language>"));
    assert!(core.contains("2026-09-25T12:34:56Z"));
    assert!(part(&files, "word/styles.xml").contains("<w:lang w:val=\"en-US\""));
}

#[test]
fn headings_use_builtin_styles_with_outline_levels() {
    let files = docx(&sample());
    let styles = xml(part(&files, "word/styles.xml")).unwrap();
    for n in 1..=6 {
        let style = styles
            .descendants()
            .find(|s| s.attribute((W, "styleId")) == Some(&format!("Heading{n}")))
            .unwrap_or_else(|| panic!("no Heading{n}"));
        assert_eq!(wval(style, "name"), Some(format!("heading {n}").as_str()));
        assert_eq!(
            wval(style, "outlineLvl"),
            Some((n - 1).to_string().as_str())
        );
    }
    let body = read_body(part(&files, "word/document.xml"));
    let headings: Vec<(String, String)> = body
        .iter()
        .filter_map(|r| match r {
            Read::Para {
                style: Some(s),
                text,
                ..
            } if s.starts_with("Heading") => Some((s.clone(), text.clone())),
            _ => None,
        })
        .collect();
    let expect = [
        ("Heading1", "Reading Guide"),
        ("Heading2", "Getting started"),
        ("Heading3", "Shortcuts"),
        ("Heading2", "Notes"),
        // The loader's "Footnotes" heading is left out: its footnote is a
        // Word footnote, so the section would be empty (ADR-0041).
    ];
    assert_eq!(
        headings,
        expect.map(|(a, b)| (a.to_owned(), b.to_owned())).to_vec()
    );
}

#[test]
fn lists_are_numbered_and_nested() {
    let files = docx(&sample());
    let body = read_body(part(&files, "word/document.xml"));
    let items: Vec<(&str, &str, &str)> = body
        .iter()
        .filter_map(|r| match r {
            Read::Para {
                num: Some((id, lvl)),
                text,
                ..
            } => Some((id.as_str(), lvl.as_str(), text.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(
        items,
        [
            ("1", "0", "Open a document."),
            ("1", "0", "Press Space to read."),
            ("2", "1", "Faster with Ctrl+Up."),
            ("2", "1", "Slower with Ctrl+Down."),
            ("1", "0", "Press Escape to stop."),
        ]
    );
    let numbering = xml(part(&files, "word/numbering.xml")).unwrap();
    let abstract_of = |num: &str| {
        numbering
            .descendants()
            .find(|n| n.has_tag_name((W, "num")) && n.attribute((W, "numId")) == Some(num))
            .and_then(|n| wval(n, "abstractNumId"))
            .unwrap()
            .to_owned()
    };
    let format_of = |abs: &str| {
        let a = numbering
            .descendants()
            .find(|n| {
                n.has_tag_name((W, "abstractNum")) && n.attribute((W, "abstractNumId")) == Some(abs)
            })
            .unwrap();
        wval(a, "numFmt").unwrap().to_owned()
    };
    assert_eq!(format_of(&abstract_of("1")), "decimal");
    assert_eq!(format_of(&abstract_of("2")), "bullet");
    // All abstract numbering definitions precede the instances (schema order).
    let text = part(&files, "word/numbering.xml");
    assert!(text.rfind("<w:abstractNum ").unwrap() < text.find("<w:num ").unwrap());
}

#[test]
fn tables_have_header_rows_and_images_have_alt_text() {
    let files = docx(&sample());
    let body = read_body(part(&files, "word/document.xml"));
    let table = body
        .iter()
        .find_map(|r| match r {
            Read::Table { header_rows, rows } => Some((*header_rows, rows.clone())),
            _ => None,
        })
        .unwrap();
    assert_eq!(table.0, 1);
    assert_eq!(
        table.1,
        vec![
            vec!["Key", "Action"],
            vec!["Space", "Read or pause"],
            vec!["Escape", "Stop"]
        ]
    );
    let document = part(&files, "word/document.xml");
    assert!(
        document.contains("<wp:docPr id=\"1\" name=\"Picture 1\" descr=\"Two coloured squares\"/>")
    );
    // 2 by 1 pixels at 96 dpi.
    assert!(document.contains("<wp:extent cx=\"19050\" cy=\"9525\"/>"));
    assert_eq!(
        files["word/media/image1.png"],
        std::fs::read(fixture("pixel.png")).unwrap()
    );
    let rels = part(&files, "word/_rels/document.xml.rels");
    assert!(rels.contains("Target=\"https://example.org/textweaver\" TargetMode=\"External\""));
    // The footnote is a real Word footnote (ADR-0041).
    assert!(document.contains("<w:footnoteReference w:id=\"1\"/>"));
    assert!(part(&files, "word/footnotes.xml").contains("lists every release"));
    assert!(document.contains("<w:pStyle w:val=\"Quote\"/>"));
    assert!(document.contains("<w:pStyle w:val=\"SourceCode\"/>"));
}

#[test]
fn round_trip_keeps_every_word_in_order() {
    let doc = sample();
    let files = docx(&doc);
    let mut body = read_body(part(&files, "word/document.xml"));
    // Footnote bodies follow the text, as in the source.
    let notes = xml(part(&files, "word/footnotes.xml")).unwrap();
    for p in notes.descendants().filter(|n| n.has_tag_name((W, "p"))) {
        // Word shows the footnote's number before its text.
        let number = p
            .ancestors()
            .find(|a| a.has_tag_name((W, "footnote")))
            .and_then(|f| f.attribute((W, "id")))
            .unwrap_or("");
        // The separators (ids -1 and 0) have no text.
        if number.parse::<i32>().is_ok_and(|n| n <= 0) {
            continue;
        }
        let text = format!("[{number}] {}", para_text(p));
        if !text.trim().is_empty() {
            body.push(Read::Para {
                style: None,
                num: None,
                text,
            });
        }
    }
    let mut text = String::new();
    for r in &body {
        match r {
            Read::Para { text: t, .. } => {
                text.push_str(t);
                text.push('\n');
            }
            Read::Table { rows, .. } => {
                for row in rows {
                    text.push_str(&row.join(" | "));
                    text.push('\n');
                }
            }
        }
    }
    // The source's "Footnotes" heading heads only a Word footnote now, so
    // it is left out; every other word is kept, in order.
    let mut expected = words(&doc.text().to_string());
    let at = expected.iter().rposition(|w| w == "footnotes").unwrap();
    expected.remove(at);
    assert_eq!(words(&text), expected);
    // The code block keeps its indentation and lines.
    assert!(text.contains("fn main() {\n    println!(\"Hello, café!\");\n}"));
}

#[test]
fn captions_breaks_and_plain_text() {
    let text = "Prices\nItem | Cost\nTea | 2\n\nNext chapter";
    let markers = vec![
        Marker::new(MarkerKind::Table, CharRange::new(0, 24)).with_label("Prices"),
        Marker::new(MarkerKind::TableRow, CharRange::new(7, 18)).with_label("header"),
        Marker::new(MarkerKind::TableRow, CharRange::new(19, 24)),
        Marker::new(MarkerKind::SectionBreak, CharRange::new(26, 38)),
    ];
    let doc = Document::new(DocumentMeta::default(), Rope::from_str(text), markers);
    let files = docx(&doc);
    let document = part(&files, "word/document.xml");
    assert!(document.contains("<w:tblCaption w:val=\"Prices\"/>"));
    assert!(document.contains("<w:pStyle w:val=\"Caption\"/>"));
    assert!(document.contains("<w:br w:type=\"page\"/>"));
    let body = read_body(document);
    let tables: Vec<_> = body
        .iter()
        .filter(|r| matches!(r, Read::Table { .. }))
        .collect();
    assert_eq!(tables.len(), 1);

    let plain = docx(&Document::from_plain_text(
        "Line one\nline two\ttabbed\n\nSecond <para> & more",
    ));
    let body = read_body(part(&plain, "word/document.xml"));
    let texts: Vec<&str> = body
        .iter()
        .filter_map(|r| match r {
            Read::Para { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts,
        ["Line one\nline two\ttabbed", "Second <para> & more"]
    );
    assert!(part(&plain, "docProps/core.xml").contains("<dc:title>Untitled document</dc:title>"));
}

#[test]
fn styles_used_are_defined() {
    let files = docx(&sample());
    let styles = part(&files, "word/styles.xml");
    let defined: BTreeSet<&str> = styles
        .split("w:styleId=\"")
        .skip(1)
        .map(|s| s.split('"').next().unwrap())
        .collect();
    let document = part(&files, "word/document.xml");
    for key in [
        "w:pStyle w:val=\"",
        "w:rStyle w:val=\"",
        "w:tblStyle w:val=\"",
    ] {
        for used in document.split(key).skip(1) {
            let used = used.split('"').next().unwrap();
            assert!(defined.contains(used), "style {used} is not defined");
        }
    }
}

/// Opens the sample in Microsoft Word through COM and reads back its
/// headings. Run with `TEXTWEAVER_WORD=1 cargo test -p textweaver-writers
/// --test docx -- --ignored` on a Windows machine with Word.
#[test]
#[ignore = "needs Microsoft Word (set TEXTWEAVER_WORD=1)"]
fn opens_in_word() {
    if std::env::var("TEXTWEAVER_WORD").as_deref() != Ok("1") {
        eprintln!("TEXTWEAVER_WORD is not 1; skipping");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sample.docx");
    std::fs::write(
        &path,
        write_to_vec(&sample(), Format::Docx, &options()).unwrap().0,
    )
    .unwrap();
    let script = format!(
        "$ErrorActionPreference = 'Stop'; $w = New-Object -ComObject Word.Application; $w.Visible = $false; try {{ $d = $w.Documents.Open('{}', $false, $true); $out = @(); foreach ($p in $d.Paragraphs) {{ $out += ('' + $p.OutlineLevel + '|' + $p.Range.Text.Trim()) }}; $out += ('LISTS|' + $d.Lists.Count); $out += ('TABLES|' + $d.Tables.Count); $out += ('HEADER|' + $d.Tables.Item(1).Rows.Item(1).HeadingFormat); $out += ('ALT|' + $d.InlineShapes.Item(1).AlternativeText); $d.Close($false); $out -join \"`n\" }} finally {{ $w.Quit() }}",
        path.display()
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .expect("powershell runs");
    let text = String::from_utf8_lossy(&out.stdout);
    eprintln!("{text}\n{}", String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success());
    // Word's OutlineLevel: 1 to 9 for headings, 10 for body text.
    assert!(text.contains("1|Reading Guide"));
    assert!(text.contains("2|Getting started"));
    assert!(text.contains("3|Shortcuts"));
    assert!(text.contains("TABLES|1"));
    assert!(
        text.contains("HEADER|-1"),
        "the first row repeats as a header"
    );
    assert!(text.contains("ALT|Two coloured squares"));
}
