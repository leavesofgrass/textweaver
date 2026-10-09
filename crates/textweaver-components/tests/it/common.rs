//! A made-up component with two small files, and its fixtures.

use std::borrow::Cow;

use textweaver_components::{Check, Component, FilePin, sha256_hex};

/// The first file's bytes: 5,000 bytes, so cancelling lands part way.
pub fn first_bytes() -> Vec<u8> {
    (0..5000u32).map(|i| (i * 7 % 251) as u8).collect()
}

/// The second file's bytes.
pub fn second_bytes() -> Vec<u8> {
    b"tokenizer for a made-up model".to_vec()
}

/// A component named `id`, with both files at example addresses.
pub fn component(id: &str) -> Component {
    let pin = |name: &str, bytes: &[u8]| FilePin {
        name: Cow::Owned(name.to_owned()),
        url: Cow::Owned(format!("https://example.invalid/{id}/{name}")),
        size: bytes.len() as u64,
        check: Check::Sha256(Cow::Owned(sha256_hex(bytes))),
    };
    Component {
        id: Cow::Owned(id.to_owned()),
        title: Cow::Borrowed("a made-up model"),
        license: Cow::Borrowed("CC0-1.0"),
        credit: Cow::Borrowed("made up for tests"),
        features: Cow::Borrowed(&[Cow::Borrowed("dictation")]),
        folder: Cow::Owned(format!("models/{id}")),
        files: Cow::Owned(vec![
            pin("model.onnx", &first_bytes()),
            pin("tokenizer.json", &second_bytes()),
        ]),
        notice: Some((Cow::Borrowed("LICENSE.txt"), Cow::Borrowed("CC0"))),
        listing: None,
    }
}

/// The fake fetcher serving `c`'s files at their public addresses.
pub fn serving(c: &Component) -> textweaver_components::fake::FakeFetcher {
    textweaver_components::fake::FakeFetcher::new()
        .with(c.files[0].url.to_string(), first_bytes())
        .with(c.files[1].url.to_string(), second_bytes())
}
