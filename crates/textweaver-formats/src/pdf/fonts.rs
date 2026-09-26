//! Fonts: turning shown string bytes into Unicode text and glyph advances,
//! and telling bold, italic, and monospaced faces apart.
//!
//! Decoding prefers the font's `/ToUnicode` CMap, then its `/Encoding`
//! (base encodings and `/Differences` through lopdf's glyph-name table),
//! then Standard encoding. Widths come from `/Widths` (simple fonts) or
//! `/W` and `/DW` (composite fonts), else from the standard 14 font metrics
//! ([`super::metrics`]) for fonts that omit them.

use std::collections::HashMap;

use lopdf::{Dictionary, Document, Encoding, Object};

use super::metrics::WIDTHS;

/// Style bits of a font.
pub(super) const BOLD: u8 = 1;
/// Italic or oblique.
pub(super) const ITALIC: u8 = 2;
/// Fixed pitch (code).
pub(super) const MONO: u8 = 4;

/// Which standard-font width column to use when a font has no widths.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Fallback {
    /// A column of [`WIDTHS`].
    Table(usize),
    /// Every glyph is this wide (Courier: 600).
    Fixed(f32),
}

/// A decoded code: its Unicode text and its width in text space units per
/// unit of font size (glyph width / 1000 for most fonts).
#[derive(Clone, Debug)]
pub(super) struct Code {
    pub text: Box<str>,
    pub width: f32,
    /// True for the single-byte code 32, to which word spacing applies.
    pub is_space_code: bool,
}

/// A font ready for text extraction.
pub(super) struct Font {
    /// Composite (Type0) fonts use two-byte codes.
    two_byte: bool,
    /// Decoded single-byte codes (simple fonts).
    table: Vec<Code>,
    /// Decoding for two-byte codes, and its cache.
    unicode: Option<Encoding<'static>>,
    cid_widths: HashMap<u32, f32>,
    default_width: f32,
    cache: HashMap<u32, Code>,
    /// Style bits ([`BOLD`], [`ITALIC`], [`MONO`]).
    pub style: u8,
}

impl Font {
    /// A font for a missing or broken font resource: single-byte codes read
    /// as Windows-1252 with an average width.
    pub(super) fn fallback() -> Font {
        let table = (0u32..256)
            .map(|c| {
                let text = win_ansi(c as u8);
                Code {
                    width: 0.5,
                    is_space_code: c == 32,
                    text,
                }
            })
            .collect();
        Font {
            two_byte: false,
            table,
            unicode: None,
            cid_widths: HashMap::new(),
            default_width: 0.5,
            cache: HashMap::new(),
            style: 0,
        }
    }

    /// Builds the font from its dictionary.
    pub(super) fn load(doc: &Document, dict: &Dictionary) -> Font {
        let subtype = dict
            .get(b"Subtype")
            .and_then(Object::as_name)
            .unwrap_or(b"");
        let base = base_font_name(doc, dict);
        let descriptor = descriptor(doc, dict);
        let style = style_of(&base, descriptor);
        if subtype == b"Type0" {
            return Self::load_type0(doc, dict, style);
        }

        // Unicode for each single-byte code: ToUnicode first, else /Encoding.
        let to_unicode = to_unicode_encoding(doc, dict);
        let encoding = dict.get_font_encoding(doc).ok();
        // Glyph space to text space (Type3 fonts carry their own matrix).
        let scale = if subtype == b"Type3" {
            dict.get(b"FontMatrix")
                .and_then(Object::as_array)
                .ok()
                .and_then(|m| m.first())
                .and_then(|a| num(doc, a))
                .unwrap_or(0.001)
        } else {
            0.001
        };
        let first = dict
            .get(b"FirstChar")
            .ok()
            .and_then(|o| num(doc, o))
            .map_or(0, |f| f.max(0.0) as u32);
        let widths: Vec<f32> = dict
            .get_deref(b"Widths", doc)
            .and_then(Object::as_array)
            .map(|a| {
                a.iter()
                    .map(|w| num(doc, w).unwrap_or(0.0) * scale)
                    .collect()
            })
            .unwrap_or_default();
        let missing = descriptor
            .and_then(|d| d.get(b"MissingWidth").ok())
            .and_then(|o| num(doc, o))
            .map(|w| w * scale);
        let fallback = fallback_widths(&base);
        let table = (0u32..256)
            .map(|c| {
                let byte = c as u8;
                let mut text = to_unicode
                    .as_ref()
                    .map(|e| decode_one(e, byte))
                    .filter(|t| !t.is_empty())
                    .or_else(|| {
                        encoding
                            .as_ref()
                            .map(|e| decode_one(e, byte))
                            .filter(|t| !t.is_empty())
                    })
                    .unwrap_or_else(|| standard(byte));
                if text.chars().any(|ch| ch.is_control() && ch != '\t') {
                    text = "".into();
                }
                let width = c
                    .checked_sub(first)
                    .and_then(|i| widths.get(i as usize).copied())
                    .filter(|w| *w > 0.0)
                    .or(missing)
                    .unwrap_or_else(|| fallback_width(fallback, &text));
                Code {
                    text,
                    width,
                    is_space_code: c == 32,
                }
            })
            .collect();
        Font {
            two_byte: false,
            table,
            unicode: None,
            cid_widths: HashMap::new(),
            default_width: 0.5,
            cache: HashMap::new(),
            style,
        }
    }

