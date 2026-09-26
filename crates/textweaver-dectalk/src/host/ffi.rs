//! A user-installed DECtalk library, loaded at run time with `libloading`
//! (ADR-0021).
//!
//! This is the only module of the DECtalk support that uses `unsafe`. It
//! drives DECtalk's documented C API (`ttsapi.h`, the "DECtalk Software"
//! API that DEC, Force Computers, and Fonix shipped): no DECtalk code is
//! in textweaver, and nothing here was taken from DECtalk's source.
//!
//! **Calls.** `TextToSpeechStartupEx` (or `TextToSpeechStartup`) with
//! `WAVE_MAPPER` and `DO_NOT_USE_AUDIO_DEVICE`, then speech-to-memory:
//! `TextToSpeechOpenInMemory(WAVE_FORMAT_1M16)` (11,025 Hz, 16-bit mono),
//! and per utterance `TextToSpeechAddBuffer`, `TextToSpeechSpeak(TTS_FORCE)`,
//! `TextToSpeechSync`, `TextToSpeechReturnBuffer`; at exit
//! `TextToSpeechCloseInMemory` and `TextToSpeechShutdown`. The voice, rate,
//! pitch, and index marks are inline commands in the text
//! ([`super::input`]), so no call depends on the numbering of DECtalk's
//! enumerations.
//!
//! **Buffers.** One buffer of [`BUFFER_SAMPLES`] samples (two minutes) and
//! [`BUFFER_MARKS`] index marks holds a whole sentence. When DECtalk fills
//! it, it hands it to the callback registered with
//! `TextToSpeechStartupEx`; the callback copies it and gives it back
//! (`TextToSpeechAddBuffer`). The callback recognizes the buffer by its
//! address in either message parameter, so it needs no message numbers.
//! After `TextToSpeechSync`, `TextToSpeechReturnBuffer` returns the partly
//! filled buffer.
//!
//! **Index marks.** `TTS_INDEX_T` gives each mark's sample number. Whether
//! DECtalk counts from the start of the memory stream or of each buffer,
//! and whether the count restarts, the host works it out from the
//! utterance's *start mark* ([`super::input::START_MARK`]): a sample number
//! at or past the start of the buffer it came in counts from the stream,
//! one below it from the buffer; the start mark's position is subtracted,
//! so word offsets count from the utterance's first sample.
//!
//! **Calling convention.** 64-bit Windows and Linux have one C calling
//! convention. A 32-bit Windows `DECtalk.dll` may use `cdecl` or `stdcall`;
//! the host takes `stdcall` when the exports carry `stdcall`'s decorated
//! names (`_TextToSpeechStartup@16`), and otherwise measures it: the first
//! `TextToSpeechStartup` call goes through a small assembly thunk that
//! restores the stack pointer itself and reports whether the library popped
//! its arguments (`stdcall`) or left them (`cdecl`). `--convention` on the
//! host's command line overrides both.
//!
//! **Layouts** (`ttsapi.h`; `DWORD` is 32 bits, pointers are native):
//!
//! ```c
//! typedef struct { DWORD dwIndexValue, dwIndexSampleNumber, dwReserved; } TTS_INDEX_T;
//! typedef struct {
//!     LPSTR lpData; LPTTS_PHONEME_T lpPhonemeArray; LPTTS_INDEX_T lpIndexArray;
//!     DWORD dwMaximumBufferLength;            /* bytes */
//!     DWORD dwMaximumNumberOfPhonemeChanges, dwMaximumNumberOfIndexMarks;
//!     DWORD dwBufferLength;                   /* bytes */
//!     DWORD dwNumberOfPhonemeChanges, dwNumberOfIndexMarks, dwReserved;
//! } TTS_BUFFER_T;
//! ```
//!
//! These signatures and layouts follow DECtalk's published API reference;
//! they have not yet been checked against a licensed DECtalk library (none
//! is installed on the development machine). The tests drive this module
//! through a stand-in library built from `examples/fake_dectalk.rs`.
#![allow(unsafe_code)]

use std::ffi::{c_char, c_void};
use std::path::Path;
use std::sync::Mutex;

use libloading::Library;

use super::input::{self, EnginePiece, START_MARK};
use super::{Engine, EngineInfo, Settings, SynthEvent};

/// A `LPTTS_HANDLE_T`.
type Handle = *mut c_void;

/// `MMSYSERR_NOERROR`.
const NO_ERROR: u32 = 0;
/// `TTS_FORCE`: speak everything given, without waiting for a clause end.
const TTS_FORCE: u32 = 1;
/// `WAVE_MAPPER`.
const WAVE_MAPPER: u32 = 0xFFFF_FFFF;
/// `DO_NOT_USE_AUDIO_DEVICE`.
const DO_NOT_USE_AUDIO_DEVICE: u32 = 0x8000_0000;
/// `WAVE_FORMAT_1M16`: 11,025 Hz, mono, 16-bit.
const WAVE_FORMAT_1M16: u32 = 0x0000_0004;
/// The sample rate of `WAVE_FORMAT_1M16`.
pub const SAMPLE_RATE: u32 = 11_025;

