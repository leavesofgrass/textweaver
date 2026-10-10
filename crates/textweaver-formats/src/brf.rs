//! Braille Ready Format (BRF) files, read as print: the clear-text braille
//! books that libraries for blind readers deliver, such as the braille
//! downloads of the NLS BARD service, often several volumes in a zip.
//!
//! A BRF file is braille in braille ASCII (one character per cell, codes
//! 0x20 to 0x5F, lowercase letters as the same cells), laid out in lines
//! and pages: lines end in CR LF, pages are separated by a form feed.
//! Unicode braille patterns (U+2800 to U+283F) are accepted too.
//!
//! The loader rebuilds the print structure from the braille layout, which
//! follows BANA's Braille Formats (2016):
//!
//! - the braille page number at the right end of each page's last line,
//!   and a print page number at the right end of its first line, are taken
//!   out of the text; each braille page becomes a `PageBreak` marker
//!   labeled with its braille page number ("3", "p1"), whose reference is
//!   the page's position in the file (1 for the first);
//! - a print page change indicator (a line of dots 3-6 ending in the print
//!   page number), and a new print page number at the top of a page, become
//!   a paragraph "Print page 12", as a braille reader meets it;
//! - a line that repeats at the top of most pages is a running head and is
//!   read once;
//! - a centered line is a heading (level 1), and centered lines in a row
//!   one heading; a line in cell 5 after a blank line is a level 2 heading;
//! - a line in cell 3 starts a paragraph, a line in cell 1 continues it
//!   (also across a page), and a blank line ends it; a line ending in the
//!   line continuation indicator (dot 5) joins the next with no space.
//!
//! This is a heuristic: poetry, tables and forms may come out joined or
//! split wrongly, and the document carries a warning saying the layout was
//! rebuilt. When it clearly fails (a paragraph of more than
//! [`MAX_PARAGRAPH_LINES`] lines), every braille line is read as its own
//! paragraph, and the warning says so.
//!
//! Each paragraph is then back-translated to print through liblouis's
//! `lou_translate --backward`, one process per file, with the code chosen by
//! [`LoadOptions::brf_code`](crate::LoadOptions): Unified English Braille
//! (`en-ueb-g2.ctb`, the default) or English Braille American Edition
//! (`en-us-g2.ctb`, for older books). Either table reads contracted and
//! uncontracted braille. The original braille stays in the file; the
//! reader's "Show original Braille" reads a page of it again with
//! [`original_pages`].
//!
//! Without liblouis the file opens as braille, in Unicode braille patterns
//! with the same structure, and the document's [`TRANSLATION_PROPERTY`] is
//! [`UNTRANSLATED`], so the reader can say how to get liblouis.
//!
//! Only clear-text braille files are read. Protected talking books are not
//! opened, and nothing here touches any library's protection.

use ropey::Rope;
use serde::{Deserialize, Serialize};
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, Marker};

use crate::builder::{Builder, OpenId};
use crate::{LoadError, LoadOptions, Loader, Source, add_warning, meta_for, title_from_path};

/// The braille code a BRF file is read in (the `[braille] brf_code`
/// setting).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BrfCode {
    /// Unified English Braille, the code of English braille books in the
    /// United States since 2016.
    #[default]
    Ueb,
    /// English Braille American Edition, the code of older American books.
    Ebae,
}

impl BrfCode {
    /// The liblouis table that reads this code.
    pub fn table(self) -> &'static str {
        match self {
            BrfCode::Ueb => "en-ueb-g2.ctb",
            BrfCode::Ebae => "en-us-g2.ctb",
        }
    }

    /// The setting's value: `ueb` or `ebae`.
    pub fn id(self) -> &'static str {
        match self {
            BrfCode::Ueb => "ueb",
            BrfCode::Ebae => "ebae",
        }
    }
}

/// The `DocumentMeta::properties` key saying what a BRF file was read as:
/// the code's [`id`](BrfCode::id), or [`UNTRANSLATED`].
pub const TRANSLATION_PROPERTY: &str = "brf.translation";

