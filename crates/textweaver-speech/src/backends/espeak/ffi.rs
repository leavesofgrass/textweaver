//! The only unsafe code in the crate: calls into libespeak-ng through
//! the declarations in `sys` (ADR-0001). Every function here is safe to call; each
//! `unsafe` block states why it is sound.
//!
//! libespeak-ng is a process-wide singleton. Callers serialize access (the
//! backend's worker thread owns synthesis; the `ENGINE` lock in the parent
//! module guards initialization), except [`cancel`], which libespeak-ng
//! documents as callable while another thread waits in `espeak_Synchronize`
//! in playback mode.

#![allow(unsafe_code)]

use std::ffi::{CStr, CString, c_char, c_int, c_short, c_void};
use std::sync::Mutex;

use super::sys;

/// Output mode for [`initialize`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mode {
    /// libespeak-ng plays audio itself (asynchronously).
    Playback,
    /// Audio comes back through the callback; nothing is played.
    Retrieval,
}

/// Event kinds the backend uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EventKind {
    Word,
    MsgTerminated,
    Other,
}

/// One synthesis event, copied out of libespeak-ng's buffer.
#[derive(Clone, Copy, Debug)]
pub(super) struct Event {
    pub kind: EventKind,
    /// 1-based character position in the text (espeak-ng 1.52 counts
    /// characters, not bytes, for UTF-8 input).
    pub text_position: i32,
    /// Length in characters.
    pub length: i32,
    /// Milliseconds of audio from the start of the message.
    pub audio_ms: i32,
}

/// Receives audio samples and events; returns true to abort synthesis.
pub(super) type Sink = Box<dyn FnMut(&[i16], &[Event]) -> bool + Send>;

static SINK: Mutex<Option<Sink>> = Mutex::new(None);

/// Installs (or removes) the receiver for the synthesis callback.
pub(super) fn set_sink(sink: Option<Sink>) {
    *SINK.lock().unwrap_or_else(|p| p.into_inner()) = sink;
}

unsafe extern "C" fn synth_callback(
    wav: *mut c_short,
    numsamples: c_int,
    events: *mut sys::espeak_EVENT,
) -> c_int {
    let samples: &[i16] = match usize::try_from(numsamples) {
        // SAFETY: libespeak-ng passes `numsamples` valid 16-bit samples at
        // `wav` for the duration of this call when `wav` is non-null.
        Ok(n) if n > 0 && !wav.is_null() => unsafe { std::slice::from_raw_parts(wav, n) },
        _ => &[],
    };
    let mut out = Vec::new();
    if !events.is_null() {
        let mut p = events;
        loop {
            // SAFETY: `events` is an array terminated by an event of type
            // LIST_TERMINATED, valid for the duration of this call; we stop
            // at the terminator and never read past it.
            let e = unsafe { *p };
            if e.type_ == sys::espeak_EVENT_TYPE_espeakEVENT_LIST_TERMINATED {
                break;
            }
            let kind = match e.type_ {
                sys::espeak_EVENT_TYPE_espeakEVENT_WORD => EventKind::Word,
                sys::espeak_EVENT_TYPE_espeakEVENT_MSG_TERMINATED => EventKind::MsgTerminated,
                _ => EventKind::Other,
            };
            out.push(Event {
                kind,
                text_position: e.text_position,
                length: e.length,
                audio_ms: e.audio_position,
            });
            // SAFETY: the terminator has not been reached, so the next
            // element is still inside the array.
            p = unsafe { p.add(1) };
        }
    }
    let mut guard = SINK.lock().unwrap_or_else(|p| p.into_inner());
    match guard.as_mut() {
        Some(sink) => c_int::from(sink(samples, &out)),
        None => 0,
    }
}

/// Lets libespeak-ng's own threads use COM, before it plays audio.
///
/// In playback mode libespeak-ng opens and writes the audio device
/// (WASAPI, through pcaudiolib) from threads it creates itself, and those
/// threads never initialize COM. With no apartment, opening the device
/// fails with "CoInitialize has not been called." and a later write uses
/// the device that never opened: the process crashes with an access
/// violation. Keeping the multithreaded apartment alive for the whole
/// process makes every thread without an apartment of its own an implicit
/// member of it, so the device opens. Done once and never undone, since
/// libespeak-ng stays loaded for the life of the process.
#[cfg(all(windows, feature = "espeak"))]
fn keep_com_available() {
    static MTA: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    MTA.get_or_init(|| {
        // SAFETY: CoIncrementMTAUsage has no preconditions and may be called
        // from any thread; the cookie is deliberately never handed to
        // CoDecrementMTAUsage, so the apartment outlives every audio thread.
        if let Err(e) = unsafe { windows::Win32::System::Com::CoIncrementMTAUsage() } {
            log::warn!("espeak: cannot keep COM available for audio output: {e}");
        }
    });
}

