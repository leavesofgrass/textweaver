//! The Speech Synthesis Manager: the C API underneath `NSSpeechSynthesizer`
//! (ApplicationServices, `SpeechSynthesis.h`).
//!
//! Probes 5 and 6 (`tools/avspeech-spike/`) found that
//! `NSSpeechSynthesizer` delivers its delegate callbacks only through the
//! process's main run loop, so on the speech thread its word and finish
//! callbacks never arrive unless the application's main thread runs that
//! loop. The Speech Synthesis Manager calls its word and done callbacks on
//! its own threads, with the same voices, the same engine, and the same
//! low first-word latency, so the `nsspeech` backend drives it directly.
//!
//! Callbacks find their channel's shared state through a global registry
//! keyed by a channel number passed as the channel's reference constant, so
//! a callback that arrives after its channel was disposed finds nothing and
//! is dropped, instead of touching freed memory.

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSNumber, NSString, NSURL};

/// `SpeechChannel`: an opaque channel pointer.
type SpeechChannel = *mut c_void;
/// `OSErr`.
type OsErr = i16;

/// `VoiceSpec`: a voice's synthesizer (creator) and voice numbers.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct VoiceSpec {
    /// The synthesizer's four-character code.
    pub(crate) creator: u32,
    /// The voice number: `VoiceNumericID` in `NSSpeechSynthesizer`'s voice
    /// attributes.
    pub(crate) id: u32,
}

/// The `VoiceSpec` whose voice number is `id`. The voice attributes carry
/// the number but not the synthesizer code, so this walks the installed
/// voices (`CountVoices`, `GetIndVoice`).
pub(crate) fn find_voice(id: u32) -> Option<VoiceSpec> {
    let mut count: i16 = 0;
    // SAFETY: valid out pointer.
    if unsafe { CountVoices(&mut count) } != 0 {
        return None;
    }
    (1..=count).find_map(|index| {
        let mut spec = VoiceSpec::default();
        // SAFETY: `index` is within 1..=count; valid out pointer.
        let err = unsafe { GetIndVoice(index, &mut spec) };
        (err == 0 && spec.id == id).then_some(spec)
    })
}

/// `CFRange`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CfRange {
    location: isize,
    length: isize,
}

type WordCallback = extern "C" fn(SpeechChannel, *mut c_void, *const c_void, CfRange);
type DoneCallback = extern "C" fn(SpeechChannel, *mut c_void);

/// `PauseSpeechAt` / `StopSpeechAt` boundary: at the end of the word.
const K_END_OF_WORD: i32 = 1;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn CountVoices(count: *mut i16) -> OsErr;
    fn GetIndVoice(index: i16, voice: *mut VoiceSpec) -> OsErr;
    fn NewSpeechChannel(voice: *mut VoiceSpec, chan: *mut SpeechChannel) -> OsErr;
    fn DisposeSpeechChannel(chan: SpeechChannel) -> OsErr;
    fn SpeakCFString(
        chan: SpeechChannel,
        string: *const AnyObject,
        options: *const c_void,
    ) -> OsErr;
    fn StopSpeech(chan: SpeechChannel) -> OsErr;
    fn PauseSpeechAt(chan: SpeechChannel, where_to_pause: i32) -> OsErr;
    fn ContinueSpeech(chan: SpeechChannel) -> OsErr;
    fn SetSpeechProperty(
        chan: SpeechChannel,
        property: &NSString,
        object: *const AnyObject,
    ) -> OsErr;
    fn CopySpeechProperty(
        chan: SpeechChannel,
        property: &NSString,
        object: *mut *mut AnyObject,
    ) -> OsErr;

    static kSpeechWordCFCallBack: &'static NSString;
    static kSpeechSpeechDoneCallBack: &'static NSString;
    static kSpeechRefConProperty: &'static NSString;
    static kSpeechRateProperty: &'static NSString;
    static kSpeechPitchBaseProperty: &'static NSString;
    static kSpeechVolumeProperty: &'static NSString;
    static kSpeechOutputToFileURLProperty: &'static NSString;
}

