//! The SAPI5 engine: `ISpVoice` through the `windows` crate.
//!
//! This is the only module in the crate with `unsafe` code: COM calls, and a
//! hand-written `IStream` ([`PcmStream`]) that receives the voice's PCM as it
//! is synthesized and forwards it to the backend at once. (The `windows`
//! crate's `#[implement]` macro needs a direct `windows-core` dependency,
//! which the workspace does not have; the stream is small enough to write
//! by hand.)
//!
//! Synthesis: COM is initialized multithreaded on the host's main thread.
//! The voice's output is an `SpStream` over [`PcmStream`] in a fixed format
//! (22,050 Hz, 16-bit mono), so SAPI converts every engine's native format.
//! Each utterance is spoken with `SPF_ASYNC`; the main thread waits on the
//! voice's notification event, reads `SPEI_START_INPUT_STREAM`,
//! `SPEI_WORD_BOUNDARY`, and `SPEI_END_INPUT_STREAM`, and reports each word
//! with its audio offset relative to the utterance's start. SAPI writes the
//! audio from its own thread straight into [`PcmStream`]. A stop purges the
//! voice (`SPF_PURGEBEFORESPEAK`), and audio still arriving for the
//! stopped utterance is dropped.
//!
//! Pitch: plain text is spoken with `SPF_IS_NOT_XML`. A pitch other than 0
//! wraps the XML-escaped text in `<pitch absmiddle="n">`; event positions
//! in the XML are mapped back to positions in the plain text.

#![allow(unsafe_code)]

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{
    E_NOINTERFACE, E_NOTIMPL, E_POINTER, HANDLE, LPARAM, S_OK, STG_E_INVALIDFUNCTION, WAIT_OBJECT_0,
};
use windows::Win32::Media::Audio::WAVEFORMATEX;
use windows::Win32::Media::Speech::{
    IEnumSpObjectTokens, ISpDataKey, ISpObjectToken, ISpObjectTokenCategory, ISpStream, ISpVoice,
    SPEI_END_INPUT_STREAM, SPEI_START_INPUT_STREAM, SPEI_WORD_BOUNDARY, SPEVENT, SPF_ASYNC,
    SPF_IS_NOT_XML, SPF_IS_XML, SPF_PURGEBEFORESPEAK, SpObjectToken, SpObjectTokenCategory,
    SpStream, SpVoice,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
    CoUninitialize, ISequentialStream, ISequentialStream_Vtbl, IStream, IStream_Vtbl, STATSTG,
    STGTY_STREAM, STREAM_SEEK, STREAM_SEEK_CUR, STREAM_SEEK_END, STREAM_SEEK_SET,
};
use windows::Win32::System::Threading::WaitForSingleObject;
use windows::core::{GUID, HRESULT, IUnknown, IUnknown_Vtbl, Interface, PCWSTR, PWSTR};

use textweaver_enginehost::serve::log_line;

use super::{Engine, Out, SpeakText};
use crate::protocol::{EndStatus, Reply, VoiceToken};

/// `SPDFID_WaveFormatEx` (sapi.h): the stream format is a `WAVEFORMATEX`.
const SPDFID_WAVE_FORMAT_EX: GUID = GUID::from_u128(0xc31adbae_527f_4ff5_a230_f62bb61ff70c);

/// `SPFEI_FLAGCHECK` (sapi.h): two reserved bits every interest mask sets.
const SPFEI_FLAGCHECK: u64 = (1 << 30) | (1 << 33);

/// `SPFEI(e)`: the interest bit of one event.
fn spfei(e: i32) -> u64 {
    (1u64 << e) | SPFEI_FLAGCHECK
}

/// `SPET_LPARAM_IS_*` (sapi.h): what an event's `lParam` holds.
const SPET_LPARAM_IS_TOKEN: i32 = 1;
const SPET_LPARAM_IS_POINTER: i32 = 2;
const SPET_LPARAM_IS_OBJECT: i32 = 3;
const SPET_LPARAM_IS_STRING: i32 = 4;

/// Longest time an utterance may go without an event or audio before the
/// host gives up on the engine.
const STALL_TIMEOUT: Duration = Duration::from_secs(30);

