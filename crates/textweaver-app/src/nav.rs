//! Navigation: sentence, paragraph, structure, chapter, history, go-to, and
//! caret moves.
//!
//! One history rule (fixing Star's three inconsistent ones): every jump by a
//! sentence or larger unit, every structural jump (heading, table, list,
//! list item, link, chapter), every find, go-to, document start or end, and
//! bookmark jump records the departure point exactly once. Caret moves (word,
//! line, page), scrolling, and Speech Cursor line moves do not. History moves
//! themselves never record.

use textweaver_a11y::{Channel, Verbosity};
use textweaver_core::{CharPos, CharRange, Direction, MarkerKind, Unit};
use textweaver_speech::Earcon;
use textweaver_text::units::unit_at;
use textweaver_text::{Document, GoTo, NavOptions, navigate};

use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;

use crate::app::{App, Mode};
use crate::playback::Playback;
use crate::text_util::{self, preview};
use crate::words::{dir_key, kind_name, unit_key, unit_name};

/// What happens to speech after a jump.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadAfter {
    /// Restart reading at the target if reading; otherwise announce.
    Follow,
    /// Always read from the target (replay, read-heading).
    Always,
}

/// Words of preview spoken for a sentence or paragraph jump.
const PREVIEW_WORDS: usize = 8;

impl App {
    fn nav_opts(&self) -> NavOptions {
        NavOptions {
            wrap: self.settings.reading.wrap_navigation,
        }
    }

    /// Builds a navigation message for the current verbosity: the content
    /// alone at Low, with its structure label at Normal, with position
    /// details at High.
    pub(crate) fn nav_message(
        &self,
        label: Option<&str>,
        target: CharPos,
        content: &str,
    ) -> String {
        let v = self.settings.speech.verbosity;
        let blank;
        let content = if content.trim().is_empty() {
            blank = self.msg("nav-blank");
            blank.as_str()
        } else {
            content
        };
        if v == Verbosity::High {
            let (line, pct) = self
                .session
                .as_ref()
                .map(|s| {
                    (
                        text_util::line_of(&s.doc, target) + 1,
                        text_util::percent(&s.doc, target),
                    )
                })
                .unwrap_or_default();
            return match label {
                Some(l) => self.msg_args(
                    "nav-message-at-labelled",
                    &args!["label" => l, "line" => line, "pct" => pct, "content" => content],
                ),
                None => self.msg_args(
                    "nav-message-at",
                    &args!["line" => line, "pct" => pct, "content" => content],
                ),
            };
        }
        match (v, label) {
            (Verbosity::Normal, Some(l)) => self.msg_args(
                "nav-message-labelled",
                &args!["label" => l, "content" => content],
            ),
            _ => content.to_owned(),
        }
    }

    /// Moves to `target`, recording history when asked, then reads or
    /// announces. In Speech Cursor mode the target's line becomes the Speech
    /// Cursor line and is read.
    pub(crate) fn jump(&mut self, target: CharPos, record: bool, read: ReadAfter, message: &str) {
        let departure = self.reading_position();
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let target = target.clamp_to(s.doc.len_chars());
        if record && let Some(from) = departure.filter(|&d| d != target) {
            s.history.record(from);
        }
        s.cursor = target;
        s.goal_column = None;
        if self.mode == Mode::SpeechCursor {
            let line = text_util::line_of(&s.doc, target);
            s.speech_cursor_line = Some(line);
            self.scroll_to_line(line);
            self.speech_cursor_read();
            // What is read aloud is not copied for a screen reader too.
            self.show_as(Channel::Line, message);
            return;
        }
        self.scroll_to_cursor();
        match (read, self.playback) {
            (ReadAfter::Always, _) => {
                self.stop_speech();
                self.read_from(target);
                self.show_nav_while_reading(message);
            }
            (ReadAfter::Follow, Playback::Reading) => {
                self.read_from(target);
                self.show_nav_while_reading(message);
            }
            (ReadAfter::Follow, _) => {
                self.follow_jump(target);
                self.tell(message);
            }
        }
    }

    /// Shows where a jump landed while reading restarts there: on the
    /// status line unless the reading is aloud and a screen reader would
    /// read the same text too (or the screen is kept quiet). Continuous
    /// reading on the status line shows the sentence itself instead.
    fn show_nav_while_reading(&mut self, message: &str) {
        if self.screen_say_all_running() {
            return;
        }
        self.show_as(Channel::Line, message);
    }

