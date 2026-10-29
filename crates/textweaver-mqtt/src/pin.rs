//! Certificate pins and join codes.
//!
//! A pin is the SHA-256 hash of a broker's public key (its
//! SubjectPublicKeyInfo, SPKI), written in Crockford's base 32 so it can be
//! read aloud and typed: digits and capital letters without I, L, O or U.
//! A reader that pins a broker trusts exactly the key behind that hash, with
//! no certificate authority. Hashing the key rather than the whole
//! certificate means an impostor cannot search for a match by changing only
//! a serial number; it would have to make a new key for every try.
//!
//! - **The full pin** has 52 characters (the 256 bits of the hash), which
//!   textweaver's own broker prints in accounts mode ("Certificate pin:").
//! - **A join code** has 16 characters: the first 12 of the pin (60 bits)
//!   and 4 random secret characters (20 bits), read as four groups of four.
//!   Anyone who reaches the port can read the pin part from the
//!   certificate, so the login password is derived from the whole code
//!   ([`JoinCode::password`]): the secret part never leaves the broker
//!   except by being read aloud or shown.
//!
//! Parsing forgives case, spaces, line breaks, dashes, dots and commas,
//! and reads O as zero and
//! I or L as one, as Crockford's alphabet intends. A wrong character or
//! length is said in words, meaning first.

use std::fmt;

use ring::{digest, hkdf, rand::SecureRandom};

use crate::Secret;

/// Crockford base 32: no I, L, O or U, so nothing looks alike when read.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Characters in a full pin: 256 bits, padded to 260.
pub const FULL_PIN_CHARS: usize = 52;
/// Characters of the pin a join code carries: 60 bits.
pub const SHORT_PIN_CHARS: usize = 12;
/// Secret characters in a join code: 20 bits.
pub const JOIN_SECRET_CHARS: usize = 4;
/// Characters in a join code.
pub const JOIN_CODE_CHARS: usize = SHORT_PIN_CHARS + JOIN_SECRET_CHARS;
/// The user name a join code logs in with.
pub const JOIN_USER: &str = "join";

/// A broker's certificate pin: the full 52 characters, or the 12 a join
/// code carries. Compared in constant time.
#[derive(Clone, PartialEq, Eq)]
pub struct Pin(String);

impl Pin {
    /// The full pin of a public key, from its SPKI in DER.
    pub fn of_spki(spki_der: &[u8]) -> Pin {
        let hash = digest::digest(&digest::SHA256, spki_der);
        Pin(encode(hash.as_ref(), FULL_PIN_CHARS))
    }

    /// The full pin of a certificate in DER, or `None` when its public key
    /// cannot be found in it.
    pub fn of_certificate(cert_der: &[u8]) -> Option<Pin> {
        spki_of_certificate(cert_der).map(Pin::of_spki)
    }

    /// Reads a pin as typed or read aloud: 52 characters (full) or 12 (the
    /// pin part of a join code). Errors are sentences, meaning first.
    pub fn parse(text: &str) -> Result<Pin, String> {
        let chars = normalize(text, "Pin error", "pins")?;
        match chars.len() {
            FULL_PIN_CHARS | SHORT_PIN_CHARS => Ok(Pin(chars)),
            0 => Err("Pin error: no pin was given.".into()),
            n => Err(format!(
                "Pin error: the pin has {n} characters; it needs {FULL_PIN_CHARS}, as the broker prints it."
            )),
        }
    }

    /// The pin's characters, without separators.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// True for the 12-character pin of a join code.
    pub fn is_short(&self) -> bool {
        self.0.len() == SHORT_PIN_CHARS
    }

    /// The first 12 characters, as a join code carries them.
    pub fn short(&self) -> Pin {
        Pin(self.0[..SHORT_PIN_CHARS].to_string())
    }

    /// The pin in groups of four, separated by dashes, for showing and
    /// reading aloud.
    pub fn grouped(&self) -> String {
        grouped(&self.0)
    }

    /// Whether a public key (SPKI in DER) has this pin: its full pin starts
    /// with this one. Constant time in the pin's characters.
    pub fn matches_spki(&self, spki_der: &[u8]) -> bool {
        let full = Pin::of_spki(spki_der);
        ct_eq(self.0.as_bytes(), &full.0.as_bytes()[..self.0.len()])
    }
}

impl fmt::Debug for Pin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pin({})", self.grouped())
    }
}

impl fmt::Display for Pin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.grouped())
    }
}

/// A join code: the 12-character short pin of the broker's per-session
/// certificate, then 4 random secret characters. It is a secret: `Debug`
/// hides it.
#[derive(Clone, PartialEq, Eq)]
pub struct JoinCode(String);

