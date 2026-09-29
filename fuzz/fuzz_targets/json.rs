//! Fuzz target: JSON, JSON Lines, and Jupyter notebooks (ADR-0044), on
//! our own JSON parser with its limits on size, nesting, and values. The
//! same bytes are loaded as a `.json`, a `.jsonl`, and an `.ipynb` file,
//! and parsed directly.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    for hint in ["json", "jsonl", "ipynb"] {
        textweaver_fuzz::load_checked(data, hint);
    }
    if let Ok(text) = std::str::from_utf8(data)
        && let Ok(v) = textweaver_formats::json::parse(text)
    {
        let (text, markers) = textweaver_formats::json::convert(&v);
        let len = text.chars().count();
        assert!(markers.iter().all(|m| m.range.end.0 <= len));
    }
});
