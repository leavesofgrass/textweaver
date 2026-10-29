//! textweaver's own broker inside the app, and the app's senders (beta 5,
//! phase 5b, with the `mqtt` feature).
//!
//! - **Start a broker** runs the same broker as `tw broker`
//!   (`textweaver_broker`), as `[live.broker]` says: on 127.0.0.1 with
//!   TLS unless other addresses are set, the accounts of `broker.toml` in
//!   the data folder, and a join code for the session unless it is
//!   turned off. Its status is in words, meaning first: said when it
//!   starts, when a reader joins or leaves, and when it refuses someone,
//!   and shown in the status bar ("Broker on, 2 clients").
//! - **Stop the broker** stops it: every connection ends and every port
//!   is free before the command answers. Quitting stops it too.
//! - **Show the join code** says the code in groups of four and shows it
//!   in a list, one group to a line, then Copy the join code and Copy the
//!   join link as the list's last rows; with no join code, the
//!   certificate pin, which readers pin instead.
//! - **Copy the join code** and **Copy the join link** put them on the
//!   clipboard, as Copy does, and say so. The join link
//!   ([`textweaver_mqtt::JoinLink`]) carries the broker's address, port
//!   and code, and the shared document's topic while one is shared into
//!   the broker. Both hold the secret: neither is said, logged, saved or
//!   shown in the status line.
//! - **Share this document live** publishes the open document as a live
//!   document message (crate::live::document), retained, on
//!   `[live.broker] share_topic`, then again each time it is saved, until
//!   **Stop sharing**. It goes into the app's broker when one runs, else
//!   through a live source chosen from a list, signed in as that source.
//! - **Send a check-in** (crate::live::checkin) publishes one entry.
//!
//! Publishing through a live source's broker connects, publishes at QoS 1
//! and waits for the acknowledgment on a helper thread, so the keys never
//! wait; the answer is said on the next tick.

use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};

use textweaver_broker::{ACCOUNTS_FILE, Accounts, Broker, BrokerConfig, BrokerEvent, Network};
use textweaver_lexicon::args;
use textweaver_mqtt::{JoinLink, document_message, format_for_name, publish_once};

use super::checkin::SendTarget;
use super::wire::{LiveList, Sent};
use crate::app::{App, ListKind};
use crate::command::Effect;

/// The largest message the app sends: the receivers' own cap.
pub const MAX_MESSAGE: usize = 256 * 1024;

/// The broker the app runs.
pub(crate) struct AppBroker {
    broker: Broker,
    /// What the broker's threads reported, said on the next tick.
    events: Arc<Mutex<Vec<BrokerEvent>>>,
}

impl std::fmt::Debug for AppBroker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppBroker")
            .field("addrs", &self.broker.local_addrs())
            .finish_non_exhaustive()
    }
}

/// The document being shared.
#[derive(Clone, Debug)]
pub(crate) struct Share {
    /// Its file, whose saves are shared again.
    path: Option<PathBuf>,
    /// The title readers see.
    title: String,
    /// The text to share when the file cannot be read as text.
    text: String,
    /// The revision last published.
    pub(crate) rev: u64,
    topic: String,
    target: SendTarget,
}

/// A publish on a helper thread, and what to say when it is done.
pub(crate) struct SendJob {
    rx: Receiver<Result<(), String>>,
    sent: Sent,
    topic: String,
}

impl std::fmt::Debug for SendJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SendJob")
            .field("topic", &self.topic)
            .finish_non_exhaustive()
    }
}

/// `--listen` values as `tw broker` reads them: an address with a port,
/// or an address alone (given `port`).
fn listen_addrs(values: &[String], port: u16) -> Result<Vec<SocketAddr>, String> {
    let mut out = Vec::new();
    for v in values.iter().map(|v| v.trim()).filter(|v| !v.is_empty()) {
        if let Ok(a) = v.parse::<SocketAddr>() {
            out.push(a);
        } else if let Ok(ip) = v
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<IpAddr>()
        {
            out.push(SocketAddr::new(ip, port));
        } else {
            return Err(v.to_owned());
        }
    }
    if out.is_empty() {
        out.push(SocketAddr::from(([127, 0, 0, 1], port)));
    }
    Ok(out)
}

/// `code` read aloud in its groups of four: "ABCD, EFGH, JKMN, PQRS".
fn said_in_groups(grouped: &str) -> String {
    grouped.split('-').collect::<Vec<_>>().join(", ")
}

impl App {
    /// True while the app runs a broker.
    pub fn live_broker_running(&self) -> bool {
        self.live_wire.broker.is_some()
    }

    /// The addresses the app's broker listens on, with their real ports.
    pub fn live_broker_addrs(&self) -> Vec<SocketAddr> {
        self.live_wire
            .broker
            .as_ref()
            .map(|b| b.broker.local_addrs().to_vec())
            .unwrap_or_default()
    }

    /// The app's broker's join code, in groups of four, if it has one.
    pub fn live_broker_join_code(&self) -> Option<String> {
        let b = self.live_wire.broker.as_ref()?;
        b.broker.join_code().map(|c| c.grouped())
    }

    /// The app's broker's join link: the address readers reach (the
    /// first one that is not loopback, else the first), its port, the join
    /// code, and the share topic while a document is shared into the
    /// broker. `None` without a join code.
    pub fn live_join_link(&self) -> Option<JoinLink> {
        let b = self.live_wire.broker.as_ref()?;
        let code = b.broker.join_code()?.clone();
        let addrs = b.broker.local_addrs();
        let addr = addrs
            .iter()
            .find(|a| !a.ip().is_loopback())
            .or_else(|| addrs.first())?;
        let topic = self
            .live_wire
            .share
            .as_ref()
            .filter(|s| s.target == SendTarget::Broker)
            .map(|s| s.topic.as_str());
        JoinLink::new(&JoinLink::host_for(addr.ip()), addr.port(), code, topic).ok()
    }

    /// True while the app's broker has a join code to copy.
    pub(crate) fn live_has_join_code(&self) -> bool {
        self.live_wire
            .broker
            .as_ref()
            .is_some_and(|b| b.broker.join_code().is_some())
    }

