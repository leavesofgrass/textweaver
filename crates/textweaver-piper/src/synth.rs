//! Text to audio with word timing: a loaded voice and a phonemizer.

use std::ops::Range;
use std::path::Path;
use std::time::{Duration, Instant};

use textweaver_core::{Pitch, Rate};

use crate::PiperError;
use crate::config::VoiceConfig;
use crate::model::PiperModel;
use crate::phonemes::{Phonemizer, prepare, word_starts};
use crate::text::{chunks, clauses};

/// The rate, in words per minute, of a Piper voice at length scale 1
/// (`en_US-joe-medium` read plain prose at 211 to 213 wpm on Saturday,
/// September 26, 2026; other voices will differ somewhat).
pub const NATURAL_WPM: f32 = 210.0;

/// Length scales outside this range garble the voices.
const LENGTH_SCALE: (f32, f32) = (0.3, 3.0);

/// How the voice should sound.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SynthParams {
    /// Speaking rate.
    pub rate: Rate,
    /// Pitch offset. Piper has no pitch control: the audio is resampled,
    /// and the phonemes lengthened or shortened to keep the rate.
    pub pitch: Pitch,
    /// Speaker, in a multi-speaker voice.
    pub speaker: u32,
}

impl SynthParams {
    /// The pitch shift as a frequency factor (1.0 for none).
    pub fn pitch_factor(&self) -> f32 {
        2f32.powf(f32::from(self.pitch.clamped().semitones()) / 12.0)
    }

    /// The model's length scale for this rate and pitch, from the voice's
    /// own default length scale.
    pub fn length_scale(&self, config: &VoiceConfig) -> f32 {
        let wpm = f32::from(self.rate.clamped().wpm());
        let base = config.inference.length_scale * NATURAL_WPM / wpm;
        base.clamp(LENGTH_SCALE.0, LENGTH_SCALE.1) * self.pitch_factor()
    }

    /// The rate actually achieved, after the length scale's limits.
    pub fn effective_wpm(&self, config: &VoiceConfig) -> u16 {
        let scale = self.length_scale(config) / self.pitch_factor();
        let wpm = config.inference.length_scale * NATURAL_WPM / scale.max(0.01);
        // In range: the scale is clamped to at least 0.3.
        wpm.round().clamp(1.0, f32::from(u16::MAX)) as u16
    }
}

/// Where a chunk's word timing came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Timing {
    /// The model's phoneme durations (`w_ceil`).
    Model,
    /// Spread over the audio by phoneme count (the graph gave none).
    Estimated,
}

/// One synthesized chunk.
#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    /// 16-bit mono audio at the voice's sample rate.
    pub samples: Vec<i16>,
    /// Each word's byte range in the text and its first sample.
    pub words: Vec<(Range<u32>, u64)>,
    /// Where the word timing came from.
    pub timing: Timing,
    /// False when some phoneme words were matched to text words by length
    /// (eSpeak joined or split words).
    pub exact: bool,
    /// Time spent phonemizing and running the model.
    pub elapsed: Duration,
}

/// A voice ready to speak: its model, its settings, and a phonemizer.
pub struct Synthesizer {
    model: PiperModel,
    config: VoiceConfig,
    phonemizer: Box<dyn Phonemizer>,
}

impl std::fmt::Debug for Synthesizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Synthesizer")
            .field("model", &self.model)
            .field("phonemizer", &self.phonemizer.name())
            .finish_non_exhaustive()
    }
}

impl Synthesizer {
    /// Loads the voice at `onnx` with its settings at `json`.
    pub fn load(
        onnx: &Path,
        json: &Path,
        phonemizer: Box<dyn Phonemizer>,
    ) -> Result<Self, PiperError> {
        let config = VoiceConfig::from_file(json)?;
        if !config.uses_espeak() && config.phoneme_type != "text" {
            return Err(PiperError::Unsupported(format!(
                "this voice uses the {:?} phonemizer, which needs the piper program",
                config.phoneme_type
            )));
        }
        let model = PiperModel::load(onnx)?;
        Ok(Synthesizer {
            model,
            config,
            phonemizer,
        })
    }

    /// The voice's settings.
    pub fn config(&self) -> &VoiceConfig {
        &self.config
    }

