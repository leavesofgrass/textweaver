//! JSON, JSON Lines, and Jupyter notebooks (ADR-0044).
//!
//! **JSON** reads as a document: every top-level key is a heading, so `h`
//! moves by key ("name: Ada Example", or "address, object, 3 keys" before
//! the members); deeper objects and arrays are headings one level down
//! ("items, array, 12 entries", then "item 1, object, 2 keys"), to level
//! six, and plain values are list items ("city: Portland"). No bracket,
//! brace, comma, or quote is read. A file that is not valid JSON is read as
//! plain text, and the document says where the JSON broke.
//!
//! **JSON Lines** (`.jsonl`, `.ndjson`): each line is a heading ("line 3,
//! object, 4 keys") with its value below it; a line that is not JSON is
//! read as its text, and says so.
//!
//! **Jupyter notebooks** (`.ipynb`): markdown cells through the Markdown
//! loader, code cells as code under a line naming their language ("Python
//! code"), text outputs as block quotes after an "Output" line, error
//! outputs by their name and message, pictures as graphics ("Output
//! picture, PNG"), and raw cells as text.
//!
//! The JSON parser is our own, so object keys keep the file's order and a
//! hostile file costs bounded work: nesting past [`MAX_DEPTH`] and files
//! past [`MAX_BYTES`] are refused, and at most [`MAX_NODES`] values are
//! read.

use ropey::Rope;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, DocumentMeta, Marker};

use crate::builder::Builder;
use crate::{
    FootnoteMode, LoadError, LoadOptions, Loader, Source, add_warning, decode_source, meta_for,
    note_encoding, title_from_path,
};

/// Largest JSON, JSON Lines, or notebook file read.
pub const MAX_BYTES: usize = 64 << 20;

/// Deepest nesting of arrays and objects read.
pub const MAX_DEPTH: usize = crate::MAX_NESTING;

/// Most values read from one file; the rest is left out, and the document
/// says so.
pub const MAX_NODES: usize = 2_000_000;

/// The warning a document carries when [`MAX_NODES`] cut it short.
pub const CUT_WARNING: &str =
    "This file holds too many values to read in full, so the rest of it was left out.";

/// A JSON value, with object keys in the file's order.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number, as written.
    Number(String),
    /// A string, unescaped.
    String(String),
    /// An array.
    Array(Vec<Json>),
    /// An object: keys and values in order.
    Object(Vec<(String, Json)>),
}

impl Json {
    /// The member `key` of an object.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(m) => m.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The string, if it is one.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(s) => Some(s),
            _ => None,
        }
    }

    /// A string, or an array of strings joined (how notebooks store
    /// multi-line text).
    fn text(&self) -> Option<String> {
        match self {
            Json::String(s) => Some(s.clone()),
            Json::Array(a) => Some(a.iter().filter_map(Json::as_str).collect()),
            _ => None,
        }
    }
}

/// Why a JSON text could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError {
    /// What was wrong, in words.
    pub what: &'static str,
    /// The line, from 1.
    pub line: usize,
    /// The column, in characters from 1.
    pub column: usize,
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at line {}, column {}",
            self.what, self.line, self.column
        )
    }
}

/// Parses one JSON value (white space around it allowed).
pub fn parse(text: &str) -> Result<Json, JsonError> {
    let mut p = JsonParser {
        s: text,
        i: 0,
        depth: 0,
        nodes: 0,
    };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.i < text.len() {
        return Err(p.error("more text after the value"));
    }
    Ok(v)
}

struct JsonParser<'s> {
    s: &'s str,
    i: usize,
    depth: usize,
    nodes: usize,
}

