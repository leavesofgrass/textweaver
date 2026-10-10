//! [`EciBackend`]: the `eci` speech backend.
//!
//! Timing (ADR-0003): `speak` sends the utterance to the host and returns at
//! once. The host synthesizes far faster than real time and streams PCM and
//! index marks back; `poll` hands them to the shared playback client
//! ([`Playback`], ADR-0012), which queues the audio and emits events
//! against the playback clock (samples actually consumed by the output):
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
//! If the host dies (a crash inside the proprietary engine) or hangs (silent
//! for [`STALL_TIMEOUT`] while it owes audio), the utterances it had not
//! finished end with `Error` then `Finished`, and the next `speak` starts a
//! new host.
//!
//! OpenEVV's root dictionary takes about a minute to load, so its host
//! starts without it and speech starts at once. On the first `speak` a
//! second host starts in the background with every dictionary; once it is
//! ready and the first host owes no audio, it takes over (the voice is sent
//! again) and the time it took is logged. A second host that fails is
//! logged and dropped; speech goes on without the root dictionary.

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use textweaver_core::Utterance;
use textweaver_enginehost::protocol::check_version;
use textweaver_enginehost::{Class, HostMsg, HostProcess, HostStart, Playback, Start, Started};
use textweaver_speech::{
    BackendId, Caps, EventSink, FileSynthesis, SpeechBackend, SpeechError, Voice, VoiceParams,
};

use crate::calibration::{self, RatePoint};
use crate::dictionaries::Dictionaries;
use crate::discovery::{self, LibraryChoice};
use crate::host::DictLoad;
use crate::language;
use crate::protocol::{PresetInfo, Reply, Request};
use crate::voices::{self, VoiceParam};
use crate::{BACKEND_ID, EciConfig, words};

/// How long to wait for a new host to report `Ready`.
const READY_TIMEOUT: Duration = Duration::from_secs(10);
/// How long the background host that loads every dictionary may take: the
/// root dictionary took about a minute on OpenEVV 0.3.0.
const FULL_DICTIONARY_TIMEOUT: Duration = Duration::from_secs(300);
/// How long the host may stay silent while it owes audio before the
/// backend treats the engine as hung, kills it, and fails the utterance
/// (Eloquence is known to hang on some inputs).
pub const STALL_TIMEOUT: Duration = Duration::from_secs(10);
/// The engine's default sample rate, used until a host reports its own.
const DEFAULT_RATE: u32 = 11025;
/// The one host this backend runs, as [`Playback`] counts hosts.
const HOST: usize = 0;

/// What the host said at start-up.
#[derive(Clone, Debug)]
struct ReadyInfo {
    sample_rate: u32,
    version: String,
    dialects: Vec<u32>,
    default_dialect: u32,
    presets: Vec<PresetInfo>,
}

/// The host's arguments for `config` and the chosen library; `every_volume`
/// loads OpenEVV's root dictionary too (the background host).
fn host_args(
    config: &EciConfig,
    library: Option<&Path>,
    path: &Path,
    every_volume: bool,
) -> Vec<std::ffi::OsString> {
    let mut args: Vec<std::ffi::OsString> = Vec::new();
    if config.fake_engine {
        args.extend(["--engine".into(), "fake".into()]);
    } else if let Some(lib) = library {
        args.extend(["--library".into(), lib.into()]);
    }
    if let Some(hz) = config.sample_rate {
        args.extend(["--sample-rate".into(), hz.to_string().into()]);
    }
    if let Some(dir) = crate::dictionaries::find_dir(&config.dictionaries, Some(path)) {
        args.extend(["--dictionaries".into(), dir.into()]);
        if !every_volume
            && library
                .is_some_and(|l| discovery::Product::from_path(l) == discovery::Product::OpenEvv)
        {
            // OpenEVV 0.3.0 takes about a minute to load the English root
            // dictionary (68,000 entries; measured October 5, 2026), far
            // past the start deadline: the engine started silent. Its main
            // and abbreviation dictionaries load in milliseconds.
            args.extend([
                "--skip-dictionary-volume".into(),
                (crate::dictionaries::Volume::Root as u8).to_string().into(),
            ]);
        }
    }
    args.extend(config.host_args.iter().cloned());
    args
}

