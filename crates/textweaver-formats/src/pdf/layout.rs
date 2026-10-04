//! Page layout: glyphs into lines, lines into blocks and tables, running
//! headers and footers removed, and blocks put in reading order.
//!
//! Reading order is star's column-aware reconstruction
//! (`star/documents/pdf.py`, `_pdf_order_boxes`): blocks that span columns
//! divide the page into bands; within a band, columns are read left to
//! right, each top to bottom. Two refinements over star, both found on a
//! browser-printed two-column page: a gutter is a strip at least 1.5% of the
//! page wide that almost no line crosses (star required 4% and no crossing,
//! so a short title over the gutter hid the columns), confirmed by several
//! full lines of text on each side; and every block that crosses a gutter is
//! a band divider, not only those 55% of the page wide. Columns are found
//! before tables, so a table in one column never takes lines from the
//! other, and a justified line split at wide word gaps is joined again. Running headers and footers are
//! star's too: text in the top or bottom tenth of the page that recurs
//! (digits ignored) on at least half the pages (and at least three), and
//! bare page numbers there ("12", "Page 12", "12 of 340", "iv").

use std::collections::HashMap;

use super::fonts::{BOLD, ITALIC, MONO};
use super::interp::{Glyph, ImageBox, NO_MCID, PageContent};

/// A run of text on one baseline without a wide gap.
#[derive(Clone, Debug)]
pub(super) struct Line {
    pub x0: f32,
    pub x1: f32,
    /// Baseline.
    pub y: f32,
    /// Size of most of the line's chars.
    pub size: f32,
    pub text: String,
    /// Fractions of the chars that are bold, italic, monospaced.
    pub bold: f32,
    pub italic: f32,
    pub mono: f32,
    /// Marked-content id of the first glyph.
    pub mcid: u32,
}

impl Line {
    pub(super) fn top(&self) -> f32 {
        self.y - self.size * 0.8
    }

    pub(super) fn bottom(&self) -> f32 {
        self.y + self.size * 0.25
    }

    fn chars(&self) -> usize {
        self.text.chars().count()
    }
}

/// A table found on a page: rows of cells (each cell's lines joined).
#[derive(Clone, Debug)]
pub(super) struct Table {
    pub rows: Vec<Vec<String>>,
    /// True when the first row looks like a header (bold).
    pub header: bool,
}

/// What a block holds.
#[derive(Clone, Debug)]
pub(super) enum Content {
    /// Lines of text, top to bottom.
    Lines(Vec<Line>),
    Table(Table),
    /// An image with alternate text.
    Image(String),
    /// A form field: "Label: value".
    Field(String),
}

/// A rectangle of content on a page.
#[derive(Clone, Debug)]
pub(super) struct Block {
    pub x0: f32,
    pub x1: f32,
    pub top: f32,
    pub bottom: f32,
    pub content: Content,
}

impl Block {
    fn from_lines(lines: Vec<Line>) -> Block {
        let x0 = lines.iter().map(|l| l.x0).fold(f32::MAX, f32::min);
        let x1 = lines.iter().map(|l| l.x1).fold(f32::MIN, f32::max);
        let top = lines.iter().map(Line::top).fold(f32::MAX, f32::min);
        let bottom = lines.iter().map(Line::bottom).fold(f32::MIN, f32::max);
        Block {
            x0,
            x1,
            top,
            bottom,
            content: Content::Lines(lines),
        }
    }

