//! Narration planning: which utterances read a range aloud (ADR-0005).
//!
//! [`plan`] splits a range into sentence-sized utterances, each with an
//! [`OffsetMap`] onto the document. Structure is spoken as `Inserted` spans
//! according to the policy and its verbosity:
//!
//! | Structure | Low | Normal | High |
//! |---|---|---|---|
//! | Heading | — | "heading level 2, " | same |
//! | List (at its first item) | — | "list with 3 items, " | same |
//! | Ordered list item number | "2. " | "2. " | "2. " |
//! | Block quote, code block, graphic (at their start) | — | "block quote, " … | same |
//! | Link (at its start) | — | — | "link, " |
//! | Footnote reference `[1]` | "footnote 1" | "footnote 1" | "footnote 1" |
//! | Table, structured | "Name is Ada, …" | "Table with 3 columns: …" then "Row 1: Name is Ada, …" | "… and 2 rows: …", "Row 1 of 2: …" |
//! | Table, flat | "Ada, Engineer, 98." | same | same |
//! | Skipped table or code block | — | "table with 3 columns, skipped" / "code block skipped" | same |
//!
//! With `announce_structure` off nothing is inserted except the separators
//! that keep table cells apart, and tables read flat. Table cell separators
//! in the canonical text (`" | "`) are always elided. Code blocks are
//! skipped with `skip_code` unless the range starts inside one (the reader
//! asked for it). Empty table cells read as "blank" in structured mode
//! instead of shifting later values to the wrong header (Star bug 7).
//!
//! [`plan_with`] also takes [`InlineSpeech`] changes from the caller: a
//! source range spoken as other words (an `Expanded` span, so the highlight
//! covers the whole range while they are said) or skipped (an `Elided`
//! span, so nothing is said and later highlights stay exact). The app uses
//! it for citations (`[@doe2020, p. 12]`): skipped, or said in words from
//! the reference library.
//!
//! Sentences longer than `max_chunk_chars` are split at whitespace, never
//! inside a word or an expanded token. Normalization (numbers,
//! abbreviations, punctuation) is not done here: the speech service applies
//! its transform chain per utterance and composes the maps.

use serde::{Deserialize, Serialize};
use textweaver_core::{
    CharPos, CharRange, Direction, MarkerKind, OffsetMap, Span, SpanKind, SpokenBuilder, Unit,
    Utterance, UtteranceId, Verbosity,
};

use crate::Document;
use crate::marker::{Marker, MarkerIndex};
use crate::units::Units;

/// How tables are read aloud (Star's `table_reading_mode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableNarration {
    /// Header names with each value, row numbers, a table introduction.
    #[default]
    Structured,
    /// Cell text only, separated by commas.
    Flat,
    /// Not read; announced as skipped.
    Skip,
}

/// How a range is turned into utterances.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NarrationPolicy {
    /// Upper bound on one utterance's length in chars; longer sentences are
    /// split at whitespace.
    pub max_chunk_chars: usize,
    /// Announce structure ("heading level 2") before marked ranges.
    pub announce_structure: bool,
    /// Skip text under `Code` markers (Star's `skip_code`).
    pub skip_code: bool,
    /// Verbosity of structure announcements.
    pub verbosity: Verbosity,
    /// How tables are read.
    pub table_mode: TableNarration,
}

impl Default for NarrationPolicy {
    fn default() -> Self {
        NarrationPolicy {
            max_chunk_chars: 400,
            announce_structure: true,
            skip_code: false,
            verbosity: Verbosity::Normal,
            table_mode: TableNarration::default(),
        }
    }
}

impl NarrationPolicy {
    fn announces(&self, at_least: Verbosity) -> bool {
        self.announce_structure && self.verbosity >= at_least
    }
}

/// A change the caller makes to how a range of the source is spoken.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineSpeech {
    /// The source range.
    pub range: CharRange,
    /// What is said instead; empty to say nothing.
    pub spoken: String,
}

impl InlineSpeech {
    /// Say `spoken` for `range`.
    pub fn say(range: CharRange, spoken: impl Into<String>) -> Self {
        InlineSpeech {
            range,
            spoken: spoken.into(),
        }
    }

    /// Say nothing for `range`.
    pub fn skip(range: CharRange) -> Self {
        InlineSpeech {
            range,
            spoken: String::new(),
        }
    }
}

/// One step of building a sentence's spoken text.
enum Piece {
    /// Spoken text with no source, anchored at a position.
    Insert(CharPos, String),
    /// Source replaced by spoken text.
    Replace(CharRange, String),
    /// Source not spoken.
    Elide(CharRange),
}

impl Piece {
    fn start(&self) -> CharPos {
        match self {
            Piece::Insert(at, _) => *at,
            Piece::Replace(r, _) | Piece::Elide(r) => r.start,
        }
    }
}