    /// The unit containing `pos`, else the one before it, else the one after.
    fn current_unit(doc: &Document, pos: CharPos, unit: Unit) -> Option<CharRange> {
        unit_at(doc, pos, unit)
            .filter(|r| r.start <= pos)
            .or_else(|| {
                navigate(doc, pos, unit, Direction::Backward, NavOptions::default())
                    .map(|t| t.range)
            })
            .or_else(|| unit_at(doc, pos, unit))
    }

    fn here(&self) -> Option<(CharPos, &Document)> {
        let pos = self.reading_position()?;
        let s = self.session.as_ref()?;
        Some((text_util::word_start(&s.doc, pos), &s.doc))
    }

    fn unit_jump(&mut self, unit: Unit, dir: Direction) {
        let opts = self.nav_opts();
        let Some((pos, doc)) = self.here() else {
            return;
        };
        match navigate(doc, pos, unit, dir, opts) {
            Some(t) => {
                let content = preview(doc, t.range, PREVIEW_WORDS);
                let mut msg = self.nav_message(None, t.range.start, &content);
                if t.wrapped {
                    msg = self.msg_args("nav-wrapped", &args!["message" => msg]);
                    self.speech.earcon(Earcon::Wrap);
                }
                self.jump(t.range.start, true, ReadAfter::Follow, &msg);
            }
            None => {
                self.speech.earcon(Earcon::Boundary);
                let missing = self.no_unit_message(unit, dir);
                self.tell(&missing);
            }
        }
    }

    pub(crate) fn next_sentence(&mut self) {
        self.unit_jump(Unit::Sentence, Direction::Forward);
    }

    /// Star's rule: more than three words into the current sentence,
    /// previous rewinds to its start; otherwise it goes to the previous
    /// sentence.
    pub(crate) fn previous_sentence(&mut self) {
        let opts = self.nav_opts();
        let Some((pos, doc)) = self.here() else {
            return;
        };
        let current = unit_at(doc, pos, Unit::Sentence).filter(|r| r.start <= pos);
        if let Some(cur) = current
            && text_util::words_between(doc, cur.start, pos, 4) > 3
        {
            let content = preview(doc, cur, PREVIEW_WORDS);
            let msg = self.nav_message(None, cur.start, &content);
            self.jump(cur.start, true, ReadAfter::Follow, &msg);
            return;
        }
        let from = current.map_or(pos, |c| c.start);
        match navigate(doc, from, Unit::Sentence, Direction::Backward, opts) {
            Some(t) => {
                let content = preview(doc, t.range, PREVIEW_WORDS);
                let mut msg = self.nav_message(None, t.range.start, &content);
                if t.wrapped {
                    msg = self.msg_args("nav-wrapped", &args!["message" => msg]);
                    self.speech.earcon(Earcon::Wrap);
                }
                self.jump(t.range.start, true, ReadAfter::Follow, &msg);
            }
            None => match current.filter(|c| c.start < pos) {
                Some(c) => {
                    let content = preview(doc, c, PREVIEW_WORDS);
                    let msg = self.nav_message(None, c.start, &content);
                    self.jump(c.start, true, ReadAfter::Follow, &msg);
                }
                None => {
                    self.speech.earcon(Earcon::Boundary);
                    let missing = self.no_unit_message(Unit::Sentence, Direction::Backward);
                    self.tell(&missing);
                }
            },
        }
    }

    pub(crate) fn replay_sentence(&mut self) {
        self.replay(Unit::Sentence);
    }

    pub(crate) fn replay_paragraph(&mut self) {
        self.replay(Unit::Paragraph);
    }

    /// Reads again from the start of the current unit; always plays.
    fn replay(&mut self, unit: Unit) {
        let Some((pos, doc)) = self.here() else {
            return;
        };
        match Self::current_unit(doc, pos, unit) {
            Some(r) => {
                let content = preview(doc, r, PREVIEW_WORDS);
                let msg = self.nav_message(None, r.start, &content);
                self.jump(r.start, true, ReadAfter::Always, &msg);
            }
            None => {
                let what = unit_name(self.cat(), unit);
                let msg = self.msg_args(
                    "nav-nothing-to-read",
                    &args!["what" => what, "unit" => unit_key(unit)],
                );
                self.tell(&msg);
            }
        }
    }

