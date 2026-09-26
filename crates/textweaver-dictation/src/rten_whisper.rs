//! Whisper in-process on RTen, the pure-Rust ONNX runtime (feature
//! `rten`, ADR-0023).
//!
//! Based on `rten-examples/src/whisper.rs`. The models are the int8 ONNX
//! exports from [onnx-community](https://huggingface.co/onnx-community)
//! (`whisper-base.en`: `onnx/encoder_model_int8.onnx`, 23 MB, and
//! `onnx/decoder_model_merged_int8.onnx`, 54 MB, with `tokenizer.json`;
//! MIT licence, from OpenAI). Audio is cut into 30-second windows, turned
//! into a log-mel spectrogram here (the mel filters are computed, not
//! shipped), encoded once per window, and decoded greedily with Whisper's
//! timestamp rules, so each finished segment is known with its times.
//!
//! [`RtenWhisper`] does the transcribing; [`RtenDictation`](crate::RtenDictation)
//! runs it on a worker thread behind the [`Dictation`](crate::Dictation)
//! trait.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rten::{Dimension, Model};
use rten_generate::filter::LogitsFilter;
use rten_generate::{Generator, GeneratorConfig, GeneratorUtils, Logits};
use rten_tensor::NdTensor;
use rten_tensor::prelude::*;
use rten_text::Tokenizer;
use rustfft::FftPlanner;
use rustfft::num_complex::Complex32;

use crate::DictationError;
use crate::capture::WHISPER_SAMPLE_RATE;
use crate::transcript::{Segment, Transcript};

/// Samples per FFT window.
pub const N_FFT: usize = 400;
/// Samples between windows (10 ms).
pub const HOP: usize = 160;
/// Seconds of audio per window the model sees.
pub const CHUNK_SECONDS: usize = 30;
/// Most tokens decoded per window (from Hugging Face Transformers).
const MAX_TOKENS: usize = 448;
/// Milliseconds per timestamp token step.
const TIMESTAMP_MS: u32 = 20;

/// The file names tried for each part, most compact first: onnx-community's
/// int8 export, its `_quantized` alias, then full precision.
pub const ENCODER_NAMES: [&str; 3] = [
    "encoder_model_int8.onnx",
    "encoder_model_quantized.onnx",
    "encoder_model.onnx",
];
/// See [`ENCODER_NAMES`].
pub const DECODER_NAMES: [&str; 4] = [
    "decoder_model_merged_int8.onnx",
    "decoder_model_merged_quantized.onnx",
    "decoder_model_merged.onnx",
    "decoder_model_int8.onnx",
];

/// A model's files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RtenWhisperFiles {
    /// The audio encoder.
    pub encoder: PathBuf,
    /// The merged text decoder (with and without the key-value cache).
    pub decoder: PathBuf,
    /// `tokenizer.json`.
    pub tokenizer: PathBuf,
    /// True for an English-only model (`base.en`), which takes no language
    /// or task tokens.
    pub english_only: bool,
}

impl RtenWhisperFiles {
    /// The files in `dir` (as downloaded: the folder may hold them
    /// directly or in `onnx/`). A folder named `*.en` or
    /// `whisper-*.en` is an English-only model.
    pub fn in_dir(dir: &Path) -> Result<Self, DictationError> {
        let find = |names: &[&str]| {
            names.iter().find_map(|n| {
                [dir.join(n), dir.join("onnx").join(n)]
                    .into_iter()
                    .find(|p| p.is_file())
            })
        };
        let missing = |what: &str| DictationError::ModelNotFound {
            model: dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            file: what.to_owned(),
            searched: vec![dir.to_owned()],
        };
        let encoder = find(&ENCODER_NAMES).ok_or_else(|| missing(ENCODER_NAMES[0]))?;
        let decoder = find(&DECODER_NAMES).ok_or_else(|| missing(DECODER_NAMES[0]))?;
        let tokenizer = find(&["tokenizer.json"]).ok_or_else(|| missing("tokenizer.json"))?;
        let english_only = dir
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(".en"));
        Ok(RtenWhisperFiles {
            encoder,
            decoder,
            tokenizer,
            english_only,
        })
    }
}

