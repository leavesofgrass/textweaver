//! Fuzz target: the RIS importer (ADR-0019). Reading RIS never fails;
//! the references it returns format and write without panicking
//! (`check_references`).

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let _ = textweaver_cite::Format::sniff(&text);
    let refs = textweaver_cite::ris::parse(&text);
    textweaver_fuzz::check_references(&refs);
});