    pub(crate) fn next_paragraph(&mut self) {
        self.unit_jump(Unit::Paragraph, Direction::Forward);
    }

    /// Previous paragraph: the start of the current one when past it,
    /// otherwise the one before.
    pub(crate) fn previous_paragraph(&mut self) {
        self.unit_jump(Unit::Paragraph, Direction::Backward);
    }

    fn marker_at(
        doc: &Document,
        kind: MarkerKind,
        range: CharRange,
    ) -> Option<&textweaver_text::Marker> {
        doc.markers()
            .iter()
            .find(|m| m.kind == kind && m.range == range)
    }

    fn marker_content(doc: &Document, kind: MarkerKind, range: CharRange) -> String {
        let label = Self::marker_at(doc, kind, range).and_then(|m| m.label.clone());
        match label {
            // An ordered item's label is its number: say it before the
            // item's text ("3. Buy milk"), not instead of it.
            Some(l) if kind == MarkerKind::ListItem => {
                let text = preview(doc, range, PREVIEW_WORDS);
                if text.starts_with(l.as_str()) {
                    text
                } else {
                    format!("{l} {text}")
                }
            }
            Some(l) => l,
            None => preview(doc, range, PREVIEW_WORDS),
        }
    }

    fn marker_label(c: &Catalog, doc: &Document, kind: MarkerKind, range: CharRange) -> String {
        let level = Self::marker_at(doc, kind, range).map_or(0, |m| m.level);
        match kind {
            MarkerKind::Heading if level > 0 => {
                c.fmt("nav-label-heading-level", &args!["level" => level])
            }
            MarkerKind::List | MarkerKind::Table => {
                let inner = if kind == MarkerKind::List {
                    MarkerKind::ListItem
                } else {
                    MarkerKind::TableRow
                };
                let n = doc
                    .markers()
                    .iter()
                    .filter(|m| m.kind == inner && range.contains_range(m.range))
                    .count();
                let id = if kind == MarkerKind::List {
                    "nav-label-list"
                } else {
                    "nav-label-table"
                };
                c.fmt(id, &args!["n" => n])
            }
            MarkerKind::ListItem if level > 1 => {
                c.fmt("nav-label-list-item-level", &args!["level" => level])
            }
            _ => capitalize(&kind_name(c, kind)),
        }
    }

    /// Jumps to the next or previous marker of `kind` (at `level`).
    fn structure_jump(
        &mut self,
        kind: MarkerKind,
        level: Option<u8>,
        dir: Direction,
        read: ReadAfter,
        missing: &str,
    ) {
        // Structure typed a moment ago counts (edit mode re-parses after a
        // pause; small texts are parsed now).
        self.refresh_structure(false);
        let opts = self.nav_opts();
        let Some((pos, doc)) = self.here() else {
            return;
        };
        let unit = Unit::Marker { kind, level };
        match navigate(doc, pos, unit, dir, opts) {
            Some(t) => {
                let label = Self::marker_label(self.cat(), doc, kind, t.range);
                // An empty graphic is a picture with no description.
                let content = if kind == MarkerKind::Image && t.range.is_empty() {
                    self.msg("nav-no-description")
                } else {
                    Self::marker_content(doc, kind, t.range)
                };
                let mut msg = self.nav_message(Some(&label), t.range.start, &content);
                if t.wrapped {
                    msg = self.msg_args("nav-wrapped", &args!["message" => msg]);
                    self.speech.earcon(Earcon::Wrap);
                }
                self.jump(t.range.start, true, read, &msg);
            }
            None => {
                self.speech.earcon(Earcon::Boundary);
                self.tell(missing);
            }
        }
    }

    /// Next or previous heading; `read` reads from it (Star's `<` `>`),
    /// otherwise it only moves (Star's `{` `}`), resuming speech if it was
    /// reading.
    pub(crate) fn heading(&mut self, dir: Direction, read: bool) {
        let read = if read {
            ReadAfter::Always
        } else {
            ReadAfter::Follow
        };
        let missing = self.no_unit_message(Unit::marker(MarkerKind::Heading), dir);
        self.structure_jump(MarkerKind::Heading, None, dir, read, &missing);
    }

