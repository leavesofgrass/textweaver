//! Live documents: a document whose text a live source changes from
//! outside edit mode, while the reader's place stays.
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
//! The parts:
//!
//! - [`source`]: [`LiveEvent`], the [`LiveSource`] trait a source
//!   implements (its poll never blocks), and [`ScriptedSource`], a source
//!   driven by hand for tests.
//! - [`log`]: the log view's entries ([`LiveLog`]): one JSON object adds
//!   or corrects an entry, a list replaces the log; rendered as a list or
//!   a table.
//! - [`values`]: the values view's fields ([`LiveValues`]): one line per
//!   field, updated in place, for sensor-style sources.
//! - [`order`]: the order a message's keys came in ([`KeyOrder`]), which
//!   both views keep.
//! - [`hub`]: the app's side ([`App::open_live_source`], `App::live_tick`
//!   and the functions commands call: the view switch, jump to newest,
//!   pause, the unread count, Save).
//! - [`json_log`]: the JSON log written while the log runs, and read back
//!   ([`App::open_live_log`]).
//! - [`announce`]: new entries said through a throttle that never
//!   interrupts reading ([`EntryThrottle`]).
//! - [`wire`]: the commands: open a configured source, set one up, close
//!   it, and the rest ([`App::open_configured_live_source`]).
//! - `form`: the one form that sets up or edits a source, and the live
//!   source files (`.twlive`) that fill it ([`App::live_setup`]).
//! - [`document`]: the document view, for live document messages: the
//!   shared document replaced in place, keeping the reader's place.
//! - `checkin`: the Publish a check-in form.
//! - `broker` (with the `mqtt` feature): textweaver's own broker inside
//!   the app, sharing a document live, and publishing check-ins.
//! - `mqtt` (with the `mqtt` feature): the MQTT client adapted to
//!   [`LiveSource`].
//!
//! A live session ([`Session::live`](crate::Session::live)) is read only:
//! edit mode is refused in words, no reading position is saved, it gets
//! no library or recent entry and is never synced, and Save writes the
//! JSON log.

pub mod announce;
#[cfg(feature = "mqtt")]
pub mod broker;
mod checkin;
pub mod document;
pub mod hub;
pub mod json_log;
pub mod log;
#[cfg(feature = "mqtt")]
pub mod mqtt;
pub mod order;
pub mod qr;
pub mod source;
pub mod values;
pub mod wire;

pub(crate) mod form;

pub use announce::{BURST, EntryThrottle, Release};
pub use document::{DOCUMENT_TYPE, SharedDoc};
pub(crate) use hub::LiveHub;
pub use hub::{LOG_WRITE_INTERVAL, LiveConnection, LiveOptions, PAUSE_QUEUE_CAP};
pub use log::{
    Field, Fields, LiveLog, LogEntry, LogLabels, LogOptions, LogView, Message, Shape, entry_object,
};
pub use order::{KeyOrder, parse_ordered};
pub use source::{LiveEvent, LivePayload, LiveSource, ScriptHandle, ScriptedSource};
pub use values::{FieldChange, LiveValues, ValueField, ValueLabels, ValuesOptions};
pub(crate) use wire::{LiveList, LiveWire};

