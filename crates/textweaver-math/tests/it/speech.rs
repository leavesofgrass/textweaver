//! Spoken math at each verbosity, with offset-map invariants on every
//! result, star's math vectors, and highlight mapping checks.

use textweaver_core::{CharRange, Verbosity};
use textweaver_math::{Math, SpeechOptions, Spoken, parse_asciimath, parse_latex, speak};

const LOW: Verbosity = Verbosity::Low;
const NORMAL: Verbosity = Verbosity::Normal;
const HIGH: Verbosity = Verbosity::High;

/// Speaks `m` and checks the map: invariants, and every mapped span inside
/// the source.
fn spoken(m: &Math, v: Verbosity) -> Spoken {
    let s = speak(m, &SpeechOptions::new(v));
    s.map
        .check_invariants(&s.text)
        .unwrap_or_else(|e| panic!("{}: {e}\n{:?}", m.source, s.map));
    let len = m.source.chars().count();
    if let Some(ext) = s.map.source_extent() {
        assert!(ext.end.0 <= len, "{}: extent {ext} past {len}", m.source);
    }
    s
}

fn latex(src: &str, v: Verbosity) -> String {
    spoken(&parse_latex(src), v).text
}

fn am(src: &str, v: Verbosity) -> String {
    spoken(&parse_asciimath(src), v).text
}

/// `(source, low, normal, high)`.
type Case = (&'static str, &'static str, &'static str, &'static str);

fn check_latex(cases: &[Case]) {
    for (src, low, normal, high) in cases {
        assert_eq!(latex(src, LOW), *low, "low: {src}");
        assert_eq!(latex(src, NORMAL), *normal, "normal: {src}");
        assert_eq!(latex(src, HIGH), *high, "high: {src}");
    }
}

fn check_am(cases: &[Case]) {
    for (src, low, normal, high) in cases {
        assert_eq!(am(src, LOW), *low, "low: {src}");
        assert_eq!(am(src, NORMAL), *normal, "normal: {src}");
        assert_eq!(am(src, HIGH), *high, "high: {src}");
    }
}

/// star's `_normalize_math_inline` vectors (`tests/test_ttstext.py`,
/// the star parity reference Part 2 section 6.1). Delimited ones are in
/// `tests/text.rs`; here each is spoken as math at normal verbosity.
#[test]
fn star_vectors() {
    // Exact.
    assert_eq!(latex(r"\frac{a}{b}", NORMAL), "a over b");
    assert_eq!(latex(r"\sqrt{x}", NORMAL), "square root of x");
    assert_eq!(
        latex(r"a \times b \leq c", NORMAL),
        "a times b less than or equal to c"
    );
    assert_eq!(latex("x^2", NORMAL), "x squared");
    assert_eq!(latex("y^{3}", NORMAL), "y cubed");
    assert_eq!(latex("x_i", NORMAL), "x sub i");
    assert_eq!(latex("x_{ij}", NORMAL), "x sub i j");
    // Better wording, deliberately: "+" is spoken rather than left to the
    // engine's punctuation setting, and "x bar" is two words, not "x-bar".
    assert_eq!(latex(r"\alpha + \beta", NORMAL), "alpha plus beta");
    assert_eq!(latex(r"\bar{x}", NORMAL), "x bar");
    // star bug Q7: a trailing `x^2` is "x squared", not "x to the 2".
    assert_eq!(latex("x^2", LOW), "x squared");
}

