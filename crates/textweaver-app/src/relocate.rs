//! Positions that survive outside edits (Phase 2).
//!
//! Every saved state carries the document text's stamp (its length and a
//! hash, [`TextStamp`]), and every reading position, bookmark, note, and
//! highlight carries the text it was on: 40 characters from a position or
//! bookmark ([`Anchor`]), the collapsed text of a note or a highlight. When
//! a document opens and its stamp differs (it was edited in Obsidian,
//! changed by `git pull`, or rewritten by another program), or a state from
//! before stamps has an anchor that no longer matches, each one is found
//! again:
//!
//! 1. its text, exactly (white space runs compared as one space), near
//!    where it should be now (the old offset scaled to the new length);
//! 2. its text exactly anywhere, the nearest match winning (a paragraph
//!    moved elsewhere);
//! 3. the most similar text nearby (word bigrams; for small edits inside
//!    the anchor itself);
//! 4. else by percentage: the same share of the way through, marked "not
//!    found" so lists can say so.
//!
//! Positions still on their text are left alone. What moved and what was
//! not found is said once: "The file changed; 3 bookmarks were moved to
//! match, 1 could not be found and is marked."

use ropey::Rope;
use textweaver_core::{CharPos, CharRange};
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;
use textweaver_store::notes as store_notes;
use textweaver_store::{Anchor, DocState, TextStamp};
use textweaver_text::Document;

/// The stamp of a document's canonical text: its length and hash.
pub(crate) fn text_stamp(doc: &Document) -> TextStamp {
    TextStamp::of(doc.text().chunks())
}

/// The key in a note's or highlight's `extra` map that marks it as not
/// found after the file changed.
pub(crate) const NOT_FOUND_KEY: &str = "not_found";

/// How far around the expected position an exact match is looked for
/// first, and similar text at all.
const NEAR: usize = 4_000;

/// How similar (0 to 1, bigram Dice) text starting with the same word must
/// be to count as found.
const SIMILAR: f64 = 0.6;

/// How similar text starting with another word must be.
const VERY_SIMILAR: f64 = 0.85;

/// The shortest first line of an anchor that is looked for on its own.
const MIN_LINE: usize = 8;

/// White space collapsed to single spaces, trimmed.
fn collapsed(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A stretch of the document with its white space collapsed, and where
/// each byte of it came from.
struct Hay {
    text: String,
    /// The document char offset of each byte of `text`.
    from: Vec<usize>,
}

impl Hay {
    fn of(rope: &Rope, range: std::ops::Range<usize>) -> Hay {
        let mut text = String::new();
        let mut from = Vec::new();
        let mut space = false;
        for (i, c) in rope.slice(range.clone()).chars().enumerate() {
            let at = range.start + i;
            if c.is_whitespace() {
                space = true;
                continue;
            }
            if space && !text.is_empty() {
                text.push(' ');
                from.push(at.saturating_sub(1));
            }
            space = false;
            let before = text.len();
            text.push(c);
            from.extend(std::iter::repeat_n(at, text.len() - before));
        }
        Hay { text, from }
    }

    /// The document range of the hay's bytes `a..b`.
    fn doc_range(&self, a: usize, b: usize) -> (usize, usize) {
        let start = self.from[a];
        let end = self.from[b.saturating_sub(1).max(a)] + 1;
        (start, end)
    }
}

/// Finds text again in a changed document.
pub(crate) struct Finder<'a> {
    rope: &'a Rope,
    old_len: Option<usize>,
    whole: Option<Hay>,
}

impl<'a> Finder<'a> {
    /// A finder over `doc`; `old` is the stamp of the text the positions
    /// were saved on, when known.
    pub(crate) fn new(doc: &'a Document, old: Option<&TextStamp>) -> Self {
        Finder {
            rope: doc.text(),
            old_len: old.map(|s| s.chars),
            whole: None,
        }
    }

    fn len(&self) -> usize {
        self.rope.len_chars()
    }

    /// Where `old` should be now: scaled by the change in length.
    fn expected(&self, old: usize) -> usize {
        let len = self.len();
        match self.old_len {
            Some(l) if l > 0 => ((old as u128 * len as u128) / l as u128) as usize,
            _ => old,
        }
        .min(len)
    }

    /// The share of the way through that `old` was, applied to the new
    /// length (`pct` when the old length is unknown).
    fn by_percent(&self, old: usize, pct: Option<u8>) -> usize {
        match (self.old_len, pct) {
            (Some(l), _) if l > 0 => self.expected(old),
            (_, Some(p)) => self.len() * usize::from(p.min(100)) / 100,
            _ => old.min(self.len()),
        }
    }