/// A nul-terminated UTF-16 copy of `s`.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Takes ownership of a COM-allocated string.
fn take_pwstr(p: PWSTR) -> String {
    if p.is_null() {
        return String::new();
    }
    // SAFETY: SAPI returned a valid, nul-terminated, CoTaskMemAlloc'ed
    // string; it is read once and freed once.
    unsafe {
        let s = p.to_string().unwrap_or_default();
        CoTaskMemFree(Some(p.0 as *const c_void));
        s
    }
}

/// State shared between the host and the [`PcmStream`] SAPI writes into.
#[derive(Debug)]
struct StreamState {
    out: Arc<Out>,
    /// The utterance whose audio is being written; 0 drops audio (after a
    /// stop, or between utterances).
    token: AtomicU64,
    /// Samples written for the current token.
    samples: AtomicU64,
    /// Total bytes written to the stream (its logical position and size).
    position: AtomicU64,
    /// An odd trailing byte from the last write (SAPI writes whole samples;
    /// this is defensive).
    carry: Mutex<Option<u8>>,
    /// Last time the stream saw audio, for stall detection.
    last_write: Mutex<Instant>,
}

impl StreamState {
    fn begin(&self, token: u64) {
        *self.carry.lock().unwrap_or_else(|e| e.into_inner()) = None;
        self.samples.store(0, Ordering::SeqCst);
        self.token.store(token, Ordering::SeqCst);
        self.touch();
    }

    fn end(&self) -> u64 {
        self.token.store(0, Ordering::SeqCst);
        self.samples.load(Ordering::SeqCst)
    }

    fn touch(&self) {
        *self.last_write.lock().unwrap_or_else(|e| e.into_inner()) = Instant::now();
    }

    fn idle_for(&self) -> Duration {
        self.last_write
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .elapsed()
    }

    fn write(&self, bytes: &[u8]) {
        self.position
            .fetch_add(bytes.len() as u64, Ordering::SeqCst);
        self.touch();
        let token = self.token.load(Ordering::SeqCst);
        if token == 0 {
            return;
        }
        let mut carry = self.carry.lock().unwrap_or_else(|e| e.into_inner());
        let mut data: Vec<u8> = Vec::with_capacity(bytes.len() + 1);
        if let Some(b) = carry.take() {
            data.push(b);
        }
        data.extend_from_slice(bytes);
        if data.len() % 2 == 1 {
            *carry = data.pop();
        }
        drop(carry);
        if data.is_empty() {
            return;
        }
        let samples: Vec<i16> = data
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect();
        self.samples
            .fetch_add(samples.len() as u64, Ordering::SeqCst);
        self.out.send(&Reply::Audio { token, samples });
    }
}

/// A write-only COM `IStream` that forwards PCM to the backend.
#[repr(C)]
struct PcmStream {
    vtbl: &'static IStream_Vtbl,
    refs: AtomicU32,
    state: Arc<StreamState>,
}

static PCM_STREAM_VTBL: IStream_Vtbl = IStream_Vtbl {
    base__: ISequentialStream_Vtbl {
        base__: IUnknown_Vtbl {
            QueryInterface: pcm_query_interface,
            AddRef: pcm_add_ref,
            Release: pcm_release,
        },
        Read: pcm_read,
        Write: pcm_write,
    },
    Seek: pcm_seek,
    SetSize: pcm_set_size,
    CopyTo: pcm_copy_to,
    Commit: pcm_commit,
    Revert: pcm_not_impl0,
    LockRegion: pcm_lock_region,
    UnlockRegion: pcm_lock_region,
    Stat: pcm_stat,
    Clone: pcm_clone,
};

impl PcmStream {
    /// A new stream as an owned `IStream` (reference count 1).
    fn create(state: Arc<StreamState>) -> IStream {
        let obj = Box::new(PcmStream {
            vtbl: &PCM_STREAM_VTBL,
            refs: AtomicU32::new(1),
            state,
        });
        // SAFETY: `obj` starts with a pointer to a complete IStream vtable
        // whose functions all treat `this` as a `PcmStream`; the returned
        // interface owns the one reference the object starts with, and the
        // object frees itself when the count reaches zero.
        unsafe { IStream::from_raw(Box::into_raw(obj).cast::<c_void>()) }
    }