impl JsonParser<'_> {
    fn error(&self, what: &'static str) -> JsonError {
        let before = &self.s[..self.i.min(self.s.len())];
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
        JsonError { what, line, column }
    }

    fn peek(&self) -> Option<u8> {
        self.s.as_bytes().get(self.i).copied()
    }

    fn ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    fn eat(&mut self, lit: &str) -> bool {
        if self.s[self.i..].starts_with(lit) {
            self.i += lit.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Result<Json, JsonError> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err(self.error("too many values"));
        }
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(Json::String),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(b't') if self.eat("true") => Ok(Json::Bool(true)),
            Some(b'f') if self.eat("false") => Ok(Json::Bool(false)),
            Some(b'n') if self.eat("null") => Ok(Json::Null),
            None => Err(self.error("the text ends where a value should be")),
            _ => Err(self.error("a value was expected")),
        }
    }

    fn enter(&mut self) -> Result<(), JsonError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(self.error("nested too deeply"));
        }
        self.i += 1;
        self.ws();
        Ok(())
    }

    fn object(&mut self) -> Result<Json, JsonError> {
        self.enter()?;
        let mut members = Vec::new();
        if self.peek() == Some(b'}') {
            self.i += 1;
            self.depth -= 1;
            return Ok(Json::Object(members));
        }
        loop {
            if self.peek() != Some(b'"') {
                return Err(self.error("a key in quotes was expected"));
            }
            let key = self.string()?;
            self.ws();
            if self.peek() != Some(b':') {
                return Err(self.error("a colon was expected after the key"));
            }
            self.i += 1;
            self.ws();
            let v = self.value()?;
            members.push((key, v));
            self.ws();
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                    self.ws();
                }
                Some(b'}') => {
                    self.i += 1;
                    self.depth -= 1;
                    return Ok(Json::Object(members));
                }
                _ => return Err(self.error("a comma or closing brace was expected")),
            }
        }
    }

    fn array(&mut self) -> Result<Json, JsonError> {
        self.enter()?;
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.i += 1;
            self.depth -= 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                    self.ws();
                }
                Some(b']') => {
                    self.i += 1;
                    self.depth -= 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err(self.error("a comma or closing bracket was expected")),
            }
        }
    }

    fn number(&mut self) -> Result<Json, JsonError> {
        let start = self.i;
        while matches!(
            self.peek(),
            Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
        ) {
            self.i += 1;
        }
        let n = &self.s[start..self.i];
        if n.parse::<f64>().is_err() {
            return Err(self.error("a number was expected"));
        }
        Ok(Json::Number(n.to_owned()))
    }

    fn hex4(&mut self) -> Option<u32> {
        let h = self.s.get(self.i..self.i + 4)?;
        let v = u32::from_str_radix(h, 16).ok()?;
        self.i += 4;
        Some(v)
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.i += 1;
        let mut out = String::new();
        loop {
            let rest = &self.s[self.i..];
            let run = rest
                .find(|c: char| c == '"' || c == '\\' || c < ' ')
                .ok_or_else(|| self.error("a string has no closing quote"))?;
            out.push_str(&rest[..run]);
            self.i += run;
            match self.peek() {
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.i += 1;
                    let Some(c) = self.peek() else {
                        return Err(self.error("a string has no closing quote"));
                    };
                    self.i += 1;
                    match c {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' | b'f' => {}
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.hex4().ok_or_else(|| self.error("a bad \\u escape"))?;
                            let c = if (0xd800..0xdc00).contains(&hi) && self.eat("\\u") {
                                let lo =
                                    self.hex4().ok_or_else(|| self.error("a bad \\u escape"))?;
                                char::from_u32(
                                    0x10000
                                        + ((hi - 0xd800) << 10)
                                        + (lo.wrapping_sub(0xdc00) & 0x3ff),
                                )
                            } else {
                                char::from_u32(hi)
                            };
                            out.push(c.unwrap_or('\u{fffd}'));
                        }
                        _ => return Err(self.error("a bad escape in a string")),
                    }
                }
                // A raw control character (a tab or line break inside a
                // string): kept, as most readers do.
                Some(_) => {
                    let c = rest[run..].chars().next().unwrap_or(' ');
                    out.push(c);
                    self.i += c.len_utf8();
                }
                None => return Err(self.error("a string has no closing quote")),
            }
        }
    }
}

// ---------------------------------------------------------------------
// Reading

