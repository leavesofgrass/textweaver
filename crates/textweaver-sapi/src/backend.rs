//! [`SapiBackend`]: the `sapi` speech backend.
//!
//! Timing (ADR-0003): `speak` sends the utterance to the host for the
//! selected voice's architecture and returns at once. The host synthesizes
//! faster than real time and streams PCM and word boundaries back; `poll`
//! moves that audio into the playback [`Feed`] and emits events against the
//! playback clock (samples actually consumed by the output):
//! - `Started` when the utterance's first sample is played;
//! - `Word { byte_range, audio_ms }` when the audio reaches the word,
//!   `audio_ms` being its offset from the utterance's first sample (pauses
//!   excluded). SAPI positions are UTF-16 offsets, mapped to UTF-8 byte
//!   ranges of `Utterance::text` ([`crate::words`]); repeated events on
//!   one range are emitted once;
//! - `Finished` when its last sample is played;
//! - `Cancelled` (from the next `poll` or `speak`) for every utterance a
//!   `stop` discarded. No event for a stopped utterance follows it.
//!
//! Utterances queue (the service's lookahead): their audio plays back to
//! back and their events fire in order. Audio that arrives for a later
//! utterance before an earlier one has ended (possible only when the voice
//! changes architecture mid-queue) waits in that utterance's buffer.
//!
//! Parameters (ADR-0004): rate maps to SAPI's `-10..=10` through the
//! measured tables in [`calibration`](crate::calibration); pitch uses
//! `<pitch absmiddle>`; volume is a playback gain (immediate, and applied to
//! `synthesize_to_file`); the voice id selects the host architecture and
//! the SAPI token.
//!
//! Voices whose word events cannot drive a highlight (Code Factory
//! Eloquence through SAPI, ADR-0007) are spoken without word events, and
//! [`capabilities`](SpeechBackend::capabilities) drops `WORD_EVENTS` and
//! `AUDIO_CLOCK` while one is selected, so the service falls back to timer
//! pacing.
//!
//! If a host dies (a crash inside a third-party engine), the utterances it
//! had not finished end with `Error` then `Finished`, and the next `speak`
//! starts a new host.

use std::cell::RefCell;
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
use crate::calibration::{self, RateTable};
use crate::protocol::{self, EndStatus, Reply, Request, VoiceToken};
use crate::voices::{Arch, Family, SapiVoice, VoiceId};
use crate::words::{Utf16Map, WordFilter};
use crate::{BACKEND_ID, SapiConfig};

/// How long to wait for a new host to report `Ready`.
const READY_TIMEOUT: Duration = Duration::from_secs(10);
/// How long `synthesize` waits for the host to finish one text.
const SYNTH_TIMEOUT: Duration = Duration::from_secs(60);
/// How long a voice listing may take.
const LIST_TIMEOUT: Duration = Duration::from_secs(15);

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
    sample_rate: u32,
}

/// Starts `path` with `args`, forwarding its stderr to the log and its
/// replies to a channel.
fn spawn_host(path: &Path, args: &[&str]) -> Result<(Child, Receiver<HostMsg>), String> {
    let mut cmd = Command::new(path);
    cmd.args(args)
        .stdin(Stdio::piped())
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
            .name("sapi-host-stderr".into())
            .spawn(move || {
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    log::debug!("sapi host: {line}");
                }
            });
    }
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("sapi-host-reader".into())
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
    Ok((child, rx))
}

/// Waits for `Ready` from a host that should run `arch`; returns the
/// sample rate.
fn await_ready(rx: &Receiver<HostMsg>, arch: Arch) -> Result<u32, String> {
    match rx.recv_timeout(READY_TIMEOUT) {
        Ok(HostMsg::Reply(Reply::Ready {
            protocol,
            sample_rate,
            arch: got,
            ..
        })) => {
            if got != arch.as_str() {
                return Err(format!("host is {got}, expected {arch}"));
            }
            if protocol != protocol::PROTOCOL_VERSION {
                return Err(format!(
                    "host speaks protocol {protocol}, expected {}",
                    protocol::PROTOCOL_VERSION
                ));
            }
            Ok(sample_rate)
        }
        Ok(HostMsg::Reply(Reply::Error { message, .. })) => Err(message),
        Ok(HostMsg::Reply(other)) => Err(format!("host sent {other:?} before Ready")),
        Ok(HostMsg::Closed(why)) => Err(why),
        Err(_) => Err("host did not start in time".into()),
    }
}

