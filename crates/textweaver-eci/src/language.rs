//! ECI language dialects and text encoding.
//!
//! ECI takes text as bytes in the language's legacy code page, not UTF-8.
//! Every Western language (English, Spanish, French, German, Italian,
//! Portuguese, Finnish, and the Nordic and Dutch engines) takes
//! **Windows-1252**. The Chinese, Japanese, Korean, and Thai engines take
//! GB 2312, Big5, Shift-JIS, UHC, or TIS-620; textweaver does not encode
//! those yet, so their dialects are listed as unsupported and never offered
//! as voices.
//!
//! Encoding rules ([`encode_cp1252`]):
//! - ASCII and Latin-1 (U+00A0..U+00FF) map to themselves; the typographic
//!   characters Windows-1252 places in 0x80..0x9F (curly quotes, dashes,
//!   ellipsis, euro, trade mark, Œ, Š, Ž, Ÿ, ...) map to those bytes.
//! - Latin Extended-A letters outside Windows-1252 (ā, č, ł, ő, ...) become
//!   their base letter, so names are still pronounced.
//! - Other dashes, minus signs, and hyphens become `-`; other spaces become a
//!   space; primes become `'` and `"`.
//! - Control characters (other than tab, line feed, and carriage return),
//!   NUL, and every other unrepresentable character become a space, so the
//!   engine never speaks a stray replacement "question mark".
//! - The backquote starts an ECI annotation, so it becomes `'` and text can
//!   never inject engine commands.

/// One ECI language dialect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dialect {
    /// The `ECILanguageDialect` code (`major << 16 | minor`).
    pub code: u32,
    /// BCP 47 tag.
    pub tag: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    /// Whether textweaver can encode text for it (Windows-1252 languages).
    pub supported: bool,
}

const fn d(code: u32, tag: &'static str, name: &'static str, supported: bool) -> Dialect {
    Dialect {
        code,
        tag,
        name,
        supported,
    }
}

/// Every dialect ECI defines that textweaver knows about.
pub const DIALECTS: &[Dialect] = &[
    d(0x0001_0000, "en-US", "English (United States)", true),
    d(0x0001_0001, "en-GB", "English (United Kingdom)", true),
    d(0x0002_0000, "es-ES", "Spanish (Spain)", true),
    d(0x0002_0001, "es-MX", "Spanish (Mexico)", true),
    d(0x0003_0000, "fr-FR", "French (France)", true),
    d(0x0003_0001, "fr-CA", "French (Canada)", true),
    d(0x0004_0000, "de-DE", "German", true),
    d(0x0005_0000, "it-IT", "Italian", true),
    d(0x0006_0000, "zh-CN", "Chinese (Mandarin)", false),
    d(0x0006_0001, "zh-TW", "Chinese (Taiwan Mandarin)", false),
    d(0x0007_0000, "pt-BR", "Portuguese (Brazil)", true),
    d(0x0007_0001, "pt-PT", "Portuguese (Portugal)", true),
    d(0x0008_0000, "ja-JP", "Japanese", false),
    d(0x0009_0000, "fi-FI", "Finnish", true),
    d(0x000A_0000, "ko-KR", "Korean", false),
    d(0x000B_0000, "yue-CN", "Cantonese", false),
    d(0x000B_0001, "yue-HK", "Cantonese (Hong Kong)", false),
    d(0x000C_0000, "nl-NL", "Dutch", true),
    d(0x000D_0000, "nb-NO", "Norwegian", true),
    d(0x000E_0000, "sv-SE", "Swedish", true),
    d(0x000F_0000, "da-DK", "Danish", true),
    d(0x0011_0000, "th-TH", "Thai", false),
];

/// American English, ECI's default.
pub const DEFAULT_DIALECT: u32 = 0x0001_0000;

/// The dialect with ECI code `code`.
pub fn dialect_by_code(code: u32) -> Option<&'static Dialect> {
    DIALECTS.iter().find(|d| d.code == code)
}

/// The dialect for a BCP 47 tag, matched case-insensitively; a bare
/// language ("en", "de") picks that language's first dialect.
pub fn dialect_by_tag(tag: &str) -> Option<&'static Dialect> {
    let tag = tag.trim().replace('_', "-");
    DIALECTS
        .iter()
        .find(|d| d.tag.eq_ignore_ascii_case(&tag))
        .or_else(|| {
            DIALECTS.iter().find(|d| {
                d.tag
                    .split('-')
                    .next()
                    .is_some_and(|lang| lang.eq_ignore_ascii_case(&tag))
            })
        })
}

/// Base letters for Latin Extended-A, U+0100..=U+017F.
const LATIN_EXT_A: &[u8; 128] = b"AaAaAaCcCcCcCcDd\
DdEeEeEeEeEeGgGg\
GgGgHhHhIiIiIiIi\
IiIiJjKkkLlLlLlL\
lLlNnNnNnnNnOoOo\
OoOoRrRrRrSsSsSs\
SsTtTtTtUuUuUuUu\
UuUuWwYyYZzZzZzs";

