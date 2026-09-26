//! Bulk conversion through the native writers (Agent M's
//! `textweaver-writers`): EPUB, DOCX, BRF, and PDF on a temporary tree,
//! with mirrored paths, skip-unchanged, writer warnings carried into the
//! results and the summary, and the PDF font check.

use std::fs;
use std::path::{Path, PathBuf};

use textweaver_convert::{
    ConvertError, ConvertOptions, Converter, OutputFormat, PdfOptions, Status, Summary, WatchEvent,
    WriteOptions,
};

fn write(root: &Path, rel: &str, text: &str) -> PathBuf {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
    fs::write(&p, text).expect("write");
    p
}

/// Markdown with structure and a missing image, HTML, and plain text.
fn tree(root: &Path) {
    write(
        root,
        "a.md",
        "---\ntitle: Alpha notes\n---\n# Alpha\n\nSome *text* and a list:\n\n- one\n- two\n\n| Name | Age |\n|------|-----|\n| Ann | 30 |\n\n![A missing cat](cat.png)\n",
    );
    write(root, "sub/b.md", "# Beta\n\n1. first\n2. second\n");
    write(
        root,
        "sub/deep/c.html",
        "<html lang=\"en\"><head><title>Gamma page</title></head><body><h1>Gamma</h1><p>Hello, world.</p></body></html>",
    );
    write(root, "notes.txt", "Plain line one.\n\nLine two.\n");
}

fn options(to: OutputFormat, out: &Path) -> ConvertOptions {
    ConvertOptions {
        to,
        out_dir: Some(out.to_owned()),
        pandoc: false,
        jobs: Some(4),
        ..ConvertOptions::default()
    }
}

const OUTPUTS: [&str; 4] = ["a", "sub/b", "sub/deep/c", "notes"];

/// Converts the tree to `to`, checks mirrored outputs and skip-unchanged,
/// and returns the first summary.
fn bulk(to: OutputFormat) -> (tempfile::TempDir, Summary) {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("in");
    let out = dir.path().join("out");
    tree(&input);
    let conv = Converter::new(options(to, &out)).expect("converter");
    let first = conv.run(std::slice::from_ref(&input)).expect("run");
    assert_eq!(
        (first.converted, first.failed),
        (4, 0),
        "{to:?}: {:?}",
        first
            .failures()
            .map(|f| (&f.source, &f.status))
            .collect::<Vec<_>>()
    );
    for rel in OUTPUTS {
        let p = out.join(format!("{rel}.{}", to.extension()));
        assert!(p.is_file(), "missing {}", p.display());
        assert!(fs::metadata(&p).expect("meta").len() > 0);
    }
    let second = conv.run(std::slice::from_ref(&input)).expect("run");
    assert_eq!((second.converted, second.skipped), (0, 4), "{to:?}");
    (dir, first)
}

fn read(dir: &tempfile::TempDir, rel: &str) -> Vec<u8> {
    fs::read(dir.path().join("out").join(rel)).expect("read output")
}

fn contains(hay: &[u8], needle: &str) -> bool {
    hay.windows(needle.len()).any(|w| w == needle.as_bytes())
}

/// The missing image is reported for a.md only, in a sentence.
fn missing_image_warned(s: &Summary) {
    let warned: Vec<&Path> = s.warnings().map(|f| f.source.as_path()).collect();
    assert_eq!(warned.len(), 1, "{warned:?}");
    assert!(warned[0].ends_with("a.md"));
    assert_eq!(s.warned, 1);
    let a = s.warnings().next().expect("warned file");
    assert!(
        a.warnings.iter().any(|w| w.contains("cat.png")),
        "{:?}",
        a.warnings
    );
    assert!(
        s.sentence().contains(" 1 file has warnings. No failures."),
        "{}",
        s.sentence()
    );
    // The JSON summary carries the warnings of that file only.
    let json = serde_json::to_value(s).expect("json");
    let files = json["files"].as_array().expect("files");
    let with: Vec<_> = files
        .iter()
        .filter(|f| f.get("warnings").is_some())
        .collect();
    assert_eq!(with.len(), 1);
}

