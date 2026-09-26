//! Fuzz target: the EPUB loader. A zip archive is loaded as it is; other
//! input becomes the book's one chapter (and its navigation document, after
//! a NUL byte), so the fuzzer reaches the chapter walker quickly.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    textweaver_fuzz::load_checked(&textweaver_fuzz::epub(data), "epub");
});
