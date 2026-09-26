//! The ECI library, loaded at run time with `libloading` (ADR-0007).
//!
//! This is the only module in the workspace's Eloquence support that uses
//! `unsafe`. ECI's C API is small; the calls used here are the ones the
//! feasibility spike measured on Code Factory's `eci.dll` 6.1 (32-bit
//! Windows, `stdcall`) and that Voxin's `libibmeci.so` exports on Linux
//! (x86_64, C calling convention). `extern "system"` is `stdcall` on 32-bit
//! Windows and the C convention everywhere else, so one set of signatures
//! serves both. ECI's callback `lParam` is a C `long` (32 bits on Windows,
//! 64 on Linux), hence `c_long`.
//!
//! Engine settings applied at creation: `eciInputType` = 1 (annotations
//! on; the text encoder turns backquotes into apostrophes so text cannot
//! inject annotations) and `eciSampleRate` as requested (0 = 8000 Hz,
//! 1 = 11025 Hz, 2 = 22050 Hz).
#![allow(unsafe_code)]

use std::ffi::{c_char, c_int, c_long, c_void};
use std::path::Path;

use libloading::Library;

use super::{Engine, EngineInfo, EnginePiece, SynthEvent};
use crate::protocol::PresetInfo;

type Hand = *mut c_void;
type Callback = unsafe extern "system" fn(Hand, c_int, c_long, *mut c_void) -> c_int;

/// `ECIParam` numbers used by the host.
mod param {
    pub const INPUT_TYPE: i32 = 1;
    pub const SAMPLE_RATE: i32 = 5;
    pub const LANGUAGE_DIALECT: i32 = 9;
}

/// `ECIMessage` values.
const MSG_WAVEFORM: c_int = 0;
const MSG_INDEX: c_int = 2;
/// `ECICallbackReturn` values.
const DATA_PROCESSED: c_int = 1;
const DATA_ABORT: c_int = 2;

/// Samples in the engine's output buffer (one callback delivers at most this).
const BUFFER_SAMPLES: usize = 4096;

struct Api {
    version: unsafe extern "system" fn(*mut c_char),
    new: unsafe extern "system" fn() -> Hand,
    delete: unsafe extern "system" fn(Hand) -> Hand,
    add_text: unsafe extern "system" fn(Hand, *const c_char) -> c_int,
    insert_index: unsafe extern "system" fn(Hand, c_int) -> c_int,
    synthesize: unsafe extern "system" fn(Hand) -> c_int,
    synchronize: unsafe extern "system" fn(Hand) -> c_int,
    stop: unsafe extern "system" fn(Hand) -> c_int,
    register_callback: unsafe extern "system" fn(Hand, Option<Callback>, *mut c_void),
    set_output_buffer: unsafe extern "system" fn(Hand, c_int, *mut i16) -> c_int,
    set_param: unsafe extern "system" fn(Hand, c_int, c_int) -> c_int,
    get_param: unsafe extern "system" fn(Hand, c_int) -> c_int,
    set_voice_param: unsafe extern "system" fn(Hand, c_int, c_int, c_int) -> c_int,
    get_voice_param: unsafe extern "system" fn(Hand, c_int, c_int) -> c_int,
    copy_voice: unsafe extern "system" fn(Hand, c_int, c_int) -> c_int,
    get_voice_name: Option<unsafe extern "system" fn(Hand, c_int, *mut c_char) -> c_int>,
    available_languages: Option<unsafe extern "system" fn(*mut c_int, *mut c_int) -> c_int>,
    error_message: Option<unsafe extern "system" fn(Hand, *mut c_void)>,
    clear_input: Option<unsafe extern "system" fn(Hand) -> c_int>,
    // Declared last so the function pointers above are never used after
    // the library is unloaded (fields drop in order; `EciEngine::drop`
    // deletes the engine first).
    _lib: Library,
}

/// Looks up a required symbol and copies the function pointer out.
macro_rules! required {
    ($lib:expr, $name:literal) => {{
        // SAFETY: the symbol is one of ECI's documented exports and the
        // field it initializes has that export's C signature (checked
        // against the spike on eci.dll 6.1 and against Voxin's eci.h).
        let sym = unsafe { $lib.get($name) }
            .map_err(|e| format!("{} is missing from the ECI library: {e}", $name))?;
        *sym
    }};
}

/// Looks up an optional symbol.
macro_rules! optional {
    ($lib:expr, $name:literal) => {{
        // SAFETY: as for `required!`; absence is tolerated.
        unsafe { $lib.get($name) }.ok().map(|s| *s)
    }};
}

