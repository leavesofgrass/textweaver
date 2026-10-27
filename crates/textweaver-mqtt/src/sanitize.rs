//! Hostile input is data. Caps on payload size, nesting depth, field count
//! and string length, checked before and after parsing; control
//! characters, terminal escape sequences and bidirectional overrides
//! stripped from every string and key before anything is shown, spoken or
//! written to Braille.

use std::collections::BTreeMap;

use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

use crate::Payload;

/// Caps on what one message may hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Largest payload accepted, in bytes (256 KiB). The client skips a
    /// bigger message as it streams in, without holding it in memory.
    pub max_payload: usize,
    /// Deepest nesting of JSON objects and lists (8).
    pub max_depth: usize,
    /// Most JSON values in one payload, every object member and list item
    /// counted (5000).
    pub max_fields: usize,
    /// Longest string kept, in characters (2000); longer ones are cut and
    /// end with "(cut)".
    pub max_string: usize,
    /// Longest text a live document message
    /// ([`crate::SharedDocument`]) keeps, in characters (250,000), with its
    /// line breaks: the one string allowed past `max_string`.
    pub max_document: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_payload: 256 * 1024,
            max_depth: 8,
            max_fields: 5000,
            max_string: 2000,
            max_document: 250_000,
        }
    }
}

/// Decodes a payload under the caps: JSON if it parses, else UTF-8 text,
/// else binary. Every string and key is cleaned with [`clean_text`].
pub fn decode_payload(payload: &[u8], limits: &Limits) -> Payload {
    let len = payload.len();
    if len > limits.max_payload {
        return Payload::TooLarge { len };
    }
    let Ok(text) = std::str::from_utf8(payload) else {
        return Payload::Binary { len };
    };
    // Depth is checked on the bytes first, so a deep payload never reaches
    // the recursive parser (serde_json's own limit is 128).
    if depth_exceeds(payload, limits.max_depth) {
        return Payload::TooLarge { len };
    }
    match serde_json::from_str::<Value>(text) {
        Ok(value) => {
            // A live document's text keeps its lines and its length.
            let document = crate::document::is_document(&value)
                .then(|| value.get("text").and_then(Value::as_str).map(str::to_owned))
                .flatten();
            let mut count = 0usize;
            match clean_value(value, limits, &mut count) {
                Some(mut v) => {
                    if let (Some(text), Some(obj)) = (document, v.as_object_mut()) {
                        let kept = clean_document_text(&text, limits.max_document);
                        obj.insert("text".into(), Value::String(kept));
                    }
                    Payload::Json(v)
                }
                None => Payload::TooLarge { len },
            }
        }
        Err(_) => Payload::Text(clean_text(text, limits.max_string)),
    }
}

/// The order a JSON payload's keys came in: for each object, by its JSON
/// Pointer (`""` for the payload itself, `"/checkins/0"` for the first
/// item of its `checkins` list), its keys as sent, each cleaned as the
/// decoded value's keys are. serde_json keeps keys sorted (textweaver
/// leaves its `preserve_order` feature off, since it would change every
/// crate's JSON output), so the order is read from the text beside it.
pub type KeyOrder = BTreeMap<String, Vec<String>>;

/// [`decode_payload`], with the key order of a JSON payload (empty for
/// any other).
pub fn decode_payload_ordered(payload: &[u8], limits: &Limits) -> (Payload, KeyOrder) {
    let decoded = decode_payload(payload, limits);
    let order = match &decoded {
        Payload::Json(_) => std::str::from_utf8(payload)
            .ok()
            .and_then(|text| key_order(text, limits.max_string))
            .unwrap_or_default(),
        _ => KeyOrder::new(),
    };
    (decoded, order)
}

/// The key order of `text`, a JSON document already checked against the
/// caps; `None` when it is not JSON.
fn key_order(text: &str, max_string: usize) -> Option<KeyOrder> {
    let mut out = KeyOrder::new();
    let mut de = serde_json::Deserializer::from_str(text);
    Walk {
        at: String::new(),
        out: &mut out,
        max_string,
    }
    .deserialize(&mut de)
    .ok()?;
    Some(out)
}

/// The JSON Pointer of member `key` of the object at `at` (RFC 6901).
fn child(at: &str, key: &str) -> String {
    format!("{at}/{}", key.replace('~', "~0").replace('/', "~1"))
}

