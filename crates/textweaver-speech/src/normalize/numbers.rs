//! Numbers, dates, times, and currency, ported from
//! `star/ttstext/numbers.py` (`_normalize_numbers`, Part 2 section 5.B.3) in
//! Star's order, each step a [`Rule`] with an offset map.
//!
//! Deliberate fixes of Star's quirks (Part 2 section 7.2):
//!
//! - Q1 times: whitespace is consumed only before an actual AM/PM, so
//!   "3:45 today" stays two words; `am`/`pm` must be whole words ("3:00
//!   amazing"); "a.m." and "p.m." are recognized ("9:30 a.m.") and a
//!   sentence-final "a.m." keeps its period; minutes below ten get "oh"
//!   ("9:05" is "nine oh five AM"); hour zero with AM is "twelve"
//!   ("00:30 AM" is "twelve thirty AM", where Star said "zero thirty
//!   AM"; 24-hour times are below); seconds are
//!   spoken instead of dropped; a verse reference after a book of the Bible
//!   ("John 3:16") is read as chapter and verse, not a time.
//! - Q2 currency: a sentence period is not eaten ("It costs $5."); more than
//!   two decimals are read as a decimal amount ("$3.14159"); pounds use
//!   "penny"/"pence".
//! - Q3 decimals and years: a sentence-final year is read ("in 2024.");
//!   comma numbers take decimals ("1,234.5"); dotted sequences such as
//!   versions and addresses ("v1.2.3", "192.168.1.1") are read part by part
//!   with "dot" (Star pinned "v1.two point three" in a test; this is the
//!   deliberate replacement); ".5" is "point five"; decades ("the 2020s")
//!   are plural years.
//!
//! **Identifiers are read as digits** (new in textweaver; Star read them as
//! amounts). Before the rules, a guard finds the number after a label that
//! says it is an identifier, and spells it digit by digit, so "PMID
//! 31769816" is "PMID three one seven six nine eight one six", never "thirty-one
//! million ...". The labels: CPT (five characters, such as "99213" or
//! "0001F"), PMID, NCT (eight digits, also written as one word,
//! "NCT04368728"), ZIP (five digits, or ZIP+4), DOI (`doi:`, `DOI` or a
//! `doi.org/` link before `10.`), ISBN (ten or thirteen digits, with
//! hyphens or spaces), and phone, telephone, tel and fax. A US phone number
//! in its usual shapes ("503-555-0123", "(503) 555-0123", "503.555.0123")
//! is read as digits without a label. Each digit word maps to its own
//! digit, so the highlight follows the digits as they are read. Dots
//! inside an identifier are read as "dot"; hyphens, slashes and letters
//! stay as written for the punctuation step and the engine.
//!
//! **Clinical units, ranges and times** (new in textweaver; health
//! sciences report, section 3). Before Star's rules:
//!
//! - a unit after a number is said in full: "5 mg" is "5 milligrams", "1
//!   mg" "1 milligram", and mcg, ng, g, kg, mL, dL, L, mmol, mEq, mm, cm,
//!   °C and °F likewise; a slash after the unit is "per" ("2 mg/kg/day"
//!   is "2 milligrams per kilogram per day", "5 mmol/L" "5 millimoles per
//!   liter");
//! - "120/80 mmHg" is "120 over 80 millimeters of mercury", and a count
//!   per volume "10^9/L" is "10^9 per liter";
//! - an en dash between numbers is a range: "6–8 weeks" is "6 to 8
//!   weeks";
//! - a decimal comma before a unit or a percent sign ("2,5%", "2,5 mg"),
//!   whose comma part is not three digits, is read as a decimal point;
//!   "1,000%" is "one thousand percent".
//!
//! The number itself stays as written for the rules below (and for the
//! engine, which reads small integers itself), so "5 milligrams" still
//! highlights "5" and then "mg".
//!
//! **24-hour times gain no AM or PM** (a deliberate change to Star, which
//! read "15:30" as "three thirty PM"): a time whose hour is written with a
//! leading zero or is 13 or more, with no AM or PM after it, is read as
//! written on a 24-hour clock: "08:05" is "eight oh five", "15:30"
//! "fifteen thirty", "15:00" "fifteen hundred", "08:00" "oh eight
//! hundred", and "00:30" "zero thirty". A time such as "3:45" keeps
//! Star's reading.

use regex::{Captures, Regex};

