//! Pandoc citation syntax: finding `[@key]`, `[see @a, p. 12; @b]`,
//! `[-@key]`, and in-text `@key [p. 3]` in plain text.
//!
//! The parser follows Pandoc's rules closely enough that anything it finds
//! Pandoc would also treat as a citation:
//!
//! - A bracketed citation is `[` … `]` whose `;`-separated parts each hold
//!   exactly one `@key` (optionally `-@key`), with free text before the key
//!   (the prefix) and after it (the locator and suffix). It is not a
//!   citation if the `[` is escaped or the `]` starts a link (`](` or `][`).
//! - An in-text citation is `@key` at the start of the text or after a
//!   space or opening punctuation, so `jon@example.com` is never one,
//!   optionally followed by ` [locator]`.
//! - Keys follow [`crate::key::is_valid_key`]; `@{odd key}` braces are
//!   accepted. Trailing punctuation is not part of a key (`@doe2020.`).
//! - A locator is a label (`p.`, `pp.`, `chap.`, `sec.`, `¶`, `fig.`, ...)
//!   and a value; a bare number is a page, as in Pandoc; `{…}` braces
//!   delimit an arbitrary locator value.

use std::ops::Range;

use hayagriva::citationberg::taxonomy::Locator as CslLocator;
use textweaver_core::CharRange;

use crate::key::is_internal_punct;

/// A citation found in text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Citation {
    /// Byte range in the text, including brackets.
    pub range: Range<usize>,
    /// The same range in characters (Unicode scalar values, ADR-0002).
    pub chars: CharRange,
    /// Whether the citation is part of the sentence (`@doe says`).
    pub narrative: bool,
    /// The cited items, in order.
    pub items: Vec<CiteItem>,
}

/// One cited work inside a citation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CiteItem {
    /// The citation key.
    pub key: String,
    /// Text before the key ("see").
    pub prefix: String,
    /// Where in the work ("p. 12").
    pub locator: Option<Locator>,
    /// Text after the locator (", emphasis added").
    pub suffix: String,
    /// `-@key`: omit the author ("Doe says (2020)").
    pub suppress_author: bool,
}

/// A pinpoint within a work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Locator {
    /// What kind of division.
    pub label: LocatorLabel,
    /// The number, range, or text ("12", "33-35", "iv").
    pub value: String,
}

/// Kinds of locator (the CSL locator types Pandoc recognizes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum LocatorLabel {
    Page,
    Chapter,
    Section,
    Paragraph,
    Figure,
    Volume,
    Line,
    Note,
    Verse,
    Book,
    Column,
    Part,
    Table,
    Equation,
    Folio,
    Issue,
    Number,
    Opus,
    SubVerbo,
    Appendix,
    Timestamp,
}

