//! Math in BRF files (ADR-0036): Nemeth between the switch indicators, or
//! UEB, wrapped to 40 cells without separating an indicator from what it
//! modifies; and the fallback (spoken words), said once in the report.

use textweaver_formats::{LoadOptions, Loader, MarkdownLoader, Source};
use textweaver_text::Document;
use textweaver_writers::{BrailleOptions, Format, MathCode, WriteOptions, write_to_vec};

fn md(src: &str) -> Document {
    MarkdownLoader
        .load(
            &Source::Bytes {
                data: src.as_bytes().to_vec(),
                hint: "md".into(),
            },
            &LoadOptions::default(),
        )
        .expect("markdown loads")
}

fn brf(src: &str, code: MathCode) -> (Vec<String>, Vec<String>) {
    let options = WriteOptions {
        braille: BrailleOptions {
            math_code: code,
            page_numbers: false,
            ..BrailleOptions::default()
        },
        ..WriteOptions::default()
    };
    let (bytes, report) = write_to_vec(&md(src), Format::Brf, &options).expect("writes");
    let text = String::from_utf8(bytes).expect("ASCII");
    let lines = text
        .split("\r\n")
        .map(|l| l.trim_end_matches('\u{c}').to_owned())
        .filter(|l| !l.is_empty())
        .collect();
    (lines, report.warnings)
}

/// `fixtures/c4/<name>`.
#[cfg(feature = "mathcat")]
fn fixture(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/c4")
        .join(name)
}

/// The quadratic fixture as BRF in both codes, against the reviewed
/// snapshots in `fixtures/c4` (set `TW_BLESS=1` to write them again, then
/// read the difference before committing).
#[cfg(feature = "mathcat")]
#[test]
fn quadratic_fixture_matches_its_snapshots() {
    let src = std::fs::read_to_string(fixture("quadratic.md")).expect("fixture");
    for (code, name) in [
        (MathCode::Nemeth, "quadratic.nemeth.brf"),
        (MathCode::Ueb, "quadratic.ueb.brf"),
    ] {
        let options = WriteOptions {
            braille: BrailleOptions {
                math_code: code,
                ..BrailleOptions::default()
            },
            ..WriteOptions::default()
        };
        let (bytes, report) = write_to_vec(&md(&src), Format::Brf, &options).expect("writes");
        assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
        let path = fixture(name);
        if std::env::var_os("TW_BLESS").is_some() {
            std::fs::write(&path, &bytes).expect("write snapshot");
            continue;
        }
        let want = std::fs::read(&path).expect("snapshot");
        assert!(
            bytes == want,
            "{name} differs:\n{}",
            String::from_utf8_lossy(&bytes)
        );
    }
}

#[cfg(feature = "mathcat")]
mod with_mathcat {
    use textweaver_writers::ueb;

    use super::*;

    /// Nemeth in UEB text: the opening indicator (dots 456, 146) and a
    /// space, the math, a space and the terminator (dots 456, 156), then
    /// the sentence's period in UEB (BANA, Guidance for Transcription
    /// Using the Nemeth Code within UEB Contexts).
    #[test]
    fn nemeth_sits_between_the_switch_indicators() {
        let (lines, warnings) = brf("So $\\frac{a+b}{c}$.\n", MathCode::Nemeth);
        assert!(warnings.is_empty(), "{warnings:?}");
        let line = lines[0].trim_start();
        let math = line
            .strip_prefix(",SO _% ")
            .and_then(|l| l.strip_suffix(" _:4"))
            .unwrap_or_else(|| panic!("{line}"));
        // Rule 62.a, example 3 of the Nemeth Code.
        assert_eq!(ueb::to_unicode(math), "⠹⠁⠬⠃⠌⠉⠼");
    }

    #[test]
    fn ueb_math_has_no_switch_indicators() {
        let (lines, warnings) = brf("So $x+y=6$.\n", MathCode::Ueb);
        assert!(warnings.is_empty(), "{warnings:?}");
        let line = lines[0].trim_start();
        assert!(!line.contains("_%"), "{line}");
        // BANA's UEB guidance, section 5, example 1, in uncontracted text.
        assert!(line.contains(&ueb::from_unicode("⠭⠐⠖⠽⠀⠐⠶⠀⠼⠋")), "{line}");
    }

    #[test]
    fn a_number_alone_stays_in_ueb() {
        let (lines, _) = brf("It is $42$ today.\n", MathCode::Nemeth);
        assert!(!lines[0].contains("_%"), "{lines:?}");
        assert!(lines[0].contains("#DB"), "{lines:?}");
    }

    /// A long formula wraps at 40 cells: no line is longer, the opening
    /// indicator is never alone at a line's end, the terminator never
    /// starts a line, and no line ends in an indicator.
    #[test]
    fn long_math_wraps_without_separating_indicators() {
        let long = (1..=14)
            .map(|i| format!("x_{{{i}}}^2"))
            .collect::<Vec<_>>()
            .join(" + ");
        for code in [MathCode::Nemeth, MathCode::Ueb] {
            let (lines, warnings) = brf(&format!("Sum: ${long} = y$.\n"), code);
            assert!(warnings.is_empty(), "{warnings:?}");
            println!(
                "{code:?}:
{}",
                lines.join(
                    "
"
                )
            );
            assert!(lines.len() > 2, "{lines:?}");
            for l in &lines {
                assert!(l.len() <= 40, "{code:?} {l:?} is {} cells", l.len());
                assert!(!l.trim_end().ends_with("_%"), "{code:?} {lines:?}");
                assert!(!l.trim_start().starts_with("_:"), "{code:?} {lines:?}");
                let last = l.trim_end().chars().last().unwrap_or(' ');
                assert!(
                    !";^.@_+".contains(last),
                    "{code:?}: {l:?} ends in an indicator or a sign"
                );
            }
            if code == MathCode::Nemeth {
                let all = lines.join(" ");
                assert_eq!(all.matches("_%").count(), 1, "{all}");
                assert_eq!(all.matches("_:").count(), 1, "{all}");
            }
        }
    }

    #[test]
    fn a_formula_mathcat_cannot_read_is_written_in_words_said_once() {
        let (lines, warnings) = brf(
            // A double superscript and a \left without \right: the parser
            // repairs both and says so, so they are read as words.
            "Broken $x^2^3$ and $\\left( x$ here.\n",
            MathCode::Nemeth,
        );
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(
            warnings[0].starts_with("2 formulas are in spoken words"),
            "{warnings:?}"
        );
        assert!(warnings[0].contains("Nemeth"), "{warnings:?}");
        assert!(!lines.join(" ").contains("_%"), "{lines:?}");
    }
}

#[cfg(not(feature = "mathcat"))]
#[test]
fn without_mathcat_math_is_spoken_words_said_once() {
    let (lines, warnings) = brf("The area is $\\pi r^2$ or $x$.\n", MathCode::Nemeth);
    assert_eq!(
        warnings,
        ["2 formulas are in spoken words: this build has no math braille."]
    );
    let spoken = brf("The area is pi r squared or x.\n", MathCode::Nemeth).0;
    assert_eq!(lines, spoken);
}
