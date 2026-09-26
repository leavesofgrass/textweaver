//! Structure of Markdown source while it is edited, and the map between
//! source positions and the canonical text read aloud.
//!
//! Edit mode shows a Markdown file's source (ADR-0002). Reading uses the
//! canonical text the Markdown loader builds from it. This module parses the
//! source with pulldown-cmark, with the loader's options, and keeps every
//! marker at its **source** position (pulldown-cmark's byte offsets, turned
//! into char offsets), so heading, list, link, and table navigation, the
//! outline, and "say position" work on the text being written.
//!
//! The same markers carry positions between the two texts. The block
//! markers of the source (headings, paragraphs, list items, table cells,
//! code blocks) and of the canonical text come in the same order, so they
//! are paired one by one ([`SourceMap`]). A position is carried by the pair
//! it falls in (or the gap between two pairs), and inside that piece by
//! walking the canonical chars through the source, where every canonical
//! char appears in order and the markup is what gets skipped. That is local
//! work, proportional to one block, where the word alignment it replaces
//! ([`crate::align`]) allocated for every word of both texts. Files of
//! 256 KB and more are parsed in the background when they open, and while
//! typing in them the markers are dropped rather than shifted on every
//! keystroke. With `cargo xtask bench` on a 10 MB file, entering edit mode
//! went from 276 ms and 213 MB of peak heap to 52 ms and 121 MB (on a
//! busier machine), and typing stays at a few hundredths of a
//! millisecond per key.
//!
//! Differences from the canonical markers, all deliberate:
//!
//! - ranges cover source text: a heading's range is its text without the
//!   `#` marks; a link's is its text without `[`, `](url)`; an inline code
//!   span, math, and a footnote reference cover their delimiters, so spell
//!   checking can skip them whole;
//! - a footnote definition is a level-1 `Footnote` marker where it is
//!   written, not a deferred section at the end;
//! - a GFM alert's label ("Note:") is not text in the source.

use std::time::{Duration, Instant};

use pulldown_cmark::{BlockQuoteKind, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use textweaver_core::{CharPos, CharRange, MarkerKind};
use textweaver_text::{Document, HEADER_ROW_LABEL, Marker};

use crate::app::App;

/// The Markdown loader's parser options (`textweaver-formats`), so the
/// source parses into the same structure the reader sees.
pub(crate) fn parser_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
        | Options::ENABLE_MATH
        | Options::ENABLE_GFM
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_WIKILINKS
}

/// A marker whose range is still in bytes.
struct Pending {
    kind: MarkerKind,
    start: usize,
    end: usize,
    level: u8,
    label: Option<String>,
    reference: Option<String>,
}

/// An open element while parsing.
struct Frame {
    /// The marker this element becomes, if any.
    kind: Option<MarkerKind>,
    /// The element's byte range.
    start: usize,
    end: usize,
    /// The bytes covered by its inline content, when it has any.
    inner: Option<(usize, usize)>,
    level: u8,
    label: Option<String>,
    reference: Option<String>,
}

impl Frame {
    fn new(kind: Option<MarkerKind>, range: &std::ops::Range<usize>) -> Self {
        Frame {
            kind,
            start: range.start,
            end: range.end,
            inner: None,
            level: 0,
            label: None,
            reference: None,
        }
    }
}

/// Byte offset of the end of `s[start..end]` with trailing whitespace
/// removed (never before `start`).
fn trim_end(s: &str, start: usize, end: usize) -> usize {
    let end = end.min(s.len()).max(start);
    start + s[start..end].trim_end().len()
}

/// Byte offset of `s[start..end]` with leading whitespace removed.
fn trim_start(s: &str, start: usize, end: usize) -> usize {
    let end = end.min(s.len()).max(start);
    end - s[start..end].trim_start().len()
}

