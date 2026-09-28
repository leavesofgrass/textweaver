//! Fuzz target: the RTF loader. Input that does not start with the RTF
//! header gets one, so the fuzzer reaches the parser at once. Loaded with
//! the default options, with notes inline, and with tracked changes said
//! in place.

#![no_main]

use libfuzzer_sys::fuzz_target;
use textweaver_formats::{Registry, RevisionMode, Source};

fuzz_target!(|data: &[u8]| {
    let mut bytes = data.to_vec();
    if !bytes.starts_with(b"{\\rtf") {
        bytes.splice(0..0, b"{\\rtf1 ".iter().copied());
    }
    textweaver_fuzz::load_checked(&bytes, "rtf");
    let options = textweaver_formats::LoadOptions {
        revisions: RevisionMode::Marked,
        ..textweaver_fuzz::options()
    };
    let source = Source::Bytes {
        data: bytes,
        hint: "rtf".into(),
    };
    if let Ok(doc) = Registry::with_builtins().load(&source, &options) {
        textweaver_fuzz::check(&doc);
    }
});
