//! `tw info`. Owner: Agent A.
//!
//! Facts about a document: format, title, author, language, size in chars,
//! words, sentences, lines, and paragraphs, structure counts (pages and
//! sections of paginated and chaptered sources included), and an estimated
//! reading time at Star's default rate of 265 words per minute.
//!
//! Counting is cheap by default. Words are counted in one pass over the
//! text ([`count_words`]), which agrees with the reader's word units
//! (UAX #29 words holding a letter or digit, hyphenated compounds joined)
//! on ordinary prose, instead of segmenting every word. Sentences need the
//! full sentence segmentation, so they are counted for documents up to
//! [`SENTENCE_LIMIT`] characters, and for larger ones only with `--exact`,
//! which also counts words with the full segmentation. On a 10 MB text
//! file this took `tw info` from about 2.7 seconds to a fraction of one.

use std::path::PathBuf;

use serde_json::{Value, json};
use textweaver_app::text::core::{MarkerKind, Unit};
use textweaver_app::text::{Document, units};

use super::text::load_document;

/// Arguments for `tw info`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Document to describe.
    pub file: PathBuf,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
    /// Count words and sentences with the reader's full segmentation, at
    /// any size (slower on long documents).
    #[arg(long)]
    pub exact: bool,
}

/// Star's default reading rate, in words per minute.
const DEFAULT_WPM: usize = 265;

/// Documents up to this many characters get a sentence count without
/// `--exact` (segmenting sentences is the slow part of `tw info`).
pub(crate) const SENTENCE_LIMIT: usize = 1_000_000;

/// Runs `tw info`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let doc = load_document(&args.file)?;
    let mut facts = facts_with(&doc, &args.file, args.exact);
    let components = components_installed();
    // Through `print_all`, so a closed pipe (`tw info --json x | head`)
    // ends quietly instead of panicking.
    if args.json {
        if let (Some((n, all)), Some(obj)) = (components, facts.as_object_mut()) {
            obj.insert("components".into(), json!({ "installed": n, "known": all }));
        }
        super::print_all(&format!(
            "{}
",
            serde_json::to_string_pretty(&facts)?
        ))
    } else {
        let mut text = describe(&facts);
        if let Some(c) = components {
            text.push_str(&components_line(c));
        }
        super::print_all(&text)
    }
}

/// How many optional components are installed in this user's data
/// folder, and how many textweaver knows (W8a-d).
fn components_installed() -> Option<(usize, usize)> {
    let paths = textweaver_app::store::Paths::platform().ok()?;
    Some(textweaver_app::components::Registry::builtin().installed_count(&paths.data_dir))
}

/// The line `tw info` ends with, the one components status line:
/// "Components: 1 of 5 installed."
fn components_line(count: (usize, usize)) -> String {
    let c = textweaver_app::lexicon::i18n::Catalog::english();
    format!("{}.\n", textweaver_app::components::status_line(&c, count))
}

/// Structure counts reported, as (JSON key, marker kind, block level filter).
const COUNTS: &[(&str, MarkerKind, Option<u8>)] = &[
    ("headings", MarkerKind::Heading, None),
    ("lists", MarkerKind::List, None),
    ("list_items", MarkerKind::ListItem, None),
    ("tables", MarkerKind::Table, None),
    ("table_rows", MarkerKind::TableRow, None),
    ("links", MarkerKind::Link, None),
    ("images", MarkerKind::Image, None),
    ("code_blocks", MarkerKind::Code, Some(1)),
    ("block_quotes", MarkerKind::Quote, None),
    ("footnotes", MarkerKind::Footnote, Some(1)),
    ("pages", MarkerKind::PageBreak, None),
    ("sections", MarkerKind::SectionBreak, None),
];

/// Which neighbours a character may join into one word (UAX #29's
/// MidLetter, MidNum, and MidNumLet, plus the hyphen the reader's word
/// units join compounds with).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mid {
    /// Between two letters (`a:b`).
    Letters,
    /// Between two digits (`1,250`).
    Digits,
    /// Between two letters or two digits (`don't`, `e.g`, `3.14`).
    Same,
    /// Between any two letters or digits (`well-known`, `12-14`).
    Any,
}

fn mid(c: char) -> Option<Mid> {
    match c {
        ':' | '\u{00B7}' | '\u{0387}' | '\u{05F4}' | '\u{2027}' | '\u{FE13}' | '\u{FE55}'
        | '\u{FF1A}' => Some(Mid::Letters),
        ',' | ';' | '\u{037E}' | '\u{0589}' | '\u{060C}' | '\u{066C}' | '\u{FE50}' | '\u{FE54}'
        | '\u{FF0C}' | '\u{FF1B}' => Some(Mid::Digits),
        '.' | '\'' | '\u{2018}' | '\u{2019}' | '\u{2024}' | '\u{FE52}' | '\u{FF07}'
        | '\u{FF0E}' => Some(Mid::Same),
        '-' | '\u{2011}' => Some(Mid::Any),
        _ => None,
    }
}