/// The markers of Markdown `source`, at source positions (chars).
pub(crate) fn source_markers(source: &str) -> Vec<Marker> {
    let mut out: Vec<Pending> = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    // Depth of open lists, and of open footnote definitions (whose blocks
    // the loader does not mark).
    let mut lists: Vec<Option<u64>> = Vec::new();
    let mut in_footnote = 0usize;
    for (event, range) in Parser::new_ext(source, parser_options()).into_offset_iter() {
        match event {
            Event::Start(tag) => {
                let frame = match tag {
                    Tag::Paragraph => {
                        let marked = lists.is_empty() && in_footnote == 0;
                        Frame::new(marked.then_some(MarkerKind::Paragraph), &range)
                    }
                    Tag::Heading { level, .. } => {
                        let mut f = Frame::new(Some(MarkerKind::Heading), &range);
                        f.level = level as u8;
                        f
                    }
                    Tag::BlockQuote(kind) => {
                        let mut f = Frame::new(Some(MarkerKind::Quote), &range);
                        f.label = kind.map(|k| alert_name(k).to_owned());
                        f
                    }
                    Tag::CodeBlock(kind) => {
                        let mut f = Frame::new(Some(MarkerKind::Code), &range);
                        f.level = 1;
                        if let CodeBlockKind::Fenced(lang) = kind {
                            let lang = lang.split_whitespace().next().unwrap_or("");
                            if !lang.is_empty() {
                                f.label = Some(lang.to_owned());
                            }
                        }
                        f
                    }
                    Tag::List(start) => {
                        lists.push(start);
                        let mut f = Frame::new(Some(MarkerKind::List), &range);
                        f.level = u8::try_from(lists.len()).unwrap_or(u8::MAX);
                        f
                    }
                    Tag::Item => {
                        let mut f = Frame::new(Some(MarkerKind::ListItem), &range);
                        f.level = u8::try_from(lists.len().max(1)).unwrap_or(u8::MAX);
                        if let Some(Some(n)) = lists.last_mut() {
                            f.label = Some(format!("{n}."));
                            *n += 1;
                        }
                        f
                    }
                    Tag::FootnoteDefinition(label) => {
                        in_footnote += 1;
                        let mut f = Frame::new(Some(MarkerKind::Footnote), &range);
                        f.level = 1;
                        f.reference = Some(label.to_string());
                        f
                    }
                    Tag::Table(_) => Frame::new(Some(MarkerKind::Table), &range),
                    Tag::TableHead => {
                        let mut f = Frame::new(Some(MarkerKind::TableRow), &range);
                        f.label = Some(HEADER_ROW_LABEL.to_owned());
                        f
                    }
                    Tag::TableRow => Frame::new(Some(MarkerKind::TableRow), &range),
                    Tag::TableCell => Frame::new(Some(MarkerKind::TableCell), &range),
                    Tag::Emphasis => Frame::new(Some(MarkerKind::Italic), &range),
                    Tag::Strong => Frame::new(Some(MarkerKind::Bold), &range),
                    Tag::Strikethrough => Frame::new(Some(MarkerKind::Strikethrough), &range),
                    Tag::Link { dest_url, .. } => {
                        let mut f = Frame::new(Some(MarkerKind::Link), &range);
                        f.reference = Some(dest_url.to_string());
                        f
                    }
                    Tag::Image { dest_url, .. } => {
                        let mut f = Frame::new(Some(MarkerKind::Image), &range);
                        f.reference = Some(dest_url.to_string());
                        f
                    }
                    Tag::HtmlBlock
                    | Tag::MetadataBlock(_)
                    | Tag::Superscript
                    | Tag::Subscript
                    | Tag::DefinitionList
                    | Tag::DefinitionListTitle
                    | Tag::DefinitionListDefinition => Frame::new(None, &range),
                };
                stack.push(frame);
            }
            Event::End(tag) => {
                let Some(f) = stack.pop() else { continue };
                match tag {
                    TagEnd::List(_) => {
                        lists.pop();
                    }
                    TagEnd::FootnoteDefinition => in_footnote = in_footnote.saturating_sub(1),
                    _ => {}
                }
                if let Some(p) = finish(source, f) {
                    out.push(p);
                }
            }
            Event::Text(_) | Event::Html(_) | Event::InlineHtml(_) => {
                note_inline(&mut stack, &range);
            }
            Event::Code(_) => {
                note_inline(&mut stack, &range);
                out.push(Pending {
                    kind: MarkerKind::Code,
                    start: range.start,
                    end: range.end,
                    level: 0,
                    label: None,
                    reference: None,
                });
            }
            Event::InlineMath(_) | Event::DisplayMath(_) => {
                note_inline(&mut stack, &range);
                let display = matches!(event, Event::DisplayMath(_));
                out.push(Pending {
                    kind: MarkerKind::Math,
                    start: range.start,
                    end: range.end,
                    level: u8::from(display),
                    label: None,
                    reference: None,
                });
            }
            Event::FootnoteReference(label) => {
                note_inline(&mut stack, &range);
                out.push(Pending {
                    kind: MarkerKind::Footnote,
                    start: range.start,
                    end: range.end,
                    level: 0,
                    label: None,
                    reference: Some(label.to_string()),
                });
            }
            Event::TaskListMarker(done) => {
                let state = if done { "checked" } else { "not checked" };
                if let Some(item) = stack
                    .iter_mut()
                    .rev()
                    .find(|f| f.kind == Some(MarkerKind::ListItem))
                {
                    item.label = Some(match item.label.take() {
                        Some(n) => format!("{n} {state}"),
                        None => state.to_owned(),
                    });
                }
            }
            Event::Rule => out.push(Pending {
                kind: MarkerKind::Rule,
                start: range.start,
                end: range.start,
                level: 0,
                label: None,
                reference: None,
            }),
            Event::SoftBreak | Event::HardBreak => {}
        }
    }
    to_char_markers(source, out)
}

