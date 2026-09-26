//! The Whisper subprocess backend.
//!
//! Each session runs on a worker thread: it converts the audio if the
//! engine needs it (whisper.cpp takes 16 kHz WAV; `ffmpeg` converts
//! anything else), runs the Whisper program, turns each printed segment
//! into a [`DictationEvent::Partial`], and reads the JSON transcript for the
//! [`DictationEvent::Final`]. Events reach the caller through
//! [`Dictation::poll`], which never blocks (the same shape as the speech
//! backends, ADR-0003).
//!
//! Cancelling kills the process and reports [`DictationEvent::Cancelled`]
//! at once; anything the worker sends afterwards belongs to an old session
//! and is dropped (a generation counter, as in the speech service).

use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::capture::{AudioCapture, WHISPER_SAMPLE_RATE};
use crate::engine::{self, DetectedEngine, WhisperEngine, find_on_path};
use crate::transcript::{DictationEvent, Transcript, WhisperModel};
use crate::{Dictation, DictationError, DictationInput, DictationState};

/// How to run Whisper.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhisperConfig {
    /// Which program.
    pub engine: WhisperEngine,
    /// The program's path.
    pub program: PathBuf,
    /// The model size.
    pub model: WhisperModel,
    /// whisper.cpp's model file; found in [`model_dirs`](Self::model_dirs)
    /// when `None`. Ignored by the other engines, which fetch and cache
    /// their own models.
    pub model_file: Option<PathBuf>,
    /// Extra folders to search for whisper.cpp model files.
    pub model_dirs: Vec<PathBuf>,
    /// The spoken language (`en`, `de`, ...); `None` lets Whisper detect it.
    pub language: Option<String>,
    /// Threads for Whisper; `None` for its default.
    pub threads: Option<u32>,
    /// `ffmpeg`, for converting audio whisper.cpp cannot read; found on
    /// `PATH` when `None`.
    pub ffmpeg: Option<PathBuf>,
    /// Extra environment variables for the Whisper process.
    pub env: Vec<(OsString, OsString)>,
    /// Where temporary files go; the system's temporary folder when `None`.
    pub work_dir: Option<PathBuf>,
}

impl WhisperConfig {
    /// A configuration for `program`, run as `engine`, with `model`.
    pub fn new(engine: WhisperEngine, program: PathBuf, model: WhisperModel) -> Self {
        WhisperConfig {
            engine,
            program,
            model,
            model_file: None,
            model_dirs: Vec::new(),
            language: None,
            threads: None,
            ffmpeg: None,
            env: Vec::new(),
            work_dir: None,
        }
    }

    /// A configuration for the first Whisper program found (see
    /// [`engine::detect`]), optionally only of `engine`.
    pub fn detect(
        engine: Option<WhisperEngine>,
        model: WhisperModel,
    ) -> Result<Self, DictationError> {
        let found: Vec<DetectedEngine> = engine::detect();
        let pick = found
            .into_iter()
            .find(|d| engine.is_none_or(|e| e == d.engine))
            .ok_or(DictationError::NoWhisper)?;
        Ok(WhisperConfig::new(pick.engine, pick.program, model))
    }

    /// The whisper.cpp model file to use.
    pub fn resolve_model_file(&self) -> Result<PathBuf, DictationError> {
        if let Some(f) = &self.model_file {
            return if f.is_file() {
                Ok(f.clone())
            } else {
                Err(DictationError::ModelNotFound {
                    model: self.model.as_str().to_owned(),
                    file: f.to_string_lossy().into_owned(),
                    searched: Vec::new(),
                })
            };
        }
        let dirs = engine::model_dirs(&self.program, &self.model_dirs);
        engine::find_model(self.model, &dirs)
    }
}

/// Distinguishes the temporary folders of concurrent sessions.
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A fresh temporary folder for one session.
fn make_work_dir(base: Option<&Path>) -> Result<PathBuf, DictationError> {
    let base = base.map_or_else(std::env::temp_dir, Path::to_owned);
    let n = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let dir = base.join(format!(
        "textweaver-dictation-{}-{n}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).map_err(|source| DictationError::Io {
        path: dir.clone(),
        source,
    })?;
    Ok(dir)
}

/// An event tagged with the session it belongs to.
type Tagged = (u64, DictationEvent);

