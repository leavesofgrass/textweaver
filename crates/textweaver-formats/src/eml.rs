//! Email (`.eml`) and web archives (`.mhtml`, `.mht`), both MIME messages
//! read with mail-parser (ADR-0035).
//!
//! **Email.** The subject is a level-1 heading, followed by a short block
//! of headers, one to a line: From, To, Cc, and Date (written out with its
//! weekday, computed from the date). Then the body: each `text/plain` part
//! as paragraphs, with quoted lines (`> ...`) under a `Quote` marker, or,
//! when a message has no plain text, its `text/html` part through the HTML
//! loader. Last, the attachments under an "Attachments" heading, each by
//! name and size; none is opened.
//!
//! **Web archives** (RFC 2557, as browsers save a page with its pictures):
//! the root HTML part is the one the `multipart/related` part's `start`
//! parameter names, else its first HTML part. It is read through the HTML
//! loader, with each `href` and `src` resolved: `cid:` addresses to their
//! part, and relative addresses against the root's `Content-Location`,
//! so a link leads to the page it named on the web, and a picture with no
//! alternative text is described by its part's `Content-Description`.
//!
//! **Limits.** A file over [`MAX_MESSAGE_BYTES`] is refused, as is one with
//! more than [`MAX_PARTS`] parts or parts nested deeper than [`MAX_DEPTH`].
//! Past [`MAX_TEXT_BYTES`] of body text the rest is left out, and the
//! document says so. mail-parser itself is safe Rust, parses without
//! recursion except for attached messages (three deep), and never panics
//! on malformed input.

use std::collections::HashMap;

use mail_parser::{Address, DateTime, Message, MessageParser, MessagePart, MimeHeaders, PartType};
use ropey::Rope;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, DocumentMeta, Marker};

use crate::builder::Builder;
use crate::html::{self, Resolved};
use crate::{LoadError, LoadOptions, Loader, Source, add_warning, meta_for, title_from_path};

/// Largest email or web archive read.
pub const MAX_MESSAGE_BYTES: usize = 128 << 20;

/// Most MIME parts in one message.
pub const MAX_PARTS: usize = 10_000;

/// Deepest nesting of multipart parts.
pub const MAX_DEPTH: usize = 32;

/// Most body text read (plain and HTML together).
pub const MAX_TEXT_BYTES: usize = 64 << 20;

/// The warning a message carries when its text was cut at
/// [`MAX_TEXT_BYTES`].
pub const TEXT_CUT_WARNING: &str =
    "This message is very long, so only the first 64 megabytes of its text are read.";

/// Loads email messages (`.eml`).
#[derive(Clone, Copy, Debug, Default)]
pub struct EmlLoader;

/// Loads web archives (`.mhtml`, `.mht`).
#[derive(Clone, Copy, Debug, Default)]
pub struct MhtmlLoader;

impl Loader for EmlLoader {
    fn id(&self) -> &'static str {
        "eml"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["eml"]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let bytes = read(source)?;
        let message = parse(&bytes, "an email message")?;
        let mut meta = meta_for(source, self.id());
        let (text, markers) = email(&message, options, &mut meta);
        if meta.title.is_none() {
            meta.title = title_from_path(source);
        }
        Ok(Document::new(meta, Rope::from_str(&text), markers))
    }
}

impl Loader for MhtmlLoader {
    fn id(&self) -> &'static str {
        "mhtml"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["mhtml", "mht"]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let bytes = read(source)?;
        let message = parse(&bytes, "a web archive")?;
        let mut meta = meta_for(source, self.id());
        let (text, markers) = web_archive(&message, options, &mut meta)?;
        if meta.title.is_none() {
            meta.title = markers
                .iter()
                .filter(|m| m.kind == MarkerKind::Heading && m.level == 1)
                .min_by_key(|m| m.range.start)
                .map(|m| {
                    text.chars()
                        .skip(m.range.start.0)
                        .take(m.range.len())
                        .collect()
                })
                .or_else(|| title_from_path(source));
        }
        Ok(Document::new(meta, Rope::from_str(&text), markers))
    }
}