/// How a reply before `Ready` counts: dictionary reports are progress
/// (loading dictionaries on a cold engine takes a while, and each report
/// restarts the wait).
fn classify(r: &Reply) -> Class {
    match r {
        Reply::Ready { protocol, .. } => match check_version(*protocol) {
            Ok(()) => Class::Ready,
            Err(e) => Class::Fail(e),
        },
        Reply::Dictionary { .. } => Class::Progress,
        Reply::Error { message, .. } => Class::Fail(message.clone()),
        other => Class::Fail(format!("host sent {other:?} before Ready")),
    }
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

/// The highlight range of each index mark, per utterance.
type Marks = Vec<Range<u32>>;

/// The ETI-Eloquence backend.
pub struct EciBackend {
    config: EciConfig,
    // Declared before `host`: audio stops before the host goes away.
    playback: Playback<Marks>,
    host: Option<HostProcess<Reply>>,
    ready: Option<ReadyInfo>,
    params: VoiceParams,
    /// Voice selection derived from `params.voice`.
    dialect: Option<u32>,
    preset: u8,
    applied: Option<Applied>,
    rate_table: &'static [RatePoint],
    /// Every dictionary file the engine has loaded, with its status.
    dictionary_loads: Vec<DictLoad>,
    /// The library the running host loaded (not set for the fake engine).
    library: Option<LibraryChoice>,
    /// The installed dialects, worked out once per host start (reading
    /// `eci.ini` when the engine cannot list them), not on every rate
    /// change.
    installed: Vec<u32>,
    /// A host being started (after a crash or a hang), with the library
    /// chosen for it: requests wait in `pending` until it is ready.
    starting: Option<(HostStart<Reply>, Option<LibraryChoice>)>,
    /// Speak requests for the host being started, in order.
    pending: Vec<Request>,
    /// The background host loading every dictionary, and when it began.
    full: Option<(HostStart<Reply>, Instant)>,
    /// That host, ready, waiting for the running one to owe no audio.
    full_ready: Option<(Started<Reply>, Instant)>,
    /// The background host was started (once per backend).
    full_tried: bool,
}

impl std::fmt::Debug for EciBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EciBackend")
            .field("config", &self.config)
            .field("host", &self.host.as_ref().map(|h| h.path().to_path_buf()))
            .field("playback", &self.playback)
            .finish_non_exhaustive()
    }
}

fn log_dictionary(l: &DictLoad) {
    let text = crate::dictionaries::status_text(l.status);
    if l.status == 0 {
        log::info!("eci: dictionary {} (volume {}): {text}", l.path, l.volume);
    } else {
        log::warn!(
            "eci: dictionary {} (volume {}): {text} ({})",
            l.path,
            l.volume,
            l.status
        );
    }
}

fn unavailable(msg: impl Into<String>) -> SpeechError {
    SpeechError::Unavailable(BACKEND_ID, msg.into())
}

/// Maps an index mark to its word's highlight range.
fn mark(index: u32) -> impl FnOnce(&mut Marks) -> Option<Range<u32>> {
    move |marks| marks.get(index as usize).cloned()
}

impl EciBackend {
    /// Starts the host and waits for the engine to report ready. Audio
    /// output opens on the first `speak`.
    pub fn new(config: EciConfig) -> Result<Self, SpeechError> {
        let playback = Playback::new(BACKEND_ID, config.output, DEFAULT_RATE);
        let mut b = EciBackend {
            config,
            playback,
            host: None,
            ready: None,
            params: VoiceParams::default(),
            dialect: None,
            preset: 1,
            applied: None,
            rate_table: calibration::VOXIN,
            dictionary_loads: Vec::new(),
            library: None,
            installed: Vec::new(),
            starting: None,
            pending: Vec::new(),
            full: None,
            full_ready: None,
            full_tried: false,
        };
        // The first start waits for the engine, so a broken installation
        // is reported here (and automatic selection can choose another).
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
        self.host.as_ref().map(HostProcess::path)
    }

    /// Every pronunciation dictionary file the engine has loaded so far
    /// (after reading any pending reports from the host), with its
    /// `ECIDictError` status (0 = loaded).
    pub fn dictionary_loads(&mut self) -> &[DictLoad] {
        let _ = self.poll_start(false);
        self.drain_host();
        &self.dictionary_loads
    }

