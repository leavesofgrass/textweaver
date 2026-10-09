//! Find and replace, one match at a time (Alt+F in edit mode).
//!
//! After the text to find and its replacement, each match is selected and
//! said with its line and what it becomes ("Match 2 of 5, line 12: teh
//! becomes the"), and a short list asks what to do: replace this one
//! (`r`), skip it (`s`), replace all the rest (`a`, which says the count
//! and asks once), or switch match case (`c`), whole words (`w`), regular
//! expression (`x`) or across lines (`l`), which counts the matches again.
//! The options are the session's search options
//! ([`crate::SearchOptions`]), shared with Find. The search starts at the
//! caret, goes to the end, then wraps to the top and stops where it
//! started. Escape stops; either way the counts are said ("Replaced 3,
//! skipped 1."). Each replacement is one undo step; "replace all the
//! rest" is one step for all of them.
//!
//! A frontend with its own find and replace panel drives the same loop
//! with [`Command::StartReplace`](crate::Command::StartReplace),
//! [`Command::ReplaceStep`](crate::Command::ReplaceStep) and
//! [`Command::SetSearchOptions`](crate::Command::SetSearchOptions), and
//! shows [`App::replace_preview`].

use textweaver_core::{CharPos, CharRange, Edit, EditOutcome};
use textweaver_editor::Selection;
use textweaver_lexicon::args;
use textweaver_speech::Earcon;

use crate::app::App;
use crate::authoring_state::{AuthoringList, Question};
use crate::command::{Confirm, Effect};
use crate::search_options::{self as so, PatternProblem, SearchOptions};
use crate::text_util;

/// Most characters of the matched text, and of what it becomes, in a
/// match's line: after "Match 2 of 5, line 12: " the change stays short
/// enough to read on one or two lines of a 40-cell display.
const SHOWN: usize = 20;

/// A find and replace in progress.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReplaceSession {
    pub(crate) query: String,
    pub(crate) with: String,
    /// Where the search started; after wrapping it stops here.
    pub(crate) origin: CharPos,
    /// The search has wrapped to the top.
    pub(crate) wrapped: bool,
    /// The match being asked about.
    pub(crate) current: Option<CharRange>,
    pub(crate) replaced: usize,
    pub(crate) skipped: usize,
    /// The current match's number and the total, from the one search each
    /// step makes (Agent P2a: no search per announcement).
    pub(crate) position: Option<(usize, usize)>,
}

/// A step of the replace loop, for a frontend's own buttons
/// ([`Command::ReplaceStep`](crate::Command::ReplaceStep)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReplaceStep {
    /// Replace the current match and go to the next.
    Replace,
    /// Leave the current match and go to the next.
    Skip,
    /// Replace the current match and all the rest, after saying how many
    /// and asking once.
    ReplaceAll,
}

/// One of the search options, as the lists switch them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchOption {
    MatchCase,
    WholeWords,
    Regex,
    AcrossLines,
}

impl SearchOption {
    /// In list order.
    pub(crate) const ALL: [SearchOption; 4] = [
        SearchOption::MatchCase,
        SearchOption::WholeWords,
        SearchOption::Regex,
        SearchOption::AcrossLines,
    ];

    fn get(self, o: SearchOptions) -> bool {
        match self {
            SearchOption::MatchCase => o.match_case,
            SearchOption::WholeWords => o.whole_words,
            SearchOption::Regex => o.regex,
            SearchOption::AcrossLines => o.across_lines,
        }
    }

    fn switched(self, mut o: SearchOptions) -> SearchOptions {
        match self {
            SearchOption::MatchCase => o.match_case = !o.match_case,
            SearchOption::WholeWords => o.whole_words = !o.whole_words,
            SearchOption::Regex => o.regex = !o.regex,
            SearchOption::AcrossLines => o.across_lines = !o.across_lines,
        }
        o
    }

    /// The list item's message ("Match case: { $state }").
    fn item_id(self) -> &'static str {
        match self {
            SearchOption::MatchCase => "replace-item-match-case",
            SearchOption::WholeWords => "replace-item-whole-words",
            SearchOption::Regex => "replace-item-regex",
            SearchOption::AcrossLines => "replace-item-across-lines",
        }
    }

    /// What the replace loop says after switching it, with the count.
    fn now_id(self) -> &'static str {
        match self {
            SearchOption::MatchCase => "replace-match-case-now",
            SearchOption::WholeWords => "replace-whole-words-now",
            SearchOption::Regex => "replace-regex-now",
            SearchOption::AcrossLines => "replace-across-lines-now",
        }
    }

    /// The letter that switches it in the lists.
    pub(crate) fn from_key(c: char) -> Option<SearchOption> {
        match c.to_ascii_lowercase() {
            'c' => Some(SearchOption::MatchCase),
            'w' => Some(SearchOption::WholeWords),
            'x' => Some(SearchOption::Regex),
            'l' => Some(SearchOption::AcrossLines),
            _ => None,
        }
    }
}