/// Where one transcription spent its time.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Timings {
    /// Spectrograms.
    pub features: Duration,
    /// The encoder.
    pub encode: Duration,
    /// The decoder.
    pub decode: Duration,
    /// Audio transcribed.
    pub audio: Duration,
}

impl Timings {
    /// Total processing time.
    pub fn total(&self) -> Duration {
        self.features + self.encode + self.decode
    }
}

/// A loaded Whisper model.
pub struct RtenWhisper {
    encoder: Model,
    decoder: Model,
    tokenizer: Tokenizer,
    english_only: bool,
    n_mels: usize,
    filters: NdTensor<f32, 2>,
    window: Vec<f32>,
}

impl std::fmt::Debug for RtenWhisper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtenWhisper")
            .field("english_only", &self.english_only)
            .field("n_mels", &self.n_mels)
            .finish_non_exhaustive()
    }
}

fn model_error(path: &Path, e: impl std::fmt::Display) -> DictationError {
    DictationError::Capture(format!(
        "The Whisper model {} could not be used: {e}",
        path.display()
    ))
}

impl RtenWhisper {
    /// Loads a model. Files are read into memory (RTen's memory-mapped
    /// loading is `unsafe`, and not used).
    pub fn load(files: &RtenWhisperFiles) -> Result<Self, DictationError> {
        let load = |p: &Path| {
            let mut bytes = std::fs::read(p).map_err(|source| DictationError::Io {
                path: p.to_owned(),
                source,
            })?;
            // 8-bit weights saturate RTen's int8 kernels on CPUs without
            // VNNI; see `onnx_patch`.
            match crate::onnx_patch::reduce_weight_range(&mut bytes) {
                Some(n) => log::debug!("{}: weights reduced to 7 bits: {n:?}", p.display()),
                None => log::warn!("{}: could not check the weights' range", p.display()),
            }
            Model::load(bytes).map_err(|e| model_error(p, e))
        };
        let encoder = load(&files.encoder)?;
        let decoder = load(&files.decoder)?;
        let json =
            std::fs::read_to_string(&files.tokenizer).map_err(|source| DictationError::Io {
                path: files.tokenizer.clone(),
                source,
            })?;
        let tokenizer = Tokenizer::from_json(&complete_tokenizer_json(&json))
            .map_err(|e| model_error(&files.tokenizer, e))?;
        let n_mels = match encoder.input_shape(0).as_deref() {
            Some([_, Dimension::Fixed(mels), _]) => *mels,
            _ => 80,
        };
        Ok(RtenWhisper {
            encoder,
            decoder,
            tokenizer,
            english_only: files.english_only,
            n_mels,
            filters: mel_filters(n_mels, N_FFT, WHISPER_SAMPLE_RATE),
            window: hann_window(N_FFT),
        })
    }

    fn token(&self, text: &str) -> Result<u32, DictationError> {
        self.tokenizer.get_token_id(text).map_err(|e| {
            DictationError::Capture(format!("The Whisper tokenizer lacks {text}: {e}"))
        })
    }

