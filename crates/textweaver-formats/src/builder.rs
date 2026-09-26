//! Builds canonical text and markers from a stream of structure and text.
//!
//! Loaders call [`Builder`] as they walk their source: open and close
//! markers, add inline text (whitespace collapsed), verbatim text (code), and
//! request line or paragraph breaks. Breaks and spaces are held back until
//! the next text arrives, so the canonical text never has leading, trailing,
//! or doubled whitespace around blocks, and markers never include the
//! separators around them (ADR-0002 canonical shape).

use textweaver_core::CharRange;
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
}

impl Builder {
    pub(crate) fn new() -> Self {
        Builder::default()
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
    }

    fn push_raw(&mut self, s: &str) {
        self.text.push_str(s);
        self.len += s.chars().count();
        if let Some(last) = s.chars().last() {
            self.line_has_text = last != '\n';
        }
    }

    /// Inline text: whitespace runs collapse to one space, dropped at line
    /// starts and ends.
    pub(crate) fn text(&mut self, s: &str) {
        let mut word = String::new();
        for c in s.chars() {
            if c.is_whitespace() {
                if !word.is_empty() {
                    self.flush();
                    self.push_raw(&word);
                    word.clear();
                }
                self.request(Break::Space);
            } else {
                word.push(c);
            }
        }
        if !word.is_empty() {
            self.flush();
            self.push_raw(&word);
        }
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

    /// Verbatim text (code): spaces and line breaks kept, leading and
    /// trailing blank lines dropped, `\r\n` normalized.
    pub(crate) fn verbatim(&mut self, s: &str) {
        let s = s.replace("\r\n", "\n").replace('\r', "\n");
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
        });
        OpenId(id)
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
        let mut marker = o.marker;
        marker.range = CharRange::new(start, self.len.max(start));
        self.markers.push(marker);
    }

    /// The canonical text and its markers; open markers are closed.
    pub(crate) fn finish(mut self) -> (String, Vec<Marker>) {
        while let Some(o) = self.open.last() {
            let id = OpenId(o.id);
            self.close(id);
        }
        (self.text, self.markers)
    }
}

#[cfg(test)]
mod tests {
    use textweaver_core::MarkerKind;

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