fn kill(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

impl Host {
    fn spawn(path: &Path, fake: bool, arch: Arch) -> Result<Host, String> {
        let args: &[&str] = if fake {
            &["--engine", "fake", "--report-arch", arch.as_str()]
        } else {
            &[]
        };
        let (mut child, rx) = spawn_host(path, args)?;
        let sample_rate = match await_ready(&rx, arch) {
            Ok(r) => r,
            Err(e) => {
                kill(&mut child);
                return Err(e);
            }
        };
        let stdin = child.stdin.take();
        Ok(Host {
            child,
            stdin,
            rx,
            path: path.to_path_buf(),
            sample_rate,
        })
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
        kill(&mut self.child);
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Lists the voice tokens a host sees (`--list-voices`), without loading
/// any engine.
pub(crate) fn list_tokens(
    path: &Path,
    fake: bool,
    arch: Arch,
    category: &str,
) -> Result<Vec<VoiceToken>, String> {
    let mut args = vec!["--list-voices", "--category", category];
    if fake {
        args.extend(["--engine", "fake", "--report-arch", arch.as_str()]);
    }
    let (mut child, rx) = spawn_host(path, &args)?;
    let result = (|| {
        await_ready(&rx, arch)?;
        let deadline = Instant::now() + LIST_TIMEOUT;
        let mut out = Vec::new();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match rx.recv_timeout(left) {
                Ok(HostMsg::Reply(Reply::Voice(v))) => out.push(v),
                Ok(HostMsg::Reply(Reply::Error { message, .. })) => return Err(message),
                Ok(HostMsg::Reply(_)) => {}
                Ok(HostMsg::Closed(_)) => return Ok(out),
                Err(_) => return Err("voice listing timed out".into()),
            }
        }
    })();
    kill(&mut child);
    result
}

/// One utterance in flight.
#[derive(Debug)]
struct Active {
    id: UtteranceId,
    token: u64,
    arch: Arch,
    map: Utf16Map,
    filter: WordFilter,
    /// Report word events (false for voices without word timing).
    words: bool,
    /// Words received and not yet emitted: (byte range, sample).
    marks: VecDeque<(Range<u32>, u64)>,
    /// Feed position of the utterance's first sample, once it has one.
    start: Option<u64>,
    /// Samples pushed into the feed.
    total: u64,
    /// Samples received before this utterance's turn to play.
    waiting: Vec<i16>,
    /// The host sent `End`.
    done: bool,
    started: bool,
    failed: Option<String>,
}

/// A `synthesize` call collecting audio instead of playing it.
#[derive(Debug)]
struct Capture {
    map: Utf16Map,
    filter: WordFilter,
    samples: Vec<i16>,
    words: Vec<(Range<u32>, u64)>,
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
    /// voice reached it, in order (repeats removed).
    pub words: Vec<(Range<u32>, u64)>,
}

/// Voice settings as last sent to one host.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Applied {
    token_id: String,
    rate: i8,
}

/// The SAPI5 backend.
pub struct SapiBackend {
    config: SapiConfig,
    hosts: [Option<Host>; 2],
    applied: [Option<Applied>; 2],
    feed: Arc<Feed>,
    player: Option<(Player, u32)>,
    params: VoiceParams,
    /// The selected voice; `None` is the system default (x64 host).
    voice: Option<VoiceId>,
    family: Family,
    /// A SAPI rate forced by [`SapiBackend::synthesize_at_rate`].
    rate_override: Option<i8>,
    next_token: u64,
    active: VecDeque<Active>,
    captures: HashMap<u64, Capture>,
    cancelled: Vec<UtteranceId>,
    voices: RefCell<Option<Vec<SapiVoice>>>,
}

impl std::fmt::Debug for SapiBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SapiBackend")
            .field("config", &self.config)
            .field("voice", &self.voice)
            .field("active", &self.active.len())
            .finish_non_exhaustive()
    }
}

fn unavailable(msg: impl Into<String>) -> SpeechError {
    SpeechError::Unavailable(BACKEND_ID, msg.into())
}

