//! In-process dictation: Whisper on RTen behind the [`Dictation`] trait
//! (feature `rten`, ADR-0023).
//!
//! A session runs on a worker thread, as with the Whisper programs: the
//! audio (a WAV file, or a capture's recording) is mixed to mono and
//! resampled to 16 kHz, split into utterances by the voice detector
//! (earshot) so silence is skipped, and each utterance is transcribed;
//! each finished segment arrives as a `Partial` event and the whole
//! transcript as `Final`. The model loads on the first session and stays
//! loaded, so later sessions start at once.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::audio::{MonoAudio, read_wav, to_whisper_rate};
use crate::capture::{AudioCapture, WHISPER_SAMPLE_RATE};
use crate::rten_whisper::{RtenWhisper, RtenWhisperFiles, Timings, to_f32};
use crate::transcript::{DictationEvent, Segment, Transcript};
use crate::vad::{VadConfig, utterances};
use crate::{Dictation, DictationError, DictationInput, DictationState};

/// How to run in-process Whisper.
#[derive(Clone, Debug, PartialEq)]
pub struct RtenConfig {
    /// The model's folder (`<data>/whisper/rten/base.en`).
    pub model_dir: PathBuf,
    /// The spoken language for multilingual models; detected when `None`.
    pub language: Option<String>,
    /// Skip silence with the voice detector (on by default).
    pub vad: Option<VadConfig>,
}

impl RtenConfig {
    /// A configuration for the model in `model_dir`, with the voice
    /// detector on.
    pub fn new(model_dir: impl Into<PathBuf>) -> Self {
        RtenConfig {
            model_dir: model_dir.into(),
            language: None,
            vad: Some(VadConfig::default()),
        }
    }
}

/// What one session measured, for `tw dictate --timings`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SessionTimings {
    /// Loading the model (zero when it was already loaded).
    pub load: Duration,
    /// From the end of the input to the final transcript.
    pub latency: Duration,
    /// The model's own steps.
    pub model: Timings,
    /// Speech found by the voice detector.
    pub speech: Duration,
}

type Tagged = (u64, DictationEvent);

enum Input {
    File(PathBuf),
    Audio(Vec<f32>),
}

/// Whisper on RTen.
///
/// One worker thread owns the model (its tokenizer cannot move between
/// threads) for the life of the backend and takes sessions in turn.
pub struct RtenDictation {
    config: RtenConfig,
    files: RtenWhisperFiles,
    state: DictationState,
    generation: Arc<AtomicU64>,
    capture: Option<Box<dyn AudioCapture>>,
    cancel: Arc<AtomicBool>,
    worker: Option<(Sender<Job>, JoinHandle<()>)>,
    tx: Sender<Tagged>,
    rx: Receiver<Tagged>,
    local: VecDeque<DictationEvent>,
    timings: Arc<Mutex<Option<SessionTimings>>>,
}

impl Drop for RtenDictation {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some((jobs, handle)) = self.worker.take() {
            drop(jobs);
            let _ = handle.join();
        }
    }
}

