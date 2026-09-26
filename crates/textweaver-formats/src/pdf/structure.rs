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
    if matches!(u.kind, Kind::Table(_) | Kind::Image | Kind::Code) || u.note {
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

/// Writes the units into `b`, with `PageBreak` markers per page (label =
/// the printed page label, `labels[page]`, or the page number) and
/// `SectionBreak` markers for the outline.
pub(super) fn emit(
    b: &mut Builder,
    units: &[Unit],
    sections: &[(usize, usize)],
    outline: &[(String, usize, u8)],
    labels: &[String],
) {
    let marker = |k: MarkerKind| Marker::new(k, CharRange::empty(0));
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
            Kind::Paragraph | Kind::Image => {
                b.paragraph_break();
                set_page(b, u.page(), &mut page);
                let id = b.open(marker(MarkerKind::Paragraph));
                let note = u
                    .note
                    .then(|| b.open(marker(MarkerKind::Footnote).with_level(1)));
                // An image is a paragraph holding its alternate text.
                let image = (u.kind == Kind::Image).then(|| b.open(marker(MarkerKind::Image)));
                // A paragraph set wholly in italic or bold (a byline, a
                // caption, a callout) keeps its emphasis.
                let emphasis: Vec<OpenId> =
                    [(u.italic, MarkerKind::Italic), (u.bold, MarkerKind::Bold)]
                        .into_iter()
                        .filter(|(f, _)| *f >= 0.9 && u.kind == Kind::Paragraph)
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
                let table = b.open(marker(MarkerKind::Table));
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
