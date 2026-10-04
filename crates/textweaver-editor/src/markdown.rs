//! Markdown formatting commands as pure functions.
//!
//! Each command takes the text and the selection and returns the edits to
//! apply (as one undo step) and the selection afterwards. Semantics follow
//! star's `star/gui/mixin_authoring.py` (the star parity reference Part 3 §4.3)
//! with its listed bugs fixed (§7 items 30 to 33):
//!
//! | Command | star | textweaver |
//! |---|---|---|
//! | Bold, italic, underline, strikethrough, inline code | wraps; applying twice doubles the markup (`****x****`) | wraps, or unwraps when the selection is already wrapped (toggles) |
//! | Wrap with no selection | inserts the placeholder and selects it | same |
//! | Heading | `# ` prefix, level 1 only, stacks (`# # Title`) | sets the line's level; the same level again removes it |
//! | Bullet, numbered, quote | prefix every line, including blank lines and a line the selection only touches at column 0 | same prefixes; blank lines in a multi-line selection and the column-0 line are left alone; applying to lines that already have the prefix removes it; bullets and numbers convert into each other |
//! | Link | `[sel or "text"](https://)`, nothing selected afterwards | same text; selects `text` when there was no selection, else the URL |
//! | Horizontal rule | `\n---\n` at the caret; after a text line it renders as a setext heading | the rule gets a blank line before it and its own line |
//! | Insert table | leading newline unless at a line start, a blank line, the skeleton, a newline | same text; selects the first header cell's text |
//! | Add table row | a row after the current line, even on the header line (which breaks the table) | on the header or separator line, the row goes after the separator; the caret goes to the new row's first cell |
//! | Insert image | `![stem](relative path)`, nothing selected | same text; selects the alt text; a path with spaces or parentheses is written as `<path>` |

use std::path::{Component, Path, PathBuf};

use ropey::Rope;
use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange, Edit};

use crate::Selection;

/// A formatting command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkdownOp {
    /// `**bold**`
    Bold,
    /// `*italic*`
    Italic,
    /// `` `code` ``
    InlineCode,
    /// `~~strikethrough~~`
    Strikethrough,
    /// `# ` .. `###### ` at the start of each selected line.
    Heading(u8),
    /// `- ` list items.
    BulletList,
    /// `1. ` numbered items.
    NumberedList,
    /// `> ` block quote.
    Quote,
    /// A fenced code block around the selected lines.
    CodeBlock,
    /// `[text](url)`
    Link,
    /// `<u>underline</u>` (inline HTML, as star wrote it).
    Underline,
    /// `---` on its own line.
    HorizontalRule,
    /// A table skeleton with a header row and `rows` body rows.
    InsertTable {
        /// Body rows (at least 1).
        rows: u16,
        /// Columns (at least 1).
        cols: u16,
    },
    /// A new empty row in the table at the caret.
    AddTableRow,
}

/// The edits for one command and the selection afterwards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Formatted {
    /// Edits, each in the coordinates left by the previous one.
    pub edits: Vec<Edit>,
    /// Selection after applying the edits.
    pub selection: Selection,
}

impl Formatted {
    fn none(sel: Selection) -> Self {
        Formatted {
            edits: Vec::new(),
            selection: sel,
        }
    }
}

/// Why a command could not run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatError {
    /// Add Table Row with the caret outside a table row.
    NotInTable,
}

impl FormatError {
    /// The message for the user (star's wording).
    pub fn message(self) -> &'static str {
        match self {
            FormatError::NotInTable => "Put the cursor inside a table row to add a row",
        }
    }
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for FormatError {}

fn len(s: &str) -> usize {
    s.chars().count()
}

fn slice(text: &Rope, r: CharRange) -> String {
    let r = r.clamp_to(text.len_chars());
    text.slice(r.to_range()).to_string()
}

/// Consecutive `c` characters ending at `end`.
fn run_before(text: &Rope, end: usize, c: char) -> usize {
    (0..end).rev().take_while(|&i| text.char(i) == c).count()
}