/// The source's bytes, refused past [`MAX_MESSAGE_BYTES`].
fn read(source: &Source) -> Result<Vec<u8>, LoadError> {
    let bytes = source.read_head(MAX_MESSAGE_BYTES + 1)?;
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(LoadError::Parse(format!(
            "the file is larger than {} MB",
            MAX_MESSAGE_BYTES >> 20
        )));
    }
    Ok(bytes)
}

/// Parses a MIME message and checks its shape against the limits. `what`
/// names it in the error ("an email message").
fn parse<'x>(bytes: &'x [u8], what: &str) -> Result<Message<'x>, LoadError> {
    let message = MessageParser::new()
        .parse(bytes)
        .filter(|m| !m.parts.is_empty())
        .ok_or_else(|| LoadError::Parse(format!("not {what}: it has no headers")))?;
    if message.parts.len() > MAX_PARTS {
        return Err(LoadError::Parse(format!(
            "it has more than {MAX_PARTS} parts"
        )));
    }
    if depth(&message) > MAX_DEPTH {
        return Err(LoadError::Parse(format!(
            "its parts are nested more than {MAX_DEPTH} levels deep"
        )));
    }
    Ok(message)
}

/// How deep the message's multipart parts nest, walked without recursion
/// and visiting each part once.
fn depth(message: &Message<'_>) -> usize {
    let mut seen = vec![false; message.parts.len()];
    let mut stack = vec![(0usize, 1usize)];
    let mut deepest = 0;
    while let Some((id, d)) = stack.pop() {
        let Some(part) = message.parts.get(id) else {
            continue;
        };
        if std::mem::replace(&mut seen[id], true) {
            continue;
        }
        deepest = deepest.max(d);
        if d > MAX_DEPTH {
            break;
        }
        if let Some(subs) = part.sub_parts() {
            stack.extend(subs.iter().map(|&s| (s as usize, d + 1)));
        }
    }
    deepest
}

// ---------------------------------------------------------------------
// Email

fn email(
    message: &Message<'_>,
    options: &LoadOptions,
    meta: &mut DocumentMeta,
) -> (String, Vec<Marker>) {
    let mut b = Builder::new();
    let subject = message.subject().map(collapse).filter(|s| !s.is_empty());
    meta.title.clone_from(&subject);
    meta.author = message.from().and_then(first_name);
    if let Some(lang) = message.root_part().content_language().as_text() {
        meta.language = Some(lang.trim().to_owned()).filter(|l| !l.is_empty());
    }
    if let Some(date) = message.date() {
        meta.properties.insert("date".into(), date.to_rfc3339());
    }

    // The subject, then the headers.
    let h = b.open(Marker::new(MarkerKind::Heading, CharRange::empty(0)).with_level(1));
    b.text(subject.as_deref().unwrap_or("No subject"));
    b.close(h);
    b.paragraph_break();
    let p = b.open(Marker::new(MarkerKind::Paragraph, CharRange::empty(0)));
    let mut first = true;
    let mut line = |b: &mut Builder, label: &str, value: String| {
        if value.is_empty() {
            return;
        }
        if !first {
            b.line_break();
        }
        first = false;
        b.text(&format!("{label}: {value}"));
    };
    line(
        &mut b,
        "From",
        message.from().map(addresses).unwrap_or_default(),
    );
    line(
        &mut b,
        "To",
        message.to().map(addresses).unwrap_or_default(),
    );
    line(
        &mut b,
        "Cc",
        message.cc().map(addresses).unwrap_or_default(),
    );
    line(
        &mut b,
        "Date",
        message.date().map(spoken_date).unwrap_or_default(),
    );
    b.close(p);
    b.paragraph_break();

    // The body.
    let resolver = PartResolver::new(message, None);
    let resolve = |r: &str| resolver.resolve(r);
    let mut budget = MAX_TEXT_BYTES;
    let mut cut = false;
    for &id in &message.text_body {
        let Some(part) = message.parts.get(id as usize) else {
            continue;
        };
        match &part.body {
            PartType::Text(text) => {
                let text = limit(text, &mut budget, &mut cut);
                plain_body(&mut b, text);
            }
            PartType::Html(text) => {
                let text = limit(text, &mut budget, &mut cut);
                let mut html_meta = DocumentMeta::default();
                b.paragraph_break();
                html::walk_resolved_into(&mut b, text, options, &mut html_meta, &resolve);
                for w in crate::warnings(&html_meta) {
                    add_warning(meta, &w);
                }
            }
            _ => {}
        }
        b.paragraph_break();
    }
    if cut {
        add_warning(meta, TEXT_CUT_WARNING);
    }

    // The attachments, listed.
    let attachments: Vec<String> = message
        .attachments
        .iter()
        .filter_map(|&id| message.parts.get(id as usize))
        .filter_map(describe_attachment)
        .collect();
    if !attachments.is_empty() {
        b.paragraph_break();
        let h = b.open(Marker::new(MarkerKind::Heading, CharRange::empty(0)).with_level(2));
        b.text("Attachments");
        b.close(h);
        b.paragraph_break();
        let list = b.open(Marker::new(MarkerKind::List, CharRange::empty(0)).with_level(1));
        for a in &attachments {
            b.line_break();
            let item = b.open(Marker::new(MarkerKind::ListItem, CharRange::empty(0)).with_level(1));
            b.text(a);
            b.close(item);
        }
        b.close(list);
    }
    b.finish()
}

