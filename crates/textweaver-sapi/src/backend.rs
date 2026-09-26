//! `SapiBackend`: the `sapi` speech backend.
//!
//! Timing (ADR-0003): `speak` sends the utterance to the host for the
//! selected voice's architecture and returns at once. The host synthesizes
//! faster than real time and streams PCM and word boundaries back; `poll`
//! hands them to the shared playback client (`Playback`, ADR-0012), which
//! queues the audio and emits events against the playback clock (samples
//! actually consumed by the output):
//! - `Started` when the utterance's first sample is played;
//! - `Word { byte_range, audio_ms }` when the audio reaches the word,
//!   `audio_ms` being its offset from the utterance's first sample (pauses
//!   excluded). SAPI positions are UTF-16 offsets, mapped to UTF-8 byte
//!   ranges of `Utterance::text` (`crate::words`); repeated events on one
//!   range are emitted once;
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
//! measured tables in `crate::calibration`; pitch uses `<pitch absmiddle>`;
//! volume is a playback gain (immediate, and applied to
//! `synthesize_to_file`); the voice id selects the host architecture and
//! the SAPI token.
//!
//! Capabilities follow the selected voice: voices whose word events cannot
//! drive a highlight (Code Factory Eloquence through SAPI, ADR-0007) are
//! spoken without word events, and `capabilities` drops `WORD_EVENTS` and
//! `AUDIO_CLOCK` while one is selected, so the service falls back to timer
//! pacing; Eloquence voices add `NATIVE_NORMALIZATION`. The backend always
//! owns playback, so it declares `PLAYBACK_EVENTS`.
//!
//! If a host dies (a crash inside a third-party engine) or hangs (silent
//! for `STALL_TIMEOUT` while it owes audio), the utterances it had not
//! finished end with `Error` then `Finished`, and the next `speak` starts a
//! new host.

use std::cell::RefCell;
use std::ops::Range;
use std::path::Path;
use std::time::{Duration, Instant};

use textweaver_core::Utterance;
use textweaver_enginehost::protocol::check_version;
use textweaver_enginehost::{HostMsg, HostProcess, Playback};
use textweaver_speech::{
    BackendId, Caps, EventSink, FileSynthesis, SpeechBackend, SpeechError, Voice, VoiceParams,
};

use crate::audio::AudioOutput;
use crate::calibration::{self, RateTable};
use crate::protocol::{Reply, Request, VoiceToken};
use crate::voices::{Arch, Family, SapiVoice, VoiceId};
use crate::words::{Utf16Map, WordFilter};
use crate::{BACKEND_ID, SapiConfig};

/// How long to wait for a new host to report `Ready`.
const READY_TIMEOUT: Duration = Duration::from_secs(10);
/// How long a voice listing may take.
const LIST_TIMEOUT: Duration = Duration::from_secs(15);
/// How long a host may stay silent while it owes audio before the backend
/// treats it as hung, kills it, and fails what it owed. The host gives up
/// on a silent voice itself after 30 s and reports an error; this catches a
/// host that stopped answering altogether (and bounds `synthesize`, as the
/// 60-second limit before the shared engine host did).
const STALL_TIMEOUT: Duration = Duration::from_secs(60);

/// A running host process.
struct Host {
    process: HostProcess<Reply>,
    sample_rate: u32,
}

/// Waits for `Ready` from a host that should run `arch`; returns the
/// sample rate.
fn await_ready(host: &mut HostProcess<Reply>, arch: Arch) -> Result<u32, String> {
    match host.recv_timeout(READY_TIMEOUT) {
        Some(HostMsg::Reply(Reply::Ready {
            protocol,
            sample_rate,
            arch: got,
            ..
        })) => {
            if got != arch.as_str() {
                return Err(format!("host is {got}, expected {arch}"));
            }
            check_version(protocol)?;
            Ok(sample_rate)
        }
        Some(HostMsg::Reply(Reply::Error { message, .. })) => Err(message),
        Some(HostMsg::Reply(other)) => Err(format!("host sent {other:?} before Ready")),
        Some(HostMsg::Closed(why)) => Err(why),
        None => Err("host did not start in time".into()),
    }
}

impl Host {
    fn spawn(path: &Path, fake: bool, arch: Arch) -> Result<Host, String> {
        let args: &[&str] = if fake {
            &["--engine", "fake", "--report-arch", arch.as_str()]
        } else {
            &[]
        };
        let mut process = HostProcess::spawn(path, args, "sapi")?;
        match await_ready(&mut process, arch) {
            Ok(sample_rate) => Ok(Host {
                process,
                sample_rate,
            }),
            Err(e) => {
                process.kill();
                Err(e)
            }
        }
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
    let mut host = HostProcess::<Reply>::spawn(path, &args, "sapi")?;
    let result = (|| {
        await_ready(&mut host, arch)?;
        let deadline = Instant::now() + LIST_TIMEOUT;
        let mut out = Vec::new();
        loop {
            match host.recv_until(deadline) {
                Some(HostMsg::Reply(Reply::Voice(v))) => out.push(v),
                Some(HostMsg::Reply(Reply::Error { message, .. })) => return Err(message),
                Some(HostMsg::Reply(_)) => {}
                Some(HostMsg::Closed(_)) => return Ok(out),
                None => return Err("voice listing timed out".into()),
            }
        }
    })();
    host.kill();
    result
}

/// Per-utterance word mapping: UTF-16 positions to byte ranges, with
/// repeats removed.
#[derive(Debug)]
struct Words {
    map: Utf16Map,
    filter: WordFilter,
    /// Report word events (false for voices without word timing).
    enabled: bool,
}

impl Words {
    fn new(text: &str, enabled: bool) -> Self {
        Words {
            map: Utf16Map::new(text),
            filter: WordFilter::new(),
            enabled,
        }
    }

