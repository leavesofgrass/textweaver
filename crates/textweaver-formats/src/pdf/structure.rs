//! From laid-out blocks to document structure: paragraphs (wrapped lines
//! joined, line-end hyphens removed, paragraphs continued across columns
//! and pages), headings, lists, code, tables, and images, emitted into the
//! canonical-text builder with `PageBreak` and `SectionBreak` markers.
//!
//! Headings are found, in order of trust, from a tagged PDF's structure
//! (`H1`–`H6`), from font size (short lines at least 15% larger than the
//! body text; each larger size is a higher level), from weight (a short,
//! single bold line when the body is not bold), from numbering ("2.3
//! Methods", "Chapter 4") on a bold or larger line, and from the document
//! outline (bookmarks), which also yields `SectionBreak` markers.

use std::collections::HashMap;

use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{HEADER_ROW_LABEL, Marker};

use super::layout::{Block, Content, Line, Page, Table, is_bullet};
use crate::builder::{Builder, OpenId};

/// A list marker at the start of `s`: `1.`, `12)`, `(a)`, `b.`, `iv)`,
/// followed by whitespace. Returns the label and the text after it.
pub(super) fn numbered_marker(s: &str) -> Option<(String, &str)> {
    let s = s.trim_start();
    let (inner, rest, paren) = if let Some(r) = s.strip_prefix('(') {
        let close = r.find(')')?;
        (&r[..close], &r[close + 1..], true)
    } else {
        let end = s.find(['.', ')'])?;
        (&s[..end], &s[end + 1..], false)
    };
    let ok = !inner.is_empty()
        && (inner.len() <= 3 && inner.chars().all(|c| c.is_ascii_digit())
            || inner.len() == 1 && inner.chars().all(|c| c.is_ascii_alphabetic())
            || inner.len() <= 5 && inner.chars().all(|c| "ivxlcdmIVXLCDM".contains(c)));
    if !ok {
        return None;
    }
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let label = if paren {
        format!("({inner})")
    } else {
        s[..inner.len() + 1].to_owned()
    };
    Some((label, rest.trim_start()))
}

/// A bullet at the start of `s` followed by whitespace; the text after it.
fn bullet_marker(s: &str) -> Option<&str> {
    let mut chars = s.char_indices();
    let (_, c) = chars.next()?;
    if !is_bullet(c) {
        return None;
    }
    let (i, next) = chars.next()?;
    next.is_whitespace().then(|| s[i..].trim_start())
}

/// The depth of a heading number: "2" → 1, "2.3" → 2, "Chapter 4" → 1.
fn heading_number(s: &str) -> Option<usize> {
    let first = s.split_whitespace().next()?;
    let rest_starts_upper = s
        .split_whitespace()
        .nth(1)
        .and_then(|w| w.chars().next())
        .is_some_and(|c| c.is_uppercase());
    let lower = first.to_lowercase();
    if ["chapter", "section", "part", "appendix", "lesson", "unit"].contains(&lower.as_str()) {
        return s.split_whitespace().nth(1).map(|_| 1);
    }
    let trimmed = first.trim_end_matches('.');
    let parts: Vec<&str> = trimmed.split('.').collect();
    let numeric = parts
        .iter()
        .all(|p| !p.is_empty() && p.len() <= 3 && p.chars().all(|c| c.is_ascii_digit()));
    let roman = parts.len() == 1
        && trimmed.len() <= 5
        && trimmed.chars().all(|c| "IVXLC".contains(c))
        && first.ends_with('.');
    ((numeric || roman) && rest_starts_upper).then_some(parts.len())
}

/// What a unit is.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Kind {
    Paragraph,
    Heading(u8),
    /// A list item: the x of its marker from the column's text margin
    /// (for nesting) and its label.
    Item {
        marker_x: f32,
        label: Option<String>,
    },
    Code,
    Table(TableRows),
    Image,
    /// A form field's line: "Label: value".
    Field,
    /// A caption found by its pattern ("Figure 3.", "Table 2:") or its
    /// tag; `figure` when it describes a picture rather than a table.
    Caption {
        figure: bool,
    },
}

/// Table rows and whether the first is a header.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct TableRows {
    pub rows: Vec<Vec<String>>,
    pub header: bool,
}

impl From<&Table> for TableRows {
    fn from(t: &Table) -> Self {
        TableRows {
            rows: t.rows.clone(),
            header: t.header,
        }
    }
}

/// Part of a unit's text on one page.
#[derive(Clone, Debug)]
pub(super) struct Piece {
    /// 0-based page.
    pub page: usize,
    pub text: String,
    /// Joined to the previous piece without a space (a word hyphenated
    /// across a column or page end).
    pub glue: bool,
}

impl Piece {
    fn new(page: usize, text: String) -> Self {
        Piece {
            page,
            text,
            glue: false,
        }
    }
}

/// A paragraph-sized piece of the document.
#[derive(Clone, Debug)]
pub(super) struct Unit {
    pub kind: Kind,
    /// Text pieces, each on one page.
    pub pieces: Vec<Piece>,
    pub size: f32,
    pub bold: f32,
    pub italic: f32,
    pub lines: usize,
    pub mcid: u32,
    /// Text with any list marker (for heading and outline matching).
    pub full: String,
    /// Top of the unit's block, as a fraction of the page height.
    pub top: f32,
    /// A note at the foot of the page (small text below the body).
    pub note: bool,
}

