//! Builds canonical text and markers from a stream of structure and text.
//!
//! Loaders call [`Builder`] as they walk their source: open and close
//! markers, add inline text (whitespace collapsed), verbatim text (code), and
//! request line or paragraph breaks. Breaks and spaces are held back until
//! the next text arrives, so the canonical text never has leading, trailing,
//! or doubled whitespace around blocks, and markers never include the
//! separators around them (ADR-0002 canonical shape).

use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::Marker;

/// A pending separator, weakest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Break {
    None,
    Space,
    Line,
    Paragraph,
}

/// Handle to an open marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OpenId(usize);

#[derive(Debug)]
struct Open {
    id: usize,
    marker: Marker,
    /// Char offset of the first content, once some has been written.
    start: Option<usize>,
    /// Keep the marker even when it has no content (table cells).
    keep_empty: bool,
    /// An anchor (see [`Builder::open_anchor`]): its range is recorded
    /// under this key instead of becoming a marker.
    anchor: Option<String>,
}

/// Canonical text under construction.
#[derive(Debug, Default)]
pub(crate) struct Builder {
    text: String,
    len: usize,
    markers: Vec<Marker>,
    open: Vec<Open>,
    next_id: usize,
    pending: Option<Break>,
    line_has_text: bool,
    /// Empty markers waiting for the next content (a horizontal rule is
    /// placed where the block after it starts).
    points: Vec<Marker>,
    /// Closed anchors: a key and the range it covered.
    anchors: Vec<(String, CharRange)>,
    /// The pending space was requested by [`close_punct`](Self::close_punct):
    /// it is dropped if the next word starts with closing punctuation.
    soft_space: bool,
}

impl Builder {
    pub(crate) fn new() -> Self {
        Builder::default()
    }

    /// A builder with room for `bytes` of canonical text: a loader passes
    /// its source's length (the text is rarely longer than its source), so
    /// the text is not copied every time it doubles. On 10 MB that was 20
    /// MB of copying.
    pub(crate) fn with_capacity(bytes: usize) -> Self {
        Builder {
            text: String::with_capacity(bytes),
            ..Builder::default()
        }
    }

    fn request(&mut self, b: Break) {
        let cur = self.pending.unwrap_or(Break::None);
        self.pending = Some(cur.max(b));
    }

    /// Emits the pending separator and fixes the start of every marker that
    /// is waiting for its first content.
    fn flush(&mut self) {
        match self.pending.take() {
            Some(Break::Paragraph) if self.len > 0 => {
                let need = if self.text.ends_with("\n\n") {
                    ""
                } else if self.text.ends_with('\n') {
                    "\n"
                } else {
                    "\n\n"
                };
                self.push_raw(need);
            }
            Some(Break::Line) if self.len > 0 && !self.text.ends_with('\n') => self.push_raw("\n"),
            Some(Break::Space) if self.line_has_text => self.push_raw(" "),
            _ => {}
        }
        for o in &mut self.open {
            if o.start.is_none() {
                o.start = Some(self.len);
            }
        }
        for mut m in self.points.drain(..) {
            m.range = CharRange::empty(self.len);
            self.markers.push(m);
        }
    }

    fn push_raw(&mut self, s: &str) {
        self.text.push_str(s);
        // ASCII (most words) is one char per byte.
        self.len += if s.is_ascii() {
            s.len()
        } else {
            s.chars().count()
        };
        // A UTF-8 text ends with '\n' exactly when its last byte is one.
        if let Some(&last) = s.as_bytes().last() {
            self.line_has_text = last != b'\n';
        }
    }

    /// Inline text: whitespace runs collapse to one space, dropped at line
    /// starts and ends.
    pub(crate) fn text(&mut self, s: &str) {
        // Words are slices of `s`, not copies built a char at a time.
        let mut start = None;
        for (i, c) in s.char_indices() {
            if c.is_whitespace() {
                if let Some(from) = start.take() {
                    self.word(&s[from..i]);
                }
                self.request(Break::Space);
                self.soft_space = false;
            } else if start.is_none() {
                start = Some(i);
            }
        }
        if let Some(from) = start {
            self.word(&s[from..]);
        }
    }

    fn word(&mut self, word: &str) {
        if self.soft_space
            && self.pending == Some(Break::Space)
            && word.starts_with(['.', ',', ';', ':', '!', '?', ')', ']'])
        {
            self.pending = None;
        }
        self.soft_space = false;
        self.flush();
        self.push_raw(word);
    }

    /// Closing text such as `")"` joined to the text before it (a pending
    /// space is dropped), then a space that is left out if the next word
    /// starts with punctuation: "(deleted by Ada Example: old) new", and
    /// "(inserted by Ada Example: s)." after a word.
    pub(crate) fn close_punct(&mut self, s: &str) {
        if self.pending == Some(Break::Space) {
            self.pending = None;
        }
        self.literal(s);
        self.soft_space();
    }

