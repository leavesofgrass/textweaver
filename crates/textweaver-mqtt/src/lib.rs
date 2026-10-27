//! A receive-only MQTT 3.1.1 client for textweaver's live documents.
//!
//! [`MqttSource::start`] runs one background thread that connects to a
//! broker, logs in, subscribes, and turns what arrives into
//! [`MqttEvent`]s. The app drains them with [`MqttSource::poll`], which
//! never blocks, after the `wake` callback tells it something is waiting.
//! [`MqttSource::stop`] ends the thread.
//!
//! - **Receive only.** The only packets the client writes are CONNECT,
//!   SUBSCRIBE, PUBACK, PINGREQ and DISCONNECT. It never publishes.
//! - **No async runtime.** rumqttc supplies only its MQTT 3.1.1 packet
//!   codec (`rumqttc::mqttbytes::v4`); the networking is plain std on one
//!   thread, with a 200 ms read tick for the stop flag and the keepalive.
//! - **TLS** is textweaver's own rustls with the ring provider, trusting
//!   the bundled Mozilla roots ([`Tls::System`]) or a CA file the user
//!   names ([`Tls::CaFile`]). There is no way to skip certificate checks.
//!   Plain TCP ([`Tls::Off`]) is for a broker on the same machine or
//!   across an encrypted tunnel such as Tailscale.
//! - **Reconnects** with backoff (1 s doubling to 60 s, plus up to a
//!   quarter at random) and subscribes again on every CONNACK, since the
//!   session is clean. Failures that only the user can fix (a wrong
//!   password, an untrusted certificate, a settings error) stop the
//!   retries, so a wrong password never hammers a broker.
//! - **Status changes are reported once**, not once per attempt:
//!   [`MqttEvent::Connecting`] when the source starts,
//!   [`MqttEvent::Connected`] on each successful login, and
//!   [`MqttEvent::Disconnected`] when the link is lost or a new reason
//!   keeps it down. Every reason is a sentence in words, meaning first, so
//!   the first cells of a Braille line say what happened.
//! - **Quiet topics.** Mosquitto grants a subscription its ACL denies and
//!   then sends nothing, so textweaver cannot tell "not allowed" from
//!   "nothing published yet". After [`MqttConfig::quiet_after`] with no
//!   message on a subscribed topic, [`MqttEvent::Quiet`] says so once.
//! - **Hostile input is data.** Payloads are capped in size, JSON nesting
//!   depth and field count ([`Limits`]), and every string, key and topic
//!   is cleaned of control characters, terminal escape sequences and
//!   bidirectional overrides ([`clean_text`]) before it leaves the crate.
//! - **The client id** is `tw-` and 16 random hex digits
//!   ([`new_client_id`]), new for each start: never a personal identifier.
//!
//! - **Pinned TLS and join codes.** [`Tls::Pinned`] trusts exactly one
//!   public key by its [`Pin`], with no certificate authority, as
//!   textweaver's own broker (`textweaver-broker`) expects; a
//!   [`JoinCode`] carries a short pin and the secret its password is
//!   derived from.
//! - **Sending** is a separate, small client, [`Publisher`]: connect, publish
//!   at QoS 1 and wait for the acknowledgment, disconnect. The receiver
//!   never publishes.
//!
//! The crate depends on no other textweaver crate. Turning JSON into log
//! entries or values is the app's job.

mod client;
mod error;
mod net;
mod pin;
mod publish;
mod sanitize;
mod tls;
pub mod topic;

use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

pub use client::MqttSource;
pub use pin::{
    FULL_PIN_CHARS, JOIN_CODE_CHARS, JOIN_SECRET_CHARS, JOIN_USER, JoinCode, Pin, SHORT_PIN_CHARS,
    ct_eq,
};
pub use publish::{PublishError, Publisher, publish_once};
pub use sanitize::{KeyOrder, Limits, clean_text, decode_payload, decode_payload_ordered};
pub use tls::{is_tailscale_addr, is_tailscale_host};

