//! Join links: one line that carries everything a reader needs to join
//! textweaver's own broker by code, to copy, paste, and later put in a QR
//! code.
//!
//! The shape is described on [`JoinLink`]. A link holds the join code,
//! secret part included, so it is a secret: `Debug` hides the code, and
//! the app never logs it, saves it, or shows it in the status line.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, UdpSocket};

use crate::pin::{JOIN_CODE_CHARS, JoinCode};
use crate::reason::MessageArg;
use crate::topic::TopicProblem;

/// The link's scheme, before `://`.
pub const JOIN_LINK_SCHEME: &str = "twjoin";

/// The port a link without one means: MQTT over TLS.
const DEFAULT_PORT: u16 = 8883;

/// The longest link read: a host name, a port, a code and a topic fit
/// well inside it.
const MAX_LINK: usize = 2048;

/// A join link: textweaver's own broker, its port, a join code, and,
/// for a shared document, the topic to follow.
///
/// # Shape
///
/// ```text
/// twjoin://HOST:PORT/CODE?topic=TOPIC
/// ```
///
/// - `twjoin://` is the scheme, in any case when read.
/// - `HOST` is a host name (letters, digits, dots and dashes), an IPv4
///   address, or an IPv6 address in square brackets, such as `[::1]`.
/// - `PORT` is 1 to 65535. Always written; a link read without one means
///   8883, MQTT over TLS.
/// - `CODE` is the join code in four groups of four, separated by dashes,
///   such as `ABCD-EFGH-JKMN-PQRS`. It is read as
///   [`JoinCode::parse`] reads a code: any case, with or without dashes.
/// - `?topic=TOPIC` is optional: the topic filter to follow, percent
///   encoded (UTF-8; every byte other than a letter, a digit, or one of
///   `- . _ ~ /` is written as `%XX`, so `#` is `%23` and `+` is `%2B`).
///   No other key may follow the question mark, and `topic` only once.
///
/// For example `twjoin://192.168.1.20:8883/ABCD-EFGH-JKMN-PQRS?topic=textweaver%2Fdocument`
/// is written as `twjoin://192.168.1.20:8883/ABCD-EFGH-JKMN-PQRS?topic=textweaver/document`,
/// since `/` is kept as it is.
///
/// Reading a link ([`JoinLink::parse`]) forgives the space around it,
/// quotes or angle brackets around it, a final period, and a label before
/// it such as `Join link:`. Anything else that is not this shape is
/// refused with a [`JoinLinkError`], whose English sentence starts with
/// "Join link error", meaning first. [`JoinLink::find`] finds a link
/// inside longer pasted text.
#[derive(Clone, PartialEq, Eq)]
pub struct JoinLink {
    host: String,
    port: u16,
    code: JoinCode,
    topic: Option<String>,
}

impl JoinLink {
    /// A link to `host` and `port` with `code` and, optionally, the topic
    /// to follow. The host and topic are checked as [`JoinLink::parse`]
    /// checks them.
    pub fn new(
        host: &str,
        port: u16,
        code: JoinCode,
        topic: Option<&str>,
    ) -> Result<JoinLink, JoinLinkError> {
        let host = host
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_owned();
        check_host(&host)?;
        if port == 0 {
            return Err(JoinLinkError::BadPort);
        }
        let topic = match topic.map(str::trim).filter(|t| !t.is_empty()) {
            Some(t) => Some(check_topic(t)?),
            None => None,
        };
        Ok(JoinLink {
            host,
            port,
            code,
            topic,
        })
    }

    /// Reads a link as copied or pasted. An error never repeats the code.
    pub fn parse(text: &str) -> Result<JoinLink, JoinLinkError> {
        if text.chars().count() > MAX_LINK {
            return Err(JoinLinkError::TooLong);
        }
        let Some(start) = scheme_at(text) else {
            return Err(JoinLinkError::NoScheme);
        };
        // A label before the link, such as "Join link:", is allowed;
        // anything after it is not.
        let link = text[start..]
            .trim_end()
            .trim_end_matches(['"', '\'', '>', '.', ')'].as_slice());
        parse_link(link)
    }

