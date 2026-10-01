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

/// Plays `samples` `factor` times faster: the pitch rises by `factor` and
/// the length shrinks by it, to `floor(len / factor)` samples, lined up
/// with the input (the filter's delay removed), so word positions scale by
/// `1 / factor`.
///
/// The resampler is band-limited (rubato's windowed sinc, Wave 7): raising
/// the pitch moves the voice's highest sounds above what the sample rate
/// can hold, and they are filtered out instead of folding back as a
/// metallic aliasing tone, as they did with the linear interpolation used
/// before. Should the filter fail, linear interpolation is the fallback.
pub fn resample(samples: &[i16], factor: f32) -> Vec<i16> {
    if samples.is_empty() || factor.is_nan() || factor <= 0.0 || (factor - 1.0).abs() < 1e-6 {
        return samples.to_vec();
    }
    let out_len = ((samples.len() as f64) / f64::from(factor)).floor() as usize;
    match band_limited(samples, 1.0 / f64::from(factor), out_len) {
        Ok(out) => out,
        Err(e) => {
            log::warn!("piper: {e}; using linear interpolation for the pitch");
            linear(samples, factor)
        }
    }
}

/// Resamples by `ratio` (output rate over input rate) with a windowed
/// sinc filter, chunk by chunk, and returns `out_len` samples.
fn band_limited(samples: &[i16], ratio: f64, out_len: usize) -> Result<Vec<i16>, String> {
    use rubato::audioadapter_buffers::direct::InterleavedSlice;
    use rubato::{
        Async, FixedAsync, Indexing, Resampler, SincInterpolationParameters, WindowFunction,
    };

    /// Input samples per step.
    const CHUNK: usize = 1024;
    let err = |e: &dyn std::fmt::Display| format!("cannot resample for the pitch: {e}");
    // 128 taps: well past what speech needs, and cheap beside the model.
    let params = SincInterpolationParameters::new(128, WindowFunction::BlackmanHarris2);
    let mut r = Async::<f32>::new_sinc(ratio, 1.0, &params, CHUNK, 1, FixedAsync::Input)
        .map_err(|e| err(&e))?;
    let input: Vec<f32> = samples.iter().map(|&s| f32::from(s) / 32768.0).collect();
    let delay = r.output_delay();
    let wanted = out_len + delay;
    let mut out: Vec<f32> = Vec::with_capacity(wanted + r.output_frames_max());
    let mut buf = vec![0.0f32; r.output_frames_max()];
    let mut padded = vec![0.0f32; r.input_frames_max()];
    let mut pos = 0usize;
    // Enough steps for the input and the filter's delay, and a bound in
    // case the filter stops producing.
    let mut steps_left = input.len() / CHUNK + delay / CHUNK.max(1) + 16;
    while out.len() < wanted {
        if steps_left == 0 {
            return Err(err(&"the filter stopped producing audio"));
        }
        steps_left -= 1;
        let need = r.input_frames_next();
        let (chunk, partial): (&[f32], Option<usize>) = if pos + need <= input.len() {
            (&input[pos..pos + need], None)
        } else {
            // The end: the rest, then silence to flush the delay.
            let rest = input.len().saturating_sub(pos);
            padded.resize(need, 0.0);
            padded.fill(0.0);
            padded[..rest].copy_from_slice(&input[pos.min(input.len())..]);
            (&padded[..need], Some(rest))
        };
        let inp = InterleavedSlice::new(chunk, 1, need).map_err(|e| err(&e))?;
        let frames = r.output_frames_next();
        buf.resize(frames.max(buf.len()), 0.0);
        let mut outp =
            InterleavedSlice::new_mut(&mut buf[..frames], 1, frames).map_err(|e| err(&e))?;
        let indexing = partial.map(|p| Indexing::new().partial_len(p));
        let (used, produced) = r
            .process_into_buffer(&inp, &mut outp, indexing.as_ref())
            .map_err(|e| err(&e))?;
        pos += used;
        out.extend_from_slice(&buf[..produced]);
    }
    Ok(out[delay..wanted]
        .iter()
        // Clamped, so in range.
        .map(|&v| (v * 32768.0).round().clamp(-32768.0, 32767.0) as i16)
        .collect())
}

