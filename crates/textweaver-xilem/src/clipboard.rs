//! The system clipboard the app's Paste reads (B1-cm), so formatted text
//! pastes as Markdown in the textweaver app as it does in the terminal
//! reader.
//!
//! - **Windows:** [`System`] reads the clipboard's plain text
//!   (`CF_UNICODETEXT`), HTML (`HTML Format`, CF_HTML, with its header,
//!   which the app skips), and Rich Text Format.
//! - **macOS and Linux:** [`System`] reads the plain text and the HTML
//!   through `arboard`, as the terminal reader does: `public.html` from
//!   the macOS pasteboard, and `text/html` from the Wayland (data-control)
//!   or X11 clipboard. RTF is not read there; word processors put HTML
//!   beside it.
//!
//! The app is given it with `App::set_clipboard`, so Ctrl+V (Cmd+V), the
//! context menu's Paste, and Paste as plain text all read it. Without
//! HTML, the plain text is pasted.
//!
//! Copying goes through Masonry's clipboard, except for a secret (the join
//! code, the join link; security QW4), which [`write_private`] puts on the
//! clipboard itself, marked private: on Windows out of clipboard history
//! (Windows+V) and the cloud clipboard, on macOS concealed for clipboard
//! managers, and on Linux marked for clipboard managers to skip.

use textweaver_app::clipboard::ClipboardText;
use textweaver_app::{Clipboard, ClipboardContents};

/// How copied text goes on the clipboard ([`write_for`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Write {
    /// Through Masonry's clipboard, as Copy and Cut go.
    Plain(String),
    /// A secret, through [`write_private`].
    Private(String),
}

/// How `copied` goes on the clipboard: a secret privately, anything else
/// as Copy goes.
pub fn write_for(copied: ClipboardText) -> Write {
    if copied.secret {
        Write::Private(copied.text)
    } else {
        Write::Plain(copied.text)
    }
}

/// Puts `text` on the system clipboard marked private: out of clipboard
/// history and the cloud clipboard where the system allows it.
///
/// # Errors
///
/// Why the clipboard could not be opened or written.
pub fn write_private(text: &str) -> Result<(), String> {
    imp::write_private(text)
}

/// The system clipboard, read each time the app pastes.
#[derive(Debug, Default)]
pub struct System;

impl Clipboard for System {
    fn read(&mut self) -> ClipboardContents {
        imp::read()
    }

    /// The clipboard's image, for Join from a QR image: on Windows its PNG
    /// or bitmap, on macOS and Linux through arboard
    /// (`textweaver_app::clipboard_image`).
    fn read_image(&mut self) -> Option<textweaver_app::ClipboardImage> {
        textweaver_app::clipboard_image::read_system_image()
    }
}

