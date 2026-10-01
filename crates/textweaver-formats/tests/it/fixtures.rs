//! Snapshot tests: every fixture in `fixtures/` loaded, and read aloud.
//!
//! The document snapshots show the canonical text and every marker with the
//! text it covers; the narration snapshots show the utterances
//! `narrate::plan` produces with the default policy. Review changes with
//! `cargo insta review` (or set `INSTA_UPDATE=always` and inspect the diff).

use std::path::PathBuf;

use serde_json::{Value, json};
use textweaver_core::Unit;
use textweaver_formats::{FootnoteMode, LoadOptions, Registry, Source};
use textweaver_text::{Document, NarrationPolicy, plan, segments};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

fn load(name: &str, options: &LoadOptions) -> Document {
    Registry::with_builtins()
        .load(&Source::Path(fixture(name)), options)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The document as JSON, without the machine-specific path.
fn view(doc: &Document) -> Value {
    let mut meta = doc.meta.clone();
    meta.path = None;
    let markers: Vec<Value> = doc
        .markers()
        .iter()
        .map(|m| {
            let mut v = json!({
                "kind": m.kind,
                "range": [m.range.start.0, m.range.end.0],
                "text": doc.slice(m.range),
            });
            if m.level != 0 {
                v["level"] = json!(m.level);
            }
            if let Some(l) = &m.label {
                v["label"] = json!(l);
            }
            if let Some(r) = &m.reference {
                v["reference"] = json!(r);
            }
            v
        })
        .collect();
    json!({ "meta": meta, "text": doc.text().to_string(), "markers": markers })
}

fn narration(doc: &Document) -> Vec<String> {
    let us = plan(doc, doc.full_range(), &NarrationPolicy::default());
    for u in &us {
        u.offset_map
            .check_invariants(&u.text)
            .unwrap_or_else(|e| panic!("{:?}: {e}", u.text));
    }
    us.into_iter().map(|u| u.text).collect()
}

#[test]
fn sample_txt() {
    let doc = load("sample.txt", &LoadOptions::default());
    insta::assert_json_snapshot!("sample_txt_document", view(&doc));
    insta::assert_json_snapshot!("sample_txt_narration", narration(&doc));
}

#[test]
fn sample_md() {
    let doc = load("sample.md", &LoadOptions::default());
    insta::assert_json_snapshot!("sample_md_document", view(&doc));
    insta::assert_json_snapshot!("sample_md_narration", narration(&doc));
}

#[test]
fn sample_md_inline_footnotes_skip_code() {
    let options = LoadOptions {
        skip_code: true,
        footnotes: FootnoteMode::Inline,
        ..LoadOptions::default()
    };
    let doc = load("sample.md", &options);
    insta::assert_json_snapshot!("sample_md_inline_skip_document", view(&doc));
}

#[test]
fn sample_html() {
    let doc = load("sample.html", &LoadOptions::default());
    insta::assert_json_snapshot!("sample_html_document", view(&doc));
    insta::assert_json_snapshot!("sample_html_narration", narration(&doc));
}

#[test]
fn headings_list_items_and_rows_are_lines() {
    // The acceptance check behind `tw text fixtures/sample.md`.
    let doc = load("sample.md", &LoadOptions::default());
    let lines: Vec<String> = segments(&doc, Unit::Line)
        .into_iter()
        .map(|r| doc.slice(r))
        .collect();
    for expected in [
        "Sample Markdown Document",
        "Lists",
        "First bullet item",
        "Nested bullet under the second",
        "Step two costs $5.25.",
        "Name | Role | Score",
        "Grace | Admiral | 100",
    ] {
        assert!(
            lines.iter().any(|l| l == expected),
            "missing line {expected:?}"
        );
    }
}

#[test]
fn sample_epub() {
    let doc = load("a/sample.epub", &LoadOptions::default());
    insta::assert_json_snapshot!("a_sample_epub_document", view(&doc));
    insta::assert_json_snapshot!("a_sample_epub_narration", narration(&doc));
}

#[test]
fn pandoc_epub() {
    let doc = load("a/pandoc.epub", &LoadOptions::default());
    insta::assert_json_snapshot!("a_pandoc_epub_document", view(&doc));
}

#[test]
fn sample_docx() {
    let doc = load("a/sample.docx", &LoadOptions::default());
    insta::assert_json_snapshot!("a_sample_docx_document", view(&doc));
    insta::assert_json_snapshot!("a_sample_docx_narration", narration(&doc));
    let inline = load(
        "a/sample.docx",
        &LoadOptions {
            skip_code: false,
            footnotes: FootnoteMode::Inline,
            ..LoadOptions::default()
        },
    );
    let text = inline.text().to_string();
    assert!(text.contains("p.m. (footnote: The footnote text lives here.) It ends"));
    assert!(!text.contains("Footnotes"));
}

#[test]
fn pandoc_docx() {
    let doc = load("a/pandoc.docx", &LoadOptions::default());
    insta::assert_json_snapshot!("a_pandoc_docx_document", view(&doc));
    let skipped = load(
        "a/pandoc.docx",
        &LoadOptions {
            skip_code: true,
            footnotes: FootnoteMode::Skip,
            ..LoadOptions::default()
        },
    );
    let text = skipped.text().to_string();
    assert!(!text.contains("println"));
    assert!(!text.contains("[1]"));
}

/// Load time of every fixture (fastest of five, from memory); run with
/// `cargo test --release -p textweaver-formats --test fixtures -- --ignored --nocapture`.
#[test]
#[ignore = "benchmark"]
fn fixture_load_times() {
    let registry = Registry::with_builtins();
    for name in [
        "sample.txt",
        "sample.md",
        "sample.html",
        "a/sample.epub",
        "a/pandoc.epub",
        "a/sample.docx",
        "a/pandoc.docx",
    ] {
        let path = fixture(name);
        let data = std::fs::read(&path).expect("readable");
        let hint = path
            .extension()
            .map(|e| e.to_string_lossy().into_owned())
            .unwrap_or_default();
        let source = Source::Bytes { data, hint };
        let mut best = std::time::Duration::MAX;
        let mut chars = 0;
        for _ in 0..5 {
            let started = std::time::Instant::now();
            let doc = registry
                .load(&source, &LoadOptions::default())
                .expect("loads");
            best = best.min(started.elapsed());
            chars = doc.len_chars();
        }
        eprintln!("{name}: {chars} chars, {best:?}");
    }
}
