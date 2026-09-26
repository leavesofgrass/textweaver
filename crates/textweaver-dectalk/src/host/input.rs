//! The text DECtalk is given: 8-bit text with inline commands.
//!
//! DECtalk reads ISO 8859-1 (Latin-1) text and takes its settings as inline
//! commands in brackets. For each utterance the host sends:
//!
//! ```text
//! [:n<letter>][:rate <wpm>][:dv ap <hz>][:index mark 1]<word 0's mark><word 0>…
//! ```
//!
//! - `[:n<letter>]` selects the speaker and resets its voice;
//! - `[:rate N]` sets the rate in words per minute;
//! - `[:dv ap N]` (only when the pitch is shifted) sets the average pitch;
//! - `[:index mark 1]` is the *start mark*: its sample number tells the
//!   host where this utterance's audio begins in DECtalk's sample count;
//! - word `i`'s mark is `[:index mark i+2]`. DECtalk's index values are
//!   kept below 32768, so an utterance reports at most 32,766 words
//!   (utterances are sentence-sized; later marks are left out).
//!
//! **Encoding.** Latin-1 letters pass through as their byte; common
//! typographic characters become their ASCII equivalents (curly quotes,
//! dashes, the ellipsis); every other character, and every control
//! character, becomes a space, so the words around it keep their places.
//! Text can never inject a command: `[` and `]` become `(` and `)`, since
//! DECtalk reads a bracket as the start of a command or of phonemic text.

use super::Settings;

/// The start mark's index value.
pub const START_MARK: u32 = 1;
/// Word `i` is reported with index value `i + WORD_MARK_BASE`.
pub const WORD_MARK_BASE: u32 = 2;
/// The largest index value used (DECtalk's index values are 15-bit).
pub const MAX_MARK: u32 = 32_767;

/// One character as DECtalk's Latin-1 text, or `None` to write a space.
fn latin1(c: char) -> Option<&'static [u8]> {
    const BYTES: [u8; 256] = {
        let mut t = [0u8; 256];
        let mut i = 0;
        while i < 256 {
            t[i] = i as u8;
            i += 1;
        }
        t
    };
    let code = c as u32;
    match c {
        '[' => Some(b"("),
        ']' => Some(b")"),
        '\u{a0}' => None,
        '\u{2018}' | '\u{2019}' | '\u{201a}' | '\u{2032}' => Some(b"'"),
        '\u{201c}' | '\u{201d}' | '\u{201e}' | '\u{2033}' => Some(b"\""),
        '\u{2010}'..='\u{2015}' | '\u{2212}' => Some(b"-"),
        '\u{2026}' => Some(b"..."),
        _ if c.is_control() => None,
        // Printable ASCII and Latin-1 (U+00A1..=U+00FF) are one byte each.
        _ if code < 0x100 => {
            let i = code as usize;
            Some(&BYTES[i..=i])
        }
        _ => None,
    }
}

/// Encodes UTF-8 text as DECtalk's Latin-1 text (see the module docs).
pub fn encode_text(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    for c in text.chars() {
        match latin1(c) {
            Some(b) => out.extend_from_slice(b),
            None => out.push(b' '),
        }
    }
    out
}

/// One piece of engine input, already encoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnginePiece {
    /// Encoded text (Latin-1, no brackets, no control characters).
    Text(Vec<u8>),
    /// The mark before word `n`.
    Index(u32),
}

/// The complete DECtalk input for one utterance, NUL-terminated.
pub fn command_string(settings: &Settings, pieces: &[EnginePiece]) -> Vec<u8> {
    let mut s = format!("[:n{}][:rate {}]", settings.speaker.letter(), settings.rate);
    if settings.pitch_hz != 0 {
        s.push_str(&format!("[:dv ap {}]", settings.pitch_hz));
    }
    s.push_str(&format!("[:index mark {START_MARK}]"));
    let mut out = s.into_bytes();
    for p in pieces {
        match p {
            EnginePiece::Text(t) => out.extend(t.iter().map(|&b| if b == 0 { b' ' } else { b })),
            EnginePiece::Index(i) => {
                if let Some(v) = i.checked_add(WORD_MARK_BASE).filter(|v| *v <= MAX_MARK) {
                    out.extend_from_slice(format!("[:index mark {v}]").as_bytes());
                }
            }
        }
    }
    out.push(0);
    out
}

/// The word index an index value reports, or `None` for the start mark and
/// values textweaver did not send.
pub fn word_of_mark(value: u32) -> Option<u32> {
    value
        .checked_sub(WORD_MARK_BASE)
        .filter(|_| value <= MAX_MARK)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voices::Speaker;

    #[test]
    fn latin1_passes_and_the_rest_becomes_ascii_or_space() {
        assert_eq!(encode_text("Café naïve"), b"Caf\xe9 na\xefve");
        assert_eq!(encode_text("“quoted” — it’s…"), b"\"quoted\" - it's...");
        assert_eq!(encode_text("日本 ok"), b"   ok");
        assert_eq!(encode_text("a\tb\nc\0d\u{85}e"), b"a b c d e");
        assert_eq!(encode_text("\u{a0}x"), b" x");
    }

    #[test]
    fn brackets_cannot_start_a_command() {
        assert_eq!(encode_text("[:np] [hh ax l ow]"), b"(:np) (hh ax l ow)");
    }

    #[test]
    fn the_command_string_sets_the_voice_then_the_marks() {
        let settings = Settings {
            speaker: Speaker::Betty,
            rate: 250,
            pitch_hz: 0,
        };
        let pieces = [
            EnginePiece::Index(0),
            EnginePiece::Text(b"Hello ".to_vec()),
            EnginePiece::Index(1),
            EnginePiece::Text(b"world".to_vec()),
        ];
        assert_eq!(
            command_string(&settings, &pieces),
            b"[:nb][:rate 250][:index mark 1][:index mark 2]Hello [:index mark 3]world\0"
        );
        let shifted = Settings {
            pitch_hz: 233,
            ..settings
        };
        let s = command_string(&shifted, &[]);
        assert_eq!(s, b"[:nb][:rate 250][:dv ap 233][:index mark 1]\0");
    }

    #[test]
    fn marks_beyond_dectalks_range_are_left_out() {
        let s = command_string(
            &Settings::default(),
            &[
                EnginePiece::Index(32_765),
                EnginePiece::Index(32_766),
                EnginePiece::Text(vec![b'a', 0, b'b']),
            ],
        );
        let text = String::from_utf8_lossy(&s);
        assert!(text.contains("[:index mark 32767]"));
        assert!(!text.contains("32768"));
        assert!(text.ends_with("a b\0"));
        assert_eq!(s.iter().filter(|&&b| b == 0).count(), 1);
    }

    #[test]
    fn mark_values_map_back_to_words() {
        assert_eq!(word_of_mark(START_MARK), None);
        assert_eq!(word_of_mark(0), None);
        assert_eq!(word_of_mark(2), Some(0));
        assert_eq!(word_of_mark(9), Some(7));
        assert_eq!(word_of_mark(40_000), None);
    }

    proptest::proptest! {
        #[test]
        fn encoded_text_is_one_byte_per_char_or_an_expansion(t in "\\PC{0,40}") {
            let e = encode_text(&t);
            proptest::prop_assert!(!e.contains(&0));
            proptest::prop_assert!(!e.contains(&b'['));
            proptest::prop_assert!(!e.contains(&b']'));
            proptest::prop_assert!(e.len() >= t.chars().count());
        }
    }
}
