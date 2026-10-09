//! RTF loader: Rich Text Format read natively, without Pandoc.
//!
//! The parser is iterative, with an explicit group stack, so no nesting
//! can overflow the thread's stack, and it never allocates more than the
//! input's size in text:
//!
//! - **Groups** nest at most [`MAX_NESTING`](crate::MAX_NESTING) deep;
//!   groups past that are read as plain text in the enclosing group's
//!   state, and the document carries
//!   [`NESTING_WARNING`](crate::NESTING_WARNING).
//! - **`\bin`** data is skipped, never read into memory, and never past
//!   the end of the input, whatever length it claims.
//! - **`\uc`** skip counts are capped at [`MAX_UNICODE_SKIP`].
//! - **Text encoding**: `\ansicpg`, `\mac`, and each font's `\fcharset` or
//!   `\cpg` pick the code page for `\'hh` bytes and raw 8-bit text,
//!   decoded through encoding_rs (double-byte code pages such as
//!   Shift_JIS and GBK included); `\u` characters (with surrogate pairs)
//!   and their `\uc` fallbacks are handled.
//! - **Destinations**: `\*` destinations textweaver does not know are
//!   skipped, as are headers, footers, color tables, list tables, and
//!   picture data. The font table, style sheet, `\info` title and author,
//!   and the revision table (`\revtbl`) are read.
//! - **Structure**: headings from paragraph styles the style sheet names
//!   `heading 1` to `heading 9` or `Title`, or from `\outlinelevel`;
//!   lists from `\ls` and `\ilvl`, labeled with the `\listtext` or
//!   `\pntext` the writer rendered; tables from `\trowd`, `\intbl`,
//!   `\cell`, and `\row` (a `\trhdr` row is the header); footnotes and
//!   endnotes (`\footnote`) as in the other loaders; `HYPERLINK` fields as
//!   links (other fields read as their result); bold, italic, and
//!   underline; hidden text (`\v`) left out; picture alt text
//!   (`wzDescription`) under an `Image` marker.
//! - **Tracked changes**: `\revised` and `\deleted` runs, with their author
//!   from `\revtbl`, follow [`LoadOptions::revisions`].

use std::collections::HashMap;

use encoding_rs::{Encoding, WINDOWS_1252};
use ropey::Rope;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, HEADER_ROW_LABEL, Marker};

use crate::builder::{Builder, OpenId};
use crate::revision::{self, ChangeKind};
use crate::{
    FootnoteMode, LoadError, LoadOptions, Loader, RevisionMode, Source, meta_for, title_from_path,
};

/// Largest `\uc` skip count honored; a larger one is read as this.
pub const MAX_UNICODE_SKIP: usize = 8;

/// Longest control word read (the RTF specification's limit); longer runs
/// of letters are cut, so a hostile word cannot grow a buffer.
const MAX_WORD: usize = 32;

/// Loads Rich Text Format files (`.rtf`).
#[derive(Clone, Copy, Debug, Default)]
pub struct RtfLoader;

impl Loader for RtfLoader {
    fn id(&self) -> &'static str {
        "rtf"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["rtf"]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let bytes = source.read()?;
        let start = bytes
            .iter()
            .position(|b| !b.is_ascii_whitespace())
            .unwrap_or(bytes.len());
        let body = bytes.get(start..).unwrap_or_default();
        let body = body.strip_prefix(b"\xef\xbb\xbf").unwrap_or(body);
        if !body.starts_with(b"{\\rtf") {
            return Err(LoadError::Parse(
                "not an RTF file: it does not start with {\\rtf".into(),
            ));
        }
        let parsed = parse(body);
        let mut meta = meta_for(source, self.id());
        meta.title = parsed.title.clone().filter(|t| !t.is_empty());
        meta.author = parsed.author.clone().filter(|a| !a.is_empty());
        let (text, markers, changes) = render(&parsed, options);
        if parsed.flattened {
            crate::add_warning(&mut meta, crate::NESTING_WARNING);
        }
        crate::annotations::record(&mut meta, Vec::new(), changes);
        if meta.title.is_none() {
            meta.title = markers
                .iter()
                .find(|m| m.kind == MarkerKind::Heading && m.level == 1)
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

/// Where text in a group goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dest {
    /// The document's text.
    Text,
    /// Dropped (unknown `\*` destinations, headers, picture data).
    Skip,
    FontTable,
    StyleSheet,
    Info,
    InfoTitle,
    InfoAuthor,
    RevTable,
    Footnote,
    FieldInst,
    ListText,
    /// Picture or shape: text dropped, but shape properties read.
    Pict,
    SpName,
    SpValue,
}

/// Character formatting and revision state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Fmt {
    bold: bool,
    italic: bool,
    underline: bool,
    hidden: bool,
    inserted: bool,
    deleted: bool,
    /// `\revauth`: index into the revision table.
    ins_author: i32,
    /// `\revauthdel`.
    del_author: i32,
    /// `\revdttm`: when the insertion was made, packed as Word's DTTM.
    ins_date: i32,
    /// `\revdttmdel`.
    del_date: i32,
}

/// An RTF revision time (`\revdttm`, Word's packed DTTM: minutes, hours,
/// day, month, and years since 1900 in bit fields) as an ISO 8601 date and
/// time, or `None` for zero or a value that is not a date.
fn dttm(v: i32) -> Option<String> {
    let v = v as u32;
    let min = v & 0x3f;
    let hour = (v >> 6) & 0x1f;
    let day = (v >> 11) & 0x1f;
    let month = (v >> 16) & 0xf;
    let year = 1900 + ((v >> 20) & 0x1ff);
    if v == 0 || !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || min > 59 {
        return None;
    }
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:00"
    ))
}

/// A change in an RTF run: its kind, author index, and packed time.
type ChangeKey = (ChangeKind, i32, i32);

#[derive(Clone, Debug)]
struct Group {
    dest: Dest,
    /// `\*` seen: the next control word must be a known destination, or
    /// the group is skipped.
    star: bool,
    uc: usize,
    font: Option<i32>,
    fmt: Fmt,
    style: i32,
    intbl: bool,
    list: Option<i32>,
    ilvl: u8,
    outline: Option<u8>,
    /// The field (index into `Parser::fields`) this group belongs to.
    field: Option<usize>,
    /// The link target runs in this group carry.
    link: Option<usize>,
}

