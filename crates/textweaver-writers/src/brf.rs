//! Braille Ready Format: UEB braille in braille ASCII, laid out in pages.
//!
//! Layout follows the common BANA conventions (Braille Formats: Principles
//! of Print-to-Braille Transcription, 2016), simplified:
//!
//! - level 1 headings centered, level 2 in cell 5, deeper levels in cell 7,
//!   each kept on the same page as the line after it; a blank line before
//!   levels 1 and 2;
//! - paragraphs start in cell 3 with runovers in cell 1, no blank lines
//!   between paragraphs;
//! - list items in cell 1 (two cells deeper per nesting level) with
//!   runovers two cells in; bullets as the UEB bullet (`_4`), numbers as the
//!   source numbered them;
//! - tables in one of three formats ([`BrailleTableFormat`]): linear, one
//!   row per line with its cells separated by semicolons, runovers in
//!   cell 3; listed (Braille Formats 11.16), each row a cell-5 heading
//!   ("first column heading: row heading") after a blank line, then each
//!   entry on its own line after its column heading and a colon, in 1-3
//!   margins; or stairstep (11.18), each row's entries in 1-1, 3-3, 5-5
//!   and 7-7 margins, the column headings in a transcriber's note at the
//!   same steps, and a table of more than four columns listed instead;
//!   a blank line before and after every table (11.2.5d), a blank
//!   entry as three guide dots, and a row kept on one braille page when it
//!   fits. The listed format's transcriber's note is written once, before
//!   the first listed table;
//! - transcriber's notes in 7-5 margins between the transcriber's note
//!   indicators (`@.<`, `@.>`; UEB Rules 3.27);
//! - bold, italic and underlined print with the UEB typeform indicators
//!   ([`ueb`]); a heading wholly in one typeform leaves it out, since its
//!   placement already shows it (UEB Rules 9.1, the "CHAPTER 6" example);
//!   in grade 2 too, placed natively around liblouis's contractions;
//! - a capitals or typeform passage that goes on over paragraphs or list
//!   items in a row is opened again at the start of each and terminated
//!   once, at its end (UEB Rules 8.5.5 and 9.9.1); each heading is
//!   capitalized by itself (8.5.6), and a quotation, table, or heading
//!   ends the series;
//! - code lines as they are, runovers in cell 3; block quotes two cells in;
//! - a section break starts a new braille page; a print page break writes
//!   the print page change indicator (a line of dots 3-6 ending in the page
//!   number);
//! - braille page numbers at the right of each page's last line
//!   ([`BrailleOptions::page_numbers`](crate::BrailleOptions)), at least
//!   three blank cells from the text;
//! - words longer than a line are divided with the line continuation
//!   indicator (dot 5);
//! - lines end in CR LF, pages are separated by a form feed.
//!
//! Translation is UEB grade 1 by [`ueb`], or grade 2 through
//! liblouis when [`BrailleGrade::Two`](crate::BrailleGrade) is chosen and
//! the `liblouis` feature is on and `lou_translate` is installed.

use std::io::Write;

use textweaver_text::Document;

use crate::math::{BRAILLE_BREAK, BRAILLE_MATH, BRAILLE_NBSP};
use crate::model::{self, Block, Inline, List, Style, Table};
use crate::ueb::{self, BLANK_ENTRY, Join, Segment, Typeform};
use crate::{
    BrailleGrade, BrailleOptions, BrailleTableFormat, Format, WriteError, WriteOptions,
    WriteReport, Writer,
};

/// Writes BRF.
#[derive(Clone, Copy, Debug, Default)]
pub struct BrfWriter;

impl Writer for BrfWriter {
    fn format(&self) -> Format {
        Format::Brf
    }

    fn write(
        &self,
        doc: &Document,
        options: &WriteOptions,
        out: &mut dyn Write,
    ) -> Result<WriteReport, WriteError> {
        let mut report = WriteReport::default();
        let mut blocks = model::blocks(doc);
        // Math is written in the math code (Nemeth or UEB) through MathCAT,
        // or as it is read aloud ("x squared"), which braille spells out
        // readably (ADR-0036).
        let math = crate::math::MathBraille::new(&options.braille);
        crate::math::replace_spans(&mut blocks, &|f| math.text_for(f));
        if let Some(summary) = math.summary() {
            report.warn(summary);
        }
        let mut items = Vec::new();
        let mut flat = Flat {
            tables: options.braille.table_format,
            listed_noted: false,
            flow: false,
            out: &mut items,
            report: &mut report,
        };
        flat.blocks(&blocks, 0);
        let texts: Vec<(&str, Join)> = items.iter().filter_map(Item::text).collect();
        let translated = translate_with_math(&texts, &math, &options.braille, &mut report)?;
        let brf = layout(&items, translated, &options.braille);
        out.write_all(brf.as_bytes())?;
        Ok(report)
    }
}

/// A piece of the document to lay out, still in print.
#[derive(Clone, Debug, PartialEq)]
enum Item {
    Heading {
        level: u8,
        text: String,
    },
    /// Text wrapped with a first-line and a runover indent (in cells).
    Text {
        text: String,
        first: usize,
        runover: usize,
        blank_before: bool,
        /// Braille written before and after the translated text (the
        /// transcriber's note indicators).
        before: &'static str,
        after: &'static str,
        /// How the text goes on from the text before it: the lines of a
        /// paragraph, and paragraphs and list items in a row, carry
        /// capitals and typeform passages over (UEB Rules 8.5.5, 9.9.1).
        join: Join,
    },
    /// A blank line.
    Blank,
    /// Keep the next this many items (all `Text`) on one braille page when
    /// they fit on one.
    Keep(usize),
    /// A new braille page.
    NewPage,
    /// Print page change indicator with the print page label.
    PrintPage(Option<String>),
}

