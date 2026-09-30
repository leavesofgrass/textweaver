//! Finding where something on a page ended up in the canonical text.
//!
//! Annotations and links are placed on the page by rectangle, but the
//! reader needs a range of characters. The glyphs under the rectangle are
//! collected ([`text_in`], [`line_near`]), and their text is looked for in
//! that page's part of the canonical text ([`TextIndex::find`]). Both sides
//! are compared without spaces and hyphens and in lowercase, so line joins,
//! removed hyphens, and list markers do not stop a match.
//!
//! Searching is linear (Knuth, Morris, and Pratt) and has a budget per
//! document, so a hostile file with thousands of annotations on a huge page
//! cannot stall the loader: past the budget, annotations fall back to the
//! start of their page.

use std::cell::Cell;
use std::collections::HashMap;

use textweaver_core::{CharPos, CharRange, MarkerKind};
use textweaver_text::Marker;

use super::interp::{Glyph, PageContent};

/// Characters compared per document before searches give up.
const BUDGET: usize = 60_000_000;

/// The longest text looked for whole; longer text is matched by its start
/// and its end.
const WHOLE: usize = 300;

/// How much of the start and the end of a long text is matched.
const ENDS: usize = 40;

/// True for the characters left out of comparisons.
fn skipped(c: char) -> bool {
    c.is_whitespace() || matches!(c, '-' | '\u{ad}' | '\u{2010}' | '\u{2011}')
}

fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// The canonical text, prepared for searching by page.
pub(super) struct TextIndex {
    /// Every character of the text.
    pub chars: Vec<char>,
    /// The compared characters, folded.
    norm: Vec<char>,
    /// For each compared character, its index in `chars`.
    pos: Vec<usize>,
    /// Page (from 0) to its range in `norm`, and its range in `chars`.
    pages: HashMap<usize, ((usize, usize), CharRange)>,
    /// Page marker labels, by page.
    labels: HashMap<usize, String>,
    /// Pages that have a range, in order.
    order: Vec<usize>,
    budget: Cell<usize>,
}

impl TextIndex {
    /// Indexes `text`, whose `PageBreak` markers were opened for the pages
    /// in `page_order`, in that order.
    pub(super) fn new(text: &str, markers: &[Marker], page_order: &[usize]) -> TextIndex {
        let chars: Vec<char> = text.chars().collect();
        let mut norm = Vec::with_capacity(chars.len());
        let mut pos = Vec::with_capacity(chars.len());
        for (i, &c) in chars.iter().enumerate() {
            if !skipped(c) {
                norm.push(fold(c));
                pos.push(i);
            }
        }
        let mut breaks: Vec<&Marker> = markers
            .iter()
            .filter(|m| m.kind == MarkerKind::PageBreak)
            .collect();
        breaks.sort_by_key(|m| m.range.start);
        let mut pages = HashMap::new();
        let mut labels = HashMap::new();
        let mut order = Vec::new();
        for (m, &page) in breaks.iter().zip(page_order) {
            let a = pos.partition_point(|&p| p < m.range.start.0);
            let b = pos.partition_point(|&p| p < m.range.end.0);
            pages.insert(page, ((a, b), m.range));
            if let Some(l) = &m.label {
                labels.insert(page, l.clone());
            }
            order.push(page);
        }
        TextIndex {
            chars,
            norm,
            pos,
            pages,
            labels,
            order,
            budget: Cell::new(BUDGET),
        }
    }

    /// The page holding the text of `page`, or the next one that has text.
    fn page_or_next(&self, page: usize) -> Option<usize> {
        self.order.iter().copied().find(|&p| p >= page)
    }

    /// Where `page`'s text starts (or the next page's with text).
    pub(super) fn page_start(&self, page: usize) -> Option<CharPos> {
        let p = self.page_or_next(page)?;
        self.pages.get(&p).map(|(_, r)| r.start)
    }

    /// `page`'s range of characters, or the next page's with text.
    pub(super) fn page_range(&self, page: usize) -> Option<CharRange> {
        let p = self.page_or_next(page)?;
        self.pages.get(&p).map(|(_, r)| *r)
    }

    /// The printed label of `page`, or of the next page with text.
    pub(super) fn page_label(&self, page: usize) -> Option<String> {
        let p = self.page_or_next(page)?;
        self.labels
            .get(&p)
            .cloned()
            .or_else(|| Some((p + 1).to_string()))
    }