#[test]
fn fractions() {
    check_latex(&[
        (
            r"\frac{a}{b}",
            "a over b",
            "a over b",
            "the fraction a over b end fraction",
        ),
        (
            r"\frac{x+1}{x-1}",
            "fraction x plus 1 over x minus 1",
            "the fraction with numerator x plus 1 and denominator x minus 1",
            "the fraction with numerator x plus 1 and denominator x minus 1 end fraction",
        ),
        (
            r"\frac{x+1}{2} + 3",
            "fraction x plus 1 over 2 end fraction plus 3",
            "the fraction with numerator x plus 1 and denominator 2 end fraction plus 3",
            "the fraction with numerator x plus 1 and denominator 2 end fraction plus 3",
        ),
        (
            r"\frac{3}{4}",
            "3 fourths",
            "3 fourths",
            "the fraction 3 over 4 end fraction",
        ),
        (
            r"\frac{1}{2}",
            "1 half",
            "1 half",
            "the fraction 1 over 2 end fraction",
        ),
        (
            r"\frac{d}{dx} e^x",
            "d over d x, e to the x",
            "d over d x, e to the x",
            "the fraction d over d x end fraction e to the x",
        ),
        (
            r"\frac{\partial f}{\partial x}",
            "partial f over partial x",
            "partial f over partial x",
            "the fraction partial f over partial x end fraction",
        ),
        (r"\binom{n}{k}", "n choose k", "n choose k", "n choose k"),
        (
            r"{a \over b}",
            "a over b",
            "a over b",
            "the fraction a over b end fraction",
        ),
    ]);
}

#[test]
fn roots_and_powers() {
    check_latex(&[
        (
            r"\sqrt{x}",
            "square root of x",
            "square root of x",
            "the square root of x end root",
        ),
        (
            r"\sqrt[3]{8}",
            "cube root of 8",
            "cube root of 8",
            "the cube root of 8 end root",
        ),
        (
            r"\sqrt[n]{x}",
            "n-th root of x",
            "n-th root of x",
            "the n-th root of x end root",
        ),
        (
            r"\sqrt{x+1} - 1",
            "square root of x plus 1 end root minus 1",
            "square root of x plus 1 end root minus 1",
            "the square root of x plus 1 end root minus 1",
        ),
        (
            "x^n",
            "x to the n",
            "x to the n-th power",
            "x to the n-th power",
        ),
        (
            "x^4",
            "x to the 4",
            "x to the 4th power",
            "x to the 4th power",
        ),
        (
            "x^{n+1}",
            "x to the n plus 1",
            "x raised to the n plus 1 power",
            "x raised to the exponent n plus 1 end exponent",
        ),
        (
            "x^{-1}",
            "x to the negative 1",
            "x to the negative 1 power",
            "x to the negative 1 power",
        ),
        ("e^x", "e to the x", "e to the x", "e to the x"),
        (
            r"e^{i\pi} + 1 = 0",
            "e to the i pi end exponent plus 1 equals 0",
            "e raised to the i pi power plus 1 equals 0",
            "e raised to the exponent i pi end exponent plus 1 equals 0",
        ),
        (
            "(a+b)^2",
            "open paren a plus b close paren squared",
            "open paren a plus b close paren squared",
            "open paren a plus b close paren squared",
        ),
        (
            "(-1)^n",
            "open paren negative 1 close paren to the n",
            "open paren negative 1 close paren to the n-th power",
            "open paren negative 1 close paren to the n-th power",
        ),
        (r"90^\circ", "90 degrees", "90 degrees", "90 degrees"),
        (
            "z^*",
            "z superscript star",
            "z superscript star",
            "z superscript star",
        ),
    ]);
}

#[test]
fn scripts() {
    check_latex(&[
        (
            "a_{n+1} = a_n + 1",
            "a sub n plus 1 end sub equals a sub n plus 1",
            "a sub n plus 1 end sub equals a sub n plus 1",
            "a sub n plus 1 end sub equals a sub n plus 1",
        ),
        (
            "x_i^2",
            "x sub i squared",
            "x sub i squared",
            "x sub i squared",
        ),
        (
            "f'(x)",
            "f prime of x",
            "f prime of x",
            "f prime of open paren x close paren",
        ),
        (
            r"\overset{def}{=}",
            "equals with d e f above",
            "equals with d e f above",
            "equals with d e f above",
        ),
    ]);
}