    /// The ECI library in use, its product, and why it was chosen.
    pub fn library(&self) -> Option<&LibraryChoice> {
        self.library.as_ref()
    }

    /// The ECI speed the current rate maps to.
    pub fn eci_speed(&self) -> i32 {
        calibration::speed_for_wpm(self.rate_table, self.params.rate.wpm())
    }

    /// Changes the extra host arguments used from the next host start on
    /// (see [`EciConfig::host_args`]; tests).
    pub fn set_host_args(&mut self, args: Vec<std::ffi::OsString>) {
        self.config.host_args = args;
    }

    /// Changes how long the host may stay silent while it owes audio
    /// before it counts as hung (`None`: [`STALL_TIMEOUT`]).
    pub fn set_stall_timeout(&mut self, timeout: Option<Duration>) {
        self.config.stall_timeout = timeout;
    }

    /// Starts the host, waiting until it is ready (the first start, and
    /// synthesizing to a file).
    fn ensure_host(&mut self) -> Result<(), SpeechError> {
        self.begin_host()?;
        self.poll_start(true)
    }

    /// Starts a host without waiting for it, unless one is running or
    /// starting (Phase 2: a restart after a crash no longer holds the
    /// speech thread; `poll` finishes it).
    fn begin_host(&mut self) -> Result<(), SpeechError> {
        if self.host.is_some() || self.starting.is_some() {
            return Ok(());
        }
        let (library, candidates) = if self.config.fake_engine {
            (None, discovery::host_candidates(&self.config, None))
        } else {
            let d = discovery::diagnose(&self.config);
            let choice = d.library.map_err(unavailable)?;
            log::info!("eci: using {}", choice.reason);
            (Some(choice), d.hosts)
        };
        if candidates.is_empty() {
            return Err(unavailable(
                "textweaver-eci-host not found (run `cargo xtask hosts`, or set TEXTWEAVER_ECI_HOST)",
            ));
        }
        let lib_path = library.as_ref().map(|c| c.candidate.path.clone());
        let config = self.config.clone();
        let start = HostStart::begin(
            candidates,
            self.config.ready_timeout.unwrap_or(READY_TIMEOUT),
            Box::new(move |path: &Path| {
                HostProcess::spawn(
                    path,
                    host_args(&config, lib_path.as_deref(), path, false),
                    "eci",
                )
            }),
        );
        self.starting = Some((start, library));
        Ok(())
    }

    /// Starts the background host with every dictionary, once, when the
    /// running host left one out (OpenEVV's root dictionary).
    fn begin_full_dictionaries(&mut self) {
        if self.full_tried || self.starting.is_some() {
            return;
        }
        let (Some(lib), Some(host)) = (
            self.library.as_ref().map(|c| c.candidate.path.clone()),
            self.host_path().map(Path::to_path_buf),
        ) else {
            return;
        };
        let skip = std::ffi::OsString::from("--skip-dictionary-volume");
        if !host_args(&self.config, Some(&lib), &host, false).contains(&skip) {
            return;
        }
        self.full_tried = true;
        log::info!("eci: loading every dictionary in the background");
        let config = self.config.clone();
        let start = HostStart::begin(
            vec![host],
            FULL_DICTIONARY_TIMEOUT,
            Box::new(move |path: &Path| {
                HostProcess::spawn(path, host_args(&config, Some(&lib), path, true), "eci")
            }),
        );
        self.full = Some((start, Instant::now()));
    }

    /// Moves the background host on: when it is ready and the running host
    /// owes no audio, it takes over. A failure is logged, never fatal.
    fn poll_full_dictionaries(&mut self) {
        if let Some((start, began)) = self.full.as_mut() {
            match start.poll(classify) {
                Start::Pending => {}
                Start::Ready(started) => {
                    let began = *began;
                    self.full = None;
                    self.full_ready = Some((started, began));
                }
                Start::Failed(why) => {
                    self.full = None;
                    log::warn!(
                        "eci: every dictionary did not load in the background ({why}); speech goes on without the root dictionary"
                    );
                }
            }
        }
        if self.host.is_none()
            || self.starting.is_some()
            || !self.pending.is_empty()
            || self.playback.owes(HOST)
        {
            return;
        }
        let Some((started, began)) = self.full_ready.take() else {
            return;
        };
        if let Some(mut old) = self.host.take() {
            old.shutdown();
        }
        // The new host reports every dictionary again.
        self.dictionary_loads.clear();
        let library = self.library.clone();
        match self.finish_start(started, library) {
            Ok(()) => log::info!(
                "eci: every dictionary in use, {:.1} s after speech started",
                began.elapsed().as_secs_f32()
            ),
            Err(e) => log::warn!("eci: the host with every dictionary did not take over: {e}"),
        }
    }

