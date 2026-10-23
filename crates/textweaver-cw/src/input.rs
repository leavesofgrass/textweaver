//! Key adapters: MIDI messages and keyboard keys as key contacts.
//!
//! **MIDI adapters** send a note on when a contact closes and a note off
//! (or a note on with velocity 0) when it opens. The notes are settings;
//! the defaults are the ones the owner's adapters document:
//!
//! - the N6ARA TinyMIDI in MIDI mode: dit note 20, dah note 21;
//! - the CTR2 MIDI in normal paddle mode: tip (dit) 20, ring (dah) 21,
//!   always on channel 1; in extended paddle mode, 30 is a straight key
//!   (and 31 PTT, which keying ignores). With its Extended BTN option on,
//!   the paddles move to 96 and 97 (98 and 99 extended), so the notes
//!   must be changed to match.
//!
//! **Keyboard adapters** are keyboards: the VBand and Vail adapters press
//! Left Control for dit and Right Control for dah; the TinyMIDI in
//! keyboard mode types `[` for dit and `]` for dah. Only the textweaver
//! app sees a key's release; the terminal reader keys from MIDI only.
//!
//! Swapping the paddles (for a left-handed operator) swaps dit and dah
//! from every source.

use crate::keyer::Input;

/// The MIDI notes and channel keying listens to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MidiMap {
    /// The dit contact's note.
    pub dit: u8,
    /// The dah contact's note.
    pub dah: u8,
    /// A straight key's note.
    pub straight: u8,
    /// The channel, 1 to 16, or 0 for any.
    pub channel: u8,
    /// Dit and dah swapped.
    pub swap: bool,
}

/// The default dit note (TinyMIDI and CTR2 MIDI).
pub const DEFAULT_DIT_NOTE: u8 = 20;
/// The default dah note.
pub const DEFAULT_DAH_NOTE: u8 = 21;
/// The default straight key note (CTR2 MIDI extended paddle mode).
pub const DEFAULT_STRAIGHT_NOTE: u8 = 30;

impl Default for MidiMap {
    fn default() -> Self {
        MidiMap {
            dit: DEFAULT_DIT_NOTE,
            dah: DEFAULT_DAH_NOTE,
            straight: DEFAULT_STRAIGHT_NOTE,
            channel: 0,
            swap: false,
        }
    }
}

/// The swapped contact, when `swap`.
fn swapped(input: Input, swap: bool) -> Input {
    match (input, swap) {
        (Input::Dit, true) => Input::Dah,
        (Input::Dah, true) => Input::Dit,
        (other, _) => other,
    }
}

impl MidiMap {
    /// The contact a MIDI message closes (true) or opens, if it is one of
    /// the notes on the channel; anything else is `None`.
    pub fn contact(&self, bytes: &[u8]) -> Option<(Input, bool)> {
        let (&status, rest) = bytes.split_first()?;
        let (&note, rest) = rest.split_first()?;
        let velocity = rest.first().copied().unwrap_or(0);
        let channel = (status & 0x0F) + 1;
        if self.channel != 0 && channel != self.channel {
            return None;
        }
        let down = match status & 0xF0 {
            0x90 => velocity > 0,
            0x80 => false,
            _ => return None,
        };
        let input = if note == self.dit {
            Input::Dit
        } else if note == self.dah {
            Input::Dah
        } else if note == self.straight {
            Input::Straight
        } else {
            return None;
        };
        Some((swapped(input, self.swap), down))
    }
}

/// A key a keyboard adapter presses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AdapterKey {
    /// Left Control (VBand and Vail adapters: dit).
    LeftControl,
    /// Right Control (dah).
    RightControl,
    /// The `[` key (TinyMIDI keyboard mode: dit).
    LeftBracket,
    /// The `]` key (dah).
    RightBracket,
}

impl AdapterKey {
    /// The contact the key is, with the paddles swapped when `swap`.
    pub fn contact(self, swap: bool) -> Input {
        let input = match self {
            AdapterKey::LeftControl | AdapterKey::LeftBracket => Input::Dit,
            AdapterKey::RightControl | AdapterKey::RightBracket => Input::Dah,
        };
        swapped(input, swap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tinymidi_and_ctr2_notes_map_to_contacts() {
        let m = MidiMap::default();
        // Note on and note off, channel 1 (the CTR2's).
        assert_eq!(m.contact(&[0x90, 20, 127]), Some((Input::Dit, true)));
        assert_eq!(m.contact(&[0x80, 20, 0]), Some((Input::Dit, false)));
        assert_eq!(m.contact(&[0x90, 21, 100]), Some((Input::Dah, true)));
        // A note on with velocity 0 is a note off.
        assert_eq!(m.contact(&[0x90, 21, 0]), Some((Input::Dah, false)));
        // Extended paddle mode: the straight key; PTT is ignored.
        assert_eq!(m.contact(&[0x90, 30, 127]), Some((Input::Straight, true)));
        assert_eq!(m.contact(&[0x90, 31, 127]), None);
        // Any channel by default.
        assert_eq!(m.contact(&[0x95, 20, 1]), Some((Input::Dit, true)));
        // Knobs (controllers) and other notes are not contacts.
        assert_eq!(m.contact(&[0xB0, 100, 65]), None);
        assert_eq!(m.contact(&[0x90, 60, 127]), None);
        assert_eq!(m.contact(&[0x90]), None);
        assert_eq!(m.contact(&[]), None);
    }

    #[test]
    fn channel_one_only_when_set() {
        let m = MidiMap {
            channel: 1,
            ..MidiMap::default()
        };
        assert_eq!(m.contact(&[0x90, 20, 127]), Some((Input::Dit, true)));
        assert_eq!(m.contact(&[0x91, 20, 127]), None);
    }

    #[test]
    fn extended_btn_notes_come_from_the_settings() {
        // CTR2 Extended BTN mode moves the paddles to 96 and 97.
        let m = MidiMap {
            dit: 96,
            dah: 97,
            straight: 98,
            channel: 1,
            swap: false,
        };
        assert_eq!(m.contact(&[0x90, 96, 127]), Some((Input::Dit, true)));
        assert_eq!(m.contact(&[0x80, 97, 64]), Some((Input::Dah, false)));
        assert_eq!(m.contact(&[0x90, 98, 127]), Some((Input::Straight, true)));
        // The old notes are now buttons, not paddles.
        assert_eq!(m.contact(&[0x90, 20, 127]), None);
    }

    #[test]
    fn swapping_swaps_dit_and_dah_only() {
        let m = MidiMap {
            swap: true,
            ..MidiMap::default()
        };
        assert_eq!(m.contact(&[0x90, 20, 127]), Some((Input::Dah, true)));
        assert_eq!(m.contact(&[0x90, 21, 127]), Some((Input::Dit, true)));
        assert_eq!(m.contact(&[0x90, 30, 127]), Some((Input::Straight, true)));
    }

    #[test]
    fn keyboard_adapters_map_down_and_up_keys() {
        assert_eq!(AdapterKey::LeftControl.contact(false), Input::Dit);
        assert_eq!(AdapterKey::RightControl.contact(false), Input::Dah);
        assert_eq!(AdapterKey::LeftBracket.contact(false), Input::Dit);
        assert_eq!(AdapterKey::RightBracket.contact(false), Input::Dah);
        assert_eq!(AdapterKey::LeftControl.contact(true), Input::Dah);
        assert_eq!(AdapterKey::RightBracket.contact(true), Input::Dit);
    }
}
