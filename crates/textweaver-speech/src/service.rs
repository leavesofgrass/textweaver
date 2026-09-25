//! The speech service: a speech thread driven by a command channel.
//!
//! Phase 0 behavior (Agent B replaces it): utterances are spoken in order;
//! each reports one `Position` covering its whole source range when it
//! starts, and `Finished` after the last. Word events, pacing, pause
//! emulation, lookahead, and normalization come in wave 1.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use serde::{Deserialize, Serialize};
use textweaver_core::{
    CharPos, CharRange, Pitch, PunctuationLevel, Rate, Utterance, UtteranceId, Volume,
};

use crate::backend::{
    BackendFactory, EventSink, RawEvent, SpeechBackend, SpeechError, VoiceParams,
};
use crate::backends::NullBackend;
use crate::pacing::PacingConfig;
use crate::queue::Generation;

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

/// What the service reports back to the application.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeechStatus {
    /// The highlight should cover `source_range` (`None` inside inserted
    /// speech such as "heading level 2").
    Position {
        /// The utterance being spoken.
        utterance: UtteranceId,
        /// Document range to highlight.
        source_range: Option<CharRange>,
    },
    /// Speech paused; `resume_at` is where reading continues (the last
    /// confirmed word; may repeat, never skips).
    Paused {
        /// Resume position, if known.
        resume_at: Option<CharPos>,
    },
    /// Speech stopped by request.
    Stopped,
    /// Everything queued has been spoken.
    Finished,
    /// The backend failed.
    BackendError(String),
}

/// Service configuration.
#[derive(Clone, Debug, Default)]
pub struct ServiceConfig {
    /// Initial voice parameters.
    pub params: VoiceParams,
    /// Pacing parameters.
    pub pacing: PacingConfig,
    /// Initial punctuation level.
    pub punctuation: PunctuationLevel,
    /// Speak capitals within words separately ("split caps").
    pub split_caps: bool,
}

/// Commands sent to the speech thread.
#[derive(Debug)]
enum Command {
    Say(String, SayMode),
    Read(Vec<Utterance>),
    Stop,
    Pause,
    Resume,
    Skip,
    SetRate(Rate),
    SetPitch(Pitch),
    SetVolume(Volume),
    SetVoice(Option<String>),
    SetPunctuation(PunctuationLevel),
    SetSplitCaps(bool),
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
}

