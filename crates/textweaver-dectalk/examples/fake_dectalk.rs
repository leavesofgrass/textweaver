//! A stand-in for DECtalk's shared library, for tests: it exports the
//! `TextToSpeech*` functions `textweaver-dectalk-host` calls, with the
//! signatures of DECtalk's published API, and "speaks" a square wave.
//! It contains no DECtalk code and makes no speech.
//!
//! Built as a `cdylib` by `cargo test` (or `cargo build --example
//! fake_dectalk`); `--features fake-stdcall` builds it with the `stdcall`
//! convention (meaningful on 32-bit Windows only).
//!
//! Behavior:
//! - `[:n<letter>]`, `[:rate N]`, `[:dv ap N]`, and `[:index mark N]` are
//!   understood; other commands are ignored. Every other byte of text is
//!   200 samples at 180 words per minute, scaled by the rate; the square
//!   wave's amplitude is `500 + 100 × speaker` (Paul 0 … Kit 8, in
//!   `SPEAKER_T` order) and its period follows the pitch.
//! - Audio fills the buffers added with `TextToSpeechAddBuffer`, in order.
//!   A full buffer goes to the callback given to `TextToSpeechStartupEx`
//!   with its address in `lParam2`; the partly filled one comes back from
//!   `TextToSpeechReturnBuffer`.
//!
//! Environment switches, read at each call:
//! - `FAKE_DECTALK_MARKS=buffer`: index sample numbers count from the start
//!   of each buffer (default: from `TextToSpeechOpenInMemory`, never reset);
//! - `FAKE_DECTALK_MAX_BYTES=N`: use at most `N` bytes of each buffer, so
//!   an utterance spans several;
//! - `FAKE_DECTALK_NO_EX=1`: `TextToSpeechStartupEx` fails, so the host
//!   falls back to `TextToSpeechStartup`;
//! - `FAKE_DECTALK_FAIL=1`: both start-up functions fail;
//! - `FAKE_DECTALK_LOG=PATH`: each spoken text is appended to `PATH`, one
//!   line per call, as its raw bytes.
#![allow(unsafe_code)]
#![allow(non_snake_case)]
// Every export has one safety contract, the one DECtalk documents: live
// handles, valid out pointers, NUL-terminated text, and buffers that stay
// valid until they are handed back.
#![allow(clippy::missing_safety_doc)]

use std::collections::VecDeque;
use std::ffi::{c_char, c_long, c_void};
use std::io::Write;
use std::sync::Mutex;

/// `TTS_INDEX_T`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TtsIndex {
    value: u32,
    sample: u32,
    reserved: u32,
}

/// `TTS_BUFFER_T`.
#[repr(C)]
pub struct TtsBuffer {
    data: *mut c_char,
    phonemes: *mut c_void,
    indices: *mut TtsIndex,
    max_buffer_length: u32,
    max_phonemes: u32,
    max_indices: u32,
    buffer_length: u32,
    phoneme_count: u32,
    index_count: u32,
    reserved: u32,
}

const SAMPLES_PER_BYTE: usize = 200;
const SAMPLE_RATE: usize = 11_025;
const SPEAKERS: &str = "phfdbuwrk";
const DEFAULT_PITCH: [usize; 9] = [122, 89, 155, 110, 208, 240, 200, 106, 306];

struct State {
    /// The callback, as an address (its type depends on the convention).
    callback: usize,
    in_memory: bool,
    /// Added buffers, as addresses.
    buffers: VecDeque<usize>,
    /// Samples since `TextToSpeechOpenInMemory`.
    stream: u64,
    speaker: usize,
    rate: usize,
    pitch: usize,
}

static STATE: Mutex<State> = Mutex::new(State {
    callback: 0,
    in_memory: false,
    buffers: VecDeque::new(),
    stream: 0,
    speaker: 0,
    rate: 180,
    pitch: 0,
});

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

