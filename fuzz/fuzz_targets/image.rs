//! Fuzz target: the picture decoders OCR reads (PNG and JPEG), with their
//! size limits, and the picture loader with OCR off.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(img) = textweaver_ocr::GrayImage::decode(data) {
        assert_eq!(img.data.len(), img.width as usize * img.height as usize);
        let small = img.fit_within(64);
        assert!(small.width <= 64 && small.height <= 64);
    }
    textweaver_fuzz::load_checked(data, "png");
});