/// Consecutive `c` characters starting at `start`.
fn run_after(text: &Rope, start: usize, c: char) -> usize {
    (start..text.len_chars())
        .take_while(|&i| text.char(i) == c)
        .count()
}

/// True when `marker` sits right outside `r` as a whole marker: for `*` and
/// `**`, runs of asterisks are read as bold (2), italic (1), or both (3).
fn wrapped_outside(text: &Rope, r: CharRange, open: &str, close: &str) -> bool {
    let (s, e) = (r.start.0, r.end.0);
    let (ol, cl) = (len(open), len(close));
    if s < ol || e + cl > text.len_chars() {
        return false;
    }
    if slice(text, CharRange::new(s - ol, s)) != open
        || slice(text, CharRange::new(e, e + cl)) != close
    {
        return false;
    }
    match open {
        "*" => {
            let (a, b) = (run_before(text, s, '*'), run_after(text, e, '*'));
            a % 2 == 1 && b % 2 == 1
        }
        "**" => run_before(text, s, '*') >= 2 && run_after(text, e, '*') >= 2,
        _ => true,
    }
}

/// star's `_qt_md_wrap`, toggling.
fn wrap(text: &Rope, sel: Selection, open: &str, close: &str, placeholder: &str) -> Formatted {
    let r = sel.range().clamp_to(text.len_chars());
    let (ol, cl) = (len(open), len(close));
    if r.is_empty() {
        let s = r.start;
        return Formatted {
            edits: vec![Edit::insert(s, format!("{open}{placeholder}{close}"))],
            selection: Selection::new(s.0 + ol, s.0 + ol + len(placeholder)),
        };
    }
    let inner = slice(text, r);
    // The selection includes the markers: unwrap it.
    let lead = inner.chars().take_while(|&c| c == '*').count();
    let trail = inner.chars().rev().take_while(|&c| c == '*').count();
    let runs_ok = match open {
        "*" => lead % 2 == 1 && trail % 2 == 1,
        "**" => lead >= 2 && trail >= 2,
        _ => true,
    };
    if r.len() >= ol + cl
        && inner.starts_with(open)
        && inner.ends_with(close)
        && runs_ok
        && !(open.starts_with('*') && inner.chars().all(|c| c == '*'))
    {
        let e = r.end.0;
        return Formatted {
            edits: vec![
                Edit::delete(CharRange::new(e - cl, e)),
                Edit::delete(CharRange::new(r.start.0, r.start.0 + ol)),
            ],
            selection: Selection::new(r.start.0, e - cl - ol),
        };
    }
    // The markers sit right outside the selection: unwrap it.
    if wrapped_outside(text, r, open, close) {
        let (s, e) = (r.start.0, r.end.0);
        return Formatted {
            edits: vec![
                Edit::delete(CharRange::new(e, e + cl)),
                Edit::delete(CharRange::new(s - ol, s)),
            ],
            selection: Selection::new(s - ol, e - ol),
        };
    }
    // star: replace the selection with open + selection + close; the caret
    // ends after `close` with nothing selected.
    let end = r.end.0 + ol + cl;
    Formatted {
        edits: vec![
            Edit::insert(r.start, open),
            Edit::insert(r.end.0 + ol, close),
        ],
        selection: Selection::caret(CharPos(end)),
    }
}

/// Lines `first..=last` affected by a selection: star's block range, minus
/// a last line the selection only reaches at column 0.
fn line_span(text: &Rope, r: CharRange) -> (usize, usize) {
    let n = text.len_chars();
    let first = text.char_to_line(r.start.0.min(n));
    let mut last = text.char_to_line(r.end.0.min(n));
    if !r.is_empty() && last > first && text.line_to_char(last) == r.end.0 {
        last -= 1;
    }
    (first, last)
}

/// A line's text without its line ending.
fn line_text(text: &Rope, line: usize) -> String {
    let s = text.line(line).to_string();
    s.trim_end_matches(['\n', '\r']).to_owned()
}

/// The list or quote marker a line starts with, and its length in chars.
fn bullet_prefix(line: &str) -> Option<usize> {
    let t = line.strip_prefix(['-', '*', '+'])?;
    t.starts_with(' ').then_some(2)
}