    /// The block's text, lines joined by spaces (for margin matching).
    pub(super) fn text(&self) -> String {
        match &self.content {
            Content::Lines(lines) => lines
                .iter()
                .map(|l| l.text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            Content::Table(t) => t
                .rows
                .iter()
                .map(|r| r.join(" "))
                .collect::<Vec<_>>()
                .join(" "),
            Content::Image(alt) | Content::Field(alt) => alt.clone(),
        }
    }
}

/// One page, laid out.
#[derive(Debug, Default)]
pub(super) struct Page {
    pub width: f32,
    pub height: f32,
    /// Blocks in reading order (after [`order`]).
    pub blocks: Vec<Block>,
    /// Text columns, left to right (one when the page is not in columns).
    pub columns: Vec<(f32, f32)>,
}

/// A span of glyphs: consecutive in the content stream, on one baseline.
#[derive(Clone, Debug)]
struct Span {
    x0: f32,
    x1: f32,
    y: f32,
    size: f32,
    text: String,
    /// Chars by style (bold, italic, mono) and in total.
    styled: [usize; 3],
    chars: usize,
    /// Char counts by size (rounded to a tenth of a point).
    mcid: u32,
}

fn is_blank(s: &str) -> bool {
    s.chars().all(char::is_whitespace)
}

/// Bullets and other list-marker glyphs, including the Private Use Area
/// bullets Word writes with Symbol and Wingdings fonts.
pub(super) fn is_bullet(c: char) -> bool {
    matches!(
        c,
        '\u{2022}'
            | '\u{25e6}'
            | '\u{25aa}'
            | '\u{25ab}'
            | '\u{25cf}'
            | '\u{25cb}'
            | '\u{25a0}'
            | '\u{25a1}'
            | '\u{2023}'
            | '\u{2043}'
            | '\u{2219}'
            | '\u{00b7}'
            | '\u{27a2}'
            | '\u{2013}'
            | '\u{2014}'
            | '-'
            | '*'
            | '\u{f0b7}'
            | '\u{f0a7}'
            | '\u{f076}'
            | '\u{f0d8}'
            | '\u{f0fc}'
            | '\u{f06e}'
            | '\u{f0a8}'
    )
}

/// True when `s` is only a list marker ("•", "1.", "(a)", "iv)").
fn is_marker_only(s: &str) -> bool {
    let s = s.trim();
    let mut chars = s.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return is_bullet(c);
    }
    super::structure::numbered_marker(s)
        .is_some_and(|(label, rest)| rest.is_empty() && !label.is_empty())
}

/// Groups a page's glyphs into spans in content order.
fn spans(page: &PageContent) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    let mut pending_space = false;
    for g in &page.glyphs {
        let text = page.glyph_text(g);
        if is_blank(text) {
            pending_space = true;
            continue;
        }
        if let Some(cur) = out.last_mut()
            && continues(cur, g)
        {
            let gap = g.x - cur.x1;
            if (pending_space || gap > 0.12 * g.size.max(cur.size)) && !cur.text.ends_with(' ') {
                cur.text.push(' ');
            }
            push_glyph(cur, g, text);
        } else {
            let mut s = Span {
                x0: g.x,
                x1: g.x,
                y: g.y,
                size: g.size,
                text: String::new(),
                styled: [0; 3],
                chars: 0,
                mcid: g.mcid,
            };
            push_glyph(&mut s, g, text);
            out.push(s);
        }
        pending_space = false;
    }
    for s in &mut out {
        let t = s.text.trim_end().len();
        s.text.truncate(t);
    }
    out
}

fn continues(cur: &Span, g: &Glyph) -> bool {
    let size = cur.size.max(g.size);
    let ratio = cur.size.max(g.size) / cur.size.min(g.size).max(0.1);
    (g.y - cur.y).abs() <= 0.3 * size
        && ratio < 1.3
        && g.x >= cur.x1 - 0.5 * size
        && g.x - cur.x1 <= 0.9 * size
}

fn push_glyph(s: &mut Span, g: &Glyph, text: &str) {
    s.text.push_str(text);
    s.x1 = s.x1.max(g.x + g.w);
    let n = text.chars().count();
    s.chars += n;
    if g.style & BOLD != 0 {
        s.styled[0] += n;
    }
    if g.style & ITALIC != 0 {
        s.styled[1] += n;
    }
    if g.style & MONO != 0 {
        s.styled[2] += n;
    }
    // The span's size is its largest glyph's; superscripts join lines later.
    s.size = s.size.max(g.size);
}

/// Merges spans into lines: spans on one baseline (superscripts and
/// subscripts included) joined left to right unless a wide gap separates
/// them; a lone list marker joins the text after it.
fn lines(mut spans: Vec<Span>) -> Vec<Line> {
    spans.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x0.total_cmp(&b.x0)));
    let mut rows: Vec<Vec<Span>> = Vec::new();
    for s in spans {
        match rows.last_mut() {
            Some(row) if same_row(&row[0], &s) => row.push(s),
            _ => rows.push(vec![s]),
        }
    }
    let mut out = Vec::new();
    for mut row in rows {
        row.sort_by(|a, b| a.x0.total_cmp(&b.x0));
        // The row's baseline: its largest span's.
        let mut current: Option<(Span, Vec<Span>)> = None;
        for s in row {
            if let Some((head, parts)) = current.as_mut() {
                let last = parts.last().unwrap_or(head);
                let size = head.size.max(s.size);
                let gap = s.x0 - last.x1;
                let marker = parts.is_empty() && is_marker_only(&head.text);
                if gap <= size * 1.0 || (marker && gap <= size * 4.0) {
                    parts.push(s);
                    continue;
                }
            }
            if let Some((head, parts)) = current.take() {
                out.push(join_line(head, parts));
            }
            current = Some((s, Vec::new()));
        }
        if let Some((head, parts)) = current {
            out.push(join_line(head, parts));
        }
    }
    out
}