/// A GFM alert's label, lowercase, as the loader writes it.
fn alert_name(kind: BlockQuoteKind) -> &'static str {
    match kind {
        BlockQuoteKind::Note => "note",
        BlockQuoteKind::Tip => "tip",
        BlockQuoteKind::Important => "important",
        BlockQuoteKind::Warning => "warning",
        BlockQuoteKind::Caution => "caution",
    }
}

/// Every open element holds this inline content.
fn note_inline(stack: &mut [Frame], range: &std::ops::Range<usize>) {
    for f in stack.iter_mut() {
        f.inner = Some(match f.inner {
            Some((a, b)) => (a.min(range.start), b.max(range.end)),
            None => (range.start, range.end),
        });
    }
}

/// The marker an element becomes, with its source range settled.
fn finish(source: &str, f: Frame) -> Option<Pending> {
    let kind = f.kind?;
    let whole = (f.start, trim_end(source, f.start, f.end));
    let (start, end) = match kind {
        // The text without its markup: `## Title` is `Title`, `[x](y)` is
        // `x`, a list item starts at its text (after `- ` or `[ ]`).
        MarkerKind::Heading | MarkerKind::Link | MarkerKind::Image => {
            f.inner.map_or(whole, |(a, b)| (a, trim_end(source, a, b)))
        }
        MarkerKind::ListItem => match f.inner {
            Some((a, _)) => (a.max(f.start), whole.1.max(a)),
            None => whole,
        },
        MarkerKind::Code => f.inner.map_or(whole, |(a, b)| (a, trim_end(source, a, b))),
        MarkerKind::TableCell => {
            let a = trim_start(source, f.start, f.end);
            let b = trim_end(source, a, f.end);
            match f.inner {
                Some((x, y)) => (x.max(a), y.min(b).max(x.max(a))),
                None => (a, a),
            }
        }
        _ => whole,
    };
    Some(Pending {
        kind,
        start,
        end: end.max(start),
        level: f.level,
        label: f.label,
        reference: f.reference,
    })
}

/// Turns byte ranges into char ranges in one pass over the text.
fn to_char_markers(source: &str, pending: Vec<Pending>) -> Vec<Marker> {
    let conv = ByteToChar::new(
        source,
        pending.iter().flat_map(|p| [p.start, p.end]).collect(),
    );
    let mut markers: Vec<Marker> = pending
        .into_iter()
        .map(|p| {
            let mut m = Marker::new(p.kind, CharRange::new(conv.get(p.start), conv.get(p.end)))
                .with_level(p.level);
            m.label = p.label;
            m.reference = p.reference;
            m
        })
        .collect();
    // Sorted here (on the parsing thread, when there is one), so the
    // document's own sort finds them in order.
    markers.sort_by_key(Marker::sort_key);
    markers
}

/// Char offsets of a set of byte offsets, found in one pass.
pub(crate) struct ByteToChar {
    ascii: bool,
    bytes: Vec<usize>,
    chars: Vec<usize>,
}

