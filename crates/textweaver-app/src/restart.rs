//! Restarting speech in place (Phase 2).
//!
//! When the speech thread dies (a panic in a backend; Agent P1a made that
//! detectable), the app switches to silence at once. If the frontend gave
//! it a way to start speech ([`App::set_speech_starter`]: the terminal
//! reader passes its own start-up with the command line's choices), it
//! then restarts speech automatically, once, with the current settings,
//! and says "Speech restarted."; after that, and at any time, the Restart
//! Speech command (Shift+F8) does it again.
//!
//! The new service starts on a helper thread (an engine can take seconds
//! to start), and the old one is shut down on another, so neither holds
//! up the keyboard; [`App::tick`] swaps the new service in when it is
//! ready.
//!
//! At startup (Wave 4, W4h) the messages said before the first engine is
//! ready ("Opened essay.", a settings warning, the welcome) are kept and
//! said once it is, in order and without cutting each other off, followed
//! by any message from starting the engine. Before, the silent service
//! swallowed them and only the status line's latest text was said.

use std::sync::Arc;

use std::sync::mpsc::{self, Receiver, TryRecvError};
use textweaver_speech::SayMode;

use textweaver_speech::SpeechService;
use textweaver_store::Settings;

use textweaver_lexicon::args;

use crate::app::App;
use crate::command::Effect;
use crate::playback::{Playback, SpeechTrack};

/// Starts a speech service for the given settings: the service, the
/// backend's name, and messages for the user (an engine that was not
/// available, for example). Runs on a helper thread.
pub type SpeechStarter =
    Arc<dyn Fn(&Settings) -> (SpeechService, String, Vec<String>) + Send + Sync + 'static>;

/// What a restart produced.
type Started = (SpeechService, String, Vec<String>);

/// The restart machinery's state.
#[derive(Default)]
pub(crate) struct Restart {
    starter: Option<SpeechStarter>,
    /// A restart in progress: the new service arrives here.
    pending: Option<Receiver<Started>>,
    /// The automatic restart after the speech thread died was used.
    auto_used: bool,
    /// Speech was self-voicing before it stopped working.
    voiced: bool,
    /// The service being started is the first one
    /// (`App::start_speech_in_background`).
    first: bool,
    /// Messages for the voice are kept in `early` until the first engine
    /// is ready.
    hold: bool,
    /// Messages said before the first engine was ready, oldest first, each
    /// with the dialog generation it belongs to (crate::announce).
    early: Vec<(String, Option<u64>)>,
}

/// Most messages kept while the first engine starts; later ones are only
/// shown (the status line has them all).
pub(crate) const EARLY_MESSAGES: usize = 16;

impl std::fmt::Debug for Restart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Restart")
            .field("starter", &self.starter.is_some())
            .field("pending", &self.pending.is_some())
            .field("auto_used", &self.auto_used)
            .finish()
    }
}

impl App {
    /// Tells the app how to start speech again with given settings, so it
    /// can restart speech in place: automatically once when the speech
    /// thread dies, and on the Restart Speech command. Without one, a dead
    /// speech thread leaves textweaver silent.
    pub fn set_speech_starter(&mut self, starter: SpeechStarter) {
        self.restart.starter = Some(starter);
    }

    /// True when the frontend gave a way to start speech: real engines are
    /// in play (the voice manager then lists the other engines' voices).
    pub(crate) fn can_start_speech(&self) -> bool {
        self.restart.starter.is_some()
    }

    /// True while speech is being restarted (or started for the first time
    /// in the background).
    pub fn speech_restarting(&self) -> bool {
        self.restart.pending.is_some()
    }

    /// Starts the speech engine on a helper thread instead of waiting for
    /// it (Wave 3): the app starts with a silent service (pass
    /// `SpeechService::null()` in `AppConfig`), and [`App::tick`] swaps
    /// the engine in when it is ready, ringing the waker. Messages said
    /// meanwhile are shown at once and spoken once the engine is ready, in
    /// order (at most 16); a reading started meanwhile goes on from where
    /// it is instead. The starter is also kept for Restart Speech, as with
    /// [`set_speech_starter`](Self::set_speech_starter).
    ///
    /// [`App::tick`]: crate::App::tick
    pub fn start_speech_in_background(&mut self, starter: SpeechStarter) {
        self.restart.starter = Some(starter);
        self.restart.first = true;
        self.restart.hold = true;
        self.restart.voiced = self.self_voicing;
        self.begin_restart();
    }