/// Everything a live source needs to connect. The password is held only in
/// memory: the app reads it from the credential store and never writes it
/// anywhere.
#[derive(Clone, Debug)]
pub struct MqttConfig {
    /// The broker's host name or IP address.
    pub host: String,
    /// The broker's port: 1883 for plain MQTT, 8883 for TLS, by convention
    /// ([`default_port`]).
    pub port: u16,
    /// How the connection is protected.
    pub tls: Tls,
    /// The user name to log in with, if the broker asks for one.
    pub username: Option<String>,
    /// The password, hidden from `Debug` and `Display`.
    pub password: Option<Secret>,
    /// Topic filters to subscribe to (`+` and `#` wildcards allowed), each
    /// asked for at QoS 1.
    pub topics: Vec<String>,
    /// The MQTT client id. [`MqttConfig::new`] makes a fresh `tw-` id; an
    /// empty id is replaced by one when the source starts.
    pub client_id: String,
    /// How often the client proves it is alive when nothing else is sent
    /// (30 s).
    pub keep_alive: Duration,
    /// How long to wait for the connection, the TLS handshake and the
    /// login (10 s).
    pub connect_timeout: Duration,
    /// After this long with no message on a subscribed topic, report
    /// [`MqttEvent::Quiet`] once (60 s). Zero turns the hint off.
    pub quiet_after: Duration,
    /// The first retry delay (1 s); it doubles up to `max_backoff`.
    pub min_backoff: Duration,
    /// The longest retry delay (60 s).
    pub max_backoff: Duration,
    /// Caps on payload size, JSON depth, fields and string length.
    pub limits: Limits,
    /// Events held for the reader before new messages are counted as
    /// skipped instead of blocking the connection (4096).
    pub queue: usize,
}

impl MqttConfig {
    /// A configuration with the defaults above, no login, no topics, and a
    /// fresh client id.
    pub fn new(host: impl Into<String>, port: u16, tls: Tls) -> Self {
        MqttConfig {
            host: host.into(),
            port,
            tls,
            username: None,
            password: None,
            topics: Vec::new(),
            client_id: new_client_id(),
            keep_alive: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            quiet_after: Duration::from_secs(60),
            min_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(60),
            limits: Limits::default(),
            queue: 4096,
        }
    }

    /// `host:port`, with brackets around an IPv6 address, as the failure
    /// texts name the broker.
    pub fn target(&self) -> String {
        if self.host.contains(':') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
}

/// The conventional port for a protection: 1883 plain, 8883 with TLS.
pub fn default_port(tls: &Tls) -> u16 {
    match tls {
        Tls::Off => 1883,
        Tls::System | Tls::CaFile(_) | Tls::Pinned(_) => 8883,
    }
}

/// How the connection is protected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tls {
    /// Plain TCP. Only for a broker on this machine or across an encrypted
    /// tunnel such as Tailscale: anything else is readable on the way.
    Off,
    /// TLS, checked against the Mozilla root certificates bundled with
    /// textweaver (webpki-roots), as a public broker's certificate would be.
    System,
    /// TLS, checked against the certificates in this PEM file only: a
    /// private CA, or a broker's self-signed certificate.
    CaFile(PathBuf),
    /// TLS, trusting exactly the public key with this pin and nothing
    /// else, with no certificate authority: textweaver's own broker prints
    /// its pin, and a join code carries a short one. Names and dates in
    /// the certificate are not checked; the key is.
    Pinned(Pin),
}

impl Tls {
    /// Whether this is one of the TLS settings.
    pub fn is_on(&self) -> bool {
        !matches!(self, Tls::Off)
    }
}

/// A password, kept out of logs: `Debug` and `Display` print `(hidden)`.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// Wraps a password.
    pub fn new(value: impl Into<String>) -> Self {
        Secret(value.into())
    }

    /// The password itself, for the login packet only.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl From<String> for Secret {
    fn from(value: String) -> Self {
        Secret(value)
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret((hidden))")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("(hidden)")
    }
}

