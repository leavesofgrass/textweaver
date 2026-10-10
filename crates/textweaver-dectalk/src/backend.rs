//! [`DectalkBackend`]: the `dectalk` speech backend.
//!
//! Timing (ADR-0003): `speak` sends the utterance to the host and returns at
//! once. The host has DECtalk synthesize it into memory and streams PCM and
//! index marks back; `poll` hands them to the shared playback client
//! ([`Playback`], ADR-0012), which emits events against the playback clock:
//! `Started` at the first sample played, `Word { byte_range, audio_ms }`
//! when the audio reaches the word's index mark, `Finished` after the last
//! sample, and `Cancelled` for every utterance a `stop` discarded.
//!
//! Parameters (ADR-0004, [`voices`](mod@crate::voices)): the voice selects a
//! speaker, the rate passes to DECtalk in words per minute (DECtalk's own
//! unit, so it is also the effective rate), pitch shifts the speaker's
//! average pitch, and volume is a playback gain (immediate, and applied to
//! files).
//!
//! If the host dies or hangs (silent for [`STALL_TIMEOUT`] while it owes
//! audio), the utterances it had not finished end with `Error` then
//! `Finished`, and the next request starts a new host.

use std::ops::Range;
use std::path::Path;
use std::time::Duration;

use textweaver_core::Utterance;
use textweaver_enginehost::protocol::check_version;
use textweaver_enginehost::{Class, HostMsg, HostProcess, HostStart, Playback, Start, Started};
use textweaver_speech::{
    BackendId, Caps, EventSink, FileSynthesis, SpeechBackend, SpeechError, Voice, VoiceParams,
};

use crate::discovery::{self, LibraryChoice};
use crate::protocol::{Reply, Request};
use crate::voices::{self, Speaker};
use crate::{AudioOutput, BACKEND_ID, DectalkConfig, words};

/// How long to wait for a new host to report `Ready`.
const READY_TIMEOUT: Duration = Duration::from_secs(10);
/// How long the host may stay silent while it owes audio before the
/// backend treats the engine as hung, kills it, and fails the utterance.
pub const STALL_TIMEOUT: Duration = Duration::from_secs(10);
/// DECtalk's sample rate, used until a host reports its own.
const DEFAULT_SAMPLE_RATE: u32 = 11_025;
/// The one host this backend runs, as [`Playback`] counts hosts.
const HOST: usize = 0;

/// What the host said at start-up.
#[derive(Clone, Debug)]
struct ReadyInfo {
    sample_rate: u32,
    version: String,
    engine: String,
}

/// Synthesized audio for one text, with each word's first sample.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Synthesis {
    /// Sample rate in Hz (16-bit mono).
    pub sample_rate: u32,
    /// The audio (volume applied).
    pub samples: Vec<i16>,
    /// Each word's byte range in the text and the sample at which DECtalk
    /// reached its index mark, in order.
    pub words: Vec<(Range<u32>, u64)>,
}

/// The voice as last sent to the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Applied {
    speaker: u8,
    rate: u16,
    pitch: u16,
}

/// The highlight range of each index mark, per utterance.
type Marks = Vec<Range<u32>>;

fn unavailable(msg: impl Into<String>) -> SpeechError {
    SpeechError::Unavailable(BACKEND_ID, msg.into())
}

/// Maps an index mark to its word's highlight range.
fn mark(index: u32) -> impl FnOnce(&mut Marks) -> Option<Range<u32>> {
    move |marks| marks.get(index as usize).cloned()
}

/// The host's arguments for `config` and the chosen library.
fn host_args(config: &DectalkConfig, library: Option<&Path>) -> Vec<std::ffi::OsString> {
    let mut args: Vec<std::ffi::OsString> = Vec::new();
    if config.fake_engine {
        args.extend(["--engine".into(), "fake".into()]);
    } else if let Some(lib) = library {
        args.extend(["--library".into(), lib.into()]);
    }
    args.extend(config.host_args.iter().cloned());
    args
}

/// How a reply before `Ready` counts.
fn classify(r: &Reply) -> Class {
    match r {
        Reply::Ready { protocol, .. } => match check_version(*protocol) {
            Ok(()) => Class::Ready,
            Err(e) => Class::Fail(e),
        },
        Reply::Error { message, .. } => Class::Fail(message.clone()),
        other => Class::Fail(format!("host sent {other:?} before Ready")),
    }
}

