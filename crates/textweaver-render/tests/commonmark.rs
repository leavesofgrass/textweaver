//! CommonMark 0.31.2 conformance of the pulldown path through the whole
//! textweaver pipeline (event pass and HTML writer), with every example of
//! the spec (`fixtures/l/commonmark-spec.json`, extracted from
//! pulldown-cmark's generated spec tests). The comrak path is checked too.

use serde::Deserialize;
use textweaver_render::{Engine, RenderOptions, render};

#[derive(Deserialize)]
struct Example {
    example: u32,
    markdown: String,
    html: String,
}

fn examples() -> Vec<Example> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/l/commonmark-spec.json"
    );
    let text = std::fs::read_to_string(path).expect("spec fixture");
    serde_json::from_str(&text).expect("spec JSON")
}

/// The same normalization pulldown-cmark's own spec tests use: self-closing
/// spellings, and line breaks between tags.
fn normalize(s: &str) -> String {
    s.replace("<br>", "<br />")
        .replace("<br/>", "<br />")
        .replace("<hr>", "<hr />")
        .replace("<hr/>", "<hr />")
        .replace(">\n<", "><")
}

fn failures(engine: Engine) -> Vec<u32> {
    let opts = RenderOptions {
        engine,
        ..RenderOptions::commonmark()
    };
    examples()
        .into_iter()
        .filter(|ex| normalize(&render(&ex.markdown, &opts).html) != normalize(&ex.html))
        .map(|ex| ex.example)
        .collect()
}

#[test]
fn pulldown_path_passes_every_spec_example() {
    let all = examples();
    assert_eq!(all.len(), 652, "CommonMark 0.31.2 has 652 examples");
    let failed = failures(Engine::PulldownCmark);
    assert!(failed.is_empty(), "failing examples: {failed:?}");
}

#[test]
fn comrak_path_passes_every_spec_example() {
    // comrak's AST is walked into the same events and written by the same
    // writer, so the output must match too.
    let failed = failures(Engine::Comrak);
    assert!(failed.is_empty(), "failing examples: {failed:?}");
}