/// Samples one speech-to-memory buffer holds (two minutes).
pub const BUFFER_SAMPLES: usize = 120 * SAMPLE_RATE as usize;
/// Index marks one buffer holds.
pub const BUFFER_MARKS: usize = 4096;
/// Samples per `Audio` frame sent to the backend.
const BLOCK: usize = 4096;

/// `TTS_INDEX_T`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct TtsIndex {
    /// `dwIndexValue`: the value in `[:index mark N]`.
    pub value: u32,
    /// `dwIndexSampleNumber`.
    pub sample: u32,
    /// `dwReserved`.
    pub reserved: u32,
}

/// `TTS_BUFFER_T`.
#[repr(C)]
#[derive(Debug)]
pub struct TtsBuffer {
    /// `lpData`: the PCM bytes.
    pub data: *mut c_char,
    /// `lpPhonemeArray` (unused: null, with a maximum of 0).
    pub phonemes: *mut c_void,
    /// `lpIndexArray`.
    pub indices: *mut TtsIndex,
    /// `dwMaximumBufferLength`, in bytes.
    pub max_buffer_length: u32,
    /// `dwMaximumNumberOfPhonemeChanges`.
    pub max_phonemes: u32,
    /// `dwMaximumNumberOfIndexMarks`.
    pub max_indices: u32,
    /// `dwBufferLength`, in bytes.
    pub buffer_length: u32,
    /// `dwNumberOfPhonemeChanges`.
    pub phoneme_count: u32,
    /// `dwNumberOfIndexMarks`.
    pub index_count: u32,
    /// `dwReserved`.
    pub reserved: u32,
}

/// A calling convention.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Convention {
    /// The C convention (the only one on 64-bit Windows and on Linux).
    Cdecl,
    /// 32-bit Windows `stdcall`.
    Stdcall,
}

impl Convention {
    /// Parses `cdecl` or `stdcall`.
    pub fn parse(s: &str) -> Option<Convention> {
        match s.trim().to_ascii_lowercase().as_str() {
            "cdecl" | "c" => Some(Convention::Cdecl),
            "stdcall" | "system" => Some(Convention::Stdcall),
            _ => None,
        }
    }

    /// The name.
    pub fn name(self) -> &'static str {
        match self {
            Convention::Cdecl => "cdecl",
            Convention::Stdcall => "stdcall",
        }
    }
}

/// One filled buffer, copied out.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Chunk {
    samples: Vec<i16>,
    /// `(index value, sample number)` as DECtalk reported them.
    marks: Vec<(u32, u32)>,
}

/// Copies the audio and marks out of a buffer DECtalk filled.
///
/// # Safety
///
/// `b` points at a live `TtsBuffer` whose `data` holds at least
/// `max_buffer_length` bytes and whose `indices` holds at least
/// `max_indices` entries, and nothing writes to them during the call.
unsafe fn read_chunk(b: *const TtsBuffer) -> Chunk {
    // SAFETY: guaranteed by the caller.
    let b = unsafe { &*b };
    let bytes = b.buffer_length.min(b.max_buffer_length) as usize;
    let n_marks = b.index_count.min(b.max_indices) as usize;
    let data: &[u8] = if b.data.is_null() || bytes == 0 {
        &[]
    } else {
        // SAFETY: `data` holds `max_buffer_length` bytes and `bytes` is at
        // most that.
        unsafe { std::slice::from_raw_parts(b.data.cast::<u8>(), bytes) }
    };
    let marks: &[TtsIndex] = if b.indices.is_null() || n_marks == 0 {
        &[]
    } else {
        // SAFETY: `indices` holds `max_indices` entries and `n_marks` is at
        // most that.
        unsafe { std::slice::from_raw_parts(b.indices, n_marks) }
    };
    Chunk {
        samples: data
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect(),
        marks: marks.iter().map(|m| (m.value, m.sample)).collect(),
    }
}

/// What the buffer callback needs, shared with the engine thread DECtalk
/// may call it from.
#[derive(Debug, Default)]
struct CallbackState {
    handle: usize,
    /// The address of the engine's buffer; 0 while no synthesis runs.
    buffer: usize,
    /// Filled buffers the callback copied, in order.
    chunks: Vec<Chunk>,
}

static CALLBACK: Mutex<CallbackState> = Mutex::new(CallbackState {
    handle: 0,
    buffer: 0,
    chunks: Vec::new(),
});

fn callback_state() -> std::sync::MutexGuard<'static, CallbackState> {
    CALLBACK.lock().unwrap_or_else(|e| e.into_inner())
}

