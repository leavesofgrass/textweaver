//! The speech service (ADR-0003): a speech thread driven by a command channel.
//!
//! [`SpeechService`] is the handle the application holds. All work happens
//! in a [`ServiceCore`] on the speech thread, which owns the backend. The
//! core is public so tests can drive it synchronously with a
//! [`FakeClock`](crate::pacing::FakeClock); the thread only feeds it
//! commands and forwards its statuses.
//!
//! What the core does:
//!
//! - **Queue.** `read` replaces the reading with new utterances; the engine
//!   holds the playing utterance plus two chunks of lookahead
//!   ([`ReadingQueue`]).
//! - **Generations.** The generation is bumped before every stop or restart;
//!   an [`EventSink`] drops events of stale generations before the service
//!   logic sees them, so a late `Word` or `Finished` never moves a newer
//!   reading's highlight (fixes Star bugs B1 and B2). Separately, every
//!   `read` gets a [`ReadingGeneration`] that its `Position`, `Paused`,
//!   `Stopped`, and `Finished` statuses carry, so the frontend drops a
//!   status from an older reading by comparing one number.
//! - **Capabilities** are re-read after every parameter change: a voice
//!   can lack word timing or normalize text itself. A change rebuilds the
//!   normalization pipeline and is reported as
//!   [`SpeechStatus::Capabilities`].
//! - **Normalization.** Every utterance goes through the transform
//!   [`Pipeline`] on its own (never across chunks), composing offset maps so
//!   positions still point into the document (ADR-0005). Utterances are
//!   normalized just before they enter the lookahead window, so reading a
//!   whole book from the cursor starts at once.
//! - **Positions.** Word events are mapped through the utterance's offset
//!   map into [`SpeechStatus::Position`]. Audio-clock events are scheduled at
//!   `audio_ms + latency_offset` on the playback clock, never fired on
//!   arrival. Engines without word events are paced by the [`TimerPacer`]
//!   (Star's clamp rules), counting the words actually spoken.
//! - **Pause.** Native when the engine has [`Caps::PAUSE`] (the playback
//!   clock stops too); otherwise emulated by stopping and restarting from the
//!   last confirmed word, which may repeat a word but never skips one.
//!   Edge cases: pause before the first word resumes at the utterance start;
//!   late events after a pause are stale and dropped; [`SpeechService::resume_at`]
//!   resumes from a moved cursor; pausing inside inserted speech ("heading
//!   level 2") resumes at the start of that inserted speech, and reports its
//!   anchor as `resume_at`; queued chunks are cancelled by id.
//! - **Say modes, characters, tones, earcons** as documented on each method.

use std::collections::VecDeque;
use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use textweaver_core::{
    CapsIndication, CharPos, CharRange, OffsetMap, Pitch, PunctuationLevel, Rate, Span, SpanKind,
    SpokenBuilder, Utterance, UtteranceId, UtteranceKind, Volume,
};

use crate::backend::{
    BackendFactory, BackendId, Caps, EventSink, RawEvent, SpeechBackend, SpeechError, VoiceParams,
};
use crate::backends::{NullBackend, resolve_preferred_voice, resolve_voice};
use crate::normalize::{self, NormalizeConfig, Pipeline};
use crate::pacing::{
    Clock, PacingConfig, PlaybackClock, SystemClock, TimerPacer, spoken_words, word_interval,
};
use crate::queue::{DEFAULT_LOOKAHEAD, Generation, ReadingQueue};

/// How `say` interacts with speech in progress.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SayMode {
    /// Stop everything and speak now.
    #[default]
    Interrupt,
    /// Speak after everything already queued.
    Queue,
    /// A status announcement: interrupts other announcements, not reading
    /// position (reading resumes afterwards where the backend allows).
    Announce,
}

/// Short non-speech sounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Earcon {
    /// Navigation wrapped around the document.
    Wrap,
    /// Reached the start or end of the document.
    Boundary,
    /// A command could not be carried out.
    Error,
    /// A mode was entered.
    ModeOn,
    /// A mode was left.
    ModeOff,
}

impl Earcon {
    /// The tones that make up the earcon, as `(hz, ms)` pairs.
    pub fn tones(self) -> &'static [(f32, u32)] {
        match self {
            Earcon::Wrap => &[(660.0, 50), (990.0, 50)],
            Earcon::Boundary => &[(330.0, 90)],
            Earcon::Error => &[(220.0, 60), (180.0, 90)],
            Earcon::ModeOn => &[(700.0, 40), (1050.0, 40)],
            Earcon::ModeOff => &[(1050.0, 40), (700.0, 40)],
        }
    }
}

/// Identifies one [`SpeechService::read`] request: the value `read` returns
/// and every status about that reading carries.
///
/// It starts at 1 and rises with every `read`, so a frontend keeps the value
/// of its latest `read` and drops any status with a different one, with no
/// heuristics. It is not [`UtteranceId::generation`], which the service
/// bumps on every internal restart (pause, resume, skip, an announcement
/// inside a reading) to drop the engine's late events; one reading can span
/// many utterance generations. 0 means "no reading yet".
pub type ReadingGeneration = u64;

/// What the service reports back to the application.
///
/// Statuses about a reading carry its [`ReadingGeneration`], so a status
/// that arrives after the frontend started a newer reading is recognizably
/// stale.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeechStatus {
    /// The highlight should cover `source_range` (`None` inside inserted
    /// speech such as "heading level 2").
    Position {
        /// The reading this position belongs to.
        generation: ReadingGeneration,
        /// The utterance being spoken.
        utterance: UtteranceId,
        /// Document range to highlight.
        source_range: Option<CharRange>,
    },
    /// The reading paused; `resume_at` is where it continues (the last
    /// confirmed word; may repeat, never skips).
    Paused {
        /// The reading that paused.
        generation: ReadingGeneration,
        /// Resume position, if known.
        resume_at: Option<CharPos>,
    },
    /// Speech stopped by request (or a reading was interrupted by speech
    /// that replaces it).
    Stopped {
        /// The latest reading when speech stopped.
        generation: ReadingGeneration,
    },
    /// Everything the reading queued has been spoken.
    Finished {
        /// The reading that finished.
        generation: ReadingGeneration,
    },
    /// The backend's capabilities changed, typically after a voice change
    /// (some SAPI voices give no word timing, some engines normalize text
    /// themselves). Frontends announce what the user loses or gains, for
    /// example "This voice does not report words; the highlight is
    /// estimated."
    Capabilities {
        /// The capabilities now in effect.
        caps: Caps,
    },
    /// The backend failed.
    BackendError(String),
}