    /// The first link inside `text`, such as a pasted message or the
    /// "Show the join code" list: `None` when `text` holds no
    /// `twjoin://`, else that link read as [`JoinLink::parse`] reads one,
    /// up to the first space or line break after it.
    pub fn find(text: &str) -> Option<Result<JoinLink, JoinLinkError>> {
        if text.chars().count() > MAX_LINK * 4 {
            return scheme_at(text).map(|_| Err(JoinLinkError::TooLong));
        }
        let start = scheme_at(text)?;
        let rest = &text[start..];
        let end = rest
            .find(|c: char| c.is_whitespace() || matches!(c, '"' | '<' | '>'))
            .unwrap_or(rest.len());
        Some(JoinLink::parse(&rest[..end]))
    }

    /// True when `text` holds the start of a join link.
    pub fn is_in(text: &str) -> bool {
        scheme_at(text).is_some()
    }

    /// The broker's host name or address, without brackets.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// The broker's port.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The join code.
    pub fn code(&self) -> &JoinCode {
        &self.code
    }

    /// The topic filter to follow, if the link names one.
    pub fn topic(&self) -> Option<&str> {
        self.topic.as_deref()
    }

    /// The host to put in a link for a broker listening on `ip`: the
    /// address itself, or for every address (`0.0.0.0` or `::`) this
    /// computer's address on its main network, as the system's routes
    /// choose it. Choosing sends nothing: a UDP socket is only pointed at
    /// a documentation address (RFC 5737, RFC 3849) and asked its own
    /// address. With no network, the loopback address.
    pub fn host_for(ip: IpAddr) -> String {
        if !ip.is_unspecified() {
            return ip.to_string();
        }
        let (bind, peer, loopback): (IpAddr, IpAddr, IpAddr) = match ip {
            IpAddr::V4(_) => (
                Ipv4Addr::UNSPECIFIED.into(),
                Ipv4Addr::new(192, 0, 2, 1).into(),
                Ipv4Addr::LOCALHOST.into(),
            ),
            IpAddr::V6(_) => (
                Ipv6Addr::UNSPECIFIED.into(),
                Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1).into(),
                Ipv6Addr::LOCALHOST.into(),
            ),
        };
        UdpSocket::bind((bind, 0))
            .and_then(|s| s.connect((peer, 9)).map(|()| s))
            .and_then(|s| s.local_addr())
            .map(|a| a.ip())
            .ok()
            .filter(|a| !a.is_unspecified())
            .unwrap_or(loopback)
            .to_string()
    }
}

impl fmt::Display for JoinLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{JOIN_LINK_SCHEME}://")?;
        if self.host.contains(':') {
            write!(f, "[{}]", self.host)?;
        } else {
            f.write_str(&self.host)?;
        }
        write!(f, ":{}/{}", self.port, self.code.grouped())?;
        if let Some(t) = &self.topic {
            write!(f, "?topic={}", percent_encode(t))?;
        }
        Ok(())
    }
}

impl fmt::Debug for JoinLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JoinLink")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("code", &self.code)
            .field("topic", &self.topic)
            .finish()
    }
}

/// Why a join link (or the join code inside it) does not read, as data,
/// so the app can say it in the reader's language. `Display` is the
/// English sentence, starting "Join link error:" or, for the code,
/// "Join code error:", meaning first; it never repeats the code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JoinLinkError {
    /// The text is far too long to be a link.
    TooLong,
    /// The text holds no `twjoin://`.
    NoScheme,
    /// The link holds a space or line break.
    Spaces,
    /// No join code after the host and port.
    NoCode,
    /// No host.
    NoHost,
    /// An IPv6 address without its closing bracket.
    Ipv6Bracket,
    /// The address in brackets is not IPv6.
    NotIpv6,
    /// The port is not 1 to 65535.
    BadPort,
    /// The host name is longer than 253 characters.
    HostTooLong,
    /// A character that host names do not use.
    HostCharacter {
        /// The character.
        character: char,
    },
    /// A key other than `topic` after the question mark.
    UnknownKey {
        /// The key, decoded, without control characters, at most 40
        /// characters.
        key: String,
    },
    /// `topic` twice.
    TopicTwice,
    /// The topic holds a control character.
    TopicControl,
    /// The topic cannot be followed.
    Topic(TopicProblem),
    /// A percent sign not followed by two hex digits.
    PercentSign,
    /// The decoded topic is not UTF-8.
    NotUtf8,
    /// The join code is far too long.
    CodeTooLong,
    /// No join code.
    CodeEmpty,
    /// The join code has the wrong number of characters.
    CodeLength {
        /// How many it has.
        count: usize,
    },
    /// A character that join codes do not use.
    CodeCharacter {
        /// The character.
        character: char,
        /// Its group of four, from 1.
        group: usize,
    },
}

