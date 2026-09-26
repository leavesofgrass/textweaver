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

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};

use textweaver_speech::SpeechService;
use textweaver_store::Settings;

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
}

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

    /// True while speech is being restarted (or started for the first time
    /// in the background).
    pub fn speech_restarting(&self) -> bool {
        self.restart.pending.is_some()
    }

    /// Starts the speech engine on a helper thread instead of waiting for
    /// it (Wave 3): the app starts with a silent service (pass
    /// `SpeechService::null()` in `AppConfig`), and [`App::tick`] swaps
    /// the engine in when it is ready, ringing the waker. Messages said
    /// meanwhile are shown; the latest is spoken once the engine is ready,
    /// and a reading started meanwhile goes on from where it is. The
    /// starter is also kept for Restart Speech, as with
    /// [`set_speech_starter`](Self::set_speech_starter).
    ///
    /// [`App::tick`]: crate::App::tick
    pub fn start_speech_in_background(&mut self, starter: SpeechStarter) {
        self.restart.starter = Some(starter);
        self.restart.first = true;
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
        let keys =
            crate::help::chords_text(&self.keymap, textweaver_keymap::ActionId::RestartSpeech);
        if self.restart.starter.is_none() {
            return if voiced {
                "textweaver is silent now; restart it to hear speech again.".into()
            } else {
                String::new()
            };
        }
        if std::mem::replace(&mut self.restart.auto_used, true) {
            return format!("textweaver is silent now. Restart speech with {keys}.");
        }
        self.begin_restart();
        "Restarting speech.".into()
    }

    /// The Restart Speech command.
    pub(crate) fn restart_speech_command(&mut self) -> Vec<Effect> {
        if self.restart.starter.is_none() {
            self.tell("Speech cannot be restarted here.");
            return vec![Effect::Redraw];
        }
        if self.restart.pending.is_some() {
            self.tell("Speech is already restarting.");
            return vec![Effect::Redraw];
        }
        self.stop_speech();
        self.restart.voiced |= self.self_voicing;
        self.show("Restarting speech.");
        self.begin_restart();
        vec![Effect::Redraw]
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
            Err(e) => self.error(&format!("Could not restart speech: {e}.")),
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
                self.error("Could not restart speech: starting it failed.");
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
        for m in messages {
            self.error(&m);
        }
        if first {
            if silent {
                self.tell("No speech engine is available; textweaver stays silent.");
            } else if let Some(pos) = reading_from {
                self.read_from(pos);
            } else if self.self_voicing
                && let Some(last) = self.status.current.clone().filter(|s| !s.is_empty())
            {
                // Say the latest message, spoken to no one while the engine
                // started ("Opened report.").
                self.speech.say(last, textweaver_speech::SayMode::Queue);
            }
            return vec![Effect::Redraw];
        }
        if silent {
            self.tell(
                "Speech restarted, but no speech engine is available; textweaver stays silent.",
            );
        } else {
            self.tell("Speech restarted.");
        }
        vec![Effect::Redraw]
    }
}
