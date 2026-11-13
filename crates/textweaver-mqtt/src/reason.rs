//! Why a live source is down, as data rather than a sentence, so the app
//! can say it in the reader's language.
//!
//! [`DisconnectReason`] has one variant for each failure the client
//! reports. Each carries only what is not words: a host, a port, a path,
//! a packet name, or the system's own error text. Its `Display` is the
//! English sentence, meaning first, for logs and the command line; the app
//! says the same sentence from its message catalog, with
//! [`DisconnectReason::message_id`] and [`DisconnectReason::args`].

use std::fmt;

use crate::sanitize::clean_text;
use crate::topic::TopicProblem;

/// A value to put into a message from the app's catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageArg {
    /// Data shown as it is: a host, a path, the system's error text.
    Text(String),
    /// A whole number.
    Number(i64),
    /// Another message from the catalog, with its own values, said first
    /// and put in as text.
    Message(&'static str, Vec<(&'static str, MessageArg)>),
}

impl MessageArg {
    fn text(s: &str) -> MessageArg {
        MessageArg::Text(s.to_owned())
    }
}

/// Why a connection step failed at the network level: the words after
/// "cannot connect to HOST:PORT:".
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IoCause {
    /// The port refused the connection.
    NothingListening,
    /// The time ran out.
    NoAnswer,
    /// The connection was reset or aborted.
    Reset,
    /// The broker closed the connection.
    Closed,
    /// No route to the host or its network.
    NoRoute,
    /// The name lookup ended without an answer.
    LookupFailed,
    /// Anything else, in the system's words (cleaned).
    Other {
        /// The system's error text.
        error: String,
    },
}

impl IoCause {
    /// The words, lower case, to follow a lead-in.
    pub fn text(&self) -> &str {
        match self {
            IoCause::NothingListening => "nothing is listening on that port",
            IoCause::NoAnswer => "no answer in time",
            IoCause::Reset => "the connection was reset",
            IoCause::Closed => "the broker closed the connection",
            IoCause::NoRoute => "no route to the host",
            IoCause::LookupFailed => "the lookup failed",
            IoCause::Other { error } => error,
        }
    }

    /// The cause as a value for a message: the catalog's words, or the
    /// system's text as it is.
    pub fn arg(&self) -> MessageArg {
        let id = match self {
            IoCause::NothingListening => "live-why-nothing-listening",
            IoCause::NoAnswer => "live-why-no-answer",
            IoCause::Reset => "live-why-reset",
            IoCause::Closed => "live-why-closed",
            IoCause::NoRoute => "live-why-no-route",
            IoCause::LookupFailed => "live-why-lookup-failed",
            IoCause::Other { error } => return MessageArg::text(error),
        };
        MessageArg::Message(id, Vec::new())
    }

    fn cleaned(self) -> IoCause {
        match self {
            IoCause::Other { error } => IoCause::Other {
                error: clean_text(&error, 300),
            },
            other => other,
        }
    }
}