/// [`TRANSLATION_PROPERTY`]'s value when liblouis could not run and the
/// file is shown as braille.
pub const UNTRANSLATED: &str = "none";

/// The warning when liblouis is missing.
pub const NO_LIBLOUIS_WARNING: &str = "liblouis is not installed, so this braille file is shown as braille. Install liblouis, then open the file again to read it as print.";

/// The warning every BRF document carries.
pub const REBUILT_WARNING: &str = "Paragraphs and headings were rebuilt from the braille layout, so a few may be joined or split wrongly.";

/// The warning when the layout could not be rebuilt.
pub const LINES_WARNING: &str =
    "The braille layout could not be rebuilt into paragraphs, so each braille line is a paragraph.";

/// The warning when a `.brf` file is not braille.
pub const NOT_BRAILLE_WARNING: &str =
    "This file is not in braille ASCII, so it is read as plain text.";

/// A paragraph longer than this many braille lines means the layout was
/// not understood (no indents and no blank lines).
pub const MAX_PARAGRAPH_LINES: usize = 60;

/// Braille ASCII for Unicode braille patterns U+2800 to U+283F, in order.
/// The same table as the BRF writer's (`textweaver-writers`, `ueb.rs`),
/// which this crate does not depend on.
const BRAILLE_ASCII: &[u8; 64] =
    b" A1B'K2L@CIF/MSP\"E3H9O6R^DJG>NTQ,*5<-U8V.%[$+X!&;:4\\0Z7(_?W]#Y)=";

/// Loads BRF files.
#[derive(Clone, Copy, Debug, Default)]
pub struct BrfLoader;

impl Loader for BrfLoader {
    fn id(&self) -> &'static str {
        "brf"
    }

    // shortcut: chosen by extension only, never by sniffing, so no other
    // text file is taken for braille; sniff form feeds and the braille
    // ASCII range if unnamed BRF files turn up.
    fn extensions(&self) -> &'static [&'static str] {
        &["brf", "brl"]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let bytes = source.read()?;
        let Some(ascii) = ascii_braille(&bytes) else {
            let mut doc = crate::TextLoader.load(source, options)?;
            add_warning(&mut doc.meta, NOT_BRAILLE_WARNING);
            return Ok(doc);
        };
        let code = options.brf_code;
        let mut doc = read(&ascii, |texts| liblouis(texts, code));
        let mut meta = meta_for(source, self.id());
        meta.title = title_from_path(source);
        meta.properties = std::mem::take(&mut doc.meta.properties);
        meta.properties
            .entry(TRANSLATION_PROPERTY.to_owned())
            .or_insert_with(|| code.id().to_owned());
        doc.meta = meta;
        Ok(doc)
    }
}

/// The pages of a BRF file as Unicode braille, one string per line (lines
/// keep their leading blank cells as spaces), for showing the original
/// braille; `None` when the bytes are not braille.
pub fn original_pages(bytes: &[u8]) -> Option<Vec<Vec<String>>> {
    let ascii = ascii_braille(bytes)?;
    Some(
        split_pages(&ascii)
            .into_iter()
            .map(|page| page.iter().map(|l| to_unicode(l)).collect())
            .collect(),
    )
}

/// Unicode braille patterns for braille ASCII (spaces stay spaces; other
/// characters pass through).
pub fn to_unicode(ascii: &str) -> String {
    ascii
        .chars()
        .map(|c| {
            let up = normalize_ascii(c).unwrap_or(c);
            BRAILLE_ASCII
                .iter()
                .position(|&b| char::from(b) == up)
                .filter(|&i| i > 0)
                .and_then(|i| char::from_u32(0x2800 + i as u32))
                .unwrap_or(up)
        })
        .collect()
}

/// A braille ASCII character in the table's form: lowercase letters and the
/// lowercase-range symbols as their uppercase cells.
fn normalize_ascii(c: char) -> Option<char> {
    match c {
        'a'..='z' => Some(c.to_ascii_uppercase()),
        '`' => Some('@'),
        '{' => Some('['),
        '|' => Some('\\'),
        '}' => Some(']'),
        '~' => Some('^'),
        ' '..='_' => Some(c),
        _ => None,
    }
}

