//! The conversion matrix: a fixture for every built-in loader, converted
//! to Markdown and to PDF through the converter, each of which must
//! succeed, produce a non-empty file, and keep the headings the loader
//! finds in the fixture.
//!
//! A loader without a fixture fails the test, so a new loader brings one
//! (add it to `FIXTURES`). The PDFs are checked with uncompressed
//! structure: every heading must be in the outline (the bookmarks built
//! from the headings), and the file must have as many heading tags. The
//! second-tool workflow runs veraPDF on PDFs like these.
//!
//! One large generated document (about 1 MB of Markdown, dense with inline
//! styles) is converted to EPUB and PDF under a hang check, so a path that
//! grows with the square of the document cannot come back unnoticed.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use textweaver_convert::{
    ConvertOptions, Converter, FileResult, Job, OutputFormat, PdfOptions, Plan, Status,
    WriteOptions,
};
use textweaver_core::MarkerKind;
use textweaver_formats::{LoadOptions, OcrOptions, Registry, Source};

/// One fixture per built-in loader: (loader id, path from the repository
/// root, fewest headings the loader must find in it).
const FIXTURES: &[(&str, &str, usize)] = &[
    ("text", "fixtures/sample.txt", 0),
    ("markdown", "fixtures/sample.md", 1),
    ("html", "fixtures/sample.html", 1),
    ("epub", "fixtures/a/sample.epub", 1),
    ("docx", "fixtures/a/sample.docx", 1),
    ("rtf", "fixtures/c2/handout.rtf", 1),
    ("odt", "fixtures/c2/notes.odt", 1),
    ("latex", "fixtures/c3/notes.tex", 1),
    ("eml", "fixtures/c3/message.eml", 0),
    ("mhtml", "fixtures/c3/page.mhtml", 1),
    ("pdf", "fixtures/a/notes.pdf", 0),
    ("image", "fixtures/w3d/scan-small.png", 0),
    ("daisy", "fixtures/k/book.dtbook", 1),
    ("pptx", "fixtures/k/lesson.pptx", 1),
    ("sheet", "fixtures/k/grades.csv", 0),
    ("archive", "fixtures/w3d/course.7z", 0),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Loading as the converter does, with OCR off: the matrix checks the
/// conversion, not recognition (which needs models this test does not
/// download).
fn load_options() -> LoadOptions {
    LoadOptions {
        ocr: OcrOptions {
            enabled: false,
            ..OcrOptions::default()
        },
        ..LoadOptions::default()
    }
}

fn options(to: OutputFormat, out: &Path) -> ConvertOptions {
    ConvertOptions {
        to,
        out_dir: Some(out.to_owned()),
        force: true,
        pandoc: false,
        load: load_options(),
        write: WriteOptions {
            pdf: PdfOptions {
                compress: false,
                ..PdfOptions::default()
            },
            ..WriteOptions::default()
        },
        ..ConvertOptions::default()
    }
}

/// Letters and digits only, lowercased: headings compared across formats
/// that escape or break them differently.
fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// The headings the loader finds in `path`, normalized.
fn headings(registry: &Registry, path: &Path) -> Vec<String> {
    let doc = registry
        .load(&Source::Path(path.to_owned()), &load_options())
        .unwrap_or_else(|e| panic!("{} does not load: {e}", path.display()));
    doc.marker_index()
        .iter(MarkerKind::Heading, None)
        .map(|m| norm(&doc.slice(m.range)))
        .filter(|h| !h.is_empty())
        .collect()
}

/// The PDF text strings after each `/Title` (the outline entries and the
/// document title), decoded from literal or UTF-16 hex strings.
fn pdf_titles(pdf: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let key = b"/Title";
    let mut i = 0;
    while let Some(off) = pdf[i..].windows(key.len()).position(|w| w == key) {
        let mut j = i + off + key.len();
        while j < pdf.len() && pdf[j].is_ascii_whitespace() {
            j += 1;
        }
        if pdf.get(j) == Some(&b'(') {
            let mut bytes = Vec::new();
            let mut depth = 0;
            j += 1;
            while j < pdf.len() {
                match pdf[j] {
                    b'\\' => {
                        j += 1;
                        if let Some(&c) = pdf.get(j) {
                            bytes.push(match c {
                                b'n' => b'\n',
                                b'r' => b'\r',
                                b't' => b'\t',
                                other => other,
                            });
                        }
                    }
                    b'(' => {
                        depth += 1;
                        bytes.push(b'(');
                    }
                    b')' if depth == 0 => break,
                    b')' => {
                        depth -= 1;
                        bytes.push(b')');
                    }
                    c => bytes.push(c),
                }
                j += 1;
            }
            out.push(if bytes.starts_with(&[0xFE, 0xFF]) {
                utf16(&bytes[2..])
            } else {
                bytes.iter().map(|&b| char::from(b)).collect()
            });
        } else if pdf.get(j) == Some(&b'<') {
            let end = pdf[j..]
                .iter()
                .position(|&b| b == b'>')
                .map_or(pdf.len(), |e| j + e);
            let hex: Vec<u8> = pdf[j + 1..end]
                .iter()
                .copied()
                .filter(u8::is_ascii_hexdigit)
                .collect();
            let bytes: Vec<u8> = hex
                .chunks(2)
                .filter_map(|p| u8::from_str_radix(std::str::from_utf8(p).ok()?, 16).ok())
                .collect();
            out.push(if bytes.starts_with(&[0xFE, 0xFF]) {
                utf16(&bytes[2..])
            } else {
                bytes.iter().map(|&b| char::from(b)).collect()
            });
        }
        i = j.max(i + off + 1);
    }
    out
}

fn utf16(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks(2)
        .map(|p| u16::from_be_bytes([p[0], *p.get(1).unwrap_or(&0)]))
        .collect();
    String::from_utf16_lossy(&units)
}

fn count(hay: &[u8], needle: &[u8]) -> usize {
    hay.windows(needle.len()).filter(|w| *w == needle).count()
}

/// Runs one job per fixture to `to`, each into its own folder (several
/// fixtures share a file name), and returns the results in fixture order.
fn run(to: OutputFormat, out: &Path) -> Vec<FileResult> {
    let conv = Converter::new(options(to, out)).expect("converter");
    let jobs = FIXTURES
        .iter()
        .map(|(id, rel, _)| {
            let source = root().join(rel);
            let name = source.file_stem().expect("a file name").to_owned();
            Job {
                output: out
                    .join(id)
                    .join(PathBuf::from(name).with_extension(to.extension())),
                root: source.parent().expect("a folder").to_owned(),
                source,
            }
        })
        .collect();
    let plan = Plan {
        jobs,
        rejected: Vec::new(),
    };
    let s = conv
        .run_plan_with(plan, |_| {}, &AtomicBool::new(false))
        .expect("run");
    assert_eq!(s.files.len(), FIXTURES.len());
    s.files
}

#[test]
fn every_loader_has_a_fixture() {
    let registry = Registry::with_builtins();
    let with_fixture: BTreeSet<&str> = FIXTURES.iter().map(|f| f.0).collect();
    let missing: Vec<&str> = registry
        .ids()
        .into_iter()
        .filter(|id| registry.loader_by_id(id).is_some_and(|l| l.available()))
        .filter(|id| !with_fixture.contains(id))
        .collect();
    assert!(
        missing.is_empty(),
        "loaders without a fixture in the conversion matrix: {missing:?}; add one to FIXTURES"
    );
    for (id, rel, _) in FIXTURES {
        let path = root().join(rel);
        assert!(path.is_file(), "{rel} is missing");
        let loader = registry.loader_for(&Source::Path(path)).map(|l| l.id());
        assert_eq!(loader, Some(*id), "{rel} is read by another loader");
    }
}

#[test]
fn every_fixture_converts_to_markdown_with_its_headings() {
    let dir = tempfile::tempdir().expect("tempdir");
    let registry = Registry::with_builtins();
    let results = run(OutputFormat::Markdown, dir.path());
    let mut problems = Vec::new();
    for ((id, rel, min), f) in FIXTURES.iter().zip(&results) {
        if let Status::Failed(reason) = &f.status {
            problems.push(format!("{id} ({rel}): failed: {reason}"));
            continue;
        }
        let text = std::fs::read_to_string(&f.output).unwrap_or_default();
        if text.trim().is_empty() {
            problems.push(format!("{id} ({rel}): empty Markdown"));
            continue;
        }
        let mut expected = headings(&registry, &f.source);
        if *id == "markdown" {
            // Markdown to Markdown is a copy: the headings the loader adds
            // (a "Footnotes" heading over the notes) are not in it.
            let source = norm(&std::fs::read_to_string(&f.source).unwrap_or_default());
            expected.retain(|h| source.contains(h.as_str()));
        }
        if expected.len() < *min {
            problems.push(format!(
                "{id} ({rel}): the loader finds {} headings, expected at least {min}",
                expected.len()
            ));
        }
        let written: Vec<String> = text
            .lines()
            .filter(|l| l.trim_start().starts_with('#'))
            .map(norm)
            .collect();
        for h in &expected {
            if !written.iter().any(|w| w.contains(h.as_str())) {
                problems.push(format!(
                    "{id} ({rel}): heading {h:?} is not a Markdown heading"
                ));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_fixture_converts_to_pdf_with_its_headings() {
    let dir = tempfile::tempdir().expect("tempdir");
    let registry = Registry::with_builtins();
    let results = run(OutputFormat::Pdf, dir.path());
    let mut problems = Vec::new();
    for ((id, rel, _), f) in FIXTURES.iter().zip(&results) {
        if let Status::Failed(reason) = &f.status {
            problems.push(format!("{id} ({rel}): failed: {reason}"));
            continue;
        }
        let pdf = std::fs::read(&f.output).unwrap_or_default();
        if !pdf.starts_with(b"%PDF-") || f.bytes_out == 0 {
            problems.push(format!("{id} ({rel}): not a PDF"));
            continue;
        }
        let expected = headings(&registry, &f.source);
        let titles: Vec<String> = pdf_titles(&pdf).iter().map(|t| norm(t)).collect();
        for h in &expected {
            if !titles.iter().any(|t| t.contains(h.as_str())) {
                problems.push(format!("{id} ({rel}): heading {h:?} is not in the outline"));
            }
        }
        let tags = count(&pdf, b"/S/H") + count(&pdf, b"/S /H");
        if tags < expected.len() {
            problems.push(format!(
                "{id} ({rel}): {tags} heading tags for {} headings",
                expected.len()
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// About `bytes` of Markdown dense with inline styles, links, and lists.
fn large_markdown(bytes: usize) -> String {
    let words = [
        "crow", "river", "reading", "braille", "voice", "chapter", "table", "note", "list",
        "heading",
    ];
    let mut out = String::from("# A large document\n\n");
    let mut n = 0usize;
    let mut section = 0;
    while out.len() < bytes {
        if n.is_multiple_of(400) {
            section += 1;
            out.push_str(&format!("\n## Section {section}\n\n"));
        }
        let w = words[n % words.len()];
        match n % 7 {
            0 => out.push_str(&format!("**{w}** ")),
            1 => out.push_str(&format!("*{w}* ")),
            2 => out.push_str(&format!("`{w}` ")),
            3 => out.push_str(&format!("[{w}](https://example.org/{w}) ")),
            _ => {
                out.push_str(w);
                out.push(' ');
            }
        }
        n += 1;
        if n.is_multiple_of(40) {
            out.push_str("\n\n");
        }
        if n.is_multiple_of(1000) {
            for i in 0..20 {
                out.push_str(&format!("- item {i} with *{w}*\n"));
            }
            out.push('\n');
        }
    }
    out
}

#[test]
fn a_large_document_converts_without_hanging() {
    let dir = tempfile::tempdir().expect("tempdir");
    let source = dir.path().join("large.md");
    std::fs::write(&source, large_markdown(1 << 20)).expect("write");
    for to in [OutputFormat::Epub, OutputFormat::Pdf] {
        let out = dir.path().join(to.extension());
        let conv = Converter::new(options(to, &out)).expect("converter");
        let started = Instant::now();
        let s = conv.run(std::slice::from_ref(&source)).expect("run");
        let took = started.elapsed();
        assert_eq!(s.converted, 1, "{}", s.sentence());
        assert!(
            took < Duration::from_secs(60),
            "1 MB to {} took {took:?}: a hang, or a path that grows with the square of the document",
            to.label()
        );
    }
}