impl App {
    /// The search options of Find and Replace.
    pub fn search_options(&self) -> SearchOptions {
        self.search
    }

    /// What the replace loop is asking about, if it runs: "Match 2 of 5,
    /// line 12: teh becomes the". A find and replace panel shows it.
    pub fn replace_preview(&self) -> Option<String> {
        self.authoring
            .replace
            .as_ref()
            .and_then(|r| r.current)
            .map(|_| self.replace_title())
    }

    /// Why `pattern` cannot be searched with the current options, in
    /// words ("Invalid pattern at character 3: unclosed group."), or
    /// `None` when it can: a panel can show it as the pattern is typed.
    pub fn search_pattern_problem(&self, pattern: &str) -> Option<String> {
        so::pattern_problem(pattern, self.search).map(|p| self.pattern_problem_message(&p))
    }

    /// An invalid pattern, in words, with where it fails.
    pub(crate) fn pattern_problem_message(&self, p: &PatternProblem) -> String {
        match p.at {
            Some(at) => self.msg_args(
                "search-invalid-pattern",
                &args!["at" => at, "reason" => p.reason.as_str()],
            ),
            None => self.msg_args(
                "search-invalid-pattern-anywhere",
                &args!["reason" => p.reason.as_str()],
            ),
        }
    }

    /// Says why `pattern` cannot be searched, if it cannot: true when it
    /// was refused.
    pub(crate) fn refuse_bad_pattern(&mut self, pattern: &str) -> bool {
        let Some(p) = so::pattern_problem(pattern, self.search) else {
            return false;
        };
        let msg = self.pattern_problem_message(&p);
        self.error(&msg);
        true
    }

    /// "Options on: match case, regular expression." when any is on.
    pub(crate) fn search_options_said(&self) -> Option<String> {
        let on = self.search.on();
        if on.is_empty() {
            return None;
        }
        let list: Vec<String> = on.iter().map(|id| self.msg(id)).collect();
        Some(self.msg_args("search-options-on", &args!["list" => list.join(", ")]))
    }

    /// Sets the search options without a word (a panel's check boxes say
    /// their own state). A replace loop in progress counts again.
    pub(crate) fn set_search_options(&mut self, opts: SearchOptions) -> Vec<Effect> {
        if self.search == opts {
            return vec![Effect::Redraw];
        }
        self.search = opts;
        match self.authoring.replace.as_ref().and_then(|r| r.current) {
            Some(m) => self.next_replace_match(m.start),
            None => vec![Effect::Redraw],
        }
    }

    /// Starts replacing `query` with `with`, one match at a time.
    pub(crate) fn start_replace(&mut self, query: String, with: String) -> Vec<Effect> {
        let origin = self
            .edit
            .as_ref()
            .and_then(|e| e.session.editor())
            .map_or(CharPos::ZERO, |ed| ed.selection().range().start);
        self.authoring.replace = Some(ReplaceSession {
            query,
            with,
            origin,
            wrapped: false,
            current: None,
            replaced: 0,
            skipped: 0,
            position: None,
        });
        self.next_replace_match(origin)
    }

    /// A panel's Replace: checks the pattern, then starts the loop at the
    /// caret ([`Command::StartReplace`](crate::Command::StartReplace)).
    pub(crate) fn start_replace_command(&mut self, find: String, with: String) -> Vec<Effect> {
        if self.edit.is_none() {
            return self.not_editing("replace-text");
        }
        self.authoring.replace = None;
        if find.is_empty() || self.refuse_bad_pattern(&find) {
            return vec![Effect::Redraw];
        }
        self.remember_answer(crate::command::PromptPurpose::ReplaceFind, &find);
        self.remember_answer(crate::command::PromptPurpose::ReplaceWith, &with);
        self.start_replace(find, with)
    }

    /// A panel's step ([`Command::ReplaceStep`](crate::Command::ReplaceStep)).
    pub(crate) fn replace_step(&mut self, step: ReplaceStep) -> Vec<Effect> {
        if self.authoring.replace.is_none() {
            return vec![Effect::Redraw];
        }
        self.replace_choice(match step {
            ReplaceStep::Replace => 0,
            ReplaceStep::Skip => 1,
            ReplaceStep::ReplaceAll => 2,
        })
    }

