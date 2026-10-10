//! The system clipboard the app's Paste reads (B1-cm), so formatted text
//! pastes as Markdown in the window as it does in the terminal reader.
//!
//! - **Windows:** [`System`] reads the clipboard's plain text
//!   (`CF_UNICODETEXT`), HTML (`HTML Format`, CF_HTML, with its header,
//!   which the app skips), and Rich Text Format, and the window gives it to
//!   the app with `App::set_clipboard`. Ctrl+V, the context menu's Paste,
//!   and Paste as plain text all read it.
//! - **Linux and macOS:** the clipboard crate Masonry's window uses reads
//!   plain text only, so Ctrl+V pastes the text it hands the window
//!   (`DocAction::Paste`), and the menu's Paste uses the text last copied
//!   in textweaver.
//!
//! shortcut: HTML and RTF are read on Windows only; reading them on Linux
//! and macOS needs a clipboard crate with those formats in the window.

use textweaver_app::{Clipboard, ClipboardContents};

/// The Windows clipboard, read each time the app pastes.
#[derive(Debug, Default)]
pub struct System;

impl Clipboard for System {
    fn read(&mut self) -> ClipboardContents {
        imp::read()
    }
}

/// Text from a clipboard format's bytes: up to the first NUL, which ends
/// the text in a clipboard block that is often larger than the text.
pub fn until_nul(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod imp {
    use std::ffi::c_void;

    use textweaver_app::ClipboardContents;

    windows_core::link!("user32.dll" "system" fn OpenClipboard(owner: isize) -> i32);
    windows_core::link!("user32.dll" "system" fn CloseClipboard() -> i32);
    windows_core::link!("user32.dll" "system" fn GetClipboardData(format: u32) -> isize);
    windows_core::link!("user32.dll" "system" fn IsClipboardFormatAvailable(format: u32) -> i32);
    windows_core::link!("user32.dll" "system" fn RegisterClipboardFormatW(name: *const u16) -> u32);
    windows_core::link!("kernel32.dll" "system" fn GlobalLock(mem: isize) -> *mut c_void);
    windows_core::link!("kernel32.dll" "system" fn GlobalUnlock(mem: isize) -> i32);
    windows_core::link!("kernel32.dll" "system" fn GlobalSize(mem: isize) -> usize);

    const CF_UNICODETEXT: u32 = 13;

    /// The clipboard's text, HTML, and RTF. Another program may hold the
    /// clipboard for a moment, so opening it is tried a few times.
    pub fn read() -> ClipboardContents {
        let html = format_id("HTML Format");
        let rtf = format_id("Rich Text Format");
        // SAFETY: the clipboard is opened with no owner window and closed
        // before returning; each handle is read only while it is open.
        unsafe {
            let mut opened = false;
            for _ in 0..5 {
                if OpenClipboard(0) != 0 {
                    opened = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            if !opened {
                return ClipboardContents::default();
            }
            let contents = ClipboardContents {
                text: block(CF_UNICODETEXT).map(|b| utf16_text(&b)),
                html: html.and_then(|f| block(f)).map(|b| super::until_nul(&b)),
                rtf: rtf.and_then(|f| block(f)).map(|b| super::until_nul(&b)),
            };
            CloseClipboard();
            contents
        }
    }

    /// The id the system gave a named clipboard format.
    fn format_id(name: &str) -> Option<u32> {
        let wide: Vec<u16> = name.encode_utf16().chain([0]).collect();
        // SAFETY: `wide` is a NUL-terminated UTF-16 string that outlives
        // the call.
        let id = unsafe { RegisterClipboardFormatW(wide.as_ptr()) };
        (id != 0).then_some(id)
    }

    /// A copy of the clipboard's block in `format`, while it is open.
    ///
    /// # Safety
    ///
    /// The clipboard must be open.
    unsafe fn block(format: u32) -> Option<Vec<u8>> {
        // SAFETY: the caller holds the clipboard open; the block is locked
        // while it is copied and its size is the system's.
        unsafe {
            if IsClipboardFormatAvailable(format) == 0 {
                return None;
            }
            let mem = GetClipboardData(format);
            if mem == 0 {
                return None;
            }
            let size = GlobalSize(mem);
            let ptr = GlobalLock(mem).cast::<u8>();
            if ptr.is_null() {
                return None;
            }
            let bytes = std::slice::from_raw_parts(ptr, size).to_vec();
            GlobalUnlock(mem);
            Some(bytes)
        }
    }

    /// UTF-16 text from a block, up to its NUL.
    fn utf16_text(bytes: &[u8]) -> String {
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|u| *u != 0)
            .collect();
        String::from_utf16_lossy(&units)
    }
}

#[cfg(not(windows))]
mod imp {
    use textweaver_app::ClipboardContents;

    /// Not read here: the window hands the app the text it pastes.
    pub fn read() -> ClipboardContents {
        ClipboardContents::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A clipboard block ends its text with a NUL and may run past it.
    #[test]
    fn text_ends_at_the_first_nul() {
        assert_eq!(until_nul(b"<p>Hi</p>\0\0junk"), "<p>Hi</p>");
        assert_eq!(until_nul(b"{\rtf1 Hi}"), "{\rtf1 Hi}");
        assert_eq!(until_nul(b""), "");
    }
}