impl Api {
    fn load(path: &Path) -> Result<Api, String> {
        // SAFETY: loading runs the library's initializers. The path comes
        // from the user's configuration or the engine's standard install
        // location; ECI libraries have no initializers with preconditions.
        let lib = unsafe { Library::new(path) }
            .map_err(|e| format!("cannot load {}: {e}", path.display()))?;
        Ok(Api {
            version: required!(lib, "eciVersion"),
            new: required!(lib, "eciNew"),
            delete: required!(lib, "eciDelete"),
            add_text: required!(lib, "eciAddText"),
            insert_index: required!(lib, "eciInsertIndex"),
            synthesize: required!(lib, "eciSynthesize"),
            synchronize: required!(lib, "eciSynchronize"),
            stop: required!(lib, "eciStop"),
            register_callback: required!(lib, "eciRegisterCallback"),
            set_output_buffer: required!(lib, "eciSetOutputBuffer"),
            set_param: required!(lib, "eciSetParam"),
            get_param: required!(lib, "eciGetParam"),
            set_voice_param: required!(lib, "eciSetVoiceParam"),
            get_voice_param: required!(lib, "eciGetVoiceParam"),
            copy_voice: required!(lib, "eciCopyVoice"),
            get_voice_name: optional!(lib, "eciGetVoiceName"),
            available_languages: optional!(lib, "eciGetAvailableLanguages"),
            error_message: optional!(lib, "eciErrorMessage"),
            clear_input: optional!(lib, "eciClearInput"),
            _lib: lib,
        })
    }
}

/// Converts a NUL-terminated buffer the engine filled into a string.
fn c_buf_to_string(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    // Engine strings are Windows-1252 / Latin-1; map bytes to chars 1:1.
    buf[..end].iter().map(|&b| char::from(b)).collect()
}

/// The real engine.
pub struct EciEngine {
    api: Api,
    h: Hand,
    /// The engine writes PCM here; allocated once, never reallocated.
    buf: Vec<i16>,
    sample_rate: u32,
    dialect: u32,
}

impl std::fmt::Debug for EciEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EciEngine")
            .field("sample_rate", &self.sample_rate)
            .field("dialect", &self.dialect)
            .finish_non_exhaustive()
    }
}

/// Synthesis state shared with the callback.
struct Ctx<'a, 'b> {
    buf: *const i16,
    out: &'a mut (dyn FnMut(SynthEvent<'_>) -> bool + 'b),
    aborted: bool,
}

unsafe extern "system" fn callback(
    _h: Hand,
    msg: c_int,
    lparam: c_long,
    data: *mut c_void,
) -> c_int {
    if data.is_null() {
        return DATA_PROCESSED;
    }
    // SAFETY: `data` is the `Ctx` that `synthesize` registered and keeps
    // alive (on its stack) until `eciSynchronize` has returned and the
    // callback has been re-registered with a null pointer. ECI calls the
    // callback while `synthesize` is blocked in `eciSynchronize`, so there
    // is no other live reference to the `Ctx`.
    let ctx = unsafe { &mut *data.cast::<Ctx<'_, '_>>() };
    if ctx.aborted {
        return DATA_ABORT;
    }
    let event = match msg {
        MSG_WAVEFORM => {
            let n = usize::try_from(lparam).unwrap_or(0).min(BUFFER_SAMPLES);
            // SAFETY: `buf` points at the engine's output buffer of
            // `BUFFER_SAMPLES` samples, and ECI has just written `lparam`
            // samples into it (clamped to the buffer size above).
            SynthEvent::Audio(unsafe { std::slice::from_raw_parts(ctx.buf, n) })
        }
        MSG_INDEX => SynthEvent::Mark(u32::try_from(lparam).unwrap_or(u32::MAX)),
        _ => return DATA_PROCESSED,
    };
    let out = &mut ctx.out;
    let go =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (out)(event))).unwrap_or(false);
    if go {
        DATA_PROCESSED
    } else {
        ctx.aborted = true;
        DATA_ABORT
    }
}

