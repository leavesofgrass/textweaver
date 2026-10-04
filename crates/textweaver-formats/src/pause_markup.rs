//! Pauses written as markup in a document's text.
//!
//! An SSML-style break in a document, `<break time="500ms"/>`,
//! `<break time="1s"/>` or `<break strength="strong"/>`, is a pause, not
//! text: the loaders leave the markup out of the canonical text (so the
//! reading view, the Braille display and the spoken text never show it)
//! and record where it was and how long it lasts in the document's
//! properties ([`PAUSES_PROPERTY`]). The reader cuts its utterances there
//! and plans a pause of that length, so every engine pauses.
//!
//! - Plain text: the markup is taken out of the text ([`strip`]).
//! - Markdown: a `<break>` tag in the raw HTML is a pause where it stands.
//! - HTML: a `<break>` element is a pause where it starts; any text the
//!   parser put inside it is read as usual.
//!
//! A `time` wins over a `strength`. Strengths follow the common SSML
//! lengths: `none` 0, `x-weak` 250 ms, `weak` 500, `medium` 750 (also a
//! bare `<break/>`), `strong` 1000, `x-strong` 1250. A break whose `time`
//! cannot be read is not a pause and stays as written.
//!
//! [`LoadOptions::keep_pause_markup`](crate::LoadOptions::keep_pause_markup)
//! (`[speech] markup_pauses = false`) turns recognition off, for documents
//! that quote SSML: plain text then keeps the markup as text.

use serde::{Deserialize, Serialize};
use textweaver_core::CharPos;
use textweaver_text::DocumentMeta;

/// The `DocumentMeta::properties` key holding the written pauses, as JSON
/// (see [`written_pauses`]).
pub const PAUSES_PROPERTY: &str = "textweaver.pauses";

/// A pause written in the document: `ms` long, where the canonical text
/// reaches `at` (after the text before the markup).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrittenPause {
    /// Where the markup was, in the canonical text.
    pub at: CharPos,
    /// How long the pause is, in milliseconds.
    pub ms: u32,
}

/// The pauses a loader recorded on a document, in document order.
/// Unreadable or missing data gives none.
pub fn written_pauses(meta: &DocumentMeta) -> Vec<WrittenPause> {
    meta.properties
        .get(PAUSES_PROPERTY)
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default()
}

/// Records `pauses` on `meta` (sorted); nothing is written for none.
pub(crate) fn record(meta: &mut DocumentMeta, mut pauses: Vec<WrittenPause>) {
    pauses.retain(|p| p.ms > 0);
    if pauses.is_empty() {
        return;
    }
    pauses.sort_by_key(|p| p.at);
    if let Ok(json) = serde_json::to_string(&pauses) {
        meta.properties.insert(PAUSES_PROPERTY.to_owned(), json);
    }
}

/// The length of a break with these attributes, in ms; `None` when the
/// `time` cannot be read or the `strength` is unknown.
pub fn break_ms(time: Option<&str>, strength: Option<&str>) -> Option<u32> {
    if let Some(t) = time {
        return parse_time(t);
    }
    match strength.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
        None | Some("medium") => Some(750),
        Some("none") => Some(0),
        Some("x-weak") => Some(250),
        Some("weak") => Some(500),
        Some("strong") => Some(1000),
        Some("x-strong") => Some(1250),
        Some(_) => None,
    }
}

/// `500ms`, `1s`, `1.5s` (SSML's time designations) in ms.
fn parse_time(t: &str) -> Option<u32> {
    let t = t.trim().to_ascii_lowercase();
    let (number, scale) = if let Some(n) = t.strip_suffix("ms") {
        (n, 1.0)
    } else if let Some(n) = t.strip_suffix('s') {
        (n, 1000.0)
    } else {
        return None;
    };
    let n = number.trim();
    if n.is_empty() || !n.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    let v: f64 = n.parse().ok()?;
    let ms = (v * scale).round();
    // Anything longer than a minute is clamped later; this only keeps the
    // cast in range.
    Some(ms.min(f64::from(u32::MAX)) as u32)
}