/// A handle DECtalk would allocate; any non-null pointer does.
fn handle() -> *mut c_void {
    static H: u8 = 0;
    (&raw const H).cast_mut().cast()
}

/// Renders `text` to samples and `(index value, sample)` marks.
fn render(st: &mut State, text: &[u8]) -> (Vec<i16>, Vec<(u32, usize)>) {
    let mut samples = Vec::new();
    let mut marks = Vec::new();
    let mut phase = 0usize;
    let mut i = 0;
    while i < text.len() {
        if text[i..].starts_with(b"[:") {
            let end = text[i..]
                .iter()
                .position(|&b| b == b']')
                .map_or(text.len(), |p| i + p);
            let cmd = String::from_utf8_lossy(&text[i + 2..end]).to_ascii_lowercase();
            let num = |prefix: &str| {
                cmd.strip_prefix(prefix)
                    .and_then(|v| v.trim().parse::<usize>().ok())
            };
            if let Some(v) = num("index mark") {
                marks.push((u32::try_from(v).unwrap_or(u32::MAX), samples.len()));
            } else if let Some(v) = num("rate") {
                st.rate = v.clamp(75, 600);
            } else if let Some(v) = num("dv ap") {
                st.pitch = v;
            } else if let Some(letter) = cmd.strip_prefix('n')
                && let Some(n) = SPEAKERS.find(letter.trim())
                && letter.trim().len() == 1
            {
                st.speaker = n;
                st.pitch = 0;
                st.rate = 180;
            }
            i = end + 1;
            continue;
        }
        let per_byte = SAMPLES_PER_BYTE * 180 / st.rate.max(1);
        let pitch = if st.pitch == 0 {
            DEFAULT_PITCH[st.speaker]
        } else {
            st.pitch
        };
        let half = (SAMPLE_RATE / pitch.max(1) / 2).max(1);
        let amp = 500 + 100 * i16::try_from(st.speaker).unwrap_or(0);
        for _ in 0..per_byte {
            phase += 1;
            samples.push(if (phase / half).is_multiple_of(2) {
                amp
            } else {
                -amp
            });
        }
        i += 1;
    }
    (samples, marks)
}

/// Whether buffer `b` has room for another sample.
///
/// # Safety
///
/// `b` is a buffer the host added and has not been handed back.
unsafe fn has_room(b: *const TtsBuffer, cap: u32) -> bool {
    // SAFETY: guaranteed by the caller.
    let b = unsafe { &*b };
    b.buffer_length + 2 <= b.max_buffer_length.min(cap) & !1
}

/// Writes one sample into buffer `b` if it has room; false when full.
///
/// # Safety
///
/// `b` is a buffer the host added and has not been handed back.
unsafe fn push_sample(b: *mut TtsBuffer, s: i16, cap: u32) -> bool {
    // SAFETY: guaranteed by the caller.
    let b = unsafe { &mut *b };
    let max = b.max_buffer_length.min(cap) & !1;
    if b.buffer_length + 2 > max {
        return false;
    }
    let at = b.buffer_length as usize;
    // SAFETY: `at + 2 <= max_buffer_length`, the size of `data`.
    unsafe {
        let p = b.data.cast::<u8>().add(at);
        let bytes = s.to_le_bytes();
        *p = bytes[0];
        *p.add(1) = bytes[1];
    }
    b.buffer_length += 2;
    true
}

/// Records an index mark in buffer `b`.
///
/// # Safety
///
/// As for [`push_sample`].
unsafe fn push_mark(b: *mut TtsBuffer, value: u32, sample: u32) {
    // SAFETY: guaranteed by the caller.
    let b = unsafe { &mut *b };
    if b.index_count < b.max_indices && !b.indices.is_null() {
        // SAFETY: `index_count < max_indices`, the size of `indices`.
        unsafe {
            *b.indices.add(b.index_count as usize) = TtsIndex {
                value,
                sample,
                reserved: 0,
            };
        }
        b.index_count += 1;
    }
}