fn same_row(first: &Span, s: &Span) -> bool {
    let big = first.size.max(s.size);
    let small = first.size.min(s.size);
    let tolerance = if big / small.max(0.1) > 1.15 {
        0.45
    } else {
        0.3
    };
    (s.y - first.y).abs() <= tolerance * big
}

fn join_line(head: Span, parts: Vec<Span>) -> Line {
    let mut text = head.text.clone();
    let mut x1 = head.x1;
    let mut styled = head.styled;
    let mut chars = head.chars;
    let mut size_chars: Vec<(f32, usize)> = vec![(head.size, head.chars)];
    let mut y = head.y;
    let mut best = head.chars;
    for p in &parts {
        let gap = p.x0 - x1;
        if gap > 0.12 * p.size.max(head.size) || is_marker_only(&text) {
            text.push(' ');
        }
        text.push_str(&p.text);
        x1 = x1.max(p.x1);
        for (a, b) in styled.iter_mut().zip(p.styled) {
            *a += b;
        }
        chars += p.chars;
        size_chars.push((p.size, p.chars));
        if p.chars > best {
            best = p.chars;
            y = p.y;
        }
    }
    let size = size_chars
        .iter()
        .max_by_key(|(_, n)| *n)
        .map_or(head.size, |(s, _)| *s);
    let frac = |n: usize| n as f32 / chars.max(1) as f32;
    Line {
        x0: head.x0,
        x1,
        y,
        size,
        text,
        bold: frac(styled[0]),
        italic: frac(styled[1]),
        mono: frac(styled[2]),
        mcid: head.mcid,
    }
}

/// Finds tables among a page's lines: at least two consecutive rows of at
/// least two narrow cells each whose left edges line up in columns. Returns
/// the tables as blocks and the lines that are not in a table.
///
/// On a recognized page (`ocr`), word boxes wander by a few pixels and a
/// slightly tilted scan moves a row's baseline across the page, so rows
/// and columns are matched more loosely, and a first row of words over
/// rows of numbers is the header (a scan has no bold to tell it by).
fn tables(lines: Vec<Line>, page_width: f32, ocr: bool) -> (Vec<Block>, Vec<Line>) {
    let row_tol = if ocr { 0.5 } else { 0.3 };
    // Scanned tables are often ruled and padded: rows further apart.
    let pitch = if ocr { 3.5 } else { 2.6 };
    // Rows: lines sharing a baseline.
    let mut rows: Vec<Vec<Line>> = Vec::new();
    let mut lines = lines;
    if ocr {
        lines.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x0.total_cmp(&b.x0)));
    }
    for l in lines {
        match rows.last_mut() {
            Some(r) if (r[0].y - l.y).abs() <= row_tol * r[0].size.max(l.size) => r.push(l),
            _ => rows.push(vec![l]),
        }
    }
    if ocr {
        for r in &mut rows {
            r.sort_by(|a, b| a.x0.total_cmp(&b.x0));
        }
    }
    let narrow = |r: &Vec<Line>| {
        r.len() >= 2
            && r.iter()
                .all(|l| l.x1 - l.x0 <= 0.3 * page_width && l.chars() <= 40)
            && r.windows(2).all(|w| w[1].x0 - w[0].x1 >= w[0].size)
    };
    let mut blocks = Vec::new();
    let mut rest = Vec::new();
    let mut i = 0;
    while i < rows.len() {
        if !narrow(&rows[i]) {
            rest.append(&mut rows[i]);
            i += 1;
            continue;
        }
        // Extend while the next row is narrow, close below, and aligned.
        let mut j = i + 1;
        while j < rows.len()
            && narrow(&rows[j])
            && rows[j][0].y - rows[j - 1][0].y <= pitch * rows[j][0].size
            && aligned(&rows[i], &rows[j], ocr)
        {
            j += 1;
        }
        if j - i >= 2 && looks_tabular(&rows, i, j, page_width) {
            let table_rows: Vec<Vec<Line>> = rows[i..j].iter_mut().map(std::mem::take).collect();
            blocks.push(table_block(table_rows, ocr));
            i = j;
        } else {
            rest.append(&mut rows[i]);
            i += 1;
        }
    }
    (blocks, rest)
}