/// The file as braille ASCII (uppercase, Unicode patterns converted, line
/// ends as line feeds), or `None` when more than one character in a
/// hundred is neither braille nor layout.
fn ascii_braille(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let mut out = String::with_capacity(text.len());
    let (mut bad, mut cells) = (0usize, 0usize);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.peek() != Some(&'\n') {
                    out.push('\n');
                }
            }
            '\n' | '\u{0C}' => out.push(c),
            '\t' => out.push(' '),
            '\u{2800}'..='\u{28FF}' => {
                cells += 1;
                // Dots 7 and 8 have no place in a BRF file; dropped.
                out.push(char::from(BRAILLE_ASCII[(c as usize - 0x2800) & 0x3F]));
            }
            _ => match normalize_ascii(c) {
                Some(n) => {
                    cells += 1;
                    out.push(n);
                }
                None => bad += 1,
            },
        }
    }
    (cells > 0 && bad * 100 <= cells).then_some(out)
}

/// The pages, each its lines without trailing blank cells. Blank lines at
/// a page's end are dropped; a trailing form feed makes no extra page.
fn split_pages(ascii: &str) -> Vec<Vec<String>> {
    let mut pages: Vec<Vec<String>> = ascii
        .split('\u{0C}')
        .map(|p| {
            let mut lines: Vec<String> = p.split('\n').map(|l| l.trim_end().to_owned()).collect();
            while lines.last().is_some_and(String::is_empty) {
                lines.pop();
            }
            lines
        })
        .collect();
    if pages.len() > 1 && pages.last().is_some_and(Vec::is_empty) {
        pages.pop();
    }
    pages
}

/// A page number written in braille: an optional letter prefix (`P` for
/// preliminary pages, `T` for transcriber's pages) and a number sign with
/// the digits a to j. Returned as print, `p1` or `12`.
fn page_token(token: &str) -> Option<String> {
    let (prefix, number) = token.split_once('#')?;
    if prefix.len() > 2 || !prefix.bytes().all(|b| b.is_ascii_uppercase()) {
        return None;
    }
    if number.is_empty() || !number.bytes().all(|b| (b'A'..=b'J').contains(&b)) {
        return None;
    }
    let digits: String = number
        .bytes()
        .map(|b| {
            if b == b'J' {
                '0'
            } else {
                char::from(b - b'A' + b'1')
            }
        })
        .collect();
    Some(format!("{}{digits}", prefix.to_ascii_lowercase()))
}

/// A page number at the right margin of `line` (on a page `width` cells
/// wide): the line without it, and the number. The number must end at the
/// margin and stand three cells clear of any text.
fn right_number(line: &str, width: usize) -> Option<(String, String)> {
    if width < 20 || line.len() + 1 < width {
        return None;
    }
    let start = line.rfind(' ')? + 1;
    let label = page_token(&line[start..])?;
    let rest = &line[..start];
    if !rest.trim().is_empty() && !rest.ends_with("   ") {
        return None;
    }
    Some((rest.trim_end().to_owned(), label))
}

/// A print page change indicator: a line of dots 3-6, ending in the new
/// print page number when it has one.
fn print_indicator(line: &str) -> Option<Option<String>> {
    let t = line.trim();
    let dashes = t.bytes().take_while(|&b| b == b'-').count();
    if dashes < 5 {
        return None;
    }
    let rest = t[dashes..].trim();
    if rest.is_empty() {
        return Some(None);
    }
    page_token(rest).map(Some)
}

/// What a rebuilt block is.
#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Paragraph,
    Heading(u8),
    /// A print page change, with the print page number.
    PrintPage(String),
}

/// A stretch of a block on one braille page.
#[derive(Clone, Debug)]
struct Piece {
    page: usize,
    braille: String,
    /// Joined to the piece before with no space (a word divided over a
    /// page with the line continuation indicator).
    glued: bool,
}

/// A heading, paragraph or print page change, in pieces by braille page.
#[derive(Clone, Debug)]
struct Block {
    kind: Kind,
    pieces: Vec<Piece>,
    lines: usize,
}

