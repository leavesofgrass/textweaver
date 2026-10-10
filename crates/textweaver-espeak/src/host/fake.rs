//! A deterministic stand-in for eSpeak NG, selected with
//! `textweaver-espeak-host --engine fake`. The backend's tests use it to
//! exercise the real host binary, the protocol, and the playback client
//! without libespeak-ng (in any environment).
//!
//! Behavior: 22,050 Hz, as eSpeak NG; every byte of text produces
//! [`SAMPLES_PER_BYTE`] samples at 175 words per minute (eSpeak NG's
//! default rate), scaled by the rate (350 wpm gives half as many); the
//! samples are a square wave whose amplitude tells the voices apart; each
//! word (a run of characters other than white space) is reported at its
//! first sample. Two voices: `fake/en` and `fake/fr`. Special words, for
//! testing failure paths: `__crash__` exits the process at once;
//! `__fail__` makes synthesis return an error; `__hang__` never returns
//! (a hung engine); `__slow__` sleeps 20 ms per audio block.

use std::time::Duration;

use super::{Engine, EngineInfo, SynthEvent, VoiceSettings};
use crate::protocol::VoiceEntry;

/// Samples per byte of text at 175 words per minute.
pub const SAMPLES_PER_BYTE: usize = 100;
/// Samples per audio block.
pub const BLOCK: usize = 512;
/// The fake engine's sample rate (eSpeak NG's).
pub const SAMPLE_RATE: u32 = 22_050;
/// eSpeak NG's default rate, at which [`SAMPLES_PER_BYTE`] holds.
pub const DEFAULT_RATE: u16 = 175;

/// The fake engine.
#[derive(Debug)]
pub struct FakeEngine {
    voice: VoiceSettings,
}

impl Default for FakeEngine {
    fn default() -> Self {
        FakeEngine {
            voice: VoiceSettings {
                voice: String::new(),
                rate: DEFAULT_RATE,
                pitch: 0,
            },
        }
    }
}

/// The fake's amplitude for a voice: 1000 for `fake/en` (and the
/// default), 2000 for `fake/fr`.
pub fn amplitude(voice: &str) -> i16 {
    if voice == "fake/fr" { 2000 } else { 1000 }
}

fn voices() -> Vec<VoiceEntry> {
    ["en", "fr"]
        .iter()
        .map(|l| VoiceEntry {
            id: format!("fake/{l}"),
            name: format!("Fake {l}"),
            gender: 0,
            languages: vec![(*l).to_owned()],
        })
        .collect()
}

impl Engine for FakeEngine {
    fn info(&mut self) -> EngineInfo {
        EngineInfo {
            sample_rate: SAMPLE_RATE,
            version: "fake 1.0".into(),
            engine: "fake".into(),
            voices: voices(),
        }
    }

    fn set_voice(&mut self, voice: &VoiceSettings) -> Result<(), String> {
        if !voice.voice.is_empty() && !voices().iter().any(|v| v.id == voice.voice) {
            return Err(format!("no voice {}", voice.voice));
        }
        self.voice = voice.clone();
        Ok(())
    }

    fn synthesize(
        &mut self,
        text: &str,
        _character: bool,
        out: &mut dyn FnMut(SynthEvent<'_>) -> bool,
    ) -> Result<bool, String> {
        if text.contains("__crash__") {
            std::process::exit(3);
        }
        if text.contains("__fail__") {
            return Err("the fake engine failed".into());
        }
        if text.contains("__hang__") {
            loop {
                std::thread::sleep(Duration::from_secs(3600));
            }
        }
        let slow = text.contains("__slow__");
        let rate = usize::from(self.voice.rate.max(1));
        let per_byte = (SAMPLES_PER_BYTE * usize::from(DEFAULT_RATE) / rate).max(1);
        let amp = amplitude(&self.voice.voice);
        let total = text.len() * per_byte;
        let mut words = word_starts(text).into_iter().peekable();
        let mut at = 0usize;
        let mut block = Vec::with_capacity(BLOCK);
        while at < total {
            let n = (total - at).min(BLOCK);
            // Every word that starts inside this block, before its audio.
            while let Some(w) = words.next_if(|w| (w.start as usize) * per_byte < at + n) {
                let sample = (w.start as usize * per_byte) as u64;
                if !out(SynthEvent::Word(w, sample)) {
                    return Ok(false);
                }
            }
            block.clear();
            block.extend((at..at + n).map(|i| if (i / 50) % 2 == 0 { amp } else { -amp }));
            if !out(SynthEvent::Audio(&block)) {
                return Ok(false);
            }
            if slow {
                std::thread::sleep(Duration::from_millis(20));
            }
            at += n;
        }
        Ok(true)
    }
}

/// The byte range of every run of characters other than white space.
pub fn word_starts(text: &str) -> Vec<std::ops::Range<u32>> {
    let to_u32 = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        match (start, c.is_whitespace()) {
            (None, false) => start = Some(i),
            (Some(s), true) => {
                out.push(to_u32(s)..to_u32(i));
                start = None;
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_and_audio_come_in_sample_order() {
        let mut e = FakeEngine::default();
        let mut log = Vec::new();
        let mut samples = 0u64;
        let done = e
            .synthesize("Hello brave world", false, &mut |ev| {
                match ev {
                    SynthEvent::Word(r, s) => {
                        assert!(s >= samples, "a word comes before its audio");
                        log.push((r, s));
                    }
                    SynthEvent::Audio(a) => samples += a.len() as u64,
                }
                true
            })
            .unwrap();
        assert!(done);
        assert_eq!(samples, 17 * SAMPLES_PER_BYTE as u64);
        assert_eq!(log, [(0..5, 0), (6..11, 600), (12..17, 1200)]);
    }

    #[test]
    fn rate_scales_the_audio_and_unknown_voices_are_refused() {
        let mut e = FakeEngine::default();
        let fast = VoiceSettings {
            voice: "fake/fr".into(),
            rate: 350,
            pitch: 0,
        };
        e.set_voice(&fast).unwrap();
        let mut n = 0;
        e.synthesize("abcd", false, &mut |ev| {
            if let SynthEvent::Audio(a) = ev {
                n += a.len();
                assert_eq!(a[0].abs(), 2000);
            }
            true
        })
        .unwrap();
        assert_eq!(n, 4 * SAMPLES_PER_BYTE / 2);
        let bad = VoiceSettings {
            voice: "zz".into(),
            ..fast
        };
        assert!(e.set_voice(&bad).is_err());
        assert!(e.synthesize("__fail__", false, &mut |_| true).is_err());
    }
}
