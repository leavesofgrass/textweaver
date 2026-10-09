//! Dictation in edit mode (Wave 6, Agent W6d; ADR-0042).
//!
//! The Dictate command starts and stops the microphone. While it records,
//! Whisper runs in-process (`textweaver-dictation`, live): the words it is
//! sure of arrive while the speaker talks and are shown on the status line
//! as they come, meaning first ("Dictating: notes for Monday."), in the
//! last 40 cells so a Braille display shows the newest words. At each pause
//! the phrase is typed at the caret, with spoken commands ("new line",
//! "period") applied, as one undo step through the editor's own path.
//!
//! **The voice while recording** (`[dictation] speak_while_recording`, off
//! by default): textweaver's voice must not be heard by the microphone, so
//! the words are held and said once, at the pause; the status line still
//! grows as they come. On, each burst is said as it arrives. Either way
//! they are said at `Polite` through the app's routing
//! ([`App::announce_as`], importance `Result`), so the interface
//! announcement level applies; errors are never silenced.
//!
//! **Nothing is lost silently.** Leaving edit mode, opening or starting
//! another document, and quitting first finish the dictation: recording
//! stops, the last phrase is transcribed and typed (at most ten seconds,
//! as quitting waits for the writer), and only then does the command go
//! on. If the model takes longer, the session is ended and that is said.
//!
//! Outside edit mode the command asks whether to turn edit mode on first.
//! Without the `dictation` feature the command stays hidden (it is a
//! pending command until [`register`] gives it a handler).
//!
//! **The model** (W8a-w): dictation uses the Whisper model chosen in the
//! settings (`[dictation] model`, base.en when none is chosen), an
//! optional component (crate::components) in `whisper/rten/<model>` in
//! the data folder, or the folder `[dictation] model_dir` names. When the
//! model is missing, Dictate asks to download it ("Dictation needs the
//! Whisper model, 79.3 MB, license MIT, unconfirmed. Download it now?"),
//! and dictation starts when the download finishes. A no is remembered
//! for the session. A damaged file, a missing file, or a missing folder
//! is said in words, never as a blank error.

use crate::app::App;
use crate::command::Effect;

/// How long finishing a dictation may take before a command that leaves
/// edit mode goes on without it (the writer thread's wait at quit).
#[cfg_attr(not(feature = "dictation"), allow(dead_code))]
pub(crate) const FINISH_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

/// The most cells the status line's dictated words take, with their
/// label: a 40-cell Braille display.
#[cfg_attr(not(feature = "dictation"), allow(dead_code))]
const STATUS_CELLS: usize = 40;

impl App {
    /// Finishes a dictation under way, then runs `then`: for the commands
    /// that end edit mode.
    pub(crate) fn dictation_finish_then(
        &mut self,
        then: fn(&mut App) -> Vec<Effect>,
    ) -> Vec<Effect> {
        self.dictation_finish();
        then(self)
    }
}

/// Gives the Dictate and Download the dictation model commands their
/// handlers (with the `dictation` feature).
pub(crate) fn register(app: &mut App) {
    #[cfg(feature = "dictation")]
    {
        app.register_handler(textweaver_keymap::ActionId::Dictate, handler);
        app.register_handler(
            textweaver_keymap::ActionId::DownloadDictationModel,
            download_handler,
        );
    }
    #[cfg(not(feature = "dictation"))]
    let _ = app;
}

#[cfg(feature = "dictation")]
fn handler(app: &mut App) -> Vec<Effect> {
    app.dictate()
}

#[cfg(feature = "dictation")]
fn download_handler(app: &mut App) -> Vec<Effect> {
    let id = crate::components::dictation_model_id(&app.settings).to_owned();
    app.ask_component_download(&id, crate::components::After::Nothing)
}

/// The last words of `words` that fit in `cells` characters, whole words
/// only (a single longer word is cut at its start instead).
#[cfg_attr(not(feature = "dictation"), allow(dead_code))]
pub(crate) fn tail_words(words: &str, cells: usize) -> String {
    let count = words.chars().count();
    if count <= cells {
        return words.to_owned();
    }
    let mut out: Vec<&str> = Vec::new();
    let mut used = 0usize;
    for w in words.split_whitespace().rev() {
        let n = w.chars().count() + usize::from(!out.is_empty());
        if used + n > cells {
            break;
        }
        used += n;
        out.push(w);
    }
    if out.is_empty() {
        let skip = count.saturating_sub(cells);
        return words.chars().skip(skip).collect();
    }
    out.reverse();
    out.join(" ")
}

