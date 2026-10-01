//! Property tests: parsing never panics on arbitrary input, spoken maps
//! always satisfy the offset-map invariants, MathML always balances, and
//! `speak_text` keeps literal text exact.

use proptest::prelude::*;
use textweaver_core::{SpanKind, Verbosity};
use textweaver_math::{
    AltText, DetectOptions, MathMlOptions, Navigator, SpeechOptions, TextOptions, find_math,
    parse_asciimath, parse_latex, speak, speak_text, to_mathml,
};

/// Math-flavoured strings: commands, braces, scripts, brackets, and a few
/// non-ASCII characters.
fn mathish() -> impl Strategy<Value = String> {
    let atoms = prop_oneof![
        Just(r"\frac".to_owned()),
        Just(r"\sqrt".to_owned()),
        Just(r"\left(".to_owned()),
        Just(r"\right)".to_owned()),
        Just(r"\begin{pmatrix}".to_owned()),
        Just(r"\end{pmatrix}".to_owned()),
        Just(r"\begin{cases}".to_owned()),
        Just(r"\end{cases}".to_owned()),
        Just(r"\sum".to_owned()),
        Just(r"\alpha".to_owned()),
        Just(r"\text{".to_owned()),
        Just(r"\hat".to_owned()),
        Just(r"\not".to_owned()),
        Just(r"\over".to_owned()),
        Just(r"\\".to_owned()),
        Just("sqrt".to_owned()),
        Just("sum_".to_owned()),
        Just("abs(".to_owned()),
        Just("[[".to_owned()),
        Just("],[".to_owned()),
        Just("{:".to_owned()),
        Just(":}".to_owned()),
        Just("->".to_owned()),
        Just("\"".to_owned()),
        proptest::string::string_regex(r"[a-z0-9{}\[\]()^_&|,.'/+\-=<> $\\é×≤]{1,3}")
            .expect("valid regex"),
    ];
    proptest::collection::vec(atoms, 0..16).prop_map(|v| v.concat())
}

fn check_all(src: &str) -> Result<(), TestCaseError> {
    for math in [parse_latex(src), parse_asciimath(src)] {
        let len = src.chars().count();
        for v in [Verbosity::Low, Verbosity::Normal, Verbosity::High] {
            let s = speak(&math, &SpeechOptions::new(v));
            prop_assert!(
                s.map.check_invariants(&s.text).is_ok(),
                "{src:?} {v:?}: {:?}\n{}\n{:?}",
                s.map.check_invariants(&s.text),
                s.text,
                s.map
            );
            if let Some(ext) = s.map.source_extent() {
                prop_assert!(ext.end.0 <= len, "{src:?}: extent past the source");
            }
        }
        let xml = to_mathml(
            &math,
            &MathMlOptions {
                display: true,
                alttext: AltText::Speech(Verbosity::Normal),
                annotation: true,
            },
        );
        prop_assert!(balanced(&xml), "{src:?}: unbalanced {xml}");
        // Walk the first two levels of navigation.
        let mut nav = Navigator::new(&math, SpeechOptions::default());
        if nav.enter().is_some() {
            while nav.next_part().is_some() {
                if nav.enter().is_some() {
                    while nav.next_part().is_some() {}
                    nav.exit();
                }
            }
        }
    }
    Ok(())
}

fn balanced(xml: &str) -> bool {
    let mut stack: Vec<&str> = Vec::new();
    let mut rest = xml;
    while let Some(lt) = rest.find('<') {
        let Some(gt) = rest[lt..].find('>').map(|g| lt + g) else {
            return false;
        };
        let tag = &rest[lt + 1..gt];
        if let Some(name) = tag.strip_prefix('/') {
            if stack.pop() != Some(name) {
                return false;
            }
        } else {
            stack.push(tag.split_whitespace().next().unwrap_or_default());
        }
        rest = &rest[gt + 1..];
    }
    stack.is_empty()
}

proptest! {
    // 512 cases by default; `PROPTEST_CASES` raises it for a longer run.
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(512),
        ..ProptestConfig::default()
    })]

    #[test]
    fn math_input_never_breaks(src in mathish()) {
        check_all(&src)?;
    }

    #[test]
    fn arbitrary_input_never_breaks(src in any::<String>()) {
        check_all(&src)?;
    }

    #[test]
    fn speak_text_keeps_text_exact(
        words in proptest::collection::vec(
            prop_oneof![
                Just("$x^2$".to_owned()),
                Just("$5".to_owned()),
                Just("$".to_owned()),
                Just(r"\(".to_owned()),
                Just(r"\)".to_owned()),
                Just("$$".to_owned()),
                Just("`".to_owned()),
                Just("\n\n".to_owned()),
                mathish(),
                proptest::string::string_regex("[a-zé ]{0,6}").expect("valid regex"),
            ],
            0..10,
        ),
        tick in any::<bool>(),
    ) {
        let text = words.concat();
        let opts = TextOptions {
            detect: DetectOptions {
                asciimath: tick.then_some('`'),
                ..DetectOptions::default()
            },
            speech: SpeechOptions::default(),
        };
        let (out, map) = speak_text(&text, &opts);
        prop_assert!(map.check_invariants(&out).is_ok(), "{text:?}: {:?}", map.check_invariants(&out));
        let chars: Vec<char> = text.chars().collect();
        for s in map.spans().iter().filter(|s| s.kind == SpanKind::Literal) {
            let spoken = &out[s.spoken.start as usize..s.spoken.end as usize];
            let original: String = chars[s.source.to_range()].iter().collect();
            prop_assert_eq!(spoken, original);
        }
        // Regions are ordered, disjoint, and inside the text.
        let regions = find_math(&text, &opts.detect);
        let mut end = 0;
        for r in regions {
            prop_assert!(r.range.start.0 >= end);
            prop_assert!(r.range.contains_range(r.content));
            prop_assert!(r.range.end.0 <= chars.len());
            end = r.range.end.0;
        }
    }
}
