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
//! libespeak-ng is loaded at run time with `libloading`, not linked, so one
//! binary (the Linux AppImage, for instance) works with or without
//! libespeak-ng installed: without it the backend reports itself
//! unavailable and another engine speaks. A copy in textweaver's
//! components folder is tried first ([`component_library`]), then the file
//! [`LIBRARY_ENV`] names, then the usual names for the platform
//! ([`library_candidates`]). The library stays loaded for the life of the
//! process. Building needs no espeak-ng headers or library.
//!
//! Names follow bindgen's, so the calling code reads as before. Layouts are
//! checked by the tests below against the C definitions (`int`-sized enums;
//! the event's `id` union is 8 bytes).

#![allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    unsafe_code
)]

use std::ffi::{OsString, c_char, c_int, c_short, c_uchar, c_uint, c_void};
use std::sync::OnceLock;

use libloading::Library;

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
/// `EE_INTERNAL_ERROR`; also what the wrappers below return when the
/// library is not loaded.
pub const espeak_ERROR_EE_INTERNAL_ERROR: espeak_ERROR = -1;

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

/// The environment variable that names a libespeak-ng file to load before
/// the usual names are tried.
pub const LIBRARY_ENV: &str = "TEXTWEAVER_ESPEAK_LIBRARY";

/// The library names (and, on Windows and macOS, the usual install paths)
/// tried in order after [`LIBRARY_ENV`].
pub fn library_candidates() -> &'static [&'static str] {
    if cfg!(windows) {
        &[
            "libespeak-ng.dll",
            r"C:\Program Files\eSpeak NG\libespeak-ng.dll",
            r"C:\Program Files (x86)\eSpeak NG\libespeak-ng.dll",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "libespeak-ng.1.dylib",
            "/opt/homebrew/lib/libespeak-ng.1.dylib",
            "/usr/local/lib/libespeak-ng.1.dylib",
            "libespeak-ng.dylib",
        ]
    } else {
        &["libespeak-ng.so.1", "libespeak-ng.so"]
    }
}

/// libespeak-ng in textweaver's components folder, placed or unpacked
/// there from a components source: tried before everything else.
pub fn component_library() -> Option<std::path::PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["libespeak-ng.dll"]
    } else if cfg!(target_os = "macos") {
        &["libespeak-ng.1.dylib", "libespeak-ng.dylib"]
    } else {
        &["libespeak-ng.so.1", "libespeak-ng.so"]
    };
    let dir = textweaver_store::components_dir()?;
    textweaver_store::find_in_components(&dir, names)
}

/// The folder to hand `espeak_Initialize` for a library at `lib`: its own
/// folder when `espeak-ng-data` is beside it (a copy from the components
/// folder carries its data), else `None` (the installed data).
fn data_dir_beside(lib: &std::path::Path) -> Option<std::ffi::CString> {
    let dir = lib.parent()?;
    if !dir.join("espeak-ng-data").is_dir() {
        return None;
    }
    std::ffi::CString::new(dir.to_str()?).ok()
}

type InitializeFn = unsafe extern "C" fn(espeak_AUDIO_OUTPUT, c_int, *const c_char, c_int) -> c_int;
type SetSynthCallbackFn = unsafe extern "C" fn(t_espeak_callback);
type SynthFn = unsafe extern "C" fn(
    *const c_void,
    usize,
    c_uint,
    espeak_POSITION_TYPE,
    c_uint,
    c_uint,
    *mut c_uint,
    *mut c_void,
) -> espeak_ERROR;
type CharFn = unsafe extern "C" fn(wchar_t) -> espeak_ERROR;
type SetParameterFn = unsafe extern "C" fn(espeak_PARAMETER, c_int, c_int) -> espeak_ERROR;
type SetVoiceByNameFn = unsafe extern "C" fn(*const c_char) -> espeak_ERROR;
type ListVoicesFn = unsafe extern "C" fn(*mut espeak_VOICE) -> *mut *const espeak_VOICE;
type NoArgFn = unsafe extern "C" fn() -> espeak_ERROR;
type TextToPhonemesFn = unsafe extern "C" fn(*mut *const c_void, c_int, c_int) -> *const c_char;