impl Item {
    /// The print to translate, and how it joins the text before it.
    /// Each heading is capitalized by itself (UEB Rules 8.5.6); a print
    /// page number does not divide the passage around it.
    fn text(&self) -> Option<(&str, Join)> {
        match self {
            Item::Heading { text, .. } => Some((text, Join::Fresh)),
            Item::Text { text, join, .. } => Some((text, *join)),
            Item::PrintPage(Some(label)) => Some((label, Join::Alone)),
            _ => None,
        }
    }
}

/// Text of inlines with line breaks kept as `\n`, and bold, italic and
/// underline marked for the translator ([`Typeform`]).
fn text_of(inlines: &[Inline]) -> String {
    let mut s = String::new();
    fn walk(v: &[Inline], s: &mut String) {
        for i in v {
            match i {
                Inline::Text(t) => s.push_str(t),
                Inline::LineBreak => s.push('\n'),
                Inline::Span(style, c) => match typeform(style) {
                    Some(form) => {
                        s.push(form.open());
                        walk(c, s);
                        s.push(form.close());
                    }
                    None => walk(c, s),
                },
            }
        }
    }
    walk(inlines, &mut s);
    s
}

fn typeform(style: &Style) -> Option<Typeform> {
    match style {
        Style::Bold => Some(Typeform::Bold),
        Style::Italic => Some(Typeform::Italic),
        Style::Underline => Some(Typeform::Underline),
        _ => None,
    }
}

/// Inline text on one line, spaces collapsed, typeforms marked.
fn line_of(inlines: &[Inline]) -> String {
    model::collapse_ws(&text_of(inlines))
}

/// A heading's text. A typeform over the whole heading is left out: the
/// heading's placement already shows it (UEB Rules 9.1, whose "CHAPTER 6"
/// example ignores the heading's change of typeform).
fn heading_of(inlines: &[Inline]) -> String {
    let mut content = inlines;
    loop {
        let visible: Vec<&Inline> = content
            .iter()
            .filter(|i| !matches!(i, Inline::Text(t) if t.trim().is_empty()))
            .collect();
        match visible.as_slice() {
            [Inline::Span(style, inner)] if typeform(style).is_some() => content = inner,
            _ => break,
        }
    }
    line_of(content)
}

fn text_item(text: String, first: usize, runover: usize) -> Item {
    Item::Text {
        text,
        first,
        runover,
        blank_before: false,
        before: "",
        after: "",
        join: Join::Fresh,
    }
}

/// The transcriber's note indicators (UEB Rules 3.27).
const NOTE_OPEN: &str = "@.<";
const NOTE_CLOSE: &str = "@.>";

/// A transcriber's note: lines of print, each with its margins, the
/// opening indicator before the first and the closing one after the last.
fn note_items(lines: Vec<(String, usize, usize)>) -> Vec<Item> {
    let n = lines.len();
    lines
        .into_iter()
        .enumerate()
        .map(|(k, (text, first, runover))| Item::Text {
            text,
            first,
            runover,
            blank_before: false,
            before: if k == 0 { NOTE_OPEN } else { "" },
            after: if k + 1 == n { NOTE_CLOSE } else { "" },
            join: Join::Fresh,
        })
        .collect()
}

/// Flattens blocks into items to lay out.
struct Flat<'a> {
    tables: BrailleTableFormat,
    /// The listed format's transcriber's note is written.
    listed_noted: bool,
    /// The last item pushed is a paragraph's or a list item's text, so the
    /// next one goes on from it as the next text element.
    flow: bool,
    out: &'a mut Vec<Item>,
    report: &'a mut WriteReport,
}