    /// Finds `raw` (white space collapsed) near `expected`: exact near,
    /// exact anywhere (nearest); then its first line alone, exactly (an
    /// anchor that ran on into the next paragraph, which moved); then
    /// similar text near. The document range found.
    fn find(&mut self, raw: &str, expected: usize) -> Option<(usize, usize)> {
        let needle = collapsed(raw);
        if needle.is_empty() {
            return None;
        }
        let len = self.len();
        let near = expected.saturating_sub(NEAR)..(expected + NEAR + needle.len()).min(len);
        let hay = Hay::of(self.rope, near.clone());
        if let Some(r) = self.exact(&hay, near.clone(), &needle, expected) {
            return Some(r);
        }
        let first = collapsed(raw.split('\n').next().unwrap_or_default());
        if first != needle
            && first.chars().count() >= MIN_LINE
            && let Some(r) = self.exact(&hay, near, &first, expected)
        {
            return Some(r);
        }
        most_similar(&hay, &needle, expected)
    }

    /// An exact match nearest to `expected`: in `hay` (the stretch `near`),
    /// else anywhere in the document.
    fn exact(
        &mut self,
        hay: &Hay,
        near: std::ops::Range<usize>,
        needle: &str,
        expected: usize,
    ) -> Option<(usize, usize)> {
        if let Some(r) = nearest_exact(hay, needle, expected) {
            return Some(r);
        }
        let len = self.len();
        if near.start > 0 || near.end < len {
            let whole = self.whole.get_or_insert_with(|| Hay::of(self.rope, 0..len));
            return nearest_exact(whole, needle, expected);
        }
        None
    }
}

/// The exact match of `needle` in `hay` nearest to `expected`.
fn nearest_exact(hay: &Hay, needle: &str, expected: usize) -> Option<(usize, usize)> {
    hay.text
        .match_indices(needle)
        .map(|(b, m)| hay.doc_range(b, b + m.len()))
        .min_by_key(|(start, _)| start.abs_diff(expected))
}

/// Character bigrams of a string.
fn bigrams(s: &str) -> Vec<(char, char)> {
    let chars: Vec<char> = s.chars().flat_map(char::to_lowercase).collect();
    let mut v: Vec<(char, char)> = chars.windows(2).map(|w| (w[0], w[1])).collect();
    v.sort_unstable();
    v
}

/// Dice similarity of two sorted bigram lists.
fn dice(a: &[(char, char)], b: &[(char, char)]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let (mut i, mut j, mut both) = (0, 0, 0usize);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                both += 1;
                i += 1;
                j += 1;
            }
        }
    }
    2.0 * both as f64 / (a.len() + b.len()) as f64
}

/// The first word of a collapsed text, lowercased, without punctuation.
fn first_word(s: &str) -> String {
    s.split(' ')
        .next()
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// The stretch of `hay` starting at a word that is most like `needle`,
/// nearest to `expected` among equals: at least [`SIMILAR`] when it starts
/// with the same word, [`VERY_SIMILAR`] otherwise (a mark must not jump to
/// a neighbour that merely shares its later words).
fn most_similar(hay: &Hay, needle: &str, expected: usize) -> Option<(usize, usize)> {
    let want = bigrams(needle);
    let word = first_word(needle);
    let n = needle.chars().count();
    let mut best: Option<(f64, usize, (usize, usize))> = None;
    let starts = std::iter::once(0).chain(
        hay.text
            .char_indices()
            .filter(|&(_, c)| c == ' ')
            .map(|(i, _)| i + 1),
    );
    for b in starts.filter(|&b| b < hay.text.len()) {
        let piece: String = hay.text[b..].chars().take(n).collect();
        let score = dice(&want, &bigrams(&piece));
        let floor = if first_word(&piece) == word {
            SIMILAR
        } else {
            VERY_SIMILAR
        };
        if score < floor {
            continue;
        }
        let range = hay.doc_range(b, b + piece.len());
        let dist = range.0.abs_diff(expected);
        let better = match best {
            None => true,
            Some((s, d, _)) => score > s + 1e-9 || ((score - s).abs() <= 1e-9 && dist < d),
        };
        if better {
            best = Some((score, dist, range));
        }
    }
    best.map(|(_, _, r)| r)
}

/// What happened to one kind of mark.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Tally {
    /// Found again somewhere else.
    pub(crate) moved: usize,
    /// Not found: placed by percentage and marked.
    pub(crate) lost: usize,
}

/// What relocating a document's state did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Relocated {
    /// The reading position moved (`moved`) or was not found (`lost`).
    pub(crate) position: Tally,
    pub(crate) bookmarks: Tally,
    pub(crate) notes: Tally,
    pub(crate) highlights: Tally,
}

