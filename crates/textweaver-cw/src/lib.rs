//! CW (Morse code) for textweaver.
//!
//! Everything here is data in, data out: no audio device, no clock, no
//! threads (apart from the small shared [`sending()`] record that the speech
//! backend writes and the status line reads). The speech backend in
//! `textweaver-engines` plays what this crate renders, and audio export
//! writes it to a file.
//!
//! - [`table`]: the International Morse code of ITU-R M.1677-1 (letters,
//!   figures, punctuation and the procedure signals, or prosigns), plus a
//!   few signs in wide use that the recommendation does not define.
//! - [`timing`]: the PARIS standard and Farnsworth spacing, with the ARRL
//!   (QEX, KE3Z) formula for the stretched gaps.
//! - [`schedule()`]: a text's elements, characters and words in time, from
//!   the text exactly as written. Word boundaries are exact by
//!   construction, so the reading highlight and RSVP follow the word being
//!   sent, to the sample.
//! - [`render()`]: the schedule as 16-bit mono samples of a sine tone,
//!   each element shaped with a raised-cosine rise and fall (about 5 ms)
//!   so it does not click. The arithmetic is plain IEEE floating point
//!   with a sine of our own, so the samples are the same on every
//!   platform.
//! - [`mod@sending`]: what is being sent now, for the status line (and so for
//!   a Braille display through the screen reader).
//!
//! Keying, Morse code typed with a key or paddle:
//!
//! - [`input`]: MIDI messages and keyboard keys from key adapters (the
//!   N6ARA TinyMIDI, the CTR2 MIDI, VBand and Vail) as key contacts.
//! - [`keyer`]: an iambic keyer (modes A and B, dot and dash memory) and
//!   straight keying, on a virtual clock.
//! - [`decode`]: tone timing to characters and word spaces, adapting to
//!   the operator's speed.
//! - [`mod@sidetone`]: the tone heard while keying, made sample by sample.

pub mod decode;
pub mod input;
pub mod keyer;
pub mod render;
pub mod schedule;
pub mod sending;
pub mod sidetone;
pub mod table;
pub mod timing;

pub use decode::{Decoded, Decoder, Keyed, decode};
pub use input::{AdapterKey, MidiMap};
pub use keyer::{Input, Keyer, KeyerMode, KeyerOptions, ToneEdge};
pub use render::{RenderOptions, render, to_samples};
pub use schedule::{Options, Schedule, SentChar, SentWord, Tone, schedule};
pub use sending::{Sending, SendingSnapshot, sending};
pub use sidetone::Sidetone;
pub use table::{PROSIGNS, Symbol, code, prosign};
pub use timing::{DEFAULT_CHAR_WPM, DEFAULT_EFFECTIVE_WPM, MAX_WPM, MIN_WPM, Timing};
