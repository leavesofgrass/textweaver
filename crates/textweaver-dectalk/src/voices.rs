//! DECtalk's speakers as textweaver voices, and the rate and pitch
//! mapping (ADR-0004, ADR-0021).
//!
//! DECtalk has nine built-in speakers. Each is selected in the text with
//! its one-letter inline command, `[:n<letter>]` (`[:np]` is Perfect
//! Paul), which every DECtalk release understands, so the host never
//! depends on the numbering of the C API's `SPEAKER_T`. The order here is
//! `SPEAKER_T`'s, and the protocol sends a speaker as its position in it.
//!
//! Voice ids are `dectalk:<name>` (`dectalk:paul`). A bare name (`paul`),
//! the full name (`Perfect Paul`), or the letter (`p`) are accepted too,
//! which also reads the voice names star saved in its settings.
//!
//! **Rate.** DECtalk's `[:rate N]` is in words per minute, 75 to 600, so
//! textweaver's canonical rate passes straight through, clamped; DECtalk's
//! own rate is the effective rate (no calibration table is needed).
//!
//! **Pitch.** Each speaker has an average pitch (`[:dv ap N]`, in Hz). A
//! pitch offset of `n` semitones scales it by `2^(n/12)`, clamped to
//! DECtalk's 50..=350 Hz; no offset sends nothing, keeping the speaker's
//! own voice exactly.
//!
//! **Volume** is a playback gain in the backend, as for ECI: DECtalk's
//! `[:volume]` commands act only on its own audio device.

use textweaver_speech::Voice;

/// Slowest rate DECtalk accepts, in words per minute.
pub const MIN_RATE: u16 = 75;
/// Fastest rate DECtalk accepts, in words per minute.
pub const MAX_RATE: u16 = 600;
/// DECtalk's default rate, in words per minute.
pub const DEFAULT_RATE: u16 = 180;
/// Lowest average pitch DECtalk accepts, in Hz.
pub const MIN_PITCH_HZ: u16 = 50;
/// Highest average pitch DECtalk accepts, in Hz.
pub const MAX_PITCH_HZ: u16 = 350;

/// One of DECtalk's nine speakers, in `SPEAKER_T` order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Speaker {
    /// Perfect Paul, the default voice.
    #[default]
    Paul,
    /// Huge Harry.
    Harry,
    /// Frail Frank.
    Frank,
    /// Doctor Dennis.
    Dennis,
    /// Beautiful Betty.
    Betty,
    /// Uppity Ursula.
    Ursula,
    /// Whispering Wendy.
    Wendy,
    /// Rough Rita.
    Rita,
    /// Kit the Kid.
    Kit,
}

impl Speaker {
    /// Every speaker, in `SPEAKER_T` order.
    pub const ALL: [Speaker; 9] = [
        Speaker::Paul,
        Speaker::Harry,
        Speaker::Frank,
        Speaker::Dennis,
        Speaker::Betty,
        Speaker::Ursula,
        Speaker::Wendy,
        Speaker::Rita,
        Speaker::Kit,
    ];

    /// The speaker at position `n` of [`Speaker::ALL`].
    pub fn from_index(n: u8) -> Option<Speaker> {
        Speaker::ALL.get(usize::from(n)).copied()
    }

    /// Position in [`Speaker::ALL`] (the protocol's speaker number).
    pub fn index(self) -> u8 {
        self as u8
    }

    /// Short name: "Paul".
    pub fn name(self) -> &'static str {
        match self {
            Speaker::Paul => "Paul",
            Speaker::Harry => "Harry",
            Speaker::Frank => "Frank",
            Speaker::Dennis => "Dennis",
            Speaker::Betty => "Betty",
            Speaker::Ursula => "Ursula",
            Speaker::Wendy => "Wendy",
            Speaker::Rita => "Rita",
            Speaker::Kit => "Kit",
        }
    }

    /// DECtalk's full name: "Perfect Paul".
    pub fn full_name(self) -> &'static str {
        match self {
            Speaker::Paul => "Perfect Paul",
            Speaker::Harry => "Huge Harry",
            Speaker::Frank => "Frail Frank",
            Speaker::Dennis => "Doctor Dennis",
            Speaker::Betty => "Beautiful Betty",
            Speaker::Ursula => "Uppity Ursula",
            Speaker::Wendy => "Whispering Wendy",
            Speaker::Rita => "Rough Rita",
            Speaker::Kit => "Kit the Kid",
        }
    }

    /// The letter of the speaker's `[:n<letter>]` command.
    pub fn letter(self) -> char {
        match self {
            Speaker::Paul => 'p',
            Speaker::Harry => 'h',
            Speaker::Frank => 'f',
            Speaker::Dennis => 'd',
            Speaker::Betty => 'b',
            Speaker::Ursula => 'u',
            Speaker::Wendy => 'w',
            Speaker::Rita => 'r',
            Speaker::Kit => 'k',
        }
    }

    /// The speaker's average pitch (`ap`) in Hz, from DECtalk's speaker
    /// definitions.
    pub fn average_pitch(self) -> u16 {
        match self {
            Speaker::Paul => 122,
            Speaker::Harry => 89,
            Speaker::Frank => 155,
            Speaker::Dennis => 110,
            Speaker::Betty => 208,
            Speaker::Ursula => 240,
            Speaker::Wendy => 200,
            Speaker::Rita => 106,
            Speaker::Kit => 306,
        }
    }

    /// "male", "female", or `None` for Kit, a child's voice.
    pub fn gender(self) -> Option<&'static str> {
        match self {
            Speaker::Paul | Speaker::Harry | Speaker::Frank | Speaker::Dennis => Some("male"),
            Speaker::Betty | Speaker::Ursula | Speaker::Wendy | Speaker::Rita => Some("female"),
            Speaker::Kit => None,
        }
    }

    /// The voice id: `dectalk:paul`.
    pub fn voice_id(self) -> String {
        format!("dectalk:{}", self.name().to_ascii_lowercase())
    }

    /// Finds a speaker by voice id (`dectalk:paul`), name (`Paul`), full
    /// name (`Perfect Paul`), or letter (`p`), ignoring case and
    /// surrounding space.
    pub fn parse(id: &str) -> Option<Speaker> {
        let s = id.trim().to_ascii_lowercase();
        let s = s.strip_prefix("dectalk:").unwrap_or(&s).trim();
        let s = s.strip_prefix("dectalk ").unwrap_or(s).trim();
        Speaker::ALL.into_iter().find(|sp| {
            s == sp.name().to_ascii_lowercase()
                || s == sp.full_name().to_ascii_lowercase()
                || (s.len() == 1 && s.starts_with(sp.letter()))
        })
    }

    /// The speaker as a textweaver voice.
    pub fn voice(self) -> Voice {
        let mut tags = vec!["DECtalk".to_owned()];
        if self == Speaker::Kit {
            tags.push("child".to_owned());
        }
        Voice {
            id: self.voice_id(),
            name: format!("DECtalk {}", self.full_name()),
            languages: vec!["en-US".to_owned()],
            gender: self.gender().map(str::to_owned),
            tags,
        }
    }
}