    /// Waits until speech started in the background (or restarted) is
    /// ready and swapped in, at most `timeout`; for tests and startup
    /// code that must speak first. True when it is ready.
    pub fn wait_for_speech_start(&mut self, timeout: std::time::Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        while self.restart.pending.is_some() {
            if std::time::Instant::now() >= deadline {
                return false;
            }
            let _ = self.restart_tick();
            if self.restart.pending.is_some() {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        true
    }

    /// After the speech thread died (from `speech_thread_died`): restarts
    /// speech once, automatically. Returns the sentence to add to the
    /// message saying speech stopped.
    pub(crate) fn restart_after_death(&mut self, voiced: bool) -> String {
        self.restart.voiced |= voiced;
        let keys = self.keys(textweaver_keymap::ActionId::RestartSpeech);
        if self.restart.starter.is_none() {
            return if voiced {
                self.msg("restart-silent-now")
            } else {
                String::new()
            };
        }
        if std::mem::replace(&mut self.restart.auto_used, true) {
            return self.msg_args("restart-silent-use-key", &args!["keys" => keys]);
        }
        self.begin_restart();
        self.msg("restart-restarting")
    }

    /// The Restart Speech command.
    pub(crate) fn restart_speech_command(&mut self) -> Vec<Effect> {
        if self.restart.starter.is_none() {
            let msg = self.msg("restart-not-here");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        if self.restart.pending.is_some() {
            let msg = self.msg("restart-already");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        self.stop_speech();
        self.restart.voiced |= self.self_voicing;
        let msg = self.msg("restart-restarting");
        self.show(&msg);
        self.begin_restart();
        vec![Effect::Redraw]
    }

    /// Says a message (keys already in their spoken form) with
    /// textweaver's voice; while the first engine is starting, keeps it to
    /// say once the engine is ready (at most [`EARLY_MESSAGES`]).
    pub(crate) fn voice_message(&mut self, spoken: String, mode: SayMode) {
        if self.restart.hold {
            if self.restart.early.len() < EARLY_MESSAGES {
                let tag = self.dialog_tag();
                self.restart.early.push((spoken, tag));
            }
            return;
        }
        self.speech.say(spoken, mode);
    }

    /// Says the messages held while the first engine started, in order,
    /// each after the one before ("Opened report." first), so none cuts
    /// another off. A message about a list or prompt that has closed
    /// since is dropped (crate::announce).
    fn say_early_messages(&mut self, early: Vec<(String, Option<u64>)>) {
        for (m, tag) in early {
            if tag.is_some_and(|g| g != self.dialog_generation) {
                continue;
            }
            self.speech.say(m, SayMode::Queue);
        }
    }

    /// Starts the new service on a helper thread.
    fn begin_restart(&mut self) {
        let Some(starter) = self.restart.starter.clone() else {
            return;
        };
        let settings = self.settings.clone();
        let (tx, rx) = mpsc::channel();
        let wake = self.waker_slot();
        let spawned = std::thread::Builder::new()
            .name("textweaver-speech-restart".into())
            .spawn(move || {
                let _ = tx.send(starter(&settings));
                wake.wake();
            });
        match spawned {
            Ok(_) => self.restart.pending = Some(rx),
            Err(e) => {
                let msg = self.msg_args("restart-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
            }
        }
    }

    /// Swaps the new service in once it has started (from
    /// [`App::tick`](crate::App::tick)).
    pub(crate) fn restart_tick(&mut self) -> Vec<Effect> {
        let Some(rx) = &self.restart.pending else {
            return Vec::new();
        };
        let (service, name, messages) = match rx.try_recv() {
            Ok(started) => started,
            Err(TryRecvError::Empty) => return Vec::new(),
            Err(TryRecvError::Disconnected) => {
                self.restart.pending = None;
                let msg = self.msg("restart-start-failed");
                self.error(&msg);
                return vec![Effect::Redraw];
            }
        };
        self.restart.pending = None;
        let first = std::mem::take(&mut self.restart.first);
        // The waker follows the new service.
        service.set_waker(self.wake.get());
        // What was being read while the engine started (silently), to read
        // on with the new one.
        let reading_from = (first && self.playback == Playback::Reading)
            .then(|| self.reading_position())
            .flatten();
        // The old service shuts down on its own thread: a stuck engine must
        // not hold up the keyboard.
        let old = std::mem::replace(&mut self.speech, service);
        let _ = std::thread::Builder::new()
            .name("textweaver-speech-shutdown".into())
            .spawn(move || drop(old));
        self.track = SpeechTrack::default();
        self.playback = Playback::Idle;
        self.continue_from = None;
        self.planned_end = None;
        self.speech_caps = self.speech.capabilities();
        let silent = name == "null" || name == "silent";
        self.backend_name = name;
        self.self_voicing = std::mem::take(&mut self.restart.voiced) && !silent;
        self.apply_voice_settings();
        // At the first start, messages from starting the engine join the
        // ones held meanwhile, after them.
        for m in messages {
            self.error(&m);
        }
        if first {
            let early = std::mem::take(&mut self.restart.early);
            self.restart.hold = false;
            if silent {
                let msg = self.msg("restart-no-engine");
                self.tell(&msg);
            } else if let Some(pos) = reading_from {
                // Reading started meanwhile: it goes on, and is what is
                // heard; the messages stay on the status line.
                self.read_from(pos);
            } else if self.self_voicing {
                self.say_early_messages(early);
            }
            return vec![Effect::Redraw];
        }
        let msg = if silent {
            self.msg("restart-done-silent")
        } else {
            self.msg("restart-done")
        };
        self.tell(&msg);
        vec![Effect::Redraw]
    }
}
