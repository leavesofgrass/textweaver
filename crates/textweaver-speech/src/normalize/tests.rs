//! star's `tests/test_ttstext.py` vectors (the star parity reference Part 2
//! section 6.1), one assertion per line, plus the bug fixes of section 7.2.
//! Every output also passes `check_invariants`, and literal spans reproduce
//! their source.

use std::collections::BTreeMap;

use proptest::prelude::*;
use textweaver_core::{CharPos, CharRange, OffsetMap, SpanKind, Utterance};

use super::*;

/// Checks the map of a transform output and returns the text.
fn checked(input: &str, (out, map): (String, OffsetMap)) -> String {
    map.check_invariants(&out)
        .unwrap_or_else(|e| panic!("{e} for {input:?} -> {out:?}\n{map:?}"));
    let chars: Vec<char> = input.chars().collect();
    for s in map.spans().iter().filter(|s| s.kind == SpanKind::Literal) {
        let spoken = &out[s.spoken.start as usize..s.spoken.end as usize];
        let src: String = chars[s.source.to_range()].iter().collect();
        assert_eq!(spoken, src, "literal span {s:?} in {input:?} -> {out:?}");
    }
    if let Some(ext) = map.source_extent() {
        assert!(ext.end.0 <= chars.len());
    }
    out
}

fn numbers(s: &str) -> String {
    checked(s, Numbers::default().apply(s))
}

fn math(s: &str) -> String {
    checked(s, Math::default().apply(s))
}

fn abbrev(s: &str, custom: &[(&str, &str)]) -> String {
    let custom: BTreeMap<String, String> = custom
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    checked(s, Abbreviations::new(&custom).apply(s))
}

fn pron(s: &str, lexicon: &[(&str, &str)]) -> String {
    let lex: BTreeMap<String, String> = lexicon
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    checked(s, Pronunciations::new(&lex).apply(s))
}

fn md(s: &str, skip_code: bool) -> String {
    checked(
        s,
        MarkdownResidue::new(skip_code, TableMode::Structured).apply(s),
    )
}

fn tables(s: &str, mode: TableMode) -> String {
    checked(s, tables_to_narration(s, mode))
}

#[test]
fn normalize_numbers_vectors() {
    // tests/test_ttstext.py:132-210
    let cases = [
        (
            "Meeting on 2024-03-15 today.",
            "Meeting on March fifteenth, twenty twenty-four today.",
        ),
        (
            "Due 03/15/2024 ok.",
            "Due March fifteenth, twenty twenty-four ok.",
        ),
        ("at 12:00", "at noon"),
        ("at 00:00", "at midnight"),
        // star: "at three thirty PM"; a 24-hour time gains no AM or PM.
        ("at 15:30", "at fifteen thirty"),
        ("at 3:45 PM", "at three forty-five PM"),
        ("at 9:15 AM", "at nine fifteen AM"),
        (
            "costs $1,234.56 total",
            "costs one thousand two hundred thirty-four dollars and fifty-six cents total",
        ),
        ("$1.00", "one dollar"),
        ("$0.50", "fifty cents"),
        (
            "75% and 3.5%",
            "seventy-five percent and three point five percent",
        ),
        ("the 1st and 22nd", "the first and twenty-second"),
        (
            "1,234,567 people",
            "one million two hundred thirty-four thousand five hundred sixty-seven people",
        ),
        ("pi is 3.14", "pi is three point one four"),
        ("year 1984 here", "year nineteen eighty-four here"),
        ("value 50000 ok", "value fifty thousand ok"),
    ];
    for (input, want) in cases {
        assert_eq!(numbers(input), want, "{input}");
    }
}

#[test]
fn normalize_numbers_deliberate_changes_to_star_vectors() {
    // star: "at zero thirty AM". Written with AM it is a twelve-hour
    // time, which has no hour zero; without, a 24-hour time.
    assert_eq!(numbers("at 00:30 AM"), "at twelve thirty AM");
    assert_eq!(numbers("at 00:30"), "at zero thirty");
    // star pinned "v1.two point three"; dotted sequences are read by part.
    assert_eq!(numbers("v1.2.3"), "v 1 dot 2 dot 3");
}

#[test]
fn star_number_bugs_are_fixed() {
    // Q1 times.
    assert_eq!(numbers("at 3:45 today"), "at three forty-five AM today");
    assert_eq!(numbers("3:00 amazing"), "three AM amazing");
    assert_eq!(numbers("John 3:16 says"), "John three sixteen says");
    assert_eq!(numbers("08:05"), "eight oh five");
    assert_eq!(numbers("9:05"), "nine oh five AM");
    assert_eq!(numbers("13:00 PM"), "one PM");
    assert_eq!(numbers("10:30:15"), "ten thirty AM and fifteen seconds");
    assert_eq!(numbers("at 9:30 a.m."), "at nine thirty AM.");
    assert_eq!(numbers("at 9:30 p.m. we eat"), "at nine thirty PM we eat");
    // Q2 currency.
    assert_eq!(numbers("It costs $5."), "It costs five dollars.");
    assert_eq!(
        numbers("$3.14159"),
        "three point one four one five nine dollars"
    );
    assert_eq!(numbers("£2.01"), "two pounds and one penny");
    assert_eq!(numbers("£0.05"), "five pence");
    assert_eq!(numbers("€1.5"), "one euro and fifty cents");
    // Q3 decimals and years.
    assert_eq!(numbers("in 2024."), "in twenty twenty-four.");
    assert_eq!(
        numbers("1,234.5"),
        "one thousand two hundred thirty-four point five"
    );
    assert_eq!(numbers("192.168.1.1"), "192 dot 168 dot 1 dot 1");
    assert_eq!(numbers("add .5 cup"), "add point five cup");
    assert_eq!(
        numbers("the 2020s and 1990s"),
        "the twenty twenties and nineteen nineties"
    );
}

#[test]
fn number_expansions_highlight_their_token() {
    let input = "It costs $5 in 2024.";
    let (out, map) = Numbers::default().apply(input);
    assert_eq!(out, "It costs five dollars in twenty twenty-four.");
    let at = |w: &str| {
        let i = out.find(w).unwrap() as u32;
        map.to_source(&out, i..i + w.len() as u32)
    };
    assert_eq!(at("dollars"), Some(CharRange::new(9, 11)));
    assert_eq!(at("twenty-four"), Some(CharRange::new(15, 19)));
    assert_eq!(at("in"), Some(CharRange::new(12, 14)));
}

