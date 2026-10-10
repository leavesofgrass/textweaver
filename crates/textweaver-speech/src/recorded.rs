//! Recorded audio played in place of speech: the narration of DAISY books.
//!
//! A DAISY book with audio pairs each phrase of its text with a clip of a
//! recording (SMIL `par` elements). To read such a book, the frontend plans
//! one utterance per phrase and hands the service a [`RecordedPlan`] naming
//! the clips of each phrase ([`SpeechService::set_recorded`]). Every
//! backend the service runs is wrapped in a layer that sends an utterance
//! whose text starts inside a planned phrase to a [`ClipPlayer`] instead of
//! the engine, and every other utterance (a phrase with no audio, an
//! announcement) to the engine. The two never sound together: an
//! utterance waits until the other side has finished what it was given,
//! so speech takes over where the book has text without audio, in order.
//!
//! The reading pipeline stays the same: the player reports `Started`,
//! `Word` and `Finished` against its own playback clock, so the highlight,
//! pause, stop and "read on" work as with any engine. Rate changes do not
//! apply to recorded audio.
//!
//! [`SpeechService::set_recorded`]: crate::SpeechService::set_recorded

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use textweaver_core::{CharRange, Utterance, UtteranceId, UtteranceKind, Volume};

use crate::backend::{
    BackendId, Caps, EventSink, FileSynthesis, RawEvent, SpeechBackend, SpeechError, Voice,
    VoiceParams,
};
use crate::voices::VoiceCache;

/// One clip of a recording: `begin` to `end` of an audio file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordedClip {
    /// The audio file (MP3 or WAV), possibly inside an archive.
    pub file: PathBuf,
    /// Where the clip starts in the file.
    pub begin: Duration,
    /// Where it ends; `None` plays to the end of the file.
    pub end: Option<Duration>,
}

/// One phrase of the text and the clips that say it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordedPar {
    /// The phrase's text in the document.
    pub range: CharRange,
    /// Its clips, played back to back.
    pub clips: Vec<RecordedClip>,
}

/// Creates the clip player on the speech thread.
pub type PlayerFactory =
    Arc<dyn Fn() -> Result<Box<dyn ClipPlayer>, SpeechError> + Send + Sync + 'static>;

/// The phrases of one reading that play recorded audio.
#[derive(Clone)]
pub struct RecordedPlan {
    /// The phrases, in document order, not overlapping.
    pub pars: Vec<RecordedPar>,
    /// Makes the player (once per service, on first use).
    pub player: PlayerFactory,
}

impl std::fmt::Debug for RecordedPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordedPlan")
            .field("pars", &self.pars.len())
            .finish_non_exhaustive()
    }
}

impl RecordedPlan {
    /// The clips of the phrase holding `start`, if one does.
    fn clips_at(&self, start: usize) -> Option<&[RecordedClip]> {
        let i = self.pars.partition_point(|p| p.range.end.0 <= start);
        self.pars
            .get(i)
            .filter(|p| p.range.start.0 <= start && start < p.range.end.0)
            .map(|p| p.clips.as_slice())
    }
}

/// Plays clips of recorded audio with the events of an engine.
///
/// For each utterance given to [`play`](Self::play): `Started` at its
/// first sample, `Word` events for the utterance's words as the audio
/// reaches them, and `Finished` after its last sample, all from
/// [`poll`](Self::poll); `Cancelled` for each one [`stop`](Self::stop)
/// discarded. Utterances play back to back in the order given.
pub trait ClipPlayer {
    /// Queues utterance `id`, whose text is `text`, as `clips`.
    fn play(&mut self, id: UtteranceId, text: &str, clips: &[RecordedClip]);
    /// Decodes ahead and delivers the events the audio has reached.
    fn poll(&mut self, sink: &mut dyn EventSink);
    /// Drops everything queued and the audio playing.
    fn stop(&mut self);
    /// Pauses the audio and its clock.
    fn pause(&mut self);
    /// Resumes after [`pause`](Self::pause).
    fn resume(&mut self);
    /// Sets the volume.
    fn set_volume(&mut self, volume: Volume);
}

/// An utterance waiting for the other side to finish.
#[derive(Debug)]
struct Waiting {
    utterance: Utterance,
    /// Its clips; `None` for the engine.
    clips: Option<Vec<RecordedClip>>,
    /// Silence after it, asked for while it waited (engine side only).
    silence_ms: Option<u32>,
}