    /// Moves a start on: reads what the host sent (waiting for the outcome
    /// with `wait`). When it is ready, the queued requests go out; when it
    /// failed, what was queued fails with the reason.
    fn poll_start(&mut self, wait: bool) -> Result<(), SpeechError> {
        let Some((start, _)) = self.starting.as_mut() else {
            return Ok(());
        };
        let outcome = if wait {
            start.wait(classify)
        } else {
            start.poll(classify)
        };
        match outcome {
            Start::Pending => Ok(()),
            Start::Ready(started) => {
                let library = self.starting.take().and_then(|(_, l)| l);
                self.finish_start(started, library)?;
                for req in std::mem::take(&mut self.pending) {
                    self.send(&req)?;
                }
                if self.playback.owes(HOST)
                    && let Some(h) = &mut self.host
                {
                    // The host owes audio from now on.
                    h.touch();
                }
                Ok(())
            }
            Start::Failed(why) => {
                self.starting = None;
                if self.config.dictionaries != Dictionaries::Off {
                    // An engine that cannot load the pronunciation
                    // dictionaries in time (or chokes on one) must not
                    // leave Eloquence silent: start it again without them.
                    // What was queued waits for that host.
                    log::warn!(
                        "eci: the host did not start with the pronunciation dictionaries ({why}); starting it without them"
                    );
                    self.config.dictionaries = Dictionaries::Off;
                    match self.begin_host() {
                        Ok(()) => return self.poll_start(wait),
                        Err(e) => log::warn!("eci: the host did not start again: {e}"),
                    }
                }
                self.pending.clear();
                log::warn!("eci: the host did not start: {why}");
                self.playback
                    .host_died(HOST, &format!("Eloquence could not start ({why})"));
                Err(unavailable(why))
            }
        }
    }

    /// A host reported `Ready`: takes it into use.
    fn finish_start(
        &mut self,
        started: Started<Reply>,
        library: Option<LibraryChoice>,
    ) -> Result<(), SpeechError> {
        let Started {
            path,
            process,
            ready,
            early,
        } = started;
        let Reply::Ready {
            sample_rate,
            version,
            dialects,
            default_dialect,
            presets,
            ..
        } = ready
        else {
            return Err(SpeechError::Engine("the host did not report Ready".into()));
        };
        let ready = ReadyInfo {
            sample_rate,
            version,
            dialects,
            default_dialect,
            presets,
        };
        for r in early {
            if let Reply::Dictionary {
                dialect,
                volume,
                status,
                path,
            } = r
            {
                let load = DictLoad {
                    dialect,
                    volume,
                    status,
                    path,
                };
                log_dictionary(&load);
                self.dictionary_loads.push(load);
            }
        }
        log::info!(
            "eci: {} (ECI {}, {} Hz)",
            path.display(),
            ready.version,
            ready.sample_rate
        );
        self.rate_table = calibration::table_for(&ready.version);
        self.library = library.map(|mut c| {
            c.candidate.product = c.candidate.product.with_version(&ready.version);
            c
        });
        self.playback.set_sample_rate(ready.sample_rate);
        self.host = Some(process);
        self.ready = Some(ready);
        self.find_installed_dialects();
        self.applied = None;
        self.apply_voice()
    }

    fn send(&mut self, req: &Request) -> Result<(), SpeechError> {
        let host = self
            .host
            .as_mut()
            .ok_or_else(|| SpeechError::Engine("Eloquence host is not running".into()))?;
        host.send(req).map_err(SpeechError::Engine)
    }