impl ByteToChar {
    /// Prepares the char offsets of `offsets` (bytes of `text`).
    pub(crate) fn new(text: &str, mut offsets: Vec<usize>) -> Self {
        if text.is_ascii() {
            return ByteToChar {
                ascii: true,
                bytes: Vec::new(),
                chars: Vec::new(),
            };
        }
        offsets.sort_unstable();
        offsets.dedup();
        let mut chars = Vec::with_capacity(offsets.len());
        let mut n = 0usize;
        let mut it = text.char_indices().peekable();
        for &b in &offsets {
            while let Some(&(i, _)) = it.peek() {
                if i >= b {
                    break;
                }
                it.next();
                n += 1;
            }
            chars.push(n);
        }
        ByteToChar {
            ascii: false,
            bytes: offsets,
            chars,
        }
    }

    /// The char offset of byte `b` (one of the offsets given).
    pub(crate) fn get(&self, b: usize) -> usize {
        if self.ascii {
            return b;
        }
        match self.bytes.binary_search(&b) {
            Ok(i) => self.chars[i],
            Err(i) => i
                .checked_sub(1)
                .map_or(0, |j| self.chars[j] + (b - self.bytes[j])),
        }
    }
}

/// The kinds that pair the two texts' blocks.
fn is_anchor(m: &Marker) -> bool {
    match m.kind {
        MarkerKind::Heading
        | MarkerKind::Paragraph
        | MarkerKind::ListItem
        | MarkerKind::TableCell => true,
        MarkerKind::Code => m.level == 1,
        _ => false,
    }
}

/// Two blocks, one per text, that hold the same content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Pair {
    canon: CharRange,
    source: CharRange,
}

/// Carries positions between the canonical text and the Markdown source it
/// was built from, by their paired blocks (see the module notes).
#[derive(Clone, Debug, Default)]
pub(crate) struct SourceMap {
    pairs: Vec<Pair>,
}

/// How far ahead in the source a canonical char is looked for before it is
/// taken to be text the loader added (a GFM alert's "Note:", a footnote
/// label).
const WINDOW: usize = 64;

impl SourceMap {
    /// Pairs the block markers of `canonical` and `source` in order. Where
    /// the sequences differ (a block one side has and the other has not),
    /// the nearer match within a few blocks wins and the rest is skipped.
    pub(crate) fn build(canonical: &Document, source: &Document) -> Self {
        let key = |m: &Marker| {
            (
                m.kind,
                if m.kind == MarkerKind::Heading {
                    m.level
                } else {
                    0
                },
            )
        };
        let a: Vec<&Marker> = canonical
            .markers()
            .iter()
            .filter(|m| is_anchor(m))
            .collect();
        let b: Vec<&Marker> = source.markers().iter().filter(|m| is_anchor(m)).collect();
        let mut pairs = Vec::with_capacity(a.len().min(b.len()));
        let (mut i, mut j) = (0, 0);
        const LOOK: usize = 8;
        while i < a.len() && j < b.len() {
            if key(a[i]) == key(b[j]) {
                pairs.push(Pair {
                    canon: a[i].range,
                    source: b[j].range,
                });
                i += 1;
                j += 1;
                continue;
            }
            let skip_b = (1..=LOOK).find(|&k| b.get(j + k).is_some_and(|m| key(m) == key(a[i])));
            let skip_a = (1..=LOOK).find(|&k| a.get(i + k).is_some_and(|m| key(m) == key(b[j])));
            match (skip_a, skip_b) {
                (Some(x), Some(y)) if x <= y => i += x,
                (_, Some(y)) => j += y,
                (Some(x), None) => i += x,
                (None, None) => {
                    i += 1;
                    j += 1;
                }
            }
        }
        // Pairs must rise in both texts; drop any that would cross.
        let mut clean: Vec<Pair> = Vec::with_capacity(pairs.len());
        for p in pairs {
            if clean
                .last()
                .is_none_or(|q| p.canon.start >= q.canon.start && p.source.start >= q.source.start)
            {
                clean.push(p);
            }
        }
        SourceMap { pairs: clean }
    }

