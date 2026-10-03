//! The caret's blink in the window's text fields follows the system
//! (Wave 9, W9a-w; GPU report QW6): Windows' cursor blink rate, steady
//! when it is set to none. Elsewhere Masonry's one-second cycle stays.
//! Masonry already stops the blink after ten seconds without typing.

use std::time::Duration;

/// The full blink cycle for a system blink time (`GetCaretBlinkTime`, the
/// time the caret is shown or hidden, in milliseconds), or `None` for a
/// steady caret (the system's "none", `INFINITE`, or 0).
pub fn cycle_for(system_ms: u32) -> Option<Duration> {
    match system_ms {
        0 | u32::MAX => None,
        ms => Some(Duration::from_millis(u64::from(ms) * 2)),
    }
}

/// Sets every text field's caret blink from the system's setting.
pub fn follow_system() {
    if let Some(ms) = imp::system_blink_ms() {
        masonry::widgets::set_caret_blink_period(cycle_for(ms));
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod imp {
    pub(super) fn system_blink_ms() -> Option<u32> {
        // SAFETY: no arguments; reads the user's setting.
        Some(unsafe { windows::Win32::UI::WindowsAndMessaging::GetCaretBlinkTime() })
    }
}

#[cfg(not(windows))]
mod imp {
    pub(super) fn system_blink_ms() -> Option<u32> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_means_steady() {
        assert_eq!(cycle_for(u32::MAX), None);
        assert_eq!(cycle_for(0), None);
        assert_eq!(cycle_for(530), Some(Duration::from_millis(1060)));
    }
}