/// Walks a JSON document, recording each object's cleaned keys under its
/// pointer; values are skipped.
struct Walk<'a> {
    at: String,
    out: &'a mut KeyOrder,
    max_string: usize,
}

impl<'de> DeserializeSeed<'de> for Walk<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        d.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Walk<'_> {
    type Value = ();

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("JSON")
    }

    fn visit_bool<E>(self, _: bool) -> Result<(), E> {
        Ok(())
    }

    fn visit_i64<E>(self, _: i64) -> Result<(), E> {
        Ok(())
    }

    fn visit_u64<E>(self, _: u64) -> Result<(), E> {
        Ok(())
    }

    fn visit_f64<E>(self, _: f64) -> Result<(), E> {
        Ok(())
    }

    fn visit_str<E>(self, _: &str) -> Result<(), E> {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        let Walk {
            at,
            out,
            max_string,
        } = self;
        let mut i = 0usize;
        while seq
            .next_element_seed(Walk {
                at: format!("{at}/{i}"),
                out: &mut *out,
                max_string,
            })?
            .is_some()
        {
            i += 1;
        }
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let Walk {
            at,
            out,
            max_string,
        } = self;
        let mut keys: Vec<String> = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            let key = clean_text(&key, max_string);
            map.next_value_seed(Walk {
                at: child(&at, &key),
                out: &mut *out,
                max_string,
            })?;
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        out.insert(at, keys);
        Ok(())
    }
}

fn depth_exceeds(bytes: &[u8], max: usize) -> bool {
    let (mut depth, mut in_string, mut escaped) = (0usize, false, false);
    for &b in bytes {
        if in_string {
            match (escaped, b) {
                (true, _) => escaped = false,
                (false, b'\\') => escaped = true,
                (false, b'"') => in_string = false,
                _ => {}
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > max {
                    return true;
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    false
}

/// Cleans a parsed value, or `None` over the field cap.
fn clean_value(value: Value, limits: &Limits, count: &mut usize) -> Option<Value> {
    *count += 1;
    if *count > limits.max_fields {
        return None;
    }
    Some(match value {
        Value::String(s) => Value::String(clean_text(&s, limits.max_string)),
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|v| clean_value(v, limits, count))
                .collect::<Option<_>>()?,
        ),
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                let key = clean_text(&k, limits.max_string);
                out.insert(key, clean_value(v, limits, count)?);
            }
            Value::Object(out)
        }
        other => other,
    })
}

/// Removes terminal escape sequences (CSI, OSC, DCS and the rest, in their
/// 7-bit and 8-bit forms), C0 and C1 control characters, DEL,
/// bidirectional embedding, override, isolate and mark characters, and the
/// byte order mark. Tabs and line breaks become single spaces, so an entry
/// stays on one line. Cuts the result to `max_chars` characters and ends
/// it with " (cut)" when it does.
pub fn clean_text(input: &str, max_chars: usize) -> String {
    clean(input, max_chars, false)
}

/// Cleans a shared document's text ([`crate::SharedDocument`]) as
/// [`clean_text`] cleans any string, but keeps its lines: `\r\n`, `\r`,
/// the next line character and the line and paragraph separators become
/// `\n`, and tabs and trailing spaces are kept. Cuts the text to
/// `max_chars` characters and ends it with a line saying "(cut)" when it
/// does.
pub fn clean_document_text(input: &str, max_chars: usize) -> String {
    clean(input, max_chars, true)
}

