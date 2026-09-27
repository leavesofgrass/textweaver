//! Fuzz target: the spreadsheet loader. The first byte picks CSV, TSV,
//! OpenDocument, or Excel; the rest is the file, or the sheet's XML inside
//! a minimal package.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let (file, hint) = textweaver_fuzz::sheet(data);
    textweaver_fuzz::load_checked(&file, hint);
});