impl SapiBackend {
    /// Creates the backend and starts the 64-bit host (the default voice's
    /// host), so a missing or broken host is reported here. Audio output
    /// opens on the first `speak`.
    pub fn new(config: SapiConfig) -> Result<Self, SpeechError> {
        let mut b = SapiBackend {
            config,
            hosts: [None, None],
            applied: [None, None],
            feed: Arc::new(Feed::default()),
            player: None,
            params: VoiceParams::default(),
            voice: None,
            family: Family::Microsoft,
            rate_override: None,
            next_token: 1,
            active: VecDeque::new(),
            captures: HashMap::new(),
            cancelled: Vec::new(),
            voices: RefCell::new(None),
        };
        b.ensure_host(Arch::X64)?;
        Ok(b)
    }

    /// Every voice with its SAPI details (architecture, family, vendor, and
    /// tags such as [`TAG_OPENEVV`](crate::voices::TAG_OPENEVV)). Listed once per backend and
    /// cached.
    pub fn voice_details(&self) -> Result<Vec<SapiVoice>, SpeechError> {
        if let Some(v) = self.voices.borrow().as_ref() {
            return Ok(v.clone());
        }
        let list = crate::list_voices(&self.config)?;
        *self.voices.borrow_mut() = Some(list.clone());
        Ok(list)
    }

    /// The host executable serving `arch`, if running.
    pub fn host_path(&self, arch: Arch) -> Option<&Path> {
        self.hosts[arch.index()].as_ref().map(|h| h.path.as_path())
    }

    /// The selected voice's family.
    pub fn family(&self) -> Family {
        self.family
    }

    /// The SAPI rate the current parameters map to.
    pub fn sapi_rate(&self) -> i8 {
        self.rate_override
            .unwrap_or_else(|| self.rate_table().rate_for(self.params.rate.wpm()))
    }

