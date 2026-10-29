//! The live document message: one retained message on a share topic that
//! carries a whole document, published by `tw share` and the app's
//! "Publish this document live", and shown by a reader's document view.
//!
//! ```json
//! {"type": "textweaver.document", "rev": 3, "title": "Agenda",
//!  "format": "markdown", "text": "# Agenda\n\n- Check-ins\n"}
//! ```
//!
//! Unlike every other string a message carries, the document's `text`
//! keeps its line breaks and may be far longer
//! ([`Limits::max_document`](crate::Limits::max_document)): the receiver
//! cleans it with [`clean_document_text`], which still removes control
//! characters, terminal escape sequences and bidirectional overrides.

use serde_json::Value;

/// The `type` of a live document message.
pub const DOCUMENT_TYPE: &str = "textweaver.document";

/// A live document message, as a reader receives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedDocument {
    /// The revision: 1 for the first share, one more for each save.
    pub rev: u64,
    /// The title readers see.
    pub title: String,
    /// `markdown` or `text`.
    pub format: String,
    /// The document, its line breaks kept.
    pub text: String,
}

impl SharedDocument {
    /// The document a decoded message carries, or `None` when it is not a
    /// live document message (no `type` of [`DOCUMENT_TYPE`], or no
    /// `text` string). A missing `rev` is 0, a missing title empty, and a
    /// format other than `markdown` is `text`.
    pub fn from_json(value: &Value) -> Option<SharedDocument> {
        let obj = value.as_object()?;
        if !is_document(value) {
            return None;
        }
        let text = obj.get("text")?.as_str()?.to_owned();
        let format = match obj.get("format").and_then(Value::as_str) {
            Some("markdown") => "markdown",
            _ => "text",
        };
        Some(SharedDocument {
            rev: obj.get("rev").and_then(Value::as_u64).unwrap_or(0),
            title: obj
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            format: format.to_owned(),
            text,
        })
    }

    /// The message to publish, as JSON text, keys in the documented order.
    pub fn message(&self) -> String {
        document_message(self.rev, &self.title, &self.format, &self.text)
    }
}

/// True when `value` is an object whose `type` is [`DOCUMENT_TYPE`].
pub fn is_document(value: &Value) -> bool {
    value.get("type").and_then(Value::as_str) == Some(DOCUMENT_TYPE)
}

/// The live document message for revision `rev`, as JSON text:
/// `{"type":"textweaver.document","rev":N,"title":...,"format":...,"text":...}`.
pub fn document_message(rev: u64, title: &str, format: &str, text: &str) -> String {
    let s = |v: &str| serde_json::to_string(v).unwrap_or_else(|_| "\"\"".into());
    format!(
        "{{\"type\":\"{DOCUMENT_TYPE}\",\"rev\":{rev},\"title\":{},\"format\":{},\"text\":{}}}",
        s(title),
        s(format),
        s(text)
    )
}

/// "markdown" for a `.md` or `.markdown` file name, "text" for the rest.
pub fn format_for_name(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".md") || lower.ends_with(".markdown") {
        "markdown"
    } else {
        "text"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clean_document_text;

    #[test]
    fn the_message_round_trips() {
        let m = document_message(3, "Agenda", "markdown", "# Net\n\"hi\"");
        assert_eq!(
            m,
            r##"{"type":"textweaver.document","rev":3,"title":"Agenda","format":"markdown","text":"# Net\n\"hi\""}"##
        );
        let v: Value = serde_json::from_str(&m).unwrap();
        let d = SharedDocument::from_json(&v).unwrap();
        assert_eq!(d.rev, 3);
        assert_eq!(d.text, "# Net\n\"hi\"");
        assert_eq!(d.message(), m);
        assert!(SharedDocument::from_json(&serde_json::json!({"text": "x"})).is_none());
        assert!(SharedDocument::from_json(&serde_json::json!({"type": DOCUMENT_TYPE})).is_none());
        assert_eq!(format_for_name("a.MD"), "markdown");
        assert_eq!(format_for_name("a.txt"), "text");
    }

    #[test]
    fn document_text_keeps_lines_and_drops_the_rest() {
        assert_eq!(
            clean_document_text("# Net\r\n\r\n- one\r- two\u{2028}three", 1000),
            "# Net\n\n- one\n- two\nthree"
        );
        assert_eq!(
            clean_document_text("a\u{1b}[31mred\u{1b}[0m\nb\u{202e}c\u{7}d\tcode", 1000),
            "ared\nbcd\tcode"
        );
        assert_eq!(
            clean_document_text("a\u{1b}]0;title\u{07}b\nnext", 1000),
            "ab\nnext"
        );
        assert_eq!(clean_document_text("abcdef", 3), "abc\n(cut)\n");
    }
}