    fn load_type0(doc: &Document, dict: &Dictionary, style: u8) -> Font {
        let unicode = to_unicode_encoding(doc, dict);
        let descendant = dict
            .get_deref(b"DescendantFonts", doc)
            .and_then(Object::as_array)
            .ok()
            .and_then(|a| a.first())
            .and_then(|o| doc.dereference(o).ok())
            .and_then(|(_, o)| o.as_dict().ok());
        let default_width = descendant
            .and_then(|d| d.get(b"DW").ok())
            .and_then(|o| num(doc, o))
            .unwrap_or(1000.0)
            / 1000.0;
        let mut cid_widths = HashMap::new();
        if let Some(w) = descendant
            .and_then(|d| d.get_deref(b"W", doc).ok())
            .and_then(|o| o.as_array().ok())
        {
            let mut i = 0;
            while i < w.len() {
                let Some(first) = num(doc, &w[i]) else { break };
                let first = first as u32;
                match w.get(i + 1).map(|o| doc.dereference(o).map(|(_, o)| o)) {
                    Some(Ok(Object::Array(list))) => {
                        for (k, v) in list.iter().enumerate() {
                            if let Some(v) = num(doc, v) {
                                cid_widths.insert(first + k as u32, v / 1000.0);
                            }
                        }
                        i += 2;
                    }
                    Some(Ok(last)) => {
                        let last = num(doc, last).map_or(first, |l| l as u32);
                        let v = w.get(i + 2).and_then(|o| num(doc, o)).unwrap_or(1000.0);
                        // Guard against absurd ranges in broken files.
                        for c in first..=last.min(first.saturating_add(65_535)) {
                            cid_widths.insert(c, v / 1000.0);
                        }
                        i += 3;
                    }
                    _ => break,
                }
            }
        }
        Font {
            two_byte: true,
            table: Vec::new(),
            unicode,
            cid_widths,
            default_width,
            cache: HashMap::new(),
            style,
        }
    }

    /// Calls `f` with each code of `bytes`, in order.
    pub(super) fn for_each_code(&mut self, bytes: &[u8], mut f: impl FnMut(&Code)) {
        if !self.two_byte {
            for &b in bytes {
                f(&self.table[usize::from(b)]);
            }
            return;
        }
        for pair in bytes.chunks(2) {
            let code = pair.iter().fold(0u32, |acc, &b| (acc << 8) | u32::from(b));
            if !self.cache.contains_key(&code) {
                let text: Box<str> = self
                    .unicode
                    .as_ref()
                    .and_then(|e| e.bytes_to_string(pair).ok())
                    .map(|s| s.replace(|c: char| c.is_control() || c == '\u{fffd}', ""))
                    .unwrap_or_default()
                    .into();
                let width = self
                    .cid_widths
                    .get(&code)
                    .copied()
                    .unwrap_or(self.default_width);
                self.cache.insert(
                    code,
                    Code {
                        text,
                        width,
                        is_space_code: false,
                    },
                );
            }
            if let Some(c) = self.cache.get(&code) {
                f(c);
            }
        }
    }
}

