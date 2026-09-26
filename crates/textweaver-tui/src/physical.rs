//! Which physical key typed a character: the digit row, whatever the
//! keyboard layout.
//!
//! The browse keys `1` to `6` (next heading at that level) and Shift with
//! them (previous) follow the physical digit key, as NVDA's and JAWS's
//! browse mode do. crossterm reports only the character typed, so:
//!
//! - **On Windows** (Windows Terminal and the classic console alike, both
//!   of which deliver console input records), the event loop waits on the
//!   console's input handle itself and, before crossterm reads a record,
//!   peeks at the pending records ([`peek_console`]). Each key-down record
//!   carries its virtual key code; `VK_1` to `VK_6` are the digit row on
//!   every layout, including French AZERTY, where the row types `& é " '
//!   ( -` without Shift. [`DigitKeys`] keeps what the peek saw, and the key
//!   handler matches each character event with it.
//! - **Elsewhere**, and on Windows when nothing was peeked,
//!   `textweaver_keymap::digits::from_typed` recognises the shifted digits
//!   of the US, UK, German, Spanish, Nordic, and Italian layouts, and with
//!   `[keyboard] digit_row = "azerty"` the French digit row.
//!
//! crossterm's keyboard enhancement flags were considered and not used:
//! the kitty keyboard protocol's alternate keys are not exposed by
//! crossterm's key events, and neither Windows Terminal nor the classic
//! console speaks that protocol.

use std::collections::VecDeque;

/// A key-down seen in the console's input: the character it types, and
/// the digit when it is a digit-row key (with whether Shift was down).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeekedKey {
    /// The character the key typed.
    pub ch: char,
    /// The digit-row key, 0 to 9, and whether Shift was down; `None` for
    /// any other key.
    pub digit: Option<(u8, bool)>,
}

/// Key-downs peeked from the console, waiting to be matched with the
/// character events crossterm reads next.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DigitKeys {
    pending: VecDeque<PeekedKey>,
}

impl DigitKeys {
    /// Replaces what is pending with a fresh peek of the console's input
    /// (only records crossterm has not read yet are there).
    pub fn set_pending(&mut self, keys: impl IntoIterator<Item = PeekedKey>) {
        self.pending = keys.into_iter().collect();
    }

    /// What the console said about the key that typed `ch`: `Some(digit)`
    /// when it was peeked (`digit` is `None` for a key off the digit row),
    /// `None` when nothing was peeked for it. Entries before it are
    /// dropped.
    pub fn take(&mut self, ch: char) -> Option<Option<(u8, bool)>> {
        let i = self.pending.iter().position(|k| k.ch == ch)?;
        let key = self.pending.drain(..=i).last()?;
        Some(key.digit)
    }

    /// True when nothing is pending.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

/// Waits up to `wait` for console input, then peeks at the key-downs not
/// yet read. `None` when the console cannot be read this way (input
/// redirected, or not Windows); the caller then waits with crossterm.
#[cfg(windows)]
#[allow(unsafe_code)]
pub fn peek_console(wait: std::time::Duration) -> Option<(bool, Vec<PeekedKey>)> {
    use windows::Win32::Foundation::WAIT_OBJECT_0;
    use windows::Win32::System::Console::{
        GetStdHandle, INPUT_RECORD, KEY_EVENT, PeekConsoleInputW, SHIFT_PRESSED, STD_INPUT_HANDLE,
    };
    use windows::Win32::System::Threading::WaitForSingleObject;

    // SAFETY: GetStdHandle has no preconditions.
    let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) }.ok()?;
    if handle.is_invalid() {
        return None;
    }
    let ms = u32::try_from(wait.as_millis()).unwrap_or(u32::MAX);
    // SAFETY: the handle is the process's standard input.
    let ready = unsafe { WaitForSingleObject(handle, ms) } == WAIT_OBJECT_0;
    if !ready {
        return Some((false, Vec::new()));
    }
    let mut records = [INPUT_RECORD::default(); 32];
    let mut read = 0u32;
    // SAFETY: the buffer holds 32 records and `read` receives the count.
    let ok = unsafe { PeekConsoleInputW(handle, &mut records, &mut read) }.is_ok();
    if !ok {
        return None;
    }
    let n = usize::try_from(read).unwrap_or(0).min(records.len());
    let mut keys = Vec::new();
    for r in &records[..n] {
        if u32::from(r.EventType) != KEY_EVENT {
            continue;
        }
        // SAFETY: EventType says the union holds a key event.
        let k = unsafe { r.Event.KeyEvent };
        if !k.bKeyDown.as_bool() {
            continue;
        }
        // SAFETY: UnicodeChar is the field crossterm reads as well.
        let unit = unsafe { k.uChar.UnicodeChar };
        let Some(ch) = char::from_u32(u32::from(unit)).filter(|c| !c.is_control()) else {
            continue;
        };
        let shift = k.dwControlKeyState & SHIFT_PRESSED != 0;
        let digit = textweaver_app::keymap::digits::digit_of_virtual_key(k.wVirtualKeyCode)
            .map(|d| (d, shift));
        keys.push(PeekedKey { ch, digit });
    }
    Some((true, keys))
}

/// Not Windows: nothing to peek.
#[cfg(not(windows))]
pub fn peek_console(_wait: std::time::Duration) -> Option<(bool, Vec<PeekedKey>)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_match_peeked_keys_in_order() {
        let mut k = DigitKeys::default();
        k.set_pending([
            PeekedKey {
                ch: 'x',
                digit: None,
            },
            PeekedKey {
                ch: '&',
                digit: Some((1, false)),
            },
            PeekedKey {
                ch: '!',
                digit: Some((1, true)),
            },
        ]);
        assert_eq!(k.take('x'), Some(None));
        assert_eq!(k.take('&'), Some(Some((1, false))));
        assert_eq!(k.take('q'), None, "not peeked");
        assert_eq!(k.take('!'), Some(Some((1, true))));
        assert!(k.is_empty());
        k.set_pending([PeekedKey { ch: 'a', digit: None }]);
        k.set_pending([]);
        assert_eq!(k.take('a'), None);
    }
}