impl EciEngine {
    /// Loads the library at `path` and creates an engine at `sample_rate`
    /// Hz (8000, 11025, or 22050; `None` keeps the engine's default).
    pub fn load(path: &Path, sample_rate: Option<u32>) -> Result<Self, String> {
        let api = Api::load(path)?;
        // SAFETY: `eciNew` takes no arguments and returns a handle or null.
        let h = unsafe { (api.new)() };
        if h.is_null() {
            return Err("eciNew failed (is the engine installed and licensed?)".into());
        }
        let mut engine = EciEngine {
            api,
            h,
            buf: vec![0; BUFFER_SAMPLES],
            sample_rate: 11025,
            dialect: 0,
        };
        // Voxin rejects an output buffer until a callback is registered.
        // SAFETY: `callback` matches ECI's callback signature and ignores
        // calls with null data (synthesis registers real data later).
        unsafe { (engine.api.register_callback)(h, Some(callback), std::ptr::null_mut()) };
        let len = c_int::try_from(engine.buf.len()).unwrap_or(c_int::MAX);
        // SAFETY: `h` is a live handle; `buf` has `len` samples and lives as
        // long as the engine (it is never resized), and is released only
        // after `eciDelete` in `Drop`.
        let ok = unsafe { (engine.api.set_output_buffer)(h, len, engine.buf.as_mut_ptr()) };
        if ok == 0 {
            return Err(engine.error("eciSetOutputBuffer failed"));
        }
        // SAFETY: plain integer parameters on a live handle.
        unsafe { (engine.api.set_param)(h, param::INPUT_TYPE, 1) };
        if let Some(hz) = sample_rate {
            let code = match hz {
                0..=9000 => 0,
                9001..=16000 => 1,
                _ => 2,
            };
            // SAFETY: plain integer parameter on a live handle.
            unsafe { (engine.api.set_param)(h, param::SAMPLE_RATE, code) };
        }
        // SAFETY: plain integer queries on a live handle.
        let code = unsafe { (engine.api.get_param)(h, param::SAMPLE_RATE) };
        engine.sample_rate = [8000, 11025, 22050][usize::try_from(code.clamp(0, 2)).unwrap_or(1)];
        // SAFETY: as above.
        let dialect = unsafe { (engine.api.get_param)(h, param::LANGUAGE_DIALECT) };
        engine.dialect = u32::try_from(dialect).unwrap_or(0);
        Ok(engine)
    }

    fn error(&self, what: &str) -> String {
        let Some(f) = self.api.error_message else {
            return what.to_string();
        };
        let mut buf = [0u8; 512];
        // SAFETY: ECI writes a NUL-terminated message of at most 100 bytes
        // (`ECI_ERROR_MESSAGE_LENGTH`) into the buffer, which is larger.
        unsafe { f(self.h, buf.as_mut_ptr().cast()) };
        let msg = c_buf_to_string(&buf);
        if msg.trim().is_empty() {
            what.to_string()
        } else {
            format!("{what}: {}", msg.trim())
        }
    }

    fn version(&self) -> String {
        let mut buf = [0u8; 256];
        // SAFETY: ECI writes a short NUL-terminated version ("6.1.0.0")
        // into the caller's buffer, which is far larger.
        unsafe { (self.api.version)(buf.as_mut_ptr().cast()) };
        c_buf_to_string(&buf)
    }

    fn dialects(&self) -> Vec<u32> {
        let Some(f) = self.api.available_languages else {
            return Vec::new();
        };
        let mut list = [0 as c_int; 64];
        let mut n: c_int = 64;
        // SAFETY: `list` holds `n` entries; ECI writes at most `n` codes
        // and stores the count in `n`.
        let r = unsafe { f(list.as_mut_ptr(), &raw mut n) };
        if r != 0 || n <= 0 {
            return Vec::new();
        }
        let n = usize::try_from(n).unwrap_or(0).min(list.len());
        list[..n]
            .iter()
            .filter_map(|&c| u32::try_from(c).ok())
            .collect()
    }

    fn presets(&self) -> Vec<PresetInfo> {
        (1..=8)
            .map(|voice: c_int| {
                let name = self.api.get_voice_name.map_or_else(String::new, |f| {
                    let mut buf = [0u8; 64];
                    // SAFETY: ECI writes a NUL-terminated name of at most
                    // 30 bytes (`ECI_VOICE_NAME_LENGTH`) into the buffer.
                    unsafe { f(self.h, voice, buf.as_mut_ptr().cast()) };
                    c_buf_to_string(&buf).trim().to_string()
                });
                let mut params = [0; 8];
                for (p, slot) in params.iter_mut().enumerate() {
                    let p = c_int::try_from(p).unwrap_or(0);
                    // SAFETY: plain integer query on a live handle.
                    *slot = unsafe { (self.api.get_voice_param)(self.h, voice, p) };
                }
                PresetInfo { name, params }
            })
            .collect()
    }
}