/// True for characters that are part of a word without being letters or
/// digits: connectors (`snake_case`), combining marks, and the zero-width
/// joiner.
fn is_word_joiner(c: char) -> bool {
    matches!(
        c,
        '_' | '\u{203F}'
            | '\u{2040}'
            | '\u{0300}'..='\u{036F}'
            | '\u{1AB0}'..='\u{1AFF}'
            | '\u{1DC0}'..='\u{1DFF}'
            | '\u{20D0}'..='\u{20FF}'
            | '\u{FE20}'..='\u{FE2F}'
            | '\u{200D}'
    )
}

/// Scripts written without spaces, where UAX #29 makes every character a
/// word of its own.
fn is_ideographic(c: char) -> bool {
    matches!(
        c,
        '\u{2E80}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}' | '\u{20000}'..='\u{3FFFF}'
    )
}

/// Words in one pass over the text: runs of letters and digits, joined
/// across one mid-word character where UAX #29 joins them (`don't`,
/// `1,250`, `e.g`) and across hyphens (`well-known`), with each ideograph a
/// word of its own. The same count as the reader's word units on ordinary
/// prose, at a small fraction of the cost.
pub(crate) fn count_words(text: impl IntoIterator<Item = char>) -> usize {
    let mut count = 0usize;
    // The last letter or digit of the word being read (is it a digit?).
    let mut last: Option<bool> = None;
    // A mid-word character right after the word, waiting for what follows.
    let mut pending: Option<Mid> = None;
    for c in text {
        if c.is_alphanumeric() {
            let digit = c.is_numeric();
            if is_ideographic(c) {
                count += 1;
                last = None;
                pending = None;
                continue;
            }
            let joins = match (last, pending) {
                (None, _) => false,
                (Some(_), None) => true,
                (Some(prev), Some(m)) => match m {
                    Mid::Letters => !prev && !digit,
                    Mid::Digits => prev && digit,
                    Mid::Same => prev == digit,
                    Mid::Any => true,
                },
            };
            if !joins {
                count += 1;
            }
            last = Some(digit);
            pending = None;
        } else if last.is_some() && pending.is_none() && is_word_joiner(c) {
            // Stays in the word.
        } else if last.is_some()
            && pending.is_none()
            && let Some(m) = mid(c)
        {
            pending = Some(m);
        } else {
            last = None;
            pending = None;
        }
    }
    count
}

/// Everything `tw info` reports, as JSON, with the fast counts (see the
/// module docs).
#[cfg(test)]
pub(crate) fn facts(doc: &Document, file: &std::path::Path) -> Value {
    facts_with(doc, file, false)
}

/// Everything `tw info` reports; `exact` counts words and sentences with
/// the full segmentation at any size.
pub(crate) fn facts_with(doc: &Document, file: &std::path::Path, exact: bool) -> Value {
    let words = if exact {
        units::segments(doc, Unit::Word).len()
    } else {
        count_words(doc.text().chars())
    };
    let sentences = (exact || doc.len_chars() <= SENTENCE_LIMIT)
        .then(|| units::segments(doc, Unit::Sentence).len());
    let index = doc.marker_index();
    let mut structure = serde_json::Map::new();
    for &(key, kind, level) in COUNTS {
        structure.insert(key.to_owned(), json!(index.count(kind, level)));
    }
    let mut headings_by_level = serde_json::Map::new();
    for level in 1..=6u8 {
        let n = index.count(MarkerKind::Heading, Some(level));
        if n > 0 {
            headings_by_level.insert(format!("h{level}"), json!(n));
        }
    }
    json!({
        "file": file.display().to_string(),
        "format": doc.meta.format,
        "title": doc.meta.title,
        "author": doc.meta.author,
        "language": doc.meta.language,
        "properties": doc.meta.properties,
        "chars": doc.len_chars(),
        "words": words,
        "sentences": sentences,
        "lines": doc.line_count(),
        "paragraphs": units::segments(doc, Unit::Paragraph).len(),
        "structure": structure,
        "headings_by_level": headings_by_level,
        "reading_minutes": words.div_ceil(DEFAULT_WPM),
        "reading_wpm": DEFAULT_WPM,
    })
}