/// Records which utterances ended while forwarding every event.
struct Watch<'a> {
    sink: &'a mut dyn EventSink,
    open: &'a mut Vec<UtteranceId>,
}

impl EventSink for Watch<'_> {
    fn emit(&mut self, id: UtteranceId, event: RawEvent) {
        if matches!(event, RawEvent::Finished | RawEvent::Cancelled) {
            self.open.retain(|o| *o != id);
        }
        self.sink.emit(id, event);
    }

    fn is_current(&self, id: UtteranceId) -> bool {
        self.sink.is_current(id)
    }
}

/// The layer around every backend the service runs: routes each utterance
/// to the engine or to the clip player (see the module docs). With no plan
/// set it passes everything to the engine.
pub(crate) struct Mixed {
    inner: Box<dyn SpeechBackend>,
    plan: Option<Arc<RecordedPlan>>,
    player: Option<Box<dyn ClipPlayer>>,
    volume: Volume,
    /// Utterances given to the engine and not yet ended.
    on_inner: Vec<UtteranceId>,
    /// Utterances given to the player and not yet ended.
    on_player: Vec<UtteranceId>,
    waiting: VecDeque<Waiting>,
    /// Waiting utterances dropped by `stop`, reported on the next poll.
    cancelled: Vec<UtteranceId>,
    /// The engine was paused natively (so it is resumed).
    inner_paused: bool,
}

impl Mixed {
    /// The layer around `inner`.
    pub(crate) fn new(inner: Box<dyn SpeechBackend>) -> Self {
        Mixed {
            inner,
            plan: None,
            player: None,
            volume: Volume::default(),
            on_inner: Vec::new(),
            on_player: Vec::new(),
            waiting: VecDeque::new(),
            cancelled: Vec::new(),
            inner_paused: false,
        }
    }

    /// Sets the recorded phrases of the reading about to start, or none.
    pub(crate) fn set_recorded(&mut self, plan: Option<Arc<RecordedPlan>>) {
        self.plan = plan;
    }

    /// The clips for `u`, when the plan says it plays recorded audio.
    fn route(&self, u: &Utterance) -> Option<Vec<RecordedClip>> {
        if u.kind != UtteranceKind::Text {
            return None;
        }
        let start = u.source_range()?.start.0;
        let plan = self.plan.as_ref()?;
        plan.clips_at(start).map(|c| c.to_vec())
    }

    /// True when an utterance for the player (`recorded`) or the engine may
    /// start now: the other side has nothing left to say.
    fn free(&self, recorded: bool) -> bool {
        if recorded {
            self.on_inner.is_empty()
        } else {
            self.on_player.is_empty()
        }
    }

    /// Hands `w` to its side; an engine's refusal is returned.
    fn dispatch(&mut self, w: Waiting, sink: &mut dyn EventSink) -> Result<(), SpeechError> {
        let id = w.utterance.id;
        if let Some(clips) = &w.clips {
            self.ensure_player();
            if let Some(p) = &mut self.player {
                self.on_player.push(id);
                p.play(id, &w.utterance.text, clips);
                return Ok(());
            }
            // No player: speech says the phrase instead.
        }
        self.on_inner.push(id);
        let result = {
            let mut watch = Watch {
                sink: &mut *sink,
                open: &mut self.on_inner,
            };
            self.inner.speak(&w.utterance, &mut watch)
        };
        if let Err(e) = result {
            self.on_inner.retain(|o| *o != id);
            return Err(e);
        }
        if let Some(ms) = w.silence_ms {
            self.inner.silence_after(id, ms);
        }
        Ok(())
    }

    /// Makes the player from the plan, once.
    fn ensure_player(&mut self) {
        if self.player.is_some() {
            return;
        }
        let Some(plan) = &self.plan else { return };
        let make = Arc::clone(&plan.player);
        match make() {
            Ok(mut p) => {
                p.set_volume(self.volume);
                self.player = Some(p);
            }
            Err(e) => log::warn!("recorded audio cannot play: {e}"),
        }
    }