/// `phonememode` bit for [`espeak_TextToPhonemes`]: IPA as UTF-8 instead
/// of eSpeak's ASCII phoneme names.
pub const espeakPHONEMES_IPA: c_int = 0x02;

/// The libespeak-ng functions the backend calls, resolved once.
struct Api {
    initialize: InitializeFn,
    set_synth_callback: SetSynthCallbackFn,
    synth: SynthFn,
    char_: CharFn,
    set_parameter: SetParameterFn,
    set_voice_by_name: SetVoiceByNameFn,
    list_voices: ListVoicesFn,
    cancel: NoArgFn,
    synchronize: NoArgFn,
    terminate: NoArgFn,
    /// `espeak_TextToPhonemes` (since eSpeak NG 1.49), for Piper's
    /// phonemizer; `None` in a library too old to have it.
    text_to_phonemes: Option<TextToPhonemesFn>,
    /// The data folder for `espeak_Initialize` when the library came with
    /// its own ([`data_dir_beside`]); `None` uses the installed data.
    data_dir: Option<std::ffi::CString>,
    /// Keeps the library mapped: the function pointers above point into
    /// it. The `Api` lives in a static, so it is never unloaded.
    _library: Library,
}

/// Copies one function pointer out of `lib`.
///
/// # Safety
///
/// `T` must be the function's C signature as `speak_lib.h` declares it,
/// and the pointer must not be called after `lib` is dropped.
unsafe fn symbol<T: Copy>(lib: &Library, name: &str) -> Result<T, String> {
    // SAFETY: the caller guarantees `T` matches the symbol's type.
    unsafe { lib.get::<T>(name) }
        .map(|s| *s)
        .map_err(|e| format!("libespeak-ng lacks {name}: {e}"))
}

impl Api {
    fn open() -> Result<Self, String> {
        let from_components = component_library();
        let data_dir = from_components.as_deref().and_then(data_dir_beside);
        let from_env = std::env::var_os(LIBRARY_ENV).filter(|v| !v.is_empty());
        let mut tried = Vec::new();
        let mut library = None;
        let mut data = None;
        for (i, name) in from_components
            .map(OsString::from)
            .into_iter()
            .chain(from_env)
            .chain(library_candidates().iter().map(OsString::from))
            .enumerate()
        {
            // SAFETY: loading libespeak-ng runs its initializers, which set
            // up only its own globals (it is a plain C library).
            match unsafe { Library::new(&name) } {
                Ok(lib) => {
                    library = Some(lib);
                    // Only the components folder's copy brings its data.
                    if i == 0 {
                        data = data_dir.clone();
                    }
                    break;
                }
                Err(e) => tried.push(format!("{}: {e}", name.to_string_lossy())),
            }
        }
        let Some(lib) = library else {
            return Err(format!(
                "libespeak-ng is not installed (tried {})",
                tried.join("; ")
            ));
        };
        // SAFETY: each type is the signature `speak_lib.h` declares for that
        // function, and every pointer is stored with `lib`, which is kept
        // for the life of the process.
        unsafe {
            Ok(Api {
                initialize: symbol(&lib, "espeak_Initialize")?,
                set_synth_callback: symbol(&lib, "espeak_SetSynthCallback")?,
                synth: symbol(&lib, "espeak_Synth")?,
                char_: symbol(&lib, "espeak_Char")?,
                set_parameter: symbol(&lib, "espeak_SetParameter")?,
                set_voice_by_name: symbol(&lib, "espeak_SetVoiceByName")?,
                list_voices: symbol(&lib, "espeak_ListVoices")?,
                cancel: symbol(&lib, "espeak_Cancel")?,
                synchronize: symbol(&lib, "espeak_Synchronize")?,
                terminate: symbol(&lib, "espeak_Terminate")?,
                text_to_phonemes: symbol(&lib, "espeak_TextToPhonemes").ok(),
                data_dir: data,
                _library: lib,
            })
        }
    }
}