/// Locator terms, longest first so `pp.` wins over `p.`. The labels are
/// Pandoc's English locator terms.
const TERMS: &[(&str, LocatorLabel)] = &[
    ("paragraphs", LocatorLabel::Paragraph),
    ("paragraph", LocatorLabel::Paragraph),
    ("chapters", LocatorLabel::Chapter),
    ("chapter", LocatorLabel::Chapter),
    ("sections", LocatorLabel::Section),
    ("section", LocatorLabel::Section),
    ("figures", LocatorLabel::Figure),
    ("figure", LocatorLabel::Figure),
    ("volumes", LocatorLabel::Volume),
    ("volume", LocatorLabel::Volume),
    ("columns", LocatorLabel::Column),
    ("column", LocatorLabel::Column),
    ("appendix", LocatorLabel::Appendix),
    ("equation", LocatorLabel::Equation),
    ("number", LocatorLabel::Number),
    ("verses", LocatorLabel::Verse),
    ("verse", LocatorLabel::Verse),
    ("pages", LocatorLabel::Page),
    ("page", LocatorLabel::Page),
    ("lines", LocatorLabel::Line),
    ("line", LocatorLabel::Line),
    ("notes", LocatorLabel::Note),
    ("note", LocatorLabel::Note),
    ("parts", LocatorLabel::Part),
    ("part", LocatorLabel::Part),
    ("table", LocatorLabel::Table),
    ("folio", LocatorLabel::Folio),
    ("issue", LocatorLabel::Issue),
    ("book", LocatorLabel::Book),
    ("opus", LocatorLabel::Opus),
    ("chaps.", LocatorLabel::Chapter),
    ("chap.", LocatorLabel::Chapter),
    ("chs.", LocatorLabel::Chapter),
    ("ch.", LocatorLabel::Chapter),
    ("secs.", LocatorLabel::Section),
    ("sec.", LocatorLabel::Section),
    ("paras.", LocatorLabel::Paragraph),
    ("para.", LocatorLabel::Paragraph),
    ("figs.", LocatorLabel::Figure),
    ("fig.", LocatorLabel::Figure),
    ("vols.", LocatorLabel::Volume),
    ("vol.", LocatorLabel::Volume),
    ("cols.", LocatorLabel::Column),
    ("col.", LocatorLabel::Column),
    ("app.", LocatorLabel::Appendix),
    ("eq.", LocatorLabel::Equation),
    ("no.", LocatorLabel::Number),
    ("nos.", LocatorLabel::Number),
    ("op.", LocatorLabel::Opus),
    ("pts.", LocatorLabel::Part),
    ("pt.", LocatorLabel::Part),
    ("tbl.", LocatorLabel::Table),
    ("fol.", LocatorLabel::Folio),
    ("bk.", LocatorLabel::Book),
    ("vv.", LocatorLabel::Verse),
    ("v.", LocatorLabel::Verse),
    ("ll.", LocatorLabel::Line),
    ("l.", LocatorLabel::Line),
    ("nn.", LocatorLabel::Note),
    ("n.", LocatorLabel::Note),
    ("s.v.", LocatorLabel::SubVerbo),
    ("pp.", LocatorLabel::Page),
    ("p.", LocatorLabel::Page),
    ("¶¶", LocatorLabel::Paragraph),
    ("¶", LocatorLabel::Paragraph),
    ("§§", LocatorLabel::Section),
    ("§", LocatorLabel::Section),
];

impl LocatorLabel {
    /// The CSL locator type.
    pub(crate) fn csl(self) -> CslLocator {
        use LocatorLabel::*;
        match self {
            Page => CslLocator::Page,
            Chapter => CslLocator::Chapter,
            Section => CslLocator::Section,
            Paragraph => CslLocator::Paragraph,
            Figure => CslLocator::Figure,
            Volume => CslLocator::Volume,
            Line => CslLocator::Line,
            Note => CslLocator::Note,
            Verse => CslLocator::Verse,
            Book => CslLocator::Book,
            Column => CslLocator::Column,
            Part => CslLocator::Part,
            Table => CslLocator::Table,
            Equation => CslLocator::Equation,
            Folio => CslLocator::Folio,
            Issue => CslLocator::Issue,
            Number => CslLocator::Issue,
            Opus => CslLocator::Opus,
            SubVerbo => CslLocator::SubVerbo,
            Appendix => CslLocator::Appendix,
            Timestamp => CslLocator::Timestamp,
        }
    }

    /// The abbreviation to write in Markdown (`p.`, `chap.`).
    pub fn abbreviation(self, plural: bool) -> &'static str {
        use LocatorLabel::*;
        match (self, plural) {
            (Page, false) => "p.",
            (Page, true) => "pp.",
            (Chapter, _) => "chap.",
            (Section, _) => "sec.",
            (Paragraph, _) => "para.",
            (Figure, _) => "fig.",
            (Volume, _) => "vol.",
            (Line, false) => "l.",
            (Line, true) => "ll.",
            (Note, false) => "n.",
            (Note, true) => "nn.",
            (Verse, false) => "v.",
            (Verse, true) => "vv.",
            (Book, _) => "bk.",
            (Column, _) => "col.",
            (Part, _) => "pt.",
            (Table, _) => "tbl.",
            (Equation, _) => "eq.",
            (Folio, _) => "fol.",
            (Issue, _) => "issue",
            (Number, _) => "no.",
            (Opus, _) => "op.",
            (SubVerbo, _) => "s.v.",
            (Appendix, _) => "app.",
            (Timestamp, _) => "at",
        }
    }

    /// The word to say aloud ("page", "pages", "chapter").
    pub fn spoken(self, plural: bool) -> &'static str {
        use LocatorLabel::*;
        match (self, plural) {
            (Page, false) => "page",
            (Page, true) => "pages",
            (Chapter, false) => "chapter",
            (Chapter, true) => "chapters",
            (Section, false) => "section",
            (Section, true) => "sections",
            (Paragraph, false) => "paragraph",
            (Paragraph, true) => "paragraphs",
            (Figure, false) => "figure",
            (Figure, true) => "figures",
            (Volume, false) => "volume",
            (Volume, true) => "volumes",
            (Line, false) => "line",
            (Line, true) => "lines",
            (Note, false) => "note",
            (Note, true) => "notes",
            (Verse, false) => "verse",
            (Verse, true) => "verses",
            (Book, _) => "book",
            (Column, _) => "column",
            (Part, _) => "part",
            (Table, _) => "table",
            (Equation, _) => "equation",
            (Folio, _) => "folio",
            (Issue, _) => "issue",
            (Number, _) => "number",
            (Opus, _) => "opus",
            (SubVerbo, _) => "under the word",
            (Appendix, _) => "appendix",
            (Timestamp, _) => "at",
        }
    }
}