/// The value of attribute `name` in a tag's inside (`break time="1s"/`).
fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(name) {
        let start = from + i;
        from = start + name.len();
        let before_ok = lower[..start]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace);
        let rest = lower[from..].trim_start();
        if !before_ok || !rest.starts_with('=') {
            continue;
        }
        let value_at = tag.len() - rest.len() + 1;
        let value = tag[value_at..].trim_start();
        let quote = value.chars().next()?;
        return if quote == '"' || quote == '\'' {
            let body = &value[1..];
            body.find(quote).map(|e| body[..e].to_owned())
        } else {
            Some(
                value
                    .split(|c: char| c.is_whitespace() || c == '/' || c == '>')
                    .next()
                    .unwrap_or_default()
                    .to_owned(),
            )
        };
    }
    None
}

/// When the inside of a tag (`break time="500ms"/`, without its angle
/// brackets) is a break: its length in ms. `None` for any other tag, for a
/// closing `/break`, and for a break whose time cannot be read.
pub fn tag_ms(inside: &str) -> Option<u32> {
    let t = inside.trim();
    let name_end = t
        .find(|c: char| c.is_whitespace() || c == '/')
        .unwrap_or(t.len());
    if !t[..name_end].eq_ignore_ascii_case("break") {
        return None;
    }
    break_ms(attr(t, "time").as_deref(), attr(t, "strength").as_deref())
}