    /// Transcribes 16 kHz mono `audio` (samples in `-1.0..=1.0`).
    /// `language` (`en`, `de`) is used by multilingual models; `None`
    /// detects it. `on_segment` hears each segment as it is finished.
    /// `cancel` stops between steps; a cancelled run returns what it has.
    pub fn transcribe(
        &self,
        audio: &[f32],
        language: Option<&str>,
        cancel: &AtomicBool,
        on_segment: &mut dyn FnMut(&Segment),
    ) -> Result<(Transcript, Timings), DictationError> {
        let mut timings = Timings {
            audio: Duration::from_secs_f64(audio.len() as f64 / f64::from(WHISPER_SAMPLE_RATE)),
            ..Timings::default()
        };
        let err =
            |e: &dyn std::fmt::Display| DictationError::Capture(format!("Whisper failed: {e}"));
        let samples_per_chunk = WHISPER_SAMPLE_RATE as usize * CHUNK_SECONDS;
        let sot = self.token("<|startoftranscript|>")?;
        let eot = self.token("<|endoftext|>")?;
        let start_of_prev = self.token("<|startofprev|>")?;
        let transcribe = self.token("<|transcribe|>")?;
        let ts_min = self.token("<|0.00|>")?;
        let ts_max = self.token("<|30.00|>")?;
        let no_timestamps = self.token("<|notimestamps|>")?;
        let mut lang_token: Option<u32> = match language {
            Some(l) if !self.english_only => Some(self.token(&format!("<|{l}|>"))?),
            _ => None,
        };
        let encoder_in = self
            .encoder
            .node_id("input_features")
            .map_err(|e| err(&e))?;
        let encoder_out = self
            .encoder
            .node_id("last_hidden_state")
            .map_err(|e| err(&e))?;
        let hidden = self
            .decoder
            .node_id("encoder_hidden_states")
            .map_err(|e| err(&e))?;

        let mut transcript = Transcript::default();
        let mut prev_tokens: Vec<u32> = Vec::new();
        let mut offset_ms: u64 = 0;
        loop {
            if cancel.load(Ordering::SeqCst) {
                break;
            }
            let start = usize::try_from(offset_ms * u64::from(WHISPER_SAMPLE_RATE) / 1000)
                .unwrap_or(usize::MAX)
                .min(audio.len());
            let chunk = &audio[start..(start + samples_per_chunk).min(audio.len())];
            // Less than a tenth of a second left is not worth a window.
            if chunk.len() < WHISPER_SAMPLE_RATE as usize / 10 {
                break;
            }
            let t = Instant::now();
            let mel = log_mel_spectrogram(chunk, samples_per_chunk, &self.filters, &self.window)
                .with_new_axis(0);
            timings.features += t.elapsed();

            let t = Instant::now();
            let [encoded] = self
                .encoder
                .run_n(vec![(encoder_in, mel.view().into())], [encoder_out], None)
                .map_err(|e| err(&e))?;
            let encoded: NdTensor<f32, 3> = encoded.try_into().map_err(|e| err(&e))?;
            timings.encode += t.elapsed();

            let t = Instant::now();
            let mut prompt = Vec::new();
            if !prev_tokens.is_empty() {
                prompt.push(start_of_prev);
                // Whisper keeps at most half its context for the previous
                // text.
                let keep = prev_tokens.len().saturating_sub(MAX_TOKENS / 2 - 1);
                prompt.extend_from_slice(&prev_tokens[keep..]);
            }
            prompt.push(sot);
            if !self.english_only {
                let lang = match lang_token {
                    Some(l) => l,
                    None => {
                        let l = self.detect_language(sot, hidden, &encoded)?;
                        lang_token = Some(l);
                        l
                    }
                };
                prompt.extend([lang, transcribe]);
            }
            prev_tokens.clear();
            let generator = self
                .generator(Some(MAX_TOKENS))
                .map_err(|e| err(&e))?
                .with_prompt(&prompt)
                .with_constant_input(hidden, encoded.view().into())
                .with_logits_filter(TimestampFilter {
                    ts_min,
                    ts_max,
                    no_timestamps,
                    prompt_len: prompt.len(),
                })
                .take(MAX_TOKENS.saturating_sub(prompt.len()))
                .stop_on_tokens([eot]);
            let mut seg_start: Option<u32> = None;
            let mut seg_tokens: Vec<u32> = Vec::new();
            let mut last_end: Option<u32> = None;
            for token in generator {
                if cancel.load(Ordering::SeqCst) {
                    break;
                }
                let token = token.map_err(|e| err(&e))?;
                prev_tokens.push(token);
                if (ts_min..=ts_max).contains(&token) {
                    let ms = (token - ts_min) * TIMESTAMP_MS;
                    match seg_start.take() {
                        Some(s) => {
                            let text = self
                                .tokenizer
                                .decode(&seg_tokens)
                                .map_err(|e| err(&e))?
                                .trim()
                                .to_owned();
                            seg_tokens.clear();
                            last_end = Some(ms);
                            if !text.is_empty() {
                                let seg = Segment {
                                    start_ms: offset_ms + u64::from(s),
                                    end_ms: offset_ms + u64::from(ms),
                                    text,
                                };
                                on_segment(&seg);
                                transcript.segments.push(seg);
                            }
                        }
                        None => seg_start = Some(ms),
                    }
                } else if token < eot {
                    seg_tokens.push(token);
                }
            }
            // Text after the last timestamp (the window ran out mid-segment).
            if !seg_tokens.is_empty() {
                let text = self
                    .tokenizer
                    .decode(&seg_tokens)
                    .map_err(|e| err(&e))?
                    .trim()
                    .to_owned();
                if !text.is_empty() {
                    let seg = Segment {
                        start_ms: offset_ms + u64::from(seg_start.unwrap_or(0)),
                        end_ms: offset_ms
                            + chunk.len() as u64 * 1000 / u64::from(WHISPER_SAMPLE_RATE),
                        text,
                    };
                    on_segment(&seg);
                    transcript.segments.push(seg);
                }
            }
            timings.decode += t.elapsed();
            if chunk.len() < samples_per_chunk {
                break;
            }
            // Go on from the last finished segment, or the next window.
            offset_ms += u64::from(match last_end {
                Some(ms) if ms > 0 => ms,
                _ => (CHUNK_SECONDS as u32) * 1000,
            });
        }
        Ok((transcript, timings))
    }

