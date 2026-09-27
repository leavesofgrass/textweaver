//! Fuzz target: the PowerPoint loader. A zip archive is loaded as it is;
//! other input becomes the one slide (and its speaker notes, after a NUL
//! byte), so the fuzzer reaches the shape walker quickly.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    textweaver_fuzz::load_checked(&textweaver_fuzz::pptx(data), "pptx");
});