use super::Transform;
use super::rewrite::{Piece, Rule, apply_rules_changed, char_after, char_before, then};
use super::words::{
    MONTHS, decimal_digits_to_words, int_to_words, ordinal_to_words, pluralize_last_word,
    year_to_words,
};
use textweaver_core::{CharPos, CharRange, OffsetMap, SpokenBuilder};

fn text(s: impl Into<String>) -> Option<Vec<Piece>> {
    Some(vec![Piece::Text(s.into())])
}

fn num(c: &Captures<'_>, i: usize) -> Option<i64> {
    c.get(i)?.as_str().replace(',', "").parse().ok()
}

fn date_words(year: i64, month: i64, day: i64) -> Option<String> {
    let m = MONTHS.get(usize::try_from(month - 1).ok()?)?;
    Some(format!(
        "{m} {}, {}",
        ordinal_to_words(day),
        year_to_words(year)
    ))
}

/// Books of the Bible and common abbreviations, for "John 3:16".
const BOOKS: &[&str] = &[
    "Genesis",
    "Gen",
    "Exodus",
    "Exod",
    "Ex",
    "Leviticus",
    "Lev",
    "Numbers",
    "Num",
    "Deuteronomy",
    "Deut",
    "Joshua",
    "Josh",
    "Judges",
    "Judg",
    "Ruth",
    "Samuel",
    "Sam",
    "Kings",
    "Kgs",
    "Chronicles",
    "Chr",
    "Ezra",
    "Nehemiah",
    "Neh",
    "Esther",
    "Esth",
    "Job",
    "Psalm",
    "Psalms",
    "Ps",
    "Proverbs",
    "Prov",
    "Ecclesiastes",
    "Eccl",
    "Song",
    "Songs",
    "Isaiah",
    "Isa",
    "Jeremiah",
    "Jer",
    "Lamentations",
    "Lam",
    "Ezekiel",
    "Ezek",
    "Daniel",
    "Dan",
    "Hosea",
    "Hos",
    "Joel",
    "Amos",
    "Obadiah",
    "Obad",
    "Jonah",
    "Micah",
    "Mic",
    "Nahum",
    "Nah",
    "Habakkuk",
    "Hab",
    "Zephaniah",
    "Zeph",
    "Haggai",
    "Hag",
    "Zechariah",
    "Zech",
    "Malachi",
    "Mal",
    "Matthew",
    "Matt",
    "Mt",
    "Mark",
    "Mk",
    "Luke",
    "Lk",
    "John",
    "Jn",
    "Acts",
    "Romans",
    "Rom",
    "Corinthians",
    "Cor",
    "Galatians",
    "Gal",
    "Ephesians",
    "Eph",
    "Philippians",
    "Phil",
    "Colossians",
    "Col",
    "Thessalonians",
    "Thess",
    "Timothy",
    "Tim",
    "Titus",
    "Philemon",
    "Phlm",
    "Hebrews",
    "Heb",
    "James",
    "Jas",
    "Peter",
    "Pet",
    "Jude",
    "Revelation",
    "Rev",
];

/// True when the word right before byte `start` (separated by one space) is
/// a book of the Bible.
fn after_book(s: &str, start: usize) -> bool {
    let Some(before) = s[..start].strip_suffix(' ') else {
        return false;
    };
    let before = before.strip_suffix('.').unwrap_or(before);
    let word_start = before
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_alphabetic())
        .map_or(0, |(i, c)| i + c.len_utf8());
    let word = &before[word_start..];
    BOOKS.contains(&word)
}

/// True when the text after byte `end` ends a sentence: end of text, a line
/// break, or whitespace followed by a capital letter.
pub(crate) fn sentence_ends_at(s: &str, end: usize) -> bool {
    let rest = &s[end..];
    match rest.chars().next() {
        None | Some('\n') => true,
        Some(c) if c.is_whitespace() => rest
            .trim_start()
            .chars()
            .next()
            .is_none_or(char::is_uppercase),
        _ => false,
    }
}

