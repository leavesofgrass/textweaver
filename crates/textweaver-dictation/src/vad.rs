//! Finding speech in audio: utterances split at the speaker's pauses.
//!
//! Whisper does not stream, and it tends to invent words in long
//! silences. A voice detector scores each 16 ms frame; [`speech_spans`]
//! turns the scores into utterances, split wherever the speaker paused for
//! about 600 ms, each padded a little so no word is clipped. Dictation
//! transcribes each utterance on its own and skips the silence between.
//!
//! Two ways in, with the same answer:
//!
//! - **Recorded audio:** [`utterances`] scores all of it, then splits.
//! - **Live audio:** [`SpeechFinder`] takes the audio as it arrives, in
//!   pieces of any size, and reports each utterance as it opens (after
//!   200 ms of speech) and as it closes (after 600 ms of silence), through
//!   [`SpanTracker`]. Over the same audio it finds exactly what
//!   [`utterances`] finds.
//!
//! The detector is earshot, a pure-Rust voice activity detector whose
//! small model is built in (features `rten` and `mic`). The splitting
//! itself needs no detector, so it is always built.

use std::ops::Range;

use crate::capture::WHISPER_SAMPLE_RATE;

/// Samples per detector frame at 16 kHz (16 ms): earshot's unit.
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

impl VadConfig {
    fn frames(ms: u32) -> usize {
        (ms as usize).div_ceil(FRAME_MS)
    }

    fn min_silence_frames(&self) -> usize {
        Self::frames(self.min_silence_ms).max(1)
    }

    fn min_speech_frames(&self) -> usize {
        Self::frames(self.min_speech_ms).max(1)
    }

    fn pad_samples(&self) -> usize {
        self.pad_ms as usize * WHISPER_SAMPLE_RATE as usize / 1000
    }
}

