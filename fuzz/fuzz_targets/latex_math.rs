//! Fuzz target: the LaTeX math parser (ADR-0018). Parsing is total: any
//! text gives a tree, and the checks in `check_math` hold for it. The same
//! text is also read as prose with `$...$` math in it, as the speech
//! pipeline reads a document, and its offset map must stay valid.

#![no_main]

use libfuzzer_sys::fuzz_target;
use textweaver_math::{Notation, TextOptions};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    textweaver_fuzz::check_math(&textweaver_math::parse(text, Notation::Latex));
    let (spoken, map) = textweaver_math::speak_text(text, &TextOptions::default());
    if let Err(e) = map.check_invariants(&spoken) {
        panic!("speak_text map for {text:?}: {e}");
    }
});
