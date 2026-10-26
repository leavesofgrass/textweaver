//! Live documents (beta 5 spike, task N0): a document whose text a live
//! source changes from outside edit mode, while the reader's place stays.
//!
//! A live view renders its entries as Markdown; [`App::live_apply`] loads
//! that Markdown and changes the open document to it by the smallest edits
//! (lines both texts have are kept). Every position the app keeps is
//! mapped across those edits, as
//! `with_document_edit` (crate::changes) does for tracked changes, but
//! nothing is cleared: the cursor, the selection, the spoken range, find
//! matches, bookmarks, notes, highlights and history all stay on the same
//! text. An append at the end moves none of them.
//!
//! [`NetLog`] is the smallest log view (a list, or a table with real
//! headers), enough to drive the spike's tests.

use textweaver_a11y::{Importance, Priority};
use textweaver_core::{Bias, CharPos, CharRange, Edit, EditOutcome, MarkerKind};
use textweaver_formats::{LoadOptions, Loader, MarkdownLoader, Source};
use textweaver_store::DocKey;
use textweaver_text::Document;

use crate::app::App;
use crate::command::Effect;
use crate::playback::Playback;

/// Marks a [`Session`](crate::Session) as a live document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveTag {
    /// The source's id, as its document key carries it.
    pub source: String,
}

/// Loads generated Markdown as a document. Markdown never fails to load;
/// an empty document stands in should the loader refuse it.
///
/// The text always ends with a line break, so the last entry's line is
/// the same before and after one is added below it: a frontend that keeps
/// a node per line keeps the last entry's node too.
pub fn load_markdown(markdown: &str) -> Document {
    let source = Source::Bytes {
        data: markdown.as_bytes().to_vec(),
        hint: "md".into(),
    };
    let mut doc = MarkdownLoader
        .load(&source, &LoadOptions::default())
        .unwrap_or_else(|_| Document::from_plain_text(""));
    let len = doc.len_chars();
    if len > 0 && doc.text().char(len - 1) != '\n' {
        let _ = doc.apply(&Edit::insert(CharPos(len), "\n"));
    }
    doc
}

/// The edits that turn `old`'s text into `new`'s, changing as little as
/// possible: lines both texts have are kept (a longest common run of
/// lines), and within a changed run the chars it starts and ends with are
/// kept. The edits are in reverse document order, each in the coordinates
/// of the text the ones before it left, so applying them one after
/// another, and mapping positions across each in turn, is exact. Empty
/// when the texts are the same.
pub fn minimal_edits(old: &Document, new: &Document) -> Vec<Edit> {
    let a: Vec<String> = old.text().lines().map(|l| l.to_string()).collect();
    let b: Vec<String> = new.text().lines().map(|l| l.to_string()).collect();
    // The char offset of each old line's start, and of the end.
    let mut starts = Vec::with_capacity(a.len() + 1);
    let mut n = 0;
    starts.push(0);
    for l in &a {
        n += l.chars().count();
        starts.push(n);
    }
    changed_runs(&a, &b)
        .into_iter()
        .rev()
        .filter_map(|(ra, rb)| {
            let old_text: String = a[ra.clone()].concat();
            let new_text: String = b[rb].concat();
            trimmed_edit(starts[ra.start], &old_text, &new_text)
        })
        .collect()
}

/// A run of lines that differ: its old lines and its new lines.
type Run = (std::ops::Range<usize>, std::ops::Range<usize>);

/// The runs of lines that differ, in order, from a longest common
/// subsequence of lines. Past a size cap, the middle between the common
/// first and last lines is one run.
fn changed_runs(a: &[String], b: &[String]) -> Vec<Run> {
    /// Cells of the comparison table (4 bytes each).
    const CAP: usize = 4_000_000;
    let mut p = 0;
    while p < a.len() && p < b.len() && a[p] == b[p] {
        p += 1;
    }
    let mut q = 0;
    while q < a.len() - p && q < b.len() - p && a[a.len() - 1 - q] == b[b.len() - 1 - q] {
        q += 1;
    }
    let (ma, mb) = (&a[p..a.len() - q], &b[p..b.len() - q]);
    let (n, m) = (ma.len(), mb.len());
    if n == 0 && m == 0 {
        return Vec::new();
    }
    if n == 0 || m == 0 || (n + 1) * (m + 1) > CAP {
        return vec![(p..p + n, p..p + m)];
    }
    // lcs[i * w + j]: the longest common run of ma[i..] and mb[j..].
    let w = m + 1;
    let mut lcs = vec![0u32; (n + 1) * w];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i * w + j] = if ma[i] == mb[j] {
                lcs[(i + 1) * w + j + 1] + 1
            } else {
                lcs[(i + 1) * w + j].max(lcs[i * w + j + 1])
            };
        }
    }
    let mut runs = Vec::new();
    let (mut i, mut j, mut ri, mut rj) = (0, 0, 0, 0);
    while i < n || j < m {
        if i < n && j < m && ma[i] == mb[j] {
            if ri < i || rj < j {
                runs.push((p + ri..p + i, p + rj..p + j));
            }
            i += 1;
            j += 1;
            (ri, rj) = (i, j);
        } else if j < m && (i == n || lcs[i * w + j + 1] >= lcs[(i + 1) * w + j]) {
            j += 1;
        } else {
            i += 1;
        }
    }
    if ri < n || rj < m {
        runs.push((p + ri..p + n, p + rj..p + m));
    }
    runs
}

