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
//! - table rows one per line, cells separated by semicolons (linear
//!   format), runovers in cell 3;
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
//! Translation is UEB grade 1 by [`ueb`](crate::ueb), or grade 2 through
//! liblouis when [`BrailleGrade::Two`](crate::BrailleGrade) is chosen and
//! the `liblouis` feature is on and `lou_translate` is installed.

use std::io::Write;

use textweaver_text::Document;

use crate::model::{self, Block, Inline, List, Table};
use crate::ueb;
use crate::{BrailleGrade, BrailleOptions, Format, WriteError, WriteOptions, WriteReport, Writer};

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
        let blocks = model::blocks(doc);
        let mut items = Vec::new();
        flatten(&blocks, 0, &mut items);
        let texts: Vec<&str> = items.iter().filter_map(Item::text).collect();
        let translated = translate_all(&texts, &options.braille, &mut report)?;
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
    },
    /// A new braille page.
    NewPage,
    /// Print page change indicator with the print page label.
    PrintPage(Option<String>),
}

impl Item {
    fn text(&self) -> Option<&str> {
        match self {
            Item::Heading { text, .. } | Item::Text { text, .. } => Some(text),
            Item::PrintPage(Some(label)) => Some(label),
            _ => None,
        }
    }
}

/// Plain text of inlines with line breaks kept as `\n`.
fn text_of(inlines: &[Inline]) -> String {
    let mut s = String::new();
    fn walk(v: &[Inline], s: &mut String) {
        for i in v {
            match i {
                Inline::Text(t) => s.push_str(t),
                Inline::LineBreak => s.push('\n'),
                Inline::Span(_, c) => walk(c, s),
            }
        }
    }
    walk(inlines, &mut s);
    s
}

fn text_item(text: String, first: usize, runover: usize) -> Item {
    Item::Text {
        text,
        first,
        runover,
        blank_before: false,
    }
}

fn flatten(blocks: &[Block], indent: usize, out: &mut Vec<Item>) {
    for b in blocks {
        match b {
            Block::Heading { level, content } => out.push(Item::Heading {
                level: *level,
                text: model::collapse_ws(&Inline::plain(content)),
            }),
            Block::Paragraph(content) => {
                // Lines after a line break start in the runover cell.
                for (n, line) in text_of(content).split('\n').enumerate() {
                    let first = if n == 0 { indent + 2 } else { indent };
                    out.push(text_item(line.to_owned(), first, indent));
                }
            }
            Block::List(list) => flatten_list(list, indent, out),
            Block::Table(t) => flatten_table(t, indent, out),
            Block::Code { text, .. } => {
                for line in text.split('\n') {
                    out.push(text_item(line.to_owned(), indent, indent + 2));
                }
            }
            Block::Quote(inner) => {
                let start = out.len();
                flatten(inner, indent + 2, out);
                if let Some(Item::Text { blank_before, .. }) = out.get_mut(start) {
                    *blank_before = true;
                }
            }
            Block::Figure(img) => {
                if !img.alt.is_empty() {
                    out.push(text_item(img.alt.clone(), indent + 2, indent));
                }
            }
            Block::Footnote { id, content } => {
                let body = model::collapse_ws(&Inline::plain(content));
                let text = if body.starts_with(&format!("[{id}]")) {
                    body
                } else {
                    format!("[{id}] {body}")
                };
                out.push(text_item(text, indent, indent + 2));
            }
            Block::SectionBreak { .. } => out.push(Item::NewPage),
            Block::PageBreak { label } => out.push(Item::PrintPage(label.clone())),
        }
    }
}

fn flatten_list(list: &List, indent: usize, out: &mut Vec<Item>) {
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
                model::collapse_ws(&text_of(content))
            }
            _ => String::new(),
        };
        let text = if first_text.is_empty() {
            marker
        } else {
            format!("{marker} {first_text}")
        };
        out.push(text_item(text, indent, indent + 2));
        flatten(rest, indent + 2, out);
    }
}