impl Block {
    fn new(kind: Kind, page: usize, braille: &str) -> Self {
        Block {
            kind,
            pieces: vec![Piece {
                page,
                braille: braille.to_owned(),
                glued: false,
            }],
            lines: 1,
        }
    }

    /// Adds a line: a space between, or none after the line continuation
    /// indicator (dot 5 right after a cell).
    fn push_line(&mut self, page: usize, braille: &str) {
        self.lines += 1;
        let Some(last) = self.pieces.last_mut() else {
            return;
        };
        let continued = last.braille.len() > 1
            && last.braille.ends_with('"')
            && !last.braille[..last.braille.len() - 1].ends_with(' ');
        if continued {
            last.braille.pop();
        }
        if last.page == page {
            if !continued {
                last.braille.push(' ');
            }
            last.braille.push_str(braille);
        } else {
            self.pieces.push(Piece {
                page,
                braille: braille.to_owned(),
                glued: continued,
            });
        }
    }
}

/// The pages after the numbers and running heads are taken out: their
/// lines, braille page labels, and top print page numbers.
struct Layout {
    pages: Vec<Vec<String>>,
    labels: Vec<Option<String>>,
    top_print: Vec<Option<String>>,
    width: usize,
}

fn layout(ascii: &str) -> Layout {
    let mut pages = split_pages(ascii);
    let width = pages.iter().flatten().map(String::len).max().unwrap_or(0);
    let mut labels = vec![None; pages.len()];
    let mut top_print = vec![None; pages.len()];
    let filled = |p: &[String]| p.iter().rposition(|l| !l.trim().is_empty());
    for (i, page) in pages.iter_mut().enumerate() {
        if let Some(last) = filled(page)
            && let Some((rest, label)) = right_number(&page[last], width)
        {
            page[last] = rest;
            labels[i] = Some(label);
        }
        if let Some(first) = page.iter().position(|l| !l.trim().is_empty())
            && filled(page).is_some_and(|last| last > first)
            && let Some((rest, label)) = right_number(&page[first], width)
        {
            page[first] = rest;
            top_print[i] = Some(label);
        }
    }
    // A running head: the same first line on at least three pages and on
    // more than half of them; read where it first appears only.
    let firsts: Vec<Option<(usize, String)>> = pages
        .iter()
        .map(|p| {
            p.iter()
                .position(|l| !l.trim().is_empty())
                .map(|n| (n, p[n].trim().to_owned()))
        })
        .collect();
    let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for (_, line) in firsts.iter().flatten() {
        *counts.entry(line.as_str()).or_default() += 1;
    }
    let head = counts
        .into_iter()
        .filter(|&(_, n)| n >= 3 && n * 2 > pages.len())
        .max_by_key(|&(l, n)| (n, l))
        .map(|(l, _)| l.to_owned());
    if let Some(head) = head {
        let mut seen = false;
        for (page, first) in pages.iter_mut().zip(&firsts) {
            if let Some((n, line)) = first
                && *line == head
            {
                if seen {
                    page[*n].clear();
                }
                seen = true;
            }
        }
    }
    Layout {
        pages,
        labels,
        top_print,
        width,
    }
}

