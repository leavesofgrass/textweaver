//! Fuzz target: a web page's response as the web loader reads it after
//! fetching (feature `url`): the `Content-Type` value, then the body. No
//! network is used and nothing is written to disk.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let (content_type, body) = textweaver_fuzz::web_response(data);
    for url in ["https://example.org/page", "https://example.org/file.pdf?x=1"] {
        if let Ok(doc) = textweaver_formats::web::read_response(
            url,
            &content_type,
            body.clone(),
            &textweaver_fuzz::options(),
        ) {
            textweaver_fuzz::check(&doc);
        }
    }
});
