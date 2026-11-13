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

/// The most chars [`App::minutes_left`] counts words in; past it, the rest
/// is estimated from that sample, so the status bar's time costs the same
/// on a book as on a page.
const TIME_SAMPLE_CHARS: usize = 4096;

impl App {
    /// About how many whole minutes of reading are left from the reading
    /// position at the current rate, for the window's status bar; `None`
    /// without a document. Counted exactly over the next 4,096 characters
    /// and scaled for the rest, so it is cheap enough to ask on every
    /// refresh; it changes once a minute, never per word.
    pub fn minutes_left(&self) -> Option<usize> {
        let wpm = self.settings.speech.rate.wpm().max(1) as usize;
        let at = self.reading_position();
        let doc = &self.session.as_ref()?.doc;
        let from = at.unwrap_or_default().clamp_to(doc.len_chars());
        let left = doc.len_chars() - from.0;
        let take = left.min(TIME_SAMPLE_CHARS);
        let sample = CharRange::new(from.0, from.0 + take);
        let words = count_words(doc.text().slice(sample.to_range()).chars());
        let words = if take < left {
            words * left / take.max(1)
        } else {
            words
        };
        Some(words / wpm)
    }

    /// "About 3 minutes left.": the reading time from the cursor to the
    /// end at the current rate (`overview-time`), for the overview and
    /// Where am I. None with no document open.
    pub(crate) fn time_left(&self) -> Option<String> {
        let minutes = self.minutes_left()?;
        Some(self.msg_args("overview-time", &args!["minutes" => minutes]))
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
        let access = accessibility_lines(self.cat(), &doc.meta);
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
        // An EPUB's own accessibility information follows, one sentence a
        // fact; other documents have none to give.
        let msg = match access {
            Some(lines) => format!("{msg} {}", lines.join(" ")),
            None => msg,
        };
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

/// An EPUB's accessibility information in words, one fact a line,
/// meaning first: "Accessibility: screen reader friendly, per the
/// publisher." (or the other leads, or "No accessibility information
/// given."), then the summary, access modes, what is enough to read it,
/// features, hazards, conformance and certifier the book gives. `None`
/// for a document that is not an EPUB. Used by the document overview and
/// `tw info`.
pub fn accessibility_lines(
    c: &textweaver_lexicon::i18n::Catalog,
    meta: &textweaver_text::DocumentMeta,
) -> Option<Vec<String>> {
    use textweaver_formats::epub::{accessibility, term_words};
    let a = accessibility(meta)?;
    if a.is_empty() {
        return Some(vec![c.tr("access-info-none")]);
    }
    let words = |terms: &[String]| {
        terms
            .iter()
            .map(|t| term_words(t))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut lines = vec![c.tr(match a.reads_as_text() {
        Some(true) => "access-info-friendly",
        Some(false) => "access-info-not-text",
        None => "access-info-given",
    })];
    if let Some(s) = &a.summary {
        let text = s.trim_end_matches('.');
        lines.push(c.fmt("access-info-summary", &args!["text" => text]));
    }
    if !a.access_modes.is_empty() {
        let list = words(&a.access_modes);
        lines.push(c.fmt("access-info-modes", &args!["list" => list]));
    }
    if !a.sufficient.is_empty() {
        let sets: Vec<String> = a
            .sufficient
            .iter()
            .map(|set| {
                set.iter()
                    .map(|t| term_words(t))
                    .reduce(|first, second| {
                        c.fmt(
                            "access-info-and",
                            &args!["first" => first, "second" => second],
                        )
                    })
                    .unwrap_or_default()
            })
            .collect();
        let list = sets
            .into_iter()
            .reduce(|first, second| {
                c.fmt(
                    "access-info-or",
                    &args!["first" => first, "second" => second],
                )
            })
            .unwrap_or_default();
        lines.push(c.fmt("access-info-sufficient", &args!["list" => list]));
    }
    if !a.features.is_empty() {
        let list = words(&a.features);
        lines.push(c.fmt("access-info-features", &args!["list" => list]));
    }
    if !a.hazards.is_empty() {
        let list = words(&a.hazards);
        lines.push(c.fmt("access-info-hazards", &args!["list" => list]));
    }
    for claim in &a.conformance {
        let text = claim.trim_end_matches('.');
        lines.push(c.fmt("access-info-conformance", &args!["text" => text]));
    }
    if let Some(name) = &a.certifier {
        let name = name.trim_end_matches('.');
        lines.push(c.fmt("access-info-certifier", &args!["name" => name]));
    }
    Some(lines)
}