impl Flat<'_> {
    /// Pushes an item that is not running text: it ends a series of
    /// paragraphs and list items. A print page change does not.
    fn push(&mut self, item: Item) {
        if !matches!(item, Item::PrintPage(_)) {
            self.flow = false;
        }
        self.out.push(item);
    }

    /// Pushes a paragraph's or list item's line: the first line goes on
    /// from running text right before it as the next element, and each
    /// later line of the same element as its next line.
    fn push_flow(&mut self, mut item: Item, first_line: bool) {
        if let Item::Text { join, .. } = &mut item {
            *join = match (first_line, self.flow) {
                (false, _) => Join::Line,
                (true, true) => Join::Element,
                (true, false) => Join::Fresh,
            };
        }
        self.out.push(item);
        self.flow = true;
    }

    fn blocks(&mut self, blocks: &[Block], indent: usize) {
        for b in blocks {
            match b {
                Block::Heading { level, content } => self.push(Item::Heading {
                    level: *level,
                    text: heading_of(content),
                }),
                Block::Paragraph(content) => {
                    // Lines after a line break start in the runover cell.
                    for (n, line) in text_of(content).split('\n').enumerate() {
                        let first = if n == 0 { indent + 2 } else { indent };
                        self.push_flow(text_item(line.to_owned(), first, indent), n == 0);
                    }
                }
                Block::List(list) => self.list(list, indent),
                Block::Table(t) => self.table(t, indent),
                Block::Code { text, .. } => {
                    for line in text.split('\n') {
                        // Keep the code's own indentation (a tab is four
                        // cells).
                        let lead: usize = line
                            .chars()
                            .take_while(|c| matches!(c, ' ' | '\t'))
                            .map(|c| if c == '\t' { 4 } else { 1 })
                            .sum();
                        self.push(text_item(
                            line.trim_start().to_owned(),
                            indent + lead,
                            indent + 2,
                        ));
                    }
                }
                Block::Quote(inner) => {
                    // A quotation is displayed apart: its passages neither
                    // go on from the text before it nor into the text
                    // after it.
                    self.flow = false;
                    let start = self.out.len();
                    self.blocks(inner, indent + 2);
                    if let Some(Item::Text { blank_before, .. }) = self.out.get_mut(start) {
                        *blank_before = true;
                    }
                    self.flow = false;
                }
                Block::Figure(img) => {
                    if !img.alt.is_empty() {
                        self.push(text_item(img.alt.clone(), indent + 2, indent));
                    }
                }
                Block::Footnote { id, content } => {
                    let body = line_of(content);
                    let text = if ueb::strip_marks(&body).starts_with(&format!("[{id}]")) {
                        body
                    } else {
                        format!("[{id}] {body}")
                    };
                    self.push(text_item(text, indent, indent + 2));
                }
                Block::SectionBreak { .. } => self.push(Item::NewPage),
                // A horizontal rule: a line of hyphens on its own line.
                Block::Rule => {
                    let mut item = text_item("-".repeat(12), indent, indent);
                    if let Item::Text { blank_before, .. } = &mut item {
                        *blank_before = true;
                    }
                    self.push(item);
                }
                Block::PageBreak { label } => self.push(Item::PrintPage(label.clone())),
            }
        }
    }

    fn list(&mut self, list: &List, indent: usize) {
        for (n, item) in list.items.iter().enumerate() {
            let marker = if list.ordered {
                item.label
                    .clone()
                    .unwrap_or_else(|| format!("{}.", list.start + n as u64))
            } else {
                "\u{2022}".to_owned()
            };
            let mut rest: &[Block] = &item.blocks;
            let first_text = match rest.first() {
                Some(Block::Paragraph(content)) => {
                    rest = &rest[1..];
                    line_of(content)
                }
                _ => String::new(),
            };
            let text = if first_text.is_empty() {
                marker
            } else {
                format!("{marker} {first_text}")
            };
            self.push_flow(text_item(text, indent, indent + 2), true);
            self.blocks(rest, indent + 2);
        }
    }

    /// A table in the chosen format (Braille Formats 2016, section 11).
    fn table(&mut self, table: &Table, indent: usize) {
        let rows: Vec<Vec<String>> = table
            .rows
            .iter()
            .map(|r| r.cells.iter().map(|c| line_of(c)).collect())
            .collect();
        let columns = table.columns().max(1);
        let mut format = self.tables;
        if format == BrailleTableFormat::Stairstep && columns > 4 {
            // 11.18: four columns at most; larger tables are listed.
            self.report.warn(
                "A table has more than four columns, so it is in listed format: \
                 the stairstep format takes four at most.",
            );
            format = BrailleTableFormat::Listed;
        }
        // A blank line before and after a table that is not boxed
        // (Braille Formats 2016, 11.2.5d).
        self.push(Item::Blank);
        if format == BrailleTableFormat::Linear {
            if let Some(c) = &table.caption {
                self.push(text_item(c.clone(), indent + 2, indent));
            }
            for row in &rows {
                // A row is not divided between braille pages (11.17).
                self.push(Item::Keep(1));
                self.push(text_item(row.join("; "), indent, indent + 2));
            }
            self.push(Item::Blank);
            return;
        }
        let (headings, body): (Option<&Vec<String>>, &[Vec<String>]) = if table.has_header() {
            (rows.first(), &rows[1..])
        } else {
            (None, &rows[..])
        };
        let blanks = body
            .iter()
            .any(|r| (0..columns).any(|j| entry(r, j).starts_with(BLANK_ENTRY)));
        let guide = if blanks {
            " Three guide dots mark an empty entry."
        } else {
            ""
        };
        if let Some(c) = &table.caption {
            self.push(text_item(c.clone(), indent + 2, indent));
        }
        if format == BrailleTableFormat::Listed {
            self.listed(headings, body, columns, indent, guide);
        } else {
            self.stairstep(headings, body, columns, indent, guide);
        }
        self.push(Item::Blank);
    }

    /// The listed format (11.16).
    fn listed(
        &mut self,
        headings: Option<&Vec<String>>,
        body: &[Vec<String>],
        columns: usize,
        indent: usize,
        guide: &str,
    ) {
        if !self.listed_noted {
            // One note for the document: every listed table is laid out
            // the same way.
            self.listed_noted = true;
            let note = format!(
                "Tables listed: each row starts with its heading in cell 5, \
                 and each entry follows its column heading and a colon.{guide}"
            );
            for item in note_items(vec![(note, indent + 6, indent + 4)]) {
                self.push(item);
            }
        }
        let heading = |j: usize| -> Option<&str> {
            headings
                .and_then(|h| h.get(j))
                .map(String::as_str)
                .filter(|h| !ueb::strip_marks(h).trim().is_empty())
        };
        for row in body {
            // A blank line before each row.
            self.push(Item::Blank);
            self.push(Item::Keep(columns));
            for j in 0..columns {
                let text = match heading(j) {
                    Some(h) => format!("{h}: {}", entry(row, j)),
                    None => entry(row, j),
                };
                // The first column heading and the row heading are a
                // cell-5 heading; each other entry is in 1-3 margins.
                let (first, runover) = if j == 0 { (4, 4) } else { (0, 2) };
                self.push(text_item(text, indent + first, indent + runover));
            }
        }
    }

    /// The stairstep format (11.18).
    fn stairstep(
        &mut self,
        headings: Option<&Vec<String>>,
        body: &[Vec<String>],
        columns: usize,
        indent: usize,
        guide: &str,
    ) {
        let intro = format!(
            "Table in stairstep format: each entry starts two cells to the right \
             of the one before it.{guide}{}",
            if headings.is_some() {
                " Column headings:"
            } else {
                ""
            }
        );
        // The column headings, in the note, at the steps of their entries.
        let mut note = vec![(intro, indent + 6, indent + 4)];
        if let Some(h) = headings {
            for j in 0..columns {
                note.push((entry(h, j), indent + 2 * j, indent + 2 * j));
            }
        }
        for item in note_items(note) {
            self.push(item);
        }
        for row in body {
            self.push(Item::Keep(columns));
            for j in 0..columns {
                let margin = indent + 2 * j;
                self.push(text_item(entry(row, j), margin, margin));
            }
        }
    }
}

/// A table entry, or three guide dots for a blank one (11.16, 11.18).
fn entry(row: &[String], j: usize) -> String {
    match row.get(j) {
        Some(c) if !ueb::strip_marks(c).trim().is_empty() => c.clone(),
        _ => BLANK_ENTRY.to_string(),
    }
}