impl Unit {
    pub(super) fn page(&self) -> usize {
        self.pieces.first().map_or(0, |p| p.page)
    }

    pub(super) fn text(&self) -> String {
        let mut s = String::new();
        for p in &self.pieces {
            if !s.is_empty() && !p.glue {
                s.push(' ');
            }
            s.push_str(&p.text);
        }
        s
    }

    fn last_text(&self) -> &str {
        self.pieces.last().map_or("", |p| p.text.as_str())
    }
}

/// Appends `next` to `acc` as the next line of the same paragraph: a
/// line-end hyphen before a lowercase letter is removed (typesetting
/// hyphenation), a soft hyphen always; otherwise a space.
pub(super) fn join_text(acc: &mut String, next: &str) {
    let next = next.trim_start();
    if acc.ends_with('\u{ad}') {
        acc.pop();
        acc.push_str(next);
        return;
    }
    let hyphen = acc.ends_with('-') || acc.ends_with('\u{2010}');
    let before = acc
        .trim_end_matches(['-', '\u{2010}'])
        .chars()
        .next_back()
        .is_some_and(char::is_alphabetic);
    let lower_next = next.chars().next().is_some_and(char::is_lowercase);
    if hyphen && before && lower_next {
        acc.pop();
        acc.push_str(next);
    } else if hyphen && before {
        acc.push_str(next);
    } else {
        if !acc.is_empty() && !acc.ends_with(' ') {
            acc.push(' ');
        }
        acc.push_str(next);
    }
}

fn ends_sentence(s: &str) -> bool {
    let t = s
        .trim_end()
        .trim_end_matches(['"', '\u{201d}', '\u{2019}', ')', ']', '\'']);
    t.ends_with(['.', '!', '?', ':', ';'])
}

/// True when a tagged PDF says this marked content is a list item.
fn tagged_item(cx: &Context<'_>, page: usize, mcid: u32) -> bool {
    cx.roles
        .get(&(page, mcid))
        .is_some_and(|r| matches!(r.as_str(), "LI" | "LBody"))
}

/// Splits a block's lines into units; `margin` is the left text edge of
/// the block's column.
fn block_units(block: &Block, page: usize, margin: f32, cx: &Context<'_>, out: &mut Vec<Unit>) {
    let lines = match &block.content {
        Content::Table(t) => {
            out.push(Unit {
                kind: Kind::Table(t.into()),
                pieces: vec![Piece::new(page, String::new())],
                size: 0.0,
                bold: 0.0,
                italic: 0.0,
                lines: t.rows.len(),
                mcid: u32::MAX,
                full: String::new(),
                top: 0.0,
                note: false,
            });
            return;
        }
        Content::Field(text) => {
            out.push(Unit {
                kind: Kind::Field,
                pieces: vec![Piece::new(page, text.clone())],
                size: 0.0,
                bold: 0.0,
                italic: 0.0,
                lines: 1,
                mcid: u32::MAX,
                full: text.clone(),
                top: 0.0,
                note: false,
            });
            return;
        }
        Content::Image(alt) => {
            out.push(Unit {
                kind: Kind::Image,
                pieces: vec![Piece::new(page, alt.clone())],
                size: 0.0,
                bold: 0.0,
                italic: 0.0,
                lines: 1,
                mcid: u32::MAX,
                full: alt.clone(),
                top: 0.0,
                note: false,
            });
            return;
        }
        Content::Lines(lines) => lines,
    };
    let mono = lines.iter().filter(|l| l.mono >= 0.8).count();
    if mono * 5 >= lines.len() * 4 && !lines.is_empty() {
        let size = lines[0].size.max(1.0);
        let text = lines
            .iter()
            .map(|l| {
                let indent = ((l.x0 - block.x0) / (0.6 * size)).round().max(0.0) as usize;
                format!("{}{}", " ".repeat(indent.min(40)), l.text)
            })
            .collect::<Vec<_>>()
            .join("\n");
        out.push(Unit {
            kind: Kind::Code,
            full: text.clone(),
            top: 0.0,
            note: false,
            pieces: vec![Piece::new(page, text)],
            size,
            bold: 0.0,
            italic: 0.0,
            lines: lines.len(),
            mcid: lines[0].mcid,
        });
        return;
    }
    let left = block.x0;
    let right = block.x1;
    let mut current: Option<Unit> = None;
    let mut prev: Option<&Line> = None;
    for line in lines {
        let text = line.text.trim();
        let marker = bullet_marker(text)
            .map(|rest| (None, rest))
            .or_else(|| {
                numbered_marker(text)
                    .filter(|(_, r)| !r.is_empty())
                    .map(|(l, r)| (Some(l), r))
            })
            .or_else(|| {
                // A tagged list item whose bullet is drawn, not written.
                (tagged_item(cx, page, line.mcid) && prev.is_none_or(|p| p.mcid != line.mcid))
                    .then_some((None, text))
            });
        let starts_new = match (&current, prev, &marker) {
            (None, _, _) | (_, _, Some(_)) => true,
            (Some(u), Some(p), None) => match &u.kind {
                Kind::Item { marker_x, .. } => line.x0 - margin < marker_x + 0.5 * line.size,
                _ => {
                    let indented =
                        line.x0 > left + 0.8 * line.size && p.x0 <= left + 0.3 * line.size;
                    // Ragged-right lines vary; only a clearly short line ends
                    // a paragraph.
                    let short_prev = p.x1 < left + 0.6 * (right - left) && ends_sentence(&p.text);
                    indented || short_prev
                }
            },
            _ => true,
        };
        if starts_new {
            if let Some(u) = current.take() {
                out.push(u);
            }
            let (kind, body) = match marker {
                Some((label, rest)) => (
                    Kind::Item {
                        marker_x: line.x0 - margin,
                        label,
                    },
                    rest.to_owned(),
                ),
                None => (Kind::Paragraph, text.to_owned()),
            };
            current = Some(Unit {
                kind,
                pieces: vec![Piece::new(page, body)],
                size: line.size,
                bold: line.bold,
                italic: line.italic,
                lines: 1,
                mcid: line.mcid,
                full: text.to_owned(),
                top: 0.0,
                note: false,
            });
        } else if let Some(u) = current.as_mut() {
            if let Some(last) = u.pieces.last_mut() {
                join_text(&mut last.text, text);
            }
            join_text(&mut u.full, text);
            let n = u.lines as f32;
            u.bold = (u.bold * n + line.bold) / (n + 1.0);
            u.italic = (u.italic * n + line.italic) / (n + 1.0);
            u.lines += 1;
        }
        prev = Some(line);
    }
    if let Some(u) = current {
        out.push(u);
    }
}

