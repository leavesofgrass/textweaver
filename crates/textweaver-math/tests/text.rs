//! Math in running text: detection without false positives on prices, and
//! the `speak_text` normalization transform with its offset map.

use textweaver_core::{CharPos, OffsetMap, SpanKind, Verbosity};
use textweaver_math::{
    Delimiter, DetectOptions, Notation, SpeechOptions, TextOptions, find_math, speak_text,
};

fn say(text: &str) -> String {
    say_with(text, &TextOptions::default())
}

fn say_with(text: &str, opts: &TextOptions) -> String {
    let (out, map) = speak_text(text, opts);
    check(text, &out, &map);
    out
}

/// Invariants, literal spans equal to their source, extent inside the text.
fn check(src: &str, out: &str, map: &OffsetMap) {
    map.check_invariants(out)
        .unwrap_or_else(|e| panic!("{src:?}: {e}\n{map:?}"));
    let chars: Vec<char> = src.chars().collect();
    for s in map.spans().iter().filter(|s| s.kind == SpanKind::Literal) {
        let spoken = &out[s.spoken.start as usize..s.spoken.end as usize];
        let original: String = chars[s.source.to_range()].iter().collect();
        assert_eq!(spoken, original, "{src:?}");
    }
    if let Some(ext) = map.source_extent() {
        assert!(ext.end.0 <= chars.len(), "{src:?}");
    }
}

/// Star's delimited math vectors (`tests/test_ttstext.py`).
#[test]
fn star_vectors() {
    assert_eq!(say(r"$\frac{a}{b}$"), "a over b");
    assert_eq!(say(r"$\sqrt{x}$"), "square root of x");
    assert_eq!(say("hello world"), "hello world");
    // Star's undelimited vectors, delimited as they appear in documents.
    assert_eq!(say("$x^2$ and $y^{3}$"), "x squared and y cubed");
    assert_eq!(say("$x_i$ and $x_{ij}$"), "x sub i and x sub i j");
    assert_eq!(say(r"$\alpha + \beta$"), "alpha plus beta");
    assert_eq!(
        say(r"$a \times b \leq c$"),
        "a times b less than or equal to c"
    );
    assert_eq!(say(r"$\bar{x}$"), "x bar");
}

/// Star's bugs Q6, Q7, Q8 and the defects table: prose is never math.
#[test]
fn star_bugs_fixed() {
    for text in [
        "snake_case word",
        "my_var_name is x",
        "[^1]",
        "It costs $5 and $10.",
        "It costs $5.",
        "$5-$10",
        "from $3.99 to $4.50 each",
        r"Pandoc \$3.99",
        "US$5 or US$6",
        "a { brace } and x^2 in prose",
        r"\alphabet soup",
        "price: $ 5 $",
    ] {
        assert_eq!(say(text), text, "{text}");
    }
    // Q7: trailing power.
    assert_eq!(say("area $x^2$"), "area x squared");
    // Q8: no global stripping or collapsing around math.
    assert_eq!(say("  a  $x$  b  "), "  a  x  b  ");
}

#[test]
fn delimiters() {
    assert_eq!(say(r"so \(a+b\) holds"), "so a plus b holds");
    assert_eq!(say(r"\[x^2\]"), "x squared");
    assert_eq!(say("$$\\frac{1}{2}$$"), "1 half");
    assert_eq!(say("The area is $\\pi r^2$."), "The area is pi r squared.");
    // Math touching a word stays a separate word.
    assert_eq!(say("the $n$th term"), "the n th term");
    // A blank line ends a paragraph; no math spans it.
    assert_eq!(say("$a\n\nb$"), "$a\n\nb$");
    // Escaped dollars are text.
    assert_eq!(say(r"\$x\$"), r"\$x\$");
}

#[test]
fn asciimath_delimiter_is_opt_in() {
    assert_eq!(say("`x^2`"), "`x^2`");
    let opts = TextOptions {
        detect: DetectOptions {
            asciimath: Some('`'),
            ..DetectOptions::default()
        },
        speech: SpeechOptions::default(),
    };
    assert_eq!(
        say_with("so `x^2/2` is", &opts),
        "so the fraction with numerator x squared and denominator 2 is"
    );
    // A configurable delimiter.
    let opts = TextOptions {
        detect: DetectOptions {
            asciimath: Some('@'),
            dollars: false,
            brackets: false,
        },
        speech: SpeechOptions::new(Verbosity::High),
    };
    assert_eq!(
        say_with("@a/b@ costs $5$", &opts),
        "the fraction a over b end fraction costs $5$"
    );
}

