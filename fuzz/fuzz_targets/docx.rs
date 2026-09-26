//! Fuzz target: the Word (DOCX) loader. A zip archive is loaded as it is;
//! other input becomes `word/document.xml` (and `word/numbering.xml`, after
//! a NUL byte), so the fuzzer reaches the XML walker quickly.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    textweaver_fuzz::load_checked(&textweaver_fuzz::docx(data), "docx");
});
