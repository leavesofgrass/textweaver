//! The `nsspeech` backend: Apple's classic speech engine, the one behind
//! `NSSpeechSynthesizer`, driven through the Speech Synthesis Manager.
//!
//! `NSSpeechSynthesizer` itself delivers its delegate callbacks only through
//! the process's main run loop (probe 5), so on the speech thread it would
//! speak without ever reporting a word or its end. The Speech Synthesis
//! Manager, which `NSSpeechSynthesizer` wraps, calls its word and done
//! callbacks on its own threads (probe 6), so this backend uses it directly:
//! same voices (enumerated through `NSSpeechSynthesizer`), same engine, same
//! first-word latency, and no dependency on the main thread.
//!
//! - The system plays the audio; `Word` events arrive as each word is about
//!   to be spoken (no `audio_ms`).
//! - Rate is words per minute natively, mapped through [`crate::rate`];
//!   pitch is the channel's pitch base (a note number) plus the semitone
//!   offset; volume is `0.0..=1.0`.
//! - Pause is native, at the end of the current word; resume continues from
//!   there.
//! - `synthesize_to_file` speaks into an AIFF file on a separate channel and
//!   converts it to WAV unless the path ends in `.aif` or `.aiff`.

#![allow(deprecated)] // NSSpeechSynthesizer's voice listing is deprecated but current.

use std::collections::VecDeque;
use std::path::Path;
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2_app_kit::{NSSpeechSynthesizer, NSVoiceGender, NSVoiceLocaleIdentifier, NSVoiceName};
use objc2_foundation::{NSNumber, NSProcessInfo, NSString, NSURL};
use textweaver_core::{Utterance, UtteranceId};
use textweaver_speech::{
    BackendId, Caps, EventSink, RawEvent, SpeechBackend, SpeechError, Voice, VoiceParams,
};

use super::ssm::{Channel, SsmEvent, VoiceSpec};
use crate::range::WordTracker;
use crate::{rate, voices};

const ID: BackendId = crate::NSSPEECH_ID;

/// Default pitch base when a channel does not report one (Reed's and
/// Samantha's measured value).
const DEFAULT_PITCH_BASE: f64 = 44.0;

/// The utterance being spoken.
struct Current {
    id: UtteranceId,
    text: String,
    /// Kept alive while the engine speaks it.
    _ns: Retained<NSString>,
    tracker: WordTracker,
    /// When `speak` was asked to start it (before opening a channel).
    started: Instant,
    /// A word callback has arrived.
    heard: bool,
}

/// Apple's classic speech engine (`NSSpeechSynthesizer`'s engine) through the
/// Speech Synthesis Manager. See the module docs.
pub struct NsSpeechBackend {
    channel: Option<Channel>,
    /// Voice the open channel speaks with.
    channel_voice: Option<String>,
    /// The channel's own pitch base, before any offset.
    pitch_base: f64,
    params: VoiceParams,
    /// The voice in use: `params.voice`, or the default voice.
    voice: Option<String>,
    current: Option<Current>,
    queue: VecDeque<Utterance>,
    paused: bool,
    /// Events produced where no sink is at hand (`stop`), delivered next.
    deferred: Vec<(UtteranceId, RawEvent)>,
    first_word_latency: Option<Duration>,
    raw_rate: Option<f64>,
}

fn unavailable(msg: impl Into<String>) -> SpeechError {
    SpeechError::Unavailable(ID, msg.into())
}

fn engine(e: impl std::fmt::Display) -> SpeechError {
    SpeechError::Engine(e.to_string())
}

/// The running macOS major version (14, 15, 26, ...).
fn os_major() -> u32 {
    let v = NSProcessInfo::processInfo().operatingSystemVersion();
    u32::try_from(v.majorVersion).unwrap_or(0)
}

/// Installed voice identifiers.
fn voice_ids() -> Vec<String> {
    NSSpeechSynthesizer::availableVoices()
        .iter()
        .map(|v| v.to_string())
        .collect()
}

fn attribute(id: &NSString, key: &NSString) -> Option<String> {
    let attrs = NSSpeechSynthesizer::attributesForVoice(id);
    let value = attrs.objectForKey(key)?;
    value.downcast_ref::<NSString>().map(|s| s.to_string())
}

