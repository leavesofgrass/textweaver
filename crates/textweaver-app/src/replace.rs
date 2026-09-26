//! Find and replace, one match at a time (Alt+F in edit mode).
//!
//! After the text to find and its replacement, each match is selected and
//! a short list asks what to do: replace this one (`r`), skip it (`s`),
//! replace all the rest (`a`), or switch match case (`c`) or whole words
//! (`w`), which counts the matches again. The search starts at the caret,
//! goes to the end, then wraps to the top and stops where it started.
//! Escape stops; either way the counts are said ("Replaced 3, skipped 1.").
//! Each replacement is one undo step; "replace all the rest" is one step
//! for all of them.

use textweaver_core::{CharPos, CharRange, Edit, EditOutcome};
use textweaver_editor::{FindOptions, Selection};
use textweaver_speech::Earcon;

use crate::app::App;
use crate::authoring_state::AuthoringList;
use crate::command::Effect;
use crate::text_util;

/// A find and replace in progress.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReplaceSession {
    pub(crate) query: String,
    pub(crate) with: String,
    pub(crate) options: FindOptions,
    /// Where the search started; after wrapping it stops here.
    pub(crate) origin: CharPos,
    /// The search has wrapped to the top.
    pub(crate) wrapped: bool,
    /// The match being asked about.
    pub(crate) current: Option<CharRange>,
    pub(crate) replaced: usize,
    pub(crate) skipped: usize,
}

fn on_off(b: bool) -> &'static str {
    if b { "on" } else { "off" }
}

impl App {
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
            options: FindOptions::default(),
            origin,
            wrapped: false,
            current: None,
            replaced: 0,
            skipped: 0,
        });
        self.next_replace_match(origin)
    }

    /// Every match with the session's options.
    fn replace_hits(&self) -> Vec<CharRange> {
        let (Some(r), Some(ed)) = (
            self.authoring.replace.as_ref(),
            self.edit.as_ref().and_then(|e| e.session.editor()),
        ) else {
            return Vec::new();
        };
        textweaver_editor::find::find_all(ed.text(), &r.query, r.options)
    }

    /// The next match at or after `from` (wrapping once, stopping at the
    /// origin), selected and asked about; the summary when none is left.
    fn next_replace_match(&mut self, from: CharPos) -> Vec<Effect> {
        let hits = self.replace_hits();
        let Some(r) = self.authoring.replace.as_mut() else {
            return vec![Effect::Redraw];
        };
        let mut found = if r.wrapped {
            hits.iter().find(|h| h.start >= from && h.start < r.origin)
        } else {
            hits.iter().find(|h| h.start >= from)
        }
        .copied();
        if found.is_none() && !r.wrapped {
            r.wrapped = true;
            found = hits.iter().find(|h| h.start < r.origin).copied();
        }
        r.current = found;
        let Some(m) = found else {
            return self.replace_done();
        };
        if let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) {
            ed.set_selection(Selection::new(m.start, m.end));
        }
        self.after_edit(&ropey::Rope::new(), &[]);
        let title = self.replace_title();
        self.tell(&format!(
            "{title}. Press r to replace, s to skip, a to replace all, Escape to stop."
        ));
        self.show_authoring_list(AuthoringList::Replace)
    }

    /// "Match 2 of 5, line 12: … the context …".
    pub(crate) fn replace_title(&self) -> String {
        let hits = self.replace_hits();
        let (Some(r), Some(s)) = (self.authoring.replace.as_ref(), self.session.as_ref()) else {
            return "Replace".to_owned();
        };
        let Some(m) = r.current else {
            return "Replace".to_owned();
        };
        let i = hits.iter().position(|h| *h == m).map_or(0, |i| i + 1);
        let line_no = text_util::line_of(&s.doc, m.start);
        let line = text_util::line_range(&s.doc, line_no);
        let context = crate::lists::one_line(&s.doc.slice(line), 80);
        format!(
            "Match {i} of {}, line {}: {context}",
            hits.len(),
            line_no + 1
        )
    }

    /// The choices, with the options' states.
    pub(crate) fn replace_items(&self) -> Vec<String> {
        let (case, whole) = self.authoring.replace.as_ref().map_or((false, false), |r| {
            (r.options.case_sensitive, r.options.whole_word)
        });
        vec![
            "Replace this one".to_owned(),
            "Skip this one".to_owned(),
            "Replace all the rest".to_owned(),
            format!("Match case: {}", on_off(case)),
            format!("Whole words only: {}", on_off(whole)),
        ]
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
                match ed.apply(Edit::replace(m, r.with.as_str())) {
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
                        self.error(&format!("Could not replace: {e}"));
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
            2 => self.replace_rest(),
            3 | 4 => {
                if let Some(r) = self.authoring.replace.as_mut() {
                    if n == 3 {
                        r.options.case_sensitive = !r.options.case_sensitive;
                    } else {
                        r.options.whole_word = !r.options.whole_word;
                    }
                }
                let hits = self.replace_hits().len();
                let (case, whole) = self.authoring.replace.as_ref().map_or((false, false), |r| {
                    (r.options.case_sensitive, r.options.whole_word)
                });
                let what = if n == 3 {
                    format!("Match case {}", on_off(case))
                } else {
                    format!("Whole words only {}", on_off(whole))
                };
                self.tell(&format!(
                    "{what}. {hits} {}.",
                    crate::lists::plural(hits, "match", "matches")
                ));
                self.next_replace_match(m.start)
            }
            _ => vec![Effect::Redraw],
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
        let Some(m) = r.current else {
            return self.replace_done();
        };
        let hits = self.replace_hits();
        let targets: Vec<CharRange> = hits
            .into_iter()
            .filter(|h| {
                if r.wrapped {
                    h.start >= m.start && h.start < r.origin
                } else {
                    h.start >= m.start || h.start < r.origin
                }
            })
            .collect();
        let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) else {
            return vec![Effect::Redraw];
        };
        let before = ed.text().clone();
        // Back to front, so each range is still valid when its turn comes.
        let mut sorted = targets.clone();
        sorted.sort_by_key(|h| std::cmp::Reverse(h.start));
        let edits: Vec<Edit> = sorted
            .iter()
            .map(|h| Edit::replace(*h, r.with.as_str()))
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
                self.error(&format!("Could not replace: {e}"));
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
            (0, 0) => format!("No matches for {}.", r.query),
            (n, 0) => format!(
                "Replaced {n} {}.",
                crate::lists::plural(n, "match", "matches")
            ),
            (n, k) => format!("Replaced {n}, skipped {k}."),
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
            self.tell(&format!(
                "Stopped. Replaced {}, skipped {}.",
                r.replaced, r.skipped
            ));
        }
    }
}