/// Rejects runs of short lines that are really text columns side by side:
/// table cells are short, rarely start in lowercase (prose continuing from
/// the line above does), and the rows around a table are not wide lines
/// starting where its columns start.
fn looks_tabular(rows: &[Vec<Line>], i: usize, j: usize, page_width: f32) -> bool {
    let cells: Vec<&Line> = rows[i..j].iter().flatten().collect();
    let n = cells.len().max(1);
    let mean = cells.iter().map(|l| l.chars()).sum::<usize>() / n;
    let lower = cells
        .iter()
        .filter(|l| l.text.chars().next().is_some_and(char::is_lowercase))
        .count();
    if mean > 20 || lower * 5 >= n * 2 {
        return false;
    }
    let starts: Vec<f32> = rows[i].iter().map(|l| l.x0).collect();
    // A neighbor row of two or more wide lines starting at these columns
    // is running text in columns.
    let near = |r: &Vec<Line>, y: f32| {
        r.first().is_some_and(|l| (l.y - y).abs() <= 2.5 * l.size)
            && r.iter()
                .filter(|l| {
                    l.x1 - l.x0 > 0.3 * page_width && starts.iter().any(|x| (x - l.x0).abs() <= 4.0)
                })
                .count()
                >= 2
    };
    let above = i
        .checked_sub(1)
        .is_some_and(|k| near(&rows[k], rows[i][0].y));
    let below = rows.get(j).is_some_and(|r| near(r, rows[j - 1][0].y));
    !(above || below)
}

/// True when most cells of `b` start at (or center on) a column of `a`.
fn aligned(a: &[Line], b: &[Line], ocr: bool) -> bool {
    let size = b.iter().map(|l| l.size).fold(0.0, f32::max);
    let tol = if ocr { (0.8 * size).max(4.0) } else { 4.0 };
    let hits = b
        .iter()
        .filter(|l| {
            a.iter().any(|c| {
                (c.x0 - l.x0).abs() <= tol
                    || (c.x1 - l.x1).abs() <= tol
                    || ((c.x0 + c.x1) / 2.0 - (l.x0 + l.x1) / 2.0).abs() <= tol
            })
        })
        .count();
    hits * 3 >= b.len() * 2
}

fn table_block(rows: Vec<Vec<Line>>, ocr: bool) -> Block {
    // Columns from the first row's cells; later cells go to the column whose
    // range they overlap most (or the nearest).
    let cols: Vec<(f32, f32)> = rows[0].iter().map(|l| (l.x0, l.x1)).collect();
    let digits = |l: &Line| l.text.chars().any(|c| c.is_ascii_digit());
    let header = if ocr {
        let numeric_rows = rows[1..].iter().filter(|r| r.iter().any(digits)).count();
        !rows[0].iter().any(digits) && numeric_rows * 2 >= rows.len() - 1
    } else {
        rows[0].iter().all(|l| l.bold >= 0.8) && rows[1..].iter().flatten().any(|l| l.bold < 0.5)
    };
    let x0 = rows.iter().flatten().map(|l| l.x0).fold(f32::MAX, f32::min);
    let x1 = rows.iter().flatten().map(|l| l.x1).fold(f32::MIN, f32::max);
    let top = rows[0].iter().map(Line::top).fold(f32::MAX, f32::min);
    let bottom = rows
        .last()
        .map_or(top, |r| r.iter().map(Line::bottom).fold(f32::MIN, f32::max));
    let mut out_rows = Vec::new();
    for r in &rows {
        let mut cells = vec![String::new(); cols.len()];
        for l in r {
            let cx = (l.x0 + l.x1) / 2.0;
            let k = cols
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    let da = if cx >= a.0 && cx <= a.1 {
                        0.0
                    } else {
                        (cx - (a.0 + a.1) / 2.0).abs()
                    };
                    let db = if cx >= b.0 && cx <= b.1 {
                        0.0
                    } else {
                        (cx - (b.0 + b.1) / 2.0).abs()
                    };
                    da.total_cmp(&db)
                })
                .map_or(0, |(k, _)| k);
            if !cells[k].is_empty() {
                cells[k].push(' ');
            }
            cells[k].push_str(&l.text);
        }
        out_rows.push(cells);
    }
    Block {
        x0,
        x1,
        top,
        bottom,
        content: Content::Table(Table {
            rows: out_rows,
            header,
        }),
    }
}