static API: OnceLock<Result<Api, String>> = OnceLock::new();

fn api() -> Result<&'static Api, &'static str> {
    API.get_or_init(Api::open).as_ref().map_err(String::as_str)
}

/// The data folder to hand `espeak_Initialize`: the components folder's
/// copy's own, or null for the installed data.
pub fn data_path() -> *const c_char {
    match api() {
        Ok(Api {
            data_dir: Some(d), ..
        }) => d.as_ptr(),
        _ => std::ptr::null(),
    }
}

/// Loads libespeak-ng (once). The error says which files were tried.
pub fn load() -> Result<(), String> {
    api().map(|_| ()).map_err(str::to_owned)
}

// The wrappers below keep the C names and signatures, so the calling code
// in `ffi` reads as it did when libespeak-ng was linked. When the library
// could not be loaded each returns an error value or does nothing;
// `ffi::initialize` calls [`load`] first and reports why.

/// Starts the engine; returns the sample rate, or -1.
///
/// # Safety
///
/// As `espeak_Initialize`: `path` is null or a valid C string.
pub unsafe fn espeak_Initialize(
    output: espeak_AUDIO_OUTPUT,
    buflength: c_int,
    path: *const c_char,
    options: c_int,
) -> c_int {
    match api() {
        // SAFETY: forwarded unchanged; the caller upholds the C contract.
        Ok(a) => unsafe { (a.initialize)(output, buflength, path, options) },
        Err(_) => -1,
    }
}

/// Sets the synthesis callback.
///
/// # Safety
///
/// As `espeak_SetSynthCallback`: the callback lives for the whole program.
pub unsafe fn espeak_SetSynthCallback(callback: t_espeak_callback) {
    if let Ok(a) = api() {
        // SAFETY: forwarded unchanged.
        unsafe { (a.set_synth_callback)(callback) }
    }
}

/// Synthesizes text.
///
/// # Safety
///
/// As `espeak_Synth`: `text` points to `size` readable bytes.
#[allow(clippy::too_many_arguments)]
pub unsafe fn espeak_Synth(
    text: *const c_void,
    size: usize,
    position: c_uint,
    position_type: espeak_POSITION_TYPE,
    end_position: c_uint,
    flags: c_uint,
    unique_identifier: *mut c_uint,
    user_data: *mut c_void,
) -> espeak_ERROR {
    match api() {
        // SAFETY: forwarded unchanged.
        Ok(a) => unsafe {
            (a.synth)(
                text,
                size,
                position,
                position_type,
                end_position,
                flags,
                unique_identifier,
                user_data,
            )
        },
        Err(_) => espeak_ERROR_EE_INTERNAL_ERROR,
    }
}

/// Speaks one character's name.
///
/// # Safety
///
/// As `espeak_Char`: the engine is initialized.
pub unsafe fn espeak_Char(character: wchar_t) -> espeak_ERROR {
    match api() {
        // SAFETY: forwarded unchanged.
        Ok(a) => unsafe { (a.char_)(character) },
        Err(_) => espeak_ERROR_EE_INTERNAL_ERROR,
    }
}

/// Sets a parameter (absolute when `relative` is 0).
///
/// # Safety
///
/// As `espeak_SetParameter`: the engine is initialized.
pub unsafe fn espeak_SetParameter(
    parameter: espeak_PARAMETER,
    value: c_int,
    relative: c_int,
) -> espeak_ERROR {
    match api() {
        // SAFETY: forwarded unchanged.
        Ok(a) => unsafe { (a.set_parameter)(parameter, value, relative) },
        Err(_) => espeak_ERROR_EE_INTERNAL_ERROR,
    }
}

/// Selects a voice by name or file name.
///
/// # Safety
///
/// As `espeak_SetVoiceByName`: `name` is a valid C string.
pub unsafe fn espeak_SetVoiceByName(name: *const c_char) -> espeak_ERROR {
    match api() {
        // SAFETY: forwarded unchanged.
        Ok(a) => unsafe { (a.set_voice_by_name)(name) },
        Err(_) => espeak_ERROR_EE_INTERNAL_ERROR,
    }
}