    /// The range of `needle` (page text as collected from glyphs) in
    /// `page`'s part of the text, else in the pages around it.
    pub(super) fn find(&self, page: usize, needle: &str) -> Option<CharRange> {
        let needle: Vec<char> = needle.chars().filter(|&c| !skipped(c)).map(fold).collect();
        if needle.is_empty() {
            return None;
        }
        let mut areas = Vec::new();
        if let Some(((a, b), _)) = self.pages.get(&page) {
            areas.push((*a, *b));
        }
        let around: Vec<(usize, usize)> = [page.saturating_sub(1), page, page + 1]
            .iter()
            .filter_map(|p| self.pages.get(p).map(|(r, _)| *r))
            .collect();
        if let (Some(lo), Some(hi)) = (
            around.iter().map(|r| r.0).min(),
            around.iter().map(|r| r.1).max(),
        ) {
            areas.push((lo, hi));
        }
        for (a, b) in areas {
            if let Some((i, j)) = self.find_in(a, b, &needle) {
                let start = self.pos[i];
                let end = self.pos[j - 1] + 1;
                return Some(CharRange::new(start, end));
            }
        }
        None
    }

    /// `needle`'s place in `norm[a..b]`, as a range of `norm` indices.
    fn find_in(&self, a: usize, b: usize, needle: &[char]) -> Option<(usize, usize)> {
        let hay = self.norm.get(a..b)?;
        if needle.len() <= WHOLE {
            return self.kmp(hay, needle).map(|i| (a + i, a + i + needle.len()));
        }
        let head = &needle[..ENDS];
        let tail = &needle[needle.len() - ENDS..];
        let i = self.kmp(hay, head)?;
        let after = &hay[i + ENDS..];
        // The end, no further than a little past the needle's own length.
        let limit = after.len().min(needle.len() * 2);
        match self.kmp(&after[..limit], tail) {
            Some(j) => Some((a + i, a + i + ENDS + j + ENDS)),
            None => Some((a + i, a + i + ENDS)),
        }
    }

    /// The first place `needle` occurs in `hay`, within the budget.
    fn kmp(&self, hay: &[char], needle: &[char]) -> Option<usize> {
        let cost = hay.len() + needle.len();
        let left = self.budget.get();
        if cost > left || needle.is_empty() {
            self.budget.set(0);
            return None;
        }
        self.budget.set(left - cost);
        let mut fail = vec![0usize; needle.len()];
        let mut k = 0;
        for i in 1..needle.len() {
            while k > 0 && needle[i] != needle[k] {
                k = fail[k - 1];
            }
            if needle[i] == needle[k] {
                k += 1;
            }
            fail[i] = k;
        }
        k = 0;
        for (i, &c) in hay.iter().enumerate() {
            while k > 0 && c != needle[k] {
                k = fail[k - 1];
            }
            if c == needle[k] {
                k += 1;
                if k == needle.len() {
                    return Some(i + 1 - k);
                }
            }
        }
        None
    }

    /// `range` cut at its first line end, so a link stays inside one block.
    pub(super) fn first_line(&self, range: CharRange) -> CharRange {
        let end = (range.start.0..range.end.0)
            .find(|&i| self.chars.get(i) == Some(&'\n'))
            .unwrap_or(range.end.0);
        let mut end = end;
        // A sentence's full stop after a link is not part of it.
        while end > range.start.0
            && self
                .chars
                .get(end - 1)
                .is_some_and(|c| c.is_whitespace() || matches!(c, '.' | ',' | ';' | ':'))
        {
            end -= 1;
        }
        CharRange::new(range.start.0, end)
    }
}

/// A rectangle on the page: `(x0, y0, x1, y1)`, y down.
pub(super) type Area = (f32, f32, f32, f32);

fn center(g: &Glyph) -> (f32, f32) {
    (g.x + g.w / 2.0, g.y - 0.3 * g.size)
}

/// The text of the glyphs whose middle lies in any of `areas`, in the
/// order they were drawn (at most a few thousand characters).
pub(super) fn text_in(content: &PageContent, areas: &[Area]) -> String {
    let mut out = String::new();
    for g in &content.glyphs {
        let (cx, cy) = center(g);
        let inside = areas.iter().any(|&(x0, y0, x1, y1)| {
            cx >= x0 - 1.0 && cx <= x1 + 1.0 && cy >= y0 - 1.0 && cy <= y1 + 1.0
        });
        if inside {
            out.push_str(content.glyph_text(g));
            if out.len() > 4_000 {
                break;
            }
        }
    }
    out
}

