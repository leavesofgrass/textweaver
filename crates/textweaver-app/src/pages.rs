//! Pages (the Braille pass, Wave 5): going to a page, and naming the page
//! in the position report, on the title line, and in the outline.
//!
//! A PDF carries a `PageBreak` marker per page, labelled with the printed
//! page label (`iv`, `A-3`) or the page number, and other paged formats
//! (DAISY page numbers, slides) do the same. When a document has them:
//!
//! - **Go to** takes `page 12` or `p 12` (and the word in the catalog's
//!   language, `goto-word-page`), and in a paged document a bare number
//!   is a page too (`line 12` is still a line). The printed label wins,
//!   so `page 3` is the page printed "3" even after pages i to x; a
//!   number no label matches is the nth page.
//! - **The position report** (Shift+W) and **say status** name the page
//!   first: "Page 12 of 30."
//! - **The outline** lists the pages when the document has no headings.

use textweaver_core::{CharPos, CharRange, MarkerKind};
use textweaver_lexicon::args;
use textweaver_text::Document;

use crate::app::App;
use crate::authoring_state::OutlineItem;
use crate::nav::ReadAfter;
use crate::text_util::{self, preview};

/// Words of a page's first line shown in the outline and said after a
/// jump.
const PAGE_PREVIEW_WORDS: usize = 8;

/// A page of a paged document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Page {
    /// Its place among the pages, from 1.
    pub(crate) n: usize,
    /// How many pages the document has.
    pub(crate) count: usize,
    /// The printed label, or the page number when the page has none.
    pub(crate) label: String,
    /// Where the page's text starts.
    pub(crate) start: CharPos,
}

impl Page {
    /// True when the printed label is not simply the page's number.
    pub(crate) fn labelled(&self) -> bool {
        self.label != self.n.to_string()
    }
}

/// Every page of `doc`, in order; empty when it has no page markers.
pub(crate) fn pages(doc: &Document) -> Vec<Page> {
    let starts: Vec<(CharPos, Option<String>)> = doc
        .marker_index()
        .iter(MarkerKind::PageBreak, None)
        .map(|m| (m.range.start, m.label.clone()))
        .collect();
    let count = starts.len();
    starts
        .into_iter()
        .enumerate()
        .map(|(i, (start, label))| Page {
            n: i + 1,
            count,
            label: label
                .map(|l| l.trim().to_owned())
                .filter(|l| !l.is_empty())
                .unwrap_or_else(|| (i + 1).to_string()),
            start,
        })
        .collect()
}

/// The page holding `pos`: the last one starting at or before it (text
/// before the first page marker belongs to the first page).
pub(crate) fn page_at(doc: &Document, pos: CharPos) -> Option<Page> {
    let all = pages(doc);
    let i = all.iter().rposition(|p| p.start <= pos).unwrap_or(0);
    all.into_iter().nth(i)
}

/// The page `wanted` names: a printed label (ignoring case) first, then a
/// page number.
pub(crate) fn find_page(doc: &Document, wanted: &str) -> Option<Page> {
    let wanted = wanted.trim();
    let all = pages(doc);
    if let Some(i) = all
        .iter()
        .position(|p| p.label.to_lowercase() == wanted.to_lowercase())
    {
        return all.into_iter().nth(i);
    }
    let n: usize = wanted.parse().ok()?;
    all.into_iter().nth(n.checked_sub(1)?)
}

impl App {
    /// True when the open document has page markers.
    pub(crate) fn has_pages(&self) -> bool {
        self.session.as_ref().is_some_and(|s| {
            s.doc
                .marker_index()
                .iter(MarkerKind::PageBreak, None)
                .next()
                .is_some()
        })
    }