/// The Speech Synthesis Manager's `VoiceSpec` for a voice identifier:
/// `VoiceNumericID` from the voice's attributes, matched against the
/// installed voices.
fn voice_spec(id: &str) -> Option<VoiceSpec> {
    let attrs = NSSpeechSynthesizer::attributesForVoice(&NSString::from_str(id));
    let value = attrs.objectForKey(&NSString::from_str("VoiceNumericID"))?;
    let n = value.downcast_ref::<NSNumber>()?;
    // A four-character code, reported as a signed number: keep its bits.
    super::ssm::find_voice(n.as_i64() as u32)
}

impl NsSpeechBackend {
    /// Creates the backend (on the speech thread). Fails when the system
    /// has no voices.
    pub fn new() -> Result<Self, SpeechError> {
        let ids = voice_ids();
        if ids.is_empty() {
            return Err(unavailable("no system voices"));
        }
        let voice = voices::default_voice(ids.iter().map(String::as_str)).map(str::to_string);
        Ok(NsSpeechBackend {
            channel: None,
            channel_voice: None,
            pitch_base: DEFAULT_PITCH_BASE,
            params: VoiceParams::default(),
            voice,
            current: None,
            queue: VecDeque::new(),
            paused: false,
            deferred: Vec::new(),
            first_word_latency: None,
            raw_rate: None,
        })
    }

    /// The voice in use (the requested one, or the default, Eloquence Reed
    /// when installed; `None` is the system default voice).
    pub fn voice(&self) -> Option<&str> {
        self.voice.as_deref()
    }

    /// Time from asking the engine to speak the most recent utterance
    /// (including opening a channel when needed) to its first word
    /// callback, as measured at the callback.
    pub fn first_word_latency(&self) -> Option<Duration> {
        self.first_word_latency
    }

    /// Bypasses the rate table and sets the engine's rate property directly
    /// (`None` restores the table). For calibration measurements.
    pub fn set_raw_rate(&mut self, engine_rate: Option<f64>) {
        self.raw_rate = engine_rate;
    }

    /// Opens a channel for the current voice and applies the parameters.
    fn open_channel(&self) -> Result<(Channel, f64), SpeechError> {
        let spec = match &self.voice {
            Some(id) => Some(voice_spec(id).ok_or_else(|| SpeechError::UnknownVoice(id.clone()))?),
            None => None,
        };
        let channel = Channel::open(spec).map_err(engine)?;
        let base = channel.pitch_base().unwrap_or(DEFAULT_PITCH_BASE);
        Ok((channel, base))
    }

    fn ensure_channel(&mut self) -> Result<&Channel, SpeechError> {
        if self.channel.is_none() || self.channel_voice != self.voice {
            self.channel = None;
            let (channel, base) = self.open_channel()?;
            self.channel = Some(channel);
            self.channel_voice = self.voice.clone();
            self.pitch_base = base;
        }
        let channel = self.channel.as_ref().ok_or_else(|| engine("no channel"))?;
        apply_params(
            channel,
            &self.params,
            self.voice.as_deref(),
            self.pitch_base,
            self.raw_rate,
        );
        Ok(channel)
    }

    fn flush_deferred(&mut self, sink: &mut dyn EventSink) {
        for (id, ev) in self.deferred.drain(..) {
            sink.emit(id, ev);
        }
    }

    /// Starts the next queued utterance unless paused or busy.
    fn start_next(&mut self, sink: &mut dyn EventSink) {
        while self.current.is_none() && !self.paused {
            let Some(u) = self.queue.pop_front() else {
                return;
            };
            if !sink.is_current(u.id) {
                sink.emit(u.id, RawEvent::Cancelled);
                continue;
            }
            if u.text.trim().is_empty() {
                sink.emit(u.id, RawEvent::Started);
                sink.emit(u.id, RawEvent::Finished);
                continue;
            }
            let t0 = Instant::now();
            let ns = NSString::from_str(&u.text);
            let started = match self.ensure_channel() {
                Ok(channel) => channel.speak(&ns).map_err(engine),
                Err(e) => Err(e),
            };
            if let Err(e) = started {
                // A failed chunk reports the error and ends, so reading can
                // go on with the next one.
                sink.emit(u.id, RawEvent::Error(e.to_string()));
                sink.emit(u.id, RawEvent::Finished);
                continue;
            }
            sink.emit(u.id, RawEvent::Started);
            self.current = Some(Current {
                id: u.id,
                tracker: WordTracker::new(&u.text),
                text: u.text,
                _ns: ns,
                started: t0,
                heard: false,
            });
        }
    }

