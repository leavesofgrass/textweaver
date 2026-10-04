//! The reading highlights the document view paints, checked through its
//! paint list ([`DocumentView::painted`]) rather than pixels: the spoken
//! sentence is underlined in every palette (high contrast too), the word
//! carries its theme attribute, marks inside the sentence stay visible, and
//! the bold word moves nothing on the line.

use std::cell::Cell;
use std::rc::Rc;

use masonry::core::{NewWidget, WidgetTag};
use masonry_testing::{TestHarness, TestHarnessParams};
use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::text::Document;
use textweaver_xilem::document::{DocFont, DocMark, DocModel, DocState, DocumentView, PaintStep};
use textweaver_xilem::system_colors;
use textweaver_xilem::theme::{self, Palette};
use textweaver_xilem::window::{self, WINDOW_UNITS};

const DOC: WidgetTag<DocumentView> = WidgetTag::named("doc");

/// A sentence long enough to wrap onto a second line in a 500 px window,
/// then a second sentence.
const TEXT: &str = "The quick brown fox jumps over the lazy dog while the reader \
listens closely to every single word of this rather long sentence. A short one.";

/// The end of the first sentence.
fn first_sentence() -> CharRange {
    let end = TEXT.find(". ").expect("two sentences") + 1;
    CharRange::new(0, end)
}

fn harness(p: &Palette) -> TestHarness<DocumentView> {
    let view = DocumentView::new(p.clone(), DocFont::default(), Rc::new(Cell::new(0)));
    let mut params = TestHarnessParams::default();
    params.window_size = (500, 400).into();
    let mut h = TestHarness::create_with(
        theme::default_properties(p),
        NewWidget::new(view).with_tag(DOC),
        params,
    );
    for b in textweaver_xilem::fonts::bundled_blobs() {
        h.register_fonts(b);
    }
    let doc = Document::from_plain_text(TEXT);
    let w = textweaver_app::DocWindow::with_budget(&doc, CharPos::ZERO, WINDOW_UNITS);
    let model = DocModel {
        paragraphs: window::window_paragraphs(&doc, w.range()),
        spans: window::window_spans(&doc, w.range()),
        doc_len: doc.len_chars(),
        title: "Test".into(),
        ..DocModel::default()
    };
    h.edit_root_widget(|mut d| DocumentView::set_model(&mut d, model));
    let _ = h.redraw();
    h
}

/// Reading the word "fox" in the first sentence.
fn reading() -> DocState {
    DocState {
        caret: CharPos(16),
        anchor: None,
        spoken: Some(CharRange::new(16, 19)),
        sentence: Some(first_sentence()),
        reading: true,
    }
}

fn read(h: &mut TestHarness<DocumentView>) -> Vec<PaintStep> {
    h.edit_root_widget(|mut d| DocumentView::set_state(&mut d, reading()));
    let _ = h.redraw();
    h.root_widget().painted().to_vec()
}

/// Every palette the window can draw with: the bundled themes and the
/// Windows contrast theme the review screenshots use.
fn every_palette() -> Vec<Palette> {
    let mut all: Vec<Palette> = textweaver_theme::builtin::all()
        .iter()
        .map(Palette::from_theme)
        .collect();
    all.push(system_colors::palette(&system_colors::NIGHT_SKY));
    all
}

fn position(steps: &[PaintStep], f: impl Fn(&PaintStep) -> bool) -> Option<usize> {
    steps.iter().position(f)
}

