//! In-process eSpeak NG (feature `espeak`), ported from star's
//! `ESpeakLibBackend` (`star/tts/espeak.py`, Part 2 section 2).
//!
//! libespeak-ng is a process-wide singleton and its calls block, so the
//! backend runs a worker thread that owns synthesis; `speak` hands it a job
//! and returns at once (ADR-0003), and `poll` delivers the events the worker
//! collected.
//!
//! **Word events.** libespeak-ng reports each word with `audio_position`,
//! milliseconds into that message's audio. They are passed on as
//! `RawEvent::Word { audio_ms }` and the service schedules the highlight at
//! `audio_ms + latency_offset` (default 120 ms, star's
//! `espeak_highlight_offset_ms`), never on arrival (star's rule, since
//! events can arrive in a burst ahead of the audio). Positions are converted
//! to UTF-8 byte ranges of the utterance text. espeak-ng 1.52 reports
//! 1-based *character* positions and lengths for UTF-8 input (verified in
//! the dev container: "Café pour Émile" gives `pour` at 6 and `Émile` at 11,
//! length 5); star assumed bytes, which is wrong for non-ASCII text.
//!
//! **Outputs** ([`EspeakOutput`]): `Playback` lets libespeak-ng play the
//! audio (the default). `Virtual` synthesizes in retrieval mode, discards
//! the audio, and paces `Finished` to the audio's real duration, so machines
//! without a sound device (the dev container, CI) get realistic event
//! timing. Select it with `TEXTWEAVER_ESPEAK_OUTPUT=virtual`.
//! [`synthesize_to_file`](SpeechBackend::synthesize_to_file) always uses
//! retrieval mode and writes a 16-bit mono WAV;
//! [`synthesize_utterance`](SpeechBackend::synthesize_utterance) also
//! returns each word's `audio_position` in that file, for word-level
//! subtitles in audio export.
//!
//! **Mapping** (ADR-0004): rate in wpm clamped to espeak's 80..=450 (one
//! mapping for every espeak build, fixing star's 0.8× CLI versus 1.0× library
//! mismatch, Q14); pitch `50 + semitones × 50 / 12` on espeak's 0..=100
//! scale; volume percent as is (espeak's 100 is normal). No native pause
//! (the service emulates it) and no tones.

//! **Phonemes** (feature `espeak-phonemes`, which `espeak` includes):
//! [`text_to_phonemes`] turns text into IPA with the same library, for
//! Piper's neural voices (`textweaver-piper`). A build with only
//! `espeak-phonemes` compiles this module but registers no espeak-ng
//! backend.

// With only `espeak-phonemes`, the backend below is compiled but unused.
#![cfg_attr(not(feature = "espeak"), allow(dead_code))]

mod ffi;
mod sys;

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use textweaver_core::{Utterance, UtteranceId, UtteranceKind};

use crate::backend::{
    BackendId, Caps, EventSink, FileSynthesis, RawEvent, SpeechBackend, SpeechError, Voice,
    VoiceParams, WordTiming,
};

/// Rate range espeak-ng accepts, in wpm.
pub const RATE_RANGE: std::ops::RangeInclusive<u16> = 80..=450;

/// What the espeak-ng backend can do.
pub const CAPS: Caps = Caps::WORD_EVENTS
    .union(Caps::AUDIO_CLOCK)
    .union(Caps::PITCH)
    .union(Caps::VOLUME)
    .union(Caps::SYNTH_TO_FILE);

/// Where the audio goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EspeakOutput {
    /// libespeak-ng plays it.
    #[default]
    Playback,
    /// Synthesized and discarded, with real-time pacing (no sound device).
    Virtual,
}

impl EspeakOutput {
    /// `Virtual` when `TEXTWEAVER_ESPEAK_OUTPUT=virtual`, else `Playback`.
    pub fn from_env() -> Self {
        match std::env::var("TEXTWEAVER_ESPEAK_OUTPUT") {
            Ok(v) if v.eq_ignore_ascii_case("virtual") => EspeakOutput::Virtual,
            _ => EspeakOutput::Playback,
        }
    }

    fn mode(self) -> ffi::Mode {
        match self {
            EspeakOutput::Playback => ffi::Mode::Playback,
            EspeakOutput::Virtual => ffi::Mode::Retrieval,
        }
    }
}

