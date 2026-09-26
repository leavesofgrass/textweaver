//! [`EciBackend`]: the `eci` speech backend.
//!
//! Timing (ADR-0003): `speak` sends the utterance to the host and returns at
//! once. The host synthesizes far faster than real time and streams PCM and
//! index marks back; `poll` moves that audio into the playback [`Feed`] and
//! emits events against the playback clock (samples actually consumed by
//! the output):
//! - `Started` when the utterance's first sample is played;
//! - `Word { byte_range, audio_ms }` when the audio reaches the index mark
//!   before that word, `audio_ms` being the mark's offset from the
//!   utterance's first sample (pauses excluded);
//! - `Finished` when its last sample is played;
//! - `Cancelled` (from the next `poll` or `speak`) for every utterance a
//!   `stop` discarded. No event for a stopped utterance follows it.
//!
//! Utterances queue: several may be spoken ahead (the service's lookahead);
//! their audio plays back to back and their events fire in order.
//!
//! Parameters (ADR-0004): rate maps to ECI speed through the measured table
//! in [`calibration`]; pitch moves the preset's pitch baseline; volume is a
//! playback gain (immediate, and applied to `synthesize_to_file`); the voice
//! selects an ECI language and preset ([`voices`]).
//!
//! If the host dies (a crash inside the proprietary engine), the utterances
//! it had not finished end with `Error` then `Finished`, and the next
//! `speak` starts a new host.

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, TryRecvError};
use std::time::{Duration, Instant};

use textweaver_core::{Utterance, UtteranceId};
use textweaver_speech::{
    BackendId, Caps, EventSink, RawEvent, SpeechBackend, SpeechError, Voice, VoiceParams,
};

use crate::audio::{Feed, Player};
use crate::calibration::{self, RatePoint};
use crate::language;
use crate::protocol::{self, EndStatus, PresetInfo, Reply, Request};
use crate::voices::{self, VoiceParam};
use crate::{BACKEND_ID, EciConfig, words};

/// How long to wait for a new host to report `Ready`.
const READY_TIMEOUT: Duration = Duration::from_secs(10);
/// How long `synthesize` waits for the host to finish one text.
const SYNTH_TIMEOUT: Duration = Duration::from_secs(60);

/// What the host said at start-up.
#[derive(Clone, Debug)]
struct ReadyInfo {
    sample_rate: u32,
    version: String,
    dialects: Vec<u32>,
    default_dialect: u32,
    presets: Vec<PresetInfo>,
}

enum HostMsg {
    Reply(Reply),
    Closed(String),
}

/// A running host process.
struct Host {
    child: Child,
    stdin: Option<ChildStdin>,
    rx: Receiver<HostMsg>,
    path: PathBuf,
}

