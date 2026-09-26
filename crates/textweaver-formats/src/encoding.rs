//! Character encodings: decoding source bytes to text.
//!
//! Star decoded everything as UTF-8 with replacement characters and never
//! set `Document.encoding` (docs/history/star-parity.md, Part 1 §7, quirk 10), so a
//! Windows-1252 text file or a Latin-1 web page lost every accented letter.
//! textweaver decides the encoding in this order:
//!
//! 1. a byte order mark (UTF-8, UTF-16LE, UTF-16BE);
//! 2. a declaration the caller found in the bytes (an HTML `<meta charset>`
//!    or `http-equiv` content type, an XML declaration), resolved with the
//!    WHATWG label table, so `iso-8859-1` means Windows-1252 as browsers
//!    read it; a declared UTF-16 without a BOM is ignored, as browsers do;
//! 3. UTF-8, when the bytes are valid UTF-8;
//! 4. Windows-1252, the usual encoding of legacy text on the machines
//!    students use (every byte decodes, so nothing is lost silently).
//!
//! The chosen encoding is reported in [`Decoded::encoding`]; loaders record
//! anything other than UTF-8 in `DocumentMeta::properties["encoding"]`.
//!
//! UTF-16 without a byte order mark (a NUL after or before nearly every
//! ASCII letter) is recognized between steps 2 and 3. Bytes that are not
//! text at all (a PDF, a zip archive, an image, a program) are recognized
//! by [`binary_kind`], so the text loaders can refuse them instead of
//! reading them aloud as garbage (docs/history/audit-2026-09.md, finding D3).

use encoding_rs::{Encoding, UTF_8, UTF_16BE, UTF_16LE, WINDOWS_1252};

/// Text decoded from bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decoded {
    /// The text, without a byte order mark, line endings unchanged.
    pub text: String,
    /// WHATWG name of the encoding used ("UTF-8", "windows-1252", ...).
    pub encoding: &'static str,
    /// True when some bytes were malformed and replaced with U+FFFD.
    pub had_errors: bool,
}

impl Decoded {
    /// True when the text was not UTF-8 (worth recording in the metadata).
    pub fn is_legacy(&self) -> bool {
        self.encoding != UTF_8.name()
    }
}

/// Decodes `bytes` by BOM, then the `declared` label, then UTF-8 when valid,
/// then Windows-1252.
pub fn decode(bytes: &[u8], declared: Option<&str>) -> Decoded {
    if let Some((enc, bom)) = Encoding::for_bom(bytes) {
        return run(enc, &bytes[bom..]);
    }
    if let Some(enc) = declared.and_then(|l| Encoding::for_label(l.trim().as_bytes()))
        && enc != UTF_16LE
        && enc != UTF_16BE
    {
        return run(enc.output_encoding(), bytes);
    }
    if let Some(enc) = utf16_without_bom(bytes) {
        return run(enc, bytes);
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => Decoded {
            text: s.to_owned(),
            encoding: UTF_8.name(),
            had_errors: false,
        },
        Err(_) => run(WINDOWS_1252, bytes),
    }
}

/// How much of the start of a file is examined to tell text from binary.
pub const SNIFF_BYTES: usize = 8192;

/// UTF-16 without a byte order mark: in the first [`SNIFF_BYTES`], most
/// code units have a NUL high byte (ASCII-range text), on the same side.
fn utf16_without_bom(bytes: &[u8]) -> Option<&'static Encoding> {
    let head = &bytes[..bytes.len().min(SNIFF_BYTES) & !1];
    let units = head.len() / 2;
    if units < 2 {
        return None;
    }
    let (mut even, mut odd) = (0usize, 0usize);
    for pair in head.chunks_exact(2) {
        even += usize::from(pair[0] == 0 && pair[1] != 0);
        odd += usize::from(pair[1] == 0 && pair[0] != 0);
    }
    // Nine in ten units ASCII-range, with the NUL on one side.
    if odd * 10 >= units * 9 && even * 20 <= units {
        Some(UTF_16LE)
    } else if even * 10 >= units * 9 && odd * 20 <= units {
        Some(UTF_16BE)
    } else {
        None
    }
}