impl SpeechStatus {
    /// The reading this status belongs to, for statuses about a reading.
    pub fn generation(&self) -> Option<ReadingGeneration> {
        match self {
            SpeechStatus::Position { generation, .. }
            | SpeechStatus::Paused { generation, .. }
            | SpeechStatus::Stopped { generation }
            | SpeechStatus::Finished { generation } => Some(*generation),
            SpeechStatus::Capabilities { .. } | SpeechStatus::BackendError(_) => None,
        }
    }
}

/// Service configuration.
#[derive(Clone, Debug)]
pub struct ServiceConfig {
    /// Initial voice parameters.
    pub params: VoiceParams,
    /// Pacing parameters.
    pub pacing: PacingConfig,
    /// Initial punctuation level.
    pub punctuation: PunctuationLevel,
    /// Speak capitals within words separately ("split caps").
    pub split_caps: bool,
    /// Normalization toggles, lexicon, and custom abbreviations.
    pub normalize: NormalizeConfig,
    /// How capitals are indicated by [`SpeechService::speak_char`].
    pub caps: CapsIndication,
    /// Rate multiplier for single characters (ADR-0004: characters use a
    /// scaled rate).
    pub char_rate_scale: f32,
    /// Semitones added for capital letters when `caps` is `Pitch`.
    pub caps_pitch_semitones: i8,
    /// When `params.voice` is `None`, pick the first voice whose name or id
    /// contains this (Star's `tts_prefer_voice`, default "eloquence"),
    /// preferring a US English variant.
    pub prefer_voice: Option<String>,
    /// Chunks handed to the engine ahead of the playing one.
    pub lookahead: usize,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        ServiceConfig {
            params: VoiceParams::default(),
            pacing: PacingConfig::default(),
            punctuation: PunctuationLevel::default(),
            split_caps: false,
            normalize: NormalizeConfig::default(),
            caps: CapsIndication::default(),
            char_rate_scale: 1.2,
            caps_pitch_semitones: 4,
            prefer_voice: None,
            lookahead: DEFAULT_LOOKAHEAD,
        }
    }
}

/// How often the core polls the backend while speech is active.
pub const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// After this many `speak` failures in a row the reading stops, instead of
/// failing through every remaining sentence of the document in one go.
pub const MAX_CONSECUTIVE_FAILURES: u32 = 3;

/// A backend error identical to one reported this recently is not reported
/// again. A frontend that announces errors through speech (which fails the
/// same way) would otherwise loop forever, and a host crash that fails
/// several queued utterances would be reported once per utterance.
pub const ERROR_REPEAT_WINDOW: Duration = Duration::from_secs(3);

/// Commands sent to the speech thread.
#[derive(Debug)]
enum Command {
    Say(String, SayMode),
    Read(Vec<Utterance>, ReadingGeneration),
    Stop,
    Pause,
    Resume,
    ResumeAt(CharPos),
    Skip,
    SetRate(Rate),
    SetPitch(Pitch),
    SetVolume(Volume),
    SetVoice(Option<String>),
    SetPunctuation(PunctuationLevel),
    SetSplitCaps(bool),
    SetNormalization(Box<NormalizeConfig>),
    SetPacing(PacingConfig),
    SpeakChar(char, Option<CharPos>),
    Tone(f32, u32),
    Earcon(Earcon),
    Shutdown,
}

/// Handle to the speech thread. Cheap calls; all work happens on the thread.
pub struct SpeechService {
    tx: Sender<Command>,
    status_rx: Receiver<SpeechStatus>,
    thread: Option<JoinHandle<()>>,
    backend_id: BackendId,
    /// The backend's current capabilities, updated by the speech thread
    /// when they change.
    caps: Arc<AtomicU32>,
    /// The last reading generation handed out by `read`.
    last_reading: AtomicU64,
}

impl std::fmt::Debug for SpeechService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpeechService")
            .field("backend", &self.backend_id)
            .field("caps", &self.capabilities())
            .finish_non_exhaustive()
    }
}

impl SpeechService {
    /// Starts the speech thread and creates the backend on it.
    pub fn spawn(factory: BackendFactory, config: ServiceConfig) -> Result<Self, SpeechError> {
        Self::spawn_with_clock(factory, config, Box::new(SystemClock::default()))
    }

