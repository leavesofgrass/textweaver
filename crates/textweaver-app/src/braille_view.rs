//! The Braille view (beta 4): the current line or sentence as braille
//! cells, for a frontend to draw as dots, as a learning and teaching tool
//! and for checking spacing.
//!
//! - **What it shows.** While speech reads, the spoken sentence; otherwise
//!   the line at the cursor (at most [`MAX_CHARS`] of it, around the
//!   cursor). The cells go [`BrailleSettings::display_cells`] at a time, as
//!   on a Braille display, and pan to the part that holds the spoken word
//!   or the cursor. A part ends after a blank cell where it can, so words
//!   are not cut in two.
//! - **Translation.** The same `[braille] grade` as BRF files: uncontracted
//!   UEB by textweaver's own translator ([`textweaver_writers::ueb`]),
//!   which says exactly which print char each cell stands for; contracted
//!   UEB through liblouis with the BRF writer's table, uncontracted when
//!   liblouis is missing ([`BrailleView::fell_back`]).
//! - **The print line.** Each run of cells is paired with the print it
//!   stands for. liblouis says nothing of that, so for contracted braille
//!   each word's prefixes are contracted too: a prefix whose braille starts
//!   the word's braille has settled, and the print between two settled
//!   prefixes owns the cells between them ([`word_groups`]).
//! - **Every space is a cell.** Two spaces in print are two blank cells, so
//!   spacing shows exactly.
//! - **Visual only.** Nothing here is announced; screen readers drive real
//!   Braille displays. Only the commands that show, hide or switch it speak.
//!
//! [`BrailleSettings::display_cells`]: textweaver_store::BrailleSettings::display_cells

use std::ops::Range;

use textweaver_core::{CharPos, CharRange};

use crate::app::App;
use crate::text_util;

/// The most chars of a line the view translates: a long paragraph-line
/// is cut to this many around the cursor.
pub const MAX_CHARS: usize = 1000;

/// Words longer than this are not split for the print line in contracted
/// braille: the whole word stands under its cells.
const MAX_WORD_PREFIXES: usize = 48;

/// Dots of the letters a to z (bit 0 is dot 1, bit 7 is dot 8).
const LETTER_DOTS: [u8; 26] = [
    1, 3, 9, 25, 17, 11, 27, 19, 10, 26, // a-j
    5, 7, 13, 29, 21, 15, 31, 23, 14, 30, // k-t
    37, 39, 58, 45, 61, 53, // u v w x y z
];

/// A run of cells and the print it stands for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    /// The cells, as indices into the line's cells.
    pub cells: Range<usize>,
    /// The print, as char indices into the line's text.
    pub chars: Range<usize>,
}

/// A translated line: its cells, and the print each run of them stands
/// for, in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BrailleLine {
    /// Each cell's dots: bit 0 is dot 1, bit 7 is dot 8.
    pub cells: Vec<u8>,
    /// Runs of cells and their print, in order, not overlapping.
    pub groups: Vec<Group>,
    /// Contracted braille was asked for and could not be made, so this is
    /// uncontracted.
    pub fell_back: bool,
}

/// A run of the shown cells and the print under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrintGroup {
    /// The cells, as indices into [`BrailleView::cells`].
    pub cells: Range<usize>,
    /// The print they stand for.
    pub print: String,
}

/// What the Braille view shows now ([`App::braille_view`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrailleView {
    /// The cells of the part shown, at most [`width`](Self::width).
    pub cells: Vec<u8>,
    /// The print under them, run by run.
    pub print: Vec<PrintGroup>,
    /// Cells on the display (`[braille] display_cells`).
    pub width: usize,
    /// Contracted braille is shown.
    pub contracted: bool,
    /// Contracted braille was asked for, but liblouis is missing or failed,
    /// so uncontracted braille is shown.
    pub fell_back: bool,
    /// The part shown, from 0, and how many parts the line has.
    pub part: usize,
    /// How many parts the line has.
    pub parts: usize,
    /// The print line is on (`[braille] viewer_print_line`).
    pub print_line: bool,
    /// Cell outlines are on (`[braille] viewer_cell_outlines`).
    pub outlines: bool,
}

