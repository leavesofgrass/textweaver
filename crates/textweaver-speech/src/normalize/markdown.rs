//! Markdown residue and table narration, ported from
//! `star/ttstext/markdown.py` (`_strip_markdown_for_tts`, Part 2 section
//! 5.A) and `star/ttstext/tables.py` (`_tables_to_narration`, 5.A.1).
//!
//! In star these ran at load time over the whole document. In textweaver the
//! Markdown loader (Agent A) builds canonical text from parser events, so
//! this transform is for Markdown syntax that survives into plain text. It
//! is off by default ([`NormalizeConfig::markdown`](super::NormalizeConfig)).
//!
//! Deliberate fixes (Part 2 section 7.2):
//!
//! - Q5: list markers no longer swallow the blank line before a list, so a
//!   paragraph break before a list survives ("para\n\n- a" keeps its break).
//! - Q6: `_emphasis_` needs non-word characters outside the underscores, so
//!   `my_var_name` is left alone; `*emphasis*` needs non-space just inside
//!   and is not taken between digits, so `2*3*4` stays arithmetic.
//! - Q10: table cells are paired with headers by column, so an empty middle
//!   cell no longer shifts later cells under the wrong header; cells beyond
//!   the header are named "column N"; GitHub tables without a leading pipe
//!   are recognized by their separator row.

use regex::Captures;
use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange, OffsetMap, SpokenBuilder};

use super::Transform;
use super::rewrite::{Piece, Rule, apply_rules, char_after, char_before, is_word, then};

/// How tables are read aloud.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableMode {
    /// "Table with 3 columns: Name, Age, City." then "Row 1: Name is Alice, ...".
    #[default]
    Structured,
    /// Cell text only: "Alice.  30.  NY."
    Flat,
    /// "Table with 3 columns — skipped."
    Skip,
}

fn keep1() -> Vec<Piece> {
    vec![Piece::Keep(1)]
}

/// An emphasis rule `delim(inner)delim` with Q6 guards.
fn emphasis(pattern: &str, underscore: bool) -> Rule {
    Rule::with(pattern, move |c: &Captures<'_>, s: &str| {
        let m = c.get(0)?;
        let inner = c.get(1)?.as_str();
        if inner.starts_with(char::is_whitespace) || inner.ends_with(char::is_whitespace) {
            return None;
        }
        let before = char_before(s, m.start());
        let after = char_after(s, m.end());
        if underscore {
            if before.is_some_and(is_word) || after.is_some_and(is_word) {
                return None;
            }
        } else if before.is_some_and(|b| b.is_ascii_digit())
            || after.is_some_and(|a| a.is_ascii_digit())
        {
            return None;
        }
        Some(keep1())
    })
}

fn line_rules(skip_code: bool) -> Vec<Rule> {
    let mut r = vec![
        // 1. List markers (before code, so nested items survive).
        Rule::new(r"(?m)^[ \t]*[-*+][ \t]+", ""),
        Rule::new(r"(?m)^[ \t]*[0-9]+[.)][ \t]+", ""),
    ];
    // 2. Code.
    if skip_code {
        r.push(Rule::new(r"```[\s\S]*?```", ""));
        r.push(Rule::new(r"~~~[\s\S]*?~~~", ""));
        r.push(Rule::new(r"(?m)^    .+$", ""));
    } else {
        r.push(Rule::new(r"```\w*\n?", ""));
        r.push(Rule::new(r"```", ""));
    }
    r.extend([
        // 3. Headings.
        Rule::new(r"(?m)^#{1,6}[ \t]+", ""),
        // 4. Horizontal rules.
        Rule::new(r"(?m)^(?:\*{3,}|-{3,}|_{3,})[ \t]*$", ""),
        // 5. Images, then links.
        Rule::new(r"!\[([^\]]*)\]\([^)]*\)", "\\1"),
        Rule::new(r"\[([^\]]*)\]\([^)]*\)", "\\1"),
        // 6. Inline code.
        Rule::new(r"`+(.+?)`+", "\\1"),
        // 7. Emphasis.
        emphasis(r"\*{3}(.+?)\*{3}", false),
        emphasis(r"_{3}(.+?)_{3}", true),
        emphasis(r"\*{2}(.+?)\*{2}", false),
        emphasis(r"_{2}(.+?)_{2}", true),
        emphasis(r"\*([^*\n]+?)\*", false),
        emphasis(r"_([^_\n]+?)_", true),
        // 8. Blockquotes.
        Rule::new(r"(?m)^(?:>[ \t]?)+", ""),
    ]);
    r
}