fn time_words(h: i64, m: i64, s: Option<i64>, explicit: Option<char>) -> String {
    let secs = s.filter(|&s| s > 0);
    if secs.is_none() && explicit.is_none() && m == 0 {
        if h == 0 {
            return "midnight".into();
        }
        if h == 12 {
            return "noon".into();
        }
    }
    let (h12, period) = match explicit {
        Some('p') if h < 12 => (h, "PM"),
        Some('a') => (if h % 12 == 0 { 12 } else { h % 12 }, "AM"),
        _ => {
            let period = if h < 12 { "AM" } else { "PM" };
            let h12 = if h == 0 {
                12
            } else if h <= 12 {
                h
            } else {
                h - 12
            };
            (h12, period)
        }
    };
    let mut out = int_to_words(h12);
    if m > 0 && m < 10 {
        out.push_str(" oh ");
        out.push_str(&int_to_words(m));
    } else if m > 0 {
        out.push(' ');
        out.push_str(&int_to_words(m));
    }
    out.push(' ');
    out.push_str(period);
    if let Some(s) = secs {
        out.push_str(&format!(
            " and {} second{}",
            int_to_words(s),
            if s == 1 { "" } else { "s" }
        ));
    }
    out
}

/// A 24-hour time as said aloud: "08:05" is "eight oh five", "15:30"
/// "fifteen thirty", "15:00" "fifteen hundred", "08:00" "oh eight
/// hundred", "00:00" "midnight". No AM or PM is added.
fn time_words_24(h: i64, m: i64, s: Option<i64>, leading_zero: bool) -> String {
    let secs = s.filter(|&s| s > 0);
    if h == 0 && m == 0 && secs.is_none() {
        return "midnight".into();
    }
    let mut out = int_to_words(h);
    if m == 0 {
        if leading_zero && h > 0 {
            out.insert_str(0, "oh ");
        }
        out.push_str(" hundred");
    } else if m < 10 {
        out.push_str(" oh ");
        out.push_str(&int_to_words(m));
    } else {
        out.push(' ');
        out.push_str(&int_to_words(m));
    }
    if let Some(s) = secs {
        out.push_str(&format!(
            " and {} second{}",
            int_to_words(s),
            if s == 1 { "" } else { "s" }
        ));
    }
    out
}

/// Clinical units after a number: `(symbol, singular, plural)`.
const UNITS: &[(&str, &str, &str)] = &[
    ("mg", "milligram", "milligrams"),
    ("mcg", "microgram", "micrograms"),
    ("ng", "nanogram", "nanograms"),
    ("g", "gram", "grams"),
    ("kg", "kilogram", "kilograms"),
    ("mL", "milliliter", "milliliters"),
    ("ml", "milliliter", "milliliters"),
    ("dL", "deciliter", "deciliters"),
    ("L", "liter", "liters"),
    ("mmol", "millimole", "millimoles"),
    ("mEq", "milliequivalent", "milliequivalents"),
    ("mmHg", "millimeter of mercury", "millimeters of mercury"),
    ("mm", "millimeter", "millimeters"),
    ("cm", "centimeter", "centimeters"),
    ("°C", "degree Celsius", "degrees Celsius"),
    ("°F", "degree Fahrenheit", "degrees Fahrenheit"),
];

/// What a unit is "per" after a slash: "mg/kg" is "milligrams per
/// kilogram", "mcg/kg/min" "micrograms per kilogram per minute".
const PER: &[(&str, &str)] = &[
    ("kg", "kilogram"),
    ("g", "gram"),
    ("L", "liter"),
    ("dL", "deciliter"),
    ("mL", "milliliter"),
    ("min", "minute"),
    ("h", "hour"),
    ("hr", "hour"),
    ("d", "day"),
    ("day", "day"),
    ("dose", "dose"),
    ("m²", "square meter"),
    ("m2", "square meter"),
];

/// A regex alternation of `items`, longest first, so "mmHg" is tried
/// before "mm".
fn alternation<'a>(items: impl Iterator<Item = &'a str>) -> String {
    let mut v: Vec<&str> = items.collect();
    v.sort_by_key(|s| std::cmp::Reverse(s.len()));
    v.iter()
        .map(|s| regex::escape(s))
        .collect::<Vec<_>>()
        .join("|")
}

/// " per kilogram per minute" for "/kg/min".
fn per_words(per: &str) -> String {
    per.split('/')
        .filter(|p| !p.is_empty())
        .filter_map(|p| PER.iter().find(|(sym, _)| *sym == p))
        .map(|(_, name)| format!(" per {name}"))
        .collect()
}