impl BrailleView {
    /// The cells as Unicode braille patterns (U+2800 to U+28FF).
    pub fn unicode(&self) -> String {
        unicode(&self.cells)
    }

    /// True when a cell uses dot 7 or dot 8: the view draws eight-dot
    /// cells.
    pub fn eight_dot(&self) -> bool {
        self.cells.iter().any(|&d| d & 0xC0 != 0)
    }
}

/// `cells` as Unicode braille patterns.
pub fn unicode(cells: &[u8]) -> String {
    cells
        .iter()
        .map(|&d| char::from_u32(0x2800 + u32::from(d)).unwrap_or(' '))
        .collect()
}

/// The dots of a Unicode braille pattern; a space is a blank cell, and
/// anything else `None`.
pub fn dots_of(c: char) -> Option<u8> {
    match c {
        '\u{2800}'..='\u{28FF}' => u8::try_from(c as u32 - 0x2800).ok(),
        ' ' => Some(0),
        _ => None,
    }
}

/// `text` with every white space char as a plain space, one each, so
/// every space is a cell.
fn spaced(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .collect()
}

/// Uncontracted UEB with each cell's print, by textweaver's translator.
#[cfg(feature = "publish")]
pub fn uncontracted(text: &str) -> BrailleLine {
    let (tr, origins) = textweaver_writers::ueb::translate_mapped(&spaced(text));
    let cells: Vec<u8> = textweaver_writers::ueb::to_unicode(&tr.braille)
        .chars()
        .map(|c| dots_of(c).unwrap_or(0))
        .collect();
    let mut groups: Vec<Group> = Vec::new();
    for (i, &o) in origins.iter().enumerate() {
        match groups.last_mut() {
            Some(g) if g.chars.start == o => g.cells.end = i + 1,
            _ => groups.push(Group {
                cells: i..i + 1,
                chars: o..o + 1,
            }),
        }
    }
    BrailleLine {
        cells,
        groups,
        fell_back: false,
    }
}

/// Uncontracted braille needs the translator, which this build leaves out.
#[cfg(not(feature = "publish"))]
pub fn uncontracted(_text: &str) -> BrailleLine {
    BrailleLine::default()
}

/// A contractor: one line of Unicode braille per text, in order
/// (liblouis, or a stand-in in tests).
pub type Contract<'a> = dyn Fn(&[&str]) -> Result<Vec<String>, String> + 'a;

/// Contracted braille of `text` with each run's print, through `contract`.
///
/// One call contracts the whole line, so passages and indicators that
/// span words come out as liblouis writes them, and each word's prefixes,
/// which place the print line ([`word_groups`]). A line whose words and
/// blank cells do not pair up gets one run for the whole line.
///
/// # Errors
///
/// When `contract` fails.
pub fn contracted(text: &str, contract: &Contract<'_>) -> Result<BrailleLine, String> {
    let text = spaced(text);
    let chars: Vec<char> = text.chars().collect();
    if chars.iter().all(|&c| c == ' ') {
        // Nothing to contract: a blank cell for each space.
        let mut groups = Vec::new();
        spaces(&mut groups, 0..chars.len(), 0..chars.len());
        return Ok(BrailleLine {
            cells: vec![0; chars.len()],
            groups,
            fell_back: false,
        });
    }
    // Words as char ranges.
    let mut words: Vec<Range<usize>> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ' ' {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && chars[i] != ' ' {
            i += 1;
        }
        words.push(start..i);
    }
    let mut inputs: Vec<String> = vec![text.clone()];
    for w in &words {
        let n = w.len();
        let from = if n > MAX_WORD_PREFIXES { n } else { 1 };
        for k in from..=n {
            inputs.push(chars[w.start..w.start + k].iter().collect());
        }
    }
    let refs: Vec<&str> = inputs.iter().map(String::as_str).collect();
    let out = contract(&refs)?;
    let cells_of = |s: &str| -> Vec<u8> { s.chars().map(|c| dots_of(c).unwrap_or(0)).collect() };
    let cells = cells_of(out.first().map_or("", String::as_str));
    // The line's words in braille: runs of cells between blank cells.
    let mut braille_words: Vec<Range<usize>> = Vec::new();
    let mut j = 0;
    while j < cells.len() {
        if cells[j] == 0 {
            j += 1;
            continue;
        }
        let start = j;
        while j < cells.len() && cells[j] != 0 {
            j += 1;
        }
        braille_words.push(start..j);
    }
    let whole = |cells: Vec<u8>| BrailleLine {
        groups: if cells.is_empty() || chars.is_empty() {
            Vec::new()
        } else {
            vec![Group {
                cells: 0..cells.len(),
                chars: 0..chars.len(),
            }]
        },
        cells,
        fell_back: false,
    };
    if braille_words.len() != words.len() {
        return Ok(whole(cells));
    }
    let mut groups: Vec<Group> = Vec::new();
    let mut next = 1;
    let mut prev_cell = 0;
    let mut prev_char = 0;
    for (w, b) in words.iter().zip(&braille_words) {
        // The blank cells and spaces before the word.
        spaces(&mut groups, prev_cell..b.start, prev_char..w.start);
        let n = w.len();
        let count = if n > MAX_WORD_PREFIXES { 1 } else { n };
        let prefixes: Vec<Vec<u8>> = out
            .get(next..next + count)
            .unwrap_or_default()
            .iter()
            .map(|s| cells_of(s))
            .collect();
        next += count;
        let word: Vec<char> = chars[w.clone()].to_vec();
        let in_line = &cells[b.clone()];
        let found = if n > MAX_WORD_PREFIXES || prefixes.len() != n {
            vec![(0..n, 0..in_line.len())]
        } else {
            word_groups(&word, &prefixes, in_line)
        };
        for (c, k) in found {
            groups.push(Group {
                cells: b.start + k.start..b.start + k.end,
                chars: w.start + c.start..w.start + c.end,
            });
        }
        prev_cell = b.end;
        prev_char = w.end;
    }
    spaces(&mut groups, prev_cell..cells.len(), prev_char..chars.len());
    Ok(BrailleLine {
        cells,
        groups,
        fell_back: false,
    })
}