/// The callback's work: when either parameter is the engine's buffer,
/// copies it, empties it, and returns `(handle, buffer)` for the caller to
/// give back to DECtalk (outside the lock, in case DECtalk calls the
/// callback again from inside `TextToSpeechAddBuffer`).
fn take_filled(lparam1: isize, lparam2: isize) -> Option<(usize, usize)> {
    let mut st = callback_state();
    let buffer = st.buffer;
    if buffer == 0 || (lparam2 as usize != buffer && lparam1 as usize != buffer) {
        return None;
    }
    let b = buffer as *mut TtsBuffer;
    // SAFETY: `buffer` is the address of the engine's boxed `TtsBuffer`,
    // registered by `synthesize` for the duration of `TextToSpeechSync`;
    // DECtalk passes it back here only after filling it and does not
    // touch it until it is added again.
    let chunk = unsafe { read_chunk(b) };
    // SAFETY: as above; the engine thread is not writing the buffer.
    unsafe {
        (*b).buffer_length = 0;
        (*b).index_count = 0;
        (*b).phoneme_count = 0;
    }
    st.chunks.push(chunk);
    Some((st.handle, buffer))
}

/// Looks a function up by its plain name, then by `stdcall`'s decorated
/// names (`_Name@N`, `Name@N`), and copies the pointer out.
///
/// # Safety
///
/// `T` must be the function's true signature.
unsafe fn find<T: Copy>(lib: &Library, name: &str, arg_bytes: usize) -> Option<T> {
    for n in [
        name.to_owned(),
        format!("_{name}@{arg_bytes}"),
        format!("{name}@{arg_bytes}"),
    ] {
        // SAFETY: guaranteed by the caller.
        if let Ok(sym) = unsafe { lib.get::<T>(n.as_bytes()) } {
            return Some(*sym);
        }
    }
    None
}

/// Whether the library exports `stdcall`-decorated names.
fn has_decorated_exports(lib: &Library) -> bool {
    ["_TextToSpeechStartup@16", "_TextToSpeechSpeak@12"]
        .iter()
        // SAFETY: only checks that the symbol exists; the pointer is not
        // used.
        .any(|n| unsafe { lib.get::<*const c_void>(n.as_bytes()) }.is_ok())
}

/// The calls the engine makes, whatever the calling convention.
trait Calls {
    /// Starts DECtalk without an audio device; `(status, handle)`.
    unsafe fn startup(&self) -> (u32, Handle);
    /// Whether [`startup`](Self::startup) registered the buffer callback.
    fn has_callback(&self) -> bool;
    unsafe fn shutdown(&self, h: Handle) -> u32;
    unsafe fn speak(&self, h: Handle, text: *const c_char, flags: u32) -> u32;
    unsafe fn sync(&self, h: Handle) -> u32;
    unsafe fn open_in_memory(&self, h: Handle, format: u32) -> u32;
    unsafe fn close_in_memory(&self, h: Handle) -> u32;
    unsafe fn add_buffer(&self, h: Handle, b: *mut TtsBuffer) -> u32;
    unsafe fn return_buffer(&self, h: Handle, b: *mut *mut TtsBuffer) -> u32;
}

