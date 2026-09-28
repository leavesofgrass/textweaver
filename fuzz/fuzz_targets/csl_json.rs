//! Fuzz target: the CSL-JSON importer (ADR-0019), the library's own
//! format. A bad file is an error, never a panic; the references it
//! returns format and write without panicking, and write back as CSL-JSON
//! that reads again (`check_references`).

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let _ = textweaver_cite::Format::sniff(&text);
    if let Ok(refs) = textweaver_cite::csljson::parse(&text) {
        textweaver_fuzz::check_references(&refs);
        let _ = textweaver_cite::Library::from_references(refs);
    }
});
