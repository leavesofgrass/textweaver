//! Fuzz target: caption files (SubRip and WebVTT), read as a transcript by
//! the caption loader: cues, voice tags, markup, character references, and
//! the minute headings. The same bytes are read as `.srt` and as `.vtt`. A
//! file may be refused with an error, but nothing may panic, and every
//! marker lies inside the text.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    textweaver_fuzz::load_checked(data, "srt");
    textweaver_fuzz::load_checked(data, "vtt");
});