/// The API for one calling convention: function pointer types, the
/// loader, and the buffer callback in that convention.
macro_rules! dectalk_api {
    ($module:ident, $abi:literal) => {
        mod $module {
            use std::ffi::{c_char, c_long};
            use std::sync::Mutex;

            use libloading::Library;

            use super::{
                Calls, DO_NOT_USE_AUDIO_DEVICE, Handle, TtsBuffer, WAVE_MAPPER, find, take_filled,
            };

            /// `void callback(LONG lParam1, LONG lParam2, DWORD param, UINT msg)`.
            pub type Callback = unsafe extern $abi fn(isize, isize, u32, u32);
            type StartupEx =
                unsafe extern $abi fn(*mut Handle, u32, u32, Option<Callback>, c_long) -> u32;
            #[cfg(windows)]
            type Startup =
                unsafe extern $abi fn(*mut std::ffi::c_void, *mut Handle, u32, u32) -> u32;
            #[cfg(not(windows))]
            type Startup = StartupEx;
            type OnHandle = unsafe extern $abi fn(Handle) -> u32;
            type Speak = unsafe extern $abi fn(Handle, *const c_char, u32) -> u32;
            type WithFormat = unsafe extern $abi fn(Handle, u32) -> u32;
            type AddBuffer = unsafe extern $abi fn(Handle, *mut TtsBuffer) -> u32;
            type ReturnBuffer = unsafe extern $abi fn(Handle, *mut *mut TtsBuffer) -> u32;

            /// `TextToSpeechAddBuffer`, for the callback.
            static ADD_BUFFER: Mutex<Option<AddBuffer>> = Mutex::new(None);

            unsafe extern $abi fn buffer_callback(l1: isize, l2: isize, _param: u32, _msg: u32) {
                let _ = std::panic::catch_unwind(|| {
                    let Some((h, b)) = take_filled(l1, l2) else {
                        return;
                    };
                    let add = *ADD_BUFFER.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(add) = add {
                        // SAFETY: gives DECtalk back the buffer it just
                        // handed over, on the handle it was added to.
                        unsafe { add(h as Handle, b as *mut TtsBuffer) };
                    }
                });
            }

            pub struct Api {
                startup_ex: Option<StartupEx>,
                startup: Option<Startup>,
                shutdown: OnHandle,
                speak: Speak,
                sync: OnHandle,
                open_in_memory: WithFormat,
                close_in_memory: OnHandle,
                add_buffer: AddBuffer,
                return_buffer: ReturnBuffer,
                callback: std::cell::Cell<bool>,
            }

            fn missing(name: &str) -> String {
                format!("{name} is missing from the library (is it DECtalk?)")
            }

            impl Api {
                pub fn load(lib: &Library) -> Result<Api, String> {
                    let ptr = std::mem::size_of::<usize>();
                    // SAFETY: each type is the documented signature of the
                    // `ttsapi.h` function of that name.
                    unsafe {
                        let startup_ex: Option<StartupEx> =
                            find(lib, "TextToSpeechStartupEx", 3 * 4 + 2 * ptr);
                        let startup: Option<Startup> = find(
                            lib,
                            "TextToSpeechStartup",
                            if cfg!(windows) { 2 * ptr + 8 } else { 3 * 4 + 2 * ptr },
                        );
                        if startup_ex.is_none() && startup.is_none() {
                            return Err(missing("TextToSpeechStartup"));
                        }
                        let api = Api {
                            startup_ex,
                            startup,
                            shutdown: find(lib, "TextToSpeechShutdown", ptr)
                                .ok_or_else(|| missing("TextToSpeechShutdown"))?,
                            speak: find(lib, "TextToSpeechSpeak", 2 * ptr + 4)
                                .ok_or_else(|| missing("TextToSpeechSpeak"))?,
                            sync: find(lib, "TextToSpeechSync", ptr)
                                .ok_or_else(|| missing("TextToSpeechSync"))?,
                            open_in_memory: find(lib, "TextToSpeechOpenInMemory", ptr + 4)
                                .ok_or_else(|| missing("TextToSpeechOpenInMemory"))?,
                            close_in_memory: find(lib, "TextToSpeechCloseInMemory", ptr)
                                .ok_or_else(|| missing("TextToSpeechCloseInMemory"))?,
                            add_buffer: find(lib, "TextToSpeechAddBuffer", 2 * ptr)
                                .ok_or_else(|| missing("TextToSpeechAddBuffer"))?,
                            return_buffer: find(lib, "TextToSpeechReturnBuffer", 2 * ptr)
                                .ok_or_else(|| missing("TextToSpeechReturnBuffer"))?,
                            callback: std::cell::Cell::new(false),
                        };
                        *ADD_BUFFER.lock().unwrap_or_else(|e| e.into_inner()) =
                            Some(api.add_buffer);
                        Ok(api)
                    }
                }
            }

            impl Calls for Api {
                unsafe fn startup(&self) -> (u32, Handle) {
                    let mut h: Handle = std::ptr::null_mut();
                    if let Some(f) = self.startup_ex {
                        // SAFETY: `h` is a valid out pointer; the callback
                        // has DECtalk's callback signature.
                        let rc = unsafe {
                            f(
                                &raw mut h,
                                WAVE_MAPPER,
                                DO_NOT_USE_AUDIO_DEVICE,
                                Some(buffer_callback),
                                0,
                            )
                        };
                        if rc == 0 && !h.is_null() {
                            self.callback.set(true);
                            return (rc, h);
                        }
                        if self.startup.is_none() {
                            return (rc, h);
                        }
                    }
                    let Some(f) = self.startup else {
                        return (u32::MAX, h);
                    };
                    self.callback.set(!cfg!(windows));
                    #[cfg(windows)]
                    // SAFETY: no window (null `HWND`); `h` is a valid out pointer.
                    let rc = unsafe {
                        f(
                            std::ptr::null_mut(),
                            &raw mut h,
                            WAVE_MAPPER,
                            DO_NOT_USE_AUDIO_DEVICE,
                        )
                    };
                    #[cfg(not(windows))]
                    // SAFETY: as for `TextToSpeechStartupEx`, whose
                    // signature it shares outside Windows.
                    let rc = unsafe {
                        f(
                            &raw mut h,
                            WAVE_MAPPER,
                            DO_NOT_USE_AUDIO_DEVICE,
                            Some(buffer_callback),
                            0,
                        )
                    };
                    (rc, h)
                }
                fn has_callback(&self) -> bool {
                    self.callback.get()
                }
                unsafe fn shutdown(&self, h: Handle) -> u32 {
                    // SAFETY: the caller passes a live handle.
                    unsafe { (self.shutdown)(h) }
                }
                unsafe fn speak(&self, h: Handle, text: *const c_char, flags: u32) -> u32 {
                    // SAFETY: the caller passes a live handle and
                    // NUL-terminated text.
                    unsafe { (self.speak)(h, text, flags) }
                }
                unsafe fn sync(&self, h: Handle) -> u32 {
                    // SAFETY: the caller passes a live handle.
                    unsafe { (self.sync)(h) }
                }
                unsafe fn open_in_memory(&self, h: Handle, format: u32) -> u32 {
                    // SAFETY: the caller passes a live handle.
                    unsafe { (self.open_in_memory)(h, format) }
                }
                unsafe fn close_in_memory(&self, h: Handle) -> u32 {
                    // SAFETY: the caller passes a live handle.
                    unsafe { (self.close_in_memory)(h) }
                }
                unsafe fn add_buffer(&self, h: Handle, b: *mut TtsBuffer) -> u32 {
                    // SAFETY: the caller passes a live handle and a buffer
                    // that stays valid until DECtalk returns it.
                    unsafe { (self.add_buffer)(h, b) }
                }
                unsafe fn return_buffer(&self, h: Handle, b: *mut *mut TtsBuffer) -> u32 {
                    // SAFETY: the caller passes a live handle and an out
                    // pointer.
                    unsafe { (self.return_buffer)(h, b) }
                }
            }
        }
    };
}