/// Translates every text around its formulas' placeholders, then splices
/// in each formula's braille. Texts without formulas are translated whole,
/// as before, each joined to the text before it as its [`Join`] says. The
/// prose around a formula is translated in pieces, each alone, and the
/// text after a formula's text starts afresh: no passage runs through
/// math.
fn translate_with_math(
    texts: &[(&str, Join)],
    math: &crate::math::MathBraille,
    options: &BrailleOptions,
    report: &mut WriteReport,
) -> Result<Vec<String>, WriteError> {
    use crate::math::{Piece, split_placeholders};
    // Each text as pieces; `None` for a whole text translated as it is.
    let plans: Vec<Option<Vec<Piece<'_>>>> = texts
        .iter()
        .map(|(t, _)| {
            let pieces = split_placeholders(t);
            pieces
                .iter()
                .any(|p| matches!(p, Piece::Math(_)))
                .then_some(pieces)
        })
        .collect();
    let mut prose: Vec<(&str, Join)> = Vec::with_capacity(texts.len());
    let mut after_math = false;
    for (&(t, join), plan) in texts.iter().zip(&plans) {
        match plan {
            None => {
                let join = match join {
                    Join::Line | Join::Element if after_math => Join::Fresh,
                    other => other,
                };
                if join != Join::Alone {
                    after_math = false;
                }
                prose.push((t, join));
            }
            Some(pieces) => {
                after_math = true;
                prose.extend(pieces.iter().filter_map(|p| match p {
                    Piece::Text(s) if !s.trim().is_empty() => Some((s.trim(), Join::Alone)),
                    _ => None,
                }));
            }
        }
    }
    let mut done = translate_all(&prose, options, report)?.into_iter();
    let mut out = Vec::with_capacity(texts.len());
    for plan in &plans {
        match plan {
            None => out.push(done.next().unwrap_or_default()),
            Some(pieces) => {
                let mut s = String::new();
                for p in pieces {
                    match p {
                        Piece::Math(placeholder) => s.push_str(placeholder),
                        Piece::Text(t) if t.trim().is_empty() => push_space(&mut s, t),
                        Piece::Text(t) => {
                            push_space(&mut s, &t[..t.len() - t.trim_start().len()]);
                            s.push_str(&done.next().unwrap_or_default());
                            push_space(&mut s, &t[t.trim_end().len()..]);
                        }
                    }
                }
                out.push(math.splice(&s));
            }
        }
    }
    Ok(out)
}

/// Pushes the whitespace `ws` as one space, or a line break if it has one.
fn push_space(s: &mut String, ws: &str) {
    if ws.contains('\n') {
        s.push('\n');
    } else if !ws.is_empty() {
        s.push(' ');
    }
}

/// Translates every text, natively or through liblouis.
fn translate_all(
    texts: &[(&str, Join)],
    options: &BrailleOptions,
    report: &mut WriteReport,
) -> Result<Vec<String>, WriteError> {
    if options.grade == BrailleGrade::Two {
        let contract = |pieces: &[&str]| louis::translate(pieces, &options.table);
        match louis_with_typeforms(texts, &contract) {
            Ok(v) => return Ok(v),
            Err(why) => report.warn(format!(
                "Contracted braille is not available ({why}), so the file is in uncontracted braille."
            )),
        }
    }
    let mut unsupported: Vec<char> = Vec::new();
    let v = ueb::translate_series(texts)
        .into_iter()
        .map(|tr| {
            for c in tr.unsupported {
                if !unsupported.contains(&c) {
                    unsupported.push(c);
                }
            }
            tr.braille
        })
        .collect();
    if !unsupported.is_empty() {
        let list: Vec<String> = unsupported
            .iter()
            .take(10)
            .map(|c| format!("U+{:04X}", *c as u32))
            .collect();
        report.warn(format!(
            "{} characters have no braille symbol and were left out: {}.",
            unsupported.len(),
            list.join(", ")
        ));
    }
    Ok(v)
}

/// A translator of plain print into contracted braille ASCII, one result
/// per text, in order: liblouis, or a stand-in in tests.
type Contract<'a> = dyn Fn(&[&str]) -> Result<Vec<String>, String> + 'a;

/// Grade 2 through liblouis, with the UEB typeform indicators.
/// `lou_translate` takes no typeforms and knows neither the typeform marks
/// nor the blank entry, so [`ueb::typeform_segments`] places the
/// indicators by the same rules as grade 1 (The Rules of Unified English
/// Braille, 2013, 9.2 to 9.9, a passage going on over paragraphs
/// included), liblouis contracts each stretch of print between them by
/// itself, and the indicators and each blank entry's guide dots are put
/// back between the contracted pieces. Capitals stay liblouis's own.
fn louis_with_typeforms(
    texts: &[(&str, Join)],
    contract: &Contract<'_>,
) -> Result<Vec<String>, String> {
    let segments = ueb::typeform_segments(texts);
    let prints: Vec<&str> = segments
        .iter()
        .flatten()
        .filter_map(|piece| match piece {
            Segment::Print(p) if !p.trim().is_empty() => Some(p.trim()),
            _ => None,
        })
        .collect();
    let mut done = contract(&prints)?.into_iter();
    Ok(segments
        .iter()
        .map(|pieces| {
            let mut s = String::new();
            for piece in pieces {
                match piece {
                    Segment::Braille(cells) => s.push_str(cells),
                    Segment::Print(p) if p.trim().is_empty() => push_space(&mut s, p),
                    Segment::Print(p) => {
                        push_space(&mut s, &p[..p.len() - p.trim_start().len()]);
                        s.push_str(done.next().unwrap_or_default().trim());
                        push_space(&mut s, &p[p.trim_end().len()..]);
                    }
                }
            }
            s
        })
        .collect())
}

/// Braille pages under construction.
struct Pages {
    width: usize,
    /// Text lines per page (the last line is reserved for the page number).
    text_lines: usize,
    lines_per_page: usize,
    numbers: bool,
    pages: Vec<Vec<String>>,
    current: Vec<String>,
}