/// Plays `samples` `factor` times faster by linear interpolation (the
/// fallback; it lets aliasing through).
fn linear(samples: &[i16], factor: f32) -> Vec<i16> {
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

    /// A sine of `hz` at 22,050 Hz, `n` samples, amplitude 16,000.
    fn sine(hz: f64, n: usize) -> Vec<i16> {
        (0..n)
            .map(|i| {
                let t = i as f64 / 22_050.0;
                (16_000.0 * (2.0 * std::f64::consts::PI * hz * t).sin()).round() as i16
            })
            .collect()
    }

    /// Root mean square of `s`, skipping `edge` samples at each end.
    fn rms(s: &[i16], edge: usize) -> f64 {
        let s = &s[edge..s.len() - edge];
        (s.iter().map(|&v| f64::from(v).powi(2)).sum::<f64>() / s.len() as f64).sqrt()
    }

    /// The frequency of `s` at 22,050 Hz from its rising zero crossings.
    fn frequency(s: &[i16]) -> f64 {
        let rising: Vec<usize> = s
            .windows(2)
            .enumerate()
            .filter(|(_, w)| w[0] < 0 && w[1] >= 0)
            .map(|(i, _)| i)
            .collect();
        let (first, last) = (rising[0], rising[rising.len() - 1]);
        (rising.len() - 1) as f64 * 22_050.0 / (last - first) as f64
    }

    #[test]
    fn resampling_shifts_pitch_and_length() {
        let s: Vec<i16> = (0..100).map(|i| i as i16 * 10).collect();
        assert_eq!(resample(&s, 2.0).len(), 50);
        assert_eq!(resample(&s, 0.5).len(), 200);
        assert_eq!(resample(&s, 1.0), s);
        assert!(resample(&[], 2.0).is_empty());
        assert_eq!(resample(&s, 0.0), s, "a factor of 0 changes nothing");
        // Five semitones up: 1 kHz becomes 1,335 Hz, and the audio is
        // that much shorter.
        let factor = 2f32.powf(5.0 / 12.0);
        let up = resample(&sine(1000.0, 22_050), factor);
        assert_eq!(up.len(), (22_050.0 / f64::from(factor)).floor() as usize);
        let hz = frequency(&up);
        assert!((hz - 1000.0 * f64::from(factor)).abs() < 5.0, "{hz} Hz");
        // Loudness is kept away from the edges.
        let ratio = rms(&up, 500) / rms(&sine(1000.0, 22_050), 500);
        assert!((ratio - 1.0).abs() < 0.02, "{ratio}");
    }

    #[test]
    fn resampling_lines_up_with_the_input() {
        // A click stays where it was, scaled: word positions depend on it.
        let mut s = vec![0i16; 8000];
        s[4000] = 30_000;
        for factor in [0.75f32, 1.5, 2.0] {
            let out = resample(&s, factor);
            let peak = (0..out.len())
                .max_by_key(|&i| out[i].unsigned_abs())
                .unwrap();
            let expected = 4000.0 / f64::from(factor);
            assert!((peak as f64 - expected).abs() <= 1.0, "{factor}: {peak}");
        }
    }

    #[test]
    fn raising_the_pitch_does_not_alias() {
        // A 9 kHz sound raised by half lands at 13.5 kHz, above what
        // 22,050 Hz audio holds (11,025 Hz). Band-limited, it is filtered
        // out; linear interpolation (before) folded it back to 8.55 kHz
        // at nearly full loudness.
        let s = sine(9000.0, 22_050);
        let before = rms(&s, 500);
        let band_limited = rms(&resample(&s, 1.5), 500) / before;
        let linear = rms(&linear(&s, 1.5), 500) / before;
        println!("alias left: band-limited {band_limited:.4}, linear {linear:.4}");
        assert!(band_limited < 0.05, "band-limited kept {band_limited}");
        assert!(linear > 0.3, "linear kept {linear}");
    }

    #[test]
    fn plans_chunks_as_byte_ranges() {
        let t = "One. Two, three.";
        let plan = Synthesizer::plan(t);
        let parts: Vec<&str> = plan.iter().map(|r| &t[r.clone()]).collect();
        assert_eq!(parts, vec!["One", "Two, three"]);
    }
}
