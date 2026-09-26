//! Hand-written declarations for the part of libespeak-ng's C API
//! (`espeak-ng/speak_lib.h`, stable since eSpeak 1.48) that the backend
//! uses.
//!
//! These replace `espeakng-sys`, whose build script ran bindgen over the
//! system headers. `speak_lib.h` includes `<stdio.h>`, so bindgen also
//! translated glibc's `struct _IO_FILE`, and bindgen 0.71's layout checks
//! for it failed to compile against the glibc of Fedora 44 and current Arch
//! (Agent X found it; Agent D4 reproduced it in `fedora:latest` and
//! `archlinux:latest`). Nothing here needs `FILE`, and without bindgen the
//! `espeak` feature no longer needs clang or libclang to build.
//!
//! Names follow bindgen's, so the calling code reads as before. Layouts are
//! checked by the tests below against the C definitions (`int`-sized enums;
//! the event's `id` union is 8 bytes).

#![allow(non_camel_case_types, non_upper_case_globals, unsafe_code)]

use std::ffi::{c_char, c_int, c_short, c_uchar, c_uint, c_void};

/// `espeak_EVENT_TYPE` (a C enum: `int`-sized).
pub type espeak_EVENT_TYPE = c_uint;
/// Retrieval mode: the end of the event list.
pub const espeak_EVENT_TYPE_espeakEVENT_LIST_TERMINATED: espeak_EVENT_TYPE = 0;
/// The start of a word.
pub const espeak_EVENT_TYPE_espeakEVENT_WORD: espeak_EVENT_TYPE = 1;
/// The end of a message.
pub const espeak_EVENT_TYPE_espeakEVENT_MSG_TERMINATED: espeak_EVENT_TYPE = 6;

/// `espeak_POSITION_TYPE`.
pub type espeak_POSITION_TYPE = c_uint;
/// Positions count characters.
pub const espeak_POSITION_TYPE_POS_CHARACTER: espeak_POSITION_TYPE = 1;

/// `espeak_AUDIO_OUTPUT`.
pub type espeak_AUDIO_OUTPUT = c_uint;
/// libespeak-ng plays the audio asynchronously.
pub const espeak_AUDIO_OUTPUT_AUDIO_OUTPUT_PLAYBACK: espeak_AUDIO_OUTPUT = 0;
/// Audio comes back through the synthesis callback.
pub const espeak_AUDIO_OUTPUT_AUDIO_OUTPUT_RETRIEVAL: espeak_AUDIO_OUTPUT = 1;

/// `espeak_ERROR` (has a negative member, so `int`).
pub type espeak_ERROR = c_int;
/// Success.
pub const espeak_ERROR_EE_OK: espeak_ERROR = 0;

/// `espeak_PARAMETER`.
pub type espeak_PARAMETER = c_uint;
/// Speaking rate in words per minute.
pub const espeak_PARAMETER_espeakRATE: espeak_PARAMETER = 1;
/// Volume, 0 to 200.
pub const espeak_PARAMETER_espeakVOLUME: espeak_PARAMETER = 2;
/// Base pitch, 0 to 100.
pub const espeak_PARAMETER_espeakPITCH: espeak_PARAMETER = 3;

/// `espeak_Synth` flag: the text is UTF-8.
pub const espeakCHARS_UTF8: c_uint = 1;
/// `espeak_Synth` flag: add a sentence pause at the end.
pub const espeakENDPAUSE: c_uint = 0x1000;

/// C's `wchar_t`: 32-bit on Unix, 16-bit on Windows.
#[cfg(not(windows))]
pub type wchar_t = i32;
/// C's `wchar_t`: 32-bit on Unix, 16-bit on Windows.
#[cfg(windows)]
pub type wchar_t = u16;

/// The `id` union of an event: a number, a name, or 8 bytes of phoneme.
#[repr(C)]
#[derive(Clone, Copy)]
pub union espeak_EVENT_id {
    /// WORD and SENTENCE events.
    pub number: c_int,
    /// MARK and PLAY events.
    pub name: *const c_char,
    /// PHONEME events.
    pub string: [c_char; 8],
}

/// `espeak_EVENT`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct espeak_EVENT {
    /// The kind of event.
    pub type_: espeak_EVENT_TYPE,
    /// Message identifier (0 for a key or character).
    pub unique_identifier: c_uint,
    /// Characters from the start of the text (1-based).
    pub text_position: c_int,
    /// Word length in characters (WORD events).
    pub length: c_int,
    /// Milliseconds into the message's audio.
    pub audio_position: c_int,
    /// Internal sample id.
    pub sample: c_int,
    /// The pointer given to `espeak_Synth`.
    pub user_data: *mut c_void,
    /// Event data.
    pub id: espeak_EVENT_id,
}

