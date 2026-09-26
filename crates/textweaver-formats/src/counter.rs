//! List and page counters (decimal, letters, roman) shared by the DOCX and
//! PDF loaders, clamped so a hostile file cannot overflow a counter or make
//! a label that fills memory.
//!
//! Word caps list start values at 32,767 and PDF page labels have no cap at
//! all; a file can claim a start of `u32::MAX` or `i64::MAX`. Counters here
//! saturate at [`MAX_COUNTER`], letter labels longer than
//! [`MAX_LETTER_REPEAT`] letters and roman numerals above 3,999 fall back to
//! decimal (as Word does past its own limits), and every label is cut to
//! [`MAX_LABEL_CHARS`] characters.

/// The largest counter value a label shows.
pub(crate) const MAX_COUNTER: u64 = 999_999;

/// The longest label, in characters, including any prefix.
pub(crate) const MAX_LABEL_CHARS: usize = 64;

/// Letter labels repeat the letter (`a`, `aa`, `aaa`); past this many
/// repeats the counter is written in decimal instead.
pub(crate) const MAX_LETTER_REPEAT: u64 = 10;

/// The largest number written in roman numerals.
const MAX_ROMAN: u64 = 3_999;

/// A counter value from any integer, clamped to `1..=MAX_COUNTER`.
pub(crate) fn clamp(n: i128) -> u64 {
    // Both bounds fit in u64, so the cast after clamping is exact.
    n.clamp(1, i128::from(MAX_COUNTER)) as u64
}

/// The next counter value: one more than `n`, never past [`MAX_COUNTER`].
pub(crate) fn next(n: u64) -> u64 {
    n.saturating_add(1).min(MAX_COUNTER)
}

/// `n` in decimal.
pub(crate) fn decimal(n: u64) -> String {
    n.min(MAX_COUNTER).to_string()
}

/// A, B, ... Z, AA, BB, ... (Word's and PDF's letter style), upper case;
/// decimal past [`MAX_LETTER_REPEAT`] repeats.
pub(crate) fn letters(n: u64) -> String {
    let n = n.clamp(1, MAX_COUNTER);
    let i = n - 1;
    let repeat = i / 26 + 1;
    if repeat > MAX_LETTER_REPEAT {
        return decimal(n);
    }
    let c = char::from(b'A' + u8::try_from(i % 26).unwrap_or(0));
    // `repeat` is at most MAX_LETTER_REPEAT, so the cast cannot truncate.
    std::iter::repeat_n(c, repeat as usize).collect()
}

/// Roman numerals, upper case; decimal above 3,999.
pub(crate) fn roman(n: u64) -> String {
    const TABLE: [(u64, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let n = n.max(1);
    if n > MAX_ROMAN {
        return decimal(n);
    }
    let mut rest = n;
    let mut s = String::new();
    for (v, r) in TABLE {
        while rest >= v {
            s.push_str(r);
            rest -= v;
        }
    }
    s
}

/// `label` cut to [`MAX_LABEL_CHARS`] characters.
pub(crate) fn cap_label(label: String) -> String {
    match label.char_indices().nth(MAX_LABEL_CHARS) {
        Some((i, _)) => label[..i].to_owned(),
        None => label,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_saturate() {
        assert_eq!(clamp(i128::from(i64::MAX)), MAX_COUNTER);
        assert_eq!(clamp(i128::from(i64::MIN)), 1);
        assert_eq!(clamp(0), 1);
        assert_eq!(clamp(42), 42);
        assert_eq!(next(u64::MAX), MAX_COUNTER);
        assert_eq!(next(MAX_COUNTER), MAX_COUNTER);
        assert_eq!(next(3), 4);
    }

    #[test]
    fn letters_and_roman_fall_back_to_decimal() {
        assert_eq!(letters(1), "A");
        assert_eq!(letters(27), "AA");
        assert_eq!(letters(260), "ZZZZZZZZZZ");
        assert_eq!(letters(261), "261");
        assert_eq!(letters(u64::MAX), MAX_COUNTER.to_string());
        assert_eq!(roman(1994), "MCMXCIV");
        assert_eq!(roman(3999), "MMMCMXCIX");
        assert_eq!(roman(4000), "4000");
        assert_eq!(roman(u64::MAX), MAX_COUNTER.to_string());
        assert_eq!(roman(0), "I");
    }

    #[test]
    fn labels_are_capped_on_char_boundaries() {
        let long = "é".repeat(MAX_LABEL_CHARS * 3);
        let capped = cap_label(long);
        assert_eq!(capped.chars().count(), MAX_LABEL_CHARS);
        assert_eq!(cap_label("3.".into()), "3.");
    }
}