#[test]
fn functions_and_big_operators() {
    check_latex(&[
        (
            r"\sin^2 x + \cos^2 x = 1",
            "sine squared x plus cosine squared x equals 1",
            "sine squared x plus cosine squared x equals 1",
            "sine squared x plus cosine squared x equals 1",
        ),
        (
            r"\sin^{-1}(x)",
            "inverse sine of x",
            "inverse sine of x",
            "inverse sine of open paren x close paren",
        ),
        (
            r"\log_2 8 = 3",
            "log base 2 of 8 equals 3",
            "log base 2 of 8 equals 3",
            "log base 2 of 8 equals 3",
        ),
        (
            r"\ln x",
            "natural log of x",
            "natural log of x",
            "natural log of x",
        ),
        (
            "f(x) = x^2 - 2x + 1",
            "f of x equals x squared minus 2 x plus 1",
            "f of x equals x squared minus 2 x plus 1",
            "f of open paren x close paren equals x squared minus 2 x plus 1",
        ),
        (
            r"\sum_{i=1}^{n} i",
            "sum from i equals 1 to n of i",
            "the sum from i equals 1 to n of i",
            "the sum from i equals 1 to n of i",
        ),
        (
            r"\int_0^1 x^2 \, dx",
            "integral from 0 to 1 of x squared d x",
            "the integral from 0 to 1 of x squared d x",
            "the integral from 0 to 1 of x squared d x",
        ),
        (
            r"\lim_{x \to 0} \frac{\sin x}{x} = 1",
            "limit as x approaches 0 of fraction sine x over x end fraction equals 1",
            "the limit as x approaches 0 of the fraction with numerator sine x and denominator x end fraction equals 1",
            "the limit as x approaches 0 of the fraction with numerator sine x and denominator x end fraction equals 1",
        ),
        (
            r"\max_{x \in S} f(x)",
            "maximum over x is an element of S of f of x",
            "the maximum over x is an element of S of f of x",
            "the maximum over x is an element of cap S of f of open paren x close paren",
        ),
        (
            r"\operatorname{Var}(X)",
            "Var of X",
            "Var of X",
            "Var of open paren cap X close paren",
        ),
    ]);
}

#[test]
fn quadratic_formula() {
    check_latex(&[(
        r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}",
        "x equals fraction negative b plus or minus square root of b squared minus 4 a c over 2 a",
        "x equals the fraction with numerator negative b plus or minus square root of b squared minus 4 a c and denominator 2 a",
        "x equals the fraction with numerator negative b plus or minus the square root of b squared minus 4 a c end root and denominator 2 a end fraction",
    )]);
}

#[test]
fn brackets_and_sets() {
    check_latex(&[
        (
            "|x - 1|",
            "absolute value of x minus 1",
            "the absolute value of x minus 1",
            "the absolute value of x minus 1 end absolute value",
        ),
        (
            "|x - 1| + 2",
            "absolute value of x minus 1 end absolute value plus 2",
            "the absolute value of x minus 1 end absolute value plus 2",
            "the absolute value of x minus 1 end absolute value plus 2",
        ),
        (
            r"\lfloor x \rfloor + \lceil y \rceil",
            "floor of x plus ceiling of y",
            "the floor of x plus the ceiling of y",
            "the floor of x end floor plus the ceiling of y end ceiling",
        ),
        (
            r"\|v\|",
            "norm of v",
            "the norm of v",
            "the norm of v end norm",
        ),
        (
            r"\{x \mid x > 0\}",
            "set of all x such that x greater than 0",
            "the set of all x such that x greater than 0",
            "the set of all x such that x greater than 0 end set",
        ),
        (
            r"\{1, 2, 3\}",
            "set 1, 2, 3",
            "the set 1 comma 2 comma 3",
            "the set 1 comma 2 comma 3 end set",
        ),
        (
            r"\left( \frac{a}{b} \right)^2",
            "open paren a over b close paren squared",
            "open paren a over b close paren squared",
            "open paren the fraction a over b end fraction close paren squared",
        ),
        (r"(x)", "x", "x", "open paren x close paren"),
        (
            "[0, 1)",
            "open bracket 0, 1 close paren",
            "open bracket 0 comma 1 close paren",
            "open bracket 0 comma 1 close paren",
        ),
    ]);
}

