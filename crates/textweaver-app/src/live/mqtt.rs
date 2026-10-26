//! The MQTT live source (beta 5, with the `mqtt` feature): textweaver-mqtt's
//! receive-only client adapted to [`LiveSource`].
//!
//! [`MqttLiveSource::start`] connects as `[live.sources.<id>]` says
//! ([`mqtt_config`]), with the password from the system credential store,
//! and turns each [`MqttEvent`] into a [`LiveEvent`] ([`live_event`]). The
//! client already cleans every string and reports each status change once;
//! the app says them (crate::live::hub). A failure only the user can fix
//! (a wrong password, an untrusted certificate, a settings error) arrives
//! as [`LiveEvent::Failed`], after which the client has stopped.

use std::path::PathBuf;
use std::time::Duration;

use textweaver_components::credentials::Password;
use textweaver_mqtt::{MqttConfig, MqttEvent, MqttSource, Payload, Secret, Tls};
use textweaver_store::{LiveSettings, LiveSourceSettings, LiveTls};

use super::KeyOrder;
use super::source::{LiveEvent, LivePayload, LiveSource};

/// What an MQTT live source reads from: the running client, or a test's
/// scripted stand-in.
pub trait MqttFeed: Send {
    /// The events since the last poll, oldest first. Never blocks.
    fn poll(&mut self) -> Vec<MqttEvent>;

    /// Ends the connection and its thread.
    fn stop(&mut self);
}

/// The running client.
struct Running(Option<MqttSource>);

impl MqttFeed for Running {
    fn poll(&mut self) -> Vec<MqttEvent> {
        self.0.as_ref().map(MqttSource::poll).unwrap_or_default()
    }

    fn stop(&mut self) {
        if let Some(s) = self.0.take() {
            s.stop();
        }
    }
}

/// A live source fed by an MQTT broker.
pub struct MqttLiveSource {
    feed: Box<dyn MqttFeed>,
}

impl MqttLiveSource {
    /// Starts the client with `config`; `wake` rings when something
    /// arrives ([`App::live_waker`](crate::App::live_waker)).
    pub fn start(config: MqttConfig, wake: Box<dyn Fn() + Send + Sync>) -> Self {
        let source = MqttSource::start(config, Box::new(wake));
        MqttLiveSource {
            feed: Box::new(Running(Some(source))),
        }
    }

    /// A source reading from `feed` (a test's scripted client).
    pub fn from_feed(feed: Box<dyn MqttFeed>) -> Self {
        MqttLiveSource { feed }
    }
}

impl LiveSource for MqttLiveSource {
    fn poll(&mut self) -> Vec<LiveEvent> {
        self.feed.poll().into_iter().map(live_event).collect()
    }

    fn stop(&mut self) {
        self.feed.stop();
    }
}

/// The app's event for a client's event.
pub fn live_event(event: MqttEvent) -> LiveEvent {
    match event {
        MqttEvent::Connecting { attempt } => LiveEvent::Connecting { attempt },
        MqttEvent::Connected {
            encrypted,
            tailscale,
        } => LiveEvent::Connected {
            encrypted,
            tailscale,
        },
        MqttEvent::Message {
            topic,
            payload,
            retained,
            key_order,
        } => LiveEvent::Message {
            topic,
            payload: match payload {
                // The message's key order, so the views keep it.
                Payload::Json(v) => LivePayload::OrderedJson(v, KeyOrder::from(key_order)),
                Payload::Text(t) => LivePayload::Text(t),
                Payload::Binary { len } => LivePayload::Binary { len },
                Payload::TooLarge { len } => LivePayload::TooLarge { len },
            },
            retained,
        },
        MqttEvent::Disconnected {
            reason,
            retrying: true,
        } => LiveEvent::Disconnected { reason },
        MqttEvent::Disconnected {
            reason,
            retrying: false,
        } => LiveEvent::Failed { reason },
        MqttEvent::Quiet { topic, seconds } => LiveEvent::Quiet { topic, seconds },
        MqttEvent::TopicRefused { topic } => LiveEvent::TopicRefused { topic },
        MqttEvent::Skipped { count } => LiveEvent::Skipped { count },
    }
}

/// The client's configuration for source `source`: host, port (the one
/// set, or the default for its encryption), encryption, user name and
/// `password`, topics, and the quiet hint from `[live]
/// quiet_hint_seconds`. A fresh neutral client id (`tw-` and random hex).
pub fn mqtt_config(
    live: &LiveSettings,
    source: &LiveSourceSettings,
    password: Option<&Password>,
) -> MqttConfig {
    let tls = match source.tls {
        LiveTls::Off => Tls::Off,
        LiveTls::System => Tls::System,
        LiveTls::CaFile => Tls::CaFile(PathBuf::from(source.ca_file.trim())),
    };
    let mut config = MqttConfig::new(source.host.trim(), source.port_or_default(), tls);
    let user = source.username.trim();
    if !user.is_empty() {
        config.username = Some(user.to_owned());
        config.password = password.map(|p| Secret::new(p.reveal()));
    }
    config.topics = source
        .topics
        .iter()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect();
    config.quiet_after = Duration::from_secs(u64::from(live.quiet_hint_seconds));
    config
}