#[test]
fn the_spoken_sentence_is_underlined_in_every_palette() {
    let palettes = every_palette();
    assert!(palettes.len() >= 24, "23 themes and high contrast");
    for p in palettes {
        let mut h = harness(&p);
        let steps = read(&mut h);
        let bands: Vec<_> = steps
            .iter()
            .filter_map(|s| match s {
                PaintStep::SentenceBand(r) => Some(*r),
                _ => None,
            })
            .collect();
        let lines = h.root_widget().sentence_underlines();
        assert!(bands.len() >= 2, "{}: the sentence wraps", p.name);
        assert_eq!(
            lines.len(),
            bands.len(),
            "{}: one underline per sentence line",
            p.name
        );
        for ((u, color), band) in lines.iter().zip(&bands) {
            assert_eq!(*color, p.sentence_line, "{}", p.name);
            assert!(u.height() >= 1.5, "{}: {u:?}", p.name);
            assert_eq!((u.x0, u.x1), (band.x0, band.x1), "{}", p.name);
            assert!(
                u.y0 > band.y0 && u.y1 <= band.y1 + 1.0,
                "{}: the line {u:?} sits under the text in {band:?}",
                p.name
            );
        }
        // The underline is drawn after the text, the band before it.
        let band_at = position(&steps, |s| matches!(s, PaintStep::SentenceBand(_)));
        let text_at = position(&steps, |s| matches!(s, PaintStep::Text(_)));
        let line_at = position(&steps, |s| matches!(s, PaintStep::SentenceUnderline(..)));
        assert!(band_at < text_at && text_at < line_at, "{}", p.name);
        // The word is drawn bold, as every bundled theme and the system
        // palette ask.
        assert!(
            steps
                .iter()
                .any(|s| matches!(s, PaintStep::WordText { bold: true, .. })),
            "{}: the word is bold",
            p.name
        );
        // Not reading: no highlight is painted.
        h.edit_root_widget(|mut d| DocumentView::set_state(&mut d, DocState::default()));
        let _ = h.redraw();
        assert!(h.root_widget().sentence_underlines().is_empty());
    }
}

#[test]
fn under_a_contrast_theme_the_underline_is_the_sentences_only_mark() {
    let p = system_colors::palette(&system_colors::NIGHT_SKY);
    assert_eq!(p.spoken_sentence, p.background, "no band to see");
    assert_eq!(p.sentence_line, p.text, "the system's text color");
    let mut h = harness(&p);
    read(&mut h);
    assert!(!h.root_widget().sentence_underlines().is_empty());
}

#[test]
fn marks_inside_the_spoken_sentence_are_painted_over_its_band() {
    let p = Palette::galaxy();
    let mut h = harness(&p);
    let marks = vec![
        (CharRange::new(4, 9), DocMark::Note),
        (CharRange::new(35, 39), DocMark::Highlight),
        (CharRange::new(40, 44), DocMark::CurrentFindHit),
    ];
    h.edit_root_widget(|mut d| DocumentView::set_marks(&mut d, marks));
    let steps = read(&mut h);
    let last_band = steps
        .iter()
        .rposition(|s| matches!(s, PaintStep::SentenceBand(_)))
        .expect("a sentence band");
    for mark in [DocMark::Note, DocMark::Highlight, DocMark::CurrentFindHit] {
        let band = position(
            &steps,
            |s| matches!(s, PaintStep::MarkBand(m, _) if *m == mark),
        )
        .unwrap_or_else(|| panic!("{mark:?} band painted"));
        let shape = position(
            &steps,
            |s| matches!(s, PaintStep::MarkShape(m, _) if *m == mark),
        )
        .unwrap_or_else(|| panic!("{mark:?} shape painted"));
        assert!(band > last_band, "{mark:?} band after the sentence band");
        assert!(shape > last_band, "{mark:?} shape after the sentence band");
    }
    // Design system E's order: every band, then the word band, the text,
    // then every shape, so no band covers a line or a box.
    let word = position(&steps, |s| matches!(s, PaintStep::WordBand(_))).expect("word band");
    let text = position(&steps, |s| matches!(s, PaintStep::Text(_))).expect("text");
    let last_band = steps
        .iter()
        .rposition(|s| matches!(s, PaintStep::MarkBand(..)))
        .expect("marks");
    let first_shape = position(&steps, |s| matches!(s, PaintStep::MarkShape(..))).expect("marks");
    assert!(last_band < word && word < text && text < first_shape);
}