#[test]
fn strip_markdown_vectors() {
    // T1 (:216-236). star merged the list into the paragraph before it (Q5);
    // the list stays its own paragraph here.
    let t1 = "# Heading\n\nSome **bold** and *italic* and `code` text.\n\n- item one\n- item two\n\n> a quote\n\n[link](http://x.com)\n\n```python\nprint(1)\n```\n";
    let out = md(t1, true);
    for want in [
        "Heading", "bold", "italic", "code", "item one", "item two", "a quote", "link",
    ] {
        assert!(out.contains(want), "{want} in {out:?}");
    }
    for bad in ["#", "*", "`", ">", "["] {
        assert!(!out.contains(bad), "{bad} in {out:?}");
    }
    assert!(!out.contains("print(1)"));
    assert_eq!(
        out,
        "Heading\n\nSome bold and italic and code text.\n\nitem one item two\n\na quote\n\nlink"
    );
    // T2 (:239-242)
    assert_eq!(md("```\nkeep me\n```", false), "keep me");
    // T3 (:461-473), with the Q5 fix.
    assert_eq!(
        md(
            "Groceries:\n\n- fruit\n    - apples\n    - bananas\n- bread\n",
            true
        ),
        "Groceries:\n\nfruit apples bananas bread"
    );
    // T4 (:476-481)
    assert_eq!(
        md("Paragraph.\n\n    x = compute_thing()\n\nAfter.\n", true),
        "Paragraph.\n\nAfter."
    );
    // T5 (:484-488)
    assert_eq!(
        md("> quoted\n>> nested quote\n> > spaced nested\n", true),
        "quoted nested quote spaced nested"
    );
    // T6 (:491-499)
    assert_eq!(
        md(
            "star is a reader\nthat converts text\nfor easier reading.\n\nNew paragraph here.\n",
            true
        ),
        "star is a reader that converts text for easier reading.\n\nNew paragraph here."
    );
}

#[test]
fn markdown_identifier_fixes() {
    // Q6: identifiers and arithmetic survive.
    assert_eq!(md("my_var_name and 2*3*4", true), "my_var_name and 2*3*4");
    assert_eq!(md("a _real_ word", true), "a real word");
    // Q5: a paragraph break before a list is kept.
    assert_eq!(md("para\n\n\n- a\n- b", true), "para\n\na b");
}

const TABLE: &str = "Intro text\n\n| Name | Age | City |\n|------|-----|------|\n| Alice | 30 | NY |\n| Bob | 25 | Boston |\n\nAfter text";

#[test]
fn tables_to_narration_vectors() {
    // tests/test_ttstext.py:380-419
    assert_eq!(
        tables(TABLE, TableMode::Structured),
        "Intro text\n\nTable with 3 columns: Name, Age, City.\nRow 1: Name is Alice, Age is 30, City is NY.\nRow 2: Name is Bob, Age is 25, City is Boston.\n\n\nAfter text"
    );
    assert_eq!(
        tables(TABLE, TableMode::Flat),
        "Intro text\n\nName.  Age.  City.\nAlice.  30.  NY.\nBob.  25.  Boston.\n\n\nAfter text"
    );
    assert_eq!(
        tables(TABLE, TableMode::Skip),
        "Intro text\n\nTable with 3 columns \u{2014} skipped.\n\n\nAfter text"
    );
    assert_eq!(
        tables("| a | b |\n|---|---|\n| 1 | 2 |", TableMode::Structured),
        "Table with 2 columns: a, b.\nRow 1: a is 1, b is 2.\n"
    );
    assert_eq!(
        tables("just text\nno tables", TableMode::Structured),
        "just text\nno tables"
    );
}

#[test]
fn table_fixes_and_mapping() {
    // Q10: an empty middle cell keeps later cells under their headers.
    assert_eq!(
        tables(
            "| a | b | c |\n|---|---|---|\n| 1 |  | 3 | 4 |",
            TableMode::Structured
        ),
        "Table with 3 columns: a, b, c.\nRow 1: a is 1, c is 3, column 4 is 4.\n"
    );
    // GitHub tables without a leading pipe.
    assert_eq!(
        tables("a | b\n--|--\n1 | 2", TableMode::Structured),
        "Table with 2 columns: a, b.\nRow 1: a is 1, b is 2.\n"
    );
    // Cells highlight their source; inserted words highlight nothing.
    let (out, map) = tables_to_narration(TABLE, TableMode::Structured);
    let alice = out.find("Alice").unwrap() as u32;
    let src = map.to_source(&out, alice..alice + 5).unwrap();
    assert_eq!(&TABLE.chars().collect::<String>()[src.to_range()], "Alice");
    let row = out.find("Row 1").unwrap() as u32;
    assert_eq!(map.to_source(&out, row..row + 3), None);
}

#[test]
fn ssml_vectors() {
    // tests/test_ttstext.py:248-293
    assert_eq!(
        text_to_ssml("Hello world. This is a test, really.", "pyttsx3", 350, 150),
        "<speak>Hello world.<break time=\"350ms\"/> This is a test,<break time=\"150ms\"/> really.</speak>"
    );
    assert_eq!(
        text_to_ssml("<speak>x</speak>", "pyttsx3", 350, 150),
        "<speak>x</speak>"
    );
    assert_eq!(
        text_to_ssml("a < b", "pyttsx3", 350, 150),
        "<speak>a &lt; b</speak>"
    );
    let tom = text_to_ssml("Tom & Jerry run", "pyttsx3", 350, 150);
    assert_eq!(tom, "<speak>Tom &amp; Jerry run</speak>");
    assert!(!tom.contains("&amp;<break"));
    assert_eq!(
        text_to_ssml("First clause; second clause", "pyttsx3", 350, 150),
        "<speak>First clause;<break time=\"150ms\"/> second clause</speak>"
    );
    let dec = text_to_ssml("Hi there. Done.", "dectalk", 350, 150);
    assert_eq!(dec, "Hi there. [:pau 350] Done.");
    assert_eq!(
        text_to_dectalk("Hi there. Wait, what?", 350, 150),
        "Hi there. [:pau 350] Wait, [:pau 150] what?"
    );
    assert_eq!(
        text_to_dectalk("One. Two, three.", 500, 200),
        "One. [:pau 500] Two, [:pau 200] three."
    );
}

#[test]
fn expand_abbreviations_vectors() {
    // tests/test_ttstext.py:299-312
    assert_eq!(
        abbrev("See Fig. 3 and e.g., this, etc. Dr. Smith.", &[]),
        "See Figure 3 and for example, this, et cetera Doctor Smith."
    );
    assert_eq!(abbrev("Smith et al. found", &[]), "Smith and others found");
    assert_eq!(
        abbrev("Use FOO here", &[("FOO", "foobar")]),
        "Use foobar here"
    );
}

#[test]
fn abbreviation_bugs_are_fixed() {
    // Q4.
    assert_eq!(abbrev("I said no.", &[]), "I said no.");
    assert_eq!(abbrev("No. I disagree.", &[]), "No. I disagree.");
    assert_eq!(
        abbrev("See No. 5 on p. 12.", &[]),
        "See Number 5 on page 12."
    );
    assert_eq!(abbrev("It took 5 min.", &[]), "It took 5 minutes.");
    assert_eq!(abbrev("Set the max. value", &[]), "Set the maximum value");
    assert_eq!(
        abbrev("Go to max. Then stop.", &[]),
        "Go to max. Then stop."
    );
    assert_eq!(
        abbrev("apples, pears, etc.", &[]),
        "apples, pears, et cetera."
    );
    // "p.m." is not "page m."
    assert_eq!(abbrev("at 9 p.m. today", &[]), "at 9 p.m. today");
    // User expansions are literal, not regex templates.
    assert_eq!(abbrev("XYZ", &[("XYZ", "a\\1b$0")]), "a\\1b$0");
}