/// What a channel's callbacks report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SsmEvent {
    /// A word is about to be spoken: its UTF-16 range in the string being
    /// spoken.
    Word {
        /// UTF-16 offset.
        location: usize,
        /// UTF-16 length.
        length: usize,
        /// When the callback ran.
        at: Instant,
    },
    /// The channel finished speaking.
    Done,
}

/// Events queued by callbacks, drained by the backend.
pub(crate) type EventQueue = Arc<Mutex<Vec<SsmEvent>>>;

fn registry() -> &'static Mutex<HashMap<u64, EventQueue>> {
    static REGISTRY: OnceLock<Mutex<HashMap<u64, EventQueue>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn push(refcon: *mut c_void, event: SsmEvent) {
    let key = refcon as usize as u64;
    let queue = {
        let map = registry().lock().unwrap_or_else(|e| e.into_inner());
        map.get(&key).cloned()
    };
    if let Some(q) = queue {
        q.lock().unwrap_or_else(|e| e.into_inner()).push(event);
    }
}

extern "C" fn on_word(
    _chan: SpeechChannel,
    refcon: *mut c_void,
    _string: *const c_void,
    range: CfRange,
) {
    push(
        refcon,
        SsmEvent::Word {
            location: usize::try_from(range.location).unwrap_or(usize::MAX),
            length: usize::try_from(range.length).unwrap_or(0),
            at: Instant::now(),
        },
    );
}

extern "C" fn on_done(_chan: SpeechChannel, refcon: *mut c_void) {
    push(refcon, SsmEvent::Done);
}

/// A Speech Synthesis Manager error code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SsmError(pub(crate) i16);

impl std::fmt::Display for SsmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Speech Synthesis Manager error {}", self.0)
    }
}

fn check(err: OsErr) -> Result<(), SsmError> {
    if err == 0 { Ok(()) } else { Err(SsmError(err)) }
}

/// One speech channel with its callbacks registered.
pub(crate) struct Channel {
    chan: SpeechChannel,
    key: u64,
    events: EventQueue,
}