impl Engine for EciEngine {
    fn info(&mut self) -> EngineInfo {
        EngineInfo {
            sample_rate: self.sample_rate,
            version: self.version(),
            dialects: self.dialects(),
            default_dialect: self.dialect,
            presets: self.presets(),
        }
    }

    fn dialect(&self) -> u32 {
        self.dialect
    }

    fn set_voice(&mut self, dialect: u32, preset: u8) -> Result<(), String> {
        if dialect != self.dialect {
            let code = c_int::try_from(dialect).map_err(|_| format!("bad dialect {dialect:#x}"))?;
            // SAFETY: plain integer parameter on a live handle.
            let r = unsafe { (self.api.set_param)(self.h, param::LANGUAGE_DIALECT, code) };
            if r < 0 {
                return Err(self.error(&format!("language {dialect:#x} is not available")));
            }
            self.dialect = dialect;
        }
        if (1..=8).contains(&preset) {
            // SAFETY: copies preset `preset` into the active voice (0) of a
            // live handle; both indices are in ECI's range 0..=8.
            let r = unsafe { (self.api.copy_voice)(self.h, c_int::from(preset), 0) };
            if r == 0 {
                return Err(self.error(&format!("cannot select voice preset {preset}")));
            }
        }
        Ok(())
    }

    fn set_voice_param(&mut self, param: u8, value: i32) -> Result<(), String> {
        // SAFETY: plain integer parameter on the active voice of a live handle.
        let r = unsafe { (self.api.set_voice_param)(self.h, 0, c_int::from(param), value) };
        if r < 0 {
            return Err(self.error(&format!("voice parameter {param} = {value} rejected")));
        }
        Ok(())
    }

    fn synthesize(
        &mut self,
        pieces: &[EnginePiece],
        out: &mut dyn FnMut(SynthEvent<'_>) -> bool,
    ) -> Result<bool, String> {
        let h = self.h;
        for p in pieces {
            match p {
                EnginePiece::Index(i) => {
                    let i = c_int::try_from(*i).unwrap_or(c_int::MAX);
                    // SAFETY: plain integer argument on a live handle.
                    if unsafe { (self.api.insert_index)(h, i) } == 0 {
                        return Err(self.error("eciInsertIndex failed"));
                    }
                }
                EnginePiece::Text(t) => {
                    let mut z = Vec::with_capacity(t.len() + 1);
                    z.extend(t.iter().map(|&b| if b == 0 { b' ' } else { b }));
                    z.push(0);
                    // SAFETY: `z` is NUL-terminated and outlives the call;
                    // ECI copies the text into its input buffer.
                    if unsafe { (self.api.add_text)(h, z.as_ptr().cast()) } == 0 {
                        return Err(self.error("eciAddText failed"));
                    }
                }
            }
        }
        let mut ctx = Ctx {
            buf: self.buf.as_ptr(),
            out,
            aborted: false,
        };
        let data: *mut c_void = (&raw mut ctx).cast();
        // SAFETY: `callback` matches ECI's callback signature, and `data`
        // points at `ctx`, which outlives synthesis: it is unregistered
        // (null data) below before `ctx` goes out of scope.
        unsafe { (self.api.register_callback)(h, Some(callback), data) };
        // SAFETY: live handle; the callback context is registered above.
        let started = unsafe { (self.api.synthesize)(h) } != 0;
        let finished = if started {
            // SAFETY: blocks until synthesis ends; callbacks run meanwhile.
            (unsafe { (self.api.synchronize)(h) }) != 0
        } else {
            false
        };
        // SAFETY: re-registers with a null context so no callback can reach
        // `ctx` after this function returns.
        unsafe { (self.api.register_callback)(h, Some(callback), std::ptr::null_mut()) };
        let aborted = ctx.aborted;
        if aborted {
            // SAFETY: live handle; stops any remaining synthesis and
            // discards queued input.
            unsafe {
                (self.api.stop)(h);
                if let Some(clear) = self.api.clear_input {
                    clear(h);
                }
            }
            return Ok(false);
        }
        if !started {
            return Err(self.error("eciSynthesize failed"));
        }
        if !finished {
            return Err(self.error("eciSynchronize failed"));
        }
        Ok(true)
    }
}

impl Drop for EciEngine {
    fn drop(&mut self) {
        // SAFETY: `h` came from `eciNew` and is deleted exactly once; the
        // library (in `api`) is unloaded only after this, when fields drop.
        unsafe { (self.api.delete)(self.h) };
    }
}