#[test]
fn apply_pronunciations_vectors() {
    // tests/test_ttstext.py:318-345
    assert_eq!(
        pron("CHF is bad", &[("CHF", "congestive heart failure")]),
        "congestive heart failure is bad"
    );
    assert_eq!(
        pron("the chf patient", &[("CHF", "see aitch eff")]),
        "the see aitch eff patient"
    );
    assert_eq!(
        pron(
            "heart attack now",
            &[("heart attack", "MI"), ("heart", "pump")]
        ),
        "MI now"
    );
    assert_eq!(pron("text", &[]), "text");
    assert_eq!(pron("hi", &[("", "x")]), "hi");
    // One pass: a replacement is not rewritten by a shorter term.
    assert_eq!(
        pron(
            "CHF",
            &[("CHF", "congestive heart failure"), ("heart", "pump")]
        ),
        "congestive heart failure"
    );
}

#[test]
fn normalize_math_vectors() {
    // tests/test_ttstext.py:351-374. star's undelimited vectors are
    // delimited here, as math is in documents (ADR-0018); the wording of
    // `\alpha + \beta` ("plus" is spoken) and `\bar{x}` ("x bar") changed
    // deliberately.
    for (input, want) in [
        (r"$\frac{a}{b}$", "a over b"),
        (r"$\sqrt{x}$", "square root of x"),
        (r"$\alpha + \beta$", "alpha plus beta"),
        (r"$a \times b \leq c$", "a times b less than or equal to c"),
        (r"$\bar{x}$", "x bar"),
        ("$x^2$ and $y^{3}$", "x squared and y cubed"),
        ("$x_i$ and $x_{ij}$", "x sub i and x sub i j"),
        ("hello world", "hello world"),
    ] {
        assert_eq!(math(input), want, "{input}");
    }
    // Undelimited notation in prose is left as written.
    assert_eq!(math("x^2 and y^{3}"), "x^2 and y^{3}");
    assert_eq!(normalize_math(r"$\sqrt{x}$"), "square root of x");
}

#[test]
fn math_bugs_are_fixed() {
    // Q6: prose identifiers.
    assert_eq!(math("snake_case word"), "snake_case word");
    assert_eq!(math("my_var_name is x"), "my_var_name is x");
    assert_eq!(math(r"$\alpha_i$"), "alpha sub i");
    // Q7: trailing power.
    assert_eq!(math("$x^2$"), "x squared");
    assert_eq!(math("area $x^2$"), "area x squared");
    // Q8: no global cleanup; dollars that are not math.
    assert_eq!(math("$5 and $10 {kept}"), "$5 and $10 {kept}");
    assert_eq!(math(r"\alphabet"), r"\alphabet");
    assert_eq!(math(r"Pandoc \$3.99"), r"Pandoc \$3.99");
    // Operator symbols in prose are still read.
    assert_eq!(math("A → B"), "A approaches B");
    assert_eq!(math("3 × 4 ≤ 12"), "3 times 4 less than or equal to 12");
}

#[test]
fn math_verbosity_and_asciimath_settings() {
    let at = |v: Verbosity, s: &str| checked(s, Math::new(v, None).apply(s));
    assert_eq!(
        at(Verbosity::Low, r"$\frac{x+1}{2}$"),
        "fraction x plus 1 over 2"
    );
    assert_eq!(
        at(Verbosity::Normal, r"$\frac{x+1}{2}$"),
        "the fraction with numerator x plus 1 and denominator 2"
    );
    assert!(
        at(Verbosity::High, r"$\frac{x+1}{2}$").contains("end fraction"),
        "{}",
        at(Verbosity::High, r"$\frac{x+1}{2}$")
    );
    // ASCIIMath is off unless a delimiter is set.
    assert_eq!(math("so `x^2` is"), "so `x^2` is");
    let am = Math::new(Verbosity::Normal, Some('`'));
    assert_eq!(
        checked("so `x^2` is", am.apply("so `x^2` is")),
        "so x squared is"
    );
    // From the settings.
    let cfg = NormalizeConfig {
        math_verbosity: Verbosity::Low,
        asciimath_delimiter: Some('`'),
        ..NormalizeConfig::default()
    };
    let m = Math::from_config(&cfg);
    assert_eq!(m.options().speech.verbosity, Verbosity::Low);
    assert_eq!(m.options().detect.asciimath, Some('`'));
    assert_eq!(NormalizeConfig::default().math_verbosity, Verbosity::Normal);
    assert_eq!(NormalizeConfig::default().asciimath_delimiter, None);
}

/// The full pipeline (as the speech service runs it) over math, with the
/// utterance placed at a document offset.
fn pipeline_utterance(text: &str, cfg: &NormalizeConfig) -> Utterance {
    let p = Pipeline::for_settings(cfg, PunctuationLevel::Some, false, false);
    let u = p.apply(Utterance::literal(text, CharPos(1000)));
    u.offset_map
        .check_invariants(&u.text)
        .unwrap_or_else(|e| panic!("{e} for {text:?} -> {:?}", u.text));
    u
}

/// The document text a spoken word highlights.
fn highlighted(text: &str, u: &Utterance, word: &str) -> String {
    let i = u
        .text
        .find(word)
        .unwrap_or_else(|| panic!("{word:?} in {:?}", u.text)) as u32;
    let r = u
        .source_for(i..i + word.len() as u32)
        .unwrap_or_else(|| panic!("no source for {word:?} in {:?}", u.text));
    let chars: Vec<char> = text.chars().collect();
    chars[r.start.0 - 1000..r.end.0 - 1000].iter().collect()
}