/// The blocks of `layout`, by the layout rules, or one per line when
/// `by_line`.
fn blocks(layout: &Layout, by_line: bool) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    // The paragraph a cell-1 line continues, and the heading a centered
    // line continues.
    let mut paragraph: Option<usize> = None;
    let mut heading: Option<usize>;
    let mut print: Option<String> = None;
    let width = layout.width;
    for (p, page) in layout.pages.iter().enumerate() {
        if let Some(n) = &layout.top_print[p]
            && print.as_ref() != Some(n)
        {
            blocks.push(Block::new(Kind::PrintPage(n.clone()), p, ""));
            print = Some(n.clone());
            paragraph = None;
        }
        heading = None;
        let mut after_blank = true;
        // Blank lines above the first line and below the last (a page's
        // padding, and the line its number was on) end nothing: a
        // paragraph goes on over the page.
        let first = page.iter().position(|l| !l.trim().is_empty());
        let last = page.iter().rposition(|l| !l.trim().is_empty());
        let lines = match (first, last) {
            (Some(f), Some(l)) => &page[f..=l],
            _ => &page[..0],
        };
        for line in lines {
            let text = line.trim();
            if text.is_empty() {
                (paragraph, heading, after_blank) = (None, None, true);
                continue;
            }
            // A print page change ends the paragraph it falls in.
            if let Some(n) = print_indicator(line) {
                if let Some(n) = n {
                    blocks.push(Block::new(Kind::PrintPage(n.clone()), p, ""));
                    print = Some(n);
                }
                (paragraph, heading, after_blank) = (None, None, true);
                continue;
            }
            let indent = line.len() - line.trim_start().len();
            let centered = indent >= 3
                && text.len() + 4 <= width
                && indent.abs_diff((width - text.len()) / 2) <= 1;
            if by_line {
                blocks.push(Block::new(Kind::Paragraph, p, text));
            } else if centered {
                match heading {
                    Some(h) if !after_blank => blocks[h].push_line(p, text),
                    _ => {
                        blocks.push(Block::new(Kind::Heading(1), p, text));
                        heading = Some(blocks.len() - 1);
                    }
                }
                paragraph = None;
            } else if indent == 4 && after_blank {
                blocks.push(Block::new(Kind::Heading(2), p, text));
                (paragraph, heading) = (None, None);
            } else if let (0, Some(i)) = (indent, paragraph) {
                blocks[i].push_line(p, text);
            } else {
                blocks.push(Block::new(Kind::Paragraph, p, text));
                (paragraph, heading) = (Some(blocks.len() - 1), None);
            }
            after_blank = false;
        }
    }
    blocks
}

/// Reads braille ASCII into a document, back-translating each piece of
/// text with `translate` (one input per piece, in order); when it fails,
/// the document shows the braille as Unicode braille patterns.
fn read(ascii: &str, translate: impl FnOnce(&[String]) -> Result<Vec<String>, String>) -> Document {
    let layout = layout(ascii);
    let mut blocks = blocks(&layout, false);
    let by_line = blocks
        .iter()
        .any(|b| b.kind == Kind::Paragraph && b.lines > MAX_PARAGRAPH_LINES);
    if by_line {
        blocks = self::blocks(&layout, true);
    }
    let inputs: Vec<String> = blocks
        .iter()
        .flat_map(|b| &b.pieces)
        .map(|piece| piece.braille.clone())
        .collect();
    let translated = match translate(&inputs) {
        Ok(t) if t.len() == inputs.len() => Some(t),
        Ok(_) => None,
        Err(e) => {
            log::warn!("braille file shown as braille: {e}");
            None
        }
    };
    let mut meta = textweaver_text::DocumentMeta::default();
    let mut b = Builder::with_capacity(ascii.len());
    let mut page: Option<(usize, OpenId)> = None;
    let mut n = 0;
    for block in &blocks {
        let marker = match &block.kind {
            Kind::Heading(level) => {
                Marker::new(MarkerKind::Heading, CharRange::empty(0)).with_level(*level)
            }
            _ => Marker::new(MarkerKind::Paragraph, CharRange::empty(0)),
        };
        b.paragraph_break();
        let id = b.open(marker);
        for piece in &block.pieces {
            if page.is_none_or(|(p, _)| p != piece.page) {
                if let Some((_, open)) = page.take() {
                    b.close(open);
                }
                let label = layout.labels[piece.page]
                    .clone()
                    .unwrap_or_else(|| (piece.page + 1).to_string());
                let m = Marker::new(MarkerKind::PageBreak, CharRange::empty(0))
                    .with_label(label)
                    .with_reference((piece.page + 1).to_string());
                page = Some((piece.page, b.open(m)));
            }
            let text = match (&block.kind, &translated) {
                (Kind::PrintPage(number), _) => format!("Print page {number}"),
                (_, Some(t)) => t[n].clone(),
                (_, None) => to_unicode(&piece.braille),
            };
            n += 1;
            if !piece.glued {
                b.space();
            }
            b.text(&text);
        }
        b.close(id);
    }
    if let Some((_, open)) = page.take() {
        b.close(open);
    }
    add_warning(&mut meta, REBUILT_WARNING);
    if by_line {
        add_warning(&mut meta, LINES_WARNING);
    }
    if translated.is_none() {
        add_warning(&mut meta, NO_LIBLOUIS_WARNING);
        meta.properties
            .insert(TRANSLATION_PROPERTY.to_owned(), UNTRANSLATED.to_owned());
    }
    meta.properties
        .insert("brf.pages".to_owned(), layout.pages.len().to_string());
    let (text, markers) = b.finish();
    Document::new(meta, Rope::from_str(&text), markers)
}

