//! The `piper` speech backend.
//!
//! A worker thread owns the loaded voice (loading a model takes a moment,
//! and running it blocks), so the speech thread never waits
//! (ADR-0003). `speak` sends the text to the worker and returns; the
//! worker synthesizes it a chunk at a time (a sentence, and the first
//! clause of a long first sentence on its own) and sends back each
//! chunk's audio and word positions. `poll` hands them to the shared
//! playback client from `textweaver-enginehost` (the one the ECI, SAPI,
//! and DECtalk hosts use), which plays the audio and fires each word
//! event as its first sample is heard (`PLAYBACK_EVENTS`). Pause is
//! native: the playback clock stops with the audio.
//!
//! **Word timing** comes from the model's phoneme durations (`w_ceil`).
//! A voice whose graph lacks them reports no word events, and the speech
//! service paces the highlight by estimate (`tw backends` says which).
//!
//! **Fallback.** A voice RTen cannot run (an operator it lacks, or a
//! phonemizer other than eSpeak's) is spoken by the `piper` program when
//! one is installed (`TEXTWEAVER_PIPER`, or `piper` on `PATH`), one
//! chunk per run, with estimated word timing.
//!
//! The registry entry ([`backend_description`], [`probe`], [`factory`])
//! follows the other engine crates, so the app registers it in one call
//! and it moves with the registry.

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use textweaver_core::Utterance;
use textweaver_enginehost::{AudioOutput, EndStatus, Playback, wav};
use textweaver_speech::{
    BackendId, BackendInfo, Caps, EventSink, FileSynthesis, SpeechBackend, SpeechError, Voice,
    VoiceParams,
};

use crate::config::VoiceConfig;
use crate::phonemes::{PhonemizerChoice, phonemizer};
use crate::store::{InstalledVoice, VoiceStore};
use crate::synth::{SynthParams, Synthesizer, Timing};

/// The backend's id.
pub const BACKEND_ID: BackendId = "piper";

/// Automatic selection priority: below Eloquence (1000), SAPI (500), and
/// DECtalk (300), above the built-in engines (espeak-ng 50).
pub const PRIORITY: i32 = 200;

/// The program run when RTen cannot run a voice.
pub const PROGRAM_ENV: &str = "TEXTWEAVER_PIPER";

/// Longest wait for one utterance written to a file.
const FILE_TIMEOUT: Duration = Duration::from_secs(300);

/// What the backend can do with a voice that reports word timing. Tones
/// are added for the audio device.
pub const CAPS: Caps = Caps::WORD_EVENTS
    .union(Caps::AUDIO_CLOCK)
    .union(Caps::PLAYBACK_EVENTS)
    .union(Caps::PAUSE)
    .union(Caps::PITCH)
    .union(Caps::VOLUME)
    .union(Caps::SYNTH_TO_FILE)
    .union(Caps::SILENCE);

/// The backend's options.
#[derive(Clone, Debug, PartialEq)]
pub struct PiperConfig {
    /// Installed voices (`<data>/piper/voices`).
    pub voices_dir: PathBuf,
    /// Where the pure-Rust phonemizer keeps its data
    /// (`<data>/piper/espeak-ng-data`).
    pub espeak_data: PathBuf,
    /// Which phonemizer to use.
    pub phonemizer: PhonemizerChoice,
    /// Where audio goes.
    pub output: AudioOutput,
    /// The voice to start with; the first installed English voice, else
    /// the first installed voice, when `None`.
    pub default_voice: Option<String>,
    /// The `piper` program for voices RTen cannot run; looked for on
    /// `PATH` (and in `TEXTWEAVER_PIPER`) when `None`.
    pub program: Option<PathBuf>,
}

impl PiperConfig {
    /// Options for the data folder `data_dir` (voices in
    /// `data_dir/piper/voices`), the audio device, automatic phonemizer.
    pub fn in_data_dir(data_dir: &Path) -> Self {
        let root = data_dir.join("piper");
        PiperConfig {
            voices_dir: root.join("voices"),
            espeak_data: root.join("espeak-ng-data"),
            phonemizer: PhonemizerChoice::Auto,
            output: AudioOutput::default(),
            default_voice: None,
            program: None,
        }
    }

