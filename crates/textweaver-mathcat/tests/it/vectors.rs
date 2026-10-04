//! MathCAT's words for the star math vectors and a few student formulas,
//! recorded from MathCAT 0.7.6-rc.3 (ADR-0029), with the offset map
//! checked for every one.
//!
//! When MathCAT is upgraded, these strings are expected to change: read
//! the new wording, then record it here.

use textweaver_core::{OffsetMap, SpanKind, Verbosity};
use textweaver_math::TextOptions;
use textweaver_mathcat::{Options, Style, speak_text};

fn options(style: Style, verbosity: Verbosity) -> Options {
    Options {
        style,
        verbosity,
        ..Options::default()
    }
}

fn say_with(text: &str, o: &Options) -> String {
    let (out, map) = speak_text(text, &TextOptions::default(), o);
    check(text, &out, &map);
    out
}

fn say(text: &str) -> String {
    say_with(text, &Options::default())
}

/// The map's invariants; literal spans equal their source; each formula
/// is one expanded span.
fn check(src: &str, out: &str, map: &OffsetMap) {
    map.check_invariants(out)
        .unwrap_or_else(|e| panic!("{src:?}: {e}\n{map:?}"));
    let chars: Vec<char> = src.chars().collect();
    for s in map.spans() {
        let spoken = &out[s.spoken.start as usize..s.spoken.end as usize];
        let original: String = chars[s.source.to_range()].iter().collect();
        match s.kind {
            SpanKind::Literal => assert_eq!(spoken, original, "{src:?}"),
            SpanKind::Expanded => assert!(!spoken.trim().is_empty(), "{src:?}"),
            _ => {}
        }
    }
}

/// star's math vectors (`tests/test_ttstext.py`), delimited as in
/// documents, in ClearSpeak at normal verbosity.
#[test]
fn star_vectors() {
    assert_eq!(say(r"$\frac{a}{b}$"), "eigh over b");
    assert_eq!(say(r"$\sqrt{x}$"), "the square root of x");
    assert_eq!(say("hello world"), "hello world");
    assert_eq!(say("$x^2$ and $y^{3}$"), "x squared and y cubed");
    assert_eq!(say("$x_i$ and $x_{ij}$"), "x sub i and x sub i j");
    assert_eq!(say(r"$\alpha + \beta$"), "alpha plus beta");
    assert_eq!(
        say(r"$a \times b \leq c$"),
        "eigh times b, is less than or equal to c"
    );
    assert_eq!(say(r"$\bar{x}$"), "x bar");
}

/// Prose is never math, as with the built-in engine.
#[test]
fn prose_is_untouched() {
    for text in [
        "snake_case word",
        "It costs $5 and $10.",
        "from $3.99 to $4.50 each",
        r"Pandoc \$3.99",
    ] {
        assert_eq!(say(text), text, "{text}");
    }
    assert_eq!(say("area $x^2$ here"), "area x squared here");
}

const QUADRATIC: &str = r"$x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$";

/// The quadratic formula in both styles at the three verbosities.
#[test]
fn quadratic_formula_by_style_and_verbosity() {
    use Style::{ClearSpeak, SimpleSpeak};
    use Verbosity::{High, Low, Normal};
    let expect = [
        (
            ClearSpeak,
            Low,
            "x equals; the fraction with numerator; negative b plus or minus; square root, b squared minus 4 eigh c; and denominator 2 eigh",
        ),
        (
            ClearSpeak,
            Normal,
            "x is equal to; the fraction with numerator; negative b plus or minus; the square root of b squared minus 4 eigh c; and denominator 2 eigh",
        ),
        (
            ClearSpeak,
            High,
            "x is equal to; the fraction with numerator; negative b plus or minus; the square root of b squared minus 4 eigh c, end root; and denominator 2 eigh; end fraction",
        ),
        (
            SimpleSpeak,
            Low,
            "x equals; fraction, negative b plus or minus; square root, b squared minus 4 eigh c, end root; over, 2 eigh, end fraction",
        ),
        (
            SimpleSpeak,
            Normal,
            "x is equal to; fraction, negative b plus or minus; the square root of b squared minus 4 eigh c, end root; over, 2 eigh, end fraction",
        ),
        (
            SimpleSpeak,
            High,
            "x is equal to; fraction, negative b plus or minus; the square root of b squared minus 4 eigh c, end root; over, 2 eigh, end fraction",
        ),
    ];
    for (style, verbosity, words) in expect {
        assert_eq!(
            say_with(QUADRATIC, &options(style, verbosity)),
            words,
            "{style} {verbosity:?}"
        );
    }
}