fn marker(kind: MarkerKind) -> Marker {
    Marker::new(kind, CharRange::empty(0))
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// What a container is, in words: "object, 3 keys", "empty array".
fn describe(v: &Json) -> Option<String> {
    match v {
        Json::Object(m) if m.is_empty() => Some("empty object".into()),
        Json::Object(m) => Some(format!("object, {}", plural(m.len(), "key", "keys"))),
        Json::Array(a) if a.is_empty() => Some("empty array".into()),
        Json::Array(a) => Some(format!("array, {}", plural(a.len(), "entry", "entries"))),
        _ => None,
    }
}

/// A plain value in words: strings as they are, "empty text" for `""`.
fn scalar(v: &Json) -> String {
    match v {
        Json::Null => "null".into(),
        Json::Bool(b) => b.to_string(),
        Json::Number(n) => n.clone(),
        Json::String(s) if s.trim().is_empty() => "empty text".into(),
        Json::String(s) => s.clone(),
        Json::Array(_) | Json::Object(_) => String::new(),
    }
}

fn key_name(k: &str) -> &str {
    if k.trim().is_empty() { "(no name)" } else { k }
}

/// Writes JSON values into a builder.
struct Reader<'b> {
    b: &'b mut Builder,
    /// An open list of plain values, and its level.
    list: Option<crate::builder::OpenId>,
}

impl Reader<'_> {
    fn end_list(&mut self) {
        if let Some(id) = self.list.take() {
            self.b.close(id);
            self.b.paragraph_break();
        }
    }

    fn item(&mut self, text: &str, level: u8) {
        if self.list.is_none() {
            self.b.paragraph_break();
            self.list = Some(self.b.open(marker(MarkerKind::List).with_level(level)));
        }
        self.b.line_break();
        let id = self.b.open(marker(MarkerKind::ListItem).with_level(level));
        self.b.text(text);
        self.b.close(id);
    }

    /// A heading at `level` (1 to 6); deeper, a line of its own.
    fn heading(&mut self, text: &str, level: usize) {
        self.end_list();
        self.b.paragraph_break();
        if let Ok(l @ 1..=6) = u8::try_from(level) {
            let id = self.b.open(marker(MarkerKind::Heading).with_level(l));
            self.b.text(text);
            self.b.close(id);
        } else {
            self.b.text(text);
        }
        self.b.paragraph_break();
    }

    /// A named value: `name` is a key, "item 3", or "line 2".
    fn named(&mut self, name: &str, v: &Json, level: usize, heading_scalars: bool) {
        match describe(v) {
            Some(what) => {
                self.heading(&format!("{name}, {what}"), level);
                self.body(v, level + 1);
            }
            None if heading_scalars => self.heading(&format!("{name}: {}", scalar(v)), level),
            None => {
                let depth = u8::try_from(level.saturating_sub(1).max(1)).unwrap_or(u8::MAX);
                self.item(&format!("{name}: {}", scalar(v)), depth);
            }
        }
    }

    /// The members of an object or the entries of an array.
    fn body(&mut self, v: &Json, level: usize) {
        let depth = u8::try_from(level.saturating_sub(1).max(1)).unwrap_or(u8::MAX);
        match v {
            Json::Object(m) => {
                for (k, v) in m {
                    self.named(key_name(k), v, level, false);
                }
            }
            Json::Array(a) => {
                let n = a.len();
                for (i, v) in a.iter().enumerate() {
                    match describe(v) {
                        Some(what) => {
                            self.heading(&format!("item {} of {n}, {what}", i + 1), level);
                            self.body(v, level + 1);
                        }
                        None => self.item(&scalar(v), depth),
                    }
                }
            }
            other => self.item(&scalar(other), depth),
        }
        self.end_list();
    }

    /// A whole JSON document: top-level keys as headings.
    fn document(&mut self, v: &Json) {
        match v {
            Json::Object(m) => {
                for (k, v) in m {
                    self.named(key_name(k), v, 1, true);
                }
            }
            Json::Array(a) => {
                if a.is_empty() {
                    self.b.text("An empty array");
                } else {
                    self.b.text(&format!(
                        "An array of {}",
                        plural(a.len(), "entry", "entries")
                    ));
                }
                self.b.paragraph_break();
                self.body(v, 1);
            }
            other => self.b.text(&scalar(other)),
        }
        self.end_list();
    }
}

/// Canonical text and markers for a JSON value.
pub fn convert(v: &Json) -> (String, Vec<Marker>) {
    let mut b = Builder::new();
    Reader {
        b: &mut b,
        list: None,
    }
    .document(v);
    b.finish()
}

