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
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use textweaver_core::{
    CapsIndication, CharPos, CharRange, OffsetMap, Pitch, PunctuationLevel, Rate, Span, SpanKind,
    SpokenBuilder, Utterance, UtteranceId, UtteranceKind, Volume,
};

use crate::backend::{
    BackendFactory, BackendId, Caps, EventSink, RawEvent, SpeechBackend, SpeechError, Voice,
    VoiceParams,
};
use crate::backends::{NullBackend, resolve_preferred_voice, resolve_voice};
use crate::normalize::{self, NormalizeConfig, Pipeline};
use crate::pacing::{
    Clock, PacingConfig, PlaybackClock, SystemClock, TimerPacer, spoken_words, word_interval,
};
use crate::queue::{DEFAULT_LOOKAHEAD, Generation, ReadingQueue};
use crate::voices::{VoiceCache, VoiceList};

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
    ///
    /// Also the last status of a speech thread that panicked: the service
    /// is dead after it ([`SpeechService::is_alive`] is false, and
    /// [`SpeechService::failure`] holds the reason). Every later
    /// command is ignored, so the frontend announces the message through
    /// something other than speech and starts a new service.
    BackendError(String),
    /// The engine crashed or went silent in the middle of a reading, and
    /// the service restarted it and goes on from the last confirmed word
    /// (the reading keeps its generation). `reason` reads well aloud
    /// ("Eloquence stopped unexpectedly", "no speech for 12 seconds").
    Restarted {
        /// The reading that goes on.
        generation: ReadingGeneration,
        /// What happened.
        reason: String,
    },
}

impl SpeechStatus {
    /// The reading this status belongs to, for statuses about a reading.
    pub fn generation(&self) -> Option<ReadingGeneration> {
        match self {
            SpeechStatus::Position { generation, .. }
            | SpeechStatus::Paused { generation, .. }
            | SpeechStatus::Stopped { generation }
            | SpeechStatus::Finished { generation }
            | SpeechStatus::Restarted { generation, .. } => Some(*generation),
            SpeechStatus::Capabilities { .. } | SpeechStatus::BackendError(_) => None,
        }
    }
}

/// When a reading's first audio started: one stamp of the
/// stop-to-first-audio probe (`cargo xtask bench`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FirstAudioStamp {
    /// The reading the audio belongs to.
    pub generation: ReadingGeneration,
    /// When the speech thread took the command that started (or
    /// restarted) the reading.
    pub requested: Instant,
    /// When the backend reported that the reading's first utterance
    /// started to sound ([`RawEvent::Started`]): for the engines whose
    /// audio textweaver plays, when the output took the first samples.
    pub started: Instant,
}

impl FirstAudioStamp {
    /// The speech thread's share: from the command to the first audio.
    pub fn latency(&self) -> Duration {
        self.started.saturating_duration_since(self.requested)
    }
}

/// The latest [`FirstAudioStamp`], shared between the speech thread, which
/// stamps it, and whoever measures (the benchmarks). Stamping costs one
/// lock per reading; nothing in the reader reads it.
#[derive(Clone, Debug, Default)]
pub struct FirstAudio(Arc<(Mutex<Option<FirstAudioStamp>>, Condvar)>);