impl JoinCode {
    /// A new code for a broker whose certificate has `pin`, with a fresh
    /// secret from the system's secure random source.
    pub fn new(pin: &Pin) -> Result<JoinCode, String> {
        let mut bytes = [0u8; 4];
        ring::rand::SystemRandom::new()
            .fill(&mut bytes)
            .map_err(|_| {
                "Join code error: no secure random numbers on this computer.".to_string()
            })?;
        let secret = encode(&bytes, JOIN_SECRET_CHARS);
        Ok(JoinCode(format!("{}{secret}", &pin.0[..SHORT_PIN_CHARS])))
    }

    /// Reads a code as typed or read aloud: 16 characters, in any case,
    /// with or without separators. Errors are sentences, meaning first.
    pub fn parse(text: &str) -> Result<JoinCode, String> {
        let chars = normalize(text, "Join code error", "join codes")?;
        match chars.len() {
            JOIN_CODE_CHARS => Ok(JoinCode(chars)),
            0 => Err("Join code error: no code was given.".into()),
            n => Err(format!(
                "Join code error: the code has {n} characters; it needs {JOIN_CODE_CHARS}, in four groups of four."
            )),
        }
    }

    /// Reads a code from pasted text, which may hold more than the code:
    /// a join link ([`crate::JoinLink`]), whose code is taken; the code
    /// across line breaks; a label before it, such as `Join code:`;
    /// quotes around it; or the "Show the join code" list as copied, whose
    /// first line is the code. When none of those gives a code, the error
    /// is [`JoinCode::parse`]'s for the whole text.
    pub fn from_pasted(text: &str) -> Result<JoinCode, String> {
        if let Some(link) = crate::link::JoinLink::find(text) {
            return link.map(|l| l.code().clone());
        }
        let whole = match JoinCode::parse(unquote(text)) {
            Ok(code) => return Ok(code),
            Err(e) => e,
        };
        for line in text.lines() {
            let after_label = line.rsplit(':').next().unwrap_or(line);
            if let Ok(code) = JoinCode::parse(unquote(after_label)) {
                return Ok(code);
            }
        }
        Err(whole)
    }

    /// The short pin the code carries, to check the broker's certificate.
    pub fn pin(&self) -> Pin {
        Pin(self.0[..SHORT_PIN_CHARS].to_string())
    }

    /// The MQTT password: HKDF-SHA-256 of the whole code, as 64 hex
    /// digits. Needs the secret part, which the certificate never shows.
    pub fn password(&self) -> Secret {
        struct Len(usize);
        impl hkdf::KeyType for Len {
            fn len(&self) -> usize {
                self.0
            }
        }
        let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"textweaver join code v1");
        let prk = salt.extract(self.0.as_bytes());
        let mut out = [0u8; 32];
        // Expanding 32 bytes from SHA-256 cannot fail.
        if let Ok(okm) = prk.expand(&[b"mqtt password"], Len(32)) {
            let _ = okm.fill(&mut out);
        }
        Secret::new(out.iter().map(|b| format!("{b:02x}")).collect::<String>())
    }

    /// The code in four groups of four, such as `ABCD-EFGH-JKMN-PQRS`.
    pub fn grouped(&self) -> String {
        grouped(&self.0)
    }
}

impl fmt::Debug for JoinCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JoinCode((hidden))")
    }
}

/// `text` without the space, quotes, angle brackets and final period
/// that copying adds around a code.
fn unquote(text: &str) -> &str {
    const QUOTES: &[char] = &[
        '"', '\'', '<', '>', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}',
    ];
    text.trim()
        .trim_matches(QUOTES)
        .trim()
        .trim_end_matches('.')
        .trim()
}

/// Constant-time equality for secrets and pins.
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// The top `chars * 5` bits of `bytes` in base 32, padded with zero bits
/// when the bytes run out.
fn encode(bytes: &[u8], chars: usize) -> String {
    let mut out = String::with_capacity(chars);
    let mut acc: u32 = 0;
    let mut bits = 0;
    let mut it = bytes.iter();
    while out.len() < chars {
        if bits < 5 {
            acc = (acc << 8) | u32::from(it.next().copied().unwrap_or(0));
            bits += 8;
        }
        let idx = (acc >> (bits - 5)) & 31;
        bits -= 5;
        acc &= (1 << bits) - 1;
        out.push(char::from(ALPHABET[idx as usize]));
    }
    out
}

/// "ABCD-EFGH-..." for showing.
fn grouped(chars: &str) -> String {
    chars
        .as_bytes()
        .chunks(4)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect::<Vec<_>>()
        .join("-")
}