/// The Windows-1252 byte for `c`, when it has one.
fn cp1252_byte(c: char) -> Option<u8> {
    let u = u32::from(c);
    if u < 0x80 || (0xA0..=0xFF).contains(&u) {
        return u8::try_from(u).ok();
    }
    Some(match c {
        '€' => 0x80,
        '‚' => 0x82,
        'ƒ' => 0x83,
        '„' => 0x84,
        '…' => 0x85,
        '†' => 0x86,
        '‡' => 0x87,
        'ˆ' => 0x88,
        '‰' => 0x89,
        'Š' => 0x8A,
        '‹' => 0x8B,
        'Œ' => 0x8C,
        'Ž' => 0x8E,
        '\u{2018}' => 0x91,
        '\u{2019}' => 0x92,
        '\u{201C}' => 0x93,
        '\u{201D}' => 0x94,
        '•' => 0x95,
        '–' => 0x96,
        '—' => 0x97,
        '˜' => 0x98,
        '™' => 0x99,
        'š' => 0x9A,
        '›' => 0x9B,
        'œ' => 0x9C,
        'ž' => 0x9E,
        'Ÿ' => 0x9F,
        _ => return None,
    })
}

/// Encodes `text` as Windows-1252 for ECI (rules in the module docs). The
/// result never contains NUL, so it can be passed as a C string.
pub fn encode_cp1252(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\t' | '\n' | '\r' => out.push(c as u8),
            '`' => out.push(b'\''),
            c if c.is_control() => out.push(b' '),
            c => {
                if let Some(b) = cp1252_byte(c) {
                    out.push(b);
                    continue;
                }
                let u = u32::from(c);
                match c {
                    '\u{0100}'..='\u{017F}' => {
                        out.push(LATIN_EXT_A[(u - 0x100) as usize]);
                    }
                    '\u{2010}'..='\u{2015}' | '\u{2212}' | '\u{2043}' | '\u{FE63}' | '\u{FF0D}' => {
                        out.push(b'-');
                    }
                    '\u{2032}' => out.push(b'\''),
                    '\u{2033}' => out.push(b'"'),
                    '\u{FB01}' => out.extend_from_slice(b"fi"),
                    '\u{FB02}' => out.extend_from_slice(b"fl"),
                    // Other spaces, and everything unrepresentable.
                    _ => out.push(b' '),
                }
            }
        }
    }
    out
}

/// Encodes `text` for dialect `code`. Unsupported dialects fall back to
/// Windows-1252, which keeps ASCII intact.
pub fn encode_for(code: u32, text: &str) -> Vec<u8> {
    let _ = code;
    encode_cp1252(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_and_latin1_pass_through() {
        assert_eq!(encode_cp1252("Dr. Smith 9:30"), b"Dr. Smith 9:30");
        assert_eq!(
            encode_cp1252("Café crème, naïve résumé"),
            b"Caf\xe9 cr\xe8me, na\xefve r\xe9sum\xe9"
        );
    }

    #[test]
    fn typographic_characters_use_the_0x80_block() {
        assert_eq!(
            encode_cp1252("\u{201C}Hi\u{201D} \u{2014} it\u{2019}s €5…"),
            b"\x93Hi\x94 \x97 it\x92s \x805\x85"
        );
        assert_eq!(encode_cp1252("Œuvre Škoda"), b"\x8cuvre \x8akoda");
    }

    #[test]
    fn unrepresentable_characters_degrade_gracefully() {
        assert_eq!(encode_cp1252("Łódź"), b"L\xf3dz");
        assert_eq!(encode_cp1252("Dvořák"), b"Dvor\xe1k");
        assert_eq!(encode_cp1252("3 − 2 ‐ 1"), b"3 - 2 - 1");
        assert_eq!(encode_cp1252("a\u{2009}b"), b"a b");
        assert_eq!(encode_cp1252("π≈3"), b"  3");
        assert_eq!(encode_cp1252("日本"), b"  ");
        assert_eq!(encode_cp1252("\u{FB01}ne"), b"fine");
    }

    #[test]
    fn annotations_and_nul_cannot_be_injected() {
        assert_eq!(encode_cp1252("`vv50 x"), b"'vv50 x");
        assert_eq!(encode_cp1252("a\0b\u{7}c"), b"a b c");
        assert!(!encode_cp1252("x\0y").contains(&0));
    }

    #[test]
    fn latin_ext_a_table_is_aligned() {
        // Spot checks at the start of each row of 16.
        for (c, b) in [
            ('\u{0100}', b'A'),
            ('\u{0110}', b'D'),
            ('\u{0120}', b'G'),
            ('\u{0130}', b'I'),
            ('\u{0141}', b'L'),
            ('\u{0150}', b'O'),
            ('\u{0160}', 0x8A),
            ('\u{0170}', b'U'),
            ('\u{017F}', b's'),
            ('ő', b'o'),
            ('ž', 0x9E),
        ] {
            assert_eq!(encode_cp1252(&c.to_string()), vec![b], "{c}");
        }
    }

    #[test]
    fn dialect_lookup() {
        assert_eq!(dialect_by_tag("en-us").unwrap().code, 0x0001_0000);
        assert_eq!(dialect_by_tag("en_GB").unwrap().code, 0x0001_0001);
        assert_eq!(dialect_by_tag("de").unwrap().tag, "de-DE");
        assert_eq!(dialect_by_code(0x0009_0000).unwrap().tag, "fi-FI");
        assert!(dialect_by_tag("xx").is_none());
        assert!(!dialect_by_tag("ja").unwrap().supported);
    }

    proptest::proptest! {
        #[test]
        fn encoding_never_contains_nul_or_backquote(s in "\\PC*") {
            let e = encode_cp1252(&s);
            proptest::prop_assert!(!e.contains(&0));
            proptest::prop_assert!(!e.contains(&b'`'));
        }
    }
}