    /// A space that is left out if the next word starts with closing
    /// punctuation (after an image's alt text: "a crow." not "a crow .").
    pub(crate) fn soft_space(&mut self) {
        self.request(Break::Space);
        self.soft_space = true;
    }

    /// Text written exactly as given (after any pending separator): used for
    /// separators such as `" | "` between table cells.
    pub(crate) fn literal(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        self.flush();
        self.push_raw(s);
    }

    /// A separator that brings its own spacing (the `" | "` between table
    /// cells): a pending space before it is dropped, and a space requested
    /// right after it (leading whitespace in the next cell) is not written.
    pub(crate) fn separator(&mut self, s: &str) {
        if self.pending == Some(Break::Space) {
            self.pending = None;
        }
        self.literal(s);
        self.line_has_text = false;
    }

    /// Verbatim text (code): spaces and line breaks kept, leading and
    /// trailing blank lines dropped, `\r\n` normalized.
    pub(crate) fn verbatim(&mut self, s: &str) {
        // Nearly all code has no '\r': borrowed, not copied twice.
        let s: std::borrow::Cow<'_, str> = if s.contains('\r') {
            s.replace("\r\n", "\n").replace('\r', "\n").into()
        } else {
            s.into()
        };
        let s = s.trim_matches('\n');
        let s = s.trim_end();
        if s.trim().is_empty() {
            return;
        }
        self.flush();
        self.push_raw(s);
    }

    /// A space between inline items, if text follows on this line.
    pub(crate) fn space(&mut self) {
        self.request(Break::Space);
    }

    /// Starts a new line before the next text (list items, table rows,
    /// `<br>`).
    pub(crate) fn line_break(&mut self) {
        self.request(Break::Line);
    }

    /// Starts a new paragraph (a blank line) before the next text.
    pub(crate) fn paragraph_break(&mut self) {
        self.request(Break::Paragraph);
    }

    /// An empty marker at the start of the next content (after the pending
    /// separator), or at the end when no content follows.
    pub(crate) fn point(&mut self, marker: Marker) {
        self.points.push(marker);
    }

    /// Opens a marker whose range starts at its first content.
    pub(crate) fn open(&mut self, marker: Marker) -> OpenId {
        self.open_with(marker, false)
    }

    /// Opens a marker at the current position (after the pending separator),
    /// kept even if nothing is written into it (an empty table cell).
    pub(crate) fn open_here(&mut self, marker: Marker) -> OpenId {
        self.flush();
        self.open_with(marker, true)
    }

    fn open_with(&mut self, marker: Marker, keep_empty: bool) -> OpenId {
        let id = self.next_id;
        self.next_id += 1;
        let start = keep_empty.then_some(self.len);
        self.open.push(Open {
            id,
            marker,
            start,
            keep_empty,
            anchor: None,
        });
        OpenId(id)
    }

    /// Opens an anchor: a range that starts at the next content and is
    /// kept, under `key`, when closed (empty at the close point when no
    /// content came). Anchors are not markers: loaders use them for
    /// comments, which travel in the document's properties.
    pub(crate) fn open_anchor(&mut self, key: String) -> OpenId {
        let id = self.next_id;
        self.next_id += 1;
        self.open.push(Open {
            id,
            marker: Marker::new(MarkerKind::Paragraph, CharRange::empty(0)),
            start: None,
            keep_empty: true,
            anchor: Some(key),
        });
        OpenId(id)
    }

    /// The open anchor with `key`, if any.
    pub(crate) fn open_anchor_id(&self, key: &str) -> Option<OpenId> {
        self.open
            .iter()
            .find(|o| o.anchor.as_deref() == Some(key))
            .map(|o| OpenId(o.id))
    }

    /// The innermost open marker of `kind`, to add to its label (a task
    /// list item's "checked").
    pub(crate) fn innermost_open_mut(&mut self, kind: MarkerKind) -> Option<&mut Marker> {
        self.open
            .iter_mut()
            .rev()
            .map(|o| &mut o.marker)
            .find(|m| m.kind == kind)
    }

    /// Where the innermost open marker of one of `kinds` that has content
    /// starts (a block id names the paragraph or list item it ends).
    pub(crate) fn innermost_open_start(&self, kinds: &[MarkerKind]) -> Option<usize> {
        self.open
            .iter()
            .rev()
            .find(|o| o.anchor.is_none() && kinds.contains(&o.marker.kind) && o.start.is_some())
            .and_then(|o| o.start)
    }

    /// Where the most recently closed marker of one of `kinds` starts (a
    /// block id on a line of its own names the table or list before it).
    pub(crate) fn last_closed_start(&self, kinds: &[MarkerKind]) -> Option<usize> {
        self.markers
            .iter()
            .rev()
            .find(|m| kinds.contains(&m.kind))
            .map(|m| m.range.start.0)
    }

    /// Closes a marker. Markers that received no content are dropped unless
    /// opened with [`open_here`](Self::open_here).
    pub(crate) fn close(&mut self, id: OpenId) {
        let Some(i) = self.open.iter().rposition(|o| o.id == id.0) else {
            return;
        };
        let o = self.open.remove(i);
        let start = match (o.start, o.keep_empty) {
            (Some(s), _) => s,
            (None, true) => self.len,
            (None, false) => return,
        };
        let range = CharRange::new(start, self.len.max(start));
        if let Some(key) = o.anchor {
            self.anchors.push((key, range));
            return;
        }
        let mut marker = o.marker;
        marker.range = range;
        self.markers.push(marker);
    }

    /// A footnote reference: `[label]` under a `Footnote` marker (level 0,
    /// reference = label).
    pub(crate) fn footnote_reference(&mut self, label: &str) {
        let id =
            self.open(Marker::new(MarkerKind::Footnote, CharRange::empty(0)).with_reference(label));
        self.literal(&format!("[{label}]"));
        self.close(id);
    }

    /// A footnote read in place: ` (footnote: text)` under a level-1
    /// `Footnote` marker.
    pub(crate) fn inline_footnote(&mut self, label: &str, text: &str) {
        self.space();
        let id = self.open(
            Marker::new(MarkerKind::Footnote, CharRange::empty(0))
                .with_level(1)
                .with_reference(label),
        );
        self.text(&format!("(footnote: {text})"));
        self.close(id);
    }

    /// The deferred footnotes after the text: a level-2 "Footnotes" heading,
    /// then one line per note, `[label] text`, each a level-1 `Footnote`
    /// marker holding a reference marker for its label.
    pub(crate) fn footnotes_section(&mut self, notes: &[(String, String)]) {
        if notes.is_empty() {
            return;
        }
        self.paragraph_break();
        let h = self.open(Marker::new(MarkerKind::Heading, CharRange::empty(0)).with_level(2));
        self.text("Footnotes");
        self.close(h);
        self.paragraph_break();
        for (label, text) in notes {
            self.line_break();
            let body = self.open(
                Marker::new(MarkerKind::Footnote, CharRange::empty(0))
                    .with_level(1)
                    .with_reference(label.as_str()),
            );
            self.footnote_reference(label);
            self.space();
            self.text(text);
            self.close(body);
        }
    }

    /// The canonical text and its markers; open markers are closed.
    pub(crate) fn finish(self) -> (String, Vec<Marker>) {
        let (text, markers, _) = self.finish_with_anchors();
        (text, markers)
    }

    /// [`finish`](Self::finish), and every anchor's range by key.
    pub(crate) fn finish_with_anchors(mut self) -> (String, Vec<Marker>, Vec<(String, CharRange)>) {
        while let Some(o) = self.open.last() {
            let id = OpenId(o.id);
            self.close(id);
        }
        for mut m in std::mem::take(&mut self.points) {
            m.range = CharRange::empty(self.len);
            self.markers.push(m);
        }
        (self.text, self.markers, self.anchors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separators_are_deferred_and_collapsed() {
        let mut b = Builder::new();
        b.paragraph_break();
        let h = b.open(Marker::new(MarkerKind::Heading, CharRange::empty(0)).with_level(1));
        b.text("  Title  ");
        b.close(h);
        b.paragraph_break();
        let p = b.open(Marker::new(MarkerKind::Paragraph, CharRange::empty(0)));
        b.text("Some   text ");
        let bold = b.open(Marker::new(MarkerKind::Bold, CharRange::empty(0)));
        b.text(" bold ");
        b.close(bold);
        b.text(" end.\n");
        b.close(p);
        b.paragraph_break();
        let empty = b.open(Marker::new(MarkerKind::Paragraph, CharRange::empty(0)));
        b.close(empty);
        let (text, markers) = b.finish();
        assert_eq!(text, "Title\n\nSome text bold end.");
        assert_eq!(markers.len(), 3);
        assert_eq!(markers[0].range, CharRange::new(0, 5));
        let bold = markers.iter().find(|m| m.kind == MarkerKind::Bold).unwrap();
        assert_eq!(&text[bold.range.to_range()], "bold");
    }

    #[test]
    fn empty_cells_are_kept() {
        let mut b = Builder::new();
        let row = b.open(Marker::new(MarkerKind::TableRow, CharRange::empty(0)));
        let c = b.open_here(Marker::new(MarkerKind::TableCell, CharRange::empty(0)));
        b.text("a");
        b.close(c);
        b.literal(" | ");
        let c = b.open_here(Marker::new(MarkerKind::TableCell, CharRange::empty(0)));
        b.close(c);
        b.close(row);
        let (text, markers) = b.finish();
        assert_eq!(text, "a | ");
        assert_eq!(markers.len(), 3);
        assert_eq!(markers[1].range, CharRange::empty(4));
    }
}
