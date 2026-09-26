//! MathML output: snapshots of representative expressions in both
//! notations, plus structural checks.

use textweaver_core::Verbosity;
use textweaver_math::{
    AltText, MathMlOptions, asciimath_to_mathml, latex_to_mathml, parse_latex, to_mathml,
};

/// Checks that tags balance and returns the MathML unchanged.
fn balanced(xml: &str) -> &str {
    let mut stack: Vec<String> = Vec::new();
    let mut rest = xml;
    while let Some(lt) = rest.find('<') {
        let gt = rest[lt..].find('>').map(|g| lt + g).expect("unclosed tag");
        let tag = &rest[lt + 1..gt];
        if let Some(name) = tag.strip_prefix('/') {
            assert_eq!(stack.pop().as_deref(), Some(name), "in {xml}");
        } else {
            let name = tag.split_whitespace().next().unwrap_or_default();
            stack.push(name.to_owned());
        }
        rest = &rest[gt + 1..];
    }
    assert!(stack.is_empty(), "unclosed {stack:?} in {xml}");
    xml
}

const LATEX: &[(&str, &str, bool)] = &[
    ("fraction", r"\frac{a}{b}", false),
    (
        "quadratic",
        r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}",
        false,
    ),
    ("sum_display", r"\sum_{i=1}^{n} i^2", true),
    ("sum_inline", r"\sum_{i=1}^{n} i^2", false),
    ("integral", r"\int_0^1 f(x)\,dx", true),
    ("limit", r"\lim_{x \to 0} \frac{\sin x}{x}", false),
    ("root", r"\sqrt[3]{x} + \sqrt{y}", false),
    (
        "matrix",
        r"\begin{pmatrix} 1 & 2 \\ 3 & 4 \end{pmatrix}",
        true,
    ),
    (
        "cases",
        r"|x| = \begin{cases} x & \text{if } x \ge 0 \\ -x & \text{otherwise} \end{cases}",
        true,
    ),
    ("accents", r"\hat{x} + \overline{AB} + \vec{v}", false),
    ("fonts", r"\mathbf{v} \in \mathbb{R}^n", false),
    ("binomial", r"\binom{n}{k}", false),
    ("left_right", r"\left[ \frac{a}{b} \right]", false),
    ("primes_and_escapes", r"f'(x) < 100\% \& a_{i,j}", false),
    ("error", r"\unknown{x}", false),
];

const ASCIIMATH: &[(&str, &str, bool)] = &[
    ("am_sum", "sum_(i=1)^n i^3=((n(n+1))/2)^2", true),
    ("am_matrix", "[[a,b],[c,d]]", false),
    ("am_functions", "sin^-1(x) + abs(x) + sqrt(x+1)", false),
    ("am_text", "x \"if\" x > 0", false),
];

#[test]
fn latex_snapshots() {
    for (name, src, display) in LATEX {
        let xml = latex_to_mathml(src, *display);
        insta::assert_snapshot!(*name, balanced(&xml));
    }
}

#[test]
fn asciimath_snapshots() {
    for (name, src, display) in ASCIIMATH {
        let xml = asciimath_to_mathml(src, *display);
        insta::assert_snapshot!(*name, balanced(&xml));
    }
}

#[test]
fn alttext_options() {
    let m = parse_latex(r"x^2 < y");
    let plain = to_mathml(
        &m,
        &MathMlOptions {
            display: false,
            alttext: AltText::None,
            annotation: false,
        },
    );
    assert_eq!(
        plain,
        "<math xmlns=\"http://www.w3.org/1998/Math/MathML\"><mrow><msup><mi>x</mi><mn>2</mn></msup><mo>&lt;</mo><mi>y</mi></mrow></math>"
    );
    let spoken = to_mathml(
        &m,
        &MathMlOptions {
            display: true,
            alttext: AltText::Speech(Verbosity::Normal),
            annotation: false,
        },
    );
    assert!(spoken.contains("display=\"block\""));
    assert!(spoken.contains("alttext=\"x squared less than y\""));
    let source = latex_to_mathml(r"a<b", false);
    assert!(source.contains("alttext=\"a&lt;b\""));
    assert!(source.contains("<annotation encoding=\"application/x-tex\">a&lt;b</annotation>"));
}