impl Default for Group {
    fn default() -> Self {
        Group {
            dest: Dest::Text,
            star: false,
            uc: 1,
            font: None,
            fmt: Fmt::default(),
            style: 0,
            intbl: false,
            list: None,
            ilvl: 0,
            outline: None,
            field: None,
            link: None,
        }
    }
}

/// One piece of a paragraph.
#[derive(Clone, Debug, PartialEq)]
enum Run {
    Text {
        text: String,
        fmt: Fmt,
        link: Option<usize>,
    },
    Break,
    Note(String),
    Image(String),
}

#[derive(Clone, Debug, Default)]
struct Para {
    runs: Vec<Run>,
    heading: Option<u8>,
    /// List depth (from 1) and the rendered label.
    list: Option<(u8, Option<String>)>,
}

#[derive(Clone, Debug)]
enum Block {
    Para(Para),
    Row { cells: Vec<Vec<Run>>, header: bool },
}

/// What the parser found.
#[derive(Debug, Default)]
struct Parsed {
    blocks: Vec<Block>,
    links: Vec<String>,
    authors: Vec<String>,
    title: Option<String>,
    author: Option<String>,
    flattened: bool,
}

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
    stack: Vec<Group>,
    /// Groups opened past the nesting limit and not yet closed.
    overflow: usize,
    ansi: &'static Encoding,
    default_font: Option<i32>,
    fonts: HashMap<i32, &'static Encoding>,
    /// The font being defined in the font table.
    font_def: Option<i32>,
    styles: HashMap<i32, String>,
    style_def: Option<i32>,
    /// Text of the current style sheet, revision table, or font entry,
    /// up to its `;`.
    entry: String,
    /// Bytes waiting to be decoded with `pending_enc`.
    pending: Vec<u8>,
    pending_enc: &'static Encoding,
    /// A high surrogate from `\u`, waiting for its low half.
    high: Option<u16>,
    /// Fallback characters still to skip after `\u`.
    skip: usize,
    para: Vec<Run>,
    label: Option<String>,
    cells: Vec<Vec<Run>>,
    header_row: bool,
    footnote: String,
    list_text: String,
    fields: Vec<String>,
    sp_name: String,
    sp_value: String,
    image_alt: Option<String>,
    info_title: String,
    info_author: String,
    out: Parsed,
}

/// Parses RTF (starting at its `{\rtf`).
fn parse(src: &[u8]) -> Parsed {
    let mut p = Parser {
        src,
        pos: 0,
        stack: vec![Group::default()],
        overflow: 0,
        ansi: WINDOWS_1252,
        default_font: None,
        fonts: HashMap::new(),
        font_def: None,
        styles: HashMap::new(),
        style_def: None,
        entry: String::new(),
        pending: Vec::new(),
        pending_enc: WINDOWS_1252,
        high: None,
        skip: 0,
        para: Vec::new(),
        label: None,
        cells: Vec::new(),
        header_row: false,
        footnote: String::new(),
        list_text: String::new(),
        fields: Vec::new(),
        sp_name: String::new(),
        sp_value: String::new(),
        image_alt: None,
        info_title: String::new(),
        info_author: String::new(),
        out: Parsed::default(),
    };
    p.run();
    p.flush_bytes();
    p.end_paragraph();
    p.end_row();
    let title = p
        .info_title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let author = p
        .info_author
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    p.out.title = Some(title);
    p.out.author = Some(author);
    p.out
}