/// Each column's range with the leftmost text in it.
fn column_margins(p: &Page) -> Vec<(f32, f32, f32)> {
    let cols = if p.columns.is_empty() {
        vec![(0.0, p.width)]
    } else {
        p.columns.clone()
    };
    cols.into_iter()
        .map(|(lo, hi)| {
            let m = p
                .blocks
                .iter()
                .filter(|b| b.x0 >= lo - 3.0 && b.x1 <= hi + 3.0)
                .map(|b| b.x0)
                .fold(f32::MAX, f32::min);
            (lo, hi, if m == f32::MAX { lo } else { m })
        })
        .collect()
}

/// Facts the classifier uses.
pub(super) struct Context<'a> {
    pub body_size: f32,
    pub body_bold: bool,
    /// Tagged-PDF structure roles by (page, mcid): "H1", "P", ...
    pub roles: &'a HashMap<(usize, u32), String>,
    /// Outline entries: (title, page, depth).
    pub outline: &'a [(String, usize, u8)],
}

fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Why a unit is a heading, for level assignment.
enum Why {
    Tag(u8),
    Size,
    Weight,
    Number,
    Outline(u8),
}

/// Units from every page's ordered blocks, classified and joined across
/// columns and pages. Also returns, for each outline entry, the unit it
/// starts at.
pub(super) fn units(pages: &[Page], cx: &Context<'_>) -> (Vec<Unit>, Vec<(usize, usize)>) {
    let mut raw = Vec::new();
    for (pi, p) in pages.iter().enumerate() {
        let margins = column_margins(p);
        for b in &p.blocks {
            let margin = margins
                .iter()
                .find(|(lo, hi, _)| b.x0 >= lo - 3.0 && b.x1 <= hi + 3.0)
                .map_or_else(|| margins.iter().map(|m| m.2).fold(b.x0, f32::min), |m| m.2);
            let first = raw.len();
            block_units(b, pi, margin, cx, &mut raw);
            let top = if p.height > 0.0 {
                b.top / p.height
            } else {
                0.0
            };
            for u in &mut raw[first..] {
                u.top = top;
            }
        }
    }
    mark_notes(&mut raw, cx.body_size);

    // Outline entries matched to units.
    let mut outline_at: Vec<Option<usize>> = vec![None; cx.outline.len()];
    let mut outline_depth: HashMap<usize, u8> = HashMap::new();
    let mut from = 0;
    for (k, (title, page, depth)) in cx.outline.iter().enumerate() {
        let want = norm(title);
        if want.is_empty() {
            continue;
        }
        let found = raw.iter().enumerate().skip(from).find(|(_, u)| {
            u.page() + 1 >= *page
                && u.page() <= page + 1
                && u.lines <= 4
                && !matches!(u.kind, Kind::Table(_) | Kind::Image | Kind::Code)
                && {
                    let have = norm(&u.full);
                    !have.is_empty()
                        && (have == want
                            || have.starts_with(&want)
                            || want.starts_with(&have) && have.len() * 2 >= want.len())
                }
        });
        if let Some((i, _)) = found {
            outline_at[k] = Some(i);
            outline_depth.entry(i).or_insert(*depth);
            from = i;
        }
    }

    // Heading candidates.
    let mut why: Vec<Option<Why>> = Vec::with_capacity(raw.len());
    for (i, u) in raw.iter().enumerate() {
        why.push(heading_reason(u, cx, outline_depth.get(&i).copied()));
    }
    // Levels: tags as given; sizes ranked largest first; weight after them;
    // numbers deepen within a size; outline-only by depth.
    let mut sizes: Vec<i32> = raw
        .iter()
        .zip(&why)
        .filter(|(_, w)| matches!(w, Some(Why::Size)))
        .map(|(u, _)| (u.size * 2.0).round() as i32)
        .collect();
    sizes.sort_unstable_by(|a, b| b.cmp(a));
    sizes.dedup();
    let size_level = |u: &Unit| {
        let k = (u.size * 2.0).round() as i32;
        sizes
            .iter()
            .position(|&s| s == k)
            .map_or(sizes.len(), |p| p)
            + 1
    };
    let weight_level = sizes.len() + 1;
    let mut min_depth: HashMap<usize, usize> = HashMap::new();
    for (u, w) in raw.iter().zip(&why) {
        if w.is_some()
            && let Some(d) = heading_number(&u.full)
        {
            let lvl = match w {
                Some(Why::Size) => size_level(u),
                _ => weight_level,
            };
            let e = min_depth.entry(lvl).or_insert(d);
            *e = (*e).min(d);
        }
    }
    let mut offsets = Vec::new();
    let mut levels: Vec<Option<u8>> = Vec::with_capacity(raw.len());
    for (i, (u, w)) in raw.iter().zip(&why).enumerate() {
        let base = match w {
            None => None,
            Some(Why::Tag(l)) => Some(usize::from(*l)),
            Some(Why::Size) => Some(size_level(u)),
            Some(Why::Weight | Why::Number) => Some(weight_level),
            Some(Why::Outline(_)) => None,
        };
        let level = base.map(|b| {
            let extra = heading_number(&u.full)
                .zip(min_depth.get(&b))
                .map_or(0, |(d, m)| d.saturating_sub(*m));
            (b + extra).clamp(1, 6)
        });
        if let (Some(l), Some(d)) = (level, outline_depth.get(&i)) {
            offsets.push(l as i32 - i32::from(*d));
        }
        levels.push(level.map(|l| l as u8));
    }
    offsets.sort_unstable();
    let offset = offsets.get(offsets.len() / 2).copied().unwrap_or(0);
    for (i, w) in why.iter().enumerate() {
        if let Some(Why::Outline(d)) = w {
            levels[i] = Some((i32::from(*d) + offset).clamp(1, 6) as u8);
        }
    }

    // Apply heading kinds, then join paragraphs across blocks.
    let mut out: Vec<Unit> = Vec::with_capacity(raw.len());
    let mut index_map = vec![0usize; raw.len()];
    for (i, (mut u, level)) in raw.into_iter().zip(levels).enumerate() {
        if let Some(l) = level {
            u.kind = Kind::Heading(l);
            let full = u.full.clone();
            u.pieces = vec![Piece::new(u.page(), full)];
        } else if u.kind == Kind::Paragraph && !u.note {
            let tagged = cx
                .roles
                .get(&(u.page(), u.mcid))
                .is_some_and(|r| r == "Caption");
            if let Some(figure) = caption_kind(&u.full, u.lines)
                .or_else(|| tagged.then(|| !u.full.trim_start().starts_with("Tab")))
            {
                u.kind = Kind::Caption { figure };
            }
        }
        // A paragraph continued on the next page skips the notes at the
        // foot of the page before (they follow it instead).
        let target = if u.note {
            out.len().checked_sub(1).filter(|&k| out[k].note)
        } else {
            out.iter().rposition(|x| !x.note)
        };
        if let Some(k) = target
            && continues_unit(&out[k], &u)
        {
            let prev = &mut out[k];
            glue_hyphen(prev, &mut u);
            prev.pieces.extend(u.pieces);
            prev.lines += u.lines;
            index_map[i] = k;
            continue;
        }
        out.push(u);
        index_map[i] = out.len() - 1;
    }
    let sections = outline_at
        .iter()
        .enumerate()
        .filter_map(|(k, at)| at.map(|i| (k, index_map[i])))
        .collect();
    (out, sections)
}