/// Utterances in frame scores, as sample ranges of 16 kHz audio of
/// `total` samples.
pub fn speech_spans(scores: &[f32], total: usize, config: &VadConfig) -> Vec<Range<usize>> {
    let min_silence = config.min_silence_frames();
    let min_speech = config.min_speech_frames();
    let pad = config.pad_samples();
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
    for f in frames.into_iter().filter(|f| f.len() >= min_speech) {
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

/// What live speech finding reports, in order: an utterance opens, then
/// closes, then the next opens. Positions are samples of 16 kHz audio
/// counted from the start of the session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpeechEvent {
    /// An utterance has begun: the speaker has talked for at least the
    /// minimum speech time. Its audio starts at `start` (padding
    /// included), which is never before the end of the last utterance.
    Opened {
        /// The utterance's first sample.
        start: usize,
    },
    /// The utterance ended at a pause (or at the end of the audio): its
    /// whole range, padding included. All of it has already arrived.
    Closed(Range<usize>),
}

/// The speech frames of an utterance still being heard.
#[derive(Clone, Copy, Debug)]
struct Candidate {
    /// First speech frame.
    first: usize,
    /// Last speech frame so far.
    last: usize,
    /// Its first sample, once opened (an `Opened` was reported, or it
    /// joined the utterance still waiting to close).
    opened: Option<usize>,
}

/// Splits frame scores into utterances as they arrive, with the same
/// rules and results as [`speech_spans`]: [`SpeechFinder`] without the
/// detector, so it can be driven by any scores.
#[derive(Clone, Debug)]
pub struct SpanTracker {
    config: VadConfig,
    /// Frames scored so far.
    frame: usize,
    current: Option<Candidate>,
    /// An utterance that has ended but is not reported closed yet: its
    /// padding may still be arriving, or speech may start close enough to
    /// join it.
    ending: Option<Range<usize>>,
    /// Where the latest speech frame ended.
    speech_end: Option<usize>,
}

impl SpanTracker {
    /// A tracker with these rules.
    pub fn new(config: VadConfig) -> Self {
        SpanTracker {
            config,
            frame: 0,
            current: None,
            ending: None,
            speech_end: None,
        }
    }

    /// Where the latest frame of speech ended (a sample), if any has been
    /// heard: how far the speaker has talked, silence after it excluded.
    pub fn speech_end(&self) -> Option<usize> {
        self.speech_end
    }

    /// The rules in use.
    pub fn config(&self) -> &VadConfig {
        &self.config
    }

    /// Frames scored so far.
    pub fn frames(&self) -> usize {
        self.frame
    }

    /// Takes the next frame's score; `seen` is how many samples have
    /// arrived, this frame's included. Events go to `out`.
    pub fn push(&mut self, score: f32, seen: usize, out: &mut Vec<SpeechEvent>) {
        let i = self.frame;
        self.frame += 1;
        let pad = self.config.pad_samples();
        if score >= self.config.threshold {
            self.speech_end = Some((i + 1) * FRAME);
            let c = self.current.get_or_insert(Candidate {
                first: i,
                last: i,
                opened: None,
            });
            c.last = i;
            if c.opened.is_none() && c.last + 1 - c.first >= self.config.min_speech_frames() {
                let start = (c.first * FRAME).saturating_sub(pad);
                match self.ending.take() {
                    // Close enough to the last utterance to join it, as
                    // `speech_spans` joins overlapping padding: it goes on.
                    Some(prev) if start <= prev.end => c.opened = Some(prev.start),
                    prev => {
                        if let Some(prev) = prev {
                            out.push(SpeechEvent::Closed(prev));
                        }
                        c.opened = Some(start);
                        out.push(SpeechEvent::Opened { start });
                    }
                }
            }
        } else if let Some(c) = self.current
            && i - c.last >= self.config.min_silence_frames()
        {
            self.current = None;
            if let Some(start) = c.opened {
                // Unclamped: the padding is waited for below.
                self.ending = Some(start..(c.last + 1) * FRAME + pad);
            }
            // Otherwise it was too short: a click or a cough.
        }
        self.release(seen, out);
    }

    /// Reports the ending utterance as closed once its padding has
    /// arrived and no speech can start close enough to join it.
    fn release(&mut self, seen: usize, out: &mut Vec<SpeechEvent>) {
        let Some(prev) = &self.ending else {
            return;
        };
        let next_start = self.current.map_or(self.frame, |c| c.first) * FRAME;
        if seen >= prev.end && next_start.saturating_sub(self.config.pad_samples()) > prev.end {
            out.push(SpeechEvent::Closed(prev.clone()));
            self.ending = None;
        }
    }

    /// The audio has ended after `total` samples: closes whatever is open
    /// (padding cut at the end), as [`speech_spans`] does.
    pub fn finish(&mut self, total: usize, out: &mut Vec<SpeechEvent>) {
        let pad = self.config.pad_samples();
        let current = self.current.take();
        let ending = self.ending.take();
        match (ending, current) {
            // A candidate that never reached the minimum is dropped (an
            // opened one would already have taken the ending utterance).
            (Some(prev), Some(Candidate { opened: None, .. })) => {
                out.push(SpeechEvent::Closed(prev.start..prev.end.min(total)));
            }
            (prev, Some(c)) => {
                if let Some(prev) = prev {
                    out.push(SpeechEvent::Closed(prev.start..prev.end.min(total)));
                }
                if let Some(start) = c.opened {
                    let end = ((c.last + 1) * FRAME + pad).min(total);
                    out.push(SpeechEvent::Closed(start..end));
                }
            }
            (Some(prev), None) => out.push(SpeechEvent::Closed(prev.start..prev.end.min(total))),
            (None, None) => {}
        }
    }
}

/// Something that finds utterances in live 16 kHz audio.
pub trait FindSpeech: Send {
    /// Takes the next piece of audio (any length).
    fn push(&mut self, samples: &[f32]) -> Vec<SpeechEvent>;

    /// The audio has ended: closes whatever is open.
    fn finish(&mut self) -> Vec<SpeechEvent>;

    /// Where the latest speech heard ended (a sample), if known.
    fn speech_end(&self) -> Option<usize> {
        None
    }
}

/// Frames scored by any function of their samples: for tests, and for
/// audio already known to be speech.
pub struct ScoredFinder<F> {
    score: F,
    tracker: SpanTracker,
    frame: Vec<f32>,
    seen: usize,
}

impl<F: FnMut(&[f32; FRAME]) -> f32 + Send> ScoredFinder<F> {
    /// A finder scoring each frame with `score`.
    pub fn new(config: VadConfig, score: F) -> Self {
        ScoredFinder {
            score,
            tracker: SpanTracker::new(config),
            frame: Vec::with_capacity(FRAME),
            seen: 0,
        }
    }

    fn score_frame(&mut self, out: &mut Vec<SpeechEvent>) {
        let mut frame = [0f32; FRAME];
        frame[..self.frame.len()].copy_from_slice(&self.frame);
        self.frame.clear();
        let s = (self.score)(&frame);
        self.tracker.push(s, self.seen, out);
    }
}

impl<F: FnMut(&[f32; FRAME]) -> f32 + Send> FindSpeech for ScoredFinder<F> {
    fn push(&mut self, mut samples: &[f32]) -> Vec<SpeechEvent> {
        let mut out = Vec::new();
        while !samples.is_empty() {
            let n = (FRAME - self.frame.len()).min(samples.len());
            self.frame.extend_from_slice(&samples[..n]);
            self.seen += n;
            samples = &samples[n..];
            if self.frame.len() == FRAME {
                self.score_frame(&mut out);
            }
        }
        out
    }

    fn finish(&mut self) -> Vec<SpeechEvent> {
        let mut out = Vec::new();
        // The last, partial frame is zero-padded, as `scores` does.
        if !self.frame.is_empty() {
            self.score_frame(&mut out);
        }
        self.tracker.finish(self.seen, &mut out);
        out
    }

    fn speech_end(&self) -> Option<usize> {
        self.tracker.speech_end()
    }
}

/// A frame scorer that can move between threads.
#[cfg(any(feature = "rten", feature = "mic"))]
type BoxedScore = Box<dyn FnMut(&[f32; FRAME]) -> f32 + Send>;

/// Speech found live with earshot: audio in, [`SpeechEvent`]s out.
#[cfg(any(feature = "rten", feature = "mic"))]
pub struct SpeechFinder(ScoredFinder<BoxedScore>);

#[cfg(any(feature = "rten", feature = "mic"))]
impl SpeechFinder {
    /// A finder with these rules and a fresh detector.
    pub fn new(config: VadConfig) -> Self {
        let mut detector = Box::new(earshot::Detector::default());
        SpeechFinder(ScoredFinder::new(
            config,
            Box::new(move |f: &[f32; FRAME]| detector.predict_f32(f)),
        ))
    }
}

#[cfg(any(feature = "rten", feature = "mic"))]
impl FindSpeech for SpeechFinder {
    fn push(&mut self, samples: &[f32]) -> Vec<SpeechEvent> {
        self.0.push(samples)
    }

    fn finish(&mut self) -> Vec<SpeechEvent> {
        self.0.finish()
    }

    fn speech_end(&self) -> Option<usize> {
        self.0.speech_end()
    }
}

/// Scores every 16 ms frame of 16 kHz `audio` with earshot (the last,
/// partial frame is zero-padded).
#[cfg(any(feature = "rten", feature = "mic"))]
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
#[cfg(any(feature = "rten", feature = "mic"))]
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

    /// Runs the tracker over scores, a frame at a time, with `total`
    /// samples in all (the last frame may be partial).
    fn track(scores: &[f32], total: usize, config: &VadConfig) -> Vec<(usize, SpeechEvent)> {
        let mut t = SpanTracker::new(*config);
        let mut out = Vec::new();
        let mut events = Vec::new();
        for (i, &s) in scores.iter().enumerate() {
            let seen = ((i + 1) * FRAME).min(total);
            t.push(s, seen, &mut out);
            events.extend(out.drain(..).map(|e| (i, e)));
        }
        t.finish(total, &mut out);
        events.extend(out.drain(..).map(|e| (scores.len(), e)));
        events
    }

    /// Checks the events are well formed and returns the closed ranges.
    fn closed(events: &[(usize, SpeechEvent)], total: usize) -> Vec<Range<usize>> {
        let mut open: Option<usize> = None;
        let mut spans = Vec::new();
        let mut last_end = 0;
        for (i, e) in events {
            match e {
                SpeechEvent::Opened { start } => {
                    assert!(open.is_none(), "opened twice: {events:?}");
                    assert!(*start >= last_end, "overlaps the last: {events:?}");
                    assert!(*start <= (i + 1) * FRAME, "opened in the future");
                    open = Some(*start);
                }
                SpeechEvent::Closed(r) => {
                    assert_eq!(open.take(), Some(r.start), "closed unopened: {events:?}");
                    // All of its audio has arrived.
                    assert!(r.end <= ((i + 1) * FRAME).min(total), "{events:?}");
                    last_end = r.end;
                    spans.push(r.clone());
                }
            }
        }
        assert!(open.is_none(), "left open: {events:?}");
        spans
    }

    #[test]
    fn live_splitting_matches_the_recorded_path() {
        let config = VadConfig {
            min_silence_ms: 48,
            min_speech_ms: 32,
            pad_ms: 0,
            ..VadConfig::default()
        };
        let s = frames("..####.##...###...#");
        let total = s.len() * FRAME;
        let events = track(&s, total, &config);
        assert_eq!(closed(&events, total), speech_spans(&s, total, &config));
        // Opened as soon as two frames of speech were heard, closed three
        // frames into the pause.
        assert_eq!(events[0], (3, SpeechEvent::Opened { start: 2 * FRAME }));
        assert_eq!(events[1], (11, SpeechEvent::Closed(2 * FRAME..9 * FRAME)));
    }

    #[test]
    fn default_rules_close_at_the_pause() {
        // 1 second of speech, then silence: open after 200 ms (13 frames),
        // closed after 600 ms (38 frames) of silence, padded 200 ms.
        let mut s = vec![0.1; 20];
        s.extend(vec![0.9; 62]);
        s.extend(vec![0.1; 100]);
        let total = s.len() * FRAME;
        let config = VadConfig::default();
        let events = track(&s, total, &config);
        let pad = 3200;
        assert_eq!(
            events,
            vec![
                (
                    32,
                    SpeechEvent::Opened {
                        start: 20 * FRAME - pad
                    }
                ),
                (
                    81 + 38,
                    SpeechEvent::Closed(20 * FRAME - pad..82 * FRAME + pad)
                ),
            ]
        );
    }

    /// A small deterministic generator for the property test.
    struct XorShift(u64);

    impl XorShift {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    #[test]
    fn live_splitting_matches_on_random_audio() {
        let mut rng = XorShift(0x9e37_79b9_7f4a_7c15);
        for case in 0..2000 {
            let config = VadConfig {
                threshold: 0.5,
                min_silence_ms: 16 * (1 + rng.below(8) as u32),
                min_speech_ms: 16 * rng.below(6) as u32,
                pad_ms: [0, 8, 16, 40, 64, 200][rng.below(6) as usize],
            };
            // Runs of speech and silence of random lengths.
            let mut s = Vec::new();
            let mut speech = rng.below(2) == 0;
            while s.len() < 120 {
                let run = 1 + rng.below(12) as usize;
                s.extend(std::iter::repeat_n(if speech { 0.9 } else { 0.1 }, run));
                speech = !speech;
            }
            let total = s.len() * FRAME - rng.below(FRAME as u64) as usize;
            let events = track(&s, total, &config);
            assert_eq!(
                closed(&events, total),
                speech_spans(&s, total, &config),
                "case {case}: {config:?} {s:?}"
            );
        }
    }

    #[test]
    fn scored_finder_takes_any_piece_size() {
        let config = VadConfig {
            min_silence_ms: 48,
            min_speech_ms: 32,
            pad_ms: 16,
            ..VadConfig::default()
        };
        // Speech is loud audio.
        let loud = |f: &[f32; FRAME]| f.iter().map(|x| x.abs()).sum::<f32>() / FRAME as f32;
        let mut audio = vec![0.0f32; 3 * FRAME];
        audio.extend(vec![0.9; 5 * FRAME]);
        audio.extend(vec![0.0; 6 * FRAME + 100]);
        let pieces = [1usize, 7, 255, 256, 257, 1000, audio.len()];
        let mut results = Vec::new();
        for size in pieces {
            let mut f = ScoredFinder::new(config, loud);
            let mut events = Vec::new();
            for chunk in audio.chunks(size) {
                events.extend(f.push(chunk));
            }
            events.extend(f.finish());
            results.push(events);
        }
        assert!(results.windows(2).all(|w| w[0] == w[1]), "{results:?}");
        assert_eq!(
            results[0],
            vec![
                SpeechEvent::Opened { start: 2 * FRAME },
                SpeechEvent::Closed(2 * FRAME..9 * FRAME),
            ]
        );
        let mut f = ScoredFinder::new(config, loud);
        assert_eq!(f.speech_end(), None);
        f.push(&audio);
        assert_eq!(f.speech_end(), Some(8 * FRAME));
    }

    #[cfg(any(feature = "rten", feature = "mic"))]
    #[test]
    fn silence_has_no_speech() {
        let silence = vec![0.0f32; 16_000];
        assert!(utterances(&silence, &VadConfig::default()).is_empty());
        assert_eq!(scores(&silence[..300]).len(), 2);
        let mut f = SpeechFinder::new(VadConfig::default());
        assert!(f.push(&silence).is_empty());
        assert!(f.finish().is_empty());
    }

    /// The speech fixtures in `fixtures/d/`, when their WAV files have
    /// been made (they are not committed; see the folder's README).
    #[cfg(any(feature = "rten", feature = "mic"))]
    #[test]
    fn live_finding_matches_on_the_fixtures() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/d");
        let mut found = 0;
        for name in ["stream-note", "stream-long", "stream-short"] {
            let Ok(bytes) = std::fs::read(dir.join(format!("{name}.wav"))) else {
                continue;
            };
            found += 1;
            let audio =
                crate::audio::to_whisper_rate(&crate::audio::read_wav(&bytes).unwrap()).unwrap();
            let config = VadConfig::default();
            let batch = utterances(&audio, &config);
            assert!(!batch.is_empty(), "{name}: no speech found");
            // Pieces the size a microphone delivers (10 ms, 20 ms, odd).
            for size in [160usize, 320, 999, 4096] {
                let mut f = SpeechFinder::new(config);
                let mut events = Vec::new();
                for chunk in audio.chunks(size) {
                    events.extend(f.push(chunk).into_iter().map(|e| (0, e)));
                }
                events.extend(f.finish().into_iter().map(|e| (0, e)));
                let live: Vec<Range<usize>> = events
                    .iter()
                    .filter_map(|(_, e)| match e {
                        SpeechEvent::Closed(r) => Some(r.clone()),
                        SpeechEvent::Opened { .. } => None,
                    })
                    .collect();
                assert_eq!(live, batch, "{name}, pieces of {size}");
            }
            eprintln!(
                "{name}: {} utterances, live and recorded agree",
                batch.len()
            );
        }
        if found == 0 {
            eprintln!("No fixture WAV files in {}; skipped.", dir.display());
        }
    }
}
