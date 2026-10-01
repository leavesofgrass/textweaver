//! Math through `textweaver-math` in both engines: LaTeX events, ASCIIMath
//! fences and (opt-in) code spans, the fallback for formulas that need
//! repair, and sanitization keeping what screen readers use.

use textweaver_render::{Engine, RenderOptions, render};

fn both(src: &str, opts: &RenderOptions) -> String {
    let pulldown = render(src, opts);
    let comrak = render(
        src,
        &RenderOptions {
            engine: Engine::Comrak,
            ..opts.clone()
        },
    );
    assert_eq!(pulldown.html, comrak.html, "engines differ for {src:?}");
    assert_eq!(pulldown.has_math, comrak.has_math);
    pulldown.html
}

#[test]
fn latex_inline_and_display() {
    let html = both(
        "Area $\\pi r^2$ and\n\n$$\\frac{a}{b}$$\n",
        &RenderOptions::default(),
    );
    assert!(
        html.contains(
            "<math xmlns=\"http://www.w3.org/1998/Math/MathML\" alttext=\"\\pi r^2\"><semantics>"
        ),
        "{html}"
    );
    assert!(html.contains("<mi>π</mi>"), "{html}");
    assert!(
        html.contains("display=\"block\" alttext=\"\\frac{a}{b}\""),
        "{html}"
    );
    assert!(
        html.contains("<mfrac><mi>a</mi><mi>b</mi></mfrac>"),
        "{html}"
    );
    assert!(render("$x$", &RenderOptions::default()).has_math);
}

#[test]
fn prices_are_not_math() {
    let html = both("It costs $5 and $10.\n", &RenderOptions::default());
    assert_eq!(html, "<p>It costs $5 and $10.</p>\n");
}

#[test]
fn asciimath_fences_and_code_spans() {
    let src = "```asciimath\nsum_(i=1)^n i\n```\n\nInline `x^2` here.\n";
    let html = both(src, &RenderOptions::default());
    assert!(
        html.starts_with("<p><math xmlns=\"http://www.w3.org/1998/Math/MathML\" display=\"block\""),
        "{html}"
    );
    assert!(html.contains("text/x-asciimath"), "{html}");
    assert!(html.contains("<munderover>"), "{html}");
    // Code spans stay code unless asked.
    assert!(html.contains("<code>x^2</code>"), "{html}");
    let on = RenderOptions {
        asciimath: true,
        ..RenderOptions::default()
    };
    let html = both(src, &on);
    assert!(!html.contains("<code>"), "{html}");
    assert!(
        html.contains("alttext=\"x^2\"><semantics><msup><mi>x</mi><mn>2</mn></msup>"),
        "{html}"
    );
    // Other fences are untouched; math off leaves the fence as code.
    let other = both("```python\nx^2\n```\n", &on);
    assert!(
        other.contains("<code class=\"language-python\">"),
        "{other}"
    );
    let off = RenderOptions { math: false, ..on };
    let html = both(src, &off);
    assert!(
        html.contains("<code class=\"language-asciimath\">"),
        "{html}"
    );
    assert!(html.contains("<code>x^2</code>"), "{html}");
    assert!(!html.contains("<math"), "{html}");
}

#[test]
fn formulas_needing_repair_show_their_source() {
    // (The engines disagree on whether `$\frac{a$` is math at all, so
    // unbalanced braces are tested in the unit tests.)
    let html = both("Bad $\\nosuch{x}$.\n", &RenderOptions::default());
    assert!(
        html.contains("<code class=\"math-error\">\\nosuch{x}</code>"),
        "{html}"
    );
}

#[test]
fn sanitizing_keeps_mathml_and_alttext() {
    let opts = RenderOptions {
        sanitize: true,
        ..RenderOptions::default()
    };
    let html = render("$\\left(x\\right)$ <script>x</script>\n", &opts).html;
    assert!(html.contains("alttext=\"\\left(x\\right)\""), "{html}");
    assert!(html.contains("form=\"prefix\""), "{html}");
    assert!(
        html.contains("<annotation encoding=\"application/x-tex\">"),
        "{html}"
    );
    assert!(!html.contains("<script"), "{html}");
}