    /// A decoder generator. rten-generate feeds a merged decoder's
    /// `use_cache_branch` as a scalar; onnx-community's exports declare it
    /// with one dimension, so for those it is fed here instead.
    fn generator(
        &self,
        kv_cache_capacity: Option<usize>,
    ) -> Result<Generator<'_>, rten_generate::GeneratorError> {
        const FLAG: &str = "use_cache_branch";
        let flag = self.decoder.find_node(FLAG).filter(|&id| {
            self.decoder
                .node_info(id)
                .and_then(|info| info.shape())
                .is_some_and(|s| s.len() == 1)
        });
        let mut config = GeneratorConfig {
            kv_cache_capacity,
            ..Default::default()
        };
        if flag.is_some() {
            // No such input: rten-generate leaves the flag to us.
            config.model_inputs.use_cache_flag = "textweaver: fed by the caller";
        }
        let g = Generator::from_model_config(&self.decoder, config)?;
        Ok(match flag {
            Some(id) => g.with_varying_input(id, &use_cache_flag),
            None => g,
        })
    }

    /// Runs one decoder step to pick the most likely language token.
    fn detect_language(
        &self,
        sot: u32,
        hidden: rten::NodeId,
        encoded: &NdTensor<f32, 3>,
    ) -> Result<u32, DictationError> {
        let lo = self.token("<|en|>")?;
        let hi = self.token("<|su|>")?;
        let mut g = self
            .generator(None)
            .map_err(|e| DictationError::Capture(format!("Whisper failed: {e}")))?
            .with_prompt(&[sot])
            .with_constant_input(hidden, encoded.view().into())
            .with_logits_filter(rten_generate::filter::token_id_filter(move |t| {
                (lo..=hi).contains(&t)
            }));
        match g.next() {
            Some(Ok(t)) => Ok(t),
            Some(Err(e)) => Err(DictationError::Capture(format!("Whisper failed: {e}"))),
            None => Ok(lo),
        }
    }
}

/// `use_cache_branch` with one dimension: false on the first run (the
/// prompt), true once the key-value cache holds the earlier tokens.
fn use_cache_flag<'a>(_batch: usize, positions: std::ops::Range<usize>) -> rten::ValueOrView<'a> {
    NdTensor::from([i32::from(positions.start != 0)]).into()
}

