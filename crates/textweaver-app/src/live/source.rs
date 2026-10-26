//! Live sources: where a live document's updates come from.
//!
//! A source runs on its own (an MQTT connection on a background thread,
//! say) and hands the app what arrived since it last asked, through
//! [`LiveSource::poll`], which never blocks. When something arrives, the
//! source rings the waker the app gave it ([`App::live_waker`]), and the
//! app polls on its next tick ([`App::tick`]).
//!
//! A source never interprets its messages: a JSON payload is turned into
//! log entries by the log view ([`super::LiveLog`]), not by the source.
//!
//! [`App::live_waker`]: crate::App::live_waker
//! [`App::tick`]: crate::App::tick

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// A message's body, as the source received it.
#[derive(Clone, Debug, PartialEq)]
pub enum LivePayload {
    /// JSON, parsed, its key order unknown: objects show their keys in
    /// sorted order.
    Json(serde_json::Value),
    /// JSON, parsed, with the order its keys came in, so the views keep
    /// the message's order. Prefer this to [`LivePayload::Json`].
    OrderedJson(serde_json::Value, super::KeyOrder),
    /// Text that is not JSON.
    Text(String),
    /// Bytes that are not text: only their length is kept.
    Binary {
        /// The length in bytes.
        len: usize,
    },
    /// A message over the source's size cap: only its length is kept.
    TooLarge {
        /// The length in bytes.
        len: usize,
    },
}

/// What a live source reports.
#[derive(Clone, Debug, PartialEq)]
pub enum LiveEvent {
    /// Connecting, or connecting again after a loss (`attempt` counts from
    /// 1).
    Connecting {
        /// Which attempt this is.
        attempt: u32,
    },
    /// Connected.
    Connected {
        /// The connection is encrypted (TLS).
        encrypted: bool,
        /// The host is reached over Tailscale, which encrypts the link.
        tailscale: bool,
    },
    /// A message arrived.
    Message {
        /// Where it was published (an MQTT topic).
        topic: String,
        /// Its body.
        payload: LivePayload,
        /// The broker kept it and sent it on subscribing (an MQTT retained
        /// message): typically the whole log so far.
        retained: bool,
    },
    /// The connection was lost; `reason` is a sentence in words.
    Disconnected {
        /// Why, in words.
        reason: String,
    },
    /// Nothing has arrived on `topic` for `seconds` since subscribing.
    Quiet {
        /// The topic.
        topic: String,
        /// How long it has been quiet.
        seconds: u64,
    },
    /// The connection could not be made or was lost, and the source
    /// stopped trying: only the user can fix it (a wrong password, an
    /// untrusted certificate, a settings error). `reason` is a sentence in
    /// words.
    Failed {
        /// Why, in words.
        reason: String,
    },
    /// The broker refused the subscription to `topic`: it may not let
    /// this account read it.
    TopicRefused {
        /// The topic.
        topic: String,
    },
    /// Messages the source left out because the reader fell behind; the
    /// connection stayed up.
    Skipped {
        /// How many.
        count: u64,
    },
}

/// A live source. Implementations run their own threads; the app only
/// polls.
pub trait LiveSource: Send {
    /// What arrived since the last poll, oldest first. Never blocks.
    fn poll(&mut self) -> Vec<LiveEvent>;

    /// Stops the source: no more events, its threads and connections
    /// end. Called once, when the live document closes.
    fn stop(&mut self);
}

/// The shared side of a [`ScriptedSource`].
#[derive(Default)]
struct Script {
    queue: VecDeque<LiveEvent>,
    stopped: bool,
    polls: usize,
    wake: Option<Box<dyn Fn() + Send + Sync>>,
}

/// A source driven by hand: tests and demonstrations push events through
/// its [`ScriptHandle`], and the app polls them as it would a real
/// source's. No network, no threads.
pub struct ScriptedSource {
    script: Arc<Mutex<Script>>,
}

/// Pushes events into a [`ScriptedSource`] and sees whether it was
/// stopped. Clones share the source.
#[derive(Clone)]
pub struct ScriptHandle {
    script: Arc<Mutex<Script>>,
}

impl ScriptedSource {
    /// A source with nothing queued, and the handle that feeds it.
    pub fn new() -> (ScriptedSource, ScriptHandle) {
        let script = Arc::new(Mutex::new(Script::default()));
        (
            ScriptedSource {
                script: Arc::clone(&script),
            },
            ScriptHandle { script },
        )
    }
}

fn lock(script: &Mutex<Script>) -> std::sync::MutexGuard<'_, Script> {
    script.lock().unwrap_or_else(|e| e.into_inner())
}

impl LiveSource for ScriptedSource {
    fn poll(&mut self) -> Vec<LiveEvent> {
        let mut s = lock(&self.script);
        s.polls += 1;
        if s.stopped {
            return Vec::new();
        }
        s.queue.drain(..).collect()
    }

    fn stop(&mut self) {
        let mut s = lock(&self.script);
        s.stopped = true;
        s.queue.clear();
    }
}

impl ScriptHandle {
    /// Queues `event` and rings the waker, if one was given
    /// ([`set_waker`](Self::set_waker)). Ignored once stopped.
    pub fn push(&self, event: LiveEvent) {
        let mut s = lock(&self.script);
        if s.stopped {
            return;
        }
        s.queue.push_back(event);
        if let Some(w) = &s.wake {
            w();
        }
    }

    /// Queues a JSON message on `topic`, as a source would deliver it.
    pub fn push_json(&self, topic: &str, json: &str) {
        let payload = match super::order::parse_ordered(json) {
            Some((v, order)) => LivePayload::OrderedJson(v, order),
            None => LivePayload::Text(json.to_owned()),
        };
        self.push(LiveEvent::Message {
            topic: topic.to_owned(),
            payload,
            retained: false,
        });
    }

    /// The waker [`push`](Self::push) rings, as a real source rings the
    /// one the app gives it ([`App::live_waker`](crate::App::live_waker)).
    pub fn set_waker(&self, wake: Box<dyn Fn() + Send + Sync>) {
        lock(&self.script).wake = Some(wake);
    }

    /// True once the app stopped the source.
    pub fn stopped(&self) -> bool {
        lock(&self.script).stopped
    }

    /// How many times the app polled the source.
    pub fn polls(&self) -> usize {
        lock(&self.script).polls
    }
}