    /// Copy the join code: the code, in groups of four, to the clipboard.
    pub fn live_copy_join_code(&mut self) -> Vec<Effect> {
        match self.live_broker_join_code() {
            Some(code) => {
                self.live_copy_secret(code, "live-join-code-copied");
                vec![Effect::Redraw]
            }
            None => self.live_say_no_join_code(),
        }
    }

    /// Copy the join link: the link, on one line, to the clipboard.
    pub fn live_copy_join_link(&mut self) -> Vec<Effect> {
        match self.live_join_link() {
            Some(link) => {
                self.live_copy_secret(link.to_string(), "live-join-link-copied");
                vec![Effect::Redraw]
            }
            None => self.live_say_no_join_code(),
        }
    }

    /// Puts a secret on the clipboard as Copy does (the frontend takes it
    /// with [`App::take_clipboard`]) and says `said_id`, never the secret.
    fn live_copy_secret(&mut self, secret: String, said_id: &str) {
        self.authoring.copied = Some(secret.clone());
        self.clipboard = Some(secret);
        let m = self.msg(said_id);
        self.tell(&m);
    }

    /// Why there is no join code to copy, in words.
    fn live_say_no_join_code(&mut self) -> Vec<Effect> {
        let id = if self.live_wire.broker.is_none() {
            "live-broker-not-running"
        } else if self.live_broker_pin().is_some() {
            "live-broker-no-join-code"
        } else {
            "live-broker-no-join-code-plain"
        };
        let m = self.msg(id);
        self.tell(&m);
        vec![Effect::Redraw]
    }

    /// The app's broker's certificate pin, in groups of four, with TLS.
    pub fn live_broker_pin(&self) -> Option<String> {
        let b = self.live_wire.broker.as_ref()?;
        b.broker.pin().map(|p| p.grouped())
    }

    /// The broker and the sharing, in words, for the status bar, meaning
    /// first ("Broker on, 2 clients", "Sharing, revision 3"); `None` when
    /// neither runs.
    pub fn live_broker_words(&self) -> Option<String> {
        let broker =
            self.live_wire.broker.as_ref().map(|b| {
                self.msg_args("live-broker-status", &args!["count" => b.broker.clients()])
            });
        let share = self
            .live_wire
            .share
            .as_ref()
            .map(|s| self.msg_args("live-share-status", &args!["rev" => s.rev]));
        match (broker, share) {
            (None, None) => None,
            (Some(b), None) => Some(b),
            (None, Some(s)) => Some(s),
            (Some(b), Some(s)) => Some(format!("{b}, {s}")),
        }
    }

    /// Start a broker: the same broker as `tw broker`, as `[live.broker]`
    /// says. Its addresses, encryption and join code are said.
    pub fn live_start_broker(&mut self) -> Vec<Effect> {
        if self.live_wire.broker.is_some() {
            let m = self.msg("live-broker-already");
            self.tell(&m);
            return self.live_say_join_code();
        }
        let Some(dir) = self.paths.as_ref().map(|p| p.data_dir.clone()) else {
            let m = self.msg_args(
                "live-broker-not-started",
                &args!["reason" => self.msg("live-broker-no-folder")],
            );
            self.error(&m);
            return vec![Effect::Redraw];
        };
        match self.live_broker_config(&dir) {
            Ok(config) => self.live_broker_run(config),
            Err(reason) => {
                let m = self.msg_args("live-broker-not-started", &args!["reason" => reason]);
                self.error(&m);
                vec![Effect::Redraw]
            }
        }
    }

    /// The broker's configuration from `[live.broker]` and `broker.toml`
    /// in `dir`; why not, in words.
    fn live_broker_config(&self, dir: &Path) -> Result<BrokerConfig, String> {
        let b = &self.settings.live.broker;
        let accounts = Accounts::load(&dir.join(ACCOUNTS_FILE))?;
        if accounts.is_empty() && !b.join_code {
            return Err(self.msg("live-broker-nobody"));
        }
        let port = b.port_or_default();
        let listen = listen_addrs(&b.listen, port)
            .map_err(|bad| self.msg_args("live-broker-bad-address", &args!["address" => bad]))?;
        let mut config = BrokerConfig::new(dir);
        config.tls = !b.plain;
        config.listen = listen;
        config.join_code = b.join_code;
        let topics: Vec<String> = b
            .join_topics
            .iter()
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .map(str::to_owned)
            .collect();
        if !topics.is_empty() {
            config.join_topics = topics;
        }
        config.accounts = accounts;
        Ok(config)
    }

    /// Starts the broker with `config` and says how it runs.
    fn live_broker_run(&mut self, config: BrokerConfig) -> Vec<Effect> {
        let tls = config.tls;
        let events: Arc<Mutex<Vec<BrokerEvent>>> = Arc::default();
        let queue = Arc::clone(&events);
        let wake = self.waker_slot();
        let started = Broker::start(config, move |event| {
            queue.lock().unwrap_or_else(|p| p.into_inner()).push(event);
            wake.wake();
        });
        let broker = match started {
            Ok(b) => b,
            Err(reason) => {
                let m = self.msg_args("live-broker-not-started", &args!["reason" => reason]);
                self.error(&m);
                return vec![Effect::Redraw];
            }
        };
        // The listening lines are said here, in one message.
        let addrs: Vec<String> = broker
            .local_addrs()
            .iter()
            .map(ToString::to_string)
            .collect();
        let local = broker
            .local_addrs()
            .iter()
            .all(|a| Network::of(a.ip()) == Network::Loopback);
        let key = match (tls, local) {
            (true, true) => "live-broker-started-local",
            (true, false) => "live-broker-started",
            (false, true) => "live-broker-started-plain-local",
            (false, false) => "live-broker-started-plain",
        };
        let m = self.msg_args(key, &args!["addresses" => addrs.join(", ")]);
        self.tell(&m);
        let plain_open = !tls
            && broker
                .local_addrs()
                .iter()
                .any(|a| matches!(Network::of(a.ip()), Network::Other | Network::Every));
        if plain_open {
            let w = self.msg("live-broker-plain-warning");
            self.warn(&w);
        }
        if let Ok(mut e) = events.lock() {
            e.retain(|ev| !matches!(ev, BrokerEvent::Listening { .. }));
        }
        self.live_wire.broker = Some(AppBroker { broker, events });
        self.live_say_join_code();
        vec![Effect::Redraw]
    }

