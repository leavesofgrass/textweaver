//! Fuzz target: live source files (`.twlive`), which come from someone
//! else. The bytes are read as one, with no folder. Nothing may panic, and
//! a file that reads writes back as a file that reads again the same, with
//! nothing left out.

#![no_main]

use libfuzzer_sys::fuzz_target;
use textweaver_store::{live_source_file_text, read_live_source_file};

fuzz_target!(|data: &[u8]| {
    let Ok(file) = read_live_source_file(data, None) else {
        return;
    };
    let text = live_source_file_text(&file.source, None);
    let again = read_live_source_file(text.as_bytes(), None).expect("a written file reads");
    assert!(again.notes.is_empty(), "{:?}", again.notes);
    assert_eq!(again.source, file.source);
});