/// The groups of blank cells `cells` for the spaces `chars`: one each when
/// they pair up, else one run.
fn spaces(groups: &mut Vec<Group>, cells: Range<usize>, chars: Range<usize>) {
    if cells.is_empty() || chars.is_empty() {
        return;
    }
    if cells.len() == chars.len() {
        for (k, c) in cells.zip(chars) {
            groups.push(Group {
                cells: k..k + 1,
                chars: c..c + 1,
            });
        }
    } else {
        groups.push(Group { cells, chars });
    }
}

/// Which print of one word stands over which of its cells in contracted
/// braille: `prefixes[j]` is the contraction of the word's first `j + 1`
/// chars by itself (the last is the whole word), and `in_line` the word's
/// cells in the line. Returns (chars, cells) runs, in order.
///
/// A prefix whose braille starts the word's braille has settled: the print
/// up to it owns the cells up to its end. A prefix that does not ("c" is
/// the wordsign `;c` alone, "ba" is `ba` but "bar" is `b>`) waits for the
/// next settled one, so its print joins a longer run ("bar" over two
/// cells). A run of lowercase letters over as many cells, each the
/// letter's own, is split letter by letter. The word's cells in the line
/// may carry indicators the word alone has not (a capitals passage); they
/// join the first or last run, or the word becomes one run.
pub fn word_groups(
    word: &[char],
    prefixes: &[Vec<u8>],
    in_line: &[u8],
) -> Vec<(Range<usize>, Range<usize>)> {
    let n = word.len();
    let whole = vec![(0..n, 0..in_line.len())];
    let Some(alone) = prefixes.last() else {
        return whole;
    };
    if n == 0 || prefixes.len() != n {
        return whole;
    }
    // Settled boundaries, kept from crossing later ones, then filled in.
    let mut raw: Vec<Option<usize>> = prefixes[..n - 1]
        .iter()
        .map(|p| alone.starts_with(p).then_some(p.len()))
        .collect();
    let mut later = alone.len();
    for v in raw.iter_mut().rev().flatten() {
        *v = (*v).min(later);
        later = *v;
    }
    let mut bounds = vec![0usize];
    for b in raw {
        let last = *bounds.last().unwrap_or(&0);
        bounds.push(b.unwrap_or(last).max(last));
    }
    bounds.push(alone.len());
    // Runs: print with no cells of its own joins the print after it.
    let mut runs: Vec<(Range<usize>, Range<usize>)> = Vec::new();
    let mut from_char = 0;
    for j in 1..=n {
        if bounds[j] > bounds[j - 1] {
            let start_cell = runs.last().map_or(0, |r| r.1.end);
            runs.push((from_char..j, start_cell..bounds[j]));
            from_char = j;
        }
    }
    if from_char < n {
        match runs.last_mut() {
            Some(r) => r.0.end = n,
            None => return whole,
        }
    }
    // Letter by letter where each cell is its letter.
    let mut split: Vec<(Range<usize>, Range<usize>)> = Vec::new();
    for (c, k) in runs {
        let letters = c.len() == k.len()
            && c.len() > 1
            && c.clone().zip(k.clone()).all(|(ci, ki)| {
                let ch = word[ci];
                ch.is_ascii_lowercase() && LETTER_DOTS[(ch as u8 - b'a') as usize] == alone[ki]
            });
        if letters {
            split.extend(c.zip(k).map(|(ci, ki)| (ci..ci + 1, ki..ki + 1)));
        } else {
            split.push((c, k));
        }
    }
    // The word in the line.
    if in_line == alone.as_slice() {
        split
    } else if in_line.ends_with(alone) {
        let extra = in_line.len() - alone.len();
        for (i, r) in split.iter_mut().enumerate() {
            r.1 = if i == 0 { 0 } else { r.1.start + extra }..r.1.end + extra;
        }
        split
    } else if in_line.starts_with(alone) {
        if let Some(r) = split.last_mut() {
            r.1.end = in_line.len();
        }
        split
    } else {
        whole
    }
}