impl FirstAudio {
    /// The latest stamp, if any reading has sounded yet.
    pub fn latest(&self) -> Option<FirstAudioStamp> {
        *self.0.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Waits up to `limit` for a stamp of reading `generation` or a later
    /// one, and returns it; `None` when none came in time.
    pub fn wait_for(
        &self,
        generation: ReadingGeneration,
        limit: Duration,
    ) -> Option<FirstAudioStamp> {
        let deadline = Instant::now() + limit;
        let (lock, ready) = &*self.0;
        let mut slot = lock.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if let Some(s) = *slot
                && s.generation >= generation
            {
                return Some(s);
            }
            let left = deadline.checked_duration_since(Instant::now())?;
            slot = ready
                .wait_timeout(slot, left)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }

    fn record(&self, stamp: FirstAudioStamp) {
        let (lock, ready) = &*self.0;
        *lock.lock().unwrap_or_else(|e| e.into_inner()) = Some(stamp);
        ready.notify_all();
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

/// How long [`SpeechService::sync`] waits for the speech thread.
pub const VOICES_TIMEOUT: Duration = Duration::from_secs(10);

/// How often the core polls the backend while speech is active.
pub const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// After this many `speak` failures in a row the reading stops, instead of
/// failing through every remaining sentence of the document in one go.
pub const MAX_CONSECUTIVE_FAILURES: u32 = 3;

/// A reading that is playing (not paused) and has made no progress (no
/// word and no finished utterance) for this long has stalled: an audio
/// device that stopped taking samples, a host that stopped answering. The
/// service resets the engine and reads on from the last confirmed word;
/// if it stalls again without progress, the reading stops with a message.
pub const STALL_TIMEOUT: Duration = Duration::from_secs(12);

/// Engine restarts (after a crash or a stall) allowed within one sentence,
/// even when the reading made progress between them. One more crash in
/// that sentence stops the reading with a message, so an engine that
/// fails every few words cannot restart forever.
pub const MAX_RESTARTS_PER_UTTERANCE: u32 = 3;

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
    Preview(String, String),
    Tone(f32, u32),
    Earcon(Earcon),
    /// The voice list arrived (from the backend's background listing).
    VoicesReady,
    /// Answer once every command before this one is handled.
    Sync(Sender<()>),
    Shutdown,
}

/// Handle to the speech thread. Cheap calls; all work happens on the thread.
///
/// **If the speech thread dies.** A panic on the speech thread (a bug in
/// the service or in a backend) is caught: the thread drops the backend
/// (which shuts its engine hosts down), sends one last
/// [`SpeechStatus::BackendError`] saying what happened, and ends. From
/// then on [`is_alive`](Self::is_alive) is false,
/// [`failure`](Self::failure) holds the reason, every command is
/// ignored, and [`poll_status`](Self::poll_status) returns
/// [`SpeechError::ServiceStopped`] once the last status has been read
/// (where [`try_status`](Self::try_status) only returns `None`). A
/// frontend should then:
///
/// 1. announce the message without speech (status line, screen reader),
///    since this service can no longer speak;
/// 2. drop this handle and [`spawn`](Self::spawn) a new service with a new
///    factory and the current settings (rate, pitch, volume, voice,
///    punctuation);
/// 3. forget the reading in progress (its generation belongs to the dead
///    service) and let the user start reading again, or start it again
///    from the last reported position.
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
    /// Cleared when the speech thread ends (a panic, or a lost handle).
    alive: Arc<AtomicBool>,
    /// Why the speech thread died, when it panicked.
    failure: Arc<Mutex<Option<String>>>,
    /// The backend's voices, listed once when it started.
    voices: VoiceCache,
    /// Called after the speech thread sends statuses ([`Waker`]).
    waker: WakerSlot,
    /// Stamped by the speech thread when a reading first sounds.
    first_audio: FirstAudio,
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
        let alive = Arc::new(AtomicBool::new(true));
        let failure = Arc::new(Mutex::new(None));
        let thread_alive = Arc::clone(&alive);
        let thread_failure = Arc::clone(&failure);
        let waker: WakerSlot = Arc::new(Mutex::new(None));
        let fatal_waker = Arc::clone(&waker);
        let first_audio = FirstAudio::default();
        let thread_first_audio = first_audio.clone();
        let link = StatusLink {
            tx: status_tx,
            caps: Arc::clone(&caps),
            waker: Arc::clone(&waker),
        };
        // The voice list's arrival reaches the speech thread as a command.
        let voices_tx = tx.clone();
        let thread = thread::Builder::new()
            .name("textweaver-speech".into())
            .spawn(move || {
                let fatal_tx = link.tx.clone();
                let ready_fail = ready_tx.clone();
                let outcome = catch_unwind(AssertUnwindSafe(move || {
                    speech_thread(
                        factory,
                        config,
                        clock,
                        &rx,
                        &ready_tx,
                        &link,
                        voices_tx,
                        thread_first_audio,
                    );
                }));
                if let Err(payload) = outcome {
                    let why = panic_message(payload.as_ref());
                    log::error!("the speech thread panicked: {why}");
                    let message = format!(
                        "speech stopped after an internal error ({why}); restart speech to go on"
                    );
                    *thread_failure.lock().unwrap_or_else(|e| e.into_inner()) = Some(why);
                    thread_alive.store(false, Ordering::SeqCst);
                    // Only one of these has a listener: `spawn` waits for
                    // the first until the backend is ready, the handle
                    // reads the second after that.
                    let _ = ready_fail.send(Err(SpeechError::Engine(message.clone())));
                    let _ = fatal_tx.send(SpeechStatus::BackendError(message));
                }
                thread_alive.store(false, Ordering::SeqCst);
                // The frontend learns the thread ended (a panic, or a
                // shutdown) without waiting for its next poll.
                wake(&fatal_waker);
            })
            .map_err(|e| SpeechError::Io(e.to_string()))?;
        let (backend_id, voices) = ready_rx.recv().map_err(|_| SpeechError::ServiceStopped)??;
        Ok(SpeechService {
            tx,
            status_rx,
            thread: Some(thread),
            backend_id,
            caps,
            last_reading: AtomicU64::new(0),
            alive,
            failure,
            voices,
            waker,
            first_audio,
        })
    }

