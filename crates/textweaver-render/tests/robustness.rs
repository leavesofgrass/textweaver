//! Rendering never panics and never lets raw markup through the sanitizer,
//! whatever the input: random mixtures of every construct the flavors
//! treat specially, for both engines and all flavors.

use proptest::prelude::*;
use textweaver_render::{Engine, Flavor, RenderOptions, render};

/// Fragments that exercise the extension code paths and their edges.
const PIECES: &[&str] = &[
    "# ",
    "## ",
    "\n",
    "\n\n",
    "> ",
    "> [!note] ",
    "> [!faq]- ",
    "- ",
    "1. ",
    "- [ ] ",
    "| a | b |\n|---|---|\n",
    "```\n",
    "~~~",
    ":::",
    "::: {.x #y}",
    "{#id .c}",
    "[[",
    "]]",
    "![[",
    "|",
    "#",
    "#tag",
    "^id",
    "==",
    "@",
    "[@key]",
    "[span]{.c}",
    "$",
    "$$",
    "\\frac{",
    "}",
    "^",
    "~",
    "~~",
    "**",
    "*",
    "_",
    "`",
    "[^1]",
    "[^1]: ",
    "<div>",
    "</div>",
    "<script>",
    "&amp;",
    "www.x.org",
    "https://a.b/c)",
    "---\n",
    "title: x\n",
    "% T\n",
    "é",
    "漢字",
    "\u{200b}",
    " ",
    "\t",
    "text",
    ":",
    "[",
    "]",
    "(",
    ")",
];

fn doc() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(PIECES), 0..60).prop_map(|v| v.concat())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn never_panics(src in doc()) {
        for engine in [Engine::PulldownCmark, Engine::Comrak] {
            for flavor in [Flavor::CommonMark, Flavor::Gfm, Flavor::Obsidian, Flavor::Pandoc] {
                let opts = RenderOptions { engine, flavor, ..RenderOptions::default() };
                let out = render(&src, &opts);
                // Every heading id is unique.
                let mut ids: Vec<&str> = out.toc.iter().map(|e| e.id.as_str()).collect();
                ids.sort_unstable();
                let before = ids.len();
                ids.dedup();
                prop_assert_eq!(before, ids.len());
            }
        }
    }

    #[test]
    fn sanitized_output_has_no_script(src in doc()) {
        let opts = RenderOptions { sanitize: true, flavor: Flavor::Obsidian, ..RenderOptions::default() };
        let out = render(&format!("{src}<script>alert(1)</script>"), &opts);
        prop_assert!(!out.html.contains("<script"));
    }
}