    /// Says the join code in groups of four, or the pin readers check.
    fn live_say_join_code(&mut self) -> Vec<Effect> {
        if let Some(code) = self.live_broker_join_code() {
            let m = self.msg_args(
                "live-broker-join-code",
                &args!["code" => said_in_groups(&code)],
            );
            self.tell(&m);
        } else if let Some(pin) = self.live_broker_pin() {
            let m = self.msg_args("live-broker-pin", &args!["pin" => said_in_groups(&pin)]);
            self.tell(&m);
        }
        vec![Effect::Redraw]
    }

    /// Show the join code: said in groups of four, and shown in a list,
    /// one group to a line, then the two Copy rows; without a join code,
    /// the certificate pin.
    pub fn live_show_join_code(&mut self) -> Vec<Effect> {
        if self.live_wire.broker.is_none() {
            let m = self.msg("live-broker-not-running");
            self.tell(&m);
            return vec![Effect::Redraw];
        }
        let (grouped, title, first) = match (self.live_broker_join_code(), self.live_broker_pin()) {
            (Some(code), _) => {
                self.live_say_join_code();
                (code.clone(), self.msg("live-join-code-title"), code)
            }
            (None, Some(pin)) => {
                let m = self.msg("live-broker-no-join-code");
                self.tell(&m);
                (pin, self.msg("live-pin-title"), String::new())
            }
            (None, None) => {
                let m = self.msg("live-broker-no-join-code-plain");
                self.tell(&m);
                return vec![Effect::Redraw];
            }
        };
        let groups: Vec<&str> = grouped.split('-').collect();
        let total = groups.len();
        let mut items = Vec::new();
        let has_code = !first.is_empty();
        if has_code {
            items.push(first);
        }
        items.extend(groups.iter().enumerate().map(|(i, g)| {
            self.msg_args(
                "live-code-group",
                &args!["n" => i + 1, "total" => total, "group" => *g],
            )
        }));
        // The Copy rows, as buttons: Enter on one copies.
        let copy_rows = has_code.then_some(items.len());
        if has_code {
            items.push(self.msg("name-copy-join-code"));
            items.push(self.msg("name-copy-join-link"));
        }
        self.list = Some(ListKind::Live(LiveList::Code(copy_rows)));
        vec![Effect::ShowList { title, items }]
    }

    /// Stop the broker: every connection ends and every port is free.
    /// Sharing into it stops too.
    pub fn live_stop_broker(&mut self) -> Vec<Effect> {
        let Some(b) = self.live_wire.broker.take() else {
            let m = self.msg("live-broker-not-running");
            self.tell(&m);
            return vec![Effect::Redraw];
        };
        b.broker.stop();
        if self
            .live_wire
            .share
            .as_ref()
            .is_some_and(|s| s.target == SendTarget::Broker)
        {
            self.live_wire.share = None;
            let m = self.msg("live-share-stopped-with-broker");
            self.note(&m);
        }
        let m = self.msg("live-broker-stopped");
        self.tell(&m);
        vec![Effect::Redraw]
    }

    /// Stops the broker without a word, when the app closes.
    pub(crate) fn live_broker_shutdown(&mut self) {
        self.live_wire.share = None;
        if let Some(b) = self.live_wire.broker.take() {
            b.broker.stop();
        }
    }

    /// Says what the broker reported and what the helper threads sent
    /// (from [`App::tick`]).
    pub(crate) fn live_broker_tick(&mut self) -> Vec<Effect> {
        let mut effects = Vec::new();
        let events: Vec<BrokerEvent> = self
            .live_wire
            .broker
            .as_ref()
            .map(|b| std::mem::take(&mut *b.events.lock().unwrap_or_else(|p| p.into_inner())))
            .unwrap_or_default();
        for event in events {
            self.live_broker_event(event);
            effects.push(Effect::Redraw);
        }
        let jobs = std::mem::take(&mut self.live_wire.jobs);
        for job in jobs {
            match job.rx.try_recv() {
                Ok(result) => {
                    self.live_sent(job.sent, &job.topic, result);
                    effects.push(Effect::Redraw);
                }
                Err(TryRecvError::Empty) => self.live_wire.jobs.push(job),
                Err(TryRecvError::Disconnected) => {
                    let reason = self.msg("live-send-lost");
                    self.live_sent(job.sent, &job.topic, Err(reason));
                    effects.push(Effect::Redraw);
                }
            }
        }
        effects
    }

    /// Says one of the broker's events, meaning first.
    fn live_broker_event(&mut self, event: BrokerEvent) {
        match event {
            BrokerEvent::Connected { user, .. } => {
                let m = self.msg_args("live-broker-joined", &args!["user" => user]);
                self.note(&m);
            }
            BrokerEvent::Left { user, .. } => {
                let m = self.msg_args("live-broker-left", &args!["user" => user]);
                self.note(&m);
            }
            BrokerEvent::LoginRefused { user, addr } => {
                let m = self.msg_args(
                    "live-broker-refused",
                    &args!["user" => user, "address" => addr.ip().to_string()],
                );
                self.tell(&m);
            }
            BrokerEvent::Listening { .. } | BrokerEvent::Stopped => {}
            other => {
                // Lockouts, refused publishes, a full broker: the broker's
                // own sentence, which leads with what happened.
                let m = self.msg_args("live-broker-event", &args!["event" => other.to_string()]);
                self.warn(&m);
            }
        }
    }