/// Whisper's timestamp rules (`ApplyTimestampRules`, simplified as in
/// `rten-examples`): pick a timestamp when timestamps are likelier than
/// any single text token, never `<|notimestamps|>`, and never a timestamp
/// earlier than the last one.
struct TimestampFilter {
    ts_min: u32,
    ts_max: u32,
    no_timestamps: u32,
    prompt_len: usize,
}

impl LogitsFilter for TimestampFilter {
    fn filter(&self, logits: Logits, prev_tokens: &[u32]) -> Logits {
        let (mut logits, indices) = logits.into_logits_indices();
        let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let exps: Vec<f32> = logits.iter().map(|&l| (l - max).exp()).collect();
        let sum: f32 = exps.iter().sum::<f32>().max(f32::MIN_POSITIVE);
        let is_ts = |t: u32| (self.ts_min..=self.ts_max).contains(&t);
        let mut ts_prob = 0.0f32;
        let mut max_text = 0.0f32;
        for (&t, &e) in indices.iter().zip(&exps) {
            let p = e / sum;
            if is_ts(t) {
                ts_prob += p;
            } else {
                max_text = max_text.max(p);
            }
        }
        let only_ts = ts_prob > max_text;
        let last_ts = prev_tokens
            .get(self.prompt_len..)
            .unwrap_or_default()
            .iter()
            .rev()
            .find(|&&t| is_ts(t))
            .copied()
            .unwrap_or(self.ts_min);
        for (&t, l) in indices.iter().zip(logits.iter_mut()) {
            let suppress =
                (only_ts && !is_ts(t)) || t == self.no_timestamps || (is_ts(t) && t < last_ts);
            if suppress {
                *l = f32::NEG_INFINITY;
            }
        }
        Logits::sparse(logits, indices)
    }
}

/// Fills in fields that older `tokenizer.json` files leave out and
/// rten-text requires: a BPE model's `ignore_merges` (added to Hugging
/// Face's format in 2024; onnx-community's Whisper files predate it). The
/// default is what the older files meant. Text that is not JSON is
/// returned unchanged, for the parser to report.
pub fn complete_tokenizer_json(json: &str) -> Cow<'_, str> {
    let Ok(mut v) = serde_json::from_str::<serde_json::Value>(json) else {
        return Cow::Borrowed(json);
    };
    let Some(model) = v.get_mut("model").and_then(|m| m.as_object_mut()) else {
        return Cow::Borrowed(json);
    };
    if model.get("type").and_then(|t| t.as_str()) != Some("BPE")
        || model.contains_key("ignore_merges")
    {
        return Cow::Borrowed(json);
    }
    model.insert("ignore_merges".into(), serde_json::Value::Bool(false));
    Cow::Owned(v.to_string())
}

/// The periodic Hann window (`torch.hann_window`).
pub fn hann_window(size: usize) -> Vec<f32> {
    (0..size)
        .map(|i| {
            (std::f32::consts::PI * i as f32 / size as f32)
                .sin()
                .powi(2)
        })
        .collect()
}

fn hz_to_mel(f: f64) -> f64 {
    // The Slaney scale (librosa's default): linear below 1 kHz, log above.
    const F_SP: f64 = 200.0 / 3.0;
    let log_step = 6.4f64.ln() / 27.0;
    if f < 1000.0 {
        f / F_SP
    } else {
        15.0 + (f / 1000.0).ln() / log_step
    }
}

fn mel_to_hz(m: f64) -> f64 {
    const F_SP: f64 = 200.0 / 3.0;
    let log_step = 6.4f64.ln() / 27.0;
    if m < 15.0 {
        m * F_SP
    } else {
        1000.0 * (log_step * (m - 15.0)).exp()
    }
}

