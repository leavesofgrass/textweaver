//! Citations in `tw convert`: formatted with textweaver-cite, a References
//! section appended, HTML citations linked to entries that exist.

use std::fs;
use std::path::{Path, PathBuf};

use textweaver_convert::{CitationOptions, ConvertOptions, Converter, OutputFormat, Status};
use textweaver_formats::{LoadOptions, Registry, Source};
use textweaver_render::{Flavor, RenderOptions};

const LIBRARY: &str = r#"[
  {"id": "doe2020", "type": "article-journal", "title": "On testing",
   "author": [{"family": "Doe", "given": "Jane"}], "issued": {"date-parts": [[2020]]},
   "container-title": "Journal of Tests", "volume": "3", "page": "1-10"},
  {"id": "roe2021", "type": "book", "title": "Reading aloud",
   "author": [{"family": "Roe", "given": "Richard"}], "issued": {"date-parts": [[2021]]},
   "publisher": "Example Press"}
]"#;

const DOC: &str = "# Paper\n\nTesting matters [@doe2020, p. 3]. As @roe2021 shows, reading aloud helps.\nMail me at ada@example.com or ask @nobody. Code `[@doe2020]` stays.\n";

fn setup() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("paper");
    fs::create_dir_all(&src).expect("mkdir");
    fs::write(src.join("references.json"), LIBRARY).expect("library");
    fs::write(src.join("paper.md"), DOC).expect("doc");
    (dir, src)
}

fn convert(src: &Path, out: &Path, to: OutputFormat, citations: CitationOptions) -> String {
    convert_with(src, out, to, citations, RenderOptions::default())
}

fn convert_with(
    src: &Path,
    out: &Path,
    to: OutputFormat,
    citations: CitationOptions,
    render: RenderOptions,
) -> String {
    let conv = Converter::new(ConvertOptions {
        to,
        out_dir: Some(out.to_owned()),
        force: true,
        citations,
        render,
        ..ConvertOptions::default()
    })
    .expect("converter");
    let s = conv.run(&[src.join("paper.md")]).expect("run");
    assert_eq!(s.converted, 1, "{:?}", s.files);
    let path = out.join(format!("paper.{}", to.extension()));
    if to.needs_writer() {
        // Read the written file back as text.
        let doc = Registry::with_builtins()
            .load(&Source::Path(path), &LoadOptions::default())
            .expect("output reads back");
        doc.text().to_string()
    } else {
        fs::read_to_string(path).expect("output")
    }
}

#[test]
fn html_citations_link_to_references_that_exist() {
    let (dir, src) = setup();
    let html = convert(
        &src,
        &dir.path().join("out"),
        OutputFormat::Html,
        CitationOptions::default(),
    );
    assert!(html.contains("(Doe, 2020, p. 3)"), "{html}");
    assert!(html.contains("Roe (2021)"), "{html}");
    assert!(html.contains("href=\"#ref-doe2020\""), "{html}");
    assert!(html.contains("id=\"ref-doe2020\""), "{html}");
    assert!(html.contains("id=\"ref-roe2021\""), "{html}");
    assert!(html.contains(">References</h2>"), "{html}");
    // Every #ref- link has its target.
    for part in html.split("href=\"#ref-").skip(1) {
        let key = part.split('"').next().unwrap();
        assert!(
            html.contains(&format!("id=\"ref-{key}\"")),
            "no entry for {key}"
        );
    }
    // Mentions, e-mail addresses, and code are left alone.
    assert!(html.contains("@nobody"), "{html}");
    assert!(html.contains("ada@example.com"), "{html}");
    assert!(html.contains("<code>[@doe2020]</code>"), "{html}");
}

#[test]
fn every_writer_gets_formatted_citations_and_references() {
    let (dir, src) = setup();
    for to in [
        OutputFormat::Pdf,
        OutputFormat::Docx,
        OutputFormat::Epub,
        OutputFormat::Text,
    ] {
        let text = convert(
            &src,
            &dir.path().join("out"),
            to,
            CitationOptions::default(),
        );
        assert!(text.contains("(Doe, 2020, p. 3)"), "{to:?}: {text}");
        assert!(text.contains("References"), "{to:?}: {text}");
        assert!(text.contains("Doe, J. (2020)"), "{to:?}: {text}");
        assert!(text.contains("Roe, R. (2021)"), "{to:?}: {text}");
        assert!(!text.contains("[@doe2020, p. 3]"), "{to:?}: {text}");
    }
}

#[test]
fn a_bibliography_file_and_a_style_can_be_chosen() {
    let (dir, src) = setup();
    fs::remove_file(src.join("references.json")).unwrap();
    let bib = dir.path().join("refs.bib");
    fs::write(
        &bib,
        "@article{doe2020, author = {Doe, Jane}, title = {On testing}, journal = {Journal of Tests}, year = {2020}, volume = {3}, pages = {1--10}}\n@book{roe2021, author = {Roe, Richard}, title = {Reading aloud}, publisher = {Example Press}, year = {2021}}\n",
    )
    .unwrap();
    let text = convert(
        &src,
        &dir.path().join("out"),
        OutputFormat::Text,
        CitationOptions {
            bibliography: Some(bib),
            style: "ieee".into(),
            ..CitationOptions::default()
        },
    );
    assert!(text.contains("[1]"), "{text}");
    assert!(text.contains("J. Doe"), "{text}");
    assert!(text.contains("References"), "{text}");
}

#[test]
fn note_styles_use_footnotes_and_pandoc_marks_missing_keys() {
    let (dir, src) = setup();
    let text = convert_with(
        &src,
        &dir.path().join("out"),
        OutputFormat::Text,
        CitationOptions {
            style: "chicago-notes".into(),
            ..CitationOptions::default()
        },
        RenderOptions {
            flavor: Flavor::Pandoc,
            ..RenderOptions::default()
        },
    );
    assert!(text.contains("Footnotes"), "{text}");
    assert!(
        text.contains("On Testing") || text.contains("On testing"),
        "{text}"
    );
    // The Pandoc flavor formats every citation, so the mention of a key
    // that is in no library is marked.
    assert!(text.contains("missing reference nobody"), "{text}");
}

#[test]
fn an_unknown_style_stops_the_run_and_markdown_output_is_untouched() {
    let err = Converter::new(ConvertOptions {
        citations: CitationOptions {
            style: "no-such-style".into(),
            ..CitationOptions::default()
        },
        ..ConvertOptions::default()
    })
    .unwrap_err();
    assert!(err.to_string().contains("no-such-style"), "{err}");

    let (dir, src) = setup();
    let md = convert(
        &src,
        &dir.path().join("out"),
        OutputFormat::Markdown,
        CitationOptions::default(),
    );
    assert_eq!(md, DOC);
}

#[test]
fn missing_keys_are_warnings() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("references.json"), LIBRARY).unwrap();
    fs::write(dir.path().join("paper.md"), "See [@doe2020; @smith1999].\n").unwrap();
    let s = Converter::new(ConvertOptions {
        to: OutputFormat::Html,
        out_dir: Some(dir.path().join("out")),
        ..ConvertOptions::default()
    })
    .unwrap()
    .run(&[dir.path().join("paper.md")])
    .unwrap();
    assert_eq!(s.files[0].status, Status::Converted);
    assert_eq!(s.warned, 1);
    assert!(
        s.files[0].warnings[0].contains("smith1999 is not in any library"),
        "{:?}",
        s.files[0].warnings
    );
}