impl Locator {
    /// Whether the value names more than one division ("12-15", "3, 5").
    pub fn is_plural(&self) -> bool {
        self.value.contains(['-', '–', ',', '&'])
    }

    /// "p. 12" or "pp. 12-15", as written in Markdown.
    pub fn written(&self) -> String {
        format!(
            "{} {}",
            self.label.abbreviation(self.is_plural()),
            self.value
        )
    }

    /// "page 12" or "pages 12 to 15", for speech.
    pub fn spoken(&self) -> String {
        let value = self.value.replace(['-', '–'], " to ");
        let value = crate::text::collapse_whitespace(&value);
        format!("{} {}", self.label.spoken(self.is_plural()), value)
    }
}

/// Every citation in `text`, in order.
pub fn find_citations(text: &str) -> Vec<Citation> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                i += 2;
                continue;
            }
            b'[' => {
                if let Some((end, items)) = parse_bracketed(text, i) {
                    out.push(Citation {
                        range: i..end,
                        chars: CharRange::default(),
                        narrative: false,
                        items,
                    });
                    i = end;
                    continue;
                }
            }
            b'@' => {
                let before = text[..i].chars().next_back();
                let starts_ok = before.is_none_or(|c| {
                    c.is_whitespace() || matches!(c, '(' | '"' | '\'' | '“' | '‘' | '—' | '–')
                });
                if starts_ok && let Some((key, key_end)) = parse_key(text, i + 1) {
                    let (end, locator, suffix) = parse_trailing_locator(text, key_end);
                    out.push(Citation {
                        range: i..end,
                        chars: CharRange::default(),
                        narrative: true,
                        items: vec![CiteItem {
                            key,
                            locator,
                            suffix,
                            ..CiteItem::default()
                        }],
                    });
                    i = end;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }
    fill_char_ranges(text, &mut out);
    out
}

/// Whether `c` counts as a citation, by the rules `tw convert` and the
/// reader share. With the Pandoc flavor (`pandoc`) every citation does, as
/// in Pandoc. With the other flavors, where `@name` is more often a mention
/// than a citation, a bracketed citation counts when one of its keys is
/// `known` (in a library), and an in-text `@key` only when all are.
pub fn counts_as_citation(c: &Citation, known: impl Fn(&str) -> bool, pandoc: bool) -> bool {
    pandoc
        || if c.narrative {
            c.items.iter().all(|i| known(&i.key))
        } else {
            c.items.iter().any(|i| known(&i.key))
        }
}

/// The citation containing character position `pos` (or ending exactly
/// there), for "what am I on?" and editing.
pub fn citation_at(text: &str, pos: usize) -> Option<Citation> {
    find_citations(text)
        .into_iter()
        .find(|c| c.chars.start.0 <= pos && pos <= c.chars.end.0)
}

fn fill_char_ranges(text: &str, cites: &mut [Citation]) {
    let mut char_idx = 0usize;
    let mut byte_idx = 0usize;
    let mut chars = text.char_indices().peekable();
    let mut to_char = |target: usize| -> usize {
        while byte_idx < target {
            match chars.next() {
                Some((b, c)) => {
                    byte_idx = b + c.len_utf8();
                    char_idx += 1;
                }
                None => break,
            }
        }
        char_idx
    };
    for c in cites.iter_mut() {
        let s = to_char(c.range.start);
        let e = to_char(c.range.end);
        c.chars = CharRange::new(s, e);
    }
}

