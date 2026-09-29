//! Math braille against published examples (ADR-0036).
//!
//! Each formula is written as textweaver writes it (LaTeX parsed by
//! `textweaver-math`, then MathML) and brailled by MathCAT. The expected
//! braille is the published transcription:
//!
//! - Nemeth: "The Nemeth Braille Code for Mathematics and Science
//!   Notation, 1972 Revision" (the "green book"), by rule and example
//!   number, as MathCAT's own test suite records them
//!   (`tests/braille/Nemeth/rules.rs` in MathCAT 0.7.6-rc.3, which also
//!   cites BANA's 2022 edition).
//! - UEB: BANA, "Guidance for Transcription Using the UEB Mathematics
//!   Code" (2019), section 5, as MathCAT's `tests/braille/UEB/iceb.rs`
//!   records them. Those examples assume contracted (grade 2) text around
//!   the math, so they are checked with `grade2`.

use textweaver_math::parse_latex;
use textweaver_mathcat::{BrailleCode, BrailleOptions, braille_mathml, mathml_for};

fn braille(latex: &str, code: BrailleCode, grade2: bool) -> String {
    let math = parse_latex(latex);
    assert!(
        math.diagnostics.is_empty(),
        "{latex}: {:?}",
        math.diagnostics
    );
    braille_mathml(&mathml_for(&math, false), &BrailleOptions { code, grade2 })
        .unwrap_or_else(|e| panic!("{latex}: {e}"))
}

#[test]
fn nemeth_matches_published_examples() {
    let cases = [
        // Rule 62.a, example 3: a simple fraction.
        ("\\frac{a+b}{c}", "⠹⠁⠬⠃⠌⠉⠼"),
        // Rule 79.g, example 2: superscripts and a comparison sign, which
        // is spaced.
        ("2^x < 3^x", "⠼⠆⠘⠭⠀⠐⠅⠀⠼⠒⠘⠭"),
        // Rule 80.a, example 1: a radical with superscripts inside, the
        // baseline indicator before the plus sign.
        ("\\sqrt{x^2+y^2}", "⠜⠭⠘⠆⠐⠬⠽⠘⠆⠐⠻"),
    ];
    for (latex, published) in cases {
        assert_eq!(
            braille(latex, BrailleCode::Nemeth, false),
            published,
            "{latex}"
        );
    }
}

#[test]
fn ueb_matches_published_examples() {
    let cases = [
        // Section 5, example 1.
        ("x+y=6", "⠭⠐⠖⠽⠀⠐⠶⠀⠼⠋"),
        // Section 5, example 2: grade 1 indicators before the superscript
        // and the capital, in contracted text.
        ("x^2+y^2=C", "⠭⠰⠔⠼⠃⠐⠖⠽⠔⠼⠃⠀⠐⠶⠀⠰⠠⠉"),
        // Section 5, example 3: a grade 1 passage for two fractions.
        ("\\frac{a}{b}+\\frac{c}{d}", "⠰⠰⠷⠁⠨⠌⠃⠾⠐⠖⠷⠉⠨⠌⠙⠾"),
    ];
    for (latex, published) in cases {
        assert_eq!(braille(latex, BrailleCode::Ueb, true), published, "{latex}");
    }
}

#[test]
fn uncontracted_ueb_needs_no_grade_1_indicators() {
    // The same formula as section 5, example 2, in uncontracted text: no
    // letter sequence can be read as a contraction, so no grade 1
    // indicator is needed.
    let b = braille("x^2+y^2=C", BrailleCode::Ueb, false);
    assert!(!b.contains('⠰'), "{b}");
    assert!(b.contains("⠐⠖"), "{b}");
}

#[test]
fn braille_is_six_dot_with_blank_cells_for_spaces() {
    for code in [BrailleCode::Nemeth, BrailleCode::Ueb] {
        let b = braille("\\frac{-b \\pm \\sqrt{b^2-4ac}}{2a}", code, false);
        assert!(!b.is_empty());
        assert!(
            b.chars().all(|c| ('\u{2800}'..='\u{283F}').contains(&c)),
            "{code}: {b}"
        );
        assert!(!b.starts_with('\u{2800}') && !b.ends_with('\u{2800}'));
    }
}