#[test]
fn tables() {
    check_latex(&[
        (
            r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}",
            "2 by 2 matrix; row 1: a, b; row 2: c, d",
            "the 2 by 2 matrix; row 1: a, b; row 2: c, d",
            "the 2 by 2 matrix; row 1: column 1: a, column 2: b; row 2: column 1: c, column 2: d; end matrix",
        ),
        (
            r"\begin{vmatrix} 1 & 2 \\ 3 & 4 \end{vmatrix} = -2",
            "2 by 2 determinant; row 1: 1, 2; row 2: 3, 4; end determinant equals negative 2",
            "the 2 by 2 determinant; row 1: 1, 2; row 2: 3, 4; end determinant equals negative 2",
            "the 2 by 2 determinant; row 1: column 1: 1, column 2: 2; row 2: column 1: 3, column 2: 4; end determinant equals negative 2",
        ),
        (
            r"f(x) = \begin{cases} x & \text{if } x \ge 0 \\ -x & \text{otherwise} \end{cases}",
            "f of x equals cases; case 1: x, if x greater than or equal to 0; case 2: negative x, otherwise",
            "f of x equals 2 cases; case 1: x, if x greater than or equal to 0; case 2: negative x, otherwise",
            "f of open paren x close paren equals 2 cases; case 1: x, if x greater than or equal to 0; case 2: negative x, otherwise; end cases",
        ),
        (
            r"\begin{aligned} x &= 1 \\ y &= 2 \end{aligned}",
            "2 lines; line 1: x equals 1; line 2: y equals 2",
            "2 lines; line 1: x equals 1; line 2: y equals 2",
            "2 lines; line 1: x equals 1; line 2: y equals 2; end lines",
        ),
        (
            r"\begin{bmatrix} 1 & \\ & 1 \end{bmatrix}",
            "2 by 2 matrix; row 1: 1, blank; row 2: blank, 1",
            "the 2 by 2 matrix; row 1: 1, blank; row 2: blank, 1",
            "the 2 by 2 matrix; row 1: column 1: 1, column 2: blank; row 2: column 1: blank, column 2: 1; end matrix",
        ),
    ]);
}

#[test]
fn letters_accents_fonts() {
    check_latex(&[
        (
            r"\hat{x} + \vec{v}",
            "x hat plus vector v",
            "x hat plus vector v",
            "x hat plus vector v",
        ),
        (r"\overline{AB}", "A B bar", "A B bar", "cap A cap B bar"),
        (
            r"\overline{x+y} = 2",
            "bar of x plus y end bar equals 2",
            "bar of x plus y end bar equals 2",
            "bar of x plus y end bar equals 2",
        ),
        (
            r"\mathbf{v} \cdot \mathbf{w}",
            "v times w",
            "v times w",
            "bold v times bold w",
        ),
        (
            r"\Gamma(n) = (n-1)!",
            "gamma n equals open paren n minus 1 close paren factorial",
            "capital gamma n equals open paren n minus 1 close paren factorial",
            "capital gamma open paren n close paren equals open paren n minus 1 close paren factorial",
        ),
        (
            r"x \in \mathbb{R}",
            "x is an element of real numbers",
            "x is an element of the real numbers",
            "x is an element of the real numbers",
        ),
        (
            r"\forall \epsilon > 0 \; \exists \delta",
            "for all epsilon greater than 0 there exists delta",
            "for all epsilon greater than 0 there exists delta",
            "for all epsilon greater than 0 there exists delta",
        ),
        (
            "f: A \\to B",
            "f colon A to B",
            "f colon A to B",
            "f colon cap A to cap B",
        ),
        (
            r"\text{speed} = \frac{d}{t}",
            "speed equals d over t",
            "speed equals d over t",
            "speed equals the fraction d over t end fraction",
        ),
        (
            r"\cancel{x}",
            "crossed out x end crossed out",
            "crossed out x end crossed out",
            "crossed out x end crossed out",
        ),
        (
            r"a \not= b",
            "a not equal to b",
            "a not equal to b",
            "a not equal to b",
        ),
        ("n!", "n factorial", "n factorial", "n factorial"),
    ]);
}