    /// Every match with the session's options.
    fn replace_hits(&self) -> Vec<CharRange> {
        let (Some(r), Some(ed)) = (
            self.authoring.replace.as_ref(),
            self.edit.as_ref().and_then(|e| e.session.editor()),
        ) else {
            return Vec::new();
        };
        so::all_matches(ed.text(), &r.query, self.search)
    }

    /// How many matches there are, counted without keeping them.
    fn replace_count(&self) -> usize {
        let (Some(r), Some(ed)) = (
            self.authoring.replace.as_ref(),
            self.edit.as_ref().and_then(|e| e.session.editor()),
        ) else {
            return 0;
        };
        so::count(ed.text(), &r.query, self.search)
    }

    /// The next match at or after `from` (wrapping once, stopping at the
    /// origin), selected and asked about; the summary when none is left.
    fn next_replace_match(&mut self, from: CharPos) -> Vec<Effect> {
        // One streamed search: the match to ask about (and the first one
        // before the origin, for wrapping), with its number and the total.
        let (Some(r), Some(ed)) = (
            self.authoring.replace.as_ref(),
            self.edit.as_ref().and_then(|e| e.session.editor()),
        ) else {
            return vec![Effect::Redraw];
        };
        let (wrapped, origin) = (r.wrapped, r.origin);
        let mut total = 0usize;
        let mut ahead: Option<(CharRange, usize)> = None;
        let mut wrap_to: Option<(CharRange, usize)> = None;
        so::each_match(ed.text(), &r.query, self.search, |h| {
            total += 1;
            let next = if wrapped {
                h.start >= from && h.start < origin
            } else {
                h.start >= from
            };
            if next && ahead.is_none() {
                ahead = Some((h, total));
            }
            if h.start < origin && wrap_to.is_none() {
                wrap_to = Some((h, total));
            }
            std::ops::ControlFlow::Continue(())
        });
        let Some(r) = self.authoring.replace.as_mut() else {
            return vec![Effect::Redraw];
        };
        let mut found = ahead;
        if found.is_none() && !r.wrapped {
            r.wrapped = true;
            found = wrap_to;
        }
        r.current = found.map(|(h, _)| h);
        r.position = found.map(|(_, n)| (n, total));
        let Some(m) = found.map(|(h, _)| h) else {
            return self.replace_done();
        };
        if let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) {
            ed.set_selection(Selection::new(m.start, m.end));
        }
        self.after_edit(&ropey::Rope::new(), &[]);
        let title = self.replace_title();
        let context = self.session.as_ref().map_or_else(String::new, |s| {
            let line = text_util::line_range(&s.doc, text_util::line_of(&s.doc, m.start));
            crate::lists::one_line(&s.doc.slice(line), 80)
        });
        let msg = self.msg_args(
            "replace-match-question",
            &args!["title" => title, "context" => context],
        );
        self.tell(&msg);
        self.show_authoring_list(AuthoringList::Replace)
    }

    /// What the current match becomes, from the edited text.
    fn current_change(&self) -> Option<(String, String)> {
        let r = self.authoring.replace.as_ref()?;
        let m = r.current?;
        let ed = self.edit.as_ref()?.session.editor()?;
        let text = ed.text();
        let m = m.clamp_to(text.len_chars());
        let found = text.slice(m.start.0..m.end.0).to_string();
        let result = so::replacement(text, m, &r.query, &r.with, self.search);
        Some((found, result))
    }

    /// Text shown in a match's line: whitespace collapsed, at most
    /// [`SHOWN`] characters, "blank" when nothing is left.
    fn shown(&self, text: &str) -> String {
        let s = crate::lists::one_line(text, SHOWN);
        if s.is_empty() {
            self.msg("nav-blank")
        } else {
            s
        }
    }

    /// "Match 2 of 5, line 12: teh becomes the" (or "… teh is
    /// removed" when the replacement is empty).
    pub(crate) fn replace_title(&self) -> String {
        let (Some(r), Some(s)) = (self.authoring.replace.as_ref(), self.session.as_ref()) else {
            return self.msg("replace-title");
        };
        let Some(m) = r.current else {
            return self.msg("replace-title");
        };
        let (i, total) = r.position.unwrap_or_else(|| {
            let hits = self.replace_hits();
            let i = hits.iter().position(|h| *h == m).map_or(0, |i| i + 1);
            (i, hits.len())
        });
        let line_no = text_util::line_of(&s.doc, m.start);
        let (found, result) = self.current_change().unwrap_or_default();
        let found = self.shown(&found);
        if result.is_empty() {
            return self.msg_args(
                "replace-match-title-removed",
                &args!["n" => i, "total" => total, "line" => line_no + 1, "found" => found],
            );
        }
        let result = self.shown(&result);
        self.msg_args(
            "replace-match-title",
            &args![
                "n" => i,
                "total" => total,
                "line" => line_no + 1,
                "found" => found,
                "result" => result
            ],
        )
    }

    /// The choices, with the options' states.
    pub(crate) fn replace_items(&self) -> Vec<String> {
        let c = self.cat();
        let mut items = vec![
            c.tr("replace-item-this"),
            c.tr("replace-item-skip"),
            c.tr("replace-item-rest"),
        ];
        items.extend(self.search_option_items());
        items
    }

    /// The search options as list items ("Regular expression: off").
    pub(crate) fn search_option_items(&self) -> Vec<String> {
        let c = self.cat();
        SearchOption::ALL
            .iter()
            .map(|o| {
                c.fmt(
                    o.item_id(),
                    &args!["state" => crate::words::on_off(c, o.get(self.search))],
                )
            })
            .collect()
    }

    /// A choice from the replace list.
    pub(crate) fn replace_choice(&mut self, n: usize) -> Vec<Effect> {
        let Some(r) = self.authoring.replace.clone() else {
            return vec![Effect::Redraw];
        };
        let Some(m) = r.current else {
            return self.replace_done();
        };
        match n {
            0 => {
                let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) else {
                    return vec![Effect::Redraw];
                };
                let before = ed.text().clone();
                let with = so::replacement(&before, m, &r.query, &r.with, self.search);
                match ed.apply(Edit::replace(m, with.as_str())) {
                    Ok(o) => {
                        let end = o.inserted.end;
                        self.shift_replace_origin(&[o]);
                        self.after_edit(&before, &[o]);
                        if let Some(r) = self.authoring.replace.as_mut() {
                            r.replaced += 1;
                        }
                        self.next_replace_match(end)
                    }
                    Err(e) => {
                        let msg = self.msg_args("replace-failed", &args!["error" => e.to_string()]);
                        self.error(&msg);
                        vec![Effect::Redraw]
                    }
                }
            }
            1 => {
                if let Some(r) = self.authoring.replace.as_mut() {
                    r.skipped += 1;
                }
                self.next_replace_match(m.end)
            }
            2 => self.ask_replace_rest(),
            _ => match SearchOption::ALL.get(n - 3) {
                Some(&o) => self.replace_switch(o, m.start),
                None => vec![Effect::Redraw],
            },
        }
    }

    /// Switches an option in the replace loop, says it with the new count,
    /// and asks about the match at or after `from`. A pattern that is not
    /// a valid regular expression keeps the option off, and says why.
    fn replace_switch(&mut self, o: SearchOption, from: CharPos) -> Vec<Effect> {
        let next = o.switched(self.search);
        let query = self
            .authoring
            .replace
            .as_ref()
            .map(|r| r.query.clone())
            .unwrap_or_default();
        if let Some(p) = so::pattern_problem(&query, next) {
            let problem = self.pattern_problem_message(&p);
            let msg = self.msg_args("replace-option-refused", &args!["problem" => problem]);
            self.error(&msg);
            return self.show_authoring_list(AuthoringList::Replace);
        }
        self.search = next;
        let hits = self.replace_count();
        let state = crate::words::on_off(self.cat(), o.get(next));
        let msg = self.msg_args(o.now_id(), &args!["state" => state, "n" => hits]);
        self.tell(&msg);
        self.next_replace_match(from)
    }

    /// The current match and the rest up to the origin: what "replace all
    /// the rest" replaces.
    fn replace_targets(&self) -> Vec<CharRange> {
        let Some(r) = self.authoring.replace.as_ref() else {
            return Vec::new();
        };
        let Some(m) = r.current else {
            return Vec::new();
        };
        self.replace_hits()
            .into_iter()
            .filter(|h| {
                if r.wrapped {
                    h.start >= m.start && h.start < r.origin
                } else {
                    h.start >= m.start || h.start < r.origin
                }
            })
            .collect()
    }

    /// `a`: says how many would be replaced, and asks once.
    fn ask_replace_rest(&mut self) -> Vec<Effect> {
        let n = self.replace_targets().len();
        let question = self.msg_args("replace-all-question", &args!["n" => n]);
        self.authoring.question = Some(Question::ReplaceAll(question.clone()));
        self.ask(&question);
        vec![Effect::Redraw]
    }

    /// The answer to "Replace all 4 remaining matches? y or n".
    pub(crate) fn confirm_replace_all(&mut self, question: String, answer: Confirm) -> Vec<Effect> {
        match answer {
            Confirm::Yes => {
                self.authoring.question = None;
                self.replace_rest()
            }
            Confirm::No => {
                self.authoring.question = None;
                let msg = self.msg("replace-all-declined");
                self.note(&msg);
                match self.authoring.replace.as_ref().and_then(|r| r.current) {
                    Some(m) => self.next_replace_match(m.start),
                    None => self.replace_done(),
                }
            }
            Confirm::Repeat => {
                self.ask(&question);
                vec![Effect::Redraw]
            }
        }
    }

    /// Keeps the origin where it was in the text after an edit before it.
    fn shift_replace_origin(&mut self, outcomes: &[EditOutcome]) {
        if let Some(r) = self.authoring.replace.as_mut() {
            for o in outcomes {
                r.origin = o.map_pos(r.origin, textweaver_core::Bias::Before);
            }
        }
    }

    /// Replaces the current match and every one after it (up to the
    /// origin), as one undo step.
    fn replace_rest(&mut self) -> Vec<Effect> {
        let Some(r) = self.authoring.replace.clone() else {
            return vec![Effect::Redraw];
        };
        if r.current.is_none() {
            return self.replace_done();
        }
        let mut targets = self.replace_targets();
        let opts = self.search;
        let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) else {
            return vec![Effect::Redraw];
        };
        let before = ed.text().clone();
        // Back to front, so each range is still valid when its turn comes;
        // each replacement is made from the text before any of them.
        targets.sort_by_key(|h| std::cmp::Reverse(h.start));
        let edits: Vec<Edit> = targets
            .iter()
            .map(|h| Edit::replace(*h, so::replacement(&before, *h, &r.query, &r.with, opts)))
            .collect();
        let outcomes: Vec<EditOutcome> = edits.iter().map(Edit::outcome).collect();
        match ed.apply_group(edits) {
            Ok(_) => {
                self.after_edit(&before, &outcomes);
                if let Some(r) = self.authoring.replace.as_mut() {
                    r.replaced += targets.len();
                    r.current = None;
                }
                self.replace_done()
            }
            Err(e) => {
                let msg = self.msg_args("replace-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
                vec![Effect::Redraw]
            }
        }
    }

    /// No match left: says what was done.
    fn replace_done(&mut self) -> Vec<Effect> {
        let Some(r) = self.authoring.replace.take() else {
            return vec![Effect::Redraw];
        };
        self.list = None;
        let msg = match (r.replaced, r.skipped) {
            (0, 0) => self.msg_args("common-no-matches", &args!["query" => r.query.as_str()]),
            (n, 0) => self.msg_args("replace-replaced", &args!["n" => n]),
            (n, k) => self.msg_args("replace-replaced-skipped", &args!["n" => n, "skipped" => k]),
        };
        if r.replaced == 0 && r.skipped == 0 {
            self.speech.earcon(Earcon::Error);
        }
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    /// Escape in the replace list.
    pub(crate) fn replace_stopped(&mut self) {
        if let Some(r) = self.authoring.replace.take() {
            let msg = self.msg_args(
                "replace-stopped",
                &args!["n" => r.replaced, "skipped" => r.skipped],
            );
            self.tell(&msg);
        }
    }

    /// The search options list (the Find menu and the palette): Enter or
    /// the option's letter switches it, and the list stays open.
    pub(crate) fn search_options_list(&mut self) -> Vec<Effect> {
        self.show_authoring_list(AuthoringList::SearchOptions)
    }

    /// Enter on item `n` of the search options list.
    pub(crate) fn search_option_chosen(&mut self, n: usize) -> Vec<Effect> {
        let Some(&o) = SearchOption::ALL.get(n) else {
            return vec![Effect::Redraw];
        };
        self.search = o.switched(self.search);
        if let Some(said) = self.search_option_items().get(n) {
            let said = said.clone();
            self.tell(&said);
        }
        // The focus stays on the option switched, also by its letter.
        self.pending_list_focus = Some(n);
        self.show_authoring_list(AuthoringList::SearchOptions)
    }
}