/// What the client reports. Every string in an event has been cleaned
/// with [`clean_text`].
#[derive(Clone, Debug, PartialEq)]
pub enum MqttEvent {
    /// The source started and is making its first connection. Reported
    /// once per start; retries after a failure are not reported again.
    Connecting {
        /// The attempt this connecting phase began with (1 for a start).
        attempt: u32,
    },
    /// Logged in. Subscriptions follow at once.
    Connected {
        /// The link is TLS.
        encrypted: bool,
        /// The broker's host or address is on a Tailscale network
        /// (100.64.0.0/10, fd7a:115c:a1e0::/48, or a name ending in
        /// `.ts.net`), whose WireGuard tunnel encrypts a plain link. A hint
        /// only: 100.64.0.0/10 is also the carrier-grade NAT range.
        tailscale: bool,
    },
    /// A message on a subscribed topic.
    Message {
        /// The topic it was published on.
        topic: String,
        /// The payload, decoded and cleaned.
        payload: Payload,
        /// The broker's retained copy, sent on subscribing: a retained
        /// whole list should replace the view.
        retained: bool,
        /// For a JSON payload, the order its keys came in ([`KeyOrder`]),
        /// so a view can keep the message's order; empty otherwise.
        key_order: KeyOrder,
    },
    /// The link was lost or could not be made, or the reason it stays down
    /// changed. Reported once per change, not once per retry.
    Disconnected {
        /// Why, as a sentence in words, meaning first ("Unreachable:",
        /// "Not authorized:", "TLS failure:", "Connection lost", "Settings
        /// error:").
        reason: String,
        /// The client keeps retrying with backoff. False when only the user
        /// can fix it (credentials, certificate, settings): the thread has
        /// stopped.
        retrying: bool,
    },
    /// No message on a subscribed topic for `seconds` since subscribing or
    /// since its last message. Reported once until a message arrives. The
    /// broker may be denying the topic without saying so.
    Quiet {
        /// The topic filter as configured.
        topic: String,
        /// How long it has been quiet.
        seconds: u64,
    },
    /// The broker refused one topic filter in its SUBACK. If it refuses
    /// every one, a [`MqttEvent::Disconnected`] follows and the client
    /// stops.
    TopicRefused {
        /// The topic filter as configured.
        topic: String,
    },
    /// Messages dropped because the reader fell more than
    /// [`MqttConfig::queue`] events behind. The connection stayed up.
    Skipped {
        /// How many.
        count: u64,
    },
}

/// A message's payload, decoded inside the crate.
#[derive(Clone, Debug, PartialEq)]
pub enum Payload {
    /// Valid JSON within the caps, every string and key cleaned.
    Json(serde_json::Value),
    /// UTF-8 text that is not JSON, cleaned and cut to the string cap.
    Text(String),
    /// Not UTF-8; not shown.
    Binary {
        /// Its size in bytes.
        len: usize,
    },
    /// Over a cap: [`Limits::max_payload`] bytes (skipped as it streamed
    /// in, never held in memory), or JSON nested deeper than
    /// [`Limits::max_depth`] or with more than [`Limits::max_fields`]
    /// values.
    TooLarge {
        /// Its size in bytes.
        len: usize,
    },
}

/// `tw-` and 16 random hex digits from the system's secure random source:
/// never a personal identifier, and new for each call.
pub fn new_client_id() -> String {
    use ring::rand::{SecureRandom, SystemRandom};
    let mut bytes = [0u8; 8];
    if SystemRandom::new().fill(&mut bytes).is_err() {
        // No system randomness: fall back to the clock, still nothing
        // personal.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        bytes = nanos.to_le_bytes();
    }
    let mut id = String::from("tw-");
    for b in bytes {
        id.push_str(&format!("{b:02x}"));
    }
    id
}

/// Runs the packet reader and the payload decoder over arbitrary bytes, as
/// if a broker had sent them, then the certificate key reader and the join
/// code and pin parsers, for the fuzz target. Not part of the API.
#[doc(hidden)]
pub fn fuzz_decode(data: &[u8]) {
    client::fuzz_read(data);
    let limits = Limits {
        max_payload: 4096,
        max_depth: 4,
        max_fields: 64,
        max_string: 64,
    };
    let _ = decode_payload(data, &limits);
    // A certificate's public key, and a typed join code or pin.
    let _ = pin::spki_of_certificate(data);
    let text = String::from_utf8_lossy(data);
    let _ = JoinCode::parse(&text);
    let _ = Pin::parse(&text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_id_is_neutral() {
        let id = new_client_id();
        assert_eq!(id.len(), 19);
        assert!(id.starts_with("tw-"));
        assert!(id[3..].chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(id, new_client_id());
    }

    #[test]
    fn secret_is_hidden() {
        let s = Secret::new("hunter2");
        assert_eq!(format!("{s}"), "(hidden)");
        assert!(!format!("{s:?}").contains("hunter2"));
        let mut c = MqttConfig::new("h", 1883, Tls::Off);
        c.password = Some(s.clone());
        assert!(!format!("{c:?}").contains("hunter2"));
        assert_eq!(s.expose(), "hunter2");
    }

    #[test]
    fn ports_and_targets() {
        assert_eq!(default_port(&Tls::Off), 1883);
        assert_eq!(default_port(&Tls::System), 8883);
        assert_eq!(
            MqttConfig::new("::1", 1883, Tls::Off).target(),
            "[::1]:1883"
        );
        assert_eq!(
            MqttConfig::new("logger", 8883, Tls::System).target(),
            "logger:8883"
        );
    }
}