impl Pages {
    fn new(options: &BrailleOptions) -> Self {
        let width = options.cells_per_line.max(12);
        let lines_per_page = options.lines_per_page.max(3);
        let text_lines = if options.page_numbers {
            lines_per_page - 1
        } else {
            lines_per_page
        };
        Pages {
            width,
            text_lines,
            lines_per_page,
            numbers: options.page_numbers,
            pages: Vec::new(),
            current: Vec::new(),
        }
    }

    fn remaining(&self) -> usize {
        self.text_lines - self.current.len()
    }

    fn new_page(&mut self) {
        if !self.current.is_empty() {
            let page = std::mem::take(&mut self.current);
            self.pages.push(page);
        }
    }

    fn line(&mut self, s: String) {
        if self.current.len() >= self.text_lines {
            self.new_page();
        }
        self.current.push(s);
    }

    /// A blank line, except at the top of a page or after another blank.
    fn blank(&mut self) {
        if self.current.last().is_some_and(|l| !l.is_empty()) && self.remaining() > 1 {
            self.current.push(String::new());
        }
    }

    fn keep(&mut self, n: usize) {
        if n <= self.text_lines && self.remaining() < n {
            self.new_page();
        }
    }

    fn finish(mut self) -> String {
        self.new_page();
        if self.pages.is_empty() {
            self.pages.push(Vec::new());
        }
        let mut out = String::new();
        for (n, mut page) in self.pages.into_iter().enumerate() {
            if self.numbers {
                page.resize(self.lines_per_page - 1, String::new());
                let num = ueb::translate(&(n + 1).to_string()).braille;
                let pad = self.width.saturating_sub(num.len());
                page.push(format!("{}{num}", " ".repeat(pad)));
            }
            if n > 0 {
                out.push('\u{0C}');
            }
            for line in page {
                out.push_str(line.trim_end());
                out.push_str("\r\n");
            }
        }
        out
    }
}

/// Cells `s` takes on a line: the math markers take none.
fn cells(s: &str) -> usize {
    s.chars()
        .filter(|&c| c != BRAILLE_BREAK && c != BRAILLE_MATH)
        .count()
}

/// The byte index just after the first `n` cells of `s`.
fn cell_index(s: &str, n: usize) -> usize {
    let mut seen = 0;
    for (i, c) in s.char_indices() {
        if c != BRAILLE_BREAK && c != BRAILLE_MATH {
            if seen == n {
                return i;
            }
            seen += 1;
        }
    }
    s.len()
}

/// A line as written: bound spaces become spaces, markers go.
fn finish_line(s: &str) -> String {
    s.chars()
        .filter(|&c| c != BRAILLE_BREAK && c != BRAILLE_MATH)
        .map(|c| if c == BRAILLE_NBSP { ' ' } else { c })
        .collect()
}

/// Where to divide a word that does not fit in `room` cells: the byte
/// index, and whether the line continuation indicator (dot 5) goes after
/// the first part.
///
/// Print words divide after a hyphen when one fits, else with the
/// indicator. Math divides before an operation sign when one fits; failing
/// that, at the room's end but never right after an indicator or a bound
/// space, and without the dot-5 indicator, which is a Nemeth symbol of its
/// own (the baseline indicator).
fn divide(word: &str, room: usize) -> Option<(usize, bool)> {
    let limit = cell_index(word, room);
    if word.contains(BRAILLE_MATH) {
        let head = &word[..limit];
        if let Some(b) = head.rfind(BRAILLE_BREAK).filter(|&b| cells(&head[..b]) > 0) {
            return Some((b, false));
        }
        // Indicators stay with what follows them, and an operation sign
        // starts the next line rather than ending this one.
        const KEEP_WITH_NEXT: &str = ",;^\".#_@+-";
        // A bound space holds both of its neighbors: no cut on either side.
        let mut cut = limit;
        while cut > 0 {
            let prev = word[..cut].chars().next_back()?;
            let next = word[cut..]
                .chars()
                .find(|&c| c != BRAILLE_BREAK && c != BRAILLE_MATH);
            if prev == BRAILLE_NBSP
                || KEEP_WITH_NEXT.contains(prev)
                || prev == BRAILLE_MATH
                || prev == BRAILLE_BREAK
                || next == Some(BRAILLE_NBSP)
            {
                cut -= prev.len_utf8();
            } else {
                break;
            }
        }
        return (cells(&word[..cut]) > 0).then_some((cut, false));
    }
    let cut = word[..limit]
        .rfind('-')
        .filter(|&h| h > 0)
        .map(|h| h + 1)
        .unwrap_or(cell_index(word, room.saturating_sub(1)));
    Some((cut, !word[..cut].ends_with('-')))
}

/// Wraps braille into lines of `width` cells with the given indents.
///
/// Math from [`MathBraille`](crate::math::MathBraille) carries markers: a
/// bound space ([`BRAILLE_NBSP`]) that never ends a line, so a Nemeth
/// switch indicator stays with the math beside it; division points before
/// operation signs ([`BRAILLE_BREAK`]); and the math mark
/// ([`BRAILLE_MATH`]). The lines returned have none of them.
fn wrap(braille: &str, width: usize, first: usize, runover: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = " ".repeat(first.min(width / 2));
    let mut empty = true;
    let runover_pad = " ".repeat(runover.min(width / 2));
    for word in braille.split(' ').filter(|w| cells(w) > 0) {
        let mut word = word.to_owned();
        loop {
            let need = if empty {
                cells(&word)
            } else {
                cells(&word) + 1
            };
            if cells(&line) + need <= width {
                if !empty {
                    line.push(' ');
                }
                line.push_str(&word);
                empty = false;
                break;
            }
            let room = width.saturating_sub(cells(&line) + usize::from(!empty));
            // Move the word down when it would fit on a fresh line, else
            // divide it.
            let fresh = width - runover_pad.len();
            if !empty && cells(&word) <= fresh {
                lines.push(std::mem::replace(&mut line, runover_pad.clone()));
                empty = true;
                continue;
            }
            let division = if room >= 3 { divide(&word, room) } else { None };
            // On an empty line something must be written, so the loop ends.
            let division =
                division.or_else(|| empty.then(|| (cell_index(&word, room.max(1)), false)));
            if let Some((cut, indicator)) = division {
                let (head, tail) = word.split_at(cut);
                if !empty {
                    line.push(' ');
                }
                line.push_str(head);
                if indicator {
                    line.push('"');
                }
                let math = head.contains(BRAILLE_MATH);
                let mut rest = tail.trim_start_matches(BRAILLE_BREAK).to_owned();
                if math && !rest.starts_with(BRAILLE_MATH) {
                    rest.insert(0, BRAILLE_MATH);
                }
                word = rest;
            }
            lines.push(std::mem::replace(&mut line, runover_pad.clone()));
            empty = true;
        }
    }
    if !empty || lines.is_empty() {
        lines.push(line);
    }
    lines.iter().map(|l| finish_line(l)).collect()
}

