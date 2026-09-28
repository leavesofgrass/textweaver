//! Find, bookmarks, and selection.
//!
//! Find is case-insensitive plain text (Star's behavior); a pattern written
//! as `/regex/` is a regular expression. Matches wrap at the document ends
//! (Star wraps search in both UIs), and the wrap is announced. Bookmarks go
//! to the first word at or after their position (Star's GUI rule; the TUI's
//! "closest word" quirk is not kept) and wrap the same way.

use textweaver_a11y::Verbosity;
use textweaver_core::{CharPos, CharRange, Direction, Unit};
use textweaver_lexicon::args;
use textweaver_speech::Earcon;
use textweaver_store::Bookmark;
use textweaver_text::{NavOptions, SearchQuery, navigate};

use crate::app::{App, FindState, ListKind};
use crate::command::{Effect, PromptPurpose};
use crate::find_scan;
use crate::nav::ReadAfter;
use crate::text_util::{self, preview};

impl App {
    /// Runs a new search from the cursor.
    pub(crate) fn run_find(&mut self, pattern: &str) {
        if pattern.is_empty() {
            let msg = self.msg("common-cancelled");
            self.note(&msg);
            return;
        }
        let (text, regex) = match pattern.strip_prefix('/').and_then(|p| p.strip_suffix('/')) {
            Some(re) if !re.is_empty() => (re.to_owned(), true),
            _ => (pattern.to_owned(), false),
        };
        let query = SearchQuery {
            pattern: text,
            regex,
            case_sensitive: false,
            whole_word: false,
            wrap: true,
            direction: Direction::Forward,
        };
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let cursor = s.cursor;
        let found = match find_scan::collect_around(
            s.doc.text(),
            &query,
            cursor,
            find_scan::MAX_STORED_HITS,
        ) {
            Ok(a) => a,
            Err(e) => {
                let msg = self.msg_args("marks-cannot-search", &args!["error" => e.to_string()]);
                self.error(&msg);
                return;
            }
        };
        let first = found.hits.iter().position(|h| h.start >= cursor);
        let count = found.total;
        s.find = Some(FindState {
            query,
            hits: found.hits,
            current: None,
            total: found.total,
            first_index: found.first_index,
        });
        if count == 0 {
            self.speech.earcon(Earcon::Error);
            let msg = self.msg_args("marks-no-matches", &args!["pattern" => pattern]);
            self.tell(&msg);
            return;
        }
        match first {
            Some(i) => self.go_to_hit(i, None),
            None => {
                self.refill_hits(CharPos::ZERO);
                self.go_to_hit(0, Some(Direction::Forward));
            }
        }
    }

    /// Searches again, keeping the matches around `pos` (stepping past the
    /// matches kept, or wrapping to the other end).
    fn refill_hits(&mut self, pos: CharPos) {
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let Some(f) = s.find.as_mut() else {
            return;
        };
        if let Ok(a) =
            find_scan::collect_around(s.doc.text(), &f.query, pos, find_scan::MAX_STORED_HITS)
        {
            f.hits = a.hits;
            f.total = a.total;
            f.first_index = a.first_index;
            f.current = None;
        }
    }