/// Style class used to keep headings out of the paragraphs next to them.
fn style_class(l: &Line) -> (bool, bool) {
    (l.bold >= 0.6, l.mono >= 0.8)
}

/// Groups lines into blocks: a line joins the block whose last line is just
/// above it (baseline distance at most 1.5 lines), overlaps it horizontally,
/// and has the same size and weight.
fn blocks(mut lines: Vec<Line>) -> Vec<Block> {
    lines.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x0.total_cmp(&b.x0)));
    let mut open: Vec<Vec<Line>> = Vec::new();
    for l in lines {
        let mut best: Option<(usize, f32)> = None;
        for (i, b) in open.iter().enumerate() {
            let Some(last) = b.last() else { continue };
            let dy = l.y - last.y;
            let size = last.size.max(l.size);
            let ratio = last.size.max(l.size) / last.size.min(l.size).max(0.1);
            let bx0 = b.iter().map(|x| x.x0).fold(f32::MAX, f32::min);
            let bx1 = b.iter().map(|x| x.x1).fold(f32::MIN, f32::max);
            let overlap = l.x1.min(bx1) - l.x0.max(bx0);
            if dy > 0.1 * size
                && dy <= 1.5 * size
                && overlap > 0.0
                && ratio < 1.12
                && style_class(last) == style_class(&l)
                && best.is_none_or(|(_, d)| dy < d)
            {
                best = Some((i, dy));
            }
        }
        match best {
            Some((i, _)) => open[i].push(l),
            None => open.push(vec![l]),
        }
    }
    open.into_iter()
        .filter(|b| !b.is_empty())
        .map(Block::from_lines)
        .collect()
}

/// Lays out one page (before running-head removal and ordering).
pub(super) fn layout(page: &PageContent, image_alts: &HashMap<u32, String>) -> Page {
    let segs = lines(spans(page));
    let columns = columns(&segs, page.width);
    // Lines by column (`None`: crossing a gutter).
    let mut groups: Vec<Vec<Line>> = vec![Vec::new(); columns.len() + 1];
    for l in segs {
        let k = column_of(&columns, l.x0, l.x1).unwrap_or(columns.len());
        groups[k].push(l);
    }
    let mut out = Vec::new();
    let mut rest = Vec::new();
    for (k, group) in groups.into_iter().enumerate() {
        if k == columns.len() {
            rest.extend(group);
            continue;
        }
        let (tables, others) = tables(group, page.width, page.ocr);
        out.extend(tables);
        rest.extend(rejoin(others));
    }
    out.extend(blocks(rest));
    for img in &page.images {
        if let Some(alt) = image_alt(img, image_alts) {
            out.push(Block {
                x0: img.x0,
                x1: img.x1,
                top: img.y0,
                bottom: img.y1,
                content: Content::Image(alt),
            });
        }
    }
    for f in &page.fields {
        out.push(Block {
            x0: f.x0,
            x1: f.x1,
            top: f.y0,
            bottom: f.y1,
            content: Content::Field(f.text.clone()),
        });
    }
    Page {
        width: page.width,
        height: page.height,
        blocks: out,
        columns,
    }
}

/// The column holding `x0..x1`, if it lies within one (with a little slack).
fn column_of(columns: &[(f32, f32)], x0: f32, x1: f32) -> Option<usize> {
    columns
        .iter()
        .position(|&(lo, hi)| x0 >= lo - 3.0 && x1 <= hi + 3.0)
}