    /// The voices folder.
    pub fn store(&self) -> VoiceStore {
        VoiceStore::new(&self.voices_dir)
    }

    /// The `piper` program to fall back on, if any.
    pub fn fallback_program(&self) -> Option<PathBuf> {
        if let Some(p) = &self.program {
            return p.is_file().then(|| p.clone());
        }
        if let Some(p) = std::env::var_os(PROGRAM_ENV).filter(|v| !v.is_empty()) {
            let p = PathBuf::from(p);
            return p.is_file().then_some(p);
        }
        let path = std::env::var_os("PATH")?;
        let exe = format!("piper{}", std::env::consts::EXE_SUFFIX);
        std::env::split_paths(&path)
            .map(|d| d.join(&exe))
            .find(|p| p.is_file())
    }
}

/// The registry description, known without starting anything.
pub fn backend_description() -> BackendInfo {
    BackendInfo {
        id: BACKEND_ID,
        name: "Piper neural voices",
        priority: PRIORITY,
        opt_in: false,
        available: false,
        caps: CAPS,
    }
}

/// True when a voice is installed and it can be phonemized (a
/// phonemizer is compiled in, or the `piper` program is installed).
pub fn probe(config: &PiperConfig) -> bool {
    if config.store().installed().is_empty() {
        return false;
    }
    cfg!(any(feature = "espeak-lib", feature = "espeak-rs")) || config.fallback_program().is_some()
}

/// A constructor for the registry.
pub fn factory(
    config: PiperConfig,
) -> impl Fn() -> Result<Box<dyn SpeechBackend>, SpeechError> + Send + Sync + 'static {
    move || PiperBackend::new(config.clone()).map(|b| Box::new(b) as Box<dyn SpeechBackend>)
}

/// The voice to start with: `wanted` if installed, else the first English
/// voice, else the first voice.
pub fn default_voice(voices: &[InstalledVoice], wanted: Option<&str>) -> Option<InstalledVoice> {
    if let Some(w) = wanted
        && let Some(v) = voices.iter().find(|v| v.key == w)
    {
        return Some(v.clone());
    }
    voices
        .iter()
        .find(|v| v.key.starts_with("en_"))
        .or(voices.first())
        .cloned()
}

enum Request {
    Load {
        key: String,
        onnx: PathBuf,
        json: PathBuf,
    },
    Speak {
        token: u64,
        text: String,
        params: SynthParams,
        epoch: u64,
    },
}

enum Reply {
    Loaded {
        key: String,
        result: Result<bool, String>,
    },
    Audio {
        token: u64,
        samples: Vec<i16>,
    },
    Word {
        token: u64,
        range: Range<u32>,
        sample: u64,
    },
    End {
        token: u64,
        status: EndStatus,
    },
    Error {
        token: u64,
        message: String,
    },
}

/// Piper voices on RTen, played through the shared playback client.
pub struct PiperBackend {
    config: PiperConfig,
    playback: Playback<()>,
    requests: Option<Sender<Request>>,
    replies: Receiver<Reply>,
    worker: Option<JoinHandle<()>>,
    epoch: Arc<AtomicU64>,
    params: VoiceParams,
    voice: Option<(String, VoiceConfig)>,
    word_timing: bool,
    failure: Option<String>,
}

impl std::fmt::Debug for PiperBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PiperBackend")
            .field("voice", &self.voice.as_ref().map(|(k, _)| k))
            .field("params", &self.params)
            .field("word_timing", &self.word_timing)
            .finish_non_exhaustive()
    }
}