/// What kind of non-text file `bytes` look like, or `None` for text.
///
/// Known signatures are named ("a PDF document", "a zip archive (such as
/// a Word document or an EPUB)", "an image", "a program"); other bytes
/// count as binary when the first [`SNIFF_BYTES`] hold a NUL byte (and are
/// not UTF-16) or more than one control character in twenty. Text with a
/// byte order mark is always text.
pub fn binary_kind(bytes: &[u8]) -> Option<&'static str> {
    if Encoding::for_bom(bytes).is_some() || utf16_without_bom(bytes).is_some() {
        return None;
    }
    const SIGNATURES: &[(&[u8], &str)] = &[
        (b"%PDF-", "a PDF document"),
        (
            b"PK\x03\x04",
            "a zip archive (such as a Word document or an EPUB)",
        ),
        (b"\x89PNG", "an image"),
        (b"\xff\xd8\xff", "an image"),
        (b"GIF8", "an image"),
        (b"RIFF", "an audio, video, or image file"),
        (b"ID3", "an audio file"),
        (b"OggS", "an audio file"),
        (b"fLaC", "an audio file"),
        (b"\x7fELF", "a program"),
        (b"MZ", "a program"),
        (b"\xd0\xcf\x11\xe0", "an old Microsoft Office document"),
        (b"{\\rtf", "a rich text (RTF) document"),
    ];
    for (magic, kind) in SIGNATURES {
        if bytes.starts_with(magic) {
            // "MZ" and "RIFF" are short: only when the rest is not text.
            if (*magic == b"MZ" || *magic == b"RIFF") && binary_by_content(bytes).is_none() {
                continue;
            }
            return Some(kind);
        }
    }
    binary_by_content(bytes)
}

fn binary_by_content(bytes: &[u8]) -> Option<&'static str> {
    let head = &bytes[..bytes.len().min(SNIFF_BYTES)];
    if head.is_empty() {
        return None;
    }
    if head.contains(&0) {
        return Some("a binary file");
    }
    let controls = head
        .iter()
        .filter(|&&b| b < 0x20 && !matches!(b, b'\t' | b'\n' | b'\r' | 0x0c | 0x1b))
        .count();
    (controls * 20 > head.len()).then_some("a binary file")
}

fn run(enc: &'static Encoding, bytes: &[u8]) -> Decoded {
    let (text, had_errors) = enc.decode_without_bom_handling(bytes);
    Decoded {
        text: text.into_owned(),
        encoding: enc.name(),
        had_errors,
    }
}

/// The charset an HTML document declares in its first 1024 bytes:
/// `<meta charset="…">` or `<meta http-equiv="Content-Type"
/// content="text/html; charset=…">`, or an XML declaration (XHTML).
pub fn sniff_html_charset(bytes: &[u8]) -> Option<String> {
    let head = &bytes[..bytes.len().min(1024)];
    let lower: Vec<u8> = head.iter().map(u8::to_ascii_lowercase).collect();
    let mut at = 0;
    while let Some(i) = find(&lower[at..], b"<meta") {
        let start = at + i;
        let end = lower[start..]
            .iter()
            .position(|&b| b == b'>')
            .map_or(lower.len(), |e| start + e);
        let tag = &lower[start..end];
        if let Some(c) = find(tag, b"charset") {
            let rest = &tag[c + b"charset".len()..];
            let rest = trim_start(rest);
            if let Some(rest) = rest.strip_prefix(b"=")
                && let Some(v) = attr_value(trim_start(rest))
            {
                return Some(v);
            }
        }
        at = end.max(start + 1);
    }
    sniff_xml_encoding(bytes)
}