/// Parses a key starting at byte `start` (just after `@`); returns the key
/// and the byte after it.
fn parse_key(text: &str, start: usize) -> Option<(String, usize)> {
    let rest = &text[start..];
    if let Some(inner) = rest.strip_prefix('{') {
        let close = inner.find('}')?;
        let key = inner[..close].trim();
        return (!key.is_empty()).then(|| (key.to_owned(), start + 1 + close + 1));
    }
    let mut chars = rest.char_indices().peekable();
    let (_, first) = chars.next()?;
    if !(first.is_alphanumeric() || first == '_') {
        return None;
    }
    let mut end = first.len_utf8();
    while let Some(&(idx, c)) = chars.peek() {
        if c.is_alphanumeric() || c == '_' {
            end = idx + c.len_utf8();
            chars.next();
        } else if is_internal_punct(c) {
            let mut look = chars.clone();
            look.next();
            match look.peek() {
                Some(&(_, n)) if n.is_alphanumeric() || n == '_' => {
                    chars.next();
                }
                _ => break,
            }
        } else {
            break;
        }
    }
    Some((rest[..end].to_owned(), start + end))
}

/// `@key [p. 33]`: a following bracket without `@` is the in-text
/// citation's locator and suffix.
fn parse_trailing_locator(text: &str, key_end: usize) -> (usize, Option<Locator>, String) {
    let rest = &text[key_end..];
    let Some(after_space) = rest.strip_prefix(' ') else {
        return (key_end, None, String::new());
    };
    if !after_space.starts_with('[') {
        return (key_end, None, String::new());
    }
    let Some(close) = matching_bracket(after_space, 0) else {
        return (key_end, None, String::new());
    };
    let inner = &after_space[1..close];
    let after = &after_space[close + 1..];
    if inner.contains('@') || after.starts_with('(') || after.starts_with('[') {
        return (key_end, None, String::new());
    }
    let (locator, suffix) = split_locator(inner);
    (key_end + 1 + close + 1, locator, suffix)
}