/// A model of [`TEXT`] with a difficult word ("quick") and syllable
/// breaks in "reader" and "closely".
fn harness_with_aids(p: &Palette) -> TestHarness<DocumentView> {
    use textweaver_xilem::window::{SpanStyle, StyledSpan};
    let mut h = harness(p);
    let doc = Document::from_plain_text(TEXT);
    let w = textweaver_app::DocWindow::with_budget(&doc, CharPos::ZERO, WINDOW_UNITS);
    let mut spans = window::window_spans(&doc, w.range());
    spans.push(StyledSpan {
        range: CharRange::new(4, 9),
        style: SpanStyle::Difficult,
    });
    let reader = TEXT.find("reader").expect("reader");
    let closely = TEXT.find("closely").expect("closely");
    let model = DocModel {
        paragraphs: window::window_paragraphs(&doc, w.range()),
        spans,
        doc_len: doc.len_chars(),
        title: "Test".into(),
        breaks: vec![CharPos(reader + 4), CharPos(closely + 5)],
        separator: "\u{b7}".into(),
    };
    h.edit_root_widget(|mut d| DocumentView::set_model(&mut d, model));
    let _ = h.redraw();
    h
}

/// Every mark the window draws has its shape (design system E), in the
/// paint order the table gives, and each reading aid's mark is drawn in its
/// own color from the palette (the theme's role, or `[colors]`), in every
/// palette, the system's contrast colors too.
#[test]
fn every_mark_shape_is_painted_in_order() {
    use textweaver_xilem::document::AidMark;
    for p in every_palette() {
        let mut h = harness_with_aids(&p);
        let marks = vec![
            (CharRange::new(20, 25), DocMark::Highlight),
            (CharRange::new(26, 30), DocMark::Note),
            (CharRange::new(31, 34), DocMark::Bookmark),
            (CharRange::new(35, 39), DocMark::FindHit),
            (CharRange::new(40, 44), DocMark::CurrentFindHit),
        ];
        h.edit_root_widget(|mut d| {
            DocumentView::set_marks(&mut d, marks.clone());
            DocumentView::set_misspelled(&mut d, vec![CharRange::new(45, 50)]);
            DocumentView::set_lint(&mut d, vec![CharRange::new(51, 58)]);
        });
        let steps = read(&mut h);
        for (_, mark) in &marks {
            assert!(
                steps
                    .iter()
                    .any(|s| matches!(s, PaintStep::MarkShape(m, _) if m == mark)),
                "{}: {mark:?} has its shape",
                p.name
            );
        }
        let aid = |want: AidMark| -> Vec<(usize, textweaver_theme::Rgb)> {
            steps
                .iter()
                .enumerate()
                .filter_map(|(i, s)| match s {
                    PaintStep::Aid(m, _, c) if *m == want => Some((i, *c)),
                    _ => None,
                })
                .collect()
        };
        for (want, color) in [
            (AidMark::DifficultWord, p.difficult_word),
            (AidMark::Syllable, p.syllable_mark),
            (AidMark::Misspelling, p.misspelling),
            (AidMark::Lint, p.lint),
        ] {
            let found = aid(want);
            assert!(!found.is_empty(), "{}: {want:?} drawn", p.name);
            assert!(
                found.iter().all(|(_, c)| *c == color),
                "{}: {want:?} in its color",
                p.name
            );
        }
        assert_eq!(aid(AidMark::Syllable).len(), 2, "{}: two dots", p.name);
        // The order: bands, the word band, the text, the shapes and the
        // aids' lines, then the sentence's line, the misspelling's dots.
        let text = position(&steps, |s| matches!(s, PaintStep::Text(_))).expect("text");
        let word = position(&steps, |s| matches!(s, PaintStep::WordBand(_))).expect("word");
        let last_band = steps
            .iter()
            .rposition(|s| matches!(s, PaintStep::MarkBand(..)))
            .expect("bands");
        let first_shape =
            position(&steps, |s| matches!(s, PaintStep::MarkShape(..))).expect("shapes");
        let lint = aid(AidMark::Lint)[0].0;
        let sentence_line = position(&steps, |s| matches!(s, PaintStep::SentenceUnderline(..)))
            .expect("sentence line");
        let dots = aid(AidMark::Misspelling)[0].0;
        assert!(last_band < word && word < text, "{}", p.name);
        assert!(text < first_shape && first_shape < lint, "{}", p.name);
        assert!(lint < sentence_line && sentence_line < dots, "{}", p.name);
    }
}