    /// Publishes `payload` on `topic` through `target`, and says the
    /// result as `sent` asks: at once into the app's broker, or on a
    /// helper thread through a live source's broker.
    pub(crate) fn live_send(
        &mut self,
        target: SendTarget,
        topic: &str,
        payload: Vec<u8>,
        retain: bool,
        sent: Sent,
    ) -> Vec<Effect> {
        if payload.len() > MAX_MESSAGE {
            let reason = self.msg_args(
                "live-send-too-large",
                &args!["len" => payload.len(), "max" => MAX_MESSAGE],
            );
            self.live_sent(sent, topic, Err(reason));
            return vec![Effect::Redraw];
        }
        match target {
            SendTarget::Broker => {
                let result = match self.live_wire.broker.as_ref() {
                    Some(b) => b.broker.publish(topic, &payload, retain),
                    None => Err(self.msg("live-broker-not-running")),
                };
                self.live_sent(sent, topic, result);
            }
            SendTarget::Source(id) => {
                let Some(source) = self.settings.live.sources.get(&id).cloned() else {
                    let m = self.msg_args("live-source-unknown", &args!["source" => id.as_str()]);
                    self.error(&m);
                    return vec![Effect::Redraw];
                };
                let password = if source.username.trim().is_empty() {
                    None
                } else {
                    match self.live_password(&id) {
                        Some(p) => Some(p),
                        None => {
                            let name = source.display_name(&id).to_owned();
                            let m = self.msg_args("live-send-no-password", &args!["name" => name]);
                            self.error(&m);
                            return vec![Effect::Redraw];
                        }
                    }
                };
                let config =
                    super::mqtt::mqtt_config(&self.settings.live, &source, password.as_ref());
                let (tx, rx) = mpsc::channel();
                let wake = self.waker_slot();
                let to = topic.to_owned();
                let spawned = std::thread::Builder::new()
                    .name("textweaver-live-send".into())
                    .spawn(move || {
                        let result = publish_once(&config, &to, &payload, retain)
                            .map_err(|e| e.text().to_owned());
                        let _ = tx.send(result);
                        wake.wake();
                    });
                match spawned {
                    Ok(_) => self.live_wire.jobs.push(SendJob {
                        rx,
                        sent,
                        topic: topic.to_owned(),
                    }),
                    Err(e) => self.live_sent(sent, topic, Err(e.to_string())),
                }
            }
        }
        vec![Effect::Redraw]
    }

    /// Says how a publish went.
    fn live_sent(&mut self, sent: Sent, topic: &str, result: Result<(), String>) {
        match (sent, result) {
            (Sent::CheckIn { first }, Ok(())) => {
                let m = self.msg_args(
                    "live-checkin-sent",
                    &args!["entry" => first, "topic" => topic],
                );
                self.tell(&m);
            }
            (Sent::Share { title, rev }, Ok(())) => {
                let m = self.msg_args(
                    "live-share-sent",
                    &args!["name" => title, "rev" => rev, "topic" => topic],
                );
                if rev <= 1 {
                    self.tell(&m);
                } else {
                    self.note(&m);
                }
            }
            (Sent::CheckIn { .. }, Err(reason)) => {
                let m = self.msg_args("live-checkin-not-sent", &args!["reason" => reason]);
                self.error(&m);
            }
            (Sent::Share { title, .. }, Err(reason)) => {
                let m = self.msg_args(
                    "live-share-not-sent",
                    &args!["name" => title, "reason" => reason],
                );
                self.error(&m);
            }
        }
    }

    // ----- Sharing ------------------------------------------------------

    /// Share this document live: the open document, published now and
    /// at each save, into the app's broker or through a chosen source.
    pub fn live_share_command(&mut self) -> Vec<Effect> {
        if let Some(s) = &self.live_wire.share {
            let m = self.msg_args(
                "live-share-already",
                &args!["name" => s.title.as_str(), "topic" => s.topic.as_str()],
            );
            self.tell(&m);
            return vec![Effect::Redraw];
        }
        let Some(s) = self.session.as_ref() else {
            let m = self.msg("live-share-no-document");
            self.tell(&m);
            return vec![Effect::Redraw];
        };
        if s.live.is_some() {
            let m = self.msg("live-share-live-document");
            self.tell(&m);
            return vec![Effect::Redraw];
        }
        if self.live_wire.broker.is_some() {
            return self.live_share_start(SendTarget::Broker);
        }
        self.live_choose_target(LiveList::ShareThrough, "live-share-through-title")
    }

    /// The live sources to publish through, as a list; with none set up,
    /// says to start a broker or set one up.
    pub(crate) fn live_choose_target(
        &mut self,
        kind: fn(Vec<String>) -> LiveList,
        title_id: &str,
    ) -> Vec<Effect> {
        let ids: Vec<String> = self.settings.live.sources.keys().cloned().collect();
        if ids.is_empty() {
            let m = self.msg("live-send-nowhere");
            self.tell(&m);
            return vec![Effect::Redraw];
        }
        let items = ids
            .iter()
            .map(|id| self.settings.live.sources[id].display_name(id).to_owned())
            .collect();
        let title = self.msg(title_id);
        self.list = Some(ListKind::Live(kind(ids)));
        vec![Effect::ShowList { title, items }]
    }

    /// Starts sharing the open document through `target`: revision 1 now.
    pub(crate) fn live_share_start(&mut self, target: SendTarget) -> Vec<Effect> {
        let Some(s) = self.session.as_ref().filter(|s| s.live.is_none()) else {
            let m = self.msg("live-share-no-document");
            self.tell(&m);
            return vec![Effect::Redraw];
        };
        let share = Share {
            path: s.doc.meta.path.clone(),
            title: s.title.clone(),
            text: s.doc.text().to_string(),
            rev: 0,
            topic: self.settings.live.broker.share_topic().to_owned(),
            target,
        };
        self.live_wire.share = Some(share);
        self.live_share_publish()
    }

    /// Publishes the next revision of the shared document.
    fn live_share_publish(&mut self) -> Vec<Effect> {
        let Some(share) = self.live_wire.share.as_mut() else {
            return vec![Effect::Redraw];
        };
        share.rev += 1;
        let (text, format) = share_text(share);
        let message = document_message(share.rev, &share.title, format, &text);
        let (target, topic) = (share.target.clone(), share.topic.clone());
        let sent = Sent::Share {
            title: share.title.clone(),
            rev: share.rev,
        };
        self.live_send(target, &topic, message.into_bytes(), true, sent)
    }

