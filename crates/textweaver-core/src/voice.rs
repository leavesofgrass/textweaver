//! Engine-independent voice parameters (ADR-0004).
//!
//! Backends map these onto their own scales and report what they actually
//! achieved (`SpeechBackend::effective_wpm`), which the timer pacer uses.

use serde::{Deserialize, Serialize};

/// Speaking rate. Canonical unit is words per minute. Serializes as a plain
/// number of wpm (`rate = 265`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "u16", into = "u16")]
pub enum Rate {
    /// Words per minute.
    Wpm(u16),
}

impl Rate {
    /// Slowest supported rate.
    pub const MIN_WPM: u16 = 50;
    /// Fastest supported rate.
    pub const MAX_WPM: u16 = 900;
    /// star's default rate.
    pub const DEFAULT_WPM: u16 = 265;

    /// The rate in words per minute.
    pub fn wpm(self) -> u16 {
        match self {
            Rate::Wpm(w) => w,
        }
    }

    /// The rate clamped to `MIN_WPM..=MAX_WPM`.
    pub fn clamped(self) -> Self {
        Rate::Wpm(self.wpm().clamp(Self::MIN_WPM, Self::MAX_WPM))
    }

    /// The rate changed by `delta` wpm, clamped.
    pub fn step(self, delta: i32) -> Self {
        let w = (i32::from(self.wpm()) + delta)
            .clamp(i32::from(Self::MIN_WPM), i32::from(Self::MAX_WPM));
        // The clamp keeps `w` inside u16 range.
        Rate::Wpm(u16::try_from(w).unwrap_or(Self::DEFAULT_WPM))
    }
}

impl From<u16> for Rate {
    fn from(wpm: u16) -> Self {
        Rate::Wpm(wpm)
    }
}

impl From<Rate> for u16 {
    fn from(rate: Rate) -> Self {
        rate.wpm()
    }
}

impl Default for Rate {
    fn default() -> Self {
        Rate::Wpm(Self::DEFAULT_WPM)
    }
}

/// Voice pitch relative to the voice's default, in semitones. Zero is the
/// voice's own pitch. Serializes as a plain number (`pitch = -2`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "i8", into = "i8")]
pub enum Pitch {
    /// Semitones above (positive) or below (negative) the default.
    Semitones(i8),
}

impl Pitch {
    /// Lowest supported offset.
    pub const MIN_SEMITONES: i8 = -12;
    /// Highest supported offset.
    pub const MAX_SEMITONES: i8 = 12;

    /// The offset in semitones.
    pub fn semitones(self) -> i8 {
        match self {
            Pitch::Semitones(s) => s,
        }
    }

    /// The pitch clamped to the supported range.
    pub fn clamped(self) -> Self {
        Pitch::Semitones(
            self.semitones()
                .clamp(Self::MIN_SEMITONES, Self::MAX_SEMITONES),
        )
    }

    /// The pitch changed by `delta` semitones, clamped.
    pub fn step(self, delta: i8) -> Self {
        Pitch::Semitones(self.semitones().saturating_add(delta)).clamped()
    }
}

impl Default for Pitch {
    fn default() -> Self {
        Pitch::Semitones(0)
    }
}

impl From<i8> for Pitch {
    fn from(s: i8) -> Self {
        Pitch::Semitones(s)
    }
}

impl From<Pitch> for i8 {
    fn from(p: Pitch) -> Self {
        p.semitones()
    }
}

/// Output volume as a percentage of the engine's maximum, `0..=100`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Volume(u8);

impl Volume {
    /// Full volume.
    pub const MAX: Volume = Volume(100);

    /// Creates a volume, clamping to `0..=100`.
    pub fn new(percent: u8) -> Self {
        Volume(percent.min(100))
    }

    /// The volume in percent.
    pub fn percent(self) -> u8 {
        self.0
    }

    /// The volume as a fraction in `0.0..=1.0`.
    pub fn fraction(self) -> f32 {
        f32::from(self.0) / 100.0
    }

    /// The volume changed by `delta` percent, clamped.
    pub fn step(self, delta: i16) -> Self {
        let v = (i16::from(self.0) + delta).clamp(0, 100);
        Volume(u8::try_from(v).unwrap_or(100))
    }
}

impl Default for Volume {
    fn default() -> Self {
        Volume::MAX
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_steps_and_clamps() {
        assert_eq!(Rate::default().wpm(), 265);
        assert_eq!(Rate::Wpm(60).step(-100).wpm(), Rate::MIN_WPM);
        assert_eq!(Rate::Wpm(880).step(100).wpm(), Rate::MAX_WPM);
        assert_eq!(Rate::Wpm(265).step(25).wpm(), 290);
    }

    #[test]
    fn pitch_and_volume() {
        assert_eq!(Pitch::default().step(3), Pitch::Semitones(3));
        assert_eq!(Pitch::Semitones(11).step(5), Pitch::Semitones(12));
        assert_eq!(Volume::new(150).percent(), 100);
        assert_eq!(Volume::new(5).step(-10).percent(), 0);
    }
}