dectalk_api!(cdecl, "C");
#[cfg(all(windows, target_arch = "x86"))]
dectalk_api!(stdcall, "system");

/// Calls a 32-bit function with five 32-bit arguments without knowing its
/// calling convention: the stack pointer is saved before the arguments go
/// on and restored after the call, whatever the callee did. Returns the
/// callee's `eax` and how many argument bytes it left on the stack (20 for
/// `cdecl`; fewer for `stdcall`, which pops its own).
///
/// # Safety
///
/// `f` must be a function that accepts these arguments under one of the two
/// conventions (extra trailing arguments are harmless under `cdecl`, and
/// under `stdcall` only what the callee pops is read).
#[cfg(all(windows, target_arch = "x86"))]
unsafe fn call_measuring_stack(f: usize, args: &[u32; 5]) -> (u32, u32) {
    let ret: u32;
    let left: u32;
    // SAFETY: the stack pointer is restored from `edi` (callee-saved in
    // both conventions) after the call; the callee's contract is the
    // caller's.
    unsafe {
        std::arch::asm!(
            "mov edi, esp",
            "push dword ptr [{args} + 16]",
            "push dword ptr [{args} + 12]",
            "push dword ptr [{args} + 8]",
            "push dword ptr [{args} + 4]",
            "push dword ptr [{args}]",
            "call {f}",
            "mov edx, edi",
            "sub edx, esp",
            "mov esp, edi",
            args = in(reg) args.as_ptr(),
            f = in(reg) f,
            out("edi") _,
            lateout("eax") ret,
            lateout("edx") left,
            clobber_abi("C"),
        );
    }
    (ret, left)
}

/// Works out a 32-bit library's calling convention (see the module docs).
#[cfg(all(windows, target_arch = "x86"))]
fn probe_convention(lib: &Library) -> Result<Convention, String> {
    if has_decorated_exports(lib) {
        return Ok(Convention::Stdcall);
    }
    // SAFETY: only the address is taken.
    let f = unsafe { lib.get::<*const c_void>(b"TextToSpeechStartup") }
        .map(|s| *s as usize)
        .map_err(|_| "TextToSpeechStartup is missing from the library (is it DECtalk?)")?;
    let mut h: Handle = std::ptr::null_mut();
    let args = [
        0, // no window
        (&raw mut h) as usize as u32,
        WAVE_MAPPER,
        DO_NOT_USE_AUDIO_DEVICE,
        0, // an extra argument, left alone by a four-argument callee
    ];
    // SAFETY: `TextToSpeechStartup(HWND, LPTTS_HANDLE_T *, UINT, DWORD)`
    // with a null window and a valid out pointer.
    let (rc, left) = unsafe { call_measuring_stack(f, &args) };
    let conv = if left >= 20 {
        Convention::Cdecl
    } else {
        Convention::Stdcall
    };
    if rc == NO_ERROR && !h.is_null() {
        let api = match conv {
            Convention::Cdecl => Box::new(cdecl::Api::load(lib)?) as Box<dyn Calls>,
            Convention::Stdcall => Box::new(stdcall::Api::load(lib)?) as Box<dyn Calls>,
        };
        // SAFETY: `h` was just started by this library.
        unsafe { api.shutdown(h) };
    }
    Ok(conv)
}