impl Host {
    fn spawn(path: &Path, config: &EciConfig) -> Result<(Host, ReadyInfo), String> {
        let mut cmd = Command::new(path);
        if config.fake_engine {
            cmd.args(["--engine", "fake"]);
        } else if let Some(lib) = config.library.clone().or_else(crate::library_path) {
            cmd.arg("--library").arg(lib);
        }
        if let Some(hz) = config.sample_rate {
            cmd.arg("--sample-rate").arg(hz.to_string());
        }
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NO_WINDOW: no console window flashes up for the host.
            cmd.creation_flags(0x0800_0000);
        }
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("cannot start {}: {e}", path.display()))?;
        let stdout = child.stdout.take().ok_or("host has no stdout")?;
        if let Some(stderr) = child.stderr.take() {
            let _ = std::thread::Builder::new()
                .name("eci-host-stderr".into())
                .spawn(move || {
                    for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                        log::debug!("eci host: {line}");
                    }
                });
        }
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("eci-host-reader".into())
            .spawn(move || {
                let mut r = BufReader::with_capacity(64 * 1024, stdout);
                loop {
                    let msg = match protocol::read_body(&mut r) {
                        Ok(Some(body)) => match Reply::decode(&body) {
                            Ok(reply) => HostMsg::Reply(reply),
                            Err(e) => HostMsg::Closed(format!("bad frame from host: {e}")),
                        },
                        Ok(None) => HostMsg::Closed("host exited".into()),
                        Err(e) => HostMsg::Closed(format!("host pipe failed: {e}")),
                    };
                    let closed = matches!(msg, HostMsg::Closed(_));
                    if tx.send(msg).is_err() || closed {
                        break;
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        let stdin = child.stdin.take();
        let mut host = Host {
            child,
            stdin,
            rx,
            path: path.to_path_buf(),
        };
        match host.rx.recv_timeout(READY_TIMEOUT) {
            Ok(HostMsg::Reply(Reply::Ready {
                sample_rate,
                version,
                dialects,
                default_dialect,
                presets,
                ..
            })) => Ok((
                host,
                ReadyInfo {
                    sample_rate,
                    version,
                    dialects,
                    default_dialect,
                    presets,
                },
            )),
            Ok(HostMsg::Reply(Reply::Error { message, .. })) => {
                host.shutdown();
                Err(message)
            }
            Ok(HostMsg::Reply(other)) => {
                host.shutdown();
                Err(format!("host sent {other:?} before Ready"))
            }
            Ok(HostMsg::Closed(why)) => {
                host.shutdown();
                Err(why)
            }
            Err(_) => {
                host.shutdown();
                Err("host did not start in time".into())
            }
        }
    }

    fn send(&mut self, req: &Request) -> Result<(), String> {
        let stdin = self.stdin.as_mut().ok_or("host input closed")?;
        protocol::write_frame(stdin, &req.encode()).map_err(|e| format!("host pipe: {e}"))
    }

    fn shutdown(&mut self) {
        if let Some(mut stdin) = self.stdin.take() {
            let _ = protocol::write_frame(&mut stdin, &Request::Quit.encode());
            let _ = stdin.flush();
        }
        let deadline = Instant::now() + Duration::from_millis(500);
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                _ => break,
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// One utterance in flight.
#[derive(Debug)]
struct Active {
    id: UtteranceId,
    token: u64,
    /// Highlight range per index mark.
    words: Vec<Range<u32>>,
    /// Marks received and not yet emitted: (index, sample in utterance).
    marks: VecDeque<(u32, u64)>,
    /// Feed position of the utterance's first sample, once known.
    start: Option<u64>,
    /// Samples pushed so far.
    total: u64,
    /// The host sent `End`.
    done: bool,
    started: bool,
    failed: Option<String>,
}

/// A `synthesize` call collecting audio instead of playing it.
#[derive(Debug, Default)]
struct Capture {
    samples: Vec<i16>,
    marks: Vec<(u32, u64)>,
    done: bool,
    failed: Option<String>,
}

/// Synthesized audio for one text, with each word's first sample.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Synthesis {
    /// Sample rate in Hz (16-bit mono).
    pub sample_rate: u32,
    /// The audio (volume applied).
    pub samples: Vec<i16>,
    /// Each word's byte range in the text and the sample at which the
    /// engine reached its index mark, in order.
    pub words: Vec<(Range<u32>, u64)>,
}

/// Voice settings as last sent to the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Applied {
    dialect: u32,
    preset: u8,
    speed: i32,
    pitch_baseline: i32,
}

/// The ETI-Eloquence backend.
pub struct EciBackend {
    config: EciConfig,
    host: Option<Host>,
    ready: Option<ReadyInfo>,
    feed: Arc<Feed>,
    player: Option<(Player, u32)>,
    params: VoiceParams,
    /// Voice selection derived from `params.voice`.
    dialect: Option<u32>,
    preset: u8,
    applied: Option<Applied>,
    rate_table: &'static [RatePoint],
    next_token: u64,
    active: VecDeque<Active>,
    captures: HashMap<u64, Capture>,
    cancelled: Vec<UtteranceId>,
}

impl std::fmt::Debug for EciBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EciBackend")
            .field("config", &self.config)
            .field("host", &self.host.as_ref().map(|h| h.path.clone()))
            .field("active", &self.active.len())
            .finish_non_exhaustive()
    }
}

fn unavailable(msg: impl Into<String>) -> SpeechError {
    SpeechError::Unavailable(BACKEND_ID, msg.into())
}

