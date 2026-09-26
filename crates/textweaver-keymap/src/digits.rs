//! The digit row, whatever the keyboard layout.
//!
//! The browse keys `1` to `6` move to the next heading of that level and
//! Shift with the digit to the previous one, as in NVDA's and JAWS's
//! browse mode, which match the physical digit key. The keymap stores
//! Shift with a digit as the character a US keyboard types (`!` for
//! Shift+1), because a character key never carries Shift
//! ([`KeyChord::new`]). Frontends turn a key press into that chord here:
//!
//! - When they know the physical key (the Windows console reports its
//!   virtual key code, `VK_0` to `VK_9`, in Windows Terminal and the
//!   classic console alike), [`digit_row_chord`] gives the chord for the
//!   digit and Shift, whatever the layout typed.
//! - When they know only the character (most terminals elsewhere),
//!   [`from_typed`] recognises the shifted digits of the common layouts:
//!   US, UK, German, Spanish, Nordic (Swedish, Finnish, Norwegian,
//!   Danish), and Italian. French AZERTY types the digits only with Shift,
//!   which the character alone cannot tell from a US digit, so it needs
//!   [`DigitRow::Azerty`] (`[keyboard] digit_row = "azerty"`).
//!
//! Only characters no layout table above uses otherwise are translated,
//! and the frontend asks only after the keymap found nothing for the
//! character, so a bound key keeps its meaning.

use crate::{Key, KeyChord};

/// Which digit row the frontend assumes when it knows only the character
/// typed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DigitRow {
    /// Digits typed without Shift, as on US, UK, German, Spanish, Nordic,
    /// and Italian keyboards; their shifted characters are recognised.
    #[default]
    Auto,
    /// French AZERTY: the digit row types `& é " ' ( - è _ ç à` without
    /// Shift and the digits with it.
    Azerty,
}

impl DigitRow {
    /// The digit row for a `[keyboard] digit_row` value (`auto` or
    /// `azerty`); anything else is `Auto`.
    pub fn from_id(id: &str) -> DigitRow {
        if id.trim().eq_ignore_ascii_case("azerty") {
            DigitRow::Azerty
        } else {
            DigitRow::Auto
        }
    }
}

/// What a US keyboard types with Shift and each digit, `0` to `9`.
const US_SHIFTED: [char; 10] = [')', '!', '@', '#', '$', '%', '^', '&', '*', '('];

/// The shifted digits of the UK, German, Spanish, Nordic, and Italian
/// layouts that differ from the US ones: the character and its digit.
/// `&` is Shift+6 on German, Spanish, Nordic, and Italian keyboards (on a
/// US keyboard it is Shift+7, which textweaver does not bind).
const OTHER_SHIFTED: [(char, u8); 6] = [
    ('"', 2), // UK, German, Spanish, Nordic, Italian
    ('£', 3), // UK, Italian
    ('§', 3), // German
    ('·', 3), // Spanish
    ('¤', 4), // Nordic
    ('&', 6), // German, Spanish, Nordic, Italian
];

/// AZERTY's digit row without Shift, `1` to `0`.
const AZERTY: [char; 10] = ['&', 'é', '"', '\'', '(', '-', 'è', '_', 'ç', 'à'];

/// The chord for the physical digit key `digit` (0 to 9), with or without
/// Shift: `1`, or `!` for Shift+1 (the character the keymap stores).
pub fn digit_row_chord(digit: u8, shift: bool) -> Option<KeyChord> {
    let d = usize::from(digit);
    if d > 9 {
        return None;
    }
    let c = if shift {
        US_SHIFTED[d]
    } else {
        char::from(b'0' + digit)
    };
    Some(KeyChord::plain(Key::Char(c)))
}

/// The digit of a Windows virtual key code on the digit row (`VK_0` is
/// 0x30, `VK_9` is 0x39; the numeric keypad has other codes).
pub fn digit_of_virtual_key(vk: u16) -> Option<u8> {
    (0x30..=0x39).contains(&vk).then(|| (vk - 0x30) as u8)
}

