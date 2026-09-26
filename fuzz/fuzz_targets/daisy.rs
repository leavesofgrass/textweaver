//! Fuzz target: the DAISY loader. A zip archive is loaded as it is (as a
//! Bookshare download would be); other input becomes a book's DTBook file
//! (and its NCX, after a NUL byte), and is also loaded as a DTBook `.xml`
//! file on its own.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    textweaver_fuzz::load_checked(&textweaver_fuzz::daisy(data), "zip");
    textweaver_fuzz::load_checked(data, "xml");
});