fn plural(n: u64, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The facts as readable lines ("Words: 120").
pub(crate) fn describe(f: &Value) -> String {
    let mut out = String::new();
    let mut line = |label: &str, value: String| {
        out.push_str(label);
        out.push_str(": ");
        out.push_str(&value);
        out.push('\n');
    };
    let s = |v: &Value| v.as_str().map(str::to_owned);
    let n = |v: &Value| v.as_u64().unwrap_or(0);
    line("File", s(&f["file"]).unwrap_or_default());
    line("Format", s(&f["format"]).unwrap_or_default());
    for (label, key) in [
        ("Title", "title"),
        ("Author", "author"),
        ("Language", "language"),
    ] {
        if let Some(v) = s(&f[key]) {
            line(label, v);
        }
    }
    for (label, key) in [
        ("Characters", "chars"),
        ("Words", "words"),
        ("Sentences", "sentences"),
        ("Lines", "lines"),
        ("Paragraphs", "paragraphs"),
    ] {
        if f[key].is_null() {
            line(
                label,
                "not counted in a document this long; use --exact".to_owned(),
            );
        } else {
            line(label, n(&f[key]).to_string());
        }
    }
    let names: &[(&str, &str, &str)] = &[
        ("headings", "heading", "headings"),
        ("lists", "list", "lists"),
        ("list_items", "list item", "list items"),
        ("tables", "table", "tables"),
        ("table_rows", "table row", "table rows"),
        ("links", "link", "links"),
        ("images", "image", "images"),
        ("code_blocks", "code block", "code blocks"),
        ("block_quotes", "block quote", "block quotes"),
        ("footnotes", "footnote", "footnotes"),
        ("pages", "page", "pages"),
        ("sections", "section", "sections"),
    ];
    let parts: Vec<String> = names
        .iter()
        .filter(|(k, ..)| n(&f["structure"][k]) > 0)
        .map(|(k, one, many)| plural(n(&f["structure"][k]), one, many))
        .collect();
    line(
        "Structure",
        if parts.is_empty() {
            "none".to_owned()
        } else {
            parts.join(", ")
        },
    );
    let minutes = n(&f["reading_minutes"]);
    line(
        "Reading time",
        format!(
            "about {} at {} words per minute",
            plural(minutes, "minute", "minutes"),
            n(&f["reading_wpm"])
        ),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_the_markdown_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sample.md");
        let doc = load_document(&path).unwrap();
        let f = facts(&doc, &path);
        assert_eq!(f["format"], "markdown");
        assert_eq!(f["title"], "Sample Markdown Document");
        assert_eq!(f["author"], "Test Author");
        assert_eq!(f["structure"]["tables"], 1);
        assert_eq!(f["structure"]["code_blocks"], 1);
        assert_eq!(f["headings_by_level"]["h2"], 4);
        let text = describe(&f);
        assert!(text.contains("Format: markdown\n"));
        assert!(text.contains("1 table,"));
        assert!(text.contains("Reading time: about 1 minute at 265 words per minute\n"));
    }

    #[test]
    fn fast_word_count_matches_the_word_units() {
        let texts = [
            "Dr. Smith read 1,250 pages at 9:30 a.m. on Friday, e.g. the appendix.",
            "Don't split well-known snake_case words; café naïve cafe\u{301}.",
            "Numbers 3.14 and 12-14, dashes \u{2014} and emoji \u{1F600} are not words.",
            "  Leading, trailing,\n\nand blank lines ... end. ",
            "\u{65e5}\u{672c}\u{8a9e} text",
            "",
        ];
        for t in texts {
            let doc = Document::from_plain_text(t);
            assert_eq!(
                count_words(t.chars()),
                units::segments(&doc, Unit::Word).len(),
                "{t:?}"
            );
        }
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sample.md");
        let doc = load_document(&path).unwrap();
        let exact = units::segments(&doc, Unit::Word).len();
        let fast = count_words(doc.text().chars());
        assert!(exact.abs_diff(fast) * 100 <= exact, "{fast} vs {exact}");
    }

    #[test]
    fn long_documents_skip_sentences_unless_exact() {
        let text = "One two. ".repeat(SENTENCE_LIMIT / 9 + 10);
        let doc = Document::from_plain_text(&text);
        let path = PathBuf::from("long.txt");
        let f = facts(&doc, &path);
        assert!(f["sentences"].is_null());
        assert_eq!(f["words"], json!(2 * (SENTENCE_LIMIT / 9 + 10)));
        assert!(
            describe(&f).contains("Sentences: not counted in a document this long; use --exact\n")
        );
        let exact = facts_with(&doc, &path, true);
        assert_eq!(exact["sentences"], json!(SENTENCE_LIMIT / 9 + 10));
        assert_eq!(exact["words"], f["words"]);
    }

    #[test]
    fn describes_every_new_format() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/a");
        for (name, format, expect) in [
            ("sample.epub", "epub", "3 sections"),
            ("sample.docx", "docx", "1 table"),
            ("running.pdf", "pdf", "3 pages"),
        ] {
            let path = dir.join(name);
            let doc = load_document(&path).unwrap();
            let f = facts(&doc, &path);
            assert_eq!(f["format"], format);
            let text = describe(&f);
            assert!(text.contains(expect), "{name}: {text}");
        }
    }
}