/// An edit replacing `old` (which starts at char `at`) with `new`, less
/// the chars both start and end with; `None` when they are the same.
fn trimmed_edit(at: usize, old: &str, new: &str) -> Option<Edit> {
    let a: Vec<char> = old.chars().collect();
    let b: Vec<char> = new.chars().collect();
    let pre = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if pre == a.len() && pre == b.len() {
        return None;
    }
    let room = a.len().min(b.len()) - pre;
    let suf = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take(room)
        .take_while(|(x, y)| x == y)
        .count();
    let inserted: String = b[pre..b.len() - suf].iter().collect();
    Some(Edit::replace(
        CharRange::new(at + pre, at + a.len() - suf),
        inserted,
    ))
}

impl App {
    /// Opens `markdown` as the live document of source `source`, titled
    /// `title`. Its reading position is never saved.
    pub fn open_live(&mut self, source: &str, title: &str, markdown: &str) -> Vec<Effect> {
        let doc = load_markdown(markdown);
        let key = DocKey(format!("live-{source}"));
        let effects = self.open_document(doc, key, title.to_owned());
        if let Some(s) = self.session.as_mut() {
            s.live = Some(LiveTag {
                source: source.to_owned(),
            });
        }
        effects
    }

    /// Changes the open live document to `markdown` in place, keeping the
    /// reader's place: every position is mapped across the smallest edits
    /// ([`minimal_edits`]), and `place`, when given, puts the cursor there
    /// instead (a view switched from a list to a table keeps the reader on
    /// the same entry). `announce` is said politely, as content that is
    /// never silenced. Returns the edits' outcomes in the order applied,
    /// or `None` when nothing changed or the open document is not live.
    pub fn live_apply(
        &mut self,
        markdown: &str,
        place: Option<CharPos>,
        announce: Option<&str>,
    ) -> Option<Vec<EditOutcome>> {
        let new = load_markdown(markdown);
        let outcomes = self.live_replace(new, place);
        if let Some(text) = announce {
            self.announce_as(text, Priority::Polite, Importance::Answer);
        }
        outcomes
    }

    /// [`live_apply`](Self::live_apply) with the document already loaded.
    fn live_replace(
        &mut self,
        mut new: Document,
        place: Option<CharPos>,
    ) -> Option<Vec<EditOutcome>> {
        let s = self.session.as_mut().filter(|s| s.live.is_some())?;
        let outcomes: Vec<EditOutcome> = minimal_edits(&s.doc, &new)
            .iter()
            .map(Edit::outcome)
            .collect();
        if outcomes.is_empty() {
            return None;
        }
        let len = new.len_chars();
        let fit = |p: CharPos| p.clamp_to(len);
        let all = |p: CharPos| outcomes.iter().fold(p, |p, o| keep_column(o, p));
        let all_range = |r: CharRange| outcomes.iter().fold(r, |r, o| o.map_range(r)).clamp_to(len);
        // Line positions, through the char at their line's start.
        let rope = s.doc.text();
        let line_pos = |line: usize| CharPos(rope.line_to_char(line.min(rope.len_lines() - 1)));
        let speech_line = s.speech_cursor_line.map(line_pos);
        let top = line_pos(self.view.top_line);
        crate::notes::shift_marks(&mut s.notes, &mut s.highlights, &mut s.bookmarks, &outcomes);
        for o in &outcomes {
            s.history.shift(o);
        }
        s.cursor = fit(place.unwrap_or_else(|| all(s.cursor)));
        s.selection = s.selection.map(all_range);
        s.selection_anchor = s.selection_anchor.map(|p| fit(all(p)));
        s.spoken = s.spoken.map(all_range);
        s.spoken_sentence = s.spoken_sentence.map(all_range);
        if let Some(f) = s.find.as_mut() {
            for h in &mut f.hits {
                *h = all_range(*h);
            }
        }
        // The new document keeps the old one's details (title, path).
        new.meta = s.doc.meta.clone();
        s.doc = new;
        let line_of = |p: CharPos| s.doc.line_of(fit(all(p)));
        s.speech_cursor_line = speech_line.map(line_of);
        let top_line = line_of(top);
        s.revision = crate::app::next_revision();
        self.view.top_line = top_line;
        if let Playback::Paused { resume_at: Some(p) } = self.playback {
            self.playback = Playback::Paused {
                resume_at: Some(fit(all(p))),
            };
        }
        self.pause_origin = self.pause_origin.map(|p| fit(all(p)));
        self.section_end = self
            .section_end
            .map(|p| fit(outcomes.iter().fold(p, |p, o| o.map_pos(p, Bias::After))));
        Some(outcomes)
    }
}

