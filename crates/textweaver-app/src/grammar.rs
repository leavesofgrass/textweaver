//! Grammar checking with Harper (`harper-core`, Apache-2.0, offline;
//! Agent W4g, ADR-0032), behind the app's `grammar` feature.
//!
//! - Ctrl+F7 and Ctrl+Shift+F7 move to the next and previous grammar
//!   problem, select it, and say it: "Grammar: Use an before a vowel. the
//!   words: a apple. Fix: an." When Harper has a fix, Alt+J lists the
//!   fixes (and "Leave it as it is"); in edit mode Enter makes the change
//!   as one undo step.
//! - Edit mode on Markdown checks the source's prose, as Harper's Markdown
//!   parser sees it (code, links' addresses and math are left out); reading
//!   checks the document's text.
//! - Spelling is left to textweaver's own checker (Alt+M), so Harper's
//!   spelling problems are not reported twice.
//!
//! The dictionary and the rules are loaded on first use and kept.

use std::sync::Arc;

use harper_core::linting::{LintGroup, LintKind, Linter, Suggestion};
use harper_core::spell::FstDictionary;
use harper_core::{Dialect, Document};
use textweaver_core::{CharRange, Direction};
use textweaver_editor::Selection;
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;
use textweaver_speech::Earcon;

use crate::app::App;
use crate::authoring_state::AuthoringList;
use crate::command::Effect;
use crate::text_util;

/// One grammar problem.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GrammarProblem {
    /// Where, in chars of the checked text.
    pub(crate) range: CharRange,
    /// Harper's message.
    pub(crate) message: String,
    /// The words the problem is about.
    pub(crate) words: String,
    /// Replacements for [`GrammarProblem::range`] that fix it, in
    /// Harper's order (empty text removes the words).
    pub(crate) fixes: Vec<String>,
}

/// Harper's dictionary and rules, kept between checks.
pub(crate) struct GrammarChecker {
    dictionary: Arc<FstDictionary>,
    rules: LintGroup,
    /// The last text checked (its hash, and whether as Markdown) and its
    /// problems: stepping through problems checks the text once.
    last: Option<(u64, bool, Vec<GrammarProblem>)>,
}

impl std::fmt::Debug for GrammarChecker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GrammarChecker")
    }
}

impl GrammarChecker {
    /// The curated dictionary and rules for American English.
    pub(crate) fn new() -> Self {
        let dictionary = FstDictionary::curated();
        let rules = LintGroup::new_curated(dictionary.clone(), Dialect::American);
        GrammarChecker {
            dictionary,
            rules,
            last: None,
        }
    }

    /// Every grammar problem in `text` (Markdown source when `markdown`),
    /// in order, spelling left out.
    pub(crate) fn check(&mut self, text: &str, markdown: bool) -> Vec<GrammarProblem> {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut h);
        let key = h.finish();
        if let Some((k, m, found)) = &self.last
            && *k == key
            && *m == markdown
        {
            return found.clone();
        }
        let found = self.check_uncached(text, markdown);
        self.last = Some((key, markdown, found.clone()));
        found
    }

    /// [`GrammarChecker::check`] without the cache.
    fn check_uncached(&mut self, text: &str, markdown: bool) -> Vec<GrammarProblem> {
        let doc = if markdown {
            Document::new_markdown_default(text, &*self.dictionary)
        } else {
            Document::new_plain_english(text, &*self.dictionary)
        };
        let mut lints = self.rules.lint(&doc);
        harper_core::remove_overlaps(&mut lints);
        let chars = doc.get_source();
        let mut out: Vec<GrammarProblem> = lints
            .into_iter()
            .filter(|l| l.lint_kind != LintKind::Spelling)
            .filter(|l| l.span.start < l.span.end && l.span.end <= chars.len())
            .map(|l| {
                let words: String = chars[l.span.start..l.span.end].iter().collect();
                let fixes = l
                    .suggestions
                    .iter()
                    .map(|s| match s {
                        Suggestion::ReplaceWith(c) => c.iter().collect(),
                        Suggestion::InsertAfter(c) => {
                            let mut w = words.clone();
                            w.extend(c.iter());
                            w
                        }
                        Suggestion::Remove => String::new(),
                    })
                    .filter(|f: &String| *f != words)
                    .fold(Vec::<String>::new(), |mut v, f| {
                        if !v.contains(&f) {
                            v.push(f);
                        }
                        v
                    });
                GrammarProblem {
                    range: CharRange::new(l.span.start, l.span.end),
                    message: l.message.trim().to_owned(),
                    words,
                    fixes,
                }
            })
            .collect();
        out.sort_by_key(|p| (p.range.start, p.range.end));
        out
    }
}