fn layout(items: &[Item], translated: Vec<String>, options: &BrailleOptions) -> String {
    let mut pages = Pages::new(options);
    let width = pages.width;
    // The braille of each item with text, for looking ahead.
    let mut ahead: Vec<Option<String>> = Vec::with_capacity(items.len());
    {
        let mut t = translated.iter();
        for item in items {
            ahead.push(item.text().map(|_| t.next().cloned().unwrap_or_default()));
        }
    }
    let mut texts = translated.into_iter();
    for (at, item) in items.iter().enumerate() {
        match item {
            Item::Blank => pages.blank(),
            Item::Keep(n) => {
                let lines: usize = items[at + 1..]
                    .iter()
                    .zip(&ahead[at + 1..])
                    .take(*n)
                    .map(|(item, braille)| match (item, braille) {
                        (
                            Item::Text {
                                first,
                                runover,
                                before,
                                after,
                                ..
                            },
                            Some(b),
                        ) if !b.trim().is_empty() => {
                            wrap(&format!("{before}{b}{after}"), width, *first, *runover).len()
                        }
                        _ => 0,
                    })
                    .sum();
                pages.keep(lines);
            }
            Item::Heading { level, .. } => {
                let braille = texts.next().unwrap_or_default();
                let lines = match level {
                    1 => center(&braille, width),
                    2 => wrap(&braille, width, 4, 4),
                    _ => wrap(&braille, width, 6, 6),
                };
                if *level <= 2 {
                    pages.blank();
                }
                // Keep the heading with the line after it.
                pages.keep(lines.len() + 1);
                for l in lines {
                    pages.line(l);
                }
                if *level == 1 {
                    pages.blank();
                }
            }
            Item::Text {
                first,
                runover,
                blank_before,
                before,
                after,
                ..
            } => {
                let braille = texts.next().unwrap_or_default();
                if braille.trim().is_empty() {
                    continue;
                }
                if *blank_before {
                    pages.blank();
                }
                let braille = format!("{before}{braille}{after}");
                for l in wrap(&braille, width, *first, *runover) {
                    pages.line(l);
                }
            }
            Item::NewPage => pages.new_page(),
            Item::PrintPage(label) => {
                let num = if label.is_some() {
                    texts.next().unwrap_or_default()
                } else {
                    String::new()
                };
                let dashes = width.saturating_sub(num.len()).max(3);
                pages.line(format!("{}{num}", "-".repeat(dashes)));
            }
        }
    }
    pages.finish()
}

fn center(braille: &str, width: usize) -> Vec<String> {
    // Centered headings leave at least three cells on each side.
    let inner = width.saturating_sub(6).max(6);
    wrap(braille, inner, 0, 0)
        .into_iter()
        .map(|l| {
            let l = l.trim().to_owned();
            let pad = (width.saturating_sub(l.len())) / 2;
            format!("{}{l}", " ".repeat(pad))
        })
        .collect()
}

/// Grade 2 through liblouis's `lou_translate`.
mod louis {
    /// One text as one line of `lou_translate`'s input.
    ///
    /// Line breaks inside the text become spaces. `lou_translate` reads
    /// backslash escapes in its input (`\n`, `\x41`, and so on), so a
    /// print backslash is doubled: unescaped, `C:\data` came back as an
    /// empty line (an invalid escape) and its paragraph was lost, and
    /// `\n` in a document became a line break.
    #[cfg(any(feature = "liblouis", test))]
    pub(super) fn input_line(text: &str) -> String {
        text.replace(['\n', '\r'], " ").replace('\\', "\\\\")
    }

    /// Translates each text (one per line) with `table`.
    #[cfg(feature = "liblouis")]
    pub(super) fn translate(texts: &[&str], table: &str) -> Result<Vec<String>, String> {
        use std::io::Write;
        use std::process::Stdio;

        let tables = format!("en-us-brf.dis,{table}");
        let mut child = textweaver_core::process::command("lou_translate")
            .args(["--forward", &tables])
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
        let lines: Vec<String> = text
            .lines()
            .map(|l| crate::ueb::from_unicode(l).to_ascii_uppercase())
            .collect();
        if lines.len() < texts.len() {
            return Err("lou_translate returned fewer lines than it was given".to_owned());
        }
        Ok(lines.into_iter().take(texts.len()).collect())
    }

    /// Without the `liblouis` feature there is no grade 2.
    #[cfg(not(feature = "liblouis"))]
    pub(super) fn translate(_texts: &[&str], _table: &str) -> Result<Vec<String>, String> {
        Err("this build has no liblouis support".to_owned())
    }

    /// True when `lou_translate` runs.
    #[cfg(all(test, feature = "liblouis"))]
    pub(super) fn available() -> bool {
        textweaver_core::process::command("lou_translate")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    }
}

#[cfg(test)]
mod tests {
    use ropey::Rope;
    use textweaver_core::{CharRange, MarkerKind};
    use textweaver_text::{DocumentMeta, Marker};

    use super::*;

    fn brf(doc: &Document, options: &WriteOptions) -> (String, WriteReport) {
        let mut out = Vec::new();
        let report = BrfWriter.write(doc, options, &mut out).unwrap();
        (String::from_utf8(out).unwrap(), report)
    }