impl Parser<'_> {
    /// The current group. The stack always holds the outermost state:
    /// `run` never pops the last entry.
    fn group(&self) -> &Group {
        &self.stack[self.stack.len() - 1]
    }

    fn group_mut(&mut self) -> &mut Group {
        let n = self.stack.len() - 1;
        &mut self.stack[n]
    }

    fn run(&mut self) {
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            match b {
                b'{' => {
                    self.pos += 1;
                    self.flush_bytes();
                    self.skip = 0;
                    if self.stack.len() > crate::MAX_NESTING {
                        self.overflow += 1;
                        self.out.flattened = true;
                    } else {
                        let mut g = self.group().clone();
                        g.star = false;
                        self.stack.push(g);
                    }
                }
                b'}' => {
                    self.pos += 1;
                    self.flush_bytes();
                    self.skip = 0;
                    if self.overflow > 0 {
                        self.overflow -= 1;
                    } else if self.stack.len() > 1 {
                        self.close_group();
                        if self.stack.len() == 1 {
                            // The document's own group closed: the rest
                            // is not RTF.
                            return;
                        }
                    }
                }
                b'\\' => self.control(),
                b'\r' | b'\n' => self.pos += 1,
                b'\t' => {
                    self.pos += 1;
                    self.flush_bytes();
                    self.chars(" ");
                }
                _ => {
                    self.pos += 1;
                    if self.skip > 0 {
                        self.skip -= 1;
                    } else {
                        self.byte(b);
                    }
                }
            }
        }
    }

    /// A text byte, decoded later with the current font's code page.
    fn byte(&mut self, b: u8) {
        let enc = self.encoding();
        if enc != self.pending_enc {
            self.flush_bytes();
            self.pending_enc = enc;
        }
        self.pending.push(b);
    }

    fn encoding(&self) -> &'static Encoding {
        self.group()
            .font
            .or(self.default_font)
            .and_then(|f| self.fonts.get(&f).copied())
            .unwrap_or(self.ansi)
    }

    fn flush_bytes(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let bytes = std::mem::take(&mut self.pending);
        let (text, _) = self.pending_enc.decode_without_bom_handling(&bytes);
        let text = text.into_owned();
        self.chars(&text);
    }

    /// Text for the current destination.
    fn chars(&mut self, s: &str) {
        let g = self.group().clone();
        match g.dest {
            Dest::Text => {
                if g.fmt.hidden {
                    return;
                }
                let link = g.link;
                match self.para.last_mut() {
                    Some(Run::Text { text, fmt, link: l }) if *fmt == g.fmt && *l == link => {
                        text.push_str(s)
                    }
                    _ => self.para.push(Run::Text {
                        text: s.to_owned(),
                        fmt: g.fmt,
                        link,
                    }),
                }
            }
            Dest::Footnote => {
                if !g.fmt.hidden {
                    self.footnote.push_str(s);
                }
            }
            Dest::FontTable | Dest::StyleSheet | Dest::RevTable => {
                for c in s.chars() {
                    if c == ';' {
                        self.end_entry(g.dest);
                    } else {
                        self.entry.push(c);
                    }
                }
            }
            Dest::InfoTitle => self.info_title.push_str(s),
            Dest::InfoAuthor => self.info_author.push_str(s),
            Dest::FieldInst => {
                if let Some(f) = g.field.and_then(|i| self.fields.get_mut(i)) {
                    f.push_str(s);
                }
            }
            Dest::ListText => self.list_text.push_str(s),
            Dest::SpName => self.sp_name.push_str(s),
            Dest::SpValue => self.sp_value.push_str(s),
            Dest::Skip | Dest::Info | Dest::Pict => {}
        }
    }

    /// The end of a font table, style sheet, or revision table entry.
    fn end_entry(&mut self, dest: Dest) {
        let name = std::mem::take(&mut self.entry);
        let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
        match dest {
            Dest::StyleSheet => {
                if let Some(s) = self.style_def.take() {
                    self.styles.insert(s, name.to_lowercase());
                }
            }
            Dest::RevTable if self.out.authors.len() < 10_000 => self.out.authors.push(name),
            _ => {}
        }
    }

    fn close_group(&mut self) {
        let Some(g) = self.stack.pop() else {
            return;
        };
        let parent = self.group().dest;
        match g.dest {
            Dest::Footnote if parent != Dest::Footnote => {
                let text = std::mem::take(&mut self.footnote);
                let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
                if !text.is_empty() && parent == Dest::Text {
                    self.para.push(Run::Note(text));
                }
            }
            Dest::ListText if parent != Dest::ListText => {
                let t = std::mem::take(&mut self.list_text);
                let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
                if !t.is_empty() {
                    self.label = Some(crate::counter::cap_label(t));
                }
            }
            Dest::StyleSheet | Dest::RevTable if !self.entry.trim().is_empty() => {
                // An entry closed without its `;`.
                self.end_entry(g.dest);
            }
            Dest::SpValue if parent != Dest::SpValue => {
                if self.sp_name.trim() == "wzDescription" {
                    let alt = self
                        .sp_value
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !alt.is_empty() {
                        self.image_alt = Some(alt);
                    }
                }
            }
            Dest::Pict if parent != Dest::Pict => {
                if let Some(alt) = self.image_alt.take()
                    && parent == Dest::Text
                {
                    self.para.push(Run::Image(alt));
                }
            }
            _ => {}
        }
        if g.dest == Dest::FontTable && parent != Dest::FontTable {
            self.entry.clear();
            self.font_def = None;
        }
        if matches!(g.dest, Dest::SpName | Dest::SpValue) && parent == Dest::Pict {
            // The next property starts afresh.
            if g.dest == Dest::SpValue {
                self.sp_name.clear();
                self.sp_value.clear();
            }
        }
    }

    fn control(&mut self) {
        // At a backslash.
        self.pos += 1;
        let Some(&c) = self.src.get(self.pos) else {
            return;
        };
        if !c.is_ascii_alphabetic() {
            self.pos += 1;
            self.symbol(c);
            return;
        }
        let start = self.pos;
        while self.pos < self.src.len() && self.src[self.pos].is_ascii_alphabetic() {
            self.pos += 1;
        }
        let word_end = self.pos.min(start + MAX_WORD);
        let word = std::str::from_utf8(&self.src[start..word_end]).unwrap_or("");
        let word = word.to_owned();
        let mut param: Option<i32> = None;
        let neg = self.src.get(self.pos) == Some(&b'-')
            && self.src.get(self.pos + 1).is_some_and(u8::is_ascii_digit);
        if neg {
            self.pos += 1;
        }
        let mut digits = 0;
        let mut n: i64 = 0;
        while self.pos < self.src.len() && self.src[self.pos].is_ascii_digit() {
            if digits < 10 {
                n = n * 10 + i64::from(self.src[self.pos] - b'0');
            }
            digits += 1;
            self.pos += 1;
        }
        if digits > 0 {
            let n = if neg { -n } else { n };
            param =
                Some(i32::try_from(n.clamp(i64::from(i32::MIN), i64::from(i32::MAX))).unwrap_or(0));
        }
        if self.src.get(self.pos) == Some(&b' ') {
            self.pos += 1;
        }
        if word == "bin" {
            // Binary data: skipped, never past the end.
            self.flush_bytes();
            let n = usize::try_from(param.unwrap_or(0).max(0)).unwrap_or(0);
            self.pos = self.pos.saturating_add(n).min(self.src.len());
            return;
        }
        if word == "u" {
            self.flush_bytes();
            self.unicode(param.unwrap_or(0));
            return;
        }
        if self.skip > 0 {
            self.skip -= 1;
            return;
        }
        self.flush_bytes();
        self.word(&word, param);
    }

    fn unicode(&mut self, n: i32) {
        let v = if n < 0 { n + 65536 } else { n };
        let v = u32::try_from(v).unwrap_or(0xFFFD);
        self.skip = self.group().uc;
        if (0xD800..0xDC00).contains(&v) {
            self.high = u16::try_from(v).ok();
            return;
        }
        let c = if (0xDC00..0xE000).contains(&v) {
            match self.high.take() {
                Some(h) => {
                    let code = 0x10000 + ((u32::from(h) - 0xD800) << 10) + (v - 0xDC00);
                    char::from_u32(code)
                }
                None => None,
            }
        } else {
            self.high = None;
            char::from_u32(v)
        };
        if let Some(c) = c.filter(|c| !c.is_control() || *c == '\t') {
            let mut buf = [0u8; 4];
            self.chars(c.encode_utf8(&mut buf));
        }
    }

    fn symbol(&mut self, c: u8) {
        if c == b'\'' {
            let hex = self
                .src
                .get(self.pos..self.pos + 2)
                .and_then(|h| std::str::from_utf8(h).ok())
                .and_then(|h| u8::from_str_radix(h, 16).ok());
            if let Some(b) = hex {
                self.pos += 2;
                if self.skip > 0 {
                    self.skip -= 1;
                } else {
                    self.byte(b);
                }
            }
            return;
        }
        if self.skip > 0 {
            self.skip -= 1;
            return;
        }
        match c {
            b'\\' | b'{' | b'}' => self.byte(c),
            b'*' => self.group_mut().star = true,
            b'~' => {
                self.flush_bytes();
                self.chars("\u{a0}");
            }
            b'_' => {
                self.flush_bytes();
                self.chars("-");
            }
            b'\r' | b'\n' => {
                self.flush_bytes();
                self.word("par", None);
            }
            b'\t' => {
                self.flush_bytes();
                self.chars(" ");
            }
            // `\-` optional hyphen, `\:` index subentry, and the rest.
            _ => {}
        }
    }

    fn set_dest(&mut self, d: Dest) {
        self.group_mut().dest = d;
    }

    fn word(&mut self, word: &str, param: Option<i32>) {
        let star = std::mem::replace(&mut self.group_mut().star, false);
        let on = param != Some(0);
        let dest = self.group().dest;
        // Destinations first.
        match word {
            "fonttbl" => return self.set_dest(Dest::FontTable),
            "stylesheet" => {
                self.style_def = Some(0);
                return self.set_dest(Dest::StyleSheet);
            }
            "info" => return self.set_dest(Dest::Info),
            "title" if dest == Dest::Info => return self.set_dest(Dest::InfoTitle),
            "author" if dest == Dest::Info => return self.set_dest(Dest::InfoAuthor),
            "revtbl" => return self.set_dest(Dest::RevTable),
            "footnote" if matches!(dest, Dest::Text | Dest::Footnote) => {
                return self.set_dest(Dest::Footnote);
            }
            "field" => {
                if self.fields.len() < 100_000 {
                    self.fields.push(String::new());
                    self.group_mut().field = Some(self.fields.len() - 1);
                }
                return;
            }
            "fldinst" => return self.set_dest(Dest::FieldInst),
            "fldrslt" => {
                let link = self
                    .group()
                    .field
                    .and_then(|f| self.fields.get(f))
                    .and_then(|inst| hyperlink(inst));
                if let Some(target) = link {
                    self.out.links.push(target);
                    self.group_mut().link = Some(self.out.links.len() - 1);
                }
                return self.set_dest(if dest == Dest::FieldInst {
                    Dest::Skip
                } else {
                    dest
                });
            }
            "listtext" | "pntext" if dest == Dest::Text => return self.set_dest(Dest::ListText),
            "pict" | "shpinst" => return self.set_dest(Dest::Pict),
            "shppict" | "shp" => return,
            "sn" if dest == Dest::Pict => return self.set_dest(Dest::SpName),
            "sv" if dest == Dest::Pict => return self.set_dest(Dest::SpValue),
            "sp" | "picprop" if dest == Dest::Pict => return,
            "colortbl" | "header" | "headerl" | "headerr" | "headerf" | "footer" | "footerl"
            | "footerr" | "footerf" | "filetbl" | "listtable" | "listoverridetable" | "rsidtbl"
            | "generator" | "xmlnstbl" | "themedata" | "colorschememapping" | "datastore"
            | "latentstyles" | "pgdsctbl" | "objdata" | "objclass" | "objname" | "nonshppict"
            | "shprslt" | "pn" | "pntxta" | "pntxtb" | "annotation" | "atnid" | "atnauthor"
            | "atndate" | "atnref" | "atrfstart" | "atrfend" | "txe" | "xe" | "tc" | "tcn"
            | "bkmkstart" | "bkmkend" | "docvar" | "private" | "userprops" | "fontemb"
            | "fontfile" | "falt" | "panose" | "template" | "mmathPr" | "wgrffmtfilter"
            | "nesttableprops" | "ftnsep" | "ftnsepc" | "ftncn" | "aftnsep" | "aftnsepc"
            | "aftncn" | "comment" | "doccomm" | "subject" | "keywords" | "operator"
            | "company" | "manager" | "category" | "hlinkbase" => {
                return self.set_dest(Dest::Skip);
            }
            _ if star => return self.set_dest(Dest::Skip),
            _ => {}
        }
        match dest {
            Dest::FontTable => match word {
                "f" => {
                    self.font_def = param;
                    self.entry.clear();
                }
                "fcharset" => {
                    if let (Some(f), Some(cs)) = (self.font_def, param)
                        && let Some(cp) = charset_codepage(cs)
                    {
                        self.fonts.insert(f, codepage(cp));
                    }
                }
                "cpg" => {
                    if let (Some(f), Some(cp)) = (self.font_def, param) {
                        self.fonts.insert(f, codepage(cp));
                    }
                }
                _ => {}
            },
            Dest::StyleSheet => match word {
                "s" => {
                    self.style_def = param;
                    self.entry.clear();
                }
                "cs" | "ds" | "ts" => {
                    self.style_def = None;
                    self.entry.clear();
                }
                _ => {}
            },
            _ => self.text_word(word, param, on),
        }
    }

    /// Control words that set state or write text.
    fn text_word(&mut self, word: &str, param: Option<i32>, on: bool) {
        match word {
            "ansi" => self.ansi = WINDOWS_1252,
            "mac" => self.ansi = codepage(10000),
            "pc" | "pca" => self.ansi = WINDOWS_1252,
            "ansicpg" => {
                if let Some(cp) = param {
                    self.ansi = codepage(cp);
                }
            }
            "deff" => self.default_font = param,
            "f" => self.group_mut().font = param,
            "uc" => {
                self.group_mut().uc = usize::try_from(param.unwrap_or(1).max(0))
                    .map_or(1, |n| n.min(MAX_UNICODE_SKIP));
            }
            "plain" => {
                let f = self.default_font;
                let g = self.group_mut();
                g.fmt = Fmt::default();
                g.font = f;
            }
            "b" => self.group_mut().fmt.bold = on,
            "i" => self.group_mut().fmt.italic = on,
            "ul" | "uld" | "uldb" | "uldash" | "uldashd" | "uldashdd" | "ulhwave" | "ulth"
            | "ulw" | "ulwave" | "uldbwave" | "ulthd" | "ulthdash" | "ulldash" => {
                self.group_mut().fmt.underline = on;
            }
            "ulnone" => self.group_mut().fmt.underline = false,
            "v" => self.group_mut().fmt.hidden = on,
            "revised" => self.group_mut().fmt.inserted = on,
            "deleted" => self.group_mut().fmt.deleted = on,
            "revauth" => self.group_mut().fmt.ins_author = param.unwrap_or(0),
            "revauthdel" => self.group_mut().fmt.del_author = param.unwrap_or(0),
            "revdttm" => self.group_mut().fmt.ins_date = param.unwrap_or(0),
            "revdttmdel" => self.group_mut().fmt.del_date = param.unwrap_or(0),
            "pard" => {
                let g = self.group_mut();
                g.style = 0;
                g.intbl = false;
                g.list = None;
                g.ilvl = 0;
                g.outline = None;
            }
            "s" => self.group_mut().style = param.unwrap_or(0),
            "intbl" => self.group_mut().intbl = on,
            "ls" => self.group_mut().list = param.filter(|&n| n > 0),
            "ilvl" => {
                self.group_mut().ilvl = u8::try_from(param.unwrap_or(0).clamp(0, 8)).unwrap_or(0);
            }
            "outlinelevel" => {
                self.group_mut().outline = param
                    .filter(|n| (0..9).contains(n))
                    .and_then(|n| u8::try_from(n + 1).ok())
                    .map(|n| n.min(6));
            }
            "par" | "sect" => match self.group().dest {
                Dest::Text => self.end_paragraph(),
                Dest::Footnote => self.footnote.push(' '),
                _ => {}
            },
            "line" => match self.group().dest {
                Dest::Text => self.para.push(Run::Break),
                _ => self.chars(" "),
            },
            "cell" => {
                if self.group().dest == Dest::Text {
                    self.end_cell();
                } else {
                    self.chars(" ");
                }
            }
            "row" => {
                if self.group().dest == Dest::Text {
                    self.end_row();
                }
            }
            "nestcell" | "nestrow" | "tab" => self.chars(" "),
            "trowd" => self.header_row = false,
            "trhdr" => self.header_row = true,
            "emdash" => self.chars("\u{2014}"),
            "endash" => self.chars("\u{2013}"),
            "emspace" | "enspace" | "qmspace" => self.chars(" "),
            "bullet" => self.chars("\u{2022}"),
            "lquote" => self.chars("\u{2018}"),
            "rquote" => self.chars("\u{2019}"),
            "ldblquote" => self.chars("\u{201c}"),
            "rdblquote" => self.chars("\u{201d}"),
            _ => {}
        }
    }

    /// Paragraph properties of the current group, for the paragraph that
    /// just ended.
    fn para_props(&mut self) -> (Option<u8>, Option<(u8, Option<String>)>) {
        let g = self.group().clone();
        let heading = g.outline.or_else(|| {
            let name = self.styles.get(&g.style)?;
            if let Some(n) = name.strip_prefix("heading ")
                && let Ok(n) = n.trim().parse::<u8>()
            {
                return Some(n.clamp(1, 6));
            }
            (name == "title").then_some(1)
        });
        let label = self.label.take();
        let list = if heading.is_none() && (g.list.is_some() || label.is_some()) {
            Some((g.ilvl + 1, label))
        } else {
            None
        };
        (heading, list)
    }

    fn end_paragraph(&mut self) {
        if self.group().intbl {
            // A paragraph inside a cell: the cell goes on with a space.
            self.label = None;
            self.para.push(Run::Text {
                text: " ".into(),
                fmt: Fmt::default(),
                link: None,
            });
            return;
        }
        if !self.cells.is_empty() {
            self.end_row();
        }
        let (heading, list) = self.para_props();
        let runs = std::mem::take(&mut self.para);
        if runs.is_empty() {
            return;
        }
        self.out.blocks.push(Block::Para(Para {
            runs,
            heading,
            list,
        }));
    }

    fn end_cell(&mut self) {
        self.label = None;
        let runs = std::mem::take(&mut self.para);
        if self.cells.len() < 1_000 {
            self.cells.push(runs);
        }
    }

    fn end_row(&mut self) {
        if !self.para.is_empty() && !self.cells.is_empty() {
            // Text after the last `\cell` of a row.
            let runs = std::mem::take(&mut self.para);
            if let Some(last) = self.cells.last_mut() {
                last.push(Run::Text {
                    text: " ".into(),
                    fmt: Fmt::default(),
                    link: None,
                });
                last.extend(runs);
            }
        }
        let cells = std::mem::take(&mut self.cells);
        if cells.is_empty() {
            return;
        }
        self.out.blocks.push(Block::Row {
            cells,
            header: self.header_row,
        });
    }
}