/// A number before a unit or a percent sign written with a decimal comma
/// ("2,5"): the comma part is not three digits, so it is not a thousands
/// separator. Returns the whole part and the decimals.
fn decimal_comma(n: &str) -> Option<(&str, &str)> {
    let (whole, frac) = n.split_once(',')?;
    (!whole.is_empty()
        && !frac.contains(',')
        && frac.len() != 3
        && !frac.contains('.')
        && whole.bytes().all(|b| b.is_ascii_digit())
        && frac.bytes().all(|b| b.is_ascii_digit()))
    .then_some((whole, frac))
}

/// "2,5" as "two point five".
fn decimal_comma_words(whole: &str, frac: &str) -> Option<String> {
    Some(format!(
        "{} point {}",
        int_to_words(whole.parse().ok()?),
        decimal_digits_to_words(frac)
    ))
}

/// Clinical rules that run before the number rules, so the numbers they
/// keep are read by those rules afterwards (health sciences report,
/// section 3).
fn clinical_rules() -> Vec<Rule> {
    let per = alternation(PER.iter().map(|(s, _)| *s));
    vec![
        // Blood pressure: "120/80 mmHg" is "120 over 80 millimeters of
        // mercury".
        Rule::with(r"\b([0-9]{2,3})/([0-9]{2,3})[ \u{a0}]?mmHg", |c, s| {
            let m = c.get(0)?;
            if char_before(s, m.start()).is_some_and(|b| b == '/' || b == '.')
                || char_after(s, m.end()).is_some_and(char::is_alphanumeric)
            {
                return None;
            }
            Some(vec![
                Piece::Keep(1),
                Piece::Text(" over ".into()),
                Piece::Keep(2),
                Piece::Text(" millimeters of mercury".into()),
            ])
        }),
        // A count per volume: "10^9/L" is "10^9 per liter" (the power of
        // ten is read by the punctuation step).
        Rule::with(
            &format!(
                r"(10(?:\^\(?[-\u{{2212}}]?[0-9]{{1,3}}\)?|[⁻]?[⁰¹²³⁴⁵⁶⁷⁸⁹]{{1,3}}))((?:/(?:{per}))+)"
            ),
            |c, s| {
                let m = c.get(0)?;
                if char_before(s, m.start()).is_some_and(|b| b.is_alphanumeric() || b == '.')
                    || char_after(s, m.end()).is_some_and(char::is_alphanumeric)
                {
                    return None;
                }
                Some(vec![
                    Piece::Keep(1),
                    Piece::Text(per_words(c.get(2)?.as_str())),
                ])
            },
        ),
        // Ranges: an en dash between numbers is "to" ("6–8 weeks").
        Rule::with(
            r"([0-9]+(?:\.[0-9]+)?)[ \u{a0}]?\u{2013}[ \u{a0}]?([0-9]+(?:\.[0-9]+)?)",
            |c, s| {
                let m = c.get(0)?;
                let before = char_before(s, m.start());
                let after = char_after(s, m.end());
                if before.is_some_and(|b| b.is_alphanumeric() || matches!(b, '.' | ':' | '/' | ','))
                    || after
                        .is_some_and(|a| a.is_ascii_digit() || matches!(a, ':' | '/' | '\u{2013}'))
                    || dot_digit_at(s, m.end())
                {
                    return None;
                }
                Some(vec![
                    Piece::Keep(1),
                    Piece::Text(" to ".into()),
                    Piece::Keep(2),
                ])
            },
        ),
        // Units after a number: "5 mg" is "5 milligrams", "2 mg/kg" "2
        // milligrams per kilogram".
        Rule::with(
            &format!(
                r"([0-9]{{1,3}}(?:,[0-9]{{3}})+(?:\.[0-9]+)?|[0-9]+(?:[.,][0-9]+)?|\.[0-9]+)[ \u{{a0}}]?({})((?:/(?:{per}))*)",
                alternation(UNITS.iter().map(|(s, _, _)| *s))
            ),
            |c, s| {
                let m = c.get(0)?;
                let n = c.get(1)?.as_str();
                let before = char_before(s, m.start());
                if before.is_some_and(|b| {
                    b.is_alphanumeric() || matches!(b, '.' | ':' | '^' | '_' | '/' | ',')
                }) || char_after(s, m.end()).is_some_and(char::is_alphanumeric)
                {
                    return None;
                }
                let sym = c.get(2)?.as_str();
                let (_, one, many) = UNITS.iter().find(|(u, _, _)| *u == sym)?;
                let unit = if n == "1" { one } else { many };
                let per = per_words(c.get(3).map_or("", |p| p.as_str()));
                if n.contains(',') && !n.contains('.') && n.matches(',').count() == 1 {
                    if let Some((whole, frac)) = decimal_comma(n) {
                        let words = decimal_comma_words(whole, frac)?;
                        return text(format!("{words} {unit}{per}"));
                    }
                    // "1,500 mg" is a thousands separator; "1234,567 mg"
                    // is neither, and stays as written.
                    if n.split(',').next().is_some_and(|w| w.len() > 3) {
                        return None;
                    }
                }
                Some(vec![Piece::Keep(1), Piece::Text(format!(" {unit}{per}"))])
            },
        ),
    ]
}

