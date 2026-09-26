//! A deterministic stand-in for DECtalk, selected with
//! `textweaver-dectalk-host --engine fake`. The backend's tests use it to
//! exercise the real host binary, the protocol, and the playback client
//! without DECtalk (in any environment, including the Linux container).
//!
//! Behavior: 11,025 Hz, as DECtalk; every byte of text produces
//! [`SAMPLES_PER_BYTE`] samples at 180 words per minute, scaled by the rate
//! (360 wpm gives half as many); the samples are a square wave whose
//! amplitude is `500 + 100 × speaker` (so tests can tell speakers apart)
//! and whose period follows the pitch; marks are reported where they fall.
//! Special words, for testing failure paths: `__crash__` exits the process
//! at once; `__fail__` makes synthesis return an error; `__hang__` never
//! returns (a hung engine); `__slow__` sleeps 50 ms per audio block.

use std::time::Duration;

use super::{Engine, EngineInfo, EnginePiece, Settings, SynthEvent};
use crate::voices;

/// Samples per byte of text at 180 words per minute.
pub const SAMPLES_PER_BYTE: usize = 200;
/// Samples per audio block.
pub const BLOCK: usize = 1024;
/// The fake engine's sample rate (DECtalk's).
pub const SAMPLE_RATE: u32 = 11_025;

/// The fake engine.
#[derive(Debug, Default)]
pub struct FakeEngine;

fn contains(t: &[u8], word: &[u8]) -> bool {
    t.windows(word.len()).any(|w| w == word)
}

/// The fake's amplitude for a speaker number.
pub fn amplitude(speaker: u8) -> i16 {
    500 + 100 * i16::from(speaker)
}

impl Engine for FakeEngine {
    fn info(&mut self) -> EngineInfo {
        EngineInfo {
            sample_rate: SAMPLE_RATE,
            version: "fake 1.0".into(),
            engine: "fake".into(),
        }
    }

    fn synthesize(
        &mut self,
        settings: &Settings,
        pieces: &[EnginePiece],
        out: &mut dyn FnMut(SynthEvent<'_>) -> bool,
    ) -> Result<bool, String> {
        let rate = usize::from(voices::rate(settings.rate));
        let per_byte = (SAMPLES_PER_BYTE * usize::from(voices::DEFAULT_RATE) / rate).max(1);
        let pitch = if settings.pitch_hz == 0 {
            settings.speaker.average_pitch()
        } else {
            settings.pitch_hz
        };
        let half_period = (SAMPLE_RATE as usize / usize::from(pitch.max(1)) / 2).max(1);
        let amp = amplitude(settings.speaker.index());
        let slow = pieces
            .iter()
            .any(|p| matches!(p, EnginePiece::Text(t) if contains(t, b"__slow__")));
        let mut block: Vec<i16> = Vec::with_capacity(BLOCK);
        let mut phase = 0usize;
        let flush = |block: &mut Vec<i16>, out: &mut dyn FnMut(SynthEvent<'_>) -> bool| {
            if block.is_empty() {
                return true;
            }
            if slow {
                std::thread::sleep(Duration::from_millis(50));
            }
            let go = out(SynthEvent::Audio(block));
            block.clear();
            go
        };
        for p in pieces {
            match p {
                EnginePiece::Index(i) => {
                    if !flush(&mut block, out) || !out(SynthEvent::Mark(*i)) {
                        return Ok(false);
                    }
                }
                EnginePiece::Text(t) => {
                    if contains(t, b"__crash__") {
                        std::process::exit(3);
                    }
                    if contains(t, b"__hang__") {
                        loop {
                            std::thread::sleep(Duration::from_secs(60));
                        }
                    }
                    if contains(t, b"__fail__") {
                        return Err("fake engine failure".into());
                    }
                    for _ in 0..t.len() * per_byte {
                        phase += 1;
                        block.push(if (phase / half_period).is_multiple_of(2) {
                            amp
                        } else {
                            -amp
                        });
                        if block.len() == BLOCK && !flush(&mut block, out) {
                            return Ok(false);
                        }
                    }
                }
            }
        }
        Ok(flush(&mut block, out))
    }
}