/// Sentence-sized utterances covering `range`, numbered from chunk 0.
pub fn plan(doc: &Document, range: CharRange, policy: &NarrationPolicy) -> Vec<Utterance> {
    plan_with(doc, range, policy, &[])
}

/// [`plan`] with the caller's changes to how some ranges are spoken
/// (`inline`, in any order; overlapping ones after the first are
/// ignored). A change that a sentence boundary cuts is said in the
/// sentence where it starts and skipped in the next. Table rows and
/// skipped code blocks are planned as usual.
pub fn plan_with(
    doc: &Document,
    range: CharRange,
    policy: &NarrationPolicy,
    inline: &[InlineSpeech],
) -> Vec<Utterance> {
    let range = range.clamp_to(doc.len_chars());
    if range.is_empty() {
        return Vec::new();
    }
    let mut changes: Vec<&InlineSpeech> = inline
        .iter()
        .filter(|c| !c.range.is_empty() && c.range.intersection(range).is_some())
        .collect();
    changes.sort_by_key(|c| (c.range.start, c.range.end));
    let mut kept: Vec<&InlineSpeech> = Vec::with_capacity(changes.len());
    for c in changes {
        if kept.last().is_none_or(|k| k.range.end <= c.range.start) {
            kept.push(c);
        }
    }
    let mut planner = Planner {
        doc,
        index: doc.marker_index(),
        policy,
        range,
        out: Vec::new(),
        done_until: CharPos::ZERO,
        table: None,
        inline: kept,
    };
    let sentences: Vec<CharRange> =
        Units::new(doc, Unit::Sentence, range.start, Direction::Forward)
            .take_while(|s| s.start < range.end)
            .collect();
    for s in sentences {
        planner.sentence(s);
    }
    planner
        .out
        .into_iter()
        .enumerate()
        .map(|(i, (text, map))| {
            let mut u = Utterance::with_map(text, map);
            u.id = UtteranceId {
                generation: 0,
                chunk: u32::try_from(i).unwrap_or(u32::MAX),
            };
            u
        })
        .collect()
}

struct Planner<'a> {
    doc: &'a Document,
    index: MarkerIndex<'a>,
    policy: &'a NarrationPolicy,
    range: CharRange,
    out: Vec<(String, OffsetMap)>,
    /// Everything before this position has been planned (tables and skipped
    /// code blocks are handled whole).
    done_until: CharPos,
    /// Facts about the table being read, computed once per table.
    table: Option<TableInfo>,
    /// The caller's changes, sorted, not overlapping.
    inline: Vec<&'a InlineSpeech>,
}

/// What row narration needs to know about a table.
struct TableInfo {
    start: CharPos,
    /// Every row, in order.
    rows: Vec<CharRange>,
    /// Index of the header row in `rows`.
    header: Option<usize>,
    /// Header cell texts, by column.
    names: Vec<String>,
    /// Widest row, in cells.
    columns: usize,
}

impl TableInfo {
    fn body_rows(&self) -> usize {
        self.rows.len() - usize::from(self.header.is_some())
    }

    /// 1-based number of `row` among the body rows.
    fn row_number(&self, row: CharRange) -> usize {
        let i = self.rows.partition_point(|r| r.start < row.start);
        (i + 1).saturating_sub(usize::from(self.header.is_some_and(|h| h <= i)))
    }
}

