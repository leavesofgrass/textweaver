//! Fuzz target: a device-independent bitmap (`CF_DIB`) from the Windows
//! clipboard, which any other program may have put there, read by
//! `dib_to_rgba` for Join from a QR image. Nothing may panic, and a bitmap
//! that reads has four bytes for every pixel, within the size limit, all
//! opaque, and passes the check other clipboards' pixels get.

#![no_main]

use libfuzzer_sys::fuzz_target;
use textweaver_app::clipboard_image::{MAX_SIDE, checked_rgba, dib_to_rgba};

fuzz_target!(|data: &[u8]| {
    if let Some((w, h, pixels)) = dib_to_rgba(data) {
        assert!(w > 0 && h > 0 && w <= MAX_SIDE && h <= MAX_SIDE);
        assert_eq!(pixels.len(), w as usize * h as usize * 4);
        assert!(pixels.chunks_exact(4).all(|p| p[3] == 255));
        assert!(checked_rgba(w, h, pixels).is_some());
    }
});