    /// True when no block was paired (plain text, or no structure).
    pub(crate) fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// The pieces holding canonical position `p` (or source position `p`,
    /// with `in_source`): a paired block, or the gap before the next one.
    fn piece(
        &self,
        p: CharPos,
        in_source: bool,
        canon_len: usize,
        source_len: usize,
    ) -> (CharRange, CharRange) {
        let start_of = |x: &Pair| {
            if in_source {
                x.source.start
            } else {
                x.canon.start
            }
        };
        let idx = self.pairs.partition_point(|x| start_of(x) <= p);
        let Some(k) = idx.checked_sub(1) else {
            // Before the first block.
            let (c_end, s_end) = self.pairs.first().map_or((canon_len, source_len), |f| {
                (f.canon.start.0, f.source.start.0)
            });
            return (CharRange::new(0, c_end), CharRange::new(0, s_end));
        };
        let x = self.pairs[k];
        let inside = if in_source {
            p <= x.source.end
        } else {
            p <= x.canon.end
        };
        if inside {
            return (x.canon, x.source);
        }
        // In the gap after this block: up to the next block that starts
        // after it.
        let next = self.pairs[k + 1..]
            .iter()
            .find(|y| y.canon.start >= x.canon.end && y.source.start >= x.source.end);
        let (c_end, s_end) = next.map_or((canon_len, source_len), |y| {
            (y.canon.start.0, y.source.start.0)
        });
        (
            CharRange::new(x.canon.end, c_end.max(x.canon.end.0)),
            CharRange::new(x.source.end, s_end.max(x.source.end.0)),
        )
    }

    /// The source position of canonical position `p`.
    pub(crate) fn to_source(&self, canonical: &Document, source: &Document, p: CharPos) -> CharPos {
        let p = p.clamp_to(canonical.len_chars());
        let (c, s) = self.piece(p, false, canonical.len_chars(), source.len_chars());
        let walk = Walk::new(canonical, c, source, s);
        walk.canon_to_source(p.0 - c.start.0)
            .clamp_to(source.len_chars())
    }

    /// The canonical position of source position `p`.
    pub(crate) fn to_canonical(
        &self,
        canonical: &Document,
        source: &Document,
        p: CharPos,
    ) -> CharPos {
        let p = p.clamp_to(source.len_chars());
        let (c, s) = self.piece(p, true, canonical.len_chars(), source.len_chars());
        let walk = Walk::new(canonical, c, source, s);
        walk.source_to_canon(p.0.saturating_sub(s.start.0))
            .clamp_to(canonical.len_chars())
    }
}

/// One piece of each text, walked together: every canonical char is found
/// in the source in order (whitespace matches whitespace), and what is
/// skipped is markup.
struct Walk {
    canon: Vec<char>,
    source: Vec<char>,
    c0: usize,
    s0: usize,
}

fn same(a: char, b: char) -> bool {
    a == b || (a.is_whitespace() && b.is_whitespace())
}

impl Walk {
    fn new(canonical: &Document, c: CharRange, source: &Document, s: CharRange) -> Self {
        let c = c.clamp_to(canonical.len_chars());
        let s = s.clamp_to(source.len_chars());
        Walk {
            canon: canonical.text().slice(c.to_range()).chars().collect(),
            source: source.text().slice(s.to_range()).chars().collect(),
            c0: c.start.0,
            s0: s.start.0,
        }
    }

    /// Where canonical char `i` is in the source piece, searching from `from`.
    fn find(&self, i: usize, from: usize) -> Option<usize> {
        let ch = *self.canon.get(i)?;
        let end = (from + WINDOW).min(self.source.len());
        (from..end).find(|&k| same(ch, self.source[k]))
    }

    /// The source position of the canonical char `offset` into the piece.
    fn canon_to_source(&self, offset: usize) -> CharPos {
        let mut s = 0usize;
        for i in 0..offset.min(self.canon.len()) {
            if let Some(k) = self.find(i, s) {
                s = k + 1;
            }
        }
        let at = if offset < self.canon.len() {
            self.find(offset, s).unwrap_or(s)
        } else {
            s
        };
        CharPos(self.s0 + at.min(self.source.len()))
    }

    /// The canonical position of the source char `offset` into the piece:
    /// the first canonical char found at or after it.
    fn source_to_canon(&self, offset: usize) -> CharPos {
        let mut s = 0usize;
        for i in 0..self.canon.len() {
            match self.find(i, s) {
                Some(k) if k >= offset => return CharPos(self.c0 + i),
                Some(k) => s = k + 1,
                None => {}
            }
        }
        CharPos(self.c0 + self.canon.len())
    }
}

