//! Summarize: the most central sentences of the selection, the chapter, or
//! the document, as a list; Enter goes to a sentence and says it
//! (ADR-0037, Agent W5s).
//!
//! The sentences come from `textweaver-summary` (LexRank, no model), as
//! many as `[summary] sentences`, in document order. What is summarized:
//!
//! - the selection, when there is one;
//! - else the chapter at the cursor, when the document has more than one
//!   (section breaks, else level-1 headings, as Next Chapter uses);
//! - else the whole document.
//!
//! The list's introduction says which, and how many sentences: "Chapter
//! summary, 5 sentences. Enter goes to the sentence." (the key fact first,
//! for a 40-cell braille line). The list is a `ListModel` list like the
//! others, so arrows say "2 of 5" and the sentence, and first letters jump.

use textweaver_core::{CharRange, MarkerKind};
use textweaver_lexicon::args;
use textweaver_summary::{Options, summarize_range};
use textweaver_text::Document;

use crate::app::{App, ListKind};
use crate::command::Effect;
use crate::nav::ReadAfter;

/// What a summary covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope {
    Selection,
    Chapter,
    Document,
}

impl App {
    /// `summarize`: the summary list, after its introduction.
    pub(crate) fn summarize(&mut self) -> Vec<Effect> {
        let Some(s) = self.session.as_ref() else {
            return vec![Effect::Redraw];
        };
        let (range, scope) = summary_scope(&s.doc, s.selection, s.cursor);
        let options = Options {
            sentences: self.settings.summary.sentences,
            ..Options::default()
        };
        let summary = summarize_range(&s.doc, range, &options);
        if summary.sentences.is_empty() {
            let msg = self.msg("summary-none");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let n = summary.sentences.len();
        let title = self.msg_args(
            match scope {
                Scope::Selection => "summary-title-selection",
                Scope::Chapter => "summary-title-chapter",
                Scope::Document => "summary-title",
            },
            &args!["n" => n],
        );
        let intro = self.msg_args(
            if summary.sampled() {
                "summary-intro-sampled"
            } else {
                "summary-intro"
            },
            &args!["title" => title.as_str()],
        );
        let (ranges, items): (Vec<CharRange>, Vec<String>) = summary
            .sentences
            .into_iter()
            .map(|x| (x.range, x.text))
            .unzip();
        self.list = Some(ListKind::Summary(ranges));
        self.tell(&intro);
        vec![Effect::ShowList { title, items }]
    }

    /// Enter on sentence `n` of the summary: the cursor goes to it and it
    /// is said (read from there when reading).
    pub(crate) fn choose_summary_sentence(&mut self, ranges: &[CharRange], n: usize) {
        let (Some(&range), Some(s)) = (ranges.get(n), self.session.as_ref()) else {
            return;
        };
        let range = range.clamp_to(s.doc.len_chars());
        let text = s.doc.slice(range);
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let msg = self.nav_message(None, range.start, &text);
        self.jump(range.start, true, ReadAfter::Follow, &msg);
    }
}

/// What Summarize covers: the selection, else the chapter at `cursor`
/// when there are chapters, else the whole document.
fn summary_scope(
    doc: &Document,
    selection: Option<CharRange>,
    cursor: textweaver_core::CharPos,
) -> (CharRange, Scope) {
    if let Some(sel) = selection.filter(|r| !r.is_empty()) {
        return (sel.clamp_to(doc.len_chars()), Scope::Selection);
    }
    let starts = |kind: MarkerKind, level: Option<u8>| -> Vec<textweaver_core::CharPos> {
        doc.markers()
            .iter()
            .filter(|m| m.kind == kind && level.is_none_or(|l| m.level == l))
            .map(|m| m.range.start)
            .collect()
    };
    let mut chapters = starts(MarkerKind::SectionBreak, None);
    if chapters.is_empty() {
        chapters = starts(MarkerKind::Heading, Some(1));
    }
    chapters.sort_unstable();
    chapters.dedup();
    if chapters.len() < 2 {
        return (doc.full_range(), Scope::Document);
    }
    let i = chapters.partition_point(|&p| p <= cursor);
    let start = if i == 0 {
        textweaver_core::CharPos::ZERO
    } else {
        chapters[i - 1]
    };
    let end = chapters.get(i).copied().unwrap_or(doc.end());
    (CharRange::new(start, end), Scope::Chapter)
}

#[cfg(test)]
mod tests;
