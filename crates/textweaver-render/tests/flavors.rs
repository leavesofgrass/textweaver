//! Flavor features, snapshot-tested per fixture, and engine parity: the
//! pulldown and comrak paths must render every fixture identically.

use textweaver_render::{
    EmbedMode, Engine, Flavor, FsResolver, PageOptions, RenderOptions, Rendered, Templates,
    render_with,
};

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/../../fixtures/l/flavors/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path)
        .expect("fixture")
        .replace("\r\n", "\n")
}

fn vault() -> FsResolver {
    FsResolver::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/l/vault"
    ))
}

fn render_both(src: &str, flavor: Flavor, embeds: EmbedMode) -> Rendered {
    let resolver = vault();
    let opts = RenderOptions {
        flavor,
        embeds,
        ..RenderOptions::default()
    };
    let pulldown = render_with(src, &opts, Some(&resolver));
    let comrak = render_with(
        src,
        &RenderOptions {
            engine: Engine::Comrak,
            ..opts
        },
        Some(&resolver),
    );
    assert_eq!(
        pulldown.html, comrak.html,
        "engines differ for the {flavor:?} fixture"
    );
    assert_eq!(pulldown.toc, comrak.toc);
    assert_eq!(pulldown.meta, comrak.meta);
    pulldown
}

/// The snapshot: the HTML, then the facts beside it.
fn snapshot_text(r: &Rendered) -> String {
    let facts = serde_json::json!({
        "title": r.title(),
        "meta": r.meta,
        "toc": r.toc,
        "tags": r.tags,
        "has_math": r.has_math,
        "has_h1": r.has_h1,
    });
    format!(
        "{}\n----- facts -----\n{}\n",
        r.html,
        serde_json::to_string_pretty(&facts).unwrap_or_default()
    )
}

#[test]
fn gfm() {
    let r = render_both(&fixture("gfm.md"), Flavor::Gfm, EmbedMode::Link);
    assert!(r.html.contains("<del>wrong</del>"));
    assert!(
        r.html
            .contains("<a href=\"http://www.example.com\">www.example.com</a>")
    );
    assert!(r.html.contains("type=\"checkbox\""));
    assert!(r.html.contains("role=\"doc-endnotes\""));
    assert!(
        r.html
            .contains("class=\"callout callout-warning\" role=\"note\"")
    );
    insta::assert_snapshot!("gfm", snapshot_text(&r));
}

#[test]
fn obsidian_with_links() {
    let r = render_both(&fixture("obsidian.md"), Flavor::Obsidian, EmbedMode::Link);
    assert!(
        r.html
            .contains("href=\"Chapter%20One.html#key-ideas\">the key ideas</a>")
    );
    assert!(
        r.html
            .contains("<p id=\"block-idea-1\">This idea matters.</p>")
    );
    assert!(r.html.contains("[[not a link]] #not-a-tag"));
    assert_eq!(r.tags, ["reading", "study/biology"]);
    insta::assert_snapshot!("obsidian", snapshot_text(&r));
}

#[test]
fn obsidian_with_inline_embeds() {
    let r = render_both(&fixture("obsidian.md"), Flavor::Obsidian, EmbedMode::Inline);
    assert!(r.html.contains("aria-label=\"Embedded note: Chapter One\""));
    assert!(r.html.contains("Light keeps ships off the rocks."));
    assert!(!r.html.contains("Not part of the key ideas."));
}

#[test]
fn pandoc() {
    let r = render_both(&fixture("pandoc.md"), Flavor::Pandoc, EmbedMode::Link);
    assert_eq!(r.title().as_deref(), Some("Pandoc features"));
    assert!(r.html.contains("<dl>"));
    assert!(r.html.contains("<div id=\"careful\" class=\"warning\">"));
    assert!(
        r.html
            .contains("<h1 id=\"intro\" class=\"lead\">Introduction</h1>")
    );
    assert!(r.html.contains("<h1 id=\"introduction\">Introduction</h1>"));
    assert!(r.html.contains("<sub>2</sub>"));
    assert!(r.html.contains("<sup>10</sup>"));
    insta::assert_snapshot!("pandoc", snapshot_text(&r));
}

#[test]
fn default_template_is_an_accessible_page() {
    let r = render_both(&fixture("gfm.md"), Flavor::Gfm, EmbedMode::Link);
    let page = Templates::builtin()
        .render("default", &r, &PageOptions::default())
        .expect("render");
    assert!(page.starts_with("<!DOCTYPE html>\n<html lang=\"en-US\">"));
    assert!(page.contains("<title>GFM features</title>"));
    assert!(page.contains("<a class=\"skip-link\" href=\"#main\">Skip to content</a>"));
    assert!(page.contains("<main id=\"main\" tabindex=\"-1\">"));
    assert!(page.contains("prefers-color-scheme: dark"));
    assert!(page.contains("prefers-reduced-motion: reduce"));
    assert!(page.contains("<nav class=\"toc\" aria-label=\"Table of contents\">"));
    // The document has its own h1, so the template adds none.
    assert_eq!(page.matches("<h1").count(), 1);
}

#[test]
fn templates_from_a_folder_can_extend_builtins() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("mine.html"), "{% extends \"fragment\" %}").expect("write");
    std::fs::write(
        dir.path().join("plain.html"),
        "<title>{{ title }}</title>{{ meta.aliases[0] }}|{{ content }}",
    )
    .expect("write");
    let mut t = Templates::builtin();
    let names = t.load_dir(dir.path()).expect("load");
    assert_eq!(names, ["mine", "plain"]);
    let r = render_both(&fixture("obsidian.md"), Flavor::Obsidian, EmbedMode::Link);
    let page = t
        .render("plain", &r, &PageOptions::default())
        .expect("render");
    assert!(page.starts_with("<title>Reading notes</title>Obsidian sample|<h1"));
    let frag = t
        .render("mine", &r, &PageOptions::default())
        .expect("render");
    assert!(frag.starts_with("<h1 id=\"reading-notes\">"));
}

#[test]
fn print_and_fragment_templates() {
    let r = render_both("Just text.\n", Flavor::Gfm, EmbedMode::Link);
    let t = Templates::builtin();
    let page = t
        .render(
            "print",
            &r,
            &PageOptions {
                fallback_title: Some("notes".into()),
                ..PageOptions::default()
            },
        )
        .expect("render");
    assert!(page.contains("<title>notes</title>"));
    assert!(page.contains("<h1>notes</h1>"));
    assert!(page.contains("@page"));
    let frag = t
        .render("fragment", &r, &PageOptions::default())
        .expect("render");
    assert_eq!(frag, "<p>Just text.</p>\n\n");
}

#[test]
fn title_and_values_are_escaped() {
    let r = render_both(
        "---\ntitle: \"<script>x</script>\"\n---\nBody\n",
        Flavor::Gfm,
        EmbedMode::Link,
    );
    let page = Templates::builtin()
        .render("default", &r, &PageOptions::default())
        .expect("render");
    assert!(page.contains("<title>&lt;script&gt;x&lt;&#x2f;script&gt;</title>"));
}