/// How long typing must pause before the structure is parsed again.
pub(crate) const REPARSE_PAUSE: Duration = Duration::from_millis(300);

/// Markdown files at least this large have their source structure parsed
/// in the background as soon as they open, so entering edit mode does not
/// wait for it (a 10 MB file takes a few hundred milliseconds to parse).
pub(crate) const PREFETCH_MIN_BYTES: u64 = 256 * 1024;

/// The source structure of the open Markdown file, parsed in the
/// background after it opened, for the next Ctrl+E.
#[derive(Debug, Default)]
pub(crate) struct Prefetch {
    /// The file and its stamp when it was read.
    key: Option<(std::path::PathBuf, crate::disk::FileStamp)>,
    /// The parse, while it runs.
    rx: Option<std::sync::mpsc::Receiver<Vec<Marker>>>,
}

/// The Markdown source of `path` as edit mode holds it (decoded, `\n`
/// line breaks).
fn source_for_edit(path: &std::path::Path) -> Option<String> {
    let text =
        textweaver_formats::source_text(&textweaver_formats::Source::Path(path.to_owned())).ok()?;
    Some(if text.contains('\r') {
        text.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        text
    })
}

/// Texts up to this many chars are parsed on the UI thread (a few
/// milliseconds); longer ones on a worker thread.
pub(crate) const INLINE_PARSE_LIMIT: usize = 262_144;

impl App {
    /// Notes that the text being edited changed: the markers were shifted
    /// by the edit and are parsed again once typing pauses.
    pub(crate) fn structure_edited(&mut self) {
        let st = &mut self.authoring.structure;
        if st.markdown {
            st.version += 1;
            st.last_edit = Some(Instant::now());
        }
    }

    /// Replaces the edited document's markers with `markers`, parsed from
    /// its current text.
    fn set_source_markers(&mut self, markers: Vec<Marker>) {
        if let Some(s) = self.session.as_mut() {
            let meta = s.doc.meta.clone();
            let text = s.doc.text().clone();
            s.doc = Document::new(meta, text, markers);
        }
        self.authoring.structure.parsed = self.authoring.structure.version;
    }

    /// Parses the edited text now when its markers are stale, for a
    /// command that needs the structure at once (navigation, the outline,
    /// leaving edit mode). A small text is parsed at once; a large one when
    /// `force` is set or its markers were dropped while typing, waiting for
    /// a background parse of the same text rather than starting another.
    pub(crate) fn refresh_structure(&mut self, force: bool) {
        let st = &self.authoring.structure;
        if self.edit.is_none() || !st.markdown || st.fresh() {
            return;
        }
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let large = s.doc.len_chars() > INLINE_PARSE_LIMIT;
        if large && !force && !s.doc.markers().is_empty() {
            // Shifted by the edits: good enough until typing pauses.
            return;
        }
        if let Some((version, rx)) = self.authoring.structure.pending.take()
            && version == self.authoring.structure.version
            && let Ok(markers) = rx.recv_timeout(Duration::from_secs(30))
        {
            self.set_source_markers(markers);
            return;
        }
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let text = s.doc.text().to_string();
        let markers = source_markers(&text);
        self.authoring.structure.pending = None;
        self.set_source_markers(markers);
    }