/// The target of a `HYPERLINK "url" \l "anchor"` field instruction.
fn hyperlink(inst: &str) -> Option<String> {
    let rest = inst.trim_start().strip_prefix("HYPERLINK")?;
    let mut url = None;
    let mut anchor = None;
    let mut next_is_anchor = false;
    let mut chars = rest.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                let flag = chars.next();
                next_is_anchor = flag == Some('l');
            }
            '"' => {
                let s: String = chars.by_ref().take_while(|&c| c != '"').collect();
                if next_is_anchor {
                    anchor = Some(s);
                } else if url.is_none() {
                    url = Some(s);
                }
                next_is_anchor = false;
            }
            c if !c.is_whitespace() && url.is_none() && !next_is_anchor => {
                // An unquoted address.
                let mut s = String::from(c);
                while let Some(&n) = chars.peek() {
                    if n.is_whitespace() {
                        break;
                    }
                    s.push(n);
                    chars.next();
                }
                url = Some(s);
            }
            _ => {}
        }
    }
    let url = url.filter(|u| !u.trim().is_empty());
    let anchor = anchor.filter(|a| !a.trim().is_empty());
    match (url, anchor) {
        (Some(u), Some(a)) => Some(format!("{u}#{a}")),
        (Some(u), None) => Some(u),
        (None, Some(a)) => Some(format!("#{a}")),
        (None, None) => None,
    }
}

