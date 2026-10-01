//! Navigation and its braille, through the vendored MathCAT (ADR-0036).
//!
//! The first test is the regression test for MathCAT issue #827: in builds
//! without unsafe code (this workspace's), `GetNavigationBraille` panicked
//! with "index out of bounds" whenever the part reached was not the whole
//! expression, because the copy of that part was made in the original
//! document and appended to a new one. MathCAT's guard turned the panic
//! into a "MathCAT crash" error, which this crate reports as `Crashed`.

use textweaver_mathcat::{
    BrailleCode, BrailleOptions, Error, NavMove, Options, navigate, navigate_start, speak_mathml,
};

/// (a + b) / 2, as textweaver writes it.
const FRACTION: &str =
    "<math><mfrac><mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow><mn>2</mn></mfrac></math>";

/// MathCAT has one place per process, so the tests take turns.
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn turn() -> std::sync::MutexGuard<'static, ()> {
    TURN.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn nemeth() -> BrailleOptions {
    BrailleOptions::default()
}

/// Issue #827: the braille of a part inside the expression.
#[test]
fn navigation_braille_inside_an_expression_does_not_panic() {
    let _turn = turn();
    let o = Options::default();
    let whole = navigate_start(FRACTION, NavMove::Current, &o, &nemeth()).expect("start");
    assert!(!whole.braille.is_empty(), "{whole:?}");
    // Into the fraction: the numerator is a part of the expression, so its
    // braille is made from a copy in a new document (the code #827 fixed).
    let inside = navigate(NavMove::ZoomIn, &o, &nemeth());
    assert_ne!(inside, Err(Error::Crashed), "issue #827 is back");
    let inside = inside.expect("zoom in");
    assert!(inside.moved, "{inside:?}");
    assert!(!inside.braille.is_empty(), "{inside:?}");
    assert_ne!(inside.braille, whole.braille);
    // Every part of it, both codes.
    for code in [BrailleCode::Nemeth, BrailleCode::Ueb] {
        let b = BrailleOptions {
            code,
            grade2: false,
        };
        navigate_start(FRACTION, NavMove::Current, &o, &b).expect("start");
        for step in [
            NavMove::ZoomIn,
            NavMove::ZoomIn,
            NavMove::Next,
            NavMove::Next,
            NavMove::ZoomOut,
            NavMove::Next,
            NavMove::End,
            NavMove::Start,
        ] {
            let s = navigate(step, &o, &b);
            assert!(s.is_ok(), "{code} {step:?}: {s:?}");
        }
    }
}

#[test]
fn steps_speak_and_say_when_nothing_moves() {
    let _turn = turn();
    let o = Options::default();
    let b = nemeth();
    let whole = navigate_start(FRACTION, NavMove::Current, &o, &b).expect("start");
    assert!(whole.speech.contains("plus b"), "{whole:?}");
    assert!(!whole.moved);
    let num = navigate(NavMove::ZoomIn, &o, &b).expect("in");
    assert!(num.speech.contains("plus b"), "{num:?}");
    let den = navigate(NavMove::Next, &o, &b).expect("next");
    assert!(den.moved);
    assert!(den.speech.contains('2'), "{den:?}");
    // The Nemeth numeral 2 (numeric indicator, then 2 in the lower part of
    // the cell): the published form in the Nemeth Code, rule 9.
    assert_eq!(den.braille, "\u{283C}\u{2806}");
    let edge = navigate(NavMove::Next, &o, &b).expect("edge");
    assert!(!edge.moved, "{edge:?}");
    assert!(!edge.speech.is_empty());
}

#[test]
fn speaking_another_formula_keeps_the_place() {
    let _turn = turn();
    let o = Options::default();
    let b = nemeth();
    navigate_start(FRACTION, NavMove::Current, &o, &b).expect("start");
    navigate(NavMove::ZoomIn, &o, &b).expect("in");
    let den = navigate(NavMove::Next, &o, &b).expect("next");
    // Another expression replaces MathCAT's current one ...
    assert_eq!(
        speak_mathml("<math><msup><mi>x</mi><mn>2</mn></msup></math>", &o).as_deref(),
        Ok("x squared")
    );
    // ... and the navigation finds its place again.
    let again = navigate(NavMove::Current, &o, &b).expect("current");
    assert_eq!(again.braille, den.braille);
    assert!(!again.moved);
}