#[test]
fn math_runs_before_numbers_and_highlights_its_source() {
    assert_eq!(
        pipeline_utterance("", &settings()).text,
        "",
        "empty stays empty"
    );
    assert_eq!(
        Pipeline::for_settings(&settings(), PunctuationLevel::Some, false, false).names()[0],
        "math"
    );

    // A power inside a sentence: "2" is not currency, "x" highlights x.
    let text = "We know that $x^2$ grows fast.";
    let u = pipeline_utterance(text, &settings());
    assert_eq!(u.text, "We know that x squared grows fast.");
    assert_eq!(highlighted(text, &u, "x squared"), "x^2");
    assert_eq!(highlighted(text, &u, "grows"), "grows");
    assert_eq!(highlighted(text, &u, "know"), "know");

    // A digit right after the opening dollar is still math, not money.
    let text = "Solve $2x = 4$ first.";
    let u = pipeline_utterance(text, &settings());
    assert_eq!(u.text, "Solve 2 x equals 4 first.");
    assert_eq!(highlighted(text, &u, "equals"), "=");

    // A fraction, word by word as an engine reports it: each part
    // highlights its own source, "over" the bar between them, and the
    // delimiters are never spoken.
    let text = r"Take \(\frac{a}{b}\) now.";
    let u = pipeline_utterance(text, &settings());
    assert_eq!(u.text, "Take a over b now.");
    let words: Vec<String> = [0..4, 5..6, 7..11, 12..13, 14..17]
        .into_iter()
        .map(|spoken| {
            let r = u.source_for(spoken).unwrap_or_default();
            text.chars()
                .skip(r.start.0 - 1000)
                .take(r.end.0 - r.start.0)
                .collect()
        })
        .collect();
    assert_eq!(words, ["Take", "a", "}{", "b", "now"]);

    // Prices stay prices: Numbers reads them after math left them alone.
    let text = "It costs $5 and $10 today.";
    let u = pipeline_utterance(text, &settings());
    assert_eq!(u.text, "It costs five dollars and ten dollars today.");
    assert_eq!(highlighted(text, &u, "ten dollars"), "$10");

    // Math and money in one sentence.
    let text = "Pay $5 when $n > 3$.";
    let u = pipeline_utterance(text, &settings());
    assert_eq!(u.text, "Pay five dollars when n greater than 3.");
    assert_eq!(highlighted(text, &u, "3"), "3");
    assert_eq!(highlighted(text, &u, "greater"), ">");
    assert_eq!(highlighted(text, &u, "five dollars"), "$5");
}

#[test]
fn math_off_leaves_latex_to_the_other_transforms() {
    let cfg = NormalizeConfig {
        math: false,
        ..settings()
    };
    let p = Pipeline::for_settings(&cfg, PunctuationLevel::Some, false, false);
    assert!(!p.names().contains(&"math"));
}

fn settings() -> NormalizeConfig {
    NormalizeConfig::default()
}

/// The math engine setting: textweaver's own speech by default, and, in a
/// build without the `mathcat` feature, whatever it says (ADR-0029).
#[test]
fn math_engine_defaults_to_builtin_and_changes_nothing_without_mathcat() {
    assert_eq!(settings().math_engine, MathEngine::Builtin);
    assert_eq!(settings().math_language, None);
    let text = r"So $\frac{a}{b} + x^2$ and \(\sqrt{y}\) end.";
    let builtin = pipeline_utterance(text, &settings());
    assert_eq!(
        builtin.text,
        "So a over b plus x squared and square root of y end."
    );
    for engine in [
        MathEngine::MathCatClearSpeak,
        MathEngine::MathCatSimpleSpeak,
    ] {
        let cfg = NormalizeConfig {
            math_engine: engine,
            math_language: Some("fr".into()),
            ..settings()
        };
        let u = pipeline_utterance(text, &cfg);
        if mathcat_available() {
            assert_ne!(u.text, builtin.text, "{engine:?}");
        } else {
            assert_eq!(u, builtin, "{engine:?}");
            assert_eq!(Math::from_config(&cfg).engine(), MathEngine::Builtin);
        }
    }
}

#[test]
fn math_engine_names_in_settings_files() {
    for (engine, name) in [
        (MathEngine::Builtin, "builtin"),
        (MathEngine::MathCatClearSpeak, "mathcat"),
        (MathEngine::MathCatSimpleSpeak, "mathcat_simplespeak"),
    ] {
        assert_eq!(
            serde_json::to_string(&engine).unwrap(),
            format!("\"{name}\"")
        );
    }
}

/// With MathCAT: its words (digits left for the engine, as with the
/// built-in speech), one highlight over the whole formula, pauses dropped
/// when punctuation is read.
#[cfg(feature = "mathcat")]
#[test]
fn mathcat_speaks_through_the_pipeline() {
    let cfg = NormalizeConfig {
        math_engine: MathEngine::MathCatClearSpeak,
        ..settings()
    };
    let text = r"We know $\frac{1}{2} + x^2$ now.";
    let u = pipeline_utterance(text, &cfg);
    assert_eq!(u.text, "We know 1 half plus x squared now.");
    assert_eq!(highlighted(text, &u, "plus"), r"\frac{1}{2} + x^2");
    assert_eq!(highlighted(text, &u, "now"), "now");

    let simple = NormalizeConfig {
        math_engine: MathEngine::MathCatSimpleSpeak,
        math_verbosity: Verbosity::Low,
        ..settings()
    };
    assert_eq!(
        pipeline_utterance(r"$|x|$", &simple).text,
        "absolute value x"
    );

    let french = NormalizeConfig {
        math_language: Some("fr-CA".into()),
        ..cfg.clone()
    };
    assert_eq!(
        pipeline_utterance(r"$\frac{a}{b}$", &french).text,
        "a sur b"
    );

    let text = r"$a \times b \leq c$";
    let some = Pipeline::for_settings(&cfg, PunctuationLevel::Some, false, false)
        .apply(Utterance::literal(text, CharPos::ZERO));
    assert_eq!(some.text, "eigh times b, is less than or equal to c");
    let all = Pipeline::for_settings(&cfg, PunctuationLevel::All, false, false)
        .apply(Utterance::literal(text, CharPos::ZERO));
    assert_eq!(all.text, "eigh times b is less than or equal to c");
    all.offset_map.check_invariants(&all.text).unwrap();
}

#[test]
fn preprocess_vectors() {
    // tests/test_ttstext.py:425-455
    assert_eq!(
        preprocess("See Fig. 3, it costs $5 in 2024 today.", &settings()),
        "See Figure 3, it costs five dollars in twenty twenty-four today."
    );
    let off = NormalizeConfig {
        use_pronunciations: false,
        abbreviations: false,
        numbers: false,
        math: false,
        ..settings()
    };
    assert_eq!(
        preprocess("See Fig. 3 costs $5", &off),
        "See Fig. 3 costs $5"
    );
    let custom = NormalizeConfig {
        pronunciations: [("CHF".to_owned(), "see aitch eff".to_owned())].into(),
        abbrev_expansions: [("XYZ".to_owned(), "exwhyzee".to_owned())].into(),
        ..settings()
    };
    assert_eq!(
        preprocess("CHF and XYZ in Fig. 1", &custom),
        "see aitch eff and exwhyzee in Figure 1"
    );
    assert_eq!(
        preprocess("It is 2024 now.", &settings()),
        "It is twenty twenty-four now."
    );
}

#[test]
fn pipeline_keeps_document_positions() {
    let p = Pipeline::for_settings(&settings(), PunctuationLevel::Some, false, false);
    let u = p.apply(Utterance::literal("Ask Dr. Lee about $5.", CharPos(100)));
    assert_eq!(u.text, "Ask Doctor Lee about five dollars.");
    u.offset_map.check_invariants(&u.text).unwrap();
    assert_eq!(u.source_for(4..10), Some(CharRange::new(104, 107)));
    assert_eq!(u.source_for(11..14), Some(CharRange::new(108, 111)));
    let d = u.text.find("dollars").unwrap() as u32;
    assert_eq!(u.source_for(d..d + 7), Some(CharRange::new(118, 120)));
}

