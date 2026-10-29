//! Fuzz target: what the join QR reader is given. The bytes are read as an
//! image file (PNG or JPEG), and straight into the detector as a square of
//! greyscale pixels and as RGBA clipboard pixels. Nothing may panic. Short
//! UTF-8 input is also made into a code, written as PNG and SVG, and must
//! read back as the same text.

#![no_main]

use libfuzzer_sys::fuzz_target;
use textweaver_qr::{GrayImage, decode_bytes, decode_image, decode_rgba, encode};

/// The widest square the raw-pixel reads use, so each run stays quick.
const MAX_SIDE: usize = 256;

fn isqrt(n: usize) -> usize {
    let mut s = 0;
    while (s + 1) * (s + 1) <= n && s < MAX_SIDE {
        s += 1;
    }
    s
}

fuzz_target!(|data: &[u8]| {
    let _ = decode_bytes(data);

    let side = isqrt(data.len());
    if side > 0
        && let Ok(img) = GrayImage::new(side as u32, side as u32, data[..side * side].to_vec())
    {
        let _ = decode_image(&img);
    }
    let side = isqrt(data.len() / 4);
    if side > 0 {
        let _ = decode_rgba(side as u32, side as u32, &data[..side * side * 4]);
    }

    if let Ok(text) = std::str::from_utf8(data)
        && !text.is_empty()
        && text.len() <= 128
    {
        let code = encode(text).expect("128 bytes fit in a QR code");
        let svg = code.to_svg(text);
        assert!(svg.starts_with("<?xml"));
        let png = code.to_png(2).expect("a small code writes as PNG");
        assert_eq!(decode_bytes(&png).expect("our own code reads"), vec![text]);
    }
});
