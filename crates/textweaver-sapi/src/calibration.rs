//! Rate and pitch mapping (ADR-0004).
//!
//! SAPI rate is an integer `-10..=10`; each voice family speaks a different
//! number of words per minute at each step. The tables below were measured
//! on 2026-09-25 by synthesizing the same 83-word passage (several
//! sentences, read as a document would be) at every rate and dividing its
//! word count by the audio's duration from the first word to the end
//! (`TEXTWEAVER_SAPI=1 cargo test -p textweaver-sapi --test real_voices
//! calibrate -- --ignored --nocapture` prints them again).
//!
//! `set_params` picks the rate whose measured wpm is closest (in ratio) to
//! the requested wpm, and `effective_wpm` reports that measured value, so
//! the timer pacer never assumes a speed the voice cannot reach. Requests
//! beyond a voice's range are clamped to its fastest or slowest rate.
//!
//! Families without a measurement (VW voices, OpenEVV, third-party engines)
//! use the Microsoft table: SAPI's rate curve is roughly the same shape for
//! every engine (about 3× faster at +10 and 3× slower at -10).
//!
//! Pitch: textweaver's semitone offset maps one-to-one onto SAPI's
//! `<pitch absmiddle>` (`-10..=10`), clamped; SAPI documents no unit, and
//! the Microsoft voices move by roughly a semitone per step.

use crate::voices::Family;

/// Words per minute at each SAPI rate, `-10..=10` (index 0 is -10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RateTable(pub [u16; 21]);

/// Microsoft David and Zira (desktop and OneCore).
pub const MICROSOFT: RateTable = RateTable([
    64, 71, 79, 87, 96, 106, 117, 129, 142, 156, 173, 191, 211, 233, 256, 283, 312, 344, 380, 419,
    462,
]);

/// eSpeak's SAPI5 voices.
pub const ESPEAK: RateTable = RateTable([
    64, 71, 79, 87, 96, 106, 117, 129, 142, 156, 173, 191, 211, 233, 256, 283, 312, 344, 380, 419,
    462,
]);

/// The table for a voice family.
pub fn table_for(family: Family) -> &'static RateTable {
    match family {
        Family::Espeak => &ESPEAK,
        Family::Microsoft | Family::Eloquence | Family::OpenEvv | Family::Other => &MICROSOFT,
    }
}

impl RateTable {
    /// The measured wpm at SAPI rate `rate` (clamped to `-10..=10`).
    pub fn wpm_at(&self, rate: i8) -> u16 {
        self.0[usize::try_from(i16::from(rate.clamp(-10, 10)) + 10).unwrap_or(10)]
    }

    /// The SAPI rate whose wpm is closest in ratio to `wpm`.
    pub fn rate_for(&self, wpm: u16) -> i8 {
        let want = f64::from(wpm.max(1));
        let mut best = (0i8, f64::INFINITY);
        for r in -10i8..=10 {
            let got = f64::from(self.wpm_at(r).max(1));
            let err = (got / want).ln().abs();
            if err < best.1 {
                best = (r, err);
            }
        }
        best.0
    }
}

/// SAPI `<pitch absmiddle>` for a semitone offset.
pub fn sapi_pitch(semitones: i8) -> i8 {
    semitones.clamp(-10, 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_rise_with_rate() {
        for t in [&MICROSOFT, &ESPEAK] {
            assert!(t.0.windows(2).all(|w| w[0] < w[1]), "{t:?}");
        }
    }

    #[test]
    fn rate_round_trips_through_the_table() {
        for t in [&MICROSOFT, &ESPEAK] {
            for r in -10i8..=10 {
                assert_eq!(t.rate_for(t.wpm_at(r)), r);
            }
        }
    }

    #[test]
    fn out_of_range_requests_clamp() {
        assert_eq!(MICROSOFT.rate_for(10), -10);
        assert_eq!(MICROSOFT.rate_for(900), 10);
        assert_eq!(MICROSOFT.wpm_at(99), MICROSOFT.0[20]);
        assert_eq!(MICROSOFT.wpm_at(-99), MICROSOFT.0[0]);
    }

    #[test]
    fn pitch_is_clamped() {
        assert_eq!(sapi_pitch(3), 3);
        assert_eq!(sapi_pitch(-12), -10);
        assert_eq!(sapi_pitch(12), 10);
    }
}