fn currency_words(symbol: &str, whole: i64, frac: Option<&str>) -> String {
    let (major, majors, minor, minors) = match symbol {
        "£" => ("pound", "pounds", "penny", "pence"),
        "€" => ("euro", "euros", "cent", "cents"),
        _ => ("dollar", "dollars", "cent", "cents"),
    };
    if let Some(f) = frac.filter(|f| f.len() > 2) {
        return format!(
            "{} point {} {majors}",
            int_to_words(whole),
            decimal_digits_to_words(f)
        );
    }
    let cents: i64 = frac
        .map(|f| format!("{f:0<2}"))
        .and_then(|f| f.parse().ok())
        .unwrap_or(0);
    let mut parts = Vec::new();
    if whole != 0 {
        parts.push(format!(
            "{} {}",
            int_to_words(whole),
            if whole == 1 { major } else { majors }
        ));
    }
    if cents != 0 {
        parts.push(format!(
            "{} {}",
            int_to_words(cents),
            if cents == 1 { minor } else { minors }
        ));
    }
    if parts.is_empty() {
        format!("zero {majors}")
    } else {
        parts.join(" and ")
    }
}

/// True when byte `i` of `s` starts `.digit`.
fn dot_digit_at(s: &str, i: usize) -> bool {
    let mut it = s[i..].chars();
    it.next() == Some('.') && it.next().is_some_and(|c| c.is_ascii_digit())
}

/// The rules, in order.
pub(crate) fn rules() -> Vec<Rule> {
    let mut rules = clinical_rules();
    rules.extend(star_rules());
    rules
}