#[test]
fn native_normalization_skips_overlapping_transforms() {
    let cfg = NormalizeConfig {
        pronunciations: [("CHF".to_owned(), "see aitch eff".to_owned())].into(),
        abbrev_expansions: [("XYZ".to_owned(), "exwhyzee".to_owned())].into(),
        ..settings()
    };
    let p = Pipeline::for_settings(&cfg, PunctuationLevel::Some, true, true);
    assert!(!p.names().contains(&"numbers"));
    let (out, _) = p.apply_text("CHF: Dr. XYZ paid $5 at 3:45 for camelCase & more");
    assert_eq!(
        out,
        "see aitch eff: Dr. exwhyzee paid $5 at 3:45 for camel Case and more"
    );
}

#[test]
fn announcements_stay_unmapped() {
    let p = Pipeline::for_settings(&settings(), PunctuationLevel::Some, false, false);
    let u = p.apply(Utterance::announcement("Rate 2024 wpm"));
    assert_eq!(u.text, "Rate twenty twenty-four wpm");
    assert!(u.offset_map.is_empty());
}

/// Sentences built from the vectors' tricky tokens.
fn token() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("Dr.".to_owned()),
        Just("$1,234.56".to_owned()),
        Just("2024-03-15".to_owned()),
        Just("3:45".to_owned()),
        Just("PM".to_owned()),
        Just("75%".to_owned()),
        Just("22nd".to_owned()),
        Just("3.14".to_owned()),
        Just("v1.2.3".to_owned()),
        Just("e.g.,".to_owned()),
        Just("x^2".to_owned()),
        Just("x_{ij}".to_owned()),
        Just(r"$\frac{a}{b}$".to_owned()),
        Just(r"\alpha".to_owned()),
        Just("**bold**".to_owned()),
        Just("[link](u)".to_owned()),
        Just("camelCase".to_owned()),
        Just("C++".to_owned()),
        Just("naïve".to_owned()),
        Just("日本語".to_owned()),
        Just("—".to_owned()),
        Just("PMID 31769816".to_owned()),
        Just("doi:10.1016/S0140-6736(20)30183-5".to_owned()),
        Just("(503) 555-0123".to_owned()),
        Just("ISBN 0-8044-2957-X".to_owned()),
        Just("q.o.d.".to_owned()),
        Just("MgSO4".to_owned()),
        Just("25µg".to_owned()),
        Just("TNF-α".to_owned()),
        Just("× 10^9".to_owned()),
        Just("10⁻³".to_owned()),
        "[a-zA-Z]{1,7}",
        "[0-9]{1,6}",
    ]
}

fn sentence() -> impl Strategy<Value = String> {
    proptest::collection::vec(token(), 1..12).prop_map(|t| format!("{}.", t.join(" ")))
}

proptest! {
    #[test]
    fn every_normalized_utterance_passes_invariants(
        s in sentence(),
        start in 0usize..1000,
        level in prop_oneof![
            Just(PunctuationLevel::None),
            Just(PunctuationLevel::Some),
            Just(PunctuationLevel::All)
        ],
        split in any::<bool>(),
        markdown in any::<bool>(),
    ) {
        let cfg = NormalizeConfig { markdown, ..settings() };
        let p = Pipeline::for_settings(&cfg, level, split, false);
        let u = p.apply(Utterance::literal(s.clone(), CharPos(start)));
        prop_assert!(u.offset_map.check_invariants(&u.text).is_ok(), "{:?}\n{:?} -> {:?}\n{:?}",
            u.offset_map.check_invariants(&u.text), s, u.text, u.offset_map);
        if let Some(ext) = u.offset_map.source_extent() {
            prop_assert!(ext.start.0 >= start && ext.end.0 <= start + s.chars().count());
        }
        let chars: Vec<char> = s.chars().collect();
        for sp in u.offset_map.spans().iter().filter(|x| x.kind == SpanKind::Literal) {
            let spoken = &u.text[sp.spoken.start as usize..sp.spoken.end as usize];
            let src: String = chars[sp.source.start.0 - start..sp.source.end.0 - start].iter().collect();
            prop_assert_eq!(spoken, src);
        }
    }
}

// Health sciences: clinical reading correctness. The vectors come from
// `docs/dev/research/health-sciences-use-cases.md`, section 3 ("Numbers,
// units, and safety", "What an abbreviation expander must never expand",
// and "Tests to add").

/// The default pipeline at punctuation "some", on its own text.
fn spoken(text: &str) -> String {
    pipeline_utterance(text, &settings()).text
}

/// The section 3 table, read by the default pipeline. Rows for gene
/// symbols, isotopes and chemistry are a later wave's; they pin today's
/// reading, so the change shows when it comes.
#[test]
fn clinical_vectors() {
    let cases = [
        // The dosing abbreviations are the medical lexicon's
        // (`medical_vectors`).
        (
            "Give 5 mg q6h PRN for pain.",
            "Give 5 milligrams q6h PRN for pain.",
        ),
        (
            "0.5 mg, not 5.0 mg",
            "zero point five milligrams, not five point zero milligrams",
        ),
        ("taper over 6–8 weeks", "taper over 6 to 8 weeks"),
        (
            "95% CI 1.2–3.4; p < 0.05; p = .03",
            "ninety-five percent CI one point two to three point four; p less than zero point zero five; p equals point zero three",
        ),
        // Later: chemistry. (The space before the semicolon is the
        // punctuation step's, unchanged here.)
        (
            "K+ 3.5 mEq/L; Ca2+; Na+",
            "K plus three point five milliequivalents per liter; Ca2 plus ; Na plus",
        ),
        ("25 mcg vs 25 µg", "25 micrograms vs 25 micrograms"),
        (
            "CPT 99213; PMID 31769816; ZIP 97239",
            "CPT nine nine two one three; PMID three one seven six nine eight one six; ZIP nine seven two three nine",
        ),
        (
            "ICD-10 E11.9; NCT04368728",
            "ICD-10 E11.9; NCT zero four three six eight seven two eight",
        ),
        ("at 08:05", "at eight oh five"),
        (
            "TNF-α; 10 U insulin; QD and QOD",
            "TNF alpha; 10 U insulin; Q D and Q O D",
        ),
        (
            "38.5°C; SpO2 98%; 120/80 mmHg",
            "thirty-eight point five degrees Celsius; SpO2 ninety-eight percent; 120 over 80 millimeters of mercury",
        ),
        (
            "WBC 11.5 × 10^9/L",
            "WBC eleven point five times ten to the ninth per liter",
        ),
        // Later: gene symbols and isotopes.
        ("BRCA1, TP53; 99mTc; 131I", "BRCA1, TP53; 99mTc; 131I"),
        // A decimal comma: the comma part is not three digits.
        ("2,5%", "two point five percent"),
    ];
    for (input, want) in cases {
        assert_eq!(spoken(input), want, "{input:?}");
    }
}