/// The real engine.
pub struct DectalkEngine {
    calls: Box<dyn Calls>,
    h: Handle,
    /// The speech-to-memory buffer (a leaked `Box`, so its address never
    /// changes and DECtalk's copy of the pointer stays valid; freed in
    /// `Drop` after DECtalk has shut down).
    buffer: *mut TtsBuffer,
    /// The buffer's storage; allocated once, never reallocated, and read
    /// only through `buffer`'s pointers.
    _data: Vec<u8>,
    _marks: Vec<TtsIndex>,
    /// DECtalk holds the buffer (it was added and not yet returned).
    queued: bool,
    convention: Convention,
    // Declared last: the library is unloaded only after everything above
    // (`Drop` shuts the engine down first).
    _lib: Library,
}

impl std::fmt::Debug for DectalkEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DectalkEngine")
            .field("convention", &self.convention)
            .field("callback", &self.calls.has_callback())
            .finish_non_exhaustive()
    }
}

fn open_library(path: &Path) -> Result<Library, String> {
    #[cfg(windows)]
    {
        // Resolve the DLL's own dependencies from its folder.
        // SAFETY: loading runs the library's initializers; the path is the
        // DECtalk the user installed or named.
        unsafe {
            libloading::os::windows::Library::load_with_flags(
                path,
                libloading::os::windows::LOAD_WITH_ALTERED_SEARCH_PATH,
            )
        }
        .map(Library::from)
        .map_err(|e| format!("cannot load {}: {e}", path.display()))
    }
    #[cfg(not(windows))]
    {
        // SAFETY: as above.
        unsafe { Library::new(path) }.map_err(|e| format!("cannot load {}: {e}", path.display()))
    }
}

impl DectalkEngine {
    /// Loads the library at `path` and starts DECtalk in speech-to-memory
    /// mode. `convention` forces a calling convention (32-bit Windows
    /// only; elsewhere there is one).
    pub fn load(path: &Path, convention: Option<Convention>) -> Result<Self, String> {
        // DECtalk finds its dictionary (`dtalk_us.dic`) in the current
        // directory; this process exists only to run it.
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty())
            && let Err(e) = std::env::set_current_dir(dir)
        {
            eprintln!(
                "textweaver-dectalk-host: cannot enter {}: {e}",
                dir.display()
            );
        }
        let lib = open_library(path)?;
        #[cfg(all(windows, target_arch = "x86"))]
        let convention = match convention {
            Some(c) => c,
            None => probe_convention(&lib)?,
        };
        #[cfg(not(all(windows, target_arch = "x86")))]
        let convention = {
            // One convention on this platform: `stdcall` is `cdecl`.
            let _ = (convention, has_decorated_exports);
            Convention::Cdecl
        };
        let calls: Box<dyn Calls> = match convention {
            Convention::Cdecl => Box::new(cdecl::Api::load(&lib)?),
            #[cfg(all(windows, target_arch = "x86"))]
            Convention::Stdcall => Box::new(stdcall::Api::load(&lib)?),
            #[cfg(not(all(windows, target_arch = "x86")))]
            Convention::Stdcall => Box::new(cdecl::Api::load(&lib)?),
        };
        // SAFETY: starts an engine on the library just loaded.
        let (rc, h) = unsafe { calls.startup() };
        if rc != NO_ERROR || h.is_null() {
            return Err(format!(
                "DECtalk did not start (TextToSpeechStartup error {rc}); is it installed and \
                 licensed, with its dictionary beside the library?"
            ));
        }
        // SAFETY: `h` is the live handle just started.
        let rc = unsafe { calls.open_in_memory(h, WAVE_FORMAT_1M16) };
        if rc != NO_ERROR {
            // SAFETY: as above.
            unsafe { calls.shutdown(h) };
            return Err(format!(
                "DECtalk cannot synthesize to memory (TextToSpeechOpenInMemory error {rc})"
            ));
        }
        let mut data = vec![0u8; BUFFER_SAMPLES * 2];
        let mut marks = vec![TtsIndex::default(); BUFFER_MARKS];
        let buffer = Box::into_raw(Box::new(TtsBuffer {
            data: data.as_mut_ptr().cast(),
            phonemes: std::ptr::null_mut(),
            indices: marks.as_mut_ptr(),
            max_buffer_length: u32::try_from(data.len()).unwrap_or(u32::MAX),
            max_phonemes: 0,
            max_indices: u32::try_from(marks.len()).unwrap_or(u32::MAX),
            buffer_length: 0,
            phoneme_count: 0,
            index_count: 0,
            reserved: 0,
        }));
        Ok(DectalkEngine {
            calls,
            h,
            buffer,
            _data: data,
            _marks: marks,
            queued: false,
            convention,
            _lib: lib,
        })
    }

    /// The calling convention in use.
    pub fn convention(&self) -> Convention {
        self.convention
    }

    /// Whether DECtalk hands full buffers to the callback (started with
    /// `TextToSpeechStartupEx`); otherwise one buffer holds each utterance.
    pub fn has_callback(&self) -> bool {
        self.calls.has_callback()
    }

    /// Runs one utterance through DECtalk and collects its buffers.
    fn collect(&mut self, text: &[u8]) -> Result<Vec<Chunk>, String> {
        let h = self.h;
        let buf = self.buffer;
        {
            let mut st = callback_state();
            st.handle = h as usize;
            st.buffer = buf as usize;
            st.chunks.clear();
        }
        if !self.queued {
            // SAFETY: DECtalk does not hold the buffer (`queued` is false),
            // so nothing else reads or writes it.
            unsafe {
                (*buf).buffer_length = 0;
                (*buf).index_count = 0;
                (*buf).phoneme_count = 0;
            }
            // SAFETY: the buffer and its storage live as long as the engine
            // and are not moved or reallocated.
            let rc = unsafe { self.calls.add_buffer(h, buf) };
            if rc != NO_ERROR {
                return Err(format!("TextToSpeechAddBuffer failed (error {rc})"));
            }
            self.queued = true;
        }
        // SAFETY: `text` is NUL-terminated and outlives the call.
        let rc = unsafe { self.calls.speak(h, text.as_ptr().cast(), TTS_FORCE) };
        if rc != NO_ERROR {
            return Err(format!("TextToSpeechSpeak failed (error {rc})"));
        }
        // SAFETY: live handle; blocks until the text is synthesized, while
        // the callback may copy full buffers.
        let rc = unsafe { self.calls.sync(h) };
        if rc != NO_ERROR {
            return Err(format!("TextToSpeechSync failed (error {rc})"));
        }
        let mut returned: *mut TtsBuffer = std::ptr::null_mut();
        // SAFETY: live handle and a valid out pointer.
        let rc = unsafe { self.calls.return_buffer(h, &raw mut returned) };
        let mut chunks = {
            let mut st = callback_state();
            st.buffer = 0;
            std::mem::take(&mut st.chunks)
        };
        if rc == NO_ERROR && returned == buf {
            self.queued = false;
            // SAFETY: DECtalk returned our buffer; nothing writes to it now.
            chunks.push(unsafe { read_chunk(buf) });
        }
        Ok(chunks)
    }
}