/// The Windows code page for an RTF `\fcharset`; `None` for the document's
/// own (`\ansicpg`).
fn charset_codepage(cs: i32) -> Option<i32> {
    Some(match cs {
        0 | 1 => return None,
        2 => 1252,
        77 => 10000,
        78 | 128 => 932,
        129 | 130 => 949,
        134 => 936,
        136 => 950,
        161 => 1253,
        162 => 1254,
        163 => 1258,
        177 => 1255,
        178 => 1256,
        186 => 1257,
        204 => 1251,
        222 => 874,
        238 => 1250,
        _ => return None,
    })
}

/// The encoding for a Windows code page; Windows-1252 for pages
/// encoding_rs does not have (the DOS pages 437 and 850).
fn codepage(cp: i32) -> &'static Encoding {
    let label = match cp {
        866 => "ibm866".to_owned(),
        874 => "windows-874".to_owned(),
        932 => "shift_jis".to_owned(),
        936 => "gbk".to_owned(),
        949 => "euc-kr".to_owned(),
        950 => "big5".to_owned(),
        1250..=1258 => format!("windows-{cp}"),
        10000 => "macintosh".to_owned(),
        10007 => "x-mac-cyrillic".to_owned(),
        20866 => "koi8-r".to_owned(),
        21866 => "koi8-u".to_owned(),
        28592..=28606 => format!("iso-8859-{}", cp - 28590),
        51932 => "euc-jp".to_owned(),
        54936 => "gb18030".to_owned(),
        65001 => "utf-8".to_owned(),
        _ => return WINDOWS_1252,
    };
    Encoding::for_label(label.as_bytes()).unwrap_or(WINDOWS_1252)
}

