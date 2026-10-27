//! A small publish client for textweaver's senders (`tw send`, `tw share`,
//! and later the app's check-in form): connect and log in, publish at QoS
//! 1 and wait for the broker's PUBACK, disconnect. It runs on the caller's
//! thread, with no async runtime and no thread of its own, and reuses the
//! receiver's connection, TLS and login code, so its failures read the
//! same. The receiver ([`crate::MqttSource`]) stays receive-only.

use std::fmt;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use bytes::BytesMut;
use rumqttc::mqttbytes::QoS;
use rumqttc::mqttbytes::v4::{Packet, Publish};

use crate::client::{Frame, Link, Stream, kind, open_link, read_packet, send};
use crate::error::{Failure, Stage};
use crate::{MqttConfig, topic};

/// Room for the topic, packet id and header around the payload.
const OVERHEAD: usize = 65_535 + 16;

/// Why a send failed, as one sentence, meaning first ("Unreachable:",
/// "Not authorized:", "TLS failure:", "Refused:", "Too large:", "Settings
/// error:").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublishError(String);

impl PublishError {
    fn new(text: impl Into<String>) -> Self {
        PublishError(crate::clean_text(&text.into(), 2000))
    }

    /// The sentence.
    pub fn text(&self) -> &str {
        &self.0
    }
}

impl From<Failure> for PublishError {
    fn from(f: Failure) -> Self {
        PublishError::new(f.text)
    }
}

impl fmt::Display for PublishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PublishError {}

/// A logged-in connection that publishes. Made by [`Publisher::connect`];
/// [`Publisher::disconnect`] ends it politely, and dropping it closes the
/// connection.
pub struct Publisher {
    stream: Box<dyn Stream>,
    buf: BytesMut,
    config: MqttConfig,
    next_id: u16,
}

impl Publisher {
    /// Connects and logs in with `config` (its topics are not used). Blocks
    /// for at most about [`MqttConfig::connect_timeout`] per step.
    pub fn connect(config: &MqttConfig) -> Result<Publisher, PublishError> {
        if config.host.trim().is_empty() {
            return Err(PublishError::new("Settings error: no host is set."));
        }
        if config.port == 0 {
            return Err(PublishError::new("Settings error: the port is 0."));
        }
        let mut config = config.clone();
        if config.client_id.is_empty() {
            config.client_id = crate::new_client_id();
        }
        let never = AtomicBool::new(false);
        let link = open_link(&config, &never, false)?.ok_or_else(|| {
            PublishError::new("Failed: the connection was stopped before it was made.")
        })?;
        let Link { stream, buf, .. } = link;
        Ok(Publisher {
            stream,
            buf,
            config,
            next_id: 0,
        })
    }

    /// Publishes `payload` on `topic` at QoS 1 and waits for the broker to
    /// acknowledge it. `retain` asks the broker to keep it for readers who
    /// subscribe later. The topic is a name, not a filter: no `+` or `#`.
    pub fn publish(
        &mut self,
        topic_name: &str,
        payload: &[u8],
        retain: bool,
    ) -> Result<(), PublishError> {
        if let Some(why) = topic::name_problem(topic_name) {
            return Err(PublishError::new(format!("Settings error: {why}.")));
        }
        let cap = self.config.limits.max_payload;
        if payload.len() > cap {
            return Err(PublishError::new(format!(
                "Too large: the message is {} bytes; the limit is {cap}.",
                payload.len()
            )));
        }
        self.next_id = self.next_id.checked_add(1).unwrap_or(1);
        let id = self.next_id;
        let mut p = Publish::new(topic_name, QoS::AtLeastOnce, payload.to_vec());
        p.pkid = id;
        p.retain = retain;
        let target = self.config.target();
        send(&mut self.stream, Packet::Publish(p))
            .map_err(|e| PublishError::from(Failure::from_io(&e, Stage::Running, &target)))?;
        let deadline = Instant::now() + self.config.connect_timeout;
        let max = cap.saturating_add(OVERHEAD);
        loop {
            match read_packet(&mut self.stream, &mut self.buf, max) {
                Ok(Some(Frame::Packet(Packet::PubAck(a)))) if a.pkid == id => return Ok(()),
                Ok(Some(Frame::Packet(Packet::PubAck(_) | Packet::PingResp))) => {}
                Ok(Some(Frame::Packet(other))) => {
                    return Err(PublishError::new(format!(
                        "Protocol error: the broker answered with {} instead of an acknowledgment.",
                        kind(&other)
                    )));
                }
                Ok(Some(Frame::Oversize(_))) => {
                    return Err(PublishError::new(
                        "Protocol error: the broker sent a message nobody asked for.",
                    ));
                }
                Ok(None) if Instant::now() < deadline => {}
                Ok(None) => {
                    return Err(PublishError::new(format!(
                        "Unreachable: {target} did not acknowledge the message in time."
                    )));
                }
                Err(e) if is_closed(&e) => {
                    return Err(PublishError::new(format!(
                        "Refused: the broker closed the connection instead of taking the message. This account may not publish to {topic_name}."
                    )));
                }
                Err(e) => {
                    return Err(Failure::from_io(&e, Stage::Running, &target).into());
                }
            }
        }
    }

    /// Says goodbye to the broker and closes the connection.
    pub fn disconnect(mut self) {
        let _ = send(&mut self.stream, Packet::Disconnect);
    }
}

fn is_closed(e: &std::io::Error) -> bool {
    use std::io::ErrorKind as K;
    matches!(
        e.kind(),
        K::UnexpectedEof | K::ConnectionReset | K::ConnectionAborted | K::BrokenPipe
    )
}

/// Connects, publishes one message at QoS 1, and disconnects.
pub fn publish_once(
    config: &MqttConfig,
    topic_name: &str,
    payload: &[u8],
    retain: bool,
) -> Result<(), PublishError> {
    let mut p = Publisher::connect(config)?;
    p.publish(topic_name, payload, retain)?;
    p.disconnect();
    Ok(())
}