/// Marks notes at the foot of pages: text smaller than the body in the
/// lower 40% of a page that also has body text above it.
fn mark_notes(units: &mut [Unit], body: f32) {
    let mut i = 0;
    while i < units.len() {
        let page = units[i].page();
        let mut j = i;
        while j < units.len() && units[j].page() == page {
            j += 1;
        }
        let body_top = units[i..j]
            .iter()
            .filter(|u| {
                u.size >= 0.95 * body && matches!(u.kind, Kind::Paragraph | Kind::Item { .. })
            })
            .map(|u| u.top)
            .fold(f32::MAX, f32::min);
        for u in &mut units[i..j] {
            u.note = matches!(u.kind, Kind::Paragraph | Kind::Item { .. })
                && u.size > 0.0
                && u.size <= 0.88 * body
                && u.top >= 0.6
                && body_top < u.top;
        }
        i = j;
    }
}

fn heading_reason(u: &Unit, cx: &Context<'_>, outline: Option<u8>) -> Option<Why> {
    if matches!(
        u.kind,
        Kind::Table(_) | Kind::Image | Kind::Code | Kind::Field
    ) || u.note
    {
        return None;
    }
    if let Some(role) = cx.roles.get(&(u.page(), u.mcid)) {
        let level = match role.as_str() {
            "Title" | "H" | "H1" => Some(1),
            r if r.len() == 2 && r.starts_with('H') => {
                r[1..].parse::<u8>().ok().filter(|l| (1..=6).contains(l))
            }
            _ => None,
        };
        if let Some(l) = level {
            return Some(Why::Tag(l));
        }
    }
    // A caption set large or bold ("Table 2: Scores") is still a caption.
    if u.kind == Kind::Paragraph && caption_kind(&u.full, u.lines).is_some() {
        return None;
    }
    let t = u.full.trim();
    let chars = t.chars().count();
    let has_letters = t.chars().any(char::is_alphabetic);
    let ends_bad = t.ends_with(['.', ',', ';'])
        && heading_number(t).is_none_or(|_| t.split_whitespace().count() > 1);
    let short = u.lines <= 3 && chars <= 200 && has_letters && !ends_bad;
    let body = cx.body_size;
    if short && u.size >= body * 1.15 {
        return Some(Why::Size);
    }
    if short && u.lines == 1 && chars <= 120 && u.bold >= 0.8 && !cx.body_bold {
        return Some(Why::Weight);
    }
    if short
        && u.lines == 1
        && heading_number(t).is_some()
        && (u.bold >= 0.5 || u.size >= body * 1.05)
    {
        return Some(Why::Number);
    }
    outline.map(Why::Outline)
}