impl PiperBackend {
    /// Starts the worker and begins loading the default voice. Fails when
    /// no voice is installed.
    pub fn new(config: PiperConfig) -> Result<Self, SpeechError> {
        let installed = config.store().installed();
        let first =
            default_voice(&installed, config.default_voice.as_deref()).ok_or_else(|| {
                SpeechError::Unavailable(
                    BACKEND_ID,
                    "no Piper voice is installed; download one in the voice manager".into(),
                )
            })?;
        let (req_tx, req_rx) = mpsc::channel();
        let (rep_tx, rep_rx) = mpsc::channel();
        let epoch = Arc::new(AtomicU64::new(0));
        let worker = {
            let epoch = Arc::clone(&epoch);
            let config = config.clone();
            std::thread::Builder::new()
                .name("textweaver-piper".into())
                .spawn(move || work(&config, &req_rx, &rep_tx, &epoch))
                .map_err(|e| SpeechError::Io(e.to_string()))?
        };
        let mut b = PiperBackend {
            playback: Playback::new(BACKEND_ID, config.output, 22_050),
            config,
            requests: Some(req_tx),
            replies: rep_rx,
            worker: Some(worker),
            epoch,
            params: VoiceParams::default(),
            voice: None,
            word_timing: true,
            failure: None,
        };
        b.load(&first)?;
        Ok(b)
    }

    /// The current voice's key.
    pub fn voice(&self) -> Option<&str> {
        self.voice.as_ref().map(|(k, _)| k.as_str())
    }

    fn send(&self, r: Request) -> Result<(), SpeechError> {
        self.requests
            .as_ref()
            .and_then(|tx| tx.send(r).ok())
            .ok_or_else(|| SpeechError::Engine("the Piper worker has stopped".into()))
    }

    /// Reads the voice's settings here (small, and they give the sample
    /// rate at once) and loads its model on the worker.
    fn load(&mut self, v: &InstalledVoice) -> Result<(), SpeechError> {
        let config = VoiceConfig::from_file(&v.json)
            .map_err(|e| SpeechError::Engine(format!("{}: {e}", v.key)))?;
        self.playback.set_sample_rate(config.audio.sample_rate);
        self.voice = Some((v.key.clone(), config));
        self.failure = None;
        self.word_timing = true;
        self.send(Request::Load {
            key: v.key.clone(),
            onnx: v.onnx.clone(),
            json: v.json.clone(),
        })
    }

    fn synth_params(&self) -> SynthParams {
        SynthParams {
            rate: self.params.rate,
            pitch: self.params.pitch,
            speaker: 0,
        }
    }

    fn dispatch(&mut self, reply: Reply) {
        match reply {
            Reply::Loaded { key, result } => {
                if self.voice.as_ref().is_some_and(|(k, _)| *k == key) {
                    match result {
                        Ok(timing) => self.word_timing = timing,
                        Err(e) => {
                            log::warn!("piper: {key}: {e}");
                            self.failure = Some(e);
                        }
                    }
                }
            }
            Reply::Audio { token, samples } => self.playback.on_audio(token, &samples),
            Reply::Word {
                token,
                range,
                sample,
            } => self.playback.on_word(token, sample, |()| Some(range)),
            Reply::End { token, status } => self.playback.on_end(token, status),
            Reply::Error { token, message } => {
                if !self.playback.on_error(token, message.clone()) {
                    log::debug!("piper: late error for {token}: {message}");
                }
            }
        }
    }

    fn drain(&mut self) {
        while let Ok(r) = self.replies.try_recv() {
            self.dispatch(r);
        }
    }
}

impl SpeechBackend for PiperBackend {
    fn id(&self) -> BackendId {
        BACKEND_ID
    }

    fn capabilities(&self) -> Caps {
        let mut caps = CAPS;
        if !self.word_timing {
            caps.remove(Caps::WORD_EVENTS | Caps::AUDIO_CLOCK);
        }
        if self.playback.output() == AudioOutput::Device {
            caps |= Caps::TONES;
        }
        caps
    }

    fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        Ok(self
            .config
            .store()
            .installed()
            .iter()
            .map(InstalledVoice::to_voice)
            .collect())
    }

    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
        if let Some(key) = &params.voice
            && self.voice() != Some(key.as_str())
        {
            let v = self
                .config
                .store()
                .get(key)
                .ok_or_else(|| SpeechError::UnknownVoice(key.clone()))?;
            self.load(&v)?;
        }
        self.playback.set_gain(params.volume.fraction());
        self.params = params.clone();
        Ok(())
    }

    fn effective_wpm(&self) -> u16 {
        match &self.voice {
            Some((_, c)) => self.synth_params().effective_wpm(c),
            None => self.params.rate.wpm(),
        }
    }

    fn speak(
        &mut self,
        utterance: &Utterance,
        sink: &mut dyn EventSink,
    ) -> Result<(), SpeechError> {
        let token = self.playback.next_token();
        self.playback.enqueue(utterance.id, token, 0, ());
        if let Err(e) = self.playback.ensure_player() {
            self.playback.on_error(token, e.to_string());
            self.playback.on_end(token, EndStatus::Failed);
        } else if let Err(e) = self.send(Request::Speak {
            token,
            text: utterance.text.clone(),
            params: self.synth_params(),
            epoch: self.epoch.load(Ordering::SeqCst),
        }) {
            self.playback.on_error(token, e.to_string());
            self.playback.on_end(token, EndStatus::Failed);
        }
        self.playback.emit(sink);
        Ok(())
    }

    fn poll(&mut self, sink: &mut dyn EventSink) {
        self.drain();
        self.playback.emit(sink);
    }

    fn silence_after(&mut self, id: textweaver_speech::core::UtteranceId, ms: u32) {
        // Structural pauses play as silence in the shared playback client.
        self.playback.silence_after(id, ms);
    }

    fn stop(&mut self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
        self.playback.stop();
    }

    fn pause(&mut self) -> Result<(), SpeechError> {
        self.playback.pause();
        Ok(())
    }

    fn resume(&mut self) -> Result<(), SpeechError> {
        self.playback.resume();
        Ok(())
    }

    fn tone(&mut self, hz: f32, ms: u32) {
        self.playback.tone(hz, ms);
    }

    fn synthesize_to_file(&mut self, text: &str, path: &Path) -> Result<(), SpeechError> {
        let u = Utterance::literal(text, textweaver_core::CharPos(0));
        self.synthesize_utterance(&u, path).map(|_| ())
    }

    fn synthesize_utterance(
        &mut self,
        utterance: &Utterance,
        path: &Path,
    ) -> Result<FileSynthesis, SpeechError> {
        if let Some(e) = &self.failure {
            return Err(SpeechError::Engine(e.clone()));
        }
        let token = self.playback.next_token();
        self.playback.capture(token, 0, ());
        self.send(Request::Speak {
            token,
            text: utterance.text.clone(),
            params: self.synth_params(),
            epoch: self.epoch.load(Ordering::SeqCst),
        })?;
        let deadline = Instant::now() + FILE_TIMEOUT;
        while self.playback.capture_done(token) == Some(false) {
            let left = deadline.saturating_duration_since(Instant::now());
            match self
                .replies
                .recv_timeout(left.min(Duration::from_millis(200)))
            {
                Ok(r) => self.dispatch(r),
                Err(RecvTimeoutError::Timeout) if left.is_zero() => {
                    self.playback.take_capture(token);
                    return Err(SpeechError::Engine("Piper took too long".into()));
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(SpeechError::Engine("the Piper worker has stopped".into()));
                }
            }
        }
        let captured = self
            .playback
            .take_capture(token)
            .ok_or_else(|| SpeechError::Engine("the audio was lost".into()))?;
        if let Some(e) = captured.failed {
            return Err(SpeechError::Engine(e));
        }
        let rate = self.playback.sample_rate();
        wav::write(path, &captured.samples, rate).map_err(|e| SpeechError::Io(e.to_string()))?;
        Ok(FileSynthesis {
            words: captured.word_timings(rate),
        })
    }
}