/// What the worker needs.
struct Job {
    generation: u64,
    config: WhisperConfig,
    model_file: Option<PathBuf>,
    input: PathBuf,
    work_dir: PathBuf,
    cancel: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
    tx: Sender<Tagged>,
}

/// Dictation through a Whisper program.
pub struct WhisperDictation {
    config: WhisperConfig,
    state: DictationState,
    generation: u64,
    capture: Option<Box<dyn AudioCapture>>,
    cancel: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
    worker: Option<JoinHandle<()>>,
    tx: Sender<Tagged>,
    rx: Receiver<Tagged>,
    /// Events produced on the caller's thread, delivered by the next poll.
    local: VecDeque<DictationEvent>,
}

impl std::fmt::Debug for WhisperDictation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WhisperDictation")
            .field("config", &self.config)
            .field("state", &self.state)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl WhisperDictation {
    /// A backend running Whisper as `config` says.
    pub fn new(config: WhisperConfig) -> Self {
        let (tx, rx) = channel();
        WhisperDictation {
            config,
            state: DictationState::Idle,
            generation: 0,
            capture: None,
            cancel: Arc::new(AtomicBool::new(false)),
            child: Arc::new(Mutex::new(None)),
            worker: None,
            tx,
            rx,
            local: VecDeque::new(),
        }
    }

    /// The configuration.
    pub fn config(&self) -> &WhisperConfig {
        &self.config
    }

    /// Starts transcribing `input` on a worker thread.
    fn spawn_job(&mut self, input: PathBuf, work_dir: PathBuf) -> Result<(), DictationError> {
        let model_file = match self.config.engine {
            WhisperEngine::Cpp => Some(self.config.resolve_model_file()?),
            _ => None,
        };
        self.cancel = Arc::new(AtomicBool::new(false));
        self.child = Arc::new(Mutex::new(None));
        let job = Job {
            generation: self.generation,
            config: self.config.clone(),
            model_file,
            input,
            work_dir,
            cancel: Arc::clone(&self.cancel),
            child: Arc::clone(&self.child),
            tx: self.tx.clone(),
        };
        let handle = std::thread::Builder::new()
            .name("textweaver-whisper".into())
            .spawn(move || run_job(job))
            .map_err(|e| DictationError::Spawn {
                program: PathBuf::from("worker thread"),
                message: e.to_string(),
            })?;
        self.worker = Some(handle);
        self.state = DictationState::Transcribing;
        self.local.push_back(DictationEvent::Transcribing);
        Ok(())
    }

    /// Kills the running Whisper process, if any.
    fn kill(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        let mut slot = self.child.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(child) = slot.as_mut() {
            let _ = child.kill();
        }
    }

    /// Blocks until the current session ends, returning every event from
    /// now to the end (for command-line use).
    pub fn wait(&mut self) -> Vec<DictationEvent> {
        let mut events: Vec<DictationEvent> = self.poll();
        while self.state != DictationState::Idle {
            match self.rx.recv() {
                Ok((generation, event)) if generation == self.generation => {
                    if event.is_terminal() {
                        self.state = DictationState::Idle;
                    }
                    events.push(event);
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        events
    }
}

impl Dictation for WhisperDictation {
    fn name(&self) -> &str {
        self.config.engine.display_name()
    }

    fn start(&mut self, input: DictationInput) -> Result<(), DictationError> {
        if self.state != DictationState::Idle {
            return Err(DictationError::Busy);
        }
        self.generation += 1;
        match input {
            DictationInput::File(path) => {
                if !path.is_file() {
                    return Err(DictationError::Io {
                        source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
                        path,
                    });
                }
                let work_dir = make_work_dir(self.config.work_dir.as_deref())?;
                if let Err(e) = self.spawn_job(path, work_dir.clone()) {
                    let _ = std::fs::remove_dir_all(&work_dir);
                    return Err(e);
                }
                Ok(())
            }
            DictationInput::Capture(mut capture) => {
                if self.config.engine == WhisperEngine::Cpp {
                    // Fail before recording, not after the student talked.
                    self.config.resolve_model_file()?;
                }
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
            // Transcribing a file already has all its input.
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
        let work_dir = match make_work_dir(self.config.work_dir.as_deref()) {
            Ok(d) => d,
            Err(e) => {
                self.state = DictationState::Idle;
                return Err(e);
            }
        };
        let wav = work_dir.join("dictation.wav");
        if let Err(e) = pcm.write_wav(&wav) {
            let _ = std::fs::remove_dir_all(&work_dir);
            self.state = DictationState::Idle;
            return Err(e);
        }
        if let Err(e) = self.spawn_job(wav, work_dir.clone()) {
            let _ = std::fs::remove_dir_all(&work_dir);
            self.state = DictationState::Idle;
            self.local.push_back(DictationEvent::Failed {
                message: e.to_string(),
            });
        }
        Ok(())
    }

    fn cancel(&mut self) {
        match self.state {
            DictationState::Idle => {}
            DictationState::Recording => {
                if let Some(mut c) = self.capture.take() {
                    c.cancel();
                }
                self.local.push_back(DictationEvent::Cancelled);
            }
            DictationState::Transcribing => {
                self.kill();
                self.local.push_back(DictationEvent::Cancelled);
            }
        }
        self.state = DictationState::Idle;
        // Anything the old worker still sends is stale.
        self.generation += 1;
    }

    fn poll(&mut self) -> Vec<DictationEvent> {
        let mut out: Vec<DictationEvent> = self.local.drain(..).collect();
        while let Ok((generation, event)) = self.rx.try_recv() {
            if generation != self.generation {
                continue;
            }
            if event.is_terminal() {
                self.state = DictationState::Idle;
            }
            out.push(event);
        }
        out
    }

    fn state(&self) -> DictationState {
        self.state
    }
}

impl Drop for WhisperDictation {
    fn drop(&mut self) {
        if self.state == DictationState::Transcribing {
            self.kill();
        }
        if let Some(mut c) = self.capture.take() {
            c.cancel();
        }
        // The worker ends on its own once the process is gone; it is not
        // joined, so dropping never blocks.
        drop(self.worker.take());
    }
}

/// Hides the console window a subprocess would flash on Windows.
fn quiet(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

/// A running child's stdout, and the thread collecting its last stderr
/// lines.
type ChildPipes = (std::process::ChildStdout, JoinHandle<VecDeque<String>>);

/// Runs `cmd` as the session's child, so cancel can kill it. Returns its
/// pipes, or `None` when cancelled before it started.
fn spawn_child(job: &Job, mut cmd: Command, program: &Path) -> Result<Option<ChildPipes>, String> {
    quiet(&mut cmd);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut slot = job.child.lock().unwrap_or_else(|e| e.into_inner());
    if job.cancel.load(Ordering::SeqCst) {
        return Ok(None);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Could not start {}: {e}", program.display()))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    *slot = Some(child);
    drop(slot);
    let err_tail = std::thread::spawn(move || {
        let mut tail = VecDeque::new();
        if let Some(mut stderr) = stderr {
            let mut buf = Vec::new();
            let _ = stderr.read_to_end(&mut buf);
            for line in String::from_utf8_lossy(&buf).lines() {
                if !line.trim().is_empty() {
                    tail.push_back(line.trim().to_owned());
                    if tail.len() > 12 {
                        tail.pop_front();
                    }
                }
            }
        }
        tail
    });
    match stdout {
        Some(out) => Ok(Some((out, err_tail))),
        None => Err("the Whisper process has no output".to_owned()),
    }
}

/// Waits for the session's child to exit. Returns its success.
fn wait_child(job: &Job) -> bool {
    let child = job.child.lock().unwrap_or_else(|e| e.into_inner()).take();
    match child {
        Some(mut c) => c.wait().is_ok_and(|s| s.success()),
        None => false,
    }
}

/// The last useful stderr line, for an error message.
fn describe_failure(tail: &VecDeque<String>) -> String {
    tail.iter()
        .rev()
        .find(|l| {
            let lower = l.to_lowercase();
            lower.contains("error") || lower.contains("failed") || lower.contains("exception")
        })
        .or_else(|| tail.back())
        .cloned()
        .unwrap_or_else(|| "it stopped without saying why".to_owned())
}

/// Converts `input` to 16 kHz mono WAV with ffmpeg for whisper.cpp.
fn convert_for_cpp(job: &Job) -> Result<Option<PathBuf>, String> {
    let is_wav = job
        .input
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("wav"));
    if is_wav {
        return Ok(Some(job.input.clone()));
    }
    let ffmpeg = job
        .config
        .ffmpeg
        .clone()
        .or_else(|| find_on_path("ffmpeg", &std::env::var_os("PATH").unwrap_or_default()))
        .ok_or_else(|| {
            "whisper.cpp reads WAV files only, and ffmpeg was not found to convert this one. Install ffmpeg, or convert the file to WAV first.".to_owned()
        })?;
    let out = job.work_dir.join("input.wav");
    let mut cmd = Command::new(&ffmpeg);
    cmd.arg("-nostdin")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-y")
        .arg("-i")
        .arg(&job.input)
        .arg("-ar")
        .arg(WHISPER_SAMPLE_RATE.to_string())
        .arg("-ac")
        .arg("1")
        .arg("-c:a")
        .arg("pcm_s16le")
        .arg(&out);
    let Some((mut stdout, err_tail)) = spawn_child(job, cmd, &ffmpeg)? else {
        return Ok(None);
    };
    let mut sink = Vec::new();
    let _ = stdout.read_to_end(&mut sink);
    let ok = wait_child(job);
    let tail = err_tail.join().unwrap_or_default();
    if job.cancel.load(Ordering::SeqCst) {
        return Ok(None);
    }
    if !ok {
        return Err(format!(
            "The audio file could not be converted: {}",
            describe_failure(&tail)
        ));
    }
    Ok(Some(out))
}

/// The worker: one transcription, start to finish.
fn run_job(job: Job) {
    let send = |event: DictationEvent| {
        let _ = job.tx.send((job.generation, event));
    };
    let result = transcribe(&job, &send);
    // Clean up before the final event, so whoever sees it sees no session
    // folder left behind.
    let _ = std::fs::remove_dir_all(&job.work_dir);
    if job.cancel.load(Ordering::SeqCst) {
        // cancel() already reported it.
    } else {
        match result {
            Ok(transcript) => send(DictationEvent::Final(transcript)),
            Err(message) => send(DictationEvent::Failed { message }),
        }
    }
}

fn transcribe(job: &Job, send: &dyn Fn(DictationEvent)) -> Result<Transcript, String> {
    let input = if job.config.engine == WhisperEngine::Cpp {
        match convert_for_cpp(job)? {
            Some(p) => p,
            None => return Ok(Transcript::default()),
        }
    } else {
        job.input.clone()
    };
    let inv = engine::invocation(
        job.config.engine,
        job.config.model,
        job.model_file.as_deref(),
        job.config.language.as_deref(),
        job.config.threads,
        &input,
        &job.work_dir,
    );
    let mut cmd = Command::new(&job.config.program);
    cmd.args(&inv.args);
    // Python programs: UTF-8 output whatever the console code page.
    cmd.env("PYTHONIOENCODING", "utf-8").env("PYTHONUTF8", "1");
    for (k, v) in &job.config.env {
        cmd.env(k, v);
    }
    let Some((stdout, err_tail)) = spawn_child(job, cmd, &job.config.program)? else {
        return Ok(Transcript::default());
    };
    let mut printed = Transcript::default();
    let mut reader = BufReader::new(stdout);
    let mut raw = Vec::new();
    loop {
        raw.clear();
        match reader.read_until(b'\n', &mut raw) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let line = String::from_utf8_lossy(&raw);
                if let Some(seg) = engine::parse_segment_line(&line) {
                    if !job.cancel.load(Ordering::SeqCst) {
                        send(DictationEvent::Partial(seg.clone()));
                    }
                    printed.segments.push(seg);
                }
            }
        }
    }
    let ok = wait_child(job);
    let tail = err_tail.join().unwrap_or_default();
    if job.cancel.load(Ordering::SeqCst) {
        return Ok(Transcript::default());
    }
    if !ok {
        return Err(format!("Whisper failed: {}", describe_failure(&tail)));
    }
    match std::fs::read_to_string(&inv.json) {
        Ok(text) => match engine::parse_json(&text) {
            Ok(t) => Ok(t),
            Err(e) if printed.segments.is_empty() => {
                Err(format!("Whisper's transcript could not be read: {e}"))
            }
            Err(_) => Ok(printed),
        },
        // Some builds print but do not write JSON; the printed segments
        // are the transcript then.
        Err(_) => Ok(printed),
    }
}
