//! Fuzz target: the archive loader (zip, tar, tar.gz, and 7z listings, and
//! the DAISY and EPUB detection inside zips). The same bytes are tried as
//! each kind.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    for hint in ["zip", "tar", "gz", "7z"] {
        textweaver_fuzz::load_checked(data, hint);
    }
});