fn number_prefix(line: &str) -> Option<usize> {
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 || digits > 9 {
        return None;
    }
    let rest = &line[digits..];
    (rest.starts_with(". ") || rest.starts_with(") ")).then_some(digits + 2)
}

fn heading_prefix(line: &str) -> Option<(u8, usize)> {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let rest = &line[hashes..];
    if rest.is_empty() {
        return Some((hashes as u8, hashes));
    }
    rest.starts_with(' ').then_some((hashes as u8, hashes + 1))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LineKind {
    Bullet,
    Numbered,
    Quote,
    Heading(u8),
}

fn existing(kind: LineKind, line: &str) -> Option<usize> {
    match kind {
        LineKind::Bullet => bullet_prefix(line),
        LineKind::Numbered => number_prefix(line),
        LineKind::Quote => line
            .starts_with('>')
            .then(|| if line.starts_with("> ") { 2 } else { 1 }),
        LineKind::Heading(_) => heading_prefix(line).map(|(_, n)| n),
    }
}

/// star's `_qt_md_line_prefix`, toggling and skipping blank lines in a
/// multi-line selection.
fn prefix_lines(text: &Rope, sel: Selection, kind: LineKind) -> Formatted {
    let r = sel.range().clamp_to(text.len_chars());
    let (first, last) = line_span(text, r);
    let lines: Vec<(usize, String)> = (first..=last).map(|l| (l, line_text(text, l))).collect();
    let multi = lines.len() > 1;
    let targets: Vec<&(usize, String)> = lines
        .iter()
        .filter(|(_, t)| !multi || !t.trim().is_empty())
        .collect();
    let all_have = !targets.is_empty()
        && targets.iter().all(|(_, t)| match kind {
            LineKind::Heading(level) => heading_prefix(t).is_some_and(|(l, _)| l == level),
            k => existing(k, t).is_some(),
        });

    let mut edits = Vec::new();
    // Line starts shift as earlier lines change; track the running delta.
    let mut delta: isize = 0;
    let mut number = 0usize;
    let mut head_shift: isize = 0;
    let mut anchor_shift: isize = 0;
    let (anchor, head) = (sel.anchor.0, sel.head.0);
    for (line, t) in &lines {
        if multi && t.trim().is_empty() {
            continue;
        }
        let start = text.line_to_char(*line);
        let at = (start as isize + delta) as usize;
        let (remove, insert) = if all_have {
            let n = match kind {
                LineKind::Heading(_) => heading_prefix(t).map_or(0, |(_, n)| n),
                k => existing(k, t).unwrap_or(0),
            };
            (n, String::new())
        } else {
            number += 1;
            match kind {
                LineKind::Bullet => (number_prefix(t).unwrap_or(0), "- ".to_owned()),
                LineKind::Numbered => (
                    bullet_prefix(t).or(number_prefix(t)).unwrap_or(0),
                    format!("{number}. "),
                ),
                LineKind::Quote => (0, "> ".to_owned()),
                LineKind::Heading(level) => (
                    heading_prefix(t).map_or(0, |(_, n)| n),
                    format!("{} ", "#".repeat(usize::from(level.clamp(1, 6)))),
                ),
            }
        };
        let (remove, insert) = match kind {
            // A line already a bullet keeps its marker when bulleting.
            LineKind::Bullet if !all_have && bullet_prefix(t).is_some() => (0, String::new()),
            LineKind::Quote if !all_have && existing(kind, t).is_some() => (0, String::new()),
            _ => (remove, insert),
        };
        if remove == 0 && insert.is_empty() {
            continue;
        }
        edits.push(Edit::replace(
            CharRange::new(at, at + remove),
            insert.clone(),
        ));
        let change = len(&insert) as isize - remove as isize;
        let shift_for = |pos: usize| -> isize {
            if pos >= start + remove {
                change
            } else if pos > start {
                // Inside a removed prefix: move to the line start.
                -((pos - start) as isize) + len(&insert) as isize
            } else if pos == start && remove == 0 {
                // At the line start: text inserted there pushes the caret.
                change
            } else {
                0
            }
        };
        head_shift += shift_for(head);
        anchor_shift += shift_for(anchor);
        delta += change;
    }
    let mv = |p: usize, s: isize| CharPos((p as isize + s).max(0) as usize);
    Formatted {
        edits,
        selection: Selection {
            anchor: mv(anchor, anchor_shift),
            head: mv(head, head_shift),
        },
    }
}

/// star's `_qt_md_link`: `[selection or "text"](https://)`.
fn link(text: &Rope, sel: Selection) -> Formatted {
    let r = sel.range().clamp_to(text.len_chars());
    let label = if r.is_empty() {
        "text".to_owned()
    } else {
        slice(text, r)
    };
    let inserted = format!("[{label}](https://)");
    let s = r.start.0;
    let selection = if r.is_empty() {
        Selection::new(s + 1, s + 1 + len(&label))
    } else {
        let url = s + 1 + len(&label) + 2;
        Selection::new(url, url + len("https://"))
    };
    Formatted {
        edits: vec![Edit::replace(r, inserted)],
        selection,
    }
}

/// A horizontal rule on its own line with a blank line before it (star's
/// `\n---\n` made a setext heading after a text line).
fn horizontal_rule(text: &Rope, sel: Selection) -> Formatted {
    let r = sel.range().clamp_to(text.len_chars());
    let s = r.start.0;
    let before = if s == 0 {
        ""
    } else if text.char(s - 1) == '\n' {
        if s >= 2 && text.char(s - 2) != '\n' {
            "\n"
        } else {
            ""
        }
    } else {
        "\n\n"
    };
    let inserted = format!("{before}---\n");
    let end = s + len(&inserted);
    Formatted {
        edits: vec![Edit::replace(r, inserted)],
        selection: Selection::caret(CharPos(end)),
    }
}

/// star's `_md_table_skeleton(rows, cols)`: a header row `| Column 1 | ... |`,
/// a separator row `| --- | ... |`, and `rows` rows of empty cells. Both
/// arguments are at least 1. Ends with a newline.
pub fn table_skeleton(rows: usize, cols: usize) -> String {
    let (rows, cols) = (rows.max(1), cols.max(1));
    let header: Vec<String> = (1..=cols).map(|i| format!("Column {i}")).collect();
    let mut out = format!("| {} |\n", header.join(" | "));
    out.push_str(&format!("| {} |\n", vec!["---"; cols].join(" | ")));
    let row = format!("| {} |\n", vec![" "; cols].join(" | "));
    for _ in 0..rows {
        out.push_str(&row);
    }
    out
}

/// star's `_qt_md_insert_table`: `("" at a line start, else "\n") + "\n" +
/// skeleton + "\n"`, replacing the selection. Selects the first header
/// cell's text.
fn insert_table(text: &Rope, sel: Selection, rows: usize, cols: usize) -> Formatted {
    let r = sel.range().clamp_to(text.len_chars());
    let s = r.start.0;
    let at_line_start = s == 0 || text.char(s - 1) == '\n';
    let lead = if at_line_start { "" } else { "\n" };
    let inserted = format!("{lead}\n{}\n", table_skeleton(rows, cols));
    let first_cell = s + len(lead) + 1 + 2;
    Formatted {
        edits: vec![Edit::replace(r, inserted)],
        selection: Selection::new(first_cell, first_cell + len("Column 1")),
    }
}

fn is_table_row(line: &str) -> bool {
    let t = line.trim();
    t.contains('|') && t.starts_with('|')
}

fn is_separator_row(line: &str) -> bool {
    let t = line.trim();
    is_table_row(t) && t.contains('-') && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

/// star's `_qt_md_table_add_row`, fixed for the header line.
fn add_table_row(text: &Rope, sel: Selection) -> Result<Formatted, FormatError> {
    let n = text.len_chars();
    let mut line = text.char_to_line(sel.head.0.min(n));
    let current = line_text(text, line);
    if !is_table_row(&current) {
        return Err(FormatError::NotInTable);
    }
    let cols = current.trim().trim_matches('|').matches('|').count() + 1;
    let lines = text.len_lines();
    // On the header or the separator, the row goes after the separator.
    if line + 1 < lines && is_separator_row(&line_text(text, line + 1)) {
        line += 1;
    }
    let end = text.line_to_char(line) + len(&line_text(text, line));
    let row = format!("| {} |", vec![" "; cols].join(" | "));
    Ok(Formatted {
        edits: vec![Edit::insert(end, format!("\n{row}"))],
        selection: Selection::caret(CharPos(end + 1 + 2)),
    })
}

/// A path relative to `base`, without touching the file system. `None`
/// when the two do not share a root (another Windows drive).
fn relative_path(target: &Path, base: &Path) -> Option<PathBuf> {
    let t: Vec<Component<'_>> = target.components().collect();
    let b: Vec<Component<'_>> = base.components().collect();
    let common = t.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if common == 0 {
        return None;
    }
    let mut out = PathBuf::new();
    for _ in common..b.len() {
        out.push("..");
    }
    for c in &t[common..] {
        out.push(c.as_os_str());
    }
    Some(out)
}

/// The image reference star writes: relative to the document's folder with
/// `/` separators, unless it climbs two or more levels (`../../`) or the
/// drives differ, in which case the absolute path.
pub fn image_reference(doc_path: Option<&Path>, image: &Path) -> String {
    let abs = std::path::absolute(image).unwrap_or_else(|_| image.to_owned());
    let rel = doc_path
        .and_then(|d| std::path::absolute(d).ok())
        .and_then(|d| d.parent().map(Path::to_owned))
        .and_then(|dir| relative_path(&abs, &dir))
        .map(|p| {
            p.components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        })
        .filter(|r| !r.starts_with("../../"));
    let reference = rel.unwrap_or_else(|| abs.to_string_lossy().replace('\\', "/"));
    if reference.contains([' ', '(', ')']) {
        format!("<{reference}>")
    } else {
        reference
    }
}

/// star's `_qt_md_insert_image`: `![stem](reference)`, replacing the
/// selection. Selects the alt text so a description can be typed.
pub fn insert_image(
    text: &Rope,
    sel: Selection,
    doc_path: Option<&Path>,
    image: &Path,
) -> Formatted {
    let r = sel.range().clamp_to(text.len_chars());
    let stem = image
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "image".to_owned());
    let inserted = format!("![{stem}]({})", image_reference(doc_path, image));
    let s = r.start.0 + 2;
    Formatted {
        edits: vec![Edit::replace(r, inserted)],
        selection: Selection::new(s, s + len(&stem)),
    }
}

/// A fenced code block around the selected lines, or an empty one at the
/// caret with the caret inside.
fn code_block(text: &Rope, sel: Selection) -> Formatted {
    let r = sel.range().clamp_to(text.len_chars());
    if r.is_empty() {
        let s = r.start.0;
        let lead = if s == 0 || text.char(s - 1) == '\n' {
            ""
        } else {
            "\n"
        };
        let inserted = format!("{lead}```\n\n```\n");
        let caret = s + len(lead) + 4;
        return Formatted {
            edits: vec![Edit::insert(s, inserted)],
            selection: Selection::caret(CharPos(caret)),
        };
    }
    let (first, last) = line_span(text, r);
    let start = text.line_to_char(first);
    let end = text.line_to_char(last) + len(&line_text(text, last));
    let body = slice(text, CharRange::new(start, end));
    let inserted = format!("```\n{body}\n```");
    let total = len(&inserted);
    Formatted {
        edits: vec![Edit::replace(CharRange::new(start, end), inserted)],
        selection: Selection::new(start, start + total),
    }
}

/// The edits that apply `op` to `sel` in `text`. A command that cannot run
/// (Add Table Row outside a table) returns no edits; use [`try_apply`] for
/// the reason.
pub fn apply(text: &Rope, sel: Selection, op: MarkdownOp) -> Formatted {
    try_apply(text, sel, op).unwrap_or_else(|_| Formatted::none(sel))
}

/// The edits that apply `op` to `sel` in `text`, or why it cannot run.
pub fn try_apply(text: &Rope, sel: Selection, op: MarkdownOp) -> Result<Formatted, FormatError> {
    Ok(match op {
        MarkdownOp::Bold => wrap(text, sel, "**", "**", "bold text"),
        MarkdownOp::Italic => wrap(text, sel, "*", "*", "italic text"),
        MarkdownOp::Underline => wrap(text, sel, "<u>", "</u>", "underlined text"),
        MarkdownOp::InlineCode => wrap(text, sel, "`", "`", "code"),
        MarkdownOp::Strikethrough => wrap(text, sel, "~~", "~~", "struck text"),
        MarkdownOp::Link => link(text, sel),
        MarkdownOp::Heading(level) => prefix_lines(text, sel, LineKind::Heading(level.clamp(1, 6))),
        MarkdownOp::BulletList => prefix_lines(text, sel, LineKind::Bullet),
        MarkdownOp::NumberedList => prefix_lines(text, sel, LineKind::Numbered),
        MarkdownOp::Quote => prefix_lines(text, sel, LineKind::Quote),
        MarkdownOp::CodeBlock => code_block(text, sel),
        MarkdownOp::HorizontalRule => horizontal_rule(text, sel),
        MarkdownOp::InsertTable { rows, cols } => {
            insert_table(text, sel, usize::from(rows), usize::from(cols))
        }
        MarkdownOp::AddTableRow => return add_table_row(text, sel),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Editor;

    fn run(text: &str, sel: Selection, op: MarkdownOp) -> (String, Selection) {
        let mut ed = Editor::new(text);
        ed.set_selection(sel);
        let f = try_apply(ed.text(), sel, op).unwrap();
        ed.apply_formatted(&f).unwrap();
        (ed.text().to_string(), ed.selection())
    }

    fn sel(a: usize, b: usize) -> Selection {
        Selection::new(a, b)
    }

    #[test]
    fn bold_is_one_undo_step() {
        let mut ed = Editor::new("make this bold");
        let s = sel(10, 14);
        let f = apply(ed.text(), s, MarkdownOp::Bold);
        ed.apply_group(f.edits).unwrap();
        assert_eq!(ed.text().to_string(), "make this **bold**");
        ed.undo();
        assert_eq!(ed.text().to_string(), "make this bold");
    }

    #[test]
    fn numbered_list_prefixes_each_line() {
        assert_eq!(
            run("a\nb", sel(0, 3), MarkdownOp::NumberedList).0,
            "1. a\n2. b"
        );
    }

    #[test]
    fn wrap_toggles_both_ways() {
        // Selection inside the markers.
        let (t, s) = run("a **b** c", sel(4, 5), MarkdownOp::Bold);
        assert_eq!(t, "a b c");
        assert_eq!(s, sel(2, 3));
        // Selection including the markers.
        let (t, s) = run("a **b** c", sel(2, 7), MarkdownOp::Bold);
        assert_eq!(t, "a b c");
        assert_eq!(s, sel(2, 3));
        // Italic inside bold adds italic rather than eating a bold star.
        assert_eq!(run("**b**", sel(2, 3), MarkdownOp::Italic).0, "***b***");
        // Italic off from bold italic.
        assert_eq!(run("***b***", sel(3, 4), MarkdownOp::Italic).0, "**b**");
        assert_eq!(run("***b***", sel(3, 4), MarkdownOp::Bold).0, "*b*");
        assert_eq!(run("x", sel(0, 1), MarkdownOp::Underline).0, "<u>x</u>");
        assert_eq!(run("<u>x</u>", sel(3, 4), MarkdownOp::Underline).0, "x");
    }

    #[test]
    fn headings_set_and_toggle_levels() {
        assert_eq!(run("# Title", sel(3, 3), MarkdownOp::Heading(1)).0, "Title");
        assert_eq!(
            run("## Title", sel(0, 0), MarkdownOp::Heading(1)).0,
            "# Title"
        );
        assert_eq!(
            run("Title", sel(2, 2), MarkdownOp::Heading(3)).0,
            "### Title"
        );
        let (_, s) = run("Title", sel(2, 2), MarkdownOp::Heading(1));
        assert_eq!(s, Selection::caret(CharPos(4)));
    }

    #[test]
    fn lists_toggle_convert_and_skip_blank_lines() {
        assert_eq!(run("- a\n- b", sel(0, 7), MarkdownOp::BulletList).0, "a\nb");
        assert_eq!(
            run("1. a\n2. b", sel(0, 9), MarkdownOp::BulletList).0,
            "- a\n- b"
        );
        assert_eq!(
            run("- a\n- b", sel(0, 7), MarkdownOp::NumberedList).0,
            "1. a\n2. b"
        );
        assert_eq!(run("a\n\nb", sel(0, 4), MarkdownOp::Quote).0, "> a\n\n> b");
        // Only one blank line: prefix it so a list can start there.
        assert_eq!(run("", sel(0, 0), MarkdownOp::BulletList).0, "- ");
        // A selection ending at column 0 of the next line leaves that line.
        assert_eq!(run("a\nb", sel(0, 2), MarkdownOp::BulletList).0, "- a\nb");
        // Mixed: lines without the marker get it; lines with it keep it.
        assert_eq!(
            run("- a\nb", sel(0, 5), MarkdownOp::BulletList).0,
            "- a\n- b"
        );
    }

    #[test]
    fn link_selects_what_to_type_next() {
        let (t, s) = run("", sel(0, 0), MarkdownOp::Link);
        assert_eq!(t, "[text](https://)");
        assert_eq!(s, sel(1, 5));
        let (t, s) = run("see here", sel(4, 8), MarkdownOp::Link);
        assert_eq!(t, "see [here](https://)");
        assert_eq!(s, sel(11, 19));
    }

    #[test]
    fn horizontal_rule_never_makes_a_heading() {
        assert_eq!(run("", sel(0, 0), MarkdownOp::HorizontalRule).0, "---\n");
        assert_eq!(
            run("a\n\n", sel(3, 3), MarkdownOp::HorizontalRule).0,
            "a\n\n---\n"
        );
        assert_eq!(
            run("a\n", sel(2, 2), MarkdownOp::HorizontalRule).0,
            "a\n\n---\n"
        );
        assert_eq!(
            run("ab", sel(1, 1), MarkdownOp::HorizontalRule).0,
            "a\n\n---\nb"
        );
    }

    #[test]
    fn add_row_errors_outside_tables_and_skips_the_separator() {
        let text = Rope::from_str("plain");
        assert_eq!(
            try_apply(&text, sel(0, 0), MarkdownOp::AddTableRow),
            Err(FormatError::NotInTable)
        );
        assert_eq!(
            apply(&text, sel(0, 0), MarkdownOp::AddTableRow).edits,
            vec![]
        );
        let (t, s) = run(
            "| a | b |\n| --- | --- |\n| 1 | 2 |",
            sel(3, 3),
            MarkdownOp::AddTableRow,
        );
        assert_eq!(t, "| a | b |\n| --- | --- |\n|   |   |\n| 1 | 2 |");
        assert_eq!(s, Selection::caret(CharPos(26)));
    }

    #[test]
    fn code_blocks() {
        assert_eq!(run("", sel(0, 0), MarkdownOp::CodeBlock).0, "```\n\n```\n");
        assert_eq!(
            run("x = 1\ny = 2", sel(1, 8), MarkdownOp::CodeBlock).0,
            "```\nx = 1\ny = 2\n```"
        );
    }

    #[test]
    fn image_references() {
        let dir = std::env::temp_dir();
        let doc = dir.join("notes").join("note.md");
        let img = dir.join("notes").join("img").join("pic.png");
        assert_eq!(image_reference(Some(&doc), &img), "img/pic.png");
        let up = dir.join("pic.png");
        assert_eq!(image_reference(Some(&doc), &up), "../pic.png");
        let far = dir.parent().unwrap_or(&dir).join("far.png");
        let r = image_reference(Some(&doc.join("deeper").join("x.md")), &far);
        assert!(!r.starts_with("../../"), "{r}");
        let spaced = dir.join("notes").join("my pic.png");
        assert_eq!(image_reference(Some(&doc), &spaced), "<my pic.png>");
    }
}