/// Byte index of the `]` matching the `[` at `open`, skipping escapes and
/// nested brackets.
fn matching_bracket(s: &str, open: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn parse_bracketed(text: &str, open: usize) -> Option<(usize, Vec<CiteItem>)> {
    let close = open + matching_bracket(&text[open..], 0)?;
    let after = &text[close + 1..];
    if after.starts_with('(') || after.starts_with('[') {
        return None; // a link or reference-style link
    }
    let inner = &text[open + 1..close];
    if !inner.contains('@') {
        return None;
    }
    let items: Option<Vec<CiteItem>> = split_items(inner).into_iter().map(parse_item).collect();
    let items = items?;
    (!items.is_empty()).then_some((close + 1, items))
}

/// Splits on `;` outside braces.
fn split_items(inner: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, c) in inner.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            ';' if depth <= 0 => {
                parts.push(&inner[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&inner[start..]);
    parts
}

fn parse_item(part: &str) -> Option<CiteItem> {
    // The key's `@` is the first one at the start or after a space or `-`.
    let at = part.char_indices().find_map(|(i, c)| {
        if c != '@' {
            return None;
        }
        let before = part[..i].chars().next_back();
        before
            .is_none_or(|b| b.is_whitespace() || b == '-')
            .then_some(i)
    })?;
    let (key, key_end) = parse_key(part, at + 1)?;
    let before = &part[..at];
    let (prefix, suppress) = match before.strip_suffix('-') {
        Some(p) if p.is_empty() || p.ends_with(char::is_whitespace) => (p, true),
        _ => (before, false),
    };
    let rest = &part[key_end..];
    if rest.contains('@')
        && rest
            .split_whitespace()
            .any(|w| w.starts_with('@') || w.starts_with("-@"))
    {
        return None; // two keys in one item: not Pandoc syntax
    }
    let (locator, suffix) = split_locator(rest);
    Some(CiteItem {
        key,
        prefix: crate::text::collapse_whitespace(prefix),
        locator,
        suffix,
        suppress_author: suppress,
    })
}

/// Splits the text after a key into a locator and the remaining suffix.
fn split_locator(rest: &str) -> (Option<Locator>, String) {
    let trimmed = rest.trim_start();
    let body = match trimmed.strip_prefix(',') {
        Some(b) => b.trim_start(),
        None if trimmed.is_empty() => return (None, String::new()),
        // In-text `@key [p. 3]` brackets have no comma.
        None => trimmed,
    };
    let lower = body.to_lowercase();
    let mut label = None;
    let mut value_start = 0;
    for (term, l) in TERMS {
        if lower.starts_with(term) {
            let after = &body[term.len()..];
            // A word term must end at a word boundary ("page 3", not "pages3x").
            let boundary = term.ends_with('.')
                || term.starts_with(['¶', '§'])
                || after.starts_with(|c: char| !c.is_alphanumeric())
                || after.is_empty();
            if boundary {
                label = Some(*l);
                value_start = term.len();
                break;
            }
        }
    }
    let value_part = body[value_start..].trim_start();
    let (value, tail) = if let Some(inner) = value_part.strip_prefix('{') {
        match inner.find('}') {
            Some(close) => (inner[..close].trim().to_owned(), &inner[close + 1..]),
            None => (String::new(), value_part),
        }
    } else {
        let len = locator_value_len(value_part);
        (
            value_part[..len]
                .trim_end()
                .trim_end_matches(',')
                .to_owned(),
            &value_part[len..],
        )
    };
    match (label, value.is_empty()) {
        (Some(l), false) => (Some(Locator { label: l, value }), normalize_suffix(tail)),
        (None, false) => (
            Some(Locator {
                label: LocatorLabel::Page,
                value,
            }),
            normalize_suffix(tail),
        ),
        _ => (None, normalize_suffix(trimmed)),
    }
}

/// Length of a locator value: numbers, roman numerals, and letters glued to
/// digits (`33`, `iv`, `12a`, `33-35`, `3, 5`, `1 & 3`).
fn locator_value_len(s: &str) -> usize {
    let is_token = |w: &str| {
        let w = w.trim_end_matches(',');
        !w.is_empty()
            && (w.chars().any(|c| c.is_ascii_digit())
                && w.chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '-' | '–' | ':' | '.'))
                || w.chars().all(|c| "ivxlcdmIVXLCDM".contains(c))
                || w.contains(['-', '–'])
                    && w.split(['-', '–'])
                        .all(|p| !p.is_empty() && p.chars().all(|c| "ivxlcdmIVXLCDM".contains(c))))
    };
    let mut end = 0;
    let mut pos = 0;
    let mut expect_value = true;
    for word in s.split_inclusive(' ') {
        let w = word.trim();
        let ok = if expect_value {
            is_token(w)
        } else {
            matches!(w, "&" | "and" | "-" | "–")
        };
        if !ok {
            break;
        }
        pos += word.len();
        if expect_value {
            end = pos;
            // A comma after a value lets another value follow ("3, 5");
            // otherwise a connector must come next.
            expect_value = w.ends_with(',');
            if expect_value
                && !s[pos..]
                    .trim_start()
                    .starts_with(|c: char| c.is_ascii_digit())
            {
                break;
            }
        } else {
            expect_value = true;
        }
    }
    end
}

fn normalize_suffix(s: &str) -> String {
    let t = crate::text::collapse_whitespace(s);
    if t.is_empty() || t.starts_with([',', '.', ';', ':', ')']) {
        t
    } else {
        format!(", {t}").replacen(", ,", ",", 1)
    }
}