/// The `encoding` of an XML declaration (`<?xml version="1.0"
/// encoding="…"?>`) at the start of `bytes`.
pub fn sniff_xml_encoding(bytes: &[u8]) -> Option<String> {
    let head = &bytes[..bytes.len().min(256)];
    let head = trim_start(head);
    if !head.starts_with(b"<?xml") {
        return None;
    }
    let end = find(head, b"?>")?;
    let decl = &head[..end];
    let e = find(decl, b"encoding")?;
    let rest = trim_start(&decl[e + b"encoding".len()..]);
    let rest = trim_start(rest.strip_prefix(b"=")?);
    attr_value(rest)
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn trim_start(s: &[u8]) -> &[u8] {
    let n = s.iter().take_while(|b| b.is_ascii_whitespace()).count();
    &s[n..]
}

/// A quoted or bare attribute value at the start of `s`.
fn attr_value(s: &[u8]) -> Option<String> {
    let (quote, body) = match s.first() {
        Some(&q @ (b'"' | b'\'')) => (Some(q), &s[1..]),
        _ => (None, s),
    };
    let len = body
        .iter()
        .position(|&b| match quote {
            Some(q) => b == q,
            None => b.is_ascii_whitespace() || matches!(b, b';' | b'"' | b'\'' | b'/' | b'>'),
        })
        .unwrap_or(body.len());
    let v = std::str::from_utf8(&body[..len]).ok()?.trim();
    (!v.is_empty()).then(|| v.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_without_a_bom_is_recognized() {
        let le: Vec<u8> = "Hello, world.\nSecond line."
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let d = decode(&le, None);
        assert_eq!(
            (d.text.as_str(), d.encoding),
            ("Hello, world.\nSecond line.", "UTF-16LE")
        );
        let be: Vec<u8> = "Hi there"
            .encode_utf16()
            .flat_map(u16::to_be_bytes)
            .collect();
        let d = decode(&be, None);
        assert_eq!((d.text.as_str(), d.encoding), ("Hi there", "UTF-16BE"));
        assert_eq!(binary_kind(&le), None);
    }

    #[test]
    fn binary_files_are_named_and_text_is_not_binary() {
        assert_eq!(binary_kind(b"%PDF-1.7\n%\xe2\xe3"), Some("a PDF document"));
        assert!(binary_kind(b"PK\x03\x04\x14\x00").unwrap().contains("zip"));
        assert_eq!(binary_kind(b"\x89PNG\r\n\x1a\n\0\0"), Some("an image"));
        assert_eq!(binary_kind(b"abc\0def"), Some("a binary file"));
        assert_eq!(binary_kind(b"\x01\x02\x03\x04 text"), Some("a binary file"));
        // Text, including text that happens to start like a signature.
        assert_eq!(binary_kind(b"MZ is a postcode area.\n"), None);
        assert_eq!(binary_kind(b"RIFF raff and friends.\n"), None);
        assert_eq!(
            binary_kind("caf\u{e9} \u{2014} na\u{ef}ve\n".as_bytes()),
            None
        );
        assert_eq!(binary_kind(b"caf\xe9 in Windows-1252\n"), None);
        assert_eq!(binary_kind(b"\x1b[1mbold\x1b[0m\tand\x0c\n"), None);
        assert_eq!(binary_kind(b"\xff\xfeh\x00\x00\x00"), None);
        assert_eq!(binary_kind(b""), None);
    }

    #[test]
    fn bom_declared_utf8_then_windows_1252() {
        let d = decode(b"\xef\xbb\xbfcaf\xc3\xa9", Some("iso-8859-1"));
        assert_eq!((d.text.as_str(), d.encoding), ("café", "UTF-8"));
        let d = decode(b"\xff\xfeh\x00i\x00", None);
        assert_eq!((d.text.as_str(), d.encoding), ("hi", "UTF-16LE"));
        let d = decode(b"caf\xe9", Some("latin1"));
        assert_eq!((d.text.as_str(), d.encoding), ("café", "windows-1252"));
        assert!(d.is_legacy());
        let d = decode("café".as_bytes(), None);
        assert_eq!((d.text.as_str(), d.encoding), ("café", "UTF-8"));
        assert!(!d.is_legacy());
        // Not UTF-8, nothing declared: Windows-1252, curly quotes included.
        let d = decode(b"\x93caf\xe9\x94", None);
        assert_eq!(d.text, "\u{201c}café\u{201d}");
        assert!(!d.had_errors);
        // A declared UTF-16 without a BOM is ignored.
        let d = decode(b"plain", Some("utf-16"));
        assert_eq!(d.encoding, "UTF-8");
        // An unknown label is ignored.
        assert_eq!(decode(b"x", Some("klingon")).encoding, "UTF-8");
    }

    #[test]
    fn sniffs_meta_and_xml_declarations() {
        assert_eq!(
            sniff_html_charset(b"<html><head><META CHARSET='Windows-1252'>").as_deref(),
            Some("windows-1252")
        );
        assert_eq!(
            sniff_html_charset(
                b"<meta name=x><meta http-equiv=\"Content-Type\" content=\"text/html; charset=ISO-8859-1\">"
            )
            .as_deref(),
            Some("iso-8859-1")
        );
        assert_eq!(
            sniff_html_charset(b"<?xml version=\"1.0\" encoding=\"koi8-r\"?><html>").as_deref(),
            Some("koi8-r")
        );
        assert_eq!(sniff_html_charset(b"<html><body>no charset</body>"), None);
        assert_eq!(sniff_xml_encoding(b"<?xml version='1.0'?><a/>"), None);
    }
}