/// Uppercase, drop spaces, line breaks, dashes, dots, commas and
/// underscores, read O as zero
/// and I or L as one. Any other character is an error naming it and its
/// group of four.
fn normalize(text: &str, what: &str, kind: &str) -> Result<String, String> {
    if text.chars().count() > 200 {
        return Err(format!("{what}: the text is far too long."));
    }
    let mut out = String::new();
    for c in text.chars() {
        if matches!(c, ' ' | '-' | '.' | '_' | ',' | '\t' | '\r' | '\n') {
            continue;
        }
        let up = match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        };
        if up.is_ascii() && ALPHABET.contains(&(up as u8)) {
            out.push(up);
        } else {
            let group = out.len() / 4 + 1;
            let shown = if c.is_control() || !c.is_ascii_graphic() {
                format!("a character with code {}", u32::from(c))
            } else {
                c.to_string()
            };
            return Err(format!(
                "{what}: {shown} in group {group} is not used in {kind}. Check that character."
            ));
        }
    }
    Ok(out)
}

/// The SubjectPublicKeyInfo of an X.509 certificate in DER, with its tag
/// and length, as rustls-webpki and rcgen give it. Strict DER: definite,
/// minimal lengths that stay inside their parent. `None` for anything
/// else.
pub(crate) fn spki_of_certificate(der: &[u8]) -> Option<&[u8]> {
    let (cert, rest) = tlv(der, 0x30)?;
    if !rest.is_empty() {
        return None;
    }
    let (tbs, _) = tlv(value(cert)?, 0x30)?;
    let mut fields = value(tbs)?;
    // [0] EXPLICIT version, optional.
    if fields.first() == Some(&0xA0) {
        let (_, after) = tlv(fields, 0xA0)?;
        fields = after;
    }
    // serialNumber, signature, issuer, validity, subject.
    for tag in [0x02, 0x30, 0x30, 0x30, 0x30] {
        let (_, after) = tlv(fields, tag)?;
        fields = after;
    }
    let (spki, _) = tlv(fields, 0x30)?;
    Some(spki)
}

/// One element with tag `tag` at the start of `input`: the whole element
/// (tag, length and value) and what follows it.
fn tlv(input: &[u8], tag: u8) -> Option<(&[u8], &[u8])> {
    if *input.first()? != tag {
        return None;
    }
    let first = *input.get(1)?;
    let (len, header) = if first < 0x80 {
        (usize::from(first), 2)
    } else {
        let n = usize::from(first & 0x7f);
        if n == 0 || n > 4 {
            return None;
        }
        let bytes = input.get(2..2 + n)?;
        if bytes[0] == 0 {
            return None; // not minimal
        }
        let len = bytes
            .iter()
            .fold(0usize, |acc, b| (acc << 8) | usize::from(*b));
        if len < 0x80 {
            return None; // should have been the short form
        }
        (len, 2 + n)
    };
    let end = header.checked_add(len)?;
    if end > input.len() {
        return None;
    }
    Some((&input[..end], &input[end..]))
}