/// Reads the source as text, refusing one past [`MAX_BYTES`].
fn text_of(source: &Source) -> Result<crate::encoding::Decoded, LoadError> {
    let head = source.read_head(MAX_BYTES + 1)?;
    if head.len() > MAX_BYTES {
        return Err(LoadError::Parse(format!(
            "the file is larger than {} megabytes",
            MAX_BYTES >> 20
        )));
    }
    decode_source(source, None)
}

/// Loads JSON (`.json`) and JSON Lines (`.jsonl`, `.ndjson`).
#[derive(Clone, Copy, Debug, Default)]
pub struct JsonLoader;

impl Loader for JsonLoader {
    fn id(&self) -> &'static str {
        "json"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["json", "jsonl", "ndjson", "geojson", "webmanifest"]
    }

    /// Data files, not documents: a folder scan leaves them out; they open
    /// by name.
    fn scan_extensions(&self) -> &'static [&'static str] {
        &[]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, _options: &LoadOptions) -> Result<Document, LoadError> {
        let decoded = text_of(source)?;
        let mut meta = meta_for(source, self.id());
        note_encoding(&mut meta, &decoded);
        meta.title = title_from_path(source);
        let lines = matches!(source.hint().as_deref(), Some("jsonl" | "ndjson"));
        let (text, markers) = if lines {
            json_lines(&decoded.text, &mut meta)
        } else {
            match parse(&decoded.text) {
                Ok(v) => convert(&v),
                Err(e) if e.what == "too many values" => {
                    add_warning(&mut meta, CUT_WARNING);
                    (decoded.text.clone(), Vec::new())
                }
                Err(e) => {
                    add_warning(
                        &mut meta,
                        &format!("This file is not valid JSON ({e}), so it is read as plain text."),
                    );
                    let mut doc = Document::from_plain_text(&decoded.text);
                    doc.meta = meta;
                    return Ok(doc);
                }
            }
        };
        Ok(Document::new(meta, Rope::from_str(&text), markers))
    }
}

/// JSON Lines: a heading per line.
fn json_lines(text: &str, meta: &mut DocumentMeta) -> (String, Vec<Marker>) {
    let mut b = Builder::new();
    let mut nodes = 0usize;
    let mut bad = 0usize;
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let name = format!("line {}", n + 1);
        let mut r = Reader {
            b: &mut b,
            list: None,
        };
        match parse(line) {
            Ok(v) => {
                nodes += count(&v);
                if nodes > MAX_NODES {
                    add_warning(meta, CUT_WARNING);
                    break;
                }
                r.named(&name, &v, 1, true);
                r.end_list();
            }
            Err(e) => {
                bad += 1;
                r.heading(&format!("{name}, not valid JSON ({e})"), 1);
                b.text(line);
            }
        }
    }
    if bad > 0 {
        add_warning(
            meta,
            &format!(
                "{} of this file's lines {} not valid JSON; {} read as plain text.",
                bad,
                if bad == 1 { "is" } else { "are" },
                if bad == 1 { "it is" } else { "they are" }
            ),
        );
    }
    b.finish()
}

fn count(v: &Json) -> usize {
    let mut n = 0;
    let mut stack = vec![v];
    while let Some(v) = stack.pop() {
        n += 1;
        match v {
            Json::Array(a) => stack.extend(a),
            Json::Object(m) => stack.extend(m.iter().map(|(_, v)| v)),
            _ => {}
        }
    }
    n
}

// ---------------------------------------------------------------------
// Notebooks

/// Loads Jupyter notebooks (`.ipynb`, nbformat 4, and the cells of 3).
#[derive(Clone, Copy, Debug, Default)]
pub struct NotebookLoader;

