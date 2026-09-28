//! Fuzz target: the BibTeX and BibLaTeX importer (ADR-0019). A bad file is
//! an error, never a panic; the references it returns format and write
//! without panicking (`check_references`).

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let _ = textweaver_cite::Format::sniff(&text);
    if let Ok(refs) = textweaver_cite::bibtex::parse(&text) {
        textweaver_fuzz::check_references(&refs);
    }
});
