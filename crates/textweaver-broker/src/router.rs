//! Who is connected, what each one subscribed to, and the retained
//! messages. One lock; publishing copies the message's bytes once and hands
//! each matching reader a cheap clone through its own bounded queue.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError};

use bytes::Bytes;
use rumqttc::mqttbytes::QoS;
use textweaver_mqtt::topic::matches;

use crate::Perms;

/// A message on its way to one client.
#[derive(Clone, Debug)]
pub(crate) struct Outgoing {
    pub(crate) topic: Arc<str>,
    pub(crate) payload: Bytes,
    pub(crate) qos: QoS,
    pub(crate) retain: bool,
}

/// One connected client.
pub(crate) struct Client {
    pub(crate) client_id: String,
    pub(crate) user: String,
    pub(crate) perms: Arc<Perms>,
    pub(crate) subs: Vec<(String, QoS)>,
    pub(crate) tx: SyncSender<Outgoing>,
    /// Set to end the client's connection: a takeover, or a full queue.
    pub(crate) kill: Arc<AtomicBool>,
}

struct Retained {
    payload: Bytes,
    qos: QoS,
}

/// What a publish did beyond delivering.
#[derive(Debug, Default)]
pub(crate) struct Outcome {
    /// Users whose queue was full; their connections are being ended.
    pub(crate) dropped: Vec<String>,
    /// The retained store was full, so this message was not kept.
    pub(crate) not_retained: bool,
}

#[derive(Default)]
pub(crate) struct Router {
    clients: HashMap<u64, Client>,
    retained: BTreeMap<String, Retained>,
}

impl Router {
    /// Adds a client. A client already using its client id is ended when
    /// it is the same user's (an MQTT takeover); another user's client id
    /// is refused, so one account cannot push another off.
    pub(crate) fn add(&mut self, id: u64, client: Client) -> Result<(), Client> {
        if let Some(old) = self
            .clients
            .values()
            .find(|c| c.client_id == client.client_id)
        {
            if old.user != client.user {
                return Err(client);
            }
            old.kill.store(true, Ordering::SeqCst);
        }
        self.clients.insert(id, client);
        Ok(())
    }

    pub(crate) fn remove(&mut self, id: u64) {
        self.clients.remove(&id);
    }

    pub(crate) fn count(&self) -> usize {
        self.clients.len()
    }

    /// Records a subscription, replacing one to the same filter.
    pub(crate) fn subscribe(&mut self, id: u64, filter: &str, qos: QoS) {
        if let Some(c) = self.clients.get_mut(&id) {
            match c.subs.iter_mut().find(|(f, _)| f == filter) {
                Some(s) => s.1 = qos,
                None => c.subs.push((filter.to_string(), qos)),
            }
        }
    }

    pub(crate) fn unsubscribe(&mut self, id: u64, filter: &str) {
        if let Some(c) = self.clients.get_mut(&id) {
            c.subs.retain(|(f, _)| f != filter);
        }
    }

    pub(crate) fn subscriptions(&self, id: u64) -> usize {
        self.clients.get(&id).map_or(0, |c| c.subs.len())
    }

    /// The retained messages a new subscription to `filter` should get,
    /// that the client may read, at no more than `qos`.
    pub(crate) fn retained_for(&self, filter: &str, qos: QoS, perms: &Perms) -> Vec<Outgoing> {
        self.retained
            .iter()
            .filter(|(topic, _)| matches(filter, topic) && perms.can_read(topic))
            .map(|(topic, r)| Outgoing {
                topic: Arc::from(topic.as_str()),
                payload: r.payload.clone(),
                qos: min_qos(qos, r.qos),
                retain: true,
            })
            .collect()
    }

    /// Delivers a message to every client subscribed to a matching filter
    /// that may read the topic, and keeps or clears the retained copy. An
    /// empty retained payload clears the topic's retained message, as MQTT
    /// says.
    pub(crate) fn publish(
        &mut self,
        topic: &str,
        payload: Bytes,
        qos: QoS,
        retain: bool,
        max_retained: usize,
    ) -> Outcome {
        let mut outcome = Outcome::default();
        if retain {
            if payload.is_empty() {
                self.retained.remove(topic);
            } else if self.retained.len() < max_retained || self.retained.contains_key(topic) {
                self.retained.insert(
                    topic.to_string(),
                    Retained {
                        payload: payload.clone(),
                        qos,
                    },
                );
            } else {
                outcome.not_retained = true;
            }
        }
        let shared: Arc<str> = Arc::from(topic);
        for c in self.clients.values() {
            if c.kill.load(Ordering::SeqCst) || !c.perms.can_read(topic) {
                continue;
            }
            let Some(best) = c
                .subs
                .iter()
                .filter(|(f, _)| matches(f, topic))
                .map(|(_, q)| *q)
                .max_by_key(|q| *q as u8)
            else {
                continue;
            };
            let out = Outgoing {
                topic: shared.clone(),
                payload: payload.clone(),
                qos: min_qos(best, qos),
                retain: false,
            };
            match c.tx.try_send(out) {
                Ok(()) | Err(TrySendError::Disconnected(_)) => {}
                Err(TrySendError::Full(_)) => {
                    c.kill.store(true, Ordering::SeqCst);
                    outcome.dropped.push(c.user.clone());
                }
            }
        }
        outcome
    }
}

pub(crate) fn min_qos(a: QoS, b: QoS) -> QoS {
    if (a as u8) <= (b as u8) { a } else { b }
}