/// The mel filter bank Whisper uses (`librosa.filters.mel(sr, n_fft,
/// n_mels)`: Slaney scale and area normalization), shape
/// `[n_mels, n_fft / 2 + 1]`.
pub fn mel_filters(n_mels: usize, n_fft: usize, sample_rate: u32) -> NdTensor<f32, 2> {
    let bins = n_fft / 2 + 1;
    let sr = f64::from(sample_rate);
    let fft_freqs: Vec<f64> = (0..bins).map(|k| k as f64 * sr / n_fft as f64).collect();
    let (lo, hi) = (hz_to_mel(0.0), hz_to_mel(sr / 2.0));
    let mel_f: Vec<f64> = (0..n_mels + 2)
        .map(|i| mel_to_hz(lo + (hi - lo) * i as f64 / (n_mels + 1) as f64))
        .collect();
    let mut out = NdTensor::zeros([n_mels, bins]);
    for i in 0..n_mels {
        let (l, c, r) = (mel_f[i], mel_f[i + 1], mel_f[i + 2]);
        let norm = 2.0 / (r - l);
        for (k, &f) in fft_freqs.iter().enumerate() {
            let lower = (f - l) / (c - l);
            let upper = (r - f) / (r - c);
            let w = lower.min(upper).max(0.0) * norm;
            out[[i, k]] = w as f32;
        }
    }
    out
}

/// Whisper's log-mel spectrogram of `audio`, zero-padded to `padded_len`
/// samples: shape `[n_mels, padded_len / HOP]`.
pub fn log_mel_spectrogram(
    audio: &[f32],
    padded_len: usize,
    filters: &NdTensor<f32, 2>,
    window: &[f32],
) -> NdTensor<f32, 2> {
    let audio: Cow<'_, [f32]> = if audio.len() < padded_len {
        let mut v = audio.to_vec();
        v.resize(padded_len, 0.0);
        Cow::Owned(v)
    } else {
        Cow::Borrowed(audio)
    };
    let n_frames = audio.len() / HOP;
    let bins = N_FFT / 2 + 1;
    let fft = FftPlanner::<f32>::new().plan_fft_forward(N_FFT);
    let mut power = NdTensor::<f32, 2>::zeros([bins, n_frames]);
    let mut buf = vec![Complex32::new(0.0, 0.0); N_FFT];
    for frame in 0..n_frames {
        for (k, slot) in buf.iter_mut().enumerate() {
            let s = audio.get(frame * HOP + k).copied().unwrap_or(0.0);
            *slot = Complex32::new(s * window.get(k).copied().unwrap_or(0.0), 0.0);
        }
        fft.process(&mut buf);
        for (b, c) in buf.iter().take(bins).enumerate() {
            power[[b, frame]] = c.norm_sqr();
        }
    }
    let n_mels = filters.size(0);
    let mut mels = NdTensor::<f32, 2>::zeros([n_mels, n_frames]);
    for m in 0..n_mels {
        for b in 0..bins {
            let w = filters[[m, b]];
            if w == 0.0 {
                continue;
            }
            for f in 0..n_frames {
                mels[[m, f]] += w * power[[b, f]];
            }
        }
    }
    let mut max = f32::NEG_INFINITY;
    mels.apply(|x| x.max(1e-10).log10());
    for &v in mels.iter() {
        max = max.max(v);
    }
    mels.apply(|x| (x.max(max - 8.0) + 4.0) / 4.0);
    mels
}

/// Mono samples in `-1.0..=1.0` from 16-bit PCM.
pub fn to_f32(samples: &[i16]) -> Vec<f32> {
    samples
        .iter()
        .map(|&s| f32::from(s) / f32::from(i16::MAX))
        .collect()
}