/// The LaTeX the EPUB loader writes for MathML (no spaces, braced
/// scripts, display delimiters) reads as the same formula.
#[test]
fn epub_mathml_latex_reads_the_same() {
    let from_epub = r"$$x=\frac{-b\pm\sqrt{b^{2}-4ac}}{2a}$$";
    assert_eq!(say(from_epub), say(QUADRATIC));
}

/// Formulas a student meets, in ClearSpeak at normal verbosity.
#[test]
fn student_formulas() {
    for (text, words) in [
        (
            r"$\sin^2 x + \cos^2 x = 1$",
            "sine squared of x, plus cosine squared of x; is equal to 1",
        ),
        (
            r"$\int_0^1 x^2 \, dx$",
            "the integral from 0, to 1 of; x squared d x",
        ),
        (r"$|x|$", "the absolute value of x"),
        (r"$\frac{1}{2}$", "1 half"),
        (
            r"$\begin{pmatrix} a & b \\ c & d \end{pmatrix}$",
            "the 2 by 2 matrix; row 1; eigh, b; row 2; c, d",
        ),
        (
            r"$\lim_{x \to 0} \frac{\sin x}{x}$",
            "the limit as x approaches 0, of sine x over x",
        ),
    ] {
        assert_eq!(say(text), words, "{text}");
    }
}

/// End words at high verbosity.
#[test]
fn high_verbosity_adds_end_words() {
    let high = options(Style::ClearSpeak, Verbosity::High);
    assert_eq!(
        say_with(r"$\frac{a}{b}$", &high),
        "eigh over b, end fraction"
    );
    assert_eq!(
        say_with(r"$|x|$", &high),
        "the absolute value of x, end absolute value"
    );
}

/// The document's language picks MathCAT's rules.
#[test]
fn languages() {
    let text = r"$\frac{a}{b} + x^2$";
    for (language, words) in [
        ("fr", "a sur b plus x au carré"),
        ("de", "a durch b plus x quadrat"),
        ("es", "a partido por b más x cuadrado"),
        ("en", "eigh over b plus x squared"),
    ] {
        let o = Options {
            language,
            ..Options::default()
        };
        assert_eq!(say_with(text, &o), words, "{language}");
    }
}

/// Without pauses (punctuation read aloud), MathCAT's commas and
/// semicolons are left out, so they are not spoken as "comma".
#[test]
fn pauses_can_be_left_out() {
    let o = Options {
        pauses: false,
        ..Options::default()
    };
    assert_eq!(
        say_with(r"$a \times b \leq c$", &o),
        "eigh times b is less than or equal to c"
    );
}

/// The whole formula is one expanded span over its content; the
/// delimiters are elided.
#[test]
fn span_level_map() {
    let text = "so $x^2$ ok";
    let (out, map) = speak_text(text, &TextOptions::default(), &Options::default());
    assert_eq!(out, "so x squared ok");
    let kinds: Vec<_> = map
        .spans()
        .iter()
        .map(|s| (s.kind, s.source.to_range()))
        .collect();
    assert_eq!(
        kinds,
        [
            (SpanKind::Literal, 0..3),
            (SpanKind::Elided, 3..4),
            (SpanKind::Expanded, 4..7),
            (SpanKind::Elided, 7..8),
            (SpanKind::Literal, 8..11),
        ]
    );
}

/// A formula the parser had to repair goes to the built-in speech, which
/// names what it could not read.
#[test]
fn repaired_formulas_use_the_builtin_speech() {
    let text = r"$\frac{a}{$";
    let builtin = textweaver_math::speak_text(text, &TextOptions::default()).0;
    assert_eq!(say(text), builtin);
}