    /// Starts every waiting utterance whose side is free, in order.
    fn drain(&mut self, sink: &mut dyn EventSink) {
        while let Some(w) = self.waiting.front() {
            if !self.free(w.clips.is_some()) {
                break;
            }
            let Some(w) = self.waiting.pop_front() else {
                break;
            };
            let id = w.utterance.id;
            if let Err(e) = self.dispatch(w, sink) {
                // It waited, so the service has moved on: it hears why
                // here, and the utterance is over.
                sink.emit(id, RawEvent::Error(e.to_string()));
                sink.emit(id, RawEvent::Finished);
            }
        }
    }

    /// After `stop` or `reset`: the player drops its audio, and waiting
    /// utterances are reported cancelled on the next poll.
    fn end_all(&mut self) {
        if let Some(p) = &mut self.player {
            p.stop();
            p.resume();
        }
        self.inner_paused = false;
        self.cancelled
            .extend(self.waiting.drain(..).map(|w| w.utterance.id));
        self.on_inner.clear();
        self.on_player.clear();
    }
}

impl SpeechBackend for Mixed {
    fn id(&self) -> BackendId {
        self.inner.id()
    }

    fn capabilities(&self) -> Caps {
        let caps = self.inner.capabilities();
        // Recorded audio pauses natively. An engine without native pause
        // still refuses while it speaks, and the service emulates it.
        if self.plan.is_some() {
            caps | Caps::PAUSE
        } else {
            caps
        }
    }

    fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        self.inner.voices()
    }

    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
        // Only the volume applies to recorded audio, never the rate.
        self.volume = params.volume;
        if let Some(p) = &mut self.player {
            p.set_volume(params.volume);
        }
        self.inner.set_params(params)
    }

    fn effective_wpm(&self) -> u16 {
        self.inner.effective_wpm()
    }

    fn speak(
        &mut self,
        utterance: &Utterance,
        sink: &mut dyn EventSink,
    ) -> Result<(), SpeechError> {
        let w = Waiting {
            utterance: utterance.clone(),
            clips: self.route(utterance),
            silence_ms: None,
        };
        if self.waiting.is_empty() && self.free(w.clips.is_some()) {
            return self.dispatch(w, sink);
        }
        self.waiting.push_back(w);
        Ok(())
    }

    fn poll(&mut self, sink: &mut dyn EventSink) {
        for id in self.cancelled.drain(..) {
            sink.emit(id, RawEvent::Cancelled);
        }
        {
            let mut watch = Watch {
                sink: &mut *sink,
                open: &mut self.on_inner,
            };
            self.inner.poll(&mut watch);
        }
        if let Some(p) = &mut self.player {
            let mut watch = Watch {
                sink: &mut *sink,
                open: &mut self.on_player,
            };
            p.poll(&mut watch);
        }
        self.drain(sink);
    }

    fn silence_after(&mut self, id: UtteranceId, ms: u32) {
        if let Some(w) = self.waiting.iter_mut().find(|w| w.utterance.id == id) {
            if w.clips.is_none() {
                w.silence_ms = Some(ms);
            }
        } else if !self.on_player.contains(&id) {
            // The engine has it (it may have finished already).
            self.inner.silence_after(id, ms);
        }
        // Recorded audio carries its own pauses.
    }

    fn stop(&mut self) {
        self.inner.stop();
        self.end_all();
    }

    fn reset(&mut self) {
        self.inner.reset();
        self.end_all();
    }

    fn pause(&mut self) -> Result<(), SpeechError> {
        if !self.on_inner.is_empty() {
            self.inner.pause()?;
            self.inner_paused = true;
        }
        if let Some(p) = &mut self.player {
            p.pause();
        }
        Ok(())
    }

    fn resume(&mut self) -> Result<(), SpeechError> {
        if let Some(p) = &mut self.player {
            p.resume();
        }
        if std::mem::take(&mut self.inner_paused) {
            self.inner.resume()?;
        }
        Ok(())
    }

    fn synthesize_to_file(
        &mut self,
        text: &str,
        path: &std::path::Path,
    ) -> Result<(), SpeechError> {
        self.inner.synthesize_to_file(text, path)
    }

    fn synthesize_utterance(
        &mut self,
        utterance: &Utterance,
        path: &std::path::Path,
    ) -> Result<FileSynthesis, SpeechError> {
        self.inner.synthesize_utterance(utterance, path)
    }

    fn tone(&mut self, hz: f32, ms: u32) {
        self.inner.tone(hz, ms);
    }

    fn voice_cache(&self) -> Option<VoiceCache> {
        self.inner.voice_cache()
    }
}

#[cfg(test)]
mod tests;