/// A scripted client for tests: the test queues events, the source
/// polls them. No network, no thread.
#[cfg(test)]
pub(crate) mod fake {
    use std::sync::{Arc, Mutex};

    use super::*;

    /// The shared queue; clones share it.
    #[derive(Clone, Default)]
    pub(crate) struct Script(Arc<Mutex<(Vec<MqttEvent>, bool)>>);

    impl Script {
        /// Queues `e` for the next poll.
        pub(crate) fn push(&self, e: MqttEvent) {
            self.0.lock().unwrap_or_else(|e| e.into_inner()).0.push(e);
        }

        /// True once the source was stopped.
        pub(crate) fn stopped(&self) -> bool {
            self.0.lock().unwrap_or_else(|e| e.into_inner()).1
        }
    }

    impl MqttFeed for Script {
        fn poll(&mut self) -> Vec<MqttEvent> {
            std::mem::take(&mut self.0.lock().unwrap_or_else(|e| e.into_inner()).0)
        }

        fn stop(&mut self) {
            self.0.lock().unwrap_or_else(|e| e.into_inner()).1 = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::Script;
    use super::*;

    #[test]
    fn every_client_event_has_its_live_event() {
        let script = Script::default();
        let mut source = MqttLiveSource::from_feed(Box::new(script.clone()));
        let json = serde_json::json!({"call": "N0CALL", "name": "Ada Example"});
        // The sender's order: name before call.
        let order: std::collections::BTreeMap<String, Vec<String>> =
            [(String::new(), vec!["name".to_owned(), "call".to_owned()])].into();
        for e in [
            MqttEvent::Connecting { attempt: 1 },
            MqttEvent::Connected {
                encrypted: false,
                tailscale: true,
            },
            MqttEvent::Message {
                topic: "log/checkins".into(),
                payload: Payload::Json(json.clone()),
                retained: true,
                key_order: order.clone(),
            },
            MqttEvent::Message {
                topic: "log/checkins".into(),
                payload: Payload::Binary { len: 3 },
                retained: false,
                key_order: Default::default(),
            },
            MqttEvent::Disconnected {
                reason: "Connection lost to logger:1883.".into(),
                retrying: true,
            },
            MqttEvent::Disconnected {
                reason: "Not authorized: check the user name and password.".into(),
                retrying: false,
            },
            MqttEvent::Quiet {
                topic: "log/checkins".into(),
                seconds: 60,
            },
            MqttEvent::TopicRefused {
                topic: "log/secret".into(),
            },
            MqttEvent::Skipped { count: 7 },
        ] {
            script.push(e);
        }
        let events = source.poll();
        assert_eq!(
            events,
            [
                LiveEvent::Connecting { attempt: 1 },
                LiveEvent::Connected {
                    encrypted: false,
                    tailscale: true
                },
                LiveEvent::Message {
                    topic: "log/checkins".into(),
                    payload: LivePayload::OrderedJson(json, KeyOrder::from(order)),
                    retained: true
                },
                LiveEvent::Message {
                    topic: "log/checkins".into(),
                    payload: LivePayload::Binary { len: 3 },
                    retained: false
                },
                LiveEvent::Disconnected {
                    reason: "Connection lost to logger:1883.".into()
                },
                LiveEvent::Failed {
                    reason: "Not authorized: check the user name and password.".into()
                },
                LiveEvent::Quiet {
                    topic: "log/checkins".into(),
                    seconds: 60
                },
                LiveEvent::TopicRefused {
                    topic: "log/secret".into()
                },
                LiveEvent::Skipped { count: 7 },
            ]
        );
        assert!(source.poll().is_empty());
        source.stop();
        assert!(script.stopped());
    }

    #[test]
    fn the_config_follows_the_source_settings() {
        let live = LiveSettings {
            quiet_hint_seconds: 30,
            ..LiveSettings::default()
        };
        let mut s = LiveSourceSettings {
            host: " logger.example.ts.net ".into(),
            topics: vec!["log/checkins".into(), " ".into()],
            ..LiveSourceSettings::default()
        };
        let pw = Password::new("example only").unwrap();
        let c = mqtt_config(&live, &s, Some(&pw));
        assert_eq!(c.host, "logger.example.ts.net");
        assert_eq!(c.port, 1883);
        assert_eq!(c.tls, Tls::Off);
        assert_eq!(c.username, None);
        assert!(c.password.is_none(), "no user name, no password");
        assert_eq!(c.topics, ["log/checkins"]);
        assert_eq!(c.quiet_after, Duration::from_secs(30));
        assert!(c.client_id.starts_with("tw-"));

        s.tls = LiveTls::CaFile;
        s.ca_file = "ca.pem".into();
        s.username = "reader".into();
        let c = mqtt_config(&live, &s, Some(&pw));
        assert_eq!(c.port, 8883);
        assert_eq!(c.tls, Tls::CaFile(PathBuf::from("ca.pem")));
        assert_eq!(c.username.as_deref(), Some("reader"));
        assert_eq!(
            c.password.as_ref().map(Secret::expose),
            Some("example only")
        );
        s.port = Some(9001);
        s.tls = LiveTls::System;
        let c = mqtt_config(&live, &s, None);
        assert_eq!((c.port, c.tls), (9001, Tls::System));
        assert!(c.password.is_none());
    }
}