impl Relocated {
    fn is_empty(&self) -> bool {
        *self == Relocated::default()
    }

    /// The sentence said once on open, or `None` when nothing moved.
    pub(crate) fn message(&self, c: &Catalog) -> Option<String> {
        if self.is_empty() {
            return None;
        }
        let kinds = [
            (self.bookmarks, "relocate-bookmarks"),
            (self.notes, "relocate-notes"),
            (self.highlights, "relocate-highlights"),
        ];
        let list = |pick: fn(&Tally) -> usize, position: bool| -> (Vec<String>, usize) {
            let mut parts = Vec::new();
            let mut total = 0;
            if position {
                parts.push(c.tr("relocate-reading-position"));
                total += 1;
            }
            for (t, id) in kinds {
                let n = pick(&t);
                if n > 0 {
                    parts.push(c.fmt(id, &args!["n" => n]));
                    total += n;
                }
            }
            (parts, total)
        };
        let (moved, moved_n) = list(|t| t.moved, self.position.moved > 0);
        let (lost, lost_n) = list(|t| t.lost, self.position.lost > 0);
        let mut clauses = Vec::new();
        if !moved.is_empty() {
            clauses.push(c.fmt(
                "relocate-moved",
                &args!["items" => join(c, &moved), "n" => moved_n],
            ));
        }
        if !lost.is_empty() {
            clauses.push(c.fmt(
                "relocate-lost",
                &args!["items" => join(c, &lost), "n" => lost_n],
            ));
        }
        Some(c.fmt("relocate-changed", &args!["clauses" => clauses.join(", ")]))
    }
}

/// "a", "a and b", "a, b, and c".
fn join(c: &Catalog, parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [a] => a.clone(),
        [a, b] => c.fmt("relocate-join-two", &args!["a" => a, "b" => b]),
        [rest @ .., last] => c.fmt(
            "relocate-join-more",
            &args!["rest" => rest.join(", "), "last" => last],
        ),
    }
}

/// The text at `pos`, as an anchor.
fn anchor_here(rope: &Rope, pos: usize) -> Anchor {
    let start = pos.min(rope.len_chars());
    let end = (start + Anchor::CONTEXT_CHARS).min(rope.len_chars());
    Anchor::from_text_at(rope.slice(start..end).chars())
}

/// True when the state's positions may be off: the text's stamp differs,
/// or (a state from before stamps) an anchor no longer matches.
pub(crate) fn needs_relocation(state: &DocState, now: &TextStamp, doc: &Document) -> bool {
    match &state.text {
        Some(old) => old != now,
        None => {
            let rope = doc.text();
            let off = |a: &Option<Anchor>, pos: CharPos| {
                a.as_ref().is_some_and(|a| anchor_here(rope, pos.0) != *a)
            };
            off(&state.anchor, state.position)
                || state.bookmarks.iter().any(|b| off(&b.anchor, b.pos))
        }
    }
}

/// Finds a position again from its anchor. `None` when it is still on its
/// text (or has no anchor); `Some((pos, found))` otherwise.
fn relocate_pos(
    finder: &mut Finder<'_>,
    anchor: Option<&Anchor>,
    pos: CharPos,
    pct: Option<u8>,
) -> Option<(CharPos, bool)> {
    let anchor = anchor?;
    let rope = finder.rope;
    if pos.0 <= rope.len_chars() && anchor_here(rope, pos.0) == *anchor {
        return None;
    }
    let expected = finder.expected(pos.0);
    match finder.find(&anchor.context, expected) {
        Some((start, _)) if start == pos.0 => None,
        Some((start, _)) => Some((CharPos(start), true)),
        None => Some((CharPos(finder.by_percent(pos.0, pct)), false)),
    }
}

/// Finds a note's or highlight's range again from its collapsed text.
/// `None` when it is still on its text.
fn relocate_range(
    finder: &mut Finder<'_>,
    text: &str,
    range: CharRange,
    max: usize,
) -> Option<(CharRange, bool)> {
    let rope = finder.rope;
    let len = rope.len_chars();
    if text.trim().is_empty() {
        return None;
    }
    if range.end.0 <= len
        && store_notes::collapse(&rope.slice(range.start.0..range.end.0).to_string(), max) == text
    {
        return None;
    }
    let old_len = range.end.0.saturating_sub(range.start.0);
    // A text cut at the limit covered more than it says.
    let cut = text.chars().count() >= max.saturating_sub(1);
    let expected = finder.expected(range.start.0);
    match finder.find(text, expected) {
        Some((start, end)) => {
            let end = if cut {
                (start + old_len).min(len).max(end)
            } else {
                end
            };
            let r = CharRange::new(start, end);
            (r != range).then_some((r, true))
        }
        None => {
            let start = finder.by_percent(range.start.0, None).min(len);
            let r = CharRange::new(start, (start + old_len).min(len));
            Some((r, false))
        }
    }
}