/// The DECtalk backend.
pub struct DectalkBackend {
    config: DectalkConfig,
    // Declared before `host`: audio stops before the host goes away.
    playback: Playback<Marks>,
    host: Option<HostProcess<Reply>>,
    ready: Option<ReadyInfo>,
    params: VoiceParams,
    speaker: Speaker,
    applied: Option<Applied>,
    /// The library the running host loaded (not set for the fake engine).
    library: Option<LibraryChoice>,
    /// A host being started (after a crash or a hang), with the library
    /// chosen for it: requests wait in `pending` until it is ready.
    starting: Option<(HostStart<Reply>, Option<LibraryChoice>)>,
    /// Speak requests for the host being started, in order.
    pending: Vec<Request>,
}

impl std::fmt::Debug for DectalkBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DectalkBackend")
            .field("config", &self.config)
            .field("host", &self.host.as_ref().map(|h| h.path().to_path_buf()))
            .field("speaker", &self.speaker)
            .field("playback", &self.playback)
            .finish_non_exhaustive()
    }
}

impl DectalkBackend {
    /// Finds DECtalk, starts the host, and waits for it to report ready.
    /// Audio output opens on the first `speak`.
    pub fn new(config: DectalkConfig) -> Result<Self, SpeechError> {
        let playback = Playback::new(BACKEND_ID, config.output, DEFAULT_SAMPLE_RATE);
        let mut b = DectalkBackend {
            config,
            playback,
            host: None,
            ready: None,
            params: VoiceParams::default(),
            speaker: Speaker::Paul,
            applied: None,
            library: None,
            starting: None,
            pending: Vec::new(),
        };
        // The first start waits, so a broken installation is reported here.
        b.ensure_host()?;
        Ok(b)
    }

    /// Changes the extra host arguments used from the next host start on
    /// (see [`DectalkConfig::host_args`]; tests).
    pub fn set_host_args(&mut self, args: Vec<std::ffi::OsString>) {
        self.config.host_args = args;
    }

    /// The engine's version, as the host reported it.
    pub fn engine_version(&self) -> Option<&str> {
        self.ready.as_ref().map(|r| r.version.as_str())
    }

    /// `dectalk`, or `fake` for the test engine.
    pub fn engine(&self) -> Option<&str> {
        self.ready.as_ref().map(|r| r.engine.as_str())
    }

    /// The engine's sample rate.
    pub fn sample_rate(&self) -> Option<u32> {
        self.ready.as_ref().map(|r| r.sample_rate)
    }

    /// The host executable in use.
    pub fn host_path(&self) -> Option<&Path> {
        self.host.as_ref().map(HostProcess::path)
    }

    /// The DECtalk library in use and why it was chosen.
    pub fn library(&self) -> Option<&LibraryChoice> {
        self.library.as_ref()
    }

    /// The selected speaker.
    pub fn speaker(&self) -> Speaker {
        self.speaker
    }

    /// Starts the host, waiting until it is ready (the first start, and
    /// synthesizing to a file).
    fn ensure_host(&mut self) -> Result<(), SpeechError> {
        self.begin_host()?;
        self.poll_start(true)
    }

    /// Starts a host without waiting for it, unless one is running or
    /// starting; `poll` finishes the start (Phase 2).
    fn begin_host(&mut self) -> Result<(), SpeechError> {
        if self.host.is_some() || self.starting.is_some() {
            return Ok(());
        }
        let (library, candidates) = if self.config.fake_engine {
            (None, discovery::host_candidates(&self.config, None))
        } else {
            let d = discovery::diagnose(&self.config);
            let choice = d.library.map_err(unavailable)?;
            log::info!("dectalk: using {}", choice.reason);
            (Some(choice), d.hosts)
        };
        if candidates.is_empty() {
            let arch = library.as_ref().and_then(|c| c.arch);
            let which = if arch == Some(discovery::Arch::X86) {
                discovery::HOST_NAME_X86
            } else {
                discovery::HOST_NAME
            };
            return Err(unavailable(format!(
                "{which} not found (run `cargo xtask hosts`, or set {})",
                crate::HOST_ENV
            )));
        }
        let args = host_args(
            &self.config,
            library.as_ref().map(|c| c.candidate.path.as_path()),
        );
        let start = HostStart::begin(
            candidates,
            READY_TIMEOUT,
            Box::new(move |path: &Path| HostProcess::spawn(path, &args, "dectalk")),
        );
        self.starting = Some((start, library));
        Ok(())
    }

