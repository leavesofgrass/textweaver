//! W5c3's documents (ADR-0035): the LaTeX subset, email, and web archives,
//! from `fixtures/c3`, and hostile inputs for each.

use std::path::PathBuf;

use textweaver_core::MarkerKind;
use textweaver_formats::{LoadError, LoadOptions, Registry, Source};
use textweaver_text::{Document, HEADER_ROW_LABEL};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/c3")
        .join(name)
}

fn open(name: &str) -> Document {
    Registry::with_builtins()
        .load(&Source::Path(fixture(name)), &LoadOptions::default())
        .expect("the fixture loads")
}

fn bytes(data: impl Into<Vec<u8>>, hint: &str) -> Result<Document, LoadError> {
    Registry::with_builtins().load(
        &Source::Bytes {
            data: data.into(),
            hint: hint.into(),
        },
        &LoadOptions::default(),
    )
}

fn kinds(d: &Document, kind: MarkerKind) -> Vec<String> {
    d.marker_index()
        .iter(kind, None)
        .map(|m| d.slice(m.range))
        .collect()
}

/// Every marker inside the text and in order, as the fuzz targets check.
fn check(d: &Document) {
    let len = d.len_chars();
    let mut last = 0;
    for m in d.markers() {
        assert!(m.range.start <= m.range.end, "{m:?}");
        assert!(m.range.end.0 <= len, "{m:?} past {len}");
        assert!(m.range.start.0 >= last, "{m:?} out of order");
        last = m.range.start.0;
    }
}

#[test]
fn course_notes_in_latex() {
    let d = open("notes.tex");
    check(&d);
    assert_eq!(d.meta.format, "latex");
    assert_eq!(d.meta.title.as_deref(), Some("Cells and Growth"));
    assert_eq!(d.meta.author.as_deref(), Some("Ada Example"));
    assert_eq!(d.meta.language.as_deref(), Some("en"));
    // Headings for `h`: the title, the abstract, the sections, and the
    // section from the included file.
    assert_eq!(
        kinds(&d, MarkerKind::Heading),
        [
            "Cells and Growth",
            "Abstract",
            "1 Cells",
            "1.1 How fast they divide",
            "2 Data",
            "3 Summary",
            "Footnotes"
        ]
    );
    // The table for `t`, with its caption and a header row.
    let table = d
        .marker_index()
        .nth(MarkerKind::Table, None, 0)
        .cloned()
        .expect("a table");
    assert_eq!(table.label.as_deref(), Some("Table 1: Cell counts by hour"));
    assert_eq!(
        d.slice(table.range),
        "Hour | Cells\n0 | 100\n1 | 200\n2 | 400"
    );
    let header = d
        .marker_index()
        .nth(MarkerKind::TableRow, None, 0)
        .and_then(|r| r.label.clone());
    assert_eq!(header.as_deref(), Some(HEADER_ROW_LABEL));
    // Formulas for the math engine, macros expanded.
    let math = kinds(&d, MarkerKind::Math);
    assert!(
        math.contains(&"$$N(t) = N_0 \\cdot 2^{t}$$".to_owned()),
        "{math:?}"
    );
    assert!(math.iter().any(|m| m.contains("\\mathbb")), "{math:?}");
    let text = d.text().to_string();
    for expected in [
        "Notes for Biology 101 on how cells divide",
        "Equation (1) is used again in Section 2.",
        "Table 1 gives the counts from the lab [@doe2020, p. 12].",
        "Definition 1 (Mitosis). Mitosis is the division",
        "Cells divide by mitosis (Definition 1).",
        "[1] Viruses are the usual exception.",
    ] {
        assert!(text.contains(expected), "{expected:?} in {text}");
    }
    assert!(!text.contains("documentclass"), "{text}");
    assert!(!text.contains('%'), "{text}");
    // The reference to a section is a link to its heading.
    let links: Vec<Option<String>> = d
        .marker_index()
        .iter(MarkerKind::Link, None)
        .map(|m| m.reference.clone())
        .collect();
    assert!(links.contains(&Some("#2-data".into())), "{links:?}");
    assert_eq!(kinds(&d, MarkerKind::ListItem).len(), 5);
    assert!(
        textweaver_formats::warnings(&d.meta).is_empty(),
        "{:?}",
        textweaver_formats::warnings(&d.meta)
    );
}

