//! Fuzz target: the LaTeX loader (ADR-0035), our own tokenizer and
//! two-pass parser with its limits on size, tokens, steps, macro
//! expansions, and nesting. Loaded with the default options, with code
//! skipped and notes inline, and with skipped commands named in place.
//! The input has no folder, so `\input` reads nothing.

#![no_main]

use libfuzzer_sys::fuzz_target;
use textweaver_formats::{Registry, Source};

fuzz_target!(|data: &[u8]| {
    textweaver_fuzz::load_checked(data, "tex");
    let options = textweaver_formats::LoadOptions {
        name_skipped_commands: true,
        ..textweaver_fuzz::options()
    };
    let source = Source::Bytes {
        data: data.to_vec(),
        hint: "tex".into(),
    };
    if let Ok(doc) = Registry::with_builtins().load(&source, &options) {
        textweaver_fuzz::check(&doc);
    }
});
