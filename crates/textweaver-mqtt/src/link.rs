//! Join links: one line that carries everything a reader needs to join
//! textweaver's own broker by code, to copy, paste, and later put in a QR
//! code.
//!
//! The shape is described on [`JoinLink`]. A link holds the join code,
//! secret part included, so it is a secret: `Debug` hides the code, and
//! the app never logs it, saves it, or shows it in the status line.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, UdpSocket};

use crate::pin::JoinCode;
use crate::topic;

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
/// refused with a sentence that starts with "Join link error", meaning
/// first. [`JoinLink::find`] finds a link inside longer pasted text.
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
    ) -> Result<JoinLink, String> {
        let host = host
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_owned();
        check_host(&host)?;
        if port == 0 {
            return Err(port_error());
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

    /// Reads a link as copied or pasted. Errors are sentences, meaning
    /// first, that never repeat the code.
    pub fn parse(text: &str) -> Result<JoinLink, String> {
        if text.chars().count() > MAX_LINK {
            return Err("Join link error: the text is far too long.".into());
        }
        let Some(start) = scheme_at(text) else {
            return Err(format!(
                "Join link error: a join link starts with {JOIN_LINK_SCHEME}://."
            ));
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
    pub fn find(text: &str) -> Option<Result<JoinLink, String>> {
        if text.chars().count() > MAX_LINK * 4 {
            return scheme_at(text)
                .map(|_| Err("Join link error: the text is far too long.".into()));
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

/// Where `twjoin://` starts in `text`, in any case.
fn scheme_at(text: &str) -> Option<usize> {
    let lower = text.to_ascii_lowercase();
    lower.find(&format!("{JOIN_LINK_SCHEME}://"))
}

/// Reads `twjoin://HOST:PORT/CODE?topic=TOPIC`, starting at the scheme.
fn parse_link(link: &str) -> Result<JoinLink, String> {
    let rest = &link[JOIN_LINK_SCHEME.len() + 3..];
    if rest.chars().any(char::is_whitespace) {
        return Err("Join link error: a join link has no spaces or line breaks.".into());
    }
    let (authority, rest) = rest.split_once('/').ok_or_else(|| {
        "Join link error: no join code was given after the host and port.".to_string()
    })?;
    let (host, port) = split_host_port(authority)?;
    let (code, query) = match rest.split_once('?') {
        Some((c, q)) => (c, Some(q)),
        None => (rest, None),
    };
    let code = code.trim_end_matches('/');
    if code.is_empty() {
        return Err("Join link error: no join code was given after the host and port.".into());
    }
    let code = JoinCode::parse(code)?;
    let mut topic = None;
    for pair in query.unwrap_or("").split('&').filter(|p| !p.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if !key.eq_ignore_ascii_case("topic") {
            let key = percent_decode(key).unwrap_or_else(|_| key.to_owned());
            let key: String = key.chars().filter(|c| !c.is_control()).take(40).collect();
            return Err(format!(
                "Join link error: {key} is not part of a join link. Only topic may follow the question mark."
            ));
        }
        if topic.is_some() {
            return Err("Join link error: the topic is given twice.".into());
        }
        let value = percent_decode(value)?;
        topic = Some(value);
    }
    JoinLink::new(&host, port, code, topic.as_deref())
}

/// `HOST:PORT`, `[V6]:PORT`, or either without the port.
fn split_host_port(authority: &str) -> Result<(String, u16), String> {
    if authority.is_empty() {
        return Err("Join link error: no host was given.".into());
    }
    let (host, port) = if let Some(after) = authority.strip_prefix('[') {
        let (h, rest) = after.split_once(']').ok_or_else(|| {
            "Join link error: an IPv6 address needs its closing square bracket.".to_string()
        })?;
        let port = match rest {
            "" => None,
            p => Some(p.strip_prefix(':').ok_or_else(port_error)?),
        };
        if h.parse::<Ipv6Addr>().is_err() {
            return Err(
                "Join link error: the address in square brackets is not an IPv6 address.".into(),
            );
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
            .ok_or_else(port_error)?,
    };
    Ok((host, port))
}

fn port_error() -> String {
    "Join link error: the port is not a number from 1 to 65535.".into()
}

/// A host name, or an IP address (an IPv6 one without brackets).
fn check_host(host: &str) -> Result<(), String> {
    if host.is_empty() {
        return Err("Join link error: no host was given.".into());
    }
    if host.contains(':') {
        return host.parse::<Ipv6Addr>().map(|_| ()).map_err(|_| {
            "Join link error: the address in square brackets is not an IPv6 address.".into()
        });
    }
    if host.len() > 253 {
        return Err("Join link error: the host name is too long.".into());
    }
    if let Some(c) = host
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '-')))
    {
        let shown = if c.is_ascii_graphic() {
            c.to_string()
        } else {
            format!("a character with code {}", u32::from(c))
        };
        return Err(format!(
            "Join link error: {shown} is not used in host names. Check the host."
        ));
    }
    Ok(())
}

/// A topic filter a reader may subscribe to, cleaned of nothing: one with
/// control characters is refused, not changed.
fn check_topic(topic: &str) -> Result<String, String> {
    if topic.chars().any(char::is_control) {
        return Err("Join link error: the topic holds a control character.".into());
    }
    if let Some(why) = topic::filter_problem(topic) {
        return Err(format!("Join link error: {why}."));
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
fn percent_decode(text: &str) -> Result<String, String> {
    let bad = || "Join link error: the topic has a percent sign not followed by two hex digits.";
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3).ok_or_else(|| bad().to_string())?;
            let hex = std::str::from_utf8(hex).map_err(|_| bad().to_string())?;
            out.push(u8::from_str_radix(hex, 16).map_err(|_| bad().to_string())?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| "Join link error: the topic is not UTF-8 text.".into())
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
            assert_eq!(JoinLink::parse(text).unwrap_err(), want, "{text}");
        }
        let long = format!("twjoin://{}/ABCD-EFGH-JKMN-PQRS", "a".repeat(3000));
        assert!(JoinLink::parse(&long).unwrap_err().contains("far too long"));
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
