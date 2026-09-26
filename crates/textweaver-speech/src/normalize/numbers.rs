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
//!   ("08:05" is "eight oh five AM"); hour zero is "twelve" ("00:30" is
//!   "twelve thirty AM", where Star said "zero thirty AM"); seconds are
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

use regex::Captures;

use super::Transform;
use super::rewrite::{Piece, Rule, apply_rules, char_after, char_before};
use super::words::{
    MONTHS, decimal_digits_to_words, int_to_words, ordinal_to_words, pluralize_last_word,
    year_to_words,
};
use textweaver_core::OffsetMap;

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
        // 5. Percent.
        Rule::with(r"\b([0-9]+)(?:\.([0-9]+))?%", |c, _| {
            let int = num(c, 1)?;
            text(match c.get(2) {
                None => format!("{} percent", int_to_words(int)),
                Some(d) => format!(
                    "{} point {} percent",
                    int_to_words(int),
                    decimal_digits_to_words(d.as_str())
                ),
            })
        }),
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

/// The numbers transform.
pub struct Numbers {
    rules: Vec<Rule>,
}

impl std::fmt::Debug for Numbers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Numbers").finish_non_exhaustive()
    }
}

impl Default for Numbers {
    fn default() -> Self {
        Numbers { rules: rules() }
    }
}

impl Transform for Numbers {
    fn name(&self) -> &'static str {
        "numbers"
    }

    fn apply(&self, input: &str) -> (String, OffsetMap) {
        apply_rules(input, &self.rules)
    }
}

/// Star `_normalize_numbers` (with the fixes listed in the module docs).
pub fn normalize_numbers(text: &str) -> String {
    Numbers::default().apply(text).0
}