/// Back-translates each braille ASCII text (one per line) through
/// liblouis's `lou_translate`, with the display table the BRF writer uses.
/// It is looked for in the components folder first, then on the PATH
/// (`textweaver_store::lou_translate`).
fn liblouis(texts: &[String], code: BrfCode) -> Result<Vec<String>, String> {
    use std::io::Write;
    use std::process::Stdio;

    if texts.is_empty() {
        return Ok(Vec::new());
    }
    let tables = format!("en-us-brf.dis,{}", code.table());
    let mut child = textweaver_store::lou_translate()
        .args(["--backward", &tables])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("lou_translate could not start: {e}"))?;
    let mut input = String::new();
    for t in texts {
        input.push_str(&input_line(t));
        input.push('\n');
    }
    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
    let output = child
        .wait_with_output()
        .map_err(|e| format!("lou_translate failed: {e}"))?;
    let _ = writer.join();
    if !output.status.success() {
        return Err(format!(
            "lou_translate failed: {}",
            textweaver_core::process::decode_output(&output.stderr).trim()
        ));
    }
    let text = textweaver_core::process::decode_output(&output.stdout);
    let lines: Vec<String> = text.lines().map(str::to_owned).collect();
    if lines.len() < texts.len() {
        return Err("lou_translate returned fewer lines than it was given".to_owned());
    }
    Ok(lines.into_iter().take(texts.len()).collect())
}