    /// Like [`spawn`](Self::spawn) with an explicit clock.
    pub fn spawn_with_clock(
        factory: BackendFactory,
        config: ServiceConfig,
        clock: Box<dyn Clock>,
    ) -> Result<Self, SpeechError> {
        let (tx, rx) = mpsc::channel();
        let (status_tx, status_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let caps = Arc::new(AtomicU32::new(0));
        let shared_caps = Arc::clone(&caps);
        let thread = thread::Builder::new()
            .name("textweaver-speech".into())
            .spawn(move || match factory() {
                Ok(backend) => {
                    let mut core = ServiceCore::new(backend, config, clock);
                    shared_caps.store(core.capabilities().bits(), Ordering::SeqCst);
                    let _ = ready_tx.send(Ok(core.backend_id()));
                    let link = StatusLink {
                        tx: status_tx,
                        caps: shared_caps,
                    };
                    if link.forward(core.take_statuses()) {
                        run(&mut core, &rx, &link);
                    }
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            })
            .map_err(|e| SpeechError::Io(e.to_string()))?;
        let backend_id = ready_rx.recv().map_err(|_| SpeechError::ServiceStopped)??;
        Ok(SpeechService {
            tx,
            status_rx,
            thread: Some(thread),
            backend_id,
            caps,
            last_reading: AtomicU64::new(0),
        })
    }

    /// A service with the silent backend.
    pub fn null() -> Self {
        Self::spawn(
            Box::new(|| Ok(Box::new(NullBackend::default()) as _)),
            ServiceConfig::default(),
        )
        .expect("the null backend cannot fail")
    }

    /// The backend's id.
    pub fn backend_id(&self) -> BackendId {
        self.backend_id
    }

    /// The backend's current capabilities (for example, to announce "pitch
    /// not supported by this voice"). The speech thread re-reads them after
    /// every parameter change, so they follow the selected voice; a change
    /// is also reported as [`SpeechStatus::Capabilities`].
    pub fn capabilities(&self) -> Caps {
        Caps::from_bits_retain(self.caps.load(Ordering::SeqCst))
    }

    fn send(&self, cmd: Command) {
        // A closed channel means the thread is gone; there is nobody to tell.
        let _ = self.tx.send(cmd);
    }

    /// Speaks `text` (not mapped to the document). See [`SayMode`].
    pub fn say(&self, text: impl Into<String>, mode: SayMode) {
        self.send(Command::Say(text.into(), mode));
    }
    /// Reads utterances in order, replacing any reading in progress.
    /// Returns the reading's generation, which every status about this
    /// reading carries (see [`ReadingGeneration`]).
    pub fn read(&self, utterances: Vec<Utterance>) -> ReadingGeneration {
        let generation = self.last_reading.fetch_add(1, Ordering::SeqCst) + 1;
        self.send(Command::Read(utterances, generation));
        generation
    }
    /// Stops all speech.
    pub fn stop(&self) {
        self.send(Command::Stop);
    }
    /// Pauses reading.
    pub fn pause(&self) {
        self.send(Command::Pause);
    }
    /// Resumes reading.
    pub fn resume(&self) {
        self.send(Command::Resume);
    }
    /// Resumes a paused reading from document position `pos` instead of the
    /// pause point (the cursor moved while paused). If `pos` is outside the
    /// paused reading, resumes at the pause point.
    pub fn resume_at(&self, pos: CharPos) {
        self.send(Command::ResumeAt(pos));
    }
    /// Skips to the next queued utterance.
    pub fn skip(&self) {
        self.send(Command::Skip);
    }
    /// Sets the rate.
    pub fn set_rate(&self, rate: Rate) {
        self.send(Command::SetRate(rate));
    }
    /// Sets the pitch.
    pub fn set_pitch(&self, pitch: Pitch) {
        self.send(Command::SetPitch(pitch));
    }
    /// Sets the volume.
    pub fn set_volume(&self, volume: Volume) {
        self.send(Command::SetVolume(volume));
    }
    /// Sets the voice (`None` for the engine default).
    pub fn set_voice(&self, voice: Option<String>) {
        self.send(Command::SetVoice(voice));
    }
    /// Sets punctuation verbosity.
    pub fn set_punctuation(&self, level: PunctuationLevel) {
        self.send(Command::SetPunctuation(level));
    }
    /// Enables or disables split caps.
    pub fn set_split_caps(&self, on: bool) {
        self.send(Command::SetSplitCaps(on));
    }
    /// Replaces the normalization settings (applies to utterances read
    /// after this call).
    pub fn set_normalization(&self, config: NormalizeConfig) {
        self.send(Command::SetNormalization(Box::new(config)));
    }
    /// Replaces the pacing settings (highlight speed, latency offset).
    pub fn set_pacing(&self, pacing: PacingConfig) {
        self.send(Command::SetPacing(pacing));
    }
    /// Speaks one character (scaled rate, caps indication), optionally
    /// mapped to a document position.
    pub fn speak_char(&self, c: char, at: Option<CharPos>) {
        self.send(Command::SpeakChar(c, at));
    }
    /// Plays a tone (ignored when the backend has no [`Caps::TONES`]).
    pub fn tone(&self, hz: f32, ms: u32) {
        self.send(Command::Tone(hz, ms));
    }
    /// Plays an earcon (ignored when the backend has no [`Caps::TONES`]).
    pub fn earcon(&self, earcon: Earcon) {
        self.send(Command::Earcon(earcon));
    }

    /// The next status update, if one is waiting.
    pub fn try_status(&self) -> Option<SpeechStatus> {
        self.status_rx.try_recv().ok()
    }

    /// The status channel, for frontends that block or select on it.
    pub fn statuses(&self) -> &Receiver<SpeechStatus> {
        &self.status_rx
    }

    /// Stops the thread and waits for it.
    pub fn shutdown(mut self) {
        self.shutdown_inner();
    }

    fn shutdown_inner(&mut self) {
        self.send(Command::Shutdown);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for SpeechService {
    fn drop(&mut self) {
        self.shutdown_inner();
    }
}

/// The speech thread's side of the status channel; it also keeps the
/// handle's copy of the capabilities current.
struct StatusLink {
    tx: Sender<SpeechStatus>,
    caps: Arc<AtomicU32>,
}

impl StatusLink {
    /// Sends `statuses`; false when the handle is gone.
    fn forward(&self, statuses: Vec<SpeechStatus>) -> bool {
        for s in statuses {
            if let SpeechStatus::Capabilities { caps } = &s {
                self.caps.store(caps.bits(), Ordering::SeqCst);
            }
            if self.tx.send(s).is_err() {
                return false;
            }
        }
        true
    }
}

fn run(core: &mut ServiceCore, rx: &Receiver<Command>, status: &StatusLink) {
    loop {
        let cmd = match core.next_wakeup() {
            None => match rx.recv() {
                Ok(c) => Some(c),
                Err(_) => break,
            },
            Some(wait) => match rx.recv_timeout(wait) {
                Ok(c) => Some(c),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            },
        };
        if let Some(cmd) = cmd {
            if matches!(cmd, Command::Shutdown) {
                core.stop_silently();
                break;
            }
            core.apply(cmd);
        }
        core.step();
        if !status.forward(core.take_statuses()) {
            return;
        }
    }
}

/// Collects engine events, dropping stale generations.
struct CollectSink {
    generation: Generation,
    events: Vec<(UtteranceId, RawEvent)>,
}

impl EventSink for CollectSink {
    fn emit(&mut self, id: UtteranceId, event: RawEvent) {
        if self.generation.is_current(id) {
            self.events.push((id, event));
        } else {
            log::trace!("dropping stale event {event:?} for {id:?}");
        }
    }

    fn is_current(&self, id: UtteranceId) -> bool {
        self.generation.is_current(id)
    }
}

/// State of the utterance whose audio is playing.
#[derive(Debug)]
struct Playing {
    id: UtteranceId,
    kind: UtteranceKind,
    words: Vec<Range<u32>>,
    started: Duration,
    pacer: TimerPacer,
    /// Audio-clock confirmations waiting for their time: `(due, word)`.
    scheduled: Vec<(Duration, usize)>,
    painted: Option<usize>,
}

#[derive(Debug)]
enum PauseState {
    /// The engine paused itself; the queue is intact.
    Native,
    /// The engine was stopped; these utterances (normalized) and then the
    /// backlog (not yet normalized) continue the reading.
    Emulated {
        utterances: Vec<Utterance>,
        backlog: VecDeque<Utterance>,
        reading: bool,
    },
}

/// The speech service logic without the thread: owns the backend, the
/// queue, the pacer, and the playback clock, and produces statuses.
///
/// Drive it with the command methods, then [`step`](Self::step) (poll the
/// backend, run timers), then [`take_statuses`](Self::take_statuses).
/// [`next_wakeup`](Self::next_wakeup) says when to step again.
pub struct ServiceCore {
    backend: Box<dyn SpeechBackend>,
    caps: Caps,
    clock: PlaybackClock,
    config: ServiceConfig,
    pipeline: Pipeline,
    params: VoiceParams,
    queue: ReadingQueue,
    /// The rest of the reading, not yet normalized: utterances are
    /// normalized just before they join the queue, so a long reading starts
    /// at once.
    backlog: VecDeque<Utterance>,
    playing: Option<Playing>,
    paused: Option<PauseState>,
    /// A `read` is in progress; `Finished` is reported when it drains.
    reading: bool,
    /// The generation of the latest `read` (0 before the first).
    reading_generation: ReadingGeneration,
    /// Parameters were changed for this character utterance.
    char_params: Option<UtteranceId>,
    last_position: Option<(UtteranceId, Option<CharRange>)>,
    events: Vec<(UtteranceId, RawEvent)>,
    out: Vec<SpeechStatus>,
    /// The engine was given something since it was last stopped.
    engine_busy: bool,
    /// `speak` failures in a row (reset by a successful `speak`).
    failures: u32,
    /// Errors reported recently, with when (wall clock), for
    /// [`ERROR_REPEAT_WINDOW`].
    recent_errors: Vec<(String, Duration)>,
}

impl std::fmt::Debug for ServiceCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceCore")
            .field("backend", &self.backend.id())
            .field("queue", &self.queue.len())
            .field("playing", &self.playing)
            .field("paused", &self.paused)
            .finish_non_exhaustive()
    }
}

impl ServiceCore {
    /// A core around `backend`. Applies the configured voice parameters,
    /// resolving `prefer_voice` when no voice is set.
    pub fn new(
        backend: Box<dyn SpeechBackend>,
        config: ServiceConfig,
        clock: Box<dyn Clock>,
    ) -> Self {
        let caps = backend.capabilities();
        let pipeline = Pipeline::for_settings(
            &config.normalize,
            config.punctuation,
            config.split_caps,
            caps.contains(Caps::NATIVE_NORMALIZATION),
        );
        let mut core = ServiceCore {
            backend,
            caps,
            clock: PlaybackClock::new(clock),
            params: config.params.clone(),
            queue: ReadingQueue::new(config.lookahead),
            backlog: VecDeque::new(),
            config,
            pipeline,
            playing: None,
            paused: None,
            reading: false,
            reading_generation: 0,
            char_params: None,
            last_position: None,
            events: Vec::new(),
            out: Vec::new(),
            engine_busy: false,
            failures: 0,
            recent_errors: Vec::new(),
        };
        if let Some(asked) = core.params.voice.clone() {
            core.params.voice = Some(core.resolve_voice_name(&asked));
        } else if let Some(prefer) = core.config.prefer_voice.clone()
            && let Ok(voices) = core.backend.voices()
        {
            core.params.voice = resolve_preferred_voice(&voices, &prefer);
        }
        core.apply_params();
        // The starting capabilities are what the handle reports at spawn;
        // only later changes are statuses.
        core.out
            .retain(|s| !matches!(s, SpeechStatus::Capabilities { .. }));
        core
    }

    /// The backend's id.
    pub fn backend_id(&self) -> BackendId {
        self.backend.id()
    }

    /// The backend's capabilities, as last read (after every parameter
    /// change).
    pub fn capabilities(&self) -> Caps {
        self.caps
    }

    /// The generation of the latest `read` (0 before the first).
    pub fn reading_generation(&self) -> ReadingGeneration {
        self.reading_generation
    }

    /// The voice parameters currently requested.
    pub fn params(&self) -> &VoiceParams {
        &self.params
    }

    /// The normalization pipeline currently applied to utterances.
    pub fn pipeline(&self) -> &Pipeline {
        &self.pipeline
    }

    /// True while paused.
    pub fn is_paused(&self) -> bool {
        self.paused.is_some()
    }

    /// True while anything is queued or playing.
    pub fn is_active(&self) -> bool {
        !self.queue.is_empty() || !self.backlog.is_empty()
    }

    /// Statuses produced since the last call.
    pub fn take_statuses(&mut self) -> Vec<SpeechStatus> {
        std::mem::take(&mut self.out)
    }

    /// How long until [`step`](Self::step) has work: `None` when idle.
    pub fn next_wakeup(&self) -> Option<Duration> {
        if self.queue.is_empty() && self.playing.is_none() {
            return None;
        }
        let mut wait = POLL_INTERVAL;
        if let Some(p) = &self.playing {
            let deadlines = p
                .scheduled
                .first()
                .map(|(due, _)| *due)
                .into_iter()
                .chain(p.pacer.next_deadline());
            for d in deadlines {
                if let Some(w) = self.clock.wait_until(d) {
                    wait = wait.min(w);
                }
            }
        }
        Some(wait)
    }

    fn apply(&mut self, cmd: Command) {
        match cmd {
            Command::Say(text, mode) => self.say(&text, mode),
            Command::Read(u, generation) => self.read_as(u, generation),
            Command::Stop => self.stop(),
            Command::Pause => self.pause(),
            Command::Resume => self.resume(),
            Command::ResumeAt(pos) => self.resume_at(pos),
            Command::Skip => self.skip(),
            Command::SetRate(r) => self.set_rate(r),
            Command::SetPitch(p) => self.set_pitch(p),
            Command::SetVolume(v) => self.set_volume(v),
            Command::SetVoice(v) => self.set_voice(v),
            Command::SetPunctuation(l) => self.set_punctuation(l),
            Command::SetSplitCaps(on) => self.set_split_caps(on),
            Command::SetNormalization(c) => self.set_normalization(*c),
            Command::SetPacing(p) => self.set_pacing(p),
            Command::SpeakChar(c, at) => self.speak_char(c, at),
            Command::Tone(hz, ms) => self.tone(hz, ms),
            Command::Earcon(e) => self.earcon(e),
            Command::Shutdown => self.stop_silently(),
        }
    }

    // ---- commands -------------------------------------------------------

    /// Reads `utterances` in order, replacing any reading in progress (and
    /// any pause). Returns the new reading's generation.
    pub fn read(&mut self, utterances: Vec<Utterance>) -> ReadingGeneration {
        let generation = self.reading_generation + 1;
        self.read_as(utterances, generation);
        generation
    }

    /// [`read`](Self::read) with a generation chosen by the caller (the
    /// [`SpeechService`] handle numbers readings itself so `read` can return
    /// at once). Generations should rise; a lower one is still used as
    /// given.
    pub fn read_as(&mut self, utterances: Vec<Utterance>, generation: ReadingGeneration) {
        self.clear_engine();
        self.paused = None;
        self.queue.start(Vec::new());
        self.backlog = utterances.into();
        self.last_position = None;
        self.reading_generation = generation;
        if self.backlog.is_empty() {
            self.reading = false;
            self.out.push(SpeechStatus::Finished { generation });
            return;
        }
        self.reading = true;
        self.pump();
    }

    /// Stops everything, forgets any pause, and reports `Stopped`.
    pub fn stop(&mut self) {
        self.stop_silently();
        self.out.push(SpeechStatus::Stopped {
            generation: self.reading_generation,
        });
    }

    fn stop_silently(&mut self) {
        self.clear_engine();
        self.backlog.clear();
        self.paused = None;
        self.reading = false;
    }

    /// Pauses the reading (no effect when idle or already paused).
    pub fn pause(&mut self) {
        if self.paused.is_some() || self.queue.is_empty() {
            return;
        }
        let (byte, resume_at) = self.resume_point();
        if self.caps.contains(Caps::PAUSE) && self.backend.pause().is_ok() {
            self.clock.pause();
            self.paused = Some(PauseState::Native);
        } else {
            let utterances = self.remainder(byte);
            let backlog = std::mem::take(&mut self.backlog);
            let reading = self.reading;
            self.clear_engine();
            self.paused = Some(PauseState::Emulated {
                utterances,
                backlog,
                reading,
            });
        }
        self.out.push(SpeechStatus::Paused {
            generation: self.reading_generation,
            resume_at,
        });
    }

    /// Resumes after [`pause`](Self::pause).
    pub fn resume(&mut self) {
        match self.paused.take() {
            None => {}
            Some(PauseState::Native) => {
                self.clock.resume();
                if let Err(e) = self.backend.resume() {
                    self.backend_error(e.to_string());
                }
            }
            Some(PauseState::Emulated {
                utterances,
                backlog,
                reading,
            }) => self.restart(utterances, backlog, reading),
        }
    }

    /// Resumes a paused reading from `pos` (the cursor moved while paused).
    pub fn resume_at(&mut self, pos: CharPos) {
        self.demote_native_pause();
        let Some(PauseState::Emulated {
            mut utterances,
            mut backlog,
            reading,
        }) = self.paused.take()
        else {
            return;
        };
        let contains = |u: &Utterance| {
            u.kind == UtteranceKind::Text
                && u.source_range()
                    .is_some_and(|r| r.start <= pos && pos < r.end)
        };
        if let Some(i) = utterances.iter().position(contains) {
            utterances.drain(..i);
        } else if let Some(j) = backlog.iter().position(contains) {
            backlog.drain(..j);
            utterances = backlog
                .pop_front()
                .map(|u| self.normalize(u))
                .into_iter()
                .collect();
        } else {
            // Not in the paused reading: resume at the pause point.
            return self.restart(utterances, backlog, reading);
        }
        let first = &utterances[0];
        let byte = first.offset_map.to_spoken(&first.text, pos).unwrap_or(0);
        let byte = snap_to_span(first, byte);
        match trim_utterance(first, byte) {
            Some(t) => utterances[0] = t,
            None => {
                utterances.remove(0);
            }
        }
        self.restart(utterances, backlog, reading);
    }

    /// Skips the playing utterance (or, while paused, the next one).
    pub fn skip(&mut self) {
        if let Some(PauseState::Emulated { utterances, .. }) = &mut self.paused {
            if !utterances.is_empty() {
                utterances.remove(0);
            }
            return;
        }
        self.demote_native_pause();
        if matches!(self.paused, Some(PauseState::Emulated { .. })) {
            return self.skip();
        }
        let Some(front) = self.queue.front().map(|u| u.id) else {
            return;
        };
        let mut rest = self.queue.take_all();
        rest.retain(|u| u.id != front);
        let backlog = std::mem::take(&mut self.backlog);
        let reading = self.reading;
        self.clear_engine();
        self.restart(rest, backlog, reading);
    }

    /// Speaks `text` according to `mode`.
    pub fn say(&mut self, text: &str, mode: SayMode) {
        let u = self.normalize(Utterance::announcement(text));
        match mode {
            SayMode::Interrupt => {
                self.demote_native_pause();
                self.interrupt();
                self.queue.push_back(u);
            }
            SayMode::Queue => {
                self.demote_native_pause();
                self.queue.push_back(u);
            }
            SayMode::Announce => {
                self.demote_native_pause();
                let reading_playing = self.paused.is_none()
                    && self.reading
                    && self.queue.iter().any(|u| u.kind == UtteranceKind::Text);
                let backlog = std::mem::take(&mut self.backlog);
                if reading_playing {
                    let (byte, _) = self.resume_point();
                    let mut rest = self.remainder(byte);
                    rest.insert(0, u);
                    self.clear_engine();
                    self.restart(rest, backlog, true);
                    return;
                }
                // Interrupt other announcements only.
                let rest: Vec<Utterance> = self
                    .queue
                    .take_all()
                    .into_iter()
                    .filter(|x| x.kind != UtteranceKind::Announcement)
                    .collect();
                let reading = self.reading;
                self.clear_engine();
                let mut all = vec![u];
                all.extend(rest);
                self.restart(all, backlog, reading);
                return;
            }
        }
        self.pump();
    }

    /// Speaks one character: its name for punctuation and whitespace, at the
    /// scaled character rate, with capitals indicated per `config.caps`
    /// (pitch when the engine has [`Caps::PITCH`], else a tone when it has
    /// [`Caps::TONES`], else the word "cap").
    pub fn speak_char(&mut self, c: char, at: Option<CharPos>) {
        self.demote_native_pause();
        self.interrupt();
        let upper = c.is_uppercase();
        let mut indication = if upper {
            self.config.caps
        } else {
            CapsIndication::None
        };
        if indication == CapsIndication::Pitch && !self.caps.contains(Caps::PITCH) {
            indication = CapsIndication::Tone;
        }
        if indication == CapsIndication::Tone && !self.caps.contains(Caps::TONES) {
            indication = CapsIndication::SayCap;
        }
        let name = normalize::char_name(c);
        let mut b = SpokenBuilder::new();
        if indication == CapsIndication::SayCap {
            b.push_inserted("cap ", at.unwrap_or_default());
        }
        let spoken = name.map_or_else(|| c.to_string(), str::to_owned);
        match at {
            Some(pos) if name.is_none() => b.push_literal(&spoken, pos),
            Some(pos) => b.push_expanded(&spoken, CharRange::new(pos, pos.saturating_add(1))),
            None => b.push_inserted(&spoken, CharPos::ZERO),
        }
        let (text, map) = b.finish();
        let map = if at.is_some() {
            map
        } else {
            OffsetMap::default()
        };
        let u = Utterance {
            id: UtteranceId::default(),
            text,
            kind: UtteranceKind::Character,
            offset_map: map,
        };
        if indication == CapsIndication::Tone {
            self.backend.tone(1200.0, 30);
        }
        let mut p = self.params.clone();
        let scaled = f32::from(p.rate.wpm()) * self.config.char_rate_scale.clamp(0.25, 4.0);
        p.rate = Rate::Wpm(scaled.round().clamp(1.0, f32::from(u16::MAX)) as u16).clamped();
        if indication == CapsIndication::Pitch {
            p.pitch = p.pitch.step(self.config.caps_pitch_semitones);
        }
        let id = self.queue.push_back(u);
        if p != self.params {
            if let Err(e) = self.backend.set_params(&p) {
                self.backend_error(e.to_string());
            }
            self.char_params = Some(id);
        }
        self.pump();
    }

    /// Plays a tone when the engine can.
    pub fn tone(&mut self, hz: f32, ms: u32) {
        if self.caps.contains(Caps::TONES) {
            self.backend.tone(hz, ms);
        }
    }

    /// Plays an earcon when the engine can play tones.
    pub fn earcon(&mut self, earcon: Earcon) {
        for &(hz, ms) in earcon.tones() {
            self.tone(hz, ms);
        }
    }

    /// Sets the rate; the timer interval follows at once when the engine
    /// changes rate live (Star bug B9), otherwise from the next utterance.
    pub fn set_rate(&mut self, rate: Rate) {
        self.params.rate = rate.clamped();
        self.apply_params();
        if self.caps.contains(Caps::LIVE_RATE) {
            let interval = self.interval();
            if let Some(p) = &mut self.playing {
                p.pacer.set_interval(interval);
            }
        }
    }

    /// Sets the pitch.
    pub fn set_pitch(&mut self, pitch: Pitch) {
        self.params.pitch = pitch.clamped();
        self.apply_params();
    }

    /// Sets the volume.
    pub fn set_volume(&mut self, volume: Volume) {
        self.params.volume = volume;
        self.apply_params();
    }

    /// Sets the voice.
    pub fn set_voice(&mut self, voice: Option<String>) {
        self.params.voice = voice.map(|v| self.resolve_voice_name(&v));
        self.apply_params();
    }

    /// A voice id for what the user typed (an id, a name such as "Zira" or
    /// "Reed", or part of one; see [`resolve_voice`]). Unknown text is
    /// passed through unchanged so the backend reports it.
    fn resolve_voice_name(&self, asked: &str) -> String {
        self.backend
            .voices()
            .ok()
            .and_then(|voices| resolve_voice(&voices, asked))
            .unwrap_or_else(|| asked.to_owned())
    }

    /// Sets punctuation verbosity for utterances read from now on.
    pub fn set_punctuation(&mut self, level: PunctuationLevel) {
        self.config.punctuation = level;
        self.rebuild_pipeline();
    }

    /// Enables or disables split caps for utterances read from now on.
    pub fn set_split_caps(&mut self, on: bool) {
        self.config.split_caps = on;
        self.rebuild_pipeline();
    }

    /// Replaces the normalization settings.
    pub fn set_normalization(&mut self, config: NormalizeConfig) {
        self.config.normalize = config;
        self.rebuild_pipeline();
    }

    /// Replaces the pacing settings.
    pub fn set_pacing(&mut self, pacing: PacingConfig) {
        self.config.pacing = pacing;
    }

    // ---- the engine loop --------------------------------------------------

    /// Polls the backend, processes its events, submits lookahead, and runs
    /// the highlight timers.
    pub fn step(&mut self) {
        if !self.queue.is_empty() {
            let mut sink = self.sink();
            self.backend.poll(&mut sink);
            self.events.extend(sink.events);
        }
        self.drain_events();
        self.pump();
        self.run_timers();
    }

    fn sink(&self) -> CollectSink {
        CollectSink {
            generation: self.queue.generation(),
            events: Vec::new(),
        }
    }

    fn normalize(&self, u: Utterance) -> Utterance {
        let u = self.pipeline.apply(u);
        if cfg!(debug_assertions)
            && !u.offset_map.is_empty()
            && let Err(e) = u.offset_map.check_invariants(&u.text)
        {
            log::warn!("normalized utterance has an invalid map: {e}");
        }
        u
    }

    fn rebuild_pipeline(&mut self) {
        self.pipeline = Pipeline::for_settings(
            &self.config.normalize,
            self.config.punctuation,
            self.config.split_caps,
            self.caps.contains(Caps::NATIVE_NORMALIZATION),
        );
    }

    fn apply_params(&mut self) {
        if self.char_params.is_some() {
            // Restored (with the new values) when the character finishes.
            return;
        }
        if let Err(e) = self.backend.set_params(&self.params) {
            self.backend_error(e.to_string());
        }
        self.refresh_caps();
    }

    /// Re-reads the backend's capabilities, which can follow the voice (a
    /// SAPI voice without word timing, an Eloquence voice that normalizes
    /// text itself). On a change: the pipeline is rebuilt for utterances
    /// normalized from now on, and `Capabilities` is reported.
    fn refresh_caps(&mut self) {
        let caps = self.backend.capabilities();
        if caps == self.caps {
            return;
        }
        let native_changed = (caps ^ self.caps).contains(Caps::NATIVE_NORMALIZATION);
        self.caps = caps;
        if native_changed {
            self.rebuild_pipeline();
        }
        self.out.push(SpeechStatus::Capabilities { caps });
    }

    fn restore_char_params(&mut self) {
        if self.char_params.take().is_some() {
            self.apply_params();
        }
    }

    fn interval(&self) -> Duration {
        word_interval(
            self.backend.effective_wpm(),
            self.config.pacing.highlight_speed,
        )
    }

    /// Stops the engine and drops the queue (bumping the generation).
    fn clear_engine(&mut self) {
        let had = self.engine_busy || !self.queue.is_empty() || self.clock.is_paused();
        self.queue.clear();
        if had {
            self.backend.stop();
        }
        self.engine_busy = false;
        self.clock.resume();
        self.playing = None;
        self.events.clear();
        self.restore_char_params();
    }

    /// Stops whatever plays for an interrupting utterance, reporting
    /// `Stopped` if a reading was playing. A pause is kept.
    fn interrupt(&mut self) {
        if self.queue.is_empty() && self.backlog.is_empty() {
            return;
        }
        let was_reading = self.reading;
        self.clear_engine();
        self.backlog.clear();
        if was_reading && self.paused.is_none() {
            self.reading = false;
            self.out.push(SpeechStatus::Stopped {
                generation: self.reading_generation,
            });
        }
    }

    /// Turns a native pause into an emulated one, so something else can be
    /// spoken while the reading stays paused.
    fn demote_native_pause(&mut self) {
        if !matches!(self.paused, Some(PauseState::Native)) {
            return;
        }
        let (byte, _) = self.resume_point();
        let utterances = self.remainder(byte);
        let backlog = std::mem::take(&mut self.backlog);
        let reading = self.reading;
        self.clear_engine();
        self.paused = Some(PauseState::Emulated {
            utterances,
            backlog,
            reading,
        });
    }

    /// Restarts the engine (new generation) with normalized `utterances`
    /// followed by the raw `backlog`.
    fn restart(&mut self, utterances: Vec<Utterance>, backlog: VecDeque<Utterance>, reading: bool) {
        self.queue.restart(utterances);
        self.backlog = backlog;
        self.reading = reading;
        self.last_position = None;
        self.pump();
    }

    /// Normalizes backlog utterances into the queue until it holds the
    /// playing utterance, the lookahead, and one more.
    fn refill(&mut self) {
        while self.queue.len() < self.config.lookahead + 2 {
            let Some(u) = self.backlog.pop_front() else {
                break;
            };
            let u = self.normalize(u);
            self.queue.push_back(u);
        }
    }

    /// Where a pause would resume: the spoken byte in the front utterance
    /// and its source position.
    fn resume_point(&self) -> (u32, Option<CharPos>) {
        let Some(front) = self.queue.iter().find(|u| u.kind == UtteranceKind::Text) else {
            return (0, None);
        };
        let mut byte = 0;
        if let Some(p) = self.playing.as_ref().filter(|p| p.id == front.id) {
            let expect = self.caps.contains(Caps::WORD_EVENTS);
            let word = match p.pacer.last_confirmed() {
                Some(w) => Some(w),
                // A free-running timer may be ahead of the audio: step back
                // one word so resume repeats rather than skips.
                None if !expect => p.painted.map(|w| w.saturating_sub(1)),
                None => p.painted,
            };
            if let Some(r) = word.and_then(|w| p.words.get(w)) {
                byte = snap_to_span(front, r.start);
            }
        }
        let resume_at = front.offset_map.resume_source(&front.text, byte);
        (byte, resume_at)
    }

    /// The reading from `byte` of the first text utterance on: that
    /// utterance trimmed, then everything after it. Announcements and
    /// characters in front of it are dropped.
    fn remainder(&self, byte: u32) -> Vec<Utterance> {
        let mut out = Vec::new();
        let mut found = false;
        for u in self.queue.iter() {
            if !found {
                if u.kind != UtteranceKind::Text {
                    continue;
                }
                found = true;
                if let Some(t) = trim_utterance(u, byte) {
                    out.push(t);
                }
                continue;
            }
            out.push(u.clone());
        }
        out
    }

    /// Hands queued utterances to the engine and processes the events that
    /// result, until nothing more can be submitted.
    fn pump(&mut self) {
        loop {
            self.refill();
            let batch = self.queue.to_submit();
            if batch.is_empty() {
                break;
            }
            for u in batch {
                let mut sink = self.sink();
                self.engine_busy = true;
                let result = self.backend.speak(&u, &mut sink);
                self.events.extend(sink.events);
                match result {
                    Ok(()) => self.failures = 0,
                    Err(e) => {
                        self.failures = self.failures.saturating_add(1);
                        self.backend_error(e.to_string());
                        self.events.push((u.id, RawEvent::Cancelled));
                        if self.failures >= MAX_CONSECUTIVE_FAILURES {
                            self.give_up();
                            return;
                        }
                    }
                }
            }
            self.drain_events();
        }
        self.finish_if_done();
    }

    /// The engine failed [`MAX_CONSECUTIVE_FAILURES`] times in a row: stop
    /// everything (the reading reports `Stopped`) and say why once.
    fn give_up(&mut self) {
        let was_reading = self.reading;
        self.stop_silently();
        // The same words every time, so a repeat within the window is quiet.
        self.backend_error(format!(
            "the speech engine failed {MAX_CONSECUTIVE_FAILURES} times in a row, so speech stopped"
        ));
        if was_reading {
            self.out.push(SpeechStatus::Stopped {
                generation: self.reading_generation,
            });
        }
    }

    /// Reports a backend error, unless the same error was reported within
    /// [`ERROR_REPEAT_WINDOW`].
    fn backend_error(&mut self, message: String) {
        let now = self.clock.wall();
        self.recent_errors
            .retain(|(_, at)| now.saturating_sub(*at) < ERROR_REPEAT_WINDOW);
        if self.recent_errors.iter().any(|(m, _)| *m == message) {
            log::debug!("not repeating speech error: {message}");
            return;
        }
        self.recent_errors.push((message.clone(), now));
        self.out.push(SpeechStatus::BackendError(message));
    }

    fn drain_events(&mut self) {
        let events = std::mem::take(&mut self.events);
        for (id, e) in events {
            self.handle_event(id, e);
        }
    }

    fn handle_event(&mut self, id: UtteranceId, event: RawEvent) {
        if !self.queue.is_current(id) || self.queue.submitted_index(id).is_none() {
            return;
        }
        match event {
            RawEvent::Started => self.begin(id),
            RawEvent::Word {
                byte_range,
                audio_ms,
            } => {
                if self.playing.as_ref().is_none_or(|p| p.id != id) {
                    self.begin(id);
                }
                let now = self.clock.now();
                let latency = self.config.pacing.latency_offset;
                let Some(p) = &mut self.playing else {
                    return;
                };
                if p.words.is_empty() {
                    return;
                }
                let w = p
                    .words
                    .partition_point(|r| r.end <= byte_range.start)
                    .min(p.words.len() - 1);
                let audio_ms = if self.caps.contains(Caps::PLAYBACK_EVENTS) {
                    None
                } else {
                    audio_ms
                };
                match audio_ms {
                    Some(ms) => {
                        let due = p.started + Duration::from_millis(u64::from(ms)) + latency;
                        if due <= now {
                            self.confirm(w);
                        } else {
                            let i = p.scheduled.partition_point(|(d, _)| *d <= due);
                            p.scheduled.insert(i, (due, w));
                        }
                    }
                    None => self.confirm(w),
                }
            }
            RawEvent::Finished | RawEvent::Cancelled => self.complete(id),
            RawEvent::Error(e) => {
                self.backend_error(e);
                self.complete(id);
            }
        }
    }

    /// The utterance `id` started playing. Utterances in front of it are
    /// done (the engine moved on without reporting them).
    fn begin(&mut self, id: UtteranceId) {
        if self.playing.as_ref().is_some_and(|p| p.id == id) {
            return;
        }
        while let Some(front) = self.queue.front().map(|u| u.id) {
            if front == id {
                break;
            }
            self.queue.complete(front);
            if self.char_params == Some(front) {
                self.restore_char_params();
            }
        }
        let Some(u) = self.queue.submitted(id) else {
            return;
        };
        let kind = u.kind;
        let words = spoken_words(&u.text);
        let now = self.clock.now();
        let mut pacer = TimerPacer::new(
            self.config.pacing,
            words.len(),
            self.interval(),
            self.caps.contains(Caps::AUDIO_CLOCK),
        );
        let first = pacer.start(now, 0, self.caps.contains(Caps::WORD_EVENTS));
        self.playing = Some(Playing {
            id,
            kind,
            words,
            started: now,
            pacer,
            scheduled: Vec::new(),
            painted: None,
        });
        if let Some(w) = first {
            self.paint(w);
        }
    }

    /// An engine-confirmed word (on arrival, or at its audio time).
    fn confirm(&mut self, w: usize) {
        let now = self.clock.now();
        if let Some(p) = &mut self.playing {
            p.pacer.on_callback(w, now);
        }
        self.paint(w);
    }

    fn paint(&mut self, w: usize) {
        let Some(p) = &mut self.playing else {
            return;
        };
        if p.kind != UtteranceKind::Text {
            return;
        }
        let Some(range) = p.words.get(w).cloned() else {
            return;
        };
        p.painted = Some(w);
        let id = p.id;
        let Some(u) = self.queue.submitted(id) else {
            return;
        };
        let source = u.source_for(range);
        let key = (id, source);
        if self.last_position == Some(key) {
            return;
        }
        self.last_position = Some(key);
        self.out.push(SpeechStatus::Position {
            generation: self.reading_generation,
            utterance: id,
            source_range: source,
        });
    }

    fn complete(&mut self, id: UtteranceId) {
        self.queue.complete(id);
        if self.playing.as_ref().is_some_and(|p| p.id == id) {
            self.playing = None;
        }
        if self.char_params == Some(id) {
            self.restore_char_params();
        }
    }

    fn finish_if_done(&mut self) {
        if self.queue.is_empty() {
            // Everything handed over has ended; nothing left to stop.
            self.engine_busy = false;
        }
        if self.queue.is_empty() && self.backlog.is_empty() && self.paused.is_none() {
            self.playing = None;
            if self.reading {
                self.reading = false;
                self.out.push(SpeechStatus::Finished {
                    generation: self.reading_generation,
                });
            }
        }
    }

    fn run_timers(&mut self) {
        let now = self.clock.now();
        let due: Vec<usize> = match &mut self.playing {
            None => return,
            Some(p) => {
                let n = p.scheduled.partition_point(|(d, _)| *d <= now);
                p.scheduled.drain(..n).map(|(_, w)| w).collect()
            }
        };
        for w in due {
            self.confirm(w);
        }
        let ticks = self
            .playing
            .as_mut()
            .map(|p| p.pacer.tick(now))
            .unwrap_or_default();
        for w in ticks {
            self.paint(w);
        }
    }
}

/// Moves `byte` back to the start of its span unless the span is literal, so
/// a resume repeats a whole expansion or inserted phrase.
fn snap_to_span(u: &Utterance, byte: u32) -> u32 {
    u.offset_map
        .spans()
        .iter()
        .find(|s| s.spoken.start <= byte && byte < s.spoken.end)
        .filter(|s| s.kind != SpanKind::Literal)
        .map_or(byte, |s| s.spoken.start)
}

/// The utterance from spoken byte `byte` on, with its map cut to match.
/// `None` when nothing is left to speak.
fn trim_utterance(u: &Utterance, byte: u32) -> Option<Utterance> {
    if byte == 0 {
        return Some(u.clone());
    }
    let b = byte as usize;
    if b >= u.text.len() || !u.text.is_char_boundary(b) {
        return None;
    }
    let text = u.text[b..].to_owned();
    if text.trim().is_empty() {
        return None;
    }
    let mut spans = Vec::new();
    for s in u.offset_map.spans() {
        if s.spoken.end <= byte {
            continue;
        }
        let mut s = s.clone();
        if s.spoken.start < byte {
            if s.kind == SpanKind::Literal {
                let skip = u.text[s.spoken.start as usize..b].chars().count();
                s.source.start = s.source.start.saturating_add(skip);
            }
            s.spoken.start = byte;
        }
        s.spoken = (s.spoken.start - byte)..(s.spoken.end - byte);
        spans.push(s);
    }
    debug_assert!(spans.first().is_none_or(|s: &Span| s.spoken.start == 0));
    let offset_map = if u.offset_map.is_empty() {
        OffsetMap::default()
    } else {
        OffsetMap::from_spans(spans, &text).unwrap_or_else(|e| {
            log::warn!("could not trim offset map: {e}");
            OffsetMap::default()
        })
    };
    Some(Utterance {
        id: u.id,
        text,
        kind: u.kind,
        offset_map,
    })
}

#[cfg(test)]
mod tests;