    /// # Safety
    /// `this` must point to a live `PcmStream` (every vtable entry is only
    /// ever called by COM with the pointer `create` handed out).
    unsafe fn from_this<'a>(this: *mut c_void) -> &'a PcmStream {
        // SAFETY: per the function's contract.
        unsafe { &*(this as *const PcmStream) }
    }
}

unsafe extern "system" fn pcm_query_interface(
    this: *mut c_void,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    if iid.is_null() || out.is_null() {
        return E_POINTER;
    }
    // SAFETY: COM passes valid pointers (checked for null above) and a live
    // object.
    unsafe {
        let iid = &*iid;
        if *iid == IUnknown::IID || *iid == ISequentialStream::IID || *iid == IStream::IID {
            pcm_add_ref(this);
            *out = this;
            S_OK
        } else {
            *out = std::ptr::null_mut();
            E_NOINTERFACE
        }
    }
}

unsafe extern "system" fn pcm_add_ref(this: *mut c_void) -> u32 {
    // SAFETY: COM calls this on a live object.
    let s = unsafe { PcmStream::from_this(this) };
    s.refs.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn pcm_release(this: *mut c_void) -> u32 {
    // SAFETY: COM calls this on a live object; the last release frees the
    // box `create` leaked, exactly once.
    unsafe {
        let s = PcmStream::from_this(this);
        let left = s.refs.fetch_sub(1, Ordering::SeqCst) - 1;
        if left == 0 {
            drop(Box::from_raw(this as *mut PcmStream));
        }
        left
    }
}

unsafe extern "system" fn pcm_read(
    _this: *mut c_void,
    _pv: *mut c_void,
    _cb: u32,
    read: *mut u32,
) -> HRESULT {
    if !read.is_null() {
        // SAFETY: a non-null out pointer from COM.
        unsafe { *read = 0 };
    }
    // Write-only: nothing to read back.
    STG_E_INVALIDFUNCTION
}

unsafe extern "system" fn pcm_write(
    this: *mut c_void,
    pv: *const c_void,
    cb: u32,
    written: *mut u32,
) -> HRESULT {
    if pv.is_null() && cb > 0 {
        return E_POINTER;
    }
    // SAFETY: COM passes a live object and `cb` readable bytes at `pv`.
    unsafe {
        let s = PcmStream::from_this(this);
        let bytes = if cb == 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(pv.cast::<u8>(), cb as usize)
        };
        s.state.write(bytes);
        if !written.is_null() {
            *written = cb;
        }
    }
    S_OK
}

unsafe extern "system" fn pcm_seek(
    this: *mut c_void,
    moveby: i64,
    origin: STREAM_SEEK,
    newpos: *mut u64,
) -> HRESULT {
    // SAFETY: a live object and an optional out pointer from COM.
    unsafe {
        let s = PcmStream::from_this(this);
        let pos = s.state.position.load(Ordering::SeqCst);
        // The stream only moves forward: report the position for queries,
        // accept seeks to the current end, refuse anything else.
        let target = match origin {
            STREAM_SEEK_SET => Some(moveby),
            STREAM_SEEK_CUR | STREAM_SEEK_END => (pos as i64).checked_add(moveby),
            _ => None,
        };
        if target != Some(pos as i64) {
            log_line(&format!(
                "sapi host: refused a seek on the output stream ({origin:?}, {moveby})"
            ));
            return STG_E_INVALIDFUNCTION;
        }
        if !newpos.is_null() {
            *newpos = pos;
        }
    }
    S_OK
}

unsafe extern "system" fn pcm_set_size(_this: *mut c_void, _size: u64) -> HRESULT {
    S_OK
}

unsafe extern "system" fn pcm_copy_to(
    _this: *mut c_void,
    _to: *mut c_void,
    _cb: u64,
    _read: *mut u64,
    _written: *mut u64,
) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn pcm_commit(_this: *mut c_void, _flags: u32) -> HRESULT {
    S_OK
}

unsafe extern "system" fn pcm_not_impl0(_this: *mut c_void) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn pcm_lock_region(
    _this: *mut c_void,
    _offset: u64,
    _cb: u64,
    _kind: u32,
) -> HRESULT {
    E_NOTIMPL
}