impl EciBackend {
    /// Starts the host and waits for the engine to report ready. Audio
    /// output opens on the first `speak`.
    pub fn new(config: EciConfig) -> Result<Self, SpeechError> {
        let mut b = EciBackend {
            config,
            host: None,
            ready: None,
            feed: Arc::new(Feed::default()),
            player: None,
            params: VoiceParams::default(),
            dialect: None,
            preset: 1,
            applied: None,
            rate_table: calibration::VOXIN,
            next_token: 1,
            active: VecDeque::new(),
            captures: HashMap::new(),
            cancelled: Vec::new(),
        };
        b.ensure_host()?;
        Ok(b)
    }

    /// The engine's version string, as the host reported it.
    pub fn engine_version(&self) -> Option<&str> {
        self.ready.as_ref().map(|r| r.version.as_str())
    }

    /// The engine's sample rate.
    pub fn sample_rate(&self) -> Option<u32> {
        self.ready.as_ref().map(|r| r.sample_rate)
    }

    /// The host executable in use.
    pub fn host_path(&self) -> Option<&Path> {
        self.host.as_ref().map(|h| h.path.as_path())
    }

    /// The ECI speed the current rate maps to.
    pub fn eci_speed(&self) -> i32 {
        calibration::speed_for_wpm(self.rate_table, self.params.rate.wpm())
    }

    fn ensure_host(&mut self) -> Result<(), SpeechError> {
        if self.host.is_some() {
            return Ok(());
        }
        let candidates = crate::host_candidates(&self.config);
        if candidates.is_empty() {
            return Err(unavailable(
                "textweaver-eci-host not found (run `cargo xtask eci-host`, or set TEXTWEAVER_ECI_HOST)",
            ));
        }
        let mut errors = Vec::new();
        for path in candidates {
            match Host::spawn(&path, &self.config) {
                Ok((host, ready)) => {
                    log::info!(
                        "eci: {} (ECI {}, {} Hz)",
                        path.display(),
                        ready.version,
                        ready.sample_rate
                    );
                    self.rate_table = calibration::table_for(&ready.version);
                    self.host = Some(host);
                    self.ready = Some(ready);
                    self.applied = None;
                    self.apply_voice()?;
                    return Ok(());
                }
                Err(e) => errors.push(format!("{}: {e}", path.display())),
            }
        }
        Err(unavailable(errors.join("; ")))
    }

    fn ensure_player(&mut self) -> Result<(), SpeechError> {
        let rate = self.ready.as_ref().map_or(11025, |r| r.sample_rate);
        if self.player.as_ref().is_some_and(|(_, r)| *r == rate) {
            return Ok(());
        }
        self.player = None;
        let p = Player::start(self.config.output, Arc::clone(&self.feed), rate)?;
        self.player = Some((p, rate));
        Ok(())
    }

    fn send(&mut self, req: &Request) -> Result<(), SpeechError> {
        let host = self
            .host
            .as_mut()
            .ok_or_else(|| SpeechError::Engine("Eloquence host is not running".into()))?;
        host.send(req).map_err(SpeechError::Engine)
    }

    /// The installed dialects: the engine's list, else the `eci.ini` next to
    /// the library, else the default dialect.
    fn installed_dialects(ready: &ReadyInfo, library: Option<&Path>) -> Vec<u32> {
        if !ready.dialects.is_empty() {
            return ready.dialects.clone();
        }
        let ini = std::env::var_os("ECIINI")
            .map(PathBuf::from)
            .or_else(|| library.and_then(Path::parent).map(|d| d.join("eci.ini")));
        if let Some(text) = ini.and_then(|p| std::fs::read_to_string(p).ok()) {
            let list = voices::ini_dialects(&text, |p| Path::new(p).exists());
            if !list.is_empty() {
                return list;
            }
        }
        vec![ready.default_dialect]
    }