#[test]
fn asciimath() {
    check_am(&[
        (
            "sum_(i=1)^n i^3=((n(n+1))/2)^2",
            "sum from i equals 1 to n of i cubed equals open paren fraction n open paren n plus 1 close paren over 2 close paren squared",
            "the sum from i equals 1 to n of i cubed equals open paren the fraction with numerator n open paren n plus 1 close paren and denominator 2 close paren squared",
            "the sum from i equals 1 to n of i cubed equals open paren the fraction with numerator n open paren n plus 1 close paren and denominator 2 end fraction close paren squared",
        ),
        (
            "[[a,b],[c,d]]",
            "2 by 2 matrix; row 1: a, b; row 2: c, d",
            "the 2 by 2 matrix; row 1: a, b; row 2: c, d",
            "the 2 by 2 matrix; row 1: column 1: a, column 2: b; row 2: column 1: c, column 2: d; end matrix",
        ),
        (
            "lim_(x->oo) 1/x = 0",
            "limit as x approaches infinity of 1 over x equals 0",
            "the limit as x approaches infinity of 1 over x equals 0",
            "the limit as x approaches infinity of the fraction 1 over x end fraction equals 0",
        ),
        (
            "int_0^1 f(x) dx",
            "integral from 0 to 1 of f of x d x",
            "the integral from 0 to 1 of f of x d x",
            "the integral from 0 to 1 of f of open paren x close paren d x",
        ),
        (
            "sin^-1(x)",
            "inverse sine of x",
            "inverse sine of x",
            "inverse sine of open paren x close paren",
        ),
        (
            "abs(x-1)",
            "absolute value of x minus 1",
            "the absolute value of x minus 1",
            "the absolute value of x minus 1 end absolute value",
        ),
        (
            "sqrt(x+1)/(x-1)",
            "fraction square root of x plus 1 over x minus 1",
            "the fraction with numerator square root of x plus 1 and denominator x minus 1",
            "the fraction with numerator the square root of x plus 1 end root and denominator x minus 1 end fraction",
        ),
        (
            "root(3)(x)",
            "cube root of x",
            "cube root of x",
            "the cube root of x end root",
        ),
        (
            "hat x + bar y",
            "x hat plus y bar",
            "x hat plus y bar",
            "x hat plus y bar",
        ),
        (
            "bbb R",
            "real numbers",
            "the real numbers",
            "the real numbers",
        ),
        (
            "x_(n+1)",
            "x sub n plus 1",
            "x sub n plus 1",
            "x sub n plus 1 end sub",
        ),
        (
            "a/b/c",
            "a over b divided by c",
            "a over b divided by c",
            "the fraction a over b end fraction divided by c",
        ),
        (
            "\"if\" x > 0",
            "if x greater than 0",
            "if x greater than 0",
            "if x greater than 0",
        ),
    ]);
}

#[test]
fn malformed_input_still_speaks() {
    assert_eq!(
        latex(r"\frac{1}{", NORMAL),
        "the fraction with numerator 1 and denominator"
    );
    assert_eq!(latex("x^", NORMAL), "x raised to the power");
    assert_eq!(latex(r"\foo{x}", NORMAL), "foo x");
    assert_eq!(latex("}{", NORMAL), "");
    assert_eq!(am("(x", NORMAL), "open paren x");
}

