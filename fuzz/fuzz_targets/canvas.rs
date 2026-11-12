//! Fuzz target: Obsidian canvas files (`.canvas`, JSON Canvas 1.0), read as
//! a document by the canvas loader: cards, groups by their rectangles, and
//! the connections. The input has no folder, so file cards resolve to
//! nothing. A canvas may be refused with an error, but nothing may panic,
//! and every marker lies inside the text.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    textweaver_fuzz::load_checked(data, "canvas");
});