/// Text columns of a page, left to right: the covered x-ranges between
/// gutters. A gutter is an interior strip at least 1.5% of the page wide
/// that lines narrower than 55% of the page (almost) never cross, with at
/// least three lines of real text (18% of the page wide) on each side.
fn columns(lines: &[Line], width: f32) -> Vec<(f32, f32)> {
    const N: usize = 400;
    let single = vec![(0.0, width)];
    if width <= 0.0 || lines.is_empty() {
        return single;
    }
    let binw = width / N as f32;
    let bin = |x: f32| ((x / binw) as isize).clamp(0, N as isize - 1) as usize;
    let mut count = [0u32; N];
    for l in lines.iter().filter(|l| l.x1 - l.x0 < 0.55 * width) {
        for c in &mut count[bin(l.x0)..=bin(l.x1)] {
            *c += 1;
        }
    }
    let max = count.iter().copied().max().unwrap_or(0);
    if max == 0 {
        return single;
    }
    let low = (max as f32 * 0.1).floor() as u32;
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < N {
        if count[i] > low {
            let mut j = i;
            while j < N && count[j] > low {
                j += 1;
            }
            runs.push((i, j));
            i = j;
        } else {
            i += 1;
        }
    }
    let min_gap = (0.015 * N as f32).ceil() as usize;
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for r in runs {
        match merged.last_mut() {
            Some(last) if r.0 - last.1 < min_gap => last.1 = r.1,
            _ => merged.push(r),
        }
    }
    let cols: Vec<(f32, f32)> = merged
        .iter()
        .map(|&(a, b)| (a as f32 * binw, b as f32 * binw))
        .filter(|&(lo, hi)| {
            lines
                .iter()
                .filter(|l| l.x0 >= lo - 3.0 && l.x1 <= hi + 3.0 && l.x1 - l.x0 >= 0.18 * width)
                .count()
                >= 3
        })
        .collect();
    if cols.len() <= 1 {
        return single;
    }
    cols
}

/// Joins lines of one column that share a baseline (a justified line split
/// at wide word gaps).
fn rejoin(mut lines: Vec<Line>) -> Vec<Line> {
    lines.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x0.total_cmp(&b.x0)));
    let mut out: Vec<Line> = Vec::with_capacity(lines.len());
    for l in lines {
        if let Some(last) = out.last_mut()
            && (l.y - last.y).abs() <= 0.3 * l.size.max(last.size)
            && (l.size - last.size).abs() <= 0.15 * l.size.max(last.size)
            && l.x0 >= last.x1
        {
            let (a, b) = (
                last.text.chars().count() as f32,
                l.text.chars().count() as f32,
            );
            let total = (a + b).max(1.0);
            last.bold = (last.bold * a + l.bold * b) / total;
            last.italic = (last.italic * a + l.italic * b) / total;
            last.mono = (last.mono * a + l.mono * b) / total;
            last.text.push(' ');
            last.text.push_str(&l.text);
            last.x1 = l.x1;
            continue;
        }
        out.push(l);
    }
    out
}

fn image_alt(img: &ImageBox, alts: &HashMap<u32, String>) -> Option<String> {
    let alt = img.alt.clone().or_else(|| {
        (img.mcid != NO_MCID)
            .then(|| alts.get(&img.mcid).cloned())
            .flatten()
    })?;
    let alt = alt.split_whitespace().collect::<Vec<_>>().join(" ");
    (!alt.is_empty()).then_some(alt)
}

/// star's page-number pattern: "12", "Page 12", "12 of 340", "12/340", "iv".
pub(super) fn is_page_number(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    let t = t.strip_prefix("page").map_or(t.as_str(), str::trim_start);
    let digits = |s: &str| !s.is_empty() && s.len() <= 4 && s.chars().all(|c| c.is_ascii_digit());
    if digits(t) {
        return true;
    }
    for sep in [" of ", "/", " / "] {
        if let Some((a, b)) = t.split_once(sep)
            && digits(a.trim())
            && digits(b.trim())
        {
            return true;
        }
    }
    !t.is_empty() && t.len() <= 7 && t.chars().all(|c| "ivxlcdm".contains(c))
}

/// Normalizes margin text so it matches across pages ("Page 3" = "Page 4").
fn norm_margin(text: &str) -> String {
    let t = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let mut out = String::with_capacity(t.len());
    let mut in_digits = false;
    for c in t.chars() {
        if c.is_ascii_digit() {
            if !in_digits {
                out.push('#');
            }
            in_digits = true;
        } else {
            out.push(c);
            in_digits = false;
        }
    }
    out
}

fn in_margin(b: &Block, height: f32) -> bool {
    b.top >= 0.9 * height || b.bottom <= 0.1 * height
}

/// Removes running headers and footers and bare page numbers (star's
/// `_pdf_running_heads_feet` and `_pdf_is_running`).
pub(super) fn remove_running(pages: &mut [Page]) {
    let mut counts: HashMap<String, usize> = HashMap::new();
    if pages.len() >= 3 {
        for p in pages.iter() {
            let mut seen = std::collections::HashSet::new();
            for b in p.blocks.iter().filter(|b| in_margin(b, p.height)) {
                let key = norm_margin(&b.text());
                if !key.is_empty() && seen.insert(key.clone()) {
                    *counts.entry(key).or_default() += 1;
                }
            }
        }
    }
    let threshold = 3.max(pages.len().div_ceil(2));
    for p in pages.iter_mut() {
        let h = p.height;
        p.blocks.retain(|b| {
            if !in_margin(b, h) || matches!(b.content, Content::Image(_) | Content::Field(_)) {
                return true;
            }
            let text = b.text();
            if is_page_number(&text) {
                return false;
            }
            counts
                .get(&norm_margin(&text))
                .is_none_or(|&n| n < threshold)
        });
    }
}

