//! The sidetone: the tone heard while keying, following the key.
//!
//! The same tone as the CW engine's, a sine shaped by the same
//! raised-cosine rise and fall ([`mod@crate::render`]), but made one sample
//! at a time as the key goes down and up, since a key's timing is not
//! known in advance. The edge always runs its whole length, so even a
//! very short contact does not click.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use crate::render::sine;

/// A keyed oscillator (see the module docs).
#[derive(Clone, Debug)]
pub struct Sidetone {
    /// Cycles per sample.
    step: f64,
    /// The phase, in cycles (0 to 1).
    phase: f64,
    /// The peak level, 0 to 1.
    level: f64,
    /// The edge, in samples.
    edge: u32,
    /// How far into the edge the envelope is: 0 silent, `edge` full.
    at: u32,
}

impl Sidetone {
    /// A tone of `hz` at `level` (0 to 1), with edges of `edge_ms`, at
    /// `sample_rate`.
    pub fn new(sample_rate: u32, hz: f64, level: f64, edge_ms: f64) -> Self {
        let rate = f64::from(sample_rate.max(1));
        let edge = (edge_ms.max(0.0) * rate / 1000.0).round();
        Sidetone {
            step: hz / rate,
            phase: 0.0,
            level: level.clamp(0.0, 1.0),
            // A few milliseconds of samples: far inside u32; at least one,
            // so an edge of 0 still sounds.
            edge: (edge as u32).max(1),
            at: 0,
        }
    }

    /// The next sample, from -1 to 1, with the key down (`down`) or up.
    pub fn next(&mut self, down: bool) -> f32 {
        if down {
            self.at = (self.at + 1).min(self.edge);
        } else {
            self.at = self.at.saturating_sub(1);
        }
        let envelope = if self.at == 0 {
            0.0
        } else {
            let t = f64::from(self.at) / f64::from(self.edge);
            0.5 * (1.0 - sine(PI * t + FRAC_PI_2))
        };
        let v = if envelope > 0.0 {
            self.level * envelope * sine(TAU * self.phase)
        } else {
            0.0
        };
        self.phase += self.step;
        self.phase -= self.phase.floor();
        v as f32
    }

    /// True while the tone sounds (the key is down, or the fall has not
    /// ended).
    pub fn sounding(&self) -> bool {
        self.at > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tone_rises_and_falls_without_a_step() {
        let mut s = Sidetone::new(22_050, 600.0, 0.5, 5.0);
        // Up: silence.
        assert!((0..100).all(|_| s.next(false) == 0.0));
        assert!(!s.sounding());
        let mut last = 0.0f32;
        let mut peak = 0.0f32;
        // 20 ms down, then 20 ms up: no jump between samples bigger than a
        // full-level sine's step at 600 Hz.
        let max_step = 0.5 * TAU * 600.0 / 22_050.0 + 0.02;
        for n in 0..882 {
            let v = s.next(n < 441);
            assert!(f64::from((v - last).abs()) <= max_step, "sample {n}");
            peak = peak.max(v.abs());
            last = v;
        }
        assert!(peak > 0.49 && peak <= 0.5, "{peak}");
        assert!(!s.sounding());
    }

    #[test]
    fn a_short_contact_still_rises_and_falls() {
        let mut s = Sidetone::new(22_050, 600.0, 1.0, 5.0);
        s.next(true);
        assert!(s.sounding());
        for _ in 0..200 {
            s.next(false);
        }
        assert!(!s.sounding());
    }
}