    /// The output sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.config.audio.sample_rate
    }

    /// True when the model reports phoneme durations.
    pub fn has_word_timing(&self) -> bool {
        self.model.has_word_timing()
    }

    /// The phonemizer's name.
    pub fn phonemizer_name(&self) -> &'static str {
        self.phonemizer.name()
    }

    /// The chunks `text` is synthesized in, as byte ranges: a sentence
    /// each, with the first clause of a long first sentence on its own.
    pub fn plan(text: &str) -> Vec<Range<usize>> {
        let cs = clauses(text);
        chunks(text, &cs)
            .into_iter()
            .filter_map(|r| {
                let first = cs.get(r.start)?;
                let last = cs.get(r.end.checked_sub(1)?)?;
                Some(first.range.start..last.range.end)
            })
            .collect()
    }

    /// Synthesizes every chunk of `text`, calling `each` with each one as
    /// it is ready (return false to stop). Word ranges are byte offsets
    /// into `text`.
    pub fn speak(
        &mut self,
        text: &str,
        params: &SynthParams,
        mut each: impl FnMut(Chunk) -> bool,
    ) -> Result<(), PiperError> {
        let cs = clauses(text);
        for r in chunks(text, &cs) {
            let chunk = self.synthesize_clauses(text, &cs[r], params)?;
            if !each(chunk) {
                break;
            }
        }
        Ok(())
    }

    /// Synthesizes the whole of `text` as one audio buffer.
    pub fn speak_all(&mut self, text: &str, params: &SynthParams) -> Result<Chunk, PiperError> {
        let mut all = Chunk {
            samples: Vec::new(),
            words: Vec::new(),
            timing: Timing::Model,
            exact: true,
            elapsed: Duration::ZERO,
        };
        self.speak(text, params, |c| {
            let offset = all.samples.len() as u64;
            all.words
                .extend(c.words.into_iter().map(|(r, s)| (r, s + offset)));
            all.samples.extend_from_slice(&c.samples);
            if c.timing == Timing::Estimated {
                all.timing = Timing::Estimated;
            }
            all.exact &= c.exact;
            all.elapsed += c.elapsed;
            true
        })?;
        Ok(all)
    }

    fn synthesize_clauses(
        &mut self,
        text: &str,
        cs: &[crate::text::Clause],
        params: &SynthParams,
    ) -> Result<Chunk, PiperError> {
        let started = Instant::now();
        let prepared = prepare(&self.config, self.phonemizer.as_mut(), text, cs)?;
        let length_scale = params.length_scale(&self.config);
        let out = self
            .model
            .infer(&self.config, &prepared.ids, length_scale, params.speaker)?;
        let pitch = params.pitch_factor();
        let mut samples = to_pcm(&out.audio);
        if (pitch - 1.0).abs() > 1e-3 {
            samples = resample(&samples, pitch);
        }
        let n = samples.len() as f64;
        let (frames, timing) = match out.durations {
            Some(d) if d.len() == prepared.ids.len() => (d, Timing::Model),
            _ => (vec![1.0; prepared.ids.len()], Timing::Estimated),
        };
        let total: f64 = frames.iter().map(|&f| f64::from(f.max(0.0))).sum();
        let per_frame = if total > 0.0 { n / total } else { 0.0 };
        let words = word_starts(&prepared.owners, &frames, per_frame)
            .into_iter()
            .filter_map(|(w, s)| Some((prepared.words.get(w as usize)?.clone(), s)))
            .collect();
        Ok(Chunk {
            samples,
            words,
            timing,
            exact: prepared.exact,
            elapsed: started.elapsed(),
        })
    }
}

/// Floats to 16-bit samples, normalized to the loudest sample as Piper
/// does (a quiet sentence is not left quiet).
pub fn to_pcm(audio: &[f32]) -> Vec<i16> {
    let peak = audio.iter().fold(0f32, |m, &s| m.max(s.abs())).max(0.01);
    let scale = 32767.0 / peak;
    audio
        .iter()
        // In range after the clamp.
        .map(|&s| (s * scale).round().clamp(-32768.0, 32767.0) as i16)
        .collect()
}

/// Plays `samples` `factor` times faster by linear interpolation: the
/// pitch rises by `factor` and the length shrinks by it.
pub fn resample(samples: &[i16], factor: f32) -> Vec<i16> {
    if samples.is_empty() || factor <= 0.0 {
        return samples.to_vec();
    }
    let factor = f64::from(factor);
    let out_len = ((samples.len() as f64) / factor).floor() as usize;
    (0..out_len)
        .map(|i| {
            let pos = i as f64 * factor;
            let j = pos.floor() as usize;
            let frac = pos - j as f64;
            let a = f64::from(samples[j.min(samples.len() - 1)]);
            let b = f64::from(samples[(j + 1).min(samples.len() - 1)]);
            // Between two i16 values, so in range.
            (a + (b - a) * frac).round() as i16
        })
        .collect()
}