    /// Math markers: a bound space never ends a line, division points take
    /// no cell and divide without dot 5, and no marker reaches the file.
    #[test]
    fn math_markers_bind_and_divide() {
        let math = format!(
            "{BRAILLE_MATH}_%{BRAILLE_NBSP}AAAA{BRAILLE_BREAK}+BBBB{BRAILLE_BREAK}+CCCC{BRAILLE_NBSP}_:"
        );
        let lines = wrap(&math, 12, 0, 0);
        assert_eq!(lines, vec!["_% AAAA+BBBB", "+CCCC _:"]);
        // Three cells are left after the print word: the opening indicator
        // would fit, but it moves down with the math it is bound to.
        let lines = wrap(&format!("XXXXXXXX {math}"), 12, 0, 0);
        assert_eq!(lines, vec!["XXXXXXXX", "_% AAAA+BBBB", "+CCCC _:"]);
        for l in &lines {
            assert!(l.len() <= 12, "{lines:?}");
            assert!(!l.ends_with("_%"), "{lines:?}");
            assert!(!l.starts_with("_:"), "{lines:?}");
            assert!(!l.contains(['\u{1}', '\u{2}', '\u{3}']), "{lines:?}");
        }
    }

    #[test]
    fn wraps_at_word_boundaries_with_indents() {
        let lines = wrap("AAAA BBBB CCCC DDDD", 10, 2, 0);
        assert_eq!(lines, vec!["  AAAA", "BBBB CCCC", "DDDD"]);
        // A word longer than a line is divided with dot 5.
        let lines = wrap("ABCDEFGHIJKLMNOP", 10, 0, 0);
        assert_eq!(lines, vec!["ABCDEFGHI\"", "JKLMNOP"]);
        // After a hyphen, no continuation indicator.
        let lines = wrap("XX ABCD-EFGHIJKL", 10, 0, 0);
        assert_eq!(lines, vec!["XX ABCD-", "EFGHIJKL"]);
        for l in wrap(
            "A BB CCC DDDD EEEEE FFFFFF GGGGGGG HHHHHHHHHHHHHHHHHHHHHH",
            10,
            2,
            2,
        ) {
            assert!(l.len() <= 10, "{l:?}");
        }
    }

    #[test]
    fn pages_have_geometry_and_numbers() {
        let text = (0..200)
            .map(|n| format!("word{n}"))
            .collect::<Vec<_>>()
            .join(" ");
        let doc = Document::from_plain_text(&text);
        let (out, report) = brf(&doc, &WriteOptions::default());
        assert!(report.warnings.is_empty());
        let pages: Vec<&str> = out.split('\u{0C}').collect();
        assert!(pages.len() >= 2);
        for (n, page) in pages.iter().enumerate() {
            let lines: Vec<&str> = page.split("\r\n").collect();
            // 25 lines each ending in CR LF.
            assert_eq!(lines.len(), 26, "page {n}");
            assert_eq!(lines[25], "");
            for l in &lines {
                assert!(l.len() <= 40, "{l:?}");
                assert!(l.chars().all(|c| (' '..='_').contains(&c)));
            }
            let num = ueb::translate(&(n + 1).to_string()).braille;
            assert!(lines[24].ends_with(&num), "{:?}", lines[24]);
            assert!(lines[24].trim_start() == num);
        }
        assert!(pages[0].starts_with("  WORD#J WORD#A WORD#B"));
    }