/// Why a live source is down. `Display` is the English sentence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DisconnectReason {
    /// CONNACK code 4.
    BadUserNameOrPassword,
    /// CONNACK code 5, which Mosquitto 2 also sends for a wrong password.
    LoginRefused,
    /// CONNACK code 1.
    ProtocolVersion,
    /// CONNACK code 2.
    ClientIdRejected,
    /// CONNACK code 3.
    BrokerUnavailable,
    /// A CONNACK that says success where a failure was expected.
    UnexpectedSuccess,
    /// A plain connection was answered with TLS.
    TlsNeeded {
        /// `host:port`.
        target: String,
    },
    /// The connection could not be made.
    CannotConnect {
        /// `host:port`.
        target: String,
        /// Why.
        cause: IoCause,
    },
    /// The connection could not be made, and no address said why.
    ConnectFailed {
        /// `host:port`.
        target: String,
    },
    /// The TLS handshake did not finish.
    Handshake {
        /// `host:port`.
        target: String,
        /// Why.
        cause: IoCause,
    },
    /// The connection was lost during the login.
    LostLoggingIn {
        /// `host:port`.
        target: String,
        /// Why.
        cause: IoCause,
        /// Add that the port may expect TLS: a plain login was cut off.
        tls_hint: bool,
    },
    /// The connection was lost.
    Lost {
        /// `host:port`.
        target: String,
        /// Why.
        cause: IoCause,
    },
    /// The broker sent DISCONNECT.
    SessionEnded {
        /// `host:port`.
        target: String,
    },
    /// The broker stopped answering pings.
    StoppedAnswering {
        /// `host:port`.
        target: String,
    },
    /// The broker accepted the connection and never answered the login.
    LoginTimeout {
        /// `host:port`.
        target: String,
    },
    /// The host name has no address.
    NoAddress {
        /// The host.
        host: String,
    },
    /// The host name could not be looked up.
    HostNotFound {
        /// The host.
        host: String,
        /// Why.
        cause: IoCause,
    },
    /// The client's thread could not start.
    ThreadFailed {
        /// The system's error text.
        error: String,
    },
    /// The broker refused every topic.
    EveryTopicRefused,
    /// The broker sent a QoS 2 message.
    UnaskedQos2,
    /// The broker sent a packet the client never expects.
    UnexpectedPacket {
        /// The packet's MQTT name, such as `PUBREC`.
        packet: String,
    },
    /// The broker answered the login with something other than CONNACK.
    NoConnack,
    /// No host is set.
    NoHost,
    /// The port is 0.
    PortZero,
    /// No topic is set.
    NoTopic,
    /// A topic filter cannot be subscribed to.
    BadTopic {
        /// What is wrong with it.
        problem: TopicProblem,
        /// The filter, cleaned.
        topic: String,
    },
    /// A password is set with no user name.
    PasswordWithoutUser,
    /// TLS settings on a plain connection.
    TlsOnPlain,
    /// The CA file cannot be read.
    CaUnreadable {
        /// The file, as shown.
        path: String,
        /// The system's error text.
        error: String,
    },
    /// The CA file is not PEM.
    CaNotPem {
        /// The file, as shown.
        path: String,
        /// The parser's error text.
        error: String,
    },
    /// A certificate in the CA file was refused.
    CaCertificateRefused {
        /// The file, as shown.
        path: String,
        /// The TLS library's error text.
        error: String,
    },
    /// The CA file holds no certificate.
    CaEmpty {
        /// The file, as shown.
        path: String,
    },
    /// The host cannot be a TLS server name.
    BadTlsHost {
        /// The host.
        host: String,
    },
    /// The certificate's issuer is not trusted.
    UnknownIssuer,
    /// The certificate is for another host name.
    WrongHostName,
    /// The certificate has expired.
    Expired,
    /// The certificate is not valid yet.
    NotValidYet,
    /// The certificate was revoked.
    Revoked,
    /// The broker's key does not match the pin.
    PinMismatch,
    /// The certificate was refused for another reason.
    CertificateRefused {
        /// The TLS library's name for the reason.
        detail: String,
    },
    /// The broker sent a TLS alert.
    HandshakeAlert {
        /// The alert's name.
        detail: String,
    },
    /// The broker sent no certificate.
    NoCertificate,
    /// The broker's TLS settings do not match textweaver's.
    Incompatible {
        /// The TLS library's name for the mismatch.
        detail: String,
    },
    /// The broker's answer was not TLS.
    NotTls {
        /// The TLS library's name for the message.
        detail: String,
    },
    /// Any other TLS failure.
    Tls {
        /// The TLS library's error text.
        error: String,
    },
}

/// The words a "not answered with TLS" hint adds.
const TLS_HINT: &str = " If this port expects TLS, turn TLS on for this source.";