/// COM exists only on Windows, and only the backend (feature `espeak`)
/// plays audio.
#[cfg(not(all(windows, feature = "espeak")))]
fn keep_com_available() {}

/// Initializes libespeak-ng in `mode` and registers the callback. Returns
/// the sample rate.
pub(super) fn initialize(mode: Mode) -> Result<u32, String> {
    let output = match mode {
        Mode::Playback => sys::espeak_AUDIO_OUTPUT_AUDIO_OUTPUT_PLAYBACK,
        Mode::Retrieval => sys::espeak_AUDIO_OUTPUT_AUDIO_OUTPUT_RETRIEVAL,
    };
    // libespeak-ng is loaded at run time; say why when it is missing.
    sys::load()?;
    if mode == Mode::Playback {
        keep_com_available();
    }
    // SAFETY: a null path selects the installed data directory; options 0
    // disables phoneme events. Callers serialize initialization.
    let rate = unsafe { sys::espeak_Initialize(output, 0, std::ptr::null(), 0) };
    let rate = u32::try_from(rate).map_err(|_| "espeak-ng failed to initialize".to_owned())?;
    // SAFETY: `synth_callback` has the signature libespeak-ng expects and
    // lives for the whole program.
    unsafe { sys::espeak_SetSynthCallback(Some(synth_callback)) };
    Ok(rate)
}

/// Shuts libespeak-ng down (before re-initializing in another mode).
pub(super) fn terminate() {
    // SAFETY: valid after a successful initialize; callers serialize it.
    unsafe { sys::espeak_Terminate() };
}

fn ok(code: sys::espeak_ERROR, what: &str) -> Result<(), String> {
    if code == sys::espeak_ERROR_EE_OK {
        Ok(())
    } else {
        Err(format!("espeak-ng {what} failed ({code})"))
    }
}

/// Selects a voice by name or identifier ("en-us", "gmw/en-US").
pub(super) fn set_voice(name: &str) -> Result<(), String> {
    let c = CString::new(name).map_err(|_| format!("bad voice name {name:?}"))?;
    // SAFETY: `c` is a valid NUL-terminated string that outlives the call.
    ok(unsafe { sys::espeak_SetVoiceByName(c.as_ptr()) }, "voice")
}

/// Translates UTF-8 `text` into IPA phonemes, clause by clause, for the
/// voice selected with [`set_voice`]. Each clause's phonemes are returned
/// in order; libespeak-ng drops the punctuation that ended each clause.
pub(super) fn text_to_phonemes(text: &str) -> Result<Vec<String>, String> {
    if !sys::has_text_to_phonemes() {
        return Err("this libespeak-ng has no espeak_TextToPhonemes".to_owned());
    }
    let c = CString::new(text.replace('\0', " ")).map_err(|e| e.to_string())?;
    let mut cursor: *const c_void = c.as_ptr().cast::<c_void>();
    let mut clauses = Vec::new();
    // A clause is at least one character, so the loop ends; the bound
    // guards against a library that never advances the pointer.
    for _ in 0..=text.len() {
        if cursor.is_null() {
            break;
        }
        // SAFETY: the engine is initialized (callers hold the engine lock
        // after initializing it); `cursor` points into `c`, a
        // NUL-terminated UTF-8 buffer that outlives the loop, and
        // libespeak-ng only moves it forward within that buffer or sets it
        // to null. The returned string is copied before the next call.
        let out = unsafe {
            sys::espeak_TextToPhonemes(
                &raw mut cursor,
                sys::espeakCHARS_UTF8 as c_int,
                sys::espeakPHONEMES_IPA,
            )
        };
        if out.is_null() {
            break;
        }
        // SAFETY: a non-null result is a NUL-terminated string owned by
        // libespeak-ng, valid until the next call.
        let clause = unsafe { CStr::from_ptr(out) }
            .to_string_lossy()
            .into_owned();
        clauses.push(clause);
    }
    Ok(clauses)
}

/// Parameters libespeak-ng takes.
#[derive(Clone, Copy, Debug)]
pub(super) enum Param {
    Rate,
    Volume,
    Pitch,
}

