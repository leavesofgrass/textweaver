//! Math navigation: moving by term, into fractions, scripts, roots, limits,
//! and tables, and back out.

use textweaver_core::Verbosity;
use textweaver_math::{
    Math, NavStep, Navigator, Role, SpeechOptions, parse_asciimath, parse_latex,
};

fn nav(m: &Math) -> Navigator<'_> {
    Navigator::new(m, SpeechOptions::new(Verbosity::Normal))
}

fn text(m: &Math, step: &NavStep) -> String {
    step.spoken.map.check_invariants(&step.spoken.text).unwrap();
    m.source_text(step.span)
}

#[test]
fn fraction_parts() {
    let m = parse_latex(r"\frac{a+b}{c}");
    let mut n = nav(&m);
    let whole = n.current();
    assert_eq!(whole.role, Role::Expression);
    assert_eq!(
        whole.spoken.text,
        "the fraction with numerator a plus b and denominator c"
    );
    assert!(whole.has_children);

    let num = n.enter().unwrap();
    assert_eq!(num.role, Role::Numerator);
    assert_eq!(num.announcement(), "numerator, a plus b");
    assert_eq!(text(&m, &num), "{a+b}");
    assert_eq!(num.depth, 1);

    let den = n.next_part().unwrap();
    assert_eq!(den.role, Role::Denominator);
    assert_eq!(den.announcement(), "denominator, c");
    assert_eq!(text(&m, &den), "c");
    assert!(n.next_part().is_none(), "no part after the denominator");
    assert_eq!(
        n.current().role,
        Role::Denominator,
        "boundary keeps position"
    );

    assert_eq!(n.previous_part().unwrap().role, Role::Numerator);
    // Into the numerator's terms.
    let a = n.enter().unwrap();
    assert_eq!((a.role, a.spoken.text.as_str()), (Role::Term, "a"));
    assert_eq!(n.next_part().unwrap().spoken.text, "plus");
    assert_eq!(n.last_part().unwrap().spoken.text, "b");
    assert_eq!(n.first_part().unwrap().spoken.text, "a");
    assert!(n.enter().is_none(), "a letter has no parts");

    assert_eq!(n.exit().unwrap().role, Role::Numerator);
    assert_eq!(n.exit().unwrap().role, Role::Expression);
    assert!(n.exit().is_none());
    assert_eq!(n.enter_role(Role::Denominator).unwrap().spoken.text, "c");
}

#[test]
fn terms_and_scripts() {
    let m = parse_latex("x^2 + 1 = y_i");
    let mut n = nav(&m);
    let first = n.enter().unwrap();
    assert_eq!(first.role, Role::Term);
    assert_eq!(first.spoken.text, "x squared");
    let base = n.enter().unwrap();
    assert_eq!(base.role, Role::Base);
    assert_eq!(n.next_part().unwrap().announcement(), "superscript, 2");
    n.exit();
    let terms: Vec<String> = std::iter::from_fn(|| n.next_part().map(|s| s.spoken.text)).collect();
    assert_eq!(terms, vec!["plus", "1", "equals", "y sub i"]);
    assert_eq!(n.enter_role(Role::Subscript).unwrap().spoken.text, "i");
}

#[test]
fn limits_roots_and_brackets() {
    let m = parse_latex(r"\sum_{i=1}^{n} \sqrt[3]{i}");
    let mut n = nav(&m);
    let sum = n.enter().unwrap();
    assert_eq!(sum.spoken.text, "the sum from i equals 1 to n");
    assert_eq!(
        n.enter_role(Role::LowerLimit).unwrap().announcement(),
        "lower limit, i equals 1"
    );
    assert_eq!(n.next_part().unwrap().announcement(), "upper limit, n");
    n.exit();
    let root = n.next_part().unwrap();
    assert_eq!(root.spoken.text, "cube root of i");
    assert_eq!(n.enter().unwrap().announcement(), "index, 3");
    assert_eq!(n.next_part().unwrap().announcement(), "radicand, i");

    let m = parse_asciimath("abs(x-1)");
    let mut n = nav(&m);
    assert_eq!(n.current().spoken.text, "the absolute value of x minus 1");
    let terms: Vec<String> = n
        .enter()
        .into_iter()
        .chain(std::iter::from_fn(|| n.next_part()))
        .map(|s| s.spoken.text)
        .collect();
    assert_eq!(terms, vec!["x", "minus", "1"]);
}

#[test]
fn table_cells() {
    let m = parse_latex(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}");
    let mut n = nav(&m);
    let a = n.enter().unwrap();
    assert_eq!(a.announcement(), "row 1, column 1, a");
    let cells: Vec<String> =
        std::iter::from_fn(|| n.next_part().map(|s| s.announcement())).collect();
    assert_eq!(
        cells,
        vec![
            "row 1, column 2, b",
            "row 2, column 1, c",
            "row 2, column 2, d"
        ]
    );
}

#[test]
fn spans_lie_inside_the_parent() {
    let m = parse_latex(r"\frac{\sqrt{x^2+1}}{2} + \left(\sum_{k} a_k\right)");
    let mut n = nav(&m);
    // Depth-first walk over every part.
    fn walk(n: &mut Navigator<'_>, parent: textweaver_core::CharRange, count: &mut usize) {
        if n.enter().is_none() {
            return;
        }
        loop {
            let cur = n.current();
            *count += 1;
            assert!(
                parent.contains_range(cur.span),
                "{:?} outside {parent}",
                cur
            );
            walk(n, cur.span, count);
            if n.next_part().is_none() {
                break;
            }
        }
        n.exit();
    }
    let root = n.current().span;
    let mut count = 0;
    walk(&mut n, root, &mut count);
    assert!(count > 10, "{count}");
}