/// Star's rules, in Star's order.
fn star_rules() -> Vec<Rule> {
    vec![
        // 1. ISO date.
        Rule::with(
            r"\b([0-9]{4})-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])\b",
            |c, _| text(date_words(num(c, 1)?, num(c, 2)?, num(c, 3)?)?),
        ),
        // 2. US date.
        Rule::with(
            r"\b(0?[1-9]|1[0-2])/(0?[1-9]|[12][0-9]|3[01])/([0-9]{4})\b",
            |c, _| text(date_words(num(c, 3)?, num(c, 1)?, num(c, 2)?)?),
        ),
        // 3. Times (and verse references).
        Rule::with(
            r"\b([01]?[0-9]|2[0-3]):([0-5][0-9])(?::([0-5][0-9]))?\b(?:[ \t]*(?:([AaPp])[Mm]\b|([AaPp])\.[Mm]\.))?",
            |c, s| {
                let m = c.get(0)?;
                let (h, mi) = (num(c, 1)?, num(c, 2)?);
                let explicit = c
                    .get(4)
                    .or_else(|| c.get(5))
                    .and_then(|x| x.as_str().chars().next())
                    .map(|x| x.to_ascii_lowercase());
                if explicit.is_none() && c.get(3).is_none() && after_book(s, m.start()) {
                    return text(format!("{} {}", int_to_words(h), int_to_words(mi)));
                }
                // A 24-hour time ("08:05", "15:30") gains no AM or PM.
                let hour = c.get(1)?.as_str();
                let leading_zero = hour.len() == 2 && hour.starts_with('0');
                if explicit.is_none() && (leading_zero || h >= 13) {
                    return text(time_words_24(h, mi, num(c, 3), leading_zero));
                }
                let mut w = time_words(h, mi, num(c, 3), explicit);
                if c.get(5).is_some() && sentence_ends_at(s, m.end()) {
                    w.push('.');
                }
                text(w)
            },
        ),
        // 4. Currency.
        Rule::with(
            r"([$£€])([0-9]{1,3}(?:,[0-9]{3})+|[0-9]+)(?:\.([0-9]+))?",
            |c, _| {
                let sym = c.get(1)?.as_str();
                text(currency_words(
                    sym,
                    num(c, 2)?,
                    c.get(3).map(|x| x.as_str()),
                ))
            },
        ),
        // 5. Percent, with thousands ("1,000%") and a decimal comma
        // ("2,5%" is "two point five percent").
        Rule::with(
            r"\b([0-9]{1,3}(?:,[0-9]{3})+|[0-9]+)(?:([.,])([0-9]+))?%",
            |c, _| {
                let int = num(c, 1)?;
                let comma_decimal = c.get(2).is_some_and(|sep| sep.as_str() == ",");
                if comma_decimal && (c.get(1)?.as_str().contains(',') || c.get(3)?.len() == 3) {
                    return None;
                }
                text(match c.get(3) {
                    None => format!("{} percent", int_to_words(int)),
                    Some(d) => format!(
                        "{} point {} percent",
                        int_to_words(int),
                        decimal_digits_to_words(d.as_str())
                    ),
                })
            },
        ),
        // 6. Ordinals.
        Rule::with(r"\b([0-9]+)(?i:st|nd|rd|th)\b", |c, _| {
            text(ordinal_to_words(num(c, 1)?))
        }),
        // 7. Comma numbers, with decimals.
        Rule::with(r"\b([0-9]{1,3}(?:,[0-9]{3})+)(?:\.([0-9]+))?\b", |c, _| {
            let whole = int_to_words(num(c, 1)?);
            text(match c.get(2) {
                None => whole,
                Some(d) => format!("{whole} point {}", decimal_digits_to_words(d.as_str())),
            })
        }),
        // 8. Dotted sequences: versions, addresses.
        Rule::with(r"[0-9]+(?:\.[0-9]+){2,}", |c, s| {
            let m = c.get(0)?;
            let before = char_before(s, m.start());
            if before.is_some_and(|b| b.is_ascii_digit() || b == '.') || dot_digit_at(s, m.end()) {
                return None;
            }
            let parts = m.as_str().split('.').collect::<Vec<_>>().join(" dot ");
            let lead = if before.is_some_and(char::is_alphabetic) {
                " "
            } else {
                ""
            };
            text(format!("{lead}{parts}"))
        }),
        // 9. Decimals.
        Rule::with(r"\b([0-9]+)\.([0-9]+)\b", |c, s| {
            let m = c.get(0)?;
            let before = char_before(s, m.start());
            let after = char_after(s, m.end());
            if before.is_some_and(|b| matches!(b, '0'..='9' | '/' | '-' | '.'))
                || after.is_some_and(|a| matches!(a, '0'..='9' | '/' | '-'))
                || dot_digit_at(s, m.end())
            {
                return None;
            }
            text(format!(
                "{} point {}",
                int_to_words(num(c, 1)?),
                decimal_digits_to_words(c.get(2)?.as_str())
            ))
        }),
        // 9b. Leading-dot decimals: ".5".
        Rule::with(r"\.([0-9]+)\b", |c, s| {
            let m = c.get(0)?;
            let before = char_before(s, m.start());
            if before.is_some_and(|b| !(b.is_whitespace() || b == '(')) {
                return None;
            }
            text(format!(
                "point {}",
                decimal_digits_to_words(c.get(1)?.as_str())
            ))
        }),
        // 10. Decades: "the 1990s".
        Rule::with(r"\b([0-9]{4})s\b", |c, _| {
            let y = num(c, 1)?;
            (1000..=2099)
                .contains(&y)
                .then(|| vec![Piece::Text(pluralize_last_word(&year_to_words(y)))])
        }),
        // 11. Plain integers of four or more digits.
        Rule::with(r"\b([0-9]{4,})\b", |c, s| {
            let m = c.get(0)?;
            if char_before(s, m.start()) == Some('.') || dot_digit_at(s, m.end()) {
                return None;
            }
            let n: i64 = m.as_str().parse().ok()?;
            text(if (1000..=2099).contains(&n) {
                year_to_words(n)
            } else {
                int_to_words(n)
            })
        }),
    ]
}

/// A US phone number: an optional area code in parentheses or before a
/// separator, three digits, a separator, four digits.
const PHONE_LABELED: &str =
    r"(?:\+?1[ .-]?)?(?:\([0-9]{3}\)[ ]?|[0-9]{3}[ .-])?[0-9]{3}[ .-][0-9]{4}";