impl Loader for NotebookLoader {
    fn id(&self) -> &'static str {
        "notebook"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ipynb"]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let decoded = text_of(source)?;
        let nb =
            parse(&decoded.text).map_err(|e| LoadError::Parse(format!("not a notebook: {e}")))?;
        let cells = nb
            .get("cells")
            .or_else(|| {
                // nbformat 3: cells inside the first worksheet.
                match nb.get("worksheets") {
                    Some(Json::Array(w)) => w.first().and_then(|w| w.get("cells")),
                    _ => None,
                }
            })
            .ok_or_else(|| LoadError::Parse("not a notebook: it has no cells".into()))?;
        let Json::Array(cells) = cells else {
            return Err(LoadError::Parse(
                "not a notebook: its cells are not a list".into(),
            ));
        };
        let mut meta = meta_for(source, self.id());
        note_encoding(&mut meta, &decoded);
        let metadata = nb.get("metadata");
        let language = metadata
            .and_then(|m| m.get("language_info"))
            .and_then(|l| l.get("name"))
            .or_else(|| {
                metadata
                    .and_then(|m| m.get("kernelspec"))
                    .and_then(|k| k.get("language"))
            })
            .and_then(Json::as_str)
            .unwrap_or("")
            .to_owned();
        if let Some(t) = metadata.and_then(|m| m.get("title")).and_then(Json::as_str) {
            meta.title = Some(t.to_owned());
        }
        let cell_options = LoadOptions {
            footnotes: match options.footnotes {
                FootnoteMode::Deferred => FootnoteMode::Inline,
                other => other,
            },
            ..options.clone()
        };
        let mut b = Builder::new();
        for cell in cells {
            let kind = cell.get("cell_type").and_then(Json::as_str).unwrap_or("");
            let text = cell
                .get("source")
                .or_else(|| cell.get("input"))
                .and_then(Json::text)
                .unwrap_or_default();
            match kind {
                "markdown" | "heading" => {
                    b.paragraph_break();
                    b = crate::markdown::convert_into(b, &text, &cell_options, &mut meta);
                    b.paragraph_break();
                }
                "code" => {
                    let lang = cell
                        .get("metadata")
                        .and_then(|m| m.get("language"))
                        .and_then(Json::as_str)
                        .unwrap_or(&language);
                    code_cell(&mut b, &text, lang, options);
                    outputs(&mut b, cell.get("outputs"));
                }
                _ => {
                    b.paragraph_break();
                    b.text(&text);
                    b.paragraph_break();
                }
            }
        }
        if meta.title.is_none() {
            meta.title = title_from_path(source);
        }
        if !language.is_empty() {
            meta.properties
                .insert("notebook.language".into(), language.clone());
        }
        let (text, markers) = b.finish();
        Ok(Document::new(meta, Rope::from_str(&text), markers))
    }
}

/// A language's name to say: `python` is "Python", `r` is "R".
fn language_word(lang: &str) -> String {
    match lang.to_ascii_lowercase().as_str() {
        "" => "Code".into(),
        "r" => "R".into(),
        "c++" | "cpp" => "C++".into(),
        "c#" | "csharp" => "C sharp".into(),
        "javascript" | "js" => "JavaScript".into(),
        "typescript" => "TypeScript".into(),
        "sql" => "SQL".into(),
        _ => {
            let mut c = lang.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        }
    }
}

fn code_cell(b: &mut Builder, text: &str, lang: &str, options: &LoadOptions) {
    b.paragraph_break();
    let word = language_word(lang);
    if word == "Code" {
        b.text("Code");
    } else {
        b.text(&format!("{word} code"));
    }
    if options.skip_code || text.trim().is_empty() {
        b.paragraph_break();
        return;
    }
    b.paragraph_break();
    let mut m = marker(MarkerKind::Code).with_level(1);
    if !lang.is_empty() {
        m = m.with_label(lang.to_ascii_lowercase());
    }
    let id = b.open(m);
    b.verbatim(text.trim_end_matches('\n'));
    b.close(id);
    b.paragraph_break();
}

/// Terminal color codes (`ESC [ ... m`), which tracebacks are full of.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for d in chars.by_ref() {
                    if d.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

fn quote(b: &mut Builder, text: &str) {
    let text = strip_ansi(text);
    if text.trim().is_empty() {
        return;
    }
    let id = b.open(marker(MarkerKind::Quote).with_label("output"));
    for (i, line) in text.trim_end().lines().enumerate() {
        if i > 0 {
            b.line_break();
        }
        b.text(line);
    }
    b.close(id);
    b.paragraph_break();
}