/// Maps `pos` across `o`; a position inside the replaced text keeps its
/// offset when the new text is long enough (the same column of a
/// corrected entry), else goes to the new text's end.
fn keep_column(o: &EditOutcome, pos: CharPos) -> CharPos {
    let r = o.removed;
    if r.start < pos && pos < r.end {
        pos.min(o.inserted.end)
    } else {
        o.map_pos(pos, Bias::Before)
    }
}

/// One entry of a [`NetLog`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    /// The callsign (the entry's id when the source sends none).
    pub call: String,
    /// The name.
    pub name: String,
    /// The notes.
    pub notes: String,
}

impl LogEntry {
    /// An entry.
    pub fn new(call: &str, name: &str, notes: &str) -> Self {
        LogEntry {
            call: call.into(),
            name: name.into(),
            notes: notes.into(),
        }
    }
}

/// How a [`NetLog`] is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogView {
    /// One entry per line, meaning first.
    List,
    /// A table with real headers.
    Table,
}

/// The smallest log view: entries rendered as Markdown.
#[derive(Clone, Debug, Default)]
pub struct NetLog {
    /// The entries, in order of arrival.
    pub entries: Vec<LogEntry>,
}

impl NetLog {
    /// Adds an entry at the end, or replaces the one with the same
    /// callsign in place. Returns its index.
    pub fn upsert(&mut self, e: LogEntry) -> usize {
        match self.entries.iter().position(|x| x.call == e.call) {
            Some(i) => {
                self.entries[i] = e;
                i
            }
            None => {
                self.entries.push(e);
                self.entries.len() - 1
            }
        }
    }

    /// The log as Markdown in `view`.
    pub fn render(&self, view: LogView) -> String {
        let mut md = String::from("# Net log\n\n");
        match view {
            LogView::List => {
                for e in &self.entries {
                    md.push_str("- ");
                    md.push_str(&Self::line(e));
                    md.push('\n');
                }
            }
            LogView::Table => {
                md.push_str("| Callsign | Name | Notes |\n|---|---|---|\n");
                for e in &self.entries {
                    md.push_str(&format!("| {} | {} | {} |\n", e.call, e.name, e.notes));
                }
            }
        }
        md
    }

    /// An entry as one line, meaning first: "N0CALL, Ada. Mobile."
    pub fn line(e: &LogEntry) -> String {
        if e.notes.is_empty() {
            format!("{}, {}.", e.call, e.name)
        } else {
            format!("{}, {}. {}", e.call, e.name, e.notes)
        }
    }

    /// Where entry `i` starts in `doc`, rendered in `view`.
    pub fn entry_start(doc: &Document, view: LogView, i: usize) -> Option<CharPos> {
        Self::entry_ranges(doc, view).get(i).map(|r| r.start)
    }

    /// Which entry `pos` is in, in `doc` rendered in `view`.
    pub fn entry_at(doc: &Document, view: LogView, pos: CharPos) -> Option<usize> {
        Self::entry_ranges(doc, view)
            .iter()
            .position(|r| r.start <= pos && pos < r.end.max(CharPos(r.start.0 + 1)))
    }

    fn entry_ranges(doc: &Document, view: LogView) -> Vec<CharRange> {
        let (kind, skip) = match view {
            LogView::List => (MarkerKind::ListItem, 0),
            LogView::Table => (MarkerKind::TableRow, 1),
        };
        doc.markers()
            .iter()
            .filter(|m| m.kind == kind)
            .skip(skip)
            .map(|m| m.range)
            .collect()
    }

    /// The cursor's place in `old` (shown in `from`) moved to the same
    /// entry and offset in `new` (shown in `to`).
    pub fn same_entry(
        old: &Document,
        from: LogView,
        new: &Document,
        to: LogView,
        pos: CharPos,
    ) -> Option<CharPos> {
        let i = Self::entry_at(old, from, pos)?;
        Self::entry_start(new, to, i)
    }
}
