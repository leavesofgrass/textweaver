//! Mapping textweaver's canonical rate and pitch (ADR-0004) onto ECI.
//!
//! Both tables were measured on 2026-09-25 with Voxin (libvoxin 1.5.8,
//! ETI-Eloquence for Linux) in the dev container, voice Reed, 11025 Hz, by
//! `cargo run -p textweaver-eci --example calibrate` (with the Voxin
//! overlay). They are data, not formulas: the engine's scales are not
//! linear, and interpolating between measured points is exact enough for
//! pacing (within about 2 %).
//!
//! **Rate.** ECI's speed voice parameter runs 0..=250. Each point is the
//! words per minute of a fixed 98-word prose passage at that speed, timed
//! from the first word's index mark to the end of the audio. The requested
//! wpm maps to a speed by linear interpolation, clamped to the table (so
//! 50 wpm gives speed 0, which achieves 58 wpm); [`effective_wpm`] reports
//! what the chosen speed achieves, which the timer pacer uses. The default
//! speed of 50 is only 158 wpm; textweaver's default 265 wpm is speed 76.
//!
//! Code Factory's Windows engine has not been measured: that needs a
//! licensed engine. Until it is, [`CODE_FACTORY`] reuses the Voxin table
//! (both are ETI-Eloquence 6.x; the Windows scale may differ).
//!
//! **Pitch.** The pitch-baseline voice parameter runs 0..=100. [`PITCH_HZ`]
//! is the median fundamental frequency of one sentence at each baseline
//! (below 25 the voice bottoms out near 42 Hz). F0 is close to exponential
//! in the baseline, roughly 1.8 baseline units per semitone around the
//! presets' values. A pitch offset of `n` semitones targets the preset's
//! own F0 times `2^(n/12)` and maps back to a baseline through the table.

/// One measured point: ECI speed and the words per minute it produced.
pub type RatePoint = (i32, u16);

/// Voxin (ETI-Eloquence for Linux, libvoxin 1.5.8), voice Reed, 11025 Hz.
pub const VOXIN: &[RatePoint] = &[
    (0, 58),
    (10, 71),
    (20, 87),
    (30, 106),
    (40, 130),
    (50, 158),
    (55, 172),
    (60, 194),
    (65, 208),
    (70, 236),
    (75, 256),
    (80, 287),
    (85, 310),
    (90, 350),
    (95, 386),
    (100, 427),
    (105, 466),
    (110, 525),
    (115, 563),
    (120, 624),
    (125, 681),
    (130, 779),
    (135, 818),
    (140, 910),
    (160, 1350),
    (180, 1966),
    (200, 2414),
    (225, 2979),
    (250, 3133),
];

/// Code Factory Eloquence for Windows: to be measured with a licensed
/// engine; provisionally the Voxin table.
pub const CODE_FACTORY: &[RatePoint] = VOXIN;

/// Median F0 in Hz per pitch baseline (Voxin, voice Reed).
pub const PITCH_HZ: &[(i32, f32)] = &[
    (25, 47.3),
    (30, 50.8),
    (35, 54.9),
    (40, 60.2),
    (45, 66.8),
    (50, 75.5),
    (55, 85.5),
    (60, 98.4),
    (65, 114.8),
    (70, 134.5),
    (75, 159.8),
    (80, 190.1),
    (85, 229.7),
    (90, 275.6),
    (95, 334.1),
    (100, 408.3),
];

/// The rate table for the running engine: Code Factory's on Windows,
/// Voxin's elsewhere.
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

/// F0 at a baseline, interpolated in log frequency and clamped to the table.
fn hz_at(baseline: i32) -> f32 {
    let (first, last) = (PITCH_HZ[0], PITCH_HZ[PITCH_HZ.len() - 1]);
    if baseline <= first.0 {
        return first.1;
    }
    if baseline >= last.0 {
        return last.1;
    }
    for pair in PITCH_HZ.windows(2) {
        let ((b0, f0), (b1, f1)) = (pair[0], pair[1]);
        if baseline >= b0 && baseline <= b1 {
            let t = (baseline - b0) as f32 / (b1 - b0) as f32;
            return (f0.ln() + (f1.ln() - f0.ln()) * t).exp();
        }
    }
    last.1
}