    /// Keys 1 to 6 (and Shift with them): the next or previous heading at
    /// `level`, moving without reading (reading follows if it was on).
    pub(crate) fn heading_level_jump(&mut self, level: u8, dir: Direction) {
        let missing = self.msg_args(
            "nav-no-heading-level",
            &args!["dir" => dir_key(dir), "level" => level],
        );
        self.structure_jump(
            MarkerKind::Heading,
            Some(level),
            dir,
            ReadAfter::Follow,
            &missing,
        );
    }

    /// The structure of the canonical line `line`, said before its text on
    /// caret and Speech Cursor moves: "heading level 2", "list item",
    /// "list item, level 2", "row 3", "code", "block quote". `None` for
    /// plain text.
    pub(crate) fn line_structure(c: &Catalog, doc: &Document, line: usize) -> Option<String> {
        let r = text_util::line_range(doc, line);
        let index = doc.marker_index();
        let at = r.start;
        if let Some(h) = index
            .starting_in(CharRange::new(at, r.end.max(at.saturating_add(1))))
            .iter()
            .find(|m| m.kind == MarkerKind::Heading)
        {
            return Some(c.fmt("nav-line-heading", &args!["level" => h.level]));
        }
        if let Some(row) = index.enclosing(MarkerKind::TableRow, at) {
            let n = index.enclosing(MarkerKind::Table, at).map_or(1, |t| {
                doc.markers()
                    .iter()
                    .filter(|m| {
                        m.kind == MarkerKind::TableRow
                            && t.range.contains_range(m.range)
                            && m.range.start <= row.range.start
                    })
                    .count()
            });
            return Some(c.fmt("nav-line-row", &args!["n" => n]));
        }
        if let Some(item) = index.enclosing(MarkerKind::ListItem, at) {
            return Some(if item.level > 1 {
                c.fmt("nav-line-list-item-level", &args!["level" => item.level])
            } else {
                kind_name(c, MarkerKind::ListItem)
            });
        }
        if let Some(m) = index
            .enclosing(MarkerKind::Code, at)
            .filter(|m| m.level == 1)
        {
            // W4g: the block's first line names its language.
            return Some(crate::authoring::code_structure(c, m, r));
        }
        if index.enclosing(MarkerKind::Quote, at).is_some() {
            return Some(kind_name(c, MarkerKind::Quote));
        }
        None
    }

    /// Which structure [`line_structure`](Self::line_structure) names for
    /// line `line`, in the same order: a heading, a table row, a list item,
    /// code, or a block quote.
    pub(crate) fn line_kind(doc: &Document, line: usize) -> Option<MarkerKind> {
        let r = text_util::line_range(doc, line);
        let index = doc.marker_index();
        let at = r.start;
        if index
            .starting_in(CharRange::new(at, r.end.max(at.saturating_add(1))))
            .iter()
            .any(|m| m.kind == MarkerKind::Heading)
        {
            return Some(MarkerKind::Heading);
        }
        [
            MarkerKind::TableRow,
            MarkerKind::ListItem,
            MarkerKind::Code,
            MarkerKind::Quote,
        ]
        .into_iter()
        .find(|&k| {
            index
                .enclosing(k, at)
                .is_some_and(|m| k != MarkerKind::Code || m.level == 1)
        })
    }

    /// Next or previous table, list, list item, or link.
    pub(crate) fn marker_jump(&mut self, kind: MarkerKind, dir: Direction) {
        let missing = self.no_unit_message(Unit::marker(kind), dir);
        self.structure_jump(kind, None, dir, ReadAfter::Follow, &missing);
    }