#[test]
fn regions() {
    let text = r"Let $x$ be $$\sum_i x_i$$ or \(y\) and \[z\].";
    let r = find_math(text, &DetectOptions::default());
    let kinds: Vec<(Delimiter, bool, Notation)> = r
        .iter()
        .map(|m| (m.delimiter, m.display, m.notation))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (Delimiter::Dollar, false, Notation::Latex),
            (Delimiter::DoubleDollar, true, Notation::Latex),
            (Delimiter::Paren, false, Notation::Latex),
            (Delimiter::Bracket, true, Notation::Latex),
        ]
    );
    let chars: Vec<char> = text.chars().collect();
    let content: Vec<String> = r
        .iter()
        .map(|m| chars[m.content.to_range()].iter().collect())
        .collect();
    assert_eq!(content, vec!["x", r"\sum_i x_i", "y", "z"]);
}

#[test]
fn map_points_into_the_document() {
    let text = "Here $\\frac{a}{b}$ ends.";
    let (out, map) = speak_text(text, &TextOptions::default());
    assert_eq!(out, "Here a over b ends.");
    check(text, &out, &map);
    let a = out.find("a over").unwrap_or_default() as u32;
    let r = map.to_source(&out, a..a + 1).unwrap_or_default();
    assert_eq!(r.start, CharPos(12));
    // "ends" is literal and exact.
    let e = out.find("ends").unwrap_or_default() as u32;
    let r = map.to_source(&out, e..e + 4).unwrap_or_default();
    let chars: Vec<char> = text.chars().collect();
    let s: String = chars[r.to_range()].iter().collect();
    assert_eq!(s, "ends");
    // Composes with an identity map as a pipeline step would.
    let id = OffsetMap::identity(text, CharPos::ZERO);
    let composed = OffsetMap::compose(&id, text, &map);
    composed.check_invariants(&out).unwrap();
}

#[test]
fn multibyte_text_around_math() {
    let text = "Größe $\\alpha$ – naïve $x^2$ café";
    assert_eq!(say(text), "Größe alpha – naïve x squared café");
}

#[test]
fn fixture_document() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/o/math-sample.md"
    );
    let text = std::fs::read_to_string(path).unwrap_or_default();
    assert!(!text.is_empty(), "missing {path}");
    for v in [Verbosity::Low, Verbosity::Normal, Verbosity::High] {
        let opts = TextOptions {
            detect: DetectOptions {
                asciimath: Some('`'),
                ..DetectOptions::default()
            },
            speech: SpeechOptions::new(v),
        };
        let out = say_with(&text, &opts);
        assert!(!out.contains("\\frac"), "{out}");
        assert!(out.contains("It costs $5 and $10 to enter."), "{out}");
    }
}

/// Another engine asked first (MathCAT, ADR-0029): its words replace the
/// whole formula as one expanded span; `None` or blanks fall back to this
/// crate's speech; prose around the math stays literal.
#[test]
fn speak_text_with_another_engine() {
    use textweaver_math::speak_text_with;
    let text = "so $x^2$ and $y$ end";
    let opts = TextOptions::default();
    let (out, map) = speak_text_with(text, &opts, &mut |r, math| {
        (math.source == "x^2").then(|| format!("  the {:?}  power ", r.delimiter))
    });
    check(text, &out, &map);
    assert_eq!(out, "so the Dollar power and y end");
    let expanded: Vec<_> = map
        .spans()
        .iter()
        .filter(|s| s.kind == SpanKind::Expanded)
        .collect();
    assert_eq!(expanded.len(), 1);
    assert_eq!(expanded[0].source.to_range(), 4..7);
    // Blank words fall back.
    let (out, map) = speak_text_with(text, &opts, &mut |_, _| Some("   ".into()));
    check(text, &out, &map);
    assert_eq!(out, say(text));
    // A formula touching a word stays a separate word.
    let (out, _) = speak_text_with("a$x$b", &opts, &mut |_, _| Some("ex".into()));
    assert_eq!(out, "a ex b");
}
