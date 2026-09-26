//! Rate calibration (ADR-0004): canonical words per minute to each engine's
//! scale, and back.
//!
//! - The classic engine (`nsspeech`) takes a rate property that is nominally
//!   words per minute, but what a voice achieves differs, and differs by
//!   macOS release: on macOS 26 it is close to the nominal value (Reed: 265
//!   gives 264 wpm live), on macOS 15 it follows the same steep curve as
//!   `AVSpeechSynthesizer` (Reed: 265 gives 498 wpm).
//! - `AVSpeechSynthesizer` takes a rate in `0.0..=1.0` (default 0.5) whose
//!   relation to words per minute is not linear and differs per voice; it
//!   is the same on macOS 15 and 26 within 8 wpm (below 130 wpm).
//!
//! Each [`RateTable`] holds measured points `(engine value, achieved wpm)`,
//! sorted by engine value, and interpolates linearly between them. The
//! points were measured on GitHub's macOS runners by synthesizing a
//! 57-word passage at each engine value and timing the audio (the
//! `*_rate_calibration` voice tests print them), and checked live at 265
//! wpm.

/// Measured `(engine value, achieved wpm)` points for one engine and voice
/// family.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RateTable {
    points: &'static [(f32, f32)],
}

impl RateTable {
    /// A table over `points`, which must be sorted by both coordinates.
    pub const fn new(points: &'static [(f32, f32)]) -> Self {
        RateTable { points }
    }

    /// The engine value that achieves `wpm`, clamped to the table's range.
    pub fn engine_value(&self, wpm: u16) -> f32 {
        interpolate(self.points.iter().map(|&(e, w)| (w, e)), f32::from(wpm))
    }

    /// The wpm the engine achieves at `engine`.
    pub fn wpm_at(&self, engine: f32) -> f32 {
        interpolate(self.points.iter().copied(), engine)
    }

    /// The wpm actually achieved when `wpm` is requested: `wpm` itself inside
    /// the table's range, the nearest end outside it.
    pub fn effective_wpm(&self, wpm: u16) -> u16 {
        let w = self.wpm_at(self.engine_value(wpm)).round();
        // Tables stay within u16 range.
        w.clamp(0.0, f32::from(u16::MAX)) as u16
    }

    /// Slowest and fastest achievable wpm.
    pub fn wpm_range(&self) -> (f32, f32) {
        match (self.points.first(), self.points.last()) {
            (Some(a), Some(b)) => (a.1, b.1),
            _ => (0.0, 0.0),
        }
    }
}

/// Piecewise-linear interpolation of `y` at `x` over `(x, y)` points sorted
/// by `x`; clamps outside the range.
fn interpolate(points: impl Iterator<Item = (f32, f32)> + Clone, x: f32) -> f32 {
    let mut prev: Option<(f32, f32)> = None;
    for (px, py) in points.clone() {
        if x <= px {
            return match prev {
                None => py,
                Some((qx, qy)) if px > qx => qy + (py - qy) * (x - qx) / (px - qx),
                Some(_) => py,
            };
        }
        prev = Some((px, py));
    }
    prev.map_or(0.0, |(_, py)| py)
}

/// The classic engine (`nsspeech`) with Eloquence voices on macOS 26 and
/// later (measured with Reed on 26.6): `(rate property, wpm)`.
pub const NS_ELOQUENCE: RateTable = RateTable::new(&[
    (80.0, 114.0),
    (100.0, 123.0),
    (120.0, 133.0),
    (140.0, 145.0),
    (160.0, 157.0),
    (175.0, 169.0),
    (190.0, 176.0),
    (200.0, 184.0),
    (210.0, 190.0),
    (220.0, 199.0),
    (230.0, 207.0),
    (240.0, 214.0),
    (250.0, 223.0),
    (265.0, 242.0),
    (280.0, 252.0),
    (300.0, 273.0),
    (330.0, 306.0),
    (360.0, 345.0),
    (400.0, 412.0),
    (450.0, 498.0),
    (500.0, 601.0),
    (550.0, 727.0),
    (600.0, 873.0),
    (650.0, 1075.0),
]);