/// A measurement of one voice on one text (`cargo xtask piper-bench`).
#[derive(Clone, Debug, PartialEq)]
pub struct Measurement {
    /// Loading the model.
    pub load: Duration,
    /// From the start of speaking to the first chunk's audio.
    pub first_audio: Duration,
    /// Synthesizing everything.
    pub total: Duration,
    /// Length of the audio.
    pub audio: Duration,
    /// Words per minute of the audio, with the text's word count.
    pub wpm: f64,
    /// Where the word timing came from.
    pub timing: Timing,
    /// The phonemizer used.
    pub phonemizer: &'static str,
}

impl Measurement {
    /// The real-time factor: synthesis time over audio time (below 1 is
    /// faster than real time).
    pub fn real_time_factor(&self) -> f64 {
        self.total.as_secs_f64() / self.audio.as_secs_f64().max(1e-9)
    }
}

/// Loads a voice and speaks `text` once, timing each step.
pub fn measure(
    onnx: &Path,
    json: &Path,
    phonemizer: Box<dyn Phonemizer>,
    text: &str,
    params: &SynthParams,
) -> Result<Measurement, PiperError> {
    let t = Instant::now();
    let mut synth = Synthesizer::load(onnx, json, phonemizer)?;
    let load = t.elapsed();
    let t = Instant::now();
    let mut first = None;
    let mut samples = 0usize;
    let mut timing = Timing::Model;
    synth.speak(text, params, |c| {
        first.get_or_insert_with(|| t.elapsed());
        samples += c.samples.len();
        if c.timing == Timing::Estimated {
            timing = Timing::Estimated;
        }
        true
    })?;
    let total = t.elapsed();
    let audio = Duration::from_secs_f64(samples as f64 / f64::from(synth.sample_rate()));
    let words = crate::text::words(text).len() as f64;
    Ok(Measurement {
        load,
        first_audio: first.unwrap_or(total),
        total,
        audio,
        wpm: words / audio.as_secs_f64().max(1e-9) * 60.0,
        timing,
        phonemizer: synth.phonemizer_name(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::tests::JSON;

    #[test]
    fn rate_maps_onto_the_length_scale() {
        let c = VoiceConfig::from_json(JSON).unwrap();
        let p = |wpm, st| SynthParams {
            rate: Rate::Wpm(wpm),
            pitch: Pitch::Semitones(st),
            speaker: 0,
        };
        assert!((p(210, 0).length_scale(&c) - 1.0).abs() < 1e-6);
        assert!((p(420, 0).length_scale(&c) - 0.5).abs() < 1e-6);
        // Clamped at both ends, and the effective rate says so.
        assert!((p(900, 0).length_scale(&c) - 0.3).abs() < 1e-6);
        assert_eq!(p(900, 0).effective_wpm(&c), 700);
        assert_eq!(p(50, 0).effective_wpm(&c), 70);
        assert_eq!(p(265, 0).effective_wpm(&c), 265);
        // A pitch shift lengthens the phonemes by the same factor.
        let up = p(210, 12);
        assert!((up.pitch_factor() - 2.0).abs() < 1e-6);
        assert!((up.length_scale(&c) - 2.0).abs() < 1e-6);
        assert_eq!(up.effective_wpm(&c), 210);
    }

    #[test]
    fn pcm_is_normalized_to_the_peak() {
        assert_eq!(to_pcm(&[0.0, 0.5, -0.25]), vec![0, 32767, -16384]);
        // Near silence is not amplified to full scale.
        assert_eq!(to_pcm(&[0.001]), vec![3277]);
        assert!(to_pcm(&[]).is_empty());
    }

    #[test]
    fn resampling_shifts_pitch_and_length() {
        let s: Vec<i16> = (0..100).map(|i| i as i16 * 10).collect();
        let up = resample(&s, 2.0);
        assert_eq!(up.len(), 50);
        assert_eq!(up[1], 20);
        let down = resample(&s, 0.5);
        assert_eq!(down.len(), 200);
        assert_eq!(down[1], 5);
        assert_eq!(resample(&s, 1.0), s);
        assert!(resample(&[], 2.0).is_empty());
    }

    #[test]
    fn plans_chunks_as_byte_ranges() {
        let t = "One. Two, three.";
        let plan = Synthesizer::plan(t);
        let parts: Vec<&str> = plan.iter().map(|r| &t[r.clone()]).collect();
        assert_eq!(parts, vec!["One", "Two, three"]);
    }
}
