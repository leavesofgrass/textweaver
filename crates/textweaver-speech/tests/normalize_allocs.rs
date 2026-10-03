//! Normalization allocates nothing for a step that leaves an utterance
//! unchanged: the bench gate counts allocations (`normalize_allocs`), and
//! most utterances pass most steps unchanged. This binary counts this
//! thread's allocations with its own global allocator.

#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::BTreeMap;

use textweaver_core::{CharPos, PunctuationLevel, Utterance};
use textweaver_speech::normalize::{
    Abbreviations, MedicalLexicon, MedicalLexiconConfig, NormalizeConfig, Numbers, Pipeline,
    Punctuation, SplitCaps, Transform,
};

/// The system allocator, counting each thread's allocation calls.
struct Counting;

thread_local! {
    // A const `Cell` never allocates, so the allocator may touch it.
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
}

fn counted() {
    let _ = ALLOCS.try_with(|n| n.set(n.get() + 1));
}

// SAFETY: every call forwards to the system allocator with the same
// arguments; the counter never affects the returned pointers.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        counted();
        // SAFETY: forwarded unchanged.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        counted();
        // SAFETY: forwarded unchanged.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: forwarded unchanged; `ptr` came from this allocator.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        counted();
        // SAFETY: forwarded unchanged; `ptr` came from this allocator.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

/// Allocations `f` makes on this thread.
fn allocations<R>(f: impl FnOnce() -> R) -> (u64, R) {
    let before = ALLOCS.with(Cell::get);
    let r = f();
    (ALLOCS.with(Cell::get) - before, r)
}

/// Prose with nothing to normalize, and prose with a word start on every
/// word (the error-prone check), capitals, digits-free.
const PLAIN: &str = "The quick brown fox jumps over the lazy dog, and then it rests by the river. \
     Reading Aloud Is Easier When Nothing Changes; Words Stay Words.";

#[test]
fn unchanged_text_costs_no_allocation_in_each_step() {
    let medical = MedicalLexicon::from_config(&MedicalLexiconConfig {
        enabled: true,
        overlay: None,
    })
    .expect("on");
    let transforms: Vec<Box<dyn Transform>> = vec![
        Box::new(Abbreviations::new(&BTreeMap::new())),
        Box::new(Numbers::default()),
        Box::new(Numbers::identifiers_only()),
        Box::new(Punctuation::new(PunctuationLevel::Some)),
        Box::new(SplitCaps),
        Box::new(medical),
    ];
    for t in &transforms {
        // Warm the regex caches first.
        assert!(t.apply_changed(PLAIN).is_none(), "{}", t.name());
        let (n, changed) = allocations(|| t.apply_changed(PLAIN));
        assert!(changed.is_none(), "{}", t.name());
        assert_eq!(n, 0, "{} allocated {n} times on unchanged text", t.name());
    }
}

#[test]
fn the_default_pipeline_allocates_little_on_unchanged_text() {
    for medical in [false, true] {
        let config = NormalizeConfig {
            medical_lexicon: MedicalLexiconConfig {
                enabled: medical,
                overlay: None,
            },
            ..NormalizeConfig::default()
        };
        let p = Pipeline::for_settings(&config, PunctuationLevel::Some, false, false);
        let _ = p.apply(Utterance::literal(PLAIN, CharPos(0)));
        let u = Utterance::literal(PLAIN, CharPos(0));
        let (n, out) = allocations(|| p.apply(u));
        assert_eq!(out.text, PLAIN);
        // Only the math step, which lives in another crate, builds its
        // output before the pipeline sees that nothing changed.
        assert!(n <= 8, "{n} allocations with the medical lexicon {medical}");
    }
}