impl SpeechService {
    /// Starts the speech thread and creates the backend on it.
    pub fn spawn(factory: BackendFactory, config: ServiceConfig) -> Result<Self, SpeechError> {
        let (tx, rx) = mpsc::channel();
        let (status_tx, status_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("textweaver-speech".into())
            .spawn(move || match factory() {
                Ok(mut backend) => {
                    let _ = backend.set_params(&config.params);
                    let _ = ready_tx.send(Ok(()));
                    run(backend.as_mut(), config, &rx, &status_tx);
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            })
            .map_err(|e| SpeechError::Io(e.to_string()))?;
        ready_rx.recv().map_err(|_| SpeechError::ServiceStopped)??;
        Ok(SpeechService {
            tx,
            status_rx,
            thread: Some(thread),
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

    fn send(&self, cmd: Command) {
        // A closed channel means the thread is gone; there is nobody to tell.
        let _ = self.tx.send(cmd);
    }

    /// Speaks `text` (not mapped to the document).
    pub fn say(&self, text: impl Into<String>, mode: SayMode) {
        self.send(Command::Say(text.into(), mode));
    }
    /// Reads utterances in order, replacing any reading in progress.
    pub fn read(&self, utterances: Vec<Utterance>) {
        self.send(Command::Read(utterances));
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
    /// Speaks one character (scaled rate, caps indication), optionally
    /// mapped to a document position.
    pub fn speak_char(&self, c: char, at: Option<CharPos>) {
        self.send(Command::SpeakChar(c, at));
    }
    /// Plays a tone.
    pub fn tone(&self, hz: f32, ms: u32) {
        self.send(Command::Tone(hz, ms));
    }
    /// Plays an earcon.
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

/// Drops stale events and forwards errors (Phase 0: positions are reported
/// per utterance, so word events are not forwarded yet).
struct Sink<'a> {
    generation: Generation,
    status: &'a Sender<SpeechStatus>,
}

impl EventSink for Sink<'_> {
    fn emit(&mut self, id: UtteranceId, event: RawEvent) {
        if !self.generation.is_current(id) {
            return;
        }
        if let RawEvent::Error(e) = event {
            let _ = self.status.send(SpeechStatus::BackendError(e));
        }
    }

    fn is_current(&self, id: UtteranceId) -> bool {
        self.generation.is_current(id)
    }
}

fn run(
    backend: &mut dyn SpeechBackend,
    config: ServiceConfig,
    rx: &Receiver<Command>,
    status: &Sender<SpeechStatus>,
) {
    let mut generation = Generation::default();
    let mut params = config.params;
    let send = |s: SpeechStatus| {
        let _ = status.send(s);
    };
    while let Ok(cmd) = rx.recv() {
        match cmd {
            Command::Read(utterances) => {
                backend.stop();
                let g = generation.bump();
                for (i, mut u) in utterances.into_iter().enumerate() {
                    u.id = UtteranceId {
                        generation: g,
                        chunk: u32::try_from(i).unwrap_or(u32::MAX),
                    };
                    send(SpeechStatus::Position {
                        utterance: u.id,
                        source_range: u.source_range(),
                    });
                    let mut sink = Sink { generation, status };
                    if let Err(e) = backend.speak(&u, &mut sink) {
                        send(SpeechStatus::BackendError(e.to_string()));
                    }
                }
                send(SpeechStatus::Finished);
            }
            Command::Say(text, mode) => {
                if mode != SayMode::Queue {
                    backend.stop();
                }
                let mut u = Utterance::announcement(text);
                u.id = UtteranceId {
                    generation: generation.get(),
                    chunk: 0,
                };
                let mut sink = Sink { generation, status };
                let _ = backend.speak(&u, &mut sink);
            }
            Command::SpeakChar(c, at) => {
                let mut u = Utterance::character(c.to_string(), at);
                u.id = UtteranceId {
                    generation: generation.get(),
                    chunk: 0,
                };
                let mut sink = Sink { generation, status };
                let _ = backend.speak(&u, &mut sink);
            }
            Command::Stop => {
                generation.bump();
                backend.stop();
                send(SpeechStatus::Stopped);
            }
            Command::Pause => {
                let _ = backend.pause();
                send(SpeechStatus::Paused { resume_at: None });
            }
            Command::Resume => {
                let _ = backend.resume();
            }
            Command::Skip => {}
            Command::SetRate(r) => {
                params.rate = r;
                let _ = backend.set_params(&params);
            }
            Command::SetPitch(p) => {
                params.pitch = p;
                let _ = backend.set_params(&params);
            }
            Command::SetVolume(v) => {
                params.volume = v;
                let _ = backend.set_params(&params);
            }
            Command::SetVoice(v) => {
                params.voice = v;
                let _ = backend.set_params(&params);
            }
            Command::SetPunctuation(level) => {
                log::debug!("punctuation {level:?} (phase 0: ignored)")
            }
            Command::SetSplitCaps(on) => log::debug!("split caps {on} (phase 0: ignored)"),
            Command::Earcon(e) => log::debug!("earcon {e:?} (phase 0: ignored)"),
            Command::Tone(hz, ms) => backend.tone(hz, ms),
            Command::Shutdown => {
                backend.stop();
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn null_service_reports_positions_then_finished() {
        let s = SpeechService::null();
        s.read(vec![
            Utterance::literal("One.", CharPos(0)),
            Utterance::literal("Two.", CharPos(5)),
        ]);
        let mut got = Vec::new();
        while let Ok(st) = s.statuses().recv_timeout(Duration::from_secs(2)) {
            let done = st == SpeechStatus::Finished;
            got.push(st);
            if done {
                break;
            }
        }
        assert_eq!(got.len(), 3);
        assert!(matches!(
            got[1],
            SpeechStatus::Position { source_range: Some(r), .. } if r == CharRange::new(5, 9)
        ));
    }
}