impl DisconnectReason {
    /// The id of this reason's sentence in the app's message catalog.
    pub fn message_id(&self) -> &'static str {
        use DisconnectReason as R;
        match self {
            R::BadUserNameOrPassword => "live-down-bad-login",
            R::LoginRefused => "live-down-login-refused",
            R::ProtocolVersion => "live-down-protocol-version",
            R::ClientIdRejected => "live-down-client-id",
            R::BrokerUnavailable => "live-down-unavailable",
            R::UnexpectedSuccess => "live-down-unexpected-success",
            R::TlsNeeded { .. } => "live-down-tls-needed",
            R::CannotConnect { .. } => "live-down-cannot-connect",
            R::ConnectFailed { .. } => "live-down-connect-failed",
            R::Handshake { .. } => "live-down-handshake",
            R::LostLoggingIn {
                tls_hint: false, ..
            } => "live-down-lost-logging-in",
            R::LostLoggingIn { tls_hint: true, .. } => "live-down-lost-logging-in-tls",
            R::Lost { .. } => "live-down-lost",
            R::SessionEnded { .. } => "live-down-session-ended",
            R::StoppedAnswering { .. } => "live-down-stopped-answering",
            R::LoginTimeout { .. } => "live-down-login-timeout",
            R::NoAddress { .. } => "live-down-no-address",
            R::HostNotFound { .. } => "live-down-host-not-found",
            R::ThreadFailed { .. } => "live-down-thread",
            R::EveryTopicRefused => "live-down-topics-refused",
            R::UnaskedQos2 => "live-down-qos2",
            R::UnexpectedPacket { .. } => "live-down-unexpected-packet",
            R::NoConnack => "live-down-no-connack",
            R::NoHost => "live-down-no-host",
            R::PortZero => "live-down-port-zero",
            R::NoTopic => "live-down-no-topic",
            R::BadTopic { .. } => "live-down-bad-topic",
            R::PasswordWithoutUser => "live-down-password-without-user",
            R::TlsOnPlain => "live-down-tls-on-plain",
            R::CaUnreadable { .. } => "live-down-ca-unreadable",
            R::CaNotPem { .. } => "live-down-ca-not-pem",
            R::CaCertificateRefused { .. } => "live-down-ca-refused",
            R::CaEmpty { .. } => "live-down-ca-empty",
            R::BadTlsHost { .. } => "live-down-bad-tls-host",
            R::UnknownIssuer => "live-down-unknown-issuer",
            R::WrongHostName => "live-down-wrong-host-name",
            R::Expired => "live-down-expired",
            R::NotValidYet => "live-down-not-valid-yet",
            R::Revoked => "live-down-revoked",
            R::PinMismatch => "live-down-pin-mismatch",
            R::CertificateRefused { .. } => "live-down-certificate-refused",
            R::HandshakeAlert { .. } => "live-down-handshake-alert",
            R::NoCertificate => "live-down-no-certificate",
            R::Incompatible { .. } => "live-down-incompatible",
            R::NotTls { .. } => "live-down-not-tls",
            R::Tls { .. } => "live-down-tls",
        }
    }

    /// The values the sentence needs, by the names the catalog uses.
    pub fn args(&self) -> Vec<(&'static str, MessageArg)> {
        use DisconnectReason as R;
        let t = MessageArg::text;
        match self {
            R::TlsNeeded { target }
            | R::ConnectFailed { target }
            | R::SessionEnded { target }
            | R::StoppedAnswering { target }
            | R::LoginTimeout { target } => vec![("target", t(target))],
            R::CannotConnect { target, cause }
            | R::Handshake { target, cause }
            | R::LostLoggingIn { target, cause, .. }
            | R::Lost { target, cause } => vec![("target", t(target)), ("why", cause.arg())],
            R::NoAddress { host } | R::BadTlsHost { host } => vec![("host", t(host))],
            R::HostNotFound { host, cause } => vec![("host", t(host)), ("why", cause.arg())],
            R::ThreadFailed { error } | R::Tls { error } => vec![("error", t(error))],
            R::UnexpectedPacket { packet } => vec![("packet", t(packet))],
            R::BadTopic { problem, topic } => vec![
                ("why", MessageArg::Message(problem.message_id(), Vec::new())),
                ("topic", t(topic)),
            ],
            R::CaUnreadable { path, error }
            | R::CaNotPem { path, error }
            | R::CaCertificateRefused { path, error } => {
                vec![("path", t(path)), ("error", t(error))]
            }
            R::CaEmpty { path } => vec![("path", t(path))],
            R::CertificateRefused { detail }
            | R::HandshakeAlert { detail }
            | R::Incompatible { detail }
            | R::NotTls { detail } => vec![("detail", t(detail))],
            R::BadUserNameOrPassword
            | R::LoginRefused
            | R::ProtocolVersion
            | R::ClientIdRejected
            | R::BrokerUnavailable
            | R::UnexpectedSuccess
            | R::EveryTopicRefused
            | R::UnaskedQos2
            | R::NoConnack
            | R::NoHost
            | R::PortZero
            | R::NoTopic
            | R::PasswordWithoutUser
            | R::TlsOnPlain
            | R::UnknownIssuer
            | R::WrongHostName
            | R::Expired
            | R::NotValidYet
            | R::Revoked
            | R::PinMismatch
            | R::NoCertificate => Vec::new(),
        }
    }

    /// The reason with every piece of text cleaned of control characters
    /// and escape sequences ([`clean_text`]), as the client reports it.
    pub(crate) fn cleaned(self) -> DisconnectReason {
        use DisconnectReason as R;
        let c = |s: String| clean_text(&s, 300);
        match self {
            R::TlsNeeded { target } => R::TlsNeeded { target: c(target) },
            R::CannotConnect { target, cause } => R::CannotConnect {
                target: c(target),
                cause: cause.cleaned(),
            },
            R::ConnectFailed { target } => R::ConnectFailed { target: c(target) },
            R::Handshake { target, cause } => R::Handshake {
                target: c(target),
                cause: cause.cleaned(),
            },
            R::LostLoggingIn {
                target,
                cause,
                tls_hint,
            } => R::LostLoggingIn {
                target: c(target),
                cause: cause.cleaned(),
                tls_hint,
            },
            R::Lost { target, cause } => R::Lost {
                target: c(target),
                cause: cause.cleaned(),
            },
            R::SessionEnded { target } => R::SessionEnded { target: c(target) },
            R::StoppedAnswering { target } => R::StoppedAnswering { target: c(target) },
            R::LoginTimeout { target } => R::LoginTimeout { target: c(target) },
            R::NoAddress { host } => R::NoAddress { host: c(host) },
            R::HostNotFound { host, cause } => R::HostNotFound {
                host: c(host),
                cause: cause.cleaned(),
            },
            R::ThreadFailed { error } => R::ThreadFailed { error: c(error) },
            R::UnexpectedPacket { packet } => R::UnexpectedPacket { packet: c(packet) },
            R::BadTopic { problem, topic } => R::BadTopic {
                problem,
                topic: c(topic),
            },
            R::CaUnreadable { path, error } => R::CaUnreadable {
                path: c(path),
                error: c(error),
            },
            R::CaNotPem { path, error } => R::CaNotPem {
                path: c(path),
                error: c(error),
            },
            R::CaCertificateRefused { path, error } => R::CaCertificateRefused {
                path: c(path),
                error: c(error),
            },
            R::CaEmpty { path } => R::CaEmpty { path: c(path) },
            R::BadTlsHost { host } => R::BadTlsHost { host: c(host) },
            R::CertificateRefused { detail } => R::CertificateRefused { detail: c(detail) },
            R::HandshakeAlert { detail } => R::HandshakeAlert { detail: c(detail) },
            R::Incompatible { detail } => R::Incompatible { detail: c(detail) },
            R::NotTls { detail } => R::NotTls { detail: c(detail) },
            R::Tls { error } => R::Tls { error: c(error) },
            other => other,
        }
    }

    /// One of every variant, and every [`IoCause`] and [`TopicProblem`],
    /// with sample values, for tests that check each sentence in every
    /// language. Not part of the API.
    #[doc(hidden)]
    pub fn examples() -> Vec<DisconnectReason> {
        use DisconnectReason as R;
        let target = || "logger.example:8883".to_string();
        let host = || "logger.example".to_string();
        let path = || "ca.pem".to_string();
        let error = || "os error 5".to_string();
        let detail = || "BadEncoding".to_string();
        let mut all = vec![
            R::BadUserNameOrPassword,
            R::LoginRefused,
            R::ProtocolVersion,
            R::ClientIdRejected,
            R::BrokerUnavailable,
            R::UnexpectedSuccess,
            R::TlsNeeded { target: target() },
            R::ConnectFailed { target: target() },
            R::Handshake {
                target: target(),
                cause: IoCause::Reset,
            },
            R::LostLoggingIn {
                target: target(),
                cause: IoCause::Closed,
                tls_hint: false,
            },
            R::LostLoggingIn {
                target: target(),
                cause: IoCause::Closed,
                tls_hint: true,
            },
            R::Lost {
                target: target(),
                cause: IoCause::NoAnswer,
            },
            R::SessionEnded { target: target() },
            R::StoppedAnswering { target: target() },
            R::LoginTimeout { target: target() },
            R::NoAddress { host: host() },
            R::HostNotFound {
                host: host(),
                cause: IoCause::LookupFailed,
            },
            R::ThreadFailed { error: error() },
            R::EveryTopicRefused,
            R::UnaskedQos2,
            R::UnexpectedPacket {
                packet: "PUBREC".into(),
            },
            R::NoConnack,
            R::NoHost,
            R::PortZero,
            R::NoTopic,
            R::PasswordWithoutUser,
            R::TlsOnPlain,
            R::CaUnreadable {
                path: path(),
                error: error(),
            },
            R::CaNotPem {
                path: path(),
                error: error(),
            },
            R::CaCertificateRefused {
                path: path(),
                error: error(),
            },
            R::CaEmpty { path: path() },
            R::BadTlsHost { host: host() },
            R::UnknownIssuer,
            R::WrongHostName,
            R::Expired,
            R::NotValidYet,
            R::Revoked,
            R::PinMismatch,
            R::CertificateRefused { detail: detail() },
            R::HandshakeAlert { detail: detail() },
            R::NoCertificate,
            R::Incompatible { detail: detail() },
            R::NotTls { detail: detail() },
            R::Tls { error: error() },
        ];
        for cause in [
            IoCause::NothingListening,
            IoCause::NoAnswer,
            IoCause::Reset,
            IoCause::Closed,
            IoCause::NoRoute,
            IoCause::LookupFailed,
            IoCause::Other { error: error() },
        ] {
            all.push(R::CannotConnect {
                target: target(),
                cause,
            });
        }
        for problem in TopicProblem::ALL {
            all.push(R::BadTopic {
                problem,
                topic: "a/#/b".into(),
            });
        }
        all
    }
}