    /// Maps a word boundary to a byte range, or `None` to drop it.
    fn map(&mut self, start: u32, len: u32) -> Option<Range<u32>> {
        if !self.enabled {
            return None;
        }
        let range = self.map.byte_range(start, len)?;
        self.filter.accept(&range).then_some(range)
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
    // Declared before `hosts`: audio stops before the hosts go away.
    playback: Playback<Words>,
    hosts: [Option<Host>; 2],
    applied: [Option<Applied>; 2],
    params: VoiceParams,
    /// The selected voice; `None` is the system default (x64 host).
    voice: Option<VoiceId>,
    family: Family,
    /// A SAPI rate forced by `SapiBackend::synthesize_at_rate`.
    rate_override: Option<i8>,
    voices: RefCell<Option<Vec<SapiVoice>>>,
}

impl std::fmt::Debug for SapiBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SapiBackend")
            .field("config", &self.config)
            .field("voice", &self.voice)
            .field("playback", &self.playback)
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
        let playback = Playback::new(BACKEND_ID, config.output, crate::host::SAMPLE_RATE);
        let mut b = SapiBackend {
            config,
            playback,
            hosts: [None, None],
            applied: [None, None],
            params: VoiceParams::default(),
            voice: None,
            family: Family::Microsoft,
            rate_override: None,
            voices: RefCell::new(None),
        };
        b.ensure_host(Arch::X64)?;
        Ok(b)
    }

