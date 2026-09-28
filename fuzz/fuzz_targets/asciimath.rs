//! Fuzz target: the ASCIIMath parser (ADR-0018). Parsing is total: any
//! text gives a tree, and the checks in `check_math` hold for it.

#![no_main]

use libfuzzer_sys::fuzz_target;
use textweaver_math::Notation;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    textweaver_fuzz::check_math(&textweaver_math::parse(text, Notation::AsciiMath));
});