impl Planner<'_> {
    fn sentence(&mut self, s: CharRange) {
        let Some(mut clip) = s.intersection(self.range) else {
            return;
        };
        if clip.end <= self.done_until {
            return;
        }
        clip.start = clip.start.max(self.done_until);
        while clip.start < clip.end
            && self
                .doc
                .char_at(clip.start)
                .is_some_and(char::is_whitespace)
        {
            clip.start = clip.start.saturating_add(1);
        }
        if clip.is_empty() {
            return;
        }
        if let Some(row) = self.index.enclosing(MarkerKind::TableRow, clip.start)
            && let Some(table) = self.index.enclosing(MarkerKind::Table, row.range.start)
        {
            self.table_row(table, row, clip.start);
            return;
        }
        if let Some(code) = self
            .index
            .enclosing(MarkerKind::Code, clip.start)
            .filter(|m| m.level == 1)
            && self.policy.skip_code
            && !code.range.contains(self.range.start)
        {
            if self.policy.announces(Verbosity::Normal) {
                self.push_notice("code block skipped", code.range.start);
            }
            self.done_until = code.range.end;
            return;
        }
        let mut pieces = Vec::new();
        let prefix = self.prefix_at(clip.start);
        if !prefix.is_empty() {
            pieces.push(Piece::Insert(clip.start, prefix));
        }
        self.inline_pieces(clip, &mut pieces);
        self.caller_pieces(clip, &mut pieces);
        let (text, map) = self.build(clip, pieces);
        self.push(text, map);
    }

    /// Announcements for markers starting at `at` (outermost first).
    fn prefix_at(&self, at: CharPos) -> String {
        let p = self.policy;
        let mut out = String::new();
        for m in self.index.starting_at(at) {
            let part = match m.kind {
                MarkerKind::Heading if p.announces(Verbosity::Normal) => {
                    format!("heading level {}", m.level)
                }
                MarkerKind::List if p.announces(Verbosity::Normal) => {
                    let n = self.list_items(m);
                    format!("list with {n} item{}", if n == 1 { "" } else { "s" })
                }
                MarkerKind::ListItem if p.announce_structure => match &m.label {
                    Some(label) => {
                        out.push_str(label);
                        out.push(' ');
                        continue;
                    }
                    None => continue,
                },
                MarkerKind::Quote if p.announces(Verbosity::Normal) => "block quote".to_owned(),
                MarkerKind::Code if m.level == 1 && p.announces(Verbosity::Normal) => {
                    "code block".to_owned()
                }
                MarkerKind::Image if p.announces(Verbosity::Normal) => {
                    MarkerKind::Image.spoken_name().to_owned()
                }
                // A horizontal rule before this block: "separator".
                MarkerKind::Rule if p.announces(Verbosity::Normal) => {
                    MarkerKind::Rule.spoken_name().to_owned()
                }
                _ => continue,
            };
            out.push_str(&part);
            out.push_str(", ");
        }
        out
    }

    fn list_items(&self, list: &Marker) -> usize {
        self.index
            .iter(MarkerKind::ListItem, Some(list.level))
            .filter(|i| list.range.contains_range(i.range))
            .count()
    }

    /// Inline pieces inside a sentence: links (High) and footnote references.
    fn inline_pieces(&self, clip: CharRange, pieces: &mut Vec<Piece>) {
        let p = self.policy;
        let search = CharRange::new(clip.start, clip.end);
        for m in self.index.starting_in(search) {
            match m.kind {
                MarkerKind::Link if p.announces(Verbosity::High) => {
                    pieces.push(Piece::Insert(m.range.start, "link, ".to_owned()));
                }
                // Struck-through text is announced like a link: at high
                // verbosity, before it.
                MarkerKind::Strikethrough if p.announces(Verbosity::High) => {
                    pieces.push(Piece::Insert(m.range.start, "strikethrough, ".to_owned()));
                }
                MarkerKind::Footnote
                    if m.level == 0 && p.announce_structure && m.range.end <= clip.end =>
                {
                    let text = self.doc.slice(m.range);
                    let label = text.trim_matches(|c| c == '[' || c == ']' || c == '^');
                    let needs_space = m.range.start > clip.start
                        && self
                            .doc
                            .char_at(m.range.start.saturating_sub(1))
                            .is_some_and(|c| !c.is_whitespace());
                    let lead = if needs_space { " " } else { "" };
                    pieces.push(Piece::Replace(m.range, format!("{lead}footnote {label}")));
                }
                _ => {}
            }
        }
        pieces.sort_by_key(Piece::start);
    }

    /// The caller's changes inside a sentence ([`plan_with`]).
    fn caller_pieces(&self, clip: CharRange, pieces: &mut Vec<Piece>) {
        if self.inline.is_empty() {
            return;
        }
        let first = self.inline.partition_point(|c| c.range.end <= clip.start);
        let mut added = false;
        for c in &self.inline[first..] {
            if c.range.start >= clip.end {
                break;
            }
            let Some(part) = c.range.intersection(clip).filter(|p| !p.is_empty()) else {
                continue;
            };
            // Pieces the planner made itself (footnotes, links) win.
            let taken = pieces.iter().any(|p| match p {
                Piece::Replace(r, _) | Piece::Elide(r) => r.intersection(part).is_some_and(|x| !x.is_empty()),
                Piece::Insert(at, _) => part.start < *at && *at < part.end,
            });
            if taken {
                continue;
            }
            if c.range.start >= clip.start && !c.spoken.is_empty() {
                pieces.push(Piece::Replace(part, c.spoken.clone()));
            } else {
                pieces.push(Piece::Elide(part));
            }
            added = true;
        }
        if added {
            pieces.sort_by_key(Piece::start);
        }
    }

    /// Builds the spoken text of `clip` with inserts and replacements.
    fn build(&self, clip: CharRange, pieces: Vec<Piece>) -> (String, OffsetMap) {
        let mut b = SpokenBuilder::new();
        let mut at = clip.start;
        for piece in pieces {
            let start = piece.start().max(at);
            if start > at {
                b.push_literal(&self.doc.slice(CharRange::new(at, start)), at);
                at = start;
            }
            match piece {
                Piece::Insert(_, text) => b.push_inserted(&text, at),
                Piece::Replace(r, text) => {
                    if r.start >= at && r.end <= clip.end {
                        b.push_expanded(&text, r);
                        at = r.end;
                    }
                }
                Piece::Elide(r) => {
                    if r.start >= at && r.end <= clip.end {
                        b.push_elided(r);
                        at = r.end;
                    }
                }
            }
        }
        if clip.end > at {
            b.push_literal(&self.doc.slice(CharRange::new(at, clip.end)), at);
        }
        b.finish()
    }

    fn push_notice(&mut self, text: &str, at: CharPos) {
        let mut b = SpokenBuilder::new();
        b.push_inserted(text, at);
        let (t, m) = b.finish();
        self.push(t, m);
    }

    fn push(&mut self, text: String, map: OffsetMap) {
        if text.trim().is_empty() {
            return;
        }
        let max = self.policy.max_chunk_chars;
        self.out.extend(split_long(text, map, max));
    }

    fn table_info(&self, table: &Marker) -> TableInfo {
        let rows: Vec<&Marker> = self
            .index
            .starting_in(table.range)
            .iter()
            .filter(|r| r.kind == MarkerKind::TableRow && table.range.contains_range(r.range))
            .collect();
        let header = rows.iter().position(|r| r.is_header_row());
        let names = header
            .map(|h| {
                self.cells(rows[h])
                    .into_iter()
                    .map(|c| self.doc.slice(c).trim().to_owned())
                    .collect()
            })
            .unwrap_or_default();
        let columns = rows.iter().map(|r| self.cells(r).len()).max().unwrap_or(0);
        TableInfo {
            start: table.range.start,
            rows: rows.iter().map(|r| r.range).collect(),
            header,
            names,
            columns,
        }
    }

    fn table_row(&mut self, table: &Marker, row: &Marker, from: CharPos) {
        if self
            .table
            .as_ref()
            .is_none_or(|t| t.start != table.range.start)
        {
            self.table = Some(self.table_info(table));
        }
        let Some(info) = self.table.take() else {
            return;
        };
        self.table_row_with(&info, table, row, from);
        self.table = Some(info);
    }

    fn table_row_with(&mut self, info: &TableInfo, table: &Marker, row: &Marker, from: CharPos) {
        let p = self.policy;
        let columns = info.columns;
        let body_rows = info.body_rows();
        let first_row = info.rows.first().is_some_and(|r| *r == row.range);
        let at_table_start = first_row && from <= row.range.start;
        let plural = |n: usize, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "s" });

        if p.table_mode == TableNarration::Skip {
            if at_table_start && p.announces(Verbosity::Normal) {
                let text = format!("table with {}, skipped", plural(columns, "column"));
                self.push_notice(&text, table.range.start);
            }
            self.done_until = table.range.end;
            return;
        }
        self.done_until = row.range.end;
        let structured = p.table_mode == TableNarration::Structured && p.announce_structure;
        let cells: Vec<CharRange> = self
            .cells(row)
            .into_iter()
            .filter(|c| c.end > from || c.start >= from)
            .filter(|c| c.start < self.range.end)
            .collect();
        if cells.is_empty() {
            return;
        }
        let whole_row = from <= row.range.start;
        let mut b = SpokenBuilder::new();
        let anchor = cells[0].start;
        if row.is_header_row() {
            if structured && at_table_start && p.verbosity >= Verbosity::Normal {
                let rows_part = if p.verbosity >= Verbosity::High {
                    format!(" and {}", plural(body_rows, "row"))
                } else {
                    String::new()
                };
                let intro = format!("Table with {}{rows_part}: ", plural(columns, "column"));
                b.push_inserted(&intro, anchor);
            }
            self.cells_flat(&mut b, &cells);
        } else {
            if structured
                && at_table_start
                && info.header.is_none()
                && p.verbosity >= Verbosity::Normal
            {
                let intro = format!("Table with {}. ", plural(columns, "column"));
                b.push_inserted(&intro, anchor);
            }
            if structured && whole_row && p.verbosity >= Verbosity::Normal {
                let n = info.row_number(row.range);
                let label = if p.verbosity >= Verbosity::High {
                    format!("Row {n} of {body_rows}: ")
                } else {
                    format!("Row {n}: ")
                };
                b.push_inserted(&label, anchor);
            }
            if structured {
                let all = self.cells(row);
                let offset = all.len() - cells.len();
                self.cells_structured(&mut b, &cells, &info.names, offset);
            } else {
                self.cells_flat(&mut b, &cells);
            }
        }
        let (text, map) = b.finish();
        self.push(text, map);
    }

    fn cells(&self, row: &Marker) -> Vec<CharRange> {
        let span = CharRange::new(row.range.start, row.range.end.saturating_add(1));
        self.index
            .starting_in(span)
            .iter()
            .filter(|m| m.kind == MarkerKind::TableCell && m.range.end <= row.range.end)
            .map(|m| m.range)
            .collect()
    }

    /// Cells separated by ", ", ending with a period; empty cells skipped.
    fn cells_flat(&self, b: &mut SpokenBuilder, cells: &[CharRange]) {
        let mut prev_end: Option<CharPos> = None;
        let mut spoken_any = false;
        for c in cells {
            if let Some(e) = prev_end {
                b.push_elided(CharRange::new(e, c.start));
            }
            prev_end = Some(c.end);
            if c.is_empty() {
                continue;
            }
            if spoken_any {
                b.push_inserted(", ", c.start);
            }
            b.push_literal(&self.doc.slice(*c), c.start);
            spoken_any = true;
        }
        self.finish_row(b, cells);
    }

    /// "Name is Ada, Role is Engineer" with "blank" for empty cells.
    fn cells_structured(
        &self,
        b: &mut SpokenBuilder,
        cells: &[CharRange],
        names: &[String],
        offset: usize,
    ) {
        let mut prev_end: Option<CharPos> = None;
        for (i, c) in cells.iter().enumerate() {
            if let Some(e) = prev_end {
                b.push_elided(CharRange::new(e, c.start));
                b.push_inserted(", ", c.start);
            }
            prev_end = Some(c.end);
            if let Some(name) = names.get(i + offset).filter(|n| !n.is_empty()) {
                b.push_inserted(&format!("{name} is "), c.start);
            }
            if c.is_empty() {
                b.push_inserted("blank", c.start);
            } else {
                b.push_literal(&self.doc.slice(*c), c.start);
            }
        }
        self.finish_row(b, cells);
    }

    fn finish_row(&self, b: &mut SpokenBuilder, cells: &[CharRange]) {
        let ends_sentence = b
            .text()
            .trim_end()
            .ends_with(['.', '!', '?', '\u{2026}', ':']);
        if !b.text().is_empty() && !ends_sentence {
            let at = cells.last().map_or(CharPos::ZERO, |c| c.end);
            b.push_inserted(".", at);
        }
    }
}