/// Builds the canonical text; returns it, its markers, and the number of
/// tracked changes.
fn render(
    parsed: &Parsed,
    options: &LoadOptions,
) -> (String, Vec<Marker>, Vec<crate::DocumentChange>) {
    let mut r = Render {
        b: Builder::new(),
        parsed,
        options,
        fmt: [None; 3],
        link: None,
        change: None,
        lists: Vec::new(),
        table: None,
        in_cell: false,
        deferred: Vec::new(),
        note_count: 0,
        recorded: Vec::new(),
    };
    for block in &parsed.blocks {
        match block {
            Block::Para(p) => {
                r.close_table();
                r.paragraph(p);
            }
            Block::Row { cells, header } => r.row(cells, *header),
        }
    }
    r.close_table();
    r.close_lists();
    let deferred = std::mem::take(&mut r.deferred);
    r.b.footnotes_section(&deferred);
    r.set_change(None);
    let recorded = std::mem::take(&mut r.recorded);
    let (text, markers) = r.b.finish();
    (text, markers, recorded)
}

const FMT_KINDS: [MarkerKind; 3] = [MarkerKind::Bold, MarkerKind::Italic, MarkerKind::Underline];

struct Render<'a> {
    b: Builder,
    parsed: &'a Parsed,
    options: &'a LoadOptions,
    fmt: [Option<OpenId>; 3],
    link: Option<(usize, OpenId)>,
    /// The change being read, with its kind, author, and time.
    change: Option<(ChangeKey, revision::Open)>,
    lists: Vec<OpenId>,
    table: Option<OpenId>,
    in_cell: bool,
    deferred: Vec<(String, String)>,
    note_count: usize,
    /// Tracked changes seen, as recorded.
    recorded: Vec<crate::DocumentChange>,
}

impl Render<'_> {
    fn marker(kind: MarkerKind) -> Marker {
        Marker::new(kind, CharRange::empty(0))
    }

    fn close_lists(&mut self) {
        while let Some(id) = self.lists.pop() {
            self.b.close(id);
        }
    }

    fn close_table(&mut self) {
        if let Some(t) = self.table.take() {
            self.b.close(t);
            self.b.paragraph_break();
        }
    }

    fn close_inline(&mut self) {
        self.set_fmt([false; 3]);
        self.set_link(None);
        self.set_change(None);
    }

    fn set_fmt(&mut self, want: [bool; 3]) {
        for (i, w) in want.into_iter().enumerate() {
            match (self.fmt[i], w) {
                (None, true) => self.fmt[i] = Some(self.b.open(Self::marker(FMT_KINDS[i]))),
                (Some(id), false) => {
                    self.b.close(id);
                    self.fmt[i] = None;
                }
                _ => {}
            }
        }
    }

    fn set_link(&mut self, want: Option<usize>) {
        if self.link.as_ref().map(|(l, _)| *l) == want {
            return;
        }
        if let Some((_, id)) = self.link.take() {
            self.set_fmt([false; 3]);
            self.b.close(id);
        }
        if let Some(l) = want
            && let Some(target) = self.parsed.links.get(l)
        {
            let id = self
                .b
                .open(Self::marker(MarkerKind::Link).with_reference(target.as_str()));
            self.link = Some((l, id));
        }
    }

    fn author(&self, i: i32) -> Option<&str> {
        usize::try_from(i)
            .ok()
            .and_then(|i| self.parsed.authors.get(i))
            .map(String::as_str)
            .filter(|a| !a.is_empty() && !a.eq_ignore_ascii_case("unknown"))
    }

    /// Opens or closes the change being read (said in place when reading
    /// marked), recording each one.
    fn set_change(&mut self, want: Option<ChangeKey>) {
        if self.change.as_ref().map(|(k, _)| *k) == want {
            return;
        }
        let marked = self.options.revisions == RevisionMode::Marked;
        if let Some((_, open)) = self.change.take() {
            if marked {
                self.set_fmt([false; 3]);
                self.set_link(None);
            }
            revision::close(&mut self.b, open, &mut self.recorded);
        }
        if let Some((kind, author, packed)) = want {
            if marked {
                self.set_fmt([false; 3]);
                self.set_link(None);
            }
            let name = self.author(author).map(str::to_owned);
            let date = dttm(packed);
            let open = revision::open(
                &mut self.b,
                kind,
                name.as_deref(),
                date.as_deref(),
                None,
                marked,
            );
            self.change = Some(((kind, author, packed), open));
        }
    }

    fn runs(&mut self, runs: &[Run]) {
        let marked = self.options.revisions == RevisionMode::Marked;
        for run in runs {
            match run {
                Run::Text { text, fmt, link } => {
                    let change = if fmt.deleted {
                        Some((ChangeKind::Deleted, fmt.del_author, fmt.del_date))
                    } else if fmt.inserted {
                        Some((ChangeKind::Inserted, fmt.ins_author, fmt.ins_date))
                    } else {
                        None
                    };
                    self.set_change(change);
                    if let Some((_, open)) = self.change.as_mut() {
                        open.saw_text(text);
                    }
                    if fmt.deleted && !marked {
                        if let Some((_, open)) = self.change.as_mut() {
                            open.push_deleted(text);
                        }
                        continue;
                    }
                    self.set_link(*link);
                    self.set_fmt([fmt.bold, fmt.italic, fmt.underline]);
                    self.b.text(text);
                }
                Run::Break => {
                    if self.in_cell {
                        self.b.space();
                    } else {
                        self.b.line_break();
                    }
                }
                Run::Note(text) => {
                    self.close_inline();
                    self.note(text);
                }
                Run::Image(alt) => {
                    self.close_inline();
                    self.b.space();
                    let id = self.b.open(Self::marker(MarkerKind::Image));
                    self.b.text(alt);
                    self.b.close(id);
                    self.b.soft_space();
                }
            }
        }
        self.close_inline();
    }

    fn note(&mut self, text: &str) {
        match self.options.footnotes {
            FootnoteMode::Skip => {}
            FootnoteMode::Inline => {
                self.note_count += 1;
                let label = self.note_count.to_string();
                self.b.inline_footnote(&label, text);
            }
            FootnoteMode::Deferred => {
                self.note_count += 1;
                let label = self.note_count.to_string();
                self.b.footnote_reference(&label);
                self.deferred.push((label, text.to_owned()));
            }
        }
    }

    fn paragraph(&mut self, p: &Para) {
        if let Some(level) = p.heading {
            self.close_lists();
            self.b.paragraph_break();
            let id = self
                .b
                .open(Self::marker(MarkerKind::Heading).with_level(level));
            self.runs(&p.runs);
            self.b.close(id);
            self.b.paragraph_break();
        } else if let Some((depth, label)) = &p.list {
            let depth = (*depth).clamp(1, 9);
            while self.lists.len() > usize::from(depth) {
                if let Some(id) = self.lists.pop() {
                    self.b.close(id);
                }
            }
            if self.lists.is_empty() {
                self.b.paragraph_break();
            }
            while self.lists.len() < usize::from(depth) {
                let d = u8::try_from(self.lists.len() + 1).unwrap_or(1);
                let id = self.b.open(Self::marker(MarkerKind::List).with_level(d));
                self.lists.push(id);
            }
            self.b.line_break();
            let mut m = Self::marker(MarkerKind::ListItem).with_level(depth);
            if let Some(l) = label.as_ref().filter(|l| !is_bullet(l)) {
                m = m.with_label(l.as_str());
            }
            let id = self.b.open(m);
            self.runs(&p.runs);
            self.b.close(id);
        } else {
            self.close_lists();
            self.b.paragraph_break();
            let id = self.b.open(Self::marker(MarkerKind::Paragraph));
            self.runs(&p.runs);
            self.b.close(id);
            self.b.paragraph_break();
        }
    }

    fn row(&mut self, cells: &[Vec<Run>], header: bool) {
        self.close_lists();
        if self.table.is_none() {
            self.b.paragraph_break();
            self.table = Some(self.b.open(Self::marker(MarkerKind::Table)));
        }
        self.b.line_break();
        let mut rm = Self::marker(MarkerKind::TableRow);
        if header {
            rm = rm.with_label(HEADER_ROW_LABEL);
        }
        let row = self.b.open(rm);
        for (j, cell) in cells.iter().enumerate() {
            if j > 0 {
                self.b.separator(crate::CELL_SEPARATOR);
            }
            let c = self.b.open_here(Self::marker(MarkerKind::TableCell));
            self.in_cell = true;
            self.runs(cell);
            self.in_cell = false;
            self.b.close(c);
        }
        self.b.close(row);
    }
}

