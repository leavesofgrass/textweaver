//! Timing: the PARIS standard and Farnsworth spacing.
//!
//! PARIS: a dit is one unit, a dah three; the gap inside a character is
//! one unit, between characters three, between words seven. The word
//! "PARIS" with the space after it is 50 units, so at `w` words per minute
//! a unit lasts `1200 / w` milliseconds.
//!
//! Farnsworth: the characters keep their speed `c` and only the gaps
//! between characters and words stretch, so that the overall speed is `s`.
//! The ARRL's formula (QEX, KE3Z, as quoted in the research) gives the
//! total added delay per PARIS word, in seconds:
//! `ta = (60 c - 37.2 s) / (s c)`, of which a character gap takes
//! `3 ta / 19` and a word gap `7 ta / 19`. An effective speed at or above
//! the character speed means plain PARIS spacing.

/// The slowest speed offered, in words per minute.
pub const MIN_WPM: u16 = 5;

/// The fastest speed offered, in words per minute.
pub const MAX_WPM: u16 = 50;

/// The default character speed (the owner's choice, October 23, 2026).
pub const DEFAULT_CHAR_WPM: u16 = 20;

/// The default effective (Farnsworth) speed.
pub const DEFAULT_EFFECTIVE_WPM: u16 = 10;

/// How long each part of the code lasts, in milliseconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timing {
    /// A dit, and the gap between the elements of one character.
    pub dit_ms: f64,
    /// The gap between two characters of a word.
    pub char_gap_ms: f64,
    /// The gap between two words.
    pub word_gap_ms: f64,
}

impl Timing {
    /// Characters at `char_wpm`, spaced for an overall `effective_wpm`.
    /// Both are kept within [`MIN_WPM`] to [`MAX_WPM`], and the effective
    /// speed is never above the character speed.
    pub fn new(char_wpm: u16, effective_wpm: u16) -> Self {
        let c = f64::from(char_wpm.clamp(MIN_WPM, MAX_WPM));
        let s = f64::from(effective_wpm.clamp(MIN_WPM, MAX_WPM)).min(c);
        let dit_ms = 1200.0 / c;
        if s >= c {
            return Timing {
                dit_ms,
                char_gap_ms: 3.0 * dit_ms,
                word_gap_ms: 7.0 * dit_ms,
            };
        }
        let ta_ms = 1000.0 * (60.0 * c - 37.2 * s) / (s * c);
        Timing {
            dit_ms,
            char_gap_ms: 3.0 * ta_ms / 19.0,
            word_gap_ms: 7.0 * ta_ms / 19.0,
        }
    }

    /// A dah.
    pub fn dah_ms(&self) -> f64 {
        3.0 * self.dit_ms
    }
}

impl Default for Timing {
    fn default() -> Self {
        Timing::new(DEFAULT_CHAR_WPM, DEFAULT_EFFECTIVE_WPM)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn paris_units_at_twenty_words_per_minute() {
        let t = Timing::new(20, 20);
        assert!(close(t.dit_ms, 60.0));
        assert!(close(t.dah_ms(), 180.0));
        assert!(close(t.char_gap_ms, 180.0));
        assert!(close(t.word_gap_ms, 420.0));
    }

    #[test]
    fn farnsworth_gaps_follow_the_arrl_formula() {
        // c = 20, s = 10: ta = (1200 - 372) / 200 s = 4.14 s.
        let t = Timing::new(20, 10);
        assert!(close(t.dit_ms, 60.0));
        assert!(close(t.char_gap_ms, 3.0 * 4140.0 / 19.0));
        assert!(close(t.word_gap_ms, 7.0 * 4140.0 / 19.0));
        // PARIS plus its word gap then lasts one minute over ten: 31 units
        // of elements and inner gaps, 4 character gaps and a word gap.
        let paris = 31.0 * t.dit_ms + 4.0 * t.char_gap_ms + t.word_gap_ms;
        assert!(close(paris, 6000.0), "{paris}");
    }

    #[test]
    fn speeds_are_kept_in_range() {
        assert_eq!(Timing::new(20, 30), Timing::new(20, 20));
        assert_eq!(Timing::new(1, 1), Timing::new(MIN_WPM, MIN_WPM));
        assert_eq!(Timing::new(90, 90), Timing::new(MAX_WPM, MAX_WPM));
        assert_eq!(Timing::default(), Timing::new(20, 10));
    }
}