/// At most `budget` bytes of `text` (cut at a character boundary).
fn limit<'t>(text: &'t str, budget: &mut usize, cut: &mut bool) -> &'t str {
    if text.len() <= *budget {
        *budget -= text.len();
        return text;
    }
    *cut = true;
    let mut end = *budget;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    *budget = 0;
    &text[..end]
}

/// A plain-text body: paragraphs at blank lines, lines kept, and quoted
/// lines (`>`) under a `Quote` marker without their `>` marks.
fn plain_body(b: &mut Builder, text: &str) {
    let text = crate::normalize_newlines(text);
    let mut block: Vec<&str> = Vec::new();
    let mut quoted = false;
    let flush = |b: &mut Builder, block: &mut Vec<&str>, quoted: bool| {
        if block.is_empty() {
            return;
        }
        b.paragraph_break();
        let kind = if quoted {
            MarkerKind::Quote
        } else {
            MarkerKind::Paragraph
        };
        let m = b.open(Marker::new(kind, CharRange::empty(0)));
        for (i, line) in block.iter().enumerate() {
            if i > 0 {
                b.line_break();
            }
            b.text(line);
        }
        b.close(m);
        block.clear();
    };
    for line in text.lines() {
        let trimmed = line.trim_start();
        let is_quote = trimmed.starts_with('>');
        let content = if is_quote {
            trimmed.trim_start_matches(['>', ' '])
        } else {
            line
        };
        if line.trim().is_empty() || (is_quote && content.trim().is_empty() && !quoted) {
            flush(b, &mut block, quoted);
            continue;
        }
        if is_quote != quoted {
            flush(b, &mut block, quoted);
            quoted = is_quote;
        }
        if !content.trim().is_empty() {
            block.push(content);
        }
    }
    flush(b, &mut block, quoted);
}

/// "report.pdf, 240 KB", or `None` for a part with no name that is not a
/// message.
fn describe_attachment(part: &MessagePart<'_>) -> Option<String> {
    if let PartType::Message(m) = &part.body {
        let subject = m.subject().map(collapse).unwrap_or_default();
        return Some(if subject.is_empty() {
            "Attached message".into()
        } else {
            format!("Attached message: {subject}")
        });
    }
    let name = part
        .attachment_name()
        .map(collapse)
        .filter(|n| !n.is_empty())?;
    Some(format!("{name}, {}", size(part.len())))
}