/// `espeak_VOICE`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct espeak_VOICE {
    /// The voice's name (UTF-8).
    pub name: *const c_char,
    /// Pairs of (priority byte, language string), ended by a zero byte.
    pub languages: *const c_char,
    /// The voice file within `espeak-ng-data/voices`.
    pub identifier: *const c_char,
    /// 0 none, 1 male, 2 female.
    pub gender: c_uchar,
    /// Age in years, or 0.
    pub age: c_uchar,
    /// Variant (for `espeak_SetVoiceByProperties`).
    pub variant: c_uchar,
    /// Internal.
    pub xx1: c_uchar,
    /// Internal.
    pub score: c_int,
    /// Internal.
    pub spare: *mut c_void,
}

/// `t_espeak_callback`: samples, their count, and the events; nonzero
/// aborts synthesis.
pub type t_espeak_callback = Option<
    unsafe extern "C" fn(wav: *mut c_short, numsamples: c_int, events: *mut espeak_EVENT) -> c_int,
>;

#[link(name = "espeak-ng")]
unsafe extern "C" {
    /// Starts the engine; returns the sample rate, or -1.
    pub fn espeak_Initialize(
        output: espeak_AUDIO_OUTPUT,
        buflength: c_int,
        path: *const c_char,
        options: c_int,
    ) -> c_int;
    /// Sets the synthesis callback.
    pub fn espeak_SetSynthCallback(callback: t_espeak_callback);
    /// Synthesizes text.
    pub fn espeak_Synth(
        text: *const c_void,
        size: usize,
        position: c_uint,
        position_type: espeak_POSITION_TYPE,
        end_position: c_uint,
        flags: c_uint,
        unique_identifier: *mut c_uint,
        user_data: *mut c_void,
    ) -> espeak_ERROR;
    /// Speaks one character's name.
    pub fn espeak_Char(character: wchar_t) -> espeak_ERROR;
    /// Sets a parameter (absolute when `relative` is 0).
    pub fn espeak_SetParameter(
        parameter: espeak_PARAMETER,
        value: c_int,
        relative: c_int,
    ) -> espeak_ERROR;
    /// Selects a voice by name or file name.
    pub fn espeak_SetVoiceByName(name: *const c_char) -> espeak_ERROR;
    /// The installed voices matching `voice_spec` (null: all), as a
    /// null-terminated array owned by the library.
    pub fn espeak_ListVoices(voice_spec: *mut espeak_VOICE) -> *mut *const espeak_VOICE;
    /// Stops speech at once.
    pub fn espeak_Cancel() -> espeak_ERROR;
    /// Waits until queued speech is done.
    pub fn espeak_Synchronize() -> espeak_ERROR;
    /// Shuts the engine down.
    pub fn espeak_Terminate() -> espeak_ERROR;
}

#[cfg(test)]
mod tests {
    use std::mem::{align_of, offset_of, size_of};

    use super::*;

    /// The C layouts of `speak_lib.h` (as bindgen computed them from the
    /// real header on Debian's espeak-ng 1.52, 64-bit and 32-bit).
    #[test]
    fn layouts_match_speak_lib_h() {
        let ptr = size_of::<*const c_void>();
        assert_eq!(size_of::<espeak_EVENT_id>(), 8);
        assert_eq!(offset_of!(espeak_EVENT, type_), 0);
        assert_eq!(offset_of!(espeak_EVENT, text_position), 8);
        assert_eq!(offset_of!(espeak_EVENT, audio_position), 16);
        assert_eq!(offset_of!(espeak_EVENT, user_data), 24);
        assert_eq!(offset_of!(espeak_EVENT, id), 24 + ptr);
        assert_eq!(size_of::<espeak_EVENT>(), if ptr == 8 { 40 } else { 36 });
        assert_eq!(offset_of!(espeak_VOICE, gender), 3 * ptr);
        assert_eq!(offset_of!(espeak_VOICE, score), 3 * ptr + 4);
        assert_eq!(offset_of!(espeak_VOICE, spare), 3 * ptr + 8);
        assert_eq!(size_of::<espeak_VOICE>(), if ptr == 8 { 40 } else { 24 });
        assert_eq!(align_of::<espeak_VOICE>(), ptr);
    }
}