    /// Find next or previous; opens the prompt when there is no search yet.
    pub(crate) fn find_step(&mut self, dir: Direction) -> Vec<Effect> {
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let Some(f) = &s.find else {
            return self.prompt(PromptPurpose::Find);
        };
        if f.total == 0 || f.hits.is_empty() {
            let msg = self.msg_args(
                "marks-no-matches",
                &args!["pattern" => f.query.pattern.as_str()],
            );
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let cursor = s.cursor;
        let kept_before = f.first_index > 0;
        let kept_after = f.first_index + f.hits.len() < f.total;
        let step = match dir {
            Direction::Forward => f.hits.iter().position(|h| h.start > cursor),
            Direction::Backward => f.hits.iter().rposition(|h| h.start < cursor),
        };
        // Matches may lie beyond the ones kept: search again around the
        // cursor before wrapping.
        let step = match (dir, step) {
            (_, Some(i)) => Some(i),
            (Direction::Forward, None) if kept_after || kept_before => {
                self.refill_hits(cursor.saturating_add(1));
                self.hit_after(cursor)
            }
            (Direction::Backward, None) if kept_after || kept_before => {
                self.refill_hits(cursor);
                self.hit_before(cursor)
            }
            (_, None) => None,
        };
        let (i, wrapped) = match (dir, step) {
            (_, Some(i)) => (Some(i), None),
            // Wrap to the other end of the document.
            (Direction::Forward, None) => {
                if self.kept_window().is_some_and(|(before, _)| before) {
                    self.refill_hits(CharPos::ZERO);
                }
                (Some(0), Some(dir))
            }
            (Direction::Backward, None) => {
                if self.kept_window().is_some_and(|(_, after)| after) {
                    self.refill_hits(CharPos(usize::MAX));
                }
                let n = self
                    .session
                    .as_ref()
                    .and_then(|s| s.find.as_ref())
                    .map_or(0, |f| f.hits.len());
                (n.checked_sub(1), Some(dir))
            }
        };
        if let Some(i) = i {
            self.go_to_hit(i, wrapped);
        }
        vec![Effect::Redraw]
    }

    /// Whether matches exist before and after the ones kept.
    fn kept_window(&self) -> Option<(bool, bool)> {
        let f = self.session.as_ref()?.find.as_ref()?;
        Some((f.first_index > 0, f.first_index + f.hits.len() < f.total))
    }

    /// Index into the kept hits of the first match after `pos`.
    fn hit_after(&self, pos: CharPos) -> Option<usize> {
        let f = self.session.as_ref()?.find.as_ref()?;
        f.hits.iter().position(|h| h.start > pos)
    }

    /// Index into the kept hits of the last match before `pos`.
    fn hit_before(&self, pos: CharPos) -> Option<usize> {
        let f = self.session.as_ref()?.find.as_ref()?;
        f.hits.iter().rposition(|h| h.start < pos)
    }

    fn go_to_hit(&mut self, i: usize, wrapped: Option<Direction>) {
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let Some(f) = s.find.as_mut() else {
            return;
        };
        let Some(&hit) = f.hits.get(i) else {
            return;
        };
        f.current = Some(i);
        let n = f.total;
        let number = f.first_index + i + 1;
        let line = text_util::line_of(&s.doc, hit.start);
        let context = preview(&s.doc, text_util::line_range(&s.doc, line), 12);
        let label = self.msg_args("marks-match-label", &args!["number" => number, "n" => n]);
        let mut msg = self.nav_message(Some(&label), hit.start, &context);
        if let Some(dir) = wrapped {
            self.speech.earcon(Earcon::Wrap);
            msg = self.msg_args(
                "marks-find-wrapped",
                &args!["dir" => crate::words::dir_key(dir), "message" => msg],
            );
        }
        self.jump(hit.start, true, ReadAfter::Follow, &msg);
    }

    /// Adds a bookmark at the reading position, named `mark1`, `mark2`, ...
    /// (the first free name, as in Star), and saves it at once.
    pub(crate) fn add_bookmark(&mut self) {
        let Some(pos) = self.reading_position() else {
            return;
        };
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let pos = text_util::word_start(&s.doc, pos);
        if let Some(b) = s.bookmarks.iter().find(|b| b.pos == pos) {
            let name = b.name.clone();
            let msg = self.msg_args("marks-bookmark-already-here", &args!["name" => name]);
            self.tell(&msg);
            return;
        }
        let name = (1..)
            .map(|n| format!("mark{n}"))
            .find(|n| !s.bookmarks.iter().any(|b| &b.name == n))
            .unwrap_or_default();
        let pct = text_util::percent(&s.doc, pos);
        s.bookmarks.push(Bookmark {
            name: name.clone(),
            pos,
            pct,
            ts: textweaver_store::now_ts(),
            anchor: Some(text_util::anchor_at(&s.doc, pos)),
            not_found: false,
        });
        s.bookmarks.sort_by_key(|b| b.pos);
        // Saved on the writer; "set" is said once the file is written (or
        // why it could not be), on the next tick. Without a place to save
        // (or while editing), it is set at once.
        let note = crate::writer::StateNote::Bookmark {
            name: name.clone(),
            pct,
        };
        if !self.save_state(note) {
            let msg = self.msg_args("marks-bookmark-set", &args!["name" => name, "pct" => pct]);
            self.tell(&msg);
        }
    }

    pub(crate) fn list_bookmarks(&mut self) -> Vec<Effect> {
        let n = self.session.as_ref().map_or(0, |s| s.bookmarks.len());
        if n == 0 {
            let msg = self.msg("marks-no-bookmarks");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let effects = self.list_bookmarks_quiet();
        let msg = self.msg_args("marks-bookmarks-intro", &args!["n" => n]);
        self.tell(&msg);
        effects
    }

    /// The bookmark list effect, without announcing it (after a delete).
    pub(crate) fn list_bookmarks_quiet(&mut self) -> Vec<Effect> {
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let items: Vec<String> = s
            .bookmarks
            .iter()
            .map(|b| {
                let line = text_util::line_of(&s.doc, b.pos);
                let end = text_util::line_range(&s.doc, line).end.max(b.pos);
                let text = preview(&s.doc, CharRange::new(b.pos, end), 6);
                self.msg_args(
                    "marks-bookmark-item",
                    &args![
                        "lost" => if b.not_found { "yes" } else { "no" },
                        "name" => b.name.as_str(),
                        "line" => line + 1,
                        "pct" => text_util::percent(&s.doc, b.pos),
                        "text" => text
                    ],
                )
            })
            .collect();
        self.list = Some(ListKind::Bookmarks);
        vec![Effect::ShowList {
            title: self.msg("marks-bookmarks-title"),
            items,
        }]
    }

    pub(crate) fn bookmark_step(&mut self, dir: Direction) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        if s.bookmarks.is_empty() {
            let msg = self.msg("marks-no-bookmarks");
            self.tell(&msg);
            return;
        }
        let cursor = s.cursor;
        let (i, wrapped) = match dir {
            Direction::Forward => match s.bookmarks.iter().position(|b| b.pos > cursor) {
                Some(i) => (i, false),
                None => (0, true),
            },
            Direction::Backward => match s.bookmarks.iter().rposition(|b| b.pos < cursor) {
                Some(i) => (i, false),
                None => (s.bookmarks.len() - 1, true),
            },
        };
        if wrapped {
            self.speech.earcon(Earcon::Wrap);
        }
        self.go_to_bookmark_at(i, wrapped);
    }

    /// Jumps to bookmark `i` (0-based, in position order).
    pub(crate) fn go_to_bookmark(&mut self, i: usize) {
        self.go_to_bookmark_at(i, false);
    }

    fn go_to_bookmark_at(&mut self, i: usize, wrapped: bool) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let Some(b) = s.bookmarks.get(i) else {
            return;
        };
        let target = text_util::first_word_at_or_after(&s.doc, b.pos);
        let line = text_util::line_of(&s.doc, target);
        let end = text_util::line_range(&s.doc, line).end.max(target);
        let content = preview(&s.doc, CharRange::new(target, end), 8);
        let label = self.msg_args("marks-bookmark-label", &args!["name" => b.name.as_str()]);
        let mut msg = self.nav_message(Some(&label), target, &content);
        if wrapped {
            msg = self.msg_args("nav-wrapped", &args!["message" => msg]);
        }
        self.jump(target, true, ReadAfter::Follow, &msg);
    }