/// Where dictated text joins the text before the caret: a space when a
/// word would otherwise run into the one before.
#[cfg_attr(not(feature = "dictation"), allow(dead_code))]
pub(crate) fn joined(before: Option<char>, text: &str) -> String {
    let needs_space = before.is_some_and(|c| !c.is_whitespace())
        && text
            .chars()
            .next()
            .is_some_and(|c| !c.is_whitespace() && !".,;:!?)]}\u{201d}'".contains(c));
    if needs_space {
        format!(" {text}")
    } else {
        text.to_owned()
    }
}

#[cfg(not(feature = "dictation"))]
mod off {
    use super::*;

    /// Without the feature: nothing to hold.
    #[derive(Debug, Default)]
    pub(crate) struct DictationSlot {
        pub(crate) question: bool,
    }

    impl App {
        pub(crate) fn dictation_tick(&mut self) -> Vec<Effect> {
            Vec::new()
        }

        pub(crate) fn dictation_finish(&mut self) {}

        pub(crate) fn dictation_shutdown(&mut self) {}

        pub(crate) fn confirm_dictation(
            &mut self,
            _answer: crate::command::Confirm,
        ) -> Vec<Effect> {
            self.dictation.question = false;
            vec![Effect::Redraw]
        }

        pub(crate) fn dictation_component_changed(&mut self, _id: &str) {}

        pub(crate) fn dictate_after_download(&mut self) -> Vec<Effect> {
            Vec::new()
        }

        /// A spoken answer (crate::reveal) needs dictation: said so.
        pub(crate) fn answer_aloud_start(&mut self) -> Vec<Effect> {
            let msg = self.msg("reveal-no-dictation");
            self.tell(&msg);
            vec![Effect::Redraw]
        }

        pub(crate) fn answer_aloud_stop(&mut self) -> Vec<Effect> {
            vec![Effect::Redraw]
        }

        pub(crate) fn answer_aloud_active(&self) -> bool {
            false
        }
    }
}

#[cfg(not(feature = "dictation"))]
pub(crate) use off::DictationSlot;

#[cfg(feature = "dictation")]
pub(crate) use on::DictationSlot;

#[cfg(feature = "dictation")]
mod on {
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use textweaver_a11y::{Announcer, Importance, Priority};
    use textweaver_core::CharRange;
    use textweaver_dictation::stream::StreamConfig;
    use textweaver_dictation::{
        Dictation, DictationEvent, DictationInput, DictationState, MicCapture, RtenConfig,
        RtenDictation, apply_spoken_commands,
    };
    use textweaver_keymap::ActionId;
    use textweaver_lexicon::args;

    use super::*;

    /// Makes a dictation backend from a model folder.
    pub(crate) type BackendFactory = Box<dyn Fn(&std::path::Path) -> Box<dyn Dictation> + Send>;

    /// The dictation backend and the session under way.
    #[derive(Default)]
    pub(crate) struct DictationSlot {
        /// The backend, kept once made so the model stays loaded.
        pub(crate) backend: Option<Box<dyn Dictation>>,
        /// Makes the backend from a model folder in place of the in-process
        /// Whisper one; tests set it so no real model or microphone is
        /// opened, even after a download drops the backend.
        pub(crate) factory: Option<BackendFactory>,
        /// A yes-or-no question is open: turn on edit mode and dictate?
        pub(crate) question: bool,
        /// The words of the phrase being spoken, committed so far.
        words: String,
        /// The phrase those words belong to.
        utterance: Option<usize>,
        /// Words not yet said (held while recording).
        held: String,
        /// A spoken answer for a prompt list (crate::reveal) is being
        /// recorded: its words so far. Nothing is typed.
        answer: Option<String>,
    }