/// The classic engine (`nsspeech`) with other voices on macOS 26 and later
/// (measured with Samantha, whose rate moves in steps): `(rate property,
/// wpm)`.
pub const NS_OTHER: RateTable = RateTable::new(&[
    (80.0, 165.0),
    (120.0, 180.0),
    (160.0, 198.0),
    (200.0, 222.0),
    (225.0, 244.0),
    (250.0, 268.0),
    (265.0, 282.0),
    (300.0, 315.0),
    (350.0, 361.0),
    (400.0, 416.0),
    (450.0, 451.0),
    (500.0, 492.0),
    (550.0, 554.0),
    (600.0, 616.0),
    (700.0, 696.0),
    (800.0, 797.0),
]);

/// The classic engine (`nsspeech`) with Eloquence voices on macOS 15 and
/// earlier (measured with Reed on 15.7): `(rate property, wpm)`.
pub const NS_ELOQUENCE_15: RateTable = RateTable::new(&[
    (80.0, 86.0),
    (100.0, 97.0),
    (120.0, 114.0),
    (140.0, 128.0),
    (160.0, 150.0),
    (175.0, 169.0),
    (190.0, 199.0),
    (200.0, 223.0),
    (210.0, 252.0),
    (220.0, 287.0),
    (230.0, 319.0),
    (240.0, 359.0),
    (250.0, 412.0),
    (265.0, 498.0),
    (280.0, 581.0),
    (300.0, 727.0),
    (330.0, 1028.0),
]);

/// The classic engine (`nsspeech`) with other voices on macOS 15 and
/// earlier (measured with Samantha on 15.7): `(rate property, wpm)`.
pub const NS_OTHER_15: RateTable = RateTable::new(&[
    (80.0, 132.0),
    (100.0, 141.0),
    (120.0, 152.0),
    (140.0, 180.0),
    (175.0, 198.0),
    (200.0, 268.0),
    (225.0, 339.0),
    (250.0, 416.0),
    (265.0, 451.0),
    (300.0, 554.0),
    (350.0, 696.0),
    (400.0, 797.0),
]);

/// `AVSpeechSynthesizer` with Eloquence voices (measured with Reed):
/// `(utterance rate, wpm)`. Above the default 0.5 the rate climbs steeply.
pub const AV_ELOQUENCE: RateTable = RateTable::new(&[
    (0.00, 62.0),
    (0.05, 67.0),
    (0.10, 76.0),
    (0.15, 82.0),
    (0.20, 93.0),
    (0.25, 101.0),
    (0.30, 114.0),
    (0.35, 123.0),
    (0.40, 139.0),
    (0.45, 150.0),
    (0.50, 169.0),
    (0.55, 214.0),
    (0.60, 273.0),
    (0.65, 345.0),
    (0.70, 444.0),
    (0.75, 581.0),
    (0.80, 727.0),
    (0.85, 922.0),
]);

/// `AVSpeechSynthesizer` with other voices (measured with Samantha):
/// `(utterance rate, wpm)`.
pub const AV_OTHER: RateTable = RateTable::new(&[
    (0.00, 99.0),
    (0.05, 110.0),
    (0.10, 116.0),
    (0.15, 132.0),
    (0.20, 141.0),
    (0.25, 152.0),
    (0.35, 165.0),
    (0.40, 180.0),
    (0.50, 198.0),
    (0.55, 256.0),
    (0.60, 315.0),
    (0.65, 361.0),
    (0.70, 416.0),
    (0.75, 492.0),
    (0.80, 554.0),
    (0.85, 616.0),
    (0.90, 696.0),
    (1.00, 797.0),
]);