/// True when `next` continues `prev` (a paragraph or item broken by a
/// column or page end): `prev` does not end a sentence and `next` starts
/// in lowercase, or `prev` ends with a hyphen.
fn continues_unit(prev: &Unit, next: &Unit) -> bool {
    let kinds =
        matches!(prev.kind, Kind::Paragraph | Kind::Item { .. }) && next.kind == Kind::Paragraph;
    if !kinds || (prev.size - next.size).abs() > 0.15 * prev.size.max(next.size) {
        return false;
    }
    let last = prev.last_text().trim_end();
    let first = next.pieces.first().map_or("", |p| p.text.as_str());
    let lower = first.chars().next().is_some_and(char::is_lowercase);
    !ends_sentence(last) && (lower || last.ends_with('-'))
}

/// When `prev` ends with a line-end hyphen before `next`'s lowercase
/// start, removes the hyphen and glues the pieces into one word.
fn glue_hyphen(prev: &mut Unit, next: &mut Unit) {
    let (Some(last), Some(first)) = (prev.pieces.last_mut(), next.pieces.first_mut()) else {
        return;
    };
    let mut probe = last.text.clone();
    join_text(&mut probe, &first.text);
    let plain = format!("{} {}", last.text, first.text.trim_start());
    if probe != plain && last.text.ends_with(['-', '\u{2010}', '\u{ad}']) {
        let joined_hyphen_kept = probe.starts_with(&last.text);
        if !joined_hyphen_kept {
            last.text.pop();
        }
        first.glue = true;
        first.text = first.text.trim_start().to_owned();
    }
}

/// Words that start a caption, in lowercase, and whether it describes a
/// picture: English, Spanish, French, German, Portuguese, and Arabic, the
/// six languages of textweaver's own messages. Arabic captions often start
/// with the article ("الشكل"), so both forms are listed.
const CAPTION_WORDS: &[(&str, bool)] = &[
    // English.
    ("figure", true),
    ("fig.", true),
    ("illustration", true),
    ("plate", true),
    ("chart", true),
    ("diagram", true),
    ("graph", true),
    ("map", true),
    ("photo", true),
    ("table", false),
    ("tab.", false),
    // Spanish ("fig." is above).
    ("figura", true),
    ("ilustración", true),
    ("imagen", true),
    ("gráfico", true),
    ("gráfica", true),
    ("diagrama", true),
    ("mapa", true),
    ("foto", true),
    ("lámina", true),
    ("tabla", false),
    ("cuadro", false),
    // French ("figure", "fig.", "illustration" and "photo" are above).
    ("graphique", true),
    ("diagramme", true),
    ("carte", true),
    ("planche", true),
    ("image", true),
    ("tableau", false),
    // German.
    ("abbildung", true),
    ("abb.", true),
    ("bild", true),
    ("grafik", true),
    ("diagramm", true),
    ("karte", true),
    ("tafel", true),
    ("tabelle", false),
    // Portuguese ("figura", "gráfico", "diagrama", "mapa" and "foto"
    // are above, with Spanish).
    ("ilustração", true),
    ("imagem", true),
    ("prancha", true),
    ("tabela", false),
    ("quadro", false),
    // Arabic: figure, drawing, picture, chart, map; table.
    ("شكل", true),
    ("الشكل", true),
    ("رسم", true),
    ("الرسم", true),
    ("صورة", true),
    ("الصورة", true),
    ("مخطط", true),
    ("المخطط", true),
    ("خريطة", true),
    ("الخريطة", true),
    ("جدول", false),
    ("الجدول", false),
];

/// Whether `word` starts as a caption word or a title does: with a capital
/// letter, or a letter of a script without case (Arabic).
fn starts_as_title(word: &str) -> bool {
    word.chars()
        .next()
        .is_some_and(|c| c.is_uppercase() || (c.is_alphabetic() && !c.is_lowercase()))
}

/// `s` with Arabic-Indic and Eastern Arabic-Indic digits as ASCII digits,
/// so "الشكل ٣" is numbered as "Figure 3" is.
fn ascii_digits(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\u{0660}'..='\u{0669}' => char::from(b'0' + (c as u32 - 0x0660) as u8),
            '\u{06F0}'..='\u{06F9}' => char::from(b'0' + (c as u32 - 0x06F0) as u8),
            '\u{066B}' => '.',
            _ => c,
        })
        .collect()
}