/// Turns the buffers of one utterance into its audio and its word marks,
/// `(word, sample from the utterance's first sample)` (see the module docs
/// on how sample numbers are read).
fn assemble(chunks: Vec<Chunk>) -> (Vec<i16>, Vec<(u32, u64)>) {
    let mut samples: Vec<i16> = Vec::new();
    let mut raw: Vec<(u32, u64)> = Vec::new();
    for c in chunks {
        let start = samples.len() as u64;
        for (value, s) in c.marks {
            let s = u64::from(s);
            raw.push((value, if s >= start { s } else { start + s }));
        }
        samples.extend(c.samples);
    }
    let total = samples.len() as u64;
    let base = raw
        .iter()
        .find(|(v, _)| *v == START_MARK)
        .map(|&(_, at)| at)
        .or_else(|| {
            // No start mark: if the numbers are clearly not this
            // utterance's own, count from the first word.
            let first = raw.iter().map(|&(_, at)| at).min()?;
            (first > total).then_some(first)
        })
        .unwrap_or(0);
    let marks = raw
        .into_iter()
        .filter_map(|(value, at)| Some((input::word_of_mark(value)?, at.saturating_sub(base))))
        .collect();
    (samples, marks)
}

impl Engine for DectalkEngine {
    fn info(&mut self) -> EngineInfo {
        EngineInfo {
            sample_rate: SAMPLE_RATE,
            version: "DECtalk".into(),
            engine: "dectalk".into(),
        }
    }