    /// From [`App::tick`] while editing: applies a finished background
    /// parse, and starts a parse once typing has paused for
    /// [`REPARSE_PAUSE`]. True when the markers changed.
    pub(crate) fn structure_tick(&mut self, now: Instant) -> bool {
        if self.edit.is_none() || !self.authoring.structure.markdown {
            self.authoring.structure.pending = None;
            return false;
        }
        let mut changed = false;
        if let Some((version, rx)) = self.authoring.structure.pending.take() {
            match rx.try_recv() {
                Ok(markers) => {
                    // A parse of text that has changed since is dropped;
                    // the next pause parses again.
                    if version == self.authoring.structure.version {
                        self.set_source_markers(markers);
                        changed = true;
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    self.authoring.structure.pending = Some((version, rx));
                    return false;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {}
            }
        }
        let st = &self.authoring.structure;
        let paused = st
            .last_edit
            .is_none_or(|t| now.saturating_duration_since(t) >= REPARSE_PAUSE);
        if st.fresh() || !paused {
            return changed;
        }
        let Some(s) = self.session.as_ref() else {
            return changed;
        };
        if s.doc.len_chars() <= INLINE_PARSE_LIMIT {
            self.refresh_structure(false);
            return true;
        }
        let rope = s.doc.text().clone();
        let version = st.version;
        let (tx, rx) = std::sync::mpsc::channel();
        let wake = self.waker_slot();
        let spawned = std::thread::Builder::new()
            .name("tw-structure".into())
            .spawn(move || {
                let text = rope.to_string();
                let _ = tx.send(source_markers(&text));
                wake.wake();
            });
        match spawned {
            Ok(_) => self.authoring.structure.pending = Some((version, rx)),
            Err(e) => {
                log::warn!("cannot start the structure parser: {e}");
                self.refresh_structure(true);
                changed = true;
            }
        }
        changed
    }

    /// Starts parsing the open Markdown file's source structure in the
    /// background, when it is large enough to be worth it and was not
    /// parsed already (called when a document opens and after edit mode).
    pub(crate) fn prefetch_structure(&mut self) {
        if self.edit.is_some() {
            return;
        }
        let Some(path) = self
            .session
            .as_ref()
            .filter(|s| s.doc.meta.format == "markdown")
            .and_then(|s| s.doc.meta.path.clone())
        else {
            self.authoring.prefetch = Prefetch::default();
            return;
        };
        let Some(stamp) = crate::disk::FileStamp::of(&path) else {
            return;
        };
        let key = (path.clone(), stamp);
        if self.authoring.prefetch.key.as_ref() == Some(&key) {
            return;
        }
        if stamp.len < PREFETCH_MIN_BYTES {
            self.authoring.prefetch = Prefetch::default();
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("tw-structure-prefetch".into())
            .spawn(move || {
                if let Some(text) = source_for_edit(&path) {
                    let _ = tx.send(source_markers(&text));
                }
            });
        self.authoring.prefetch = match spawned {
            Ok(_) => Prefetch {
                key: Some(key),
                rx: Some(rx),
            },
            Err(e) => {
                log::warn!("cannot start the structure parser: {e}");
                Prefetch::default()
            }
        };
    }

    /// The structure parsed in the background for the open file, when the
    /// file is unchanged since (waiting for a parse still running).
    pub(crate) fn prefetched_markers(&mut self) -> Option<Vec<Marker>> {
        let p = std::mem::take(&mut self.authoring.prefetch);
        let (path, stamp) = p.key?;
        let open = self.session.as_ref().and_then(|s| s.doc.meta.path.clone());
        if open.as_ref() != Some(&path) || crate::disk::FileStamp::of(&path) != Some(stamp) {
            return None;
        }
        p.rx?.recv_timeout(Duration::from_secs(30)).ok()
    }

    /// Waits (up to `timeout`) for a background structure parse and applies
    /// it; for tests and benchmarks. True when the markers are current.
    pub fn wait_for_structure(&mut self, timeout: Duration) -> bool {
        if let Some((version, rx)) = self.authoring.structure.pending.take()
            && let Ok(markers) = rx.recv_timeout(timeout)
            && version == self.authoring.structure.version
        {
            self.set_source_markers(markers);
        }
        self.authoring.structure.fresh()
    }
}

#[cfg(test)]
mod tests {
    use textweaver_formats::{LoadOptions, Loader, MarkdownLoader, Source};

    use super::*;

    fn source_doc(src: &str) -> Document {
        Document::new(
            textweaver_text::DocumentMeta::default(),
            ropey::Rope::from_str(src),
            source_markers(src),
        )
    }

    fn canonical(src: &str) -> Document {
        MarkdownLoader
            .load(
                &Source::Bytes {
                    data: src.as_bytes().to_vec(),
                    hint: "md".into(),
                },
                &LoadOptions::default(),
            )
            .unwrap()
    }

    fn texts(d: &Document, kind: MarkerKind) -> Vec<String> {
        d.marker_index()
            .iter(kind, None)
            .map(|m| d.slice(m.range))
            .collect()
    }

    const SAMPLE: &str = "---\ntitle: T\n---\n\n# Title\n\nSome *text* and a [link](other.md#part).\n\n## Lists\n\n- one\n- [x] two\n  - nested\n\n1. first\n2. second\n\n| A | B |\n|---|---|\n| 1 |   |\n\n```rust\nfn main() {}\n```\n\nA note.[^1] And `code` and $x^2$.\n\n[^1]: The note.\n";

    #[test]
    fn markers_sit_on_the_source_text() {
        let d = source_doc(SAMPLE);
        assert_eq!(texts(&d, MarkerKind::Heading), ["Title", "Lists"]);
        assert_eq!(
            texts(&d, MarkerKind::ListItem),
            ["one", "two\n  - nested", "nested", "first", "second"]
        );
        let labels: Vec<Option<String>> = d
            .marker_index()
            .iter(MarkerKind::ListItem, None)
            .map(|m| m.label.clone())
            .collect();
        assert_eq!(
            labels,
            [
                None,
                Some("checked".into()),
                None,
                Some("1.".into()),
                Some("2.".into())
            ]
        );
        assert_eq!(texts(&d, MarkerKind::Link), ["link"]);
        let link = d.marker_index().nth(MarkerKind::Link, None, 0).cloned();
        assert_eq!(
            link.and_then(|m| m.reference).as_deref(),
            Some("other.md#part")
        );
        assert_eq!(texts(&d, MarkerKind::TableCell), ["A", "B", "1", ""]);
        let rows: Vec<bool> = d
            .marker_index()
            .iter(MarkerKind::TableRow, None)
            .map(Marker::is_header_row)
            .collect();
        assert_eq!(rows, [true, false]);
        let code: Vec<(String, u8)> = d
            .marker_index()
            .iter(MarkerKind::Code, None)
            .map(|m| (d.slice(m.range), m.level))
            .collect();
        assert_eq!(
            code,
            [("fn main() {}".to_owned(), 1), ("`code`".to_owned(), 0)]
        );
        assert_eq!(texts(&d, MarkerKind::Math), ["$x^2$"]);
        let notes: Vec<(String, u8)> = d
            .marker_index()
            .iter(MarkerKind::Footnote, None)
            .map(|m| (d.slice(m.range), m.level))
            .collect();
        assert_eq!(
            notes,
            [("[^1]".to_owned(), 0), ("[^1]: The note.".to_owned(), 1)]
        );
        // Paragraphs outside lists, as the loader marks them.
        assert_eq!(
            texts(&d, MarkerKind::Paragraph),
            [
                "Some *text* and a [link](other.md#part).",
                "A note.[^1] And `code` and $x^2$."
            ]
        );
    }

    #[test]
    fn multibyte_text_gets_char_ranges() {
        let src = "# Café naïve\n\n- ünïcode ✓\n";
        let d = source_doc(src);
        assert_eq!(texts(&d, MarkerKind::Heading), ["Café naïve"]);
        assert_eq!(texts(&d, MarkerKind::ListItem), ["ünïcode ✓"]);
    }

    #[test]
    fn positions_cross_between_source_and_canonical() {
        let c = canonical(SAMPLE);
        let s = source_doc(SAMPLE);
        let map = SourceMap::build(&c, &s);
        assert!(!map.is_empty());
        let ct = c.text().to_string();
        let st = s.text().to_string();
        for word in [
            "Title", "text", "link", "Lists", "nested", "second", "main", "note", "code",
        ] {
            let cp = CharPos(ct[..ct.find(word).unwrap()].chars().count());
            let sp = map.to_source(&c, &s, cp);
            let found = s.slice(CharRange::new(sp, sp.saturating_add(word.chars().count())));
            assert_eq!(found, word, "{word}: canonical {cp:?} to source {sp:?}");
            // And back.
            let back = map.to_canonical(&c, &s, sp);
            assert_eq!(back, cp, "{word}");
        }
        // Markup maps to the text it marks up.
        let hashes = CharPos(st.find("## Lists").unwrap());
        let back = map.to_canonical(&c, &s, hashes);
        assert_eq!(
            c.slice(CharRange::new(back, back.saturating_add(5))),
            "Lists"
        );
    }

    #[test]
    fn byte_to_char_handles_offsets_between_given_ones() {
        let text = "aé b";
        let conv = ByteToChar::new(text, vec![0, 3, 5]);
        assert_eq!(conv.get(0), 0);
        assert_eq!(conv.get(3), 2);
        assert_eq!(conv.get(5), 4);
        assert_eq!(conv.get(4), 3);
    }
}