/// Writes a citation in Pandoc syntax, for inserting at the cursor:
/// `[@doe2020]`, `[@doe2020, p. 12]`, `[@a; @b]`, or in-text
/// `@doe2020 [p. 12]`. Keys that are not plain Pandoc keys are braced.
pub fn write_citation(items: &[CiteItem], narrative: bool) -> String {
    fn key(k: &str) -> String {
        if crate::key::is_valid_key(k) {
            format!("@{k}")
        } else {
            format!("@{{{k}}}")
        }
    }
    fn tail(item: &CiteItem) -> String {
        let mut s = String::new();
        if let Some(l) = &item.locator {
            s.push_str(", ");
            s.push_str(&l.written());
        }
        if !item.suffix.is_empty() {
            if !item.suffix.starts_with([',', '.', ';', ':', ')']) {
                s.push(' ');
            }
            s.push_str(&item.suffix);
        }
        s
    }
    if narrative && let [item] = items {
        let t = tail(item);
        return match t.strip_prefix(", ") {
            Some(inner) => format!("{} [{inner}]", key(&item.key)),
            None if t.is_empty() => key(&item.key),
            None => format!("{} [{}]", key(&item.key), t.trim_start()),
        };
    }
    let parts: Vec<String> = items
        .iter()
        .map(|item| {
            let mut s = String::new();
            if !item.prefix.is_empty() {
                s.push_str(&item.prefix);
                s.push(' ');
            }
            if item.suppress_author {
                s.push('-');
            }
            s.push_str(&key(&item.key));
            s.push_str(&tail(item));
            s
        })
        .collect();
    format!("[{}]", parts.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(text: &str) -> Vec<Vec<String>> {
        find_citations(text)
            .into_iter()
            .map(|c| c.items.into_iter().map(|i| i.key).collect())
            .collect()
    }

    #[test]
    fn simple_and_grouped() {
        assert_eq!(keys("As shown [@doe2020]."), vec![vec!["doe2020"]]);
        assert_eq!(keys("[@a; @b; @c]"), vec![vec!["a", "b", "c"]]);
        let c = &find_citations("[see @doe2020, pp. 33-35, emphasis added; also @roe, chap. 1]")[0];
        assert_eq!(c.items[0].prefix, "see");
        assert_eq!(
            c.items[0].locator,
            Some(Locator {
                label: LocatorLabel::Page,
                value: "33-35".into()
            })
        );
        assert_eq!(c.items[0].suffix, ", emphasis added");
        assert_eq!(c.items[1].prefix, "also");
        assert_eq!(
            c.items[1].locator.as_ref().map(|l| l.label),
            Some(LocatorLabel::Chapter)
        );
    }

    #[test]
    fn bare_numbers_are_pages_and_braces_are_verbatim() {
        let c = &find_citations("[@doe, 12]")[0];
        assert_eq!(
            c.items[0].locator.as_ref().map(Locator::written).as_deref(),
            Some("p. 12")
        );
        let c = &find_citations("[@doe, p. {iv, 34-37}]")[0];
        assert_eq!(
            c.items[0].locator.as_ref().map(|l| l.value.as_str()),
            Some("iv, 34-37")
        );
        let c = &find_citations("[@doe, pp. 3, 5]")[0];
        assert_eq!(
            c.items[0].locator.as_ref().map(|l| l.value.as_str()),
            Some("3, 5")
        );
    }

    #[test]
    fn suppress_author_and_narrative() {
        let c = &find_citations("Doe says [-@doe2020, p. 3].")[0];
        assert!(c.items[0].suppress_author);
        let c = &find_citations("@doe2020 [p. 33] says so.")[0];
        assert!(c.narrative);
        assert_eq!(
            c.items[0].locator.as_ref().map(Locator::spoken).as_deref(),
            Some("page 33")
        );
        assert_eq!(
            &"@doe2020 [p. 33] says so."[c.range.clone()],
            "@doe2020 [p. 33]"
        );
    }

    #[test]
    fn no_false_positives() {
        assert!(keys("Mail jon@example.com or [a link](http://x) or \\[@not].").is_empty());
        assert!(keys("[@doe](http://example.com)").is_empty());
        assert!(keys("[no citation here]").is_empty());
        assert_eq!(keys("Trailing @doe2020."), vec![vec!["doe2020"]]);
        assert_eq!(keys("@{odd key!}"), vec![vec!["odd key!"]]);
    }

    #[test]
    fn char_ranges_count_scalars() {
        let text = "Café naïve [@doe] and ¶ @roe";
        let cites = find_citations(text);
        assert_eq!(cites[0].chars, CharRange::new(11, 17));
        let chars: Vec<char> = text.chars().collect();
        let s: String = chars[cites[1].chars.start.0..cites[1].chars.end.0]
            .iter()
            .collect();
        assert_eq!(s, "@roe");
        assert_eq!(
            citation_at(text, 12)
                .map(|c| c.items[0].key.clone())
                .as_deref(),
            Some("doe")
        );
        assert!(citation_at(text, 3).is_none());
    }

    #[test]
    fn writing_round_trips() {
        for src in [
            "[@doe2020]",
            "[see @a, p. 12; -@b, chap. 3, emphasis added]",
            "@doe [p. 33]",
            "[@{odd key}]",
        ] {
            let c = &find_citations(src)[0];
            assert_eq!(write_citation(&c.items, c.narrative), src);
        }
    }
}