macro_rules! exports {
    ($abi:literal) => {
        type Callback = unsafe extern $abi fn(isize, isize, u32, u32);

        /// Hands a full buffer to the callback, outside the lock.
        fn deliver(buffer: usize) -> bool {
            let cb = state().callback;
            if cb == 0 {
                return false;
            }
            // SAFETY: `cb` was stored from a `Callback` in
            // `TextToSpeechStartupEx`.
            let f: Callback = unsafe { std::mem::transmute::<usize, Callback>(cb) };
            // SAFETY: the host's callback, given the host's own buffer.
            unsafe { f(0, buffer as isize, 0, 1) };
            true
        }

        /// `TextToSpeechStartupEx`.
        #[unsafe(no_mangle)]
        pub unsafe extern $abi fn TextToSpeechStartupEx(
            ph: *mut *mut c_void,
            _device: u32,
            _options: u32,
            callback: Option<Callback>,
            _param: c_long,
        ) -> u32 {
            if env("FAKE_DECTALK_FAIL").is_some() || env("FAKE_DECTALK_NO_EX").is_some() {
                return 1;
            }
            if ph.is_null() {
                return 11;
            }
            state().callback = callback.map_or(0, |f| f as usize);
            // SAFETY: checked non-null; the host passes an out pointer.
            unsafe { *ph = handle() };
            0
        }

        /// `TextToSpeechStartup` (Windows form: a window, no callback).
        #[cfg(windows)]
        #[unsafe(no_mangle)]
        pub unsafe extern $abi fn TextToSpeechStartup(
            _hwnd: *mut c_void,
            ph: *mut *mut c_void,
            _device: u32,
            _options: u32,
        ) -> u32 {
            if env("FAKE_DECTALK_FAIL").is_some() {
                return 1;
            }
            if ph.is_null() {
                return 11;
            }
            state().callback = 0;
            // SAFETY: checked non-null.
            unsafe { *ph = handle() };
            0
        }

        /// `TextToSpeechStartup` (Unix form: as `TextToSpeechStartupEx`).
        #[cfg(not(windows))]
        #[unsafe(no_mangle)]
        pub unsafe extern $abi fn TextToSpeechStartup(
            ph: *mut *mut c_void,
            _device: u32,
            _options: u32,
            callback: Option<Callback>,
            _param: c_long,
        ) -> u32 {
            if env("FAKE_DECTALK_FAIL").is_some() {
                return 1;
            }
            if ph.is_null() {
                return 11;
            }
            state().callback = callback.map_or(0, |f| f as usize);
            // SAFETY: checked non-null.
            unsafe { *ph = handle() };
            0
        }

        /// `TextToSpeechShutdown`.
        #[unsafe(no_mangle)]
        pub unsafe extern $abi fn TextToSpeechShutdown(_h: *mut c_void) -> u32 {
            let mut st = state();
            st.buffers.clear();
            st.in_memory = false;
            0
        }

        /// `TextToSpeechOpenInMemory`.
        #[unsafe(no_mangle)]
        pub unsafe extern $abi fn TextToSpeechOpenInMemory(_h: *mut c_void, format: u32) -> u32 {
            if format != 4 {
                return 2;
            }
            let mut st = state();
            st.in_memory = true;
            st.stream = 0;
            0
        }

        /// `TextToSpeechCloseInMemory`.
        #[unsafe(no_mangle)]
        pub unsafe extern $abi fn TextToSpeechCloseInMemory(_h: *mut c_void) -> u32 {
            let mut st = state();
            st.in_memory = false;
            st.buffers.clear();
            0
        }

        /// `TextToSpeechAddBuffer`.
        #[unsafe(no_mangle)]
        pub unsafe extern $abi fn TextToSpeechAddBuffer(
            _h: *mut c_void,
            b: *mut TtsBuffer,
        ) -> u32 {
            let mut st = state();
            if !st.in_memory || b.is_null() {
                return 3;
            }
            st.buffers.push_back(b as usize);
            0
        }

        /// `TextToSpeechReturnBuffer`.
        #[unsafe(no_mangle)]
        pub unsafe extern $abi fn TextToSpeechReturnBuffer(
            _h: *mut c_void,
            out: *mut *mut TtsBuffer,
        ) -> u32 {
            if out.is_null() {
                return 11;
            }
            let b = state().buffers.pop_front();
            // SAFETY: checked non-null.
            unsafe { *out = b.map_or(std::ptr::null_mut(), |b| b as *mut TtsBuffer) };
            if b.is_some() { 0 } else { 4 }
        }

        /// `TextToSpeechSync`: synthesis already happened in
        /// `TextToSpeechSpeak`.
        #[unsafe(no_mangle)]
        pub unsafe extern $abi fn TextToSpeechSync(_h: *mut c_void) -> u32 {
            0
        }

        /// `TextToSpeechSpeak`.
        #[unsafe(no_mangle)]
        pub unsafe extern $abi fn TextToSpeechSpeak(
            _h: *mut c_void,
            text: *const c_char,
            _flags: u32,
        ) -> u32 {
            if text.is_null() {
                return 11;
            }
            // SAFETY: the host passes NUL-terminated text.
            let bytes = unsafe { std::ffi::CStr::from_ptr(text) }.to_bytes().to_vec();
            if let Some(path) = env("FAKE_DECTALK_LOG")
                && let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
            {
                let _ = f.write_all(&bytes);
                let _ = f.write_all(b"\n");
            }
            let (samples, marks) = {
                let mut st = state();
                if !st.in_memory {
                    return 5;
                }
                render(&mut st, &bytes)
            };
            let per_buffer = env("FAKE_DECTALK_MARKS").is_some_and(|v| v == "buffer");
            let cap = env("FAKE_DECTALK_MAX_BYTES")
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(u32::MAX);
            let mut marks = marks.into_iter().peekable();
            let mut next = 0usize;
            // The stream position where the current buffer starts.
            let mut buffer_start = state().stream;
            loop {
                let Some(b) = state().buffers.front().copied() else {
                    // Nowhere to write: a real engine would wait for a buffer.
                    return 6;
                };
                let bp = b as *mut TtsBuffer;
                let mut full = false;
                while next <= samples.len() {
                    // A mark goes with the sample after it, so a mark at a
                    // buffer boundary opens the next buffer.
                    // SAFETY: `bp` is the host's added buffer.
                    if next < samples.len() && !unsafe { has_room(bp, cap) } {
                        full = true;
                        break;
                    }
                    while let Some(&(value, at)) = marks.peek() {
                        if at != next {
                            break;
                        }
                        let stream = buffer_start
                            + u64::from(
                                // SAFETY: `bp` is the host's added buffer.
                                unsafe { (*bp).buffer_length } / 2,
                            );
                        let sample = if per_buffer {
                            stream - buffer_start
                        } else {
                            stream
                        };
                        // SAFETY: as above.
                        unsafe { push_mark(bp, value, u32::try_from(sample).unwrap_or(u32::MAX)) };
                        marks.next();
                    }
                    if next == samples.len() {
                        break;
                    }
                    // SAFETY: as above.
                    unsafe { push_sample(bp, samples[next], cap) };
                    next += 1;
                }
                // SAFETY: as above.
                let written = u64::from(unsafe { (*bp).buffer_length } / 2);
                if !full {
                    let mut st = state();
                    st.stream = buffer_start + written;
                    return 0;
                }
                state().buffers.pop_front();
                buffer_start += written;
                state().stream = buffer_start;
                if !deliver(b) {
                    // No callback: the buffer is lost to the host, as the
                    // engine would have stalled.
                    return 6;
                }
            }
        }
    };
}

#[cfg(not(feature = "fake-stdcall"))]
exports!("C");
#[cfg(feature = "fake-stdcall")]
exports!("system");