/// The digit-row chord for a character typed without Ctrl or Alt, when
/// the character is a digit key of the layout `row` and the keymap stores
/// that key under another character; `None` otherwise.
///
/// With [`DigitRow::Auto`]: `"` `£` `§` `·` `¤` `&` give Shift with their
/// digit. With [`DigitRow::Azerty`]: `& é " ' ( -` and the rest of the row
/// give the digits, and the digits give Shift with themselves.
pub fn from_typed(c: char, row: DigitRow) -> Option<KeyChord> {
    match row {
        DigitRow::Auto => OTHER_SHIFTED
            .iter()
            .find(|(ch, _)| *ch == c)
            .and_then(|(_, d)| digit_row_chord(*d, true)),
        DigitRow::Azerty => {
            if let Some(d) = c.to_digit(10) {
                return digit_row_chord(u8::try_from(d).ok()?, true);
            }
            let d = AZERTY.iter().position(|ch| *ch == c)?;
            digit_row_chord(u8::try_from((d + 1) % 10).ok()?, false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActionId, Frontend, Keymap, Layer, Platform};

    fn heading(chord: Option<KeyChord>) -> Option<ActionId> {
        let map = Keymap::defaults(Platform::Linux, Frontend::Terminal);
        map.lookup(&chord?, Layer::Browse)
    }

    #[test]
    fn physical_digits_reach_heading_levels() {
        for level in 1..=6u8 {
            assert_eq!(
                heading(digit_row_chord(level, false)),
                ActionId::next_heading_at(level)
            );
            assert_eq!(
                heading(digit_row_chord(level, true)),
                ActionId::previous_heading_at(level)
            );
        }
        assert_eq!(digit_row_chord(10, false), None);
        assert_eq!(digit_of_virtual_key(0x31), Some(1));
        assert_eq!(digit_of_virtual_key(0x30), Some(0));
        assert_eq!(digit_of_virtual_key(0x61), None, "numpad 1");
        assert_eq!(digit_of_virtual_key(0x41), None, "A");
    }

    /// Shift with 1 to 6 on each layout the character table covers.
    #[test]
    fn shifted_digits_of_common_layouts() {
        let layouts: [(&str, [char; 6]); 6] = [
            ("US", ['!', '@', '#', '$', '%', '^']),
            ("UK", ['!', '"', '£', '$', '%', '^']),
            ("German", ['!', '"', '§', '$', '%', '&']),
            ("Spanish", ['!', '"', '·', '$', '%', '&']),
            ("Nordic", ['!', '"', '#', '¤', '%', '&']),
            ("Italian", ['!', '"', '£', '$', '%', '&']),
        ];
        for (name, chars) in layouts {
            for (i, c) in chars.into_iter().enumerate() {
                let level = u8::try_from(i + 1).unwrap();
                // US characters are bound directly; the others translate.
                let chord = from_typed(c, DigitRow::Auto).or_else(|| c.to_string().parse().ok());
                assert_eq!(
                    heading(chord),
                    ActionId::previous_heading_at(level),
                    "{name} Shift+{level} types {c}"
                );
            }
        }
        // Digits are the next heading on these layouts.
        assert_eq!(from_typed('1', DigitRow::Auto), None);
        assert_eq!(from_typed('x', DigitRow::Auto), None);
    }

    #[test]
    fn azerty_digit_row() {
        let unshifted = ['&', 'é', '"', '\'', '(', '-'];
        for (i, c) in unshifted.into_iter().enumerate() {
            let level = u8::try_from(i + 1).unwrap();
            assert_eq!(
                heading(from_typed(c, DigitRow::Azerty)),
                ActionId::next_heading_at(level),
                "{c}"
            );
            let digit = char::from(b'0' + level);
            assert_eq!(
                heading(from_typed(digit, DigitRow::Azerty)),
                ActionId::previous_heading_at(level),
                "Shift+{digit}"
            );
        }
        assert_eq!(
            from_typed('à', DigitRow::Azerty),
            digit_row_chord(0, false)
        );
        assert_eq!(DigitRow::from_id(" AZERTY "), DigitRow::Azerty);
        assert_eq!(DigitRow::from_id("qwerty"), DigitRow::Auto);
    }
}