    /// Sets (or, with `None`, clears) the callback the speech thread calls
    /// after it sends statuses: a word heard, reading finished, an error.
    /// A frontend's event loop can then wait for input and for this instead
    /// of polling [`try_status`](Self::try_status) on a timer. See
    /// [`Waker`].
    pub fn set_waker(&self, waker: Option<Waker>) {
        *self.waker.lock().unwrap_or_else(|e| e.into_inner()) = waker;
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

    /// When the latest reading first sounded, for the stop-to-first-audio
    /// probe. Take it before the handle moves into the application; the
    /// clone sees every later stamp.
    pub fn first_audio(&self) -> FirstAudio {
        self.first_audio.clone()
    }

    /// The backend's current capabilities (for example, to announce "pitch
    /// not supported by this voice"). The speech thread re-reads them after
    /// every parameter change, so they follow the selected voice; a change
    /// is also reported as [`SpeechStatus::Capabilities`].
    pub fn capabilities(&self) -> Caps {
        Caps::from_bits_retain(self.caps.load(Ordering::SeqCst))
    }

    /// Sends a command to the speech thread. A closed channel means the
    /// thread is gone: that is recorded (see [`is_alive`](Self::is_alive))
    /// rather than mistaken for a command that is still to be carried out.
    fn send(&self, cmd: Command) -> Result<(), SpeechError> {
        if self.tx.send(cmd).is_err() {
            if self.alive.swap(false, Ordering::SeqCst) {
                log::warn!("speech command dropped: the speech thread has stopped");
            }
            return Err(SpeechError::ServiceStopped);
        }
        Ok(())
    }

    /// Sends a command whose failure the caller learns from
    /// [`is_alive`](Self::is_alive) and the statuses.
    fn post(&self, cmd: Command) {
        let _ = self.send(cmd);
    }

    /// False once the speech thread has ended: it panicked (see the type's
    /// docs for what the frontend should do), or it was shut down. Every
    /// command is ignored from then on.
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst) && self.thread.as_ref().is_some_and(|t| !t.is_finished())
    }

    /// Why the speech thread died, when it panicked: the panic's own text
    /// (its last [`SpeechStatus::BackendError`] puts it in a sentence).
    pub fn failure(&self) -> Option<String> {
        self.failure
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Speaks `text` (not mapped to the document). See [`SayMode`].
    pub fn say(&self, text: impl Into<String>, mode: SayMode) {
        self.post(Command::Say(text.into(), mode));
    }
    /// Reads utterances in order, replacing any reading in progress.
    /// Returns the reading's generation, which every status about this
    /// reading carries (see [`ReadingGeneration`]).
    pub fn read(&self, utterances: Vec<Utterance>) -> ReadingGeneration {
        let generation = self.last_reading.fetch_add(1, Ordering::SeqCst) + 1;
        self.post(Command::Read(utterances, generation));
        generation
    }
    /// Stops all speech.
    pub fn stop(&self) {
        self.post(Command::Stop);
    }
    /// Pauses reading.
    pub fn pause(&self) {
        self.post(Command::Pause);
    }
    /// Resumes reading.
    pub fn resume(&self) {
        self.post(Command::Resume);
    }
    /// Resumes a paused reading from document position `pos` instead of the
    /// pause point (the cursor moved while paused). If `pos` is outside the
    /// paused reading, resumes at the pause point.
    pub fn resume_at(&self, pos: CharPos) {
        self.post(Command::ResumeAt(pos));
    }
    /// Skips to the next queued utterance.
    pub fn skip(&self) {
        self.post(Command::Skip);
    }
    /// Sets the rate.
    pub fn set_rate(&self, rate: Rate) {
        self.post(Command::SetRate(rate));
    }
    /// Sets the pitch.
    pub fn set_pitch(&self, pitch: Pitch) {
        self.post(Command::SetPitch(pitch));
    }
    /// Sets the volume.
    pub fn set_volume(&self, volume: Volume) {
        self.post(Command::SetVolume(volume));
    }
    /// Sets the voice (`None` for the engine default).
    pub fn set_voice(&self, voice: Option<String>) {
        self.post(Command::SetVoice(voice));
    }
    /// Sets punctuation verbosity.
    pub fn set_punctuation(&self, level: PunctuationLevel) {
        self.post(Command::SetPunctuation(level));
    }
    /// Enables or disables split caps.
    pub fn set_split_caps(&self, on: bool) {
        self.post(Command::SetSplitCaps(on));
    }
    /// Replaces the normalization settings (applies to utterances read
    /// after this call).
    pub fn set_normalization(&self, config: NormalizeConfig) {
        self.post(Command::SetNormalization(Box::new(config)));
    }
    /// Replaces the pacing settings (highlight speed, latency offset).
    pub fn set_pacing(&self, pacing: PacingConfig) {
        self.post(Command::SetPacing(pacing));
    }
    /// Speaks one character (scaled rate, caps indication), optionally
    /// mapped to a document position.
    pub fn speak_char(&self, c: char, at: Option<CharPos>) {
        self.post(Command::SpeakChar(c, at));
    }
    /// Speaks `text` once in `voice` (a voice id of this engine), then
    /// goes back to the voice in use: the voice manager's preview. It
    /// interrupts what is speaking, as a character does.
    pub fn preview(&self, voice: impl Into<String>, text: impl Into<String>) {
        self.post(Command::Preview(voice.into(), text.into()));
    }
    /// Plays a tone (ignored when the backend has no [`Caps::TONES`]).
    pub fn tone(&self, hz: f32, ms: u32) {
        self.post(Command::Tone(hz, ms));
    }
    /// Plays an earcon (ignored when the backend has no [`Caps::TONES`]).
    pub fn earcon(&self, earcon: Earcon) {
        self.post(Command::Earcon(earcon));
    }

    /// The backend's voices, from the list made once when it started;
    /// never waits (Phase 2). While the list is still being made (SAPI
    /// lists its voices in the background), this is an error saying so:
    /// use [`voice_list`](Self::voice_list) to tell that apart and
    /// [`voice_cache`](Self::voice_cache) to be told when they arrive.
    pub fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        self.voice_list().into_result()
    }

    /// What is known about the backend's voices now: still loading, the
    /// list, or why listing failed. Never waits.
    pub fn voice_list(&self) -> VoiceList {
        if !self.is_alive() {
            return VoiceList::Failed(SpeechError::ServiceStopped);
        }
        self.voices.get()
    }

    /// The backend's voice list, for waiting on it
    /// ([`VoiceCache::on_ready`], [`VoiceCache::wait`]).
    pub fn voice_cache(&self) -> &VoiceCache {
        &self.voices
    }

    /// Waits until the speech thread has handled every command sent before
    /// this call (a round trip), at most [`VOICES_TIMEOUT`]. False when it
    /// did not answer in time or has stopped.
    pub fn sync(&self) -> bool {
        let (reply, answer) = mpsc::channel();
        if self.send(Command::Sync(reply)).is_err() {
            return false;
        }
        match answer.recv_timeout(VOICES_TIMEOUT) {
            Ok(()) => true,
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => false,
        }
    }

    /// The next status update, if one is waiting. `None` both when nothing
    /// is waiting and when the speech thread has died; tell them apart with
    /// [`poll_status`](Self::poll_status) or [`is_alive`](Self::is_alive).
    pub fn try_status(&self) -> Option<SpeechStatus> {
        self.poll_status().ok().flatten()
    }

    /// The next status update: `Ok(Some(_))` when one is waiting, `Ok(None)`
    /// when none is waiting and the speech thread is running, and
    /// [`SpeechError::ServiceStopped`] when the thread has ended and every
    /// status it sent has been read (after a panic, the last one is the
    /// [`SpeechStatus::BackendError`] that says why).
    pub fn poll_status(&self) -> Result<Option<SpeechStatus>, SpeechError> {
        match self.status_rx.try_recv() {
            Ok(s) => Ok(Some(s)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                self.alive.store(false, Ordering::SeqCst);
                Err(SpeechError::ServiceStopped)
            }
        }
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
        self.post(Command::Shutdown);
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

/// A callback the speech thread calls after it sends statuses, so a
/// frontend's event loop can sleep until there is something to apply
/// instead of polling (Wave 3). It runs on the speech thread: it must be
/// quick and must not block (post an event to the GUI's event loop, send on
/// a channel).
pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// Where the waker is kept: shared by the handle, which sets it, and the
/// speech thread, which calls it.
type WakerSlot = Arc<Mutex<Option<Waker>>>;

/// Calls the waker in `slot`, if one is set.
fn wake(slot: &WakerSlot) {
    let waker = slot.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if let Some(w) = waker {
        w();
    }
}

/// The speech thread's side of the status channel; it also keeps the
/// handle's copy of the capabilities current.
struct StatusLink {
    tx: Sender<SpeechStatus>,
    caps: Arc<AtomicU32>,
    waker: WakerSlot,
}

impl StatusLink {
    /// Sends `statuses`, then wakes the frontend if any were sent; false
    /// when the handle is gone.
    fn forward(&self, statuses: Vec<SpeechStatus>) -> bool {
        let any = !statuses.is_empty();
        for s in statuses {
            if let SpeechStatus::Capabilities { caps } = &s {
                self.caps.store(caps.bits(), Ordering::SeqCst);
            }
            if self.tx.send(s).is_err() {
                return false;
            }
        }
        if any {
            wake(&self.waker);
        }
        true
    }
}

/// The speech thread: creates the backend, reports readiness, and runs the
/// command loop. A panic anywhere in here is caught by the caller.
#[allow(clippy::too_many_arguments)]
fn speech_thread(
    factory: BackendFactory,
    config: ServiceConfig,
    clock: Box<dyn Clock>,
    rx: &Receiver<Command>,
    ready: &Sender<Result<(BackendId, VoiceCache), SpeechError>>,
    link: &StatusLink,
    voices_tx: Sender<Command>,
    first_audio: FirstAudio,
) {
    match factory() {
        Ok(backend) => {
            let mut core = ServiceCore::new(backend, config, clock);
            core.first_audio = first_audio;
            link.caps
                .store(core.capabilities().bits(), Ordering::SeqCst);
            let voices = core.voice_cache().clone();
            if voices.get().is_loading() {
                voices.on_ready(move || {
                    let _ = voices_tx.send(Command::VoicesReady);
                });
            }
            let _ = ready.send(Ok((core.backend_id(), voices)));
            if link.forward(core.take_statuses()) {
                run(&mut core, rx, link);
            }
        }
        Err(e) => {
            let _ = ready.send(Err(e));
        }
    }
}

/// The text of a panic payload.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_owned())
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
    /// Parameters were changed for this one utterance (a character at its
    /// own rate, a voice preview); the usual ones come back when it ends.
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
    /// Engine restarts (after a crash or a stall) since the reading last
    /// got past the point the latest restart resumed from.
    recoveries: u32,
    /// Where the latest restart resumed (the document position of the
    /// repeated word). Only a word reported beyond it, or the end of the
    /// sentence holding it, is progress: the repeated word itself is not,
    /// or an engine that crashes on the same word would restart forever.
    recovery_mark: Option<CharPos>,
    /// Restarts within one sentence, keyed by where the sentence ends in
    /// the document (unchanged when a restart trims its start), for
    /// [`MAX_RESTARTS_PER_UTTERANCE`].
    sentence_restarts: Option<(Option<CharPos>, u32)>,
    /// Error events since the reading last made progress.
    event_failures: u32,
    /// The backend's voices, listed once when it started.
    voices: VoiceCache,
    /// A voice asked for while the list was loading, resolved (a name
    /// such as "Zira" to its id) once it arrives.
    pending_voice: Option<String>,
    /// `prefer_voice` could not be resolved yet: the list was loading.
    prefer_pending: bool,
    /// When (wall clock) the reading last made progress or was handed to
    /// the engine, for [`STALL_TIMEOUT`].
    last_progress: Duration,
    /// A reading was started or restarted and has not sounded yet: its
    /// generation and when the command came, for [`FirstAudio`].
    awaiting_audio: Option<(ReadingGeneration, Instant)>,
    /// Where the first sound of each reading is stamped.
    first_audio: FirstAudio,
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
        // Listed once: in the background by backends that list slowly,
        // else here, with the answer (or the failure) kept.
        let voices = backend
            .voice_cache()
            .unwrap_or_else(|| VoiceCache::ready(backend.voices()));
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
            recoveries: 0,
            recovery_mark: None,
            sentence_restarts: None,
            event_failures: 0,
            last_progress: Duration::ZERO,
            voices,
            pending_voice: None,
            prefer_pending: false,
            awaiting_audio: None,
            first_audio: FirstAudio::default(),
        };
        if let Some(asked) = core.params.voice.clone() {
            core.params.voice = Some(core.resolve_voice_name(&asked));
        } else if let Some(prefer) = core.config.prefer_voice.clone() {
            match core.voices.get() {
                VoiceList::Ready(voices) => {
                    core.params.voice = resolve_preferred_voice(&voices, &prefer);
                }
                VoiceList::Loading => core.prefer_pending = true,
                VoiceList::Failed(_) => {}
            }
        }
        core.apply_params();
        // The starting capabilities are what the handle reports at spawn;
        // only later changes are statuses.
        core.out
            .retain(|s| !matches!(s, SpeechStatus::Capabilities { .. }));
        core
    }

    /// When each reading first sounded (see [`SpeechService::first_audio`]).
    pub fn first_audio(&self) -> &FirstAudio {
        &self.first_audio
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

    /// The backend's voices, from the list made when it started.
    pub fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        self.voices.get().into_result()
    }

    /// The backend's voice list.
    pub fn voice_cache(&self) -> &VoiceCache {
        &self.voices
    }

    /// The voice list arrived: resolves a voice asked for by name while it
    /// was loading, and the preferred voice, and applies them.
    pub fn voices_arrived(&mut self) {
        let VoiceList::Ready(voices) = self.voices.get() else {
            self.pending_voice = None;
            self.prefer_pending = false;
            return;
        };
        let mut changed = false;
        if let Some(asked) = self.pending_voice.take()
            && self.params.voice.as_deref() == Some(asked.as_str())
            && let Some(id) = resolve_voice(&voices, &asked)
            && id != asked
        {
            self.params.voice = Some(id);
            changed = true;
        }
        if std::mem::take(&mut self.prefer_pending)
            && self.params.voice.is_none()
            && let Some(prefer) = self.config.prefer_voice.clone()
        {
            self.params.voice = resolve_preferred_voice(&voices, &prefer);
            changed |= self.params.voice.is_some();
        }
        if changed {
            self.apply_params();
        } else {
            // The list can tell a backend more about its voice (SAPI's
            // voice families): its capabilities may follow.
            self.refresh_caps();
        }
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
            Command::Preview(voice, text) => self.preview(&voice, &text),
            Command::Tone(hz, ms) => self.tone(hz, ms),
            Command::Earcon(e) => self.earcon(e),
            Command::VoicesReady => self.voices_arrived(),
            Command::Sync(reply) => {
                let _ = reply.send(());
            }
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
        self.recoveries = 0;
        self.recovery_mark = None;
        self.sentence_restarts = None;
        self.event_failures = 0;
        self.last_progress = self.clock.wall();
        if self.backlog.is_empty() {
            self.reading = false;
            self.out.push(SpeechStatus::Finished { generation });
            return;
        }
        self.reading = true;
        self.awaiting_audio = Some((generation, Instant::now()));
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
        self.awaiting_audio = None;
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

    /// Speaks `text` once in `voice`, interrupting what is speaking, then
    /// goes back to the usual parameters when it ends (as
    /// [`speak_char`](Self::speak_char) does for its rate). `voice` is a
    /// voice id of this engine, used as it is.
    pub fn preview(&mut self, voice: &str, text: &str) {
        self.demote_native_pause();
        self.interrupt();
        let u = self.normalize(Utterance::announcement(text));
        let mut p = self.params.clone();
        p.voice = Some(voice.to_owned());
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
        let changed = self.params.rate != rate.clamped();
        self.params.rate = rate.clamped();
        self.apply_params();
        if self.caps.contains(Caps::LIVE_RATE) {
            let interval = self.interval();
            if let Some(p) = &mut self.playing {
                p.pacer.set_interval(interval);
            }
        } else if changed {
            self.respeak_with_new_params();
        }
    }

    /// Sets the pitch.
    pub fn set_pitch(&mut self, pitch: Pitch) {
        let changed = self.params.pitch != pitch.clamped();
        self.params.pitch = pitch.clamped();
        self.apply_params();
        if changed && !self.caps.contains(Caps::LIVE_RATE) {
            self.respeak_with_new_params();
        }
    }

    /// Sets the volume.
    pub fn set_volume(&mut self, volume: Volume) {
        let changed = self.params.volume != volume;
        self.params.volume = volume;
        self.apply_params();
        if changed && !self.caps.contains(Caps::LIVE_RATE) {
            self.respeak_with_new_params();
        }
    }

    /// Engines without [`Caps::LIVE_RATE`] (Eloquence, SAPI) have the
    /// playing sentence and the lookahead synthesized already, so a new
    /// rate, pitch, or volume used to be heard only two or three sentences
    /// later. While a reading plays, restart it from the last confirmed
    /// word (may repeat a word, never skips one), as an announcement does.
    ///
    /// Only engines that report words: a timer-paced engine resumes one
    /// word behind the painted highlight, and a frontend that judges a
    /// restarted reading by its positions (textweaver-app before it tracks
    /// reading generations) would take the step back for a stale reading.
    fn respeak_with_new_params(&mut self) {
        let playing = self.paused.is_none()
            && self.reading
            && self.char_params.is_none()
            && self.caps.contains(Caps::WORD_EVENTS)
            && self.queue.iter().any(|u| u.kind == UtteranceKind::Text);
        if !playing {
            return;
        }
        let (byte, _) = self.resume_point();
        let rest = self.remainder(byte);
        let backlog = std::mem::take(&mut self.backlog);
        self.clear_engine();
        self.restart(rest, backlog, true);
    }

    /// Sets the voice.
    pub fn set_voice(&mut self, voice: Option<String>) {
        self.pending_voice = None;
        self.params.voice = voice.map(|v| self.resolve_voice_name(&v));
        self.apply_params();
    }

    /// A voice id for what the user typed (an id, a name such as "Zira" or
    /// "Reed", or part of one; see [`resolve_voice`]). Unknown text is
    /// passed through unchanged so the backend reports it.
    ///
    /// While the voice list is still loading, the text is passed through
    /// and resolved when the list arrives ([`voices_arrived`](Self::voices_arrived)).
    fn resolve_voice_name(&mut self, asked: &str) -> String {
        match self.voices.get() {
            VoiceList::Ready(voices) => {
                self.pending_voice = None;
                resolve_voice(&voices, asked).unwrap_or_else(|| asked.to_owned())
            }
            VoiceList::Loading => {
                self.pending_voice = Some(asked.to_owned());
                asked.to_owned()
            }
            VoiceList::Failed(_) => asked.to_owned(),
        }
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
        self.check_stall();
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
        if reading {
            self.awaiting_audio = Some((self.reading_generation, Instant::now()));
        }
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
                    Ok(()) => {
                        self.failures = 0;
                        // Handing the engine more counts for the watchdog:
                        // starting a host can take seconds.
                        self.last_progress = self.clock.wall();
                    }
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
                let reached = self
                    .queue
                    .submitted(id)
                    .filter(|u| u.kind == UtteranceKind::Text)
                    .and_then(|u| u.source_for(byte_range))
                    .map(|r| r.start);
                self.progress(reached);
            }
            RawEvent::Finished => {
                let reached = self
                    .queue
                    .submitted(id)
                    .filter(|u| u.kind == UtteranceKind::Text)
                    .and_then(Utterance::source_range)
                    .map(|r| r.end);
                self.progress(reached);
                self.complete(id);
            }
            RawEvent::Cancelled => self.complete(id),
            RawEvent::Error(e) => {
                let text = self
                    .queue
                    .submitted(id)
                    .is_some_and(|u| u.kind == UtteranceKind::Text);
                if text && self.reading && self.paused.is_none() && self.recoveries == 0 {
                    // A host crash fails everything it owed: read on from
                    // the last confirmed word with a fresh engine instead
                    // of skipping the rest of the sentence and the
                    // lookahead (audit finding R4).
                    self.recover(&e);
                    return;
                }
                self.event_failures = self.event_failures.saturating_add(1);
                self.backend_error(e);
                self.complete(id);
                if self.event_failures >= MAX_CONSECUTIVE_FAILURES {
                    self.give_up();
                }
            }
        }
    }

    /// The reading moved on: a word was reported or an utterance finished.
    /// `reached` is where in the document: the start of the word, or the
    /// end of the finished sentence (`None` for inserted speech and
    /// announcements).
    ///
    /// Any progress keeps the watchdog quiet. Only progress beyond the
    /// point the latest restart resumed from allows another restart: the
    /// resumed reading repeats the last confirmed word, and an engine that
    /// crashes on the same text reports that word every time.
    fn progress(&mut self, reached: Option<CharPos>) {
        self.event_failures = 0;
        self.last_progress = self.clock.wall();
        if self.recoveries == 0 {
            return;
        }
        let passed = match (self.recovery_mark, reached) {
            (Some(mark), Some(at)) => at > mark,
            (None, reached) => reached.is_some(),
            (Some(_), None) => false,
        };
        if passed {
            self.recoveries = 0;
            self.recovery_mark = None;
        }
    }

    /// Resets the engine and reads on from the last confirmed word, once
    /// per stretch without progress past the previous resume point, and at
    /// most [`MAX_RESTARTS_PER_UTTERANCE`] times per sentence; reports
    /// [`SpeechStatus::Restarted`].
    fn recover(&mut self, reason: &str) {
        let sentence_end = self
            .queue
            .iter()
            .find(|u| u.kind == UtteranceKind::Text)
            .and_then(Utterance::source_range)
            .map(|r| r.end);
        let count = match self.sentence_restarts {
            Some((end, n)) if end == sentence_end => n.saturating_add(1),
            _ => 1,
        };
        if count > MAX_RESTARTS_PER_UTTERANCE {
            log::warn!("speech: not restarting the engine again: {reason}");
            let was_reading = self.reading;
            self.stop_silently();
            self.backend.reset();
            self.backend_error(format!(
                "the speech engine stopped {count} times in the same sentence ({reason}), so reading stopped"
            ));
            if was_reading {
                self.out.push(SpeechStatus::Stopped {
                    generation: self.reading_generation,
                });
            }
            return;
        }
        self.sentence_restarts = Some((sentence_end, count));
        log::warn!("speech: restarting the engine: {reason}");
        self.recoveries = self.recoveries.saturating_add(1);
        let (byte, resume_at) = self.resume_point();
        self.recovery_mark = resume_at;
        let rest = self.remainder(byte);
        let backlog = std::mem::take(&mut self.backlog);
        self.clear_engine();
        self.backend.reset();
        self.out.push(SpeechStatus::Restarted {
            generation: self.reading_generation,
            reason: reason.to_owned(),
        });
        self.last_progress = self.clock.wall();
        self.restart(rest, backlog, true);
    }

    /// The watchdog: a playing reading with no progress for
    /// [`STALL_TIMEOUT`] is restarted once, then stopped with a message
    /// (audit finding R5).
    fn check_stall(&mut self) {
        if !self.reading || self.paused.is_some() || self.queue.is_empty() || !self.engine_busy {
            return;
        }
        let now = self.clock.wall();
        if now.saturating_sub(self.last_progress) < STALL_TIMEOUT {
            return;
        }
        let secs = STALL_TIMEOUT.as_secs();
        if self.recoveries == 0 {
            self.recover(&format!("no speech for {secs} seconds"));
            return;
        }
        let was_reading = self.reading;
        self.stop_silently();
        self.backend_error(format!(
            "no speech for {secs} seconds, even after restarting the voice, so reading stopped. Check the audio device"
        ));
        if was_reading {
            self.out.push(SpeechStatus::Stopped {
                generation: self.reading_generation,
            });
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
        if let Some((generation, requested)) = self.awaiting_audio.take() {
            self.first_audio.record(FirstAudioStamp {
                generation,
                requested,
                started: Instant::now(),
            });
        }
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