/// How a character is shown in an error: itself when it is printable
/// ASCII, else its code.
fn printable(c: char) -> bool {
    c.is_ascii_graphic()
}

impl JoinLinkError {
    /// The error is about the join code inside the link.
    pub fn is_about_the_code(&self) -> bool {
        matches!(
            self,
            JoinLinkError::CodeTooLong
                | JoinLinkError::CodeEmpty
                | JoinLinkError::CodeLength { .. }
                | JoinLinkError::CodeCharacter { .. }
        )
    }

    /// The id of the words after the lead-in, lower case, in the app's
    /// message catalog.
    pub fn message_id(&self) -> &'static str {
        use JoinLinkError as E;
        match self {
            E::TooLong | E::CodeTooLong => "live-link-bad-too-long",
            E::NoScheme => "live-link-bad-scheme",
            E::Spaces => "live-link-bad-spaces",
            E::NoCode => "live-link-bad-no-code",
            E::NoHost => "live-link-bad-no-host",
            E::Ipv6Bracket => "live-link-bad-bracket",
            E::NotIpv6 => "live-link-bad-not-ipv6",
            E::BadPort => "live-link-bad-port",
            E::HostTooLong => "live-link-bad-host-too-long",
            E::HostCharacter { character } if printable(*character) => {
                "live-link-bad-host-character"
            }
            E::HostCharacter { .. } => "live-link-bad-host-character-code",
            E::UnknownKey { .. } => "live-link-bad-key",
            E::TopicTwice => "live-link-bad-topic-twice",
            E::TopicControl => "live-link-bad-topic-control",
            E::Topic(_) => "live-link-bad-topic",
            E::PercentSign => "live-link-bad-percent",
            E::NotUtf8 => "live-link-bad-utf8",
            E::CodeEmpty => "live-link-bad-code-empty",
            E::CodeLength { .. } => "live-link-bad-code-length",
            E::CodeCharacter { character, .. } if printable(*character) => {
                "live-link-bad-code-character"
            }
            E::CodeCharacter { .. } => "live-link-bad-code-character-code",
        }
    }

    /// The values the words need, by the names the catalog uses.
    pub fn args(&self) -> Vec<(&'static str, MessageArg)> {
        use JoinLinkError as E;
        let character = |c: char| {
            if printable(c) {
                ("character", MessageArg::Text(c.to_string()))
            } else {
                ("code", MessageArg::Number(i64::from(u32::from(c))))
            }
        };
        match self {
            E::NoScheme => vec![("scheme", MessageArg::Text(JOIN_LINK_SCHEME.into()))],
            E::HostCharacter { character: c } => vec![character(*c)],
            E::UnknownKey { key } => vec![("key", MessageArg::Text(key.clone()))],
            E::Topic(problem) => {
                vec![("why", MessageArg::Message(problem.message_id(), Vec::new()))]
            }
            E::CodeLength { count } => vec![
                (
                    "count",
                    MessageArg::Number(i64::try_from(*count).unwrap_or(i64::MAX)),
                ),
                ("needed", MessageArg::Number(JOIN_CODE_CHARS as i64)),
            ],
            E::CodeCharacter {
                character: c,
                group,
            } => vec![
                character(*c),
                (
                    "group",
                    MessageArg::Number(i64::try_from(*group).unwrap_or(i64::MAX)),
                ),
            ],
            _ => Vec::new(),
        }
    }

    /// The English sentence without its "Join link error:" lead-in, as
    /// `tw join-qr` prints it after "No join link:". A code error keeps
    /// its "Join code error:" lead-in.
    pub fn detail(&self) -> String {
        let all = self.to_string();
        match all.strip_prefix("Join link error: ") {
            Some(rest) => rest.to_owned(),
            None => all,
        }
    }

    /// The words after the lead-in, in English.
    fn words(&self) -> String {
        use JoinLinkError as E;
        let shown = |c: char| {
            if printable(c) {
                c.to_string()
            } else {
                format!("a character with code {}", u32::from(c))
            }
        };
        match self {
            E::TooLong | E::CodeTooLong => "the text is far too long.".into(),
            E::NoScheme => format!("a join link starts with {JOIN_LINK_SCHEME}://."),
            E::Spaces => "a join link has no spaces or line breaks.".into(),
            E::NoCode => "no join code was given after the host and port.".into(),
            E::NoHost => "no host was given.".into(),
            E::Ipv6Bracket => "an IPv6 address needs its closing square bracket.".into(),
            E::NotIpv6 => "the address in square brackets is not an IPv6 address.".into(),
            E::BadPort => "the port is not a number from 1 to 65535.".into(),
            E::HostTooLong => "the host name is too long.".into(),
            E::HostCharacter { character } => format!(
                "{} is not used in host names. Check the host.",
                shown(*character)
            ),
            E::UnknownKey { key } => format!(
                "{key} is not part of a join link. Only topic may follow the question mark."
            ),
            E::TopicTwice => "the topic is given twice.".into(),
            E::TopicControl => "the topic holds a control character.".into(),
            E::Topic(problem) => format!("{problem}."),
            E::PercentSign => "the topic has a percent sign not followed by two hex digits.".into(),
            E::NotUtf8 => "the topic is not UTF-8 text.".into(),
            E::CodeEmpty => "no code was given.".into(),
            E::CodeLength { count } => {
                let chars = if *count == 1 {
                    "character"
                } else {
                    "characters"
                };
                format!(
                    "the code has {count} {chars}; it needs {JOIN_CODE_CHARS}, in four groups of four."
                )
            }
            E::CodeCharacter { character, group } => format!(
                "{} in group {group} is not used in join codes. Check that character.",
                shown(*character)
            ),
        }
    }

    /// One of every variant, with sample values, for tests that check
    /// each sentence in every language. Not part of the API.
    #[doc(hidden)]
    pub fn examples() -> Vec<JoinLinkError> {
        use JoinLinkError as E;
        let mut all = vec![
            E::TooLong,
            E::NoScheme,
            E::Spaces,
            E::NoCode,
            E::NoHost,
            E::Ipv6Bracket,
            E::NotIpv6,
            E::BadPort,
            E::HostTooLong,
            E::HostCharacter { character: '_' },
            E::HostCharacter { character: '\u{7}' },
            E::UnknownKey { key: "user".into() },
            E::TopicTwice,
            E::TopicControl,
            E::PercentSign,
            E::NotUtf8,
            E::CodeTooLong,
            E::CodeEmpty,
            E::CodeLength { count: 1 },
            E::CodeLength { count: 15 },
            E::CodeCharacter {
                character: '!',
                group: 2,
            },
            E::CodeCharacter {
                character: '\u{e9}',
                group: 1,
            },
        ];
        all.extend(TopicProblem::ALL.into_iter().map(E::Topic));
        all
    }
}