unsafe extern "system" fn pcm_stat(this: *mut c_void, stat: *mut STATSTG, _flags: u32) -> HRESULT {
    if stat.is_null() {
        return E_POINTER;
    }
    // SAFETY: a live object and a writable STATSTG from COM.
    unsafe {
        let s = PcmStream::from_this(this);
        *stat = STATSTG {
            r#type: STGTY_STREAM.0 as u32,
            cbSize: s.state.position.load(Ordering::SeqCst),
            ..STATSTG::default()
        };
    }
    S_OK
}

unsafe extern "system" fn pcm_clone(_this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    if !out.is_null() {
        // SAFETY: a non-null out pointer from COM.
        unsafe { *out = std::ptr::null_mut() };
    }
    E_NOTIMPL
}

/// The SAPI5 engine of the host.
pub struct SapiEngine {
    voice: ISpVoice,
    /// Keeps the output stream alive for the voice's lifetime.
    _output: ISpStream,
    state: Arc<StreamState>,
    notify: HANDLE,
    /// Declared last so COM is uninitialized after every interface above is
    /// released (fields drop in declaration order).
    _com: ComGuard,
}

impl SapiEngine {
    /// Initializes COM on this thread and creates a voice writing into a
    /// [`PcmStream`] at `sample_rate` Hz, 16-bit mono.
    pub fn new(out: Arc<Out>, sample_rate: u32) -> Result<SapiEngine, String> {
        // SAFETY: COM is initialized once on the host's main thread, before
        // any COM call, and every interface below is used on this thread or
        // (the stream) is free-threaded.
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .ok()
                .map_err(|e| format!("CoInitializeEx: {e}"))?;
            // Declared first, so on an early return it drops after the
            // interfaces created below.
            let com = ComGuard;
            let voice: ISpVoice = CoCreateInstance(&SpVoice, None, CLSCTX_ALL)
                .map_err(|e| format!("cannot create SpVoice: {e}"))?;
            let state = Arc::new(StreamState {
                out,
                token: AtomicU64::new(0),
                samples: AtomicU64::new(0),
                position: AtomicU64::new(0),
                carry: Mutex::new(None),
                last_write: Mutex::new(Instant::now()),
            });
            let base = PcmStream::create(Arc::clone(&state));
            let output: ISpStream = CoCreateInstance(&SpStream, None, CLSCTX_ALL)
                .map_err(|e| format!("cannot create SpStream: {e}"))?;
            let format = WAVEFORMATEX {
                wFormatTag: 1, // WAVE_FORMAT_PCM
                nChannels: 1,
                nSamplesPerSec: sample_rate,
                nAvgBytesPerSec: sample_rate * 2,
                nBlockAlign: 2,
                wBitsPerSample: 16,
                cbSize: 0,
            };
            output
                .SetBaseStream(&base, &SPDFID_WAVE_FORMAT_EX, &format)
                .map_err(|e| format!("SetBaseStream: {e}"))?;
            voice
                .SetOutput(&output, false)
                .map_err(|e| format!("SetOutput: {e}"))?;
            let interest = spfei(SPEI_START_INPUT_STREAM.0)
                | spfei(SPEI_END_INPUT_STREAM.0)
                | spfei(SPEI_WORD_BOUNDARY.0);
            voice
                .SetInterest(interest, interest)
                .map_err(|e| format!("SetInterest: {e}"))?;
            voice
                .SetNotifyWin32Event()
                .map_err(|e| format!("SetNotifyWin32Event: {e}"))?;
            let notify = voice.GetNotifyEventHandle();
            Ok(SapiEngine {
                voice,
                _output: output,
                state,
                notify,
                _com: com,
            })
        }
    }

    /// Reads every queued event, calling `f(event id, audio offset in
    /// bytes, wParam, lParam)` and freeing whatever the event owns.
    fn drain_events(&self, mut f: impl FnMut(i32, u64, usize, isize)) {
        loop {
            let mut events = [SPEVENT::default(); 16];
            let mut fetched = 0u32;
            // SAFETY: `events` has room for the 16 requested; each fetched
            // event's lParam is released according to its type exactly once.
            unsafe {
                if self
                    .voice
                    .GetEvents(16, events.as_mut_ptr(), &mut fetched)
                    .is_err()
                {
                    return;
                }
                for e in &events[..(fetched as usize).min(16)] {
                    let id = e._bitfield & 0xffff;
                    let kind = (e._bitfield >> 16) & 0xffff;
                    f(id, e.ullAudioStreamOffset, e.wParam.0, e.lParam.0);
                    free_lparam(kind, e.lParam);
                }
            }
            if fetched < 16 {
                return;
            }
        }
    }

    fn purge(&self) {
        self.state.end();
        // SAFETY: a null text with SPF_PURGEBEFORESPEAK is SAPI's documented
        // way to stop; the voice is live.
        unsafe {
            let _ = self.voice.Speak(
                PCWSTR::null(),
                (SPF_ASYNC.0 | SPF_PURGEBEFORESPEAK.0) as u32,
                None,
            );
            let _ = self.voice.WaitUntilDone(2000);
        }
        self.drain_events(|_, _, _, _| {});
    }
}