/// A byte as Windows-1252 (WinAnsiEncoding differs only in unused codes).
fn win_ansi(byte: u8) -> Box<str> {
    if byte < 0x20 {
        return "".into();
    }
    let bytes = [byte];
    let (text, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
    text.into_owned().into()
}

/// A byte in Adobe StandardEncoding (the built-in encoding of most Type 1
/// text fonts), for fonts lopdf cannot decode.
fn standard(byte: u8) -> Box<str> {
    let c = match byte {
        0x27 => '\u{2019}',
        0x60 => '\u{2018}',
        0x20..=0x7e => char::from(byte),
        _ => STANDARD_HIGH
            .iter()
            .find(|(b, _)| *b == byte)
            .map_or('\0', |(_, c)| *c),
    };
    if c == '\0' {
        "".into()
    } else {
        c.to_string().into()
    }
}

/// The upper half of StandardEncoding.
const STANDARD_HIGH: &[(u8, char)] = &[
    (0xa1, '¡'),
    (0xa2, '¢'),
    (0xa3, '£'),
    (0xa4, '\u{2044}'),
    (0xa5, '¥'),
    (0xa6, 'ƒ'),
    (0xa7, '§'),
    (0xa8, '¤'),
    (0xa9, '\''),
    (0xaa, '\u{201c}'),
    (0xab, '«'),
    (0xac, '\u{2039}'),
    (0xad, '\u{203a}'),
    (0xae, '\u{fb01}'),
    (0xaf, '\u{fb02}'),
    (0xb1, '\u{2013}'),
    (0xb2, '\u{2020}'),
    (0xb3, '\u{2021}'),
    (0xb4, '·'),
    (0xb6, '¶'),
    (0xb7, '\u{2022}'),
    (0xb8, '\u{201a}'),
    (0xb9, '\u{201e}'),
    (0xba, '\u{201d}'),
    (0xbb, '»'),
    (0xbc, '\u{2026}'),
    (0xbd, '\u{2030}'),
    (0xbf, '¿'),
    (0xd0, '\u{2014}'),
    (0xe1, 'Æ'),
    (0xe3, 'ª'),
    (0xe8, 'Ł'),
    (0xe9, 'Ø'),
    (0xea, 'Œ'),
    (0xeb, 'º'),
    (0xf1, 'æ'),
    (0xf5, 'ı'),
    (0xf8, 'ł'),
    (0xf9, 'ø'),
    (0xfa, 'œ'),
    (0xfb, 'ß'),
];

fn num(doc: &Document, o: &Object) -> Option<f32> {
    let o = doc.dereference(o).ok()?.1;
    match o {
        Object::Integer(i) => Some(*i as f32),
        Object::Real(r) => Some(*r),
        _ => None,
    }
}

fn decode_one(enc: &Encoding<'_>, byte: u8) -> Box<str> {
    enc.bytes_to_string(&[byte]).unwrap_or_default().into()
}

/// The font's ToUnicode CMap as an owned encoding (lopdf reads `/Encoding`
/// first, so the dictionary is cloned without it).
fn to_unicode_encoding(doc: &Document, dict: &Dictionary) -> Option<Encoding<'static>> {
    dict.get(b"ToUnicode").ok()?;
    let mut d = dict.clone();
    d.remove(b"Encoding");
    d.set("Type", Object::Name(b"Font".to_vec()));
    match d.get_font_encoding(doc).ok()? {
        Encoding::UnicodeMapEncoding(cmap) => Some(Encoding::UnicodeMapEncoding(cmap)),
        _ => None,
    }
}