/// A caption's number: `3`, `2.1`, `A.4`, `3-2`, `4b`, `IV`, `S1`.
fn caption_number(s: &str) -> bool {
    let roman = !s.is_empty() && s.len() <= 6 && s.chars().all(|c| "IVXLC".contains(c));
    let body = s
        .strip_prefix(|c: char| c.is_ascii_uppercase())
        .filter(|r| r.starts_with(|c: char| c.is_ascii_digit()))
        .unwrap_or(s);
    let body = body.trim_end_matches(|c: char| c.is_ascii_lowercase());
    let numeric = !body.is_empty()
        && body.len() <= 9
        && body
            .split(['.', '-', '\u{2013}'])
            .all(|p| !p.is_empty() && p.len() <= 3 && p.chars().all(|c| c.is_ascii_digit()));
    roman || numeric
}

/// Whether `text` is a caption, by its pattern: a caption word, a number,
/// then punctuation ("Figure 3.", "Table 2:", "Fig. 4 -") or a capitalized
/// title on a short paragraph ("Table 2 Results by year"). Returns whether
/// it describes a picture. "Figure 3 shows..." is a sentence, not a
/// caption.
pub(super) fn caption_kind(text: &str, lines: usize) -> Option<bool> {
    let mut words = text.split_whitespace();
    let first = words.next()?;
    let lower = first.to_lowercase();
    let figure = CAPTION_WORDS
        .iter()
        .find(|(w, _)| lower == *w)
        .map(|&(_, f)| f)?;
    // The word must be written as a word starts: "Figure" or "FIGURE"
    // (Arabic has no capitals).
    if !starts_as_title(first) {
        return None;
    }
    let number = ascii_digits(words.next()?);
    // A number in parentheses, "(3)", as Arabic captions often write it,
    // sets the number apart as punctuation after it does.
    let number = match number.strip_prefix('(').and_then(|n| n.split_once(')')) {
        Some((n, "")) => return caption_number(n).then_some(figure),
        Some((n, rest)) => format!("{n}{rest}"),
        None => number,
    };
    // The Arabic comma, colon-like marks, and the full stop end a number
    // as the Latin ones do.
    let bare = number.trim_end_matches([
        '.', ':', '\u{2014}', '\u{2013}', '|', ',', '\u{060C}', '\u{06D4}',
    ]);
    if !caption_number(bare) {
        return None;
    }
    let punctuated = bare.len() < number.len() && !number.ends_with([',', '\u{060C}']);
    let next = words.next();
    if punctuated {
        return Some(figure);
    }
    match next {
        None => Some(figure),
        Some("-" | "\u{2013}" | "\u{2014}" | "|" | ":") => Some(figure),
        // A capitalized title. A script without capitals (Arabic) cannot
        // tell a title from a sentence ("Figure 3 shows..."), so there
        // the number must be set apart by punctuation.
        Some(w) if w.starts_with(char::is_uppercase) && lines <= 4 => Some(figure),
        _ => None,
    }
}