    fn synthesize(
        &mut self,
        settings: &Settings,
        pieces: &[EnginePiece],
        out: &mut dyn FnMut(SynthEvent<'_>) -> bool,
    ) -> Result<bool, String> {
        let text = input::command_string(settings, pieces);
        let chunks = self.collect(&text)?;
        let (samples, mut marks) = assemble(chunks);
        Ok(super::emit_in_order(&samples, &mut marks, BLOCK, out))
    }
}

impl Drop for DectalkEngine {
    fn drop(&mut self) {
        {
            let mut st = callback_state();
            st.buffer = 0;
            st.chunks.clear();
        }
        // SAFETY: `h` came from `startup` and is shut down exactly once;
        // closing memory mode returns the buffer before its storage goes.
        unsafe {
            self.calls.close_in_memory(self.h);
            self.calls.shutdown(self.h);
        }
        // SAFETY: `buffer` came from `Box::into_raw` in `load` and is freed
        // once, now that DECtalk has shut down and no longer holds it.
        drop(unsafe { Box::from_raw(self.buffer) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(n: usize, marks: &[(u32, u32)]) -> Chunk {
        Chunk {
            samples: vec![1; n],
            marks: marks.to_vec(),
        }
    }

    #[test]
    fn stream_numbers_count_from_the_start_mark() {
        // DECtalk had already produced 50,000 samples before this utterance.
        let (s, marks) = assemble(vec![chunk(1000, &[(1, 50_000), (2, 50_000), (3, 50_400)])]);
        assert_eq!(s.len(), 1000);
        assert_eq!(marks, [(0, 0), (1, 400)]);
    }

    #[test]
    fn buffer_relative_numbers_are_placed_after_earlier_buffers() {
        let (s, marks) = assemble(vec![
            chunk(1000, &[(1, 0), (2, 10), (3, 900)]),
            chunk(500, &[(4, 100), (5, 450)]),
        ]);
        assert_eq!(s.len(), 1500);
        assert_eq!(marks, [(0, 10), (1, 900), (2, 1100), (3, 1450)]);
    }

    #[test]
    fn stream_numbers_across_buffers_stay_as_they_are() {
        let (_, marks) = assemble(vec![
            chunk(1000, &[(1, 0), (2, 10)]),
            chunk(500, &[(3, 1100)]),
        ]);
        assert_eq!(marks, [(0, 10), (1, 1100)]);
    }

    #[test]
    fn a_missing_start_mark_falls_back_sensibly() {
        let (_, marks) = assemble(vec![chunk(100, &[(2, 7), (3, 50)])]);
        assert_eq!(marks, [(0, 7), (1, 50)]);
        let (_, marks) = assemble(vec![chunk(100, &[(2, 9000), (3, 9050)])]);
        assert_eq!(marks, [(0, 0), (1, 50)]);
    }

    #[test]
    fn unknown_values_are_dropped() {
        let (_, marks) = assemble(vec![chunk(10, &[(1, 0), (0, 3), (40_000, 4), (2, 5)])]);
        assert_eq!(marks, [(0, 5)]);
    }

    #[test]
    fn buffers_are_read_within_their_limits() {
        let mut data = vec![0u8; 8];
        data[..4].copy_from_slice(&[1, 0, 0xff, 0xff]);
        let mut marks = vec![
            TtsIndex {
                value: 1,
                sample: 0,
                reserved: 0
            };
            2
        ];
        let b = TtsBuffer {
            data: data.as_mut_ptr().cast(),
            phonemes: std::ptr::null_mut(),
            indices: marks.as_mut_ptr(),
            max_buffer_length: 8,
            max_phonemes: 0,
            max_indices: 2,
            buffer_length: 4,
            phoneme_count: 0,
            index_count: 9, // more than it holds: clamped
            reserved: 0,
        };
        // SAFETY: `b` describes `data` and `marks`, which outlive the call.
        let c = unsafe { read_chunk(&raw const b) };
        assert_eq!(c.samples, [1, -1]);
        assert_eq!(c.marks, [(1, 0), (1, 0)]);
    }

    #[test]
    fn conventions_parse() {
        assert_eq!(Convention::parse("stdcall"), Some(Convention::Stdcall));
        assert_eq!(Convention::parse(" CDECL "), Some(Convention::Cdecl));
        assert_eq!(Convention::parse("fastcall"), None);
        assert_eq!(Convention::Stdcall.name(), "stdcall");
    }

    #[test]
    fn the_callback_takes_only_its_own_buffer() {
        let mut data = vec![0u8; 4];
        let mut marks = vec![TtsIndex::default(); 1];
        let mut b = Box::new(TtsBuffer {
            data: data.as_mut_ptr().cast(),
            phonemes: std::ptr::null_mut(),
            indices: marks.as_mut_ptr(),
            max_buffer_length: 4,
            max_phonemes: 0,
            max_indices: 1,
            buffer_length: 4,
            phoneme_count: 0,
            index_count: 1,
            reserved: 0,
        });
        let addr = (&raw mut *b) as usize;
        {
            let mut st = callback_state();
            st.handle = 42;
            st.buffer = addr;
            st.chunks.clear();
        }
        assert_eq!(take_filled(1, 2), None);
        assert_eq!(take_filled(0, addr as isize), Some((42, addr)));
        assert_eq!(b.buffer_length, 0, "emptied for DECtalk to refill");
        assert_eq!(take_filled(addr as isize, 0), Some((42, addr)));
        let chunks = {
            let mut st = callback_state();
            st.buffer = 0;
            std::mem::take(&mut st.chunks)
        };
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].samples.len(), 2);
        assert!(chunks[1].samples.is_empty());
        assert_eq!(take_filled(0, addr as isize), None, "unregistered");
    }
}