/// The initialized engine: mode and sample rate.
static ENGINE: Mutex<Option<(ffi::Mode, u32)>> = Mutex::new(None);

/// Makes sure libespeak-ng runs in `mode`, re-initializing if needed.
fn ensure(mode: ffi::Mode) -> Result<u32, String> {
    let mut e = ENGINE.lock().unwrap_or_else(|p| p.into_inner());
    if let Some((m, rate)) = *e {
        if m == mode {
            return Ok(rate);
        }
        ffi::terminate();
        *e = None;
    }
    let rate = ffi::initialize(mode)?;
    *e = Some((mode, rate));
    Ok(rate)
}

/// True when libespeak-ng initializes (its data is installed).
pub fn available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        let e = ENGINE.lock().unwrap_or_else(|p| p.into_inner());
        if e.is_some() {
            return true;
        }
        drop(e);
        ensure(ffi::Mode::Playback).is_ok()
    })
}

/// Translates `text` into IPA phonemes with libespeak-ng's `voice`
/// (`"en-us"`), one string per clause, as `espeak_TextToPhonemes` returns
/// them: words separated by spaces, stress marks kept, and the punctuation
/// that ended each clause dropped (Piper's phonemizer adds it back).
///
/// libespeak-ng is a process-wide singleton: this selects `voice` for the
/// whole process, so a running espeak-ng backend speaks its next utterance
/// with the voice it sets again for it (it sets its voice before each one).
/// Calls are serialized with the backend's initialization. Initializes the
/// library in retrieval mode (no audio device) when nothing has yet.
///
/// Errors say why: the library is missing, too old to have
/// `espeak_TextToPhonemes`, or has no such voice.
pub fn text_to_phonemes(voice: &str, text: &str) -> Result<Vec<String>, String> {
    let mut e = ENGINE.lock().unwrap_or_else(|p| p.into_inner());
    if e.is_none() {
        let rate = ffi::initialize(ffi::Mode::Retrieval)?;
        *e = Some((ffi::Mode::Retrieval, rate));
    }
    ffi::set_voice(voice)?;
    ffi::text_to_phonemes(text)
}

/// True when libespeak-ng loads and can translate text into phonemes
/// (`espeak_TextToPhonemes`, eSpeak NG 1.49 and later).
pub fn phonemes_available() -> bool {
    sys::load().is_ok() && sys::has_text_to_phonemes()
}

/// Maps canonical parameters onto espeak's scales.
fn espeak_values(p: &VoiceParams) -> (i32, i32, i32) {
    let rate = p.rate.wpm().clamp(*RATE_RANGE.start(), *RATE_RANGE.end());
    let pitch = (50.0 + f32::from(p.pitch.semitones()) * 50.0 / 12.0)
        .round()
        .clamp(0.0, 100.0) as i32;
    (i32::from(rate), pitch, i32::from(p.volume.percent()))
}

fn apply_params(p: &VoiceParams) -> Result<(), String> {
    let voice = p.voice.as_deref().unwrap_or("en-us");
    if let Err(e) = ffi::set_voice(voice)
        && p.voice.is_some()
    {
        return Err(e);
    }
    let (rate, pitch, volume) = espeak_values(p);
    ffi::set_param(ffi::Param::Rate, rate)?;
    ffi::set_param(ffi::Param::Pitch, pitch)?;
    ffi::set_param(ffi::Param::Volume, volume)?;
    Ok(())
}

/// UTF-8 byte offset of every char of `text`, plus `text.len()`.
fn char_offsets(text: &str) -> Vec<u32> {
    let to_u32 = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    text.char_indices()
        .map(|(i, _)| to_u32(i))
        .chain(std::iter::once(to_u32(text.len())))
        .collect()
}

/// Byte range for espeak's 1-based char position and char length.
fn byte_range(offsets: &[u32], position: i32, length: i32) -> std::ops::Range<u32> {
    let last = offsets.len().saturating_sub(1);
    let start = usize::try_from(position - 1).unwrap_or(0).min(last);
    let end = start
        .saturating_add(usize::try_from(length).unwrap_or(0))
        .min(last)
        .max(start);
    offsets[start]..offsets[end]
}

