//! Finding speech in audio with earshot, a pure-Rust voice activity
//! detector whose small model is built in (features `rten` and `mic`).
//!
//! Whisper does not stream, and it tends to invent words in long
//! silences. The detector scores each 16 ms frame; [`speech_spans`] turns
//! the scores into utterances, split wherever the speaker paused for
//! about 600 ms, each padded a little so no word is clipped. Dictation
//! transcribes each utterance on its own and skips the silence between.

use std::ops::Range;

use crate::capture::WHISPER_SAMPLE_RATE;

/// Samples per detector frame at 16 kHz (16 ms).
pub const FRAME: usize = 256;

/// Frame length in milliseconds.
const FRAME_MS: usize = 16;

/// How speech is found.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VadConfig {
    /// A frame scoring at least this is speech.
    pub threshold: f32,
    /// A pause at least this long ends an utterance.
    pub min_silence_ms: u32,
    /// Speech shorter than this (a click, a cough) is dropped.
    pub min_speech_ms: u32,
    /// Audio kept before and after each utterance.
    pub pad_ms: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        VadConfig {
            threshold: 0.5,
            min_silence_ms: 600,
            min_speech_ms: 200,
            pad_ms: 200,
        }
    }
}

/// Utterances in frame scores, as sample ranges of 16 kHz audio of
/// `total` samples.
pub fn speech_spans(scores: &[f32], total: usize, config: &VadConfig) -> Vec<Range<usize>> {
    let ms_frames = |ms: u32| (ms as usize).div_ceil(FRAME_MS);
    let min_silence = ms_frames(config.min_silence_ms).max(1);
    let min_speech = ms_frames(config.min_speech_ms);
    let pad = config.pad_ms as usize * WHISPER_SAMPLE_RATE as usize / 1000;
    let mut frames: Vec<Range<usize>> = Vec::new();
    let mut start: Option<usize> = None;
    let mut last_speech = 0;
    for (i, &s) in scores.iter().enumerate() {
        if s >= config.threshold {
            if start.is_none() {
                start = Some(i);
            }
            last_speech = i;
        } else if let Some(st) = start
            && i - last_speech >= min_silence
        {
            frames.push(st..last_speech + 1);
            start = None;
        }
    }
    if let Some(st) = start {
        frames.push(st..last_speech + 1);
    }
    let mut out: Vec<Range<usize>> = Vec::new();
    for f in frames.into_iter().filter(|f| f.len() >= min_speech.max(1)) {
        let s = (f.start * FRAME).saturating_sub(pad);
        let e = (f.end * FRAME + pad).min(total);
        match out.last_mut() {
            // Padding can make neighbours overlap: join them.
            Some(prev) if s <= prev.end => prev.end = e,
            _ => out.push(s..e),
        }
    }
    out
}

/// Scores every 16 ms frame of 16 kHz `audio` with earshot (the last,
/// partial frame is zero-padded).
pub fn scores(audio: &[f32]) -> Vec<f32> {
    let mut detector = earshot::Detector::default();
    let mut frame = [0f32; FRAME];
    audio
        .chunks(FRAME)
        .map(|c| {
            frame.fill(0.0);
            frame[..c.len()].copy_from_slice(c);
            detector.predict_f32(&frame)
        })
        .collect()
}

/// The utterances in 16 kHz `audio`.
pub fn utterances(audio: &[f32], config: &VadConfig) -> Vec<Range<usize>> {
    speech_spans(&scores(audio), audio.len(), config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames(pattern: &str) -> Vec<f32> {
        pattern
            .chars()
            .map(|c| if c == '#' { 0.9 } else { 0.1 })
            .collect()
    }

    #[test]
    fn pauses_split_utterances() {
        let config = VadConfig {
            min_silence_ms: 48, // 3 frames
            min_speech_ms: 32,  // 2 frames
            pad_ms: 0,
            ..VadConfig::default()
        };
        //           0123456789012345678
        let s = frames("..####.##...###...#");
        let spans = speech_spans(&s, s.len() * FRAME, &config);
        // "####.##" is one utterance (a 1-frame pause), "###" another, and
        // the lone "#" is too short.
        assert_eq!(spans, vec![2 * FRAME..9 * FRAME, 12 * FRAME..15 * FRAME]);
    }

    #[test]
    fn padding_is_clamped_and_joins_neighbours() {
        let config = VadConfig {
            min_silence_ms: 32,
            min_speech_ms: 16,
            pad_ms: 32, // 512 samples
            ..VadConfig::default()
        };
        let s = frames("#...#");
        let total = s.len() * FRAME;
        assert_eq!(speech_spans(&s, total, &config), vec![0..total]);
        assert!(speech_spans(&frames("....."), total, &config).is_empty());
        assert!(speech_spans(&[], 0, &config).is_empty());
    }

    #[test]
    fn silence_has_no_speech() {
        let silence = vec![0.0f32; 16_000];
        assert!(utterances(&silence, &VadConfig::default()).is_empty());
        assert_eq!(scores(&silence[..300]).len(), 2);
    }
}