impl Drop for PiperBackend {
    fn drop(&mut self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
        self.playback.close();
        // Closing the channel ends the worker after its current chunk.
        self.requests = None;
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

/// How the worker speaks the loaded voice.
enum Engine {
    Rten(Box<Synthesizer>),
    Program { program: PathBuf, onnx: PathBuf },
    Failed(String),
}

/// The worker loop: loads voices and synthesizes utterances in order.
fn work(
    config: &PiperConfig,
    requests: &Receiver<Request>,
    replies: &Sender<Reply>,
    epoch: &AtomicU64,
) {
    let mut engine = Engine::Failed("no voice is loaded".into());
    for request in requests {
        match request {
            Request::Load { key, onnx, json } => {
                engine = load_engine(config, &onnx, &json);
                let result = match &engine {
                    Engine::Rten(s) => Ok(s.has_word_timing()),
                    Engine::Program { .. } => Ok(false),
                    Engine::Failed(e) => Err(e.clone()),
                };
                if replies.send(Reply::Loaded { key, result }).is_err() {
                    return;
                }
            }
            Request::Speak {
                token,
                text,
                params,
                epoch: at,
            } => {
                let status = speak_one(&mut engine, token, &text, &params, at, epoch, replies);
                if replies.send(Reply::End { token, status }).is_err() {
                    return;
                }
            }
        }
    }
}

fn load_engine(config: &PiperConfig, onnx: &Path, json: &Path) -> Engine {
    let rten = phonemizer(config.phonemizer, &config.espeak_data)
        .and_then(|p| Synthesizer::load(onnx, json, p));
    match rten {
        Ok(s) => {
            log::info!(
                "piper: loaded {} (phonemizer {}, word timing {})",
                onnx.display(),
                s.phonemizer_name(),
                if s.has_word_timing() {
                    "from the model"
                } else {
                    "estimated"
                }
            );
            Engine::Rten(Box::new(s))
        }
        Err(e) => match config.fallback_program() {
            Some(program) => {
                log::warn!(
                    "piper: RTen cannot run {} ({e}); using {}",
                    onnx.display(),
                    program.display()
                );
                Engine::Program {
                    program,
                    onnx: onnx.to_owned(),
                }
            }
            None => Engine::Failed(e.to_string()),
        },
    }
}

fn speak_one(
    engine: &mut Engine,
    token: u64,
    text: &str,
    params: &SynthParams,
    at: u64,
    epoch: &AtomicU64,
    replies: &Sender<Reply>,
) -> EndStatus {
    let current = || epoch.load(Ordering::SeqCst) == at;
    if !current() {
        return EndStatus::Aborted;
    }
    let fail = |message: String| {
        let _ = replies.send(Reply::Error { token, message });
        EndStatus::Failed
    };
    let mut sent = 0u64;
    let mut aborted = false;
    let mut send_chunk = |samples: Vec<i16>, words: Vec<(Range<u32>, u64)>| {
        if !current() {
            aborted = true;
            return false;
        }
        for (range, sample) in words {
            let _ = replies.send(Reply::Word {
                token,
                range,
                sample: sent + sample,
            });
        }
        sent += samples.len() as u64;
        replies.send(Reply::Audio { token, samples }).is_ok()
    };
    let result = match engine {
        Engine::Rten(s) => s.speak(text, params, |chunk| {
            let words = match chunk.timing {
                Timing::Model => chunk.words,
                // Estimated words are left to the service's pacer.
                Timing::Estimated => Vec::new(),
            };
            send_chunk(chunk.samples, words)
        }),
        Engine::Program { program, onnx } => {
            for r in Synthesizer::plan(text) {
                match run_program(program, onnx, &text[r]) {
                    Ok(samples) => {
                        if !send_chunk(samples, Vec::new()) {
                            break;
                        }
                    }
                    Err(e) => return fail(e),
                }
            }
            Ok(())
        }
        Engine::Failed(e) => return fail(format!("the Piper voice could not be loaded: {e}")),
    };
    match result {
        Err(e) => fail(e.to_string()),
        Ok(()) if aborted => EndStatus::Aborted,
        Ok(()) => EndStatus::Done,
    }
}

/// Runs the `piper` program on `text` and reads the WAV it writes.
fn run_program(program: &Path, onnx: &Path, text: &str) -> Result<Vec<i16>, String> {
    use std::io::Write;
    let dir = std::env::temp_dir();
    let out = dir.join(format!(
        "textweaver-piper-{}-{}.wav",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    let mut cmd = std::process::Command::new(program);
    cmd.arg("--model")
        .arg(onnx)
        .arg("--output_file")
        .arg(&out)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("cannot start {}: {e}", program.display()))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.replace('\n', " ").as_bytes());
        let _ = stdin.write_all(b"\n");
    }
    let output = child
        .wait_with_output()
        .map_err(|e| format!("{}: {e}", program.display()))?;
    let bytes = std::fs::read(&out);
    let _ = std::fs::remove_file(&out);
    if !output.status.success() {
        return Err(format!(
            "{} failed: {}",
            program.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let bytes = bytes.map_err(|e| format!("{}: {e}", out.display()))?;
    wav::read_wav(&bytes)
        .map(|(_, s)| s)
        .ok_or_else(|| "piper wrote an unreadable WAV file".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Licence;

    fn voice(key: &str) -> InstalledVoice {
        InstalledVoice {
            key: key.into(),
            onnx: PathBuf::from(format!("{key}.onnx")),
            json: PathBuf::from(format!("{key}.onnx.json")),
            licence: Licence::classify(""),
        }
    }

    #[test]
    fn the_default_voice_prefers_the_wanted_then_english() {
        let vs = [voice("de_DE-thorsten-medium"), voice("en_US-joe-medium")];
        assert_eq!(default_voice(&vs, None).unwrap().key, "en_US-joe-medium");
        assert_eq!(
            default_voice(&vs, Some("de_DE-thorsten-medium"))
                .unwrap()
                .key,
            "de_DE-thorsten-medium"
        );
        assert_eq!(
            default_voice(&vs, Some("missing")).unwrap().key,
            "en_US-joe-medium"
        );
        assert!(default_voice(&[], None).is_none());
    }

    #[test]
    fn no_voices_means_unavailable() {
        let tmp = tempfile::tempdir().unwrap();
        let mut config = PiperConfig::in_data_dir(tmp.path());
        config.program = Some(tmp.path().join("no-such-piper"));
        assert!(!probe(&config));
        assert!(matches!(
            PiperBackend::new(config),
            Err(SpeechError::Unavailable("piper", _))
        ));
        let d = backend_description();
        assert_eq!(d.id, "piper");
        assert!(d.caps.contains(Caps::WORD_EVENTS | Caps::PLAYBACK_EVENTS));
    }

    #[test]
    fn a_voice_that_will_not_load_fails_each_utterance() {
        use textweaver_core::UtteranceId;
        use textweaver_speech::RawEvent;

        #[derive(Default)]
        struct Sink(Vec<RawEvent>);
        impl EventSink for Sink {
            fn emit(&mut self, _id: UtteranceId, event: RawEvent) {
                self.0.push(event);
            }
            fn is_current(&self, _id: UtteranceId) -> bool {
                true
            }
        }

        let tmp = tempfile::tempdir().unwrap();
        let mut config = PiperConfig::in_data_dir(tmp.path());
        config.output = AudioOutput::Null { speed: 50.0 };
        config.program = Some(tmp.path().join("no-such-piper"));
        let dir = config.store().voice_dir("en_US-bad-medium");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("en_US-bad-medium.onnx"), b"not a model").unwrap();
        std::fs::write(
            dir.join("en_US-bad-medium.onnx.json"),
            crate::config::tests::JSON,
        )
        .unwrap();
        let mut b = PiperBackend::new(config).unwrap();
        assert_eq!(b.voice(), Some("en_US-bad-medium"));
        assert_eq!(b.voices().unwrap()[0].name, "Bad (medium)");
        let mut sink = Sink::default();
        b.speak(
            &Utterance::literal("Hello.", textweaver_core::CharPos(0)),
            &mut sink,
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !sink.0.contains(&RawEvent::Finished) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
            b.poll(&mut sink);
        }
        assert!(
            matches!(
                sink.0.as_slice(),
                [RawEvent::Started, RawEvent::Error(_), RawEvent::Finished]
            ),
            "{:?}",
            sink.0
        );
        let path = tmp.path().join("x.wav");
        assert!(b.synthesize_to_file("Hi.", &path).is_err());
    }
}