impl std::fmt::Debug for RtenDictation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtenDictation")
            .field("config", &self.config)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl RtenDictation {
    /// A backend for the model `config` names. Fails at once when its
    /// files are missing.
    pub fn new(config: RtenConfig) -> Result<Self, DictationError> {
        let files = RtenWhisperFiles::in_dir(&config.model_dir)?;
        let (tx, rx) = channel();
        Ok(RtenDictation {
            config,
            files,
            state: DictationState::Idle,
            generation: Arc::new(AtomicU64::new(0)),
            capture: None,
            cancel: Arc::new(AtomicBool::new(false)),
            worker: None,
            tx,
            rx,
            local: VecDeque::new(),
            timings: Arc::new(Mutex::new(None)),
        })
    }

    /// The last finished session's timings.
    pub fn last_timings(&self) -> Option<SessionTimings> {
        *self.timings.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn spawn(&mut self, input: Input) -> Result<(), DictationError> {
        self.cancel = Arc::new(AtomicBool::new(false));
        let job = Job {
            generation: self.generation.load(Ordering::SeqCst),
            input,
            cancel: Arc::clone(&self.cancel),
            stopped: Instant::now(),
        };
        if self.worker.is_none() {
            let (jobs_tx, jobs_rx) = channel::<Job>();
            let files = self.files.clone();
            let language = self.config.language.clone();
            let vad = self.config.vad;
            let tx = self.tx.clone();
            let timings = Arc::clone(&self.timings);
            let handle = std::thread::Builder::new()
                .name("textweaver-whisper-rten".into())
                .spawn(move || {
                    // Built here: the model never leaves this thread.
                    let mut worker = Worker {
                        files,
                        language,
                        vad,
                        model: None,
                        tx,
                        timings,
                    };
                    for job in jobs_rx {
                        worker.run(&job);
                    }
                })
                .map_err(|e| DictationError::Capture(e.to_string()))?;
            self.worker = Some((jobs_tx, handle));
        }
        let sent = self
            .worker
            .as_ref()
            .is_some_and(|(jobs, _)| jobs.send(job).is_ok());
        if !sent {
            self.worker = None;
            return Err(DictationError::Capture(
                "The dictation worker has stopped.".into(),
            ));
        }
        self.state = DictationState::Transcribing;
        self.local.push_back(DictationEvent::Transcribing);
        Ok(())
    }

    /// Blocks until the session ends and returns every event from now on
    /// (for the command line).
    pub fn wait(&mut self) -> Vec<DictationEvent> {
        let mut events = self.poll();
        while self.state != DictationState::Idle {
            match self.rx.recv() {
                Ok((g, e)) if g == self.generation.load(Ordering::SeqCst) => {
                    if e.is_terminal() {
                        self.state = DictationState::Idle;
                    }
                    events.push(e);
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        events
    }
}

impl Dictation for RtenDictation {
    fn name(&self) -> &str {
        "Whisper (in-process)"
    }

    fn start(&mut self, input: DictationInput) -> Result<(), DictationError> {
        if self.state != DictationState::Idle {
            return Err(DictationError::Busy);
        }
        self.generation.fetch_add(1, Ordering::SeqCst);
        match input {
            DictationInput::File(path) => {
                if !path.is_file() {
                    return Err(DictationError::Io {
                        source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
                        path,
                    });
                }
                self.spawn(Input::File(path))
            }
            DictationInput::Capture(mut capture) => {
                capture.start()?;
                self.capture = Some(capture);
                self.state = DictationState::Recording;
                self.local.push_back(DictationEvent::Recording);
                Ok(())
            }
        }
    }

    fn stop(&mut self) -> Result<(), DictationError> {
        if self.state != DictationState::Recording {
            return Ok(());
        }
        let Some(mut capture) = self.capture.take() else {
            self.state = DictationState::Idle;
            return Ok(());
        };
        let pcm = match capture.stop() {
            Ok(p) => p,
            Err(e) => {
                self.state = DictationState::Idle;
                self.local.push_back(DictationEvent::Failed {
                    message: format!("Recording failed: {e}"),
                });
                return Ok(());
            }
        };
        if pcm.is_empty() {
            self.state = DictationState::Idle;
            self.local.push_back(DictationEvent::Failed {
                message: "No audio was recorded. Check your microphone.".to_owned(),
            });
            return Ok(());
        }
        let audio = if pcm.sample_rate == WHISPER_SAMPLE_RATE {
            to_f32(&pcm.samples)
        } else {
            let mono = MonoAudio {
                sample_rate: pcm.sample_rate,
                samples: to_f32(&pcm.samples),
            };
            to_whisper_rate(&mono).map_err(DictationError::Capture)?
        };
        self.spawn(Input::Audio(audio))
    }

    fn cancel(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some(mut c) = self.capture.take() {
            c.cancel();
        }
        if self.state != DictationState::Idle {
            self.generation.fetch_add(1, Ordering::SeqCst);
            self.state = DictationState::Idle;
            self.local.push_back(DictationEvent::Cancelled);
        }
    }

    fn poll(&mut self) -> Vec<DictationEvent> {
        let mut out: Vec<DictationEvent> = self.local.drain(..).collect();
        let current = self.generation.load(Ordering::SeqCst);
        while let Ok((g, e)) = self.rx.try_recv() {
            if g != current {
                continue;
            }
            if e.is_terminal() {
                self.state = DictationState::Idle;
            }
            out.push(e);
        }
        out
    }

    fn state(&self) -> DictationState {
        self.state
    }
}

/// One session for the worker.
struct Job {
    generation: u64,
    input: Input,
    cancel: Arc<AtomicBool>,
    stopped: Instant,
}

/// The worker thread's state: the model, loaded on the first session.
struct Worker {
    files: RtenWhisperFiles,
    language: Option<String>,
    vad: Option<VadConfig>,
    model: Option<RtenWhisper>,
    tx: Sender<Tagged>,
    timings: Arc<Mutex<Option<SessionTimings>>>,
}

impl Worker {
    fn run(&mut self, job: &Job) {
        let event = match self.transcribe(job) {
            Ok(t) => DictationEvent::Final(t),
            Err(message) => DictationEvent::Failed { message },
        };
        let _ = self.tx.send((job.generation, event));
    }

    fn transcribe(&mut self, job: &Job) -> Result<Transcript, String> {
        let audio = match &job.input {
            Input::File(p) => {
                let bytes = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
                to_whisper_rate(&read_wav(&bytes)?)?
            }
            Input::Audio(a) => a.clone(),
        };
        let mut timings = SessionTimings::default();
        if self.model.is_none() {
            let t = Instant::now();
            self.model = Some(RtenWhisper::load(&self.files).map_err(|e| e.to_string())?);
            timings.load = t.elapsed();
        }
        let Some(model) = &self.model else {
            return Err("The Whisper model is not loaded.".into());
        };
        let spans = match &self.vad {
            Some(c) => utterances(&audio, c),
            None => std::iter::once(0..audio.len()).collect(),
        };
        let rate = u64::from(WHISPER_SAMPLE_RATE);
        let mut all = Transcript::default();
        for span in spans {
            if job.cancel.load(Ordering::SeqCst) {
                break;
            }
            timings.speech += Duration::from_millis(span.len() as u64 * 1000 / rate);
            let offset = span.start as u64 * 1000 / rate;
            let tx = &self.tx;
            let mut on_segment = |s: &Segment| {
                let _ = tx.send((
                    job.generation,
                    DictationEvent::Partial(Segment {
                        start_ms: s.start_ms + offset,
                        end_ms: s.end_ms + offset,
                        text: s.text.clone(),
                    }),
                ));
            };
            let (t, m) = model
                .transcribe(
                    &audio[span],
                    self.language.as_deref(),
                    &job.cancel,
                    &mut on_segment,
                )
                .map_err(|e| e.to_string())?;
            timings.model.features += m.features;
            timings.model.encode += m.encode;
            timings.model.decode += m.decode;
            timings.model.audio += m.audio;
            all.segments.extend(t.segments.into_iter().map(|s| Segment {
                start_ms: s.start_ms + offset,
                end_ms: s.end_ms + offset,
                text: s.text,
            }));
        }
        timings.latency = job.stopped.elapsed();
        log::info!("whisper (rten): {timings:?}");
        *self.timings.lock().unwrap_or_else(|e| e.into_inner()) = Some(timings);
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_models_fail_at_once() {
        let tmp = tempfile::tempdir().unwrap();
        let e = RtenDictation::new(RtenConfig::new(tmp.path().join("base.en")))
            .unwrap_err()
            .to_string();
        assert!(e.contains("encoder_model_int8.onnx"), "{e}");
    }

    /// Transcribes a WAV with a real model. Needs
    /// `TEXTWEAVER_WHISPER_RTEN_MODEL` (a model folder) and
    /// `TEXTWEAVER_WHISPER_RTEN_WAV` (16-bit speech); prints the timings.
    #[test]
    #[ignore = "needs a Whisper model and a recording"]
    fn transcribes_a_recording() {
        let (Some(dir), Some(wav)) = (
            std::env::var_os("TEXTWEAVER_WHISPER_RTEN_MODEL"),
            std::env::var_os("TEXTWEAVER_WHISPER_RTEN_WAV"),
        ) else {
            return;
        };
        let mut d = RtenDictation::new(RtenConfig::new(PathBuf::from(dir))).unwrap();
        for run in 0..2 {
            d.start(DictationInput::File(PathBuf::from(&wav))).unwrap();
            let events = d.wait();
            let text = events
                .iter()
                .find_map(|e| match e {
                    DictationEvent::Final(t) => Some(t.text()),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{events:?}"));
            eprintln!("run {run}: {text}");
            eprintln!("run {run}: {:?}", d.last_timings());
            assert!(!text.is_empty());
        }
    }
}