    /// Sends voice, speed, and pitch to the host when they differ from what
    /// it has.
    fn apply_voice(&mut self) -> Result<(), SpeechError> {
        let Some(ready) = &self.ready else {
            return Ok(());
        };
        let dialect = self.dialect.unwrap_or(ready.default_dialect);
        let preset_baseline = ready
            .presets
            .get(usize::from(self.preset.saturating_sub(1)))
            .map_or(65, |p| p.params[VoiceParam::PitchBaseline as usize]);
        let want = Applied {
            dialect,
            preset: self.preset,
            speed: calibration::speed_for_wpm(self.rate_table, self.params.rate.wpm()),
            pitch_baseline: calibration::pitch_baseline(
                preset_baseline,
                self.params.pitch.semitones(),
            ),
        };
        let prev = self.applied;
        if prev == Some(want) {
            return Ok(());
        }
        let voice_changed =
            prev.is_none_or(|p| p.dialect != want.dialect || p.preset != want.preset);
        if voice_changed {
            self.send(&Request::SetVoice {
                dialect: want.dialect,
                preset: want.preset,
            })?;
        }
        if voice_changed || prev.is_some_and(|p| p.speed != want.speed) {
            self.send(&Request::SetVoiceParam {
                param: VoiceParam::Speed as u8,
                value: want.speed,
            })?;
        }
        if voice_changed || prev.is_some_and(|p| p.pitch_baseline != want.pitch_baseline) {
            self.send(&Request::SetVoiceParam {
                param: VoiceParam::PitchBaseline as u8,
                value: want.pitch_baseline,
            })?;
        }
        self.applied = Some(want);
        Ok(())
    }

    /// Reads everything the host has sent so far.
    fn drain_host(&mut self) {
        loop {
            let msg = match &self.host {
                Some(h) => match h.rx.try_recv() {
                    Ok(m) => m,
                    Err(TryRecvError::Empty) => return,
                    Err(TryRecvError::Disconnected) => HostMsg::Closed("host exited".into()),
                },
                None => return,
            };
            self.handle(msg);
        }
    }

    fn handle(&mut self, msg: HostMsg) {
        match msg {
            HostMsg::Closed(why) => self.host_died(&why),
            HostMsg::Reply(r) => self.handle_reply(r),
        }
    }

    fn host_died(&mut self, why: &str) {
        log::warn!("eci: {why}");
        self.host = None;
        self.applied = None;
        let at = self.feed.pushed();
        for a in &mut self.active {
            if !a.done {
                a.done = true;
                a.start.get_or_insert(at);
                a.failed = Some(format!("Eloquence stopped unexpectedly ({why})"));
            }
        }
        for c in self.captures.values_mut() {
            if !c.done {
                c.done = true;
                c.failed = Some(format!("Eloquence stopped unexpectedly ({why})"));
            }
        }
    }

    fn handle_reply(&mut self, r: Reply) {
        match r {
            Reply::Audio { token, samples } => {
                if let Some(c) = self.captures.get_mut(&token) {
                    c.samples.extend_from_slice(&samples);
                } else if let Some(a) = self.active.iter_mut().find(|a| a.token == token) {
                    let at = self.feed.push(&samples);
                    a.start.get_or_insert(at);
                    a.total += samples.len() as u64;
                }
            }
            Reply::Mark {
                token,
                index,
                sample,
            } => {
                if let Some(c) = self.captures.get_mut(&token) {
                    c.marks.push((index, sample));
                } else if let Some(a) = self.active.iter_mut().find(|a| a.token == token) {
                    a.start.get_or_insert(self.feed.pushed());
                    a.marks.push_back((index, sample));
                }
            }
            Reply::End { token, status, .. } => {
                if let Some(c) = self.captures.get_mut(&token) {
                    c.done = true;
                    if status == EndStatus::Failed && c.failed.is_none() {
                        c.failed = Some("synthesis failed".into());
                    }
                } else if let Some(a) = self.active.iter_mut().find(|a| a.token == token) {
                    a.done = true;
                    a.start.get_or_insert(self.feed.pushed());
                    if status == EndStatus::Failed && a.failed.is_none() {
                        a.failed = Some("synthesis failed".into());
                    }
                }
            }
            Reply::Error { token, message } => {
                if let Some(c) = self.captures.get_mut(&token) {
                    c.failed = Some(message);
                } else if let Some(a) = self.active.iter_mut().find(|a| a.token == token) {
                    a.failed = Some(message);
                } else {
                    log::warn!("eci: {message}");
                }
            }
            Reply::Ready { .. } => log::warn!("eci: unexpected Ready from host"),
        }
    }