    /// Every voice with its SAPI details (architecture, family, vendor, and
    /// tags such as [`TAG_OPENEVV`](crate::voices::TAG_OPENEVV)). Listed
    /// once per backend and cached.
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
        self.hosts[arch.index()].as_ref().map(|h| h.process.path())
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
                "{} not found (run `cargo xtask hosts`, or set {})",
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
                    self.sync_sample_rate();
                    return Ok(());
                }
                Err(e) => errors.push(format!("{}: {e}", path.display())),
            }
        }
        Err(unavailable(errors.join("; ")))
    }

    /// The running hosts' sample rate (the first one's), else the hosts'
    /// fixed rate.
    fn sample_rate(&self) -> u32 {
        self.hosts
            .iter()
            .flatten()
            .map(|h| h.sample_rate)
            .next()
            .unwrap_or(crate::host::SAMPLE_RATE)
    }

    fn sync_sample_rate(&mut self) {
        let rate = self.sample_rate();
        self.playback.set_sample_rate(rate);
    }

    fn send(&mut self, arch: Arch, req: &Request) -> Result<(), SpeechError> {
        let host = self.hosts[arch.index()]
            .as_mut()
            .ok_or_else(|| SpeechError::Engine("the SAPI host is not running".into()))?;
        host.process.send(req).map_err(SpeechError::Engine)
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

    /// Reads everything the hosts have sent so far, and kills a host that
    /// hung.
    fn drain_hosts(&mut self) {
        for arch in Arch::ALL {
            while let Some(msg) = self.hosts[arch.index()]
                .as_mut()
                .and_then(|h| h.process.try_recv())
            {
                self.handle(arch, msg);
            }
            let owes = self.playback.owes(arch.index());
            if self.hosts[arch.index()]
                .as_ref()
                .is_some_and(|h| h.process.stalled(owes, STALL_TIMEOUT))
            {
                self.kill_host(arch, "the voice stopped responding");
            }
        }
    }

    /// Kills a hung host; everything it owed fails.
    fn kill_host(&mut self, arch: Arch, why: &str) {
        if let Some(mut h) = self.hosts[arch.index()].take() {
            h.process.kill();
        }
        self.host_died(arch, why);
    }

    fn handle(&mut self, arch: Arch, msg: HostMsg<Reply>) {
        match msg {
            HostMsg::Closed(why) => self.host_died(arch, &why),
            HostMsg::Reply(r) => self.handle_reply(r),
        }
    }

    fn host_died(&mut self, arch: Arch, why: &str) {
        log::warn!("sapi: {arch} host: {why}");
        self.hosts[arch.index()] = None;
        self.applied[arch.index()] = None;
        self.playback.host_died(
            arch.index(),
            &format!("the voice stopped unexpectedly ({why})"),
        );
        self.sync_sample_rate();
    }

    fn handle_reply(&mut self, r: Reply) {
        match r {
            Reply::Audio { token, samples } => self.playback.on_audio(token, &samples),
            Reply::Word {
                token,
                start,
                len,
                sample,
            } => self
                .playback
                .on_word(token, sample, |w: &mut Words| w.map(start, len)),
            Reply::End { token, status, .. } => self.playback.on_end(token, status),
            Reply::Error { token, message } => {
                if token == 0 {
                    log::warn!("sapi: {message}");
                    // A voice that could not be loaded: send it again next
                    // time rather than assuming it is applied.
                    self.applied = [None, None];
                } else {
                    self.playback.on_error(token, message);
                }
            }
            Reply::Ready { .. } | Reply::Voice(_) => {}
        }
    }

    /// Synthesizes `text` with the current voice and parameters, returning
    /// the audio and word offsets instead of playing it.
    pub fn synthesize(&mut self, text: &str) -> Result<Synthesis, SpeechError> {
        self.capture_text(text, true)
    }

    /// Synthesizes `text` into memory; word offsets are collected only when
    /// `words` is true.
    fn capture_text(&mut self, text: &str, words: bool) -> Result<Synthesis, SpeechError> {
        let arch = self.prepare()?;
        let token = self.playback.next_token();
        self.playback
            .capture(token, arch.index(), Words::new(text, words));
        let pitch = self.pitch();
        if let Err(e) = self.send(
            arch,
            &Request::Speak {
                token,
                text: text.to_owned(),
                pitch,
            },
        ) {
            self.playback.take_capture(token);
            return Err(e);
        }
        if let Some(h) = &mut self.hosts[arch.index()] {
            h.process.touch();
        }
        while self.playback.capture_done(token) == Some(false) {
            let msg = match &mut self.hosts[arch.index()] {
                Some(h) => match h.process.recv_timeout(STALL_TIMEOUT) {
                    Some(m) => m,
                    None => {
                        self.kill_host(arch, "the voice stopped responding");
                        continue;
                    }
                },
                None => HostMsg::Closed("host is not running".into()),
            };
            self.handle(arch, msg);
        }
        let c = self
            .playback
            .take_capture(token)
            .ok_or_else(|| SpeechError::Engine("synthesis lost".into()))?;
        if let Some(e) = c.failed {
            return Err(SpeechError::Engine(e));
        }
        Ok(Synthesis {
            sample_rate: self.sample_rate(),
            samples: c.samples,
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
        // The backend plays the audio itself, so word events fire when the
        // word is heard.
        let mut caps =
            Caps::PAUSE | Caps::PITCH | Caps::VOLUME | Caps::SYNTH_TO_FILE | Caps::PLAYBACK_EVENTS;
        if self.config.output == AudioOutput::Device {
            caps |= Caps::TONES;
        }
        if self.family.has_word_timing() {
            caps |= Caps::WORD_EVENTS | Caps::AUDIO_CLOCK;
        }
        if self.family.normalizes_natively() {
            caps |= Caps::NATIVE_NORMALIZATION;
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
        self.playback.set_gain(params.volume.fraction());
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
        self.playback.emit(sink);
        let arch = self.prepare()?;
        self.playback.ensure_player()?;
        let token = self.playback.next_token();
        let pitch = self.pitch();
        self.send(
            arch,
            &Request::Speak {
                token,
                text: utterance.text.clone(),
                pitch,
            },
        )?;
        if !self.playback.owes(arch.index())
            && let Some(h) = &mut self.hosts[arch.index()]
        {
            // The host starts owing audio now: its stall timer starts here.
            h.process.touch();
        }
        self.playback.enqueue(
            utterance.id,
            token,
            arch.index(),
            Words::new(&utterance.text, self.family.has_word_timing()),
        );
        Ok(())
    }

    fn poll(&mut self, sink: &mut dyn EventSink) {
        self.drain_hosts();
        self.playback.emit(sink);
    }

    fn stop(&mut self) {
        for arch in self
            .playback
            .stop()
            .into_iter()
            .filter_map(Arch::from_index)
        {
            if let Err(e) = self.send(arch, &Request::Stop) {
                log::warn!("sapi: cannot stop the {arch} host: {e}");
            }
        }
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
        crate::wav::write(path, &s.samples, s.sample_rate)
            .map_err(|e| SpeechError::Io(format!("{}: {e}", path.display())))
    }

    /// Writes the utterance as a WAV and reports each word at its
    /// word-boundary event's audio offset in that file (ADR-0011). Voices
    /// without usable word timing (Code Factory's Eloquence through SAPI,
    /// ADR-0007) report none, so export times their cues by sentence.
    fn synthesize_utterance(
        &mut self,
        utterance: &Utterance,
        path: &Path,
    ) -> Result<FileSynthesis, SpeechError> {
        let s = self.capture_text(&utterance.text, self.family.has_word_timing())?;
        crate::wav::write(path, &s.samples, s.sample_rate)
            .map_err(|e| SpeechError::Io(format!("{}: {e}", path.display())))?;
        Ok(FileSynthesis {
            words: textweaver_enginehost::word_timings(&s.words, s.sample_rate),
        })
    }

    fn tone(&mut self, hz: f32, ms: u32) {
        self.playback.tone(hz, ms);
    }
}