    /// Works out the installed dialects for the host that just started.
    fn find_installed_dialects(&mut self) {
        let library = self.library.as_ref().map(|c| c.candidate.path.as_path());
        self.installed = self
            .ready
            .as_ref()
            .map(|ready| Self::installed_dialects(ready, library))
            .unwrap_or_default();
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

    /// Reads everything the host has sent so far, and kills it if it hung.
    fn drain_host(&mut self) {
        while let Some(msg) = self.host.as_mut().and_then(HostProcess::try_recv) {
            self.handle(msg);
        }
        let stall = self.stall_timeout();
        let owes = self.playback.owes(HOST);
        if self.host.as_ref().is_some_and(|h| h.stalled(owes, stall)) {
            self.kill_host("Eloquence stopped responding");
        }
    }

    fn stall_timeout(&self) -> Duration {
        self.config.stall_timeout.unwrap_or(STALL_TIMEOUT)
    }

    /// Kills a hung host; everything it owed fails.
    fn kill_host(&mut self, why: &str) {
        if let Some(mut h) = self.host.take() {
            h.kill();
        }
        self.host_died(why);
    }

    fn handle(&mut self, msg: HostMsg<Reply>) {
        match msg {
            HostMsg::Closed(why) => self.host_died(&why),
            HostMsg::Reply(r) => self.handle_reply(r),
        }
    }

    fn host_died(&mut self, why: &str) {
        log::warn!("eci: {why}");
        self.host = None;
        self.applied = None;
        self.playback
            .host_died(HOST, &format!("Eloquence stopped unexpectedly ({why})"));
    }

    fn handle_reply(&mut self, r: Reply) {
        match r {
            Reply::Audio { token, samples } => self.playback.on_audio(token, &samples),
            Reply::Mark {
                token,
                index,
                sample,
            } => self.playback.on_word(token, sample, mark(index)),
            Reply::End { token, status, .. } => self.playback.on_end(token, status),
            Reply::Error { token, message } => {
                if let Err(message) = self.playback_error(token, message) {
                    log::warn!("eci: {message}");
                }
            }
            Reply::Dictionary {
                dialect,
                volume,
                status,
                path,
            } => {
                let load = DictLoad {
                    dialect,
                    volume,
                    status,
                    path,
                };
                log_dictionary(&load);
                self.dictionary_loads.push(load);
            }
            Reply::Ready { .. } => log::warn!("eci: unexpected Ready from host"),
        }
    }

    /// Attaches an error to its utterance; gives it back when no utterance
    /// has that token.
    fn playback_error(&mut self, token: u64, message: String) -> Result<(), String> {
        if token != 0 && self.playback.on_error(token, message.clone()) {
            Ok(())
        } else {
            Err(message)
        }
    }

    /// Synthesizes `text` without playing it: audio (volume applied) and
    /// each word's first sample. Speech in progress keeps playing.
    pub fn synthesize(&mut self, text: &str) -> Result<Synthesis, SpeechError> {
        self.drain_host();
        self.ensure_host()?;
        self.apply_voice()?;
        let ws = words::words(text);
        let token = self.playback.next_token();
        self.playback.capture(
            token,
            HOST,
            ws.iter().map(|w| w.highlight.clone()).collect(),
        );
        let sent = self.send(&Request::Speak {
            token,
            pieces: words::pieces(text, &ws),
        });
        if let Err(e) = sent {
            self.playback.take_capture(token);
            return Err(e);
        }
        if let Some(h) = &mut self.host {
            h.touch();
        }
        let stall = self.stall_timeout();
        while self.playback.capture_done(token) == Some(false) {
            let msg = match &mut self.host {
                Some(h) => match h.recv_timeout(stall) {
                    Some(m) => m,
                    None => {
                        self.kill_host("Eloquence stopped responding");
                        continue;
                    }
                },
                None => HostMsg::Closed("host is not running".into()),
            };
            self.handle(msg);
        }
        let cap = self
            .playback
            .take_capture(token)
            .ok_or_else(|| SpeechError::Engine("synthesis lost".into()))?;
        if let Some(e) = cap.failed {
            return Err(SpeechError::Engine(e));
        }
        Ok(Synthesis {
            sample_rate: self.playback.sample_rate(),
            samples: cap.samples,
            words: cap.words,
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
            | Caps::SYNTH_TO_FILE
            | Caps::NATIVE_NORMALIZATION
            | Caps::PLAYBACK_EVENTS
            | Caps::SILENCE;
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
        Ok(voices::voice_list(&self.installed, &ready.presets))
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
            if self.ready.is_some() && !self.installed.contains(&d.code) {
                return Err(SpeechError::UnknownVoice(format!(
                    "{} ({} is not installed)",
                    params.voice.clone().unwrap_or_default(),
                    d.name
                )));
            }
            self.dialect = Some(code);
        } else if params.voice.is_none() {
            self.dialect = None;
        }
        if let Some(p) = sel.preset {
            self.preset = p;
        }
        self.params = params.clone();
        self.playback.set_gain(params.volume.fraction());
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
        self.playback.emit(sink);
        self.begin_host()?;
        self.poll_start(false)?;
        self.playback.ensure_player()?;
        let ws = words::words(&utterance.text);
        let token = self.playback.next_token();
        let req = Request::Speak {
            token,
            pieces: words::pieces(&utterance.text, &ws),
        };
        if self.starting.is_some() {
            // Sent when the host is ready (poll).
            self.pending.push(req);
        } else {
            self.apply_voice()?;
            self.send(&req)?;
            if !self.playback.owes(HOST)
                && let Some(h) = &mut self.host
            {
                // The host starts owing audio now: its stall timer starts
                // here.
                h.touch();
            }
        }
        self.playback.enqueue(
            utterance.id,
            token,
            HOST,
            ws.into_iter().map(|w| w.highlight).collect(),
        );
        // Speech has started: now the slow dictionary may load.
        self.begin_full_dictionaries();
        Ok(())
    }

    fn poll(&mut self, sink: &mut dyn EventSink) {
        if let Err(e) = self.poll_start(false) {
            log::warn!("eci: {e}");
        }
        self.drain_host();
        self.playback.emit(sink);
        self.poll_full_dictionaries();
    }

    fn silence_after(&mut self, id: textweaver_speech::core::UtteranceId, ms: u32) {
        // Structural pauses play as silence in the shared playback client.
        self.playback.silence_after(id, ms);
    }

    fn stop(&mut self) {
        // Utterances waiting for a starting host are simply not sent.
        self.pending.clear();
        let hosts = self.playback.stop();
        if !hosts.is_empty()
            && self.host.is_some()
            && let Err(e) = self.send(&Request::Stop)
        {
            log::warn!("eci: {e}");
        }
    }

    /// Kills the host and closes the audio output; the next `speak` starts
    /// a new host and reopens the device (a device that stopped taking
    /// samples, a host that stopped answering).
    fn reset(&mut self) {
        self.stop();
        self.playback.close();
        if let Some((mut s, _)) = self.starting.take() {
            s.cancel();
        }
        if let Some((mut s, _)) = self.full.take() {
            s.cancel();
        }
        if let Some((mut s, _)) = self.full_ready.take() {
            s.process.kill();
        }
        if let Some(mut h) = self.host.take() {
            h.kill();
        }
        self.applied = None;
    }

    fn pause(&mut self) -> Result<(), SpeechError> {
        self.playback.pause();
        Ok(())
    }

    fn resume(&mut self) -> Result<(), SpeechError> {
        self.playback.resume();
        Ok(())
    }

    fn synthesize_to_file(&mut self, text: &str, path: &Path) -> Result<(), SpeechError> {
        let s = self.synthesize(text)?;
        textweaver_enginehost::wav::write(path, &s.samples, s.sample_rate)
            .map_err(|e| SpeechError::Io(e.to_string()))
    }

    /// Writes the utterance as a WAV and reports each word at its index
    /// mark's sample offset in that file (ADR-0011): the word timings are
    /// exact on the file's own clock.
    fn synthesize_utterance(
        &mut self,
        utterance: &Utterance,
        path: &Path,
    ) -> Result<FileSynthesis, SpeechError> {
        let s = self.synthesize(&utterance.text)?;
        textweaver_enginehost::wav::write(path, &s.samples, s.sample_rate)
            .map_err(|e| SpeechError::Io(format!("{}: {e}", path.display())))?;
        Ok(FileSynthesis {
            words: textweaver_enginehost::word_timings(&s.words, s.sample_rate),
        })
    }

    fn tone(&mut self, hz: f32, ms: u32) {
        self.playback.tone(hz, ms);
    }
}

impl Drop for EciBackend {
    fn drop(&mut self) {
        // Stop audio before the host goes away.
        self.playback.close();
        if let Some((mut s, _)) = self.full.take() {
            s.cancel();
        }
        if let Some((mut s, _)) = self.full_ready.take() {
            s.process.kill();
        }
        if let Some(mut h) = self.host.take() {
            h.shutdown();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::{LibraryCandidate, Product, Source};

    /// OpenEVV's root dictionary takes about a minute to load, so the host
    /// leaves it out; other engines load every volume.
    #[test]
    fn openevv_hosts_leave_out_the_root_dictionary() {
        let dir = tempfile::tempdir().unwrap();
        let config = EciConfig {
            dictionaries: Dictionaries::Dir(dir.path().to_path_buf()),
            ..EciConfig::default()
        };
        let host = Path::new("textweaver-eci-host");
        let skip = |lib: &str, every: bool| {
            host_args(&config, Some(Path::new(lib)), host, every)
                .iter()
                .any(|a| a == "--skip-dictionary-volume")
        };
        assert!(skip(r"C:\Program Files\OpenEVV\lib\x86_64\eci.dll", false));
        assert!(!skip("/usr/lib/libvoxin.so.1", false));
        // The background host loads the root dictionary too.
        assert!(!skip(r"C:\Program Files\OpenEVV\lib\x86_64\eci.dll", true));
        let off = EciConfig {
            dictionaries: Dictionaries::Off,
            ..EciConfig::default()
        };
        assert!(
            !host_args(
                &off,
                Some(Path::new(r"C:\Program Files\OpenEVV\eci.dll")),
                host,
                false
            )
            .iter()
            .any(|a| a == "--skip-dictionary-volume")
        );
    }

    /// A backend whose engine lists no dialects (as some ECI builds), with
    /// an `eci.ini` next to its library, and no host.
    fn without_host(dir: &Path) -> EciBackend {
        let syn = |name: &str| {
            let p = dir.join(name);
            std::fs::write(&p, b"").unwrap();
            p.display().to_string()
        };
        let ini = format!(
            "[1.0]\nPath={}\n[1.1]\nPath={}\n",
            syn("enu.syn"),
            syn("eng.syn")
        );
        std::fs::write(dir.join("eci.ini"), ini).unwrap();
        let mut b = EciBackend {
            config: EciConfig::default(),
            playback: Playback::new(BACKEND_ID, crate::AudioOutput::Null { speed: 1.0 }, 8000),
            host: None,
            ready: Some(ReadyInfo {
                sample_rate: 8000,
                version: "6.1".into(),
                dialects: Vec::new(),
                default_dialect: 0x0001_0000,
                presets: Vec::new(),
            }),
            params: VoiceParams::default(),
            dialect: None,
            preset: 1,
            applied: None,
            rate_table: calibration::VOXIN,
            dictionary_loads: Vec::new(),
            library: Some(LibraryChoice {
                candidate: LibraryCandidate {
                    path: dir.join("eci.dll"),
                    product: Product::Unknown,
                    source: Source::Option,
                    exists: true,
                },
                arch: None,
                reason: "test".into(),
            }),
            installed: Vec::new(),
            starting: None,
            pending: Vec::new(),
            full: None,
            full_ready: None,
            full_tried: false,
        };
        b.find_installed_dialects();
        b
    }

    #[test]
    fn installed_dialects_are_read_once_per_host_start_not_per_rate_change() {
        if std::env::var_os("ECIINI").is_some() {
            // The machine points ECI elsewhere; this test builds its own.
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let mut b = without_host(dir.path());
        let british = VoiceParams {
            voice: Some("en-GB".into()),
            ..VoiceParams::default()
        };
        b.set_params(&british).unwrap();
        // Before: every rate change read eci.ini and checked every language
        // file again. Now the list from the host start is used, so a
        // missing file changes nothing until the host restarts.
        std::fs::remove_file(dir.path().join("eci.ini")).unwrap();
        for wpm in [150, 200, 250] {
            let p = VoiceParams {
                rate: textweaver_core::Rate::Wpm(wpm),
                ..british.clone()
            };
            b.set_params(&p).unwrap();
        }
        assert_eq!(
            b.voices().unwrap().len(),
            2 * 8,
            "two dialects, eight presets"
        );
        // A new host start works it out again.
        b.find_installed_dialects();
        assert!(b.set_params(&british).is_err(), "only the default is left");
    }
}
