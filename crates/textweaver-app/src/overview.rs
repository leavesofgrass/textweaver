//! The document overview and the reading passes (W9x-i, idea cards 4
//! and 2).
//!
//! - **Document overview**, on request beside Say Position: the title,
//!   then how many headings, tables, pictures and footnotes the document
//!   has, then about how many minutes of reading are left from the cursor
//!   at the current rate. Meaning first, in words: the title and the
//!   headings fit the first 40 cells of a Braille line. The counts come
//!   from the marker index and the words from one pass over the rest of
//!   the text, so it is quick on a large document.
//! - **Reading passes**: continuous reading says every sentence (full
//!   text), the first sentence of each paragraph with the headings, or the
//!   headings only. One command cycles them, and the pass is named aloud
//!   every time it changes and every time reading starts in a skim, so a
//!   skim is never taken for the whole text. It is called "First
//!   sentences", not "skim", because the `skim` speed preset exists. The
//!   pass lasts for the session and is not saved: the next start reads the
//!   full text.

use textweaver_core::{CharRange, MarkerKind};
use textweaver_lexicon::args;
use textweaver_text::ReadingPass;

use crate::app::App;
use crate::authoring::count_words;

impl App {
    /// "About 3 minutes left.": the reading time from the cursor to the
    /// end at the current rate (`overview-time`), for the overview and
    /// Where am I. None with no document open.
    pub(crate) fn time_left(&self) -> Option<String> {
        let wpm = self.settings.speech.rate.wpm().max(1) as usize;
        let at = self.reading_position();
        let doc = &self.session.as_ref()?.doc;
        let from = at.unwrap_or_default().clamp_to(doc.len_chars());
        let left = CharRange::new(from, doc.end());
        let words = count_words(doc.text().slice(left.to_range()).chars());
        Some(self.msg_args("overview-time", &args!["minutes" => words / wpm]))
    }

    /// Says the document overview: title, structure counts, minutes left.
    pub(crate) fn document_overview(&mut self) {
        let Some(time) = self.time_left() else {
            return;
        };
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let doc = &s.doc;
        let index = doc.marker_index();
        let count = |kind: MarkerKind, level: Option<u8>| index.count(kind, level);
        let headings = count(MarkerKind::Heading, None);
        let tables = count(MarkerKind::Table, None);
        let pictures = count(MarkerKind::Image, None);
        let footnotes = count(MarkerKind::Footnote, Some(1));
        let title = s.title.clone();
        let msg = self.msg_args(
            "overview-line",
            &args![
                "title" => title,
                "headings" => headings,
                "tables" => tables,
                "pictures" => pictures,
                "footnotes" => footnotes,
                "time" => time,
            ],
        );
        self.tell(&msg);
    }

    /// Changes the reading pass to the next one and names it.
    pub(crate) fn cycle_reading_pass(&mut self) {
        self.reading_pass = self.reading_pass.next();
        let name = self.reading_pass_name();
        let msg = self.msg_args("reading-pass-changed", &args!["pass" => name]);
        self.tell(&msg);
    }

    /// The reading pass in words: "full text", "first sentences",
    /// "headings".
    pub(crate) fn reading_pass_name(&self) -> String {
        self.msg(match self.reading_pass {
            ReadingPass::Full => "reading-pass-full",
            ReadingPass::FirstSentences => "reading-pass-first-sentences",
            ReadingPass::Headings => "reading-pass-headings",
        })
    }

    /// What continuous reading says first in a skim ("Pass: first
    /// sentences."), so it is never mistaken for the whole text. `None`
    /// for the full text.
    pub(crate) fn reading_pass_lead(&self) -> Option<String> {
        (self.reading_pass != ReadingPass::Full).then(|| {
            let name = self.reading_pass_name();
            self.msg_args("reading-pass-changed", &args!["pass" => name])
        })
    }
}