impl fmt::Display for JoinLinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lead = if self.is_about_the_code() {
            "Join code error"
        } else {
            "Join link error"
        };
        write!(f, "{lead}: {}", self.words())
    }
}

impl std::error::Error for JoinLinkError {}

/// Where `twjoin://` starts in `text`, in any case.
fn scheme_at(text: &str) -> Option<usize> {
    let lower = text.to_ascii_lowercase();
    lower.find(&format!("{JOIN_LINK_SCHEME}://"))
}

/// Reads `twjoin://HOST:PORT/CODE?topic=TOPIC`, starting at the scheme.
fn parse_link(link: &str) -> Result<JoinLink, JoinLinkError> {
    let rest = &link[JOIN_LINK_SCHEME.len() + 3..];
    if rest.chars().any(char::is_whitespace) {
        return Err(JoinLinkError::Spaces);
    }
    let (authority, rest) = rest.split_once('/').ok_or(JoinLinkError::NoCode)?;
    let (host, port) = split_host_port(authority)?;
    let (code, query) = match rest.split_once('?') {
        Some((c, q)) => (c, Some(q)),
        None => (rest, None),
    };
    let code = code.trim_end_matches('/');
    if code.is_empty() {
        return Err(JoinLinkError::NoCode);
    }
    let code = JoinCode::read(code)?;
    let mut topic = None;
    for pair in query.unwrap_or("").split('&').filter(|p| !p.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if !key.eq_ignore_ascii_case("topic") {
            let key = percent_decode(key).unwrap_or_else(|_| key.to_owned());
            let key: String = key.chars().filter(|c| !c.is_control()).take(40).collect();
            return Err(JoinLinkError::UnknownKey { key });
        }
        if topic.is_some() {
            return Err(JoinLinkError::TopicTwice);
        }
        let value = percent_decode(value)?;
        topic = Some(value);
    }
    JoinLink::new(&host, port, code, topic.as_deref())
}