    /// Chapters are section breaks when the document has them (EPUB spine
    /// items, DOCX sections), otherwise level-1 headings. Previous rewinds to
    /// the chapter start when more than five words in (Star's rule).
    pub(crate) fn chapter(&mut self, dir: Direction) {
        self.refresh_structure(false);
        let Some((pos, doc)) = self.here() else {
            return;
        };
        let has = |k: MarkerKind, l: Option<u8>| {
            doc.markers()
                .iter()
                .any(|m| m.kind == k && l.is_none_or(|l| m.level == l))
        };
        let (kind, level) = if has(MarkerKind::SectionBreak, None) {
            (MarkerKind::SectionBreak, None)
        } else if has(MarkerKind::Heading, Some(1)) {
            (MarkerKind::Heading, Some(1))
        } else {
            let msg = self.msg("nav-no-chapters");
            self.tell(&msg);
            return;
        };
        let unit = Unit::Marker { kind, level };
        let opts = self.nav_opts();
        let target = match dir {
            Direction::Forward => navigate(doc, pos, unit, dir, opts),
            Direction::Backward => {
                let containing = navigate(
                    doc,
                    pos.saturating_add(1),
                    unit,
                    Direction::Backward,
                    NavOptions::default(),
                )
                .filter(|t| t.range.start <= pos);
                match containing {
                    Some(c) if text_util::words_between(doc, c.range.start, pos, 6) > 5 => Some(c),
                    Some(c) => navigate(doc, c.range.start, unit, dir, opts),
                    None => navigate(doc, pos, unit, dir, opts),
                }
            }
        };
        match target {
            Some(t) => {
                let content = Self::marker_content(doc, kind, t.range);
                let label = self.msg("nav-chapter");
                let mut msg = self.nav_message(Some(&label), t.range.start, &content);
                if t.wrapped {
                    msg = self.msg_args("nav-wrapped", &args!["message" => msg]);
                }
                self.jump(t.range.start, true, ReadAfter::Follow, &msg);
            }
            None => {
                self.speech.earcon(Earcon::Boundary);
                let missing = self.msg_args("nav-no-chapter", &args!["dir" => dir_key(dir)]);
                self.tell(&missing);
            }
        }
    }

    pub(crate) fn history_back(&mut self) {
        // Back across a followed link first, when nothing was jumped to
        // since arriving.
        if self.link_history_back() {
            return;
        }
        let Some(current) = self.reading_position() else {
            return;
        };
        let Some(s) = self.session.as_mut() else {
            return;
        };
        match s.history.back(current) {
            Some(p) => {
                let msg = self.position_message(&self.msg("nav-back"), p);
                self.jump(p, false, ReadAfter::Follow, &msg);
            }
            None => {
                let msg = self.msg("nav-no-earlier-history");
                self.tell(&msg);
            }
        }
    }

    pub(crate) fn history_forward(&mut self) {
        let Some(s) = self.session.as_mut() else {
            return;
        };
        match s.history.forward() {
            Some(p) => {
                let msg = self.position_message(&self.msg("nav-forward"), p);
                self.jump(p, false, ReadAfter::Follow, &msg);
            }
            None => {
                let msg = self.msg("nav-no-forward-history");
                self.tell(&msg);
            }
        }
    }

    /// "Label, line N: preview" (verbosity-shaped).
    fn position_message(&self, label: &str, pos: CharPos) -> String {
        let Some(s) = self.session.as_ref() else {
            return String::new();
        };
        let line = text_util::line_of(&s.doc, pos);
        let content = preview(
            &s.doc,
            CharRange::new(pos, text_util::line_range(&s.doc, line).end),
            PREVIEW_WORDS,
        );
        let label = self.msg_args(
            "nav-label-line",
            &args!["label" => label, "line" => line + 1],
        );
        self.nav_message(Some(&label), pos, &content)
    }