    /// Sets the selection (an empty range clears it).
    pub(crate) fn select(&mut self, range: CharRange) {
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let range = range.clamp_to(s.doc.len_chars());
        if range.is_empty() {
            s.selection = None;
            s.selection_anchor = None;
            let msg = self.msg("marks-selection-cleared");
            self.note(&msg);
            return;
        }
        s.selection = Some(range);
        s.selection_anchor = Some(range.start);
        s.cursor = range.end;
        let text = preview(&s.doc, range, 8);
        let msg = match self.settings.speech.verbosity {
            Verbosity::Low => text,
            _ => self.msg_args("marks-selected", &args!["text" => text]),
        };
        self.tell(&msg);
    }

    /// Moves the free end of the selection by one unit and says what was
    /// added or removed.
    pub(crate) fn extend_selection(&mut self, unit: Unit, dir: Direction) {
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let doc = &s.doc;
        let anchor = s.selection_anchor.unwrap_or(s.cursor);
        let head = s.cursor;
        let new_head = match (unit, dir) {
            (Unit::Line, Direction::Forward) => {
                let r = text_util::line_range(doc, text_util::line_of(doc, head));
                if head < r.end {
                    r.end
                } else {
                    let next = text_util::line_of(doc, head) + 1;
                    if next < text_util::line_count(doc) {
                        text_util::line_range(doc, next).end
                    } else {
                        head
                    }
                }
            }
            (Unit::Line, Direction::Backward) => {
                let line = text_util::line_of(doc, head);
                let r = text_util::line_range(doc, line);
                if head > r.start || line == 0 {
                    r.start
                } else {
                    text_util::line_range(doc, line - 1).start
                }
            }
            (Unit::Grapheme, _) => navigate(doc, head, Unit::Grapheme, dir, NavOptions::default())
                .map_or(head, |t| t.range.start),
            (_, Direction::Forward) => match text_util::word_containing(doc, head) {
                Some(w) if head < w.end => w.end,
                _ => navigate(doc, head, Unit::Word, dir, NavOptions::default())
                    .map_or(head, |t| t.range.end),
            },
            (_, Direction::Backward) => navigate(doc, head, Unit::Word, dir, NavOptions::default())
                .map_or(head, |t| t.range.start),
        };
        if new_head == head {
            self.speech.earcon(Earcon::Boundary);
            let msg = self.msg(match dir {
                Direction::Forward => "nav-end-of-document-stop",
                Direction::Backward => "nav-top-of-document-stop",
            });
            self.tell(&msg);
            return;
        }
        let changed = CharRange::new(head, new_head);
        let text = s.doc.slice(changed);
        let sel = CharRange::new(anchor, new_head);
        let grew = sel.contains_range(changed);
        s.selection_anchor = Some(anchor);
        s.selection = (!sel.is_empty()).then_some(sel);
        s.cursor = new_head;
        let what = if grew { "selected" } else { "unselected" };
        let msg = match textweaver_editor::echo::summarize(&text, what) {
            Some(summary) => summary,
            // Low says the text alone.
            None if self.settings.speech.verbosity == Verbosity::Low => {
                text_util::spoken_fragment(&text)
            }
            None => text_util::selection_change_message(&text, what),
        };
        self.scroll_to_cursor();
        self.tell(&msg);
    }

    /// Position of bookmark `i`, if any (tests and frontends).
    pub fn bookmark_position(&self, i: usize) -> Option<CharPos> {
        self.session.as_ref()?.bookmarks.get(i).map(|b| b.pos)
    }
}
