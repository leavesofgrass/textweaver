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
//!
//! **Live** ([`RtenConfig::live`], ADR-0042): with a capture that offers
//! live audio, the worker transcribes while recording runs. Utterances
//! are found as the audio arrives, words two runs agree on arrive as
//! `Committed` events while the speaker talks, and each utterance is
//! finished at its pause rather than when recording stops (see
//! [`stream`](crate::stream)).

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::audio::{MonoAudio, read_wav, to_whisper_rate};
use crate::capture::{AudioCapture, LiveAudio, WHISPER_SAMPLE_RATE};
use crate::rten_whisper::{RtenWhisper, RtenWhisperFiles, Timings, to_f32};
use crate::stream::{ChannelFeed, Recognizer, Signals, StreamConfig, run_live, spawn_listener};
use crate::transcript::{DictationEvent, Segment, Transcript};
use crate::vad::{SpeechFinder, VadConfig, utterances};
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
    /// Transcribe while recording, when the capture offers live audio
    /// (off by default: the whole recording is transcribed after `stop`).
    /// Needs the voice detector.
    pub live: Option<StreamConfig>,
}

impl RtenConfig {
    /// A configuration for the model in `model_dir`, with the voice
    /// detector on.
    pub fn new(model_dir: impl Into<PathBuf>) -> Self {
        RtenConfig {
            model_dir: model_dir.into(),
            language: None,
            vad: Some(VadConfig::default()),
            live: None,
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
    /// Live sessions: utterances found.
    pub utterances: usize,
    /// Live sessions: recognizer runs, partial and final.
    pub runs: usize,
    /// Live sessions: partial runs cancelled at a pause.
    pub cancelled_runs: usize,
    /// Live sessions: words committed before their utterance's pause.
    pub early_words: usize,
    /// Live sessions: words committed in all.
    pub words: usize,
}

type Tagged = (u64, DictationEvent);

enum Input {
    File(PathBuf),
    Audio(Vec<f32>),
    Live {
        live: LiveAudio,
        signals: Arc<Signals>,
        config: StreamConfig,
    },
}

/// The worker thread: its job queue, its handle, and a channel that
/// disconnects when it has finished.
struct WorkerHandle {
    jobs: Sender<Job>,
    handle: JoinHandle<()>,
    done: Receiver<()>,
}

/// How long dropping a backend waits for its worker (the writer thread's
/// wait at quit is the same). A decode stops between tokens once
/// cancelled, but the encoder runs to its end: about a second, several
/// under load.
pub const SHUTDOWN_WAIT: Duration = Duration::from_secs(10);

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
    /// A live session's audio and signals.
    session: Option<(LiveAudio, Arc<Signals>)>,
    cancel: Arc<AtomicBool>,
    worker: Option<WorkerHandle>,
    tx: Sender<Tagged>,
    rx: Receiver<Tagged>,
    local: VecDeque<DictationEvent>,
    timings: Arc<Mutex<Option<SessionTimings>>>,
}

impl Drop for RtenDictation {
    fn drop(&mut self) {
        if !self.shutdown(SHUTDOWN_WAIT) {
            log::warn!(
                "dictation: the Whisper worker did not stop within {} seconds",
                SHUTDOWN_WAIT.as_secs()
            );
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
            session: None,
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

    /// Ends any session (discarding it, as [`Dictation::cancel`] does)
    /// and stops the worker, waiting at most `wait` for it. True when it
    /// stopped in time; otherwise it is left to finish on its own. For
    /// quitting and closing: nothing is lost without a `Cancelled` event.
    pub fn shutdown(&mut self, wait: Duration) -> bool {
        self.cancel();
        self.cancel.store(true, Ordering::SeqCst);
        let Some(w) = self.worker.take() else {
            return true;
        };
        drop(w.jobs);
        match w.done.recv_timeout(wait) {
            Ok(()) | Err(RecvTimeoutError::Disconnected) => {
                let _ = w.handle.join();
                true
            }
            Err(RecvTimeoutError::Timeout) => false,
        }
    }

    /// Sends a session to the worker, starting it the first time.
    fn send_job(&mut self, input: Input) -> Result<(), DictationError> {
        self.cancel = Arc::new(AtomicBool::new(false));
        let job = Job {
            generation: self.generation.load(Ordering::SeqCst),
            input,
            cancel: Arc::clone(&self.cancel),
            stopped: Instant::now(),
        };
        if self.worker.is_none() {
            let (jobs_tx, jobs_rx) = channel::<Job>();
            let (done_tx, done_rx) = channel::<()>();
            let files = self.files.clone();
            let language = self.config.language.clone();
            let vad = self.config.vad;
            let tx = self.tx.clone();
            let timings = Arc::clone(&self.timings);
            let handle = std::thread::Builder::new()
                .name("textweaver-whisper-rten".into())
                .spawn(move || {
                    // Disconnects `done` when the thread ends, however.
                    let _done = done_tx;
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
                        worker.run(job);
                    }
                })
                .map_err(|e| DictationError::Capture(e.to_string()))?;
            self.worker = Some(WorkerHandle {
                jobs: jobs_tx,
                handle,
                done: done_rx,
            });
        }
        let sent = self
            .worker
            .as_ref()
            .is_some_and(|w| w.jobs.send(job).is_ok());
        if !sent {
            self.worker = None;
            return Err(DictationError::Capture(
                "The dictation worker has stopped.".into(),
            ));
        }
        Ok(())
    }

    /// Starts transcribing recorded input.
    fn transcribe(&mut self, input: Input) -> Result<(), DictationError> {
        self.send_job(input)?;
        self.state = DictationState::Transcribing;
        self.local.push_back(DictationEvent::Transcribing);
        Ok(())
    }

    /// Blocks until the session ends and returns every event from now on
    /// (for the command line).
    pub fn wait(&mut self) -> Vec<DictationEvent> {
        let mut events = Vec::new();
        self.wait_each(|e| events.push(e.clone()));
        events
    }

    /// Blocks until the session ends, handing each event to `each` as it
    /// arrives (for the command line's live output).
    pub fn wait_each(&mut self, mut each: impl FnMut(&DictationEvent)) {
        for e in self.poll() {
            each(&e);
        }
        while self.state != DictationState::Idle {
            match self.rx.recv() {
                Ok((g, e)) if g == self.generation.load(Ordering::SeqCst) => {
                    if e.is_terminal() {
                        self.finished();
                    }
                    each(&e);
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    }

    /// The session has ended (its final event has arrived).
    fn finished(&mut self) {
        self.state = DictationState::Idle;
        self.session = None;
        // A live session can end while recording: a paced file ran out, or
        // the worker failed. The capture is released either way.
        if let Some(mut c) = self.capture.take() {
            c.cancel();
        }
    }

    /// Starts a live session on `capture`, if the configuration and the
    /// capture allow it. Hands the capture back otherwise.
    fn start_live(
        &mut self,
        mut capture: Box<dyn AudioCapture>,
    ) -> Result<Option<Box<dyn AudioCapture>>, DictationError> {
        let Some(config) = self.config.live.filter(|_| self.config.vad.is_some()) else {
            return Ok(Some(capture));
        };
        let Some(live) = capture.live() else {
            return Ok(Some(capture));
        };
        let signals = Arc::new(Signals::default());
        capture.start()?;
        let sent = self.send_job(Input::Live {
            live: live.clone(),
            signals: Arc::clone(&signals),
            config,
        });
        if let Err(e) = sent {
            capture.cancel();
            return Err(e);
        }
        self.capture = Some(capture);
        self.session = Some((live, signals));
        self.state = DictationState::Recording;
        self.local.push_back(DictationEvent::Recording);
        Ok(None)
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
                self.transcribe(Input::File(path))
            }
            DictationInput::Capture(capture) => {
                let Some(mut capture) = self.start_live(capture)? else {
                    return Ok(());
                };
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
        let pcm = capture.stop();
        if let Some((live, signals)) = &self.session {
            // The worker has the audio already; it finishes what is left.
            live.end();
            if let Err(e) = pcm {
                signals.cancel();
                self.session = None;
                self.generation.fetch_add(1, Ordering::SeqCst);
                self.state = DictationState::Idle;
                self.local.push_back(DictationEvent::Failed {
                    message: format!("Recording failed: {e}"),
                });
                return Ok(());
            }
            self.state = DictationState::Transcribing;
            self.local.push_back(DictationEvent::Transcribing);
            return Ok(());
        }
        let pcm = match pcm {
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
                message: NO_AUDIO.to_owned(),
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
        self.transcribe(Input::Audio(audio))
    }

    fn cancel(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some(mut c) = self.capture.take() {
            c.cancel();
        }
        if let Some((live, signals)) = self.session.take() {
            signals.cancel();
            live.end();
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
                self.finished();
            }
            out.push(e);
        }
        out
    }

    fn state(&self) -> DictationState {
        self.state
    }
}

/// Said when a recording has no audio at all.
const NO_AUDIO: &str = "No audio was recorded. Check your microphone.";

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

/// Whisper as the live loop's recognizer, adding up its timings.
struct WhisperRecognizer<'a> {
    model: &'a RtenWhisper,
    language: Option<&'a str>,
    timings: Timings,
    runs: usize,
}

impl Recognizer for WhisperRecognizer<'_> {
    fn recognize(&mut self, audio: &[f32], cancel: &AtomicBool) -> Result<Vec<Segment>, String> {
        self.runs += 1;
        let (t, m) = self
            .model
            .transcribe(audio, self.language, cancel, &mut |_| {})
            .map_err(|e| e.to_string())?;
        self.timings.features += m.features;
        self.timings.encode += m.encode;
        self.timings.decode += m.decode;
        self.timings.audio += m.audio;
        Ok(t.segments)
    }
}

impl Worker {
    fn run(&mut self, job: Job) {
        let generation = job.generation;
        let event = match job.input {
            Input::Live {
                live,
                signals,
                config,
            } => match self.live(generation, live, &signals, &config) {
                Ok(Some(e)) => e,
                // Cancelled: cancel() already said so.
                Ok(None) => return,
                Err(message) => DictationEvent::Failed { message },
            },
            input => match self.transcribe(&job.cancel, job.stopped, input, generation) {
                Ok(t) => DictationEvent::Final(t),
                Err(message) => DictationEvent::Failed { message },
            },
        };
        let _ = self.tx.send((generation, event));
    }

    fn load(&mut self, timings: &mut SessionTimings) -> Result<(), String> {
        if self.model.is_none() {
            let t = Instant::now();
            self.model = Some(RtenWhisper::load(&self.files).map_err(|e| e.to_string())?);
            timings.load = t.elapsed();
        }
        Ok(())
    }

    /// A live session: the listener finds utterances while the loop
    /// transcribes them. Returns the final event, or `None` when
    /// cancelled.
    fn live(
        &mut self,
        generation: u64,
        live: LiveAudio,
        signals: &Arc<Signals>,
        config: &StreamConfig,
    ) -> Result<Option<DictationEvent>, String> {
        let mut timings = SessionTimings::default();
        let vad = self.vad.unwrap_or_default();
        let (rx, listener) =
            spawn_listener(live, Box::new(SpeechFinder::new(vad)), Arc::clone(signals))
                .map_err(|e| format!("Dictation could not start listening: {e}"))?;
        let mut feed = ChannelFeed::new(rx);
        // The listener keeps finding speech while the model loads.
        let loaded = self.load(&mut timings);
        let result = loaded.and_then(|()| {
            let Some(model) = &self.model else {
                return Err("The Whisper model is not loaded.".to_owned());
            };
            let mut rec = WhisperRecognizer {
                model,
                language: self.language.as_deref(),
                timings: Timings::default(),
                runs: 0,
            };
            let tx = &self.tx;
            let report = run_live(&mut rec, &mut feed, signals, config, &mut |e| {
                let _ = tx.send((generation, e));
            })?;
            Ok(report.map(|r| (r, rec.timings, rec.runs)))
        });
        // However it ended, the listener stops and is joined.
        let finished = matches!(result, Ok(Some(_)));
        if !finished {
            signals.cancel();
        }
        drop(feed);
        let _ = listener.join();
        let Some((report, model, runs)) = result? else {
            return Ok(None);
        };
        if report.samples == 0 {
            return Ok(Some(DictationEvent::Failed {
                message: NO_AUDIO.to_owned(),
            }));
        }
        timings.model = model;
        timings.runs = runs;
        timings.utterances = report.utterances;
        timings.cancelled_runs = report.cancelled_runs;
        timings.early_words = report.early_words;
        timings.words = report.words;
        timings.speech = report
            .transcript
            .segments
            .iter()
            .map(|s| Duration::from_millis(s.end_ms.saturating_sub(s.start_ms)))
            .sum();
        timings.latency = report
            .ended_at
            .map_or(Duration::ZERO, |end| report.finished_at.saturating_sub(end));
        log::info!("whisper (rten, live): {timings:?}");
        *self.timings.lock().unwrap_or_else(|e| e.into_inner()) = Some(timings);
        Ok(Some(DictationEvent::Final(report.transcript)))
    }

    fn transcribe(
        &mut self,
        cancel: &AtomicBool,
        stopped: Instant,
        input: Input,
        generation: u64,
    ) -> Result<Transcript, String> {
        let audio = match input {
            Input::File(p) => {
                let bytes = std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
                to_whisper_rate(&read_wav(&bytes)?)?
            }
            Input::Audio(a) => a,
            Input::Live { .. } => return Err("A live session reached the batch path.".into()),
        };
        let mut timings = SessionTimings::default();
        self.load(&mut timings)?;
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
            if cancel.load(Ordering::SeqCst) {
                break;
            }
            timings.speech += Duration::from_millis(span.len() as u64 * 1000 / rate);
            let offset = span.start as u64 * 1000 / rate;
            let tx = &self.tx;
            let mut on_segment = |s: &Segment| {
                let _ = tx.send((
                    generation,
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
                    cancel,
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
        timings.latency = stopped.elapsed();
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

    /// Live dictation with a real model over the speech fixtures, played
    /// in at speaking pace (never aloud), against the same audio
    /// transcribed at once. Needs `TEXTWEAVER_WHISPER_RTEN_MODEL` (a model
    /// folder) and the fixtures' WAV files (`fixtures/d/README.md`);
    /// prints what was committed when, and the timings. Run with
    /// `--release`.
    #[test]
    #[ignore = "needs a Whisper model and the fixture recordings"]
    fn live_on_the_fixtures() {
        use crate::capture::PacedCapture;
        let Some(dir) = std::env::var_os("TEXTWEAVER_WHISPER_RTEN_MODEL") else {
            return;
        };
        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/d");
        let mut config = RtenConfig::new(PathBuf::from(dir));
        let mut batch = RtenDictation::new(config.clone()).unwrap();
        config.live = Some(StreamConfig::default());
        let mut live = RtenDictation::new(config).unwrap();
        for name in ["stream-note", "stream-long", "stream-short"] {
            let wav = fixtures.join(format!("{name}.wav"));
            let Ok(bytes) = std::fs::read(&wav) else {
                eprintln!("{name}: no recording, skipped");
                continue;
            };
            batch.start(DictationInput::File(wav)).unwrap();
            let batch_text = batch
                .wait()
                .iter()
                .find_map(|e| match e {
                    DictationEvent::Final(t) => Some(t.text()),
                    _ => None,
                })
                .unwrap();
            let samples = to_whisper_rate(&read_wav(&bytes).unwrap()).unwrap();
            let seconds = samples.len() as f64 / 16_000.0;
            let t0 = Instant::now();
            live.start(DictationInput::Capture(Box::new(PacedCapture::new(
                samples,
            ))))
            .unwrap();
            let mut text = None;
            live.wait_each(|e| match e {
                DictationEvent::Committed { text, at_pause, .. } => eprintln!(
                    "{name}: at {:.1} s{}: {text}",
                    t0.elapsed().as_secs_f64(),
                    if *at_pause { " (pause)" } else { "" }
                ),
                DictationEvent::Final(t) => text = Some(t.text()),
                DictationEvent::Failed { message } => panic!("{message}"),
                _ => {}
            });
            let text = text.unwrap();
            eprintln!(
                "{name}: {seconds:.1} s of audio, done at {:.1} s",
                t0.elapsed().as_secs_f64()
            );
            eprintln!("{name}: live:  {text}");
            eprintln!("{name}: batch: {batch_text}");
            eprintln!("{name}: {:?}", live.last_timings());
            assert!(!text.is_empty());
        }
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