#[test]
fn identifiers_are_read_as_digits() {
    let cases = [
        ("CPT 99213", "CPT nine nine two one three"),
        ("CPT code 0001F.", "CPT code zero zero zero one F."),
        (
            "PMID: 31769816.",
            "PMID: three one seven six nine eight one six.",
        ),
        ("pmid 123", "pmid one two three"),
        (
            "NCT 04368728",
            "NCT zero four three six eight seven two eight",
        ),
        (
            "ZIP 97239-1234",
            "ZIP nine seven two three nine-one two three four",
        ),
        ("zip code 97239", "zip code nine seven two three nine"),
        (
            "doi:10.1038/nature12373.",
            "doi: one zero dot one zero three eight slash nature one two three seven three.",
        ),
        (
            "See https://doi.org/10.1000/182 today",
            "See https: slash slash doi.org slash one zero dot one zero zero zero slash one eight two today",
        ),
        (
            "ISBN 978-0-306-40615-7",
            "ISBN nine seven eight-zero-three zero six-four zero six one five-seven",
        ),
        (
            "ISBN-10: 0-8044-2957-X",
            "ISBN-10: zero-eight zero four four-two nine five seven-X",
        ),
        (
            "Phone: (503) 494-8311",
            "Phone: (five zero three) four nine four-eight three one one",
        ),
        ("tel. 555-0123", "tel. five five five-zero one two three"),
        (
            "Call 503-555-0123 or 503.555.0123.",
            "Call five zero three-five five five-zero one two three or five zero three dot five five five dot zero one two three.",
        ),
    ];
    for (input, want) in cases {
        assert_eq!(spoken(input), want, "{input:?}");
    }
    // Not identifiers: amounts stay amounts.
    for (input, want) in [
        (
            "In 2024 we saw 31769816 cases.",
            "In twenty twenty-four we saw thirty-one million seven hundred sixty-nine thousand eight hundred sixteen cases.",
        ),
        (
            // Not five digits: read as numbers are (here as a year).
            "ZIP 1234 files",
            "ZIP twelve thirty-four files",
        ),
        (
            "ISBN 12345",
            "ISBN twelve thousand three hundred forty-five",
        ),
        ("on 2024-03-15", "on March fifteenth, twenty twenty-four"),
    ] {
        assert_eq!(spoken(input), want, "{input:?}");
    }
}

#[test]
fn identifier_digits_highlight_themselves() {
    let text = "See PMID 31769816 now.";
    let u = pipeline_utterance(text, &settings());
    assert_eq!(
        u.text,
        "See PMID three one seven six nine eight one six now."
    );
    assert_eq!(highlighted(text, &u, "three"), "3");
    assert_eq!(highlighted(text, &u, "seven"), "7");
    assert_eq!(highlighted(text, &u, "six now"), "6 now");
    assert_eq!(highlighted(text, &u, "PMID"), "PMID");
    let text = "doi:10.1000/182";
    let u = pipeline_utterance(text, &settings());
    assert_eq!(highlighted(text, &u, "dot"), ".");
    // Every word of the identifier, as an engine reports it.
    let text = "ZIP 97239";
    let u = pipeline_utterance(text, &settings());
    let mut at = 0u32;
    let mut words: Vec<String> = Vec::new();
    for w in u.text.split(' ') {
        let r = u.source_for(at..at + w.len() as u32).unwrap_or_default();
        words.push(
            text.chars()
                .skip(r.start.0 - 1000)
                .take(r.end.0 - r.start.0)
                .collect(),
        );
        at += w.len() as u32 + 1;
    }
    assert_eq!(words, ["ZIP", "9", "7", "2", "3", "9"]);
}

#[test]
fn error_prone_abbreviations_are_spelled_never_expanded() {
    let cases = [
        ("Give QD.", "Give Q D."),
        (
            "Give QOD and q.o.d. and qod",
            "Give Q O D and Q O D and Q O D",
        ),
        ("Take 1 tab q.d. Then rest.", "Take 1 tab Q D. Then rest."),
        ("Take q.d. with food", "Take Q D with food"),
        ("Q.D. or Q.O.D.", "Q D or Q O D."),
        ("10 IU and 10 U", "10 I U and 10 U"),
        ("MS, MSO4 and MgSO4", "M S, M S O 4 and M G S O 4"),
        ("5 cc of saline", "5 C C of saline"),
        ("give 25µg now", "give 25 micrograms now"),
        ("SC, SQ, HS, hs, TIW", "S C, S Q, H S, H S, T I W"),
        ("AD AS AU OD OS OU", "A D A S A U O D O S O U"),
        ("2 drops a.u. daily", "2 drops A U daily"),
        ("TPA and HCTZ", "T P A and H C T Z"),
        // Everyday words and look-alikes stay as written.
        ("SUCH AS THIS", "SUCH AS THIS"),
        ("as is the msg; cc'd; QDs", "as is the msg; cc'd; QDs"),
        ("Ask Dr. Lee", "Ask Doctor Lee"),
    ];
    for (input, want) in cases {
        assert_eq!(spoken(input), want, "{input:?}");
    }
    // Neither the built-ins nor the user's entries can expand them.
    let custom = [
        ("QD", "every day"),
        ("MS", "morphine sulfate"),
        ("PRN", "as needed"),
    ];
    assert_eq!(abbrev("QD, MS, PRN", &custom), "Q D, M S, as needed");
    // The spelled word highlights the abbreviation.
    let text = "Take q.o.d. now";
    let u = pipeline_utterance(text, &settings());
    assert_eq!(highlighted(text, &u, "Q O D"), "q.o.d.");
    assert_eq!(highlighted(text, &u, "now"), "now");
    for abbr in ERROR_PRONE_ABBREVIATIONS {
        let out = abbrev(&format!("x {abbr} y"), &[]);
        assert!(
            out == format!("x {abbr} y") || !out.contains(abbr),
            "{abbr:?} -> {out:?}"
        );
    }
}

#[test]
fn native_engines_still_get_the_clinical_guards() {
    let p = Pipeline::for_settings(&settings(), PunctuationLevel::Some, false, true);
    assert!(p.names().contains(&"identifiers"));
    assert!(p.names().contains(&"abbreviations"));
    let (out, map) = p.apply_text("PMID 123, QD, $5 at 3:45 in 2024");
    map.check_invariants(&out).unwrap();
    assert_eq!(out, "PMID one two three, Q D, $5 at 3:45 in 2024");
}