    /// The page a go-to answer names: `page 12` or `p 12` in any
    /// document, and a bare number in a paged one (where line numbers are
    /// the reader's, not the printed book's); `line 12` stays a line.
    pub(crate) fn page_answer(&self, text: &str) -> Option<String> {
        if let Some(page) = crate::goto::parse_page_in(self.cat(), text) {
            return Some(page);
        }
        let t = text.trim();
        (self.has_pages() && !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()))
            .then(|| t.to_owned())
    }

    /// The page at `pos`, if the document has pages.
    pub(crate) fn page_of(&self, pos: CharPos) -> Option<Page> {
        page_at(&self.session.as_ref()?.doc, pos)
    }

    /// "page 12 of 30" (or "page iv, 4 of 30"), for the title line and the
    /// position report; `said_id` and `labelled_id` are the two message
    /// ids.
    pub(crate) fn page_words(&self, page: &Page, said_id: &str, labelled_id: &str) -> String {
        if page.labelled() {
            self.msg_args(
                labelled_id,
                &args!["label" => page.label.as_str(), "n" => page.n, "pages" => page.count],
            )
        } else {
            self.msg_args(said_id, &args!["page" => page.n, "pages" => page.count])
        }
    }

    /// Go to page `wanted` (a printed label or a number); records history
    /// and says "Page 12, line 400: its first words".
    pub(crate) fn go_to_page(&mut self, wanted: &str) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let count = pages(&s.doc).len();
        if count == 0 {
            let msg = self.msg("pages-none");
            self.error(&msg);
            return;
        }
        let Some(page) = find_page(&s.doc, wanted) else {
            let msg = self.msg_args(
                "pages-no-such-page",
                &args!["page" => wanted.trim(), "pages" => count],
            );
            self.error(&msg);
            return;
        };
        self.jump_to_page(&page);
    }

    /// Jumps to the first word of `page` (recorded in history) and says
    /// "Page 12, line 400: its first words".
    pub(crate) fn jump_to_page(&mut self, page: &Page) {
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let target = {
            let w = text_util::first_word_at_or_after(&s.doc, page.start);
            if w < page.start { page.start } else { w }
        };
        let label = self.msg_args("pages-label", &args!["label" => page.label.as_str()]);
        let line = text_util::line_of(&s.doc, target);
        let content = preview(
            &s.doc,
            CharRange::new(target, text_util::line_range(&s.doc, line).end),
            PAGE_PREVIEW_WORDS,
        );
        let label = self.msg_args(
            "nav-label-line",
            &args!["label" => label, "line" => line + 1],
        );
        let msg = self.nav_message(Some(&label), target, &content);
        self.jump(target, true, ReadAfter::Follow, &msg);
    }

    /// The outline's items when a document has pages and no headings:
    /// "Page 12: its first words", one per page. Level 0 marks a page
    /// (headings have levels 1 to 6).
    pub(crate) fn page_outline(&self, doc: &Document) -> Vec<OutlineItem> {
        pages(doc)
            .into_iter()
            .map(|p| {
                let line = text_util::line_of(doc, p.start);
                let first = preview(
                    doc,
                    CharRange::new(p.start, text_util::line_range(doc, line).end),
                    PAGE_PREVIEW_WORDS,
                );
                OutlineItem {
                    pos: p.start,
                    level: 0,
                    text: self.msg_args(
                        "pages-outline-item",
                        &args!["label" => p.label.as_str(), "text" => first],
                    ),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use textweaver_text::Marker;

    /// Three pages, printed "ii", "1", and unlabelled.
    fn paged() -> Document {
        let text = "Preface words here.\nFirst page text.\nSecond page text.\n";
        Document::new(
            textweaver_text::DocumentMeta::default(),
            ropey::Rope::from_str(text),
            vec![
                Marker::new(
                    MarkerKind::PageBreak,
                    CharRange::new(CharPos(0), CharPos(20)),
                )
                .with_label("ii"),
                Marker::new(
                    MarkerKind::PageBreak,
                    CharRange::new(CharPos(20), CharPos(37)),
                )
                .with_label("1"),
                Marker::new(
                    MarkerKind::PageBreak,
                    CharRange::new(CharPos(37), CharPos(55)),
                ),
            ],
        )
    }

    #[test]
    fn pages_are_found_by_label_then_number() {
        let doc = paged();
        let all = pages(&doc);
        assert_eq!(all.len(), 3);
        assert_eq!(all[2].label, "3");
        assert!(all[0].labelled());
        assert!(!all[2].labelled());
        // "1" is the printed label of the second page, not the first page.
        assert_eq!(find_page(&doc, "1").map(|p| p.n), Some(2));
        assert_eq!(find_page(&doc, "II").map(|p| p.n), Some(1));
        assert_eq!(find_page(&doc, "3").map(|p| p.n), Some(3));
        assert_eq!(find_page(&doc, "4"), None);
        assert_eq!(find_page(&doc, "0"), None);
        assert_eq!(page_at(&doc, CharPos(25)).map(|p| p.n), Some(2));
        assert_eq!(page_at(&doc, CharPos(54)).map(|p| p.n), Some(3));
    }

    #[test]
    fn a_document_without_page_markers_has_no_pages() {
        let doc = Document::from_plain_text("No pages.\n");
        assert!(pages(&doc).is_empty());
        assert_eq!(page_at(&doc, CharPos(0)), None);
        assert_eq!(find_page(&doc, "1"), None);
    }
}