/// A size in words a listener can take in: "240 KB", "3.1 MB".
fn size(bytes: usize) -> String {
    const KB: usize = 1024;
    if bytes < KB {
        return format!("{bytes} bytes");
    }
    if bytes < KB * KB {
        return format!("{} KB", bytes.div_ceil(KB));
    }
    let tenths = (bytes * 10).div_ceil(KB * KB);
    format!("{}.{} MB", tenths / 10, tenths % 10)
}

/// "Ada Example (ada@example.org), bo@example.org".
fn addresses(address: &Address<'_>) -> String {
    address
        .iter()
        .filter_map(|a| {
            let name = a.name().map(collapse).filter(|n| !n.is_empty());
            let addr = a.address().map(str::trim).filter(|n| !n.is_empty());
            match (name, addr) {
                (Some(n), Some(a)) if n != a => Some(format!("{n} ({a})")),
                (Some(n), _) => Some(n),
                (None, Some(a)) => Some(a.to_owned()),
                (None, None) => None,
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The first sender's name, else address.
fn first_name(address: &Address<'_>) -> Option<String> {
    let a = address.first()?;
    a.name()
        .map(collapse)
        .filter(|n| !n.is_empty())
        .or_else(|| a.address().map(str::to_owned))
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// "Monday, September 28, 2026, 10:15, UTC minus 7": the weekday computed
/// from the date (never recalled), the time zone in words.
fn spoken_date(d: &DateTime) -> String {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let Some(month) = MONTHS.get(usize::from(d.month).wrapping_sub(1)) else {
        return String::new();
    };
    if d.day == 0 || d.day > 31 {
        return String::new();
    }
    let weekday = weekday(i64::from(d.year), u32::from(d.month), u32::from(d.day));
    let zone = if d.tz_hour == 0 && d.tz_minute == 0 {
        "UTC".to_owned()
    } else {
        let sign = if d.tz_before_gmt { "minus" } else { "plus" };
        if d.tz_minute == 0 {
            format!("UTC {sign} {}", d.tz_hour)
        } else {
            format!("UTC {sign} {}:{:02}", d.tz_hour, d.tz_minute)
        }
    };
    format!(
        "{weekday}, {month} {}, {}, {}:{:02}, {zone}",
        d.day, d.year, d.hour, d.minute
    )
}

/// The weekday of a date in the proleptic Gregorian calendar.
fn weekday(year: i64, month: u32, day: u32) -> &'static str {
    const NAMES: [&str; 7] = [
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
    ];
    // Days since 1970-01-01 (a Thursday), Howard Hinnant's algorithm.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(month);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let i = usize::try_from(days.rem_euclid(7)).unwrap_or(0);
    NAMES[i]
}

// ---------------------------------------------------------------------
// Web archives

fn web_archive(
    message: &Message<'_>,
    options: &LoadOptions,
    meta: &mut DocumentMeta,
) -> Result<(String, Vec<Marker>), LoadError> {
    let root_id = root_part(message)
        .ok_or_else(|| LoadError::Parse("it holds no web page (no text/html part)".into()))?;
    let root = &message.parts[root_id];
    let base = root.content_location().map(str::trim).map(str::to_owned);
    let text = match &root.body {
        PartType::Html(t) | PartType::Text(t) => t.as_ref(),
        _ => "",
    };
    let mut budget = MAX_TEXT_BYTES;
    let mut cut = false;
    let text = limit(text, &mut budget, &mut cut);
    let resolver = PartResolver::new(message, base.clone());
    let resolve = |r: &str| resolver.resolve(r);
    let mut b = Builder::new();
    html::walk_resolved_into(&mut b, text, options, meta, &resolve);
    if cut {
        add_warning(meta, TEXT_CUT_WARNING);
    }
    if let Some(base) = base {
        meta.properties.insert("source-address".into(), base);
    }
    Ok(b.finish())
}

/// The root HTML part: the one `start` names, else the first HTML part.
fn root_part(message: &Message<'_>) -> Option<usize> {
    let root = message.parts.first()?;
    if let Some(start) = root
        .content_type()
        .and_then(|ct| ct.attribute("start"))
        .map(bare_id)
    {
        let found = message
            .parts
            .iter()
            .position(|p| p.content_id().map(bare_id) == Some(start));
        if found.is_some() {
            return found;
        }
    }
    message
        .parts
        .iter()
        .position(|p| matches!(p.body, PartType::Html(_)))
}

/// A Content-ID without its angle brackets.
fn bare_id(id: &str) -> &str {
    id.trim().trim_start_matches('<').trim_end_matches('>')
}

/// Resolves the references in an archived page against its parts.
struct PartResolver {
    base: Option<String>,
    /// `Content-Location` and `cid:` addresses, to each part's location
    /// and description.
    parts: HashMap<String, (Option<String>, Option<String>)>,
}

impl PartResolver {
    fn new(message: &Message<'_>, base: Option<String>) -> Self {
        let mut parts = HashMap::new();
        for p in &message.parts {
            let location = p.content_location().map(|l| l.trim().to_owned());
            let description = p
                .content_description()
                .map(collapse)
                .filter(|d| !d.is_empty());
            if let Some(l) = &location {
                parts
                    .entry(l.clone())
                    .or_insert_with(|| (location.clone(), description.clone()));
            }
            if let Some(id) = p.content_id() {
                parts
                    .entry(format!("cid:{}", bare_id(id)))
                    .or_insert_with(|| (location.clone(), description.clone()));
            }
        }
        PartResolver { base, parts }
    }

    fn resolve(&self, reference: &str) -> Option<Resolved> {
        let reference = reference.trim();
        if reference.is_empty() || reference.starts_with('#') {
            return None;
        }
        if let Some(id) = reference
            .get(..4)
            .filter(|p| p.eq_ignore_ascii_case("cid:"))
            .map(|_| &reference[4..])
        {
            let (location, description) = self.parts.get(&format!("cid:{}", bare_id(id)))?;
            return Some(Resolved {
                target: location.clone().unwrap_or_else(|| reference.to_owned()),
                description: description.clone(),
            });
        }
        let target = match &self.base {
            Some(base) => join(base, reference),
            None => reference.to_owned(),
        };
        let description = self
            .parts
            .get(&target)
            .or_else(|| self.parts.get(reference))
            .and_then(|(_, d)| d.clone());
        (target != reference || description.is_some()).then_some(Resolved {
            target,
            description,
        })
    }
}

/// `reference` resolved against the absolute address `base`, as a browser
/// resolves a relative link (a small subset of RFC 3986: scheme-relative,
/// root-relative, and path-relative addresses, with `.` and `..` steps).
fn join(base: &str, reference: &str) -> String {
    let has_scheme = |s: &str| {
        s.split_once(':').is_some_and(|(scheme, _)| {
            !scheme.is_empty()
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
        })
    };
    if has_scheme(reference) || !has_scheme(base) {
        return reference.to_owned();
    }
    let Some((scheme, rest)) = base.split_once("://") else {
        return reference.to_owned();
    };
    if let Some(r) = reference.strip_prefix("//") {
        return format!("{scheme}://{r}");
    }
    let (host, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let path = path.split(['?', '#']).next().unwrap_or("/");
    let (reference, suffix) = match reference.find(['?', '#']) {
        Some(i) => (&reference[..i], &reference[i..]),
        None => (reference, ""),
    };
    let joined = if reference.starts_with('/') {
        reference.to_owned()
    } else {
        let dir = &path[..path.rfind('/').map_or(0, |i| i + 1)];
        format!("{dir}{reference}")
    };
    let mut out: Vec<&str> = Vec::new();
    let segments: Vec<&str> = joined.split('/').collect();
    for (i, seg) in segments.iter().enumerate() {
        match *seg {
            "." => {}
            ".." => {
                out.pop();
            }
            s if s.is_empty() && i > 0 && i + 1 < segments.len() => {}
            s => out.push(s),
        }
    }
    let mut path = out.join("/");
    if !path.starts_with('/') {
        path.insert(0, '/');
    }
    if matches!(segments.last(), Some(&".") | Some(&"..")) && !path.ends_with('/') {
        path.push('/');
    }
    format!("{scheme}://{host}{path}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(loader: &dyn Loader, data: &str, hint: &str) -> Result<Document, LoadError> {
        loader.load(
            &Source::Bytes {
                data: data.as_bytes().to_vec(),
                hint: hint.into(),
            },
            &LoadOptions::default(),
        )
    }

    fn kinds(d: &Document, kind: MarkerKind) -> Vec<String> {
        d.marker_index()
            .iter(kind, None)
            .map(|m| d.slice(m.range))
            .collect()
    }

    const PLAIN: &str = "From: Ada Example <ada@example.org>\r\nTo: bo@example.org, \"Cy Example\" <cy@example.org>\r\nSubject: Quiz on =?utf-8?q?Thursday?=\r\nDate: Mon, 28 Sep 2026 10:15:00 -0700\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nHi Bo,\r\n\r\nThe quiz moved.\r\nSee you there.\r\n\r\n> Is the quiz\r\n> still on?\r\n\r\n-- \r\nAda\r\n";

    #[test]
    fn a_plain_email_reads_headers_then_body() {
        let d = load(&EmlLoader, PLAIN, "eml").expect("loads");
        assert_eq!(
            d.text().to_string(),
            "Quiz on Thursday\n\nFrom: Ada Example (ada@example.org)\nTo: bo@example.org, Cy Example (cy@example.org)\nDate: Monday, September 28, 2026, 10:15, UTC minus 7\n\nHi Bo,\n\nThe quiz moved.\nSee you there.\n\nIs the quiz\nstill on?\n\n--\nAda"
        );
        assert_eq!(d.meta.title.as_deref(), Some("Quiz on Thursday"));
        assert_eq!(d.meta.author.as_deref(), Some("Ada Example"));
        assert_eq!(kinds(&d, MarkerKind::Quote), ["Is the quiz\nstill on?"]);
    }

    #[test]
    fn html_only_mail_and_attachments() {
        let src = "From: ada@example.org\r\nSubject: Notes\r\nContent-Type: multipart/mixed; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<p>See <b>the notes</b> <img src=\"cid:logo@x\"></p>\r\n--b\r\nContent-Type: image/png\r\nContent-ID: <logo@x>\r\nContent-Description: The course logo\r\nContent-Disposition: inline\r\nContent-Transfer-Encoding: base64\r\n\r\niVBORw0KGgo=\r\n--b\r\nContent-Type: application/pdf; name=\"notes.pdf\"\r\nContent-Disposition: attachment; filename=\"notes.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0xLjQK\r\n--b--\r\n";
        let d = load(&EmlLoader, src, "eml").expect("loads");
        let text = d.text().to_string();
        assert!(text.contains("See the notes The course logo"), "{text}");
        assert!(
            text.ends_with("Attachments\n\nnotes.pdf, 9 bytes"),
            "{text}"
        );
        assert_eq!(kinds(&d, MarkerKind::Bold), ["the notes"]);
    }

    #[test]
    fn not_a_message_is_refused() {
        assert!(matches!(
            load(&EmlLoader, "", "eml"),
            Err(LoadError::Parse(_))
        ));
    }

    #[test]
    fn web_archive_reads_the_root_page_with_resolved_references() {
        let src = "From: <Saved by a browser>\r\nSubject: Cells\r\nMIME-Version: 1.0\r\nContent-Type: multipart/related; type=\"text/html\"; boundary=\"B\"; start=\"<page@x>\"\r\n\r\n--B\r\nContent-Type: text/css\r\nContent-Location: https://example.org/biology/style.css\r\n\r\np{}\r\n--B\r\nContent-Type: text/html; charset=utf-8\r\nContent-ID: <page@x>\r\nContent-Location: https://example.org/biology/cells.html\r\nContent-Transfer-Encoding: quoted-printable\r\n\r\n<html><head><title>Cells</title></head><body><h1>Cells</h1><p>A <a href=3D\"../index.html\">home</a> link. <img src=3D\"cell.png\"><img src=3D\"cid:x\" alt=3D\"A cell\"></p><math><mi>x</mi></math></body></html>\r\n--B\r\nContent-Type: image/png\r\nContent-Location: https://example.org/biology/cell.png\r\nContent-Description: A drawing of a cell\r\nContent-Transfer-Encoding: base64\r\n\r\niVBORw0KGgo=\r\n--B--\r\n";
        let d = load(&MhtmlLoader, src, "mhtml").expect("loads");
        assert_eq!(d.meta.title.as_deref(), Some("Cells"));
        let text = d.text().to_string();
        assert!(
            text.starts_with("Cells\n\nA home link. A drawing of a cell A cell"),
            "{text}"
        );
        let links: Vec<Option<String>> = d
            .marker_index()
            .iter(MarkerKind::Link, None)
            .map(|m| m.reference.clone())
            .collect();
        assert_eq!(links, [Some("https://example.org/index.html".into())]);
        assert_eq!(kinds(&d, MarkerKind::Math).len(), 1);
    }

    #[test]
    fn an_archive_without_a_page_is_refused() {
        let src = "Content-Type: text/plain\r\n\r\nNothing.\r\n";
        assert!(matches!(
            load(&MhtmlLoader, src, "mht"),
            Err(LoadError::Parse(_))
        ));
    }

    #[test]
    fn deep_and_wide_messages_are_refused() {
        let mut deep = String::from("Subject: x\r\n");
        for i in 0..40 {
            deep.push_str(&format!(
                "Content-Type: multipart/mixed; boundary=\"b{i}\"\r\n\r\n--b{i}\r\n"
            ));
        }
        deep.push_str("Content-Type: text/plain\r\n\r\nx\r\n");
        let err = load(&EmlLoader, &deep, "eml").expect_err("too deep");
        assert!(err.to_string().contains("nested"), "{err}");
        let mut wide =
            String::from("Subject: x\r\nContent-Type: multipart/mixed; boundary=\"b\"\r\n\r\n");
        for _ in 0..(MAX_PARTS + 5) {
            wide.push_str("--b\r\nContent-Type: text/plain\r\n\r\nx\r\n");
        }
        wide.push_str("--b--\r\n");
        let err = load(&EmlLoader, &wide, "eml").expect_err("too many parts");
        assert!(err.to_string().contains("parts"), "{err}");
    }

    #[test]
    fn dates_sizes_and_joins() {
        assert_eq!(weekday(2026, 9, 28), "Monday");
        assert_eq!(weekday(2026, 9, 24), "Thursday");
        assert_eq!(weekday(2000, 2, 29), "Tuesday");
        assert_eq!(weekday(1970, 1, 1), "Thursday");
        assert_eq!(size(9), "9 bytes");
        assert_eq!(size(240 * 1024), "240 KB");
        assert_eq!(size(3 * 1024 * 1024 + 100_000), "3.1 MB");
        let base = "https://example.org/a/b/page.html?x=1";
        assert_eq!(join(base, "c.png"), "https://example.org/a/b/c.png");
        assert_eq!(join(base, "../c.png"), "https://example.org/a/c.png");
        assert_eq!(
            join(base, "/root.html#top"),
            "https://example.org/root.html#top"
        );
        assert_eq!(
            join(base, "//cdn.example.org/x"),
            "https://cdn.example.org/x"
        );
        assert_eq!(
            join(base, "mailto:ada@example.org"),
            "mailto:ada@example.org"
        );
        assert_eq!(join("not a url", "x.png"), "x.png");
    }
}