/// One braille text as one line of `lou_translate`'s input: it reads
/// backslash escapes, and the backslash is a braille cell (dots 1-2-5-6,
/// "ou"), so it is doubled, as the BRF writer does.
fn input_line(braille: &str) -> String {
    braille.replace('\\', "\\\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in for liblouis: each text in lowercase, in angle brackets.
    fn stand_in(texts: &[String]) -> Result<Vec<String>, String> {
        Ok(texts
            .iter()
            .map(|t| format!("<{}>", t.to_lowercase()))
            .collect())
    }

    fn missing(_: &[String]) -> Result<Vec<String>, String> {
        Err("lou_translate could not start".to_owned())
    }

    fn page(lines: &[&str]) -> String {
        lines.join("\r\n")
    }

    fn markers(doc: &Document, kind: MarkerKind) -> Vec<(String, Option<String>)> {
        doc.markers()
            .iter()
            .filter(|m| m.kind == kind)
            .map(|m| (doc.slice(m.range), m.label.clone()))
            .collect()
    }

    /// Two 40-cell pages: a centered two-line heading, a paragraph with a
    /// runover, a print page change, a cell-5 heading, a paragraph that
    /// goes on over the page with a divided word, and braille page numbers.
    fn book() -> String {
        let p1 = page(&[
            "               ,*APT] ,\"O",
            "                ,! ,RIV]",
            "  ,! RIV] RAN PA/ ! OLD MILL1",
            "& ! *N FOLL[$ X",
            "-------------------------------------#B",
            "",
            "    ,ROAD",
            "  ,NOBODY 0 9 A HURRY4 ,! RIV] RAN TO",
            "! SE\"",
            "",
            "                                      #A",
        ]);
        let p2 = page(&[
            "A4",
            "",
            "lower case cells",
            "                                      #B",
        ]);
        format!("{p1}\r\n\x0C{p2}\r\n\x0C")
    }

    #[test]
    fn pages_headings_and_paragraphs_are_rebuilt() {
        let doc = read(&ascii_braille(book().as_bytes()).unwrap(), stand_in);
        assert_eq!(
            doc.text().to_string(),
            "<,*apt] ,\"o ,! ,riv]>\n\n<,! riv] ran pa/ ! old mill1 & ! *n foll[$ x>\n\n\
             Print page 2\n\n<,road>\n\n<,nobody 0 9 a hurry4 ,! riv] ran to ! se><a4>\n\n\
             <lower case cells>"
        );
        let headings = markers(&doc, MarkerKind::Heading);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[1].0, "<,road>");
        let pages = markers(&doc, MarkerKind::PageBreak);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].1.as_deref(), Some("1"));
        assert_eq!(
            pages[1],
            ("<a4>\n\n<lower case cells>".to_owned(), Some("2".into()))
        );
        assert_eq!(crate::warnings(&doc.meta), vec![REBUILT_WARNING.to_owned()]);
        assert_eq!(
            doc.meta.properties.get("brf.pages").map(String::as_str),
            Some("2")
        );
    }

    #[test]
    fn without_liblouis_the_braille_is_shown_as_braille() {
        let doc = read(&ascii_braille(book().as_bytes()).unwrap(), missing);
        let text = doc.text().to_string();
        assert!(
            text.starts_with("\u{2820}\u{2821}\u{2801}\u{280f}"),
            "{text}"
        );
        assert!(text.contains("Print page 2"), "{text}");
        assert_eq!(
            doc.meta
                .properties
                .get(TRANSLATION_PROPERTY)
                .map(String::as_str),
            Some(UNTRANSLATED)
        );
        assert!(crate::warnings(&doc.meta).contains(&NO_LIBLOUIS_WARNING.to_owned()));
    }

    #[test]
    fn running_heads_and_top_print_numbers_are_read_once() {
        let mut pages = Vec::new();
        for n in ["A", "B", "C", "D"] {
            pages.push(page(&[
                &format!("                ,! ,RIV]              #{n}"),
                "  ,TEXT4",
                &format!("                                      #{n}"),
            ]));
        }
        let doc = read(&pages.join("\n\u{0C}"), stand_in);
        assert_eq!(
            doc.text().to_string(),
            "Print page 1\n\n<,! ,riv]>\n\n<,text4>\n\nPrint page 2\n\n<,text4>\n\n\
             Print page 3\n\n<,text4>\n\nPrint page 4\n\n<,text4>"
        );
    }

    #[test]
    fn a_layout_without_paragraphs_is_read_line_by_line() {
        let lines: Vec<String> = (0..70)
            .map(|i| format!("LINE #{}", "A".repeat(i % 5 + 1)))
            .collect();
        let doc = read(&lines.join("\n"), stand_in);
        assert_eq!(markers(&doc, MarkerKind::Paragraph).len(), 70);
        assert!(crate::warnings(&doc.meta).contains(&LINES_WARNING.to_owned()));
    }

    #[test]
    fn braille_is_recognized_and_other_text_is_not() {
        assert_eq!(
            ascii_braille(b",! qk\r\n#ab").as_deref(),
            Some(",! QK\n#AB")
        );
        assert_eq!(
            ascii_braille("\u{2820}\u{282e}".as_bytes()).as_deref(),
            Some(",!")
        );
        assert_eq!(
            ascii_braille("caf\u{e9} na\u{ef}ve r\u{e9}sum\u{e9}".as_bytes()),
            None
        );
        assert_eq!(ascii_braille(b""), None);
        assert_eq!(page_token("#AJ").as_deref(), Some("10"));
        assert_eq!(page_token("P#C").as_deref(), Some("p3"));
        assert_eq!(page_token("#AK"), None);
        assert_eq!(page_token(",#A"), None);
    }

    #[test]
    fn original_pages_are_unicode_braille() {
        let pages = original_pages(book().as_bytes()).unwrap();
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[1][0], "\u{2801}\u{2832}");
        assert!(pages[0][0].starts_with("               \u{2820}"));
        assert_eq!(original_pages(&[0xff, 0xfe, 0, 1]), None);
    }

    #[test]
    fn backslashes_are_doubled_for_lou_translate() {
        assert_eq!(input_line("AB\\C"), "AB\\\\C");
    }
}