/// A view helper for tests: the filter bank as rows.
#[cfg(test)]
fn row(t: &NdTensor<f32, 2>, i: usize) -> Vec<f32> {
    let v: rten_tensor::NdTensorView<'_, f32, 1> = t.slice(i);
    v.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hann_window_is_periodic() {
        let w = hann_window(4);
        assert!((w[0] - 0.0).abs() < 1e-6);
        assert!((w[1] - 0.5).abs() < 1e-6);
        assert!((w[2] - 1.0).abs() < 1e-6);
        assert!((w[3] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn mel_scale_round_trips() {
        for f in [0.0, 440.0, 999.0, 1000.0, 4000.0, 8000.0] {
            assert!((mel_to_hz(hz_to_mel(f)) - f).abs() < 1e-6, "{f}");
        }
        assert!((hz_to_mel(1000.0) - 15.0).abs() < 1e-9);
    }

    #[test]
    fn mel_filters_match_librosa() {
        // Values from librosa.filters.mel(sr=16000, n_fft=400, n_mels=80)
        // (the `mel_80` table rten-examples ships).
        let f = mel_filters(80, 400, 16_000);
        assert_eq!(f.shape(), [80, 201]);
        let r0 = row(&f, 0);
        assert!((r0[1] - 0.024_862_4).abs() < 1e-5, "{}", r0[1]);
        assert_eq!(r0[0], 0.0);
        assert!(r0[2..].iter().all(|&x| x == 0.0));
        let r79 = row(&f, 79);
        let nonzero: Vec<usize> = (0..201).filter(|&k| r79[k] > 0.0).collect();
        assert_eq!(nonzero.first(), Some(&186));
        assert!((r79.iter().copied().fold(0.0, f32::max) - 0.003_164_7).abs() < 1e-5);
        // Each filter has positive area, and the table's sum and peak
        // match librosa's.
        for i in 0..80 {
            assert!(row(&f, i).iter().sum::<f32>() > 0.0, "{i}");
        }
        let sum: f32 = f.iter().sum();
        assert!((sum - 1.999_024).abs() < 1e-3, "{sum}");
        let peak = f.iter().copied().fold(0.0, f32::max);
        assert!((peak - 0.025_880_7).abs() < 1e-5, "{peak}");
    }

    #[test]
    fn spectrogram_shape_and_range() {
        let filters = mel_filters(80, N_FFT, 16_000);
        let window = hann_window(N_FFT);
        let tone: Vec<f32> = (0..16_000)
            .map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / 16_000.0).sin() * 0.5)
            .collect();
        let mel = log_mel_spectrogram(&tone, 32_000, &filters, &window);
        assert_eq!(mel.shape(), [80, 200]);
        let max = mel.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let min = mel.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(max <= 1.0 + (4.0 / 4.0) && min >= max - 2.0 - 1e-6);
    }

    #[test]
    fn old_tokenizer_files_are_completed() {
        let old = r#"{"model": {"type": "BPE", "vocab": {}, "merges": []}}"#;
        let fixed = complete_tokenizer_json(old);
        assert!(fixed.contains(r#""ignore_merges":false"#), "{fixed}");
        let new = r#"{"model": {"type": "BPE", "ignore_merges": true}}"#;
        assert!(matches!(complete_tokenizer_json(new), Cow::Borrowed(_)));
        assert!(matches!(
            complete_tokenizer_json("not json"),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn to_f32_scales() {
        assert_eq!(to_f32(&[0, i16::MAX]), vec![0.0, 1.0]);
    }

    #[test]
    fn missing_files_are_named() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("base.en");
        std::fs::create_dir_all(dir.join("onnx")).unwrap();
        let e = RtenWhisperFiles::in_dir(&dir).unwrap_err().to_string();
        assert!(e.contains("encoder_model_int8.onnx"), "{e}");
        for n in [
            "onnx/encoder_model_int8.onnx",
            "onnx/decoder_model_merged_int8.onnx",
            "tokenizer.json",
        ] {
            std::fs::write(dir.join(n), b"x").unwrap();
        }
        let f = RtenWhisperFiles::in_dir(&dir).unwrap();
        assert!(f.english_only);
        assert!(f.encoder.ends_with("encoder_model_int8.onnx"));
    }
}