/// The same without a label: the area code is required, and the
/// separators are hyphens or dots, so a run of numbers in prose is not
/// taken for a phone number.
const PHONE_BARE: &str =
    r"(?:\+1[ .-]?|1[.-])?(?:\([0-9]{3}\)[ ]?|[0-9]{3}[.-])[0-9]{3}[.-][0-9]{4}";

/// The identifier pattern: a label (not spoken differently) and the
/// identifier after it, in a named group by kind.
fn identifier_pattern() -> String {
    let sep = r"[ \t\u{a0}]*";
    [
        format!(r"(?i:\b(?:CPT|HCPCS)(?:[ ]code)?[:\x23]?{sep})(?P<cpt>[0-9]{{4}}[0-9A-Za-z])\b"),
        format!(r"(?i:\bPMID:?{sep})(?P<pmid>[0-9]{{1,9}})\b"),
        format!(r"(?i:\bNCT{sep})(?P<nct>[0-9]{{8}})\b"),
        format!(r"(?i:\bZIP(?:[ ]code)?:?{sep})(?P<zip>[0-9]{{5}}(?:-[0-9]{{4}})?)\b"),
        format!(r"(?i:(?:\bdoi:?{sep}|\bdoi\.org/))(?P<doi>10\.[0-9]{{4,9}}/\S+)"),
        format!(r"(?i:\bISBN(?:-1[03])?:?{sep})(?P<isbn>[0-9][0-9 -]{{8,15}}[0-9Xx])\b"),
        format!(
            r"(?i:\b(?:phone|telephone|tel\.?|fax)(?:[ ](?:number|no\.))?:?{sep})(?P<phone>{PHONE_LABELED})\b"
        ),
        format!(r"(?P<bare>{PHONE_BARE})"),
    ]
    .join("|")
}

/// The identifier guard: spells identifiers digit by digit (see the module
/// docs).
struct Identifiers {
    re: Regex,
}

impl Identifiers {
    fn new() -> Self {
        let re = Regex::new(&identifier_pattern())
            .unwrap_or_else(|e| panic!("bad built-in pattern: {e}"));
        Identifiers { re }
    }

    /// The byte range of the identifier in one match, or `None` when the
    /// match is not one after all.
    fn identifier(c: &Captures<'_>, s: &str) -> Option<std::ops::Range<usize>> {
        if let Some(m) = c.name("doi") {
            // Trailing sentence punctuation is not part of a DOI; a closing
            // parenthesis is, when it closes one inside the DOI.
            let mut end = m.end();
            loop {
                let t = &s[m.start()..end];
                let last = t.chars().next_back()?;
                let unbalanced = last == ')' && t.matches(')').count() > t.matches('(').count();
                if matches!(
                    last,
                    '.' | ',' | ';' | ':' | '!' | '?' | '\'' | '"' | ']' | '>'
                ) || unbalanced
                {
                    end -= last.len_utf8();
                } else {
                    break;
                }
            }
            return Some(m.start()..end);
        }
        if let Some(m) = c.name("isbn") {
            let digits = m
                .as_str()
                .chars()
                .filter(|c| c.is_ascii_digit() || matches!(c, 'X' | 'x'))
                .count();
            let x_last = m
                .as_str()
                .char_indices()
                .all(|(i, c)| !matches!(c, 'X' | 'x') || i + 1 == m.as_str().len());
            return (matches!(digits, 10 | 13) && x_last).then(|| m.range());
        }
        if let Some(m) = c.name("bare") {
            // Not inside a longer run of digits, words or separators.
            let before = char_before(s, m.start());
            let after = char_after(s, m.end());
            let next_digit = s[m.end()..]
                .chars()
                .nth(1)
                .is_some_and(|c| c.is_ascii_digit());
            if before.is_some_and(|b| b.is_alphanumeric() || matches!(b, '-' | '.' | '/' | '+'))
                || after.is_some_and(char::is_alphanumeric)
                || (after.is_some_and(|a| matches!(a, '-' | '.' | '/')) && next_digit)
            {
                return None;
            }
            return Some(m.range());
        }
        ["cpt", "pmid", "nct", "zip", "phone"]
            .iter()
            .find_map(|k| c.name(k))
            .map(|m| m.range())
    }

