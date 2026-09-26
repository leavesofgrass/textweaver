//! Star's `tests/test_ttstext.py` vectors (docs/star-parity.md Part 2
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
        ("at 15:30", "at three thirty PM"),
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
    // Star: "at zero thirty AM"; twelve-hour clocks have no hour zero.
    assert_eq!(numbers("at 00:30"), "at twelve thirty AM");
    // Star pinned "v1.two point three"; dotted sequences are read by part.
    assert_eq!(numbers("v1.2.3"), "v 1 dot 2 dot 3");
}

#[test]
fn star_number_bugs_are_fixed() {
    // Q1 times.
    assert_eq!(numbers("at 3:45 today"), "at three forty-five AM today");
    assert_eq!(numbers("3:00 amazing"), "three AM amazing");
    assert_eq!(numbers("John 3:16 says"), "John three sixteen says");
    assert_eq!(numbers("08:05"), "eight oh five AM");
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
    // T1 (:216-236). Star merged the list into the paragraph before it (Q5);
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
    // tests/test_ttstext.py:351-374. Star's undelimited vectors are
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