/// The baseline for a target F0, interpolated in log frequency and
/// clamped to 0..=100.
fn baseline_for_hz(hz: f32) -> i32 {
    let (first, last) = (PITCH_HZ[0], PITCH_HZ[PITCH_HZ.len() - 1]);
    if hz <= first.1 {
        // Below the table the voice barely changes; go no lower than the
        // proportional step below the first point.
        let per = (PITCH_HZ[1].1 / first.1).ln() / (PITCH_HZ[1].0 - first.0) as f32;
        let b = first.0 as f32 + (hz / first.1).ln() / per;
        return (b.round() as i32).clamp(0, 100);
    }
    if hz >= last.1 {
        return 100;
    }
    for pair in PITCH_HZ.windows(2) {
        let ((b0, f0), (b1, f1)) = (pair[0], pair[1]);
        if hz >= f0 && hz <= f1 {
            let t = (hz.ln() - f0.ln()) / (f1.ln() - f0.ln());
            return (b0 as f32 + (b1 - b0) as f32 * t).round() as i32;
        }
    }
    100
}

/// The pitch baseline for `semitones` relative to a preset's baseline.
pub fn pitch_baseline(preset_baseline: i32, semitones: i8) -> i32 {
    if semitones == 0 {
        return preset_baseline.clamp(0, 100);
    }
    let target = hz_at(preset_baseline) * 2f32.powf(f32::from(semitones) / 12.0);
    baseline_for_hz(target)
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
        for pair in PITCH_HZ.windows(2) {
            assert!(pair[0].0 < pair[1].0 && pair[0].1 < pair[1].1, "{pair:?}");
        }
    }

    #[test]
    fn wpm_round_trips_within_the_supported_range() {
        for wpm in (58..=900).step_by(7) {
            let s = speed_for_wpm(VOXIN, wpm);
            let back = effective_wpm(VOXIN, s);
            // One speed step is up to 4 % of the rate at the fast end.
            let tol = (u32::from(wpm) * 4 / 100 + 3) as u16;
            assert!(back.abs_diff(wpm) <= tol, "{wpm} -> speed {s} -> {back}");
        }
        assert_eq!(speed_for_wpm(VOXIN, 265), 76);
        assert_eq!(speed_for_wpm(VOXIN, 158), 50);
    }

    #[test]
    fn out_of_range_rates_clamp_and_report_what_is_achieved() {
        assert_eq!(speed_for_wpm(VOXIN, 50), 0);
        assert_eq!(effective_wpm(VOXIN, speed_for_wpm(VOXIN, 50)), 58);
        assert_eq!(speed_for_wpm(VOXIN, 900), 139);
        assert_eq!(effective_wpm(VOXIN, 250), 3133);
        assert_eq!(speed_for_wpm(&[], 300), 50);
    }

    #[test]
    fn pitch_offsets_move_by_semitones_and_clamp() {
        assert_eq!(pitch_baseline(65, 0), 65);
        // Reed (65): an octave up lands on 85 (229.7 Hz = 2 x 114.8).
        assert_eq!(pitch_baseline(65, 12), 85);
        // A semitone is about two baseline units near Reed.
        assert_eq!(pitch_baseline(65, 1), 67);
        assert_eq!(pitch_baseline(65, -1), 63);
        let down = pitch_baseline(65, -12);
        assert!((36..=39).contains(&down), "{down}");
        // Shelley (81) two octaves up is past the top.
        assert_eq!(pitch_baseline(81, 12), 99);
        assert_eq!(pitch_baseline(93, 12), 100);
        assert_eq!(pitch_baseline(30, -12), 0);
        // Monotonic in semitones.
        for b in [40, 65, 81, 93] {
            let mut prev = -1;
            for st in -12..=12 {
                let v = pitch_baseline(b, st);
                assert!(v >= prev, "baseline {b} st {st}");
                prev = v;
            }
        }
    }
}