/// Finds every position, bookmark, note, and highlight of `state` again in
/// `doc`, in place, and says what happened. Items not found are placed by
/// percentage and marked (`Bookmark::not_found`, and [`NOT_FOUND_KEY`] in
/// a note's or highlight's extra fields); items found again lose the mark.
pub(crate) fn relocate(state: &mut DocState, doc: &Document) -> Relocated {
    let mut finder = Finder::new(doc, state.text.as_ref());
    let mut out = Relocated::default();
    if let Some((pos, found)) = relocate_pos(
        &mut finder,
        state.anchor.as_ref(),
        state.position,
        Some(state.pct),
    ) {
        state.position = pos;
        state.anchor = Some(anchor_here(finder.rope, pos.0));
        if found {
            out.position.moved = 1;
        } else {
            out.position.lost = 1;
        }
    }
    let len = finder.len();
    for b in &mut state.bookmarks {
        if let Some((pos, found)) = relocate_pos(&mut finder, b.anchor.as_ref(), b.pos, Some(b.pct))
        {
            b.pos = pos;
            b.pct = textweaver_store::percent(pos, len);
            b.not_found = !found;
            if found {
                b.anchor = Some(anchor_here(finder.rope, pos.0));
                out.bookmarks.moved += 1;
            } else {
                out.bookmarks.lost += 1;
            }
        }
    }
    state.bookmarks.sort_by_key(|b| b.pos);
    for n in &mut state.notes {
        let anchor = n.anchor.clone();
        if let Some((range, found)) =
            relocate_range(&mut finder, &anchor, n.range, store_notes::ANCHOR_MAX_CHARS)
        {
            n.range = range;
            mark(&mut n.extra, !found);
            if found {
                out.notes.moved += 1;
            } else {
                out.notes.lost += 1;
            }
        }
    }
    state.notes.sort_by_key(|n| (n.range.start, n.range.end));
    for h in &mut state.highlights {
        let text = h.text.clone();
        if let Some((range, found)) = relocate_range(
            &mut finder,
            &text,
            h.range,
            store_notes::HIGHLIGHT_TEXT_MAX_CHARS,
        ) {
            h.range = range;
            mark(&mut h.extra, !found);
            if found {
                out.highlights.moved += 1;
            } else {
                out.highlights.lost += 1;
            }
        }
    }
    state
        .highlights
        .sort_by_key(|h| (h.range.start, h.range.end));
    // History has no anchors: it follows the change in length.
    for h in &mut state.history {
        *h = CharPos(finder.expected(h.0));
    }
    state.history.dedup();
    state.text = Some(text_stamp(doc));
    out
}

fn mark(extra: &mut serde_json::Map<String, serde_json::Value>, lost: bool) {
    if lost {
        extra.insert(NOT_FOUND_KEY.to_owned(), serde_json::Value::Bool(true));
    } else {
        extra.remove(NOT_FOUND_KEY);
    }
}

/// True when a note or highlight is marked as not found.
pub(crate) fn is_marked(extra: &serde_json::Map<String, serde_json::Value>) -> bool {
    extra
        .get(NOT_FOUND_KEY)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_read_well() {
        let r = Relocated {
            bookmarks: Tally { moved: 3, lost: 1 },
            ..Relocated::default()
        };
        assert_eq!(
            r.message(&Catalog::english()).unwrap(),
            "The file changed; 3 bookmarks were moved to match, 1 bookmark could not be found and is marked."
        );
        let r = Relocated {
            position: Tally { moved: 1, lost: 0 },
            notes: Tally { moved: 2, lost: 0 },
            highlights: Tally { moved: 0, lost: 2 },
            ..Relocated::default()
        };
        assert_eq!(
            r.message(&Catalog::english()).unwrap(),
            "The file changed; your reading position and 2 notes were moved to match, 2 highlights could not be found and are marked."
        );
        assert_eq!(Relocated::default().message(&Catalog::english()), None);
    }

    #[test]
    fn similar_text_is_found_when_the_anchor_itself_was_edited() {
        let doc = Document::from_plain_text(
            "An opening line.\nThe quick brown fox jumps over the lazy dog today.\nThe end.\n",
        );
        let mut f = Finder::new(&doc, None);
        // One word changed inside the anchor.
        let (start, _) = f
            .find("The quick brown cat jumps over the lazy dog", 10)
            .unwrap();
        assert_eq!(start, 17);
        assert_eq!(f.find("completely different words entirely here", 10), None);
    }
}