/// A list label that is only a bullet character (read as nothing, as in
/// the other loaders).
fn is_bullet(label: &str) -> bool {
    label.chars().count() == 1 && !label.chars().any(char::is_alphanumeric)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_with(rtf: &str, options: &LoadOptions) -> Document {
        RtfLoader
            .load(
                &Source::Bytes {
                    data: rtf.as_bytes().to_vec(),
                    hint: "rtf".into(),
                },
                options,
            )
            .expect("the RTF loads")
    }

    fn load(rtf: &str) -> Document {
        load_with(rtf, &LoadOptions::default())
    }

    /// The `u` control word's backslash and name, built at run time so
    /// no tool reads it as a Unicode escape in this source.
    fn uni_word() -> String {
        format!("{}u", '\\')
    }

    /// An RTF Unicode character, `u` with its value.
    fn uni(n: i32) -> String {
        format!("{}{n}", uni_word())
    }

    fn texts(doc: &Document, kind: MarkerKind) -> Vec<String> {
        doc.markers()
            .iter()
            .filter(|m| m.kind == kind)
            .map(|m| doc.slice(m.range).to_string())
            .collect()
    }

    const HEAD: &str = r"{\rtf1\ansi\ansicpg1252\deff0{\fonttbl{\f0\fswiss\fcharset0 Arial;}{\f1\fcharset204 Times Cyr;}}{\stylesheet{\s0 Normal;}{\s1\sbasedon0 heading 1;}{\s2 Heading 2;}{\*\cs10 Default Paragraph Font;}}{\info{\title Handout}{\author Ada Example}{\operator Someone}}";

    #[test]
    fn headings_paragraphs_and_formatting() {
        let doc = load(&format!(
            r"{HEAD}\pard\s1 Week one\par\pard\plain Some {{\b bold}} and {{\i italic}} text.\par\pard\s2 Part\par}}"
        ));
        assert_eq!(
            doc.text().to_string(),
            "Week one\n\nSome bold and italic text.\n\nPart"
        );
        assert_eq!(texts(&doc, MarkerKind::Heading), vec!["Week one", "Part"]);
        assert_eq!(texts(&doc, MarkerKind::Bold), vec!["bold"]);
        assert_eq!(texts(&doc, MarkerKind::Italic), vec!["italic"]);
        assert_eq!(doc.meta.title.as_deref(), Some("Handout"));
        assert_eq!(doc.meta.author.as_deref(), Some("Ada Example"));
    }

    #[test]
    fn code_pages_unicode_and_skips() {
        // Windows-1252 bytes, a Cyrillic font, \u with its fallback, and
        // a surrogate pair.
        let doc = load(&format!(
            r"{HEAD}\pard caf\'e9 {{\f1 \'cf\'f0\'e8\'e2\'e5\'f2}} {}? \uc2{} EU x\uc1{}?{}?\par}}",
            uni(8212),
            uni(8364),
            uni(-10179),
            uni(-8704)
        ));
        assert_eq!(
            doc.text().to_string(),
            "café Привет \u{2014} \u{20ac} x\u{1f600}"
        );
    }

    #[test]
    fn double_byte_code_pages() {
        let doc = load(
            r"{\rtf1\ansi\ansicpg932\deff0{\fonttbl{\f0\fcharset128 MS Mincho;}}\pard \'93\'fa\'96\'7b\par}",
        );
        assert_eq!(doc.text().to_string(), "日本");
    }

    #[test]
    fn destinations_are_skipped_and_symbols_read() {
        let doc = load(&format!(
            r"{HEAD}{{\header Page header}}{{\*\unknowndest secret}}{{\colortbl;\red0\green0\blue0;}}\pard A\~B\emdash C \{{x\}} \\ \lquote q\rquote{{\v hidden}}\par}}"
        ));
        // The non-breaking space reads as a space.
        assert_eq!(
            doc.text().to_string(),
            "A B\u{2014}C {x} \\ \u{2018}q\u{2019}"
        );
    }

    #[test]
    fn lists_take_their_rendered_labels() {
        let doc = load(&format!(
            r"{HEAD}\pard\ls1\ilvl0{{\listtext 1.\tab}}First\par{{\listtext 2.\tab}}Second\par\pard\ls1\ilvl1{{\listtext a.\tab}}Inner\par\pard\ls2{{\listtext \'b7\tab}}Bullet\par\pard After\par}}"
        ));
        assert_eq!(
            doc.text().to_string(),
            "First\nSecond\nInner\nBullet\n\nAfter"
        );
        let items: Vec<(String, u8, Option<String>)> = doc
            .markers()
            .iter()
            .filter(|m| m.kind == MarkerKind::ListItem)
            .map(|m| (doc.slice(m.range).to_string(), m.level, m.label.clone()))
            .collect();
        assert_eq!(
            items,
            vec![
                ("First".into(), 1, Some("1.".into())),
                ("Second".into(), 1, Some("2.".into())),
                ("Inner".into(), 2, Some("a.".into())),
                ("Bullet".into(), 1, None),
            ]
        );
    }

    #[test]
    fn tables_rows_and_header() {
        let doc = load(&format!(
            r"{HEAD}\pard Before\par\trowd\trhdr\cellx1000\cellx2000\pard\intbl Name\cell Mark\cell\row\trowd\cellx1000\cellx2000\pard\intbl Ada\cell A\par plus\cell\row\pard After\par}}"
        ));
        assert_eq!(
            doc.text().to_string(),
            "Before\n\nName | Mark\nAda | A plus\n\nAfter"
        );
        let rows: Vec<Option<String>> = doc
            .markers()
            .iter()
            .filter(|m| m.kind == MarkerKind::TableRow)
            .map(|m| m.label.clone())
            .collect();
        assert_eq!(rows, vec![Some(HEADER_ROW_LABEL.to_owned()), None]);
        assert_eq!(texts(&doc, MarkerKind::TableCell).len(), 4);
    }

    #[test]
    fn footnotes_and_links() {
        let rtf = format!(
            r#"{HEAD}\pard See{{\super\chftn}}{{\footnote\pard\plain{{\super\chftn}} The source.}} {{\field{{\*\fldinst {{HYPERLINK "https://example.org/a"}}}}{{\fldrslt {{\ul the page}}}}}}.\par}}"#
        );
        let doc = load(&rtf);
        assert_eq!(
            doc.text().to_string(),
            "See[1] the page.\n\nFootnotes\n\n[1] The source."
        );
        let links: Vec<Option<String>> = doc
            .markers()
            .iter()
            .filter(|m| m.kind == MarkerKind::Link)
            .map(|m| m.reference.clone())
            .collect();
        assert_eq!(links, vec![Some("https://example.org/a".into())]);
        let inline = load_with(
            &rtf,
            &LoadOptions {
                footnotes: FootnoteMode::Inline,
                ..LoadOptions::default()
            },
        );
        assert_eq!(
            inline.text().to_string(),
            "See (footnote: The source.) the page."
        );
        assert_eq!(
            hyperlink(r#"HYPERLINK \l "sec2""#).as_deref(),
            Some("#sec2")
        );
        assert_eq!(
            hyperlink("HYPERLINK https://x.org").as_deref(),
            Some("https://x.org")
        );
        assert_eq!(hyperlink("PAGE"), None);
    }

    #[test]
    fn tracked_changes_follow_the_option() {
        let rtf = format!(
            r"{HEAD}{{\*\revtbl {{Unknown;}}{{Ada Example;}}}}\pard The {{\deleted\revauthdel1 old }}{{\revised\revauth1 new }}plan.\par}}"
        );
        let doc = load(&rtf);
        assert_eq!(doc.text().to_string(), "The new plan.");
        assert_eq!(crate::revision_count(&doc.meta), 2);
        let marked = load_with(
            &rtf,
            &LoadOptions {
                revisions: RevisionMode::Marked,
                ..LoadOptions::default()
            },
        );
        assert_eq!(
            marked.text().to_string(),
            "The (deleted by Ada Example: old) (inserted by Ada Example: new) plan."
        );
        assert_eq!(texts(&marked, MarkerKind::Strikethrough), vec!["old"]);
        assert_eq!(texts(&marked, MarkerKind::Underline), vec!["new"]);
    }

    #[test]
    fn picture_alt_text() {
        let doc = load(&format!(
            r"{HEAD}\pard A {{\*\shppict{{\pict{{\*\picprop{{\sp{{\sn wzDescription}}{{\sv A crow on a wire}}}}}}\pngblip 89504e47}}}}{{\nonshppict{{\pict\wmetafile8 0102}}}} B\par}}"
        ));
        assert_eq!(doc.text().to_string(), "A A crow on a wire B");
        assert_eq!(texts(&doc, MarkerKind::Image), vec!["A crow on a wire"]);
    }

    #[test]
    fn hostile_input_stays_bounded() {
        // Deep nesting, a huge \bin, a huge \uc, and a long control word.
        let deep = format!(
            r"{{\rtf1 {}deep{} {}c99999999{} xyzabcdefghijk\bin999999999999 tail",
            "{".repeat(100_000),
            "}".repeat(100_000),
            uni_word(),
            uni(8364)
        );
        let doc = load(&deep);
        assert!(doc.text().to_string().starts_with("deep"));
        assert!(crate::warnings(&doc.meta).contains(&crate::NESTING_WARNING.to_owned()));
        let word = format!(r"{{\rtf1 \{}5 text}}", "a".repeat(100_000));
        assert_eq!(load(&word).text().to_string(), "text");
        let err = RtfLoader.load(
            &Source::Bytes {
                data: b"not rtf".to_vec(),
                hint: "rtf".into(),
            },
            &LoadOptions::default(),
        );
        assert!(matches!(err, Err(LoadError::Parse(_))));
    }
}