/// The parts of `cells` a display of `width` cells shows in turn: each at
/// most `width` cells, ending after a blank cell where one falls inside.
pub fn parts(cells: &[u8], width: usize) -> Vec<Range<usize>> {
    let width = width.max(1);
    let mut out = Vec::new();
    let mut start = 0;
    while start < cells.len() {
        let mut end = (start + width).min(cells.len());
        if end < cells.len() && cells[end] != 0 {
            // Break after the last blank cell, so no word is cut.
            if let Some(b) = cells[start + 1..end].iter().rposition(|&d| d == 0) {
                end = start + 1 + b + 1;
            }
        }
        out.push(start..end);
        start = end;
    }
    if out.is_empty() {
        out.push(0..0);
    }
    out
}

/// One part of a line, as a display of some width shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    /// Its cells.
    pub cells: Vec<u8>,
    /// The runs that fall in it, their cells as indices into
    /// [`cells`](Self::cells) (a run divided between two parts is cut).
    pub groups: Vec<Group>,
    /// Which part, from 0.
    pub part: usize,
    /// How many parts the line has.
    pub parts: usize,
}

/// The part of `line` a display of `width` cells shows for print char
/// `focus`: the part with the first run that ends after it.
pub fn view_of(line: &BrailleLine, width: usize, focus: usize) -> Part {
    let all = parts(&line.cells, width);
    let cell = line
        .groups
        .iter()
        .find(|g| g.chars.end > focus)
        .map_or(0, |g| g.cells.start);
    let part = all
        .iter()
        .position(|r| r.contains(&cell))
        .unwrap_or(all.len().saturating_sub(1));
    let range = all[part].clone();
    let groups = line
        .groups
        .iter()
        .filter(|g| g.cells.start < range.end && g.cells.end > range.start)
        .map(|g| Group {
            cells: g.cells.start.max(range.start) - range.start
                ..g.cells.end.min(range.end) - range.start,
            chars: g.chars.clone(),
        })
        .collect();
    Part {
        cells: line.cells[range].to_vec(),
        groups,
        part,
        parts: all.len(),
    }
}