/// `HOST:PORT`, `[V6]:PORT`, or either without the port.
fn split_host_port(authority: &str) -> Result<(String, u16), JoinLinkError> {
    if authority.is_empty() {
        return Err(JoinLinkError::NoHost);
    }
    let (host, port) = if let Some(after) = authority.strip_prefix('[') {
        let (h, rest) = after.split_once(']').ok_or(JoinLinkError::Ipv6Bracket)?;
        let port = match rest {
            "" => None,
            p => Some(p.strip_prefix(':').ok_or(JoinLinkError::BadPort)?),
        };
        if h.parse::<Ipv6Addr>().is_err() {
            return Err(JoinLinkError::NotIpv6);
        }
        (h.to_owned(), port)
    } else {
        match authority.split_once(':') {
            Some((h, p)) => (h.to_owned(), Some(p)),
            None => (authority.to_owned(), None),
        }
    };
    let port = match port {
        None => DEFAULT_PORT,
        Some(p) => p
            .parse::<u16>()
            .ok()
            .filter(|p| *p > 0)
            .ok_or(JoinLinkError::BadPort)?,
    };
    Ok((host, port))
}

/// A host name, or an IP address (an IPv6 one without brackets).
fn check_host(host: &str) -> Result<(), JoinLinkError> {
    if host.is_empty() {
        return Err(JoinLinkError::NoHost);
    }
    if host.contains(':') {
        return host
            .parse::<Ipv6Addr>()
            .map(|_| ())
            .map_err(|_| JoinLinkError::NotIpv6);
    }
    if host.len() > 253 {
        return Err(JoinLinkError::HostTooLong);
    }
    if let Some(character) = host
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '-')))
    {
        return Err(JoinLinkError::HostCharacter { character });
    }
    Ok(())
}

/// A topic filter a reader may subscribe to, cleaned of nothing: one with
/// control characters is refused, not changed.
fn check_topic(topic: &str) -> Result<String, JoinLinkError> {
    if topic.chars().any(char::is_control) {
        return Err(JoinLinkError::TopicControl);
    }
    if let Some(problem) = TopicProblem::of_filter(topic) {
        return Err(JoinLinkError::Topic(problem));
    }
    Ok(topic.to_owned())
}