/// Plain text with its break markup taken out, and the pauses it held
/// (positions in the returned text). A break between two spaces leaves
/// one space. Closing `</break>` tags are dropped too. Text with no break
/// markup is returned unchanged.
pub fn strip(text: &str) -> (String, Vec<WrittenPause>) {
    if !contains_break(text) {
        return (text.to_owned(), Vec::new());
    }
    let mut out = String::with_capacity(text.len());
    let mut chars = 0usize;
    let mut pauses = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find('<') {
        let (before, tail) = rest.split_at(i);
        out.push_str(before);
        chars += before.chars().count();
        let Some(close) = tail.find('>') else {
            rest = tail;
            break;
        };
        let inside = &tail[1..close];
        let closing = inside
            .trim()
            .strip_prefix('/')
            .is_some_and(|n| n.trim().eq_ignore_ascii_case("break"));
        let ms = tag_ms(inside);
        if ms.is_none() && !closing {
            out.push('<');
            chars += 1;
            rest = &tail[1..];
            continue;
        }
        if let Some(ms) = ms {
            pauses.push(WrittenPause {
                at: CharPos(chars),
                ms,
            });
        }
        rest = &tail[close + 1..];
        // "Wait <break/> now": one space stays, not two.
        let spaced_before = out.is_empty() || out.ends_with([' ', '\t', '\n']);
        if spaced_before && rest.starts_with([' ', '\t']) {
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    (out, pauses)
}

/// True when `text` may hold a break tag (a cheap scan before [`strip`]).
fn contains_break(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.windows(6).any(|w| {
        w[0] == b'<' && w[1..].eq_ignore_ascii_case(b"break")
            || w[0] == b'/' && w[1..].eq_ignore_ascii_case(b"break")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_and_strengths_have_their_lengths() {
        assert_eq!(tag_ms(r#"break time="500ms"/"#), Some(500));
        assert_eq!(tag_ms(r#"break time="1s"/"#), Some(1000));
        assert_eq!(tag_ms("BREAK time='1.5s' /"), Some(1500));
        assert_eq!(tag_ms(r#"break strength="strong"/"#), Some(1000));
        assert_eq!(tag_ms(r#"break strength="x-weak"/"#), Some(250));
        assert_eq!(tag_ms("break/"), Some(750));
        assert_eq!(tag_ms("break"), Some(750));
        // Time wins over strength.
        assert_eq!(
            tag_ms(r#"break strength="x-strong" time="200ms"/"#),
            Some(200)
        );
        assert_eq!(tag_ms(r#"break time="soon"/"#), None);
        assert_eq!(tag_ms(r#"break strength="loud"/"#), None);
        assert_eq!(tag_ms("breakfast"), None);
        assert_eq!(tag_ms("/break"), None);
        assert_eq!(tag_ms("b"), None);
    }

    #[test]
    fn strip_takes_the_markup_out_and_keeps_the_positions() {
        let (text, pauses) = strip(
            r#"One.<break time="500ms"/> Two. <break time="1s"/> Three é <break strength="strong"/>"#,
        );
        assert_eq!(text, "One. Two. Three é ");
        assert_eq!(
            pauses,
            [
                WrittenPause {
                    at: CharPos(4),
                    ms: 500
                },
                WrittenPause {
                    at: CharPos(10),
                    ms: 1000
                },
                WrittenPause {
                    at: CharPos(18),
                    ms: 1000
                },
            ]
        );
    }

    #[test]
    fn other_angle_brackets_stay() {
        let t = "a < b, <b>bold</b>, <break time=\"x\"/> and x > y";
        let (text, pauses) = strip(t);
        assert_eq!(text, t);
        assert!(pauses.is_empty());
        let (text, _) = strip("plain");
        assert_eq!(text, "plain");
    }

    #[test]
    fn pauses_survive_the_properties() {
        let mut meta = DocumentMeta::default();
        record(&mut meta, Vec::new());
        assert!(meta.properties.is_empty());
        let p = vec![
            WrittenPause {
                at: CharPos(9),
                ms: 0,
            },
            WrittenPause {
                at: CharPos(5),
                ms: 300,
            },
        ];
        record(&mut meta, p);
        assert_eq!(
            written_pauses(&meta),
            [WrittenPause {
                at: CharPos(5),
                ms: 300
            }]
        );
    }

    fn load(text: &str, hint: &str, keep: bool) -> textweaver_text::Document {
        let src = crate::Source::Bytes {
            data: text.as_bytes().to_vec().into(),
            hint: hint.into(),
        };
        let options = crate::LoadOptions {
            keep_pause_markup: keep,
            ..crate::LoadOptions::default()
        };
        crate::Registry::with_builtins()
            .load(&src, &options)
            .expect("loads")
    }

    fn lengths(doc: &textweaver_text::Document) -> Vec<(usize, u32)> {
        written_pauses(&doc.meta)
            .iter()
            .map(|p| (p.at.0, p.ms))
            .collect()
    }

    const THREE: &str = "Ready.<break time=\"500ms\"/> Set. <break time=\"1s\"/> Go <break strength=\"x-strong\"/>now.";

    #[test]
    fn plain_text_markdown_and_html_read_breaks_as_pauses() {
        // Plain text keeps its spaces; Markdown places a pause before the
        // space that follows the text.
        for (hint, at) in [("txt", [6, 12, 15]), ("md", [6, 11, 14])] {
            let doc = load(THREE, hint, false);
            let text = doc.text().to_string();
            assert!(!text.contains("break"), "{hint}: {text}");
            assert_eq!(text.trim_end(), "Ready. Set. Go now.", "{hint}");
            let expected: Vec<(usize, u32)> = at.into_iter().zip([500, 1000, 1250]).collect();
            assert_eq!(lengths(&doc), expected, "{hint}");
        }
        let html = format!("<html><body><p>{THREE}</p></body></html>");
        let doc = load(&html, "html", false);
        assert_eq!(doc.text().to_string(), "Ready. Set. Go now.");
        assert_eq!(
            lengths(&doc).iter().map(|p| p.1).collect::<Vec<_>>(),
            [500, 1000, 1250]
        );
        // Each pause is after the text before its markup.
        let at: Vec<usize> = lengths(&doc).iter().map(|p| p.0).collect();
        assert_eq!(at, [6, 11, 14]);
    }

    #[test]
    fn the_setting_off_keeps_the_markup_in_plain_text() {
        let doc = load(THREE, "txt", true);
        assert_eq!(doc.text().to_string(), THREE);
        assert!(lengths(&doc).is_empty());
        // Markdown and HTML drop the unknown tag, as before.
        let doc = load(THREE, "md", true);
        assert!(lengths(&doc).is_empty());
    }
}