#[test]
fn an_email_reads_headers_then_body() {
    let d = open("message.eml");
    check(&d);
    assert_eq!(d.meta.format, "eml");
    assert_eq!(d.meta.title.as_deref(), Some("Lab notes for Thursday"));
    let text = d.text().to_string();
    assert!(
        text.starts_with(
            "Lab notes for Thursday\n\nFrom: Ada Example (ada@example.org)\nTo: Bo Example (bo@example.org)\nCc: study-group@example.org\nDate: Monday, September 28, 2026, 10:15, UTC minus 7\n\nHi Bo,\n\nThe lab moved to Thursday."
        ),
        "{text}"
    );
    // The plain part is read, not the HTML alternative as well.
    assert_eq!(text.matches("The lab moved").count(), 1, "{text}");
    assert!(text.contains("Bring the café receipt"), "{text}");
    assert_eq!(
        kinds(&d, MarkerKind::Quote),
        ["Is the lab still on Tuesday?"]
    );
    assert!(
        text.ends_with("Attachments\n\ncounts.csv, 23 bytes\nnotes.pdf, 22 bytes"),
        "{text}"
    );
    assert_eq!(
        kinds(&d, MarkerKind::Heading),
        ["Lab notes for Thursday", "Attachments"]
    );
}

#[test]
fn a_web_archive_reads_as_the_page() {
    let d = open("page.mhtml");
    check(&d);
    assert_eq!(d.meta.format, "mhtml");
    assert_eq!(d.meta.title.as_deref(), Some("Cells"));
    assert_eq!(d.meta.language.as_deref(), Some("en"));
    let text = d.text().to_string();
    assert!(
        text.starts_with("Cells\n\nEvery living thing is made of cells. See the course page."),
        "{text}"
    );
    // The navigation is left out, as in any web page.
    assert!(!text.contains("Home"), "{text}");
    assert!(
        text.contains("A drawing of a cell with its nucleus in the middle"),
        "{text}"
    );
    let links: Vec<Option<String>> = d
        .marker_index()
        .iter(MarkerKind::Link, None)
        .map(|m| m.reference.clone())
        .collect();
    assert_eq!(links, [Some("https://example.org/index.html".into())]);
    let images: Vec<Option<String>> = d
        .marker_index()
        .iter(MarkerKind::Image, None)
        .map(|m| m.reference.clone())
        .collect();
    assert_eq!(
        images,
        [Some("https://example.org/biology/cell.png".into())]
    );
    assert_eq!(kinds(&d, MarkerKind::Math).len(), 1);
    assert_eq!(kinds(&d, MarkerKind::Table).len(), 1);
}

#[test]
fn hostile_latex_is_bounded() {
    let cases: Vec<String> = vec![
        // Macros that expand forever.
        "\\newcommand{\\a}{\\a\\a}\\a".into(),
        "\\def\\b{\\b x}\\b".into(),
        // Deep groups, environments, and arguments.
        "{".repeat(200_000),
        "\\begin{quote}".repeat(20_000),
        "\\emph{".repeat(20_000),
        "\\footnote{".repeat(5_000),
        "\\section{".repeat(5_000),
        // Unclosed math, verbatim, and optional arguments.
        "$".repeat(100_000),
        format!("\\begin{{equation}}{}", "x".repeat(100_000)),
        "\\verb".repeat(10_000),
        "\\item[".repeat(20_000),
        "\\begin{tabular}{l}".repeat(2_000) + &"& \\\\".repeat(10_000),
        // Many labels and references.
        (0..20_000)
            .map(|i| format!("\\label{{l{i}}}\\ref{{l{i}}}"))
            .collect(),
        // Includes with no folder.
        "\\input{x}".repeat(1_000),
    ];
    for src in cases {
        let started = std::time::Instant::now();
        let d = bytes(src.as_bytes().to_vec(), "tex").expect("reads as text");
        check(&d);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(30),
            "{} bytes took {:?}",
            src.len(),
            started.elapsed()
        );
    }
    // Past the size limit, the file is refused.
    let big = vec![b'a'; textweaver_formats::latex::MAX_SOURCE_BYTES + 1];
    assert!(matches!(bytes(big, "tex"), Err(LoadError::Parse(_))));
    // A binary file is refused, not read.
    assert!(matches!(
        bytes(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec(), "tex"),
        Err(LoadError::Binary(..))
    ));
}

#[test]
fn hostile_messages_are_bounded() {
    let cases: Vec<Vec<u8>> = vec![
        Vec::new(),
        b"\r\n\r\n".to_vec(),
        vec![0xff; 10_000],
        // A boundary that never closes, and parts that claim to be
        // messages inside messages.
        b"Content-Type: multipart/mixed; boundary=x\r\n\r\n--x\r\n".repeat(1_000),
        b"Content-Type: message/rfc822\r\n\r\n".repeat(1_000),
        // Headers without end, and encoded words without end.
        b"Subject: =?utf-8?q?".repeat(10_000),
        b"From: ".repeat(10_000),
    ];
    for data in cases {
        for hint in ["eml", "mhtml"] {
            if let Ok(d) = bytes(data.clone(), hint) {
                check(&d);
            }
        }
    }
}
