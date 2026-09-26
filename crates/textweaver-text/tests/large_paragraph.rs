//! Sentence steps in a multi-megabyte paragraph stay fast (sentence
//! windowing, `units` module docs). Integration tests see the real window
//! sizes, not the small ones the unit tests use.

use std::time::{Duration, Instant};

use textweaver_text::core::{CharPos, Unit};
use textweaver_text::{Document, next_unit, prev_unit, unit_at};

/// About `bytes` of wrapped prose with no blank line: one paragraph.
fn paragraph(bytes: usize) -> Document {
    let sentence =
        "Dr. Smith read 1,250 pages of the report at 9:30 a.m. on Friday, e.g. the appendix. ";
    let mut text = String::with_capacity(bytes + 200);
    let mut line_len = 0;
    while text.len() < bytes {
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

/// The time of one sentence step from the middle of `doc`, averaged over
/// 50 steps forward, 50 back, and one `unit_at`.
fn step_time(doc: &Document) -> Duration {
    let mid = CharPos(doc.len_chars() / 2);
    let started = Instant::now();
    let mut pos = mid;
    for _ in 0..50 {
        let s = next_unit(doc, pos, Unit::Sentence).expect("a next sentence");
        assert!(s.start > pos);
        assert!(doc.slice(s).starts_with("Dr. Smith"), "{:?}", doc.slice(s));
        pos = s.start;
    }
    for _ in 0..50 {
        let s = prev_unit(doc, pos, Unit::Sentence).expect("a previous sentence");
        assert!(s.start < pos);
        pos = s.start;
    }
    let at = unit_at(doc, mid, Unit::Sentence).expect("a sentence");
    assert!(at.len() < 100);
    started.elapsed() / 101
}

#[test]
fn sentence_steps_in_a_huge_paragraph_are_fast() {
    // A step in a 4 MB paragraph must cost about what it costs in a 40 KB
    // one: windowed, its work does not grow with the paragraph. Without
    // windowing it would be about a hundred times slower. Comparing with a
    // baseline measured on the same machine in the same run keeps the test
    // meaningful on a slow or busy CI runner, where any fixed wall-clock
    // bound is either too loose to catch a regression or flaky.
    let small = paragraph(40_000);
    let huge = paragraph(4_000_000);
    // Warm up (allocations, caches), then measure; the baseline is the
    // faster of two runs, so one noisy run cannot make it too strict.
    let _ = step_time(&small);
    let baseline = step_time(&small).min(step_time(&small));
    let per_step = step_time(&huge);
    eprintln!("sentence step: {baseline:?} in 40 KB, {per_step:?} in 4 MB");
    // Twenty times the baseline, plus 20 ms for scheduling noise: far
    // below the hundredfold cost of segmenting the whole paragraph.
    let bound = baseline * 20 + Duration::from_millis(20);
    assert!(
        per_step < bound,
        "{per_step:?} per step in 4 MB, over {bound:?} (baseline {baseline:?} in 40 KB)"
    );
}