fn flatten_table(table: &Table, indent: usize, out: &mut Vec<Item>) {
    if let Some(c) = &table.caption {
        out.push(text_item(c.clone(), indent + 2, indent));
    }
    for row in &table.rows {
        let cells: Vec<String> = row
            .cells
            .iter()
            .map(|c| model::collapse_ws(&Inline::plain(c)))
            .collect();
        out.push(text_item(cells.join("; "), indent, indent + 2));
    }
}

/// Translates every text, natively or through liblouis.
fn translate_all(
    texts: &[&str],
    options: &BrailleOptions,
    report: &mut WriteReport,
) -> Result<Vec<String>, WriteError> {
    if options.grade == BrailleGrade::Two {
        match louis::translate(texts, &options.table) {
            Ok(v) => return Ok(v),
            Err(why) => report.warn(format!(
                "Contracted braille is not available ({why}), so the file is in uncontracted braille."
            )),
        }
    }
    let mut unsupported: Vec<char> = Vec::new();
    let v = texts
        .iter()
        .map(|t| {
            let tr = ueb::translate(t);
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

/// Wraps braille into lines of `width` cells with the given indents.
fn wrap(braille: &str, width: usize, first: usize, runover: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = " ".repeat(first.min(width / 2));
    let mut empty = true;
    let runover_pad = " ".repeat(runover.min(width / 2));
    for word in braille.split(' ').filter(|w| !w.is_empty()) {
        let mut word = word.to_owned();
        loop {
            let need = if empty { word.len() } else { word.len() + 1 };
            if line.len() + need <= width {
                if !empty {
                    line.push(' ');
                }
                line.push_str(&word);
                empty = false;
                break;
            }
            let room = width.saturating_sub(line.len() + usize::from(!empty));
            // Divide after a hyphen when it fits, else with the line
            // continuation indicator; move the word down when it would fit
            // on a fresh line.
            let fresh = width - runover_pad.len();
            if !empty && word.len() <= fresh {
                lines.push(std::mem::replace(&mut line, runover_pad.clone()));
                empty = true;
                continue;
            }
            if room >= 3 {
                let cut = word[..room]
                    .rfind('-')
                    .filter(|&h| h > 0)
                    .map(|h| h + 1)
                    .unwrap_or(room - 1);
                let (head, tail) = word.split_at(cut);
                if !empty {
                    line.push(' ');
                }
                line.push_str(head);
                if !head.ends_with('-') {
                    line.push('"');
                }
                word = tail.to_owned();
            }
            lines.push(std::mem::replace(&mut line, runover_pad.clone()));
            empty = true;
        }
    }
    if !empty || lines.is_empty() {
        lines.push(line);
    }
    lines
}

fn layout(items: &[Item], translated: Vec<String>, options: &BrailleOptions) -> String {
    let mut pages = Pages::new(options);
    let width = pages.width;
    let mut texts = translated.into_iter();
    for item in items {
        match item {
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
                ..
            } => {
                let braille = texts.next().unwrap_or_default();
                if braille.trim().is_empty() {
                    continue;
                }
                if *blank_before {
                    pages.blank();
                }
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
    /// Translates each text (one per line) with `table`.
    #[cfg(feature = "liblouis")]
    pub(super) fn translate(texts: &[&str], table: &str) -> Result<Vec<String>, String> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        let tables = format!("en-us-brf.dis,{table}");
        let mut child = Command::new("lou_translate")
            .args(["--forward", &tables])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("lou_translate could not start: {e}"))?;
        let mut input = String::new();
        for t in texts {
            // One line per text; line breaks inside a text become spaces.
            input.push_str(&t.replace(['\n', '\r'], " "));
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
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let text = String::from_utf8_lossy(&output.stdout);
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
        std::process::Command::new("lou_translate")
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
        assert_eq!(lines[5], ",A2 ,B");
        assert_eq!(lines[6], "#A2 #B");
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
            return;
        }
        assert!(out.starts_with("  THE CAT"));
        assert!(report.warnings[0].contains("uncontracted"));
    }
}