/// Releases what an event's `lParam` owns (`SpClearEvent`).
///
/// # Safety
/// `lparam` must come from an event SAPI just returned, with type `kind`.
unsafe fn free_lparam(kind: i32, lparam: LPARAM) {
    if lparam.0 == 0 {
        return;
    }
    // SAFETY: per the function's contract, the pointer is owned by us now.
    unsafe {
        match kind {
            SPET_LPARAM_IS_POINTER | SPET_LPARAM_IS_STRING => {
                CoTaskMemFree(Some(lparam.0 as *const c_void));
            }
            SPET_LPARAM_IS_TOKEN | SPET_LPARAM_IS_OBJECT => {
                drop(IUnknown::from_raw(lparam.0 as *mut c_void));
            }
            _ => {}
        }
    }
}

impl Drop for SapiEngine {
    fn drop(&mut self) {
        self.state.end();
        // SAFETY: the voice is live (fields drop after this body, and the
        // COM guard last of all).
        unsafe {
            let _ = self.voice.Speak(
                PCWSTR::null(),
                (SPF_ASYNC.0 | SPF_PURGEBEFORESPEAK.0) as u32,
                None,
            );
        }
    }
}

/// Uninitializes COM when the host's engine is gone.
struct ComGuard;

impl Drop for ComGuard {
    fn drop(&mut self) {
        // SAFETY: paired with the CoInitializeEx in `SapiEngine::new`; the
        // engine (and every interface) was dropped before this guard.
        unsafe { CoUninitialize() };
    }
}

/// Reads a string value from a data key; empty when absent.
fn get_string(key: &ISpDataKey, name: &str) -> String {
    let n = wide(name);
    // SAFETY: `n` is nul-terminated and outlives the call; the result is
    // freed by `take_pwstr`.
    unsafe {
        key.GetStringValue(PCWSTR(n.as_ptr()))
            .map(take_pwstr)
            .unwrap_or_default()
    }
}

fn token_info(token: &ISpObjectToken) -> Option<VoiceToken> {
    // SAFETY: plain COM calls on a live token; strings are freed by
    // `take_pwstr`.
    unsafe {
        let token_id = take_pwstr(token.GetId().ok()?);
        let key: &ISpDataKey = token;
        let attrs = wide("Attributes");
        let attributes = key.OpenKey(PCWSTR(attrs.as_ptr())).ok();
        let attr = |name: &str| {
            attributes
                .as_ref()
                .map(|a| get_string(a, name))
                .unwrap_or_default()
        };
        let mut name = attr("Name");
        if name.is_empty() {
            name = key
                .GetStringValue(PCWSTR::null())
                .map(take_pwstr)
                .unwrap_or_default();
        }
        Some(VoiceToken {
            token_id,
            name,
            language: attr("Language"),
            gender: attr("Gender"),
            vendor: attr("Vendor"),
        })
    }
}

impl Engine for SapiEngine {
    fn set_voice(&mut self, token_id: &str) -> Result<(), String> {
        // SAFETY: COM calls on this thread; `id` outlives `SetId`.
        unsafe {
            if token_id.is_empty() {
                return self
                    .voice
                    .SetVoice(None::<&ISpObjectToken>)
                    .map_err(|e| format!("cannot select the default voice: {e}"));
            }
            let token: ISpObjectToken = CoCreateInstance(&SpObjectToken, None, CLSCTX_ALL)
                .map_err(|e| format!("cannot create a voice token: {e}"))?;
            let id = wide(token_id);
            token
                .SetId(PCWSTR::null(), PCWSTR(id.as_ptr()), false)
                .map_err(|e| format!("no voice token {token_id}: {e}"))?;
            self.voice
                .SetVoice(&token)
                .map_err(|e| format!("cannot load voice {token_id}: {e}"))
        }
    }