/// Every speaker as a voice, Paul first.
pub fn voice_list() -> Vec<Voice> {
    Speaker::ALL.into_iter().map(Speaker::voice).collect()
}

/// DECtalk's rate for a canonical rate in words per minute.
pub fn rate(wpm: u16) -> u16 {
    wpm.clamp(MIN_RATE, MAX_RATE)
}

/// The average pitch to send for `speaker` shifted by `semitones`, or 0
/// (send nothing) when there is no shift.
pub fn pitch_hz(speaker: Speaker, semitones: i8) -> u16 {
    if semitones == 0 {
        return 0;
    }
    let hz = f64::from(speaker.average_pitch()) * 2f64.powf(f64::from(semitones) / 12.0);
    // Clamped to 50..=350 before the conversion, so it cannot overflow.
    hz.round()
        .clamp(f64::from(MIN_PITCH_HZ), f64::from(MAX_PITCH_HZ)) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_speakers_in_speaker_t_order() {
        let names: Vec<&str> = Speaker::ALL.iter().map(|s| s.name()).collect();
        assert_eq!(
            names,
            [
                "Paul", "Harry", "Frank", "Dennis", "Betty", "Ursula", "Wendy", "Rita", "Kit"
            ]
        );
        for (i, s) in Speaker::ALL.into_iter().enumerate() {
            assert_eq!(usize::from(s.index()), i);
            assert_eq!(Speaker::from_index(s.index()), Some(s));
        }
        assert_eq!(Speaker::from_index(9), None);
        // Every letter is distinct.
        let mut letters: Vec<char> = Speaker::ALL.iter().map(|s| s.letter()).collect();
        letters.sort_unstable();
        letters.dedup();
        assert_eq!(letters.len(), 9);
    }

    #[test]
    fn voices_are_listed_with_ids_names_and_tags() {
        let v = voice_list();
        assert_eq!(v.len(), 9);
        assert_eq!(v[0].id, "dectalk:paul");
        assert_eq!(v[0].name, "DECtalk Perfect Paul");
        assert_eq!(v[0].languages, ["en-US"]);
        assert_eq!(v[0].gender.as_deref(), Some("male"));
        assert!(v[0].has_tag("dectalk"));
        assert_eq!(v[4].name, "DECtalk Beautiful Betty");
        assert_eq!(v[4].gender.as_deref(), Some("female"));
        assert_eq!(v[8].gender, None);
        assert!(v[8].has_tag("child"));
    }

    #[test]
    fn ids_names_and_letters_parse() {
        for s in Speaker::ALL {
            assert_eq!(Speaker::parse(&s.voice_id()), Some(s));
            assert_eq!(Speaker::parse(s.name()), Some(s));
            assert_eq!(Speaker::parse(s.full_name()), Some(s));
            assert_eq!(Speaker::parse(&s.voice().name), Some(s));
            assert_eq!(Speaker::parse(&s.letter().to_string()), Some(s));
        }
        assert_eq!(Speaker::parse("  DECtalk:BETTY "), Some(Speaker::Betty));
        assert_eq!(Speaker::parse("x"), None);
        assert_eq!(Speaker::parse("eci:enu:reed"), None);
        assert_eq!(Speaker::parse(""), None);
    }

    #[test]
    fn rate_passes_through_within_dectalks_range() {
        assert_eq!(rate(180), 180);
        assert_eq!(rate(40), 75);
        assert_eq!(rate(900), 600);
    }

    #[test]
    fn pitch_scales_the_speakers_own_average() {
        assert_eq!(pitch_hz(Speaker::Paul, 0), 0);
        assert_eq!(pitch_hz(Speaker::Paul, 12), 244);
        assert_eq!(pitch_hz(Speaker::Paul, -12), 61);
        assert_eq!(pitch_hz(Speaker::Betty, 2), 233);
        assert_eq!(pitch_hz(Speaker::Kit, 12), 350);
        assert_eq!(pitch_hz(Speaker::Harry, -24), 50);
    }
}