/// Sets a parameter to an absolute value.
pub(super) fn set_param(param: Param, value: i32) -> Result<(), String> {
    let p = match param {
        Param::Rate => sys::espeak_PARAMETER_espeakRATE,
        Param::Volume => sys::espeak_PARAMETER_espeakVOLUME,
        Param::Pitch => sys::espeak_PARAMETER_espeakPITCH,
    };
    // SAFETY: plain value call; `relative = 0` means absolute.
    ok(
        unsafe { sys::espeak_SetParameter(p, value, 0) },
        "parameter",
    )
}

/// Synthesizes UTF-8 `text`. Returns when queued (playback) or when
/// synthesis is done (retrieval).
pub(super) fn synth(text: &str) -> Result<(), String> {
    let c = CString::new(text.replace('\0', " ")).map_err(|e| e.to_string())?;
    let bytes = c.as_bytes_with_nul();
    let flags = sys::espeakCHARS_UTF8 | sys::espeakENDPAUSE;
    // SAFETY: `c` is a NUL-terminated UTF-8 buffer of `bytes.len()` bytes
    // that outlives the call (libespeak-ng copies it before returning in
    // playback mode); the out-pointer and user data may be null.
    let code = unsafe {
        sys::espeak_Synth(
            c.as_ptr().cast::<c_void>(),
            bytes.len(),
            0,
            sys::espeak_POSITION_TYPE_POS_CHARACTER,
            0,
            flags,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    ok(code, "synthesis")
}

/// Speaks one character by name.
pub(super) fn synth_char(c: char) -> Result<(), String> {
    let wide = u32::from(c);
    // SAFETY: plain value call with a Unicode code point.
    ok(
        unsafe { sys::espeak_Char(wide as sys::wchar_t) },
        "character",
    )
}

/// Waits until everything queued has been spoken.
pub(super) fn synchronize() {
    // SAFETY: plain call; blocks the calling (worker) thread only.
    unsafe { sys::espeak_Synchronize() };
}

/// Stops synthesis and playback at once (playback mode).
pub(super) fn cancel() {
    // SAFETY: libespeak-ng supports cancelling from another thread while
    // the worker waits in `espeak_Synchronize` (asynchronous playback mode).
    unsafe { sys::espeak_Cancel() };
}

/// A voice as libespeak-ng lists it.
#[derive(Clone, Debug)]
pub(super) struct VoiceInfo {
    pub name: String,
    pub identifier: String,
    pub languages: Vec<String>,
    pub gender: u8,
}

fn string(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    // SAFETY: non-null strings in espeak_VOICE are NUL-terminated and live
    // as long as libespeak-ng's voice list.
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

/// Lists installed voices.
pub(super) fn list_voices() -> Vec<VoiceInfo> {
    // SAFETY: a null spec lists every voice; the result is a
    // null-terminated array owned by libespeak-ng.
    let list = unsafe { sys::espeak_ListVoices(std::ptr::null_mut()) };
    let mut out = Vec::new();
    if list.is_null() {
        return out;
    }
    let mut i = 0;
    loop {
        // SAFETY: the array is terminated by a null pointer; `i` never
        // passes it.
        let v = unsafe { *list.add(i) };
        if v.is_null() {
            break;
        }
        // SAFETY: non-null entries point to valid espeak_VOICE structs.
        let voice = unsafe { *v };
        out.push(VoiceInfo {
            name: string(voice.name),
            identifier: string(voice.identifier),
            languages: languages(voice.languages),
            gender: voice.gender,
        });
        i += 1;
    }
    out
}

/// Parses espeak's language list: repeated (priority byte, NUL-terminated
/// name), ended by a zero priority byte.
fn languages(p: *const c_char) -> Vec<String> {
    let mut out = Vec::new();
    if p.is_null() {
        return out;
    }
    let mut cur = p;
    loop {
        // SAFETY: the list is a sequence of (priority byte, C string) pairs
        // ended by a zero byte; we read one byte, then one C string.
        let priority = unsafe { *cur };
        if priority == 0 {
            break;
        }
        // SAFETY: as above; the name starts right after the priority byte.
        let name = unsafe { CStr::from_ptr(cur.add(1)) };
        let bytes = name.to_bytes();
        if bytes.is_empty() {
            break;
        }
        out.push(String::from_utf8_lossy(bytes).into_owned());
        // SAFETY: skip the priority byte, the name, and its NUL.
        cur = unsafe { cur.add(1 + bytes.len() + 1) };
    }
    out
}
