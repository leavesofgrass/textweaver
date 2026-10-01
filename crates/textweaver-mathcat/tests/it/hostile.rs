//! Hostile MathML: every input is answered (words or an error), nothing
//! panics or hangs, and the thread still speaks the next formula.

use textweaver_mathcat::{Error, MAX_MATHML_BYTES, Options, speak_mathml};

fn still_speaks() {
    assert_eq!(
        speak_mathml(
            "<math><mfrac><mi>a</mi><mi>b</mi></mfrac></math>",
            &Options::default()
        )
        .as_deref(),
        Ok("eigh over b")
    );
}

#[test]
fn hostile_inputs_are_answered() {
    let deep = format!(
        "<math>{}<mi>x</mi>{}</math>",
        "<mrow>".repeat(10_000),
        "</mrow>".repeat(10_000)
    );
    let deep_fracs = format!(
        "<math>{}<mn>1</mn>{}</math>",
        "<mfrac><mn>1</mn>".repeat(5_000),
        "</mfrac>".repeat(5_000)
    );
    let huge = format!(
        "<math><mtext>{}</mtext></math>",
        "x".repeat(MAX_MATHML_BYTES + 1)
    );
    let wide = format!(
        "<math><mrow>{}</mrow></math>",
        "<mi>x</mi><mo>+</mo>".repeat(2_000)
    );
    let long_but_allowed = format!(
        "<math><mrow>{}<mi>x</mi></mrow></math>",
        "<mi>x</mi><mo>+</mo>".repeat(200)
    );
    let laughs = concat!(
        r#"<?xml version="1.0"?><!DOCTYPE lolz [<!ENTITY lol "lol">"#,
        r#"<!ENTITY lol2 "&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;">]>"#,
        "<math><mi>&lol2;</mi></math>"
    );
    let inputs: Vec<(&str, String)> =
        vec![
        ("empty", String::new()),
        ("not xml", "<<<>>>&&&".into()),
        ("unclosed", "<math><mfrac><mi>a</mi>".into()),
        ("mismatched", "<math><mi>a</mo></math>".into()),
        ("unknown entity", "<math><mi>&bogus;</mi></math>".into()),
        ("not math", "<html><body>hi</body></html>".into()),
        ("empty math", "<math></math>".into()),
        ("empty fraction", "<math><mfrac></mfrac></math>".into()),
        (
            "too many children",
            "<math><mfrac><mi>a</mi><mi>b</mi><mi>c</mi></mfrac></math>".into(),
        ),
        ("script", "<math><script>alert(1)</script></math>".into()),
        ("control chars", "<math><mi>\u{0}\u{1b}[2J</mi></math>".into()),
        ("billion laughs", laughs.into()),
        ("deep rows", deep),
        ("deep fractions", deep_fracs),
        ("huge", huge),
        ("wide", wide),
        ("long but allowed", long_but_allowed),
        (
            "bad attributes",
            r#"<math display="\u{202e}" mathvariant="&lt;"><mi mathsize="-1e308">x</mi></math>"#
                .into(),
        ),
        (
            "strange tokens",
            "<math><mn>1e99999999</mn><mo>\u{fffd}</mo><mi>\u{10ffff}</mi></math>".into(),
        ),
    ];
    for (name, mathml) in inputs {
        let result = speak_mathml(&mathml, &Options::default());
        assert!(
            !matches!(result, Err(Error::Timeout | Error::Unavailable)),
            "{name}: {result:?}"
        );
        if let Ok(words) = &result {
            assert!(!words.chars().any(|c| c.is_control()), "{name}: {words:?}");
        }
        still_speaks();
    }
}

#[test]
fn oversized_and_deep_input_is_refused() {
    let huge = format!("<math><mi>{}</mi></math>", "x".repeat(MAX_MATHML_BYTES));
    assert!(matches!(
        speak_mathml(&huge, &Options::default()),
        Err(Error::Rejected(_))
    ));
    let deep = format!(
        "<math>{}{}</math>",
        "<mrow>".repeat(500),
        "</mrow>".repeat(500)
    );
    assert!(matches!(
        speak_mathml(&deep, &Options::default()),
        Err(Error::Rejected(_))
    ));
    still_speaks();
}
