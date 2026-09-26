//! Numbers as words, ported from `star/ttstext/numbers.py` (Part 2 section
//! 5.B.3 helpers) with Star's exact wording: no "and", hyphenated tens,
//! "nineteen oh five" years.

const ONES: [&str; 20] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];

const TENS: [&str; 10] = [
    "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];

const SCALES: [(u64, &str); 4] = [
    (1_000_000_000_000, "trillion"),
    (1_000_000_000, "billion"),
    (1_000_000, "million"),
    (1_000, "thousand"),
];

const ORDINALS: [&str; 13] = [
    "zeroth", "first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth",
    "tenth", "eleventh", "twelfth",
];

/// Month names, January first.
pub const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn small(n: u64) -> &'static str {
    ONES[usize::try_from(n).unwrap_or(0).min(19)]
}

fn unsigned_words(n: u64) -> String {
    if n < 20 {
        return small(n).to_owned();
    }
    if n < 100 {
        let (t, o) = (n / 10, n % 10);
        let tens = TENS[usize::try_from(t).unwrap_or(0)];
        return if o == 0 {
            tens.to_owned()
        } else {
            format!("{tens}-{}", small(o))
        };
    }
    if n < 1000 {
        let (h, r) = (n / 100, n % 100);
        let mut s = format!("{} hundred", small(h));
        if r != 0 {
            s.push(' ');
            s.push_str(&unsigned_words(r));
        }
        return s;
    }
    for (div, label) in SCALES {
        if n >= div {
            let (q, r) = (n / div, n % div);
            let mut s = format!("{} {label}", unsigned_words(q));
            if r != 0 {
                s.push(' ');
                s.push_str(&unsigned_words(r));
            }
            return s;
        }
    }
    n.to_string()
}

/// Star `_int_to_words`: `1234` is "one thousand two hundred thirty-four".
pub fn int_to_words(n: i64) -> String {
    if n < 0 {
        format!("negative {}", unsigned_words(n.unsigned_abs()))
    } else {
        unsigned_words(n.unsigned_abs())
    }
}

/// Star `_year_to_words`: `1984` is "nineteen eighty-four", `1905` is
/// "nineteen oh five", `2000` is "two thousand"; outside `100..=2999` a
/// plain number.
pub fn year_to_words(y: i64) -> String {
    if !(100..=2999).contains(&y) {
        return int_to_words(y);
    }
    let (century, decade) = (y / 100, y % 100);
    if decade == 0 {
        if y % 1000 == 0 {
            return int_to_words(y);
        }
        return format!("{} hundred", int_to_words(century));
    }
    if decade < 10 {
        format!("{} oh {}", int_to_words(century), int_to_words(decade))
    } else {
        format!("{} {}", int_to_words(century), int_to_words(decade))
    }
}

fn unsigned_ordinal(n: u64) -> String {
    if n <= 12 {
        return ORDINALS[usize::try_from(n).unwrap_or(0)].to_owned();
    }
    if n < 20 {
        return format!("{}th", small(n));
    }
    if n < 100 {
        let (t, o) = (n / 10, n % 10);
        if o != 0 {
            return format!("{}-{}", unsigned_words(t * 10), unsigned_ordinal(o));
        }
        let w = unsigned_words(n);
        return format!("{}ieth", &w[..w.len() - 1]);
    }
    if n < 1000 {
        let (h, r) = (n / 100, n % 100);
        if r != 0 {
            return format!("{} {}", unsigned_words(h * 100), unsigned_ordinal(r));
        }
        return format!("{}th", unsigned_words(n));
    }
    for (div, label) in SCALES {
        if n >= div {
            let (q, r) = (n / div, n % div);
            if r != 0 {
                return format!("{} {label} {}", unsigned_words(q), unsigned_ordinal(r));
            }
            return format!("{} {label}th", unsigned_words(q));
        }
    }
    format!("{n}th")
}

/// Star `_ordinal_to_words`: `21` is "twenty-first", `100` is
/// "one hundredth", `1000001` is "one million first".
pub fn ordinal_to_words(n: i64) -> String {
    if n < 0 {
        format!("negative {}", unsigned_ordinal(n.unsigned_abs()))
    } else {
        unsigned_ordinal(n.unsigned_abs())
    }
}

/// Star `_decimal_digits_to_words`: each digit on its own, "305" is
/// "three zero five".
pub fn decimal_digits_to_words(digits: &str) -> String {
    digits
        .chars()
        .filter_map(|c| c.to_digit(10))
        .map(|d| small(u64::from(d)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The plural of a spoken number phrase for decades: "twenty twenty" to
/// "twenty twenties", "nineteen hundred" to "nineteen hundreds".
pub fn pluralize_last_word(s: &str) -> String {
    if let Some(stem) = s.strip_suffix('y') {
        format!("{stem}ies")
    } else if s.ends_with('x') || s.ends_with('s') {
        format!("{s}es")
    } else {
        format!("{s}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_to_words_vectors() {
        // tests/test_ttstext.py:35-55
        for (n, w) in [
            (0, "zero"),
            (1, "one"),
            (5, "five"),
            (13, "thirteen"),
            (20, "twenty"),
            (21, "twenty-one"),
            (42, "forty-two"),
            (100, "one hundred"),
            (105, "one hundred five"),
            (1000, "one thousand"),
            (1234, "one thousand two hundred thirty-four"),
            (1_000_000, "one million"),
            (2_500_000, "two million five hundred thousand"),
            (-7, "negative seven"),
        ] {
            assert_eq!(int_to_words(n), w, "{n}");
        }
    }

    #[test]
    fn year_to_words_vectors() {
        // tests/test_ttstext.py:61-82
        for (y, w) in [
            (1984, "nineteen eighty-four"),
            (2024, "twenty twenty-four"),
            (1900, "nineteen hundred"),
            (1905, "nineteen oh five"),
            (2009, "twenty oh nine"),
            (50, "fifty"),
            (3000, "three thousand"),
            (2000, "two thousand"),
            (1000, "one thousand"),
        ] {
            assert_eq!(year_to_words(y), w, "{y}");
        }
    }

    #[test]
    fn ordinal_to_words_vectors() {
        // tests/test_ttstext.py:88-110
        for (n, w) in [
            (1, "first"),
            (2, "second"),
            (3, "third"),
            (4, "fourth"),
            (11, "eleventh"),
            (12, "twelfth"),
            (13, "thirteenth"),
            (20, "twentieth"),
            (21, "twenty-first"),
            (23, "twenty-third"),
            (100, "one hundredth"),
            (101, "one hundred first"),
            (1000, "one thousandth"),
            (1_000_000, "one millionth"),
            (1_000_001, "one million first"),
            (-3, "negative third"),
        ] {
            assert_eq!(ordinal_to_words(n), w, "{n}");
        }
    }

    #[test]
    fn decimal_digits_vectors() {
        // tests/test_ttstext.py:116-126
        assert_eq!(decimal_digits_to_words("14"), "one four");
        assert_eq!(decimal_digits_to_words("0"), "zero");
        assert_eq!(decimal_digits_to_words("305"), "three zero five");
        assert_eq!(decimal_digits_to_words("7"), "seven");
    }

    #[test]
    fn plurals() {
        assert_eq!(pluralize_last_word("twenty twenty"), "twenty twenties");
        assert_eq!(pluralize_last_word("nineteen hundred"), "nineteen hundreds");
        assert_eq!(pluralize_last_word("nineteen eighty"), "nineteen eighties");
        assert_eq!(pluralize_last_word("two thousand"), "two thousands");
    }
}
