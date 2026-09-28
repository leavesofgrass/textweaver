//! Unit tests: the MathML checks, the language and verbosity mappings, and
//! the thread surviving a panic.

use super::*;

#[test]
fn languages_map_to_mathcat_or_english() {
    assert_eq!(language_for(None), "en");
    assert_eq!(language_for(Some("")), "en");
    assert_eq!(language_for(Some("fr-CA")), "fr");
    assert_eq!(language_for(Some("es_MX")), "es");
    assert_eq!(language_for(Some("DE")), "de");
    assert_eq!(language_for(Some("no")), "nb");
    assert_eq!(language_for(Some("pt-BR")), "en");
    assert_eq!(language_for(Some("x-klingon")), "en");
}

#[test]
fn verbosity_maps_to_three_levels() {
    assert_eq!(mathcat_verbosity(Verbosity::Low), "Terse");
    assert_eq!(mathcat_verbosity(Verbosity::Normal), "Medium");
    assert_eq!(mathcat_verbosity(Verbosity::High), "Verbose");
}

#[test]
fn oversized_and_deep_mathml_is_refused_before_mathcat() {
    let deep = format!("<math>{}{}</math>", "<mrow>".repeat(200), "</mrow>".repeat(200));
    assert!(matches!(check_mathml(&deep), Err(Error::Rejected(_))));
    let flat = format!("<math>{}</math>", "<mi>x</mi>".repeat(MAX_MATHML_ELEMENTS - 1));
    assert_eq!(check_mathml(&flat), Ok(()));
    let long = format!("<math>{}</math>", "<mi>x</mi>".repeat(MAX_MATHML_ELEMENTS));
    assert!(matches!(check_mathml(&long), Err(Error::Rejected(_))));
    let big = format!("<math><mtext>{}</mtext></math>", "a".repeat(MAX_MATHML_BYTES));
    assert!(matches!(check_mathml(&big), Err(Error::Rejected(_))));
    // Self-closing elements, comments, and declarations add no depth.
    let empties = format!("<math>{}<!-- c --><?pi?></math>", "<mspace/>".repeat(500));
    assert_eq!(check_mathml(&empties), Ok(()));
    // Unterminated tags end the scan without a panic.
    assert!(check_mathml("<math><mi").is_ok());
    assert!(check_mathml("<").is_ok());
}

#[test]
fn pauses_are_removed_but_decimal_commas_stay() {
    assert_eq!(
        engine::without_pauses("x is equal to; 3,5 times y, end;"),
        "x is equal to 3,5 times y end"
    );
}

/// A panic inside the engine is answered with `Crashed`; the thread keeps
/// serving and the next expression is spoken.
#[test]
fn the_thread_survives_a_panic() {
    assert_eq!(engine::panic_for_test(), Err(Error::Crashed));
    assert_eq!(engine::panic_for_test(), Err(Error::Crashed));
    let words = speak_mathml(
        "<math><msup><mi>x</mi><mn>2</mn></msup></math>",
        &Options::default(),
    );
    assert_eq!(words.as_deref(), Ok("x squared"));
}