#[test]
fn symbols_outside_math_are_named() {
    let cases = [
        ("ΔG and β-blocker", "delta G and beta-blocker"),
        ("5α-reductase", "5 alpha-reductase"),
        ("(γ) and π", "(gamma) and pi"),
        ("5 µm and 5 μm; μ", "5 micro m and 5 micro m; mu"),
        ("−5 and 3 − 2", "minus 5 and 3 minus 2"),
        ("A ⇌ B", "A in equilibrium with B"),
        (
            "10⁹ cells and 10⁻³ M",
            "ten to the ninth cells and ten to the negative third M",
        ),
        (
            "1.5 x 10^9 or 2*10^(-3)",
            "one point five times ten to the ninth or 2 times ten to the negative third",
        ),
        ("box 10^9", "box ten to the ninth"),
        ("210^3 and 10^x", "210 caret 3 and 10 caret x"),
    ];
    for (input, want) in cases {
        assert_eq!(spoken(input), want, "{input:?}");
    }
    // Arrows when math is off; the math transform reads "→" in prose as
    // "approaches", and that is unchanged.
    let no_math = NormalizeConfig {
        math: false,
        ..settings()
    };
    assert_eq!(
        pipeline_utterance("A → B", &no_math).text,
        "A right arrow B"
    );
    assert_eq!(spoken("A → B"), "A approaches B");
    // Letters are words: named at every punctuation level.
    for level in [
        PunctuationLevel::None,
        PunctuationLevel::Some,
        PunctuationLevel::All,
    ] {
        let (out, map) = Punctuation::new(level).apply("TNF-α");
        map.check_invariants(&out).unwrap();
        assert_eq!(out, "TNF alpha", "{level:?}");
    }
    // Highlights.
    let text = "WBC 11.5 × 10^9/L";
    let u = pipeline_utterance(text, &settings());
    assert_eq!(highlighted(text, &u, "ten to the ninth"), "10^9");
    assert_eq!(highlighted(text, &u, "times"), "×");
    let text = "TNF-α level";
    let u = pipeline_utterance(text, &settings());
    assert_eq!(highlighted(text, &u, "alpha"), "-α");
    assert_eq!(char_name('α'), Some("alpha"));
    assert_eq!(char_name('µ'), Some("micro"));
    assert_eq!(char_name('−'), Some("minus"));
    assert_eq!(char_name('⇌'), Some("in equilibrium with"));
}

// Health sciences: the medical pronunciation layer (report sections 2 and 3,
// section 7 items 3 and 5).

/// The default pipeline with the medical lexicon on.
fn medical_settings() -> NormalizeConfig {
    NormalizeConfig {
        medical_lexicon: MedicalLexiconConfig {
            enabled: true,
            overlay: None,
        },
        ..settings()
    }
}

/// The report's twenty test terms, through the whole pipeline with the
/// medical lexicon on: each read as the respelling the lexicon carries,
/// each highlighting its own word.
#[test]
fn medical_vectors() {
    let terms = [
        ("acetaminophen", "uh-SEE-tuh-MIN-uh-fen"),
        ("atorvastatin", "uh-TOR-vuh-STAT-in"),
        ("metoprolol", "meh-TOE-pruh-lol"),
        ("warfarin", "WAR-fuh-rin"),
        ("lisinopril", "lye-SIN-oh-pril"),
        ("hydroxyzine", "hye-DROK-sih-zeen"),
        ("hydralazine", "hye-DRAL-uh-zeen"),
        ("ceftriaxone", "sef-try-AKS-own"),
        ("phenytoin", "FEN-ih-toyn"),
        ("levetiracetam", "lee-veh-tye-RASS-eh-tam"),
        ("furosemide", "fyoo-ROH-seh-mide"),
        ("dyspnea", "DISP-nee-uh"),
        ("ischemia", "is-KEE-mee-uh"),
        ("cholecystitis", "koh-lee-sis-TYE-tis"),
        ("creatinine", "kree-AT-ih-neen"),
        ("sphygmomanometer", "sfig-moh-muh-NOM-eh-ter"),
        ("Guillain-Barré", "ghee-YAN bah-RAY"),
        ("Sjögren", "SHOW-grin"),
        ("Raynaud", "ray-NOH"),
        ("ileum", "ILL-ee-um"),
        ("ilium", "ILL-ee-um"),
        ("Wernicke", "VER-nih-kuh"),
    ];
    let cfg = medical_settings();
    for (term, want) in terms {
        let text = format!("Note {term} today.");
        let u = pipeline_utterance(&text, &cfg);
        assert_eq!(u.text, format!("Note {want} today."), "{term}");
        assert_eq!(highlighted(&text, &u, want), term, "{term}");
        assert_eq!(highlighted(&text, &u, "today"), "today", "{term}");
    }
    // The dosing line of the report's table, with the lexicon on.
    assert_eq!(
        pipeline_utterance("Give 5 mg q6h PRN for pain.", &cfg).text,
        "Give 5 milligrams every 6 hours as needed for pain."
    );
    // Off by default.
    assert_eq!(spoken("Give warfarin."), "Give warfarin.");
    assert!(
        !Pipeline::for_settings(&settings(), PunctuationLevel::Some, false, false)
            .names()
            .contains(&"medical_lexicon")
    );
}

#[test]
fn medical_lexicon_runs_after_the_users_entries_and_before_the_community_lexicon() {
    let mut cfg = medical_settings();
    cfg.pronunciations
        .insert("warfarin".into(), "WAR farin".into());
    cfg.community_lexicon.enabled = true;
    let names = Pipeline::for_settings(&cfg, PunctuationLevel::Some, false, false).names();
    let at = |n: &str| names.iter().position(|x| *x == n).unwrap();
    assert!(at("pronunciations") < at("medical_lexicon"));
    assert!(at("medical_lexicon") < at("abbreviations"));
    if names.contains(&"community_lexicon") {
        assert!(at("medical_lexicon") < at("community_lexicon"));
    }
    // The user's own entry wins.
    assert_eq!(
        pipeline_utterance("Give warfarin.", &cfg).text,
        "Give WAR farin."
    );
    // Engines that normalize natively get it too.
    let native = Pipeline::for_settings(&cfg, PunctuationLevel::Some, false, true);
    assert!(native.names().contains(&"medical_lexicon"));
}

#[test]
fn a_user_overlay_entry_wins_over_the_bundled_tier() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("medical-lexicon.toml");
    std::fs::write(
        &path,
        "warfarin = \"WAR-far-in\"\n[dosing]\nPRN = \"when needed\"\n",
    )
    .unwrap();
    let cfg = NormalizeConfig {
        medical_lexicon: MedicalLexiconConfig {
            enabled: true,
            overlay: Some(path),
        },
        ..settings()
    };
    let text = "Warfarin PRN, metformin";
    let u = pipeline_utterance(text, &cfg);
    assert_eq!(u.text, "WAR-far-in when needed, met-FOR-min");
    assert_eq!(highlighted(text, &u, "WAR-far-in"), "Warfarin");
    assert_eq!(highlighted(text, &u, "when needed"), "PRN");
}

#[test]
fn tall_man_names_are_kept_whole() {
    // Split caps on, lexicon off: never "hydr OX Yzine".
    let p = Pipeline::for_settings(&settings(), PunctuationLevel::Some, true, false);
    let (out, map) = p.apply_text("Not hydrOXYzine or DOPamine, but camelCase.");
    map.check_invariants(&out).unwrap();
    assert_eq!(out, "Not hydrOXYzine or DOPamine, but camel Case.");
    // Lexicon on: the respelling, or the plain name.
    let p = Pipeline::for_settings(&medical_settings(), PunctuationLevel::Some, true, false);
    let (out, map) = p.apply_text("hydrOXYzine, hydrALAZINE and cycloSPORINE");
    map.check_invariants(&out).unwrap();
    assert_eq!(out, "hye-DROK-sih-zeen, hye-DRAL-uh-zeen and cyclosporine");
}