/// Column x-ranges from a vertical projection of the blocks' x-extents
/// (star's `_pdf_detect_columns`), for pages where [`columns`] found none.
fn detect_columns(blocks: &[&Block], page_width: f32) -> Vec<(f32, f32)> {
    if blocks.is_empty() || page_width <= 0.0 {
        return vec![(0.0, page_width)];
    }
    const N: usize = 200;
    let binw = page_width / N as f32;
    let mut covered = [false; N];
    for b in blocks {
        let lo = ((b.x0 / binw) as isize).clamp(0, N as isize - 1) as usize;
        let hi = ((b.x1 / binw) as isize).clamp(0, N as isize - 1) as usize;
        for c in covered.iter_mut().take(hi + 1).skip(lo) {
            *c = true;
        }
    }
    let mut regions: Vec<(f32, f32)> = Vec::new();
    let mut i = 0;
    while i < N {
        if covered[i] {
            let mut j = i;
            while j < N && covered[j] {
                j += 1;
            }
            regions.push((i as f32 * binw, j as f32 * binw));
            i = j;
        } else {
            i += 1;
        }
    }
    if regions.is_empty() {
        return vec![(0.0, page_width)];
    }
    let gutter = 0.04 * page_width;
    let mut merged = vec![regions[0]];
    for (lo, hi) in regions.into_iter().skip(1) {
        let last = merged.len() - 1;
        if lo - merged[last].1 < gutter {
            merged[last].1 = hi;
        } else {
            merged.push((lo, hi));
        }
    }
    merged
}

/// Puts a page's blocks in reading order (star's `_pdf_order_boxes`, with
/// the gutter refinements in the module docs).
pub(super) fn order(page: &mut Page) {
    let width = page.width;
    let mut blocks = std::mem::take(&mut page.blocks);
    if blocks.len() <= 1 {
        page.blocks = blocks;
        return;
    }
    let full_w = 0.55 * width;
    let ranges = if page.columns.len() > 1 {
        page.columns.clone()
    } else {
        let cols: Vec<&Block> = blocks.iter().filter(|b| b.x1 - b.x0 < full_w).collect();
        detect_columns(&cols, width)
    };
    if ranges.len() <= 1 {
        blocks.sort_by(|a, b| a.top.total_cmp(&b.top).then(a.x0.total_cmp(&b.x0)));
        page.blocks = blocks;
        return;
    }
    let gutters: Vec<(f32, f32)> = ranges.windows(2).map(|w| (w[0].1, w[1].0)).collect();
    let is_divider =
        |b: &Block| b.x1 - b.x0 >= full_w || gutters.iter().any(|&(lo, hi)| b.x0 < lo && b.x1 > hi);
    let mut dividers: Vec<f32> = blocks
        .iter()
        .filter(|b| is_divider(b))
        .map(|b| (b.top + b.bottom) / 2.0)
        .collect();
    dividers.sort_by(f32::total_cmp);
    let col_index = |b: &Block| {
        let cx = (b.x0 + b.x1) / 2.0;
        ranges
            .iter()
            .position(|&(lo, hi)| lo <= cx && cx <= hi)
            .unwrap_or_else(|| {
                ranges
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        let ca = ((a.0 + a.1) / 2.0 - cx).abs();
                        let cb = ((b.0 + b.1) / 2.0 - cx).abs();
                        ca.total_cmp(&cb)
                    })
                    .map_or(0, |(i, _)| i)
            })
    };
    let band = |b: &Block| {
        let cy = (b.top + b.bottom) / 2.0;
        dividers.iter().filter(|&&d| d < cy).count()
    };
    blocks.sort_by(|a, b| {
        let key = |x: &Block| {
            let div = is_divider(x);
            (
                band(x),
                usize::from(div),
                if div { 0 } else { col_index(x) },
            )
        };
        key(a)
            .cmp(&key(b))
            .then(a.top.total_cmp(&b.top))
            .then(a.x0.total_cmp(&b.x0))
    });
    page.blocks = blocks;
}