/// Writes the units into `b`, with `PageBreak` markers per page (label =
/// the printed page label, `labels[page]`, or the page number) and
/// `SectionBreak` markers for the outline. Returns the pages whose
/// `PageBreak` markers were opened, in order.
pub(super) fn emit(
    b: &mut Builder,
    units: &[Unit],
    sections: &[(usize, usize)],
    outline: &[(String, usize, u8)],
    labels: &[String],
) -> Vec<usize> {
    let marker = |k: MarkerKind| Marker::new(k, CharRange::empty(0));
    // A table caption labels the table right after it, or else right
    // before it, on the same page.
    let mut table_labels: HashMap<usize, String> = HashMap::new();
    for (i, u) in units.iter().enumerate() {
        if u.kind != (Kind::Caption { figure: false }) {
            continue;
        }
        let is_table =
            |k: usize| matches!(units[k].kind, Kind::Table(_)) && units[k].page() == u.page();
        let target = [i + 1, i.wrapping_sub(1)]
            .into_iter()
            .find(|&k| k < units.len() && is_table(k) && !table_labels.contains_key(&k));
        if let Some(k) = target {
            table_labels.insert(k, u.text());
        }
    }
    // A figure caption beside a tagged picture on its page is that
    // picture's caption: the picture is the graphic, so the caption is not
    // said as a second one.
    let beside_picture = |i: usize| {
        [i + 1, i.wrapping_sub(1)].into_iter().any(|k| {
            units
                .get(k)
                .is_some_and(|v| v.kind == Kind::Image && v.page() == units[i].page())
        })
    };
    let opened = std::cell::RefCell::new(Vec::new());
    let mut page: Option<(usize, OpenId)> = None;
    let mut open_sections: Vec<(u8, OpenId)> = Vec::new();
    let mut lists: Vec<(f32, OpenId)> = Vec::new();
    let mut set_page = |b: &mut Builder, p: usize, page: &mut Option<(usize, OpenId)>| {
        // Pages only move forward (notes moved after a paragraph that
        // continued onto the next page stay in that page's range).
        if page.as_ref().is_some_and(|(cur, _)| *cur >= p) {
            return;
        }
        if let Some((_, id)) = page.take() {
            b.close(id);
        }
        let label = labels
            .get(p)
            .cloned()
            .unwrap_or_else(|| (p + 1).to_string());
        let m = marker(MarkerKind::PageBreak).with_label(label);
        *page = Some((p, b.open(m)));
        opened.borrow_mut().push(p);
    };
    let close_lists = |b: &mut Builder, lists: &mut Vec<(f32, OpenId)>| {
        while let Some((_, id)) = lists.pop() {
            b.close(id);
        }
    };
    for (i, u) in units.iter().enumerate() {
        for &(k, _) in sections.iter().filter(|(_, at)| *at == i) {
            let depth = outline[k].2;
            while let Some(&(d, id)) = open_sections.last() {
                if d < depth {
                    break;
                }
                b.close(id);
                open_sections.pop();
            }
            let mut m = marker(MarkerKind::SectionBreak).with_level(depth);
            if !outline[k].0.trim().is_empty() {
                m = m.with_label(outline[k].0.trim());
            }
            open_sections.push((depth, b.open(m)));
        }
        if !matches!(u.kind, Kind::Item { .. }) {
            close_lists(b, &mut lists);
        }
        match &u.kind {
            Kind::Heading(level) => {
                b.paragraph_break();
                set_page(b, u.page(), &mut page);
                let id = b.open(marker(MarkerKind::Heading).with_level(*level));
                b.text(&u.text());
                b.close(id);
                b.paragraph_break();
            }
            Kind::Field => {
                b.paragraph_break();
                set_page(b, u.page(), &mut page);
                let id = b.open(marker(MarkerKind::Paragraph));
                b.text(&u.text());
                b.close(id);
                b.paragraph_break();
            }
            Kind::Paragraph | Kind::Image | Kind::Caption { .. } => {
                b.paragraph_break();
                set_page(b, u.page(), &mut page);
                let id = b.open(marker(MarkerKind::Paragraph));
                let note = u
                    .note
                    .then(|| b.open(marker(MarkerKind::Footnote).with_level(1)));
                // An image is a paragraph holding its alternate text; a
                // figure's caption describes its picture, as in the LaTeX
                // loader, so it is a graphic too (untagged pictures have no
                // other sign), unless a tagged picture is beside it.
                let figure = u.kind == (Kind::Caption { figure: true }) && !beside_picture(i);
                let image =
                    (u.kind == Kind::Image || figure).then(|| b.open(marker(MarkerKind::Image)));
                // A paragraph set wholly in italic or bold (a byline, a
                // caption, a callout) keeps its emphasis.
                let emphasis: Vec<OpenId> =
                    [(u.italic, MarkerKind::Italic), (u.bold, MarkerKind::Bold)]
                        .into_iter()
                        .filter(|(f, _)| *f >= 0.9 && u.kind != Kind::Image && image.is_none())
                        .map(|(_, k)| b.open(marker(k)))
                        .collect();
                pieces(b, u, &mut page, &mut set_page);
                for e in emphasis.into_iter().rev() {
                    b.close(e);
                }
                if let Some(img) = image {
                    b.close(img);
                }
                if let Some(n) = note {
                    b.close(n);
                }
                b.close(id);
                b.paragraph_break();
            }
            Kind::Item { marker_x, label } => {
                // Nesting by marker position.
                while lists.last().is_some_and(|(x, _)| *x > marker_x + 2.0) {
                    if let Some((_, id)) = lists.pop() {
                        b.close(id);
                    }
                }
                if lists.is_empty() {
                    b.paragraph_break();
                }
                if lists.last().is_none_or(|(x, _)| *x < marker_x - 2.0) {
                    let depth = u8::try_from(lists.len() + 1).unwrap_or(u8::MAX);
                    set_page(b, u.page(), &mut page);
                    let id = b.open(marker(MarkerKind::List).with_level(depth));
                    lists.push((*marker_x, id));
                }
                let depth = u8::try_from(lists.len().max(1)).unwrap_or(u8::MAX);
                b.line_break();
                set_page(b, u.page(), &mut page);
                let mut m = marker(MarkerKind::ListItem).with_level(depth);
                if let Some(l) = label {
                    m = m.with_label(l.clone());
                }
                let id = b.open(m);
                pieces(b, u, &mut page, &mut set_page);
                b.close(id);
            }
            Kind::Code => {
                b.paragraph_break();
                set_page(b, u.page(), &mut page);
                let id = b.open(marker(MarkerKind::Code).with_level(1));
                b.verbatim(&u.text());
                b.close(id);
                b.paragraph_break();
            }
            Kind::Table(t) => {
                b.paragraph_break();
                set_page(b, u.page(), &mut page);
                let mut tm = marker(MarkerKind::Table);
                if let Some(l) = table_labels.get(&i) {
                    tm = tm.with_label(l.clone());
                }
                let table = b.open(tm);
                for (r, row) in t.rows.iter().enumerate() {
                    b.line_break();
                    let mut rm = marker(MarkerKind::TableRow);
                    if r == 0 && t.header {
                        rm = rm.with_label(HEADER_ROW_LABEL);
                    }
                    let row_id = b.open(rm);
                    for (c, cell) in row.iter().enumerate() {
                        if c > 0 {
                            b.separator(crate::CELL_SEPARATOR);
                        }
                        let id = b.open_here(marker(MarkerKind::TableCell));
                        b.text(cell);
                        b.close(id);
                    }
                    b.close(row_id);
                }
                b.close(table);
                b.paragraph_break();
            }
        }
    }
    close_lists(b, &mut lists);
    opened.into_inner()
}