/// Splits an utterance longer than `max` chars at whitespace (dropping the
/// whitespace), never inside an expanded token. A single run of more than
/// `max` chars without whitespace is cut at a char boundary.
fn split_long(text: String, map: OffsetMap, max: usize) -> Vec<(String, OffsetMap)> {
    if max == 0 || text.chars().count() <= max {
        return vec![(text, map)];
    }
    let spans = map.spans();
    let inside_expanded = |b: usize| {
        spans.iter().any(|s| {
            s.kind == SpanKind::Expanded
                && (s.spoken.start as usize) < b
                && b < s.spoken.end as usize
        })
    };
    let mut out = Vec::new();
    let mut start = 0usize;
    loop {
        let rest = &text[start..];
        if rest.chars().count() <= max {
            break;
        }
        let limit = start + rest.char_indices().nth(max).map_or(rest.len(), |(i, _)| i);
        // Last whitespace at or before `limit` that is a legal cut.
        let window_end = text[limit..]
            .chars()
            .next()
            .map_or(text.len(), |c| limit + c.len_utf8());
        let cut = text[start..window_end]
            .char_indices()
            .rev()
            .find(|&(i, c)| c.is_whitespace() && i > 0 && !inside_expanded(start + i))
            .map(|(i, _)| start + i);
        let (piece_end, next_start) = match cut {
            Some(ws) => {
                let mut a = ws;
                while a > start && text[..a].ends_with(char::is_whitespace) {
                    a -= text[..a].chars().next_back().map_or(1, char::len_utf8);
                }
                let mut z = ws;
                while z < text.len() && text[z..].starts_with(char::is_whitespace) {
                    z += text[z..].chars().next().map_or(1, char::len_utf8);
                }
                (a, z)
            }
            None => {
                let mut z = limit;
                if let Some(s) = spans.iter().find(|s| {
                    s.kind == SpanKind::Expanded
                        && (s.spoken.start as usize) < z
                        && z < s.spoken.end as usize
                }) {
                    z = s.spoken.end as usize;
                }
                (z, z)
            }
        };
        if piece_end <= start || next_start >= text.len() {
            break;
        }
        out.push(sub_utterance(&text, &map, start, piece_end));
        start = next_start;
    }
    out.push(sub_utterance(&text, &map, start, text.len()));
    out.retain(|(t, _)| !t.trim().is_empty());
    out
}

