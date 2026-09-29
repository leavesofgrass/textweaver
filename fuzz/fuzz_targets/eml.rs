//! Fuzz target: email and web archives (ADR-0035). The bytes are loaded as
//! an email (`.eml`: headers, a plain or HTML body, attachments listed) and
//! as a web archive (`.mhtml`: the root HTML part with its references
//! resolved), through mail-parser and the limits on size, parts, nesting,
//! and body text.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    textweaver_fuzz::load_checked(data, "eml");
    textweaver_fuzz::load_checked(data, "mhtml");
});