/// `/BaseFont` without a subset prefix (`ABCDEF+Arial-BoldMT` → `Arial-BoldMT`).
pub(super) fn base_font_name(doc: &Document, dict: &Dictionary) -> String {
    let name = dict
        .get_deref(b"BaseFont", doc)
        .and_then(Object::as_name)
        .map(|n| String::from_utf8_lossy(n).into_owned())
        .unwrap_or_default();
    match name.split_once('+') {
        Some((prefix, rest))
            if prefix.len() == 6 && prefix.chars().all(|c| c.is_ascii_uppercase()) =>
        {
            rest.to_owned()
        }
        _ => name,
    }
}

fn descriptor<'a>(doc: &'a Document, dict: &'a Dictionary) -> Option<&'a Dictionary> {
    let own = dict
        .get_deref(b"FontDescriptor", doc)
        .and_then(Object::as_dict)
        .ok();
    own.or_else(|| {
        dict.get_deref(b"DescendantFonts", doc)
            .and_then(Object::as_array)
            .ok()?
            .first()
            .and_then(|o| doc.dereference(o).ok())
            .and_then(|(_, o)| o.as_dict().ok())?
            .get_deref(b"FontDescriptor", doc)
            .and_then(Object::as_dict)
            .ok()
    })
}

/// Bold, italic, and fixed-pitch bits from the font name and descriptor.
fn style_of(base: &str, descriptor: Option<&Dictionary>) -> u8 {
    let lower = base.to_ascii_lowercase();
    let mut style = 0;
    let weight = descriptor
        .and_then(|d| d.get(b"FontWeight").ok())
        .and_then(|o| match o {
            Object::Integer(i) => Some(*i as f32),
            Object::Real(r) => Some(*r),
            _ => None,
        });
    let flags = descriptor
        .and_then(|d| d.get(b"Flags").ok())
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(0);
    let bold_name = [
        "bold",
        "black",
        "heavy",
        "semibold",
        "demibold",
        "demi",
        "extrabold",
    ]
    .iter()
    .any(|w| lower.contains(w))
        || lower.ends_with(",b")
        || lower.ends_with("-b");
    if bold_name || weight.is_some_and(|w| w >= 600.0) || flags & (1 << 18) != 0 {
        style |= BOLD;
    }
    let italic_angle = descriptor
        .and_then(|d| d.get(b"ItalicAngle").ok())
        .and_then(|o| match o {
            Object::Integer(i) => Some(*i as f32),
            Object::Real(r) => Some(*r),
            _ => None,
        })
        .unwrap_or(0.0);
    if lower.contains("italic")
        || lower.contains("oblique")
        || lower.ends_with(",i")
        || flags & (1 << 6) != 0
        || italic_angle.abs() > 1.0
    {
        style |= ITALIC;
    }
    if flags & 1 != 0
        || [
            "courier",
            "mono",
            "consol",
            "menlo",
            "inconsolata",
            "sourcecodepro",
            "fixed",
        ]
        .iter()
        .any(|w| lower.contains(w))
    {
        style |= MONO;
    }
    style
}

fn fallback_widths(base: &str) -> Fallback {
    let lower = base.to_ascii_lowercase();
    if lower.contains("courier") || lower.contains("mono") {
        return Fallback::Fixed(0.6);
    }
    let bold = lower.contains("bold") || lower.contains("black");
    let italic = lower.contains("italic") || lower.contains("oblique");
    if lower.contains("times") || lower.contains("serif") && !lower.contains("sans") {
        return Fallback::Table(match (bold, italic) {
            (false, false) => 2,
            (true, false) => 3,
            (false, true) => 4,
            (true, true) => 5,
        });
    }
    Fallback::Table(usize::from(bold))
}

fn fallback_width(f: Fallback, text: &str) -> f32 {
    match f {
        Fallback::Fixed(w) => w,
        Fallback::Table(col) => {
            let Some(c) = text.chars().next() else {
                return 0.25;
            };
            WIDTHS
                .binary_search_by_key(&c, |(ch, _)| *ch)
                .map_or(0.5, |i| f32::from(WIDTHS[i].1[col]) / 1000.0)
        }
    }
}
