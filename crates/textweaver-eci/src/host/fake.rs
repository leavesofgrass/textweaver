//! A deterministic stand-in for ECI, selected with
//! `textweaver-eci-host --engine fake`. Tests use it to exercise the real
//! host binary, the protocol, and the backend without the proprietary
//! engine (for example in the Linux container).
//!
//! Behavior: 8000 Hz; every byte of text produces `samples_per_byte`
//! samples of a quiet square wave (so word lengths differ); index marks are
//! reported where they fall; the speed voice parameter scales the duration
//! (50 is 1x, 100 is twice as fast). Special words, for testing failure
//! paths: `__crash__` exits the process at once; `__fail__` makes synthesis
//! return an error; `__hang__` never returns (a hung engine); `__slow__`
//! sleeps 50 ms per audio block.

use std::time::Duration;

use super::{Engine, EngineInfo, EnginePiece, SynthEvent};
use crate::language::DEFAULT_DIALECT;
use crate::protocol::PresetInfo;

/// Fake engine settings.
#[derive(Clone, Debug)]
pub struct FakeConfig {
    /// Samples per byte of text at speed 50.
    pub samples_per_byte: usize,
    /// Samples per audio callback.
    pub block: usize,
    /// Dictionary directory; files found there are reported as loaded.
    pub dictionaries: Option<std::path::PathBuf>,
    /// Dictionary volumes left out (ECI numbers: 0 main, 1 root, 2
    /// abbreviations).
    pub skip_volumes: Vec<u8>,
    /// How long loading each dictionary file takes (tests: an engine that
    /// loads a large dictionary slowly, as OpenEVV 0.3.0 does).
    pub dictionary_delay: std::time::Duration,
}

impl Default for FakeConfig {
    fn default() -> Self {
        FakeConfig {
            samples_per_byte: 400,
            block: 1024,
            dictionaries: None,
            skip_volumes: Vec::new(),
            dictionary_delay: std::time::Duration::ZERO,
        }
    }
}

/// The fake engine.
#[derive(Debug)]
pub struct FakeEngine {
    config: FakeConfig,
    dialect: u32,
    params: [i32; 8],
    loaded: Vec<u32>,
}

const PRESET_NAMES: [&str; 8] = [
    "Fake Reed",
    "Fake Shelley",
    "Fake Bobby",
    "Fake Rocko",
    "Fake Glen",
    "Fake Sandy",
    "Fake Grandma",
    "Fake Grandpa",
];

fn preset_params(i: usize) -> [i32; 8] {
    let gender = i32::from(matches!(i, 1 | 2 | 5 | 6));
    let pitch = 60 + i32::try_from(i).unwrap_or(0) * 3;
    [gender, 50, pitch, 30, 0, 0, 50, 90]
}

impl FakeEngine {
    /// A fake engine.
    pub fn new(config: FakeConfig) -> Self {
        FakeEngine {
            config,
            dialect: DEFAULT_DIALECT,
            params: preset_params(0),
            loaded: Vec::new(),
        }
    }
}

impl Engine for FakeEngine {
    fn info(&mut self) -> EngineInfo {
        EngineInfo {
            sample_rate: 8000,
            version: "fake 1.0".into(),
            dialects: vec![0x0001_0000, 0x0001_0001, 0x0004_0000],
            default_dialect: self.dialect,
            presets: (0..8)
                .map(|i| PresetInfo {
                    name: PRESET_NAMES[i].into(),
                    params: preset_params(i),
                })
                .collect(),
        }
    }

    fn dialect(&self) -> u32 {
        self.dialect
    }

    fn set_voice(&mut self, dialect: u32, preset: u8) -> Result<(), String> {
        if !matches!(dialect, 0x0001_0000 | 0x0001_0001 | 0x0004_0000) {
            return Err(format!("dialect {dialect:#x} is not installed"));
        }
        self.dialect = dialect;
        if (1..=8).contains(&preset) {
            self.params = preset_params(usize::from(preset - 1));
        }
        Ok(())
    }

    fn set_voice_param(&mut self, param: u8, value: i32) -> Result<(), String> {
        let slot = self
            .params
            .get_mut(usize::from(param))
            .ok_or_else(|| format!("no voice parameter {param}"))?;
        *slot = value;
        Ok(())
    }

    fn activate_dictionaries(&mut self) -> Vec<super::DictLoad> {
        let Some(dir) = &self.config.dictionaries else {
            return Vec::new();
        };
        if self.loaded.contains(&self.dialect) {
            return Vec::new();
        }
        self.loaded.push(self.dialect);
        let delay = self.config.dictionary_delay;
        crate::dictionaries::files_for(dir, self.dialect)
            .into_iter()
            .filter(|(v, _)| !self.config.skip_volumes.contains(&(*v as u8)))
            .map(|(v, p)| {
                std::thread::sleep(delay);
                super::DictLoad {
                    dialect: self.dialect,
                    volume: v as u8,
                    status: 0,
                    path: p.display().to_string(),
                }
            })
            .collect()
    }

    fn synthesize(
        &mut self,
        pieces: &[EnginePiece],
        out: &mut dyn FnMut(SynthEvent<'_>) -> bool,
    ) -> Result<bool, String> {
        let speed = usize::try_from(self.params[6].clamp(10, 250)).unwrap_or(50);
        let per_byte = (self.config.samples_per_byte * 50 / speed).max(1);
        let slow = pieces
            .iter()
            .any(|p| matches!(p, EnginePiece::Text(t) if t.windows(8).any(|w| w == b"__slow__")));
        let mut block: Vec<i16> = Vec::with_capacity(self.config.block);
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
                    if t.windows(9).any(|w| w == b"__crash__") {
                        std::process::exit(3);
                    }
                    if t.windows(8).any(|w| w == b"__hang__") {
                        loop {
                            std::thread::sleep(Duration::from_secs(60));
                        }
                    }
                    if t.windows(8).any(|w| w == b"__fail__") {
                        return Err("fake engine failure".into());
                    }
                    for _ in 0..t.len() * per_byte {
                        phase += 1;
                        block.push(if (phase / 20).is_multiple_of(2) {
                            800
                        } else {
                            -800
                        });
                        if block.len() == self.config.block && !flush(&mut block, out) {
                            return Ok(false);
                        }
                    }
                }
            }
        }
        Ok(flush(&mut block, out))
    }
}
