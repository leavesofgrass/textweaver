//! Prosody markup, ported from `star/ttstext/ssml.py` (Part 2 section 5.C).
//!
//! These produce engine markup, not spoken text, so they carry no offset
//! map; an engine that takes SSML gets its word positions from marks
//! (`Caps::SSML_MARKS`, wave 2). Fix: `'` is escaped too (star quirk Q9).

use std::sync::LazyLock;

use regex::{Captures, Regex};

fn re(p: &str) -> Regex {
    Regex::new(p).unwrap_or_else(|e| panic!("bad built-in pattern: {e}"))
}

// Compiled once per process, not once per sentence: compiling these five
// patterns cost a few hundred microseconds on every utterance an SSML or
// DECtalk engine spoke, on the speech thread.
static PARAGRAPHS: LazyLock<Regex> = LazyLock::new(|| re(r"\n{2,}"));
static SENTENCE_ENDS: LazyLock<Regex> = LazyLock::new(|| re(r"([.!?\u{2026}])(\s+)"));
static CLAUSES: LazyLock<Regex> = LazyLock::new(|| re(r"([,:])(\s+)"));
static SEMICOLONS: LazyLock<Regex> = LazyLock::new(|| re(r";(\s+)"));
static DASHES: LazyLock<Regex> = LazyLock::new(|| re(r"[\u{2014}\u{2013}]"));
static DECTALK_CLAUSES: LazyLock<Regex> = LazyLock::new(|| re(r"([,;:])(\s+)"));

/// star `_text_to_ssml(text, backend, sentence_ms, clause_ms)`: escapes the
/// text and adds `<break>`s after sentences, clauses, and dashes. Text that
/// already starts with `<speak>` is returned unchanged; `backend ==
/// "dectalk"` gives [`text_to_dectalk`] instead.
pub fn text_to_ssml(text: &str, backend: &str, sentence_ms: u32, clause_ms: u32) -> String {
    if text.trim_start().starts_with("<speak>") {
        return text.to_owned();
    }
    if backend == "dectalk" {
        return text_to_dectalk(text, sentence_ms, clause_ms);
    }
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;");
    let s = PARAGRAPHS.replace_all(
        &escaped,
        format!("<break time=\"{}ms\"/>\n", 2 * sentence_ms),
    );
    let s = SENTENCE_ENDS.replace_all(&s, format!("$1<break time=\"{sentence_ms}ms\"/>$2"));
    let s = CLAUSES.replace_all(&s, format!("$1<break time=\"{clause_ms}ms\"/>$2"));
    let s = semicolons(&s, clause_ms);
    let s = DASHES.replace_all(&s, format!(" <break time=\"{clause_ms}ms\"/> "));
    format!("<speak>{s}</speak>")
}

/// `;(\s+)` gets a clause break unless the semicolon ends an entity.
fn semicolons(s: &str, clause_ms: u32) -> String {
    SEMICOLONS
        .replace_all(s, |c: &Captures<'_>| {
            let start = c.get(0).map_or(0, |m| m.start());
            let entity = ["&amp", "&lt", "&gt", "&quot", "&apos"]
                .iter()
                .any(|e| s[..start].ends_with(e));
            let ws = c.get(1).map_or("", |m| m.as_str());
            if entity {
                format!(";{ws}")
            } else {
                format!(";<break time=\"{clause_ms}ms\"/>{ws}")
            }
        })
        .into_owned()
}

/// star `_text_to_dectalk(text, sentence_ms, clause_ms)`: DECtalk `[:pau]`
/// commands after sentences, clauses, and dashes.
pub fn text_to_dectalk(text: &str, sentence_ms: u32, clause_ms: u32) -> String {
    let s = SENTENCE_ENDS.replace_all(text, format!("$1 [:pau {sentence_ms}] "));
    let s = DECTALK_CLAUSES.replace_all(&s, format!("$1 [:pau {clause_ms}] "));
    let s = DASHES.replace_all(&s, format!(" [:pau {clause_ms}] "));
    s.into_owned()
}