use textweaver_a11y::{Importance, Priority};
use textweaver_core::{Bias, CharPos, CharRange, Edit, EditOutcome};
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
    /// The source is running; false once stopped, and for a saved log
    /// reopened.
    pub receiving: bool,
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
    if n == 0 || m == 0 {
        return vec![(p..p + n, p..p + m)];
    }
    if (n + 1) * (m + 1) > CAP {
        // A long log resent with a few lines changed: few differences,
        // found without the table.
        return few_changes(ma, mb, p).unwrap_or_else(|| vec![(p..p + n, p..p + m)]);
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

/// The runs of lines that differ between `a` and `b` (offset by `p`), from
/// a shortest edit script (Myers's greedy algorithm), when it takes at most
/// [`MAX_D`] lines added or removed: time and memory grow with the lines
/// that differ, not with the product of the lengths. `None` past that.
fn few_changes(a: &[String], b: &[String], p: usize) -> Option<Vec<Run>> {
    /// The most lines added or removed searched for.
    const MAX_D: usize = 1_000;
    let (n, m) = (a.len() as isize, b.len() as isize);
    let off = MAX_D as isize + 1;
    let mut v = vec![0isize; 2 * MAX_D + 3];
    // The furthest x on each diagonal before each step, for the way back.
    let mut trace: Vec<Vec<isize>> = Vec::new();
    let at = |k: isize| (k + off) as usize;
    let back = |v: &[isize], k: isize, d: isize| {
        if k == -d || (k != d && v[at(k - 1)] < v[at(k + 1)]) {
            k + 1
        } else {
            k - 1
        }
    };
    let mut end = None;
    'search: for d in 0..=MAX_D as isize {
        trace.push(v.clone());
        for k in (-d..=d).step_by(2) {
            let mut x = if back(&v, k, d) == k + 1 {
                v[at(k + 1)]
            } else {
                v[at(k - 1)] + 1
            };
            let mut y = x - k;
            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[at(k)] = x;
            if x >= n && y >= m {
                end = Some(d);
                break 'search;
            }
        }
    }
    // Back from the end, the lines both have, last first.
    let (mut x, mut y) = (n, m);
    let mut same = Vec::new();
    for d in (1..=end?).rev() {
        let v = &trace[d as usize];
        let k = x - y;
        let pk = back(v, k, d);
        let px = v[at(pk)];
        let py = px - pk;
        let (sx, sy) = if pk == k + 1 {
            (px, py + 1)
        } else {
            (px + 1, py)
        };
        while x > sx && y > sy {
            x -= 1;
            y -= 1;
            same.push((x as usize, y as usize));
        }
        (x, y) = (px, py);
    }
    same.extend((0..x as usize).rev().map(|i| (i, i)));
    let mut runs = Vec::new();
    let (mut ri, mut rj) = (0, 0);
    for (i, j) in same.into_iter().rev() {
        if ri < i || rj < j {
            runs.push((p + ri..p + i, p + rj..p + j));
        }
        (ri, rj) = (i + 1, j + 1);
    }
    if ri < a.len() || rj < b.len() {
        runs.push((p + ri..p + a.len(), p + rj..p + b.len()));
    }
    Some(runs)
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
    /// The text is the caller's to change ([`App::live_apply`]); a log
    /// fed by a source opens with [`App::open_live_source`] instead.
    pub fn open_live(&mut self, source: &str, title: &str, markdown: &str) -> Vec<Effect> {
        self.live_leave();
        self.open_live_keyed(DocKey(format!("live-{source}")), source, title, markdown)
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
    pub(crate) fn live_replace(
        &mut self,
        new: Document,
        place: Option<CharPos>,
    ) -> Option<Vec<EditOutcome>> {
        let s = self.session.as_ref().filter(|s| s.live.is_some())?;
        let outcomes: Vec<EditOutcome> = minimal_edits(&s.doc, &new)
            .iter()
            .map(Edit::outcome)
            .collect();
        self.live_replace_by(new, outcomes, place)
    }

    /// [`live_replace`](Self::live_replace) when the edits from the open
    /// document's text to `new`'s are known (an entry added or corrected,
    /// crate::live::hub): `outcomes` in the order applied, each in the
    /// coordinates the ones before it left.
    pub(crate) fn live_replace_by(
        &mut self,
        mut new: Document,
        outcomes: Vec<EditOutcome>,
        place: Option<CharPos>,
    ) -> Option<Vec<EditOutcome>> {
        let s = self.session.as_mut().filter(|s| s.live.is_some())?;
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

/// `old` with the chars `at` replaced by the chars `from` of `piece`, a
/// part of the same Markdown parsed on its own, and the edit that does it.
/// Markers inside `at` go and `piece`'s markers inside `from` come in
/// their place; a list or a table holding `around` grows (or shrinks)
/// with it; markers after `at` move. `None` when a marker other than a
/// list or table reaches into `at` from outside, or a range is out of
/// bounds: the caller loads the whole view instead.
///
/// For a corrected entry `at` and `around` are its range in `old`; for
/// entries added at the end, `at` is the empty range at the end of the
/// last entry, which is `around`.
pub(crate) fn splice(
    old: &Document,
    at: CharRange,
    around: CharRange,
    piece: &Document,
    from: CharRange,
) -> Option<(Document, Edit)> {
    use textweaver_core::MarkerKind;
    if at.end.0 > old.len_chars() || from.end.0 > piece.len_chars() || at.start > at.end {
        return None;
    }
    let inserted = piece.text().slice(from.start.0..from.end.0).to_string();
    let delta = from.len() as isize - at.len() as isize;
    let moved = |p: CharPos| CharPos(p.0.checked_add_signed(delta).unwrap_or(0));
    let container = |k: MarkerKind| matches!(k, MarkerKind::List | MarkerKind::Table);
    let mut markers = Vec::with_capacity(old.markers().len() + piece.markers().len());
    for m in old.markers() {
        let r = m.range;
        let mut m = m.clone();
        if container(m.kind) && r.contains_range(around) {
            m.range.end = moved(r.end);
        } else if !container(m.kind) && !at.is_empty() && at.contains_range(r) {
            continue;
        } else if r.end <= at.start {
            // Before the edit: as it was.
        } else if r.start >= at.end {
            m.range = CharRange::new(moved(r.start), moved(r.end));
        } else {
            return None;
        }
        markers.push(m);
    }
    for m in piece.markers() {
        // Inserting after `around`, an empty marker where `from` starts
        // ends the entry before it, which `old` already has.
        let before = at.is_empty() && m.range.is_empty() && m.range.start == from.start;
        if container(m.kind) || !from.contains_range(m.range) || before {
            continue;
        }
        let mut m = m.clone();
        let shift = |p: CharPos| CharPos(p.0 - from.start.0 + at.start.0);
        m.range = CharRange::new(shift(m.range.start), shift(m.range.end));
        markers.push(m);
    }
    let mut text = old.text().clone();
    text.remove(at.start.0..at.end.0);
    text.insert(at.start.0, &inserted);
    let edit = Edit::replace(at, inserted);
    Some((Document::new(old.meta.clone(), text, markers), edit))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The runs turn `a` into `b`.
    fn rebuilds(a: &[String], b: &[String], runs: &[Run]) {
        let mut out = Vec::new();
        let mut i = 0;
        for (ra, rb) in runs {
            out.extend_from_slice(&a[i..ra.start]);
            out.extend_from_slice(&b[rb.clone()]);
            i = ra.end;
        }
        out.extend_from_slice(&a[i..]);
        assert_eq!(out, b);
    }

    #[test]
    fn few_changes_finds_what_the_table_finds() {
        let mut seed = 7u64;
        let mut below = |n: u64| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) % n
        };
        for _ in 0..300 {
            let a: Vec<String> = (0..below(12)).map(|_| below(4).to_string()).collect();
            let b: Vec<String> = (0..below(12)).map(|_| below(4).to_string()).collect();
            let table = changed_runs(&a, &b);
            let few = few_changes(&a, &b, 0).unwrap();
            // Both keep the same number of lines: a longest common run.
            let common =
                |runs: &[Run]| a.len() - runs.iter().map(|(ra, _)| ra.len()).sum::<usize>();
            assert_eq!(common(&few), common(&table), "{a:?} {b:?}");
            rebuilds(&a, &b, &few);
            rebuilds(&a, &b, &table);
        }
    }
}