fn final_rules() -> Vec<Rule> {
    vec![
        // 10. Three or more newlines to two.
        Rule::new(r"(\n\n)\n+", "\\1"),
        // 11. Soft breaks: a lone newline is a space.
        Rule::with(r"\n", |c, s| {
            let m = c.get(0)?;
            let lone =
                char_before(s, m.start()) != Some('\n') && char_after(s, m.end()) != Some('\n');
            lone.then(|| vec![Piece::Text(" ".into())])
        }),
        // 12. Strip.
        Rule::new(r"\A\s+", ""),
        Rule::new(r"\s+\z", ""),
    ]
}

/// The Markdown residue transform.
pub struct MarkdownResidue {
    lines: Vec<Rule>,
    finals: Vec<Rule>,
    table_mode: TableMode,
}

impl std::fmt::Debug for MarkdownResidue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MarkdownResidue")
            .field("table_mode", &self.table_mode)
            .finish_non_exhaustive()
    }
}

impl MarkdownResidue {
    /// A transform that drops code blocks when `skip_code` and narrates
    /// tables per `table_mode`.
    pub fn new(skip_code: bool, table_mode: TableMode) -> Self {
        MarkdownResidue {
            lines: line_rules(skip_code),
            finals: final_rules(),
            table_mode,
        }
    }
}

impl Transform for MarkdownResidue {
    fn name(&self) -> &'static str {
        "markdown"
    }

    fn apply(&self, input: &str) -> (String, OffsetMap) {
        let acc = apply_rules(input, &self.lines);
        let tables = acc
            .0
            .contains('|')
            .then(|| tables_to_narration(&acc.0, self.table_mode));
        let mut acc = then(acc, tables);
        for r in &self.finals {
            let step = r.apply(&acc.0);
            acc = then(acc, step);
        }
        acc
    }
}

/// star `_strip_markdown_for_tts(md, skip_code, table_mode)`.
pub fn strip_markdown(md: &str, skip_code: bool, table_mode: TableMode) -> String {
    MarkdownResidue::new(skip_code, table_mode).apply(md).0
}

// ---- tables ------------------------------------------------------------------

/// A line of the input: byte range (without the newline) and char start.
#[derive(Clone, Copy, Debug)]
struct Line {
    start: usize,
    end: usize,
    char_start: usize,
}

/// A table cell: trimmed text and its char range in the input.
#[derive(Clone, Debug)]
struct Cell {
    text: String,
    range: CharRange,
}

fn split_lines(s: &str) -> Vec<Line> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut char_start = 0;
    for (i, c) in s.char_indices() {
        if c == '\n' {
            out.push(Line {
                start,
                end: i,
                char_start,
            });
            char_start += s[start..i].chars().count() + 1;
            start = i + 1;
        }
    }
    out.push(Line {
        start,
        end: s.len(),
        char_start,
    });
    out
}

fn cells(s: &str, line: Line) -> Vec<Cell> {
    let text = &s[line.start..line.end];
    // Byte range of the row without surrounding whitespace and edge pipes.
    let lead_ws = text.len() - text.trim_start().len();
    let mut a = lead_ws;
    let mut b = text.trim_end().len();
    if a >= b {
        return Vec::new();
    }
    if text[a..b].starts_with('|') {
        a += 1;
    }
    if b > a && text[a..b].ends_with('|') {
        b -= 1;
    }
    let mut out = Vec::new();
    let mut cell_start = a;
    let inner = &text[a..b];
    let mut push = |from: usize, to: usize| {
        let raw = &text[from..to];
        let t_start = from + (raw.len() - raw.trim_start().len());
        let t_end = t_start + raw.trim().len();
        let cs = line.char_start + text[..t_start].chars().count();
        let ce = cs + text[t_start..t_end].chars().count();
        out.push(Cell {
            text: text[t_start..t_end].to_owned(),
            range: CharRange::new(cs, ce),
        });
    };
    for (i, c) in inner.char_indices() {
        if c == '|' {
            push(cell_start, a + i);
            cell_start = a + i + 1;
        }
    }
    push(cell_start, b);
    out
}

fn is_separator(cells: &[Cell]) -> bool {
    let mut any = false;
    for c in cells {
        if c.text.is_empty() {
            continue;
        }
        if !c.text.chars().all(|ch| ch == '-' || ch == ':') || !c.text.contains('-') {
            return false;
        }
        any = true;
    }
    any
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// Builds narration for one table while mapping cells to the source.
struct TableWriter<'a> {
    b: &'a mut SpokenBuilder,
    /// Source chars consumed so far (everything before is spoken or elided).
    cursor: usize,
}