    #[test]
    fn configurable_geometry_without_numbers() {
        let doc = Document::from_plain_text("One two three four five six seven eight nine ten");
        let options = WriteOptions {
            braille: BrailleOptions {
                cells_per_line: 12,
                lines_per_page: 3,
                page_numbers: false,
                ..BrailleOptions::default()
            },
            ..WriteOptions::default()
        };
        let (out, _) = brf(&doc, &options);
        let pages: Vec<&str> = out.split('\u{0C}').collect();
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0], "  ,ONE TWO\r\nTHREE FOUR\r\nFIVE SIX\r\n");
        assert_eq!(pages[1], "SEVEN EIGHT\r\nNINE TEN\r\n");
    }

    #[test]
    fn structure_is_laid_out() {
        let text = "Title\n\nIntro text.\n\nFirst\nSecond\n\nA | B\n1 | 2\n\nMore";
        let markers = vec![
            Marker::new(MarkerKind::Heading, CharRange::new(0, 5)).with_level(1),
            Marker::new(MarkerKind::Paragraph, CharRange::new(7, 18)),
            Marker::new(MarkerKind::List, CharRange::new(20, 32)).with_level(1),
            Marker::new(MarkerKind::ListItem, CharRange::new(20, 25))
                .with_level(1)
                .with_label("1."),
            Marker::new(MarkerKind::ListItem, CharRange::new(26, 32))
                .with_level(1)
                .with_label("2."),
            Marker::new(MarkerKind::Table, CharRange::new(34, 45)),
            Marker::new(MarkerKind::TableRow, CharRange::new(34, 39))
                .with_label(textweaver_text::HEADER_ROW_LABEL),
            Marker::new(MarkerKind::TableRow, CharRange::new(40, 45)),
            Marker::new(MarkerKind::SectionBreak, CharRange::new(47, 51)),
        ];
        let doc = Document::new(DocumentMeta::default(), Rope::from_str(text), markers);
        let (out, _) = brf(&doc, &WriteOptions::default());
        let pages: Vec<&str> = out.split('\u{0C}').collect();
        assert_eq!(pages.len(), 2);
        let lines: Vec<&str> = pages[0].split("\r\n").collect();
        // Centered heading, then a blank line.
        assert_eq!(lines[0].trim(), ",TITLE");
        assert_eq!(lines[0].len(), (40 - 6) / 2 + 6);
        assert_eq!(lines[1], "");
        assert_eq!(lines[2], "  ,INTRO TEXT4");
        assert_eq!(lines[3], "#A4 ,FIRST");
        assert_eq!(lines[4], "#B4 ,SECOND");
        // A blank line before and after the table (Braille Formats 11.2.5d).
        assert_eq!(lines[5], "");
        assert_eq!(lines[6], ",A2 ,B");
        assert_eq!(lines[7], "#A2 #B");
        assert_eq!(lines[8], "");
        assert!(pages[1].starts_with("  ,MORE\r\n"));
    }

    #[test]
    fn print_page_indicator_and_unsupported_report() {
        let doc = Document::new(
            DocumentMeta::default(),
            Rope::from_str("One \u{1F600}\n\nTwo"),
            vec![
                Marker::new(MarkerKind::Paragraph, CharRange::new(0, 5)),
                Marker::new(MarkerKind::PageBreak, CharRange::empty(7)).with_label("12"),
                Marker::new(MarkerKind::Paragraph, CharRange::new(7, 10)),
            ],
        );
        let (out, report) = brf(&doc, &WriteOptions::default());
        let lines: Vec<&str> = out.split("\r\n").collect();
        assert_eq!(lines[0], "  ,ONE");
        assert_eq!(lines[1], format!("{}#AB", "-".repeat(37)));
        assert_eq!(lines[2], "  ,TWO");
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("U+1F600"), "{report:?}");
    }

    #[test]
    fn code_keeps_its_indentation() {
        let doc = Document::new(
            DocumentMeta::default(),
            Rope::from_str("if x {\n    y();\n\tz\n}"),
            vec![Marker::new(MarkerKind::Code, CharRange::new(0, 21)).with_level(1)],
        );
        let (out, _) = brf(&doc, &WriteOptions::default());
        let lines: Vec<&str> = out.split("\r\n").take(4).collect();
        assert_eq!(lines, ["IF X _<", "    Y\"<\">2", "    Z", "_>"]);
    }

    /// Grade 2's typeform indicators around a stand-in for liblouis that
    /// shows which print it was given: each stretch between indicators is
    /// contracted by itself, the indicators and guide dots go between them,
    /// and a passage over two paragraphs is opened in each and terminated
    /// once (UEB Rules 9.4 and 9.9.1).
    #[test]
    fn grade_two_puts_typeform_indicators_around_contracted_print() {
        let bold = Typeform::Bold;
        let italic = Typeform::Italic;
        let first = format!(
            "Click the {}Up One Level{} button.",
            bold.open(),
            bold.close()
        );
        let second = format!("{}One two{}", italic.open(), italic.close());
        let third = format!("{}three.{} Four", italic.open(), italic.close());
        let blank = format!("a {BLANK_ENTRY} b");
        let texts = [
            (first.as_str(), Join::Fresh),
            (second.as_str(), Join::Fresh),
            (third.as_str(), Join::Element),
            (blank.as_str(), Join::Fresh),
        ];
        let given = std::cell::RefCell::new(Vec::new());
        let stand_in = |pieces: &[&str]| -> Result<Vec<String>, String> {
            given
                .borrow_mut()
                .extend(pieces.iter().map(|p| p.to_string()));
            Ok(pieces
                .iter()
                .map(|p| format!("<{}>", p.to_uppercase()))
                .collect())
        };
        let out = louis_with_typeforms(&texts, &stand_in).unwrap();
        assert_eq!(
            out,
            [
                "<CLICK THE> ^7<UP ONE LEVEL>^' <BUTTON.>",
                ".7<ONE TWO>",
                ".7<THREE.>.' <FOUR>",
                "<A> \"\"\" <B>",
            ]
        );
        // No mark reaches liblouis.
        for piece in given.borrow().iter() {
            assert_eq!(&ueb::strip_marks(piece), piece);
        }
        // A failing translator is reported, so the writer falls back.
        let failing = |_: &[&str]| -> Result<Vec<String>, String> { Err("no".into()) };
        assert!(louis_with_typeforms(&texts, &failing).is_err());
    }

    #[test]
    fn liblouis_input_doubles_backslashes_and_joins_lines() {
        // lou_translate reads backslash escapes in its input.
        assert_eq!(louis::input_line(r"C:\data \n"), r"C:\\data \\n");
        assert_eq!(louis::input_line("one\ntwo\r\nthree"), "one two  three");
        assert_eq!(louis::input_line("plain"), "plain");
    }

    #[test]
    fn grade_two_without_liblouis_falls_back_with_a_warning() {
        let doc = Document::from_plain_text("the cat");
        let options = WriteOptions {
            braille: BrailleOptions {
                grade: BrailleGrade::Two,
                ..BrailleOptions::default()
            },
            ..WriteOptions::default()
        };
        let (out, report) = brf(&doc, &options);
        #[cfg(feature = "liblouis")]
        if louis::available() {
            // "the" is one cell in grade 2.
            assert!(out.starts_with("  ! CAT"), "{out:?}");
            assert!(report.warnings.is_empty());
            // A bold word takes the bold word indicator (UEB Rules 9.3)
            // before liblouis's contraction of it.
            let doc = Document::new(
                DocumentMeta::default(),
                Rope::from_str("the cat"),
                vec![
                    Marker::new(MarkerKind::Paragraph, CharRange::new(0, 7)),
                    Marker::new(MarkerKind::Bold, CharRange::new(4, 7)),
                ],
            );
            let (out, report) = brf(&doc, &options);
            assert!(out.starts_with("  ! ^1CAT"), "{out:?}");
            assert!(report.warnings.is_empty());
            // A print backslash reaches liblouis as itself, not as an
            // escape: it comes out as UEB's backslash, dots 456 then 16
            // (`_*`), and its paragraph is kept.
            let doc = Document::from_plain_text("see C:\\data now\n\nnext");
            let (out, _) = brf(&doc, &options);
            assert!(out.contains("SEE ,C3_*DATA N["), "{out:?}");
            assert!(out.contains("NEXT"), "{out:?}");
            return;
        }
        assert!(out.starts_with("  THE CAT"));
        assert!(report.warnings[0].contains("uncontracted"));
    }
}