/// "grammar: Use an before a vowel. the words: a apple." with the first
/// fix after it when there is one.
pub(crate) fn describe(c: &Catalog, p: &GrammarProblem) -> String {
    let mut msg = p.message.trim_end_matches('.').to_owned();
    msg.push('.');
    let words = p.words.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = c.fmt("grammar-said", &args!["message" => msg, "words" => words]);
    if let Some(f) = p.fixes.first() {
        out.push(' ');
        if f.trim().is_empty() {
            out.push_str(&c.tr("grammar-fix-remove"));
        } else {
            out.push_str(&c.fmt("grammar-fix", &args!["fix" => f]));
        }
    }
    out
}

impl App {
    /// Every grammar problem of the document: the Markdown source in edit
    /// mode on Markdown, else the text.
    fn grammar_problems(&mut self) -> Vec<GrammarProblem> {
        let markdown = self.edit.is_some() && self.authoring.structure.markdown;
        let Some(text) = self.session.as_ref().map(|s| s.doc.text().to_string()) else {
            return Vec::new();
        };
        let checker = self
            .authoring
            .grammar
            .get_or_insert_with(GrammarChecker::new);
        checker.check(&text, markdown)
    }

    /// Ctrl+F7 and Ctrl+Shift+F7: the next or previous grammar problem;
    /// selects it and says it.
    pub(crate) fn grammar_step(&mut self, dir: Direction) {
        let Some(pos) = self.session.as_ref().map(|s| s.cursor) else {
            return;
        };
        let selected = match self.edit.as_ref().and_then(|e| e.session.editor()) {
            Some(ed) => Some(ed.selection().range()),
            None => self.session.as_ref().and_then(|s| s.selection),
        };
        let from = selected.map_or(pos, |r| r.start.min(pos));
        let all = self.grammar_problems();
        let found = match dir {
            Direction::Forward => all.iter().find(|p| {
                p.range.start > pos || (p.range.start == pos && selected != Some(p.range))
            }),
            Direction::Backward => all.iter().rev().find(|p| p.range.start < from),
        }
        .cloned();
        let Some(p) = found else {
            self.speech.earcon(Earcon::Boundary);
            let msg = if all.is_empty() {
                self.msg("grammar-none")
            } else {
                let n = all.len();
                self.msg_args(
                    if dir == Direction::Forward {
                        "grammar-no-more"
                    } else {
                        "grammar-no-earlier"
                    },
                    &args!["n" => n, "count" => crate::words::grouped(self.cat(), n)],
                )
            };
            self.tell(&msg);
            return;
        };
        self.stop_speech();
        if let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) {
            ed.set_selection(Selection::new(p.range.start, p.range.end));
            self.after_edit(&ropey::Rope::new(), &[]);
        } else if let Some(s) = self.session.as_mut() {
            s.cursor = p.range.start;
            s.selection = Some(p.range);
            s.selection_anchor = Some(p.range.start);
        }
        self.scroll_to_cursor();
        let mut msg = describe(self.cat(), &p);
        if !p.fixes.is_empty() {
            let k = self.keys(textweaver_keymap::ActionId::SpellingSuggestions);
            msg.push(' ');
            msg.push_str(&self.msg_args("grammar-lists-fixes", &args!["key" => k]));
        }
        if self.settings.speech.verbosity >= textweaver_a11y::Verbosity::High {
            let line = self
                .session
                .as_ref()
                .map_or(0, |s| text_util::line_of(&s.doc, p.range.start) + 1);
            msg.push(' ');
            msg.push_str(&self.msg_args("grammar-line", &args!["line" => line]));
        }
        self.tell(&msg);
    }

    /// The grammar problem at the cursor or selection, for Alt+J when the
    /// word there is not misspelled.
    pub(crate) fn grammar_here(&mut self) -> Option<GrammarProblem> {
        let s = self.session.as_ref()?;
        let pos = s.cursor;
        let selected = match self.edit.as_ref().and_then(|e| e.session.editor()) {
            Some(ed) => Some(ed.selection().range()),
            None => s.selection,
        };
        self.grammar_problems()
            .into_iter()
            .find(|p| selected == Some(p.range) || (p.range.start <= pos && pos <= p.range.end))
    }

    /// Alt+J on a grammar problem: its fixes, then "Leave it as it is".
    pub(crate) fn grammar_fixes(&mut self, p: GrammarProblem) -> Vec<Effect> {
        if p.fixes.is_empty() {
            let described = describe(self.cat(), &p);
            let msg = self.msg_args(
                "grammar-no-fix",
                &args!["described" => described.trim_end_matches('.')],
            );
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let n = p.fixes.len();
        let id = if self.edit.is_some() {
            "grammar-fixes-edit"
        } else {
            "grammar-fixes"
        };
        let msg = self.msg_args(
            id,
            &args![
                "words" => p.words.split_whitespace().collect::<Vec<_>>().join(" "),
                "n" => n
            ],
        );
        self.tell(&msg);
        self.show_authoring_list(AuthoringList::Grammar {
            words: p.words,
            range: p.range,
            fixes: p.fixes,
        })
    }

    /// A choice from the grammar fixes list: index `n` of `fixes`, or
    /// past them, "Leave it as it is".
    pub(crate) fn grammar_fix_chosen(
        &mut self,
        range: CharRange,
        fixes: &[String],
        n: usize,
    ) -> Vec<Effect> {
        let Some(fix) = fixes.get(n) else {
            let msg = self.msg("grammar-left-as-is");
            self.note(&msg);
            return vec![Effect::Redraw];
        };
        if self.edit.is_none() {
            let k = self.keys(textweaver_keymap::ActionId::ToggleEditMode);
            let msg = self.msg_args(
                "grammar-fix-not-editing",
                &args![
                    "fix" => crate::authoring::grammar_fix_label(self.cat(), fix),
                    "key" => k
                ],
            );
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) else {
            return vec![Effect::Redraw];
        };
        let before = ed.text().clone();
        ed.set_selection(Selection::new(range.start, range.end));
        // An empty fix removes the words; either way one undo step.
        match ed.insert_text(fix) {
            Ok(o) => {
                self.after_edit(&before, &[o]);
                let msg = if fix.is_empty() {
                    self.msg("grammar-removed")
                } else {
                    self.msg_args("grammar-changed", &args!["fix" => fix.as_str()])
                };
                self.tell(&msg);
            }
            Err(e) => {
                let msg = self.msg_args("grammar-change-failed", &args!["error" => e.to_string()]);
                self.error(&msg);
            }
        }
        vec![Effect::Redraw]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harper_finds_an_article_problem_with_a_fix() {
        let mut g = GrammarChecker::new();
        let found = g.check("She ate a apple yesterday.", false);
        let p = found
            .iter()
            .find(|p| p.words.contains("a apple") || p.words == "a")
            .unwrap_or_else(|| panic!("{found:?}"));
        assert!(!p.message.is_empty());
        assert!(p.fixes.iter().any(|f| f.contains("an")), "{p:?}");
        let said = describe(&Catalog::english(), p);
        assert!(said.starts_with("Grammar: "), "{said}");
        assert!(said.contains("The words: "), "{said}");
        assert!(said.contains(" Fix: "), "{said}");
    }

    #[test]
    fn spelling_is_left_to_textweavers_checker() {
        let mut g = GrammarChecker::new();
        let found = g.check("This sentance is fine otherwise.", false);
        assert!(
            found.iter().all(|p| !p.words.contains("sentance")),
            "{found:?}"
        );
    }

    #[test]
    fn markdown_code_is_not_checked() {
        let mut g = GrammarChecker::new();
        let found = g.check("Some text.\n\n```\na apple a apple\n```\n", true);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn fix_labels() {
        use crate::authoring::grammar_fix_label;
        let c = Catalog::english();
        assert_eq!(grammar_fix_label(&c, ""), "Remove the words");
        assert_eq!(grammar_fix_label(&c, "an"), "an");
    }
}