impl TableWriter<'_> {
    fn skip_to(&mut self, pos: usize) {
        if pos > self.cursor {
            self.b.push_elided(CharRange::new(self.cursor, pos));
            self.cursor = pos;
        }
    }

    fn insert(&mut self, s: &str) {
        self.b.push_inserted(s, CharPos(self.cursor));
    }

    fn cell(&mut self, c: &Cell) {
        self.skip_to(c.range.start.0);
        self.b.push_literal(&c.text, c.range.start);
        self.cursor = c.range.end.0;
    }
}

/// star `_tables_to_narration(text, mode)`: Markdown pipe tables become
/// spoken sentences, with the cell text mapped to its source.
pub fn tables_to_narration(text: &str, mode: TableMode) -> (String, OffsetMap) {
    let lines = split_lines(text);
    let row = |i: usize| cells(text, lines[i]);
    let has_pipe = |i: usize| text[lines[i].start..lines[i].end].contains('|');
    let pipe_started = |i: usize| {
        let t = text[lines[i].start..lines[i].end].trim();
        t.starts_with('|') && t.contains('|')
    };
    let mut b = SpokenBuilder::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let gfm =
            has_pipe(i) && i + 1 < lines.len() && has_pipe(i + 1) && is_separator(&row(i + 1));
        if !(pipe_started(i) || gfm) {
            b.push_literal(&text[line.start..line.end], CharPos(line.char_start));
            if i + 1 < lines.len() {
                let nl = line.char_start + text[line.start..line.end].chars().count();
                b.push_literal("\n", CharPos(nl));
            }
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < lines.len()
            && if gfm {
                has_pipe(j) && !text[lines[j].start..lines[j].end].trim().is_empty()
            } else {
                pipe_started(j)
            }
        {
            j += 1;
        }
        let block_end = {
            let last = lines[j - 1];
            last.char_start + text[last.start..last.end].chars().count()
        };
        let mut w = TableWriter {
            b: &mut b,
            cursor: line.char_start,
        };
        narrate_block(&mut w, &(i..j).map(row).collect::<Vec<_>>(), mode);
        // star's block ends with an empty line.
        w.insert("\n");
        w.skip_to(block_end);
        if j < lines.len() {
            b.push_literal("\n", CharPos(block_end));
        }
        i = j;
    }
    b.finish()
}

fn narrate_block(w: &mut TableWriter<'_>, rows: &[Vec<Cell>], mode: TableMode) {
    if mode == TableMode::Skip {
        let n = rows
            .first()
            .map_or(0, |r| r.iter().filter(|c| !c.text.is_empty()).count());
        w.insert(&format!(
            "Table with {n} column{} \u{2014} skipped.",
            plural(n)
        ));
        return;
    }
    let rows: Vec<&Vec<Cell>> = rows.iter().filter(|r| !is_separator(r)).collect();
    let Some((header, data)) = rows.split_first() else {
        return;
    };
    let names: Vec<&Cell> = header.iter().filter(|c| !c.text.is_empty()).collect();
    match mode {
        TableMode::Flat => {
            for (n, r) in rows.iter().enumerate() {
                if n > 0 {
                    w.insert("\n");
                }
                let cs: Vec<&Cell> = r.iter().filter(|c| !c.text.is_empty()).collect();
                for (k, c) in cs.iter().enumerate() {
                    if k > 0 {
                        w.insert(".  ");
                    }
                    w.cell(c);
                }
                w.insert(".");
            }
        }
        TableMode::Structured | TableMode::Skip => {
            if names.is_empty() {
                w.insert("Table.");
            } else {
                w.insert(&format!(
                    "Table with {} column{}: ",
                    names.len(),
                    plural(names.len())
                ));
                for (k, c) in names.iter().enumerate() {
                    if k > 0 {
                        w.insert(", ");
                    }
                    w.cell(c);
                }
                w.insert(".");
            }
            for (ri, r) in data.iter().enumerate() {
                w.insert(&format!("\nRow {}: ", ri + 1));
                let mut first = true;
                for (col, c) in r.iter().enumerate() {
                    if c.text.is_empty() {
                        continue;
                    }
                    if !first {
                        w.insert(", ");
                    }
                    first = false;
                    if names.is_empty() {
                        w.cell(c);
                        continue;
                    }
                    let name = header
                        .get(col)
                        .filter(|h| !h.text.is_empty())
                        .map_or_else(|| format!("column {}", col + 1), |h| h.text.clone());
                    w.insert(&format!("{name} is "));
                    w.cell(c);
                }
                if first {
                    w.insert("empty");
                }
                w.insert(".");
            }
        }
    }
}