/// Every byte other than a letter, a digit, or `- . _ ~ /` as `%XX`.
fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~' | b'/') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `%XX` back to bytes, and the bytes as UTF-8. A `+` stays a `+`.
fn percent_decode(text: &str) -> Result<String, JoinLinkError> {
    let bad = JoinLinkError::PercentSign;
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3).ok_or(bad.clone())?;
            let hex = std::str::from_utf8(hex).map_err(|_| bad.clone())?;
            out.push(u8::from_str_radix(hex, 16).map_err(|_| bad.clone())?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| JoinLinkError::NotUtf8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code() -> JoinCode {
        JoinCode::parse("ABCD-EFGH-JKMN-PQRS").unwrap()
    }

    #[test]
    fn a_link_is_written_in_its_documented_shape() {
        let l = JoinLink::new("192.168.1.20", 8883, code(), Some("textweaver/document")).unwrap();
        assert_eq!(
            l.to_string(),
            "twjoin://192.168.1.20:8883/ABCD-EFGH-JKMN-PQRS?topic=textweaver/document"
        );
        let l = JoinLink::new("::1", 1884, code(), None).unwrap();
        assert_eq!(l.to_string(), "twjoin://[::1]:1884/ABCD-EFGH-JKMN-PQRS");
        let l = JoinLink::new("club.example", 8883, code(), Some("net log/#")).unwrap();
        assert_eq!(
            l.to_string(),
            "twjoin://club.example:8883/ABCD-EFGH-JKMN-PQRS?topic=net%20log/%23"
        );
    }

    #[test]
    fn links_read_back_what_was_written() {
        for (host, topic) in [
            ("192.168.1.20", Some("textweaver/document")),
            ("::1", None),
            ("club.example", Some("net log/#")),
            ("100.101.1.2", Some("café/+/news")),
        ] {
            let l = JoinLink::new(host, 8883, code(), topic).unwrap();
            let back = JoinLink::parse(&l.to_string()).unwrap();
            assert_eq!(back, l);
            assert_eq!(back.topic(), topic);
            assert_eq!(back.host(), host);
        }
    }

    #[test]
    fn reading_forgives_what_copying_adds() {
        let want = JoinLink::new("10.0.0.5", 8883, code(), Some("a/b")).unwrap();
        for text in [
            "twjoin://10.0.0.5:8883/ABCD-EFGH-JKMN-PQRS?topic=a/b",
            "  TWJOIN://10.0.0.5:8883/abcd.efgh.jkmn.pqrs?topic=a%2Fb \n",
            "\"twjoin://10.0.0.5:8883/ABCDEFGHJKMNPQRS?topic=a/b\"",
            "<twjoin://10.0.0.5:8883/ABCD-EFGH-JKMN-PQRS?topic=a/b>",
            "Join link: twjoin://10.0.0.5:8883/ABCD-EFGH-JKMN-PQRS?topic=a/b.",
            "twjoin://10.0.0.5/ABCD-EFGH-JKMN-PQRS/?TOPIC=a/b",
        ] {
            let got = JoinLink::parse(text).map_err(|e| format!("{text}: {e}"));
            assert_eq!(got.unwrap(), want, "{text}");
        }
        assert_eq!(
            JoinLink::parse("twjoin://[::1]/ABCD-EFGH-JKMN-PQRS")
                .unwrap()
                .port(),
            8883
        );
    }

    #[test]
    fn find_takes_the_link_out_of_longer_text() {
        let text = "Join code: ABCD-EFGH-JKMN-PQRS\nJoin link: twjoin://10.0.0.5:8883/ABCD-EFGH-JKMN-PQRS\nGroup 1 of 4: ABCD";
        let l = JoinLink::find(text).unwrap().unwrap();
        assert_eq!(l.host(), "10.0.0.5");
        assert!(JoinLink::find("ABCD-EFGH-JKMN-PQRS").is_none());
        assert!(JoinLink::is_in("see TWJOIN://x"));
        assert!(!JoinLink::is_in("twjoin:x"));
    }

    #[test]
    fn anything_else_is_refused_in_words() {
        let cases = [
            (
                "http://10.0.0.5/ABCD",
                "Join link error: a join link starts with twjoin://.",
            ),
            (
                "twjoin://",
                "Join link error: no join code was given after the host and port.",
            ),
            (
                "twjoin://10.0.0.5:8883/",
                "Join link error: no join code was given after the host and port.",
            ),
            (
                "twjoin:///ABCD-EFGH-JKMN-PQRS",
                "Join link error: no host was given.",
            ),
            (
                "twjoin://10.0.0.5:0/ABCD-EFGH-JKMN-PQRS",
                "Join link error: the port is not a number from 1 to 65535.",
            ),
            (
                "twjoin://10.0.0.5:http/ABCD-EFGH-JKMN-PQRS",
                "Join link error: the port is not a number from 1 to 65535.",
            ),
            (
                "twjoin://bad_host/ABCD-EFGH-JKMN-PQRS",
                "Join link error: _ is not used in host names. Check the host.",
            ),
            (
                "twjoin://[zz]/ABCD-EFGH-JKMN-PQRS",
                "Join link error: the address in square brackets is not an IPv6 address.",
            ),
            (
                "twjoin://h/ABCD-EFGH-JKMN-PQRS?user=x",
                "Join link error: user is not part of a join link. Only topic may follow the question mark.",
            ),
            (
                "twjoin://h/ABCD-EFGH-JKMN-PQRS?topic=a&topic=b",
                "Join link error: the topic is given twice.",
            ),
            (
                "twjoin://h/ABCD-EFGH-JKMN-PQRS?topic=a%2",
                "Join link error: the topic has a percent sign not followed by two hex digits.",
            ),
            (
                "twjoin://h/ABCD-EFGH-JKMN-PQRS?topic=a/%23/b",
                "Join link error: # may only be a whole last level of a topic.",
            ),
            (
                "twjoin://h/ABCD-EFGH-JKMN-PQRS?topic=a%0Ab",
                "Join link error: the topic holds a control character.",
            ),
            (
                "twjoin://h/ABCD-EFGH-JKMN-PQR",
                "Join code error: the code has 15 characters; it needs 16, in four groups of four.",
            ),
        ];
        for (text, want) in cases {
            assert_eq!(
                JoinLink::parse(text).unwrap_err().to_string(),
                want,
                "{text}"
            );
        }
        let long = format!("twjoin://{}/ABCD-EFGH-JKMN-PQRS", "a".repeat(3000));
        assert_eq!(JoinLink::parse(&long).unwrap_err(), JoinLinkError::TooLong);
    }

    #[test]
    fn errors_are_data_with_one_english_sentence_each() {
        assert_eq!(
            JoinLink::parse("twjoin://h:0/ABCD-EFGH-JKMN-PQRS").unwrap_err(),
            JoinLinkError::BadPort
        );
        assert_eq!(
            JoinLink::parse("twjoin://h/ABCD-EFGH-JKMN-PQR").unwrap_err(),
            JoinLinkError::CodeLength { count: 15 }
        );
        assert_eq!(
            JoinLinkError::BadPort.detail(),
            "the port is not a number from 1 to 65535."
        );
        assert_eq!(
            JoinLinkError::CodeEmpty.detail(),
            "Join code error: no code was given."
        );
        assert_eq!(
            JoinLinkError::CodeLength { count: 1 }.to_string(),
            "Join code error: the code has 1 character; it needs 16, in four groups of four."
        );
        for e in JoinLinkError::examples() {
            let s = e.to_string();
            assert!(s.ends_with('.'), "{s}");
            assert!(!s.contains("ABCD"), "{s}");
        }
    }

    #[test]
    fn the_code_is_hidden_from_debug() {
        let l = JoinLink::new("h", 8883, code(), None).unwrap();
        let shown = format!("{l:?}");
        assert!(!shown.contains("ABCD"), "{shown}");
        assert!(shown.contains("(hidden)"), "{shown}");
    }

    #[test]
    fn a_host_is_chosen_for_every_address() {
        assert_eq!(
            JoinLink::host_for("127.0.0.1".parse().unwrap()),
            "127.0.0.1"
        );
        assert_eq!(JoinLink::host_for("::1".parse().unwrap()), "::1");
        // Every address: some address of this computer, never 0.0.0.0.
        let any = JoinLink::host_for("0.0.0.0".parse().unwrap());
        assert!(
            any.parse::<IpAddr>().is_ok_and(|a| !a.is_unspecified()),
            "{any}"
        );
    }
}