/// The value of an element [`tlv`] returned.
fn value(element: &[u8]) -> Option<&[u8]> {
    let first = *element.get(1)?;
    let header = if first < 0x80 {
        2
    } else {
        2 + usize::from(first & 0x7f)
    };
    element.get(header..)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_is_stable_and_padded() {
        assert_eq!(encode(&[0xff; 8], 12), "ZZZZZZZZZZZZ");
        assert_eq!(encode(&[0; 8], 12), "000000000000");
        // 256 bits in 52 characters: the last one carries 1 bit and 4 zero
        // bits of padding.
        let full = encode(&[0xff; 32], 52);
        assert_eq!(full.len(), 52);
        assert!(full.starts_with("ZZZZ"));
        assert!(full.ends_with('G'), "{full}"); // 10000 in binary
        assert_eq!(encode(&[0b1000_0100, 0b0010_0000], 3), "GGG");
    }

    #[test]
    fn join_codes_forgive_case_and_separators() {
        let c = JoinCode::parse("abcd-efgh jkmn.pqrs").unwrap();
        assert_eq!(c.grouped(), "ABCD-EFGH-JKMN-PQRS");
        assert_eq!(c.pin().as_str(), "ABCDEFGHJKMN");
        // O reads as zero, I and L as one.
        let o = JoinCode::parse("OOOO IIII LLLL 0000").unwrap();
        assert_eq!(o.grouped(), "0000-1111-1111-0000");
        assert_eq!(format!("{o:?}"), "JoinCode((hidden))");
    }

    #[test]
    fn join_code_typos_are_said_in_words() {
        let e = JoinCode::parse("ABCD-EFGH-JKMN-PQR").unwrap_err();
        assert_eq!(
            e,
            "Join code error: the code has 15 characters; it needs 16, in four groups of four."
        );
        let e = JoinCode::parse("ABCD-EFGH-JKUN-PQRS").unwrap_err();
        assert_eq!(
            e,
            "Join code error: U in group 3 is not used in join codes. Check that character."
        );
        let e = JoinCode::parse("ABCD\u{202e}").unwrap_err();
        assert!(e.contains("a character with code 8238"), "{e}");
        assert_eq!(
            JoinCode::parse(" - ").unwrap_err(),
            "Join code error: no code was given."
        );
    }

    #[test]
    fn pasted_codes_keep_only_the_code() {
        let want = JoinCode::parse("ABCD-EFGH-JKMN-PQRS").unwrap();
        for text in [
            "ABCD-EFGH-JKMN-PQRS",
            "ABCD\nEFGH\r\nJKMN\nPQRS\n",
            "Join code: ABCD-EFGH-JKMN-PQRS",
            "Join code: ABCD, EFGH, JKMN, PQRS.",
            "\"abcd-efgh-jkmn-pqrs\"",
            "\u{201c}ABCD EFGH JKMN PQRS\u{201d}",
            "ABCD-EFGH-JKMN-PQRS\nGroup 1 of 4: ABCD\nGroup 2 of 4: EFGH\nGroup 3 of 4: JKMN\nGroup 4 of 4: PQRS",
            "twjoin://10.0.0.5:8883/ABCD-EFGH-JKMN-PQRS?topic=a",
        ] {
            assert_eq!(JoinCode::from_pasted(text).as_ref(), Ok(&want), "{text:?}");
        }
        assert_eq!(
            JoinCode::from_pasted("ABCD-EFGH-JKUN-PQRS").unwrap_err(),
            "Join code error: U in group 3 is not used in join codes. Check that character."
        );
        assert!(
            JoinCode::from_pasted("twjoin://h/ABCD")
                .unwrap_err()
                .starts_with("Join code error: the code has 4")
        );
    }

    #[test]
    fn password_needs_the_whole_code() {
        let a = JoinCode::parse("AAAAAAAAAAAA0000").unwrap();
        let b = JoinCode::parse("AAAAAAAAAAAA0001").unwrap();
        assert_ne!(a.password(), b.password());
        assert_eq!(
            a.password(),
            JoinCode::parse("aaaa-aaaa-aaaa-oooo").unwrap().password()
        );
        assert_eq!(a.password().expose().len(), 64);
    }

    #[test]
    fn pins_parse_full_or_short() {
        let full = Pin::of_spki(b"some key");
        assert_eq!(full.as_str().len(), FULL_PIN_CHARS);
        let back = Pin::parse(&full.grouped().to_lowercase()).unwrap();
        assert_eq!(back, full);
        assert!(full.matches_spki(b"some key"));
        assert!(!full.matches_spki(b"another key"));
        let short = full.short();
        assert!(short.is_short());
        assert!(short.matches_spki(b"some key"));
        assert!(
            Pin::parse("ABCD")
                .unwrap_err()
                .starts_with("Pin error: the pin has 4")
        );
    }

    #[test]
    fn new_code_carries_the_pin() {
        let pin = Pin::of_spki(b"key");
        let code = JoinCode::new(&pin).unwrap();
        assert_eq!(code.pin(), pin.short());
        assert_eq!(code.grouped().len(), 19);
    }

    #[test]
    fn spki_walker_matches_rcgen() {
        use rcgen::PublicKeyData;
        let ck = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
        let spki = ck.signing_key.subject_public_key_info();
        let found = spki_of_certificate(ck.cert.der()).unwrap();
        assert_eq!(found, spki.as_slice());
        assert_eq!(
            Pin::of_certificate(ck.cert.der()).unwrap(),
            Pin::of_spki(&spki)
        );
    }

    #[test]
    fn spki_walker_refuses_garbage() {
        assert!(spki_of_certificate(&[]).is_none());
        assert!(spki_of_certificate(&[0x30, 0x80]).is_none());
        assert!(spki_of_certificate(&[0x30, 0x81, 0x05, 0, 0, 0, 0, 0]).is_none());
        assert!(spki_of_certificate(&[0x30, 0x84, 0xff, 0xff, 0xff, 0xff]).is_none());
        assert!(spki_of_certificate(&[0x30, 0x02, 0x30, 0x00]).is_none());
    }
}
