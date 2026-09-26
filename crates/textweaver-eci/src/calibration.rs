//! Mapping textweaver's canonical rate and pitch (ADR-0004) onto ECI.
//!
//! **Rate.** ECI's speed voice parameter runs 0..=250 and is not linear in
//! words per minute. [`VOXIN`] was measured with Voxin (libvoxin 1.5.8,
//! ETI-Eloquence for Linux) by synthesizing a fixed 105-word passage of
//! ordinary prose at each speed with voice Reed and dividing the word count
//! by the audio duration (`cargo run -p textweaver-eci --example calibrate`
//! in the container with the Voxin overlay). The requested wpm is mapped to
//! a speed by linear interpolation between the measured points, clamped to
//! the table; [`effective_wpm`] reports the rate the chosen speed actually
//! achieves, which the timer pacer uses.
//!
//! Code Factory's Windows engine has not been measured (it needs a licensed
//! engine). Until it is, [`CODE_FACTORY`] is the Voxin table, since both are
//! builds of the same ETI-Eloquence 6.x core and share the speed scale.
//!
//! **Pitch.** The pitch baseline voice parameter runs 0..=100. Measured
//! with Voxin (median F0 of a sustained sentence per baseline value), each
//! baseline step raises F0 by about [`BASELINE_PER_SEMITONE`]⁻¹ semitones
//! across the useful range, so a semitone offset maps to the preset's own
//! baseline plus `semitones * BASELINE_PER_SEMITONE`, clamped to 0..=100.

/// One measured point: ECI speed and the words per minute it produced.
pub type RatePoint = (i32, u16);

/// Voxin (ETI-Eloquence for Linux, libvoxin 1.5.8), voice Reed, 11025 Hz.
pub const VOXIN: &[RatePoint] = &[
    (0, 76),
    (10, 94),
    (20, 111),
    (30, 129),
    (40, 147),
    (50, 164),
    (60, 182),
    (70, 199),
    (80, 217),
    (90, 234),
    (100, 252),
    (120, 287),
    (140, 322),
    (160, 357),
    (180, 392),
    (200, 428),
    (225, 471),
    (250, 515),
];

/// Code Factory Eloquence for Windows: not yet measured (needs a licensed
/// engine); provisionally the Voxin table.
pub const CODE_FACTORY: &[RatePoint] = VOXIN;

/// Pitch baseline units per semitone.
pub const BASELINE_PER_SEMITONE: f32 = 3.0;

/// The rate table for the running engine, chosen by the version string the
/// host reported and the platform.
pub fn table_for(_version: &str) -> &'static [RatePoint] {
    if cfg!(windows) { CODE_FACTORY } else { VOXIN }
}

/// The ECI speed for `wpm`, interpolated and clamped to the table.
pub fn speed_for_wpm(table: &[RatePoint], wpm: u16) -> i32 {
    let (Some(first), Some(last)) = (table.first(), table.last()) else {
        return 50;
    };
    if wpm <= first.1 {
        return first.0;
    }
    if wpm >= last.1 {
        return last.0;
    }
    for pair in table.windows(2) {
        let ((s0, w0), (s1, w1)) = (pair[0], pair[1]);
        if wpm >= w0 && wpm <= w1 {
            if w1 == w0 {
                return s0;
            }
            let t = f32::from(wpm - w0) / f32::from(w1 - w0);
            return s0 + ((s1 - s0) as f32 * t).round() as i32;
        }
    }
    last.0
}

/// The wpm an ECI speed achieves, interpolated from the table.
pub fn effective_wpm(table: &[RatePoint], speed: i32) -> u16 {
    let (Some(first), Some(last)) = (table.first(), table.last()) else {
        return 0;
    };
    if speed <= first.0 {
        return first.1;
    }
    if speed >= last.0 {
        return last.1;
    }
    for pair in table.windows(2) {
        let ((s0, w0), (s1, w1)) = (pair[0], pair[1]);
        if speed >= s0 && speed <= s1 {
            let t = (speed - s0) as f32 / (s1 - s0).max(1) as f32;
            return (f32::from(w0) + (f32::from(w1) - f32::from(w0)) * t).round() as u16;
        }
    }
    last.1
}

/// The pitch baseline for `semitones` relative to a preset's baseline.
pub fn pitch_baseline(preset_baseline: i32, semitones: i8) -> i32 {
    let delta = (f32::from(semitones) * BASELINE_PER_SEMITONE).round() as i32;
    (preset_baseline + delta).clamp(0, 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_strictly_increasing() {
        for t in [VOXIN, CODE_FACTORY] {
            for pair in t.windows(2) {
                assert!(pair[0].0 < pair[1].0 && pair[0].1 < pair[1].1, "{pair:?}");
            }
            assert_eq!(t.first().map(|p| p.0), Some(0));
            assert_eq!(t.last().map(|p| p.0), Some(250));
        }
    }

    #[test]
    fn wpm_round_trips_within_the_table() {
        for wpm in (VOXIN[0].1..=VOXIN[VOXIN.len() - 1].1).step_by(7) {
            let s = speed_for_wpm(VOXIN, wpm);
            let back = effective_wpm(VOXIN, s);
            assert!(back.abs_diff(wpm) <= 3, "{wpm} -> speed {s} -> {back}");
        }
    }

    #[test]
    fn out_of_range_rates_clamp_and_report_what_is_achieved() {
        assert_eq!(speed_for_wpm(VOXIN, 50), 0);
        assert_eq!(effective_wpm(VOXIN, speed_for_wpm(VOXIN, 50)), VOXIN[0].1);
        assert_eq!(speed_for_wpm(VOXIN, 900), 250);
        assert_eq!(effective_wpm(VOXIN, 250), VOXIN[VOXIN.len() - 1].1);
        assert_eq!(speed_for_wpm(&[], 300), 50);
    }

    #[test]
    fn pitch_offsets_move_the_baseline_and_clamp() {
        assert_eq!(pitch_baseline(65, 0), 65);
        assert_eq!(pitch_baseline(65, 2), 71);
        assert_eq!(pitch_baseline(65, -3), 56);
        assert_eq!(pitch_baseline(65, 12), 100);
        assert_eq!(pitch_baseline(10, -12), 0);
    }
}
