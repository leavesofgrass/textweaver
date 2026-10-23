//! The tone: a schedule as 16-bit mono samples.
//!
//! Each element is a sine tone shaped by a raised-cosine rise and fall,
//! `0.5 (1 - cos(pi t / R))` over the edge time `R` (5 ms by default,
//! never more than half the element), so the tone starts and stops
//! without a click. The sine runs on through the gaps (its phase is the
//! sample's position), so every element starts where the oscillator is.
//!
//! The sine is our own polynomial, not the platform's: plain additions,
//! multiplications and divisions are exact to IEEE 754 everywhere, so the
//! same schedule renders the same samples on Windows, Linux and macOS,
//! and a golden hash can check them.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use crate::schedule::Schedule;

/// How a schedule sounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderOptions {
    /// Samples per second.
    pub sample_rate: u32,
    /// The tone's pitch in hertz.
    pub hz: f64,
    /// The peak level, from 0 (silent) to 1 (full scale).
    pub level: f64,
    /// The rise and fall of each element, in milliseconds.
    pub edge_ms: f64,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            sample_rate: 22_050,
            hz: 600.0,
            level: 0.5,
            edge_ms: 5.0,
        }
    }
}

/// The sample at `ms` milliseconds, at `sample_rate`: rounded to the
/// nearest sample, as the renderer places every tone. Word and character
/// times convert with this, so they match the audio exactly.
pub fn to_samples(ms: f64, sample_rate: u32) -> u64 {
    let s = (ms * f64::from(sample_rate) / 1000.0).round();
    if s <= 0.0 {
        0
    } else {
        // Times are at most minutes long: far inside u64.
        s as u64
    }
}

/// `sin(x)` for any finite `x`, from a Taylor polynomial on
/// `[-pi/2, pi/2]` (error below 1e-11), with IEEE-exact operations only.
pub(crate) fn sine(x: f64) -> f64 {
    // Into [-pi, pi).
    let mut r = x - TAU * ((x + PI) / TAU).floor();
    // Into [-pi/2, pi/2]: sin(pi - r) = sin(r).
    if r > FRAC_PI_2 {
        r = PI - r;
    } else if r < -FRAC_PI_2 {
        r = -PI - r;
    }
    let r2 = r * r;
    // Horner's form of r - r^3/3! + ... - r^15/15!.
    let mut sum = 1.0;
    for n in (1..=7u32).rev() {
        let k = f64::from(2 * n);
        sum = 1.0 - r2 / (k * (k + 1.0)) * sum;
    }
    r * sum
}

/// The envelope `k + 0.5` samples into an edge of `edge` samples.
fn rise(k: u64, edge: u64) -> f64 {
    if k >= edge {
        return 1.0;
    }
    // Sample counts are small: exact in f64.
    let t = (k as f64 + 0.5) / edge as f64;
    0.5 * (1.0 - sine(PI * t + FRAC_PI_2))
}

/// Renders `schedule` (see the module docs): `schedule.length_ms` of
/// audio, its trailing word gap included.
pub fn render(schedule: &Schedule, options: &RenderOptions) -> Vec<i16> {
    let rate = options.sample_rate.max(1);
    let total = usize::try_from(to_samples(schedule.length_ms, rate)).unwrap_or(0);
    let mut out = vec![0i16; total];
    let level = options.level.clamp(0.0, 1.0) * f64::from(i16::MAX);
    let step = options.hz / f64::from(rate);
    let edge_ms = options.edge_ms.max(0.0);
    for tone in &schedule.tones {
        let a = to_samples(tone.start_ms, rate);
        let b = to_samples(tone.end_ms, rate).min(total as u64);
        if b <= a {
            continue;
        }
        let edge = to_samples(edge_ms, rate).min((b - a) / 2);
        for n in a..b {
            let k = (n - a).min(b - 1 - n);
            // The phase as a fraction of a cycle keeps the argument small.
            let cycles = step * n as f64;
            let phase = TAU * (cycles - cycles.floor());
            let v = level * rise(k, edge) * sine(phase);
            let i = usize::try_from(n).unwrap_or(usize::MAX);
            if let Some(slot) = out.get_mut(i) {
                *slot = v.round().clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::{Options, schedule};
    use crate::timing::Timing;

    /// FNV-1a over the samples' little-endian bytes.
    fn fnv(samples: &[i16]) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for s in samples {
            for b in s.to_le_bytes() {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        h
    }

    #[test]
    fn the_sine_is_accurate() {
        for i in -400..=400 {
            let x = f64::from(i) * 0.05;
            assert!((sine(x) - x.sin()).abs() < 1e-10, "{x}");
        }
    }

    #[test]
    fn samples_round_to_the_nearest() {
        assert_eq!(to_samples(60.0, 22_050), 1323);
        assert_eq!(to_samples(0.0, 22_050), 0);
        assert_eq!(to_samples(1000.0 / 22_050.0 * 0.6, 22_050), 1);
    }

    #[test]
    fn elements_rise_and_fall_without_a_click() {
        let opts = Options {
            timing: Timing::new(20, 20),
            prosigns: true,
        };
        let s = schedule("E", &opts);
        let r = RenderOptions::default();
        let pcm = render(&s, &r);
        // A dit (60 ms) and a word gap (420 ms).
        assert_eq!(pcm.len(), to_samples(480.0, 22_050) as usize);
        let dit = to_samples(60.0, 22_050) as usize;
        // The edges are small, the middle reaches the level.
        assert!(pcm[0].unsigned_abs() < 200, "{}", pcm[0]);
        assert!(pcm[dit - 1].unsigned_abs() < 200, "{}", pcm[dit - 1]);
        let peak = pcm[..dit]
            .iter()
            .map(|s| s.unsigned_abs())
            .max()
            .unwrap_or(0);
        assert!((16_000..=16_384).contains(&peak), "{peak}");
        // Silence after.
        assert!(pcm[dit..].iter().all(|s| *s == 0));
        // No sample steps by more than a sine at 600 Hz can.
        let max_step = pcm[..dit]
            .windows(2)
            .map(|w| (i32::from(w[1]) - i32::from(w[0])).abs())
            .max()
            .unwrap_or(0);
        assert!(max_step < 3000, "{max_step}");
    }

    #[test]
    fn a_short_clip_renders_the_same_everywhere() {
        // "CQ" at 20 WPM, Farnsworth 10, 600 Hz, 22,050 Hz. The hash is
        // of the samples' little-endian bytes. If the renderer changes on
        // purpose, print the new hash and check the clip by ear first.
        let s = schedule("CQ", &Options::default());
        let pcm = render(&s, &RenderOptions::default());
        assert_eq!(pcm.len(), to_samples(s.length_ms, 22_050) as usize);
        assert_eq!(fnv(&pcm), GOLDEN, "new hash {:#018x}", fnv(&pcm));
    }

    const GOLDEN: u64 = 0x5762_1a0f_6eb1_70d1;
}