/// What the paste key pastes: the clipboard as [`System`] read it, with
/// the plain text the window handed over when the read has none (the
/// clipboard could not be opened, or holds no text form textweaver reads).
pub fn with_window_text(read: ClipboardContents, window_text: String) -> ClipboardContents {
    if read.text.as_deref().is_some_and(|t| !t.is_empty()) {
        return read;
    }
    ClipboardContents {
        text: Some(window_text),
        ..read
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
    windows_core::link!("user32.dll" "system" fn EmptyClipboard() -> i32);
    windows_core::link!("user32.dll" "system" fn SetClipboardData(format: u32, mem: isize) -> isize);
    windows_core::link!("kernel32.dll" "system" fn GlobalAlloc(flags: u32, bytes: usize) -> isize);
    windows_core::link!("kernel32.dll" "system" fn GlobalFree(mem: isize) -> isize);

    const CF_UNICODETEXT: u32 = 13;
    /// `GMEM_MOVEABLE`, as the clipboard wants its blocks.
    const GMEM_MOVEABLE: u32 = 0x0002;

    /// Puts `text` on the clipboard with the two formats Windows reads to
    /// keep it out of clipboard history and the cloud clipboard
    /// (`CanIncludeInClipboardHistory` and `CanUploadToCloudClipboard`,
    /// each a zero DWORD), as arboard's `exclude_from_history` and
    /// `exclude_from_cloud` do.
    pub fn write_private(text: &str) -> Result<(), String> {
        let wide: Vec<u16> = text.encode_utf16().chain([0]).collect();
        let bytes: Vec<u8> = wide.iter().flat_map(|u| u.to_le_bytes()).collect();
        let zero = 0u32.to_le_bytes();
        let marks = ["CanIncludeInClipboardHistory", "CanUploadToCloudClipboard"];
        // SAFETY: the clipboard is opened with no owner window and closed
        // before returning; each block is allocated, filled while locked,
        // and handed to the system, which then owns it (or freed when the
        // system refuses it).
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
                return Err("the clipboard is in use by another program".into());
            }
            let mut result = if EmptyClipboard() != 0 {
                put(CF_UNICODETEXT, &bytes)
            } else {
                Err("the clipboard could not be emptied".into())
            };
            for mark in marks {
                if result.is_ok() {
                    result = match format_id(mark) {
                        Some(f) => put(f, &zero),
                        None => Err(format!("no clipboard format {mark}")),
                    };
                }
            }
            CloseClipboard();
            result
        }
    }

    /// Hands the clipboard a copy of `bytes` in `format`.
    ///
    /// # Safety
    ///
    /// The clipboard must be open and emptied by this program.
    unsafe fn put(format: u32, bytes: &[u8]) -> Result<(), String> {
        // SAFETY: the block is `bytes.len()` long, locked while it is
        // filled, and owned by the system once SetClipboardData succeeds.
        unsafe {
            let mem = GlobalAlloc(GMEM_MOVEABLE, bytes.len());
            if mem == 0 {
                return Err("out of memory for the clipboard".into());
            }
            let ptr = GlobalLock(mem).cast::<u8>();
            if ptr.is_null() {
                GlobalFree(mem);
                return Err("the clipboard block could not be locked".into());
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            GlobalUnlock(mem);
            if SetClipboardData(format, mem) == 0 {
                GlobalFree(mem);
                return Err("the clipboard refused the text".into());
            }
            Ok(())
        }
    }

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
    use std::cell::RefCell;

    use textweaver_app::ClipboardContents;

    thread_local! {
        /// The clipboard a private copy went through, kept open: on X11
        /// the copied text lives as long as the clipboard object does.
        static KEPT: RefCell<Option<arboard::Clipboard>> = const { RefCell::new(None) };
    }

    /// Puts `text` on the clipboard through arboard, concealed on macOS
    /// and marked for Linux clipboard managers to skip.
    pub fn write_private(text: &str) -> Result<(), String> {
        KEPT.with(|kept| {
            let mut kept = kept.borrow_mut();
            if kept.is_none() {
                *kept = Some(arboard::Clipboard::new().map_err(|e| e.to_string())?);
            }
            let Some(clip) = kept.as_mut() else {
                return Err("no clipboard".into());
            };
            let set = clip.set();
            #[cfg(target_os = "macos")]
            let set = {
                use arboard::SetExtApple;
                set.exclude_from_history()
            };
            #[cfg(all(
                unix,
                not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
            ))]
            let set = {
                use arboard::SetExtLinux;
                set.exclude_from_history()
            };
            set.text(text.to_owned()).map_err(|e| e.to_string())
        })
    }

    /// The clipboard's text and HTML through `arboard`. Empty when the
    /// clipboard cannot be opened (with no display, for one).
    pub fn read() -> ClipboardContents {
        let Ok(mut c) = arboard::Clipboard::new() else {
            return ClipboardContents::default();
        };
        ClipboardContents {
            text: c.get_text().ok(),
            html: c.get().html().ok().filter(|h| !h.trim().is_empty()),
            rtf: None,
        }
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

    /// Security QW4: a secret goes the private way, anything else as Copy
    /// goes. (Neither touches the clipboard here.)
    #[test]
    fn a_secret_is_written_privately() {
        let secret = ClipboardText {
            text: "ABCD-EFGH".into(),
            secret: true,
        };
        assert_eq!(write_for(secret), Write::Private("ABCD-EFGH".into()));
        let plain = ClipboardText {
            text: "words".into(),
            secret: false,
        };
        assert_eq!(write_for(plain), Write::Plain("words".into()));
    }

    /// The paste key keeps the clipboard's HTML, and takes the window's
    /// plain text only when the read has none.
    #[test]
    fn window_text_fills_in_a_missing_plain_text() {
        let read = ClipboardContents {
            text: None,
            html: Some("<h1>Hi</h1>".into()),
            rtf: None,
        };
        let merged = with_window_text(read, "Hi".into());
        assert_eq!(merged.text.as_deref(), Some("Hi"));
        assert_eq!(merged.html.as_deref(), Some("<h1>Hi</h1>"));

        let read = ClipboardContents::text("from the clipboard");
        let merged = with_window_text(read.clone(), "from the window".into());
        assert_eq!(merged, read);

        let merged = with_window_text(ClipboardContents::default(), "plain".into());
        assert_eq!(merged, ClipboardContents::text("plain"));
    }
}