enum Job {
    Speak {
        id: UtteranceId,
        text: String,
        character: Option<char>,
        params: VoiceParams,
        epoch: u64,
    },
    ToFile {
        text: String,
        path: std::path::PathBuf,
        params: VoiceParams,
        reply: Sender<Result<FileSynthesis, SpeechError>>,
    },
}

/// The espeak-ng backend.
pub struct EspeakBackend {
    output: EspeakOutput,
    jobs: Option<Sender<Job>>,
    events: Receiver<(UtteranceId, RawEvent)>,
    worker: Option<JoinHandle<()>>,
    epoch: Arc<AtomicU64>,
    voices: Vec<Voice>,
    params: VoiceParams,
}

impl std::fmt::Debug for EspeakBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EspeakBackend")
            .field("output", &self.output)
            .field("params", &self.params)
            .finish_non_exhaustive()
    }
}

impl EspeakBackend {
    /// Starts the worker and initializes libespeak-ng for `output`.
    pub fn new(output: EspeakOutput) -> Result<Self, SpeechError> {
        let (job_tx, job_rx) = mpsc::channel::<Job>();
        let (ev_tx, ev_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let epoch = Arc::new(AtomicU64::new(0));
        let worker_epoch = Arc::clone(&epoch);
        let worker = std::thread::Builder::new()
            .name("textweaver-espeak".into())
            .spawn(move || {
                let started = ensure(output.mode()).map(|_| ffi::list_voices());
                match started {
                    Ok(voices) => {
                        let _ = ready_tx.send(Ok(voices));
                        work(output, &job_rx, &ev_tx, &worker_epoch);
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                    }
                }
            })
            .map_err(|e| SpeechError::Io(e.to_string()))?;
        let voices = ready_rx
            .recv()
            .map_err(|_| SpeechError::Unavailable("espeak", "worker died".into()))?
            .map_err(|e| SpeechError::Unavailable("espeak", e))?;
        let voices = voices
            .into_iter()
            .map(|v| Voice {
                id: if v.identifier.is_empty() {
                    v.name.clone()
                } else {
                    v.identifier
                },
                name: v.name,
                languages: v.languages,
                gender: match v.gender {
                    1 => Some("male".into()),
                    2 => Some("female".into()),
                    _ => None,
                },
                tags: Vec::new(),
            })
            .collect();
        Ok(EspeakBackend {
            output,
            jobs: Some(job_tx),
            events: ev_rx,
            worker: Some(worker),
            epoch,
            voices,
            params: VoiceParams::default(),
        })
    }

    /// The output this backend was created with.
    pub fn output(&self) -> EspeakOutput {
        self.output
    }

    fn send(&self, job: Job) -> Result<(), SpeechError> {
        self.jobs
            .as_ref()
            .ok_or(SpeechError::ServiceStopped)?
            .send(job)
            .map_err(|_| SpeechError::Engine("espeak worker stopped".into()))
    }
}

/// The worker loop: one job at a time.
fn work(
    output: EspeakOutput,
    jobs: &Receiver<Job>,
    events: &Sender<(UtteranceId, RawEvent)>,
    epoch: &Arc<AtomicU64>,
) {
    let mut applied: Option<VoiceParams> = None;
    while let Ok(job) = jobs.recv() {
        match job {
            Job::Speak {
                id,
                text,
                character,
                params,
                epoch: job_epoch,
            } => {
                if epoch.load(Ordering::SeqCst) != job_epoch {
                    let _ = events.send((id, RawEvent::Cancelled));
                    continue;
                }
                let sample_rate = match ensure(output.mode()) {
                    Ok(rate) => rate,
                    Err(e) => {
                        let _ = events.send((id, RawEvent::Error(e)));
                        continue;
                    }
                };
                if applied.as_ref() != Some(&params) {
                    if let Err(e) = apply_params(&params) {
                        let _ = events.send((id, RawEvent::Error(e)));
                        continue;
                    }
                    applied = Some(params);
                }
                speak_one(
                    output,
                    sample_rate,
                    id,
                    &text,
                    character,
                    job_epoch,
                    events,
                    epoch,
                );
            }
            Job::ToFile {
                text,
                path,
                params,
                reply,
            } => {
                let r = to_file(&text, &path, &params);
                // The engine mode changed; parameters must be re-applied.
                applied = None;
                let _ = reply.send(r);
            }
        }
    }
    ffi::set_sink(None);
}

#[allow(clippy::too_many_arguments)]
fn speak_one(
    output: EspeakOutput,
    sample_rate: u32,
    id: UtteranceId,
    text: &str,
    character: Option<char>,
    job_epoch: u64,
    events: &Sender<(UtteranceId, RawEvent)>,
    epoch: &Arc<AtomicU64>,
) {
    let offsets = char_offsets(text);
    let samples = Arc::new(AtomicU64::new(0));
    {
        let events = events.clone();
        let epoch = Arc::clone(epoch);
        let samples = Arc::clone(&samples);
        ffi::set_sink(Some(Box::new(move |wav: &[i16], evs: &[ffi::Event]| {
            samples.fetch_add(wav.len() as u64, Ordering::SeqCst);
            for e in evs.iter().filter(|e| e.kind == ffi::EventKind::Word) {
                let _ = events.send((
                    id,
                    RawEvent::Word {
                        byte_range: byte_range(&offsets, e.text_position, e.length),
                        audio_ms: u32::try_from(e.audio_ms).ok(),
                    },
                ));
            }
            epoch.load(Ordering::SeqCst) != job_epoch
        })));
    }
    let started = Instant::now();
    let _ = events.send((id, RawEvent::Started));
    let r = match character {
        Some(c) => ffi::synth_char(c),
        None => ffi::synth(text),
    };
    if r.is_ok() {
        ffi::synchronize();
    }
    ffi::set_sink(None);
    if let Err(e) = r {
        let _ = events.send((id, RawEvent::Error(e)));
        return;
    }
    if output == EspeakOutput::Virtual && sample_rate > 0 {
        // Play the discarded audio in real time: wait out its duration,
        // interruptibly.
        let secs = samples.load(Ordering::SeqCst) as f64 / f64::from(sample_rate);
        let until = started + Duration::from_secs_f64(secs);
        while Instant::now() < until {
            if epoch.load(Ordering::SeqCst) != job_epoch {
                break;
            }
            std::thread::sleep(Duration::from_millis(5).min(until - Instant::now()));
        }
    }
    let end = if epoch.load(Ordering::SeqCst) == job_epoch {
        RawEvent::Finished
    } else {
        RawEvent::Cancelled
    };
    let _ = events.send((id, end));
}

/// Synthesizes `text` in retrieval mode into a WAV file, collecting each
/// word's `audio_position` (milliseconds from the start of the audio).
fn to_file(text: &str, path: &Path, params: &VoiceParams) -> Result<FileSynthesis, SpeechError> {
    let engine = |e: String| SpeechError::Engine(e);
    let rate = ensure(ffi::Mode::Retrieval).map_err(engine)?;
    apply_params(params).map_err(engine)?;
    let pcm: Arc<Mutex<Vec<i16>>> = Arc::default();
    let words: Arc<Mutex<Vec<WordTiming>>> = Arc::default();
    {
        let pcm = Arc::clone(&pcm);
        let words = Arc::clone(&words);
        let offsets = char_offsets(text);
        ffi::set_sink(Some(Box::new(move |wav: &[i16], evs: &[ffi::Event]| {
            pcm.lock()
                .unwrap_or_else(|p| p.into_inner())
                .extend_from_slice(wav);
            let mut w = words.lock().unwrap_or_else(|p| p.into_inner());
            for e in evs.iter().filter(|e| e.kind == ffi::EventKind::Word) {
                w.push(WordTiming {
                    byte_range: byte_range(&offsets, e.text_position, e.length),
                    audio_ms: u32::try_from(e.audio_ms).unwrap_or(0),
                });
            }
            false
        })));
    }
    let r = ffi::synth(text);
    if r.is_ok() {
        ffi::synchronize();
    }
    ffi::set_sink(None);
    r.map_err(engine)?;
    let pcm = std::mem::take(&mut *pcm.lock().unwrap_or_else(|p| p.into_inner()));
    std::fs::write(path, wav_bytes(&pcm, rate)).map_err(|e| SpeechError::Io(e.to_string()))?;
    let words = std::mem::take(&mut *words.lock().unwrap_or_else(|p| p.into_inner()));
    Ok(FileSynthesis { words })
}

pub use crate::wav::wav_bytes;

impl SpeechBackend for EspeakBackend {
    fn id(&self) -> BackendId {
        "espeak"
    }

