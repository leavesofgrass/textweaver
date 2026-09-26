//! Fuzz target: the HTML loader (charset sniffing included).

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    textweaver_fuzz::load_checked(data, "html");
});