/// The start of the line of text nearest `area` (a note in the margin
/// beside a line, a sticky note on a paragraph): at most 40 characters from
/// the line's left end.
pub(super) fn line_near(content: &PageContent, area: Area) -> String {
    let (ax, ay) = ((area.0 + area.2) / 2.0, (area.1 + area.3) / 2.0);
    let Some(near) = content.glyphs.iter().min_by(|a, b| {
        let d = |g: &Glyph| {
            let (x, y) = center(g);
            // Being on the same line matters more than being close along it.
            (y - ay).abs() * 4.0 + (x - ax).abs()
        };
        d(a).total_cmp(&d(b))
    }) else {
        return String::new();
    };
    let mut line: Vec<&Glyph> = content
        .glyphs
        .iter()
        .filter(|g| (g.y - near.y).abs() <= 0.3 * near.size.max(g.size))
        .collect();
    line.sort_by(|a, b| a.x.total_cmp(&b.x));
    let mut out = String::new();
    for g in line {
        out.push_str(content.glyph_text(g));
        if out.chars().filter(|c| !c.is_whitespace()).count() >= 40 {
            break;
        }
    }
    out
}

/// The glyphs' text at or below `top` on the page: the start of the first
/// line there, to find where a link's destination lands.
pub(super) fn line_below(content: &PageContent, top: f32) -> String {
    let Some(first) = content
        .glyphs
        .iter()
        .filter(|g| g.y >= top - 0.2 * g.size)
        .min_by(|a, b| a.y.total_cmp(&b.y))
    else {
        return String::new();
    };
    line_near(
        content,
        (first.x, first.y - first.size, first.x + first.w, first.y),
    )
}

#[cfg(test)]
mod tests {
    use textweaver_core::CharRange;

    use super::*;

    fn index(text: &str) -> TextIndex {
        let len = text.chars().count();
        let half = text
            .find("PAGE2")
            .map_or(len, |b| text[..b].chars().count());
        let markers = vec![
            Marker::new(MarkerKind::PageBreak, CharRange::new(0, half)).with_label("iv"),
            Marker::new(MarkerKind::PageBreak, CharRange::new(half, len)).with_label("v"),
        ];
        TextIndex::new(text, &markers, &[0, 2])
    }

    #[test]
    fn page_text_is_found_across_joins_and_hyphens() {
        let text = "A title\n\nThe informa\u{ad}tion here is well known. PAGE2 Another page with information.";
        let ix = index(text);
        // Collected from glyphs: a line-end hyphen, other spacing, capitals.
        let r = ix.find(0, "informa-tion  HERE").unwrap();
        let found: String = ix.chars[r.start.0..r.end.0].iter().collect();
        assert_eq!(found, "informa\u{ad}tion here");
        // Found on the page asked for first, though it is on both.
        let r = ix.find(2, "information").unwrap();
        assert!(r.start.0 > text.find("PAGE2").unwrap());
        assert_eq!(ix.find(0, "   "), None);
        assert_eq!(ix.find(0, "not there"), None);
    }

    #[test]
    fn pages_without_text_lead_to_the_next_page() {
        let ix = index("One. PAGE2 Two.");
        assert_eq!(ix.page_label(1).as_deref(), Some("v"));
        assert_eq!(ix.page_start(1), ix.page_start(2));
        assert_eq!(ix.page_label(0).as_deref(), Some("iv"));
        assert_eq!(ix.page_start(9), None);
    }

    #[test]
    fn long_text_matches_by_its_ends_and_links_stay_on_one_line() {
        let body = "word ".repeat(200);
        let text = format!("Start {body}end.\nNext line");
        let ix = index(&text);
        let r = ix.find(0, &format!("Start {body}end.")).unwrap();
        assert_eq!(r.start.0, 0);
        assert_eq!(ix.chars[r.end.0 - 1], '.');
        // Cut at the line end, and before the sentence's full stop.
        let cut = ix.first_line(CharRange::new(0, ix.chars.len()));
        assert_eq!(ix.chars[cut.end.0 - 1], 'd');
        assert_eq!(ix.chars[cut.end.0], '.');
    }

    #[test]
    fn a_hostile_search_stops_at_the_budget() {
        let text = "a".repeat(10_000);
        let ix = index(&text);
        ix.budget.set(15_000);
        let needle = format!("{}b", "a".repeat(100));
        assert_eq!(ix.find(0, &needle), None);
        // The budget is spent: even a present text is not looked for.
        assert_eq!(ix.find(0, "aaa"), None);
    }
}