impl Channel {
    /// Opens a channel for `voice` (`None`: the system default voice).
    pub(crate) fn open(voice: Option<VoiceSpec>) -> Result<Self, SsmError> {
        static NEXT_KEY: AtomicU64 = AtomicU64::new(1);
        let mut spec = voice.unwrap_or_default();
        let spec_ptr = if voice.is_some() {
            &mut spec as *mut VoiceSpec
        } else {
            std::ptr::null_mut()
        };
        let mut chan: SpeechChannel = std::ptr::null_mut();
        // SAFETY: `spec_ptr` is null or points to a live VoiceSpec; `chan`
        // is a valid out pointer.
        check(unsafe { NewSpeechChannel(spec_ptr, &mut chan) })?;
        if chan.is_null() {
            return Err(SsmError(-1));
        }
        let key = NEXT_KEY.fetch_add(1, Ordering::Relaxed);
        let events: EventQueue = Arc::default();
        registry()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key, events.clone());
        let channel = Channel { chan, key, events };
        let word: WordCallback = on_word;
        let done: DoneCallback = on_done;
        // The callbacks and the reference constant are passed as numbers
        // holding the addresses, as SpeechSynthesis.h specifies.
        channel.set_number(Key::RefCon, NSNumber::new_u64(key))?;
        channel.set_number(Key::WordCallback, NSNumber::new_usize(word as usize))?;
        channel.set_number(Key::DoneCallback, NSNumber::new_usize(done as usize))?;
        Ok(channel)
    }

    /// The events queued so far, oldest first.
    pub(crate) fn take_events(&self) -> Vec<SsmEvent> {
        std::mem::take(&mut *self.events.lock().unwrap_or_else(|e| e.into_inner()))
    }

    fn set_number(&self, key: Key, value: Retained<NSNumber>) -> Result<(), SsmError> {
        let obj: &AnyObject = &value;
        // SAFETY: the channel is open; the key is a valid property name; the
        // value is a live CFNumber (toll-free bridged NSNumber) for the
        // numeric properties this module sets.
        check(unsafe { SetSpeechProperty(self.chan, key.name(), obj) })
    }

    fn get_number(&self, key: Key) -> Option<f64> {
        let mut out: *mut AnyObject = std::ptr::null_mut();
        // SAFETY: valid channel, property name, and out pointer. The result
        // follows the Copy rule, so it is retained and we take ownership.
        let err = unsafe { CopySpeechProperty(self.chan, key.name(), &mut out) };
        if err != 0 || out.is_null() {
            return None;
        }
        // SAFETY: `out` is a +1 retained object from a Copy function.
        let obj: Retained<AnyObject> = unsafe { Retained::from_raw(out) }?;
        obj.downcast_ref::<NSNumber>().map(|n| n.as_f64())
    }

    /// Sets the rate in words per minute.
    pub(crate) fn set_rate(&self, wpm: f64) -> Result<(), SsmError> {
        self.set_number(Key::Rate, NSNumber::new_f64(wpm))
    }

    /// Sets the pitch base (a MIDI-like note number; 44 is Reed's default).
    pub(crate) fn set_pitch_base(&self, base: f64) -> Result<(), SsmError> {
        self.set_number(Key::PitchBase, NSNumber::new_f64(base))
    }

    /// The current pitch base.
    pub(crate) fn pitch_base(&self) -> Option<f64> {
        self.get_number(Key::PitchBase)
    }

    /// Sets the volume, `0.0..=1.0`.
    pub(crate) fn set_volume(&self, volume: f64) -> Result<(), SsmError> {
        self.set_number(Key::Volume, NSNumber::new_f64(volume.clamp(0.0, 1.0)))
    }

    /// Sends speech to an AIFF file at `url` instead of the speakers (or
    /// back to the speakers with `None`).
    pub(crate) fn set_output_file(&self, url: Option<&NSURL>) -> Result<(), SsmError> {
        let obj: *const AnyObject = url.map_or(std::ptr::null(), |u| {
            let a: &AnyObject = u;
            a as *const AnyObject
        });
        // SAFETY: valid channel and key; the value is null or a live CFURL
        // (toll-free bridged NSURL).
        check(unsafe { SetSpeechProperty(self.chan, Key::OutputFile.name(), obj) })
    }

    /// Starts speaking `text`; returns at once.
    pub(crate) fn speak(&self, text: &NSString) -> Result<(), SsmError> {
        let obj: &AnyObject = text;
        // SAFETY: valid channel; `text` is a live CFString (toll-free bridged
        // NSString) which the caller keeps alive while it is spoken.
        check(unsafe { SpeakCFString(self.chan, obj, std::ptr::null()) })
    }

    /// Pauses at the end of the current word.
    pub(crate) fn pause(&self) -> Result<(), SsmError> {
        // SAFETY: valid channel.
        check(unsafe { PauseSpeechAt(self.chan, K_END_OF_WORD) })
    }

    /// Resumes after a pause.
    pub(crate) fn resume(&self) -> Result<(), SsmError> {
        // SAFETY: valid channel.
        check(unsafe { ContinueSpeech(self.chan) })
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        // Unregister first: callbacks racing with disposal find nothing.
        registry()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.key);
        // SAFETY: the channel is valid and disposed exactly once.
        unsafe {
            let _ = StopSpeech(self.chan);
            let _ = DisposeSpeechChannel(self.chan);
        }
    }
}

#[derive(Clone, Copy)]
enum Key {
    WordCallback,
    DoneCallback,
    RefCon,
    Rate,
    PitchBase,
    Volume,
    OutputFile,
}

impl Key {
    fn name(self) -> &'static NSString {
        // SAFETY: these are immutable CFString constants exported by
        // ApplicationServices.
        unsafe {
            match self {
                Key::WordCallback => kSpeechWordCFCallBack,
                Key::DoneCallback => kSpeechSpeechDoneCallBack,
                Key::RefCon => kSpeechRefConProperty,
                Key::Rate => kSpeechRateProperty,
                Key::PitchBase => kSpeechPitchBaseProperty,
                Key::Volume => kSpeechVolumeProperty,
                Key::OutputFile => kSpeechOutputToFileURLProperty,
            }
        }
    }
}
