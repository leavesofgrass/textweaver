//! Failures in words. Each text starts with its meaning ("Refused",
//! "Not authorized", "Unreachable", "TLS failure", "Connection lost",
//! "Settings error"), so the first cells of a Braille line say what
//! happened.

use std::fmt;
use std::io;

use rumqttc::mqttbytes::v4::ConnectReturnCode;

/// What kind of failure, which decides whether to retry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// The broker could not be reached: name lookup, refused port, timeout.
    Unreachable,
    /// The connection was made and then lost.
    Lost,
    /// The broker refused the connection for a reason other than the
    /// account (protocol version, client id).
    Refused,
    /// Wrong user name or password, or the account may not connect.
    NotAuthorized,
    /// The broker refused every subscription.
    SubscriptionRefused,
    /// The TLS handshake or certificate check failed.
    Tls,
    /// The broker broke the MQTT protocol.
    Protocol,
    /// The settings cannot work (bad host name, unreadable CA file).
    Config,
}

impl Kind {
    /// Whether retrying could help without the user changing something.
    /// Wrong credentials, untrusted certificates and bad settings will not
    /// fix themselves, and retrying them would lock accounts on some
    /// brokers.
    pub(crate) fn retry_helps(self) -> bool {
        matches!(self, Kind::Unreachable | Kind::Lost | Kind::Protocol)
    }
}

/// A failure and its sentence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Failure {
    pub(crate) kind: Kind,
    pub(crate) text: String,
}

impl Failure {
    pub(crate) fn new(kind: Kind, text: impl Into<String>) -> Self {
        Failure {
            kind,
            text: text.into(),
        }
    }

    /// A CONNACK refusal in words.
    pub(crate) fn from_connack(code: ConnectReturnCode) -> Self {
        use ConnectReturnCode as C;
        match code {
            C::BadUserNamePassword => Failure::new(
                Kind::NotAuthorized,
                "Not authorized: the broker refused the user name or password.",
            ),
            C::NotAuthorized => Failure::new(
                Kind::NotAuthorized,
                // Mosquitto 2 sends this code (5) for a wrong password too.
                "Not authorized: the broker refused the login. Check the user name and password.",
            ),
            C::RefusedProtocolVersion => Failure::new(
                Kind::Refused,
                "Refused: the broker does not accept MQTT 3.1.1.",
            ),
            C::BadClientId => {
                Failure::new(Kind::Refused, "Refused: the broker rejected the client id.")
            }
            C::ServiceUnavailable => Failure::new(
                Kind::Unreachable,
                "Unreachable: the broker says it is unavailable right now.",
            ),
            C::Success => Failure::new(Kind::Protocol, "Protocol error: unexpected success code."),
        }
    }

    /// An I/O error, worded for the user. A rustls error inside it becomes
    /// a TLS failure.
    pub(crate) fn from_io(err: &io::Error, during: Stage, target: &str) -> Self {
        if let Some(tls) = err
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<rustls::Error>())
        {
            return Failure::new(Kind::Tls, crate::tls::describe(tls));
        }
        if err
            .get_ref()
            .is_some_and(|inner| inner.downcast_ref::<TlsExpected>().is_some())
        {
            return Failure::new(
                Kind::Tls,
                format!("TLS needed: {target} answered with TLS. Turn TLS on for this source."),
            );
        }
        let why = match err.kind() {
            io::ErrorKind::ConnectionRefused => "nothing is listening on that port".to_string(),
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => "no answer in time".to_string(),
            io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted => {
                "the connection was reset".to_string()
            }
            io::ErrorKind::UnexpectedEof => "the broker closed the connection".to_string(),
            io::ErrorKind::HostUnreachable | io::ErrorKind::NetworkUnreachable => {
                "no route to the host".to_string()
            }
            _ => crate::sanitize::clean_text(&err.to_string(), 300),
        };
        match during {
            Stage::Connect => Failure::new(
                Kind::Unreachable,
                format!("Unreachable: cannot connect to {target}: {why}."),
            ),
            Stage::Handshake => Failure::new(
                Kind::Tls,
                format!("TLS failure: the handshake with {target} did not finish: {why}."),
            ),
            Stage::Login => Failure::new(
                Kind::Lost,
                format!("Connection lost while logging in to {target}: {why}."),
            ),
            Stage::Running => {
                Failure::new(Kind::Lost, format!("Connection lost to {target}: {why}."))
            }
        }
    }
}

/// The broker answered a plain CONNECT with a TLS record.
#[derive(Debug)]
pub(crate) struct TlsExpected;

impl fmt::Display for TlsExpected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the broker answered with TLS")
    }
}

impl std::error::Error for TlsExpected {}

/// Where in the session an I/O error happened, which decides its words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    Connect,
    Handshake,
    Login,
    Running,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mosquitto_code_5_says_check_the_password() {
        let f = Failure::from_connack(ConnectReturnCode::NotAuthorized);
        assert_eq!(f.kind, Kind::NotAuthorized);
        assert!(!f.kind.retry_helps());
        assert_eq!(
            f.text,
            "Not authorized: the broker refused the login. Check the user name and password."
        );
    }

    #[test]
    fn io_errors_in_words() {
        let e = io::Error::from(io::ErrorKind::ConnectionRefused);
        let f = Failure::from_io(&e, Stage::Connect, "host:1883");
        assert_eq!(
            f.text,
            "Unreachable: cannot connect to host:1883: nothing is listening on that port."
        );
        assert!(f.kind.retry_helps());
        let e = io::Error::other(TlsExpected);
        let f = Failure::from_io(&e, Stage::Login, "host:8883");
        assert_eq!(f.kind, Kind::Tls);
        assert!(f.text.starts_with("TLS needed: host:8883"));
    }
}