    /// Jumps to a go-to target (records history).
    pub(crate) fn go_to(&mut self, target: GoTo) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let pos = textweaver_text::go_to(&s.doc, target);
        let pos = match target {
            GoTo::End => text_util::last_start(&s.doc, Unit::Word).unwrap_or(pos),
            _ => pos,
        };
        let pct = text_util::percent(&s.doc, pos);
        let msg = self.position_message(&self.msg_args("nav-percent", &args!["pct" => pct]), pos);
        self.jump(pos, true, ReadAfter::Follow, &msg);
    }

    /// Start of the document, or its last word.
    pub(crate) fn document_edge(&mut self, dir: Direction) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let (pos, label) = match dir {
            Direction::Backward => (CharPos::ZERO, "nav-top-of-document"),
            Direction::Forward => (
                text_util::last_start(&s.doc, Unit::Word).unwrap_or(s.doc.end()),
                "nav-end-of-document",
            ),
        };
        let line = text_util::line_of(&s.doc, pos);
        let content = preview(
            &s.doc,
            CharRange::new(pos, text_util::line_range(&s.doc, line).end),
            PREVIEW_WORDS,
        );
        let msg = self.msg_args(
            "nav-edge-message",
            &args!["edge" => self.msg(label), "content" => content],
        );
        self.jump(pos, true, ReadAfter::Follow, &msg);
    }

    /// Caret by word: stops reading and says the word.
    pub(crate) fn caret_word(&mut self, dir: Direction) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let cursor = s.cursor;
        match navigate(&s.doc, cursor, Unit::Word, dir, NavOptions::default()) {
            Some(t) => {
                let word = s.doc.slice(t.range);
                // A citation is said in words; a note's passage says so.
                let said = self.citation_description_at(t.range.start).unwrap_or(word);
                let said = match self.difficult_word_note(t.range) {
                    Some(n) => format!("{said}{n}"),
                    None => said,
                };
                let said = match self.note_here_suffix(t.range.start) {
                    Some(n) if cursor_left_note(self, cursor, t.range.start) => {
                        format!("{said}. {n}")
                    }
                    _ => said,
                };
                self.caret_to(t.range.start);
                self.speak_content(Channel::Caret, &said);
            }
            None => {
                self.speech.earcon(Earcon::Boundary);
                let edge = self.edge_message(dir);
                self.tell(&edge);
            }
        }
    }

    /// Moves the caret without history or reading.
    pub(crate) fn caret_to(&mut self, pos: CharPos) {
        self.stop_speech();
        let sc = self.mode == Mode::SpeechCursor;
        if let Some(s) = self.session.as_mut() {
            s.cursor = pos.clamp_to(s.doc.len_chars());
            if sc {
                s.speech_cursor_line = Some(text_util::line_of(&s.doc, s.cursor));
            }
        }
        self.scroll_to_cursor();
    }

    /// Caret by line: to the word nearest the goal column on the next line
    /// that has words (Star's rule), then says that line.
    pub(crate) fn caret_line(&mut self, dir: Direction) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let doc = &s.doc;
        let line = text_util::line_of(doc, s.cursor);
        let goal = s
            .goal_column
            .unwrap_or_else(|| s.cursor.0 - text_util::line_range(doc, line).start.0);
        let count = text_util::line_count(doc);
        let mut next = line;
        let found = loop {
            next = match dir {
                Direction::Forward if next + 1 < count => next + 1,
                Direction::Backward if next > 0 => next - 1,
                _ => break None,
            };
            let r = text_util::line_range(doc, next);
            if !text_util::is_blank(doc, r) {
                break Some(r);
            }
        };
        let Some(r) = found else {
            self.speech.earcon(Earcon::Boundary);
            let edge = self.edge_message(dir);
            self.tell(&edge);
            return;
        };
        let at = CharPos(r.start.0 + goal.min(r.len()));
        let target = text_util::word_containing(doc, at)
            .map(|w| w.start)
            .or_else(|| {
                unit_at(doc, at, Unit::Word)
                    .filter(|w| w.start < r.end)
                    .map(|w| w.start)
            })
            .or_else(|| {
                navigate(
                    doc,
                    at,
                    Unit::Word,
                    Direction::Backward,
                    NavOptions::default(),
                )
                .filter(|w| w.range.start >= r.start)
                .map(|w| w.range.start)
            })
            .unwrap_or(r.start);
        let text = doc.slice(r);
        // Structure first: "heading level 2, Methods" (Normal and above).
        let text = match Self::line_structure(self.cat(), doc, text_util::line_of(doc, r.start)) {
            Some(kind) if self.settings.speech.verbosity >= Verbosity::Normal => {
                format!("{kind}, {text}")
            }
            _ => text,
        };
        self.caret_to(target);
        if let Some(s) = self.session.as_mut() {
            s.goal_column = Some(goal);
        }
        self.speak_content(Channel::Caret, &text);
    }

    /// Page moves: by the viewport height less four lines (Star's rule);
    /// reading, if any, restarts at the new place.
    pub(crate) fn page(&mut self, dir: Direction) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let lines = match usize::from(self.view.height) {
            0 => 20,
            h => h.saturating_sub(4).max(1),
        };
        let line = if self.mode == Mode::SpeechCursor {
            s.speech_cursor_line.unwrap_or_else(|| s.line())
        } else {
            s.line()
        };
        let last = text_util::line_count(&s.doc) - 1;
        let target_line = match dir {
            Direction::Forward => (line + lines).min(last),
            Direction::Backward => line.saturating_sub(lines),
        };
        if target_line == line {
            self.speech.earcon(Earcon::Boundary);
            let edge = self.edge_message(dir);
            self.tell(&edge);
            return;
        }
        let start = text_util::line_range(&s.doc, target_line).start;
        let target = if self.mode == Mode::SpeechCursor {
            start
        } else {
            let w = text_util::first_word_at_or_after(&s.doc, start);
            if w < start { start } else { w }
        };
        let msg = self.position_message(&self.msg("nav-page"), target);
        self.jump(target, false, ReadAfter::Follow, &msg);
    }

    /// Says where the reader is: line, percentage, and the heading above.
    pub(crate) fn say_position(&mut self) {
        // The heading above, as written so far (edit mode).
        self.refresh_structure(false);
        let Some(pos) = self.reading_position() else {
            return;
        };
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let doc = &s.doc;
        let line = text_util::line_of(doc, pos) + 1;
        let lines = text_util::line_count(doc);
        let pct = text_util::percent(doc, pos);
        let mut msg = self.msg_args(
            "nav-position",
            &args!["line" => line, "lines" => lines, "pct" => pct],
        );
        // A paged document (a PDF): the page first, "Page 12 of 30."
        if let Some(page) = crate::pages::page_at(doc, pos) {
            let page = self.page_words(&page, "pages-position", "pages-position-labelled");
            msg = format!("{page} {msg}");
        }
        if self.settings.speech.verbosity >= Verbosity::Normal
            && let Some((word, words)) = self.word_position(pos)
        {
            msg.push(' ');
            msg.push_str(&self.msg_args(
                "nav-position-word",
                &args![
                    "word" => crate::words::grouped(self.cat(), word),
                    "words" => crate::words::grouped(self.cat(), words)
                ],
            ));
        }
        if let Some(t) = self.table_position(pos) {
            msg.push(' ');
            msg.push_str(&t);
        }
        if self.settings.speech.verbosity >= Verbosity::Normal {
            let heading = navigate(
                doc,
                pos.saturating_add(1),
                Unit::marker(MarkerKind::Heading),
                Direction::Backward,
                NavOptions::default(),
            );
            if let Some(h) = heading {
                let text = Self::marker_content(doc, MarkerKind::Heading, h.range);
                msg.push(' ');
                msg.push_str(&self.msg_args("nav-position-heading", &args!["heading" => text]));
            }
        }
        if self.settings.speech.verbosity >= Verbosity::High {
            msg.push_str(&format!(" {}.", s.title));
            if self.mode != Mode::Browse {
                let mode = crate::words::mode_name(self.cat(), self.mode);
                msg.push(' ');
                msg.push_str(&self.msg_args("nav-position-mode", &args!["mode" => mode]));
            }
        }
        self.tell(&msg);
    }
}

impl App {
    /// "No next heading.", "No previous sentence.": nothing further by
    /// `unit` in `dir`.
    pub(crate) fn no_unit_message(&self, unit: Unit, dir: Direction) -> String {
        let what = unit_name(self.cat(), unit);
        let id = match dir {
            Direction::Forward => "nav-no-next",
            Direction::Backward => "nav-no-previous",
        };
        self.msg_args(id, &args!["what" => what, "unit" => unit_key(unit)])
    }

    /// "End of document." or "Top of document.": a caret or page move
    /// that cannot go further.
    pub(crate) fn edge_message(&self, dir: Direction) -> String {
        self.msg(match dir {
            Direction::Forward => "nav-end-of-document-stop",
            Direction::Backward => "nav-top-of-document-stop",
        })
    }
}

/// True when a caret move from `from` to `to` enters a note's passage (so
/// "has a note" is said once, not on every word inside it).
fn cursor_left_note(app: &App, from: CharPos, to: CharPos) -> bool {
    let (Some(a), Some(b)) = (app.note_here_suffix(from), app.note_here_suffix(to)) else {
        return true;
    };
    a != b
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}