    /// Moves a start on (waiting for the outcome with `wait`): queued
    /// requests go out when the host is ready, and fail with the reason
    /// when it could not start.
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
                    h.touch();
                }
                Ok(())
            }
            Start::Failed(why) => {
                self.starting = None;
                self.pending.clear();
                log::warn!("dectalk: the host did not start: {why}");
                self.playback
                    .host_died(HOST, &format!("DECtalk could not start ({why})"));
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
            ..
        } = started;
        let Reply::Ready {
            sample_rate,
            version,
            engine,
            ..
        } = ready
        else {
            return Err(SpeechError::Engine("the host did not report Ready".into()));
        };
        let ready = ReadyInfo {
            sample_rate,
            version,
            engine,
        };
        log::info!(
            "dectalk: {} ({}, {} Hz)",
            path.display(),
            ready.version,
            ready.sample_rate
        );
        self.playback.set_sample_rate(ready.sample_rate);
        self.host = Some(process);
        self.ready = Some(ready);
        self.library = library;
        self.applied = None;
        self.apply_voice()
    }

    fn send(&mut self, req: &Request) -> Result<(), SpeechError> {
        let host = self
            .host
            .as_mut()
            .ok_or_else(|| SpeechError::Engine("the DECtalk host is not running".into()))?;
        host.send(req).map_err(SpeechError::Engine)
    }

    /// The voice the host should have now.
    fn wanted(&self) -> Applied {
        Applied {
            speaker: self.speaker.index(),
            rate: voices::rate(self.params.rate.wpm()),
            pitch: voices::pitch_hz(self.speaker, self.params.pitch.semitones()),
        }
    }

    /// Sends the voice to the host when it differs from what it has.
    fn apply_voice(&mut self) -> Result<(), SpeechError> {
        if self.host.is_none() {
            return Ok(());
        }
        let want = self.wanted();
        if self.applied == Some(want) {
            return Ok(());
        }
        self.send(&Request::SetVoice {
            speaker: want.speaker,
            rate: want.rate,
            pitch: want.pitch,
        })?;
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
            self.kill_host("DECtalk stopped responding");
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
        log::warn!("dectalk: {why}");
        self.host = None;
        self.applied = None;
        self.playback
            .host_died(HOST, &format!("DECtalk stopped unexpectedly ({why})"));
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
                if token == 0 || !self.playback.on_error(token, message.clone()) {
                    log::warn!("dectalk: {message}");
                }
            }
            Reply::Ready { .. } => log::warn!("dectalk: unexpected Ready from host"),
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
                        self.kill_host("DECtalk stopped responding");
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

impl SpeechBackend for DectalkBackend {
    fn id(&self) -> BackendId {
        BACKEND_ID
    }

    fn capabilities(&self) -> Caps {
        let mut caps = crate::CAPS;
        if self.config.output == AudioOutput::Device {
            caps |= Caps::TONES;
        }
        caps
    }

    fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        Ok(voices::voice_list())
    }

    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
        // Another engine's voice (a SAPI voice saved during a fallback)
        // keeps the current speaker without an error; rate, pitch and
        // volume always apply.
        match &params.voice {
            None => self.speaker = Speaker::Paul,
            Some(id) => match Speaker::parse(id) {
                Some(speaker) => self.speaker = speaker,
                None => log::debug!(
                    "dectalk: voice {id} is another engine's; keeping the current speaker"
                ),
            },
        }
        self.params = params.clone();
        self.playback.set_gain(params.volume.fraction());
        self.apply_voice()
    }

    fn effective_wpm(&self) -> u16 {
        voices::rate(self.params.rate.wpm())
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
        Ok(())
    }

    fn poll(&mut self, sink: &mut dyn EventSink) {
        if let Err(e) = self.poll_start(false) {
            log::warn!("dectalk: {e}");
        }
        self.drain_host();
        self.playback.emit(sink);
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
            log::warn!("dectalk: {e}");
        }
    }

    /// Kills the host and closes the audio output; the next `speak` starts
    /// a new host (in `poll`) and reopens the device.
    fn reset(&mut self) {
        self.stop();
        self.playback.close();
        if let Some((mut s, _)) = self.starting.take() {
            s.cancel();
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
            .map_err(|e| SpeechError::Io(format!("{}: {e}", path.display())))
    }

    /// Writes the utterance as a WAV and reports each word at its index
    /// mark's sample offset in that file (ADR-0011).
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

impl Drop for DectalkBackend {
    fn drop(&mut self) {
        // Stop audio before the host goes away.
        self.playback.close();
        if let Some(mut h) = self.host.take() {
            h.shutdown();
        }
    }
}