#[test]
fn epub_bulk() {
    let (dir, s) = bulk(OutputFormat::Epub);
    assert!(s.sentence().starts_with("Converted 4 files to EPUB in "));
    missing_image_warned(&s);
    let a = read(&dir, "a.epub");
    assert!(a.starts_with(b"PK"));
    // The OCF mimetype entry comes first, stored.
    assert!(contains(&a[..80], "mimetypeapplication/epub+zip"));
    assert!(contains(&a, "nav.xhtml"));
}

#[test]
fn docx_bulk() {
    let (dir, s) = bulk(OutputFormat::Docx);
    assert!(s.sentence().starts_with("Converted 4 files to Word in "));
    missing_image_warned(&s);
    let c = read(&dir, "sub/deep/c.docx");
    assert!(c.starts_with(b"PK"));
    assert!(contains(&c, "word/document.xml"));
    assert!(contains(&c, "word/styles.xml"));
}

#[test]
fn brf_bulk() {
    let (dir, s) = bulk(OutputFormat::Brf);
    assert!(s.sentence().starts_with("Converted 4 files to braille in "));
    let notes = String::from_utf8(read(&dir, "notes.brf")).expect("BRF is ASCII");
    // Uncontracted UEB: paragraphs start in cell 3, a comma marks a
    // capital, 4 is the period.
    assert!(notes.starts_with("  ,PLAIN LINE ONE4"), "{notes:?}");
    assert!(notes.contains("\r\n"));
    for line in notes.split("\r\n") {
        let line = line.trim_start_matches('\u{c}');
        assert!(line.chars().count() <= 40, "{line:?}");
        assert!(line.chars().all(|c| (' '..='_').contains(&c)), "{line:?}");
    }
    let b = String::from_utf8(read(&dir, "sub/b.brf")).expect("ascii");
    assert!(b.contains(",BETA"), "{b:?}");
}

#[test]
fn pdf_bulk_or_a_clear_font_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join("out");
    match Converter::new(options(OutputFormat::Pdf, &out)) {
        Ok(_) => {
            let (dir, s) = bulk(OutputFormat::Pdf);
            assert!(s.sentence().starts_with("Converted 4 files to PDF in "));
            missing_image_warned(&s);
            let a = read(&dir, "a.pdf");
            assert!(a.starts_with(b"%PDF-"));
            assert!(contains(&a, "%%EOF"));
        }
        Err(ConvertError::Output(msg)) => {
            eprintln!("no PDF font on this machine: {msg}");
            assert!(msg.starts_with("PDF output needs a font"), "{msg}");
            assert!(msg.contains("TEXTWEAVER_PDF_FONT"), "{msg}");
        }
        Err(e) => panic!("unexpected error: {e}"),
    }
}

#[test]
fn a_named_font_that_cannot_be_used_is_one_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bad = write(dir.path(), "not-a-font.ttf", "this is not a font");
    let err = Converter::new(ConvertOptions {
        write: WriteOptions {
            pdf: PdfOptions {
                font: Some(bad),
                ..PdfOptions::default()
            },
            ..WriteOptions::default()
        },
        ..options(OutputFormat::Pdf, &dir.path().join("out"))
    })
    .unwrap_err();
    assert!(
        err.to_string().starts_with("Cannot write PDF files: "),
        "{err}"
    );
    // Other formats never look for a font.
    assert!(
        Converter::new(ConvertOptions {
            write: WriteOptions {
                pdf: PdfOptions {
                    font: Some(dir.path().join("missing.ttf")),
                    ..PdfOptions::default()
                },
                ..WriteOptions::default()
            },
            ..options(OutputFormat::Epub, &dir.path().join("out"))
        })
        .is_ok()
    );
}

#[test]
fn a_single_file_and_watch_sentences_carry_warnings() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("in");
    tree(&input);
    let out = dir.path().join("out");
    let s = Converter::new(options(OutputFormat::Epub, &out))
        .expect("converter")
        .run(&[input.join("a.md")])
        .expect("run");
    assert_eq!(s.converted, 1);
    assert!(out.join("a.epub").is_file());
    let f = s.files[0].clone();
    assert_eq!(f.status, Status::Converted);
    let sentence = WatchEvent::File(f).sentence();
    assert!(
        sentence.starts_with("Converted a.md. Warning: ") && sentence.contains("cat.png"),
        "{sentence}"
    );
}