const PICTURES: &[(&str, &str)] = &[
    ("image/png", "PNG"),
    ("image/jpeg", "JPEG"),
    ("image/gif", "GIF"),
    ("image/svg+xml", "SVG"),
];

fn outputs(b: &mut Builder, outputs: Option<&Json>) {
    let Some(Json::Array(outputs)) = outputs else {
        return;
    };
    let mut said = false;
    let mut heading = |b: &mut Builder| {
        if !said {
            b.text("Output");
            b.paragraph_break();
            said = true;
        }
    };
    for out in outputs {
        let kind = out.get("output_type").and_then(Json::as_str).unwrap_or("");
        match kind {
            "stream" => {
                if let Some(t) = out.get("text").and_then(Json::text) {
                    heading(b);
                    quote(b, &t);
                }
            }
            "error" | "pyerr" => {
                let name = out.get("ename").and_then(Json::as_str).unwrap_or("Error");
                let value = out.get("evalue").and_then(Json::as_str).unwrap_or("");
                heading(b);
                quote(b, &format!("Error: {name}: {value}"));
            }
            _ => {
                let data = out.get("data").unwrap_or(out);
                let picture = PICTURES
                    .iter()
                    .find(|(mime, _)| data.get(mime).is_some() || data.get(&mime[6..]).is_some());
                if let Some((mime, name)) = picture {
                    heading(b);
                    let alt = data
                        .get("text/plain")
                        .and_then(Json::text)
                        .map(|t| strip_ansi(&t))
                        .filter(|t| !t.trim().is_empty() && !t.starts_with('<'));
                    let words = match alt {
                        Some(a) => format!("Output picture, {name}: {}", a.trim()),
                        None => format!("Output picture, {name}"),
                    };
                    let id = b.open(marker(MarkerKind::Image).with_reference(*mime));
                    b.text(&words);
                    b.close(id);
                    b.paragraph_break();
                } else if let Some(t) = data
                    .get("text/plain")
                    .or_else(|| data.get("text"))
                    .and_then(Json::text)
                {
                    heading(b);
                    quote(b, &t);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(src: &str, hint: &str) -> Document {
        let source = Source::Bytes {
            data: src.as_bytes().to_vec(),
            hint: hint.into(),
        };
        match hint {
            "ipynb" => NotebookLoader.load(&source, &LoadOptions::default()),
            _ => JsonLoader.load(&source, &LoadOptions::default()),
        }
        .unwrap()
    }

    fn kinds(d: &Document, kind: MarkerKind) -> Vec<String> {
        d.marker_index()
            .iter(kind, None)
            .map(|m| d.slice(m.range))
            .collect()
    }

    #[test]
    fn parser_keeps_order_and_unescapes() {
        let v = parse(r#" {"b": [1, -2.5e3, true, null], "a": "x\"é😀\n"} "#).unwrap();
        let Json::Object(m) = &v else { panic!() };
        assert_eq!(m[0].0, "b");
        assert_eq!(m[1].1, Json::String("x\"\u{e9}\u{1f600}\n".into()));
        for bad in [
            "",
            "{",
            "[1,]",
            "{\"a\" 1}",
            "\"open",
            "01x",
            "[1] 2",
            "tru",
        ] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }
        let e = parse("{\n  \"a\": ?\n}").unwrap_err();
        assert_eq!((e.line, e.column), (2, 8));
        let deep = "[".repeat(MAX_DEPTH + 1) + &"]".repeat(MAX_DEPTH + 1);
        assert_eq!(parse(&deep).unwrap_err().what, "nested too deeply");
    }

    #[test]
    fn json_reads_keys_as_headings_without_punctuation() {
        let d = load(
            r#"{"name": "Ada Example", "age": 36, "address": {"city": "Portland", "zip": "97201", "geo": {"lat": 45.5}}, "tags": ["a", "b"], "jobs": [{"title": "Reader"}], "none": {}, "": null}"#,
            "json",
        );
        assert_eq!(
            kinds(&d, MarkerKind::Heading),
            [
                "name: Ada Example",
                "age: 36",
                "address, object, 3 keys",
                "geo, object, 1 key",
                "tags, array, 2 entries",
                "jobs, array, 1 entry",
                "item 1 of 1, object, 1 key",
                "none, empty object",
                "(no name): null",
            ]
        );
        let levels: Vec<u8> = d
            .marker_index()
            .iter(MarkerKind::Heading, None)
            .map(|m| m.level)
            .collect();
        assert_eq!(levels, [1, 1, 1, 2, 1, 1, 2, 1, 1]);
        assert_eq!(
            kinds(&d, MarkerKind::ListItem),
            [
                "city: Portland",
                "zip: 97201",
                "lat: 45.5",
                "a",
                "b",
                "title: Reader"
            ]
        );
        let text = d.text().to_string();
        for noise in ['{', '}', '[', ']', '"'] {
            assert!(!text.contains(noise), "{noise} in {text}");
        }
    }

    #[test]
    fn invalid_json_is_text_and_json_lines_are_headings() {
        let d = load("{\"a\": 1,,}", "json");
        assert_eq!(d.text().to_string(), "{\"a\": 1,,}");
        let w = crate::warnings(&d.meta);
        assert!(
            w[0].contains("not valid JSON (a key in quotes was expected at line 1, column 9)"),
            "{w:?}"
        );
        let l = load(
            "{\"id\": 1, \"ok\": true}\n\n[1, 2]\nnot json\n\"text\"\n",
            "jsonl",
        );
        assert_eq!(
            kinds(&l, MarkerKind::Heading),
            [
                "line 1, object, 2 keys",
                "line 3, array, 2 entries",
                "line 4, not valid JSON (a value was expected at line 1, column 1)",
                "line 5: text",
            ]
        );
        assert!(l.text().to_string().contains("not json"));
        assert_eq!(crate::warnings(&l.meta).len(), 1);
    }

    #[test]
    fn notebooks_read_cells_by_kind() {
        let nb = r##"{
 "metadata": {"kernelspec": {"language": "python", "name": "python3"}},
 "nbformat": 4,
 "cells": [
  {"cell_type": "markdown", "source": ["# Growth\n", "Some *notes* with $x^2$."]},
  {"cell_type": "code", "source": "print('hi')\n", "outputs": [
     {"output_type": "stream", "name": "stdout", "text": ["hi\n"]},
     {"output_type": "display_data", "data": {"image/png": "iVBOR", "text/plain": ["<Figure size 640x480>"]}},
     {"output_type": "execute_result", "data": {"text/plain": "42"}},
     {"output_type": "error", "ename": "ValueError", "evalue": "bad", "traceback": ["\u001b[31mTrace"]}
  ]},
  {"cell_type": "code", "metadata": {"language": "r"}, "source": [], "outputs": []},
  {"cell_type": "raw", "source": "Raw words."}
 ]
}"##;
        let d = load(nb, "ipynb");
        let text = d.text().to_string();
        assert_eq!(d.meta.title.as_deref(), Some("Growth"));
        assert_eq!(kinds(&d, MarkerKind::Heading), ["Growth"]);
        assert_eq!(kinds(&d, MarkerKind::Math), ["$x^2$"]);
        assert!(
            text.contains("Python code\n\nprint('hi')\n\nOutput\n\nhi"),
            "{text}"
        );
        assert!(text.contains("Output picture, PNG"), "{text}");
        assert!(text.contains("42"), "{text}");
        assert!(text.contains("Error: ValueError: bad"), "{text}");
        assert!(!text.contains("Trace"), "{text}");
        assert!(text.contains("R code\n\nRaw words."), "{text}");
        let code: Vec<Option<String>> = d
            .marker_index()
            .iter(MarkerKind::Code, Some(1))
            .map(|m| m.label.clone())
            .collect();
        assert_eq!(code, [Some("python".into())]);
        assert_eq!(kinds(&d, MarkerKind::Image), ["Output picture, PNG"]);
        let bad = NotebookLoader.load(
            &Source::Bytes {
                data: b"{\"cells\": 3}".to_vec(),
                hint: "ipynb".into(),
            },
            &LoadOptions::default(),
        );
        assert!(matches!(bad, Err(LoadError::Parse(_))));
    }
}
