//! Prompts with hidden answers, on the shared list model (B1-s1): the
//! self-test from the study sheet uses it, and the cards session (B1-f1)
//! is meant to reuse it.
//!
//! The API, for a new kind of prompt list:
//!
//! - Build a [`RevealList`] from [`RevealItem`]s (a prompt and its
//!   answer) with a title, and show it with [`App::show_reveal_list`],
//!   which says the list's introduction first.
//! - Each row says its prompt; the answer stays hidden until Enter
//!   ([`App::choose_reveal`]) reveals it: the answer is said, the row then
//!   shows it, and the list stays open on the same row. Enter on a row
//!   already shown says the answer again.
//! - Space ([`App::reveal_answer_aloud`]) records a spoken answer through
//!   dictation, when the build has it; when recording ends the words are
//!   read back ("You said: ..."), before the reveal. The machine never
//!   grades the answer: the reader compares.
//!
//! The list is `ListKind::Reveal` in the app; the frontends show it like
//! any list.

use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;

use crate::app::{App, ListKind};
use crate::command::Effect;

/// One prompt and its hidden answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RevealItem {
    /// What is said first: the question.
    pub(crate) prompt: String,
    /// What Enter reveals.
    pub(crate) answer: String,
}

/// A list of prompts whose answers are revealed one by one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RevealList {
    /// The list's title.
    pub(crate) title: String,
    items: Vec<RevealItem>,
    /// Which answers are shown.
    shown: Vec<bool>,
    /// The row a spoken answer is being recorded for.
    pub(crate) listening: Option<usize>,
}

impl RevealList {
    /// A list titled `title`, every answer hidden.
    pub(crate) fn new(title: impl Into<String>, items: Vec<RevealItem>) -> Self {
        let shown = vec![false; items.len()];
        Self {
            title: title.into(),
            items,
            shown,
            listening: None,
        }
    }

    /// How many prompts the list has.
    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    /// The rows as shown: the prompt, and the answer once revealed.
    pub(crate) fn rows(&self, c: &Catalog) -> Vec<String> {
        self.items
            .iter()
            .zip(&self.shown)
            .map(|(item, &shown)| {
                if shown {
                    c.fmt(
                        "reveal-row-shown",
                        &args!["prompt" => item.prompt.as_str(), "answer" => item.answer.as_str()],
                    )
                } else {
                    item.prompt.clone()
                }
            })
            .collect()
    }

    /// Reveals row `n`'s answer and returns it.
    pub(crate) fn reveal(&mut self, n: usize) -> Option<&str> {
        let shown = self.shown.get_mut(n)?;
        *shown = true;
        self.items.get(n).map(|i| i.answer.as_str())
    }
}

impl App {
    /// Shows `list` after saying `intro`, focused on its first row.
    pub(crate) fn show_reveal_list(&mut self, intro: &str, list: RevealList) -> Vec<Effect> {
        self.tell(intro);
        self.reshow_reveal_list(list, 0)
    }

    /// Shows `list` again, focused on row `n`, without saying the row
    /// again (what changed was just said).
    fn reshow_reveal_list(&mut self, list: RevealList, n: usize) -> Vec<Effect> {
        let title = list.title.clone();
        let items = list.rows(self.cat());
        self.pending_list_focus = Some(n);
        self.list = Some(ListKind::Reveal(list));
        vec![Effect::ShowList { title, items }]
    }

    /// Enter on row `n`: finishes a spoken answer under way (it is read
    /// back first), then reveals and says the answer. The list stays.
    pub(crate) fn choose_reveal(&mut self, mut list: RevealList, n: usize) -> Vec<Effect> {
        if list.listening.is_some() {
            // The read-back comes before the reveal.
            self.list = Some(ListKind::Reveal(list));
            self.dictation_finish();
            match self.list.take() {
                Some(ListKind::Reveal(l)) => list = l,
                other => {
                    self.list = other;
                    return vec![Effect::Redraw];
                }
            }
        }
        if let Some(answer) = list.reveal(n).map(str::to_owned) {
            let msg = self.msg_args("reveal-answer", &args!["answer" => answer]);
            self.tell(&msg);
        }
        self.list_reshow_quiet = true;
        self.reshow_reveal_list(list, n)
    }

    /// Space on row `n`: starts recording a spoken answer for it, or stops
    /// the recording under way. The list stays.
    pub(crate) fn reveal_answer_aloud(&mut self, mut list: RevealList, n: usize) -> Vec<Effect> {
        let mut effects = if list.listening.is_some() {
            self.answer_aloud_stop()
        } else {
            list.listening = Some(n);
            self.answer_aloud_start()
        };
        // A question (download the model?) replaces the list.
        if effects
            .iter()
            .any(|e| matches!(e, Effect::Prompt { .. } | Effect::ShowList { .. }))
            || self.confirmation_pending()
        {
            return effects;
        }
        if !self.answer_aloud_active() {
            list.listening = None;
        }
        self.list_reshow_quiet = true;
        effects.extend(self.reshow_reveal_list(list, n));
        effects
    }

    /// The spoken answer, when recording ends: read back, never graded.
    #[cfg(feature = "dictation")]
    pub(crate) fn reveal_heard(&mut self, words: &str) {
        if let Some(ListKind::Reveal(l)) = self.list.as_mut() {
            l.listening = None;
        }
        let words = words.trim().trim_end_matches(['.', ',', ';']);
        let msg = if words.is_empty() {
            self.msg("reveal-heard-nothing")
        } else {
            self.msg_args("reveal-you-said", &args!["words" => words])
        };
        self.tell(&msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<RevealItem> {
        vec![
            RevealItem {
                prompt: "First?".into(),
                answer: "One.".into(),
            },
            RevealItem {
                prompt: "Second?".into(),
                answer: "Two.".into(),
            },
        ]
    }

    #[test]
    fn answers_stay_hidden_until_revealed() {
        let c = Catalog::english();
        let mut l = RevealList::new("Test", items());
        assert_eq!(l.len(), 2);
        assert_eq!(l.rows(&c), ["First?", "Second?"]);
        assert_eq!(l.reveal(1), Some("Two."));
        assert_eq!(l.rows(&c), ["First?", "Second? Answer: Two."]);
        assert_eq!(l.reveal(5), None);
    }
}
