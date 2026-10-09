//! The polished page (B1-p3): an exported fixture page carries
//! highlighted code, figures, scrolling tables, footnotes with return
//! links, callouts with their type word first, MathML, and the reader's
//! typography; it passes `tools/check_site_a11y.py`; and every color pair
//! its stylesheet draws passes the theme contrast checker in every
//! built-in theme.

use std::path::Path;
use std::process::Command;

use textweaver_render::{Flavor, PageOptions, RenderOptions, Templates, Typography, render};
use textweaver_theme::ColorRole as C;
use textweaver_theme::Requirement;
use textweaver_theme::check::check_pair;

fn page() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/l/page/polished.md"
    );
    let src = std::fs::read_to_string(path)
        .expect("fixture")
        .replace("\r\n", "\n");
    let doc = render(
        &src,
        &RenderOptions {
            flavor: Flavor::Obsidian,
            ..RenderOptions::default()
        },
    );
    let typography = Typography {
        font_family: "\"Atkinson Hyperlegible\", sans-serif".into(),
        size_pt: 16.0,
        weight: 400,
        line_height: 1.5,
        paragraph_spacing: 1.0,
        letter_spacing: 0.0,
        word_spacing: 0.0,
        measure: 66,
    };
    Templates::builtin()
        .render(
            "default",
            &doc,
            &PageOptions {
                typography: Some(typography),
                ..PageOptions::default()
            },
        )
        .expect("page")
}

#[test]
fn the_page_has_every_polished_part() {
    let html = page();
    for part in [
        // Code: tokens by class; keywords bold, comments italic in CSS.
        "<pre><code class=\"language-rust\">",
        "<span class=\"tok-keyword\">fn</span>",
        "<span class=\"tok-comment\">// Add two numbers.",
        "<pre><code class=\"language-python\">",
        "<pre><code>A block with no language stays plain.",
        ".tok-keyword { color: var(--tw-heading2); font-weight: bold; }",
        // Tables scroll in a named box that takes focus.
        "<div class=\"table-scroll\" role=\"group\" aria-label=\"Table 1\" tabindex=\"0\">",
        "<th>Planet</th>",
        // Figures: the title is the caption, the alt text stays.
        "<figure>",
        "alt=\"A crow on a fence post at dusk\"",
        "<figcaption>A crow keeps watch</figcaption>",
        // Callouts: the type word first.
        "<strong>Tip: Remember</strong>",
        "<strong>Warning</strong>",
        // Footnotes with return links.
        "role=\"doc-backlink\"",
        "aria-label=\"Back to reference 1\"",
        // Math as MathML with its source as the text alternative.
        "<math",
        "alttext=\"\\pi r^2\"",
        // The reader's typography, and the media rules.
        "--tw-type-measure: 66ch;",
        "--tw-type-size: 133.333%;",
        "@media (forced-colors: active)",
        "@media (prefers-reduced-motion: reduce)",
        "@media print",
    ] {
        assert!(html.contains(part), "missing {part:?}");
    }
    assert!(
        !html.contains("title=\"A crow keeps watch\""),
        "the caption is read once"
    );
}

/// Python 3 as `scripts/dev-check.ps1` finds it: the py launcher first
/// (`python` on Windows may be the Microsoft Store stub), then `python3`,
/// then `python`.
fn python() -> Option<Vec<&'static str>> {
    let candidates: [&[&'static str]; 3] = [&["py", "-3"], &["python3"], &["python"]];
    candidates.into_iter().find_map(|c| {
        let out = Command::new(c[0])
            .args(&c[1..])
            .arg("--version")
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr);
        (out.status.success() && text.contains("Python 3")).then(|| c.to_vec())
    })
}

#[test]
fn the_exported_page_passes_the_accessibility_checker() {
    let Some(py) = python() else {
        eprintln!(
            "Skipped: Python 3 is not installed, so tools/check_site_a11y.py cannot check the exported page."
        );
        return;
    };
    let dir = tempfile::tempdir().expect("temp dir");
    let file = dir.path().join("polished.html");
    std::fs::write(&file, page()).expect("write page");
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/check_site_a11y.py");
    let out = Command::new(py[0])
        .args(&py[1..])
        .arg(&script)
        .arg("--page")
        .arg(&file)
        .output()
        .expect("run the checker");
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Every text color `style.css` draws on a fill, with what draws it.
/// Code blocks, callouts, and the contents sit on the page background:
/// on the code background, token and link colors fell short in Galaxy,
/// Galaxy Light, and Gruvbox (B1-p3), so only plain text and code are
/// drawn on it.
const PAGE_PAIRS: &[(C, C, &str)] = &[
    (C::Text, C::Background, "body text"),
    (C::Link, C::Background, "links, callouts, the contents"),
    (C::Code, C::Background, "code blocks"),
    (C::DimText, C::Background, "code comments, captions, quotes"),
    (C::Heading2, C::Background, "code keywords"),
    (C::Quote, C::Background, "code strings"),
    (C::Heading4, C::Background, "code numbers"),
    (C::Heading1, C::Background, "code function names"),
    (C::Heading3, C::Background, "code type names"),
    (C::Error, C::Background, "formulas that could not be read"),
    (C::Code, C::CodeBackground, "inline code"),
    (
        C::Text,
        C::CodeBackground,
        "table headers, tags, the skip link",
    ),
    (C::Text, C::Surface, "alternate table rows and their links"),
];

#[test]
fn every_page_color_pair_passes_the_theme_checker() {
    let mut failures = Vec::new();
    let mut checked = 0;
    // The themes that meet AA themselves, the required ones among them;
    // the others keep star's colors and are labelled as below AA (the
    // owner's policy in textweaver-theme), so their page is too.
    for theme in textweaver_theme::builtin::all() {
        let passes = textweaver_theme::check(theme).passed();
        assert!(
            passes || !textweaver_theme::star::must_meet_aa(theme.name()),
            "{} must meet AA",
            theme.meta.display_name
        );
        if !passes {
            continue;
        }
        for &(fg, bg, what) in PAGE_PAIRS {
            let c = check_pair(
                theme.color(fg),
                theme.color(bg),
                Requirement::Text,
                theme.kind(),
            );
            checked += 1;
            if !c.passed {
                failures.push(format!(
                    "Fail: {}, {} on {} ({what}): {}",
                    theme.meta.display_name,
                    fg.label(),
                    bg.label(),
                    c.describe()
                ));
            }
        }
    }
    assert!(checked > 50, "{checked}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
