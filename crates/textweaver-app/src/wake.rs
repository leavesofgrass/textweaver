//! The waker (Wave 3, Agent W3a): how a frontend learns there is something
//! to apply without polling every 30 ms.
//!
//! A frontend gives the app a [`Waker`] with [`App::set_waker`]: a
//! callback, called from other threads, that must be quick and must not
//! block. A GUI posts an event to its event loop (winit's
//! `EventLoopProxy::send_event`); a server sends on a channel
//! ([`channel_waker`]). The waker rings when:
//!
//! - the speech thread sends statuses: a word heard, a reading paused,
//!   stopped, or finished, an engine error, the thread ending;
//! - the writer thread finishes a job: a save, a state save, a settings
//!   save, a change-on-disk check;
//! - background work finishes: a document opened in the background, a
//!   restarted or first-started speech engine, the misspelling count after
//!   a save, an export, a reference lookup, a library scan, the Markdown
//!   structure parsed while editing, the voice list.
//!
//! When woken, the frontend calls [`App::poll_speech`] and [`App::tick`]
//! on its own thread, as it does after a key press. Timers the app keeps
//! (RSVP words, autosave, the periodic position save) still need a tick
//! now and then: [`App::tick_interval`] says how long the frontend may
//! sleep when nothing wakes it.
//!
//! [`App::set_waker`]: crate::App::set_waker
//! [`App::poll_speech`]: crate::App::poll_speech
//! [`App::tick`]: crate::App::tick
//! [`App::tick_interval`]: crate::App::tick_interval

use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub use textweaver_speech::Waker;

use crate::app::App;

/// Where the app keeps the waker: shared with the writer thread and with
/// every background job, which ring it when they finish.
#[derive(Clone, Default)]
pub(crate) struct WakeSlot(Arc<Mutex<Option<Waker>>>);

impl std::fmt::Debug for WakeSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("WakeSlot")
            .field(&self.get().is_some())
            .finish()
    }
}

impl WakeSlot {
    pub(crate) fn get(&self) -> Option<Waker> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn set(&self, waker: Option<Waker>) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = waker;
    }

    /// Rings the waker, if one is set.
    pub(crate) fn wake(&self) {
        if let Some(w) = self.get() {
            w();
        }
    }
}

/// A waker that sends `()` on a channel, and the channel's receiving end.
/// A frontend that waits on a channel anyway (the JSON-RPC server) selects
/// on it; several rings before it looks collapse into a few messages.
pub fn channel_waker() -> (Waker, Receiver<()>) {
    let (tx, rx) = channel::<()>();
    let tx: Mutex<Sender<()>> = Mutex::new(tx);
    let waker: Waker = Arc::new(move || {
        if let Ok(t) = tx.lock() {
            let _ = t.send(());
        }
    });
    (waker, rx)
}

/// How long a frontend may sleep between ticks when nothing wakes it and no
/// key is pressed: long enough to cost nothing, short enough for the
/// autosave, the periodic position save, and the disk check.
pub const IDLE_TICK: Duration = Duration::from_millis(250);

/// The tick interval while something times itself in small steps: Markdown
/// structure parsed when typing pauses, continuous reading on the status
/// line.
pub const BUSY_TICK: Duration = Duration::from_millis(50);

impl App {
    /// Sets (or clears, with `None`) the waker: a callback the app rings
    /// from other threads when there is something to apply (see the
    /// [`wake`](crate::wake) module). It is passed on to the speech
    /// service, to a speech service started later, to the writer thread,
    /// and to background jobs.
    pub fn set_waker(&mut self, waker: Option<Waker>) {
        self.speech.set_waker(waker.clone());
        self.wake.set(waker);
    }

    /// How long the frontend may wait before its next [`App::tick`] when
    /// nothing wakes it: the next RSVP word, [`BUSY_TICK`] while editing
    /// or reading on the status line, else [`IDLE_TICK`]. Speech statuses
    /// and finished background work ring the waker instead.
    pub fn tick_interval(&self, now: Instant) -> Duration {
        let base = if self.edit.is_some() || self.screen_say_all.is_some() || self.opening() {
            BUSY_TICK
        } else {
            IDLE_TICK
        };
        self.rsvp_wait(now).map_or(base, |w| w.min(base))
    }

    /// A clone of the wake slot, for a background job to ring when it
    /// finishes.
    pub(crate) fn waker_slot(&self) -> WakeSlot {
        self.wake.clone()
    }
}