impl fmt::Display for DisconnectReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use DisconnectReason as R;
        match self {
            R::BadUserNameOrPassword => {
                f.write_str("Not authorized: the broker refused the user name or password.")
            }
            R::LoginRefused => f.write_str(
                "Not authorized: the broker refused the login. Check the user name and password.",
            ),
            R::ProtocolVersion => f.write_str("Refused: the broker does not accept MQTT 3.1.1."),
            R::ClientIdRejected => f.write_str("Refused: the broker rejected the client id."),
            R::BrokerUnavailable => {
                f.write_str("Unreachable: the broker says it is unavailable right now.")
            }
            R::UnexpectedSuccess => f.write_str("Protocol error: unexpected success code."),
            R::TlsNeeded { target } => write!(
                f,
                "TLS needed: {target} answered with TLS. Turn TLS on for this source."
            ),
            R::CannotConnect { target, cause } => write!(
                f,
                "Unreachable: cannot connect to {target}: {}.",
                cause.text()
            ),
            R::ConnectFailed { target } => write!(f, "Unreachable: cannot connect to {target}."),
            R::Handshake { target, cause } => write!(
                f,
                "TLS failure: the handshake with {target} did not finish: {}.",
                cause.text()
            ),
            R::LostLoggingIn {
                target,
                cause,
                tls_hint,
            } => {
                write!(
                    f,
                    "Connection lost while logging in to {target}: {}.",
                    cause.text()
                )?;
                if *tls_hint {
                    f.write_str(TLS_HINT)?;
                }
                Ok(())
            }
            R::Lost { target, cause } => {
                write!(f, "Connection lost to {target}: {}.", cause.text())
            }
            R::SessionEnded { target } => write!(
                f,
                "Connection lost to {target}: the broker ended the session."
            ),
            R::StoppedAnswering { target } => write!(
                f,
                "Connection lost to {target}: the broker stopped answering."
            ),
            R::LoginTimeout { target } => write!(
                f,
                "Unreachable: {target} accepted the connection but did not answer the login in time."
            ),
            R::NoAddress { host } => write!(f, "Unreachable: the host {host} has no address."),
            R::HostNotFound { host, cause } => write!(
                f,
                "Unreachable: cannot find the host {host}: {}.",
                cause.text()
            ),
            R::ThreadFailed { error } => write!(
                f,
                "Failed: cannot start the connection thread: {error} Open the source again."
            ),
            R::EveryTopicRefused => f.write_str(
                "Not authorized: the broker refused every topic this source asked for.",
            ),
            R::UnaskedQos2 => {
                f.write_str("Protocol error: the broker sent QoS 2, which was not asked for.")
            }
            R::UnexpectedPacket { packet } => write!(
                f,
                "Protocol error: unexpected {packet} packet from the broker."
            ),
            R::NoConnack => {
                f.write_str("Protocol error: the broker did not answer the login with CONNACK.")
            }
            R::NoHost => f.write_str("Settings error: no host is set."),
            R::PortZero => f.write_str("Settings error: the port is 0."),
            R::NoTopic => f.write_str("Settings error: no topic is set."),
            R::BadTopic { problem, topic } => write!(f, "Settings error: {problem}: {topic}."),
            R::PasswordWithoutUser => f.write_str("Settings error: a password needs a user name."),
            R::TlsOnPlain => {
                f.write_str("Settings error: TLS was asked for on a plain connection.")
            }
            R::CaUnreadable { path, error } => write!(
                f,
                "Settings error: cannot read the CA file {path}: {error} Check the CA file in the source's settings."
            ),
            R::CaNotPem { path, error } => write!(
                f,
                "Settings error: the CA file {path} is not valid PEM: {error} Choose a PEM certificate file."
            ),
            R::CaCertificateRefused { path, error } => write!(
                f,
                "Settings error: a certificate in {path} was refused: {error} Check the CA file in the source's settings."
            ),
            R::CaEmpty { path } => {
                write!(f, "Settings error: the CA file {path} holds no certificate.")
            }
            R::BadTlsHost { host } => write!(
                f,
                "Settings error: {host} is not a valid host name for TLS."
            ),
            R::UnknownIssuer => f.write_str(
                "TLS failure: the broker's certificate is not trusted. Its issuer is unknown; set a CA file if the broker uses a private certificate.",
            ),
            R::WrongHostName => f.write_str(
                "TLS failure: the broker's certificate is for a different host name.",
            ),
            R::Expired => f.write_str("TLS failure: the broker's certificate has expired."),
            R::NotValidYet => f.write_str(
                "TLS failure: the broker's certificate is not valid yet. Check this computer's clock.",
            ),
            R::Revoked => f.write_str("TLS failure: the broker's certificate was revoked."),
            R::PinMismatch => f.write_str(
                "TLS failure: the broker's key does not match the pin. Check the pin or join code; this may not be the broker you meant.",
            ),
            R::CertificateRefused { detail } => write!(
                f,
                "TLS failure: the broker's certificate was refused ({detail})."
            ),
            R::HandshakeAlert { detail } => write!(
                f,
                "TLS failure: the broker ended the handshake ({detail})."
            ),
            R::NoCertificate => f.write_str("TLS failure: the broker sent no certificate."),
            R::Incompatible { detail } => write!(
                f,
                "TLS failure: the broker's TLS settings are not compatible ({detail})."
            ),
            R::NotTls { detail } => write!(
                f,
                "TLS failure: the broker did not answer with TLS ({detail}). Is this a plain port?"
            ),
            R::Tls { error } => write!(f, "TLS failure: {error} Check the source's TLS settings."),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_example_has_its_own_sentence() {
        let all = DisconnectReason::examples();
        let mut texts: Vec<String> = all.iter().map(ToString::to_string).collect();
        texts.sort();
        texts.dedup();
        assert_eq!(texts.len(), all.len());
        for r in &all {
            let s = r.to_string();
            assert!(s.ends_with('.') || s.ends_with('?'), "{s}");
            assert!(s.starts_with(|c: char| c.is_ascii_uppercase()), "{s}");
        }
    }

    #[test]
    fn cleaning_takes_escapes_out_of_the_data() {
        let r = DisconnectReason::NoAddress {
            host: "evil\u{1b}[2Jhost".into(),
        }
        .cleaned();
        assert!(!r.to_string().contains('\u{1b}'), "{r}");
    }
}