/// Writes a unit's pieces, switching the page marker between them.
fn pieces(
    b: &mut Builder,
    u: &Unit,
    page: &mut Option<(usize, OpenId)>,
    set_page: &mut impl FnMut(&mut Builder, usize, &mut Option<(usize, OpenId)>),
) {
    for (k, piece) in u.pieces.iter().enumerate() {
        if k > 0 && !piece.glue {
            b.space();
        }
        set_page(b, piece.page, page);
        // With no pending separator, a glued piece continues the word.
        b.text(&piece.text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_and_numbers() {
        assert_eq!(
            numbered_marker("1. Open it"),
            Some(("1.".into(), "Open it"))
        );
        assert_eq!(
            numbered_marker("(b) second"),
            Some(("(b)".into(), "second"))
        );
        assert_eq!(numbered_marker("iv) four"), Some(("iv)".into(), "four")));
        assert_eq!(numbered_marker("3.14 is pi"), None);
        assert_eq!(numbered_marker("e.g. this"), None);
        assert_eq!(bullet_marker("\u{2022} item"), Some("item"));
        assert_eq!(bullet_marker("-5 degrees"), None);
        assert_eq!(heading_number("2.3 Methods"), Some(2));
        assert_eq!(heading_number("1 Introduction"), Some(1));
        assert_eq!(heading_number("Chapter 4"), Some(1));
        assert_eq!(heading_number("12 apples were sold"), None);
    }

    #[test]
    fn captions_are_found_by_their_pattern() {
        assert_eq!(caption_kind("Figure 3. The water cycle", 1), Some(true));
        assert_eq!(caption_kind("Table 2: Scores by year", 1), Some(false));
        assert_eq!(caption_kind("Fig. 4 - Map of the valley", 1), Some(true));
        assert_eq!(caption_kind("FIGURE 1.2: Overview", 2), Some(true));
        assert_eq!(caption_kind("Table 2 Results by year", 1), Some(false));
        assert_eq!(caption_kind("Table S1.", 1), Some(false));
        assert_eq!(caption_kind("Plate IV", 1), Some(true));
        // Sentences that mention a figure are not captions.
        assert_eq!(caption_kind("Figure 3 shows the cycle.", 1), None);
        assert_eq!(caption_kind("Figure 3, which follows, shows it.", 1), None);
        assert_eq!(caption_kind("Table 2 Results by year and more", 9), None);
        assert_eq!(caption_kind("table 2: lower case", 1), None);
        assert_eq!(caption_kind("Tables 2: a list", 1), None);
        assert_eq!(caption_kind("Figure it out.", 1), None);
        assert_eq!(caption_kind("Figure", 1), None);
    }

    /// Caption words in the six languages of textweaver's messages.
    #[test]
    fn captions_are_found_in_six_languages() {
        // Spanish.
        assert_eq!(caption_kind("Figura 3. El ciclo del agua", 1), Some(true));
        assert_eq!(caption_kind("Tabla 2: Resultados por año", 1), Some(false));
        assert_eq!(caption_kind("Cuadro 1 Datos de 2020", 1), Some(false));
        assert_eq!(caption_kind("ILUSTRACIÓN 4: Mapa", 1), Some(true));
        assert_eq!(caption_kind("Figura 3 muestra el ciclo.", 1), None);
        // French.
        assert_eq!(caption_kind("Tableau 2 \u{2013} Résultats", 1), Some(false));
        assert_eq!(caption_kind("Graphique 5. Ventes", 1), Some(true));
        assert_eq!(caption_kind("Tableau 2 présente les résultats.", 1), None);
        // German.
        assert_eq!(
            caption_kind("Abbildung 3: Der Wasserkreislauf", 1),
            Some(true)
        );
        assert_eq!(caption_kind("Abb. 4 Karte des Tals", 1), Some(true));
        assert_eq!(caption_kind("Tabelle 2: Ergebnisse", 1), Some(false));
        assert_eq!(caption_kind("Tabelle 2 zeigt die Ergebnisse.", 1), None);
        // Portuguese.
        assert_eq!(caption_kind("Tabela 1 - Dados", 1), Some(false));
        assert_eq!(caption_kind("Ilustração 2: O ciclo", 1), Some(true));
        assert_eq!(caption_kind("Quadro 3. Resumo", 1), Some(false));
        // Arabic: with or without the article, Arabic-Indic or ASCII
        // digits, a number in parentheses.
        assert_eq!(caption_kind("الشكل ٣: دورة الماء", 1), Some(true));
        assert_eq!(caption_kind("شكل 2. خريطة", 1), Some(true));
        assert_eq!(caption_kind("جدول (1) النتائج", 1), Some(false));
        assert_eq!(caption_kind("الجدول ٢", 1), Some(false));
        // Without capitals, a word after the number may begin a sentence
        // ("Figure 3 shows the cycle"), so it is not a caption.
        assert_eq!(caption_kind("الشكل ٣ يوضح الدورة", 1), None);
    }

    #[test]
    fn joins_lines_and_hyphens() {
        let mut s = String::from("informa-");
        join_text(&mut s, "tion here");
        assert_eq!(s, "information here");
        let mut s = String::from("well-");
        join_text(&mut s, "Known");
        assert_eq!(s, "well-Known");
        let mut s = String::from("end.");
        join_text(&mut s, "Next");
        assert_eq!(s, "end. Next");
        let mut s = String::from("soft\u{ad}");
        join_text(&mut s, "ware");
        assert_eq!(s, "software");
    }
}
