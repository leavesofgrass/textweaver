//! Sentence steps in a multi-megabyte paragraph stay fast (sentence
//! windowing, `units` module docs). Integration tests see the real window
//! sizes, not the small ones the unit tests use.

use std::time::{Duration, Instant};

use textweaver_text::core::{CharPos, Unit};
use textweaver_text::{Document, next_unit, prev_unit, unit_at};

/// About 4 MB of wrapped prose with no blank line: one paragraph.
fn huge_paragraph() -> Document {
    let sentence =
        "Dr. Smith read 1,250 pages of the report at 9:30 a.m. on Friday, e.g. the appendix. ";
    let mut text = String::with_capacity(4_200_000);
    let mut line_len = 0;
    while text.len() < 4_000_000 {
        text.push_str(sentence);
        line_len += sentence.len();
        if line_len > 70 {
            text.pop();
            text.push('\n');
            line_len = 0;
        }
    }
    Document::from_plain_text(&text)
}

#[test]
fn sentence_steps_in_a_huge_paragraph_are_fast() {
    let doc = huge_paragraph();
    let mid = CharPos(doc.len_chars() / 2);
    let started = Instant::now();
    let mut pos = mid;
    for _ in 0..50 {
        let s = next_unit(&doc, pos, Unit::Sentence).expect("a next sentence");
        assert!(s.start > pos);
        assert!(doc.slice(s).starts_with("Dr. Smith"), "{:?}", doc.slice(s));
        pos = s.start;
    }
    for _ in 0..50 {
        let s = prev_unit(&doc, pos, Unit::Sentence).expect("a previous sentence");
        assert!(s.start < pos);
        pos = s.start;
    }
    let at = unit_at(&doc, mid, Unit::Sentence).expect("a sentence");
    assert!(at.len() < 100);
    let per_step = started.elapsed() / 101;
    eprintln!("sentence step in a 4 MB paragraph: {per_step:?}");
    // Segmenting the whole paragraph takes seconds in a debug build; a
    // windowed step takes milliseconds. The bound is loose for slow CI.
    assert!(
        per_step < Duration::from_millis(250),
        "{per_step:?} per step"
    );
}