/// The part of an utterance between spoken bytes `a..b`, with its map
/// rebased to start at 0.
fn sub_utterance(text: &str, map: &OffsetMap, a: usize, b: usize) -> (String, OffsetMap) {
    let piece = text[a..b].to_owned();
    let (a32, b32) = (to_u32(a), to_u32(b));
    let mut spans: Vec<Span> = Vec::new();
    for s in map.spans() {
        if s.kind == SpanKind::Elided {
            let x = s.spoken.start;
            let inside = a32 <= x && (x < b32 || (x == b32 && b == text.len()));
            if inside {
                spans.push(Span {
                    spoken: (x - a32)..(x - a32),
                    source: s.source,
                    kind: s.kind,
                });
            }
            continue;
        }
        let x = s.spoken.start.max(a32);
        let y = s.spoken.end.min(b32);
        if x >= y {
            continue;
        }
        let source = match s.kind {
            SpanKind::Literal => {
                let skip = text[s.spoken.start as usize..x as usize].chars().count();
                let take = text[x as usize..y as usize].chars().count();
                let st = s.source.start.saturating_add(skip);
                CharRange::new(st, st.saturating_add(take))
            }
            SpanKind::Expanded | SpanKind::Inserted | SpanKind::Elided => s.source,
        };
        spans.push(Span {
            spoken: (x - a32)..(y - a32),
            source,
            kind: s.kind,
        });
    }
    let map = OffsetMap::from_spans(spans, &piece).unwrap_or_else(|_| {
        // Unreachable for maps built by `plan`; fall back to an anchor-only
        // map so the utterance still speaks.
        let mut b = SpokenBuilder::new();
        let at = map.source_extent().map_or(CharPos::ZERO, |r| r.start);
        b.push_inserted(&piece, at);
        b.finish().1
    });
    (piece, map)
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use ropey::Rope;

    use super::*;
    use crate::marker::HEADER_ROW_LABEL;
    use crate::{DocumentMeta, Marker};

    fn texts(us: &[Utterance]) -> Vec<&str> {
        us.iter().map(|u| u.text.as_str()).collect()
    }

    fn check(us: &[Utterance]) {
        for (i, u) in us.iter().enumerate() {
            u.offset_map.check_invariants(&u.text).unwrap();
            assert_eq!(u.id.chunk as usize, i);
        }
    }

    #[test]
    fn plans_one_utterance_per_sentence() {
        let d = Document::from_plain_text("One two. Three four.");
        let us = plan(&d, d.full_range(), &NarrationPolicy::default());
        check(&us);
        assert_eq!(us.len(), 2);
        assert_eq!(us[1].text, "Three four.");
        assert_eq!(us[1].source_range(), Some(CharRange::new(9, 20)));
        assert_eq!(
            us[1].source_for(0..5),
            Some(CharRange::new(CharPos(9), CharPos(14)))
        );
    }

    #[test]
    fn starts_mid_sentence() {
        let d = Document::from_plain_text("One two three. Four.");
        let us = plan(&d, CharRange::new(4, 20), &NarrationPolicy::default());
        check(&us);
        assert_eq!(texts(&us), ["two three.", "Four."]);
        assert_eq!(us[0].source_range(), Some(CharRange::new(4, 14)));
    }

    fn structured_doc() -> Document {
        // Heading, list, table, code, footnote.
        let text = "Title\n\nFirst\nSecond\n\nName | Score\nAda | 98\nBob | \n\nlet x = 1;\n\nSee this.[1]";
        let r = |a: usize, b: usize| CharRange::new(a, b);
        let markers = vec![
            Marker::new(MarkerKind::Heading, r(0, 5)).with_level(1),
            Marker::new(MarkerKind::List, r(7, 19)).with_level(1),
            Marker::new(MarkerKind::ListItem, r(7, 12))
                .with_level(1)
                .with_label("1."),
            Marker::new(MarkerKind::ListItem, r(13, 19))
                .with_level(1)
                .with_label("2."),
            Marker::new(MarkerKind::Table, r(21, 49)),
            Marker::new(MarkerKind::TableRow, r(21, 33)).with_label(HEADER_ROW_LABEL),
            Marker::new(MarkerKind::TableCell, r(21, 25)),
            Marker::new(MarkerKind::TableCell, r(28, 33)),
            Marker::new(MarkerKind::TableRow, r(34, 42)),
            Marker::new(MarkerKind::TableCell, r(34, 37)),
            Marker::new(MarkerKind::TableCell, r(40, 42)),
            Marker::new(MarkerKind::TableRow, r(43, 49)),
            Marker::new(MarkerKind::TableCell, r(43, 46)),
            Marker::new(MarkerKind::TableCell, r(49, 49)),
            Marker::new(MarkerKind::Code, r(51, 61)).with_level(1),
            Marker::new(MarkerKind::Footnote, r(72, 75)).with_reference("1"),
        ];
        let d = Document::new(DocumentMeta::default(), Rope::from_str(text), markers);
        assert_eq!(d.slice(r(43, 49)), "Bob | ");
        assert_eq!(d.slice(r(72, 75)), "[1]");
        d
    }

    #[test]
    fn structure_is_announced_as_inserted_spans() {
        let d = structured_doc();
        let us = plan(&d, d.full_range(), &NarrationPolicy::default());
        check(&us);
        assert_eq!(
            texts(&us),
            [
                "heading level 1, Title",
                "list with 2 items, 1. First",
                "2. Second",
                "Table with 2 columns: Name, Score.",
                "Row 1: Name is Ada, Score is 98.",
                "Row 2: Name is Bob, Score is blank.",
                "code block, let x = 1;",
                "See this. footnote 1",
            ]
        );
        // The heading announcement maps to nothing; the title to itself.
        let u = &us[0];
        assert_eq!(u.source_for(0..7), None);
        let t = u.text.find("Title").unwrap() as u32;
        assert_eq!(u.source_for(t..t + 5), Some(CharRange::new(0, 5)));
        // "98" highlights the cell.
        let row = &us[4];
        let at = row.text.find("98").unwrap() as u32;
        assert_eq!(row.source_for(at..at + 2), Some(CharRange::new(40, 42)));
        // "footnote 1" highlights "[1]".
        let f = &us[7];
        let at = f.text.find("footnote").unwrap() as u32;
        assert_eq!(f.source_for(at..at + 8), Some(CharRange::new(72, 75)));
    }

    #[test]
    fn rules_and_strikethrough_can_be_heard() {
        let text = "Keep old text.\n\nAfter the rule.";
        let r = |a: usize, b: usize| CharRange::new(a, b);
        let markers = vec![
            Marker::new(MarkerKind::Strikethrough, r(5, 8)),
            Marker::new(MarkerKind::Rule, r(16, 16)),
        ];
        let d = Document::new(DocumentMeta::default(), Rope::from_str(text), markers);
        let us = plan(&d, d.full_range(), &NarrationPolicy::default());
        check(&us);
        assert_eq!(texts(&us), ["Keep old text.", "separator, After the rule."]);
        let high = NarrationPolicy {
            verbosity: Verbosity::High,
            ..NarrationPolicy::default()
        };
        let us = plan(&d, d.full_range(), &high);
        check(&us);
        assert_eq!(
            texts(&us),
            [
                "Keep strikethrough, old text.",
                "separator, After the rule."
            ]
        );
        let quiet = NarrationPolicy {
            announce_structure: false,
            ..NarrationPolicy::default()
        };
        assert_eq!(
            texts(&plan(&d, d.full_range(), &quiet)),
            ["Keep old text.", "After the rule."]
        );
    }

    #[test]
    fn low_verbosity_and_flat_tables() {
        let d = structured_doc();
        let low = NarrationPolicy {
            verbosity: Verbosity::Low,
            table_mode: TableNarration::Flat,
            skip_code: true,
            ..NarrationPolicy::default()
        };
        let us = plan(&d, d.full_range(), &low);
        check(&us);
        assert_eq!(
            texts(&us),
            [
                "Title",
                "1. First",
                "2. Second",
                "Name, Score.",
                "Ada, 98.",
                "Bob.",
                "See this. footnote 1",
            ]
        );
        let none = NarrationPolicy {
            announce_structure: false,
            ..NarrationPolicy::default()
        };
        let us = plan(&d, d.full_range(), &none);
        check(&us);
        assert_eq!(texts(&us)[0], "Title");
        assert_eq!(texts(&us)[4], "Ada, 98.");
        assert_eq!(texts(&us)[7], "See this.[1]");
    }

    #[test]
    fn skipped_code_and_tables_are_announced() {
        let d = structured_doc();
        let skip = NarrationPolicy {
            skip_code: true,
            table_mode: TableNarration::Skip,
            ..NarrationPolicy::default()
        };
        let us = plan(&d, d.full_range(), &skip);
        check(&us);
        assert_eq!(
            texts(&us),
            [
                "heading level 1, Title",
                "list with 2 items, 1. First",
                "2. Second",
                "table with 2 columns, skipped",
                "code block skipped",
                "See this. footnote 1",
            ]
        );
        // Reading from inside the code block reads it.
        let us = plan(&d, CharRange::new(55, d.len_chars()), &skip);
        assert_eq!(texts(&us)[0], "x = 1;");
    }

    #[test]
    fn high_verbosity_numbers_rows() {
        let d = structured_doc();
        let high = NarrationPolicy {
            verbosity: Verbosity::High,
            ..NarrationPolicy::default()
        };
        let us = plan(&d, d.full_range(), &high);
        check(&us);
        assert_eq!(us[3].text, "Table with 2 columns and 2 rows: Name, Score.");
        assert_eq!(us[4].text, "Row 1 of 2: Name is Ada, Score is 98.");
    }

    #[test]
    fn long_sentences_split_at_whitespace() {
        let d = Document::from_plain_text("alpha beta gamma delta epsilon zeta.");
        let policy = NarrationPolicy {
            max_chunk_chars: 12,
            ..NarrationPolicy::default()
        };
        let us = plan(&d, d.full_range(), &policy);
        check(&us);
        assert_eq!(
            texts(&us),
            ["alpha beta", "gamma delta", "epsilon", "zeta."]
        );
        assert_eq!(us[1].source_range(), Some(CharRange::new(11, 22)));
        let long = Document::from_plain_text("abcdefghijklmnopqrstuvwxyz");
        let us = plan(&long, long.full_range(), &policy);
        check(&us);
        assert_eq!(texts(&us), ["abcdefghijkl", "mnopqrstuvwx", "yz"]);
        assert_eq!(us[2].source_range(), Some(CharRange::new(24, 26)));
    }

    /// Where each spoken word of `u` lands in the source.
    fn word_sources(u: &Utterance) -> Vec<(String, CharRange)> {
        let mut out = Vec::new();
        let mut start = None;
        let text = &u.text;
        for (i, c) in text.char_indices().chain(std::iter::once((text.len(), ' '))) {
            let word = c.is_alphanumeric() || c == ',';
            match (word, start) {
                (true, None) => start = Some(i),
                (false, Some(a)) => {
                    let w = text[a..i].trim_end_matches(',').to_owned();
                    if let Some(r) = u.source_for(a as u32..(a + w.len()) as u32) {
                        out.push((w, r));
                    }
                    start = None;
                }
                _ => {}
            }
        }
        out
    }

    #[test]
    fn caller_changes_skip_or_say_ranges_and_keep_highlights_exact() {
        let text = "Reading helps [@doe2020, p. 12] a lot. Next one.";
        let d = Document::from_plain_text(text);
        let cite = CharRange::new(14, 31);
        assert_eq!(d.slice(cite), "[@doe2020, p. 12]");
        let lot = CharRange::new(34, 37);
        let next = CharRange::new(39, 43);

        // Skipped: an Elided span; the words after it keep their places.
        let skip = [InlineSpeech::skip(CharRange::new(13, 31))];
        let us = plan_with(&d, d.full_range(), &NarrationPolicy::default(), &skip);
        check(&us);
        assert_eq!(texts(&us), ["Reading helps a lot.", "Next one."]);
        let map = &us[0].offset_map;
        assert!(map.spans().iter().any(|sp| sp.kind == SpanKind::Elided
            && sp.source == CharRange::new(13, 31)));
        let words = word_sources(&us[0]);
        assert!(words.contains(&("lot".to_owned(), lot)), "{words:?}");
        assert!(words.contains(&("helps".to_owned(), CharRange::new(8, 13))));
        assert_eq!(word_sources(&us[1])[0], ("Next".to_owned(), next));

        // Said in words: an Expanded span over the whole citation.
        let say = [InlineSpeech::say(cite, "Doe and Roe, 2020, page 12")];
        let us = plan_with(&d, d.full_range(), &NarrationPolicy::default(), &say);
        check(&us);
        assert_eq!(
            texts(&us),
            ["Reading helps Doe and Roe, 2020, page 12 a lot.", "Next one."]
        );
        let at = us[0].text.find("Roe").unwrap();
        assert_eq!(us[0].source_for(at as u32..at as u32 + 3), Some(cite));
        let words = word_sources(&us[0]);
        assert!(words.contains(&("lot".to_owned(), lot)), "{words:?}");
        assert_eq!(word_sources(&us[1])[0], ("Next".to_owned(), next));
        // Without changes the text is read as written.
        let plain = plan(&d, d.full_range(), &NarrationPolicy::default());
        assert!(plain[0].text.contains("[@doe2020, p. 12]"));
    }

    #[test]
    fn a_change_cut_by_a_sentence_end_is_said_once() {
        // "p." does not end a sentence, but "etc." can; the change is said
        // where it starts and skipped after the cut.
        let d = Document::from_plain_text("See it [@a, etc. More]. Then.");
        let r = CharRange::new(7, 23);
        let us = plan_with(
            &d,
            d.full_range(),
            &NarrationPolicy::default(),
            &[InlineSpeech::say(r, "A, 2020")],
        );
        check(&us);
        let joined = texts(&us).join(" | ");
        assert_eq!(joined.matches("A, 2020").count(), 1, "{joined}");
        assert!(!joined.contains("More"), "{joined}");
        assert!(joined.contains("Then."), "{joined}");
        // Overlapping changes after the first are ignored.
        let us = plan_with(
            &d,
            d.full_range(),
            &NarrationPolicy::default(),
            &[InlineSpeech::skip(r), InlineSpeech::say(CharRange::new(8, 10), "x")],
        );
        check(&us);
        assert!(!texts(&us).join(" ").contains('x'));
    }
}

#[cfg(test)]
mod props {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn utterances_are_valid_and_ordered(
            words in proptest::collection::vec("[a-zé]{1,12}[.,!?]?( |\n|\n\n)", 1..60),
            max in 5usize..80,
            a in 0usize..400,
        ) {
            let text = words.concat();
            let d = Document::from_plain_text(&text);
            let policy = NarrationPolicy { max_chunk_chars: max, ..NarrationPolicy::default() };
            let start = a.min(d.len_chars());
            let us = plan(&d, CharRange::new(start, d.len_chars()), &policy);
            let mut last = CharPos(start);
            for u in &us {
                u.offset_map.check_invariants(&u.text).unwrap();
                let r = u.source_range().unwrap();
                prop_assert!(r.start >= last);
                prop_assert!(u.text.chars().count() <= max);
                // Plain text: every spoken char is literal source text.
                prop_assert_eq!(d.slice(r), u.text.clone());
                last = r.end;
            }
        }
    }
}