    /// Emits every event the playback clock has reached.
    fn advance(&mut self, sink: &mut dyn EventSink) {
        for id in self.cancelled.drain(..) {
            sink.emit(id, RawEvent::Cancelled);
        }
        let consumed = self.feed.consumed();
        let rate = u64::from(self.ready.as_ref().map_or(11025, |r| r.sample_rate).max(1));
        while let Some(a) = self.active.front_mut() {
            let Some(start) = a.start else { break };
            if !a.started {
                let begun = if a.total > 0 {
                    consumed > start
                } else {
                    a.done && consumed >= start
                };
                if !begun {
                    break;
                }
                a.started = true;
                sink.emit(a.id, RawEvent::Started);
            }
            let finished = a.done && consumed >= start + a.total;
            while let Some(&(index, sample)) = a.marks.front() {
                if !finished && start + sample > consumed {
                    break;
                }
                a.marks.pop_front();
                if let Some(range) = a.words.get(index as usize) {
                    let ms = u32::try_from(sample * 1000 / rate).unwrap_or(u32::MAX);
                    sink.emit(
                        a.id,
                        RawEvent::Word {
                            byte_range: range.clone(),
                            audio_ms: Some(ms),
                        },
                    );
                }
            }
            if !finished {
                break;
            }
            if let Some(e) = a.failed.take() {
                sink.emit(a.id, RawEvent::Error(e));
            }
            sink.emit(a.id, RawEvent::Finished);
            self.active.pop_front();
        }
    }

    /// Synthesizes `text` without playing it: audio (volume applied) and
    /// each word's first sample. Speech in progress keeps playing.
    pub fn synthesize(&mut self, text: &str) -> Result<Synthesis, SpeechError> {
        self.drain_host();
        self.ensure_host()?;
        self.apply_voice()?;
        let ws = words::words(text);
        let token = self.next_token;
        self.next_token += 1;
        self.captures.insert(token, Capture::default());
        let sent = self.send(&Request::Speak {
            token,
            pieces: words::pieces(text, &ws),
        });
        if let Err(e) = sent {
            self.captures.remove(&token);
            return Err(e);
        }
        let deadline = Instant::now() + SYNTH_TIMEOUT;
        while !self.captures.get(&token).is_some_and(|c| c.done) {
            let left = deadline.saturating_duration_since(Instant::now());
            let msg = match &self.host {
                Some(h) => match h.rx.recv_timeout(left) {
                    Ok(m) => m,
                    Err(RecvTimeoutError::Timeout) => {
                        self.captures.remove(&token);
                        return Err(SpeechError::Engine("Eloquence took too long".into()));
                    }
                    Err(RecvTimeoutError::Disconnected) => HostMsg::Closed("host exited".into()),
                },
                None => HostMsg::Closed("host is not running".into()),
            };
            self.handle(msg);
        }
        let cap = self.captures.remove(&token).unwrap_or_default();
        if let Some(e) = cap.failed {
            return Err(SpeechError::Engine(e));
        }
        let gain = self.feed.gain();
        let samples = if (gain - 1.0).abs() < f32::EPSILON {
            cap.samples
        } else {
            cap.samples
                .iter()
                .map(|&s| (f32::from(s) * gain).round().clamp(-32768.0, 32767.0) as i16)
                .collect()
        };
        Ok(Synthesis {
            sample_rate: self.ready.as_ref().map_or(11025, |r| r.sample_rate),
            samples,
            words: cap
                .marks
                .iter()
                .filter_map(|&(i, s)| ws.get(i as usize).map(|w| (w.highlight.clone(), s)))
                .collect(),
        })
    }
}

impl SpeechBackend for EciBackend {
    fn id(&self) -> BackendId {
        BACKEND_ID
    }

    fn capabilities(&self) -> Caps {
        let mut caps = Caps::WORD_EVENTS
            | Caps::AUDIO_CLOCK
            | Caps::PAUSE
            | Caps::PITCH
            | Caps::VOLUME
            | Caps::SYNTH_TO_FILE;
        if self.config.output == crate::AudioOutput::Device {
            caps |= Caps::TONES;
        }
        caps
    }

    fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        let ready = self
            .ready
            .as_ref()
            .ok_or_else(|| unavailable("the engine has not started"))?;
        let library = self.config.library.clone().or_else(crate::library_path);
        let dialects = Self::installed_dialects(ready, library.as_deref());
        Ok(voices::voice_list(&dialects, &ready.presets))
    }

    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
        let sel = match &params.voice {
            None => voices::VoiceSel {
                dialect: None,
                preset: Some(1),
            },
            Some(id) => {
                voices::parse_voice_id(id).ok_or_else(|| SpeechError::UnknownVoice(id.clone()))?
            }
        };
        if let Some(code) = sel.dialect {
            let d = language::dialect_by_code(code)
                .filter(|d| d.supported)
                .ok_or_else(|| {
                    SpeechError::UnknownVoice(params.voice.clone().unwrap_or_default())
                })?;
            if let Some(ready) = &self.ready {
                let library = self.config.library.clone().or_else(crate::library_path);
                if !Self::installed_dialects(ready, library.as_deref()).contains(&d.code) {
                    return Err(SpeechError::UnknownVoice(format!(
                        "{} ({} is not installed)",
                        params.voice.clone().unwrap_or_default(),
                        d.name
                    )));
                }
            }
            self.dialect = Some(code);
        } else if params.voice.is_none() {
            self.dialect = None;
        }
        if let Some(p) = sel.preset {
            self.preset = p;
        }
        self.params = params.clone();
        self.feed.set_gain(params.volume.fraction());
        if self.host.is_some() {
            self.apply_voice()?;
        }
        Ok(())
    }

    fn effective_wpm(&self) -> u16 {
        calibration::effective_wpm(self.rate_table, self.eci_speed())
    }

    fn speak(
        &mut self,
        utterance: &Utterance,
        sink: &mut dyn EventSink,
    ) -> Result<(), SpeechError> {
        self.drain_host();
        self.advance(sink);
        self.ensure_host()?;
        self.ensure_player()?;
        self.apply_voice()?;
        let ws = words::words(&utterance.text);
        let token = self.next_token;
        self.next_token += 1;
        self.send(&Request::Speak {
            token,
            pieces: words::pieces(&utterance.text, &ws),
        })?;
        self.active.push_back(Active {
            id: utterance.id,
            token,
            words: ws.into_iter().map(|w| w.highlight).collect(),
            marks: VecDeque::new(),
            start: None,
            total: 0,
            done: false,
            started: false,
            failed: None,
        });
        Ok(())
    }

    fn poll(&mut self, sink: &mut dyn EventSink) {
        self.drain_host();
        self.advance(sink);
    }

    fn stop(&mut self) {
        if !self.active.is_empty() && self.host.is_some() {
            if let Err(e) = self.send(&Request::Stop) {
                log::warn!("eci: {e}");
            }
        }
        self.feed.clear();
        self.feed.set_paused(false);
        self.cancelled.extend(self.active.drain(..).map(|a| a.id));
    }

    fn pause(&mut self) -> Result<(), SpeechError> {
        self.feed.set_paused(true);
        Ok(())
    }

    fn resume(&mut self) -> Result<(), SpeechError> {
        self.feed.set_paused(false);
        Ok(())
    }

    fn synthesize_to_file(&mut self, text: &str, path: &Path) -> Result<(), SpeechError> {
        let s = self.synthesize(text)?;
        let mut f = std::io::BufWriter::new(
            std::fs::File::create(path).map_err(|e| SpeechError::Io(e.to_string()))?,
        );
        crate::wav::write_wav(&mut f, s.sample_rate, &s.samples)
            .map_err(|e| SpeechError::Io(e.to_string()))
    }

    fn tone(&mut self, hz: f32, ms: u32) {
        if self.ensure_player().is_ok() {
            if let Some((p, _)) = &self.player {
                p.tone(hz, ms, self.feed.gain());
            }
        }
    }
}

impl Drop for EciBackend {
    fn drop(&mut self) {
        // Stop audio before the host goes away.
        self.feed.clear();
        self.player = None;
        if let Some(mut h) = self.host.take() {
            h.shutdown();
        }
    }
}