    fn capabilities(&self) -> Caps {
        CAPS
    }

    fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        Ok(self.voices.clone())
    }

    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
        if let Some(v) = &params.voice {
            let known = self
                .voices
                .iter()
                .any(|x| &x.id == v || x.name.eq_ignore_ascii_case(v) || x.languages.contains(v));
            if !known {
                // Keep the current voice, apply everything else.
                let voice = self.params.voice.take();
                self.params = VoiceParams {
                    voice,
                    ..params.clone()
                };
                return Err(SpeechError::UnknownVoice(v.clone()));
            }
        }
        self.params = params.clone();
        Ok(())
    }

    fn effective_wpm(&self) -> u16 {
        self.params
            .rate
            .wpm()
            .clamp(*RATE_RANGE.start(), *RATE_RANGE.end())
    }

    fn speak(
        &mut self,
        utterance: &Utterance,
        _sink: &mut dyn EventSink,
    ) -> Result<(), SpeechError> {
        let mut chars = utterance.text.chars();
        let character = match (utterance.kind, chars.next(), chars.next()) {
            (UtteranceKind::Character, Some(c), None) => Some(c),
            _ => None,
        };
        self.send(Job::Speak {
            id: utterance.id,
            text: utterance.text.clone(),
            character,
            params: self.params.clone(),
            epoch: self.epoch.load(Ordering::SeqCst),
        })
    }

    fn poll(&mut self, sink: &mut dyn EventSink) {
        while let Ok((id, e)) = self.events.try_recv() {
            sink.emit(id, e);
        }
    }

    fn stop(&mut self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
        if self.output == EspeakOutput::Playback {
            ffi::cancel();
        }
    }

    fn synthesize_to_file(&mut self, text: &str, path: &Path) -> Result<(), SpeechError> {
        self.write_file(text, path).map(|_| ())
    }

    fn synthesize_utterance(
        &mut self,
        utterance: &Utterance,
        path: &Path,
    ) -> Result<FileSynthesis, SpeechError> {
        self.write_file(&utterance.text, path)
    }
}