#[test]
fn clinical_units_are_said_in_full() {
    let cases = [
        ("Give 1 mg now", "Give 1 milligram now"),
        ("Give 5mg now", "Give 5 milligrams now"),
        ("25 mcg", "25 micrograms"),
        ("25 µg", "25 micrograms"),
        ("10 mL of saline", "10 milliliters of saline"),
        ("1 mL", "1 milliliter"),
        ("5 mmol/L", "5 millimoles per liter"),
        ("2 mg/kg", "2 milligrams per kilogram"),
        ("15 mg/kg/day", "15 milligrams per kilogram per day"),
        (
            "0.1 mcg/kg/min",
            "zero point one micrograms per kilogram per minute",
        ),
        ("100 mg/dL", "100 milligrams per deciliter"),
        ("1,500 mg", "one thousand five hundred milligrams"),
        ("2,5 mg", "two point five milligrams"),
        ("4 g and 70 kg", "4 grams and 70 kilograms"),
        (
            "37°C or 98.6 °F",
            "37 degrees Celsius or ninety-eight point six degrees Fahrenheit",
        ),
        ("140 mEq/L", "140 milliequivalents per liter"),
        ("120/80 mmHg", "120 over 80 millimeters of mercury"),
        ("5 mm", "5 millimeters"),
        (
            "WBC 11.5 × 10^9/L",
            "WBC eleven point five times ten to the ninth per liter",
        ),
        // Not units: part of a word, or no number before.
        ("5 mgs and mg/kg and 5 Lb", "5 mgs and mg slash kg and 5 Lb"),
    ];
    for (input, want) in cases {
        assert_eq!(spoken(input), want, "{input:?}");
    }
    // The number and the unit highlight themselves.
    let text = "Give 5 mg/kg now";
    let u = pipeline_utterance(text, &settings());
    assert_eq!(u.text, "Give 5 milligrams per kilogram now");
    assert_eq!(highlighted(text, &u, "5"), "5");
    assert_eq!(highlighted(text, &u, "milligrams"), " mg/kg");
    assert_eq!(highlighted(text, &u, "now"), "now");
    let text = "BP 120/80 mmHg.";
    let u = pipeline_utterance(text, &settings());
    assert_eq!(highlighted(text, &u, "over"), "/");
    assert_eq!(highlighted(text, &u, "80"), "80");
}

#[test]
fn en_dash_ranges_are_read_as_to() {
    let cases = [
        ("6–8 weeks", "6 to 8 weeks"),
        ("6 – 8 weeks", "6 to 8 weeks"),
        ("give 6–8 mg", "give 6 to 8 milligrams"),
        ("1.5–2.5 mL", "one point five to two point five milliliters"),
        ("pages 12–15", "pages 12 to 15"),
        ("1990–1995", "nineteen ninety to nineteen ninety-five"),
        // Not ranges: a hyphen.
        ("6-8 weeks", "6-8 weeks"),
    ];
    for (input, want) in cases {
        assert_eq!(spoken(input), want, "{input:?}");
    }
    let text = "taper over 6–8 weeks";
    let u = pipeline_utterance(text, &settings());
    assert_eq!(highlighted(text, &u, "to"), "–");
    assert_eq!(highlighted(text, &u, "8"), "8");
}

#[test]
fn twenty_four_hour_times_gain_no_am_or_pm() {
    let cases = [
        ("at 08:05", "at eight oh five"),
        ("at 15:30", "at fifteen thirty"),
        ("at 15:00", "at fifteen hundred"),
        ("at 08:00", "at oh eight hundred"),
        ("at 00:30", "at zero thirty"),
        ("at 00:00", "at midnight"),
        (
            "at 23:59:30",
            "at twenty-three fifty-nine and thirty seconds",
        ),
        // Twelve-hour times keep their reading.
        ("at 3:45 PM", "at three forty-five PM"),
        ("at 8:05 a.m. today", "at eight oh five AM today"),
        ("at 12:00", "at noon"),
        ("John 3:16", "John three sixteen"),
    ];
    for (input, want) in cases {
        assert_eq!(spoken(input), want, "{input:?}");
    }
}

/// Offset maps stay exact through every new rule, at every punctuation
/// level and with split caps.
#[test]
fn clinical_rules_keep_the_offset_map_exact() {
    let samples = [
        "Give 5 mg q6h PRN; 2 mg/kg/day; 120/80 mmHg; 6–8 weeks at 08:05.",
        "hydrOXYzine 25 mg, WARFARIN 2,5 mg, Raynaud's; 10^9/L; 2,5%.",
        "Guillain-Barré and Sjögren’s; 1,500 mg at 15:30; 37°C.",
    ];
    for cfg in [settings(), medical_settings()] {
        for level in [
            PunctuationLevel::None,
            PunctuationLevel::Some,
            PunctuationLevel::All,
        ] {
            for split in [false, true] {
                let p = Pipeline::for_settings(&cfg, level, split, false);
                for s in samples {
                    let u = p.apply(Utterance::literal(s, CharPos(7)));
                    u.offset_map
                        .check_invariants(&u.text)
                        .unwrap_or_else(|e| panic!("{e} for {s:?} -> {:?}", u.text));
                    let chars: Vec<char> = s.chars().collect();
                    for sp in u
                        .offset_map
                        .spans()
                        .iter()
                        .filter(|x| x.kind == SpanKind::Literal)
                    {
                        let spoken = &u.text[sp.spoken.start as usize..sp.spoken.end as usize];
                        let src: String = chars[sp.source.start.0 - 7..sp.source.end.0 - 7]
                            .iter()
                            .collect();
                        assert_eq!(spoken, src, "{s:?} -> {:?}", u.text);
                    }
                }
            }
        }
    }
}

/// `apply_changed` agrees with `apply`, and is `None` exactly when the text
/// is unchanged.
#[test]
fn apply_changed_matches_apply() {
    let cfg = medical_settings();
    let transforms: Vec<Box<dyn Transform>> = vec![
        Box::new(Abbreviations::new(&BTreeMap::new())),
        Box::new(Numbers::default()),
        Box::new(Numbers::identifiers_only()),
        Box::new(Punctuation::new(PunctuationLevel::Some)),
        Box::new(SplitCaps),
        Box::new(MedicalLexicon::from_config(&cfg.medical_lexicon).unwrap()),
    ];
    for s in [
        "",
        "plain words only",
        "Dr. Lee gave 5 mg at 08:05, QD.",
        "camelCase hydrOXYzine PMID 123",
    ] {
        for t in &transforms {
            let full = t.apply(s);
            match t.apply_changed(s) {
                None => assert_eq!(full.0, s, "{} on {s:?}", t.name()),
                Some(changed) => {
                    assert_ne!(changed.0, s, "{} on {s:?}", t.name());
                    assert_eq!(changed, full, "{} on {s:?}", t.name());
                }
            }
        }
    }
}
