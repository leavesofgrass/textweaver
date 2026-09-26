//! Find, bookmarks, and selection.
//!
//! Find is case-insensitive plain text (Star's behavior); a pattern written
//! as `/regex/` is a regular expression. Matches wrap at the document ends
//! (Star wraps search in both UIs), and the wrap is announced. Bookmarks go
//! to the first word at or after their position (Star's GUI rule; the TUI's
//! "closest word" quirk is not kept) and wrap the same way.

use textweaver_a11y::Verbosity;
use textweaver_core::{CharPos, CharRange, Direction, Unit};
use textweaver_speech::Earcon;
use textweaver_store::Bookmark;
use textweaver_text::{NavOptions, SearchQuery, find_all, navigate};

use crate::app::{App, FindState, ListKind};
use crate::command::{Effect, PromptPurpose};
use crate::nav::ReadAfter;
use crate::text_util::{self, preview};

impl App {
    /// Runs a new search from the cursor.
    pub(crate) fn run_find(&mut self, pattern: &str) {
        if pattern.is_empty() {
            self.note("Cancelled.");
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
        let hits = match find_all(&s.doc, &query) {
            Ok(h) => h,
            Err(e) => {
                self.error(&format!("Cannot search: {e}."));
                return;
            }
        };
        let cursor = s.cursor;
        let first = hits.iter().position(|h| h.start >= cursor);
        s.find = Some(FindState {
            query,
            hits,
            current: None,
        });
        let count = s.find.as_ref().map_or(0, |f| f.hits.len());
        if count == 0 {
            self.speech.earcon(Earcon::Error);
            self.tell(&format!("No matches for {pattern}."));
            return;
        }
        match first {
            Some(i) => self.go_to_hit(i, None),
            None => self.go_to_hit(0, Some(Direction::Forward)),
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
        if f.hits.is_empty() {
            let msg = format!("No matches for {}.", f.query.pattern);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let cursor = s.cursor;
        let (i, wrapped) = match dir {
            Direction::Forward => match f.hits.iter().position(|h| h.start > cursor) {
                Some(i) => (i, None),
                None => (0, Some(dir)),
            },
            Direction::Backward => match f.hits.iter().rposition(|h| h.start < cursor) {
                Some(i) => (i, None),
                None => (f.hits.len() - 1, Some(dir)),
            },
        };
        self.go_to_hit(i, wrapped);
        vec![Effect::Redraw]
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
        let n = f.hits.len();
        let line = text_util::line_of(&s.doc, hit.start);
        let context = preview(&s.doc, text_util::line_range(&s.doc, line), 12);
        let label = format!("Match {} of {n}", i + 1);
        let mut msg = self.nav_message(Some(&label), hit.start, &context);
        if let Some(dir) = wrapped {
            self.speech.earcon(Earcon::Wrap);
            let edge = match dir {
                Direction::Forward => "Wrapped to top.",
                Direction::Backward => "Wrapped to bottom.",
            };
            msg = format!("{edge} {msg}");
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
            let msg = format!("Bookmark {} is already here.", b.name);
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
        });
        s.bookmarks.sort_by_key(|b| b.pos);
        match self.save_position() {
            Ok(()) => self.tell(&format!("Bookmark {name} set at {pct} percent.")),
            Err(e) => {
                // Kept for this session and saved again with the position;
                // the user must not believe it is safe on disk.
                log::warn!("cannot save bookmarks: {e}");
                self.speech.earcon(Earcon::Error);
                self.error(&format!(
                    "Bookmark {name} is set for now, but could not be saved: {e}."
                ));
            }
        }
    }

    pub(crate) fn list_bookmarks(&mut self) -> Vec<Effect> {
        let n = self.session.as_ref().map_or(0, |s| s.bookmarks.len());
        if n == 0 {
            self.tell("No bookmarks.");
            return vec![Effect::Redraw];
        }
        let effects = self.list_bookmarks_quiet();
        self.tell(&format!(
            "Bookmarks, {n} {}. Enter goes to one, Delete deletes it, F2 renames it.",
            if n == 1 { "item" } else { "items" }
        ));
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
                format!(
                    "{}, line {}, {} percent: {text}",
                    b.name,
                    line + 1,
                    text_util::percent(&s.doc, b.pos)
                )
            })
            .collect();
        self.list = Some(ListKind::Bookmarks);
        vec![Effect::ShowList {
            title: "Bookmarks".into(),
            items,
        }]
    }

    pub(crate) fn bookmark_step(&mut self, dir: Direction) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        if s.bookmarks.is_empty() {
            self.tell("No bookmarks.");
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
        let label = format!("Bookmark {}", b.name);
        let mut msg = self.nav_message(Some(&label), target, &content);
        if wrapped {
            msg = format!("Wrapped. {msg}");
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
            self.note("Selection cleared.");
            return;
        }
        s.selection = Some(range);
        s.selection_anchor = Some(range.start);
        s.cursor = range.end;
        let text = preview(&s.doc, range, 8);
        let msg = match self.settings.speech.verbosity {
            Verbosity::Low => text,
            _ => format!("Selected {text}"),
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
            self.tell(match dir {
                Direction::Forward => "End of document.",
                Direction::Backward => "Top of document.",
            });
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