/// The cleaner behind [`clean_text`] (`lines` false) and
/// [`clean_document_text`] (`lines` true).
fn clean(input: &str, max_chars: usize, lines: bool) -> String {
    let mut out = String::with_capacity(input.len().min(max_chars.saturating_mul(4)));
    let mut chars = input.chars().peekable();
    let mut kept = 0usize;
    let mut cut = false;
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => {
                match chars.peek().copied() {
                    Some('[') => {
                        chars.next();
                        skip_csi(&mut chars);
                    }
                    Some(']' | 'P' | 'X' | '^' | '_') => {
                        chars.next();
                        skip_string(&mut chars);
                    }
                    // Two-character sequences, and the intermediate forms
                    // such as ESC ( B: drop the intermediates and the final.
                    Some(_) => {
                        while let Some(&n) = chars.peek() {
                            chars.next();
                            if !(' '..='/').contains(&n) {
                                break;
                            }
                        }
                    }
                    None => {}
                }
                continue;
            }
            '\u{9b}' => {
                skip_csi(&mut chars);
                continue;
            }
            '\u{9d}' | '\u{90}' | '\u{98}' | '\u{9e}' | '\u{9f}' => {
                skip_string(&mut chars);
                continue;
            }
            '\t' if lines => push(&mut out, '\t', &mut kept, max_chars, &mut cut),
            '\r' if lines => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                push(&mut out, '\n', &mut kept, max_chars, &mut cut);
            }
            '\n' | '\u{85}' | '\u{2028}' | '\u{2029}' if lines => {
                push(&mut out, '\n', &mut kept, max_chars, &mut cut);
            }
            '\t' | '\n' | '\r' | '\u{0b}' | '\u{0c}' | '\u{85}' | '\u{2028}' | '\u{2029}' => {
                if !out.ends_with(' ') && !out.is_empty() {
                    push(&mut out, ' ', &mut kept, max_chars, &mut cut);
                }
            }
            c if c.is_control() => {} // other C0, DEL, C1
            '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{061c}'
            | '\u{feff}' => {}
            c => push(&mut out, c, &mut kept, max_chars, &mut cut),
        }
        if cut {
            break;
        }
    }
    if lines {
        if cut {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("(cut)\n");
        }
        return out;
    }
    let mut out = out.trim_end().to_string();
    if cut {
        out.push_str(" (cut)");
    }
    out
}

fn push(out: &mut String, c: char, kept: &mut usize, max: usize, cut: &mut bool) {
    if *kept >= max {
        *cut = true;
        return;
    }
    out.push(c);
    *kept += 1;
}

/// CSI: parameter and intermediate bytes, then one final byte in @ to ~.
fn skip_csi(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    for c in chars.by_ref() {
        if ('@'..='~').contains(&c) {
            return;
        }
        if !(' '..='?').contains(&c) {
            return; // malformed: stop at anything that is not a CSI byte
        }
    }
}

