//! A huge image is refused from its header, before any large allocation
//! (security review, L9). A test program of its own, because it counts
//! the program's allocations: the largest single one while the image is
//! read must stay small.

// A counting global allocator needs `unsafe impl GlobalAlloc`; it only
// passes each call on to the system allocator.
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use textweaver_qr::{MAX_QR_PIXELS, QrError, decode_bytes, decode_rgba};

/// The system allocator, noting the largest allocation asked for.
struct Counting;

static LARGEST: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every call is passed straight to the system allocator.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LARGEST.fetch_max(layout.size(), Ordering::SeqCst);
        // SAFETY: the caller's contract is the system allocator's.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from `alloc` above, with this layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// The start of a baseline JPEG declaring `width` by `height` pixels:
/// the frame header only, no image data.
fn jpeg_header(width: u16, height: u16) -> Vec<u8> {
    let mut b = vec![0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x0B, 0x08];
    b.extend(height.to_be_bytes());
    b.extend(width.to_be_bytes());
    b.extend([0x01, 0x01, 0x11, 0x00, 0xFF, 0xD9]);
    b
}

#[test]
fn a_20000_by_20000_jpeg_is_refused_before_any_large_allocation() {
    let jpeg = jpeg_header(20_000, 20_000);
    LARGEST.store(0, Ordering::SeqCst);
    let err = decode_bytes(&jpeg).unwrap_err();
    let largest = LARGEST.load(Ordering::SeqCst);
    assert!(matches!(err, QrError::Image(_)), "{err}");
    let said = err.to_string();
    assert!(
        said.contains("too large to read a QR code") && said.contains("20000 by 20000"),
        "{said}"
    );
    // The decoder would need 400 MB of luma; the refusal needs a sentence.
    assert!(largest < 64 * 1024, "largest allocation {largest} bytes");

    // Clipboard pixels over the limit are refused before conversion.
    let err = decode_rgba(8_000, 6_000, &[]).unwrap_err();
    const _: () = assert!(8_000 * 6_000 > MAX_QR_PIXELS);
    assert!(err.to_string().contains("8000 by 6000"), "{err}");
}