    impl std::fmt::Debug for DictationSlot {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("DictationSlot")
                .field(
                    "backend",
                    &self.backend.as_ref().map(|b| b.name().to_owned()),
                )
                .field("question", &self.question)
                .field("words", &self.words)
                .finish_non_exhaustive()
        }
    }

    impl DictationSlot {
        fn state(&self) -> DictationState {
            self.backend
                .as_ref()
                .map_or(DictationState::Idle, |b| b.state())
        }

        /// True while a session records or finishes.
        pub(crate) fn active(&self) -> bool {
            self.state() != DictationState::Idle
        }
    }

    impl App {
        /// Installs the dictation backend (a test's fake, or a recognizer
        /// a frontend brings), replacing the in-process Whisper one.
        pub fn set_dictation_backend(&mut self, backend: Box<dyn Dictation>) {
            if let Some(mut old) = self.dictation.backend.take() {
                old.shutdown(FINISH_WAIT);
            }
            self.dictation.backend = Some(backend);
        }

        /// The Dictate command: starts or stops dictation.
        pub(crate) fn dictate(&mut self) -> Vec<Effect> {
            match self.dictation.state() {
                DictationState::Recording => {
                    if let Some(b) = self.dictation.backend.as_mut()
                        && let Err(e) = b.stop()
                    {
                        let msg =
                            self.msg_args("dictation-failed", &args!["error" => e.to_string()]);
                        self.error(&msg);
                    }
                    self.take_dictation_events();
                    return vec![Effect::Redraw];
                }
                DictationState::Transcribing => {
                    let msg = self.msg("dictation-busy");
                    self.tell(&msg);
                    return vec![Effect::Redraw];
                }
                DictationState::Idle => {}
            }
            if !self.is_editing() {
                self.dictation.question = true;
                let msg = self.msg("dictation-needs-edit");
                self.ask(&msg);
                return vec![Effect::Redraw];
            }
            self.start_dictation()
        }

        /// The answer to "turn edit mode on and dictate?".
        pub(crate) fn confirm_dictation(&mut self, answer: crate::command::Confirm) -> Vec<Effect> {
            use crate::command::Confirm;
            match answer {
                Confirm::Yes => {
                    self.dictation.question = false;
                    let mut effects = self.toggle_edit();
                    if self.is_editing() {
                        effects.extend(self.start_dictation());
                    }
                    effects
                }
                Confirm::No => {
                    self.dictation.question = false;
                    let msg = self.msg("common-cancelled");
                    self.tell(&msg);
                    vec![Effect::Redraw]
                }
                Confirm::Repeat => {
                    let msg = self.msg("dictation-needs-edit");
                    self.ask(&msg);
                    vec![Effect::Redraw]
                }
            }
        }

        /// The in-process model's folder, when it is ready: the setting's
        /// folder when it exists, else the chosen model's component folder
        /// when its files are there. Otherwise says why in words, and asks
        /// to download a missing or damaged model (once a session).
        fn dictation_model_ready(&mut self) -> Result<PathBuf, Vec<Effect>> {
            use crate::components::{After, Status};
            if let Some(dir) = self.settings.dictation.model_dir.clone() {
                if dir.is_dir() {
                    return Ok(dir);
                }
                let msg = self.msg_args(
                    "dictation-model-no-folder",
                    &args!["dir" => dir.display().to_string()],
                );
                self.error(&msg);
                return Err(vec![Effect::Redraw]);
            }
            let chosen = crate::components::dictation_model_id(&self.settings).to_owned();
            let found = self
                .component_and_dir(&chosen)
                .or_else(|| self.component_and_dir(crate::components::WHISPER_BASE_EN));
            let Some((c, dir)) = found else {
                let msg = self.msg_args(
                    "dictation-no-model",
                    &args!["dir" => "whisper/rten/base.en"],
                );
                self.error(&msg);
                return Err(vec![Effect::Redraw]);
            };
            match c.status_in(&dir) {
                Status::Installed => return Ok(dir),
                Status::Damaged(file) => {
                    let msg = self.msg_args("dictation-model-damaged", &args!["file" => file]);
                    self.error(&msg);
                }
                Status::NotInstalled | Status::Partial(_) => {}
            }
            if self.component_declined(&c.id) {
                let msg = self.msg("dictation-model-declined");
                self.tell(&msg);
                return Err(vec![Effect::Redraw]);
            }
            let effects = self.ask_component_download(&c.id, After::Dictate);
            if !self.confirmation_pending() {
                // Not asked (no downloads in this build): say where the
                // model goes, so it can be placed by hand.
                let msg = self.msg_args(
                    "dictation-no-model",
                    &args!["dir" => dir.display().to_string()],
                );
                self.error(&msg);
            }
            Err(effects)
        }

        /// A component changed: when it is a dictation model, the backend
        /// is let go (not while recording), so the next Dictate loads the
        /// model again or asks for it.
        pub(crate) fn dictation_component_changed(&mut self, id: &str) {
            let model = crate::components::DICTATION_MODELS.contains(&id)
                || id == crate::components::dictation_model_id(&self.settings);
            if model
                && !self.dictation.active()
                && let Some(mut old) = self.dictation.backend.take()
            {
                old.shutdown(FINISH_WAIT);
            }
        }

        /// The model finished downloading after the dictation question:
        /// dictation starts, in edit mode.
        pub(crate) fn dictate_after_download(&mut self) -> Vec<Effect> {
            if self.is_editing() && !self.dictation.active() {
                self.start_dictation()
            } else {
                Vec::new()
            }
        }

        fn start_dictation(&mut self) -> Vec<Effect> {
            if self.dictation.backend.is_none() {
                let dir = match self.dictation_model_ready() {
                    Ok(dir) => dir,
                    Err(effects) => return effects,
                };
                let mut config = RtenConfig::new(&dir);
                config.live = Some(StreamConfig::default());
                let made = match &self.dictation.factory {
                    Some(make) => Ok(make(&dir)),
                    None => RtenDictation::new(config).map(|b| Box::new(b) as Box<dyn Dictation>),
                };
                match made {
                    Ok(b) => self.dictation.backend = Some(b),
                    Err(e) => {
                        // The real reason, in words.
                        log::warn!("dictation model in {}: {e}", dir.display());
                        let msg = match e {
                            textweaver_dictation::DictationError::ModelNotFound {
                                file, ..
                            } => self
                                .msg_args("dictation-model-file-missing", &args!["file" => file]),
                            other => self
                                .msg_args("dictation-failed", &args!["error" => other.to_string()]),
                        };
                        self.error(&msg);
                        return vec![Effect::Redraw];
                    }
                }
            }
            // The reading voice stops: the microphone must not hear it.
            self.stop_speech();
            self.dictation.words.clear();
            self.dictation.held.clear();
            self.dictation.utterance = None;
            let started = self
                .dictation
                .backend
                .as_mut()
                .map(|b| b.start(DictationInput::Capture(Box::new(MicCapture::new()))));
            if let Some(Err(e)) = started {
                let msg = self.msg_args("dictation-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
                return vec![Effect::Redraw];
            }
            self.take_dictation_events();
            vec![Effect::Redraw]
        }

        /// Starts recording a spoken answer for a prompt list
        /// (crate::reveal), in any mode: the words are read back when it
        /// ends, never typed.
        pub(crate) fn answer_aloud_start(&mut self) -> Vec<Effect> {
            if self.dictation.active() {
                let msg = self.msg("dictation-busy");
                self.tell(&msg);
                return vec![Effect::Redraw];
            }
            self.dictation.answer = Some(String::new());
            let effects = self.start_dictation();
            if !self.dictation.active() {
                self.dictation.answer = None;
            }
            effects
        }

        /// Stops recording a spoken answer; the words are read back when
        /// the last phrase is transcribed.
        pub(crate) fn answer_aloud_stop(&mut self) -> Vec<Effect> {
            if self.dictation.state() != DictationState::Recording {
                let msg = self.msg("dictation-busy");
                self.tell(&msg);
                return vec![Effect::Redraw];
            }
            if let Some(b) = self.dictation.backend.as_mut()
                && let Err(e) = b.stop()
            {
                let msg = self.msg_args("dictation-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
            }
            self.take_dictation_events();
            vec![Effect::Redraw]
        }

        /// True while a spoken answer is recorded or transcribed.
        pub(crate) fn answer_aloud_active(&self) -> bool {
            self.dictation.answer.is_some()
        }

        /// A spoken answer ended without words reaching the reader (the
        /// recognizer failed or was cancelled): the prompt list stops
        /// waiting for it.
        fn answer_aloud_dropped(&mut self) {
            if self.dictation.answer.take().is_some()
                && let Some(crate::app::ListKind::Reveal(l)) = self.list.as_mut()
            {
                l.listening = None;
            }
        }

        /// Applies what the dictation backend reported since the last
        /// tick. Called from [`App::tick`].
        pub(crate) fn dictation_tick(&mut self) -> Vec<Effect> {
            if self.take_dictation_events() {
                vec![Effect::Redraw]
            } else {
                Vec::new()
            }
        }

        /// Takes and applies the backend's events; true when there were
        /// any.
        fn take_dictation_events(&mut self) -> bool {
            let events = match self.dictation.backend.as_mut() {
                Some(b) => b.poll(),
                None => return false,
            };
            let any = !events.is_empty();
            for e in events {
                self.on_dictation_event(e);
            }
            any
        }

        fn on_dictation_event(&mut self, event: DictationEvent) {
            match event {
                DictationEvent::Recording if self.dictation.answer.is_some() => {
                    let msg = self.msg("reveal-listening");
                    self.announce_as(&msg, Priority::Polite, Importance::Result);
                }
                DictationEvent::Recording => {
                    let key = self.key(ActionId::Dictate);
                    let msg = self.msg_args("dictation-listening", &args!["key" => key]);
                    self.announce_as(&msg, Priority::Polite, Importance::Result);
                }
                DictationEvent::Transcribing => {
                    let msg = self.msg("dictation-finishing");
                    self.announce_as(&msg, Priority::Polite, Importance::Progress);
                }
                DictationEvent::Committed {
                    text, utterance, ..
                } => {
                    if text.trim().is_empty() {
                        return;
                    }
                    if self.dictation.utterance != Some(utterance) {
                        self.dictation.utterance = Some(utterance);
                        self.dictation.words.clear();
                    }
                    push_words(&mut self.dictation.words, &text);
                    // A spoken answer is read back whole at the end.
                    let answering = self.dictation.answer.is_some();
                    if self.settings.dictation.speak_while_recording && !answering {
                        self.announce_as(&text, Priority::Polite, Importance::Result);
                    } else if !answering {
                        push_words(&mut self.dictation.held, &text);
                    }
                    self.show_dictation_line();
                }
                DictationEvent::Partial(segment) if self.dictation.answer.is_some() => {
                    if let Some(answer) = self.dictation.answer.as_mut() {
                        push_words(answer, &segment.text);
                    }
                    self.show_dictation_line();
                }
                DictationEvent::Final(_) if self.dictation.answer.is_some() => {
                    let words = self.dictation.answer.take().unwrap_or_default();
                    self.dictation.words.clear();
                    self.dictation.utterance = None;
                    self.reveal_heard(&words);
                }
                DictationEvent::Partial(segment) => {
                    // The phrase is finished at its pause: typed, and said
                    // if it was held.
                    self.type_dictated(&segment.text);
                    let held = std::mem::take(&mut self.dictation.held);
                    if !held.is_empty() {
                        self.announce_as(&held, Priority::Polite, Importance::Result);
                    }
                    self.show_dictation_line();
                }
                DictationEvent::NoWords { .. } => {
                    let msg = self.msg("dictation-no-words");
                    self.announce_as(&msg, Priority::Polite, Importance::Result);
                }
                DictationEvent::Final(_) => {
                    // Every phrase was typed at its pause.
                    self.dictation.words.clear();
                    self.dictation.utterance = None;
                    let msg = self.msg("dictation-done");
                    self.announce_as(&msg, Priority::Polite, Importance::Result);
                }
                DictationEvent::Failed { message } => {
                    self.answer_aloud_dropped();
                    self.dictation.words.clear();
                    let msg = self.msg_args("dictation-failed", &args!["error" => message]);
                    self.error(&msg);
                }
                DictationEvent::Cancelled => self.answer_aloud_dropped(),
            }
        }

        /// The status line while dictating: the label, then as many of the
        /// phrase's newest words as fit in 40 cells. Shown, not said:
        /// the words are said by [`on_dictation_event`](Self::on_dictation_event)
        /// when the setting allows, and the Braille display follows the
        /// status line.
        fn show_dictation_line(&mut self) {
            let label = self.msg_args("dictation-status", &args!["words" => ""]);
            let room = STATUS_CELLS.saturating_sub(label.trim_end().chars().count() + 1);
            let words = tail_words(&self.dictation.words, room.max(8));
            let line = self.msg_args("dictation-status", &args!["words" => words]);
            let shown = self.screen_text(&line);
            self.status.announce(&shown, Priority::Polite);
        }

        /// Types a finished phrase at the caret, with spoken commands
        /// applied, as one undo step (the editor's path for a change a text
        /// control made, which says nothing itself).
        fn type_dictated(&mut self, raw: &str) {
            let text = apply_spoken_commands(raw.trim());
            if text.is_empty() {
                return;
            }
            let Some((range, before)) = self.edit.as_ref().and_then(|e| {
                let ed = e.session.editor()?;
                let sel = ed.selection();
                let (a, b) = if sel.anchor <= sel.head {
                    (sel.anchor.0, sel.head.0)
                } else {
                    (sel.head.0, sel.anchor.0)
                };
                let before = a.checked_sub(1).and_then(|i| ed.text().get_char(i));
                Some((CharRange::new(a, b), before))
            }) else {
                // Edit mode ended without finishing first: say so, never
                // drop the words silently.
                let msg = self.msg_args("dictation-not-typed", &args!["text" => text.as_str()]);
                self.error(&msg);
                return;
            };
            let text = joined(before, &text);
            self.replace_range(range, &text);
        }

        /// Finishes a dictation under way before edit mode ends (leaving
        /// it, opening another document, quitting): stops recording and
        /// types the rest, waiting at most [`FINISH_WAIT`]. When that runs
        /// out, the session ends and that is said.
        pub(crate) fn dictation_finish(&mut self) {
            self.dictation_finish_within(FINISH_WAIT);
        }

        /// [`dictation_finish`](Self::dictation_finish), waiting at most
        /// `wait`.
        pub(crate) fn dictation_finish_within(&mut self, wait: Duration) {
            if !self.dictation.active() {
                return;
            }
            if self.dictation.state() == DictationState::Recording
                && let Some(b) = self.dictation.backend.as_mut()
                && let Err(e) = b.stop()
            {
                log::warn!("dictation: stopping failed: {e}");
            }
            let deadline = Instant::now() + wait;
            loop {
                self.take_dictation_events();
                if !self.dictation.active() {
                    return;
                }
                if Instant::now() >= deadline {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            if let Some(b) = self.dictation.backend.as_mut() {
                b.shutdown(Duration::ZERO);
            }
            self.take_dictation_events();
            let msg = self.msg("dictation-lost");
            self.error(&msg);
        }

        /// Quitting: finishes any dictation, then stops the backend
        /// (waiting at most [`FINISH_WAIT`] for its worker).
        pub(crate) fn dictation_shutdown(&mut self) {
            self.dictation_finish();
            if let Some(mut b) = self.dictation.backend.take()
                && !b.shutdown(FINISH_WAIT)
            {
                log::warn!("dictation: the recognizer did not stop in time");
            }
        }
    }

    /// Appends `text` to `words` with one space between.
    fn push_words(words: &mut String, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        if !words.is_empty() {
            words.push(' ');
        }
        words.push_str(text);
    }

    #[cfg(test)]
    mod tests {
        use std::collections::VecDeque;
        use std::sync::{Arc, Mutex};

        use textweaver_dictation::{DictationError, Segment, Transcript};

        use super::*;
        use crate::command::{CaretMove, Command, Confirm};
        use crate::{AppConfig, Mode};

        /// What the app announced (to the screen reader and the voice).
        #[derive(Clone, Default)]
        struct Said(Arc<Mutex<Vec<String>>>);

        impl Said {
            fn all(&self) -> Vec<String> {
                self.0.lock().unwrap().clone()
            }
        }

        impl Announcer for Said {
            fn announce(&mut self, text: &str, _priority: Priority) {
                self.0.lock().unwrap().push(text.to_owned());
            }
        }

        /// A recognizer that says what the test tells it to.
        #[derive(Default)]
        struct Script {
            events: VecDeque<DictationEvent>,
            /// Said when recording stops.
            at_stop: Vec<DictationEvent>,
            state: Option<DictationState>,
            starts: usize,
            shutdowns: usize,
        }

        #[derive(Clone, Default)]
        struct Fake(Arc<Mutex<Script>>);

        impl Fake {
            fn push(&self, e: DictationEvent) {
                self.0.lock().unwrap().events.push_back(e);
            }
            fn at_stop(&self, e: Vec<DictationEvent>) {
                self.0.lock().unwrap().at_stop = e;
            }
        }

        impl Dictation for Fake {
            fn name(&self) -> &str {
                "fake"
            }
            fn start(&mut self, _input: DictationInput) -> Result<(), DictationError> {
                let mut s = self.0.lock().unwrap();
                s.starts += 1;
                s.state = Some(DictationState::Recording);
                s.events.push_back(DictationEvent::Recording);
                Ok(())
            }
            fn stop(&mut self) -> Result<(), DictationError> {
                let mut s = self.0.lock().unwrap();
                s.state = Some(DictationState::Transcribing);
                s.events.push_back(DictationEvent::Transcribing);
                let rest = std::mem::take(&mut s.at_stop);
                s.events.extend(rest);
                Ok(())
            }
            fn cancel(&mut self) {
                let mut s = self.0.lock().unwrap();
                s.state = None;
                s.events.push_back(DictationEvent::Cancelled);
            }
            fn poll(&mut self) -> Vec<DictationEvent> {
                let mut s = self.0.lock().unwrap();
                let out: Vec<DictationEvent> = s.events.drain(..).collect();
                if out.iter().any(DictationEvent::is_terminal) {
                    s.state = None;
                }
                out
            }
            fn state(&self) -> DictationState {
                self.0.lock().unwrap().state.unwrap_or(DictationState::Idle)
            }
            fn shutdown(&mut self, _wait: Duration) -> bool {
                self.cancel();
                self.0.lock().unwrap().shutdowns += 1;
                true
            }
        }

        fn committed(text: &str, utterance: usize) -> DictationEvent {
            DictationEvent::Committed {
                text: text.into(),
                utterance,
                at_pause: false,
            }
        }

        fn phrase(text: &str) -> DictationEvent {
            DictationEvent::Partial(Segment {
                start_ms: 0,
                end_ms: 1000,
                text: text.into(),
            })
        }

        fn final_event() -> DictationEvent {
            DictationEvent::Final(Transcript::default())
        }

        /// An app with a Markdown document open (not editing), and the
        /// fake recognizer.
        fn reading(text: &str) -> (App, Fake, Said) {
            let said = Said::default();
            let mut app = App::new(AppConfig {
                announcer: Box::new(said.clone()),
                ..AppConfig::for_tests()
            });
            let mut doc = textweaver_text::Document::from_plain_text(text);
            doc.meta.format = "markdown".into();
            app.open_document(doc, crate::store::DocKey::untitled(7), "notes".into());
            let fake = Fake::default();
            app.set_dictation_backend(Box::new(fake.clone()));
            (app, fake, said)
        }

        /// The same, editing, with the caret at the end of the text.
        fn editing(text: &str) -> (App, Fake, Said) {
            let (mut app, fake, said) = reading(text);
            app.enter_edit(Some(text.to_owned()));
            assert!(app.is_editing());
            app.dispatch(Command::MoveCaret {
                by: CaretMove::DocumentEdge,
                direction: textweaver_core::Direction::Forward,
                extend: false,
            });
            (app, fake, said)
        }

        fn source(app: &App) -> String {
            app.edit
                .as_ref()
                .and_then(|e| e.session.editor())
                .map(|ed| ed.text().to_string())
                .unwrap_or_default()
        }

        /// The text in the editor, or the document's once edit mode ended.
        fn text_now(app: &App) -> String {
            if app.is_editing() {
                source(app)
            } else {
                app.session().unwrap().doc.text().to_string()
            }
        }

        fn tick(app: &mut App) {
            app.tick(Instant::now());
        }

        /// The self-test (crate::reveal): Space records a spoken answer in
        /// any mode, it is read back and never typed, and Enter then
        /// reveals the passage.
        #[test]
        fn a_spoken_answer_is_read_back_before_the_reveal() {
            use crate::list_model::ListKey;
            let (mut app, fake, said) = reading("Kidneys filter the blood.\n");
            app.add_note("What filters the blood?");
            app.dispatch(Command::Action(ActionId::SelfTest));
            assert!(app.list_model().is_some());
            app.dispatch(Command::ListKey(ListKey::Char(' ')));
            assert_eq!(fake.0.lock().unwrap().starts, 1);
            assert!(!app.is_editing());
            assert!(
                said.all()
                    .iter()
                    .any(|t| t.starts_with("Answer aloud now."))
            );
            assert!(app.list_model().is_some(), "the list stays");
            fake.push(committed("the kidneys", 0));
            fake.push(phrase("The kidneys."));
            tick(&mut app);
            fake.at_stop(vec![final_event()]);
            app.dispatch(Command::ListKey(ListKey::Char(' ')));
            assert!(
                said.all()
                    .iter()
                    .any(|t| t == "You said: The kidneys. Enter shows the answer."),
                "{:?}",
                said.all()
            );
            assert_eq!(text_now(&app), "Kidneys filter the blood.\n");
            app.dispatch(Command::ListKey(ListKey::Enter));
            assert_eq!(
                said.all().last().map(String::as_str),
                Some("Answer: Kidneys filter the blood.")
            );
        }

        #[test]
        fn the_command_is_offered() {
            let app = App::new(AppConfig::for_tests());
            assert!(app.is_available(ActionId::Dictate));
        }

        #[test]
        fn a_phrase_is_typed_at_the_caret_with_its_commands() {
            let (mut app, fake, _said) = editing("Notes");
            app.dispatch(Command::Action(ActionId::Dictate));
            assert_eq!(fake.0.lock().unwrap().starts, 1);
            assert!(
                app.status_text().starts_with("Dictating"),
                "{}",
                app.status_text()
            );
            fake.push(committed("for Monday", 0));
            tick(&mut app);
            assert_eq!(app.status_text(), "Dictating: for Monday");
            // Not typed until the pause.
            assert_eq!(source(&app), "Notes");
            fake.push(phrase("for Monday period new line"));
            tick(&mut app);
            assert_eq!(source(&app), "Notes for Monday.\n");
            // One undo step takes the whole phrase away.
            app.dispatch(Command::Action(ActionId::Undo));
            assert_eq!(source(&app), "Notes");
        }

        #[test]
        fn the_status_line_keeps_the_newest_words_in_40_cells() {
            let (mut app, fake, _said) = editing("x");
            app.dispatch(Command::Action(ActionId::Dictate));
            fake.push(committed(
                "When the library opened its new reading room on the third floor",
                0,
            ));
            tick(&mut app);
            let line = app.status_text().to_owned();
            assert!(line.starts_with("Dictating: "), "{line}");
            assert!(line.chars().count() <= 40, "{line}");
            assert!(line.ends_with("third floor"), "{line}");
            // A new phrase starts the line again.
            fake.push(committed("Stop reading.", 1));
            tick(&mut app);
            assert_eq!(app.status_text(), "Dictating: Stop reading.");
        }

        #[test]
        fn words_are_held_until_the_pause_by_default() {
            let (mut app, fake, said) = editing("x");
            app.dispatch(Command::Action(ActionId::Dictate));
            fake.push(committed("notes for", 0));
            tick(&mut app);
            assert!(
                !said.all().iter().any(|t| t.contains("notes for")),
                "said while recording: {:?}",
                said.all()
            );
            // Still on the status line, for the Braille display.
            assert_eq!(app.status_text(), "Dictating: notes for");
            fake.push(committed("Monday", 0));
            fake.push(phrase("notes for Monday"));
            tick(&mut app);
            let all = said.all();
            assert!(
                all.iter().any(|t| t == "notes for Monday"),
                "held words said once at the pause: {all:?}"
            );
        }

        #[test]
        fn words_can_be_said_as_they_come() {
            let (mut app, fake, said) = editing("x");
            app.settings.dictation.speak_while_recording = true;
            app.dispatch(Command::Action(ActionId::Dictate));
            fake.push(committed("notes for", 0));
            tick(&mut app);
            assert!(
                said.all().iter().any(|t| t == "notes for"),
                "{:?}",
                said.all()
            );
            assert_eq!(app.status_text(), "Dictating: notes for");
        }

        #[test]
        fn outside_edit_mode_it_asks_first() {
            let (mut app, fake, _said) = reading("Hello");
            assert!(!app.is_editing());
            app.dispatch(Command::Action(ActionId::Dictate));
            assert!(app.confirmation_pending());
            assert_eq!(fake.0.lock().unwrap().starts, 0);
            app.dispatch(Command::Confirm(Confirm::No));
            assert!(!app.confirmation_pending());
            assert_eq!(fake.0.lock().unwrap().starts, 0);
            app.dispatch(Command::Action(ActionId::Dictate));
            app.dispatch(Command::Confirm(Confirm::Yes));
            assert!(app.is_editing());
            assert_eq!(fake.0.lock().unwrap().starts, 1);
            assert_ne!(app.mode(), Mode::Prompt);
        }

        #[test]
        fn pressing_it_again_stops_and_types_the_rest() {
            let (mut app, fake, _said) = editing("A");
            app.dispatch(Command::Action(ActionId::Dictate));
            fake.at_stop(vec![phrase("last words"), final_event()]);
            app.dispatch(Command::Action(ActionId::Dictate));
            tick(&mut app);
            assert_eq!(source(&app), "A last words");
            assert!(!app.dictation.active());
            assert_eq!(app.status_text(), "Dictation done.");
        }

        #[test]
        fn leaving_edit_mode_finishes_the_dictation_first() {
            let (mut app, fake, _said) = editing("A");
            app.dispatch(Command::Action(ActionId::Dictate));
            fake.push(committed("never", 0));
            tick(&mut app);
            fake.at_stop(vec![phrase("never lost"), final_event()]);
            app.dispatch(Command::Action(ActionId::ToggleEditMode));
            // The phrase was typed before edit mode ended.
            let text = text_now(&app);
            assert!(text.contains("A never lost"), "{text:?}");
            assert!(!app.dictation.active());
        }

        #[test]
        fn quitting_finishes_and_stops_the_recognizer() {
            let (mut app, fake, _said) = editing("A");
            app.dispatch(Command::Action(ActionId::Dictate));
            fake.at_stop(vec![phrase("goodbye"), final_event()]);
            app.shutdown();
            let s = fake.0.lock().unwrap();
            assert_eq!(s.shutdowns, 1);
            assert!(app.dictation.backend.is_none());
        }

        #[test]
        fn a_recognizer_that_never_finishes_is_said_to_have_stopped() {
            let (mut app, fake, _said) = editing("A");
            app.dispatch(Command::Action(ActionId::Dictate));
            // Stopping leaves it transcribing for good; a short wait keeps
            // the test fast.
            fake.0.lock().unwrap().at_stop.clear();
            app.dictation_finish_within(Duration::from_millis(50));
            assert!(!app.dictation.active());
            assert_eq!(
                app.status_text(),
                "Error: Dictation stopped before its last words were typed."
            );
        }

        #[test]
        fn a_phrase_with_no_words_is_said() {
            let (mut app, fake, _said) = editing("A");
            app.dispatch(Command::Action(ActionId::Dictate));
            fake.push(DictationEvent::NoWords { utterance: 0 });
            tick(&mut app);
            assert_eq!(app.status_text(), "No words recognized in that phrase.");
        }

        #[test]
        fn a_failure_is_never_silenced() {
            let (mut app, fake, _said) = editing("A");
            app.settings.accessibility.interface_announcements =
                textweaver_store::InterfaceAnnouncements::Off;
            app.dispatch(Command::Action(ActionId::Dictate));
            fake.push(DictationEvent::Failed {
                message: "No audio was recorded. Check your microphone.".into(),
            });
            tick(&mut app);
            assert!(
                app.status_text().contains("No audio was recorded"),
                "{}",
                app.status_text()
            );
        }

        #[test]
        fn words_fit_the_display() {
            assert_eq!(tail_words("one two three", 40), "one two three");
            assert_eq!(tail_words("one two three", 9), "two three");
            assert_eq!(tail_words("supercalifragilistic", 5), "istic");
            assert_eq!(joined(Some('s'), "word"), " word");
            assert_eq!(joined(Some('s'), ". Next"), ". Next");
            assert_eq!(joined(Some(' '), "word"), "word");
            assert_eq!(joined(None, "word"), "word");
            assert_eq!(joined(Some('a'), "\nnext"), "\nnext");
        }
    }
}