/// The `nsspeech` table for a voice on macOS `os_major` (26 and later use
/// the newer tables; earlier releases the macOS 15 ones).
pub fn ns_table(voice_id: Option<&str>, os_major: u32) -> RateTable {
    let eloquence = voice_id.is_some_and(crate::voices::is_eloquence);
    match (eloquence, os_major >= 26) {
        (true, true) => NS_ELOQUENCE,
        (false, true) => NS_OTHER,
        (true, false) => NS_ELOQUENCE_15,
        (false, false) => NS_OTHER_15,
    }
}

/// The `avspeech` table for a voice.
pub fn av_table(voice_id: Option<&str>) -> RateTable {
    if voice_id.is_some_and(crate::voices::is_eloquence) {
        AV_ELOQUENCE
    } else {
        AV_OTHER
    }
}

/// Pitch multiplier for a semitone offset: `2^(s/12)`, so ±12 semitones is
/// 0.5 to 2.0, which is `AVSpeechUtterance.pitchMultiplier`'s range.
pub fn pitch_multiplier(semitones: i8) -> f32 {
    2f32.powf(f32::from(semitones.clamp(-12, 12)) / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [RateTable; 6] = [
        NS_ELOQUENCE,
        NS_OTHER,
        NS_ELOQUENCE_15,
        NS_OTHER_15,
        AV_ELOQUENCE,
        AV_OTHER,
    ];

    #[test]
    fn tables_are_sorted_on_both_axes() {
        for t in ALL {
            for pair in t.points.windows(2) {
                assert!(pair[0].0 < pair[1].0, "{t:?}");
                assert!(pair[0].1 < pair[1].1, "{t:?}");
            }
        }
    }

    #[test]
    fn interpolation_round_trips_inside_the_range() {
        for t in ALL {
            let (lo, hi) = t.wpm_range();
            for wpm in [150u16, 200, 265, 350] {
                let w = f32::from(wpm);
                if w < lo || w > hi {
                    continue;
                }
                let e = t.engine_value(wpm);
                assert!((t.wpm_at(e) - w).abs() < 0.5, "{t:?} {wpm}");
                assert_eq!(t.effective_wpm(wpm), wpm);
            }
        }
    }

    #[test]
    fn default_rate_is_reachable_everywhere() {
        for t in ALL {
            let (lo, hi) = t.wpm_range();
            assert!(lo <= 265.0 && hi >= 265.0, "{t:?}");
        }
        // Reed at the default 265 wpm: property 292 on macOS 26, 214 on 15.
        assert!((NS_ELOQUENCE.engine_value(265) - 292.4).abs() < 1.0);
        assert!((NS_ELOQUENCE_15.engine_value(265) - 214.5).abs() < 1.0);
    }

    #[test]
    fn requests_outside_the_range_clamp() {
        let t = RateTable::new(&[(0.0, 100.0), (1.0, 500.0)]);
        assert_eq!(t.engine_value(50), 0.0);
        assert_eq!(t.engine_value(900), 1.0);
        assert_eq!(t.effective_wpm(50), 100);
        assert_eq!(t.effective_wpm(900), 500);
        assert_eq!(t.engine_value(300), 0.5);
        assert_eq!(t.wpm_at(0.25), 200.0);
        assert_eq!(RateTable::new(&[]).wpm_range(), (0.0, 0.0));
    }

    #[test]
    fn pitch_multipliers() {
        assert_eq!(pitch_multiplier(0), 1.0);
        assert!((pitch_multiplier(12) - 2.0).abs() < 1e-6);
        assert!((pitch_multiplier(-12) - 0.5).abs() < 1e-6);
        assert!((pitch_multiplier(100) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn eloquence_voices_use_their_own_tables() {
        assert_eq!(ns_table(Some(crate::DEFAULT_VOICE), 26), NS_ELOQUENCE);
        assert_eq!(ns_table(Some(crate::DEFAULT_VOICE), 15), NS_ELOQUENCE_15);
        assert_eq!(ns_table(None, 14), NS_OTHER_15);
        assert_eq!(ns_table(None, 27), NS_OTHER);
        assert_eq!(av_table(Some(crate::DEFAULT_VOICE)), AV_ELOQUENCE);
        assert_eq!(av_table(None), AV_OTHER);
    }
}