    /// A save of `path` finished: the shared document, if it is that
    /// file, is shared again as the next revision.
    pub(crate) fn live_share_saved(&mut self, path: &Path) {
        let same = self
            .live_wire
            .share
            .as_ref()
            .and_then(|s| s.path.as_deref())
            .is_some_and(|p| p == path);
        if !same {
            return;
        }
        // The document's text as saved, should the file not read.
        if let (Some(share), Some(s)) = (self.live_wire.share.as_mut(), self.session.as_ref()) {
            share.text = s.doc.text().to_string();
        }
        let _ = self.live_share_publish();
    }

    /// Stop sharing: no more revisions; readers keep the last one.
    pub fn live_stop_sharing(&mut self) -> Vec<Effect> {
        let Some(s) = self.live_wire.share.take() else {
            let m = self.msg("live-share-none");
            self.tell(&m);
            return vec![Effect::Redraw];
        };
        let m = self.msg_args(
            "live-share-stopped",
            &args!["name" => s.title, "rev" => s.rev],
        );
        self.tell(&m);
        vec![Effect::Redraw]
    }

    // ----- Check-ins ----------------------------------------------------

    /// Send a check-in: the form, for the app's broker, the open live
    /// source, or a source chosen from a list.
    pub fn live_checkin_command(&mut self) -> Vec<Effect> {
        let open = self.open_configured_id();
        if self.live_wire.broker.is_some() {
            return self.open_checkin_form(SendTarget::Broker, open);
        }
        if let Some(id) = open {
            return self.open_checkin_form(SendTarget::Source(id.clone()), Some(id));
        }
        self.live_choose_target(LiveList::CheckInThrough, "live-checkin-through-title")
    }
}

/// The shared document's text and format: its file's text, read again,
/// when it is a Markdown or text file; else the text it opened with.
fn share_text(share: &Share) -> (String, &'static str) {
    if let Some(path) = &share.path {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let lower = name.to_ascii_lowercase();
        let textual = [".md", ".markdown", ".txt", ".text"]
            .iter()
            .any(|e| lower.ends_with(e));
        if textual
            && std::fs::metadata(path).is_ok_and(|m| m.len() <= MAX_MESSAGE as u64)
            && let Ok(text) = std::fs::read_to_string(path)
        {
            return (text, format_for_name(&name));
        }
    }
    (share.text.clone(), "text")
}

#[cfg(test)]
mod tests {
    //! Every broker here listens on 127.0.0.1 with a port the system
    //! picks: loopback only, never a real broker, never audio.

    use std::time::{Duration, Instant};

    use textweaver_a11y::{LogAnnouncer, Shared};
    use textweaver_store::{LiveSourceSettings, LiveTls, Paths};

    use super::*;
    use crate::AppConfig;
    use crate::command::{Command, PromptPurpose};
    use crate::list_model::{ListKey, PromptKey};
    use crate::live::form::JOIN_USER;
    use textweaver_keymap::ActionId;

    struct Side {
        app: App,
        said: Shared<LogAnnouncer>,
        _tmp: tempfile::TempDir,
    }

    impl Side {
        fn new() -> Side {
            textweaver_components::fake::memory_credentials();
            let tmp = tempfile::tempdir().unwrap();
            let said = Shared::new(LogAnnouncer::default());
            let mut app = App::new(AppConfig {
                announcer: Box::new(said.clone()),
                paths: Some(Paths::under(tmp.path())),
                ..AppConfig::for_tests()
            });
            let _ = app.update_settings(|s| {
                s.live.broker.listen = vec!["127.0.0.1:0".into()];
            });
            Side {
                app,
                said,
                _tmp: tmp,
            }
        }

        fn texts(&self) -> Vec<String> {
            self.said
                .lock()
                .texts()
                .iter()
                .map(|s| s.to_string())
                .collect()
        }

        fn any(&self, needle: &str) -> bool {
            self.texts().iter().any(|t| t.contains(needle))
        }

        fn count(&self, needle: &str) -> usize {
            self.texts().iter().filter(|t| t.contains(needle)).count()
        }

        fn act(&mut self, a: ActionId) {
            self.app.dispatch(Command::Action(a));
        }

        fn key(&mut self, k: PromptKey) {
            self.app.dispatch(Command::PromptKey(k));
        }

        fn type_in(&mut self, text: &str) {
            for c in text.chars() {
                self.key(PromptKey::Char(c));
            }
        }

        fn doc_text(&self) -> String {
            self.app
                .session()
                .map(|s| s.doc.text().to_string())
                .unwrap_or_default()
        }

        fn file(&self, name: &str, text: &str) -> PathBuf {
            let p = self._tmp.path().join(name);
            std::fs::write(&p, text).unwrap();
            p
        }
    }