impl App {
    /// What the Braille view shows now, or `None` while it is off
    /// (`[braille] viewer`), with no document, or in a build without the
    /// braille translator. Kept for the text and grade, so a frontend may
    /// ask on every frame.
    pub fn braille_view(&self) -> Option<BrailleView> {
        let b = &self.settings.braille;
        if !b.viewer || !cfg!(feature = "publish") {
            return None;
        }
        let s = self.session.as_ref()?;
        let (range, focus) = braille_source(s);
        let text = s.doc.slice(range);
        let contracted = b.grade == textweaver_store::BrailleGrade::Contracted;
        let line = crate::frame_cache::cached(
            &self.frame_cache.braille,
            (text.clone(), contracted),
            || translate(&text, contracted),
        );
        let (low, high) = textweaver_store::DISPLAY_CELLS_RANGE;
        let width = usize::from(b.display_cells.clamp(low, high));
        let focus = focus.0.saturating_sub(range.start.0);
        let shown = view_of(&line, width, focus);
        let chars: Vec<char> = text.chars().collect();
        let print = shown
            .groups
            .into_iter()
            .map(|g| PrintGroup {
                print: chars
                    .get(g.chars)
                    .map(|c| c.iter().collect())
                    .unwrap_or_default(),
                cells: g.cells,
            })
            .collect();
        Some(BrailleView {
            cells: shown.cells,
            print,
            width,
            contracted: contracted && !line.fell_back,
            fell_back: line.fell_back,
            part: shown.part,
            parts: shown.parts,
            print_line: b.viewer_print_line,
            outlines: b.viewer_cell_outlines,
        })
    }

    /// `braille_view_toggle`: the Braille view shown or hidden; saved.
    pub(crate) fn braille_view_toggle(&mut self) {
        if !cfg!(feature = "publish") {
            let msg = self.msg("braille-view-not-in-build");
            self.tell(&msg);
            return;
        }
        let on = !self.settings.braille.viewer;
        self.settings.braille.viewer = on;
        self.settings_dirty = true;
        let msg = self.msg(if on {
            "braille-view-shown"
        } else {
            "braille-view-hidden"
        });
        self.tell(&msg);
    }

    /// `braille_grade_toggle`: uncontracted or contracted braille, for the
    /// Braille view and BRF files; saved. Contracted says so when liblouis
    /// cannot run.
    pub(crate) fn braille_grade_toggle(&mut self) {
        use textweaver_store::BrailleGrade as G;
        let grade = match self.settings.braille.grade {
            G::Uncontracted => G::Contracted,
            G::Contracted => G::Uncontracted,
        };
        self.settings.braille.grade = grade;
        self.settings_dirty = true;
        let msg = match grade {
            G::Uncontracted => self.msg("braille-grade-uncontracted"),
            G::Contracted if louis(&["a"]).is_ok() => self.msg("braille-grade-contracted"),
            G::Contracted => self.msg("braille-grade-contracted-no-liblouis"),
        };
        self.tell(&msg);
    }
}

/// What the view translates, and the position it pans to: the spoken
/// sentence and word while speech reads, else the cursor's line.
fn braille_source(s: &crate::app::Session) -> (CharRange, CharPos) {
    if let Some(sentence) = s.spoken_sentence {
        let focus = s.spoken.map_or(sentence.start, |w| w.start);
        return (sentence, focus);
    }
    let line = text_util::line_range(&s.doc, text_util::line_of(&s.doc, s.cursor));
    if line.len() <= MAX_CHARS {
        return (line, s.cursor);
    }
    let half = MAX_CHARS / 2;
    let from = CharPos(s.cursor.0.saturating_sub(half).max(line.start.0));
    let start = text_util::word_start(&s.doc, from).max(line.start);
    let end = CharPos((start.0 + MAX_CHARS).min(line.end.0));
    (CharRange::new(start, end), s.cursor)
}

/// `text` in the grade asked for; contracted falls back to uncontracted.
fn translate(text: &str, contracted_asked: bool) -> BrailleLine {
    if contracted_asked && let Ok(line) = contracted(text, &louis) {
        return line;
    }
    let mut line = uncontracted(text);
    line.fell_back = contracted_asked;
    line
}

/// liblouis with the BRF writer's grade 2 table.
#[cfg(feature = "publish")]
fn louis(texts: &[&str]) -> Result<Vec<String>, String> {
    let table = textweaver_writers::BrailleOptions::default().table;
    textweaver_writers::louis_unicode(texts, &table)
}

/// No liblouis without the BRF writer.
#[cfg(not(feature = "publish"))]
fn louis(_texts: &[&str]) -> Result<Vec<String>, String> {
    Err("this build has no braille translator".to_owned())
}

#[cfg(test)]
mod tests;