    fn handle_events(&mut self, sink: &mut dyn EventSink) {
        let Some(channel) = &self.channel else {
            return;
        };
        for event in channel.take_events() {
            match event {
                SsmEvent::Word {
                    location,
                    length,
                    at,
                } => {
                    let Some(cur) = self.current.as_mut() else {
                        continue;
                    };
                    if !cur.heard {
                        cur.heard = true;
                        self.first_word_latency = Some(at.saturating_duration_since(cur.started));
                    }
                    if let Some(range) = cur.tracker.word(&cur.text, location, length) {
                        sink.emit(
                            cur.id,
                            RawEvent::Word {
                                byte_range: range,
                                audio_ms: None,
                            },
                        );
                    }
                }
                SsmEvent::Done => {
                    if let Some(cur) = self.current.take() {
                        sink.emit(cur.id, RawEvent::Finished);
                    }
                }
            }
        }
    }
}

/// Applies rate (`raw_rate`, or the canonical rate through the table),
/// pitch, and volume to a channel.
fn apply_params(
    channel: &Channel,
    params: &VoiceParams,
    voice: Option<&str>,
    pitch_base: f64,
    raw_rate: Option<f64>,
) {
    let table = rate::ns_table(voice, os_major());
    let engine_rate =
        raw_rate.unwrap_or_else(|| f64::from(table.engine_value(params.rate.clamped().wpm())));
    if let Err(e) = channel.set_rate(engine_rate) {
        log::debug!("nsspeech: rate {engine_rate}: {e}");
    }
    let pitch = pitch_base + f64::from(params.pitch.clamped().semitones());
    if let Err(e) = channel.set_pitch_base(pitch) {
        log::debug!("nsspeech: pitch {pitch}: {e}");
    }
    if let Err(e) = channel.set_volume(f64::from(params.volume.fraction())) {
        log::debug!("nsspeech: volume: {e}");
    }
}

impl SpeechBackend for NsSpeechBackend {
    fn id(&self) -> BackendId {
        ID
    }

    fn capabilities(&self) -> Caps {
        Caps::WORD_EVENTS | Caps::PAUSE | Caps::PITCH | Caps::VOLUME | Caps::SYNTH_TO_FILE
    }

    fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        let mut out: Vec<Voice> = NSSpeechSynthesizer::availableVoices()
            .iter()
            .map(|id| {
                let name = attribute(&id, NSVoiceName_key()).unwrap_or_default();
                let lang = attribute(&id, NSVoiceLocaleIdentifier_key()).unwrap_or_default();
                let gender = attribute(&id, NSVoiceGender_key())
                    .map(|g| g.trim_start_matches("VoiceGender").to_ascii_lowercase());
                voices::voice(&id.to_string(), &name, &lang, gender.as_deref())
            })
            .collect();
        voices::sort_voices(&mut out);
        Ok(out)
    }

    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
        let voice = match &params.voice {
            Some(id) => {
                if voice_spec(id).is_none() {
                    return Err(SpeechError::UnknownVoice(id.clone()));
                }
                Some(id.clone())
            }
            None => {
                voices::default_voice(voice_ids().iter().map(String::as_str)).map(str::to_string)
            }
        };
        self.params = params.clone();
        self.voice = voice;
        Ok(())
    }

    fn effective_wpm(&self) -> u16 {
        rate::ns_table(self.voice.as_deref(), os_major())
            .effective_wpm(self.params.rate.clamped().wpm())
    }

    fn speak(
        &mut self,
        utterance: &Utterance,
        sink: &mut dyn EventSink,
    ) -> Result<(), SpeechError> {
        self.flush_deferred(sink);
        self.queue.push_back(utterance.clone());
        self.start_next(sink);
        Ok(())
    }

    fn poll(&mut self, sink: &mut dyn EventSink) {
        self.flush_deferred(sink);
        self.handle_events(sink);
        self.start_next(sink);
    }

    fn stop(&mut self) {
        if let Some(cur) = self.current.take() {
            self.deferred.push((cur.id, RawEvent::Cancelled));
            // Closing the channel stops it and guarantees that no callback
            // from the stopped speech reaches the next utterance.
            self.channel = None;
        }
        for u in self.queue.drain(..) {
            self.deferred.push((u.id, RawEvent::Cancelled));
        }
        self.paused = false;
    }

    fn pause(&mut self) -> Result<(), SpeechError> {
        self.paused = true;
        if self.current.is_some()
            && let Some(channel) = &self.channel
        {
            channel.pause().map_err(engine)?;
        }
        Ok(())
    }

    fn resume(&mut self) -> Result<(), SpeechError> {
        if !self.paused {
            return Ok(());
        }
        self.paused = false;
        // With nothing in progress, the next `poll` starts the queue.
        if self.current.is_some()
            && let Some(channel) = &self.channel
        {
            channel.resume().map_err(engine)?;
        }
        Ok(())
    }

    fn synthesize_to_file(&mut self, text: &str, path: &Path) -> Result<(), SpeechError> {
        let (channel, base) = self.open_channel()?;
        apply_params(
            &channel,
            &self.params,
            self.voice.as_deref(),
            base,
            self.raw_rate,
        );
        let aiff_path = aiff_path_for(path);
        let url = NSURL::from_file_path(&aiff_path)
            .ok_or_else(|| SpeechError::Io(format!("bad path {}", aiff_path.display())))?;
        channel.set_output_file(Some(&url)).map_err(engine)?;
        let ns = NSString::from_str(text);
        channel.speak(&ns).map_err(engine)?;
        // Synthesis to a file runs far faster than real time; allow an hour
        // of speech at the slowest rate before giving up.
        let deadline = Instant::now()
            + Duration::from_secs(60)
            + Duration::from_millis(text.len() as u64 * 100);
        let mut done = false;
        while !done && Instant::now() < deadline {
            done = channel.take_events().contains(&SsmEvent::Done);
            if !done {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        drop(ns);
        drop(channel);
        if !done {
            return Err(SpeechError::Engine("synthesis to file timed out".into()));
        }
        finish_file(&aiff_path, path)
    }
}

/// The AIFF path the engine writes for an export to `path`.
fn aiff_path_for(path: &Path) -> std::path::PathBuf {
    if is_aiff(path) {
        path.to_path_buf()
    } else {
        let mut p = path.as_os_str().to_owned();
        p.push(".tmp.aiff");
        std::path::PathBuf::from(p)
    }
}

fn is_aiff(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("aiff") || e.eq_ignore_ascii_case("aif"))
}

/// Converts the engine's AIFF to WAV at `path` unless `path` is AIFF.
fn finish_file(aiff: &Path, path: &Path) -> Result<(), SpeechError> {
    if aiff == path {
        return Ok(());
    }
    let result = std::fs::read(aiff)
        .map_err(|e| SpeechError::Io(e.to_string()))
        .and_then(|bytes| {
            crate::audio::read_aiff(&bytes).map_err(|e| SpeechError::Io(e.to_string()))
        })
        .and_then(|pcm| {
            crate::audio::write_wav(path, &pcm).map_err(|e| SpeechError::Io(e.to_string()))
        });
    let _ = std::fs::remove_file(aiff);
    result
}

#[allow(non_snake_case)]
fn NSVoiceName_key() -> &'static NSString {
    // SAFETY: an immutable constant string exported by AppKit.
    unsafe { NSVoiceName }
}

#[allow(non_snake_case)]
fn NSVoiceLocaleIdentifier_key() -> &'static NSString {
    // SAFETY: an immutable constant string exported by AppKit.
    unsafe { NSVoiceLocaleIdentifier }
}

#[allow(non_snake_case)]
fn NSVoiceGender_key() -> &'static NSString {
    // SAFETY: an immutable constant string exported by AppKit.
    unsafe { NSVoiceGender }
}