/// Char-weighted most common line size (to half a point) and whether most
/// body-size text is bold.
pub(super) fn body_style(pages: &[Page]) -> (f32, bool) {
    let mut by_size: HashMap<i32, (usize, usize)> = HashMap::new();
    for p in pages {
        for b in &p.blocks {
            if let Content::Lines(lines) = &b.content {
                for l in lines {
                    let key = (l.size * 2.0).round() as i32;
                    let e = by_size.entry(key).or_default();
                    let n = l.chars();
                    e.0 += n;
                    e.1 += (l.bold * n as f32).round() as usize;
                }
            }
        }
    }
    by_size
        .into_iter()
        .max_by_key(|(_, (n, _))| *n)
        .map_or((10.0, false), |(k, (n, b))| (k as f32 / 2.0, b * 2 > n))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(x0: f32, x1: f32, top: f32, bottom: f32, text: &str) -> Block {
        Block {
            x0,
            x1,
            top,
            bottom,
            content: Content::Image(text.to_owned()),
        }
    }

    /// A page of recognized words: one glyph per word, as OCR places them.
    fn recognized(words: &[(&str, f32, f32)]) -> PageContent {
        let mut page = PageContent {
            width: 600.0,
            height: 800.0,
            ocr: true,
            ..PageContent::default()
        };
        for &(w, x, y) in words {
            let start = page.text.len() as u32;
            page.text.push_str(w);
            page.glyphs.push(Glyph {
                x,
                y,
                w: w.chars().count() as f32 * 5.5,
                size: 10.0,
                start,
                len: w.len() as u32,
                style: 0,
                mcid: NO_MCID,
            });
        }
        page
    }

    #[test]
    fn a_recognized_table_keeps_its_rows_and_columns() {
        // Columns that wander by a few points, rows by a point or two (a
        // slightly tilted scan), and no bold anywhere.
        let page = recognized(&[
            ("Name", 100.0, 200.0),
            ("Year", 252.0, 201.0),
            ("Score", 398.0, 202.0),
            ("Ada", 103.0, 230.0),
            ("2024", 247.0, 231.5),
            ("91", 401.0, 232.0),
            ("Bo", 98.0, 260.0),
            ("2025", 254.0, 261.0),
            ("87", 396.0, 262.5),
            ("Cy", 101.0, 290.0),
            ("2026", 250.0, 291.0),
            ("78", 402.0, 292.0),
        ]);
        let laid = layout(&page, &HashMap::new());
        let tables: Vec<&Table> = laid
            .blocks
            .iter()
            .filter_map(|b| match &b.content {
                Content::Table(t) => Some(t),
                _ => None,
            })
            .collect();
        assert_eq!(tables.len(), 1, "{:?}", laid.blocks);
        let t = tables[0];
        assert!(t.header);
        assert_eq!(t.rows.len(), 4);
        assert_eq!(t.rows[0], ["Name", "Year", "Score"]);
        assert_eq!(t.rows[2], ["Bo", "2025", "87"]);
    }

    #[test]
    fn page_numbers_like_star() {
        for t in ["12", "Page 12", "12 of 340", "12/340", "iv", " page 3 "] {
            assert!(is_page_number(t), "{t}");
        }
        for t in ["Chapter 12", "12 Angry Men", "", "12345"] {
            assert!(!is_page_number(t), "{t}");
        }
        assert_eq!(norm_margin("Page  3 of 10"), "page # of #");
    }

    #[test]
    fn two_columns_read_left_then_right_with_bands() {
        let mut page = Page {
            width: 600.0,
            height: 800.0,
            columns: Vec::new(),
            blocks: vec![
                block(320.0, 560.0, 100.0, 300.0, "right top"),
                block(40.0, 280.0, 100.0, 300.0, "left top"),
                block(40.0, 560.0, 40.0, 60.0, "title"),
                block(40.0, 560.0, 320.0, 340.0, "band"),
                block(320.0, 560.0, 360.0, 500.0, "right bottom"),
                block(40.0, 280.0, 360.0, 500.0, "left bottom"),
            ],
        };
        order(&mut page);
        let texts: Vec<String> = page.blocks.iter().map(Block::text).collect();
        assert_eq!(
            texts,
            [
                "title",
                "left top",
                "right top",
                "band",
                "left bottom",
                "right bottom"
            ]
        );
    }
}