    fn rate_table(&self) -> &'static RateTable {
        calibration::table_for(self.family)
    }

    fn arch(&self) -> Arch {
        self.voice.as_ref().map_or(Arch::X64, |v| v.arch)
    }

    fn ensure_host(&mut self, arch: Arch) -> Result<(), SpeechError> {
        if self.hosts[arch.index()].is_some() {
            return Ok(());
        }
        let candidates = crate::host_candidates(&self.config, arch);
        if candidates.is_empty() {
            return Err(unavailable(format!(
                "{} not found (run `cargo xtask sapi-host`, or set {})",
                crate::host_file_name(arch),
                crate::host_env(arch)
            )));
        }
        let mut errors = Vec::new();
        for path in candidates {
            match Host::spawn(&path, self.config.fake_engine, arch) {
                Ok(host) => {
                    log::info!("sapi: {} host {}", arch, path.display());
                    self.hosts[arch.index()] = Some(host);
                    self.applied[arch.index()] = None;
                    return Ok(());
                }
                Err(e) => errors.push(format!("{}: {e}", path.display())),
            }
        }
        Err(unavailable(errors.join("; ")))
    }

    fn sample_rate(&self) -> u32 {
        self.hosts
            .iter()
            .flatten()
            .map(|h| h.sample_rate)
            .next()
            .unwrap_or(crate::host::SAMPLE_RATE)
    }

    fn ensure_player(&mut self) -> Result<(), SpeechError> {
        let rate = self.sample_rate();
        if self.player.as_ref().is_some_and(|(_, r)| *r == rate) {
            return Ok(());
        }
        self.player = None;
        let p = Player::start(self.config.output, Arc::clone(&self.feed), rate)?;
        self.player = Some((p, rate));
        Ok(())
    }

    fn send(&mut self, arch: Arch, req: &Request) -> Result<(), SpeechError> {
        let host = self.hosts[arch.index()]
            .as_mut()
            .ok_or_else(|| SpeechError::Engine("the SAPI host is not running".into()))?;
        host.send(req).map_err(SpeechError::Engine)
    }

    /// Starts the host for the selected voice and sends it the voice and
    /// rate if they changed.
    fn prepare(&mut self) -> Result<Arch, SpeechError> {
        let arch = self.arch();
        self.ensure_host(arch)?;
        let want = Applied {
            token_id: self
                .voice
                .as_ref()
                .map(|v| v.token_id.clone())
                .unwrap_or_default(),
            rate: self.sapi_rate(),
        };
        let prev = self.applied[arch.index()].clone();
        if prev.as_ref() == Some(&want) {
            return Ok(arch);
        }
        if prev.as_ref().is_none_or(|p| p.token_id != want.token_id) {
            self.send(
                arch,
                &Request::SetVoice {
                    token_id: want.token_id.clone(),
                },
            )?;
        }
        if prev.as_ref().is_none_or(|p| p.rate != want.rate) {
            self.send(arch, &Request::SetRate { rate: want.rate })?;
        }
        self.applied[arch.index()] = Some(want);
        Ok(arch)
    }

    fn pitch(&self) -> i8 {
        calibration::sapi_pitch(self.params.pitch.semitones())
    }

    /// Reads everything the hosts have sent so far.
    fn drain_hosts(&mut self) {
        for arch in Arch::ALL {
            while let Some(h) = &self.hosts[arch.index()] {
                let msg = match h.rx.try_recv() {
                    Ok(m) => m,
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => HostMsg::Closed("host exited".into()),
                };
                self.handle(arch, msg);
            }
        }
    }

    fn handle(&mut self, arch: Arch, msg: HostMsg) {
        match msg {
            HostMsg::Closed(why) => self.host_died(arch, &why),
            HostMsg::Reply(r) => self.handle_reply(r),
        }
    }

    fn host_died(&mut self, arch: Arch, why: &str) {
        log::warn!("sapi: {arch} host: {why}");
        self.hosts[arch.index()] = None;
        self.applied[arch.index()] = None;
        let message = format!("the voice stopped unexpectedly ({why})");
        for a in self.active.iter_mut().filter(|a| a.arch == arch && !a.done) {
            a.done = true;
            a.failed = Some(message.clone());
        }
        for c in self.captures.values_mut().filter(|c| !c.done) {
            c.done = true;
            c.failed = Some(message.clone());
        }
        self.advance_queue();
    }

    /// Index of the utterance whose audio goes into the feed now: the
    /// first one not yet ended.
    fn playing_index(&self) -> Option<usize> {
        self.active.iter().position(|a| !a.done)
    }

    /// After an utterance ends, lets the next ones into the feed: pushes
    /// their buffered audio and fixes their start positions.
    fn advance_queue(&mut self) {
        let open = self.playing_index().unwrap_or(self.active.len());
        for i in 0..self.active.len() {
            if i > open {
                break;
            }
            let a = &mut self.active[i];
            if a.start.is_none() {
                a.start = Some(self.feed.pushed());
            }
            if !a.waiting.is_empty() {
                let w = std::mem::take(&mut a.waiting);
                self.feed.push(&w);
                a.total += w.len() as u64;
            }
        }
    }

    fn handle_reply(&mut self, r: Reply) {
        match r {
            Reply::Audio { token, samples } => {
                if let Some(c) = self.captures.get_mut(&token) {
                    c.samples.extend_from_slice(&samples);
                    return;
                }
                let playing = self.playing_index();
                let Some(i) = self.active.iter().position(|a| a.token == token) else {
                    return;
                };
                if Some(i) == playing {
                    let at = self.feed.push(&samples);
                    let a = &mut self.active[i];
                    a.start.get_or_insert(at);
                    a.total += samples.len() as u64;
                } else {
                    self.active[i].waiting.extend_from_slice(&samples);
                }
            }
            Reply::Word {
                token,
                start,
                len,
                sample,
            } => {
                if let Some(c) = self.captures.get_mut(&token) {
                    if let Some(range) = c.map.byte_range(start, len) {
                        if c.filter.accept(&range) {
                            c.words.push((range, sample));
                        }
                    }
                    return;
                }
                if let Some(a) = self.active.iter_mut().find(|a| a.token == token) {
                    if !a.words {
                        return;
                    }
                    if let Some(range) = a.map.byte_range(start, len) {
                        if a.filter.accept(&range) {
                            a.marks.push_back((range, sample));
                        }
                    }
                }
            }
            Reply::End { token, status, .. } => {
                if let Some(c) = self.captures.get_mut(&token) {
                    c.done = true;
                    if status == EndStatus::Failed && c.failed.is_none() {
                        c.failed = Some("synthesis failed".into());
                    }
                    return;
                }
                if let Some(a) = self.active.iter_mut().find(|a| a.token == token) {
                    a.done = true;
                    if status == EndStatus::Failed && a.failed.is_none() {
                        a.failed = Some("synthesis failed".into());
                    }
                    self.advance_queue();
                }
            }
            Reply::Error { token, message } => {
                if token == 0 {
                    log::warn!("sapi: {message}");
                    // A voice that could not be loaded: send it again next
                    // time rather than assuming it is applied.
                    self.applied = [None, None];
                } else if let Some(c) = self.captures.get_mut(&token) {
                    c.failed = Some(message);
                } else if let Some(a) = self.active.iter_mut().find(|a| a.token == token) {
                    a.failed = Some(message);
                }
            }
            Reply::Ready { .. } | Reply::Voice(_) => {}
        }
    }

    /// Emits cancellations and every event the playback clock has reached.
    fn emit(&mut self, sink: &mut dyn EventSink) {
        for id in self.cancelled.drain(..) {
            sink.emit(id, RawEvent::Cancelled);
        }
        let rate = u64::from(self.sample_rate().max(1));
        let played = self.feed.consumed();
        while let Some(a) = self.active.front_mut() {
            let Some(start) = a.start else { break };
            if !sink.is_current(a.id) {
                // A stale utterance (the service moved on without `stop`):
                // nothing it says matters any more.
            } else {
                if !a.started && (played > start || (a.done && played >= start + a.total)) {
                    a.started = true;
                    sink.emit(a.id, RawEvent::Started);
                }
                if a.started {
                    while let Some((range, sample)) = a.marks.front() {
                        if played < start + sample && !(a.done && played >= start + a.total) {
                            break;
                        }
                        let audio_ms = u32::try_from(sample * 1000 / rate).unwrap_or(u32::MAX);
                        sink.emit(
                            a.id,
                            RawEvent::Word {
                                byte_range: range.clone(),
                                audio_ms: Some(audio_ms),
                            },
                        );
                        a.marks.pop_front();
                    }
                }
            }
            if !(a.done && played >= start + a.total) {
                break;
            }
            let Some(a) = self.active.pop_front() else {
                break;
            };
            if !a.started {
                sink.emit(a.id, RawEvent::Started);
            }
            if let Some(e) = a.failed {
                sink.emit(a.id, RawEvent::Error(e));
            }
            sink.emit(a.id, RawEvent::Finished);
        }
    }

    /// Synthesizes `text` with the current voice and parameters, returning
    /// the audio and word offsets instead of playing it.
    pub fn synthesize(&mut self, text: &str) -> Result<Synthesis, SpeechError> {
        let arch = self.prepare()?;
        let token = self.next_token;
        self.next_token += 1;
        self.captures.insert(
            token,
            Capture {
                map: Utf16Map::new(text),
                filter: WordFilter::new(),
                samples: Vec::new(),
                words: Vec::new(),
                done: false,
                failed: None,
            },
        );
        let pitch = self.pitch();
        if let Err(e) = self.send(
            arch,
            &Request::Speak {
                token,
                text: text.to_owned(),
                pitch,
            },
        ) {
            self.captures.remove(&token);
            return Err(e);
        }
        let deadline = Instant::now() + SYNTH_TIMEOUT;
        loop {
            if self.captures.get(&token).is_none_or(|c| c.done) {
                break;
            }
            let left = deadline.saturating_duration_since(Instant::now());
            let msg = match &self.hosts[arch.index()] {
                Some(h) => match h.rx.recv_timeout(left.min(Duration::from_millis(50))) {
                    Ok(m) => m,
                    Err(RecvTimeoutError::Timeout) if left.is_zero() => {
                        self.captures.remove(&token);
                        return Err(SpeechError::Engine("synthesis timed out".into()));
                    }
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => HostMsg::Closed("host exited".into()),
                },
                None => HostMsg::Closed("host is not running".into()),
            };
            self.handle(arch, msg);
        }
        let c = self
            .captures
            .remove(&token)
            .ok_or_else(|| SpeechError::Engine("synthesis lost".into()))?;
        if let Some(e) = c.failed {
            return Err(SpeechError::Engine(e));
        }
        let gain = self.params.volume.fraction();
        let samples = if (gain - 1.0).abs() < f32::EPSILON {
            c.samples
        } else {
            c.samples
                .iter()
                .map(|&s| (f32::from(s) * gain) as i16)
                .collect()
        };
        Ok(Synthesis {
            sample_rate: self.sample_rate(),
            samples,
            words: c.words,
        })
    }

    /// Synthesizes at a given SAPI rate (`-10..=10`), ignoring the wpm
    /// mapping: the calibration measurement uses it.
    pub fn synthesize_at_rate(
        &mut self,
        text: &str,
        sapi_rate: i8,
    ) -> Result<Synthesis, SpeechError> {
        self.rate_override = Some(sapi_rate.clamp(-10, 10));
        let r = self.synthesize(text);
        self.rate_override = None;
        r
    }

    fn select_voice(&mut self, id: Option<&str>) -> Result<(), SpeechError> {
        let Some(id) = id.map(str::trim).filter(|s| !s.is_empty()) else {
            self.voice = None;
            self.family = Family::Microsoft;
            return Ok(());
        };
        let parsed = VoiceId::parse(id).ok_or_else(|| SpeechError::UnknownVoice(id.into()))?;
        // A listed voice gives its family; an id missing from a successful
        // listing is unknown. If listing failed, the id is tried as is.
        let family = match self.voice_details() {
            Ok(list) => list
                .into_iter()
                .find(|v| v.voice.id.eq_ignore_ascii_case(&parsed.to_string()))
                .map(|v| v.family)
                .ok_or_else(|| SpeechError::UnknownVoice(id.into()))?,
            Err(e) => {
                log::warn!("sapi: cannot list voices ({e}); trying {id}");
                Family::of(&VoiceToken {
                    token_id: parsed.token_id.clone(),
                    ..VoiceToken::default()
                })
            }
        };
        self.voice = Some(parsed);
        self.family = family;
        Ok(())
    }
}