    fn set_rate(&mut self, rate: i8) -> Result<(), String> {
        // SAFETY: a COM call on a live voice.
        unsafe {
            self.voice
                .SetRate(i32::from(rate.clamp(-10, 10)))
                .map_err(|e| format!("SetRate: {e}"))
        }
    }

    fn speak(
        &mut self,
        token: u64,
        text: &str,
        pitch: i8,
        out: &Out,
        stopped: &dyn Fn() -> bool,
    ) -> Result<(EndStatus, u64), String> {
        let prepared = SpeakText::new(text, pitch);
        let flags = SPF_ASYNC.0
            | if prepared.xml() {
                SPF_IS_XML.0
            } else {
                SPF_IS_NOT_XML.0
            };
        self.state.begin(token);
        // SAFETY: `prepared.wide` is nul-terminated and stays alive until
        // this function returns, after synthesis ended or was purged.
        let spoke = unsafe {
            self.voice
                .Speak(PCWSTR(prepared.wide.as_ptr()), flags as u32, None)
        };
        if let Err(e) = spoke {
            self.state.end();
            return Err(format!("Speak: {e}"));
        }
        let mut base: Option<u64> = None;
        let mut done = false;
        let mut last_event = Instant::now();
        while !done {
            // SAFETY: the handle belongs to the live voice.
            let signalled = unsafe { WaitForSingleObject(self.notify, 10) } == WAIT_OBJECT_0;
            if signalled {
                last_event = Instant::now();
            }
            self.drain_events(|id, offset, wparam, lparam| {
                if id == SPEI_START_INPUT_STREAM.0 {
                    base.get_or_insert(offset);
                } else if id == SPEI_WORD_BOUNDARY.0 {
                    let origin = *base.get_or_insert(0);
                    let pos = u32::try_from(lparam).unwrap_or(0);
                    let len = u32::try_from(wparam).unwrap_or(0);
                    let (start, len) = prepared.to_plain(pos, len);
                    out.send(&Reply::Word {
                        token,
                        start,
                        len,
                        sample: offset.saturating_sub(origin) / 2,
                    });
                } else if id == SPEI_END_INPUT_STREAM.0 {
                    done = true;
                }
            });
            if done {
                break;
            }
            if stopped() {
                self.purge();
                return Ok((EndStatus::Aborted, 0));
            }
            if last_event.elapsed() > STALL_TIMEOUT && self.state.idle_for() > STALL_TIMEOUT {
                self.purge();
                return Err("the voice stopped responding".into());
            }
        }
        // Every audio write precedes the end of the speak call.
        // SAFETY: a COM call on a live voice.
        unsafe {
            let _ = self.voice.WaitUntilDone(5000);
        }
        Ok((EndStatus::Done, self.state.end()))
    }

    fn voices(&self, category: &str) -> Result<Vec<VoiceToken>, String> {
        // SAFETY: COM calls on this thread; `cat_id` outlives `SetId`; each
        // token returned by `Next` is owned and released by drop.
        unsafe {
            let cat: ISpObjectTokenCategory =
                CoCreateInstance(&SpObjectTokenCategory, None, CLSCTX_ALL)
                    .map_err(|e| format!("cannot create a token category: {e}"))?;
            let cat_id = wide(category);
            cat.SetId(PCWSTR(cat_id.as_ptr()), false)
                .map_err(|e| format!("no voice category {category}: {e}"))?;
            let tokens: IEnumSpObjectTokens = cat
                .EnumTokens(PCWSTR::null(), PCWSTR::null())
                .map_err(|e| format!("cannot list voices: {e}"))?;
            let mut out = Vec::new();
            loop {
                let mut t: Option<ISpObjectToken> = None;
                let mut fetched = 0u32;
                if tokens.Next(1, &mut t, Some(&mut fetched)).is_err() || fetched == 0 {
                    break;
                }
                if let Some(info) = t.as_ref().and_then(token_info) {
                    out.push(info);
                }
            }
            Ok(out)
        }
    }
}