/// OSC, DCS, SOS, PM, APC: up to BEL, ESC \ or the 8-bit ST.
fn skip_string(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    while let Some(c) = chars.next() {
        match c {
            '\u{07}' | '\u{9c}' => return,
            '\u{1b}' => {
                if chars.peek() == Some(&'\\') {
                    chars.next();
                }
                return;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean(s: &str) -> String {
        clean_text(s, 2000)
    }

    #[test]
    fn strips_escape_sequences() {
        assert_eq!(clean("N0CALL\u{1b}[31m red\u{1b}[0m"), "N0CALL red");
        assert_eq!(clean("a\u{1b}]0;window title\u{07}b"), "ab");
        assert_eq!(
            clean("a\u{1b}]8;;http://x\u{1b}\\link\u{1b}]8;;\u{1b}\\b"),
            "alinkb"
        );
        assert_eq!(clean("a\u{1b}Pq#0;2;0;0;0\u{1b}\\b"), "ab");
        assert_eq!(clean("a\u{9b}2Jb"), "ab");
        assert_eq!(clean("a\u{1b}(Bb"), "ab");
        assert_eq!(clean("a\u{1b}cb"), "ab");
        assert_eq!(clean("trailing\u{1b}"), "trailing");
    }

    #[test]
    fn strips_controls_and_bidi() {
        assert_eq!(clean("a\u{0}b\u{7}c\u{7f}d\u{85}e"), "abcd e");
        assert_eq!(clean("N0\u{202e}LLAC"), "N0LLAC");
        assert_eq!(clean("x\u{2066}y\u{2069}z\u{feff}"), "xyz");
        assert_eq!(clean("Mobile,\r\n\tportable"), "Mobile, portable");
        assert_eq!(
            clean("Ada \u{1f44b}\u{1f3fd} caf\u{e9}"),
            "Ada \u{1f44b}\u{1f3fd} caf\u{e9}"
        );
    }

    #[test]
    fn json_keeps_its_key_order_beside_the_value() {
        let (p, order) = decode_payload_ordered(
            br#"{"zeta":1,"call\u001b[2J":{"b":1,"a":2},"list":[{"y":1,"x":2}]}"#,
            &Limits::default(),
        );
        assert!(matches!(p, Payload::Json(_)));
        assert_eq!(order[""], ["zeta", "call", "list"]);
        assert_eq!(order["/call"], ["b", "a"]);
        assert_eq!(order["/list/0"], ["y", "x"]);
        let (p, order) = decode_payload_ordered(b"plain", &Limits::default());
        assert_eq!(p, Payload::Text("plain".into()));
        assert!(order.is_empty());
    }

    #[test]
    fn cuts_long_strings_in_words() {
        assert_eq!(clean_text("abcdefgh", 4), "abcd (cut)");
        assert_eq!(clean_text("abcd", 4), "abcd");
    }

    #[test]
    fn decodes_json_text_and_binary() {
        let l = Limits::default();
        assert_eq!(
            decode_payload(br#"{"call":"N0CALL"}"#, &l),
            Payload::Json(serde_json::json!({"call": "N0CALL"}))
        );
        assert_eq!(
            decode_payload(b"hello\x1b[2J there", &l),
            Payload::Text("hello there".into())
        );
        assert_eq!(
            decode_payload(&[0xff, 0xfe, 0x00], &l),
            Payload::Binary { len: 3 }
        );
    }

    #[test]
    fn caps_size_depth_and_fields() {
        let l = Limits {
            max_payload: 100,
            max_depth: 3,
            max_fields: 5,
            max_string: 50,
            max_document: 100,
        };
        assert_eq!(
            decode_payload(&[b' '; 101], &l),
            Payload::TooLarge { len: 101 }
        );
        assert_eq!(
            decode_payload(b"[[[[1]]]]", &l),
            Payload::TooLarge { len: 9 }
        );
        // Brackets inside strings do not count.
        assert!(matches!(
            decode_payload(br#"{"a":"[[[[[[["}"#, &l),
            Payload::Json(_)
        ));
        assert_eq!(
            decode_payload(b"[1,2,3,4,5]", &l),
            Payload::TooLarge { len: 11 }
        );
        assert_eq!(decode_payload(b"{", &l), Payload::Text("{".into()));
        // A deep payload is refused before the parser sees it.
        let deep = "[".repeat(100_000);
        let big = Limits {
            max_payload: 1 << 20,
            ..l
        };
        assert_eq!(
            decode_payload(deep.as_bytes(), &big),
            Payload::TooLarge { len: 100_000 }
        );
    }

    #[test]
    fn a_document_message_keeps_its_lines_and_a_larger_cap() {
        let l = Limits {
            max_string: 10,
            max_document: 40,
            ..Limits::default()
        };
        let msg = crate::document_message(
            2,
            "Agenda for the net",
            "markdown",
            "# Net\r\n\r\n- one\u{1b}[2J\n- two\u{202e}",
        );
        let Payload::Json(v) = decode_payload(msg.as_bytes(), &l) else {
            panic!("not JSON")
        };
        assert_eq!(v["text"], "# Net\n\n- one\n- two");
        // Every other string keeps the usual cap.
        assert_eq!(v["title"], "Agenda for (cut)");
        let long = crate::document_message(1, "t", "text", &"line\n".repeat(20));
        let Payload::Json(v) = decode_payload(long.as_bytes(), &l) else {
            panic!("not JSON")
        };
        let text = v["text"].as_str().unwrap();
        assert!(text.ends_with("\n(cut)\n"), "{text:?}");
        assert_eq!(text.lines().count(), 9, "{text:?}");
        // Not a document: line breaks become spaces.
        let Payload::Json(v) = decode_payload(br#"{"text":"a\nb"}"#, &l) else {
            panic!("not JSON")
        };
        assert_eq!(v["text"], "a b");
    }

    #[test]
    fn cleans_keys_and_values() {
        let Payload::Json(v) = decode_payload(
            br#"{"call\u001b[2J":"N0CALL\u001b]0;pwn\u0007","name":"Ada\u202e"}"#,
            &Limits::default(),
        ) else {
            panic!("not JSON")
        };
        assert_eq!(v["call"], "N0CALL");
        assert_eq!(v["name"], "Ada");
    }
}