impl SpeechBackend for SapiBackend {
    fn id(&self) -> BackendId {
        BACKEND_ID
    }

    fn capabilities(&self) -> Caps {
        let mut caps = Caps::PAUSE | Caps::PITCH | Caps::VOLUME | Caps::SYNTH_TO_FILE | Caps::TONES;
        if self.family.has_word_timing() {
            caps |= Caps::WORD_EVENTS | Caps::AUDIO_CLOCK;
        }
        caps
    }

    fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        Ok(self.voice_details()?.into_iter().map(|v| v.voice).collect())
    }

    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
        if params.voice != self.params.voice {
            self.select_voice(params.voice.as_deref())?;
        }
        self.params = params.clone();
        self.feed.set_gain(params.volume.fraction());
        Ok(())
    }

    fn effective_wpm(&self) -> u16 {
        self.rate_table().wpm_at(self.sapi_rate())
    }

    fn speak(
        &mut self,
        utterance: &Utterance,
        sink: &mut dyn EventSink,
    ) -> Result<(), SpeechError> {
        self.drain_hosts();
        self.emit(sink);
        let arch = self.prepare()?;
        self.ensure_player()?;
        let token = self.next_token;
        self.next_token += 1;
        let pitch = self.pitch();
        self.send(
            arch,
            &Request::Speak {
                token,
                text: utterance.text.clone(),
                pitch,
            },
        )?;
        self.active.push_back(Active {
            id: utterance.id,
            token,
            arch,
            map: Utf16Map::new(&utterance.text),
            filter: WordFilter::new(),
            words: self.family.has_word_timing(),
            marks: VecDeque::new(),
            start: None,
            total: 0,
            waiting: Vec::new(),
            done: false,
            started: false,
            failed: None,
        });
        Ok(())
    }

    fn poll(&mut self, sink: &mut dyn EventSink) {
        self.drain_hosts();
        self.emit(sink);
    }

    fn stop(&mut self) {
        let mut archs = [false, false];
        for a in self.active.drain(..) {
            archs[a.arch.index()] = true;
            self.cancelled.push(a.id);
        }
        for arch in Arch::ALL {
            if archs[arch.index()] {
                if let Err(e) = self.send(arch, &Request::Stop) {
                    log::warn!("sapi: cannot stop the {arch} host: {e}");
                }
            }
        }
        self.feed.clear();
        self.feed.set_paused(false);
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
        crate::wav::write(path, &s.samples, s.sample_rate)
            .map_err(|e| SpeechError::Io(format!("{}: {e}", path.display())))
    }

    fn tone(&mut self, hz: f32, ms: u32) {
        if self.ensure_player().is_ok() {
            if let Some((p, _)) = &self.player {
                p.tone(hz, ms, self.params.volume.fraction());
            }
        }
    }
}