/// The installed voices matching `voice_spec` (null: all), as a
/// null-terminated array owned by the library; null when it is not loaded.
///
/// # Safety
///
/// As `espeak_ListVoices`: `voice_spec` is null or valid.
pub unsafe fn espeak_ListVoices(voice_spec: *mut espeak_VOICE) -> *mut *const espeak_VOICE {
    match api() {
        // SAFETY: forwarded unchanged.
        Ok(a) => unsafe { (a.list_voices)(voice_spec) },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Stops speech at once.
///
/// # Safety
///
/// As `espeak_Cancel`.
pub unsafe fn espeak_Cancel() -> espeak_ERROR {
    match api() {
        // SAFETY: forwarded unchanged.
        Ok(a) => unsafe { (a.cancel)() },
        Err(_) => espeak_ERROR_EE_INTERNAL_ERROR,
    }
}

/// Waits until queued speech is done.
///
/// # Safety
///
/// As `espeak_Synchronize`.
pub unsafe fn espeak_Synchronize() -> espeak_ERROR {
    match api() {
        // SAFETY: forwarded unchanged.
        Ok(a) => unsafe { (a.synchronize)() },
        Err(_) => espeak_ERROR_EE_INTERNAL_ERROR,
    }
}

/// Shuts the engine down.
///
/// # Safety
///
/// As `espeak_Terminate`.
pub unsafe fn espeak_Terminate() -> espeak_ERROR {
    match api() {
        // SAFETY: forwarded unchanged.
        Ok(a) => unsafe { (a.terminate)() },
        Err(_) => espeak_ERROR_EE_INTERNAL_ERROR,
    }
}

/// Translates the next clause at `*textptr` into phonemes and moves
/// `*textptr` past it (to null after the last clause). Returns null when
/// the library is missing or too old to have the function.
///
/// # Safety
///
/// As `espeak_TextToPhonemes`: the engine is initialized, `*textptr`
/// points to a NUL-terminated string in the encoding `textmode` names, and
/// the returned string is read before the next call.
pub unsafe fn espeak_TextToPhonemes(
    textptr: *mut *const c_void,
    textmode: c_int,
    phonememode: c_int,
) -> *const c_char {
    match api() {
        // SAFETY: forwarded unchanged.
        Ok(Api {
            text_to_phonemes: Some(f),
            ..
        }) => unsafe { f(textptr, textmode, phonememode) },
        _ => std::ptr::null(),
    }
}

/// True when the loaded library has `espeak_TextToPhonemes`.
pub fn has_text_to_phonemes() -> bool {
    api().is_ok_and(|a| a.text_to_phonemes.is_some())
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

    #[test]
    fn a_copy_with_its_data_brings_its_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("libespeak-ng.dll");
        assert_eq!(data_dir_beside(&lib), None);
        std::fs::create_dir_all(tmp.path().join("espeak-ng-data")).unwrap();
        let got = data_dir_beside(&lib).unwrap();
        assert_eq!(got.to_str().unwrap(), tmp.path().to_str().unwrap());
    }

    /// Whatever this machine has installed, loading either works or says
    /// which files it tried, and the wrappers never crash without it.
    #[test]
    fn a_missing_library_is_reported_not_fatal() {
        assert!(!library_candidates().is_empty());
        if let Err(e) = load() {
            assert!(e.contains("libespeak-ng"), "{e}");
            // SAFETY: without the library the wrappers call nothing.
            unsafe {
                assert_eq!(espeak_Cancel(), espeak_ERROR_EE_INTERNAL_ERROR);
                assert!(espeak_ListVoices(std::ptr::null_mut()).is_null());
                assert_eq!(
                    espeak_Initialize(
                        espeak_AUDIO_OUTPUT_AUDIO_OUTPUT_RETRIEVAL,
                        0,
                        std::ptr::null(),
                        0
                    ),
                    -1
                );
            }
        }
    }
}