    /// Spells `id` (bytes of `s`, starting at char `at`): each digit as its
    /// word, each dot as "dot", everything else as written, with spaces
    /// between words.
    fn spell(b: &mut SpokenBuilder, s: &str, id: std::ops::Range<usize>, at: usize) {
        let mut prev: Option<char> = None;
        for (k, ch) in s[id].chars().enumerate() {
            let pos = CharPos(at + k);
            let last_alnum = b
                .text()
                .chars()
                .next_back()
                .is_some_and(char::is_alphanumeric);
            let letter_after_letter = ch.is_alphabetic() && prev.is_some_and(char::is_alphabetic);
            // "doi:10.1000" is "doi: one zero ...", not "doi:one".
            let after_colon = prev.is_none() && b.text().ends_with(':');
            if ch.is_alphanumeric() && (last_alnum || after_colon) && !letter_after_letter {
                b.push_inserted(" ", pos);
            }
            let range = CharRange::new(pos, CharPos(at + k + 1));
            if ch.is_ascii_digit() {
                b.push_expanded(&decimal_digits_to_words(&ch.to_string()), range);
            } else if ch == '.' {
                if last_alnum {
                    b.push_inserted(" ", pos);
                }
                b.push_expanded("dot", range);
            } else {
                let mut buf = [0u8; 4];
                b.push_literal(ch.encode_utf8(&mut buf), pos);
            }
            prev = Some(ch);
        }
    }

    /// Applies the guard. `None` when there is no identifier.
    fn apply(&self, input: &str) -> Option<(String, OffsetMap)> {
        // `captures_at` builds capture slots even when nothing matches.
        if !self.re.is_match(input) {
            return None;
        }
        let mut b = SpokenBuilder::new();
        let (mut cur, mut cur_char, mut pos) = (0usize, 0usize, 0usize);
        let mut any = false;
        while pos <= input.len() {
            let Some(c) = self.re.captures_at(input, pos) else {
                break;
            };
            let Some(m) = c.get(0) else {
                break;
            };
            let Some(id) = Self::identifier(&c, input).filter(|id| id.start < id.end) else {
                pos = input[m.start()..]
                    .chars()
                    .next()
                    .map_or(input.len() + 1, |ch| m.start() + ch.len_utf8());
                continue;
            };
            b.push_literal(&input[cur..id.start], CharPos(cur_char));
            cur_char += input[cur..id.start].chars().count();
            Self::spell(&mut b, input, id.clone(), cur_char);
            cur_char += input[id.clone()].chars().count();
            cur = id.end;
            pos = id.end;
            any = true;
        }
        if !any {
            return None;
        }
        b.push_literal(&input[cur..], CharPos(cur_char));
        Some(b.finish())
    }
}

/// The numbers transform.
pub struct Numbers {
    identifiers: Identifiers,
    /// Only the identifier guard, for engines that read numbers natively.
    only_identifiers: bool,
    rules: Vec<Rule>,
}

impl std::fmt::Debug for Numbers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Numbers")
            .field("only_identifiers", &self.only_identifiers)
            .finish_non_exhaustive()
    }
}

impl Default for Numbers {
    fn default() -> Self {
        Numbers {
            identifiers: Identifiers::new(),
            only_identifiers: false,
            rules: rules(),
        }
    }
}

impl Numbers {
    /// Only the identifier guard (identifiers read as digits), for engines
    /// that read numbers, dates and amounts themselves but would still
    /// read "PMID 31769816" as an amount. Its name is `identifiers`.
    pub fn identifiers_only() -> Self {
        Numbers {
            identifiers: Identifiers::new(),
            only_identifiers: true,
            rules: Vec::new(),
        }
    }
}

impl Transform for Numbers {
    fn name(&self) -> &'static str {
        if self.only_identifiers {
            "identifiers"
        } else {
            "numbers"
        }
    }

    fn apply(&self, input: &str) -> (String, OffsetMap) {
        self.apply_changed(input)
            .unwrap_or_else(|| super::identity(input))
    }

    fn apply_changed(&self, input: &str) -> Option<(String, OffsetMap)> {
        // Every rule reads a digit: without one there is nothing to do.
        if !input.bytes().any(|b| b.is_ascii_digit()) {
            return None;
        }
        match self.identifiers.apply(input) {
            None => apply_rules_changed(input, &self.rules),
            Some(acc) => {
                let step = apply_rules_changed(&acc.0, &self.rules);
                Some(then(acc, step))
            }
        }
    }
}

/// Star `_normalize_numbers` (with the fixes listed in the module docs).
pub fn normalize_numbers(text: &str) -> String {
    Numbers::default().apply(text).0
}