impl EspeakBackend {
    /// Synthesizes `text` into a WAV file on the worker thread.
    fn write_file(&mut self, text: &str, path: &Path) -> Result<FileSynthesis, SpeechError> {
        let (tx, rx) = mpsc::channel();
        self.send(Job::ToFile {
            text: text.to_owned(),
            path: path.to_owned(),
            params: self.params.clone(),
            reply: tx,
        })?;
        match rx.recv_timeout(Duration::from_secs(600)) {
            Ok(r) => r,
            Err(RecvTimeoutError::Timeout) => {
                Err(SpeechError::Engine("synthesis timed out".into()))
            }
            Err(RecvTimeoutError::Disconnected) => {
                Err(SpeechError::Engine("espeak worker stopped".into()))
            }
        }
    }
}

impl Drop for EspeakBackend {
    fn drop(&mut self) {
        self.stop();
        self.jobs = None;
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_positions_become_byte_ranges() {
        let t = "Café pour Émile";
        let o = char_offsets(t);
        let at = |p, l| {
            let r = byte_range(&o, p, l);
            &t[r.start as usize..r.end as usize]
        };
        // Values observed from espeak-ng 1.52 for this text.
        assert_eq!(at(1, 4), "Café");
        assert_eq!(at(6, 4), "pour");
        assert_eq!(at(11, 5), "Émile");
        // Out of range positions clamp instead of panicking.
        assert_eq!(at(0, 99), t);
        assert_eq!(at(99, 3), "");
    }

    #[test]
    fn parameter_mapping() {
        use textweaver_core::{Pitch, Rate, Volume};
        let p = |rate, pitch, vol| VoiceParams {
            rate: Rate::Wpm(rate),
            pitch: Pitch::Semitones(pitch),
            volume: Volume::new(vol),
            voice: None,
        };
        assert_eq!(espeak_values(&p(265, 0, 100)), (265, 50, 100));
        assert_eq!(espeak_values(&p(900, 12, 50)), (450, 100, 50));
        assert_eq!(espeak_values(&p(50, -12, 0)), (80, 0, 0));
        assert_eq!(espeak_values(&p(200, 6, 100)).1, 75);
    }
}