/// Under the system's contrast colors every band is the page, so the
/// marks' shapes and the sentence's underline are what is seen: all are
/// still drawn.
#[test]
fn under_system_colors_the_shapes_are_drawn_where_bands_are_the_page() {
    let p = system_colors::palette(&system_colors::NIGHT_SKY);
    assert_eq!(p.user_highlight, p.background);
    assert_eq!(p.spoken_sentence, p.background);
    let mut h = harness(&p);
    let marks = vec![
        (CharRange::new(20, 25), DocMark::Highlight),
        (CharRange::new(26, 30), DocMark::Note),
    ];
    h.edit_root_widget(|mut d| DocumentView::set_marks(&mut d, marks));
    let steps = read(&mut h);
    let shapes = steps
        .iter()
        .filter(|s| matches!(s, PaintStep::MarkShape(..)))
        .count();
    assert_eq!(shapes, 2);
    assert!(!h.root_widget().sentence_underlines().is_empty());
}

/// The reading aids' colors come from the theme's derived roles, which
/// `[colors]` sets: a color chosen there is the one the window draws.
#[test]
fn the_aids_colors_follow_the_theme_roles() {
    use textweaver_theme::{ColorRole, Rgb};
    let mut t = textweaver_theme::builtin::default_theme().clone();
    let orange = Rgb::from_u32(0xff8800);
    let blue = Rgb::from_u32(0x3366ff);
    t.set_color(ColorRole::Misspelling, orange);
    t.set_color(ColorRole::Lint, blue);
    t.set_color(ColorRole::DifficultWord, orange);
    t.set_color(ColorRole::SyllableMark, blue);
    t.set_color(ColorRole::Ruler, blue);
    let p = Palette::from_theme(&t);
    assert_eq!(
        (
            p.misspelling,
            p.lint,
            p.difficult_word,
            p.syllable_mark,
            p.ruler_focus
        ),
        (orange, blue, orange, blue, blue)
    );
}

#[test]
fn the_bold_word_moves_nothing_on_the_line() {
    let p = Palette::galaxy();
    assert!(p.spoken_word_attrs.bold);
    let mut h = harness(&p);
    // A mark across the whole sentence: its bands are the layout's own
    // line extents, so any reflow would change them.
    let mark = vec![(first_sentence(), DocMark::FindHit)];
    h.edit_root_widget(|mut d| DocumentView::set_marks(&mut d, mark));
    let _ = h.redraw();
    let rects = |steps: &[PaintStep]| -> Vec<_> {
        steps
            .iter()
            .filter_map(|s| match s {
                PaintStep::MarkBand(_, r) => Some(*r),
                _ => None,
            })
            .collect()
    };
    let quiet = rects(h.root_widget().painted());
    let paras = h.root_widget().visible_paragraphs().to_vec();
    let steps = read(&mut h);
    assert_eq!(rects(&steps), quiet, "line widths unchanged while reading");
    assert_eq!(h.root_widget().visible_paragraphs(), paras.as_slice());
    // The bold word is drawn inside its own band.
    let band = steps
        .iter()
        .find_map(|s| match s {
            PaintStep::WordBand(r) => Some(*r),
            _ => None,
        })
        .expect("word band");
    let clip = steps
        .iter()
        .find_map(|s| match s {
            PaintStep::WordText {
                clip, bold: true, ..
            } => Some(*clip),
            _ => None,
        })
        .expect("bold word");
    assert_eq!(clip, band.inflate(3.0, 1.0));
}