    /// Ticks every side until `done`, for at most 20 seconds.
    fn until(sides: &mut [&mut Side], what: &str, done: impl Fn(&[&mut Side]) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(20);
        while !done(sides) {
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(20));
            for s in sides.iter_mut() {
                s.app.tick(Instant::now());
            }
        }
    }

    /// The reader's source for the sharer's broker, joined by code
    /// through the form, then opened from the offer.
    fn join_through_the_form(r: &mut Side, port: u16, code: &str, topic: &str) {
        r.act(ActionId::SetUpLiveSource);
        r.type_in("Club");
        r.key(PromptKey::Tab);
        r.type_in("127.0.0.1");
        r.key(PromptKey::Tab);
        for _ in 0..4 {
            r.key(PromptKey::Down);
        }
        assert_eq!(r.app.prompt_model().unwrap().text(), "Join code");
        r.key(PromptKey::Tab);
        assert!(
            r.app
                .prompt_model()
                .unwrap()
                .label
                .starts_with("Join code, 4 of 7"),
            "{}",
            r.app.prompt_model().unwrap().label
        );
        // Typed as read aloud: lower case, spaces for dashes.
        r.type_in(&code.to_lowercase().replace('-', " "));
        r.key(PromptKey::Tab);
        r.key(PromptKey::SetText(port.to_string()));
        r.key(PromptKey::Tab);
        r.type_in(topic);
        r.key(PromptKey::Tab);
        assert!(
            r.app
                .prompt_model()
                .unwrap()
                .label
                .starts_with("View, 7 of 7"),
            "no user name when joining by code: {}",
            r.app.prompt_model().unwrap().label
        );
        r.key(PromptKey::Enter);
        // Saved without a password prompt: the code gave it.
        assert!(r.any("Password kept for Club"), "{:?}", r.texts());
        let s = &r.app.settings().live.sources["club"];
        assert_eq!((s.tls, s.username.as_str()), (LiveTls::Pinned, JOIN_USER));
        assert_eq!(s.pin, code[..14]);
        let toml = r.app.settings().to_minimal_toml().unwrap();
        assert!(
            !toml.contains(&code[15..]),
            "the secret is kept nowhere: {toml}"
        );
        // The offer: Open it now.
        assert!(r.app.list_model().is_some());
        r.app.dispatch(Command::ListKey(ListKey::Enter));
    }

    #[test]
    fn the_broker_starts_on_loopback_says_its_code_in_groups_and_stops() {
        let mut s = Side::new();
        assert_eq!(s.app.live_broker_words(), None);
        s.act(ActionId::StartBroker);
        assert!(s.app.live_broker_running(), "{:?}", s.texts());
        let addr = s.app.live_broker_addrs()[0];
        assert!(addr.ip().is_loopback() && addr.port() != 0);
        assert!(
            s.any(&format!("Broker on: {addr}, TLS, this computer only.")),
            "{:?}",
            s.texts()
        );
        let code = s.app.live_broker_join_code().unwrap();
        assert_eq!(code.len(), 19);
        let said = format!("Join code: {}.", code.replace('-', ", "));
        assert!(s.any(&said), "{said} in {:?}", s.texts());
        assert!(said.chars().count() <= 40, "one Braille line: {said}");
        assert_eq!(
            s.app.live_broker_words().as_deref(),
            Some("Broker on, 0 clients")
        );
        assert!(
            s.app
                .title_parts(None)
                .iter()
                .any(|p| p == "Broker on, 0 clients"),
            "{:?}",
            s.app.title_parts(None)
        );
        // Shown in a list, one group to a line.
        s.act(ActionId::ShowJoinCode);
        let list = s.app.list_model().unwrap();
        assert_eq!(list.title, "Join code");
        assert_eq!(list.items[0], code);
        let groups: Vec<&str> = code.split('-').collect();
        assert_eq!(list.items[1], format!("Group 1 of 4: {}", groups[0]));
        assert_eq!(list.items.len(), 7, "the code, four groups, two Copy rows");
        s.app.dispatch(Command::ListKey(ListKey::Escape));
        // Starting again says it runs, and the code again.
        s.act(ActionId::StartBroker);
        assert!(s.any("The broker is already running."));
        s.act(ActionId::StopBroker);
        assert!(!s.app.live_broker_running());
        assert!(s.any("Broker stopped. Every reader is disconnected."));
        // The port is free again.
        std::net::TcpListener::bind(addr).expect("the port is free after Stop");
        s.act(ActionId::StopBroker);
        assert!(s.any("No broker is running."));
        s.app.shutdown();
    }

    #[test]
    fn a_reader_joins_by_code_follows_a_shared_document_and_cannot_publish() {
        let mut s = Side::new();
        s.act(ActionId::StartBroker);
        let port = s.app.live_broker_addrs()[0].port();
        let code = s.app.live_broker_join_code().unwrap();
        let path = s.file("agenda.md", "# Agenda\n\n- Check-ins\n- Traffic\n- Close\n");
        s.app.dispatch(Command::Open(path.clone()));
        s.act(ActionId::ShareDocumentLive);
        assert!(
            s.any("Shared: Agenda, revision 1, on textweaver/document."),
            "{:?}",
            s.texts()
        );
        assert!(
            s.app
                .title_parts(None)
                .iter()
                .any(|p| p.contains("Sharing, revision 1"))
        );

        let mut r = Side::new();
        join_through_the_form(&mut r, port, &code, "textweaver/document");
        until(&mut [&mut s, &mut r], "the shared document", |v| {
            v[1].app.live_shows_document()
        });
        assert!(r.any("Agenda, revision 1, shared live."), "{:?}", r.texts());
        let text = r.doc_text();
        assert!(
            text.contains("Check-ins\nTraffic\nClose\n"),
            "lines kept: {text:?}"
        );
        assert_eq!(r.app.session().unwrap().title, "Agenda");
        assert_eq!(
            r.app.live_position().unwrap()[0],
            "Revision 1",
            "{:?}",
            r.app.live_position()
        );
        // The reader is on Traffic; a new revision keeps them there.
        let at = text.find("Traffic").unwrap();
        let at = textweaver_core::CharPos(text[..at].chars().count());
        r.app.session.as_mut().unwrap().cursor = at;
        let heard = r.texts().len();
        std::fs::write(
            &path,
            "# Agenda\n\nOpening words.\n\n- Check-ins\n- Traffic\n- Close\n",
        )
        .unwrap();
        s.app.live_share_saved(&path);
        until(&mut [&mut s, &mut r], "revision 2", |v| {
            v[1].app.live_document_revision() == Some(2)
        });
        let text = r.doc_text();
        let cursor = r.app.session().unwrap().cursor.0;
        let rest: String = text.chars().skip(cursor).take(7).collect();
        assert_eq!(rest, "Traffic", "the reader's place stays: {text:?}");
        let new: Vec<String> = r.texts()[heard..].to_vec();
        assert_eq!(new, ["Agenda updated, revision 2."], "nothing read again");
        // A join code reads only: a check-in through it is refused.
        r.act(ActionId::SendCheckIn);
        r.type_in("N0CALL");
        r.key(PromptKey::Enter);
        until(&mut [&mut s, &mut r], "the refusal", |v| {
            v[1].any("Check-in not sent")
        });
        assert!(r.any("Refused"), "{:?}", r.texts());
        assert!(s.any("Publish refused: join"), "{:?}", s.texts());
        // Stop sharing: readers keep the last revision.
        s.act(ActionId::StopSharing);
        assert!(s.any("Stopped sharing Agenda. Readers keep revision 2."));
        r.app.shutdown();
        s.app.shutdown();
        assert!(!s.app.live_broker_running());
    }

    #[test]
    fn a_check_in_reaches_every_reader_in_the_forms_order() {
        let mut s = Side::new();
        s.act(ActionId::StartBroker);
        let port = s.app.live_broker_addrs()[0].port();
        let code = s.app.live_broker_join_code().unwrap();
        let mut r = Side::new();
        join_through_the_form(&mut r, port, &code, "netlog/checkins");
        until(&mut [&mut s, &mut r], "the reader", |v| {
            v[0].app.live_broker_words().as_deref() == Some("Broker on, 1 client")
        });
        assert!(s.any("Joined: join."), "{:?}", s.texts());
        s.act(ActionId::SendCheckIn);
        assert!(
            s.any("Check-in to this computer's broker, on netlog/checkins."),
            "{:?}",
            s.texts()
        );
        assert_eq!(s.app.prompt_model().unwrap().label, "Callsign, 1 of 3");
        s.type_in("N0CALL");
        s.key(PromptKey::Tab);
        s.type_in("Ada Example");
        s.key(PromptKey::Enter);
        assert!(
            s.any("Sent: N0CALL, on netlog/checkins."),
            "{:?}",
            s.texts()
        );
        until(&mut [&mut s, &mut r], "the entry", |v| {
            v[1].app.live_log().is_some_and(|l| l.len() == 1)
        });
        let line = r.app.live_newest_line().unwrap();
        assert!(line.starts_with("N0CALL"), "{line}");
        assert!(line.contains("Ada Example"), "{line}");
        r.app.shutdown();
        s.app.shutdown();
    }

    #[test]
    fn nobody_could_sign_in_is_said_and_nothing_starts() {
        let mut s = Side::new();
        let _ = s.app.update_settings(|st| st.live.broker.join_code = false);
        s.act(ActionId::StartBroker);
        assert!(!s.app.live_broker_running());
        assert!(
            s.any("Broker not started: no accounts and no join code"),
            "{:?}",
            s.texts()
        );
        let _ = s
            .app
            .update_settings(|st| st.live.broker.listen = vec!["here".into()]);
        let _ = s.app.update_settings(|st| st.live.broker.join_code = true);
        s.act(ActionId::StartBroker);
        assert!(
            s.any("here is not an address to listen on."),
            "{:?}",
            s.texts()
        );
        // Sharing with nothing to send through says how to get one.
        let path = s.file("notes.txt", "one\ntwo\n");
        s.app.dispatch(Command::Open(path));
        s.act(ActionId::ShareDocumentLive);
        assert!(
            s.any("Nothing to send through. Start a broker"),
            "{:?}",
            s.texts()
        );
        let _ = LiveSourceSettings::default();
        assert_eq!(s.count("Broker on"), 0);
    }

    #[test]
    fn the_join_code_and_link_copy_from_the_menu_the_list_and_the_context_menu() {
        let mut s = Side::new();
        s.act(ActionId::CopyJoinLink);
        assert!(s.any("No broker is running."), "{:?}", s.texts());
        assert_eq!(s.app.take_clipboard(), None);
        s.act(ActionId::StartBroker);
        let addr = s.app.live_broker_addrs()[0];
        let code = s.app.live_broker_join_code().unwrap();
        let heard = s.texts().len();
        s.act(ActionId::CopyJoinCode);
        assert_eq!(s.app.take_clipboard().as_deref(), Some(code.as_str()));
        s.act(ActionId::CopyJoinLink);
        let link = s.app.take_clipboard().unwrap();
        assert_eq!(link, format!("twjoin://{addr}/{code}"));
        let said = s.texts()[heard..].to_vec();
        assert_eq!(said, ["Join code copied.", "Join link copied."]);
        // Sharing into the broker puts the topic in the link.
        let path = s.file("agenda.md", "# Agenda\n");
        s.app.dispatch(Command::Open(path));
        s.act(ActionId::ShareDocumentLive);
        s.act(ActionId::CopyJoinLink);
        assert_eq!(
            s.app.take_clipboard().unwrap(),
            format!("twjoin://{addr}/{code}?topic=textweaver/document")
        );
        // The list's last two rows are the Copy buttons.
        s.act(ActionId::ShowJoinCode);
        let items = s.app.list_model().unwrap().items.clone();
        assert_eq!(items.len(), 7);
        assert_eq!(items[5], "Copy the join code");
        assert_eq!(items[6], "Copy the join link");
        for _ in 0..6 {
            s.app.dispatch(Command::ListKey(ListKey::Down));
        }
        s.app.dispatch(Command::ListKey(ListKey::Enter));
        assert!(s.app.take_clipboard().unwrap().starts_with("twjoin://"));
        assert!(s.app.list_model().is_none(), "the list closes");
        // The context menu offers both while there is a code.
        let context: Vec<ActionId> = s
            .app
            .context_menu()
            .iter()
            .filter_map(|i| match i.kind {
                crate::menu::MenuItemKind::Action(a) => Some(a),
                _ => None,
            })
            .collect();
        assert!(context.contains(&ActionId::CopyJoinLink), "{context:?}");
        assert!(context.contains(&ActionId::CopyJoinCode), "{context:?}");
        // Neither the code nor the link is ever said, or in the status.
        for t in s.texts() {
            assert!(!t.contains("twjoin"), "the link was said: {t}");
        }
        assert!(
            !s.app.title_parts(None).iter().any(|p| p.contains(&code)),
            "{:?}",
            s.app.title_parts(None)
        );
        s.act(ActionId::StopBroker);
        assert!(!s.app.context_menu().iter().any(|i| matches!(
            i.kind,
            crate::menu::MenuItemKind::Action(ActionId::CopyJoinLink)
        )));
        s.app.shutdown();
    }

    #[test]
    fn a_pasted_join_link_fills_the_form_and_enter_joins() {
        let mut s = Side::new();
        s.act(ActionId::StartBroker);
        let path = s.file("agenda.md", "# Agenda\n\n- Traffic\n");
        s.app.dispatch(Command::Open(path));
        s.act(ActionId::ShareDocumentLive);
        s.act(ActionId::CopyJoinLink);
        let link = s.app.take_clipboard().unwrap();
        let port = s.app.live_broker_addrs()[0].port();

        let mut r = Side::new();
        r.act(ActionId::SetUpLiveSource);
        let heard = r.texts().len();
        // Pasted into the name, the first field.
        r.key(PromptKey::Paste(format!("Join link: {link}\n")));
        let new = r.texts()[heard..].to_vec();
        assert!(
            new[0].starts_with("Filled from the join link: host, port, code and topic."),
            "{new:?}"
        );
        for t in r.texts() {
            assert!(!t.contains("twjoin"), "the link was echoed: {t}");
        }
        let model = r.app.prompt_model().unwrap();
        assert!(model.label.starts_with("Name, 1 of 7"), "{}", model.label);
        assert_eq!(model.text(), "127.0.0.1", "the name is the host");
        // Enter saves and connects, with no question first.
        r.key(PromptKey::Enter);
        assert!(r.any("Password kept for 127.0.0.1"), "{:?}", r.texts());
        let src = r
            .app
            .settings()
            .live
            .sources
            .values()
            .next()
            .unwrap()
            .clone();
        assert_eq!(src.port, Some(port).filter(|p| *p != 8883));
        assert_eq!(src.topics, ["textweaver/document"]);
        assert_eq!(src.username, JOIN_USER);
        let toml = r.app.settings().to_minimal_toml().unwrap();
        assert!(!toml.contains("twjoin"), "the link is kept nowhere: {toml}");
        assert!(
            r.app
                .prompt_history(PromptPurpose::LiveSetup)
                .iter()
                .all(|a| !a.contains("twjoin")),
            "no link in the history"
        );
        until(&mut [&mut s, &mut r], "the shared document", |v| {
            v[1].app.live_shows_document()
        });
        r.app.shutdown();
        s.app.shutdown();
    }

    #[test]
    fn the_join_code_field_takes_the_code_out_of_what_is_pasted() {
        let mut r = Side::new();
        r.act(ActionId::SetUpLiveSource);
        r.key(PromptKey::Tab);
        r.key(PromptKey::Tab);
        for _ in 0..4 {
            r.key(PromptKey::Down);
        }
        r.key(PromptKey::Tab);
        assert!(r.app.prompt_model().unwrap().is_masked());
        let heard = r.texts().len();
        // The Show the join code list, as copied: line breaks and labels.
        r.key(PromptKey::Paste(
            "ABCD-EFGH-JKMN-PQRS\nGroup 1 of 4: ABCD\nGroup 2 of 4: EFGH\n".into(),
        ));
        assert_eq!(r.app.prompt_model().unwrap().text(), "ABCD-EFGH-JKMN-PQRS");
        let new = r.texts()[heard..].to_vec();
        assert_eq!(new[0], "Join code pasted.", "{new:?}");
        assert!(
            new.iter().all(|t| !t.contains("ABCD")),
            "never said: {new:?}"
        );
        // A link that is not one is refused in words, and not kept.
        r.key(PromptKey::Paste(
            "twjoin://host:0/ABCD-EFGH-JKMN-PQRS".into(),
        ));
        assert!(
            r.any("Not a join link: the port is not a number from 1 to 65535."),
            "{:?}",
            r.texts()
        );
        assert_eq!(r.app.prompt_model().unwrap().text(), "ABCD-EFGH-JKMN-PQRS");
        // A GUI field sends its whole text: the same.
        r.key(PromptKey::SetText(String::new()));
        r.key(PromptKey::SetText("Join code: abcd efgh jkmn pqrs.".into()));
        assert_eq!(r.app.prompt_model().unwrap().text(), "ABCD-EFGH-JKMN-PQRS");
    }

    #[test]
    fn open_a_join_link_fills_the_form_from_the_clipboard() {
        let mut r = Side::new();
        let clip = crate::clipboard::FakeClipboard::new();
        r.app.set_clipboard(Box::new(clip.clone()));
        r.act(ActionId::OpenJoinLink);
        assert!(
            r.any("Join from a link. Paste the join link"),
            "{:?}",
            r.texts()
        );
        r.key(PromptKey::Escape);
        clip.set(crate::clipboard::ClipboardContents::text(
            "twjoin://10.0.0.5:8884/ABCD-EFGH-JKMN-PQRS",
        ));
        r.act(ActionId::OpenJoinLink);
        assert!(
            r.any("Join from the link on the clipboard."),
            "{:?}",
            r.texts()
        );
        assert!(
            r.any("Filled from the join link: host, port and code. Type the topic."),
            "{:?}",
            r.texts()
        );
        let model = r.app.prompt_model().unwrap();
        assert!(model.label.starts_with("Topic, 6 of 7"), "{}", model.label);
    }

    #[test]
    fn addresses_read_as_tw_broker_reads_them() {
        let a = listen_addrs(&["127.0.0.1".into(), "[::1]:1884".into()], 8883).unwrap();
        assert_eq!(a[0].to_string(), "127.0.0.1:8883");
        assert_eq!(a[1].to_string(), "[::1]:1884");
        assert_eq!(
            listen_addrs(&[], 1883).unwrap()[0].to_string(),
            "127.0.0.1:1883"
        );
        assert_eq!(listen_addrs(&["here".into()], 1).unwrap_err(), "here");
        assert_eq!(
            said_in_groups("ABCD-EFGH-JKMN-PQRS"),
            "ABCD, EFGH, JKMN, PQRS"
        );
    }
}