/// Bytes of the first occurrence of `word` in `text`.
fn bytes(text: &str, word: &str) -> std::ops::Range<u32> {
    let a = text
        .find(word)
        .unwrap_or_else(|| panic!("{word} in {text}"));
    a as u32..(a + word.len()) as u32
}

#[test]
fn words_map_to_their_source() {
    let m = parse_latex(r"\frac{a}{b} + x^2");
    let s = spoken(&m, NORMAL);
    assert_eq!(s.text, "a over b plus x squared");
    let src = |r: CharRange| m.source_text(r);
    let at = |w: &str| s.map.to_source(&s.text, bytes(&s.text, w)).map(src);
    assert_eq!(at("a").as_deref(), Some("a"));
    assert_eq!(at("over").as_deref(), Some("}{"));
    assert_eq!(at("plus").as_deref(), Some("+"));
    assert_eq!(at("squared").as_deref(), Some("^2"));
    // "x" maps to the x, exactly.
    let x = s.text.find(" x ").map(|i| i as u32 + 1).unwrap_or_default();
    assert_eq!(
        s.map.to_source(&s.text, x..x + 1).map(src).as_deref(),
        Some("x")
    );
}

#[test]
fn greek_and_commands_map_to_the_command() {
    let m = parse_latex(r"\alpha \leq \beta");
    let s = spoken(&m, NORMAL);
    assert_eq!(s.text, "alpha less than or equal to beta");
    let r = s.map.to_source(&s.text, bytes(&s.text, "alpha")).unwrap();
    assert!(m.source_text(r).contains("alpha"));
    let r = s.map.to_source(&s.text, bytes(&s.text, "less")).unwrap();
    assert_eq!(m.source_text(r), r"\leq");
}

#[test]
fn end_markers_map_to_closing_source_and_inserted_words_do_not_highlight() {
    // "end fraction" stands for the denominator's closing brace.
    let m = parse_latex(r"\frac{x+1}{2} + 3");
    let s = spoken(&m, NORMAL);
    let r = s
        .map
        .to_source(&s.text, bytes(&s.text, "end fraction"))
        .unwrap();
    assert_eq!(m.source_text(r), "}");
    // "power" has no source of its own: no highlight, resume at the
    // exponent's end.
    let m = parse_latex("x^{n+1} y");
    let s = spoken(&m, NORMAL);
    assert_eq!(s.text, "x raised to the n plus 1 power y");
    let power = bytes(&s.text, "power");
    assert_eq!(s.map.to_source(&s.text, power.clone()), None);
    let resume = s.map.resume_source(&s.text, power.start).unwrap();
    assert_eq!(resume.0, 7);
}

#[test]
fn out_of_order_words_stay_valid() {
    // "x hat": "hat" comes from `\hat{`, before the x.
    let m = parse_latex(r"\hat{x}");
    let s = spoken(&m, NORMAL);
    assert_eq!(s.text, "x hat");
    let r = s.map.to_source(&s.text, bytes(&s.text, "x")).unwrap();
    assert_eq!(m.source_text(r), "x");
    // "inverse sine": "inverse" comes from `^{-1}`, after `\sin`.
    let m = parse_latex(r"\sin^{-1} x");
    let s = spoken(&m, NORMAL);
    assert_eq!(s.text, "inverse sine x");
}

#[test]
fn deep_nesting_is_bounded() {
    let src = "{".repeat(500) + "x" + &"}".repeat(500);
    let m = parse_latex(&src);
    assert!(!m.is_clean());
    let _ = spoken(&m, HIGH);
    let src = r"\sqrt".repeat(300) + "x";
    let _ = spoken(&parse_latex(&src), NORMAL);
    let src = "sqrt ".repeat(300) + "x";
    let _ = spoken(&parse_asciimath(&src), NORMAL);
    let src = "(".repeat(300) + "x";
    let _ = spoken(&parse_asciimath(&src), NORMAL);
    let src = "x^".repeat(300) + "2";
    let _ = spoken(&parse_latex(&src), NORMAL);
}
