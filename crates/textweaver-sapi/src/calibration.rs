//! Rate and pitch mapping (ADR-0004).
//!
//! SAPI rate is an integer `-10..=10`; each voice family speaks a different
//! number of words per minute at each step. The tables below were measured
//! on 2026-09-25 by synthesizing the same 63-word passage (several
//! sentences, read as a document would be) at every rate and dividing its
//! word count by the audio's duration from the first word to the end
//! (`TEXTWEAVER_SAPI=1 cargo test -p textweaver-sapi --test it --
//! real_voices::calibrate --ignored --nocapture` prints them again).
//!
//! | rate | -10 | -5 | 0 | +5 | +10 |
//! |---|---|---|---|---|---|
//! | Microsoft David Desktop | 53 | 92 | 158 | 284 | 483 |
//! | Microsoft Zira Desktop | 54 | 95 | 162 | 290 | 444 |
//! | eSpeak-en (SAPI) | 91 | 170 | 214 | 331 | 576 |
//!
//! [`MICROSOFT`] is the mean of David and Zira. `set_params` picks the rate
//! whose measured wpm is closest (in ratio) to the requested wpm, and
//! `effective_wpm` reports that measured value, so the timer pacer never
//! assumes a speed the voice cannot reach. Requests beyond a voice's range
//! are clamped to its fastest or slowest rate: SAPI tops out near 460 wpm
//! (Microsoft) and 580 wpm (eSpeak), well below textweaver's 900.
//!
//! Families without a measurement (VW voices, OpenEVV, other engines) use
//! the Microsoft table: SAPI's rate curve has roughly the same shape for
//! every engine (about 3 times faster at +10, 3 times slower at -10).
//!
//! Pitch: SAPI's `<pitch absmiddle>` (`-10..=10`) moved Microsoft David's
//! median fundamental frequency from 71 Hz (-10) through 90 Hz (0) to
//! 119 Hz (+10), about 0.45 semitone per step. A semitone offset therefore
//! maps to `round(2.2 × semitones)`, clamped, so SAPI voices reach about
//! ±4.5 semitones of textweaver's ±12.

use crate::voices::Family;

/// Words per minute at each SAPI rate, `-10..=10` (index 0 is -10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RateTable(pub [u16; 21]);

/// Microsoft David and Zira (desktop voices; also used for OneCore).
pub const MICROSOFT: RateTable = RateTable([
    54, 60, 67, 75, 84, 94, 105, 117, 132, 146, 160, 184, 205, 230, 257, 287, 319, 353, 389, 428,
    464,
]);

/// eSpeak's SAPI5 voices.
pub const ESPEAK: RateTable = RateTable([
    91, 119, 135, 148, 158, 170, 180, 189, 197, 206, 214, 229, 242, 269, 298, 331, 372, 415, 461,
    512, 576,
]);

/// SAPI pitch steps per semitone (measured: 0.45 semitone per step).
pub const PITCH_STEPS_PER_SEMITONE: f32 = 2.2;

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
    (f32::from(semitones) * PITCH_STEPS_PER_SEMITONE)
        .round()
        .clamp(-10.0, 10.0) as i8
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
    fn star_default_rate_maps_close() {
        // 265 wpm: Microsoft rate 4 (257), eSpeak rate 3 (269).
        assert_eq!(MICROSOFT.rate_for(265), 4);
        assert_eq!(ESPEAK.rate_for(265), 3);
    }

    #[test]
    fn out_of_range_requests_clamp() {
        assert_eq!(MICROSOFT.rate_for(10), -10);
        assert_eq!(MICROSOFT.rate_for(900), 10);
        assert_eq!(MICROSOFT.wpm_at(99), MICROSOFT.0[20]);
        assert_eq!(MICROSOFT.wpm_at(-99), MICROSOFT.0[0]);
    }

    #[test]
    fn pitch_maps_at_the_measured_slope() {
        assert_eq!(sapi_pitch(0), 0);
        assert_eq!(sapi_pitch(1), 2);
        assert_eq!(sapi_pitch(-2), -4);
        assert_eq!(sapi_pitch(4), 9);
        assert_eq!(sapi_pitch(-12), -10);
        assert_eq!(sapi_pitch(12), 10);
    }
}
