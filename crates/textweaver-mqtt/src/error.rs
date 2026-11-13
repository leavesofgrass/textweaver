//! Failures, each with its kind and its [`DisconnectReason`]. Each
//! reason's sentence starts with its meaning ("Refused", "Not
//! authorized", "Unreachable", "TLS failure", "Connection lost",
//! "Settings error"), so the first cells of a Braille line say what
//! happened.

use std::fmt;
use std::io;

use rumqttc::mqttbytes::v4::ConnectReturnCode;

use crate::reason::{DisconnectReason, IoCause};

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

/// A failure and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Failure {
    pub(crate) kind: Kind,
    pub(crate) reason: DisconnectReason,
}

impl Failure {
    pub(crate) fn new(kind: Kind, reason: DisconnectReason) -> Self {
        Failure { kind, reason }
    }

    /// A CONNACK refusal in words.
    pub(crate) fn from_connack(code: ConnectReturnCode) -> Self {
        use ConnectReturnCode as C;
        use DisconnectReason as R;
        match code {
            C::BadUserNamePassword => Failure::new(Kind::NotAuthorized, R::BadUserNameOrPassword),
            // Mosquitto 2 sends this code (5) for a wrong password too.
            C::NotAuthorized => Failure::new(Kind::NotAuthorized, R::LoginRefused),
            C::RefusedProtocolVersion => Failure::new(Kind::Refused, R::ProtocolVersion),
            C::BadClientId => Failure::new(Kind::Refused, R::ClientIdRejected),
            C::ServiceUnavailable => Failure::new(Kind::Unreachable, R::BrokerUnavailable),
            C::Success => Failure::new(Kind::Protocol, R::UnexpectedSuccess),
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
                DisconnectReason::TlsNeeded {
                    target: target.to_owned(),
                },
            );
        }
        let cause = match err.kind() {
            io::ErrorKind::ConnectionRefused => IoCause::NothingListening,
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => IoCause::NoAnswer,
            io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted => IoCause::Reset,
            io::ErrorKind::UnexpectedEof => IoCause::Closed,
            io::ErrorKind::HostUnreachable | io::ErrorKind::NetworkUnreachable => IoCause::NoRoute,
            _ => IoCause::Other {
                error: crate::sanitize::clean_text(&err.to_string(), 300),
            },
        };
        let target = target.to_owned();
        match during {
            Stage::Connect => Failure::new(
                Kind::Unreachable,
                DisconnectReason::CannotConnect { target, cause },
            ),
            Stage::Handshake => {
                Failure::new(Kind::Tls, DisconnectReason::Handshake { target, cause })
            }
            Stage::Login => Failure::new(
                Kind::Lost,
                DisconnectReason::LostLoggingIn {
                    target,
                    cause,
                    tls_hint: false,
                },
            ),
            Stage::Running => Failure::new(Kind::Lost, DisconnectReason::Lost { target, cause }),
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
            f.reason.to_string(),
            "Not authorized: the broker refused the login. Check the user name and password."
        );
    }

    #[test]
    fn io_errors_in_words() {
        let e = io::Error::from(io::ErrorKind::ConnectionRefused);
        let f = Failure::from_io(&e, Stage::Connect, "host:1883");
        assert_eq!(
            f.reason.to_string(),
            "Unreachable: cannot connect to host:1883: nothing is listening on that port."
        );
        assert!(f.kind.retry_helps());
        let e = io::Error::other(TlsExpected);
        let f = Failure::from_io(&e, Stage::Login, "host:8883");
        assert_eq!(f.kind, Kind::Tls);
        assert!(f.reason.to_string().starts_with("TLS needed: host:8883"));
    }
}
